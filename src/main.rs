use claude_workshop_project::simulation::run_all;

#[tokio::main]
async fn main() {
    let node_counts = vec![4, 7, 10];
    let value = 42u64;
    let results = run_all(&node_counts, value).await;

    println!(
        "{:<12} | {:>4} | {:>10} | {:>14}",
        "Protocol", "N", "Messages", "Duration (µs)"
    );
    println!("{}", "-".repeat(48));
    for r in &results {
        println!(
            "{:<12} | {:>4} | {:>10} | {:>14}",
            r.protocol, r.n, r.message_count, r.duration_us
        );
    }
}
