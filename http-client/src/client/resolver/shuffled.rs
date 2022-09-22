use super::{ResolveOptions, ResolveResult, Resolver};
use rand::{prelude::*, thread_rng};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// 域名解析随机混淆器
///
/// 基于一个域名解析器实例，但将其返回的解析结果打乱
#[derive(Debug, Default, Clone)]
pub struct ShuffledResolver<R: ?Sized> {
    base_resolver: R,
}

impl<R> ShuffledResolver<R> {
    /// 创建域名解析随机混淆器
    #[inline]
    pub const fn new(base_resolver: R) -> Self {
        Self { base_resolver }
    }
}

impl<R: Resolver + Clone> Resolver for ShuffledResolver<R> {
    #[inline]
    fn resolve(&self, domain: &str, opts: ResolveOptions) -> ResolveResult {
        let mut answers = self.base_resolver.resolve(domain, opts)?;
        answers.ip_addrs_mut().shuffle(&mut thread_rng());
        Ok(answers)
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_resolve<'a>(&'a self, domain: &'a str, opts: ResolveOptions<'a>) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            let mut answers = self.base_resolver.async_resolve(domain, opts).await?;
            answers.ip_addrs_mut().shuffle(&mut thread_rng());
            Ok(answers)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::make_static_resolver;
    use std::{
        collections::HashSet,
        error::Error,
        net::{IpAddr, Ipv4Addr},
        result::Result,
    };

    const IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
        IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2)),
        IpAddr::V4(Ipv4Addr::new(3, 3, 3, 3)),
    ];

    #[test]
    fn test_shuffled_resolver() -> Result<(), Box<dyn Error>> {
        let resolver = ShuffledResolver::new(make_static_resolver(IPS.to_vec().into()));
        let ips = resolver.resolve("testdomain.com", Default::default())?;
        assert_eq!(make_set(ips.ip_addrs()), make_set(IPS));
        Ok(())
    }

    fn make_set(ips: impl AsRef<[IpAddr]>) -> HashSet<IpAddr> {
        let mut h = HashSet::new();
        h.extend(ips.as_ref());
        h
    }
}
