use super::{
    super::{ApiResult, CacheController},
    Region,
};
use auto_impl::auto_impl;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

mod regions_cache;

mod bucket_regions_queryer;
pub use bucket_regions_queryer::{
    BucketRegionsProvider, BucketRegionsQueryer, BucketRegionsQueryerBuilder,
};

mod regions_provider;
pub use regions_provider::RegionsProvider;

mod static_region_provider;
pub use static_region_provider::StaticRegionsProvider;

mod cached_regions_provider;
pub use cached_regions_provider::CachedRegionsProvider;

mod structs;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// 区域信息提供者
///
/// 为区域信息提供者的实现提供接口支持
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait RegionProvider: Debug + Sync + Send {
    /// 返回七牛区域信息
    fn get(&self, opts: &GetOptions) -> ApiResult<GotRegion>;

    /// 返回多个七牛区域信息
    #[inline]
    fn get_all(&self, opts: &GetOptions) -> ApiResult<GotRegions> {
        let region = self.get(opts)?.into_region();
        Ok(vec![region].into())
    }

    /// 异步返回七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get<'a>(&'a self, opts: &'a GetOptions) -> BoxFuture<'a, ApiResult<GotRegion>> {
        Box::pin(async move { self.get(opts) })
    }

    /// 异步返回多个七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get_all<'a>(&'a self, opts: &'a GetOptions) -> BoxFuture<'a, ApiResult<GotRegions>> {
        Box::pin(async move { self.get_all(opts) })
    }

    #[inline]
    fn cache_controller(&self) -> Option<&dyn CacheController> {
        None
    }
}

#[derive(Clone, Debug, Default)]
pub struct GetOptions {}

#[derive(Clone, Debug)]
pub struct GotRegion(Region);

impl From<GotRegion> for Region {
    #[inline]
    fn from(result: GotRegion) -> Self {
        result.0
    }
}

impl From<Region> for GotRegion {
    #[inline]
    fn from(region: Region) -> Self {
        Self(region)
    }
}

impl GotRegion {
    #[inline]
    pub fn region(&self) -> &Region {
        &self.0
    }

    #[inline]
    pub fn region_mut(&mut self) -> &mut Region {
        &mut self.0
    }

    #[inline]
    pub fn into_region(self) -> Region {
        self.0
    }
}

impl Deref for GotRegion {
    type Target = Region;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GotRegion {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl RegionProvider for Region {
    #[inline]
    fn get(&self, _opts: &GetOptions) -> ApiResult<GotRegion> {
        Ok(self.to_owned().into())
    }
}

#[derive(Clone, Debug)]
pub struct GotRegions(Vec<Region>);

impl From<GotRegions> for Vec<Region> {
    #[inline]
    fn from(results: GotRegions) -> Self {
        results.0
    }
}

impl From<Vec<Region>> for GotRegions {
    #[inline]
    fn from(regions: Vec<Region>) -> Self {
        Self(regions)
    }
}

impl FromIterator<Region> for GotRegions {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Region>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl<'a> IntoIterator for &'a GotRegions {
    type Item = &'a Region;
    type IntoIter = std::slice::Iter<'a, Region>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl IntoIterator for GotRegions {
    type Item = Region;
    type IntoIter = std::vec::IntoIter<Region>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl GotRegions {
    #[inline]
    pub fn regions(&self) -> &[Region] {
        &self.0
    }

    #[inline]
    pub fn regions_mut(&mut self) -> &mut Vec<Region> {
        &mut self.0
    }

    #[inline]
    pub fn into_regions(self) -> Vec<Region> {
        self.0
    }
}

impl AsRef<[Region]> for GotRegions {
    #[inline]
    fn as_ref(&self) -> &[Region] {
        &self.0
    }
}

impl AsMut<[Region]> for GotRegions {
    #[inline]
    fn as_mut(&mut self) -> &mut [Region] {
        &mut self.0
    }
}

impl Deref for GotRegions {
    type Target = [Region];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GotRegions {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
