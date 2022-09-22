use auto_impl::auto_impl;
use dyn_clonable::clonable;
use http::Uri;
use qiniu_apis::http_client::ApiResult;
use std::{fmt::Debug, mem::take, time::Duration};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// 生成下载 URL 列表的接口
///
/// 同时提供阻塞接口和异步接口，异步接口则需要启用 `async` 功能
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait DownloadUrlsGenerator: Clone + Debug + Sync + Send {
    /// 生成下载 URL 列表
    ///
    /// 该方法的异步版本为 [`Self::async_generate`]。
    fn generate(&self, object_name: &str, options: GeneratorOptions<'_>) -> ApiResult<Vec<Uri>>;

    /// 异步生成下载 URL 列表
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_generate<'a>(
        &'a self,
        object_name: &'a str,
        options: GeneratorOptions<'a>,
    ) -> BoxFuture<'a, ApiResult<Vec<Uri>>> {
        Box::pin(async move { self.generate(object_name, options) })
    }
}

/// 生成下载 URL 的选项
#[derive(Copy, Debug, Clone, Default)]
pub struct GeneratorOptions<'a> {
    _unused: Option<&'a ()>,
    ttl: Option<Duration>,
}

/// 生成下载 URL 的选项构建器
#[derive(Copy, Debug, Clone, Default)]
pub struct GeneratorOptionsBuilder<'a>(GeneratorOptions<'a>);

impl<'a> GeneratorOptions<'a> {
    /// 创建 生成下载 URL 的选项构建器
    #[inline]
    pub fn builder() -> GeneratorOptionsBuilder<'a> {
        Default::default()
    }

    /// 获取下载 URL 有效期
    #[inline]
    pub fn ttl(&self) -> Option<Duration> {
        self.ttl
    }
}

impl<'a> GeneratorOptionsBuilder<'a> {
    /// 设置下载 URL 有效期
    #[inline]
    pub fn ttl(&mut self, ttl: Duration) -> &mut Self {
        self.0.ttl = Some(ttl);
        self
    }

    /// 构建生成下载 URL 的选项
    #[inline]
    pub fn build(&mut self) -> GeneratorOptions<'a> {
        take(&mut self.0)
    }
}

mod urls_signer;
pub use urls_signer::UrlsSigner;

mod endpoints;
pub use endpoints::{EndpointsUrlGenerator, EndpointsUrlGeneratorBuilder};

mod static_domains;
pub use static_domains::{StaticDomainsUrlsGenerator, StaticDomainsUrlsGeneratorBuilder};
