use crate::{config::Config, http::request, utils::auth::Auth};
use derive_builder::Builder;
use getset::Getters;
use lazy_static::lazy_static;
use maplit::hashmap;
use qiniu_http::Result as HTTPResult;
use std::{collections::HashMap, sync::Arc};

#[derive(Getters, Builder)]
#[get = "pub"]
#[builder(pattern = "owned")]
pub struct Region {
    region_id: Option<&'static str>,
    up_http_urls: Vec<String>,
    up_https_urls: Vec<String>,
    io_http_urls: Vec<String>,
    io_https_urls: Vec<String>,
    rs_http_url: String,
    rs_https_url: String,
    rsf_http_url: String,
    rsf_https_url: String,
    api_http_url: String,
    api_https_url: String,
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

    pub fn rs_url(&self, https: bool) -> String {
        if https {
            self.rs_https_url.to_owned()
        } else {
            self.rs_http_url.to_owned()
        }
    }

    pub fn rsf_url(&self, https: bool) -> String {
        if https {
            self.rsf_https_url.to_owned()
        } else {
            self.rsf_http_url.to_owned()
        }
    }

    pub fn api_url(&self, https: bool) -> String {
        if https {
            self.api_https_url.to_owned()
        } else {
            self.api_http_url.to_owned()
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
}

lazy_static! {
    static ref HUA_DONG: Region = RegionBuilder::default()
        .region_id(Some("z0"))
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
        .rs_http_url("http://rs.qiniu.com".into())
        .rs_https_url("https://rs.qbox.me".into())
        .rsf_http_url("http://rsf.qiniu.com".into())
        .rsf_https_url("https://rsf.qbox.me".into())
        .api_http_url("http://api.qiniu.com".into())
        .api_https_url("https://api.qiniu.com".into())
        .build()
        .unwrap();
    static ref HUA_BEI: Region = RegionBuilder::default()
        .region_id(Some("z1"))
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
        .rs_http_url("http://rs-z1.qiniu.com".into())
        .rs_https_url("https://rs-z1.qbox.me".into())
        .rsf_http_url("http://rsf-z1.qiniu.com".into())
        .rsf_https_url("https://rsf-z1.qbox.me".into())
        .api_http_url("http://api-z1.qiniu.com".into())
        .api_https_url("https://api-z1.qiniu.com".into())
        .build()
        .unwrap();
    static ref HUA_NAN: Region = RegionBuilder::default()
        .region_id(Some("z2"))
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
        .rs_http_url("http://rs-z2.qiniu.com".into())
        .rs_https_url("https://rs-z2.qbox.me".into())
        .rsf_http_url("http://rsf-z2.qiniu.com".into())
        .rsf_https_url("https://rsf-z2.qbox.me".into())
        .api_http_url("http://api-z2.qiniu.com".into())
        .api_https_url("https://api-z2.qiniu.com".into())
        .build()
        .unwrap();
    static ref NORTH_AMERICA: Region = RegionBuilder::default()
        .region_id(Some("na0"))
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
        .rs_http_url("http://rs-na0.qiniu.com".into())
        .rs_https_url("https://rs-na0.qbox.me".into())
        .rsf_http_url("http://rsf-na0.qiniu.com".into())
        .rsf_https_url("https://rsf-na0.qbox.me".into())
        .api_http_url("http://api-na0.qiniu.com".into())
        .api_https_url("https://api-na0.qiniu.com".into())
        .build()
        .unwrap();
    static ref SINGAPORE: Region = RegionBuilder::default()
        .region_id(Some("as0"))
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
        .rs_http_url("http://rs-as0.qiniu.com".into())
        .rs_https_url("https://rs-as0.qbox.me".into())
        .rsf_http_url("http://rsf-as0.qiniu.com".into())
        .rsf_https_url("https://rsf-as0.qbox.me".into())
        .api_http_url("http://api-as0.qiniu.com".into())
        .api_https_url("https://api-as0.qiniu.com".into())
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
