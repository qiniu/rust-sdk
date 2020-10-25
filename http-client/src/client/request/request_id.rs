use dashmap::DashSet;
use once_cell::sync::Lazy;
use std::{
    fmt,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

static IDS: Lazy<DashSet<usize>> = Lazy::new(Default::default);
static CURRENT_ID: AtomicUsize = AtomicUsize::new(0);

pub(super) struct RequestId(usize);

impl RequestId {
    pub(super) fn new() -> Self {
        loop {
            let id = CURRENT_ID.fetch_add(1, Relaxed);
            if IDS.insert(id) {
                return RequestId(id);
            }
        }
    }

    #[inline]
    pub(super) fn get(&self) -> usize {
        self.0
    }
}

impl Drop for RequestId {
    #[inline]
    fn drop(&mut self) {
        IDS.remove(&self.0);
    }
}

impl fmt::Display for RequestId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for RequestId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
