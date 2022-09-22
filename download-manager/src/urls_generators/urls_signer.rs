use super::{DownloadUrlsGenerator, GeneratorOptions};
use http::Uri;
use qiniu_apis::{credential::CredentialProvider, http_client::ApiResult};
use std::time::Duration;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// URL 列表签名器
#[derive(Debug, Clone)]
pub struct UrlsSigner {
    generator: Box<dyn DownloadUrlsGenerator>,
    credential: Box<dyn CredentialProvider>,
}

impl UrlsSigner {
    /// 创建静态私有空间域名下载 URL 列表生成构建器
    ///
    /// 必须添加第一个域名
    #[inline]
    pub fn new(credential: impl CredentialProvider + 'static, generator: impl DownloadUrlsGenerator + 'static) -> Self {
        Self {
            credential: Box::new(credential),
            generator: Box::new(generator),
        }
    }
}

impl DownloadUrlsGenerator for UrlsSigner {
    fn generate(&self, object_name: &str, options: GeneratorOptions<'_>) -> ApiResult<Vec<Uri>> {
        let credential = self.credential.get(Default::default())?;
        let ttl = options.ttl().unwrap_or(Duration::from_secs(3600));
        Ok(self
            .generator
            .generate(object_name, options)?
            .into_iter()
            .map(|url| credential.sign_download_url(url, ttl))
            .collect())
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_generate<'a>(
        &'a self,
        object_name: &'a str,
        options: GeneratorOptions<'a>,
    ) -> BoxFuture<'a, ApiResult<Vec<Uri>>> {
        Box::pin(async move {
            let credential = self.credential.async_get(Default::default()).await?;
            let ttl = options.ttl().unwrap_or(Duration::from_secs(3600));
            Ok(self
                .generator
                .async_generate(object_name, options)
                .await?
                .into_iter()
                .map(|url| credential.sign_download_url(url, ttl))
                .collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{super::StaticDomainsUrlsGenerator, *};
    use qiniu_apis::credential::Credential;

    #[test]
    fn test_urls_signer() -> anyhow::Result<()> {
        let generator = UrlsSigner::new(
            Credential::new("ak", "sk"),
            StaticDomainsUrlsGenerator::builder("first.domain.com")
                .use_https(false)
                .add_domain("second.domain.com")
                .build(),
        );
        let urls = generator.generate(
            "abc/def/中文",
            GeneratorOptions::builder().ttl(Duration::from_secs(100)).build(),
        )?;
        assert!(urls
            .get(0)
            .unwrap()
            .to_string()
            .starts_with("http://first.domain.com/abc/def/%E4%B8%AD%E6%96%87?e="));
        assert!(urls.get(0).unwrap().to_string().contains("&token=ak"));
        assert!(urls
            .get(1)
            .unwrap()
            .to_string()
            .starts_with("http://second.domain.com/abc/def/%E4%B8%AD%E6%96%87?e="));
        assert!(urls.get(1).unwrap().to_string().contains("&token=ak"));
        Ok(())
    }
}
