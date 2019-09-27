use super::{
    region::Region,
    uploader::{BucketUploader, Uploader},
};
use crate::{config::Config, credential::Credential, http};
use once_cell::sync::OnceCell;
use qiniu_http::Result;
use std::{borrow::Cow, iter::Iterator};

pub struct Bucket<'r> {
    name: Cow<'r, str>,
    credential: Credential,
    config: Config,
    region: OnceCell<Cow<'r, Region>>,
    regions: OnceCell<Vec<Region>>,
    domains: OnceCell<Vec<Cow<'r, str>>>,
    http_client: http::Client,
}

pub struct BucketBuilder<'r> {
    name: Cow<'r, str>,
    credential: Credential,
    config: Config,
    region: Option<Cow<'r, Region>>,
    regions: Option<Vec<Region>>,
    domains: Option<Vec<Cow<'r, str>>>,
    http_client: http::Client,
}

pub struct BucketRegionIter<'a, 'r: 'a> {
    bucket: &'a Bucket<'r>,
    itered: usize,
}

impl<'r> BucketBuilder<'r> {
    pub(crate) fn new<B: Into<Cow<'r, str>>>(name: B, credential: Credential, config: Config) -> BucketBuilder<'r> {
        BucketBuilder {
            name: name.into(),
            http_client: http::Client::new(credential.clone(), config.clone()),
            credential: credential,
            config: config,
            region: None,
            regions: None,
            domains: None,
        }
    }

    pub fn region(mut self, region: &'r Region) -> BucketBuilder<'r> {
        self.region = Some(Cow::Borrowed(region));
        self
    }

    pub fn auto_detect_region(mut self) -> Result<BucketBuilder<'r>> {
        let mut regions = Region::query(self.name.as_ref(), self.credential.clone(), self.config.clone())?;
        self.region = Some(Cow::Owned(regions.swap_remove(0)));
        if !regions.is_empty() {
            self.regions = Some(regions);
        }
        Ok(self)
    }

    pub fn domain(mut self, domain: &'r str) -> BucketBuilder<'r> {
        match &mut self.domains {
            Some(domains) => {
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
            credential: self.credential,
            http_client: self.http_client,
            config: self.config,
            region: self
                .region
                .map(|r| OnceCell::from(r))
                .unwrap_or_else(|| OnceCell::new()),
            regions: self
                .regions
                .map(|r| OnceCell::from(r))
                .unwrap_or_else(|| OnceCell::new()),
            domains: self
                .domains
                .map(|d| OnceCell::from(d))
                .unwrap_or_else(|| OnceCell::new()),
        }
    }

    fn uc_url(&self) -> &'static str {
        Region::uc_url(self.config.use_https())
    }
}

impl<'r> Bucket<'r> {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn region(&self) -> Result<&Region> {
        self.region
            .get_or_try_init(|| {
                let mut regions = Region::query(self.name(), self.credential.clone(), self.config.clone())?;
                let first_region = Cow::Owned(regions.swap_remove(0));
                self.regions.get_or_init(|| regions);
                Ok(first_region)
            })
            .map(|region| region.as_ref())
    }

    pub fn regions<'a>(&'a self) -> Result<BucketRegionIter<'a, 'r>> {
        self.region()?;
        Ok(BucketRegionIter {
            bucket: self,
            itered: 0,
        })
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

    pub fn uploader(&self) -> Result<BucketUploader> {
        Uploader::new(self.credential.clone(), self.config.clone()).for_bucket(self)
    }

    fn rs_url(&self) -> &'static str {
        self.region()
            .unwrap_or_else(|_| Region::hua_dong())
            .rs_url(self.config.use_https())
    }

    fn uc_url(&self) -> &'static str {
        Region::uc_url(self.config.use_https())
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

impl<'a, 'r: 'a> Iterator for BucketRegionIter<'a, 'r> {
    type Item = &'a Region;

