#[cfg_attr(feature = "docs", doc(cfg(feature = "c_ares")))]
pub use c_ares;
#[cfg_attr(feature = "docs", doc(cfg(feature = "c_ares")))]
pub use c_ares_resolver;

use super::{
    super::ResponseError, owned_resolver_options::OwnedResolveOptions, ResolveOptions, ResolveResult, Resolver,
};
use c_ares::{
    AddressFamily::{INET, INET6},
    Error as CAresError, HostResults as CAresHostResults,
};
use c_ares_resolver::{Error as CAresResolverError, Options as CAresResolverOptions, Resolver as CallbackResolver};
use cfg_if::cfg_if;
use qiniu_http::ResponseErrorKind as HttpResponseErrorKind;
use std::{
    fmt,
    net::IpAddr,
    sync::{mpsc, Arc},
};

#[cfg(feature = "async")]
use {
    c_ares_resolver::FutureResolver,
    futures::future::{join, BoxFuture},
};

type CAresResolverResult<T> = Result<T, CAresResolverError>;

/// [`c-ares`](https://c-ares.org/) 域名解析器
///
/// 基于 [`c-ares`](https://c-ares.org/) 库的域名解析接口实现
#[cfg_attr(feature = "docs", doc(cfg(feature = "c_ares")))]
#[derive(Clone)]
pub struct CAresResolver(Arc<CAresResolverInner>);

struct CAresResolverInner {
    callback_resolver: CallbackResolver,

    #[cfg(feature = "async")]
    future_resolver: FutureResolver,
}

impl CAresResolver {
    /// 创建 [`c-ares`](https://c-ares.org/) 域名解析器
    #[inline]
    #[cfg(not(feature = "async"))]
    pub fn new_with_options(options: CAresResolverOptions) -> CAresResolverResult<Self> {
        Ok(Self(Arc::new(CAresResolverInner {
            callback_resolver: CallbackResolver::with_options(options)?,
        })))
    }

    /// 创建 [`c-ares`](https://c-ares.org/) 域名解析器
    #[inline]
    #[cfg(any(feature = "async"))]
    pub fn new_with_options(
        sync_options: CAresResolverOptions,
        async_options: CAresResolverOptions,
    ) -> CAresResolverResult<Self> {
        Ok(Self(Arc::new(CAresResolverInner {
            callback_resolver: CallbackResolver::with_options(sync_options)?,
            future_resolver: FutureResolver::with_options(async_options)?,
        })))
    }

    /// 创建默认的 [`c-ares`](https://c-ares.org/) 域名解析器
    #[inline]
    pub fn new() -> CAresResolverResult<Self> {
        cfg_if! {
            if #[cfg(any(feature = "async"))] {
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
        let (tx, rx) = mpsc::channel();
        let tx2 = tx.to_owned();

        self.0.callback_resolver.get_host_by_name(domain, INET, {
            let opts = OwnedResolveOptions::from(opts);
            move |results| {
                tx.send(
                    results
                        .map_err(|err| owned_convert_c_ares_error_to_response_error(err, opts))
                        .map(convert_hosts_to_ip_addrs),
                )
                .unwrap();
            }
        });
        self.0.callback_resolver.get_host_by_name(domain, INET6, {
            let opts = OwnedResolveOptions::from(opts);
            move |results| {
                tx2.send(
                    results
                        .map_err(|err| owned_convert_c_ares_error_to_response_error(err, opts))
                        .map(convert_hosts_to_ip_addrs),
                )
                .unwrap();
            }
        });

        match (rx.recv().unwrap(), rx.recv().unwrap()) {
            (Ok(ip_addrs_1), Ok(ip_addrs_2)) => {
                let mut ip_addrs = ip_addrs_1.to_vec();
                ip_addrs.extend_from_slice(&ip_addrs_2);
                Ok(ip_addrs.into_boxed_slice().into())
            }
            (Ok(ip_addrs), _) => Ok(ip_addrs.into()),
            (_, Ok(ip_addrs)) => Ok(ip_addrs.into()),
            (Err(err), _) => Err(err),
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_resolve<'a>(&'a self, domain: &'a str, opts: ResolveOptions<'a>) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            let task1 = self.0.future_resolver.get_host_by_name(domain, INET);
            let task2 = self.0.future_resolver.get_host_by_name(domain, INET6);
            let (results1, results2) = join(task1, task2).await;
            match (
                results1
                    .map_err(|err| convert_c_ares_error_to_response_error(err, opts))
                    .map(convert_resolver_hosts_to_ip_addrs),
                results2
                    .map_err(|err| convert_c_ares_error_to_response_error(err, opts))
                    .map(convert_resolver_hosts_to_ip_addrs),
            ) {
                (Ok(ip_addrs_1), Ok(ip_addrs_2)) => {
                    let mut ip_addrs = ip_addrs_1.to_vec();
                    ip_addrs.extend_from_slice(&ip_addrs_2);
                    Ok(ip_addrs.into_boxed_slice().into())
                }
                (Ok(ip_addrs), _) => Ok(ip_addrs.into()),
                (_, Ok(ip_addrs)) => Ok(ip_addrs.into()),
                (Err(err), _) => Err(err),
            }
        })
    }
}

fn convert_hosts_to_ip_addrs(results: CAresHostResults) -> Box<[IpAddr]> {
    results.addresses().collect()
}

#[cfg(feature = "async")]
use c_ares_resolver::HostResults as CAresResolverHostResults;

#[cfg(feature = "async")]
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

fn owned_convert_c_ares_error_to_response_error(err: CAresError, opts: OwnedResolveOptions) -> ResponseError {
    let mut err = ResponseError::new(HttpResponseErrorKind::DnsServerError.into(), err);
    if let Some(retried) = opts.retried() {
        err = err.retried(retried);
    }
    err
}

#[cfg(all(test, feature = "async"))]
mod tests {
    use super::*;
    use std::{
        collections::HashSet,
        error::Error,
        net::{IpAddr, Ipv4Addr, Ipv6Addr},
        result::Result,
    };

    const DOMAIN: &str = "dns.alidns.com";
    const IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(223, 5, 5, 5)),
        IpAddr::V4(Ipv4Addr::new(223, 6, 6, 6)),
        IpAddr::V6(Ipv6Addr::new(0x2400, 0x3200, 0, 0, 0, 0, 0, 1)),
        IpAddr::V6(Ipv6Addr::new(0x2400, 0x3200, 0xbaba, 0, 0, 0, 0, 1)),
    ];

    #[test]
    fn test_c_ares_resolver() -> Result<(), Box<dyn Error>> {
        let resolver = CAresResolver::new()?;
        let ips = resolver.resolve(DOMAIN, Default::default())?;
        assert!(is_subset_of(IPS, ips.ip_addrs()));
        Ok(())
    }

    #[tokio::test]
    async fn test_async_c_ares_resolver() -> Result<(), Box<dyn Error>> {
        let resolver = CAresResolver::new()?;
        let ips = resolver.async_resolve(DOMAIN, Default::default()).await?;
        assert!(is_subset_of(IPS, ips.ip_addrs()));
        Ok(())
    }

    fn make_set(ips: impl AsRef<[IpAddr]>) -> HashSet<IpAddr> {
        HashSet::from_iter(ips.as_ref().iter().copied())
    }

    fn is_subset_of(ips1: impl AsRef<[IpAddr]>, ips2: impl AsRef<[IpAddr]>) -> bool {
        make_set(ips1).is_subset(&make_set(ips2))
    }
}
