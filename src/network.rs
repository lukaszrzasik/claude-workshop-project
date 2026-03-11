use crate::types::NodeId;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

type SenderMap<Msg> = Arc<Mutex<HashMap<NodeId, mpsc::UnboundedSender<(NodeId, Msg)>>>>;

pub struct Network<Msg: Clone + Send + 'static> {
    senders: SenderMap<Msg>,
    message_count: Arc<AtomicU64>,
    node_ids: Vec<NodeId>,
}

impl<Msg: Clone + Send + 'static> Network<Msg> {
    pub fn new(
        node_ids: Vec<NodeId>,
    ) -> (
        Self,
        HashMap<NodeId, mpsc::UnboundedReceiver<(NodeId, Msg)>>,
    ) {
        let mut senders = HashMap::new();
        let mut receivers = HashMap::new();

        for &id in &node_ids {
            let (tx, rx) = mpsc::unbounded_channel();
            senders.insert(id, tx);
            receivers.insert(id, rx);
        }

        let net = Network {
            senders: Arc::new(Mutex::new(senders)),
            message_count: Arc::new(AtomicU64::new(0)),
            node_ids: node_ids.clone(),
        };

        (net, receivers)
    }

    pub fn send(&self, from: NodeId, to: NodeId, msg: Msg) {
        let senders = self.senders.lock().unwrap();
        if let Some(tx) = senders.get(&to) {
            let _ = tx.send((from, msg));
            self.message_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn broadcast(&self, from: NodeId, msg: Msg) {
        let senders = self.senders.lock().unwrap();
        for &id in &self.node_ids {
            if id != from {
                if let Some(tx) = senders.get(&id) {
                    let _ = tx.send((from, msg.clone()));
                    self.message_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    pub fn message_count(&self) -> u64 {
        self.message_count.load(Ordering::Relaxed)
    }
}

impl<Msg: Clone + Send + 'static> Clone for Network<Msg> {
    fn clone(&self) -> Self {
        Network {
            senders: Arc::clone(&self.senders),
            message_count: Arc::clone(&self.message_count),
            node_ids: self.node_ids.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_send() {
        let node_ids: Vec<NodeId> = vec![0, 1, 2];
        let (net, mut receivers) = Network::<u64>::new(node_ids);

        net.send(0, 1, 42u64);
        assert_eq!(net.message_count(), 1);

        let rx1 = receivers.get_mut(&1).unwrap();
        let (from, val) = rx1.recv().await.unwrap();
        assert_eq!(from, 0);
        assert_eq!(val, 42);
    }

    #[tokio::test]
    async fn test_network_broadcast() {
        let node_ids: Vec<NodeId> = vec![0, 1, 2];
        let (net, mut receivers) = Network::<u64>::new(node_ids);

        net.broadcast(0, 99u64);
        // Should send to nodes 1 and 2 (not 0)
        assert_eq!(net.message_count(), 2);

        let (_, v1) = receivers.get_mut(&1).unwrap().recv().await.unwrap();
        let (_, v2) = receivers.get_mut(&2).unwrap().recv().await.unwrap();
        assert_eq!(v1, 99);
        assert_eq!(v2, 99);
    }

    #[test]
    fn test_network_clone_shares_count() {
        let node_ids: Vec<NodeId> = vec![0, 1];
        let (net, _receivers) = Network::<u64>::new(node_ids);
        let net2 = net.clone();
        net.send(0, 1, 1u64);
        assert_eq!(net2.message_count(), 1);
    }
}
