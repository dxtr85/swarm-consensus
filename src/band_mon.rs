use std::time::{Duration, SystemTime};

pub struct BandwidthMonitor {
    used_tokens_history: [u64; 16],
    used_tokens_index: usize,
    current_usage: u64,
    current_start: SystemTime,
    period_time: Duration,
}
impl BandwidthMonitor {
    pub fn new(period_time: Duration) -> Self {
        Self {
            used_tokens_history: [0; 16],
            used_tokens_index: 0,
            current_usage: 0,
            current_start: SystemTime::now(),
            period_time,
        }
    }
    pub fn average(&self) -> u64 {
        let sum: u64 = self.used_tokens_history.iter().sum();
        sum >> 4
    }
    pub fn update(&mut self, used_tokens: u64) {
        let now = SystemTime::now();
        self.current_usage += used_tokens;
        if now.duration_since(self.current_start).unwrap() >= self.period_time {
            self.used_tokens_history[self.used_tokens_index] = self.current_usage;
            self.current_usage = 0;
            self.current_start = now;
            self.used_tokens_index += 1;
            if self.used_tokens_index >= 16 {
                self.used_tokens_index = 0;
            }
        }
    }
}
