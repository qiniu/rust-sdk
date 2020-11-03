mod endpoint;
mod endpoints;
mod provider;
mod region;

pub use endpoint::{
    DomainWithPort, DomainWithPortParseError, Endpoint, EndpointParseError, IpAddrWithPort,
    IpAddrWithPortParseError,
};
pub use endpoints::{IntoEndpoints, ServiceName};
pub use provider::{
    BucketRegionsProvider, BucketRegionsQueryer, BucketRegionsQueryerBuilder, RegionProvider,
};
pub use region::{Region, RegionBuilder};

pub(super) use endpoints::Endpoints;
