mod cache_key;
mod endpoint;
mod endpoints_provider;
mod region;
mod regions_provider;

pub use endpoint::{
    DomainWithPort, DomainWithPortParseError, Endpoint, EndpointParseError, IpAddrWithPort, IpAddrWithPortParseError,
};
pub use endpoints_provider::{
    BucketDomainsProvider, BucketDomainsQueryer, BucketDomainsQueryerBuilder, Endpoints, EndpointsBuilder,
    EndpointsProvider, GetOptions as EndpointsGetOptions, GetOptionsBuilder as EndpointsGetOptionsBuilder,
    InvalidServiceName, RegionsProviderEndpoints, ServiceName,
};
pub use region::{Region, RegionBuilder};
pub use regions_provider::{
    AllRegionsProvider, AllRegionsProviderBuilder, BucketRegionsProvider, BucketRegionsQueryer,
    BucketRegionsQueryerBuilder, GetOptions as RegionsGetOptions, GotRegion, GotRegions, RegionsProvider,
    StaticRegionsProvider,
};
