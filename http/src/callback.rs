use anyhow::Result as AnyResult;
use http::{header::HeaderName, HeaderValue, StatusCode};

pub(super) type OnProgress<'r> = &'r (dyn Fn(&TransferProgressInfo) -> AnyResult<()> + Send + Sync);
pub(super) type OnStatusCode<'r> = &'r (dyn Fn(StatusCode) -> AnyResult<()> + Send + Sync);
pub(super) type OnHeader<'r> = &'r (dyn Fn(&HeaderName, &HeaderValue) -> AnyResult<()> + Send + Sync);

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
