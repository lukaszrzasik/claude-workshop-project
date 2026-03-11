use crate::hotstuff::run_hotstuff;
use crate::pbft::run_pbft;
use crate::tendermint::run_tendermint;
use crate::types::Value;

#[derive(Debug)]
pub struct SimResult {
    pub protocol: &'static str,
    pub n: usize,
    pub message_count: u64,
    pub rounds: u64,
    pub duration_us: u64,
}

pub async fn run_all(node_counts: &[usize], value: Value) -> Vec<SimResult> {
    let mut results = Vec::new();
    for &n in node_counts {
        let m = run_pbft(n, value).await;
        results.push(SimResult {
            protocol: "PBFT",
            n,
            message_count: m.message_count,
            rounds: m.rounds,
            duration_us: m.duration_us,
        });

        let m = run_tendermint(n, value).await;
        results.push(SimResult {
            protocol: "Tendermint",
            n,
            message_count: m.message_count,
            rounds: m.rounds,
            duration_us: m.duration_us,
        });

        let m = run_hotstuff(n, value).await;
        results.push(SimResult {
            protocol: "HotStuff",
            n,
            message_count: m.message_count,
            rounds: m.rounds,
            duration_us: m.duration_us,
        });
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_all_single_count() {
        let results = run_all(&[4], 42).await;
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].protocol, "PBFT");
        assert_eq!(results[1].protocol, "Tendermint");
        assert_eq!(results[2].protocol, "HotStuff");
        for r in &results {
            assert_eq!(r.n, 4);
            assert!(r.message_count > 0);
            assert_eq!(r.rounds, 1);
        }
    }

    #[tokio::test]
    async fn test_run_all_multiple_counts() {
        let results = run_all(&[4, 7], 99).await;
        assert_eq!(results.len(), 6);
        // First three are n=4
        for r in &results[0..3] {
            assert_eq!(r.n, 4);
        }
        // Last three are n=7
        for r in &results[3..6] {
            assert_eq!(r.n, 7);
        }
    }
}
