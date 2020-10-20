use super::{
    super::{CachedResolver, Resolver, ResponseError, SimpleResolver},
    Chooser, ChosenResult,
};
use std::{any::Any, net::IpAddr};

#[derive(Debug)]
pub struct SimpleChooser<R: Resolver> {
    resolver: R,
}

impl<R: Resolver> SimpleChooser<R> {
    #[inline]
    pub fn new(resolver: R) -> Self {
        Self { resolver }
    }
}

impl Default for SimpleChooser<CachedResolver<SimpleResolver>> {
    #[inline]
    fn default() -> Self {
        Self::new(CachedResolver::<SimpleResolver>::default())
    }
}

impl<R: Resolver> Chooser for SimpleChooser<R> {
    #[inline]
    fn choose(&self, domain: &str) -> ChosenResult {
        self.resolver.resolve(domain).map_or_else(
            |_| ChosenResult::UseThisDomainDirectly,
            |ips| ChosenResult::IPs(ips.into()),
        )
    }

    #[inline]
    fn mark_as_failed(&self, _domain: &str, _ip: IpAddr, _error: ResponseError) {
        // Do nothing
    }

    #[inline]
    fn resolver(&self) -> &dyn Resolver {
        &self.resolver
    }

    #[inline]
    fn resolver_mut(&mut self) -> &mut dyn Resolver {
        &mut self.resolver
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_chooser(&self) -> &dyn Chooser {
        self
    }
}
