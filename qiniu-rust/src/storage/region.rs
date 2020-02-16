//! 区域模块
//!
//! 区域存储七牛不同公有云区域的域名，以及提供定制私有云区域的接口
use crate::{
    config::Config,
    http::{Client, Result},
};
use assert_impl::assert_impl;
use derive_builder::Builder;
use getset::{CopyGetters, Getters};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::{borrow::Cow, convert::AsRef};

/// 存储区域 ID
///
/// 枚举类，仅包含七牛公有云的所有存储区域 ID。
/// 对于私有云，则应采用其他方案替代
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RegionId {
    /// 华东区区域 ID
    Z0,
    /// 华北区区域 ID
    Z1,
    /// 华南区区域 ID
    Z2,
    /// 东南亚地区区域 ID
    AS0,
    /// 北美地区区域 ID
    NA0,
}

impl RegionId {
    /// 获取区域 ID 的字符串
    pub fn as_str(self) -> &'static str {
        match self {
            RegionId::Z0 => "z0",
            RegionId::Z1 => "z1",
            RegionId::Z2 => "z2",
            RegionId::AS0 => "as0",
            RegionId::NA0 => "na0",
        }
    }

    /// 根据区域 ID 获取区域实例
    ///
    /// 仅对公有云区域有效，对于私有云，则应该自行构建区域实例
    pub fn as_region(self) -> &'static Region {
        match self {
            RegionId::Z0 => Region::z0(),
            RegionId::Z1 => Region::z1(),
            RegionId::Z2 => Region::z2(),
            RegionId::AS0 => Region::as0(),
            RegionId::NA0 => Region::na0(),
        }
    }
}

impl AsRef<str> for RegionId {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// 区域
///
/// 区域实例负责管理七牛多个服务器的 URL，用于为存储管理器或上传管理器提供 URL。
#[derive(Getters, CopyGetters, Builder, Clone, Debug, Default)]
#[builder(default, pattern = "owned", setter(into))]
pub struct Region {
    /// 存储区域 ID
    ///
    /// 需要注意：通过七牛服务器查询获得的区域实例，`region_id` 将会返回 `None`
    #[get_copy = "pub"]
    region_id: Option<RegionId>,

    /// 上传服务器 URL 列表（HTTP 协议）
    #[get = "pub"]
    up_http_urls: Vec<Cow<'static, str>>,

    /// 上传服务器 URL 列表（HTTPS 协议）
    #[get = "pub"]
    up_https_urls: Vec<Cow<'static, str>>,

    /// IO 服务器 URL 列表（HTTP 协议）
    #[get = "pub"]
    io_http_urls: Vec<Cow<'static, str>>,

    /// IO 服务器 URL 列表（HTTPS 协议）
    #[get = "pub"]
    io_https_urls: Vec<Cow<'static, str>>,

    /// RS 服务器 URL 列表（HTTP 协议）
    #[get = "pub"]
    rs_http_urls: Vec<Cow<'static, str>>,

    /// RS 服务器 URL 列表（HTTPS 协议）
    #[get = "pub"]
    rs_https_urls: Vec<Cow<'static, str>>,

    /// RSF 服务器 URL 列表（HTTP 协议）
    #[get = "pub"]
    rsf_http_urls: Vec<Cow<'static, str>>,

    /// RSF 服务器 URL 列表（HTTPS 协议）
    #[get = "pub"]
    rsf_https_urls: Vec<Cow<'static, str>>,

    /// API 服务器 URL 列表（HTTP 协议）
    #[get = "pub"]
    api_http_urls: Vec<Cow<'static, str>>,

    /// API 服务器 URL 列表（HTTPS 协议）
    #[get = "pub"]
    api_https_urls: Vec<Cow<'static, str>>,
}

impl Region {
    /// 获取上传服务器 URL 列表
    pub fn up_urls_owned(&self, https: bool) -> Vec<Cow<'static, str>> {
        if https {
            self.up_https_urls.clone()
        } else {
            self.up_http_urls.clone()
        }
    }

    /// 获取上传服务器 URL 引用的列表
    pub fn up_urls_ref(&self, https: bool) -> Vec<&str> {
        if https {
            self.up_https_urls.iter().map(|url| url.as_ref()).collect()
        } else {
            self.up_http_urls.iter().map(|url| url.as_ref()).collect()
        }
    }

