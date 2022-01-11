mod endpoint;
mod endpoints;
mod provider;
mod region;

pub use endpoint::{
    DomainWithPort, DomainWithPortParseError, Endpoint, EndpointParseError, IpAddrWithPort,
    IpAddrWithPortParseError,
};
pub use endpoints::{
    Endpoints, EndpointsBuilder, EndpointsProvider, InvalidServiceName, RegionProviderEndpoints,
    ServiceName,
};
pub use provider::{
    BucketRegionsProvider, BucketRegionsQueryer, BucketRegionsQueryerBuilder,
    CachedRegionsProvider, GetOptions, GotRegion, GotRegions, RegionProvider, RegionsProvider,
    StaticRegionsProvider,
};
pub use region::{Region, RegionBuilder};
