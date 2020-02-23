use libc::size_t;
use qiniu_ng::utils::thread_pool;

/// @brief 重新创建 qiniu_ng 使用的全局线程池
/// @details
///     仅在某些情况下（例如，在线程池已经被初始化后 fork 进程，则在子进程内，线程池存储的线程具柄无法使用）才需要调用该方法。
///     使用该方法也可以用于调整全局线程池线程数量。
/// @param[in] num_threads 调整全局线程池数量。如果传入 0，则表示不改变线程池数量
/// @retval *char 版本号字符串
#[no_mangle]
pub extern "C" fn qiniu_ng_recreate_global_thread_pool(num_threads: size_t) {
    thread_pool::recreate_thread_pool(num_threads)
}
