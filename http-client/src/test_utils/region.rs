use super::super::Region;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

pub(crate) fn single_up_domain_region() -> Region {
    Region::builder("chaotic")
        .push_up_endpoint(("fakedomain.withport.com".to_owned(), 8080))
        .build()
}

pub(crate) fn chaotic_up_domains_region() -> Region {
    Region::builder("chaotic")
        .push_up_endpoint(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
        .push_up_endpoint("fakedomain.withoutport.com".to_owned())
        .push_up_endpoint(IpAddr::V6(Ipv6Addr::new(
            0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff,
        )))
        .push_up_endpoint(SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00b, 0x2ff),
            8081,
            0,
            0,
        )))
        .push_up_endpoint(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(192, 168, 1, 2),
            8080,
        )))
        .push_up_endpoint(("fakedomain.withport.com".to_owned(), 8080))
        .push_up_old_endpoint(IpAddr::V4(Ipv4Addr::new(192, 168, 2, 1)))
        .push_up_old_endpoint("old_fakedomain.withoutport.com".to_owned())
        .push_up_old_endpoint(IpAddr::V6(Ipv6Addr::new(
            0, 0, 0, 0, 0, 0xffff, 0xd00a, 0x2ff,
        )))
        .push_up_old_endpoint(SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xd00b, 0x2ff),
            8081,
            0,
            0,
        )))
        .push_up_old_endpoint(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(192, 168, 2, 2),
            8080,
        )))
        .push_up_old_endpoint(("old_fakedomain.withport.com".to_owned(), 8080))
        .build()
}
