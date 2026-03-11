use crate::metrics::Metrics;
use crate::network::Network;
use crate::types::{NodeId, Value};
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot;

#[derive(Clone, Debug, PartialEq)]
pub enum HsPhase {
    Prepare,
    PreCommit,
    Commit,
}

#[derive(Clone, Debug)]
pub enum HsMsg {
    Prepare {
        value: Value,
    },
    PreCommit {
        value: Value,
    },
    Commit {
        value: Value,
    },
    Decide {
        value: Value,
    },
    Vote {
        phase: HsPhase,
        sender: NodeId,
        value: Value,
    },
}

// ---- Leader node ----

struct HsLeader {
    node_id: NodeId,
    net: Network<HsMsg>,
    rx: UnboundedReceiver<(NodeId, HsMsg)>,
    decide_tx: oneshot::Sender<Value>,
    value: Value,
    quorum: usize,
}

impl HsLeader {
    async fn run(mut self) {
        let mut prepare_votes: HashMap<NodeId, Value> = HashMap::new();
        let mut precommit_votes: HashMap<NodeId, Value> = HashMap::new();
        let mut commit_votes: HashMap<NodeId, Value> = HashMap::new();

        // Phase 1: send Prepare to all replicas
        self.net
            .broadcast(self.node_id, HsMsg::Prepare { value: self.value });

        loop {
            let (_from, msg) = match self.rx.recv().await {
                Some(m) => m,
                None => break,
            };

            if let HsMsg::Vote {
                phase,
                sender,
                value,
            } = msg
            {
                match phase {
                    HsPhase::Prepare => {
                        prepare_votes.insert(sender, value);
                        if prepare_votes.len() >= self.quorum {
                            let matching =
                                prepare_votes.values().filter(|&&v| v == self.value).count();
                            if matching >= self.quorum {
                                // Phase 2: broadcast PreCommit QC
                                self.net.broadcast(
                                    self.node_id,
                                    HsMsg::PreCommit { value: self.value },
                                );
                                // Clear to avoid re-triggering
                                prepare_votes.clear();
                            }
                        }
                    }
                    HsPhase::PreCommit => {
                        precommit_votes.insert(sender, value);
                        if precommit_votes.len() >= self.quorum {
                            let matching = precommit_votes
                                .values()
                                .filter(|&&v| v == self.value)
                                .count();
                            if matching >= self.quorum {
                                // Phase 3: broadcast Commit QC
                                self.net
                                    .broadcast(self.node_id, HsMsg::Commit { value: self.value });
                                precommit_votes.clear();
                            }
                        }
                    }
                    HsPhase::Commit => {
                        commit_votes.insert(sender, value);
                        if commit_votes.len() >= self.quorum {
                            let matching =
                                commit_votes.values().filter(|&&v| v == self.value).count();
                            if matching >= self.quorum {
                                // Phase 4: broadcast Decide — leader also decides
                                self.net
                                    .broadcast(self.node_id, HsMsg::Decide { value: self.value });
                                let _ = self.decide_tx.send(self.value);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---- Replica node ----

struct HsReplica {
    node_id: NodeId,
    net: Network<HsMsg>,
    rx: UnboundedReceiver<(NodeId, HsMsg)>,
    decide_tx: oneshot::Sender<Value>,
    leader_id: NodeId,
}

impl HsReplica {
    async fn run(mut self) {
        loop {
            let (from, msg) = match self.rx.recv().await {
                Some(m) => m,
                None => break,
            };

            if from != self.leader_id {
                continue;
            }

            match msg {
                HsMsg::Prepare { value } => {
                    // Vote for Prepare — send only to leader
                    self.net.send(
                        self.node_id,
                        self.leader_id,
                        HsMsg::Vote {
                            phase: HsPhase::Prepare,
                            sender: self.node_id,
                            value,
                        },
                    );
                }
                HsMsg::PreCommit { value } => {
                    // Vote for PreCommit — send only to leader
                    self.net.send(
                        self.node_id,
                        self.leader_id,
                        HsMsg::Vote {
                            phase: HsPhase::PreCommit,
                            sender: self.node_id,
                            value,
                        },
                    );
                }
                HsMsg::Commit { value } => {
                    // Vote for Commit — send only to leader
                    self.net.send(
                        self.node_id,
                        self.leader_id,
                        HsMsg::Vote {
                            phase: HsPhase::Commit,
                            sender: self.node_id,
                            value,
                        },
                    );
                }
                HsMsg::Decide { value } => {
                    let _ = self.decide_tx.send(value);
                    break;
                }
                // Replicas ignore Vote messages
                HsMsg::Vote { .. } => {}
            }
        }
    }
}

pub async fn run_hotstuff(n: usize, value: Value) -> Metrics {
    let node_ids: Vec<NodeId> = (0..n as u64).collect();
    let f = (n - 1) / 3;
    let quorum = 2 * f + 1;
    let leader_id: NodeId = 0;

    let (net, mut receivers) = Network::<HsMsg>::new(node_ids.clone());

    let start = Instant::now();

    let mut decision_senders: Vec<oneshot::Sender<Value>> = Vec::new();
    let mut decision_receivers: Vec<oneshot::Receiver<Value>> = Vec::new();
    for _ in 0..n {
        let (tx, rx) = oneshot::channel();
        decision_senders.push(tx);
        decision_receivers.push(rx);
    }

    let mut handles = Vec::new();
    for &node_id in node_ids.iter() {
        let net_clone = net.clone();
        let rx = receivers.remove(&node_id).unwrap();
        let decide_tx = decision_senders.remove(0);

        if node_id == leader_id {
            let leader = HsLeader {
                node_id,
                net: net_clone,
                rx,
                decide_tx,
                value,
                quorum,
            };
            let handle = tokio::spawn(async move { leader.run().await });
            handles.push(handle);
        } else {
            let replica = HsReplica {
                node_id,
                net: net_clone,
                rx,
                decide_tx,
                leader_id,
            };
            let handle = tokio::spawn(async move { replica.run().await });
            handles.push(handle);
        }
    }

    // Wait for all nodes to decide
    for rx in decision_receivers {
        rx.await.unwrap();
    }

    let duration_us = start.elapsed().as_micros() as u64;

    for handle in handles {
        handle.await.unwrap();
    }

    let message_count = net.message_count();

    Metrics::new(message_count, 1, duration_us)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hotstuff_4_nodes() {
        let metrics = run_hotstuff(4, 42).await;
        assert!(metrics.message_count > 0);
        assert_eq!(metrics.rounds, 1);
    }

    #[tokio::test]
    async fn test_hotstuff_7_nodes() {
        let metrics = run_hotstuff(7, 99).await;
        assert!(metrics.message_count > 0);
        assert_eq!(metrics.rounds, 1);
    }

    #[test]
    fn test_hs_phase_eq() {
        assert_eq!(HsPhase::Prepare, HsPhase::Prepare);
        assert_ne!(HsPhase::Prepare, HsPhase::Commit);
    }

    #[test]
    fn test_hs_msg_clone() {
        let msg = HsMsg::Prepare { value: 42 };
        let msg2 = msg.clone();
        match msg2 {
            HsMsg::Prepare { value } => assert_eq!(value, 42),
            _ => panic!("unexpected variant"),
        }
    }
}
