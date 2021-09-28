use super::{super::APIResult, Endpoint, Region, RegionProvider};
use serde::{Deserialize, Serialize};
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Endpoints {
    preferred: Box<[Endpoint]>,
    alternative: Box<[Endpoint]>,
}

impl Endpoints {
    #[inline]
    pub fn builder(endpoint: impl Into<Endpoint>) -> EndpointsBuilder {
        EndpointsBuilder {
            preferred: vec![endpoint.into()],
            alternative: vec![],
        }
    }

    #[inline]
    pub fn new(endpoint: impl Into<Endpoint>) -> Self {
        Self::builder(endpoint).build()
    }

    #[inline]
    pub fn preferred(&self) -> &[Endpoint] {
        &self.preferred
    }

    #[inline]
    pub fn alternative(&self) -> &[Endpoint] {
        &self.alternative
    }

    fn from_region(region: &Region, services: &[ServiceName]) -> Self {
        let mut builder = EndpointsBuilder {
            preferred: vec![],
            alternative: vec![],
        };

        for service in services {
            let e = match service {
                ServiceName::Up => region.up(),
                ServiceName::Io => region.io(),
                ServiceName::Uc => region.uc(),
                ServiceName::Rs => region.rs(),
                ServiceName::Rsf => region.rsf(),
                ServiceName::Api => region.api(),
                ServiceName::S3 => region.s3(),
            };
            builder.preferred.extend_from_slice(e.preferred());
            builder.alternative.extend_from_slice(e.alternative());
        }
        builder.build()
    }

    #[inline]
    fn from_region_provider(
        region_provider: &dyn RegionProvider,
        services: &[ServiceName],
    ) -> APIResult<Self> {
        Ok(Self::from_region(&region_provider.get()?, services))
    }

    #[cfg(feature = "async")]
    #[inline]
    async fn async_from_region_provider(
        region_provider: &dyn RegionProvider,
        services: &[ServiceName],
    ) -> APIResult<Self> {
        Ok(Self::from_region(
            &region_provider.async_get().await?,
            services,
        ))
    }
}

impl From<Box<[Endpoint]>> for Endpoints {
    #[inline]
    fn from(endpoints: Box<[Endpoint]>) -> Self {
        Self {
            preferred: endpoints,
            alternative: Default::default(),
        }
    }
}

impl From<(Box<[Endpoint]>, Box<[Endpoint]>)> for Endpoints {
    #[inline]
    fn from(endpoints: (Box<[Endpoint]>, Box<[Endpoint]>)) -> Self {
        Self {
            preferred: endpoints.0,
            alternative: endpoints.1,
        }
    }
}

impl From<Vec<Endpoint>> for Endpoints {
    #[inline]
    fn from(endpoints: Vec<Endpoint>) -> Self {
        Self {
            preferred: endpoints.into_boxed_slice(),
            alternative: Default::default(),
        }
    }
}

impl From<(Vec<Endpoint>, Vec<Endpoint>)> for Endpoints {
    #[inline]
    fn from(endpoints: (Vec<Endpoint>, Vec<Endpoint>)) -> Self {
        Self {
            preferred: endpoints.0.into_boxed_slice(),
            alternative: endpoints.1.into_boxed_slice(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EndpointsBuilder {
    preferred: Vec<Endpoint>,
    alternative: Vec<Endpoint>,
}

impl EndpointsBuilder {
    #[inline]
    pub fn add_preferred_endpoint(mut self, endpoint: impl Into<Endpoint>) -> Self {
        self.preferred.push(endpoint.into());
        self
    }

    #[inline]
    pub fn add_alternative_endpoint(mut self, endpoint: impl Into<Endpoint>) -> Self {
        self.alternative.push(endpoint.into());
        self
    }

    #[inline]
    pub fn build(self) -> Endpoints {
        Endpoints {
            preferred: self.preferred.into_boxed_slice(),
            alternative: self.alternative.into_boxed_slice(),
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
    pub(in super::super) fn into_endpoints(self, services: &[ServiceName]) -> APIResult<Endpoints> {
        let endpoints = match self.inner {
            Inner::Endpoints(endpoints) => endpoints,
            Inner::Region(region) => Endpoints::from_region(region, services),
            Inner::Provider(provider) => Endpoints::from_region_provider(provider, services)?,
        };
        Ok(endpoints)
    }

    #[cfg(feature = "async")]
    pub(in super::super) async fn async_into_endpoints(
        self,
        services: &[ServiceName],
    ) -> APIResult<Endpoints> {
        let endpoints = match self.inner {
            Inner::Endpoints(endpoints) => endpoints,
            Inner::Region(region) => Endpoints::from_region(region, services),
            Inner::Provider(provider) => {
                Endpoints::async_from_region_provider(provider, services).await?
            }
        };
        Ok(endpoints)
    }
}