    /// 获取 IO 服务器 URL 列表
    pub fn io_urls_owned(&self, https: bool) -> Vec<Cow<'static, str>> {
        if https {
            self.io_https_urls.clone()
        } else {
            self.io_http_urls.clone()
        }
    }

    /// 获取 IO 服务器 URL 引用的列表
    pub fn io_urls_ref(&self, https: bool) -> Vec<&str> {
        if https {
            self.io_https_urls.iter().map(|url| url.as_ref()).collect()
        } else {
            self.io_http_urls.iter().map(|url| url.as_ref()).collect()
        }
    }

    /// 获取 RS 服务器 URL 列表
    pub fn rs_urls_owned(&self, https: bool) -> Vec<Cow<'static, str>> {
        if https {
            self.rs_https_urls.clone()
        } else {
            self.rs_http_urls.clone()
        }
    }

    /// 获取 RS 服务器 URL 引用的列表
    pub fn rs_urls_ref(&self, https: bool) -> Vec<&str> {
        if https {
            self.rs_https_urls.iter().map(|url| url.as_ref()).collect()
        } else {
            self.rs_http_urls.iter().map(|url| url.as_ref()).collect()
        }
    }

    /// 获取 RSF 服务器 URL 列表
    pub fn rsf_urls_owned(&self, https: bool) -> Vec<Cow<'static, str>> {
        if https {
            self.rsf_https_urls.clone()
        } else {
            self.rsf_http_urls.clone()
        }
    }

    /// 获取 RSF 服务器 URL 引用的列表
    pub fn rsf_urls_ref(&self, https: bool) -> Vec<&str> {
        if https {
            self.rsf_https_urls.iter().map(|url| url.as_ref()).collect()
        } else {
            self.rsf_http_urls.iter().map(|url| url.as_ref()).collect()
        }
    }

    /// 获取 API 服务器 URL 列表
    pub fn api_urls_owned(&self, https: bool) -> Vec<Cow<'static, str>> {
        if https {
            self.api_https_urls.clone()
        } else {
            self.api_http_urls.clone()
        }
    }

    /// 获取 API 服务器 URL 引用的列表
    pub fn api_urls_ref(&self, https: bool) -> Vec<&str> {
        if https {
            self.api_https_urls.iter().map(|url| url.as_ref()).collect()
        } else {
            self.api_http_urls.iter().map(|url| url.as_ref()).collect()
        }
    }

    /// 获取华东区公有云区域实例
    pub fn z0() -> &'static Region {
        &HUA_DONG
    }

    /// 获取华东区公有云区域实例
    pub fn hua_dong() -> &'static Region {
        &HUA_DONG
    }

    /// 获取华东区公有云区域实例
    pub fn east_china() -> &'static Region {
        &HUA_DONG
    }

    /// 获取华北区公有云区域实例
    pub fn z1() -> &'static Region {
        &HUA_BEI
    }

    /// 获取华北区公有云区域实例
    pub fn hua_bei() -> &'static Region {
        &HUA_BEI
    }

    /// 获取华北区公有云区域实例
    pub fn north_china() -> &'static Region {
        &HUA_BEI
    }

    /// 获取华南区公有云区域实例
    pub fn z2() -> &'static Region {
        &HUA_NAN
    }

    /// 获取华南区公有云区域实例
    pub fn hua_nan() -> &'static Region {
        &HUA_NAN
    }

    /// 获取华南区公有云区域实例
    pub fn south_china() -> &'static Region {
        &HUA_NAN
    }

    /// 获取北美地区公有云区域实例
    pub fn na0() -> &'static Region {
        &NORTH_AMERICA
    }

    /// 获取东南亚地区公有云区域实例
    pub fn as0() -> &'static Region {
        &SINGAPORE
    }

    /// 获取所有公有云区域实例
    pub fn all() -> &'static [&'static Region] {
        &ALL_REGIONS[..]
    }

    /// 查询七牛服务器，根据存储空间名称获取区域列表
    pub fn query<'a>(
        bucket: impl Into<Cow<'a, str>>,
        access_key: impl Into<Cow<'a, str>>,
        config: Config,
    ) -> Result<Box<[Region]>> {
        let uc_url = config.uc_url();
        let result: RegionQueryResults = Client::new(config)
            .get("/v3/query", &[&uc_url])
            .query("ak", access_key.into())
            .query("bucket", bucket.into())
            .accept_json()
            .no_body()
            .send()?
            .parse_json()?;
        Ok(result.into_regions())
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl From<Region> for Cow<'_, Region> {
    fn from(region: Region) -> Self {
        Cow::Owned(region)
    }
}

