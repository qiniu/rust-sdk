#[cfg_attr(feature = "docs", doc(cfg(feature = "c_ares")))]
pub use c_ares;
#[cfg_attr(feature = "docs", doc(cfg(feature = "c_ares")))]
pub use c_ares_resolver;

use super::{super::ResponseError, ResolveOptions, ResolveResult, Resolver};
use c_ares::{AddressFamily::UNSPEC, Error as CAresError};
use c_ares_resolver::{BlockingResolver, Error as CAresResolverError, Options as CAresResolverOptions};
use cfg_if::cfg_if;
use qiniu_http::ResponseErrorKind as HttpResponseErrorKind;
use std::{fmt, net::IpAddr, sync::Arc};

#[cfg(feature = "async")]
use {c_ares_resolver::FutureResolver, futures::future::BoxFuture};

type CAresResolverResult<T> = Result<T, CAresResolverError>;

/// [`c-ares`](https://c-ares.org/) 域名解析器
///
/// 基于 [`c-ares`](https://c-ares.org/) 库的域名解析接口实现
#[cfg_attr(feature = "docs", doc(cfg(feature = "c_ares")))]
#[derive(Clone)]
pub struct CAresResolver(Arc<CAresResolverInner>);

struct CAresResolverInner {
    resolver: BlockingResolver,

    #[cfg(feature = "async")]
    future_resolver: FutureResolver,
}

impl CAresResolver {
    /// 创建 [`c-ares`](https://c-ares.org/) 域名解析器
    #[inline]
    #[cfg(not(feature = "async"))]
    pub fn new_with_options(options: CAresResolverOptions) -> CAresResolverResult<Self> {
        Ok(Self(Arc::new(CAresResolverInner {
            resolver: BlockingResolver::with_options(options)?,
        })))
    }

    /// 创建 [`c-ares`](https://c-ares.org/) 域名解析器
    #[inline]
    #[cfg(not(feature = "async"))]
    pub fn new_with_resolver(resolver: BlockingResolver) -> Self {
        Self(Arc::new(CAresResolverInner { resolver }))
    }

    /// 创建 [`c-ares`](https://c-ares.org/) 域名解析器
    #[inline]
    #[cfg(feature = "async")]
    pub fn new_with_options(
        sync_options: CAresResolverOptions,
        async_options: CAresResolverOptions,
    ) -> CAresResolverResult<Self> {
        Ok(Self(Arc::new(CAresResolverInner {
            resolver: BlockingResolver::with_options(sync_options)?,
            future_resolver: FutureResolver::with_options(async_options)?,
        })))
    }

    /// 创建 [`c-ares`](https://c-ares.org/) 域名解析器
    #[inline]
    #[cfg(feature = "async")]
    pub fn new_with_resolvers(resolver: BlockingResolver, future_resolver: FutureResolver) -> Self {
        Self(Arc::new(CAresResolverInner {
            resolver,
            future_resolver,
        }))
    }

    /// 创建默认的 [`c-ares`](https://c-ares.org/) 域名解析器
    #[inline]
    pub fn new() -> CAresResolverResult<Self> {
        cfg_if! {
            if #[cfg(feature = "async")] {
                Self::new_with_options(CAresResolverOptions::new(), CAresResolverOptions::new())
            } else {
                Self::new_with_options(CAresResolverOptions::new())
            }
        }
    }
}

impl fmt::Debug for CAresResolver {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CAresResolver").finish()
    }
}

impl Resolver for CAresResolver {
    fn resolve(&self, domain: &str, opts: ResolveOptions) -> ResolveResult {
        self.0
            .resolver
            .get_host_by_name(domain, UNSPEC)
            .map(convert_resolver_hosts_to_ip_addrs)
            .or_else(convert_no_data)
            .map_err(|err| convert_c_ares_error_to_response_error(err, opts))
            .map(|answers| answers.into())
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_resolve<'a>(&'a self, domain: &'a str, opts: ResolveOptions<'a>) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            self.0
                .future_resolver
                .get_host_by_name(domain, UNSPEC)
                .await
                .map(convert_resolver_hosts_to_ip_addrs)
                .or_else(convert_no_data)
                .map_err(|err| convert_c_ares_error_to_response_error(err, opts))
                .map(|answers| answers.into())
        })
    }
}

fn convert_no_data(err: CAresError) -> Result<Box<[IpAddr]>, CAresError> {
    if err == CAresError::ENODATA {
        Ok(Default::default())
    } else {
        Err(err)
    }
}

