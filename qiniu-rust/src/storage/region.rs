use crate::{config::Config, credential::Credential, http};
use derive_builder::Builder;
use getset::{CopyGetters, Getters};
use lazy_static::lazy_static;
use maplit::hashmap;
use qiniu_http::Result;
use serde::Deserialize;
use std::{borrow::Cow, collections::HashMap};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RegionId {
    Z0,
    Z1,
    Z2,
    AS0,
    NA0,
}

impl RegionId {
    pub fn as_str(&self) -> &'static str {
        match self {
            RegionId::Z0 => "z0",
            RegionId::Z1 => "z1",
            RegionId::Z2 => "z2",
            RegionId::AS0 => "as0",
            RegionId::NA0 => "na0",
        }
    }

    pub fn as_region(&self) -> &'static Region {
        match self {
            RegionId::Z0 => Region::z0(),
            RegionId::Z1 => Region::z1(),
            RegionId::Z2 => Region::z2(),
            RegionId::AS0 => Region::as0(),
            RegionId::NA0 => Region::na0(),
        }
    }
}

#[derive(Getters, CopyGetters, Builder, Clone, Debug)]
#[builder(pattern = "owned", setter(into, strip_option))]
pub struct Region {
    #[get_copy = "pub"]
    #[builder(default = "None")]
    region_id: Option<RegionId>,

    #[get = "pub"]
    up_http_urls: Vec<Cow<'static, str>>,

    #[get = "pub"]
    up_https_urls: Vec<Cow<'static, str>>,

    #[get = "pub"]
    io_http_urls: Vec<Cow<'static, str>>,

    #[get = "pub"]
    io_https_urls: Vec<Cow<'static, str>>,

    #[get_copy = "pub"]
    rs_http_url: &'static str,

    #[get_copy = "pub"]
    rs_https_url: &'static str,

    #[get_copy = "pub"]
    rsf_http_url: &'static str,

    #[get_copy = "pub"]
    rsf_https_url: &'static str,

    #[get_copy = "pub"]
    api_http_url: &'static str,

    #[get_copy = "pub"]
    api_https_url: &'static str,
}

impl Region {
    pub fn up_urls(&self, https: bool) -> Vec<&str> {
        if https {
            self.up_https_urls.iter().map(|url| url.as_ref()).collect()
        } else {
            self.up_http_urls.iter().map(|url| url.as_ref()).collect()
        }
    }

    pub fn io_urls(&self, https: bool) -> Vec<&str> {
        if https {
            self.io_https_urls.iter().map(|url| url.as_ref()).collect()
        } else {
            self.io_http_urls.iter().map(|url| url.as_ref()).collect()
        }
    }

    pub fn rs_url(&self, https: bool) -> &'static str {
        if https {
            self.rs_https_url
        } else {
            self.rs_http_url
        }
    }

    pub fn rsf_url(&self, https: bool) -> &'static str {
        if https {
            self.rsf_https_url
        } else {
            self.rsf_http_url
        }
    }

    pub fn api_url(&self, https: bool) -> &'static str {
        if https {
            self.api_https_url
        } else {
            self.api_http_url
        }
    }

    pub fn uc_url(https: bool) -> &'static str {
        if https {
            "https://uc.qbox.me"
        } else {
            "http://uc.qbox.me"
        }
    }

    pub fn fusion_url(https: bool) -> &'static str {
        if https {
            "https://fusion.qiniuapi.com"
        } else {
            "http://fusion.qiniuapi.com"
        }
    }

    pub fn pili_url(https: bool) -> &'static str {
        if https {
            "https://pili.qiniuapi.com"
        } else {
            "http://pili.qiniuapi.com"
        }
    }

    pub fn rtc_url(https: bool) -> &'static str {
        if https {
            "https://rtc.qiniuapi.com"
        } else {
            "http://rtc.qiniuapi.com"
        }
    }

    pub fn z0() -> &'static Region {
        &HUA_DONG
    }

    pub fn hua_dong() -> &'static Region {
        &HUA_DONG
    }

    pub fn east_china() -> &'static Region {
        &HUA_DONG
    }

    pub fn z1() -> &'static Region {
        &HUA_BEI
    }

    pub fn hua_bei() -> &'static Region {
        &HUA_BEI
    }

    pub fn north_china() -> &'static Region {
        &HUA_DONG
    }

    pub fn z2() -> &'static Region {
        &HUA_NAN
    }

    pub fn hua_nan() -> &'static Region {
        &HUA_NAN
    }

    pub fn south_china() -> &'static Region {
        &HUA_NAN
    }

    pub fn na0() -> &'static Region {
        &NORTH_AMERICA
    }

    pub fn north_america() -> &'static Region {
        &NORTH_AMERICA
    }

    pub fn as0() -> &'static Region {
        &SINGAPORE
    }

    pub fn singapore() -> &'static Region {
        &SINGAPORE
    }

    pub fn all() -> &'static [&'static Region] {
        &ALL_REGIONS[..]
    }

    pub fn query<B: Into<String>>(bucket: B, credential: &Credential, config: Config) -> Result<Vec<Region>> {
        let (access_key, uc_url) = (credential.access_key().to_owned(), Self::uc_url(config.use_https()));
        let result: RegionQueryResults = http::Client::new(config)
            .get("/v3/query", &[uc_url])
            .query("ak", access_key)
            .query("bucket", bucket)
            .accept_json()
            .no_body()
            .send()?
            .parse_json()?;
        Ok(result.into_regions())
    }
}

