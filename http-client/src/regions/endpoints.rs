use super::{super::APIResult, Endpoint, Region, RegionProvider};
use std::{error::Error, fmt, str::FromStr};

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

#[derive(Debug, Clone)]
pub struct InvalidServiceName(Box<str>);

impl FromStr for ServiceName {
    type Err = InvalidServiceName;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "up" => Ok(Self::Up),
            "io" => Ok(Self::Io),
            "uc" => Ok(Self::Uc),
            "rs" => Ok(Self::Rs),
            "rsf" => Ok(Self::Rsf),
            "api" => Ok(Self::Api),
            "s3" => Ok(Self::S3),
            service_name => Err(InvalidServiceName(service_name.into())),
        }
    }
}

impl fmt::Display for InvalidServiceName {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid service name: {}", self.0)
    }
}

impl Error for InvalidServiceName {}

#[derive(Default, Clone, Debug)]
pub struct Endpoints {
    endpoints: Box<[Endpoint]>,
    old_endpoints: Box<[Endpoint]>,
}

impl Endpoints {
    #[inline]
    pub fn endpoints(&self) -> &[Endpoint] {
        &self.endpoints
    }

    #[inline]
    pub fn old_endpoints(&self) -> &[Endpoint] {
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

#[derive(Default, Clone, Debug)]
pub struct EndpointsBuilder {
    endpoints: Vec<Endpoint>,
    old_endpoints: Vec<Endpoint>,
}

impl EndpointsBuilder {
    #[inline]
    pub fn add_endpoint(mut self, endpoint: impl Into<Endpoint>) -> Self {
        self.endpoints.push(endpoint.into());
        self
    }

    #[inline]
    pub fn add_old_endpoint(mut self, endpoint: impl Into<Endpoint>) -> Self {
        self.old_endpoints.push(endpoint.into());
        self
    }

    #[inline]
    pub fn build(self) -> Endpoints {
        Endpoints {
            endpoints: self.endpoints.into_boxed_slice(),
            old_endpoints: self.old_endpoints.into_boxed_slice(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IntoEndpoints<'r> {
    inner: Inner<'r>,
}

#[derive(Debug, Clone)]
enum Inner<'r> {
    Endpoints(Endpoints),
    Region(&'r Region),
    Provider(&'r dyn RegionProvider),
}

impl From<Endpoints> for IntoEndpoints<'_> {
    #[inline]
    fn from(endpoints: Endpoints) -> Self {
        Self {
            inner: Inner::Endpoints(endpoints),
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
            Inner::Endpoints(endpoints) => endpoints,
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
            Inner::Endpoints(endpoints) => endpoints,
            Inner::Region(region) => Endpoints::from_region(region, service),
            Inner::Provider(provider) => {
                Endpoints::async_from_region_provider(provider, service).await?
            }
        };
        Ok(endpoints)
    }
}
