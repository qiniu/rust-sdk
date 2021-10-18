mod cache;
mod chained;
mod shuffled;
mod simple;
mod timeout;

use super::{super::CacheController, APIResult};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    fmt::Debug,
    net::IpAddr,
    ops::{Deref, DerefMut},
};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

pub trait Resolver: Any + Debug + Sync + Send {
    fn resolve(&self, domain: &str, opts: &ResolveOptions) -> ResolveResult;

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_resolve<'a>(
        &'a self,
        domain: &'a str,
        opts: &'a ResolveOptions,
    ) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move { self.resolve(domain, opts) })
    }

    fn as_any(&self) -> &dyn Any;
    fn as_resolver(&self) -> &dyn Resolver;
    fn cache_controller(&self) -> Option<&dyn CacheController> {
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResolveOptions {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolveAnswers(Box<[IpAddr]>);

impl ResolveAnswers {
    #[inline]
    pub fn ip_addrs(&self) -> &[IpAddr] {
        &self.0
    }

    #[inline]
    pub fn ip_addrs_mut(&mut self) -> &mut Box<[IpAddr]> {
        &mut self.0
    }

    #[inline]
    pub fn into_ip_addrs(self) -> Box<[IpAddr]> {
        self.0
    }
}

impl From<Box<[IpAddr]>> for ResolveAnswers {
    #[inline]
    fn from(ip_addrs: Box<[IpAddr]>) -> Self {
        Self(ip_addrs)
    }
}

impl From<Vec<IpAddr>> for ResolveAnswers {
    #[inline]
    fn from(ip_addrs: Vec<IpAddr>) -> Self {
        Self(ip_addrs.into_boxed_slice())
    }
}

impl From<ResolveAnswers> for Box<[IpAddr]> {
    #[inline]
    fn from(answers: ResolveAnswers) -> Self {
        answers.0
    }
}

impl From<ResolveAnswers> for Vec<IpAddr> {
    #[inline]
    fn from(answers: ResolveAnswers) -> Self {
        answers.0.into()
    }
}

impl AsRef<[IpAddr]> for ResolveAnswers {
    #[inline]
    fn as_ref(&self) -> &[IpAddr] {
        &self.0
    }
}

impl AsMut<[IpAddr]> for ResolveAnswers {
    #[inline]
    fn as_mut(&mut self) -> &mut [IpAddr] {
        &mut self.0
    }
}

impl Deref for ResolveAnswers {
    type Target = [IpAddr];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ResolveAnswers {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub type ResolveResult = APIResult<ResolveAnswers>;

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
