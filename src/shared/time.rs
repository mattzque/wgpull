use std::time::SystemTime;
use chrono::{DateTime, Local};


/// Trait for getting the current time.
/// This is used to allow mocking the time in tests.
pub trait CurrentTime {
    /// Returns the current time as a `SystemTime`.
    fn now(&self) -> SystemTime;

    /// Returns the current time as a `DateTime<Local>`.
    fn now_chrono(&self) -> DateTime<Local> {
        DateTime::from(self.now())
    }
}

#[derive(Default)]
pub struct CurrentSystemTime;

impl CurrentTime for CurrentSystemTime {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}