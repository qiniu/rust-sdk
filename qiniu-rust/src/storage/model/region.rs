use crate::{config::Config, http::request, utils::auth::Auth};
use derive_builder::Builder;
use getset::{CopyGetters, Getters};
use lazy_static::lazy_static;
use maplit::hashmap;
use qiniu_http::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Getters, CopyGetters, Builder)]
#[builder(pattern = "owned", setter(into, strip_option))]
pub struct Region {
    #[get_copy = "pub"]
    #[builder(default = "None")]
    region_id: Option<&'static str>,

    #[get = "pub"]
    up_http_urls: Vec<String>,

    #[get = "pub"]
    up_https_urls: Vec<String>,

    #[get = "pub"]
    io_http_urls: Vec<String>,

    #[get = "pub"]
    io_https_urls: Vec<String>,

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
    pub fn up_urls(&self, https: bool) -> Vec<String> {
        if https {
            self.up_https_urls.to_owned()
        } else {
            self.up_http_urls.to_owned()
        }
    }

    pub fn io_urls(&self, https: bool) -> Vec<String> {
        if https {
            self.io_https_urls.to_owned()
        } else {
            self.io_http_urls.to_owned()
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

    pub fn z1() -> &'static Region {
        &HUA_BEI
    }

    pub fn hua_bei() -> &'static Region {
        &HUA_BEI
    }

    pub fn z2() -> &'static Region {
        &HUA_NAN
    }

    pub fn hua_nan() -> &'static Region {
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

    pub fn query<B: Into<String>>(bucket: B, auth: Auth, config: Config) -> Result<Region> {
        let (access_key, uc_url) = (auth.access_key().to_owned(), Self::uc_url(config.use_https()));
        let result: RegionQueryResult =
            request::Builder::new(auth, config, qiniu_http::Method::GET, "/v2/query", &[uc_url])
                .query("ak", access_key)
                .query("bucket", bucket)
                .accept_json()
                .no_body()
                .send()?
                .parse_json()
                .unwrap()?;

        let infer_region = result
            .io
            .src
            .main
            .first()
            .and_then(|domain| INFER_DOMAINS_MAP.get(domain.as_str()).map(|region| *region))
            .unwrap_or_else(|| Region::hua_dong());
        Ok(RegionBuilder::default()
            .up_http_urls(
                [&result.up.acc, &result.up.src]
                    .into_iter()
                    .map(|domains| {
                        [Some(&domains.main), domains.backup.as_ref()]
                            .into_iter()
                            .filter_map(|&domains| domains)
                            .flatten()
                            .map(|domain| "http://".to_owned() + domain)
                            .collect::<Vec<_>>()
                    })
                    .flatten()
                    .collect::<Vec<_>>(),
            )
            .up_https_urls(
                [&result.up.acc, &result.up.src, &result.up.old_acc, &result.up.old_src]
                    .into_iter()
                    .map(|domains| {
                        [Some(&domains.main), domains.backup.as_ref()]
                            .into_iter()
                            .filter_map(|&domains| domains)
                            .flatten()
                            .map(|domain| "https://".to_owned() + domain)
                            .collect::<Vec<_>>()
                    })
                    .flatten()
                    .collect::<Vec<_>>(),
            )
            .io_http_urls(
                [Some(&result.io.src.main), result.io.src.backup.as_ref()]
                    .into_iter()
                    .filter_map(|&domains| domains)
                    .flatten()
                    .map(|domain| "http://".to_owned() + domain)
                    .collect::<Vec<_>>(),
            )
            .io_https_urls(
                [Some(&result.io.src.main), result.io.src.backup.as_ref()]
                    .into_iter()
                    .filter_map(|&domains| domains)
                    .flatten()
                    .map(|domain| "https://".to_owned() + domain)
                    .collect::<Vec<_>>(),
            )
            .rs_http_url(infer_region.rs_http_url())
            .rs_https_url(infer_region.rs_https_url())
            .rsf_http_url(infer_region.rsf_http_url())
            .rsf_https_url(infer_region.rsf_https_url())
            .api_http_url(infer_region.api_http_url())
            .api_https_url(infer_region.api_https_url())
            .build()
            .unwrap())
    }
}

