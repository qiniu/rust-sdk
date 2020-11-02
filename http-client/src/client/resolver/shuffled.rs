use super::{ResolveResult, Resolver};
use rand::{seq::SliceRandom, thread_rng};
use std::any::Any;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug, Default, Clone, Copy)]
pub struct ShuffledResolver<R: Resolver> {
    base_resolver: R,
}

impl<R: Resolver> ShuffledResolver<R> {
    #[inline]
    pub fn new(base_resolver: R) -> Self {
        Self { base_resolver }
    }

    #[inline]
    pub fn base_resolver(&self) -> &R {
        &self.base_resolver
    }
}

impl<R: Resolver> Resolver for ShuffledResolver<R> {
    #[inline]
    fn resolve(&self, domain: &str) -> ResolveResult {
        let mut chosen_ips = self.base_resolver().resolve(domain)?;
        chosen_ips.shuffle(&mut thread_rng());
        Ok(chosen_ips)
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_resolve<'a>(&'a self, domain: &'a str) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            let mut chosen_ips = self.base_resolver().async_resolve(domain).await?;
            chosen_ips.shuffle(&mut thread_rng());
            Ok(chosen_ips)
        })
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
