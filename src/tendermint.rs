use crate::metrics::Metrics;
use crate::network::Network;
use crate::types::{NodeId, Value};
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot;

#[derive(Clone, Debug)]
pub enum TmMsg {
    Proposal {
        round: u64,
        value: Value,
    },
    Prevote {
        round: u64,
        sender: NodeId,
        value: Value,
    },
    Precommit {
        round: u64,
        sender: NodeId,
        value: Value,
    },
}

struct TmNode {
    node_id: NodeId,
    net: Network<TmMsg>,
    rx: UnboundedReceiver<(NodeId, TmMsg)>,
    decide_tx: oneshot::Sender<Value>,
    is_proposer: bool,
    value: Value,
    round: u64,
    quorum: usize,
}

impl TmNode {
    async fn run(mut self) {
        let mut prevotes: HashMap<NodeId, Value> = HashMap::new();
        let mut precommits: HashMap<NodeId, Value> = HashMap::new();
        let mut prevoted = false;
        let mut precommitted = false;

        // Proposer sends proposal to all replicas and immediately prevotes for its own proposal
        if self.is_proposer {
            self.net.broadcast(
                self.node_id,
                TmMsg::Proposal {
                    round: self.round,
                    value: self.value,
                },
            );
            // Proposer prevotes for its own value and broadcasts to all other nodes
            self.net.broadcast(
                self.node_id,
                TmMsg::Prevote {
                    round: self.round,
                    sender: self.node_id,
                    value: self.value,
                },
            );
            prevotes.insert(self.node_id, self.value);
            prevoted = true;
        }

        loop {
            let (from, msg) = match self.rx.recv().await {
                Some(m) => m,
                None => break,
            };

            match msg {
                TmMsg::Proposal {
                    round: msg_round,
                    value: msg_value,
                } => {
                    if msg_round == self.round && from == 0 && !prevoted {
                        prevoted = true;
                        self.net.broadcast(
                            self.node_id,
                            TmMsg::Prevote {
                                round: self.round,
                                sender: self.node_id,
                                value: msg_value,
                            },
                        );
                        prevotes.insert(self.node_id, msg_value);
                    }
                }
                TmMsg::Prevote {
                    round: msg_round,
                    sender,
                    value: msg_value,
                } => {
                    if msg_round == self.round {
                        prevotes.insert(sender, msg_value);

                        if !precommitted && prevotes.len() >= self.quorum {
                            let matching = prevotes.values().filter(|&&v| v == self.value).count();
                            if matching >= self.quorum {
                                precommitted = true;
                                self.net.broadcast(
                                    self.node_id,
                                    TmMsg::Precommit {
                                        round: self.round,
                                        sender: self.node_id,
                                        value: self.value,
                                    },
                                );
                                precommits.insert(self.node_id, self.value);
                            }
                        }
                    }
                }
                TmMsg::Precommit {
                    round: msg_round,
                    sender,
                    value: msg_value,
                } => {
                    if msg_round == self.round {
                        precommits.insert(sender, msg_value);

                        if precommits.len() >= self.quorum {
                            let matching =
                                precommits.values().filter(|&&v| v == self.value).count();
                            if matching >= self.quorum {
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

pub async fn run_tendermint(n: usize, value: Value) -> Metrics {
    let node_ids: Vec<NodeId> = (0..n as u64).collect();
    let f = (n - 1) / 3;
    let quorum = 2 * f + 1;
    let round: u64 = 0;

    let (net, mut receivers) = Network::<TmMsg>::new(node_ids.clone());

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
        let is_proposer = node_id == 0;

        let node = TmNode {
            node_id,
            net: net_clone,
            rx,
            decide_tx,
            is_proposer,
            value,
            round,
            quorum,
        };

        let handle = tokio::spawn(async move { node.run().await });
        handles.push(handle);
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
    async fn test_tendermint_4_nodes() {
        let metrics = run_tendermint(4, 42).await;
        assert!(metrics.message_count > 0);
        assert_eq!(metrics.rounds, 1);
    }

    #[tokio::test]
    async fn test_tendermint_7_nodes() {
        let metrics = run_tendermint(7, 99).await;
        assert!(metrics.message_count > 0);
        assert_eq!(metrics.rounds, 1);
    }

    #[test]
    fn test_tm_msg_clone() {
        let msg = TmMsg::Proposal { round: 0, value: 1 };
        let msg2 = msg.clone();
        match msg2 {
            TmMsg::Proposal { round, value } => {
                assert_eq!(round, 0);
                assert_eq!(value, 1);
            }
            _ => panic!("unexpected variant"),
        }
    }
}