lazy_static! {
    static ref HUA_DONG: Region = RegionBuilder::default()
        .region_id("z0")
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
        .region_id("z1")
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
        .region_id("z2")
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
        .region_id("na0")
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
        .region_id("as0")
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RegionQueryResult {
    io: RegionQueryResultForIO,
    up: RegionQueryResultForUP,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RegionQueryResultForIO {
    src: RegionQueryResultDomains,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RegionQueryResultForUP {
    src: RegionQueryResultDomains,
    acc: RegionQueryResultDomains,
    old_src: RegionQueryResultDomains,
    old_acc: RegionQueryResultDomains,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RegionQueryResultDomains {
    main: Vec<String>,
    backup: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigBuilder;
    use qiniu_http::Headers;
    use qiniu_test_utils::http_call_mock::JSONCallMock;

    #[test]
    fn test_query_region_by_expected_domain() {
        let config = ConfigBuilder::default()
            .http_request_call(Box::new(JSONCallMock {
                status_code: 200,
                response_headers: Headers::new(),
                response_body: RegionQueryResult {
                    io: RegionQueryResultForIO {
                        src: RegionQueryResultDomains {
                            main: vec!["iovip.qbox.me".into()],
                            backup: None,
                        },
                    },
                    up: RegionQueryResultForUP {
                        src: RegionQueryResultDomains {
                            main: vec!["up.qiniup.com".into()],
                            backup: Some(vec!["up-jjh.qiniup.com".into(), "up-xs.qiniup.com".into()]),
                        },
                        acc: RegionQueryResultDomains {
                            main: vec!["upload.qiniup.com".into()],
                            backup: Some(vec!["upload-jjh.qiniup.com".into(), "upload-xs.qiniup.com".into()]),
                        },
                        old_src: RegionQueryResultDomains {
                            main: vec!["up.qbox.me".into()],
                            backup: None,
                        },
                        old_acc: RegionQueryResultDomains {
                            main: vec!["upload.qbox.me".into()],
                            backup: None,
                        },
                    },
                },
            }))
            .build()
            .unwrap();
        let region = Region::query("z0-bucket", get_auth(), config).unwrap();
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
    }

    #[test]
    fn test_query_region_by_unexpected_domain() {
        let config = ConfigBuilder::default()
            .http_request_call(Box::new(JSONCallMock {
                status_code: 200,
                response_headers: Headers::new(),
                response_body: RegionQueryResult {
                    io: RegionQueryResultForIO {
                        src: RegionQueryResultDomains {
                            main: vec!["iovip-z5.qbox.me".into()],
                            backup: None,
                        },
                    },
                    up: RegionQueryResultForUP {
                        src: RegionQueryResultDomains {
                            main: vec!["up-z5.qiniup.com".into()],
                            backup: Some(vec!["up-jjh-z5.qiniup.com".into(), "up-xs-z5.qiniup.com".into()]),
                        },
                        acc: RegionQueryResultDomains {
                            main: vec!["upload-z5.qiniup.com".into()],
                            backup: Some(vec![
                                "upload-jjh-z5.qiniup.com".into(),
                                "upload-xs-z5.qiniup.com".into(),
                            ]),
                        },
                        old_src: RegionQueryResultDomains {
                            main: vec!["up-z5.qbox.me".into()],
                            backup: None,
                        },
                        old_acc: RegionQueryResultDomains {
                            main: vec!["upload-z5.qbox.me".into()],
                            backup: None,
                        },
                    },
                },
            }))
            .build()
            .unwrap();
        let region = Region::query("z5-bucket", get_auth(), config).unwrap();
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
    }

    fn get_auth() -> Auth {
        Auth::new("abcdefghklmnopq", "1234567890")
    }
}
