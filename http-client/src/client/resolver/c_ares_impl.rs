pub extern crate c_ares;
pub extern crate c_ares_resolver;

use super::{ResolveError, ResolveResult, Resolver};
use c_ares::{
    AddressFamily::{INET, INET6},
    Error as CAresError, HostResults as CAresHostResults,
};
use c_ares_resolver::{
    Error as CAresResolverError, HostResults as CAresResolverHostResults,
    Options as CAresResolverOptions, Resolver as CallbackResolver,
};
use std::{
    any::Any,
    fmt,
    io::{Error as IOError, ErrorKind as IOErrorKind},
    net::IpAddr,
    result,
    sync::mpsc,
};

#[cfg(feature = "async")]
use {
    c_ares_resolver::FutureResolver,
    futures::future::{join, BoxFuture},
};

type CAresResolverResult<T> = result::Result<T, CAresResolverError>;

pub struct CAresResolver {
    callback_resolver: CallbackResolver,

    #[cfg(feature = "async")]
    future_resolver: FutureResolver,
}

impl CAresResolver {
    #[inline]
    #[cfg(not(feature = "async"))]
    pub fn new(options: CAresResolverOptions) -> CAresResolverResult<Self> {
        Ok(Self {
            callback_resolver: CallbackResolver::with_options(options)?,
        })
    }

    #[inline]
    #[cfg(any(feature = "async"))]
    pub fn new(
        sync_options: CAresResolverOptions,
        async_options: CAresResolverOptions,
    ) -> CAresResolverResult<Self> {
        Ok(Self {
            callback_resolver: CallbackResolver::with_options(sync_options)?,
            future_resolver: FutureResolver::with_options(async_options)?,
        })
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

        self.callback_resolver
            .get_host_by_name(domain, INET, move |results| {
                tx.send(
                    results
                        .map_err(convert_c_ares_error_to_resolve_error)
                        .map(convert_hosts_to_ip_addrs),
                )
                .unwrap();
            });
        self.callback_resolver
            .get_host_by_name(domain, INET6, move |results| {
                tx2.send(
                    results
                        .map_err(convert_c_ares_error_to_resolve_error)
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
            let task1 = self.future_resolver.get_host_by_name(domain, INET);
            let task2 = self.future_resolver.get_host_by_name(domain, INET6);
            let (results1, results2) = join(task1, task2).await;
            match (
                results1
                    .map_err(convert_c_ares_error_to_resolve_error)
                    .map(convert_resolver_hosts_to_ip_addrs),
                results2
                    .map_err(convert_c_ares_error_to_resolve_error)
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

#[inline]
fn convert_resolver_hosts_to_ip_addrs(results: CAresResolverHostResults) -> Box<[IpAddr]> {
    results.addresses.into_boxed_slice()
}

#[inline]
fn convert_c_ares_error_to_resolve_error(err: CAresError) -> ResolveError {
    IOError::new(IOErrorKind::Other, err).into()
}