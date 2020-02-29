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
/// 在每次 Fork 新进程后，应该在子进程内调用该方法以重建全局线程池，否则部分 SDK 功能在子进程内可能无法正常使用。
/// 使用该方法也可以用于调整全局线程池线程数量。
///
/// # Arguments
///
/// * `num_threads` - 调整全局线程池数量。如果传入 0，则表示不改变线程池数量。
pub fn recreate_thread_pool(mut num_threads: usize) {
    let mut thread_pool = THREAD_POOL.write().unwrap();
    if num_threads == 0 {
        num_threads = thread_pool.current_num_threads();
    }
    *thread_pool = create_thread_pool(num_threads);
}

fn create_thread_pool(num_threads: usize) -> ThreadPool {
    ThreadPoolBuilder::new()
        .thread_name(|index| format!("qiniu_ng_global_thread_{}", index))
        .num_threads(num_threads)
        .build()
        .unwrap()
}
