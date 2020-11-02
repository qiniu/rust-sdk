use super::{ResolveResult, Resolver};
use std::{any::Any, collections::VecDeque, sync::Arc};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug, Clone)]
pub struct ChainedResolver {
    resolvers: Arc<[Box<dyn Resolver>]>,
}

impl ChainedResolver {
    #[inline]
    pub fn builder(first_resolver: Box<dyn Resolver>) -> ChainedResolverBuilder {
        ChainedResolverBuilder::new(first_resolver)
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

#[derive(Debug)]
pub struct ChainedResolverBuilder {
    resolvers: VecDeque<Box<dyn Resolver>>,
}

impl ChainedResolverBuilder {
    #[inline]
    pub fn new(first_resolver: Box<dyn Resolver>) -> Self {
        Self {
            resolvers: vec![first_resolver].into(),
        }
    }

    #[inline]
    pub fn append_resolver(mut self, resolver: Box<dyn Resolver>) -> Self {
        self.resolvers.push_back(resolver);
        self
    }

    #[inline]
    pub fn prepend_resolver(mut self, resolver: Box<dyn Resolver>) -> Self {
        self.resolvers.push_front(resolver);
        self
    }

    #[inline]
    pub fn build(self) -> ChainedResolver {
        ChainedResolver {
            resolvers: Vec::from(self.resolvers).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{make_dumb_resolver, make_error_resolver, make_static_resolver};
    use qiniu_http::ResponseErrorKind;
    use std::{
        error::Error,
        net::{IpAddr, Ipv4Addr},
        result::Result,
    };

    #[test]
    fn test_chained_resolver() -> Result<(), Box<dyn Error>> {
        let resolver = ChainedResolver::builder(Box::new(make_static_resolver(
            vec![
                IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2)),
            ]
            .into(),
        )))
        .prepend_resolver(Box::new(make_dumb_resolver()))
        .prepend_resolver(Box::new(make_error_resolver(
            ResponseErrorKind::LocalIOError.into(),
            "Test Local IO Error",
        )))
        .build();

        let ips = resolver.resolve("testdomain.com")?;
        assert_eq!(
            ips.as_ref(),
            &[
                IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
                IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2)),
            ][..]
        );

        Ok(())
    }
}
