use super::Region;
use crate::{config::Config, http, utils::auth::Auth};
use once_cell::sync::OnceCell;
use qiniu_http::Result;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, slice::Iter};

pub struct Bucket<'r> {
    name: Cow<'r, str>,
    auth: Auth,
    config: Config,
    region: OnceCell<Cow<'r, Region>>,
    domains: OnceCell<Vec<Cow<'r, str>>>,
    http_client: http::Client,
    use_https: bool,
}

pub struct BucketBuilder<'r> {
    name: Cow<'r, str>,
    auth: Auth,
    config: Config,
    region: Option<Cow<'r, Region>>,
    domains: Option<Vec<Cow<'r, str>>>,
    http_client: http::Client,
    use_https: bool,
}

impl<'r> BucketBuilder<'r> {
    pub(crate) fn new<B: Into<Cow<'r, str>>>(name: B, auth: Auth, config: Config) -> BucketBuilder<'r> {
        BucketBuilder {
            name: name.into(),
            use_https: config.use_https(),
            http_client: http::Client::new(auth.clone(), config.clone()),
            auth: auth,
            config: config,
            region: None,
            domains: None,
        }
    }

    pub fn region(mut self, region: &'r Region) -> BucketBuilder<'r> {
        self.region = Some(Cow::Borrowed(region));
        self
    }

    pub fn auto_detect_region(mut self) -> Result<BucketBuilder<'r>> {
        self.region = Some(Cow::Owned(Region::query(
            self.name.as_ref(),
            self.auth.clone(),
            self.config.clone(),
        )?));
        Ok(self)
    }

    pub fn domain(mut self, domain: &'r str) -> BucketBuilder<'r> {
        match self.domains {
            Some(ref mut domains) => {
                domains.push(Cow::from(domain));
            }
            None => {
                self.domains = Some(vec![Cow::from(domain)]);
            }
        }
        self
    }

    pub fn auto_detect_domains(mut self) -> Result<BucketBuilder<'r>> {
        self.domains = Some(
            domain::query(&self.http_client, self.uc_url(), self.name.as_ref())?
                .into_iter()
                .map(|domain| Cow::Owned(domain))
                .collect(),
        );
        Ok(self)
    }

    pub fn build(self) -> Bucket<'r> {
        Bucket {
            name: self.name,
            auth: self.auth,
            use_https: self.use_https,
            http_client: self.http_client,
            config: self.config,
            region: self
                .region
                .map(|r| OnceCell::from(r))
                .unwrap_or_else(|| OnceCell::new()),
            domains: self
                .domains
                .map(|d| OnceCell::from(d))
                .unwrap_or_else(|| OnceCell::new()),
        }
    }

    fn uc_url(&self) -> &'static str {
        Region::uc_url(self.use_https)
    }
}

impl<'r> Bucket<'r> {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn region(&self) -> Result<&Region> {
        self.region
            .get_or_try_init(|| {
                Ok(Cow::Owned(Region::query(
                    self.name(),
                    self.auth.clone(),
                    self.config.clone(),
                )?))
            })
            .map(|region| region.as_ref())
    }

    pub fn domains(&self) -> Result<Vec<&str>> {
        let domains = self.domains.get_or_try_init(|| {
            Ok(domain::query(&self.http_client, self.uc_url(), self.name())?
                .into_iter()
                .map(|domain| Cow::Owned(domain))
                .collect())
        })?;
        Ok(domains.iter().map(|domain| domain.as_ref()).collect())
    }

    fn rs_url(&self) -> &'static str {
        self.region()
            .unwrap_or_else(|_| Region::hua_dong())
            .rs_url(self.use_https)
    }

    fn uc_url(&self) -> &'static str {
        Region::uc_url(self.use_https)
    }
}

mod domain {
    use crate::http;
    use qiniu_http::Result;

    pub(super) fn query(http_client: &http::Client, uc_url: &str, bucket_name: &str) -> Result<Vec<String>> {
        Ok(http_client
            .get("/v6/domain/list", &[uc_url])
            .query("tbl", bucket_name)
            .token(http::Token::V1)
            .no_body()
            .send()?
            .parse_json()?)
    }
}

#[cfg(test)]
mod tests {
    use super::{super::RegionId, *};
    use crate::config::ConfigBuilder;
    use qiniu_http::Headers;
    use qiniu_test_utils::http_call_mock::{CounterCallMock, JSONCallMock};
    use serde_json::json;
    use std::{boxed::Box, sync::Arc, thread};

    #[test]
    fn test_storage_bucket_set_region() {
        let config: Config = ConfigBuilder::default()
            .http_request_call(Box::new(http::PanickedHTTPCaller("Should not call it")))
            .build()
            .unwrap();
        let bucket = BucketBuilder::new("test-bucket", get_auth(), config)
            .region(Region::hua_bei())
            .build();
        assert_eq!(bucket.region().unwrap().region_id(), Some(RegionId::Z1));
    }

