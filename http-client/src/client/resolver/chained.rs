use super::{ResolveResult, Resolver};
use std::{any::Any, sync::Arc};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug, Clone)]
pub struct ChainedResolver {
    resolvers: Arc<[Box<dyn Resolver>]>,
}

impl ChainedResolver {
    #[inline]
    pub fn builder() -> ChainedResolverBuilder {
        ChainedResolverBuilder::default()
    }
}

impl Resolver for ChainedResolver {
    #[inline]
    fn resolve(&self, domain: &str) -> ResolveResult {
        let mut last_result: Option<ResolveResult> = None;
        for resolver in self.resolvers.iter() {
            match resolver.resolve(domain) {
                Ok(ips) if !ips.is_empty() => return Ok(ips),
                result => last_result = Some(result),
            }
        }
        last_result.expect("None resolver is tried")
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_resolve<'a>(&'a self, domain: &'a str) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            let mut last_result: Option<ResolveResult> = None;
            for resolver in self.resolvers.iter() {
                match resolver.async_resolve(domain).await {
                    Ok(ips) if !ips.is_empty() => return Ok(ips),
                    result => last_result = Some(result),
                }
            }
            last_result.expect("None resolver is tried")
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

#[derive(Debug, Default)]
pub struct ChainedResolverBuilder {
    resolvers: Vec<Box<dyn Resolver>>,
}

impl ChainedResolverBuilder {
    #[inline]
    pub fn push_resolver(mut self, resolver: Box<dyn Resolver>) -> Self {
        self.resolvers.push(resolver);
        self
    }

    #[inline]
    pub fn build(self) -> ChainedResolver {
        ChainedResolver {
            resolvers: self.resolvers.into(),
        }
    }
}
