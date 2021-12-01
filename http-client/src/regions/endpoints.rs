use super::{super::ApiResult, Endpoint, Region, RegionProvider};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, error::Error, fmt, mem::take, str::FromStr, sync::Arc};

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
    preferred: Arc<[Endpoint]>,
    alternative: Arc<[Endpoint]>,
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
    pub(super) fn public_uc_endpoints() -> &'static Self {
        static DEFAULT_UC_ENDPOINTS: Lazy<Endpoints> = Lazy::new(|| {
            Endpoints::builder(Endpoint::new_from_domain("uc.qbox.me"))
                .add_preferred_endpoint(Endpoint::new_from_domain("api.qiniu.com"))
                .build()
        });
        &DEFAULT_UC_ENDPOINTS
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
    ) -> ApiResult<Self> {
        Ok(Self::from_region(
            region_provider.get(&Default::default())?.region(),
            services,
        ))
    }

    #[cfg(feature = "async")]
    #[inline]
    async fn async_from_region_provider(
        region_provider: &dyn RegionProvider,
        services: &[ServiceName],
    ) -> ApiResult<Self> {
        Ok(Self::from_region(
            region_provider
                .async_get(&Default::default())
                .await?
                .region(),
            services,
        ))
    }
}

impl From<Vec<Endpoint>> for Endpoints {
    #[inline]
    fn from(endpoints: Vec<Endpoint>) -> Self {
        Self {
            preferred: endpoints.into(),
            alternative: Arc::new([]),
        }
    }
}

impl FromIterator<Endpoint> for Endpoints {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Endpoint>>(iter: T) -> Self {
        Self {
            preferred: iter.into_iter().collect(),
            alternative: Arc::new([]),
        }
    }
}

impl<'a> IntoIterator for &'a Endpoints {
    type Item = &'a Endpoint;
    type IntoIter = std::slice::Iter<'a, Endpoint>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.preferred.iter()
    }
}

impl From<(Vec<Endpoint>, Vec<Endpoint>)> for Endpoints {
    #[inline]
    fn from(endpoints: (Vec<Endpoint>, Vec<Endpoint>)) -> Self {
        Self {
            preferred: endpoints.0.into(),
            alternative: endpoints.1.into(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct EndpointsBuilder {
    preferred: Vec<Endpoint>,
    alternative: Vec<Endpoint>,
}

impl EndpointsBuilder {
    #[inline]
    pub fn add_preferred_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.preferred.push(endpoint.into());
        self
    }

    #[inline]
    pub fn add_alternative_endpoint(&mut self, endpoint: impl Into<Endpoint>) -> &mut Self {
        self.alternative.push(endpoint.into());
        self
    }

    #[inline]
    pub fn build(&mut self) -> Endpoints {
        let owned = take(self);
        Endpoints {
            preferred: owned.preferred.into(),
            alternative: owned.alternative.into(),
        }
    }
}

impl FromIterator<Endpoint> for EndpointsBuilder {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Endpoint>>(iter: T) -> Self {
        Self {
            preferred: Vec::from_iter(iter),
            alternative: Default::default(),
        }
    }
}

impl Extend<Endpoint> for EndpointsBuilder {
    #[inline]
    fn extend<T: IntoIterator<Item = Endpoint>>(&mut self, iter: T) {
        self.preferred.extend(iter)
    }
}

#[cfg(feature = "async")]
type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + 'a + Send>>;

pub trait EndpointsProvider: fmt::Debug + Send + Sync {
    fn get_endpoints<'e>(&'e self, services: &[ServiceName]) -> ApiResult<Cow<'e, Endpoints>>;

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get_endpoints<'a>(
        &'a self,
        services: &'a [ServiceName],
    ) -> BoxFuture<'a, ApiResult<Cow<'a, Endpoints>>> {
        Box::pin(async move { self.get_endpoints(services) })
    }
}

impl EndpointsProvider for Endpoint {
    #[inline]
    fn get_endpoints<'e>(&'e self, _services: &[ServiceName]) -> ApiResult<Cow<'e, Endpoints>> {
        Ok(Cow::Owned(Endpoints::builder(self.to_owned()).build()))
    }
}

impl EndpointsProvider for Endpoints {
    #[inline]
    fn get_endpoints<'e>(&'e self, _services: &[ServiceName]) -> ApiResult<Cow<'e, Endpoints>> {
        Ok(Cow::Borrowed(self))
    }
}

impl<'a> EndpointsProvider for &'a Endpoints {
    #[inline]
    fn get_endpoints<'e>(&'e self, _services: &[ServiceName]) -> ApiResult<Cow<'e, Endpoints>> {
        Ok(Cow::Borrowed(self))
    }
}

impl<T: RegionProvider> EndpointsProvider for T {
    #[inline]
    fn get_endpoints<'e>(&'e self, services: &[ServiceName]) -> ApiResult<Cow<'e, Endpoints>> {
        Ok(Cow::Owned(Endpoints::from_region_provider(self, services)?))
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get_endpoints<'a>(
        &'a self,
        services: &'a [ServiceName],
    ) -> BoxFuture<'a, ApiResult<Cow<'a, Endpoints>>> {
        Box::pin(async move {
            Ok(Cow::Owned(
                Endpoints::async_from_region_provider(self, services).await?,
            ))
        })
    }
}
