use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type SafeDebouncer = Arc<Mutex<Debouncer>>;

pub fn get_safe_debouncer(interval: Duration) -> SafeDebouncer {
    return Arc::new(Mutex::new(Debouncer::new(interval)));
}

pub struct Debouncer {
    interval: Duration,
    last_run: Option<Instant>,
}

impl Debouncer {
    pub fn new(interval: Duration) -> Self {
        return Debouncer {
            interval: interval,
            last_run: None,
        };
    }

    pub fn debounce(&mut self, f: impl Fn()) {
        if self.last_run.is_none() {
            self.last_run = Some(Instant::now());
            f();
            return;
        }
        let then = self.last_run.unwrap();
        let now = Instant::now();

        if now.duration_since(then) > self.interval {
            self.last_run = Some(Instant::now());
            f();
            return;
        }
        return;
    }
}
