use anyhow::Result as AnyResult;
use http::{header::HeaderName, HeaderValue, StatusCode};
use std::{
    fmt::{self, Debug},
    ops::Deref,
};

macro_rules! impl_callback {
    ($name:ident, $name_str:literal, $callback:path) => {
        impl<'r> $name<'r> {
            /// 创建回调函数
            #[inline]
            pub fn new(callback: impl $callback + Send + Sync + 'r) -> Self {
                Self::Boxed(Box::new(callback))
            }

            /// 创建回调函数的引用
            #[inline]
            pub fn reference(callback: &'r (dyn $callback + Send + Sync + 'r)) -> Self {
                Self::Referenced(callback)
            }
        }

        impl<'r, T: $callback + Send + Sync + 'r> From<T> for $name<'r> {
            #[inline]
            fn from(callback: T) -> Self {
                Self::new(callback)
            }
        }

        impl<'r> Deref for $name<'r> {
            type Target = (dyn $callback + Send + Sync + 'r);

            #[inline]
            fn deref(&self) -> &Self::Target {
                match self {
                    Self::Boxed(callback) => callback.deref(),
                    Self::Referenced(callback) => callback,
                }
            }
        }

        impl Debug for $name<'_> {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct($name_str).finish()
            }
        }
    };
}

/// 上传进度回调
pub enum OnProgressCallback<'r> {
    /// 用 Box 包装的上传进度回调
    Boxed(Box<dyn Fn(TransferProgressInfo) -> AnyResult<()> + Send + Sync + 'r>),
    /// 上传进度回调的引用
    Referenced(&'r (dyn Fn(TransferProgressInfo) -> AnyResult<()> + Send + Sync + 'r)),
}
impl_callback!(OnProgressCallback, "OnProgressCallback", Fn(TransferProgressInfo) -> AnyResult<()>);

pub(super) type OnProgress<'r> = &'r (dyn Fn(TransferProgressInfo<'_>) -> AnyResult<()> + Send + Sync + 'r);

/// 响应状态回调
pub enum OnStatusCodeCallback<'r> {
    /// 用 Box 包装的响应状态回调
    Boxed(Box<dyn Fn(StatusCode) -> AnyResult<()> + Send + Sync + 'r>),
    /// 响应状态回调的引用
    Referenced(&'r (dyn Fn(StatusCode) -> AnyResult<()> + Send + Sync + 'r)),
}
impl_callback!(OnStatusCodeCallback, "OnStatusCodeCallback", Fn(StatusCode) -> AnyResult<()>);

pub(super) type OnStatusCode<'r> = &'r (dyn Fn(StatusCode) -> AnyResult<()> + Send + Sync + 'r);

type OnHeaderCallbackFn<'r> = Box<dyn Fn(&HeaderName, &HeaderValue) -> AnyResult<()> + Send + Sync + 'r>;
type OnHeaderCallbackFnRef<'r> = &'r (dyn Fn(&HeaderName, &HeaderValue) -> AnyResult<()> + Send + Sync + 'r);

/// 接受到响应 Header 回调
pub enum OnHeaderCallback<'r> {
    /// 用 Box 包装的接受到响应 Header 回调
    Boxed(OnHeaderCallbackFn<'r>),
    /// 接受到响应 Header 回调的引用
    Referenced(OnHeaderCallbackFnRef<'r>),
}
impl_callback!(OnHeaderCallback, "OnHeaderCallback", Fn(&HeaderName, &HeaderValue) -> AnyResult<()>);

pub(super) type OnHeader<'r> = &'r (dyn Fn(&HeaderName, &HeaderValue) -> AnyResult<()> + Send + Sync + 'r);

/// 数据传输进度信息
#[derive(Debug, Clone, Copy)]
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
