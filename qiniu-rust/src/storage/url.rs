//! 对象访问 URL 模块

use crate::{
    http::{
        Client as HTTPClient, HeaderNameOwned as HTTPHeaderNameOwned, HeadersOwned as HTTPHeadersOwned,
        Result as HTTPResult,
    },
    Credential,
};
use std::{borrow::Cow, fmt, time::Duration};
use url::Url;

/// URL
///
/// 封装一个可以访问对象数据的 URL
#[derive(Clone)]
pub struct URL(URLInner);

#[derive(Clone)]
enum URLInner {
    PublicURL(PublicURL),
    PrivateURL(PrivateURL),
}

#[derive(Clone)]
struct PublicURL {
    use_https: bool,
    domain: Cow<'static, str>,
    backup_domains: Vec<Cow<'static, str>>,
    key: Cow<'static, str>,
    query: Vec<(Cow<'static, str>, Cow<'static, str>)>,
    fop: Cow<'static, str>,
}

#[derive(Clone)]
struct PrivateURL {
    base_url: PublicURL,
    credential: Credential,
    deadline: Duration,
}

impl PublicURL {
    #[inline]
    fn new(
        use_https: bool,
        domain: Cow<'static, str>,
        backup_domains: Vec<Cow<'static, str>>,
        key: Cow<'static, str>,
        query: Vec<(Cow<'static, str>, Cow<'static, str>)>,
        fop: Cow<'static, str>,
    ) -> Self {
        Self {
            use_https,
            domain,
            backup_domains,
            key,
            query,
            fop,
        }
    }

    #[inline]
    fn generate_base_url(&self) -> String {
        self.generate_base_url_with_domain(&self.domain)
    }

    fn generate_base_urls(&self) -> Vec<String> {
        let mut base_urls = Vec::with_capacity(1 + self.backup_domains.len());
        base_urls.push(self.generate_base_url());
        for domain in self.backup_domains.iter() {
            base_urls.push(self.generate_base_url_with_domain(domain));
        }
        base_urls
    }

    fn generate_base_url_with_domain(&self, domain: &str) -> String {
        let mut s = String::with_capacity(domain.len() + 8);
        if self.use_https {
            s.push_str("https://");
        } else {
            s.push_str("http://");
        }
        s.push_str(domain.as_ref());
        s.push_str("/");
        s
    }

    #[inline]
    fn generate_url(&self) -> Url {
        self.generate_url_with_domain(&self.domain)
    }

    fn generate_url_with_domain(&self, domain: &str) -> Url {
        let mut url = Url::parse(&self.generate_base_url_with_domain(domain)).expect("Given domain is invalid");
        url.set_path(&self.key);
        if !self.fop.is_empty() {
            url.set_query(Some(&self.fop));
        }
        if !self.query.is_empty() {
            let mut query_pairs = url.query_pairs_mut();
            for (query_name, query_value) in self.query.iter() {
                query_pairs.append_pair(query_name, query_value);
            }
        }
        url
    }

    fn head(&self, client: &HTTPClient) -> HTTPResult<HeaderInfo> {
        self._head(client, &|_| {})
    }

    fn _head(&self, client: &HTTPClient, callback: &dyn Fn(&mut Url)) -> HTTPResult<HeaderInfo> {
        let base_urls = self.generate_base_urls();
        let base_urls = base_urls.iter().map(|url| url.as_str()).collect::<Vec<_>>();
        let mut request_builder = client
            .head(&self.key, &base_urls)
            .fop(Cow::Borrowed(&self.fop))
            .idempotent()
            .follow_redirection();
        for (query_name, query_value) in self.query.iter() {
            request_builder = request_builder.query(query_name.to_owned(), query_value.to_owned());
        }
        let request = request_builder.on_url_constructed(callback).no_body();
        let response = request.send()?;
        let mut metadata = HTTPHeadersOwned::new();

        for (header_name, header_value) in response.headers().iter() {
            if header_name.starts_with("X-Qn-Meta-") {
                let metadata_key =
                    HTTPHeaderNameOwned::new(Cow::Borrowed(header_name.get("X-Qn-Meta-".len()..).unwrap()));
                let metadata_value = header_value.to_owned();
                metadata.insert(metadata_key, metadata_value);
            }
        }

        Ok(HeaderInfo {
            content_type: response.header(&"Content-Type".into()).map(|value| value.to_owned()),
            size: response.header(&"Content-Length".into()).map(|value| value.to_owned()),
            etag: response.header(&"Etag".into()).map(|value| value.to_owned()),
            metadata,
        })
    }
}