lazy_static! {
    static ref HUA_DONG: Region = RegionBuilder::default()
        .region_id(RegionId::Z0)
        .up_http_urls(vec![
            "http://upload.qiniup.com".into(),
            "http://up.qiniup.com".into(),
            "http://upload.qbox.me".into(),
            "http://up.qbox.me".into(),
        ])
        .up_https_urls(vec![
            "https://upload.qiniup.com".into(),
            "https://up.qiniup.com".into(),
            "https://upload.qbox.me".into(),
            "https://up.qbox.me".into(),
        ])
        .io_http_urls(vec!["http://iovip.qbox.me".into()])
        .io_https_urls(vec!["https://iovip.qbox.me".into()])
        .rs_http_url("http://rs.qiniu.com")
        .rs_https_url("https://rs.qbox.me")
        .rsf_http_url("http://rsf.qiniu.com")
        .rsf_https_url("https://rsf.qbox.me")
        .api_http_url("http://api.qiniu.com")
        .api_https_url("https://api.qiniu.com")
        .build()
        .unwrap();
    static ref HUA_BEI: Region = RegionBuilder::default()
        .region_id(RegionId::Z1)
        .up_http_urls(vec![
            "http://upload-z1.qiniup.com".into(),
            "http://up-z1.qiniup.com".into(),
            "http://upload-z1.qbox.me".into(),
            "http://up-z1.qbox.me".into()
        ])
        .up_https_urls(vec![
            "https://upload-z1.qiniup.com".into(),
            "https://up-z1.qiniup.com".into(),
            "https://upload-z1.qbox.me".into(),
            "https://up-z1.qbox.me".into()
        ])
        .io_http_urls(vec!["http://iovip-z1.qbox.me".into()])
        .io_https_urls(vec!["https://iovip-z1.qbox.me".into()])
        .rs_http_url("http://rs-z1.qiniu.com")
        .rs_https_url("https://rs-z1.qbox.me")
        .rsf_http_url("http://rsf-z1.qiniu.com")
        .rsf_https_url("https://rsf-z1.qbox.me")
        .api_http_url("http://api-z1.qiniu.com")
        .api_https_url("https://api-z1.qiniu.com")
        .build()
        .unwrap();
    static ref HUA_NAN: Region = RegionBuilder::default()
        .region_id(RegionId::Z2)
        .up_http_urls(vec![
            "http://upload-z2.qiniup.com".into(),
            "http://up-z2.qiniup.com".into(),
            "http://upload-z2.qbox.me".into(),
            "http://up-z2.qbox.me".into()
        ])
        .up_https_urls(vec![
            "https://upload-z2.qiniup.com".into(),
            "https://up-z2.qiniup.com".into(),
            "https://upload-z2.qbox.me".into(),
            "https://up-z2.qbox.me".into()
        ])
        .io_http_urls(vec!["http://iovip-z2.qbox.me".into()])
        .io_https_urls(vec!["https://iovip-z2.qbox.me".into()])
        .rs_http_url("http://rs-z2.qiniu.com")
        .rs_https_url("https://rs-z2.qbox.me")
        .rsf_http_url("http://rsf-z2.qiniu.com")
        .rsf_https_url("https://rsf-z2.qbox.me")
        .api_http_url("http://api-z2.qiniu.com")
        .api_https_url("https://api-z2.qiniu.com")
        .build()
        .unwrap();
    static ref NORTH_AMERICA: Region = RegionBuilder::default()
        .region_id(RegionId::NA0)
        .up_http_urls(vec![
            "http://upload-na0.qiniup.com".into(),
            "http://up-na0.qiniup.com".into(),
            "http://upload-na0.qbox.me".into(),
            "http://up-na0.qbox.me".into()
        ])
        .up_https_urls(vec![
            "https://upload-na0.qiniup.com".into(),
            "https://up-na0.qiniup.com".into(),
            "https://upload-na0.qbox.me".into(),
            "https://up-na0.qbox.me".into()
        ])
        .io_http_urls(vec!["http://iovip-na0.qbox.me".into()])
        .io_https_urls(vec!["https://iovip-na0.qbox.me".into()])
        .rs_http_url("http://rs-na0.qiniu.com")
        .rs_https_url("https://rs-na0.qbox.me")
        .rsf_http_url("http://rsf-na0.qiniu.com")
        .rsf_https_url("https://rsf-na0.qbox.me")
        .api_http_url("http://api-na0.qiniu.com")
        .api_https_url("https://api-na0.qiniu.com")
        .build()
        .unwrap();
    static ref SINGAPORE: Region = RegionBuilder::default()
        .region_id(RegionId::AS0)
        .up_http_urls(vec![
            "http://upload-as0.qiniup.com".into(),
            "http://up-as0.qiniup.com".into(),
            "http://upload-as0.qbox.me".into(),
            "http://up-as0.qbox.me".into()
        ])
        .up_https_urls(vec![
            "https://upload-as0.qiniup.com".into(),
            "https://up-as0.qiniup.com".into(),
            "https://upload-as0.qbox.me".into(),
            "https://up-as0.qbox.me".into()
        ])
        .io_http_urls(vec!["http://iovip-as0.qbox.me".into()])
        .io_https_urls(vec!["https://iovip-as0.qbox.me".into()])
        .rs_http_url("http://rs-as0.qiniu.com")
        .rs_https_url("https://rs-as0.qbox.me")
        .rsf_http_url("http://rsf-as0.qiniu.com")
        .rsf_https_url("https://rsf-as0.qbox.me")
        .api_http_url("http://api-as0.qiniu.com")
        .api_https_url("https://api-as0.qiniu.com")
        .build()
        .unwrap();
    static ref ALL_REGIONS: [&'static Region; 5] = [
        Region::hua_dong(),
        Region::hua_bei(),
        Region::hua_nan(),
        Region::north_america(),
        Region::singapore(),
    ];
    static ref INFER_DOMAINS_MAP: HashMap<&'static str, &'static Region> = {
        hashmap! {
            "iovip.qbox.me" => Region::hua_dong(),
            "iovip-z1.qbox.me" => Region::hua_bei(),
            "iovip-z2.qbox.me" => Region::hua_nan(),
            "iovip-na0.qbox.me" => Region::north_america(),
            "iovip-as0.qbox.me" => Region::singapore(),
        }
    };
}

