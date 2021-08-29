mod endpoint;
mod endpoints;
mod provider;
mod region;

pub use endpoint::{
    DomainWithPort, DomainWithPortParseError, Endpoint, EndpointParseError, IpAddrWithPort,
    IpAddrWithPortParseError,
};
pub use endpoints::{Endpoints, EndpointsBuilder, IntoEndpoints, InvalidServiceName, ServiceName};
pub use provider::{
    BucketRegionsProvider, BucketRegionsQueryer, BucketRegionsQueryerBuilder,
    CachedRegionsProvider, RegionProvider, RegionsProvider, StaticRegionProvider,
};
pub use region::{Region, RegionBuilder};
