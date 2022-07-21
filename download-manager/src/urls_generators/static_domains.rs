use super::{DownloadUrlsGenerator, EndpointsUrlGenerator, GeneratorOptions};
use http::Uri;
use qiniu_apis::http_client::{ApiResult, Endpoint, Endpoints, EndpointsBuilder};
use std::mem::take;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// 静态公开空间域名下载 URL 列表生成器
#[derive(Debug, Clone)]
pub struct StaticDomainsUrlsGenerator(EndpointsUrlGenerator);

impl StaticDomainsUrlsGenerator {
    /// 创建静态公开空间域名下载 URL 列表生成构建器
    ///
    /// 必须添加第一个域名
    #[inline]
    pub fn builder(first_domain: impl Into<Endpoint>) -> StaticDomainsUrlsGeneratorBuilder {
        StaticDomainsUrlsGeneratorBuilder::new(first_domain)
    }

    /// 创建静态公开空间域名下载 URL 列表生成器
    ///
    /// 只能添加一个域名
    #[inline]
    pub fn new(first_domain: impl Into<Endpoint>) -> Self {
        StaticDomainsUrlsGeneratorBuilder::new(first_domain).build()
    }
}

/// 静态公开空间域名下载 URL 列表生成构建器
#[derive(Debug, Clone)]
pub struct StaticDomainsUrlsGeneratorBuilder {
    endpoints: EndpointsBuilder,
    use_https: bool,
}

impl StaticDomainsUrlsGeneratorBuilder {
    /// 创建静态公开空间域名下载 URL 列表生成构建器
    ///
    /// 必须添加第一个域名
    #[inline]
    pub fn new(first_domain: impl Into<Endpoint>) -> Self {
        Self {
            endpoints: Endpoints::builder(first_domain.into()),
            use_https: false,
        }
    }

    /// 是否启用 HTTPS 协议
    ///
    /// 默认为 HTTPS 协议
    #[inline]
    pub fn use_https(&mut self, use_https: bool) -> &mut Self {
        self.use_https = use_https;
        self
    }

    /// 添加新的域名到静态公开空间域名下载 URL 列表生成构建器
    #[inline]
    pub fn add_domain(&mut self, domain: impl Into<Endpoint>) -> &mut Self {
        self.endpoints.add_preferred_endpoint(domain.into());
        self
    }

    /// 构建静态公开空间域名下载 URL 列表生成器
    #[inline]
    pub fn build(&mut self) -> StaticDomainsUrlsGenerator {
        StaticDomainsUrlsGenerator(
            EndpointsUrlGenerator::builder(self.endpoints.build())
                .use_https(take(&mut self.use_https))
                .build(),
        )
    }
}

impl DownloadUrlsGenerator for StaticDomainsUrlsGenerator {
    fn generate(&self, object_name: &str, options: GeneratorOptions<'_>) -> ApiResult<Vec<Uri>> {
        self.0.generate(object_name, options)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_generate<'a>(
        &'a self,
        object_name: &'a str,
        options: GeneratorOptions<'a>,
    ) -> BoxFuture<'a, ApiResult<Vec<Uri>>> {
        self.0.async_generate(object_name, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urls_signer() -> anyhow::Result<()> {
        let generator = StaticDomainsUrlsGeneratorBuilder::new("first.domain.com")
            .add_domain("second.domain.com")
            .use_https(false)
            .build();
        let urls = generator.generate("abc/def/中文", Default::default())?;
        assert!(urls
            .get(0)
            .unwrap()
            .to_string()
            .starts_with("http://first.domain.com/abc/def/%E4%B8%AD%E6%96%87"));
        assert!(!urls.get(0).unwrap().to_string().contains("?e="));
        assert!(!urls.get(0).unwrap().to_string().contains("&token=ak"));
        assert!(urls
            .get(1)
            .unwrap()
            .to_string()
            .starts_with("http://second.domain.com/abc/def/%E4%B8%AD%E6%96%87"));
        assert!(!urls.get(1).unwrap().to_string().contains("?e="));
        assert!(!urls.get(1).unwrap().to_string().contains("&token=ak"));
        Ok(())
    }
}