    #[test]
    fn test_storage_bucket_prequery_region() {
        let mock = CounterCallMock::new(JSONCallMock::new(
            200,
            Headers::new(),
            json!({
                "io": { "src": { "main": [ "iovip.qbox.me" ] } },
                "up": {
                    "acc": { "backup": [ "upload-jjh.qiniup.com", "upload-xs.qiniup.com" ], "main": [ "upload.qiniup.com" ] },
                    "old_acc": { "info": "compatible to non-SNI device", "main": [ "upload.qbox.me" ] },
                    "old_src": { "info": "compatible to non-SNI device", "main": [ "up.qbox.me" ] },
                    "src": { "backup": [ "up-jjh.qiniup.com", "up-xs.qiniup.com" ], "main": [ "up.qiniup.com" ] }
                }
            }),
        ));
        let config: Config = ConfigBuilder::default()
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        let bucket = BucketBuilder::new("test-bucket", get_auth(), config)
            .auto_detect_region()
            .unwrap()
            .build();
        assert_eq!(mock.call_called(), 1);
        assert!(bucket
            .region()
            .unwrap()
            .up_urls(true)
            .contains(&"https://up-xs.qiniup.com".into()));
        assert!(bucket
            .region()
            .unwrap()
            .up_urls(true)
            .contains(&"https://up-jjh.qiniup.com".into()));
        assert!(bucket
            .region()
            .unwrap()
            .up_urls(true)
            .contains(&"https://upload.qbox.me".into()));
        assert_eq!(mock.call_called(), 1);
    }

    #[test]
    fn test_storage_bucket_query_region() {
        let mock = CounterCallMock::new(JSONCallMock::new(
            200,
            Headers::new(),
            json!({
                "io": { "src": { "main": [ "iovip.qbox.me" ] } },
                "up": {
                    "acc": { "backup": [ "upload-jjh.qiniup.com", "upload-xs.qiniup.com" ], "main": [ "upload.qiniup.com" ] },
                    "old_acc": { "info": "compatible to non-SNI device", "main": [ "upload.qbox.me" ] },
                    "old_src": { "info": "compatible to non-SNI device", "main": [ "up.qbox.me" ] },
                    "src": { "backup": [ "up-jjh.qiniup.com", "up-xs.qiniup.com" ], "main": [ "up.qiniup.com" ] }
                }
            }),
        ));
        let config: Config = ConfigBuilder::default()
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        let bucket = Arc::new(BucketBuilder::new("test-bucket", get_auth(), config).build());
        assert_eq!(mock.call_called(), 0);

        let mut threads = Vec::with_capacity(3);
        let b = bucket.clone();
        threads.push(thread::spawn(move || {
            assert!(b
                .region()
                .unwrap()
                .up_urls(true)
                .contains(&"https://up-xs.qiniup.com".into()));
        }));

        let b = bucket.clone();
        threads.push(thread::spawn(move || {
            assert!(b
                .region()
                .unwrap()
                .up_urls(true)
                .contains(&"https://up-jjh.qiniup.com".into()));
        }));

        let b = bucket.clone();
        threads.push(thread::spawn(move || {
            assert!(b
                .region()
                .unwrap()
                .up_urls(true)
                .contains(&"https://upload.qbox.me".into()));
        }));

        threads.into_iter().for_each(|thread| thread.join().unwrap());
        assert_eq!(mock.call_called(), 1);
    }

    #[test]
    fn test_storage_bucket_set_domain() {
        let config: Config = ConfigBuilder::default()
            .http_request_call(Box::new(http::PanickedHTTPCaller("Should not call it")))
            .build()
            .unwrap();
        let bucket = BucketBuilder::new("test-bucket", get_auth(), config)
            .domain("abc.com")
            .domain("def.com")
            .build();
        assert_eq!(bucket.domains().unwrap().len(), 2);
        assert_eq!(bucket.domains().unwrap().first(), Some(&"abc.com"));
    }

    #[test]
    fn test_storage_bucket_prequery_domain() {
        let mock = CounterCallMock::new(JSONCallMock::new(200, Headers::new(), json!(["abc.com", "def.com"])));
        let config: Config = ConfigBuilder::default()
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        let bucket = BucketBuilder::new("test-bucket", get_auth(), config)
            .auto_detect_domains()
            .unwrap()
            .build();
        assert_eq!(mock.call_called(), 1);
        assert!(bucket.domains().unwrap().contains(&"abc.com"));
        assert!(bucket.domains().unwrap().contains(&"def.com"));
        assert_eq!(mock.call_called(), 1);
    }

    #[test]
    fn test_storage_bucket_query_domain() {
        let mock = CounterCallMock::new(JSONCallMock::new(200, Headers::new(), json!(["abc.com", "def.com"])));
        let config: Config = ConfigBuilder::default()
            .http_request_call(mock.as_boxed())
            .build()
            .unwrap();
        let bucket = Arc::new(BucketBuilder::new("test-bucket", get_auth(), config).build());
        assert_eq!(mock.call_called(), 0);

        let mut threads = Vec::with_capacity(3);
        let b = bucket.clone();
        threads.push(thread::spawn(move || {
            assert!(b.domains().unwrap().contains(&"abc.com"));
        }));

        let b = bucket.clone();
        threads.push(thread::spawn(move || {
            assert!(b.domains().unwrap().contains(&"def.com"));
        }));

        threads.into_iter().for_each(|thread| thread.join().unwrap());
        assert_eq!(mock.call_called(), 1);
    }

    fn get_auth() -> Auth {
        Auth::new("abcdefghklmnopq", "1234567890")
    }
}
