use super::super::{Region, RegionProviderEndpoints};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

fn single_up_domain_region() -> Region {
    Region::builder("chaotic")
        .push_up_preferred_endpoint(("fakedomain.withport.com".to_owned(), 8080).into())
        .build()
}

pub(crate) fn single_up_domain_endpoint() -> RegionProviderEndpoints<Region> {
    RegionProviderEndpoints::new(single_up_domain_region())
}

pub(crate) fn chaotic_up_domains_region() -> Region {
    Region::builder("chaotic")
        .push_up_preferred_endpoint(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into())
        .push_up_preferred_endpoint("fakedomain.withoutport.com".parse().unwrap())
        .push_up_preferred_endpoint(
            IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff)).into(),
        )
        .push_up_preferred_endpoint(
            SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00b, 0x2ff),
                8081,
                0,
                0,
            ))
            .into(),
        )
        .push_up_preferred_endpoint(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 2), 8080)).into(),
        )
        .push_up_preferred_endpoint(("fakedomain.withport.com".to_owned(), 8080).into())
        .push_up_alternative_endpoint(IpAddr::V4(Ipv4Addr::new(192, 168, 2, 1)).into())
        .push_up_alternative_endpoint("alternative_fakedomain.withoutport.com".parse().unwrap())
        .push_up_alternative_endpoint(
            IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xd00a, 0x2ff)).into(),
        )
        .push_up_alternative_endpoint(
            SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xd00b, 0x2ff),
                8081,
                0,
                0,
            ))
            .into(),
        )
        .push_up_alternative_endpoint(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 2, 2), 8080)).into(),
        )
        .push_up_alternative_endpoint(
            ("alternative_fakedomain.withport.com".to_owned(), 8080).into(),
        )
        .build()
}

pub(crate) fn chaotic_up_domains_endpoint() -> RegionProviderEndpoints<Region> {
    RegionProviderEndpoints::new(chaotic_up_domains_region())
}
