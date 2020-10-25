use super::{super::APIResult, Endpoint, Region, RegionProvider};
use std::net::{IpAddr, SocketAddr};

#[derive(Default, Clone, Debug)]
pub(in super::super) struct Endpoints {
    endpoints: Box<[Endpoint]>,
    old_endpoints: Box<[Endpoint]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum ServiceName {
    Up,
    Io,
    Uc,
    Rs,
    Rsf,
    Api,
    S3,
}

impl Endpoints {
    #[inline]
    pub(in super::super) fn endpoints(&self) -> &[Endpoint] {
        &self.endpoints
    }

    #[inline]
    pub(in super::super) fn old_endpoints(&self) -> &[Endpoint] {
        &self.old_endpoints
    }

    fn from_region(region: &Region, service: ServiceName) -> Self {
        match service {
            ServiceName::Up => region.up().to_owned(),
            ServiceName::Io => region.io().to_owned(),
            ServiceName::Uc => region.uc().to_owned(),
            ServiceName::Rs => region.rs().to_owned(),
            ServiceName::Rsf => region.rsf().to_owned(),
            ServiceName::Api => region.api().to_owned(),
            ServiceName::S3 => region.s3().to_owned(),
        }
    }

    #[inline]
    fn from_region_provider(
        region_provider: &dyn RegionProvider,
        service: ServiceName,
    ) -> APIResult<Self> {
        Ok(Self::from_region(&region_provider.get()?, service))
    }

