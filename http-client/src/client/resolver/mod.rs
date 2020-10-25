mod cache;
mod shuffled;
mod simple;

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

pub type ResolveResult = APIResult<Box<[IpAddr]>>;

pub use cache::{CachedResolver, PersistentError, PersistentResult};
pub use shuffled::ShuffledResolver;
pub use simple::SimpleResolver;

#[cfg(any(feature = "c_ares"))]
mod c_ares_impl;

#[cfg(any(feature = "c_ares"))]
pub use c_ares_impl::{c_ares, c_ares_resolver, CAresResolver};
