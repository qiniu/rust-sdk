use super::{super::ResponseError, ResolveOptions, ResolveResult, Resolver};
use dns_lookup::lookup_host;
use qiniu_http::ResponseErrorKind as HttpResponseErrorKind;
use std::io::Error as IOError;

#[cfg(feature = "async")]
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
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_resolve<'a>(&'a self, domain: &'a str, opts: ResolveOptions<'a>) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move { self.resolve(domain, opts) })
    }
}

fn convert_io_error_to_response_error(err: IOError, opts: ResolveOptions) -> ResponseError {
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
        net::{IpAddr, Ipv4Addr},
        result::Result,
    };

    const DOMAIN: &str = "dns.alidns.com";
    const IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(223, 5, 5, 5)),
        IpAddr::V4(Ipv4Addr::new(223, 6, 6, 6)),
    ];

    #[test]
    fn test_simple_resolver() -> Result<(), Box<dyn Error>> {
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
