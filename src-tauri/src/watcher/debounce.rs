use tokio::time::{Duration, Instant};

pub struct Debouncer {
    interval: Duration,
    last_fire: Option<Instant>,
}

impl Debouncer {
    pub fn new(ms: u64) -> Self {
        Self {
            interval: Duration::from_millis(ms),
            last_fire: None,
        }
    }

    pub async fn should_fire(&mut self) -> bool {
        let now = Instant::now();
        if let Some(last) = self.last_fire {
            if now.duration_since(last) < self.interval {
                // Wait for the remaining debounce time
                tokio::time::sleep(self.interval - now.duration_since(last)).await;
            }
        }
        self.last_fire = Some(Instant::now());
        true
    }
}
