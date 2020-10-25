use super::super::super::{DomainWithPort, IpAddrWithPort};

#[derive(Debug, Clone)]
pub(super) enum DomainOrIpAddr {
    Domain(DomainInfo),
    IpAddr(IpAddrWithPort),
}

#[derive(Debug, Clone)]
pub(super) struct DomainInfo {
    domain_with_port: DomainWithPort,
    resolved_ips: Vec<IpAddrWithPort>,
}

impl DomainOrIpAddr {
    #[inline]
    pub(super) fn new_from_domain(
        domain_with_port: DomainWithPort,
        resolved_ips: Vec<IpAddrWithPort>,
    ) -> Self {
        Self::Domain(DomainInfo {
            domain_with_port,
            resolved_ips,
        })
    }

    #[inline]
    pub(super) fn as_domain(&self) -> Option<&DomainInfo> {
        match self {
            Self::Domain(domain) => Some(domain),
            _ => None,
        }
    }
}

impl From<IpAddrWithPort> for DomainOrIpAddr {
    #[inline]
    fn from(ip_addr: IpAddrWithPort) -> Self {
        Self::IpAddr(ip_addr)
    }
}

impl DomainInfo {
    #[inline]
    pub(super) fn domain_with_port(&self) -> &DomainWithPort {
        &self.domain_with_port
    }

    #[inline]
    pub(super) fn resolved_ips(&self) -> &[IpAddrWithPort] {
        &self.resolved_ips
    }
}
