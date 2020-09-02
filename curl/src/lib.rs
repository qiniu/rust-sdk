use qiniu_http::{HTTPCaller, Request, ResponseResult};
use std::{
    any::Any,
    path::{Path, PathBuf},
};

mod sync;
use sync::sync_http_call;

/// 基于 Curl 的 HTTP 客户端实现
#[derive(Debug)]
pub struct CurlHTTPCaller {
    buffer_size: usize,
    temp_dir: Option<PathBuf>,
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

    /// 创建基于 Curl 的 HTTP 客户端构建器
    #[inline]
    pub fn builder() -> CurlHTTPCallerBuilder {
        CurlHTTPCallerBuilder::default()
    }
}

impl Default for CurlHTTPCaller {
    fn default() -> Self {
        Self {
            buffer_size: 1 << 22,
            temp_dir: None,
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
    pub fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.inner.buffer_size = buffer_size;
        self
    }

    /// 设置临时文件目录路径，用于缓存尺寸大于 `buffer_size` 的 HTTP 响应体，默认为系统临时目录
    pub fn temp_dir(mut self, temp_dir: Option<PathBuf>) -> Self {
        self.inner.temp_dir = temp_dir;
        self
    }

    /// 构建基于 Curl 的 HTTP 客户端
    pub fn build(self) -> CurlHTTPCaller {
        self.inner
    }
}

impl HTTPCaller for CurlHTTPCaller {
    #[inline]
    fn call(&self, request: &Request) -> ResponseResult {
        sync_http_call(self, request)
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
