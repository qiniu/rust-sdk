use super::{
    super::{super::ApiResult, Region},
    GetOptions, GotRegion, GotRegions, RegionProvider,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct StaticRegionsProvider {
    regions: Arc<[Region]>,
}

impl StaticRegionsProvider {
    #[inline]
    pub fn new(regions: impl Into<Vec<Region>>) -> Self {
        let regions = Arc::<[_]>::from(regions.into());
        debug_assert!(!regions.is_empty(), "regions must not be empty");
        Self { regions }
    }
}

impl RegionProvider for StaticRegionsProvider {
    #[inline]
    fn get(&self, _opts: &GetOptions) -> ApiResult<GotRegion> {
        Ok(self
            .regions
            .first()
            .expect("regions must not be empty")
            .to_owned()
            .into())
    }

    #[inline]
    fn get_all(&self, _opts: &GetOptions) -> ApiResult<GotRegions> {
        Ok(Vec::from_iter(self.regions.iter().cloned()).into())
    }
}

impl From<Region> for StaticRegionsProvider {
    #[inline]
    fn from(region: Region) -> Self {
        Self {
            regions: Arc::new([region]),
        }
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
