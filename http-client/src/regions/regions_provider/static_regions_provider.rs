use super::{
    super::{super::ApiResult, Region},
    GetOptions, GotRegion, GotRegions, RegionsProvider,
};

/// 静态区域信息提供者
#[derive(Debug, Clone)]
pub struct StaticRegionsProvider {
    regions: Vec<Region>,
}

impl StaticRegionsProvider {
    /// 根据一个区域创建静态区域信息提供者
    #[inline]
    pub fn new(region: impl Into<Region>) -> Self {
        Self {
            regions: vec![region.into()],
        }
    }

    /// 向静态区域信息提供者追加更多区域
    #[inline]
    pub fn append(&mut self, region: impl Into<Region>) -> &mut Self {
        self.regions.push(region.into());
        self
    }
}

impl RegionsProvider for StaticRegionsProvider {
    fn get(&self, _opts: GetOptions) -> ApiResult<GotRegion> {
        Ok(self.regions.get(0).cloned().expect("regions must not be empty").into())
    }

    #[inline]
    fn get_all(&self, _opts: GetOptions) -> ApiResult<GotRegions> {
        Ok(Vec::from_iter(self.regions.iter().cloned()).into())
    }
}

impl From<Region> for StaticRegionsProvider {
    #[inline]
    fn from(region: Region) -> Self {
        Self::new(region)
    }
}

impl FromIterator<Region> for StaticRegionsProvider {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Region>>(iter: T) -> Self {
        Self {
            regions: iter.into_iter().collect(),
        }
    }
}

impl<'a> IntoIterator for &'a StaticRegionsProvider {
    type Item = &'a Region;
    type IntoIter = std::slice::Iter<'a, Region>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.regions.iter()
    }
}

impl Extend<Region> for StaticRegionsProvider {
    #[inline]
    fn extend<T: IntoIterator<Item = Region>>(&mut self, iter: T) {
        self.regions.extend(iter)
    }
}
