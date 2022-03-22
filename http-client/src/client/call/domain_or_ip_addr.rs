use super::super::super::{DomainWithPort, Endpoint, IpAddrWithPort};

#[derive(Debug, Clone)]
pub(super) enum DomainOrIpAddr {
    Domain {
        domain_with_port: DomainWithPort,
        resolved_ips: Vec<IpAddrWithPort>,
    },
    IpAddr(IpAddrWithPort),
}

impl DomainOrIpAddr {
    pub(super) fn new_from_domain(domain_with_port: DomainWithPort, resolved_ips: Vec<IpAddrWithPort>) -> Self {
        Self::Domain {
            domain_with_port,
            resolved_ips,
        }
    }
}

impl From<IpAddrWithPort> for DomainOrIpAddr {
    #[inline]
    fn from(ip_addr: IpAddrWithPort) -> Self {
        Self::IpAddr(ip_addr)
    }
}

impl From<DomainOrIpAddr> for Endpoint {
    #[inline]
    fn from(domain_or_ip_addr: DomainOrIpAddr) -> Self {
        match domain_or_ip_addr {
            DomainOrIpAddr::Domain { domain_with_port, .. } => Endpoint::DomainWithPort(domain_with_port),
            DomainOrIpAddr::IpAddr(ip_addr_with_port) => Endpoint::IpAddrWithPort(ip_addr_with_port),
        }
    }
}
