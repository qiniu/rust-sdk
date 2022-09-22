use super::{super::ResponseError, ResolveOptions, ResolveResult, Resolver};
use async_std::task::block_on;
use async_std_resolver::{resolver, resolver_from_system_conf, AsyncStdResolver as AsyncResolver};
use futures::future::BoxFuture;
use qiniu_http::ResponseErrorKind as HttpResponseErrorKind;
use std::fmt;
pub use trust_dns_resolver;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    error::ResolveError,
};

/// [`trust-dns`](https://trust-dns.org/) 域名解析器
///
/// 基于 [`trust-dns`](https://trust-dns.org/) 库的域名解析接口实现，由于该接口只有异步实现，即使使用阻塞接口，也会调用异步实现
#[cfg_attr(feature = "docs", doc(cfg(all(feature = "trust_dns", feature = "async"))))]
#[derive(Clone)]
pub struct TrustDnsResolver {
    #[cfg(feature = "async")]
    resolver: AsyncResolver,
}

type TrustDnsResolveResult<T> = Result<T, ResolveError>;

impl TrustDnsResolver {
    /// 创建 [`trust-dns`](https://trust-dns.org/) 域名解析器
    #[inline]
    pub async fn new(config: ResolverConfig, options: ResolverOpts) -> TrustDnsResolveResult<Self> {
        Ok(Self {
            resolver: resolver(config, options).await?,
        })
    }

    /// 创建默认的 [`trust-dns`](https://trust-dns.org/) 域名解析器
    #[inline]
    pub async fn default() -> TrustDnsResolveResult<Self> {
        Ok(Self {
            resolver: resolver(Default::default(), Default::default()).await?,
        })
    }

    /// 通过系统的 system.conf 文件创建 [`trust-dns`](https://trust-dns.org/) 域名解析器
    #[inline]
    pub async fn from_system_conf() -> TrustDnsResolveResult<Self> {
        Ok(Self {
            resolver: resolver_from_system_conf().await?,
        })
    }
}

impl Resolver for TrustDnsResolver {
    fn resolve(&self, domain: &str, opts: ResolveOptions) -> ResolveResult {
        block_on(async move { self.async_resolve(domain, opts).await })
    }

    fn async_resolve<'a>(&'a self, domain: &'a str, opts: ResolveOptions<'a>) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            Ok(self
                .resolver
                .lookup_ip(domain)
                .await
                .map_err(|err| convert_trust_dns_error_to_response_error(err, opts))?
                .iter()
                .collect::<Vec<_>>()
                .into())
        })
    }
}

impl fmt::Debug for TrustDnsResolver {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrustDnsResolver").finish()
    }
}

fn convert_trust_dns_error_to_response_error(err: ResolveError, opts: ResolveOptions) -> ResponseError {
    let mut err = ResponseError::new(HttpResponseErrorKind::DnsServerError.into(), err);
    if let Some(retried) = opts.retried() {
        err = err.retried(retried);
    }
    err
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
        let mut opts = ResolverOpts::default();
        opts.ip_strategy = LookupIpStrategy::Ipv4AndIpv6;
        let resolver = TrustDnsResolver::new(
            ResolverConfig::from_parts(None, vec![], NameServerConfigGroup::from_ips_clear(IPS, 53, true)),
            opts,
        )
        .await?;
        let ips = resolver.async_resolve(DOMAIN, Default::default()).await?;
        assert_eq!(make_set(ips.ip_addrs()), make_set(IPS));
        Ok(())
    }

    fn make_set(ips: impl AsRef<[IpAddr]>) -> HashSet<IpAddr> {
        let mut h = HashSet::new();
        h.extend(ips.as_ref());
        h
    }
}
