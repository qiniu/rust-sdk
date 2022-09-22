use super::{DownloadUrlsGenerator, GeneratorOptions};
use http::Uri;
use qiniu_apis::{
    http::ResponseErrorKind,
    http_client::{ApiResult, Endpoints, EndpointsProvider, ResponseError},
};
use std::sync::Arc;
use url_escape::encode_path_to_string;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// 终端地址下载 URL 列表生成器
#[derive(Debug, Clone)]
pub struct EndpointsUrlGenerator {
    endpoints: Arc<dyn EndpointsProvider>,
    use_https: bool,
}

/// 终端地址下载 URL 列表生成构建器
#[derive(Debug, Clone)]
pub struct EndpointsUrlGeneratorBuilder(EndpointsUrlGenerator);

impl EndpointsUrlGenerator {
    /// 创建终端地址下载 URL 列表生成构建器
    #[inline]
    pub fn builder(endpoints: impl EndpointsProvider + 'static) -> EndpointsUrlGeneratorBuilder {
        EndpointsUrlGeneratorBuilder::new(endpoints)
    }
}

impl EndpointsUrlGeneratorBuilder {
    /// 创建终端地址下载 URL 列表生成构建器
    #[inline]
    pub fn new(endpoints: impl EndpointsProvider + 'static) -> Self {
        Self(EndpointsUrlGenerator {
            endpoints: Arc::new(endpoints),
            use_https: true,
        })
    }

    /// 是否启用 HTTPS 协议
    ///
    /// 默认为 HTTPS 协议
    #[inline]
    pub fn use_https(&mut self, use_https: bool) -> &mut Self {
        self.0.use_https = use_https;
        self
    }

    /// 构建终端地址下载 URL 列表生成器
    #[inline]
    pub fn build(&self) -> EndpointsUrlGenerator {
        self.0.to_owned()
    }
}

impl DownloadUrlsGenerator for EndpointsUrlGenerator {
    fn generate(&self, object_name: &str, _options: GeneratorOptions<'_>) -> ApiResult<Vec<Uri>> {
        let endpoints = self.endpoints.get_endpoints(Default::default())?;
        generate_public_urls(&endpoints, object_name, self.use_https).collect()
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_generate<'a>(
        &'a self,
        object_name: &'a str,
        _options: GeneratorOptions<'a>,
    ) -> BoxFuture<'a, ApiResult<Vec<Uri>>> {
        Box::pin(async move {
            let endpoints = self.endpoints.async_get_endpoints(Default::default()).await?;
            generate_public_urls(&endpoints, object_name, self.use_https).collect()
        })
    }
}

fn generate_public_urls<'a>(
    endpoints: &'a Endpoints,
    object_name: &'a str,
    use_https: bool,
) -> impl Iterator<Item = ApiResult<Uri>> + 'a {
    endpoints.preferred().iter().map(move |endpoint| {
        let mut path = "/".to_owned();
        encode_path_to_string(object_name, &mut path);
        let mut builder = Uri::builder().authority(endpoint.to_string()).path_and_query(path);
        if use_https {
            builder = builder.scheme("https");
        } else {
            builder = builder.scheme("http");
        }
        builder
            .build()
            .map_err(|err| ResponseError::new(ResponseErrorKind::InvalidUrl.into(), err))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoints_url_generator() -> anyhow::Result<()> {
        let generator = EndpointsUrlGenerator::builder(
            Endpoints::builder("first.domain.com".parse()?)
                .add_preferred_endpoint("second.domain.com".parse()?)
                .build(),
        )
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
