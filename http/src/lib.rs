#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![deny(
    single_use_lifetimes,
    missing_debug_implementations,
    large_assignments,
    exported_private_dependencies,
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    explicit_outlives_requirements,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_docs,
    non_ascii_idents,
    indirect_structural_match,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unreachable_pub,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications
)]

//! # qiniu-http
//!
//! ## 七牛 HTTP 客户端接口库
//!
//! 为更高层的 HTTP 客户端提供基础 HTTP 请求接口 [`HttpCaller`]（同时提供阻塞接口和异步接口，异步接口则需要启用 `async` 功能），
//! 使不同的 HTTP 客户端基于相同的接口实现，
//! 以便于七牛 API 调用层可以灵活切换 HTTP 客户端实现。
//! 该接口库只关注 HTTP 调用相关逻辑，不包含七牛 API 调用相关逻辑。

mod callback;
mod error;
mod request;
mod response;

use auto_impl::auto_impl;
pub use callback::{OnHeaderCallback, OnProgressCallback, OnStatusCodeCallback, TransferProgressInfo};
pub use error::{
    Error as ResponseError, ErrorBuilder as ResponseErrorBuilder, ErrorKind as ResponseErrorKind, MapError,
};
pub use http::{
    header::{self, HeaderMap, HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue},
    method::{InvalidMethod, Method},
    status::{InvalidStatusCode, StatusCode},
    uri::{self, Uri},
    Extensions, Version,
};
use once_cell::sync::OnceCell;
pub use request::{
    Request, RequestBody as SyncRequestBody, RequestBuilder, RequestParts, RequestPartsBuilder, UserAgent,
};
pub use response::{
    Metrics, MetricsBuilder, Response, ResponseBody as SyncResponseBody, ResponseBuilder, ResponseParts,
    Result as ResponseResult,
};
use std::{
    fmt::Debug,
    io::{Result as IoResult, Seek, SeekFrom},
};

/// 阻塞 HTTP 响应
pub type SyncRequest<'r> = Request<'r, SyncRequestBody<'r>>;
/// 阻塞 HTTP 响应构建器
pub type SyncRequestBuilder<'r> = RequestBuilder<'r, SyncRequestBody<'r>>;

/// 阻塞 HTTP 响应
pub type SyncResponse = Response<SyncResponseBody>;
/// 阻塞 HTTP 响应构建器
pub type SyncResponseBuilder = ResponseBuilder<SyncResponseBody>;
/// 阻塞 HTTP 响应结果
pub type SyncResponseResult = ResponseResult<SyncResponseBody>;

#[cfg(feature = "async")]
mod async_req_resp {
    pub use super::{request::AsyncRequestBody, response::AsyncResponseBody};
    use super::{
        request::{Request, RequestBuilder},
        response::{Response, ResponseBuilder, Result as ResponseResult},
    };

    /// 异步 HTTP 响应
    pub type AsyncRequest<'r> = Request<'r, AsyncRequestBody<'r>>;
    /// 异步 HTTP 响应构建器
    pub type AsyncRequestBuilder<'r> = RequestBuilder<'r, AsyncRequestBody<'r>>;

    /// 异步 HTTP 响应
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub type AsyncResponse = Response<AsyncResponseBody>;

    /// 异步 HTTP 响应构建器
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub type AsyncResponseBuilder = ResponseBuilder<AsyncResponseBody>;

    /// 异步 HTTP 响应结果
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    pub type AsyncResponseResult = ResponseResult<AsyncResponseBody>;
}

#[cfg(feature = "async")]
pub use {
    async_req_resp::{
        AsyncRequest, AsyncRequestBody, AsyncRequestBuilder, AsyncResponse, AsyncResponseBody, AsyncResponseBuilder,
        AsyncResponseResult,
    },
    futures_lite::{AsyncRead, AsyncSeek},
};

#[cfg(feature = "async")]
use std::{future::Future, pin::Pin};

#[cfg(feature = "async")]
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a + Send>>;

static LIBRARY_USER_AGENT: OnceCell<UserAgent> = OnceCell::new();

/// 全局设置库 UserAgent
///
/// 通常提供给封装七牛 SDK 的库使用，可以将库名称及版本号写入，方便之后进行调试
///
/// 该方法只能调用一次，一旦调用，全局生效
///
/// 每个请求的 UserAgent 由七牛 SDK 固定 UserAgent + 库 UserAgent + 请求的追加 UserAgent 三部分组成
pub fn set_library_user_agent(user_agent: UserAgent) -> Result<(), UserAgent> {
    LIBRARY_USER_AGENT.set(user_agent)
}

/// HTTP 请求处理接口
///
/// 实现该接口，即可处理所有七牛 SDK 发送的 HTTP 请求
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait HttpCaller: Debug + Send + Sync {
    /// 阻塞发送 HTTP 请求
    ///
    /// 该方法的异步版本为 [`Self::async_call`]。
    fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult;

    /// 异步发送 HTTP 请求
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_call<'a>(&'a self, request: &'a mut AsyncRequest<'_>) -> BoxFuture<'a, AsyncResponseResult>;

    /// 是否实现了 IP 地址解析功能
    #[inline]
    fn is_resolved_ip_addrs_supported(&self) -> bool {
        false
    }

    /// 是否返回响应指标信息功能
    #[inline]
    fn is_response_metrics_supported(&self) -> bool {
        false
    }
}

/// 重置输入流接口
///
/// 该接口相当于实现 `seek(SeekFrom::Start(0))`
pub trait Reset {
    /// 重置输入流
    ///
    /// 相当于 `seek(SeekFrom::Start(0))`
    fn reset(&mut self) -> IoResult<()>;
}

impl<T: Seek> Reset for T {
    #[inline]
    fn reset(&mut self) -> IoResult<()> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }
}

/// 异步重置输入流接口
///
/// 该接口相当于实现 `seek(SeekFrom::Start(0))`
#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub trait AsyncReset {
    /// 异步重置输入流
    ///
    /// 相当于 `seek(SeekFrom::Start(0))`
    fn reset(&mut self) -> BoxFuture<IoResult<()>>;
}

#[cfg(feature = "async")]
impl<T: AsyncSeek + Unpin + Send + Sync> AsyncReset for T {
    #[inline]
    fn reset(&mut self) -> BoxFuture<IoResult<()>> {
        use futures_lite::io::AsyncSeekExt;

        Box::pin(async move {
            self.seek(SeekFrom::Start(0)).await?;
            Ok(())
        })
    }
}

/// 将所有 Trait 全部重新导出，方便统一导入
pub mod prelude {
    pub use super::{HttpCaller, Metrics, Reset};

    #[cfg(feature = "async")]
    pub use super::AsyncReset;
}
