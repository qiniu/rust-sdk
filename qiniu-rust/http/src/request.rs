use super::{Headers, Method, RequestConfig};
use std::borrow::Cow;

/// 请求 URL
pub type URL<'b> = Cow<'b, str>;

/// 请求体
pub type Body<'b> = Cow<'b, [u8]>;

/// HTTP 请求
///
/// 封装 HTTP 请求相关字段
#[derive(Debug, Clone)]
pub struct Request<'b> {
    url: URL<'b>,
    method: Method,
    headers: Headers<'b>,
    body: Body<'b>,
    config: RequestConfig<'b>,
}

impl<'b> Request<'b> {
    /// 请求 URL
    #[inline]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// 修改请求 URL
    #[inline]
    pub fn url_mut(&mut self) -> &mut URL<'b> {
        &mut self.url
    }

    /// 请求 HTTP 方法
    #[inline]
    pub fn method(&self) -> Method {
        self.method
    }

    /// 修改请求 HTTP 方法
    #[inline]
    pub fn method_mut(&mut self) -> &mut Method {
        &mut self.method
    }

    /// 请求 HTTP Headers
    #[inline]
    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    /// 修改请求 HTTP Headers
    #[inline]
    pub fn headers_mut(&mut self) -> &mut Headers<'b> {
        &mut self.headers
    }

    /// 请求体
    #[inline]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// 修改请求体
    #[inline]
    pub fn body_mut(&mut self) -> &mut Body<'b> {
        &mut self.body
    }

    /// 请求配置
    #[inline]
    pub fn config(&self) -> &RequestConfig {
        &self.config
    }

    /// 修改请求配置
    #[inline]
    pub fn config_mut(&mut self) -> &mut RequestConfig<'b> {
        &mut self.config
    }
}

impl Default for Request<'_> {
    fn default() -> Self {
        Self {
            url: "http://localhost".into(),
            method: Method::GET,
            headers: Default::default(),
            body: Default::default(),
            config: Default::default(),
        }
    }
}

/// HTTP 请求生成器
#[derive(Default, Debug, Clone)]
pub struct RequestBuilder<'r> {
    inner: Request<'r>,
}

impl<'r> RequestBuilder<'r> {
    /// 设置请求 URL
    #[inline]
    pub fn url(&mut self, url: URL<'r>) -> &mut Self {
        self.inner.url = url;
        self
    }

    /// 设置请求 HTTP 方法
    #[inline]
    pub fn method(&mut self, method: Method) -> &mut Self {
        self.inner.method = method;
        self
    }

    /// 设置请求 HTTP Headers
    #[inline]
    pub fn headers(&mut self, headers: Headers<'r>) -> &mut Self {
        self.inner.headers = headers;
        self
    }

    /// 设置请求体
    #[inline]
    pub fn body(&mut self, body: Body<'r>) -> &mut Self {
        self.inner.body = body;
        self
    }

    /// 设置请求配置
    #[inline]
    pub fn config(&mut self, config: RequestConfig<'r>) -> &mut Self {
        self.inner.config = config;
        self
    }

    /// 构建 HTTP 请求
    #[inline]
    pub fn build(&self) -> Request<'r> {
        self.inner.clone()
    }

    /// 重置构建器
    #[inline]
    pub fn reset(&mut self) {
        self.inner = Default::default();
    }
}
