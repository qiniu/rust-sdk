use super::{super::APIResult, Region, RegionProvider};

#[derive(Default, Clone, Debug)]
pub(crate) struct Domains {
    domains: Vec<String>,
    old_domains: Vec<String>,
}

pub(crate) enum ServiceName {
    Up,
    Io,
    Uc,
    Rs,
    Rsf,
    Api,
    S3,
}

impl Domains {
    #[inline]
    pub(crate) fn domains(&self) -> &[String] {
        &self.domains
    }

    #[inline]
    pub(crate) fn domains_mut(&mut self) -> &mut Vec<String> {
        &mut self.domains
    }

    #[inline]
    pub(crate) fn old_domains(&self) -> &[String] {
        &self.old_domains
    }

    #[inline]
    pub(crate) fn old_domains_mut(&mut self) -> &mut Vec<String> {
        &mut self.old_domains
    }

    pub(crate) fn from_region(region: &Region, service: ServiceName) -> Self {
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
    pub(crate) fn from_region_provider(
        region_provider: &dyn RegionProvider,
        service: ServiceName,
    ) -> APIResult<Self> {
        Ok(Self::from_region(&region_provider.get()?, service))
    }

    #[cfg(feature = "async")]
    #[inline]
    pub(crate) async fn async_from_region_provider(
        region_provider: &dyn RegionProvider,
        service: ServiceName,
    ) -> APIResult<Self> {
        Ok(Self::from_region(
            &region_provider.async_get().await?,
            service,
        ))
    }
}

impl From<Vec<String>> for Domains {
    #[inline]
    fn from(domains: Vec<String>) -> Self {
        Self {
            domains,
            old_domains: Vec::new(),
        }
    }
}

pub(crate) enum IntoDomains<'r> {
    Domains(Vec<String>),
    Region(&'r Region),
    Provider(&'r dyn RegionProvider),
}

impl From<Vec<String>> for IntoDomains<'_> {
    #[inline]
    fn from(domains: Vec<String>) -> Self {
        Self::Domains(domains)
    }
}

impl<'r> From<&'r Region> for IntoDomains<'r> {
    #[inline]
    fn from(region: &'r Region) -> Self {
        Self::Region(region)
    }
}
impl<'r> From<&'r dyn RegionProvider> for IntoDomains<'r> {
    #[inline]
    fn from(provider: &'r dyn RegionProvider) -> Self {
        Self::Provider(provider)
    }
}

impl IntoDomains<'_> {
    pub(crate) fn into_domains(self, service: ServiceName) -> APIResult<Domains> {
        let domains = match self {
            Self::Domains(domains) => domains.into(),
            Self::Region(region) => Domains::from_region(region, service),
            Self::Provider(provider) => Domains::from_region_provider(provider, service)?,
        };
        Ok(domains)
    }

    #[cfg(feature = "async")]
    pub(crate) async fn async_into_domains(self, service: ServiceName) -> APIResult<Domains> {
        let domains = match self {
            Self::Domains(domains) => domains.into(),
            Self::Region(region) => Domains::from_region(region, service),
            Self::Provider(provider) => {
                Domains::async_from_region_provider(provider, service).await?
            }
        };
        Ok(domains)
    }
}