#[derive(Deserialize, Debug, Clone)]
struct RegionQueryResults {
    hosts: Vec<RegionQueryResult>,
}

#[derive(Deserialize, Debug, Clone)]
struct RegionQueryResult {
    io: RegionQueryResultForIO,
    up: RegionQueryResultForUP,
}

#[derive(Deserialize, Debug, Clone)]
struct RegionQueryResultForIO {
    src: RegionQueryResultDomains,
}

#[derive(Deserialize, Debug, Clone)]
struct RegionQueryResultForUP {
    src: RegionQueryResultDomains,
    acc: RegionQueryResultDomains,
    old_src: RegionQueryResultDomains,
    old_acc: RegionQueryResultDomains,
}

#[derive(Deserialize, Debug, Clone)]
struct RegionQueryResultDomains {
    main: Vec<String>,
    backup: Option<Vec<String>>,
}

impl RegionQueryResults {
    fn into_regions(self) -> Vec<Region> {
        self.hosts
            .into_iter()
            .map(|host_result| host_result.into_region())
            .collect::<Vec<_>>()
    }
}

impl RegionQueryResult {
    fn into_region(self) -> Region {
        let infer_region = self
            .io
            .src
            .main
            .first()
            .and_then(|domain| INFER_DOMAINS_MAP.get(domain.as_str()).map(|&region| region))
            .unwrap_or_else(|| Region::hua_dong());
        RegionBuilder::default()
            .up_http_urls(
                [&self.up.acc, &self.up.src]
                    .into_iter()
                    .map(|domains| {
                        [Some(&domains.main), domains.backup.as_ref()]
                            .into_iter()
                            .filter_map(|&domains| domains)
                            .flatten()
                            .map(|domain| Cow::Owned("http://".to_owned() + domain))
                            .collect::<Vec<_>>()
                    })
                    .flatten()
                    .collect::<Vec<_>>(),
            )
            .up_https_urls(
                [&self.up.acc, &self.up.src, &self.up.old_acc, &self.up.old_src]
                    .into_iter()
                    .map(|domains| {
                        [Some(&domains.main), domains.backup.as_ref()]
                            .into_iter()
                            .filter_map(|&domains| domains)
                            .flatten()
                            .map(|domain| Cow::Owned("https://".to_owned() + domain))
                            .collect::<Vec<_>>()
                    })
                    .flatten()
                    .collect::<Vec<_>>(),
            )
            .io_http_urls(
                [Some(&self.io.src.main), self.io.src.backup.as_ref()]
                    .into_iter()
                    .filter_map(|&domains| domains)
                    .flatten()
                    .map(|domain| Cow::Owned("http://".to_owned() + domain))
                    .collect::<Vec<_>>(),
            )
            .io_https_urls(
                [Some(&self.io.src.main), self.io.src.backup.as_ref()]
                    .into_iter()
                    .filter_map(|&domains| domains)
                    .flatten()
                    .map(|domain| Cow::Owned("https://".to_owned() + domain))
                    .collect::<Vec<_>>(),
            )
            .rs_http_url(infer_region.rs_http_url())
            .rs_https_url(infer_region.rs_https_url())
            .rsf_http_url(infer_region.rsf_http_url())
            .rsf_https_url(infer_region.rsf_https_url())
            .api_http_url(infer_region.api_http_url())
            .api_https_url(infer_region.api_https_url())
            .build()
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigBuilder;
    use qiniu_http::Headers;
    use qiniu_test_utils::http_call_mock::JSONCallMock;
    use serde_json::json;
    use std::{boxed::Box, error::Error, result::Result};

    #[test]
    fn test_query_region_by_expected_domain() -> Result<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .http_request_call(Box::new(JSONCallMock::new(
                200,
                Headers::new(),
                json!({
                    "hosts":[{
                        "io": { "src": { "main": [ "iovip.qbox.me" ] } },
                        "up": {
                            "acc": { "backup": [ "upload-jjh.qiniup.com", "upload-xs.qiniup.com" ], "main": [ "upload.qiniup.com" ] },
                            "old_acc": { "info": "compatible to non-SNI device", "main": [ "upload.qbox.me" ] },
                            "old_src": { "info": "compatible to non-SNI device", "main": [ "up.qbox.me" ] },
                            "src": { "backup": [ "up-jjh.qiniup.com", "up-xs.qiniup.com" ], "main": [ "up.qiniup.com" ] }
                        }
                    }]
                }),
            )))
            .build()
            ?;
        let regions = Region::query("z0-bucket", &get_credential(), config)?;
        assert_eq!(regions.len(), 1);
        let region = regions.first().unwrap();
        assert_eq!(region.region_id(), None);
        assert_eq!(
            region.up_http_urls(),
            &[
                "http://upload.qiniup.com".to_string(),
                "http://upload-jjh.qiniup.com".to_string(),
                "http://upload-xs.qiniup.com".to_string(),
                "http://up.qiniup.com".to_string(),
                "http://up-jjh.qiniup.com".to_string(),
                "http://up-xs.qiniup.com".to_string(),
            ],
        );
        assert_eq!(
            region.up_https_urls(),
            &[
                "https://upload.qiniup.com".to_string(),
                "https://upload-jjh.qiniup.com".to_string(),
                "https://upload-xs.qiniup.com".to_string(),
                "https://up.qiniup.com".to_string(),
                "https://up-jjh.qiniup.com".to_string(),
                "https://up-xs.qiniup.com".to_string(),
                "https://upload.qbox.me".to_string(),
                "https://up.qbox.me".to_string(),
            ],
        );
        assert_eq!(region.io_http_urls(), &["http://iovip.qbox.me".to_string()]);
        assert_eq!(region.io_https_urls(), &["https://iovip.qbox.me".to_string()]);
        assert_eq!(region.rs_http_url(), "http://rs.qiniu.com");
        assert_eq!(region.rs_https_url(), "https://rs.qbox.me");
        assert_eq!(region.rsf_http_url(), "http://rsf.qiniu.com");
        assert_eq!(region.rsf_https_url(), "https://rsf.qbox.me");
        assert_eq!(region.api_http_url(), "http://api.qiniu.com");
        assert_eq!(region.api_https_url(), "https://api.qiniu.com");
        Ok(())
    }

