use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type SafeDebouncer = Arc<Mutex<Debouncer>>; // 非同期でdebounce
pub fn get_safe_debouncer(interval: Duration) -> SafeDebouncer {
    return Arc::new(Mutex::new(Debouncer::new(interval)));
}

// ファイルごとにdebounce
pub struct SafeFileDebouncer {
    debounce_interval: u64,
    debouncers: HashMap<String, SafeDebouncer>,
}

impl SafeFileDebouncer {
    pub fn new(debounce_interval: u64) -> SafeFileDebouncer {
        return SafeFileDebouncer {
            debounce_interval: debounce_interval,
            debouncers: HashMap::new(),
        };
    }

    pub fn get(&mut self, filepath: &str) -> &SafeDebouncer {
        if !self.debouncers.contains_key(filepath) {
            let debounce_interval = std::time::Duration::from_millis(self.debounce_interval);
            let debouncer = get_safe_debouncer(debounce_interval);
            self.debouncers.insert(String::from(filepath), debouncer);
        }

        return &self.debouncers[filepath];
    }
}

// 普通にdebounce
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

    pub fn debounce(&mut self, f: impl Fn()) -> bool {
        if self.last_run.is_none() {
            self.last_run = Some(Instant::now());
            f();
            return true;
        }
        let then = self.last_run.unwrap();
        let now = Instant::now();

        if now.duration_since(then) > self.interval {
            self.last_run = Some(Instant::now());
            f();
            return true;
        }
        return false;
    }
}
