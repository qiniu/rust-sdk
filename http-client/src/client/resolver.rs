use std::{any::Any, fmt::Debug, io::Error as IOError, net::IpAddr, result::Result};
use thiserror::Error;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

pub trait Resolver: Any + Debug + Sync + Send {
    fn resolve(&self, domain: &str, port: u16) -> ResolveResult;

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_retry<'a>(&'a self, domain: &'a str, port: u16) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move { self.resolve(domain, port) })
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

#[derive(Debug)]
pub struct SimpleResolver;

impl Resolver for SimpleResolver {
    #[inline]
    fn resolve(&self, domain: &str, port: u16) -> ResolveResult {
        use std::net::ToSocketAddrs;
        Ok((domain, port)
            .to_socket_addrs()?
            .map(|socket_addr| socket_addr.ip())
            .collect())
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_resolver(&self) -> &dyn Resolver {
        self
    }
}

// TODO: Default RequestRetier