    #[cfg(feature = "async")]
    #[inline]
    async fn async_from_region_provider(
        region_provider: &dyn RegionProvider,
        service: ServiceName,
    ) -> APIResult<Self> {
        Ok(Self::from_region(
            &region_provider.async_get().await?,
            service,
        ))
    }
}

impl From<Box<[Endpoint]>> for Endpoints {
    #[inline]
    fn from(endpoints: Box<[Endpoint]>) -> Self {
        Self {
            endpoints,
            old_endpoints: Default::default(),
        }
    }
}

impl From<(Box<[Endpoint]>, Box<[Endpoint]>)> for Endpoints {
    #[inline]
    fn from(endpoints: (Box<[Endpoint]>, Box<[Endpoint]>)) -> Self {
        Self {
            endpoints: endpoints.0,
            old_endpoints: endpoints.1,
        }
    }
}

impl From<Vec<Endpoint>> for Endpoints {
    #[inline]
    fn from(endpoints: Vec<Endpoint>) -> Self {
        Self {
            endpoints: endpoints.into_boxed_slice(),
            old_endpoints: Default::default(),
        }
    }
}

impl From<(Vec<Endpoint>, Vec<Endpoint>)> for Endpoints {
    #[inline]
    fn from(endpoints: (Vec<Endpoint>, Vec<Endpoint>)) -> Self {
        Self {
            endpoints: endpoints.0.into_boxed_slice(),
            old_endpoints: endpoints.1.into_boxed_slice(),
        }
    }
}

impl From<Box<[String]>> for Endpoints {
    #[inline]
    fn from(domains: Box<[String]>) -> Self {
        domains.as_ref().into()
    }
}

impl From<Vec<String>> for Endpoints {
    #[inline]
    fn from(domains: Vec<String>) -> Self {
        domains.as_slice().into()
    }
}

impl<'a> From<&'a [String]> for Endpoints {
    #[inline]
    fn from(domains: &'a [String]) -> Self {
        Self {
            endpoints: convert_from_domains_to_endpoints(domains),
            old_endpoints: Default::default(),
        }
    }
}

impl From<Box<[(String, u16)]>> for Endpoints {
    #[inline]
    fn from(domains_with_port: Box<[(String, u16)]>) -> Self {
        domains_with_port.as_ref().into()
    }
}

impl From<Vec<(String, u16)>> for Endpoints {
    #[inline]
    fn from(domains_with_port: Vec<(String, u16)>) -> Self {
        domains_with_port.as_slice().into()
    }
}

impl<'a> From<&'a [(String, u16)]> for Endpoints {
    #[inline]
    fn from(domains_with_port: &'a [(String, u16)]) -> Self {
        Self {
            endpoints: convert_from_domains_with_port_to_endpoints(domains_with_port),
            old_endpoints: Default::default(),
        }
    }
}

impl From<Box<[IpAddr]>> for Endpoints {
    #[inline]
    fn from(ip_addrs: Box<[IpAddr]>) -> Self {
        ip_addrs.as_ref().into()
    }
}

impl From<Vec<IpAddr>> for Endpoints {
    #[inline]
    fn from(ip_addrs: Vec<IpAddr>) -> Self {
        ip_addrs.as_slice().into()
    }
}

impl<'a> From<&'a [IpAddr]> for Endpoints {
    #[inline]
    fn from(ip_addrs: &'a [IpAddr]) -> Self {
        Self {
            endpoints: convert_from_ip_addrs_to_endpoints(ip_addrs),
            old_endpoints: Default::default(),
        }
    }
}

impl From<Box<[SocketAddr]>> for Endpoints {
    #[inline]
    fn from(socket_addrs: Box<[SocketAddr]>) -> Self {
        socket_addrs.as_ref().into()
    }
}

impl From<Vec<SocketAddr>> for Endpoints {
    #[inline]
    fn from(socket_addrs: Vec<SocketAddr>) -> Self {
        socket_addrs.as_slice().into()
    }
}

impl<'a> From<&'a [SocketAddr]> for Endpoints {
    #[inline]
    fn from(socket_addrs: &'a [SocketAddr]) -> Self {
        Self {
            endpoints: convert_from_socket_addr_to_endpoints(socket_addrs),
            old_endpoints: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IntoEndpoints<'r> {
    inner: Inner<'r>,
}

#[derive(Debug, Clone)]
enum Inner<'r> {
    Endpoints(Box<[Endpoint]>),
    Region(&'r Region),
    Provider(&'r dyn RegionProvider),
}

impl From<Box<[Endpoint]>> for IntoEndpoints<'_> {
    #[inline]
    fn from(endpoints: Box<[Endpoint]>) -> Self {
        Self {
            inner: Inner::Endpoints(endpoints),
        }
    }
}

impl From<Vec<Endpoint>> for IntoEndpoints<'_> {
    #[inline]
    fn from(endpoints: Vec<Endpoint>) -> Self {
        Self {
            inner: Inner::Endpoints(endpoints.into()),
        }
    }
}

impl From<Box<[String]>> for IntoEndpoints<'_> {
    #[inline]
    fn from(domains: Box<[String]>) -> Self {
        domains.as_ref().into()
    }
}

impl From<Vec<String>> for IntoEndpoints<'_> {
    #[inline]
    fn from(domains: Vec<String>) -> Self {
        domains.as_slice().into()
    }
}

impl<'a> From<&'a [String]> for IntoEndpoints<'_> {
    #[inline]
    fn from(domains: &'a [String]) -> Self {
        Self {
            inner: Inner::Endpoints(convert_from_domains_to_endpoints(domains)),
        }
    }
}

impl From<Box<[(String, u16)]>> for IntoEndpoints<'_> {
    #[inline]
    fn from(domains_with_port: Box<[(String, u16)]>) -> Self {
        domains_with_port.as_ref().into()
    }
}

impl From<Vec<(String, u16)>> for IntoEndpoints<'_> {
    #[inline]
    fn from(domains_with_port: Vec<(String, u16)>) -> Self {
        domains_with_port.as_slice().into()
    }
}

impl<'a> From<&'a [(String, u16)]> for IntoEndpoints<'_> {
    #[inline]
    fn from(domains_with_port: &'a [(String, u16)]) -> Self {
        Self {
            inner: Inner::Endpoints(convert_from_domains_with_port_to_endpoints(
                domains_with_port,
            )),
        }
    }
}

