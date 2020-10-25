use super::{ResolveResult, Resolver};
use rand::{seq::SliceRandom, thread_rng};
use std::any::Any;

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
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_resolver(&self) -> &dyn Resolver {
        self
    }
}
