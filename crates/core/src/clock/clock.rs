use std::time::{SystemTime, UNIX_EPOCH};

use crate::clock::AppTime;

pub trait Clock: Send + Sync {
    fn now_millis(&self) -> AppTime;
}

pub struct AppClock;

impl AppClock {
    pub fn new() -> Self {
        Self {}
    }
}

impl Clock for AppClock {
    #[inline]
    fn now_millis(&self) -> AppTime {
        let dur = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        AppTime::from(dur.as_millis())
    }
}
