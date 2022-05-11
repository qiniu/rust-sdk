use super::{Endpoint, Endpoints};
use assert_impl::assert_impl;
use serde::{Deserialize, Serialize};
use std::{mem::take, sync::Arc};

/// 七牛存储区域
///
/// 提供七牛不同服务的终端地址列表
///
/// ```
/// use qiniu_http_client::Region;
///
/// # fn main() -> anyhow::Result<()> {
/// let region = Region::builder("z0")
///     .add_uc_preferred_endpoint("uc.qbox.me".parse()?)
///     .add_up_preferred_endpoint("upload.qiniup.com".parse()?)
///     .add_up_preferred_endpoint("up.qiniup.com".parse()?)
///     .add_up_alternative_endpoint("up.qbox.me".parse()?)
///     .add_rs_preferred_endpoint("rs.qbox.me".parse()?)
///     .add_rsf_preferred_endpoint("rsf.qbox.me".parse()?)
///     .add_api_preferred_endpoint("api.qiniu.com".parse()?)
///     .build();
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Region {
    inner: Arc<RegionInner>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
struct RegionInner {
    region_id: Box<str>,
    s3_region_id: Box<str>,
    up: Endpoints,
    io: Endpoints,
    uc: Endpoints,
    rs: Endpoints,
    rsf: Endpoints,
    api: Endpoints,
    s3: Endpoints,
}

impl Region {
    /// 获取区域 ID
    #[inline]
    pub fn region_id(&self) -> &str {
        &self.inner.region_id
    }

    /// 获取 S3 区域 ID
    #[inline]
    pub fn s3_region_id(&self) -> &str {
        &self.inner.s3_region_id
    }

    /// 获取上传服务主要终端列表
    ///
    /// 与 `up().preferred()` 等效
    #[inline]
    pub fn up_preferred_endpoints(&self) -> &[Endpoint] {
        self.up().preferred()
    }

    /// 获取上传服务备选终端列表
    ///
    /// 与 `up().alternative()` 等效
    #[inline]
    pub fn up_alternative_endpoints(&self) -> &[Endpoint] {
        self.up().alternative()
    }

    /// 获取下载服务主要终端列表
    ///
    /// 与 `io().preferred()` 等效
    #[inline]
    pub fn io_preferred_endpoints(&self) -> &[Endpoint] {
        self.io().preferred()
    }

    /// 获取下载服务备选终端列表
    ///
    /// 与 `io().alternative()` 等效
    #[inline]
    pub fn io_alternative_endpoints(&self) -> &[Endpoint] {
        self.io().alternative()
    }

    /// 获取存储空间管理服务主要终端列表
    ///
    /// 与 `uc().preferred()` 等效
    #[inline]
    pub fn uc_preferred_endpoints(&self) -> &[Endpoint] {
        self.uc().preferred()
    }

    /// 获取存储空间管理服务备选终端列表
    ///
    /// 与 `uc().alternative()` 等效
    #[inline]
    pub fn uc_alternative_endpoints(&self) -> &[Endpoint] {
        self.uc().alternative()
    }

    /// 获取元数据管理服务主要终端列表
    ///
    /// 与 `rs().preferred()` 等效
    #[inline]
    pub fn rs_preferred_endpoints(&self) -> &[Endpoint] {
        self.rs().preferred()
    }

    /// 获取元数据管理服务备选终端列表
    ///
    /// 与 `rs().alternative()` 等效
    #[inline]
    pub fn rs_alternative_endpoints(&self) -> &[Endpoint] {
        self.rs().alternative()
    }

    /// 获取元数据列举服务主要终端列表
    ///
    /// 与 `rsf().preferred()` 等效
    #[inline]
    pub fn rsf_preferred_endpoints(&self) -> &[Endpoint] {
        self.rsf().preferred()
    }

    /// 获取元数据列举服务备选终端列表
    ///
    /// 与 `rsf().alternative()` 等效
    #[inline]
    pub fn rsf_alternative_endpoints(&self) -> &[Endpoint] {
        self.rsf().alternative()
    }

    /// 获取 API 入口服务主要终端列表
    ///
    /// 与 `api().preferred()` 等效
    #[inline]
    pub fn api_preferred_endpoints(&self) -> &[Endpoint] {
        self.api().preferred()
    }

    /// 获取 API 入口服务备选终端列表
    ///
    /// 与 `api().alternative()` 等效
    #[inline]
    pub fn api_alternative_endpoints(&self) -> &[Endpoint] {
        self.api().alternative()
    }

    /// 获取 S3 入口服务主要终端列表
    ///
    /// 与 `s3().preferred()` 等效
    #[inline]
    pub fn s3_preferred_endpoints(&self) -> &[Endpoint] {
        self.s3().preferred()
    }

    /// 获取 S3 入口服务备选终端列表
    ///
    /// 与 `s3().alternative()` 等效
    #[inline]
    pub fn s3_alternative_endpoints(&self) -> &[Endpoint] {
        self.s3().alternative()
    }

    /// 获取上传服务终端地址列表
    #[inline]
    pub fn up(&self) -> &Endpoints {
        &self.inner.up
    }

    /// 获取下载服务终端地址列表
    #[inline]
    pub fn io(&self) -> &Endpoints {
        &self.inner.io
    }

    /// 获取存储空间管理服务终端地址列表
    #[inline]
    pub fn uc(&self) -> &Endpoints {
        &self.inner.uc
    }

    /// 获取元数据管理服务终端地址列表
    #[inline]
    pub fn rs(&self) -> &Endpoints {
        &self.inner.rs
    }

    /// 获取元数据列举服务终端地址列表
    #[inline]
    pub fn rsf(&self) -> &Endpoints {
        &self.inner.rsf
    }

    /// 获取 API 入口服务终端地址列表
    #[inline]
    pub fn api(&self) -> &Endpoints {
        &self.inner.api
    }

    /// 获取 S3 入口服务终端地址列表
    #[inline]
    pub fn s3(&self) -> &Endpoints {
        &self.inner.s3
    }

    /// 创建区域构建器
    #[inline]
    pub fn builder(region_id: impl Into<String>) -> RegionBuilder {
        RegionBuilder::new(region_id.into())
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

/// 区域构建器
///
/// ```
/// use qiniu_http_client::RegionBuilder;
///
/// # fn main() -> anyhow::Result<()> {
/// let region = RegionBuilder::new("z0")
///     .add_uc_preferred_endpoint("10.11.0.178:10221".parse()?) // 添加一个终端地址
///     .add_uc_preferred_endpoint("10.11.0.180:10221".parse()?)
///     .add_up_preferred_endpoint("10.11.0.178:5010".parse()?)
///     .add_up_preferred_endpoint("10.11.0.180:5010".parse()?)
///     .add_io_preferred_endpoint("10.11.0.178:5000".parse()?)
///     .add_io_preferred_endpoint("10.11.0.180:5000".parse()?)
///     .add_rs_preferred_endpoints(["10.11.0.178:9433".parse()?, "10.11.0.180:9433".parse()?]) // 添加多个终端地址
///     .add_rsf_preferred_endpoints(["10.11.0.178:7913".parse()?, "10.11.0.180:7913".parse()?])
///     .build();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct RegionBuilder {
    region_id: String,
    s3_region_id: String,
    up_preferred: Vec<Endpoint>,
    up_alternative: Vec<Endpoint>,
    io_preferred: Vec<Endpoint>,
    io_alternative: Vec<Endpoint>,
    uc_preferred: Vec<Endpoint>,
    uc_alternative: Vec<Endpoint>,
    rs_preferred: Vec<Endpoint>,
    rs_alternative: Vec<Endpoint>,
    rsf_preferred: Vec<Endpoint>,
    rsf_alternative: Vec<Endpoint>,
    api_preferred: Vec<Endpoint>,
    api_alternative: Vec<Endpoint>,
    s3_preferred: Vec<Endpoint>,
    s3_alternative: Vec<Endpoint>,
}

impl RegionBuilder {
    /// 创建新的区域，传入终端 ID
    pub fn new(region_id: impl Into<String>) -> Self {
        Self {
            region_id: region_id.into(),
            s3_region_id: Default::default(),
            up_preferred: Default::default(),
            up_alternative: Default::default(),
            io_preferred: Default::default(),
            io_alternative: Default::default(),
            uc_preferred: Default::default(),
            uc_alternative: Default::default(),
            rs_preferred: Default::default(),
            rs_alternative: Default::default(),
            rsf_preferred: Default::default(),
            rsf_alternative: Default::default(),
            api_preferred: Default::default(),
            api_alternative: Default::default(),
            s3_preferred: Default::default(),
            s3_alternative: Default::default(),
        }
    }

    /// 设置 S3 区域 ID
    #[inline]
    pub fn s3_region_id(&mut self, s3_region_id: impl Into<String>) -> &mut Self {
        self.s3_region_id = s3_region_id.into();
        self
    }

    /// 添加访问终端地址到上传服务主要终端地址列表
    #[inline]
    pub fn add_up_preferred_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.up_preferred.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到上传服务主要终端地址列表
    #[inline]
    pub fn add_up_preferred_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.up_preferred.extend(endpoints);
        self
    }

    /// 添加访问终端地址到上传服务备选终端地址列表
    #[inline]
    pub fn add_up_alternative_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.up_alternative.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到上传服务备选终端地址列表
    #[inline]
    pub fn add_up_alternative_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.up_alternative.extend(endpoints);
        self
    }

    /// 添加访问终端地址到下载服务主要终端地址列表
    #[inline]
    pub fn add_io_preferred_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.io_preferred.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到下载服务主要终端地址列表
    #[inline]
    pub fn add_io_preferred_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.io_preferred.extend(endpoints);
        self
    }

    /// 添加访问终端地址到下载服务备选终端地址列表
    #[inline]
    pub fn add_io_alternative_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.io_alternative.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到下载服务备选终端地址列表
    #[inline]
    pub fn add_io_alternative_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.io_alternative.extend(endpoints);
        self
    }

    /// 添加访问终端地址到存储空间管理服务主要终端地址列表
    #[inline]
    pub fn add_uc_preferred_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.uc_preferred.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到存储空间管理服务主要终端地址列表
    #[inline]
    pub fn add_uc_preferred_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.uc_preferred.extend(endpoints);
        self
    }

    /// 添加访问终端地址到存储空间管理服务备选终端地址列表
    #[inline]
    pub fn add_uc_alternative_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.uc_alternative.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到存储空间管理服务备选终端地址列表
    #[inline]
    pub fn add_uc_alternative_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.uc_alternative.extend(endpoints);
        self
    }

    /// 添加访问终端地址到元数据管理服务主要终端地址列表
    #[inline]
    pub fn add_rs_preferred_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.rs_preferred.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到元数据管理服务主要终端地址列表
    #[inline]
    pub fn add_rs_preferred_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.rs_preferred.extend(endpoints);
        self
    }

    /// 添加访问终端地址到元数据管理服务备选终端地址列表
    #[inline]
    pub fn add_rs_alternative_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.rs_alternative.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到元数据管理服务备选终端地址列表
    #[inline]
    pub fn add_rs_alternative_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.rs_alternative.extend(endpoints);
        self
    }

    /// 添加访问终端地址到元数据列举服务主要终端地址列表
    #[inline]
    pub fn add_rsf_preferred_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.rsf_preferred.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到元数据列举服务主要终端地址列表
    #[inline]
    pub fn add_rsf_preferred_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.rsf_preferred.extend(endpoints);
        self
    }

    /// 添加访问终端地址到元数据列举服务备选终端地址列表
    #[inline]
    pub fn add_rsf_alternative_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.rsf_alternative.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到元数据列举服务备选终端地址列表
    #[inline]
    pub fn add_rsf_alternative_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.rsf_alternative.extend(endpoints);
        self
    }

    /// 添加访问终端地址到 API 入口服务主要终端地址列表
    #[inline]
    pub fn add_api_preferred_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.api_preferred.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到 API 入口服务主要终端地址列表
    #[inline]
    pub fn add_api_preferred_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.api_preferred.extend(endpoints);
        self
    }

    /// 添加访问终端地址到 API 入口服务备选终端地址列表
    #[inline]
    pub fn add_api_alternative_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.api_alternative.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到 API 入口服务备选终端地址列表
    #[inline]
    pub fn add_api_alternative_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.api_alternative.extend(endpoints);
        self
    }

    /// 添加访问终端地址到 S3 入口服务主要终端地址列表
    #[inline]
    pub fn add_s3_preferred_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.s3_preferred.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到 S3 入口服务主要终端地址列表
    #[inline]
    pub fn add_s3_preferred_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.s3_preferred.extend(endpoints);
        self
    }

    /// 添加访问终端地址到 S3 入口服务备选终端地址列表
    #[inline]
    pub fn add_s3_alternative_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.s3_alternative.push(endpoint);
        self
    }

    /// 添加多个访问终端地址到 S3 入口服务备选终端地址列表
    #[inline]
    pub fn add_s3_alternative_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.s3_alternative.extend(endpoints);
        self
    }

    /// 构建区域
    pub fn build(&mut self) -> Region {
        let owned = take(self);
        Region {
            inner: Arc::new(RegionInner {
                region_id: owned.region_id.into_boxed_str(),
                s3_region_id: owned.s3_region_id.into_boxed_str(),
                up: (owned.up_preferred, owned.up_alternative).into(),
                io: (owned.io_preferred, owned.io_alternative).into(),
                uc: (owned.uc_preferred, owned.uc_alternative).into(),
                rs: (owned.rs_preferred, owned.rs_alternative).into(),
                rsf: (owned.rsf_preferred, owned.rsf_alternative).into(),
                api: (owned.api_preferred, owned.api_alternative).into(),
                s3: (owned.s3_preferred, owned.s3_alternative).into(),
            }),
        }
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}
