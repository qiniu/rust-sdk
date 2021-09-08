use super::{super::ResponseError, ResolveAnswers, ResolveResult, Resolver};
use dns_lookup::lookup_host;
use qiniu_http::ResponseErrorKind as HTTPResponseErrorKind;
use std::any::Any;

#[derive(Default, Debug, Clone, Copy)]
pub struct SimpleResolver;

impl Resolver for SimpleResolver {
    #[inline]
    fn resolve(&self, domain: &str) -> ResolveResult {
        let ip_addrs = lookup_host(domain)
            .map(|ips| ips.into_boxed_slice())
            .map_err(|err| ResponseError::new(HTTPResponseErrorKind::DNSServerError.into(), err))?;
        Ok(ResolveAnswers::new(ip_addrs))
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

#[cfg(test)]
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
    fn test_simple_resolver() -> Result<(), Box<dyn Error>> {
        let resolver = SimpleResolver;
        let ips = resolver.resolve(DOMAIN)?;
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
