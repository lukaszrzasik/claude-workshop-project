use claude_workshop_project::hotstuff::run_hotstuff;

#[tokio::test]
async fn test_hotstuff_n4() {
    let metrics = run_hotstuff(4, 42).await;
    assert!(metrics.message_count > 0);
    assert_eq!(metrics.rounds, 1);
}

#[tokio::test]
async fn test_hotstuff_n7() {
    let metrics = run_hotstuff(7, 99).await;
    assert!(metrics.message_count > 0);
    assert_eq!(metrics.rounds, 1);
}
