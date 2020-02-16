//! 存储空间模块

use super::{
    region::{Region, RegionId},
    uploader::{BucketUploaderBuilder, UploadManager},
};
use crate::{
    credential::Credential,
    http::{Client, Result},
};
use assert_impl::assert_impl;
use once_cell::sync::OnceCell;
use std::{borrow::Cow, iter::Iterator};

/// 存储空间
///
/// 封装存储空间相关功能
pub struct Bucket<'r> {
    name: Cow<'r, str>,
    credential: Cow<'r, Credential>,
    upload_manager: UploadManager,
    region: OnceCell<Cow<'r, Region>>,
    backup_regions: OnceCell<Box<[Region]>>,
    domains: OnceCell<Box<[Cow<'r, str>]>>,
    http_client: Client,
}

/// 存储空间生成器
///
/// 注意，该结构体仅用于在 SDK 中配置生成存储空间实例，而非在七牛云服务器上创建新的存储空间。
/// 事实上，除非您使用了私有云，或七牛以外的 CDN 服务商，否则您总是可以直接构建存储空间，存储空间为以懒加载的方式从七牛服务器获取区域信息和下载域名，SDK 确保懒加载的线程安全。
///
/// ```rust,no_run
/// use qiniu_ng::{Client, Config};
/// # use std::{result::Result, error::Error};
///
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let client = Client::new("[Access Key]", "[Secret Key]", Config::default());
/// let bucket = client.storage().bucket("[Bucket name]").build();
/// # Ok(())
/// # }
/// ```
pub struct BucketBuilder<'r> {
    name: Cow<'r, str>,
    credential: Cow<'r, Credential>,
    upload_manager: UploadManager,
    region: Option<Cow<'r, Region>>,
    backup_regions: Option<Box<[Region]>>,
    domains: Option<Vec<Cow<'r, str>>>,
    http_client: Client,
}

/// 存储空间区域迭代器
pub struct BucketRegionIter<'a, 'r: 'a> {
    bucket: &'a Bucket<'r>,
    itered: usize,
}

impl<'r> BucketBuilder<'r> {
    pub(crate) fn new(
        name: Cow<'r, str>,
        credential: Cow<'r, Credential>,
        upload_manager: UploadManager,
    ) -> BucketBuilder<'r> {
        BucketBuilder {
            name,
            credential,
            http_client: Client::new(upload_manager.config().clone()),
            upload_manager,
            region: None,
            backup_regions: None,
            domains: None,
        }
    }

    /// 指定存储空间区域
    ///
    /// 注意：该方法不要与 `region_id` 方法连用
    pub fn region(mut self, region: impl Into<Cow<'r, Region>>) -> BucketBuilder<'r> {
        self.region = Some(region.into());
        self
    }

    /// 指定存储空间备用区域
    ///
    /// 注意，应该首先设置存储空间区域，然后再设置备用区域。
    /// 如果只设置备用区域而不设置存储空间区域
    pub fn backup_regions(mut self, regions: impl Into<Vec<Region>>) -> BucketBuilder<'r> {
        self.backup_regions = Some(regions.into().into_boxed_slice());
        self
    }

    /// 指定存储空间 ID
    ///
    /// 该方法仅适用于指定七牛公有云区域。
    /// 如果使用的是私有云，则请调用 `region` 方法。该方法不要与 `region` 方法连用。
    pub fn region_id(self, region_id: RegionId) -> BucketBuilder<'r> {
        self.region(Cow::Borrowed(region_id.as_region()))
    }

    /// 自动检测区域
    ///
    /// 将连接七牛服务器查询当前存储空间所在区域和备用区域
    ///
    /// 注意，如果调用了该方法，则不应该再调用 `region`，`backup_regions` 和 `region_id` 方法。
    /// 除非有特殊需求，否则不建议您调用该方法，而是尽量使用懒加载的方式在必要时自动检测区域
    pub fn auto_detect_region(mut self) -> Result<BucketBuilder<'r>> {
        let mut regions: Vec<Region> = Region::query(
            self.name.as_ref(),
            self.credential.access_key(),
            self.upload_manager.config().clone(),
        )?
        .into();
        self.region = Some(Cow::Owned(regions.swap_remove(0)));
        if !regions.is_empty() {
            self.backup_regions = Some(regions.into());
        }
        Ok(self)
    }

    /// 新增下载域名
    ///
    /// 注意，可以先调用 `auto_detect_domains` 方法然后再调用该方法，SDK 将优先使用最后新增的域名
    pub fn prepend_domain(mut self, domain: impl Into<Cow<'r, str>>) -> BucketBuilder<'r> {
        match &mut self.domains {
            Some(domains) => {
                domains.push(domain.into());
            }
            None => {
                self.domains = Some(vec![domain.into()]);
            }
        }
        self
    }

    /// 自动检测下载域名
    ///
    /// 将连接七牛服务器查询当前存储空间的下载域名列表
    pub fn auto_detect_domains(mut self) -> Result<BucketBuilder<'r>> {
        self.domains = Some(
            domain::query(&self.http_client, &self.credential, self.name.as_ref())?
                .into_iter()
                .map(Cow::Owned)
                .collect(),
        );
        Ok(self)
    }

    /// 生成存储空间
    ///
    /// 注意，该方法仅用于在 SDK 中配置生成存储空间实例，而非在七牛云服务器上创建新的存储空间
    pub fn build(self) -> Bucket<'r> {
        Bucket {
            name: self.name,
            credential: self.credential,
            upload_manager: self.upload_manager,
            http_client: self.http_client,
            region: self.region.map(OnceCell::from).unwrap_or_else(OnceCell::new),
            backup_regions: self.backup_regions.map(OnceCell::from).unwrap_or_else(OnceCell::new),
            domains: self
                .domains
                .map(|mut domains| {
                    domains.reverse(); // 反转 domains，后 prepend 的放到开头
                    OnceCell::from(domains.into_boxed_slice())
                })
                .unwrap_or_else(OnceCell::new),
        }
    }
}

