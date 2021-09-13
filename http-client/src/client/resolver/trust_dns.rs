use super::{super::ResponseError, ResolveAnswers, ResolveResult, Resolver};
use async_std::task::block_on;
use async_std_resolver::{resolver, resolver_from_system_conf, AsyncStdResolver as AsyncResolver};
use futures::future::BoxFuture;
use qiniu_http::ResponseErrorKind as HTTPResponseErrorKind;
use std::{any::Any, fmt};
pub use trust_dns_resolver;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    error::ResolveError,
};

#[cfg_attr(
    feature = "docs",
    doc(cfg(all(feature = "trust_dns", feature = "async")))
)]
pub struct TrustDnsResolver {
    #[cfg(feature = "async")]
    resolver: AsyncResolver,
}

type TrustDnsResolveResult<T> = Result<T, ResolveError>;

impl TrustDnsResolver {
    #[inline]
    pub async fn new(config: ResolverConfig, options: ResolverOpts) -> TrustDnsResolveResult<Self> {
        Ok(Self {
            resolver: resolver(config, options).await?,
        })
    }

    #[inline]
    pub async fn default() -> TrustDnsResolveResult<Self> {
        Ok(Self {
            resolver: resolver(Default::default(), Default::default()).await?,
        })
    }

    #[inline]
    pub async fn from_system_conf() -> TrustDnsResolveResult<Self> {
        Ok(Self {
            resolver: resolver_from_system_conf().await?,
        })
    }
}

impl Resolver for TrustDnsResolver {
    fn resolve(&self, domain: &str) -> ResolveResult {
        block_on(async move { self.async_resolve(domain).await })
    }

    fn async_resolve<'a>(&'a self, domain: &'a str) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            Ok(ResolveAnswers::new(
                self.resolver
                    .lookup_ip(domain)
                    .await
                    .map_err(convert_trust_dns_error_to_response_error)?
                    .iter()
                    .collect(),
            ))
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

impl fmt::Debug for TrustDnsResolver {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrustDnsResolver").finish()
    }
}

#[inline]
fn convert_trust_dns_error_to_response_error(err: ResolveError) -> ResponseError {
    ResponseError::new(HTTPResponseErrorKind::DNSServerError.into(), err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashSet,
        error::Error,
        net::{IpAddr, Ipv4Addr, Ipv6Addr},
        result::Result,
    };
    use trust_dns_resolver::config::{LookupIpStrategy, NameServerConfigGroup};

    const DOMAIN: &str = "dns.alidns.com";
    const IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(223, 5, 5, 5)),
        IpAddr::V4(Ipv4Addr::new(223, 6, 6, 6)),
        IpAddr::V6(Ipv6Addr::new(0x2400, 0x3200, 0, 0, 0, 0, 0, 1)),
        IpAddr::V6(Ipv6Addr::new(0x2400, 0x3200, 0xbaba, 0, 0, 0, 0, 1)),
    ];

    #[tokio::test]
    async fn test_trust_dns_resolver() -> Result<(), Box<dyn Error>> {
        let resolver = TrustDnsResolver::new(
            ResolverConfig::from_parts(
                None,
                vec![],
                NameServerConfigGroup::from_ips_clear(IPS, 53, true),
            ),
            ResolverOpts {
                ip_strategy: LookupIpStrategy::Ipv4AndIpv6,
                ..Default::default()
            },
        )
        .await?;
        let ips = resolver.async_resolve(DOMAIN).await?;
        assert_eq!(make_set(ips.ip_addrs()), make_set(IPS));
        Ok(())
    }

    #[inline]
    fn make_set(ips: impl AsRef<[IpAddr]>) -> HashSet<IpAddr> {
        let mut h = HashSet::new();
        h.extend(ips.as_ref());
        h
    }
}
