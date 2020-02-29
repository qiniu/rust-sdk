use libc::size_t;
use qiniu_ng::utils::thread_pool;

/// @brief 重新创建 qiniu_ng 使用的全局线程池
/// @details
///     在每次 Fork 新进程后，应该在子进程内调用该方法以重建全局线程池，否则部分 SDK 功能在子进程内可能无法正常使用。
///     使用该方法也可以用于调整全局线程池线程数量。
/// @param[in] num_threads 调整全局线程池数量。如果传入 0，则表示不改变线程池数量
#[no_mangle]
pub extern "C" fn qiniu_ng_recreate_global_thread_pool(num_threads: size_t) {
    thread_pool::recreate_thread_pool(num_threads)
}
