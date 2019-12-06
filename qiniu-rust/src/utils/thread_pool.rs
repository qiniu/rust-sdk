use lazy_static::lazy_static;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::sync::RwLock;

lazy_static! {
    pub(crate) static ref THREAD_POOL: RwLock<ThreadPool> = RwLock::new(create_thread_pool(1));
}

pub fn recreate_thread_pool() {
    let mut thread_pool = THREAD_POOL.write().unwrap();
    *thread_pool = create_thread_pool(thread_pool.current_num_threads());
}

fn create_thread_pool(num_threads: usize) -> ThreadPool {
    ThreadPoolBuilder::new()
        .thread_name(|index| format!("qiniu_ng_global_thread_{}", index))
        .num_threads(num_threads)
        .build()
        .unwrap()
}
