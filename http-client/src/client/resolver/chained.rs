use super::{
    super::{ResponseError, ResponseErrorKind},
    ResolveOptions, ResolveResult, Resolver,
};
use std::collections::VecDeque;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

#[derive(Debug)]
pub struct ChainedResolver {
    resolvers: Box<[Box<dyn Resolver>]>,
}

impl ChainedResolver {
    #[inline]
    pub fn builder(first_resolver: Box<dyn Resolver>) -> ChainedResolverBuilder {
        ChainedResolverBuilder::new(first_resolver)
    }
}

impl Resolver for ChainedResolver {
    #[inline]
    fn resolve(&self, domain: &str, opts: &ResolveOptions) -> ResolveResult {
        let mut last_result: Option<ResolveResult> = None;
        for resolver in self.resolvers.iter() {
            match resolver.resolve(domain, opts) {
                Ok(answers) if !answers.ip_addrs().is_empty() => return Ok(answers),
                result => last_result = Some(result),
            }
        }
        last_result.unwrap_or_else(|| Err(no_try_error()))
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_resolve<'a>(
        &'a self,
        domain: &'a str,
        opts: &'a ResolveOptions,
    ) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            let mut last_result: Option<ResolveResult> = None;
            for resolver in self.resolvers.iter() {
                match resolver.async_resolve(domain, opts).await {
                    Ok(answers) if !answers.ip_addrs().is_empty() => return Ok(answers),
                    result => last_result = Some(result),
                }
            }
            last_result.unwrap_or_else(|| Err(no_try_error()))
        })
    }
}

#[inline]
fn no_try_error() -> ResponseError {
    ResponseError::new(ResponseErrorKind::NoTry, "None resolver is tried")
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

    const IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
        IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2)),
    ];

    #[test]
    fn test_chained_resolver() -> Result<(), Box<dyn Error>> {
        let resolver =
            ChainedResolver::builder(Box::new(make_static_resolver(IPS.to_vec().into())))
                .prepend_resolver(Box::new(make_dumb_resolver()))
                .prepend_resolver(Box::new(make_error_resolver(
                    ResponseErrorKind::LocalIOError.into(),
                    "Test Local IO Error",
                )))
                .build();

        let ips = resolver.resolve("testdomain.com", &Default::default())?;
        assert_eq!(ips.ip_addrs(), IPS);

        let resolver = ChainedResolver::builder(Box::new(make_dumb_resolver()))
            .prepend_resolver(Box::new(make_static_resolver(IPS.to_vec().into())))
            .prepend_resolver(Box::new(make_error_resolver(
                ResponseErrorKind::LocalIOError.into(),
                "Test Local IO Error",
            )))
            .build();

        let ips = resolver.resolve("testdomain.com", &Default::default())?;
        assert_eq!(ips.ip_addrs(), IPS,);

        let resolver = ChainedResolver::builder(Box::new(make_error_resolver(
            ResponseErrorKind::LocalIOError.into(),
            "Test Local IO Error",
        )))
        .prepend_resolver(Box::new(make_dumb_resolver()))
        .prepend_resolver(Box::new(make_static_resolver(IPS.to_vec().into())))
        .build();

        let ips = resolver.resolve("testdomain.com", &Default::default())?;
        assert_eq!(ips.ip_addrs(), IPS,);

        Ok(())
    }
}
