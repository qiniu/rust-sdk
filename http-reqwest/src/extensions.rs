use std::time::Duration;

/// 请求超时时长扩展
#[derive(Debug)]
pub struct TimeoutExtension(Duration);

impl TimeoutExtension {
    /// 新建请求超时时长扩展
    #[inline]
    pub fn new(timeout: Duration) -> Self {
        Self(timeout)
    }

    /// 获取请求超时时长扩展的值
    #[inline]
    pub fn get(&self) -> Duration {
        self.0
    }
}
