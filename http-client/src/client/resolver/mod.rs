mod cache;
mod chained;
mod shuffled;
mod simple;
mod timeout;

use super::APIResult;
use std::{any::Any, fmt::Debug, net::IpAddr};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

pub trait Resolver: Any + Debug + Sync + Send {
    fn resolve(&self, domain: &str) -> ResolveResult;

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_resolve<'a>(&'a self, domain: &'a str) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move { self.resolve(domain) })
    }

    fn as_any(&self) -> &dyn Any;
    fn as_resolver(&self) -> &dyn Resolver;
}

#[derive(Debug, Clone, Default)]
pub struct ResolveAnswers {
    ip_addrs: Box<[IpAddr]>,
}

impl ResolveAnswers {
    #[inline]
    pub fn new(ip_addrs: Box<[IpAddr]>) -> Self {
        Self { ip_addrs }
    }

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

pub type ResolveResult = APIResult<ResolveAnswers>;

pub use cache::{CachedResolver, PersistentError, PersistentResult};
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
