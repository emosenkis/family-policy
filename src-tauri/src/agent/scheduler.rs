use chrono::{DateTime, Utc};
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;

/// Polling scheduler with jitter to prevent thundering herd
pub struct PollingScheduler {
    base_interval: Duration,
    jitter_range: Duration,
}

impl PollingScheduler {
    /// Create a new polling scheduler
    ///
    /// # Arguments
    /// * `interval_secs` - Base polling interval in seconds
    /// * `jitter_secs` - Maximum jitter to add/subtract in seconds
    pub fn new(interval_secs: u64, jitter_secs: u64) -> Self {
        Self {
            base_interval: Duration::from_secs(interval_secs),
            jitter_range: Duration::from_secs(jitter_secs),
        }
    }

    /// Sleep until next poll time with jitter
    pub async fn sleep_until_next_poll(&self) {
        let sleep_duration = self.calculate_next_interval();
        tracing::debug!(
            "Sleeping for {} seconds until next poll",
            sleep_duration.as_secs()
        );
        sleep(sleep_duration).await;
    }

    /// Calculate the next poll time (current time + interval + jitter)
    pub fn next_poll_time(&self) -> DateTime<Utc> {
        let sleep_duration = self.calculate_next_interval();
        Utc::now() + chrono::Duration::from_std(sleep_duration).unwrap()
    }

    /// Calculate the next sleep interval with jitter
    fn calculate_next_interval(&self) -> Duration {
        let jitter = self.random_jitter();
        self.base_interval + jitter
    }

    /// Generate random jitter in range [0, jitter_range]
    fn random_jitter(&self) -> Duration {
        let jitter_secs = rand::thread_rng().gen_range(0..=self.jitter_range.as_secs());
        Duration::from_secs(jitter_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polling_scheduler_creates_with_correct_intervals() {
        let scheduler = PollingScheduler::new(300, 60);
        assert_eq!(scheduler.base_interval, Duration::from_secs(300));
        assert_eq!(scheduler.jitter_range, Duration::from_secs(60));
    }

    #[test]
    fn polling_scheduler_jitter_is_within_range() {
        let scheduler = PollingScheduler::new(300, 60);

        // Test multiple times to ensure randomness
        for _ in 0..100 {
            let interval = scheduler.calculate_next_interval();
            assert!(interval >= Duration::from_secs(300));
            assert!(interval <= Duration::from_secs(360));
        }
    }

    #[test]
    fn polling_scheduler_next_poll_time_is_in_future() {
        let scheduler = PollingScheduler::new(300, 60);
        let now = Utc::now();
        let next = scheduler.next_poll_time();

        assert!(next > now);
        assert!(next <= now + chrono::Duration::seconds(360));
    }

    #[test]
    fn polling_scheduler_with_zero_jitter() {
        let scheduler = PollingScheduler::new(300, 0);

        for _ in 0..10 {
            let interval = scheduler.calculate_next_interval();
            assert_eq!(interval, Duration::from_secs(300));
        }
    }

    #[test]
    fn random_jitter_produces_different_values() {
        let scheduler = PollingScheduler::new(300, 60);

        let mut values = std::collections::HashSet::new();
        for _ in 0..50 {
            let jitter = scheduler.random_jitter();
            values.insert(jitter.as_secs());
        }

        // With 50 samples from 0-60, we should get at least a few different values
        assert!(values.len() > 5);
    }
}
