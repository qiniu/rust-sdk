use super::{
    super::{cache::IsCacheValid, ApiResult},
    Region,
};
use auto_impl::auto_impl;
use dyn_clonable::clonable;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{self, Debug, Display},
    ops::{Deref, DerefMut},
    time::{Duration, SystemTime},
};

mod regions_cache;

mod bucket_regions_queryer;
pub use bucket_regions_queryer::{BucketRegionsProvider, BucketRegionsQueryer, BucketRegionsQueryerBuilder};

mod all_regions_provider;
pub use all_regions_provider::{AllRegionsProvider, AllRegionsProviderBuilder};

mod static_regions_provider;
pub use static_regions_provider::StaticRegionsProvider;

mod structs;

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// 区域信息获取接口
///
/// 可以获取一个区域也可以获取多个区域
///
/// 同时提供阻塞获取接口和异步获取接口，异步获取接口则需要启用 `async` 功能
#[clonable]
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait RegionsProvider: Clone + Debug + Sync + Send {
    /// 返回七牛区域信息
    ///
    /// 该方法的异步版本为 [`Self::async_get`]。
    fn get(&self, opts: GetOptions) -> ApiResult<GotRegion>;

    /// 返回多个七牛区域信息
    ///
    /// 该方法的异步版本为 [`Self::async_get_all`]。
    #[inline]
    fn get_all(&self, opts: GetOptions) -> ApiResult<GotRegions> {
        let region = self.get(opts)?.into_region();
        Ok(vec![region].into())
    }

    /// 异步返回七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get(&self, opts: GetOptions) -> BoxFuture<'_, ApiResult<GotRegion>> {
        Box::pin(async move { self.get(opts) })
    }

    /// 异步返回多个七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_get_all(&self, opts: GetOptions) -> BoxFuture<'_, ApiResult<GotRegions>> {
        Box::pin(async move { self.get_all(opts) })
    }
}

/// 获取区域信息的选项
#[derive(Copy, Clone, Debug, Default)]
pub struct GetOptions {}

/// 获取的区域信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GotRegion {
    region: Region,
    lifetime: Option<Duration>,
    got_at: SystemTime,
}

impl From<GotRegion> for Region {
    #[inline]
    fn from(result: GotRegion) -> Self {
        result.region
    }
}

impl From<Region> for GotRegion {
    #[inline]
    fn from(region: Region) -> Self {
        Self {
            region,
            got_at: SystemTime::now(),
            lifetime: None,
        }
    }
}

impl GotRegion {
    /// 获取的区域信息
    #[inline]
    pub fn region(&self) -> &Region {
        &self.region
    }

    /// 获取的区域信息的可变引用
    #[inline]
    pub fn region_mut(&mut self) -> &mut Region {
        &mut self.region
    }

    /// 获取的生命周期
    #[inline]
    pub fn lifetime(&self) -> Option<Duration> {
        self.lifetime
    }

    /// 获取的生命周期的可变引用
    #[inline]
    pub fn lifetime_mut(&mut self) -> &mut Option<Duration> {
        &mut self.lifetime
    }

    /// 转换为区域信息
    #[inline]
    pub fn into_region(self) -> Region {
        self.region
    }
}

impl IsCacheValid for GotRegion {
    fn is_valid(&self) -> bool {
        if let Some(lifetime) = self.lifetime {
            if let Ok(elapsed) = self.got_at.elapsed() {
                elapsed <= lifetime
            } else {
                false // 如果发生时间倒流，则立即判定为 INVALID
            }
        } else {
            true
        }
    }
}

impl PartialEq for GotRegion {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.region == other.region
    }
}

impl Eq for GotRegion {}

impl Deref for GotRegion {
    type Target = Region;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.region()
    }
}

impl DerefMut for GotRegion {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.region_mut()
    }
}

impl RegionsProvider for Region {
    #[inline]
    fn get(&self, _opts: GetOptions) -> ApiResult<GotRegion> {
        Ok(self.to_owned().into())
    }
}

impl RegionsProvider for GotRegion {
    #[inline]
    fn get(&self, _opts: GetOptions) -> ApiResult<GotRegion> {
        Ok(self.to_owned())
    }
}

