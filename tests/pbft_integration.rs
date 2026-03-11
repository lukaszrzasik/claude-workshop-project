use claude_workshop_project::pbft::run_pbft;

#[tokio::test]
async fn test_pbft_4_nodes_consensus() {
    // f=1, quorum=3, n=4
    let metrics = run_pbft(4, 42).await;
    assert!(metrics.message_count > 0, "should have sent messages");
    assert_eq!(metrics.rounds, 1, "should complete in 1 round");
}

#[tokio::test]
async fn test_pbft_7_nodes_consensus() {
    // f=2, quorum=5, n=7
    let metrics = run_pbft(7, 99).await;
    assert!(metrics.message_count > 0, "should have sent messages");
    assert_eq!(metrics.rounds, 1, "should complete in 1 round");
}
