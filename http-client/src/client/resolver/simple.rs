use super::{super::ResponseError, ResolveOptions, ResolveResult, Resolver};
use dns_lookup::lookup_host;
use qiniu_http::ResponseErrorKind as HttpResponseErrorKind;
use std::io::Error as IOError;

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
use futures::future::BoxFuture;

/// 简单域名解析器
///
/// 基于 [`libc`](https://man7.org/linux/man-pages/man7/libc.7.html) 库的域名解析接口实现
#[derive(Default, Debug, Clone, Copy)]
pub struct SimpleResolver;

impl Resolver for SimpleResolver {
    #[inline]
    fn resolve(&self, domain: &str, opts: ResolveOptions) -> ResolveResult {
        lookup_host(domain)
            .map(|ips| ips.into_boxed_slice().into())
            .map_err(|err| convert_io_error_to_response_error(err, opts))
    }

    #[inline]
    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
    )]
    fn async_resolve<'a>(&'a self, domain: &'a str, opts: ResolveOptions<'a>) -> BoxFuture<'a, ResolveResult> {
        let resolver = self.to_owned();
        let domain = domain.to_owned();
        let retried = opts.retried.cloned();
        Box::pin(async move {
            match qiniu_utils::async_task::spawn_blocking(move || {
                let mut opts_builder = ResolveOptions::builder();
                if let Some(retried) = &retried {
                    opts_builder.retried(retried);
                }
                resolver.resolve(&domain, opts_builder.build())
            })
            .await
            {
                Ok(Ok(results)) => Ok(results),
                Ok(Err(err)) => Err(err),
                Err(err) => Err(ResponseError::new(
                    super::super::ResponseErrorKind::SystemCallError,
                    err,
                )),
            }
        })
    }
}

fn convert_io_error_to_response_error(err: IOError, opts: ResolveOptions) -> ResponseError {
    let mut err = ResponseError::new(HttpResponseErrorKind::DnsServerError.into(), err);
    if let Some(retried) = opts.retried() {
        err = err.retried(retried);
    }
    err
}

#[cfg(all(test, any(feature = "async-std-runtime", feature = "tokio-runtime")))]
mod tests {
    use super::*;
    use anyhow::Result as AnyResult;
    use std::{
        collections::HashSet,
        net::{IpAddr, Ipv4Addr},
    };

    const DOMAIN: &str = "dns.alidns.com";
    const IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(223, 5, 5, 5)),
        IpAddr::V4(Ipv4Addr::new(223, 6, 6, 6)),
    ];

    #[test]
    fn test_simple_resolver() -> AnyResult<()> {
        let resolver = SimpleResolver;
        let ips = resolver.resolve(DOMAIN, Default::default())?;
        assert!(is_subset_of(IPS, ips.ip_addrs()));
        Ok(())
    }

    fn make_set(ips: impl AsRef<[IpAddr]>) -> HashSet<IpAddr> {
        let mut h = HashSet::new();
        h.extend(ips.as_ref());
        h
    }

    fn is_subset_of(ips1: impl AsRef<[IpAddr]>, ips2: impl AsRef<[IpAddr]>) -> bool {
        make_set(ips1).is_subset(&make_set(ips2))
    }
}
