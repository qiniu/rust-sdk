use super::{Endpoint, Endpoints};
use serde::{Deserialize, Serialize};
use std::{mem::take, sync::Arc};

/// 七牛存储区域
///
/// 提供七牛不同区域的域名
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

    /// 获取上传域名列表
    #[inline]
    pub fn up_preferred_endpoints(&self) -> &[Endpoint] {
        self.up().preferred()
    }

    #[inline]
    #[doc(hidden)]
    pub fn up_alternative_endpoints(&self) -> &[Endpoint] {
        self.up().alternative()
    }

    /// 获取下载域名列表
    #[inline]
    pub fn io_preferred_endpoints(&self) -> &[Endpoint] {
        self.io().preferred()
    }

    #[inline]
    #[doc(hidden)]
    pub fn io_alternative_endpoints(&self) -> &[Endpoint] {
        self.io().alternative()
    }

    /// 获取 UC 域名列表
    #[inline]
    pub fn uc_preferred_endpoints(&self) -> &[Endpoint] {
        self.uc().preferred()
    }

    #[inline]
    #[doc(hidden)]
    pub fn uc_alternative_endpoints(&self) -> &[Endpoint] {
        self.uc().alternative()
    }

    /// 获取 RS 域名列表
    #[inline]
    pub fn rs_preferred_endpoints(&self) -> &[Endpoint] {
        self.rs().preferred()
    }

    #[inline]
    #[doc(hidden)]
    pub fn rs_alternative_endpoints(&self) -> &[Endpoint] {
        self.rs().alternative()
    }

    /// 获取 RSF 域名列表
    #[inline]
    pub fn rsf_preferred_endpoints(&self) -> &[Endpoint] {
        self.rsf().preferred()
    }

    #[inline]
    #[doc(hidden)]
    pub fn rsf_alternative_endpoints(&self) -> &[Endpoint] {
        self.rsf().alternative()
    }

    /// 获取 API 域名列表
    #[inline]
    pub fn api_preferred_endpoints(&self) -> &[Endpoint] {
        self.api().preferred()
    }

    #[inline]
    #[doc(hidden)]
    pub fn api_alternative_endpoints(&self) -> &[Endpoint] {
        self.api().alternative()
    }

    /// 获取 S3 域名列表
    #[inline]
    pub fn s3_preferred_endpoints(&self) -> &[Endpoint] {
        self.s3().preferred()
    }

    #[inline]
    #[doc(hidden)]
    pub fn s3_alternative_endpoints(&self) -> &[Endpoint] {
        self.s3().alternative()
    }

    /// 创建新的区域
    #[inline]
    pub fn builder(region_id: impl Into<String>) -> RegionBuilder {
        RegionBuilder::new(region_id.into())
    }

    pub(super) fn up(&self) -> &Endpoints {
        &self.inner.up
    }

    pub(super) fn io(&self) -> &Endpoints {
        &self.inner.io
    }

    pub(super) fn uc(&self) -> &Endpoints {
        &self.inner.uc
    }

    pub(super) fn rs(&self) -> &Endpoints {
        &self.inner.rs
    }

    pub(super) fn rsf(&self) -> &Endpoints {
        &self.inner.rsf
    }

    pub(super) fn api(&self) -> &Endpoints {
        &self.inner.api
    }

    pub(super) fn s3(&self) -> &Endpoints {
        &self.inner.s3
    }
}

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
    /// 创建新的区域，传入域名 ID
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

    /// 追加访问地址到上传访问地址列表
    #[inline]
    pub fn push_up_preferred_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.up_preferred.push(endpoint.into());
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn push_up_alternative_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.up_alternative.push(endpoint.into());
        self
    }

    /// 追加访问地址到下载访问地址列表
    #[inline]
    pub fn push_io_preferred_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.io_preferred.push(endpoint.into());
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn push_io_alternative_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.io_alternative.push(endpoint.into());
        self
    }

    /// 追加访问地址到 UC 访问地址列表
    #[inline]
    pub fn push_uc_preferred_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.uc_preferred.push(endpoint.into());
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn push_uc_alternative_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.uc_alternative.push(endpoint.into());
        self
    }

    /// 追加访问地址到 RS 访问地址列表
    #[inline]
    pub fn push_rs_preferred_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.rs_preferred.push(endpoint.into());
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn push_rs_alternative_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.rs_alternative.push(endpoint.into());
        self
    }

    /// 追加访问地址到 RSF 访问地址列表
    #[inline]
    pub fn push_rsf_preferred_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.rsf_preferred.push(endpoint.into());
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn push_rsf_alternative_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.rsf_alternative.push(endpoint.into());
        self
    }

    /// 追加访问地址到 API 访问地址列表
    #[inline]
    pub fn push_api_preferred_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.api_preferred.push(endpoint.into());
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn push_api_alternative_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.api_alternative.push(endpoint.into());
        self
    }

    /// 追加访问地址到 S3 访问地址列表
    #[inline]
    pub fn push_s3_preferred_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.s3_preferred.push(endpoint.into());
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn push_s3_alternative_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.s3_alternative.push(endpoint.into());
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
}