impl<'a> From<&'a Region> for Cow<'a, Region> {
    fn from(region: &'a Region) -> Self {
        Cow::Borrowed(region)
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
        .rs_http_urls(vec!["http://rs.qiniu.com".into()])
        .rs_https_urls(vec!["https://rs.qbox.me".into()])
        .rsf_http_urls(vec!["http://rsf.qiniu.com".into()])
        .rsf_https_urls(vec!["https://rsf.qbox.me".into()])
        .api_http_urls(vec!["http://api.qiniu.com".into()])
        .api_https_urls(vec!["https://api.qiniu.com".into()])
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
        .rs_http_urls(vec!["http://rs-z1.qiniu.com".into()])
        .rs_https_urls(vec!["https://rs-z1.qbox.me".into()])
        .rsf_http_urls(vec!["http://rsf-z1.qiniu.com".into()])
        .rsf_https_urls(vec!["https://rsf-z1.qbox.me".into()])
        .api_http_urls(vec!["http://api-z1.qiniu.com".into()])
        .api_https_urls(vec!["https://api-z1.qiniu.com".into()])
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
        .rs_http_urls(vec!["http://rs-z2.qiniu.com".into()])
        .rs_https_urls(vec!["https://rs-z2.qbox.me".into()])
        .rsf_http_urls(vec!["http://rsf-z2.qiniu.com".into()])
        .rsf_https_urls(vec!["https://rsf-z2.qbox.me".into()])
        .api_http_urls(vec!["http://api-z2.qiniu.com".into()])
        .api_https_urls(vec!["https://api-z2.qiniu.com".into()])
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
        .rs_http_urls(vec!["http://rs-na0.qiniu.com".into()])
        .rs_https_urls(vec!["https://rs-na0.qbox.me".into()])
        .rsf_http_urls(vec!["http://rsf-na0.qiniu.com".into()])
        .rsf_https_urls(vec!["https://rsf-na0.qbox.me".into()])
        .api_http_urls(vec!["http://api-na0.qiniu.com".into()])
        .api_https_urls(vec!["https://api-na0.qiniu.com".into()])
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
        .rs_http_urls(vec!["http://rs-as0.qiniu.com".into()])
        .rs_https_urls(vec!["https://rs-as0.qbox.me".into()])
        .rsf_http_urls(vec!["http://rsf-as0.qiniu.com".into()])
        .rsf_https_urls(vec!["https://rsf-as0.qbox.me".into()])
        .api_http_urls(vec!["http://api-as0.qiniu.com".into()])
        .api_https_urls(vec!["https://api-as0.qiniu.com".into()])
        .build()
        .unwrap();
    static ref ALL_REGIONS: [&'static Region; 5] =
        [Region::z0(), Region::z1(), Region::z2(), Region::na0(), Region::as0()];
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
    fn into_regions(self) -> Box<[Region]> {
        self.hosts
            .into_iter()
            .map(|host_result| host_result.into_region())
            .collect()
    }
}

impl RegionQueryResult {
    fn into_region(self) -> Region {
        RegionBuilder::default()
            .up_http_urls(
                [&self.up.acc, &self.up.src]
                    .iter()
                    .map(|domains| {
                        [Some(&domains.main), domains.backup.as_ref()]
                            .iter()
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
                    .iter()
                    .map(|domains| {
                        [Some(&domains.main), domains.backup.as_ref()]
                            .iter()
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
                    .iter()
                    .filter_map(|&domains| domains)
                    .flatten()
                    .map(|domain| Cow::Owned("http://".to_owned() + domain))
                    .collect::<Vec<_>>(),
            )
            .io_https_urls(
                [Some(&self.io.src.main), self.io.src.backup.as_ref()]
                    .iter()
                    .filter_map(|&domains| domains)
                    .flatten()
                    .map(|domain| Cow::Owned("https://".to_owned() + domain))
                    .collect::<Vec<_>>(),
            )
            // TODO: Add rs, rsf, api URLs here
            .build()
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ConfigBuilder,
        credential::Credential,
        http::{DomainsManagerBuilder, Headers},
    };
    use qiniu_test_utils::http_call_mock::JSONCallMock;
    use serde_json::json;
    use std::{boxed::Box, error::Error, result::Result};

    #[test]
    fn test_query_region_by_expected_domain() -> Result<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .http_request_handler(JSONCallMock::new(
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
            )).build();
        let regions = Region::query("z0-bucket", get_credential().access_key(), config)?;
        assert_eq!(regions.len(), 1);
        let region = regions.first().unwrap();
        assert_eq!(region.region_id(), None);
        assert_eq!(
            region.up_http_urls(),
            &[
                "http://upload.qiniup.com".to_owned(),
                "http://upload-jjh.qiniup.com".to_owned(),
                "http://upload-xs.qiniup.com".to_owned(),
                "http://up.qiniup.com".to_owned(),
                "http://up-jjh.qiniup.com".to_owned(),
                "http://up-xs.qiniup.com".to_owned(),
            ],
        );
        assert_eq!(
            region.up_https_urls(),
            &[
                "https://upload.qiniup.com".to_owned(),
                "https://upload-jjh.qiniup.com".to_owned(),
                "https://upload-xs.qiniup.com".to_owned(),
                "https://up.qiniup.com".to_owned(),
                "https://up-jjh.qiniup.com".to_owned(),
                "https://up-xs.qiniup.com".to_owned(),
                "https://upload.qbox.me".to_owned(),
                "https://up.qbox.me".to_owned(),
            ],
        );
        assert_eq!(region.io_http_urls(), &["http://iovip.qbox.me".to_owned()]);
        assert_eq!(region.io_https_urls(), &["https://iovip.qbox.me".to_owned()]);
        assert!(region.rs_http_urls().is_empty());
        assert!(region.rs_https_urls().is_empty());
        assert!(region.rsf_http_urls().is_empty());
        assert!(region.rsf_https_urls().is_empty());
        assert!(region.api_http_urls().is_empty());
        assert!(region.api_https_urls().is_empty());
        Ok(())
    }

    #[test]
    fn test_query_region_by_unexpected_domain() -> Result<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
            .http_request_handler(JSONCallMock::new(
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
            ))
            .build();
        let regions = Region::query("z5-bucket", get_credential().access_key(), config)?;
        assert_eq!(regions.len(), 1);
        let region = regions.first().unwrap();
        assert_eq!(region.region_id(), None);
        assert_eq!(
            region.up_http_urls(),
            &[
                "http://upload-z5.qiniup.com".to_owned(),
                "http://upload-jjh-z5.qiniup.com".to_owned(),
                "http://upload-xs-z5.qiniup.com".to_owned(),
                "http://up-z5.qiniup.com".to_owned(),
                "http://up-jjh-z5.qiniup.com".to_owned(),
                "http://up-xs-z5.qiniup.com".to_owned(),
            ],
        );
        assert_eq!(
            region.up_https_urls(),
            &[
                "https://upload-z5.qiniup.com".to_owned(),
                "https://upload-jjh-z5.qiniup.com".to_owned(),
                "https://upload-xs-z5.qiniup.com".to_owned(),
                "https://up-z5.qiniup.com".to_owned(),
                "https://up-jjh-z5.qiniup.com".to_owned(),
                "https://up-xs-z5.qiniup.com".to_owned(),
                "https://upload-z5.qbox.me".to_owned(),
                "https://up-z5.qbox.me".to_owned(),
            ],
        );
        assert_eq!(region.io_http_urls(), &["http://iovip-z5.qbox.me".to_owned()]);
        assert_eq!(region.io_https_urls(), &["https://iovip-z5.qbox.me".to_owned()]);
        assert!(region.rs_http_urls().is_empty());
        assert!(region.rs_https_urls().is_empty());
        assert!(region.rsf_http_urls().is_empty());
        assert!(region.rsf_https_urls().is_empty());
        assert!(region.api_http_urls().is_empty());
        assert!(region.api_https_urls().is_empty());
        Ok(())
    }

    fn get_credential() -> Credential {
        Credential::new("abcdefghklmnopq", "1234567890")
    }
}
