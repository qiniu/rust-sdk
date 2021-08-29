use super::{ResolveResult, Resolver};
use rand::{prelude::*, thread_rng};
use std::any::Any;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug, Default, Clone)]
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
        let mut answers = self.base_resolver().resolve(domain)?;
        answers.ip_addrs_mut().shuffle(&mut thread_rng());
        Ok(answers)
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_resolve<'a>(&'a self, domain: &'a str) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            let mut answers = self.base_resolver().async_resolve(domain).await?;
            answers.ip_addrs_mut().shuffle(&mut thread_rng());
            Ok(answers)
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