impl From<Box<[IpAddr]>> for IntoEndpoints<'_> {
    #[inline]
    fn from(ip_addrs: Box<[IpAddr]>) -> Self {
        ip_addrs.as_ref().into()
    }
}

impl From<Vec<IpAddr>> for IntoEndpoints<'_> {
    #[inline]
    fn from(ip_addrs: Vec<IpAddr>) -> Self {
        ip_addrs.as_slice().into()
    }
}

impl<'a> From<&'a [IpAddr]> for IntoEndpoints<'_> {
    #[inline]
    fn from(ip_addrs: &'a [IpAddr]) -> Self {
        Self {
            inner: Inner::Endpoints(convert_from_ip_addrs_to_endpoints(ip_addrs)),
        }
    }
}

impl From<Box<[SocketAddr]>> for IntoEndpoints<'_> {
    #[inline]
    fn from(socket_addrs: Box<[SocketAddr]>) -> Self {
        socket_addrs.as_ref().into()
    }
}

impl From<Vec<SocketAddr>> for IntoEndpoints<'_> {
    #[inline]
    fn from(socket_addrs: Vec<SocketAddr>) -> Self {
        socket_addrs.as_slice().into()
    }
}

impl<'a> From<&'a [SocketAddr]> for IntoEndpoints<'_> {
    #[inline]
    fn from(socket_addrs: &'a [SocketAddr]) -> Self {
        Self {
            inner: Inner::Endpoints(convert_from_socket_addr_to_endpoints(socket_addrs)),
        }
    }
}

impl<'r> From<&'r Region> for IntoEndpoints<'r> {
    #[inline]
    fn from(region: &'r Region) -> Self {
        Self {
            inner: Inner::Region(region),
        }
    }
}
impl<'r> From<&'r dyn RegionProvider> for IntoEndpoints<'r> {
    #[inline]
    fn from(provider: &'r dyn RegionProvider) -> Self {
        Self {
            inner: Inner::Provider(provider),
        }
    }
}

impl IntoEndpoints<'_> {
    pub(in super::super) fn into_endpoints(self, service: ServiceName) -> APIResult<Endpoints> {
        let endpoints = match self.inner {
            Inner::Endpoints(endpoints) => endpoints.into(),
            Inner::Region(region) => Endpoints::from_region(region, service),
            Inner::Provider(provider) => Endpoints::from_region_provider(provider, service)?,
        };
        Ok(endpoints)
    }

    #[cfg(feature = "async")]
    pub(in super::super) async fn async_into_endpoints(
        self,
        service: ServiceName,
    ) -> APIResult<Endpoints> {
        let endpoints = match self.inner {
            Inner::Endpoints(endpoints) => endpoints.into(),
            Inner::Region(region) => Endpoints::from_region(region, service),
            Inner::Provider(provider) => {
                Endpoints::async_from_region_provider(provider, service).await?
            }
        };
        Ok(endpoints)
    }
}

#[inline]
fn convert_from_domains_to_endpoints(domains: &[String]) -> Box<[Endpoint]> {
    domains
        .iter()
        .map(|domain| Endpoint::new_from_domain(domain.as_str()))
        .collect()
}

#[inline]
fn convert_from_domains_with_port_to_endpoints(
    domains_with_port: &[(String, u16)],
) -> Box<[Endpoint]> {
    domains_with_port
        .iter()
        .map(|(domain, port)| Endpoint::new_from_domain_with_port(domain.as_str(), *port))
        .collect()
}

#[inline]
fn convert_from_ip_addrs_to_endpoints(ip_addrs: &[IpAddr]) -> Box<[Endpoint]> {
    ip_addrs
        .iter()
        .map(|addr| Endpoint::new_from_ip_addr(*addr))
        .collect()
}

#[inline]
fn convert_from_socket_addr_to_endpoints(socket_addrs: &[SocketAddr]) -> Box<[Endpoint]> {
    socket_addrs
        .iter()
        .map(|addr| Endpoint::new_from_socket_addr(*addr))
        .collect()
}
