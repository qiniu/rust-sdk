mod cache;
mod chained;
mod shuffled;
mod simple;
mod timeout;

use super::{super::cache::IsCacheValid, ApiResult, RetriedStatsInfo};
use auto_impl::auto_impl;
use dyn_clonable::clonable;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    mem::take,
    net::IpAddr,
    ops::{Deref, DerefMut},
};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// 域名解析的接口
///
/// 同时提供阻塞接口和异步接口，异步接口则需要启用 `async` 功能
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait Resolver: Clone + Debug + Sync + Send {
    /// 解析域名
    ///
    /// 该方法的异步版本为 [`Self::async_resolve`]。
    fn resolve(&self, domain: &str, opts: ResolveOptions<'_>) -> ResolveResult;

    /// 异步解析域名
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_resolve<'a>(&'a self, domain: &'a str, opts: ResolveOptions<'a>) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move { self.resolve(domain, opts) })
    }
}

/// 解析域名的选项
#[derive(Copy, Debug, Clone, Default)]
pub struct ResolveOptions<'a> {
    retried: Option<&'a RetriedStatsInfo>,
}

impl<'a> ResolveOptions<'a> {
    /// 获取重试统计信息
    #[inline]
    pub fn retried(&'a self) -> Option<&'a RetriedStatsInfo> {
        self.retried
    }

    /// 创建解析域名的选项构建器
    #[inline]
    pub fn builder() -> ResolveOptionsBuilder<'a> {
        Default::default()
    }
}

/// 解析域名的选项构建器
#[derive(Copy, Debug, Clone, Default)]
pub struct ResolveOptionsBuilder<'a>(ResolveOptions<'a>);

impl<'a> ResolveOptionsBuilder<'a> {
    /// 设置重试统计信息
    #[inline]
    pub fn retried(&mut self, retried: &'a RetriedStatsInfo) -> &mut Self {
        self.0.retried = Some(retried);
        self
    }

    /// 构建解析域名的选项
    #[inline]
    pub fn build(&mut self) -> ResolveOptions<'a> {
        take(&mut self.0)
    }
}

/// 解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveAnswers {
    ip_addrs: Vec<IpAddr>,
}

impl ResolveAnswers {
    /// 获取 IP 地址列表
    #[inline]
    pub fn ip_addrs(&self) -> &[IpAddr] {
        &self.ip_addrs
    }

    /// 获取 IP 地址列表的可变引用
    #[inline]
    pub fn ip_addrs_mut(&mut self) -> &mut Vec<IpAddr> {
        &mut self.ip_addrs
    }

    /// 转换为 IP 地址列表
    #[inline]
    pub fn into_ip_addrs(self) -> Vec<IpAddr> {
        self.ip_addrs
    }
}

impl IsCacheValid for ResolveAnswers {}

impl From<Box<[IpAddr]>> for ResolveAnswers {
    #[inline]
    fn from(ip_addrs: Box<[IpAddr]>) -> Self {
        Self {
            ip_addrs: ip_addrs.into(),
        }
    }
}

impl From<Vec<IpAddr>> for ResolveAnswers {
    #[inline]
    fn from(ip_addrs: Vec<IpAddr>) -> Self {
        Self { ip_addrs }
    }
}

impl FromIterator<IpAddr> for ResolveAnswers {
    #[inline]
    fn from_iter<T: IntoIterator<Item = IpAddr>>(iter: T) -> Self {
        Self {
            ip_addrs: Vec::from_iter(iter),
        }
    }
}

impl<'a> IntoIterator for &'a ResolveAnswers {
    type Item = &'a IpAddr;
    type IntoIter = std::slice::Iter<'a, IpAddr>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.ip_addrs.iter()
    }
}

impl Extend<IpAddr> for ResolveAnswers {
    fn extend<T: IntoIterator<Item = IpAddr>>(&mut self, iter: T) {
        self.ip_addrs.extend(iter);
    }
}

impl From<ResolveAnswers> for Box<[IpAddr]> {
    #[inline]
    fn from(answers: ResolveAnswers) -> Self {
        answers.ip_addrs.into()
    }
}

impl From<ResolveAnswers> for Vec<IpAddr> {
    #[inline]
    fn from(answers: ResolveAnswers) -> Self {
        answers.ip_addrs
    }
}

impl AsRef<[IpAddr]> for ResolveAnswers {
    #[inline]
    fn as_ref(&self) -> &[IpAddr] {
        &self.ip_addrs
    }
}

impl AsMut<[IpAddr]> for ResolveAnswers {
    #[inline]
    fn as_mut(&mut self) -> &mut [IpAddr] {
        &mut self.ip_addrs
    }
}

impl Deref for ResolveAnswers {
    type Target = [IpAddr];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ip_addrs
    }
}

impl DerefMut for ResolveAnswers {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ip_addrs
    }
}

/// 域名解析结果
pub type ResolveResult = ApiResult<ResolveAnswers>;

pub use cache::{CachedResolver, CachedResolverBuilder};
pub use chained::{ChainedResolver, ChainedResolverBuilder};
pub use shuffled::ShuffledResolver;
pub use simple::SimpleResolver;
pub use timeout::TimeoutResolver;

mod owned_resolver_options;

#[cfg(any(feature = "c_ares"))]
mod c_ares_impl;

#[cfg(any(feature = "c_ares"))]
pub use c_ares_impl::{c_ares, c_ares_resolver, CAresResolver};

#[cfg(all(feature = "trust_dns", feature = "async"))]
mod trust_dns;

#[cfg(all(feature = "trust_dns", feature = "async"))]
pub use trust_dns::{trust_dns_resolver, TrustDnsResolver};
