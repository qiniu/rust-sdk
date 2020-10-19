use super::Domains;
use std::sync::Arc;

/// 七牛存储区域
///
/// 提供七牛不同区域的域名
#[derive(Clone, Debug)]
pub struct Region {
    region_id: String,
    s3_region_id: String,
    up: Arc<Domains>,
    io: Arc<Domains>,
    uc: Arc<Domains>,
    rs: Arc<Domains>,
    rsf: Arc<Domains>,
    api: Arc<Domains>,
    s3: Arc<Domains>,
}

impl Region {
    /// 获取区域 ID
    #[inline]
    pub fn region_id(&self) -> &str {
        &self.region_id
    }

    /// 获取 S3 区域 ID
    #[inline]
    pub fn s3_region_id(&self) -> &str {
        &self.s3_region_id
    }

    /// 获取上传域名列表
    #[inline]
    pub fn up_domains(&self) -> &[String] {
        &self.up.domains()
    }

    #[inline]
    #[doc(hidden)]
    pub fn up_old_domains(&self) -> &[String] {
        &self.up.old_domains()
    }

    /// 获取下载域名列表
    #[inline]
    pub fn io_domains(&self) -> &[String] {
        &self.io.domains()
    }

    #[inline]
    #[doc(hidden)]
    pub fn io_old_domains(&self) -> &[String] {
        &self.io.old_domains()
    }

    /// 获取 UC 域名列表
    #[inline]
    pub fn uc_domains(&self) -> &[String] {
        &self.uc.domains()
    }

    #[inline]
    #[doc(hidden)]
    pub fn uc_old_domains(&self) -> &[String] {
        &self.uc.old_domains()
    }

    /// 获取 RS 域名列表
    #[inline]
    pub fn rs_domains(&self) -> &[String] {
        &self.rs.domains()
    }

    #[inline]
    #[doc(hidden)]
    pub fn rs_old_domains(&self) -> &[String] {
        &self.rs.old_domains()
    }

    /// 获取 RSF 域名列表
    #[inline]
    pub fn rsf_domains(&self) -> &[String] {
        &self.rsf.domains()
    }

    #[inline]
    #[doc(hidden)]
    pub fn rsf_old_domains(&self) -> &[String] {
        &self.rsf.old_domains()
    }

    /// 获取 API 域名列表
    #[inline]
    pub fn api_domains(&self) -> &[String] {
        &self.api.domains()
    }

    #[inline]
    #[doc(hidden)]
    pub fn api_old_domains(&self) -> &[String] {
        &self.api.old_domains()
    }

    /// 获取 S3 域名列表
    #[inline]
    pub fn s3_domains(&self) -> &[String] {
        &self.s3.domains()
    }

    #[inline]
    #[doc(hidden)]
    pub fn s3_old_domains(&self) -> &[String] {
        &self.s3.old_domains()
    }

    /// 创建新的区域
    #[inline]
    pub fn builder(region_id: String) -> RegionBuilder {
        RegionBuilder::new(region_id)
    }

    #[inline]
    pub(super) fn up(&self) -> &Domains {
        &self.up
    }

    #[inline]
    pub(super) fn io(&self) -> &Domains {
        &self.io
    }

    #[inline]
    pub(super) fn uc(&self) -> &Domains {
        &self.uc
    }

    #[inline]
    pub(super) fn rs(&self) -> &Domains {
        &self.rs
    }

    #[inline]
    pub(super) fn rsf(&self) -> &Domains {
        &self.rsf
    }

    #[inline]
    pub(super) fn api(&self) -> &Domains {
        &self.api
    }

    #[inline]
    pub(super) fn s3(&self) -> &Domains {
        &self.s3
    }
}

pub struct RegionBuilder {
    region_id: String,
    s3_region_id: String,
    up: Domains,
    io: Domains,
    uc: Domains,
    rs: Domains,
    rsf: Domains,
    api: Domains,
    s3: Domains,
}

impl RegionBuilder {
    /// 创建新的区域，传入域名 ID
    pub fn new(region_id: String) -> Self {
        Self {
            region_id,
            s3_region_id: Default::default(),
            up: Default::default(),
            io: Default::default(),
            uc: Default::default(),
            rs: Default::default(),
            rsf: Default::default(),
            api: Default::default(),
            s3: Default::default(),
        }
    }

    /// 设置 S3 区域 ID
    #[inline]
    pub fn s3_region_id(mut self, s3_region_id: String) -> Self {
        self.s3_region_id = s3_region_id;
        self
    }

    /// 追加域名到上传域名列表
    #[inline]
    pub fn up_domain(mut self, domain: String) -> Self {
        self.up.domains_mut().push(domain);
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn up_old_domain(mut self, domain: String) -> Self {
        self.up.old_domains_mut().push(domain);
        self
    }

    /// 追加域名到下载域名列表
    #[inline]
    pub fn io_domain(mut self, domain: String) -> Self {
        self.io.domains_mut().push(domain);
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn io_old_domain(mut self, domain: String) -> Self {
        self.io.old_domains_mut().push(domain);
        self
    }

    /// 追加域名到 UC 域名列表
    #[inline]
    pub fn uc_domain(mut self, domain: String) -> Self {
        self.uc.domains_mut().push(domain);
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn uc_old_domain(mut self, domain: String) -> Self {
        self.uc.old_domains_mut().push(domain);
        self
    }

    /// 追加域名到 RS 域名列表
    #[inline]
    pub fn rs_domain(mut self, domain: String) -> Self {
        self.rs.domains_mut().push(domain);
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn rs_old_domain(mut self, domain: String) -> Self {
        self.rs.old_domains_mut().push(domain);
        self
    }

    /// 追加域名到 RSF 域名列表
    #[inline]
    pub fn rsf_domain(mut self, domain: String) -> Self {
        self.rsf.domains_mut().push(domain);
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn rsf_old_domain(mut self, domain: String) -> Self {
        self.rsf.old_domains_mut().push(domain);
        self
    }

    /// 追加域名到 API 域名列表
    #[inline]
    pub fn api_domain(mut self, domain: String) -> Self {
        self.api.domains_mut().push(domain);
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn api_old_domain(mut self, domain: String) -> Self {
        self.api.old_domains_mut().push(domain);
        self
    }

    /// 追加域名到 S3 域名列表
    #[inline]
    pub fn s3_domain(mut self, domain: String) -> Self {
        self.s3.domains_mut().push(domain);
        self
    }

    #[inline]
    #[doc(hidden)]
    pub fn s3_old_domain(mut self, domain: String) -> Self {
        self.s3.old_domains_mut().push(domain);
        self
    }

    /// 构建区域
    #[inline]
    pub fn build(self) -> Region {
        Region {
            region_id: self.region_id,
            s3_region_id: self.s3_region_id,
            up: Arc::new(self.up),
            io: Arc::new(self.io),
            uc: Arc::new(self.uc),
            rs: Arc::new(self.rs),
            rsf: Arc::new(self.rsf),
            api: Arc::new(self.api),
            s3: Arc::new(self.s3),
        }
    }
}
