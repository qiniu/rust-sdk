#[cfg_attr(feature = "docs", doc(cfg(r#c_ares)))]
pub extern crate c_ares;
#[cfg_attr(feature = "docs", doc(cfg(r#c_ares)))]
pub extern crate c_ares_resolver;

use super::{super::ResponseError, ResolveResult, Resolver};
use c_ares::{
    AddressFamily::{INET, INET6},
    Error as CAresError, HostResults as CAresHostResults,
};
use c_ares_resolver::{
    Error as CAresResolverError, Options as CAresResolverOptions, Resolver as CallbackResolver,
};
use cfg_if::cfg_if;
use qiniu_http::ResponseErrorKind as HTTPResponseErrorKind;
use std::{any::Any, fmt, net::IpAddr, result, sync::mpsc, sync::Arc};

#[cfg(feature = "async")]
use {
    c_ares_resolver::FutureResolver,
    futures::future::{join, BoxFuture},
};

type CAresResolverResult<T> = result::Result<T, CAresResolverError>;

#[cfg_attr(feature = "docs", doc(cfg(r#c_ares)))]
#[derive(Clone)]
pub struct CAresResolver {
    inner: Arc<CAresResolverInner>,
}

struct CAresResolverInner {
    callback_resolver: CallbackResolver,

    #[cfg(feature = "async")]
    future_resolver: FutureResolver,
}

impl CAresResolver {
    #[inline]
    #[cfg(not(feature = "async"))]
    pub fn new_with_options(options: CAresResolverOptions) -> CAresResolverResult<Self> {
        Ok(Self {
            inner: Arc::new(CAresResolverInner {
                callback_resolver: CallbackResolver::with_options(options)?,
            }),
        })
    }

    #[inline]
    #[cfg(any(feature = "async"))]
    pub fn new_with_options(
        sync_options: CAresResolverOptions,
        async_options: CAresResolverOptions,
    ) -> CAresResolverResult<Self> {
        Ok(Self {
            inner: Arc::new(CAresResolverInner {
                callback_resolver: CallbackResolver::with_options(sync_options)?,
                future_resolver: FutureResolver::with_options(async_options)?,
            }),
        })
    }

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
    fn resolve(&self, domain: &str) -> ResolveResult {
        let (tx, rx) = mpsc::channel();
        let tx2 = tx.to_owned();

        self.inner
            .callback_resolver
            .get_host_by_name(domain, INET, move |results| {
                tx.send(
                    results
                        .map_err(convert_c_ares_error_to_response_error)
                        .map(convert_hosts_to_ip_addrs),
                )
                .unwrap();
            });
        self.inner
            .callback_resolver
            .get_host_by_name(domain, INET6, move |results| {
                tx2.send(
                    results
                        .map_err(convert_c_ares_error_to_response_error)
                        .map(convert_hosts_to_ip_addrs),
                )
                .unwrap();
            });

        match (rx.recv().unwrap(), rx.recv().unwrap()) {
            (Ok(ip_addrs_1), Ok(ip_addrs_2)) => {
                let mut ip_addrs = ip_addrs_1.to_vec();
                ip_addrs.extend_from_slice(&ip_addrs_2);
                Ok(ip_addrs.into_boxed_slice())
            }
            (Ok(ip_addrs), _) => Ok(ip_addrs),
            (_, Ok(ip_addrs)) => Ok(ip_addrs),
            (Err(err), _) => Err(err),
        }
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_resolve<'a>(&'a self, domain: &'a str) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            let task1 = self.inner.future_resolver.get_host_by_name(domain, INET);
            let task2 = self.inner.future_resolver.get_host_by_name(domain, INET6);
            let (results1, results2) = join(task1, task2).await;
            match (
                results1
                    .map_err(convert_c_ares_error_to_response_error)
                    .map(convert_resolver_hosts_to_ip_addrs),
                results2
                    .map_err(convert_c_ares_error_to_response_error)
                    .map(convert_resolver_hosts_to_ip_addrs),
            ) {
                (Ok(ip_addrs_1), Ok(ip_addrs_2)) => {
                    let mut ip_addrs = ip_addrs_1.to_vec();
                    ip_addrs.extend_from_slice(&ip_addrs_2);
                    Ok(ip_addrs.into_boxed_slice())
                }
                (Ok(ip_addrs), _) => Ok(ip_addrs),
                (_, Ok(ip_addrs)) => Ok(ip_addrs),
                (Err(err), _) => Err(err),
            }
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

#[inline]
fn convert_hosts_to_ip_addrs(results: CAresHostResults) -> Box<[IpAddr]> {
    results.addresses().collect()
}

#[cfg(feature = "async")]
use c_ares_resolver::HostResults as CAresResolverHostResults;

#[inline]
#[cfg(feature = "async")]
fn convert_resolver_hosts_to_ip_addrs(results: CAresResolverHostResults) -> Box<[IpAddr]> {
    results.addresses.into_boxed_slice()
}

#[inline]
fn convert_c_ares_error_to_response_error(err: CAresError) -> ResponseError {
    ResponseError::new(HTTPResponseErrorKind::DNSServerError.into(), err)
}
