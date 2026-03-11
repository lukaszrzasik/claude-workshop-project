use claude_workshop_project::tendermint::run_tendermint;

#[tokio::test]
async fn test_tendermint_n4() {
    let metrics = run_tendermint(4, 42).await;
    assert!(metrics.message_count > 0);
    assert_eq!(metrics.rounds, 1);
}

#[tokio::test]
async fn test_tendermint_n7() {
    let metrics = run_tendermint(7, 99).await;
    assert!(metrics.message_count > 0);
    assert_eq!(metrics.rounds, 1);
}