/// 获取的区域列表信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GotRegions {
    regions: Vec<Region>,
    lifetime: Option<Duration>,
    got_at: SystemTime,
}

impl From<GotRegions> for Vec<Region> {
    #[inline]
    fn from(results: GotRegions) -> Self {
        results.regions
    }
}

impl From<Vec<Region>> for GotRegions {
    #[inline]
    fn from(regions: Vec<Region>) -> Self {
        Self {
            regions,
            got_at: SystemTime::now(),
            lifetime: None,
        }
    }
}

impl FromIterator<Region> for GotRegions {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Region>>(iter: T) -> Self {
        Vec::from_iter(iter).into()
    }
}

impl Extend<Region> for GotRegions {
    #[inline]
    fn extend<T: IntoIterator<Item = Region>>(&mut self, iter: T) {
        self.regions.extend(iter)
    }
}

impl<'a> IntoIterator for &'a GotRegions {
    type Item = &'a Region;
    type IntoIter = std::slice::Iter<'a, Region>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.regions.iter()
    }
}

impl IntoIterator for GotRegions {
    type Item = Region;
    type IntoIter = std::vec::IntoIter<Region>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.regions.into_iter()
    }
}

impl GotRegions {
    /// 获取的区域信息列表
    #[inline]
    pub fn regions(&self) -> &[Region] {
        &self.regions
    }

    /// 获取的区域信息列表的可变引用
    #[inline]
    pub fn regions_mut(&mut self) -> &mut Vec<Region> {
        &mut self.regions
    }

    /// 获取的生命周期
    #[inline]
    pub fn lifetime(&self) -> Option<Duration> {
        self.lifetime
    }

    /// 获取的生命周期的可变引用
    #[inline]
    pub fn lifetime_mut(&mut self) -> &mut Option<Duration> {
        &mut self.lifetime
    }

    /// 转换为区域信息列表
    #[inline]
    pub fn into_regions(self) -> Vec<Region> {
        self.regions
    }
}

impl IsCacheValid for GotRegions {
    #[inline]
    fn is_valid(&self) -> bool {
        if let Some(lifetime) = self.lifetime {
            if let Ok(elapsed) = self.got_at.elapsed() {
                elapsed <= lifetime
            } else {
                false // 如果发生时间倒流，则立即判定为 INVALID
            }
        } else {
            true
        }
    }
}

impl PartialEq for GotRegions {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.regions == other.regions
    }
}

impl Eq for GotRegions {}

impl AsRef<[Region]> for GotRegions {
    #[inline]
    fn as_ref(&self) -> &[Region] {
        self.regions()
    }
}

impl AsMut<[Region]> for GotRegions {
    #[inline]
    fn as_mut(&mut self) -> &mut [Region] {
        self.regions_mut()
    }
}

impl Deref for GotRegions {
    type Target = [Region];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.regions()
    }
}

impl DerefMut for GotRegions {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.regions_mut()
    }
}

impl RegionsProvider for GotRegions {
    #[inline]
    fn get(&self, opts: GetOptions) -> ApiResult<GotRegion> {
        self.get_all(opts).map(|regions| {
            regions
                .into_regions()
                .into_iter()
                .next()
                .expect("Regions are empty")
                .into()
        })
    }

    #[inline]
    fn get_all(&self, _opts: GetOptions) -> ApiResult<GotRegions> {
        Ok(self.to_owned())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EmptyRegionError;

impl Display for EmptyRegionError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&"regions must not be empty", f)
    }
}

impl Error for EmptyRegionError {}

impl TryFrom<GotRegions> for GotRegion {
    type Error = EmptyRegionError;

    fn try_from(value: GotRegions) -> Result<Self, Self::Error> {
        if let Some(region) = value.regions.into_iter().next() {
            Ok(Self {
                region,
                lifetime: value.lifetime,
                got_at: value.got_at,
            })
        } else {
            Err(EmptyRegionError)
        }
    }
}

impl From<GotRegion> for GotRegions {
    #[inline]
    fn from(value: GotRegion) -> Self {
        Self {
            regions: vec![value.region],
            lifetime: value.lifetime,
            got_at: value.got_at,
        }
    }
}