    #[test]
    fn test_query_region_by_unexpected_domain() -> Result<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .http_request_call(Box::new(JSONCallMock::new(
                200,
                Headers::new(),
                json!({
                    "hosts": [{
                        "io": { "src": { "main": [ "iovip-z5.qbox.me" ] } },
                        "up": {
                            "acc": { "backup": [ "upload-jjh-z5.qiniup.com", "upload-xs-z5.qiniup.com" ], "main": [ "upload-z5.qiniup.com" ] },
                            "old_acc": { "info": "compatible to non-SNI device", "main": [ "upload-z5.qbox.me" ] },
                            "old_src": { "info": "compatible to non-SNI device", "main": [ "up-z5.qbox.me" ] },
                            "src": { "backup": [ "up-jjh-z5.qiniup.com", "up-xs-z5.qiniup.com" ], "main": [ "up-z5.qiniup.com" ] }
                        }
                    }]
                }),
            )))
            .build()?;
        let regions = Region::query("z5-bucket", &get_credential(), config)?;
        assert_eq!(regions.len(), 1);
        let region = regions.first().unwrap();
        assert_eq!(region.region_id(), None);
        assert_eq!(
            region.up_http_urls(),
            &[
                "http://upload-z5.qiniup.com".to_string(),
                "http://upload-jjh-z5.qiniup.com".to_string(),
                "http://upload-xs-z5.qiniup.com".to_string(),
                "http://up-z5.qiniup.com".to_string(),
                "http://up-jjh-z5.qiniup.com".to_string(),
                "http://up-xs-z5.qiniup.com".to_string(),
            ],
        );
        assert_eq!(
            region.up_https_urls(),
            &[
                "https://upload-z5.qiniup.com".to_string(),
                "https://upload-jjh-z5.qiniup.com".to_string(),
                "https://upload-xs-z5.qiniup.com".to_string(),
                "https://up-z5.qiniup.com".to_string(),
                "https://up-jjh-z5.qiniup.com".to_string(),
                "https://up-xs-z5.qiniup.com".to_string(),
                "https://upload-z5.qbox.me".to_string(),
                "https://up-z5.qbox.me".to_string(),
            ],
        );
        assert_eq!(region.io_http_urls(), &["http://iovip-z5.qbox.me".to_string()]);
        assert_eq!(region.io_https_urls(), &["https://iovip-z5.qbox.me".to_string()]);
        assert_eq!(region.rs_http_url(), "http://rs.qiniu.com");
        assert_eq!(region.rs_https_url(), "https://rs.qbox.me");
        assert_eq!(region.rsf_http_url(), "http://rsf.qiniu.com");
        assert_eq!(region.rsf_https_url(), "https://rsf.qbox.me");
        assert_eq!(region.api_http_url(), "http://api.qiniu.com");
        assert_eq!(region.api_https_url(), "https://api.qiniu.com");
        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
