use std::{
    net::{IpAddr, SocketAddr},
    num::NonZeroU16,
};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DomainWithPort {
    domain: Box<str>,
    port: Option<NonZeroU16>,
}

impl DomainWithPort {
    #[inline]
    pub fn new(domain: impl Into<Box<str>>) -> Self {
        DomainWithPort {
            domain: domain.into(),
            port: None,
        }
    }

    #[inline]
    pub fn new_with_port(domain: impl Into<Box<str>>, port: u16) -> Self {
        DomainWithPort {
            domain: domain.into(),
            port: NonZeroU16::new(port),
        }
    }

    #[inline]
    pub fn domain(&self) -> &str {
        &self.domain
    }

    #[inline]
    pub fn port(&self) -> Option<NonZeroU16> {
        self.port
    }

    #[inline]
    pub fn into_domain_and_port(self) -> (String, Option<NonZeroU16>) {
        (self.domain.into(), self.port)
    }
}

impl From<Box<str>> for DomainWithPort {
    #[inline]
    fn from(domain: Box<str>) -> Self {
        Self::new(domain)
    }
}

impl From<(Box<str>, u16)> for DomainWithPort {
    #[inline]
    fn from(domain_with_port: (Box<str>, u16)) -> Self {
        Self::new_with_port(domain_with_port.0, domain_with_port.1)
    }
}

impl From<String> for DomainWithPort {
    #[inline]
    fn from(domain: String) -> Self {
        Self::new(domain)
    }
}

impl From<(String, u16)> for DomainWithPort {
    #[inline]
    fn from(domain_with_port: (String, u16)) -> Self {
        Self::new_with_port(domain_with_port.0, domain_with_port.1)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct IpAddrWithPort {
    ip_addr: IpAddr,
    port: Option<NonZeroU16>,
}

impl IpAddrWithPort {
    #[inline]
    pub fn new(ip_addr: IpAddr) -> Self {
        IpAddrWithPort {
            ip_addr,
            port: None,
        }
    }

    #[inline]
    pub fn new_with_port(ip_addr: IpAddr, port: u16) -> Self {
        IpAddrWithPort {
            ip_addr,
            port: NonZeroU16::new(port),
        }
    }

    #[inline]
    pub fn ip_addr(&self) -> IpAddr {
        self.ip_addr
    }

    #[inline]
    pub fn port(&self) -> Option<NonZeroU16> {
        self.port
    }
}

impl From<IpAddr> for IpAddrWithPort {
    #[inline]
    fn from(ip_addr: IpAddr) -> Self {
        Self::new(ip_addr)
    }
}

impl From<IpAddrWithPort> for IpAddr {
    #[inline]
    fn from(ip_addr_with_port: IpAddrWithPort) -> Self {
        ip_addr_with_port.ip_addr()
    }
}

impl From<SocketAddr> for IpAddrWithPort {
    #[inline]
    fn from(socket_addr: SocketAddr) -> Self {
        Self::new_with_port(socket_addr.ip(), socket_addr.port())
    }
}

impl From<IpAddrWithPort> for SocketAddr {
    #[inline]
    fn from(ip_addr_with_port: IpAddrWithPort) -> Self {
        Self::new(
            ip_addr_with_port.ip_addr(),
            ip_addr_with_port.port().map_or(0, |port| port.get()),
        )
    }
}

impl From<(IpAddr, u16)> for IpAddrWithPort {
    #[inline]
    fn from(ip_addr_with_port: (IpAddr, u16)) -> Self {
        Self::new_with_port(ip_addr_with_port.0, ip_addr_with_port.1)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Endpoint {
    DomainWithPort(DomainWithPort),
    IpAddrWithPort(IpAddrWithPort),
}

impl Endpoint {
    #[inline]
    pub fn new_from_domain(domain: impl Into<Box<str>>) -> Self {
        Self::DomainWithPort(DomainWithPort {
            domain: domain.into(),
            port: None,
        })
    }

    #[inline]
    pub fn new_from_domain_with_port(domain: impl Into<Box<str>>, port: u16) -> Self {
        Self::DomainWithPort(DomainWithPort {
            domain: domain.into(),
            port: NonZeroU16::new(port),
        })
    }

    #[inline]
    pub fn new_from_ip_addr(ip_addr: IpAddr) -> Self {
        Self::IpAddrWithPort(IpAddrWithPort {
            ip_addr: ip_addr.into(),
            port: None,
        })
    }

    #[inline]
    pub fn new_from_socket_addr(addr: SocketAddr) -> Self {
        Self::IpAddrWithPort(IpAddrWithPort {
            ip_addr: addr.ip(),
            port: NonZeroU16::new(addr.port()),
        })
    }

    #[inline]
    pub fn domain(&self) -> Option<&str> {
        match self {
            Self::DomainWithPort(domain_with_port) => Some(domain_with_port.domain()),
            _ => None,
        }
    }

    #[inline]
    pub fn ip_addr(&self) -> Option<IpAddr> {
        match self {
            Self::IpAddrWithPort(ip_addr_with_port) => Some(ip_addr_with_port.ip_addr()),
            _ => None,
        }
    }

    #[inline]
    pub fn port(&self) -> Option<NonZeroU16> {
        match self {
            Self::DomainWithPort(domain_with_port) => domain_with_port.port(),
            Self::IpAddrWithPort(ip_addr_with_port) => ip_addr_with_port.port(),
        }
    }
}

impl From<DomainWithPort> for Endpoint {
    #[inline]
    fn from(domain_with_port: DomainWithPort) -> Self {
        Self::DomainWithPort(domain_with_port)
    }
}

impl From<IpAddrWithPort> for Endpoint {
    #[inline]
    fn from(ip_addr_with_port: IpAddrWithPort) -> Self {
        Self::IpAddrWithPort(ip_addr_with_port)
    }
}

impl From<Box<str>> for Endpoint {
    #[inline]
    fn from(domain: Box<str>) -> Self {
        DomainWithPort::new(domain).into()
    }
}

impl From<(Box<str>, u16)> for Endpoint {
    #[inline]
    fn from(domain_with_port: (Box<str>, u16)) -> Self {
        DomainWithPort::new_with_port(domain_with_port.0, domain_with_port.1).into()
    }
}

impl From<String> for Endpoint {
    #[inline]
    fn from(domain: String) -> Self {
        DomainWithPort::new(domain).into()
    }
}

impl From<(String, u16)> for Endpoint {
    #[inline]
    fn from(domain_with_port: (String, u16)) -> Self {
        DomainWithPort::new_with_port(domain_with_port.0, domain_with_port.1).into()
    }
}

impl From<IpAddr> for Endpoint {
    #[inline]
    fn from(ip_addr: IpAddr) -> Self {
        IpAddrWithPort::new(ip_addr).into()
    }
}

impl From<SocketAddr> for Endpoint {
    #[inline]
    fn from(socket_addr: SocketAddr) -> Self {
        IpAddrWithPort::new_with_port(socket_addr.ip(), socket_addr.port()).into()
    }
}
