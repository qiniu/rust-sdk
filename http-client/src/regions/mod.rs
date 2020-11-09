mod endpoint;
mod endpoints;
mod provider;
mod region;

pub use endpoint::{
    DomainWithPort, DomainWithPortParseError, Endpoint, EndpointParseError, IpAddrWithPort,
    IpAddrWithPortParseError,
};
pub use endpoints::{IntoEndpoints, InvalidServiceName, ServiceName};
pub use provider::{
    BucketRegionsProvider, BucketRegionsQueryer, BucketRegionsQueryerBuilder, RegionProvider,
    RegionsProvider, RegionsProviderBuilder,
};
pub use region::{Region, RegionBuilder};

pub(super) use endpoints::Endpoints;
