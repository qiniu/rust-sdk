use lazy_static::lazy_static;
use rayon::{ThreadPool, ThreadPoolBuilder};

lazy_static! {
    pub(crate) static ref THREAD_POOL: ThreadPool = ThreadPoolBuilder::new()
        .thread_name(|index| format!("qiniu_ng_global_thread_{}", index))
        .num_threads(1)
        .build()
        .unwrap();
}