impl fmt::Display for PublicURL {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.generate_url().into_string().fmt(f)
    }
}

impl PrivateURL {
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn new(
        credential: Credential,
        deadline: Duration,
        use_https: bool,
        domain: Cow<'static, str>,
        backup_domains: Vec<Cow<'static, str>>,
        key: Cow<'static, str>,
        query: Vec<(Cow<'static, str>, Cow<'static, str>)>,
        fop: Cow<'static, str>,
    ) -> Self {
        Self {
            base_url: PublicURL::new(use_https, domain, backup_domains, key, query, fop),
            credential,
            deadline,
        }
    }

    fn generate_url(&self) -> Url {
        let mut url = self.base_url.generate_url();
        self.credential.sign_download_url(&mut url, self.deadline, false);
        url
    }

    fn head(&self, client: &HTTPClient) -> HTTPResult<HeaderInfo> {
        self.base_url._head(client, &|url| {
            self.credential.sign_download_url(url, self.deadline, false)
        })
    }
}

impl fmt::Display for PrivateURL {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.generate_url().into_string().fmt(f)
    }
}

impl URL {
    pub(super) fn new_public_url(
        use_https: bool,
        domain: Cow<'static, str>,
        backup_domains: Vec<Cow<'static, str>>,
        key: Cow<'static, str>,
        query: Vec<(Cow<'static, str>, Cow<'static, str>)>,
        fop: Cow<'static, str>,
    ) -> Self {
        Self(URLInner::PublicURL(PublicURL::new(
            use_https,
            domain,
            backup_domains,
            key,
            query,
            fop,
        )))
    }

    pub(super) fn new_private_url(
        credential: Credential,
        deadline: Duration,
        use_https: bool,
        domain: Cow<'static, str>,
        backup_domains: Vec<Cow<'static, str>>,
        key: Cow<'static, str>,
        query: Vec<(Cow<'static, str>, Cow<'static, str>)>,
        fop: Cow<'static, str>,
    ) -> Self {
        Self(URLInner::PrivateURL(PrivateURL::new(
            credential,
            deadline,
            use_https,
            domain,
            backup_domains,
            key,
            query,
            fop,
        )))
    }

    pub(super) fn head(&self, client: &HTTPClient) -> HTTPResult<HeaderInfo> {
        match &self.0 {
            URLInner::PublicURL(public_url) => public_url.head(client),
            URLInner::PrivateURL(private_url) => private_url.head(client),
        }
    }
}

impl fmt::Display for URL {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            URLInner::PublicURL(public_url) => public_url.fmt(f),
            URLInner::PrivateURL(private_url) => private_url.fmt(f),
        }
    }
}

/// 访问下载 URL 时获得的 Header 信息
#[derive(Clone, Debug)]
pub struct HeaderInfo {
    content_type: Option<String>,
    size: Option<String>,
    etag: Option<String>,
    metadata: HTTPHeadersOwned,
}