use c_ares_resolver::HostResults as CAresResolverHostResults;

fn convert_resolver_hosts_to_ip_addrs(results: CAresResolverHostResults) -> Box<[IpAddr]> {
    results.addresses.into_boxed_slice()
}

fn convert_c_ares_error_to_response_error(err: CAresError, opts: ResolveOptions) -> ResponseError {
    let mut err = ResponseError::new(HttpResponseErrorKind::DnsServerError.into(), err);
    if let Some(retried) = opts.retried() {
        err = err.retried(retried);
    }
    err
}

#[cfg(all(test, feature = "async"))]
mod tests {
    use super::*;
    use crate::test_utils::{make_record_set, make_zone, start_mock_dns_server};
    use anyhow::{anyhow, Result as AnyResult};
    use futures::future::abortable;
    use std::{
        collections::{HashMap, HashSet},
        net::{IpAddr, Ipv4Addr, SocketAddr},
    };
    use tokio::{net::UdpSocket, task::spawn, task::spawn_blocking};
    use trust_dns_server::{
        authority::Catalog,
        proto::rr::{Name, RData, RecordType},
        ServerFuture,
    };

    const DEFAULT_TTL: u32 = 3600;

    #[tokio::test]
    async fn test_c_ares_resolver() -> AnyResult<()> {
        let (dns_server_addr, dns_server) = mock_dns_server().await?;
        let (dns_server, abort_handle) = abortable(async move { dns_server.block_until_done().await });
        let join_handle = spawn(dns_server);

        spawn_blocking(move || -> AnyResult<()> {
            let callback_resolver = BlockingResolver::new()?;
            callback_resolver.set_servers(&[&dns_server_addr.to_string()])?;
            let future_resolver = FutureResolver::new()?;
            let resolver = CAresResolver::new_with_resolvers(callback_resolver, future_resolver);
            {
                let ips = resolver.resolve("dns.alidns.com", Default::default())?;
                assert_eq!(
                    make_set(ips.ip_addrs()),
                    make_set([
                        IpAddr::V4(Ipv4Addr::new(2, 3, 4, 5)),
                        IpAddr::V4(Ipv4Addr::new(3, 4, 5, 6)),
                    ])
                );
            }

            {
                let ips = resolver.resolve("alidns.com", Default::default())?;
                assert_eq!(
                    make_set(ips.ip_addrs()),
                    make_set([IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)),])
                );
            }

            Ok(())
        })
        .await??;
        abort_handle.abort();
        let _ = join_handle.await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_async_c_ares_resolver() -> AnyResult<()> {
        let (dns_server_addr, dns_server) = mock_dns_server().await?;
        let (dns_server, abort_handle) = abortable(async move { dns_server.block_until_done().await });
        let join_handle = spawn(dns_server);

        let callback_resolver = BlockingResolver::new()?;
        let future_resolver = FutureResolver::new()?;
        future_resolver.set_servers(&[&dns_server_addr.to_string()])?;
        let resolver = CAresResolver::new_with_resolvers(callback_resolver, future_resolver);
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

        abort_handle.abort();
        let _ = join_handle.await?;
        Ok(())
    }

    fn make_set(ips: impl AsRef<[IpAddr]>) -> HashSet<IpAddr> {
        HashSet::from_iter(ips.as_ref().iter().copied())
    }

    async fn mock_dns_server() -> AnyResult<(SocketAddr, ServerFuture<Catalog>)> {
        let root_name = Name::from_str_relaxed("alidns.com")?;
        let root_record_set = make_record_set(
            root_name.to_owned(),
            RecordType::A,
            DEFAULT_TTL,
            [(root_name.to_owned(), DEFAULT_TTL, RData::A(Ipv4Addr::new(1, 2, 3, 4)))],
        );
        let sub_name = Name::from_str_relaxed("dns.alidns.com.")?;
        let sub_record_set = make_record_set(
            sub_name.to_owned(),
            RecordType::A,
            DEFAULT_TTL,
            [
                (sub_name.to_owned(), DEFAULT_TTL, RData::A(Ipv4Addr::new(2, 3, 4, 5))),
                (sub_name.to_owned(), DEFAULT_TTL, RData::A(Ipv4Addr::new(3, 4, 5, 6))),
            ],
        );

        let zone = make_zone(
            root_name.to_owned(),
            [
                (root_name.to_owned(), RecordType::A, root_record_set),
                (sub_name.to_owned(), RecordType::A, sub_record_set),
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
