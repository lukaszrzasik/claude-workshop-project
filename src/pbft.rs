use crate::metrics::Metrics;
use crate::network::Network;
use crate::types::{NodeId, Value};
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot;

#[derive(Clone, Debug)]
pub enum PbftMsg {
    PrePrepare {
        seq: u64,
        value: Value,
    },
    Prepare {
        seq: u64,
        sender: NodeId,
        value: Value,
    },
    Commit {
        seq: u64,
        sender: NodeId,
        value: Value,
    },
}

struct PbftNode {
    node_id: NodeId,
    net: Network<PbftMsg>,
    rx: UnboundedReceiver<(NodeId, PbftMsg)>,
    decide_tx: oneshot::Sender<Value>,
    is_primary: bool,
    value: Value,
    seq: u64,
    quorum: usize,
}

impl PbftNode {
    async fn run(mut self) {
        let mut prepare_msgs: HashMap<NodeId, Value> = HashMap::new();
        let mut commit_msgs: HashMap<NodeId, Value> = HashMap::new();
        let mut prepared = false;

        // Primary sends pre-prepare
        if self.is_primary {
            self.net.broadcast(
                self.node_id,
                PbftMsg::PrePrepare {
                    seq: self.seq,
                    value: self.value,
                },
            );
            // Primary counts its own prepare
            self.net.broadcast(
                self.node_id,
                PbftMsg::Prepare {
                    seq: self.seq,
                    sender: self.node_id,
                    value: self.value,
                },
            );
            prepare_msgs.insert(self.node_id, self.value);
        }

        loop {
            let (from, msg) = match self.rx.recv().await {
                Some(m) => m,
                None => break,
            };

            match msg {
                PbftMsg::PrePrepare {
                    seq: msg_seq,
                    value: msg_value,
                } => {
                    if msg_seq == self.seq && from == 0 && !self.is_primary {
                        self.net.broadcast(
                            self.node_id,
                            PbftMsg::Prepare {
                                seq: self.seq,
                                sender: self.node_id,
                                value: msg_value,
                            },
                        );
                        prepare_msgs.insert(self.node_id, msg_value);
                    }
                }
                PbftMsg::Prepare {
                    seq: msg_seq,
                    sender,
                    value: msg_value,
                } => {
                    if msg_seq == self.seq {
                        prepare_msgs.insert(sender, msg_value);

                        if !prepared && prepare_msgs.len() >= self.quorum {
                            let matching =
                                prepare_msgs.values().filter(|&&v| v == self.value).count();
                            if matching >= self.quorum {
                                prepared = true;
                                self.net.broadcast(
                                    self.node_id,
                                    PbftMsg::Commit {
                                        seq: self.seq,
                                        sender: self.node_id,
                                        value: self.value,
                                    },
                                );
                                commit_msgs.insert(self.node_id, self.value);
                            }
                        }
                    }
                }
                PbftMsg::Commit {
                    seq: msg_seq,
                    sender,
                    value: msg_value,
                } => {
                    if msg_seq == self.seq {
                        commit_msgs.insert(sender, msg_value);

                        if commit_msgs.len() >= self.quorum {
                            let matching =
                                commit_msgs.values().filter(|&&v| v == self.value).count();
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

pub async fn run_pbft(n: usize, value: Value) -> Metrics {
    let node_ids: Vec<NodeId> = (0..n as u64).collect();
    let f = (n - 1) / 3;
    let quorum = 2 * f + 1;
    let seq: u64 = 1;

    let (net, mut receivers) = Network::<PbftMsg>::new(node_ids.clone());

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
        let is_primary = node_id == 0;

        let node = PbftNode {
            node_id,
            net: net_clone,
            rx,
            decide_tx,
            is_primary,
            value,
            seq,
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
    async fn test_pbft_4_nodes() {
        let metrics = run_pbft(4, 42).await;
        assert!(metrics.message_count > 0);
        assert_eq!(metrics.rounds, 1);
    }

    #[tokio::test]
    async fn test_pbft_7_nodes() {
        let metrics = run_pbft(7, 99).await;
        assert!(metrics.message_count > 0);
        assert_eq!(metrics.rounds, 1);
    }

    #[test]
    fn test_pbft_msg_clone() {
        let msg = PbftMsg::PrePrepare { seq: 1, value: 42 };
        let msg2 = msg.clone();
        match msg2 {
            PbftMsg::PrePrepare { seq, value } => {
                assert_eq!(seq, 1);
                assert_eq!(value, 42);
            }
            _ => panic!("unexpected variant"),
        }
    }
}
