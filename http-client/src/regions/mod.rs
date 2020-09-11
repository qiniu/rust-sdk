mod domains;
mod provider;
mod region;

pub(super) use domains::{Domains, IntoDomains, ServiceName};
pub use provider::RegionProvider;
pub use region::{Region, RegionBuilder};
