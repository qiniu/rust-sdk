use std::time::Duration;

pub struct TimeoutExtension(Duration);

impl TimeoutExtension {
    #[inline]
    pub fn new(timeout: Duration) -> Self {
        Self(timeout)
    }

    #[inline]
    pub fn get(&self) -> Duration {
        self.0
    }
}
