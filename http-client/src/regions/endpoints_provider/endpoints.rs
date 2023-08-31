use super::{
    super::{super::ApiResult, Endpoint, Region, RegionsProvider},
    ServiceName,
};
use md5::{
    digest::{generic_array::GenericArray, OutputSizeUser},
    Digest, Md5,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    mem::take,
    sync::{Arc, OnceLock},
};

type Md5Value = GenericArray<u8, <Md5 as OutputSizeUser>::OutputSize>;

/// 终端地址列表
///
/// 存储一个七牛服务的多个终端地址，包含主要地址列表和备选地址列表
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Endpoints {
    preferred: Arc<[Endpoint]>,
    alternative: Arc<[Endpoint]>,
    #[serde(skip)]
    md5: Arc<OnceLock<Md5Value>>,
}

impl Endpoints {
    /// 创建终端地址列表构建器
    ///
    /// 必须提供一个主要终端地址
    #[inline]
    pub fn builder(endpoint: Endpoint) -> EndpointsBuilder {
        EndpointsBuilder {
            preferred: vec![endpoint],
            alternative: vec![],
        }
    }

    pub(in super::super) fn public_uc_endpoints() -> &'static Self {
        static DEFAULT_UC_ENDPOINTS: OnceLock<Endpoints> = OnceLock::new();

        DEFAULT_UC_ENDPOINTS.get_or_init(|| {
            Endpoints::builder(Endpoint::new_from_domain("kodo-config.qiniuapi.com"))
                .add_preferred_endpoint(Endpoint::new_from_domain("api.qiniu.com"))
                .add_alternative_endpoint(Endpoint::new_from_domain("uc.qbox.me"))
                .build()
        })
    }

    /// 创建只包含一个主要终端地址的终端地址列表
    #[inline]
    pub fn new(endpoint: Endpoint) -> Self {
        Self::builder(endpoint).build()
    }

    /// 返回主要终端地址列表
    #[inline]
    pub fn preferred(&self) -> &[Endpoint] {
        &self.preferred
    }

    /// 返回备选终端地址列表
    #[inline]
    pub fn alternative(&self) -> &[Endpoint] {
        &self.alternative
    }

    /// 对比两组终端地址列表是否相似
    ///
    /// 相似指的是，两个终端地址列表内主要终端地址列表中的域名相同，备选终端地址列表中的域名也相同，但顺序可能不同
    #[inline]
    pub fn similar(&self, other: &Self) -> bool {
        self.md5() == other.md5()
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

    pub(super) fn from_region_provider(
        region_provider: &dyn RegionsProvider,
        services: &[ServiceName],
    ) -> ApiResult<Self> {
        Ok(Self::from_region(
            region_provider.get(Default::default())?.region(),
            services,
        ))
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    pub(super) async fn async_from_region_provider(
        region_provider: &dyn RegionsProvider,
        services: &[ServiceName],
    ) -> ApiResult<Self> {
        Ok(Self::from_region(
            region_provider.async_get(Default::default()).await?.region(),
            services,
        ))
    }

    pub(in super::super) fn md5(&self) -> &Md5Value {
        self.md5.get_or_init(|| {
            let mut preferred_endpoints = self.preferred().iter().map(|e| e.to_string()).collect::<Vec<_>>();
            let mut alternative_endpoints = self.alternative().iter().map(|e| e.to_string()).collect::<Vec<_>>();

            preferred_endpoints.sort();
            alternative_endpoints.sort();

            let mut md5 = preferred_endpoints
                .into_iter()
                .fold(Md5::default(), |mut md5, endpoint| {
                    md5.update(endpoint.as_bytes());
                    md5.update(b"\0");
                    md5
                });
            md5.update(b"\n");
            alternative_endpoints
                .into_iter()
                .fold(md5, |mut md5, endpoint| {
                    md5.update(endpoint.as_bytes());
                    md5.update(b"\0");
                    md5
                })
                .finalize()
        })
    }
}

impl Default for Endpoints {
    #[inline]
    fn default() -> Self {
        Self {
            preferred: Arc::new([]),
            alternative: Arc::new([]),
            md5: Default::default(),
        }
    }
}

impl From<Vec<Endpoint>> for Endpoints {
    #[inline]
    fn from(endpoints: Vec<Endpoint>) -> Self {
        Self {
            preferred: endpoints.into(),
            alternative: Arc::new([]),
            md5: Default::default(),
        }
    }
}

impl FromIterator<Endpoint> for Endpoints {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Endpoint>>(iter: T) -> Self {
        Self {
            preferred: iter.into_iter().collect(),
            alternative: Arc::new([]),
            md5: Default::default(),
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
            md5: Default::default(),
        }
    }
}

impl<'p> From<&'p Endpoints> for Cow<'p, Endpoints> {
    #[inline]
    fn from(endpoints: &'p Endpoints) -> Self {
        Cow::Borrowed(endpoints)
    }
}

impl From<Endpoints> for Cow<'_, Endpoints> {
    #[inline]
    fn from(endpoints: Endpoints) -> Self {
        Cow::Owned(endpoints)
    }
}

impl<'p> From<Cow<'p, Endpoints>> for Endpoints {
    #[inline]
    fn from(endpoints: Cow<'p, Endpoints>) -> Self {
        match endpoints {
            Cow::Borrowed(endpoints) => endpoints.to_owned(),
            Cow::Owned(endpoints) => endpoints,
        }
    }
}

/// 终端地址列表构建器
#[derive(Clone, Debug, Default)]
pub struct EndpointsBuilder {
    preferred: Vec<Endpoint>,
    alternative: Vec<Endpoint>,
}

impl EndpointsBuilder {
    /// 添加一个主要终端地址
    #[inline]
    pub fn add_preferred_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.preferred.push(endpoint);
        self
    }

    /// 添加一个备选终端地址
    #[inline]
    pub fn add_alternative_endpoint(&mut self, endpoint: Endpoint) -> &mut Self {
        self.alternative.push(endpoint);
        self
    }

    /// 添加多个主要终端地址
    #[inline]
    pub fn add_preferred_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.preferred.extend(endpoints);
        self
    }

    /// 添加多个备选终端地址
    #[inline]
    pub fn add_alternative_endpoints(&mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> &mut Self {
        self.alternative.extend(endpoints);
        self
    }

    /// 构建终端地址列表
    #[inline]
    pub fn build(&mut self) -> Endpoints {
        let owned = take(self);
        Endpoints {
            preferred: owned.preferred.into(),
            alternative: owned.alternative.into(),
            md5: Default::default(),
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
