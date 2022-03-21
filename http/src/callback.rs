use std::ops::Not;

use http::{header::HeaderName, HeaderValue, StatusCode};
use smart_default::SmartDefault;

pub(super) type OnProgress<'r> = &'r (dyn Fn(&TransferProgressInfo) -> CallbackResult + Send + Sync);
pub(super) type OnStatusCode<'r> = &'r (dyn Fn(StatusCode) -> CallbackResult + Send + Sync);
pub(super) type OnHeader<'r> = &'r (dyn Fn(&HeaderName, &HeaderValue) -> CallbackResult + Send + Sync);

/// 数据传输进度信息
#[derive(Debug)]
pub struct TransferProgressInfo<'b> {
    transferred_bytes: u64,
    total_bytes: u64,
    body: &'b [u8],
}

impl<'b> TransferProgressInfo<'b> {
    /// 创建数据传输进度信息
    #[inline]
    pub fn new(transferred_bytes: u64, total_bytes: u64, body: &'b [u8]) -> Self {
        Self {
            transferred_bytes,
            total_bytes,
            body,
        }
    }

    /// 获取已经传输的数据量
    ///
    /// 单位为字节
    #[inline]
    pub fn transferred_bytes(&self) -> u64 {
        self.transferred_bytes
    }

    /// 获取总共需要传输的数据量
    ///
    /// 单位为字节
    #[inline]
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// 获取当前传输的数据
    #[inline]
    pub fn body(&self) -> &[u8] {
        self.body
    }
}

/// 回调函数调用结果
#[must_use]
#[derive(SmartDefault, Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum CallbackResult {
    /// 回调函数调用成功，继续执行
    #[default]
    Continue,

    /// 回调函数调用失败，当前请求将被取消
    Cancel,
}

impl CallbackResult {
    /// 是否继续执行
    #[inline]
    pub fn is_continue(self) -> bool {
        self == Self::Continue
    }

    /// 是否取消执行
    #[inline]
    pub fn is_cancelled(self) -> bool {
        self == Self::Cancel
    }
}

impl FromIterator<CallbackResult> for CallbackResult {
    #[inline]
    fn from_iter<T: IntoIterator<Item = CallbackResult>>(iter: T) -> Self {
        if iter.into_iter().any(|result| result.is_cancelled()) {
            Self::Cancel
        } else {
            Self::Continue
        }
    }
}

impl Not for CallbackResult {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        if self.is_cancelled() {
            Self::Continue
        } else {
            Self::Cancel
        }
    }
}
