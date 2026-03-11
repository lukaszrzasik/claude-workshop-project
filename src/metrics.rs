#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub message_count: u64,
    pub rounds: u64,
    pub duration_us: u64,
}

impl Metrics {
    pub fn new(message_count: u64, rounds: u64, duration_us: u64) -> Self {
        Self {
            message_count,
            rounds,
            duration_us,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_default() {
        let m = Metrics::default();
        assert_eq!(m.message_count, 0);
        assert_eq!(m.rounds, 0);
        assert_eq!(m.duration_us, 0);
    }

    #[test]
    fn test_metrics_new() {
        let m = Metrics::new(10, 1, 500);
        assert_eq!(m.message_count, 10);
        assert_eq!(m.rounds, 1);
        assert_eq!(m.duration_us, 500);
    }

    #[test]
    fn test_metrics_clone() {
        let m = Metrics::new(5, 2, 100);
        let m2 = m.clone();
        assert_eq!(m2.message_count, 5);
        assert_eq!(m2.rounds, 2);
        assert_eq!(m2.duration_us, 100);
    }
}
