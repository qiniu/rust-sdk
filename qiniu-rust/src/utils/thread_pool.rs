//! 七牛 Rust SDK 内置全局线程池
//!
//! 为 Rust SDK 提供线程池，以实现类似于异步持久化，异步上传日志之类的功能
//!
//! 目前，该线程池中仅有最多一个线程

use lazy_static::lazy_static;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::sync::RwLock;

lazy_static! {
    pub(crate) static ref THREAD_POOL: RwLock<ThreadPool> = RwLock::new(create_thread_pool(1));
}

/// 重建线程池
///
/// 仅在某些情况下（例如，在线程池已经被初始化后 fork 进程，则在子进程内，线程池存储的线程具柄无法使用）才需要调用该方法
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
