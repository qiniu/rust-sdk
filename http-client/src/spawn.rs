use anyhow::Result;
use once_cell::sync::Lazy;
use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};

type Task = Box<dyn FnOnce() + Send + 'static>;
type TasksDequeue = VecDeque<Task>;
type TasksMap = HashMap<String, TasksDequeue>;

static THREADS_MAP_LOCK: Lazy<Mutex<TasksMap>> = Lazy::new(Default::default);

pub(super) fn spawn<F: FnOnce() + Send + 'static>(task_name: String, f: F) -> Result<()> {
    let mut threads_map = THREADS_MAP_LOCK.lock().unwrap();
    if let Some(dequeue) = threads_map.get_mut(&task_name) {
        dequeue.push_back(Box::new(f));
        return Ok(());
    } else {
        let mut dequeue = TasksDequeue::with_capacity(1);
        dequeue.push_back(Box::new(f));
        threads_map.insert(task_name.to_owned(), dequeue);
        return spawn_inner(task_name);
    }

    fn spawn_inner(task_name: String) -> Result<()> {
        _spawn(task_name.to_owned(), move || {
            while let Some(task) = get_task(&task_name) {
                task();
            }
        })
    }

    fn get_task(task_name: &str) -> Option<Task> {
        let mut threads_map = THREADS_MAP_LOCK.lock().unwrap();
        if let Some(dequeue) = threads_map.get_mut(task_name) {
            if let Some(task) = dequeue.pop_front() {
                return Some(task);
            }
            threads_map.remove(task_name);
        }
        None
    }

    #[cfg(not(feature = "async"))]
    fn _spawn<F: FnOnce() + Send + 'static>(task_name: String, f: F) -> Result<()> {
        std::thread::Builder::new()
            .name(task_name)
            .spawn(f)
            .map(|_| ())
            .map_err(|err| err.into())
    }

    #[cfg(feature = "async")]
    fn _spawn<F: FnOnce() + Send + 'static>(_task_name: String, f: F) -> Result<()> {
        async_std::task::spawn_blocking(f);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        thread::sleep,
        time::Duration,
    };

    #[test]
    fn test_spawn() -> Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let spawned_task_1 = Arc::new(AtomicUsize::new(0));
        let spawned_task_2 = Arc::new(AtomicUsize::new(0));

        for i in 0..1000usize {
            let spawned_task_1 = spawned_task_1.to_owned();
            spawn("task1".to_owned(), move || {
                if i == 0 {
                    sleep(Duration::from_secs(1));
                }
                spawned_task_1.fetch_add(1, Ordering::Relaxed);
            })?;
            let spawned_task_2 = spawned_task_2.to_owned();
            spawn("task2".to_owned(), move || {
                if i == 0 {
                    sleep(Duration::from_secs(1));
                }
                spawned_task_2.fetch_add(1, Ordering::Relaxed);
            })?;
        }

        assert_eq!(spawned_task_1.load(Ordering::Relaxed), 0);
        assert_eq!(spawned_task_2.load(Ordering::Relaxed), 0);

        sleep(Duration::from_secs(2));

        assert_eq!(spawned_task_1.load(Ordering::Relaxed), 1000);
        assert_eq!(spawned_task_2.load(Ordering::Relaxed), 1000);

        Ok(())
    }
}