impl<'r> Bucket<'r> {
    /// 存储空间名称
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// 存储空间区域
    ///
    /// 如果区域在存储空间生成前未指定，则该方法可能会连接七牛服务器查询当前存储空间所在区域
    pub fn region(&self) -> Result<&Region> {
        self.region
            .get_or_try_init(|| {
                let mut regions: Vec<Region> = Region::query(
                    self.name(),
                    self.credential.access_key(),
                    self.upload_manager.config().clone(),
                )?
                .into();
                let first_region = Cow::Owned(regions.swap_remove(0));
                self.backup_regions.get_or_init(|| regions.into());
                Ok(first_region)
            })
            .map(|region| region.as_ref())
    }

    /// 存储空间区域迭代器
    ///
    /// 该迭代器将首先返回当前存储空间所在区域，随后返回所有备用区域
    ///
    /// 如果区域在存储空间生成前未指定，则该方法可能会连接七牛服务器查询当前存储空间所在区域和备用区域
    pub fn regions<'a>(&'a self) -> Result<BucketRegionIter<'a, 'r>> {
        self.region()?;
        Ok(BucketRegionIter {
            bucket: self,
            itered: 0,
        })
    }

    /// 存储空间下载域名列表
    ///
    /// 如果下载域名在存储空间生成前未指定，则该方法可能会连接七牛服务器查询当前存储空间下载域名列表
    pub fn domains(&self) -> Result<Vec<&str>> {
        let domains = self.domains.get_or_try_init(|| {
            Ok(domain::query(&self.http_client, &self.credential, self.name())?
                .into_iter()
                .map(Cow::Owned)
                .collect())
        })?;
        Ok(domains.iter().map(|domain| domain.as_ref()).collect())
    }

    /// 获取当前存储空间上传生成器
    pub fn uploader(&self) -> BucketUploaderBuilder {
        self.upload_manager.for_bucket(self)
    }

    fn rs_urls(&self) -> Vec<Cow<'static, str>> {
        let mut rs_urls = self
            .region()
            .map(|region| region.rs_urls_owned(self.upload_manager.config().use_https()))
            .unwrap_or_else(|_| Vec::new());
        rs_urls.push(Cow::Owned(self.upload_manager.config().rs_url()));
        rs_urls
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

mod domain {
    use crate::{
        credential::Credential,
        http::{Client, Result, TokenVersion},
    };
    use std::borrow::Borrow;

    pub(super) fn query(http_client: &Client, credential: &Credential, bucket_name: &str) -> Result<Vec<String>> {
        Ok(http_client
            .get("/v6/domain/list", &[&http_client.config().api_url()])
            .query("tbl", bucket_name)
            .token(TokenVersion::V2, credential.borrow().into())
            .no_body()
            .send()?
            .parse_json()?)
    }
}

