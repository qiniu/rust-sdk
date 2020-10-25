mod endpoint;
mod endpoints;
mod provider;
mod region;

pub use endpoint::{DomainWithPort, Endpoint, IpAddrWithPort};
pub use endpoints::{IntoEndpoints, ServiceName};
pub use provider::RegionProvider;
pub use region::{Region, RegionBuilder};

pub(super) use endpoints::Endpoints;
