#![cfg_attr(feature = "docs", feature(doc_cfg))]

pub use qiniu_http::{self as http, HTTPCaller, Request, SyncResponseResult};
use std::{
    any::Any,
    path::{Path, PathBuf},
};

mod sync;
use sync::sync_http_call;

#[cfg(feature = "async")]
mod r#async;
#[cfg(feature = "async")]
use r#async::async_http_call;

mod utils;

#[cfg(feature = "async")]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct MultiOptions {
    http_1_pipelining_length: usize,
    http_2_multiplexing: bool,
    max_connections: usize,
    max_connections_per_host: usize,
    connection_cache_size: usize,
}

/// 基于 Curl 的 HTTP 客户端实现
#[derive(Debug)]
pub struct CurlHTTPCaller {
    buffer_size: usize,
    temp_dir: Option<PathBuf>,

    #[cfg(feature = "async")]
    multi_options: MultiOptions,
}

impl CurlHTTPCaller {
    /// 获取内存缓存区大小
    #[inline]
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// 获取临时文件目录路径
    #[inline]
    pub fn temp_dir(&self) -> Option<&Path> {
        self.temp_dir.as_deref()
    }

    /// 获取 HTTP/1.1 最大管线化连接数
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    #[inline]
    pub fn http_1_pipelining_length(&self) -> usize {
        self.multi_options.http_1_pipelining_length
    }

    /// 获取 HTTP/1.1 最大管线化连接数
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    #[inline]
    pub fn http_2_multiplexing(&self) -> bool {
        self.multi_options.http_2_multiplexing
    }

    /// 获取连接数最大值
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    #[inline]
    pub fn max_connections(&self) -> usize {
        self.multi_options.max_connections
    }

    /// 获取连接单个主机的连接数最大值
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    #[inline]
    pub fn max_connections_per_host(&self) -> usize {
        self.multi_options.max_connections_per_host
    }

    /// 获取连接池最大值
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    #[inline]
    pub fn connection_cache_size(&self) -> usize {
        self.multi_options.connection_cache_size
    }

    /// 创建基于 Curl 的 HTTP 客户端构建器
    #[inline]
    pub fn builder() -> CurlHTTPCallerBuilder {
        CurlHTTPCallerBuilder::default()
    }

    #[cfg(feature = "async")]
    #[inline]
    fn clone_multi_options(&self) -> MultiOptions {
        self.multi_options.to_owned()
    }
}

impl Default for CurlHTTPCaller {
    #[inline]
    fn default() -> Self {
        Self {
            buffer_size: 1 << 22,
            temp_dir: None,

            #[cfg(feature = "async")]
            multi_options: MultiOptions {
                http_1_pipelining_length: 5,
                http_2_multiplexing: true,
                max_connections: 0,
                max_connections_per_host: 0,
                connection_cache_size: 0,
            },
        }
    }
}

/// 基于 Curl 的 HTTP 客户端构建器
#[derive(Default)]
pub struct CurlHTTPCallerBuilder {
    inner: CurlHTTPCaller,
}

impl CurlHTTPCallerBuilder {
    /// 设置内存缓存区大小，默认为 4 MB
    #[inline]
    pub fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.inner.buffer_size = buffer_size;
        self
    }

    /// 设置临时文件目录路径，用于缓存尺寸大于 `buffer_size` 的 HTTP 响应体，默认为系统临时目录
    #[inline]
    pub fn temp_dir(mut self, temp_dir: Option<PathBuf>) -> Self {
        self.inner.temp_dir = temp_dir;
        self
    }

    /// 设置 HTTP/1.1 最大管线化连接数，默认为 5
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    #[inline]
    pub fn http_1_pipelining_length(mut self, length: usize) -> Self {
        self.inner.multi_options.http_1_pipelining_length = length;
        self
    }

    /// 设置 HTTP/1.1 最大管线化连接数，默认为 true
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    #[inline]
    pub fn http_2_multiplexing(mut self, multiplexing: bool) -> Self {
        self.inner.multi_options.http_2_multiplexing = multiplexing;
        self
    }

    /// 设置连接数最大值，默认为 0，表示无限制
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    #[inline]
    pub fn max_connections(mut self, connections: usize) -> Self {
        self.inner.multi_options.max_connections = connections;
        self
    }

    /// 设置连接单个主机的连接数最大值，默认为 0，表示无限制
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    #[inline]
    pub fn max_connections_per_host(mut self, connections: usize) -> Self {
        self.inner.multi_options.max_connections_per_host = connections;
        self
    }

    /// 设置连接池最大值，默认为 0，表示使用 libcurl 默认的连接池策略
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    #[inline]
    pub fn connection_cache_size(mut self, size: usize) -> Self {
        self.inner.multi_options.connection_cache_size = size;
        self
    }

    /// 构建基于 Curl 的 HTTP 客户端
    #[inline]
    pub fn build(self) -> CurlHTTPCaller {
        self.inner
    }
}

#[cfg(feature = "async")]
pub use {futures::future::BoxFuture, http::AsyncResponseResult};

impl HTTPCaller for CurlHTTPCaller {
    #[inline]
    fn call(&self, request: &Request) -> SyncResponseResult {
        sync_http_call(self, request)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_call<'a>(&'a self, request: &'a Request) -> BoxFuture<'a, AsyncResponseResult> {
        Box::pin(async move { async_http_call(self, request).await })
    }

    #[inline]
    fn as_http_caller(&self) -> &dyn HTTPCaller {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }
}
