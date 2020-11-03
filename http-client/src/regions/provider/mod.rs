use super::{super::APIResult, Region};
use std::{any::Any, fmt::Debug};

mod bucket_regions_queryer;
pub use bucket_regions_queryer::{
    BucketRegionsProvider, BucketRegionsQueryer, BucketRegionsQueryerBuilder,
};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// 区域信息提供者
///
/// 为区域信息提供者的实现提供接口支持
pub trait RegionProvider: Any + Debug + Sync + Send {
    /// 返回七牛区域信息
    fn get(&self) -> APIResult<Region>;

    /// 返回多个七牛区域信息
    #[inline]
    fn get_all(&self) -> APIResult<Vec<Region>> {
        let region = self.get()?;
        Ok(vec![region])
    }

    /// 异步返回七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get(&self) -> BoxFuture<APIResult<Region>> {
        Box::pin(async move { self.get() })
    }

    /// 异步返回多个七牛区域信息
    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    fn async_get_all(&self) -> BoxFuture<APIResult<Vec<Region>>> {
        Box::pin(async move { self.get_all() })
    }

    fn as_any(&self) -> &dyn Any;
    fn as_region_provider(&self) -> &dyn RegionProvider;
}

// TODO: Region ID Queryer
