use super::{
    super::{ResponseError, ResponseErrorKind},
    ResolveOptions, ResolveResult, Resolver,
};
use cfg_if::cfg_if;
use futures::future::BoxFuture;
use qiniu_http::ResponseErrorKind as HttpResponseErrorKind;
use qiniu_utils::async_task::block_on;
use std::fmt;
pub use trust_dns_resolver;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    error::ResolveError,
};

cfg_if! {
    if #[cfg(feature = "async-std-runtime")] {
        use async_std_resolver::{resolver, resolver_from_system_conf, AsyncStdResolver as AsyncResolver};
    } else if #[cfg(feature = "tokio-runtime")] {
        use trust_dns_resolver::TokioAsyncResolver as AsyncResolver;

        async fn resolver(config: ResolverConfig, options: ResolverOpts) -> AsyncResolver {
            AsyncResolver::tokio(config, options)
        }

        async fn resolver_from_system_conf() -> Result<AsyncResolver, ResolveError> {
            AsyncResolver::tokio_from_system_conf()
        }
    }
}

/// [`trust-dns`](https://trust-dns.org/) 域名解析器
///
/// 基于 [`trust-dns`](https://trust-dns.org/) 库的域名解析接口实现，由于该接口只有异步实现，即使使用阻塞接口，也会调用异步实现
#[cfg_attr(
    feature = "docs",
    doc(cfg(all(feature = "trust_dns", any(feature = "async-std-runtime", feature = "tokio-runtime"))))
)]
#[derive(Clone)]
pub struct TrustDnsResolver {
    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    resolver: AsyncResolver,
}

type TrustDnsResolveResult<T> = Result<T, ResolveError>;

impl TrustDnsResolver {
    /// 创建 [`trust-dns`](https://trust-dns.org/) 域名解析器
    #[inline]
    pub async fn new(config: ResolverConfig, options: ResolverOpts) -> Self {
        Self {
            resolver: resolver(config, options).await,
        }
    }

    /// 创建默认的 [`trust-dns`](https://trust-dns.org/) 域名解析器
    #[inline]
    pub async fn default() -> Self {
        Self {
            resolver: resolver(Default::default(), Default::default()).await,
        }
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
        match block_on(async move { self.async_resolve(domain, opts).await }) {
            Ok(Ok(results)) => Ok(results),
            Ok(Err(err)) => Err(err),
            Err(err) => Err(ResponseError::new(ResponseErrorKind::SystemCallError, err)),
        }
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

#[cfg(all(test, feature = "tokio-runtime"))]
mod tests {
    use super::*;
    use crate::test_utils::{make_record_set, make_zone, start_mock_dns_server};
    use anyhow::{anyhow, Result as AnyResult};
    use qiniu_utils::async_task::spawn;
    use std::{
        collections::{HashMap, HashSet},
        net::{IpAddr, Ipv4Addr, SocketAddr},
    };
    use tokio::net::UdpSocket;
    use trust_dns_resolver::config::{LookupIpStrategy, NameServerConfigGroup};
    use trust_dns_server::{
        authority::Catalog,
        proto::rr::{
            rdata::{A, CNAME},
            Name, RData, RecordType,
        },
        ServerFuture,
    };

    const DEFAULT_TTL: u32 = 3600;

    #[qiniu_utils::async_runtime::test]
    async fn test_trust_dns_resolver() -> AnyResult<()> {
        let (dns_server_addr, dns_server) = mock_dns_server().await?;
        let (shutdown_signal, dns_server) = dns_server.graceful();
        let join_handle = spawn(dns_server);

        let mut opts = ResolverOpts::default();
        opts.ip_strategy = LookupIpStrategy::Ipv4AndIpv6;
        let resolver = TrustDnsResolver::new(
            ResolverConfig::from_parts(
                None,
                vec![],
                NameServerConfigGroup::from_ips_clear([dns_server_addr.ip()].as_slice(), dns_server_addr.port(), true),
            ),
            opts,
        )
        .await;

        {
            let ips = resolver.async_resolve("dns.alidns.com", Default::default()).await?;
            assert_eq!(
                make_set(ips.ip_addrs()),
                make_set([
                    IpAddr::V4(Ipv4Addr::new(2, 3, 4, 5)),
                    IpAddr::V4(Ipv4Addr::new(3, 4, 5, 6)),
                ])
            );
        }

        {
            let ips = resolver.async_resolve("alidns.com", Default::default()).await?;
            assert_eq!(
                make_set(ips.ip_addrs()),
                make_set([IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)),])
            );
        }

        {
            let ips = resolver.async_resolve("www.alidns.com", Default::default()).await?;
            assert_eq!(
                make_set(ips.ip_addrs()),
                make_set([IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)),])
            );
        }

        shutdown_signal.shutdown().await;
        join_handle.await??;
        Ok(())
    }

    fn make_set(ips: impl AsRef<[IpAddr]>) -> HashSet<IpAddr> {
        let mut h = HashSet::new();
        h.extend(ips.as_ref());
        h
    }

    async fn mock_dns_server() -> AnyResult<(SocketAddr, ServerFuture<Catalog>)> {
        let root_name = Name::from_str_relaxed("alidns.com")?;
        let root_record_set = make_record_set(
            root_name.to_owned(),
            RecordType::A,
            DEFAULT_TTL,
            [(root_name.to_owned(), DEFAULT_TTL, RData::A(A::new(1, 2, 3, 4)))],
        );
        let sub_name = Name::from_str_relaxed("dns.alidns.com.")?;
        let sub_record_set = make_record_set(
            sub_name.to_owned(),
            RecordType::A,
            DEFAULT_TTL,
            [
                (sub_name.to_owned(), DEFAULT_TTL, RData::A(A::new(2, 3, 4, 5))),
                (sub_name.to_owned(), DEFAULT_TTL, RData::A(A::new(3, 4, 5, 6))),
            ],
        );
        let other_names = Name::from_str_relaxed("*.alidns.com.")?;
        let other_record_set = make_record_set(
            other_names.to_owned(),
            RecordType::CNAME,
            DEFAULT_TTL,
            [(
                other_names.to_owned(),
                DEFAULT_TTL,
                RData::CNAME(CNAME(root_name.to_owned())),
            )],
        );

        let zone = make_zone(
            root_name.to_owned(),
            [
                (root_name.to_owned(), RecordType::A, root_record_set),
                (sub_name.to_owned(), RecordType::A, sub_record_set),
                (other_names.to_owned(), RecordType::CNAME, other_record_set),
            ],
        )
        .map_err(|err| anyhow!(err))?;
        let udp_socket = UdpSocket::bind("127.0.0.1:0").await?;
        let socket_addr = udp_socket.local_addr()?;
        let mut zones = HashMap::with_capacity(2);
        zones.insert(root_name, zone);
        Ok((socket_addr, start_mock_dns_server(udp_socket, zones)))
    }
}