impl<'a, 'r: 'a> Iterator for BucketRegionIter<'a, 'r> {
    type Item = &'a Region;

    fn next(&mut self) -> Option<Self::Item> {
        if self.itered == 0 {
            self.bucket.region.get().map(|region| {
                self.itered += 1;
                region.as_ref()
            })
        } else {
            self.bucket.backup_regions.get().and_then(|regions| {
                self.itered += 1;
                regions.get(self.itered - 2)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{region::RegionId, uploader::UploadManager},
        *,
    };
    use crate::{
        config::ConfigBuilder,
        credential::Credential,
        http::{DomainsManagerBuilder, Headers, PanickedHTTPCaller},
    };
    use qiniu_test_utils::http_call_mock::{CounterCallMock, JSONCallMock};
    use serde_json::json;
    use std::{boxed::Box, error::Error, result::Result, sync::Arc, thread};

    #[test]
    fn test_storage_bucket_set_region() -> Result<(), Box<dyn Error>> {
        let bucket = BucketBuilder::new(
            "test-bucket".into(),
            get_credential().into(),
            UploadManager::new(
                ConfigBuilder::default()
                    .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
                    .http_request_handler(PanickedHTTPCaller("Should not call it"))
                    .build(),
            ),
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
    fn test_storage_bucket_set_region_id() -> Result<(), Box<dyn Error>> {
        let bucket = BucketBuilder::new(
            "test-bucket".into(),
            get_credential().into(),
            UploadManager::new(
                ConfigBuilder::default()
                    .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
                    .http_request_handler(PanickedHTTPCaller("Should not call it"))
                    .build(),
            ),
        )
        .region_id(RegionId::Z2)
        .build();
        assert_eq!(bucket.region()?.region_id(), Some(RegionId::Z2));
        let regions = bucket.regions()?.collect::<Vec<_>>();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions.first().unwrap().region_id(), Some(RegionId::Z2));
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
            "test-bucket".into(),
            get_credential().into(),
            UploadManager::new(
                ConfigBuilder::default()
                    .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
                    .http_request_handler(mock.clone())
                    .build(),
            ),
        )
        .auto_detect_region()?
        .build();
        assert_eq!(mock.call_called(), 1);

        let region = bucket.region()?;
        assert!(region.up_urls_ref(true).contains(&"https://up-xs.qiniup.com"));
        assert!(region
            .up_urls_owned(true)
            .contains(&Cow::Borrowed("https://up-xs.qiniup.com")));
        assert!(region.up_urls_ref(true).contains(&"https://up-jjh.qiniup.com"));
        assert!(region
            .up_urls_owned(true)
            .contains(&Cow::Borrowed("https://up-jjh.qiniup.com")));
        assert!(region.up_urls_ref(true).contains(&"https://upload.qbox.me"));
        assert!(region
            .up_urls_owned(true)
            .contains(&Cow::Borrowed("https://upload.qbox.me")));

        let regions = bucket.regions()?.collect::<Vec<_>>();
        assert_eq!(regions.len(), 2);
        assert!(regions
            .get(1)
            .unwrap()
            .up_urls_ref(true)
            .contains(&"https://up-xs-z1.qiniup.com"));
        assert!(regions
            .get(1)
            .unwrap()
            .up_urls_ref(true)
            .contains(&"https://up-jjh-z1.qiniup.com"));
        assert!(regions
            .get(1)
            .unwrap()
            .up_urls_ref(true)
            .contains(&"https://upload-z1.qbox.me"));

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
                "test-bucket".into(),
                get_credential().into(),
                UploadManager::new(
                    ConfigBuilder::default()
                        .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
                        .http_request_handler(mock.clone())
                        .build(),
                ),
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
                    .up_urls_ref(true)
                    .contains(&"https://up-xs.qiniup.com"));
            }));
        }

        {
            let bucket = bucket.clone();
            threads.push(thread::spawn(move || {
                assert!(bucket
                    .region()
                    .unwrap()
                    .up_urls_ref(true)
                    .contains(&"https://up-jjh.qiniup.com"));
            }));
        }

        {
            let bucket = bucket.clone();
            threads.push(thread::spawn(move || {
                assert!(bucket
                    .region()
                    .unwrap()
                    .up_urls_ref(true)
                    .contains(&"https://upload.qbox.me"));
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
                    .up_urls_ref(true)
                    .contains(&"https://up-xs-z2.qiniup.com"));
                assert!(regions
                    .get(1)
                    .unwrap()
                    .up_urls_ref(true)
                    .contains(&"https://up-jjh-z2.qiniup.com"));
                assert!(regions
                    .get(1)
                    .unwrap()
                    .up_urls_ref(true)
                    .contains(&"https://upload-z2.qbox.me"));
            }));
        }

        threads.into_iter().for_each(|thread| thread.join().unwrap());
        assert_eq!(mock.call_called(), 1);

        Ok(())
    }

    #[test]
    fn test_storage_bucket_set_domain() -> Result<(), Box<dyn Error>> {
        let bucket = BucketBuilder::new(
            "test-bucket".into(),
            get_credential().into(),
            UploadManager::new(
                ConfigBuilder::default()
                    .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
                    .http_request_handler(PanickedHTTPCaller("Should not call it"))
                    .build(),
            ),
        )
        .prepend_domain("abc.com")
        .prepend_domain("def.com")
        .build();
        assert_eq!(bucket.domains()?.len(), 2);
        assert_eq!(bucket.domains()?.get(0), Some(&"def.com"));
        assert_eq!(bucket.domains()?.get(1), Some(&"abc.com"));
        Ok(())
    }

    #[test]
    fn test_storage_bucket_prequery_domain() -> Result<(), Box<dyn Error>> {
        let mock = CounterCallMock::new(JSONCallMock::new(200, Headers::new(), json!(["abc.com", "def.com"])));
        let bucket = BucketBuilder::new(
            "test-bucket".into(),
            get_credential().into(),
            UploadManager::new(
                ConfigBuilder::default()
                    .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
                    .http_request_handler(mock.clone())
                    .build(),
            ),
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
                "test-bucket".into(),
                get_credential().into(),
                UploadManager::new(
                    ConfigBuilder::default()
                        .domains_manager(DomainsManagerBuilder::default().disable_url_resolution().build())
                        .http_request_handler(mock.clone())
                        .build(),
                ),
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
