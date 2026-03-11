pub type NodeId = u64;
pub type Value = u64;
pub type ViewNumber = u64;
pub type SeqNumber = u64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_aliases() {
        let node_id: NodeId = 1;
        let value: Value = 42;
        let view: ViewNumber = 0;
        let seq: SeqNumber = 1;
        assert_eq!(node_id, 1);
        assert_eq!(value, 42);
        assert_eq!(view, 0);
        assert_eq!(seq, 1);
    }
}