    fn next(&mut self) -> Option<Self::Item> {
        if self.itered == 0 {
            return self.bucket.region.get().map(|region| {
                self.itered += 1;
                region.as_ref()
            });
        } else {
            return self.bucket.regions.get().and_then(|regions| {
                self.itered += 1;
                regions.get(self.itered - 2)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::region::RegionId, *};
    use crate::config::ConfigBuilder;
    use qiniu_http::Headers;
    use qiniu_test_utils::http_call_mock::{CounterCallMock, JSONCallMock};
    use serde_json::json;
    use std::{boxed::Box, error::Error, result::Result, sync::Arc, thread};

    #[test]
    fn test_storage_bucket_set_region() -> Result<(), Box<dyn Error>> {
        let bucket = BucketBuilder::new(
            "test-bucket",
            get_credential(),
            ConfigBuilder::default()
                .http_request_call(Box::new(http::PanickedHTTPCaller("Should not call it")))
                .build()?,
        )
        .region(Region::hua_bei())
        .build();
        assert_eq!(bucket.region()?.region_id(), Some(RegionId::Z1));
        let regions = bucket.regions()?.collect::<Vec<_>>();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions.first().unwrap().region_id(), Some(RegionId::Z1));
        Ok(())
    }

    #[test]
    fn test_storage_bucket_prequery_region() -> Result<(), Box<dyn Error>> {
        let mock = CounterCallMock::new(JSONCallMock::new(
            200,
            Headers::new(),
            json!({
                "hosts": [{
                    "io": { "src": { "main": [ "iovip.qbox.me" ] } },
                    "up": {
                        "acc": { "backup": [ "upload-jjh.qiniup.com", "upload-xs.qiniup.com" ], "main": [ "upload.qiniup.com" ] },
                        "old_acc": { "info": "compatible to non-SNI device", "main": [ "upload.qbox.me" ] },
                        "old_src": { "info": "compatible to non-SNI device", "main": [ "up.qbox.me" ] },
                        "src": { "backup": [ "up-jjh.qiniup.com", "up-xs.qiniup.com" ], "main": [ "up.qiniup.com" ] }
                    }
                },{
                    "io": { "src": { "main": [ "iovip-z1.qbox.me" ] } },
                    "up": {
                        "acc": { "backup": [ "upload-jjh-z1.qiniup.com", "upload-xs-z1.qiniup.com" ], "main": [ "upload-z1.qiniup.com" ] },
                        "old_acc": { "info": "compatible to non-SNI device", "main": [ "upload-z1.qbox.me" ] },
                        "old_src": { "info": "compatible to non-SNI device", "main": [ "up-z1.qbox.me" ] },
                        "src": { "backup": [ "up-jjh-z1.qiniup.com", "up-xs-z1.qiniup.com" ], "main": [ "up-z1.qiniup.com" ] }
                    }
                }]
            }),
        ));
        let bucket = BucketBuilder::new(
            "test-bucket",
            get_credential(),
            ConfigBuilder::default().http_request_call(mock.as_boxed()).build()?,
        )
        .auto_detect_region()?
        .build();
        assert_eq!(mock.call_called(), 1);
        assert!(bucket
            .region()?
            .up_urls(true)
            .contains(&"https://up-xs.qiniup.com".into()));
        assert!(bucket
            .region()?
            .up_urls(true)
            .contains(&"https://up-jjh.qiniup.com".into()));
        assert!(bucket
            .region()?
            .up_urls(true)
            .contains(&"https://upload.qbox.me".into()));

        let regions = bucket.regions()?.collect::<Vec<_>>();
        assert_eq!(regions.len(), 2);
        assert!(regions
            .get(1)
            .unwrap()
            .up_urls(true)
            .contains(&"https://up-xs-z1.qiniup.com".into()));
        assert!(regions
            .get(1)
            .unwrap()
            .up_urls(true)
            .contains(&"https://up-jjh-z1.qiniup.com".into()));
        assert!(regions
            .get(1)
            .unwrap()
            .up_urls(true)
            .contains(&"https://upload-z1.qbox.me".into()));

        assert_eq!(mock.call_called(), 1);

        Ok(())
    }

    #[test]
    fn test_storage_bucket_query_region() -> Result<(), Box<dyn Error>> {
        let mock = CounterCallMock::new(JSONCallMock::new(
            200,
            Headers::new(),
            json!({
                "hosts": [{
                    "io": { "src": { "main": [ "iovip.qbox.me" ] } },
                    "up": {
                        "acc": { "backup": [ "upload-jjh.qiniup.com", "upload-xs.qiniup.com" ], "main": [ "upload.qiniup.com" ] },
                        "old_acc": { "info": "compatible to non-SNI device", "main": [ "upload.qbox.me" ] },
                        "old_src": { "info": "compatible to non-SNI device", "main": [ "up.qbox.me" ] },
                        "src": { "backup": [ "up-jjh.qiniup.com", "up-xs.qiniup.com" ], "main": [ "up.qiniup.com" ] }
                    }
                },{
                    "io": { "src": { "main": [ "iovip-z2.qbox.me" ] } },
                    "up": {
                        "acc": { "backup": [ "upload-jjh-z2.qiniup.com", "upload-xs-z2.qiniup.com" ], "main": [ "upload-z2.qiniup.com" ] },
                        "old_acc": { "info": "compatible to non-SNI device", "main": [ "upload-z2.qbox.me" ] },
                        "old_src": { "info": "compatible to non-SNI device", "main": [ "up-z2.qbox.me" ] },
                        "src": { "backup": [ "up-jjh-z2.qiniup.com", "up-xs-z2.qiniup.com" ], "main": [ "up-z2.qiniup.com" ] }
                    }
                }]
            }),
        ));
        let bucket = Arc::new(
            BucketBuilder::new(
                "test-bucket",
                get_credential(),
                ConfigBuilder::default().http_request_call(mock.as_boxed()).build()?,
            )
            .build(),
        );
        assert_eq!(mock.call_called(), 0);

        let mut threads = Vec::with_capacity(4);
        {
            let bucket = bucket.clone();
            threads.push(thread::spawn(move || {
                assert!(bucket
                    .region()
                    .unwrap()
                    .up_urls(true)
                    .contains(&"https://up-xs.qiniup.com".into()));
            }));
        }

        {
            let bucket = bucket.clone();
            threads.push(thread::spawn(move || {
                assert!(bucket
                    .region()
                    .unwrap()
                    .up_urls(true)
                    .contains(&"https://up-jjh.qiniup.com".into()));
            }));
        }

        {
            let bucket = bucket.clone();
            threads.push(thread::spawn(move || {
                assert!(bucket
                    .region()
                    .unwrap()
                    .up_urls(true)
                    .contains(&"https://upload.qbox.me".into()));
            }));
        }

        {
            let bucket = bucket.clone();
            threads.push(thread::spawn(move || {
                let regions = bucket.regions().unwrap().collect::<Vec<_>>();
                assert_eq!(regions.len(), 2);
                assert!(regions
                    .get(1)
                    .unwrap()
                    .up_urls(true)
                    .contains(&"https://up-xs-z2.qiniup.com".into()));
                assert!(regions
                    .get(1)
                    .unwrap()
                    .up_urls(true)
                    .contains(&"https://up-jjh-z2.qiniup.com".into()));
                assert!(regions
                    .get(1)
                    .unwrap()
                    .up_urls(true)
                    .contains(&"https://upload-z2.qbox.me".into()));
            }));
        }

        threads.into_iter().for_each(|thread| thread.join().unwrap());
        assert_eq!(mock.call_called(), 1);

        Ok(())
    }

    #[test]
    fn test_storage_bucket_set_domain() -> Result<(), Box<dyn Error>> {
        let bucket = BucketBuilder::new(
            "test-bucket",
            get_credential(),
            ConfigBuilder::default()
                .http_request_call(Box::new(http::PanickedHTTPCaller("Should not call it")))
                .build()?,
        )
        .domain("abc.com")
        .domain("def.com")
        .build();
        assert_eq!(bucket.domains()?.len(), 2);
        assert_eq!(bucket.domains()?.first(), Some(&"abc.com"));
        Ok(())
    }

    #[test]
    fn test_storage_bucket_prequery_domain() -> Result<(), Box<dyn Error>> {
        let mock = CounterCallMock::new(JSONCallMock::new(200, Headers::new(), json!(["abc.com", "def.com"])));
        let bucket = BucketBuilder::new(
            "test-bucket",
            get_credential(),
            ConfigBuilder::default().http_request_call(mock.as_boxed()).build()?,
        )
        .auto_detect_domains()?
        .build();
        assert_eq!(mock.call_called(), 1);
        assert!(bucket.domains()?.contains(&"abc.com"));
        assert!(bucket.domains()?.contains(&"def.com"));
        assert_eq!(mock.call_called(), 1);
        Ok(())
    }

    #[test]
    fn test_storage_bucket_query_domain() -> Result<(), Box<dyn Error>> {
        let mock = CounterCallMock::new(JSONCallMock::new(200, Headers::new(), json!(["abc.com", "def.com"])));
        let bucket = Arc::new(
            BucketBuilder::new(
                "test-bucket",
                get_credential(),
                ConfigBuilder::default().http_request_call(mock.as_boxed()).build()?,
            )
            .build(),
        );
        assert_eq!(mock.call_called(), 0);

        let mut threads = Vec::with_capacity(3);
        {
            let bucket = bucket.clone();
            threads.push(thread::spawn(move || {
                assert!(bucket.domains().unwrap().contains(&"abc.com"));
            }));
        }

        {
            let bucket = bucket.clone();
            threads.push(thread::spawn(move || {
                assert!(bucket.domains().unwrap().contains(&"def.com"));
            }));
        }

        threads.into_iter().for_each(|thread| thread.join().unwrap());
        assert_eq!(mock.call_called(), 1);
        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
