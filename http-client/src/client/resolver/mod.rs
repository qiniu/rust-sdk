mod cache;
mod simple;

use std::{any::Any, fmt::Debug, io::Error as IOError, net::IpAddr, result::Result};
use thiserror::Error;

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

#[derive(Error, Debug)]
pub enum ResolveError {
    #[error("Resolve domain name error: {0}")]
    IOError(#[from] IOError),
}
pub type ResolveResult = Result<Box<[IpAddr]>, ResolveError>;

pub use cache::{CachedResolver, PersistentError, PersistentResult};
pub use simple::SimpleResolver;
