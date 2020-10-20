use super::{ResolveResult, Resolver};
use dns_lookup::lookup_host;
use std::any::Any;

#[derive(Debug)]
pub struct SimpleResolver;

impl Resolver for SimpleResolver {
    #[inline]
    fn resolve(&self, domain: &str) -> ResolveResult {
        let ip_addrs = lookup_host(domain).map(|ips| ips.into_boxed_slice())?;
        Ok(ip_addrs)
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
