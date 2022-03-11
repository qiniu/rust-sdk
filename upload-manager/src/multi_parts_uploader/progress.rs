use crossbeam_utils::Backoff;
use std::{
    collections::HashMap,
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    sync::{Arc, RwLock},
};

#[derive(Clone)]
pub(super) struct ProgressesKey {
    progresses: Progresses,
    index: u64,
    total_size: u64,
}

impl ProgressesKey {
    pub(super) fn update_part(&self, new_size: u64) -> bool {
        self.progresses.update_part(self, new_size)
    }

    pub(super) fn complete_part(&self) -> bool {
        self.progresses.complete_part(self)
    }

    pub(super) fn delete_part(&self) -> bool {
        self.progresses.delete_part(self)
    }

    pub(super) fn current_uploaded(&self) -> u64 {
        self.progresses.current_uploaded()
    }
}

impl PartialEq for ProgressesKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Eq for ProgressesKey {}

impl Hash for ProgressesKey {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl Debug for ProgressesKey {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.index.fmt(f)
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct Progresses(Arc<RwLock<ProgressesInner>>);

#[derive(Debug, Default)]
struct ProgressesInner {
    uploaded: u64,
    next_key: u64,
    map: HashMap<u64, u64>,
}

impl Progresses {
    pub(super) fn add_new_part(&self, part_size: u64) -> ProgressesKey {
        self.try_write(move |inner| {
            let index = inner.add_new_part();
            ProgressesKey {
                progresses: self.to_owned(),
                index,
                total_size: part_size,
            }
        })
    }

    pub(super) fn update_part(&self, key: &ProgressesKey, new_size: u64) -> bool {
        self.try_write(move |inner| inner.update_part(key.index, new_size))
    }

    pub(super) fn complete_part(&self, key: &ProgressesKey) -> bool {
        self.try_write(move |inner| inner.complete_part(key.index, key.total_size))
    }

    pub(super) fn delete_part(&self, key: &ProgressesKey) -> bool {
        self.try_write(move |inner| inner.delete_part(key.index))
    }

    pub(super) fn current_uploaded(&self) -> u64 {
        self.try_read(move |inner| inner.current_uploaded())
    }

    fn try_write<F: FnOnce(&mut ProgressesInner) -> T, T>(&self, f: F) -> T {
        let backoff = Backoff::new();
        loop {
            if let Ok(mut inner) = self.0.try_write() {
                return f(&mut inner);
            } else {
                backoff.spin();
            }
        }
    }

    fn try_read<F: FnOnce(&ProgressesInner) -> T, T>(&self, f: F) -> T {
        let backoff = Backoff::new();
        loop {
            if let Ok(mut inner) = self.0.try_read() {
                return f(&mut inner);
            } else {
                backoff.spin();
            }
        }
    }
}

impl ProgressesInner {
    fn add_new_part(&mut self) -> u64 {
        let index = self.next_key;
        self.next_key += 1;
        self.map.insert(index, 0);
        index
    }

    fn update_part(&mut self, index: u64, new_size: u64) -> bool {
        if let Some(value) = self.map.get_mut(&index) {
            *value = new_size;
            true
        } else {
            false
        }
    }

    fn complete_part(&mut self, index: u64, total_size: u64) -> bool {
        if self.delete_part(index) {
            self.uploaded += total_size;
            true
        } else {
            false
        }
    }

    fn delete_part(&mut self, index: u64) -> bool {
        self.map.remove(&index).is_some()
    }

    fn current_uploaded(&self) -> u64 {
        self.uploaded + self.map.values().sum::<u64>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::spawn as thread_spawn;

    const PART_SIZE: u64 = 1 << 20;

    #[test]
    fn test_progresses() {
        let progresses = Progresses::default();

        let threads: Vec<_> = (0..3)
            .map(|_| {
                let key = progresses.add_new_part(PART_SIZE);
                thread_spawn(move || {
                    for uploaded in 0..=PART_SIZE {
                        assert!(key.update_part(uploaded));
                    }
                    assert!(key.complete_part());
                })
            })
            .collect();

        let mut last_uploaded = 0u64;
        loop {
            let current_uploaded = progresses.current_uploaded();
            assert!(current_uploaded >= last_uploaded);
            assert!(current_uploaded <= 3 * PART_SIZE);
            if current_uploaded >= 3 * PART_SIZE {
                break;
            }
            last_uploaded = current_uploaded;
        }

        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(progresses.current_uploaded(), 3 * PART_SIZE);
    }
}
