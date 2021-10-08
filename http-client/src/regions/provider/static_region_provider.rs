use super::{
    super::{super::APIResult, Region},
    RegionProvider,
};
use std::any::Any;

#[derive(Debug, Clone)]
pub struct StaticRegionProvider {
    regions: Box<[Region]>,
}

impl StaticRegionProvider {
    #[inline]
    pub fn new(regions: impl Into<Vec<Region>>) -> Self {
        let regions = regions.into().into_boxed_slice();
        debug_assert!(!regions.is_empty(), "regions must not be empty");
        Self { regions }
    }
}

impl RegionProvider for StaticRegionProvider {
    #[inline]
    fn get(&self) -> APIResult<Region> {
        Ok(self
            .regions
            .first()
            .expect("regions must not be empty")
            .to_owned())
    }

    #[inline]
    fn get_all(&self) -> APIResult<Vec<Region>> {
        Ok(self.regions.to_owned().into_vec())
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_region_provider(&self) -> &dyn RegionProvider {
        self
    }
}