impl HeaderInfo {
    /// 获取 Header 信息中的 Content-Type 字段
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_ref().map(|s| s.as_str())
    }

    /// 获取 Header 信息中的 Content-Length 字段
    pub fn size(&self) -> Option<&str> {
        self.size.as_ref().map(|s| s.as_str())
    }

    /// 获取 Header 信息中的 Etag 字段
    pub fn etag(&self) -> Option<&str> {
        self.etag.as_ref().map(|s| s.as_str())
    }

    /// 获取 Header 信息中的 Metadata 字段
    pub fn metadata(&self) -> &HTTPHeadersOwned {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        http::{Client as HTTPClient, DomainsManagerBuilder, HeadersOwned},
        ConfigBuilder,
    };
    use qiniu_test_utils::http_call_mock::{ErrorResponseMock, HeadResponse, URLRecorderCallMock};
    use std::{error::Error, result::Result, time::Duration};

    #[test]
    fn test_public_url_generate() -> Result<(), Box<dyn Error>> {
        let url = PublicURL::new(
            true,
            "test-a.com".into(),
            vec![],
            "test key.flv".into(),
            vec![("attname".into(), "test key.flv".into())],
            "avinfo".into(),
        );
        assert_eq!(
            url.to_string(),
            "https://test-a.com/test%20key.flv?avinfo&attname=test+key.flv"
        );
        Ok(())
    }

    #[test]
    fn test_public_url_head() -> Result<(), Box<dyn Error>> {
        let mut headers = HeadersOwned::new();
        headers.insert("Content-Type".into(), "video/mp4".into());
        headers.insert("Content-Length".into(), "123456".into());
        headers.insert("Etag".into(), "abcdefghklmnopq".into());
        headers.insert("X-Qn-Meta-Metakey-1".into(), "metavalue-1".into());
        headers.insert("X-Qn-Meta-Metakey-2".into(), "metavalue-2".into());
        let client = HTTPClient::new(
            ConfigBuilder::default()
                .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
                .http_request_handler(HeadResponse::new(200, headers))
                .build(),
        );
        let url = PublicURL::new(
            true,
            "test-a.com".into(),
            vec![],
            "test key.flv".into(),
            vec![("attname".into(), "test key.flv".into())],
            "avinfo".into(),
        );
        let header_info = url.head(&client)?;
        assert_eq!(header_info.content_type(), Some("video/mp4"));
        assert_eq!(header_info.size(), Some("123456"));
        assert_eq!(header_info.etag(), Some("abcdefghklmnopq"));
        assert_eq!(
            header_info.metadata().get(&"metakey-1".into()),
            Some(&"metavalue-1".into())
        );
        assert_eq!(
            header_info.metadata().get(&"metakey-2".into()),
            Some(&"metavalue-2".into())
        );
        Ok(())
    }

    #[test]
    fn test_private_urls_head() -> Result<(), Box<dyn Error>> {
        let caller = URLRecorderCallMock::new(ErrorResponseMock::new(502, "Bad gateway"));
        let client = HTTPClient::new(
            ConfigBuilder::default()
                .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
                .http_request_handler(caller.clone())
                .build(),
        );

        let url = PrivateURL::new(
            get_credential(),
            Duration::from_secs(123_456_789),
            true,
            "test-a.com".into(),
            vec!["test-b.com".into(), "test-c.com".into()],
            "test key.flv".into(),
            vec![("attname".into(), "test key.flv".into())],
            "avinfo".into(),
        );

        url.head(&client).unwrap_err();
        let urls_called = caller.urls_called();
        assert_eq!(urls_called.len(), 3);
        assert_eq!(urls_called.get(0), Some(&"https://test-a.com/test%20key.flv?avinfo&attname=test+key.flv&e=123456789&token=abcdefghklmnopq%3A6JVKiUUTAtqVctGYY7xlfxbMzQc%3D".into()));
        assert_eq!(urls_called.get(1), Some(&"https://test-b.com/test%20key.flv?avinfo&attname=test+key.flv&e=123456789&token=abcdefghklmnopq%3AkCPuDYCByjCskktvBqqLSfaL7bU%3D".into()));
        assert_eq!(urls_called.get(2), Some(&"https://test-c.com/test%20key.flv?avinfo&attname=test+key.flv&e=123456789&token=abcdefghklmnopq%3APM-V8sijpo5QypM6GimxpiuLFEw%3D".into()));
        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
