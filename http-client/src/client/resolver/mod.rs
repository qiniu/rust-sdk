mod cache;
mod chained;
mod shuffled;
mod simple;
mod timeout;

use super::{super::CacheController, ApiResult};
use auto_impl::auto_impl;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    net::IpAddr,
    ops::{Deref, DerefMut},
};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait Resolver: Debug + Sync + Send {
    fn resolve(&self, domain: &str, opts: &ResolveOptions) -> ResolveResult;

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_resolve<'a>(
        &'a self,
        domain: &'a str,
        opts: &'a ResolveOptions,
    ) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move { self.resolve(domain, opts) })
    }

    fn cache_controller(&self) -> Option<&dyn CacheController> {
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResolveOptions {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveAnswers {
    ip_addrs: Box<[IpAddr]>,
}

impl ResolveAnswers {
    #[inline]
    pub fn ip_addrs(&self) -> &[IpAddr] {
        &self.ip_addrs
    }

    #[inline]
    pub fn ip_addrs_mut(&mut self) -> &mut Box<[IpAddr]> {
        &mut self.ip_addrs
    }

    #[inline]
    pub fn into_ip_addrs(self) -> Box<[IpAddr]> {
        self.ip_addrs
    }
}

impl From<Box<[IpAddr]>> for ResolveAnswers {
    #[inline]
    fn from(ip_addrs: Box<[IpAddr]>) -> Self {
        Self { ip_addrs }
    }
}

impl From<Vec<IpAddr>> for ResolveAnswers {
    #[inline]
    fn from(ip_addrs: Vec<IpAddr>) -> Self {
        Self {
            ip_addrs: ip_addrs.into_boxed_slice(),
        }
    }
}

impl FromIterator<IpAddr> for ResolveAnswers {
    #[inline]
    fn from_iter<T: IntoIterator<Item = IpAddr>>(iter: T) -> Self {
        Self {
            ip_addrs: Vec::from_iter(iter).into(),
        }
    }
}

impl From<ResolveAnswers> for Box<[IpAddr]> {
    #[inline]
    fn from(answers: ResolveAnswers) -> Self {
        answers.ip_addrs
    }
}

impl From<ResolveAnswers> for Vec<IpAddr> {
    #[inline]
    fn from(answers: ResolveAnswers) -> Self {
        answers.ip_addrs.into()
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

pub type ResolveResult = ApiResult<ResolveAnswers>;

pub use cache::CachedResolver;
pub use chained::{ChainedResolver, ChainedResolverBuilder};
pub use shuffled::ShuffledResolver;
pub use simple::SimpleResolver;
pub use timeout::TimeoutResolver;

#[cfg(any(feature = "c_ares"))]
mod c_ares_impl;

#[cfg(any(feature = "c_ares"))]
pub use c_ares_impl::{c_ares, c_ares_resolver, CAresResolver};

#[cfg(all(feature = "trust_dns", feature = "async"))]
mod trust_dns;

#[cfg(all(feature = "trust_dns", feature = "async"))]
pub use trust_dns::{trust_dns_resolver, TrustDnsResolver};
