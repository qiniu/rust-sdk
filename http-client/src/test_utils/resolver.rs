use super::super::{ResolveOptions, ResolveResult, Resolver, ResponseError, ResponseErrorKind};
use rand::{prelude::*, thread_rng};
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

pub(crate) fn make_dumb_resolver() -> impl Resolver + Clone {
    #[derive(Debug, Clone)]
    struct FakeResolver;

    impl Resolver for FakeResolver {
        fn resolve(&self, _domain: &str, _opts: ResolveOptions) -> ResolveResult {
            Ok(vec![].into())
        }
    }

    FakeResolver
}

pub(crate) fn make_static_resolver(ip_addrs: Arc<[IpAddr]>) -> impl Resolver + Clone {
    #[derive(Debug, Clone)]
    struct StaticResolver(Arc<[IpAddr]>);

    impl Resolver for StaticResolver {
        fn resolve(&self, _domain: &str, _opts: ResolveOptions) -> ResolveResult {
            Ok(self.0.to_vec().into())
        }
    }

    StaticResolver(ip_addrs)
}

pub(crate) fn make_random_resolver() -> impl Resolver + Clone {
    #[derive(Debug, Clone, Copy)]
    struct RandomResolver;

    impl Resolver for RandomResolver {
        fn resolve(&self, _domain: &str, _opts: ResolveOptions) -> ResolveResult {
            let ips = vec![IpAddr::V4(Ipv4Addr::from(thread_rng().gen::<u32>()))];
            Ok(ips.into())
        }
    }

    RandomResolver
}

pub(crate) fn make_error_resolver(error_kind: ResponseErrorKind, message: impl Into<String>) -> impl Resolver + Clone {
    #[derive(Debug, Clone)]
    struct ErrorResolver {
        error_kind: ResponseErrorKind,
        message: String,
    }

    impl Resolver for ErrorResolver {
        fn resolve(&self, _domain: &str, _opts: ResolveOptions) -> ResolveResult {
            Err(ResponseError::new_with_msg(self.error_kind, self.message.to_owned()))
        }
    }

    ErrorResolver {
        error_kind,
        message: message.into(),
    }
}

#[cfg(all(feature = "async", any(feature = "c_ares", feature = "trust_dns")))]
mod mock_dns_server {
    use super::*;
    use std::collections::{BTreeMap, HashMap};
    use tokio::net::UdpSocket;
    use trust_dns_server::{
        authority::{AuthorityObject, Catalog, ZoneType},
        client::rr::RrKey,
        proto::rr::{rdata::SOA, DNSClass, Name, RData, Record, RecordSet, RecordType},
        store::in_memory::InMemoryAuthority,
        ServerFuture,
    };

    pub(crate) fn start_mock_dns_server(
        listen: UdpSocket,
        zone_authorities: HashMap<Name, Box<dyn AuthorityObject>>,
    ) -> ServerFuture<Catalog> {
        let mut server_future = ServerFuture::new(make_dns_authority_catalog(zone_authorities));
        server_future.register_socket(listen);
        return server_future;

        fn make_dns_authority_catalog(zone_authorities: HashMap<Name, Box<dyn AuthorityObject>>) -> Catalog {
            let mut catalog = Catalog::new();
            for (name, authority) in zone_authorities {
                catalog.upsert(name.into(), authority);
            }
            catalog
        }
    }

    pub(crate) fn make_zone<I: IntoIterator<Item = (Name, RecordType, RecordSet)>>(
        origin: Name,
        record_infos: I,
    ) -> Result<Box<dyn AuthorityObject>, String> {
        let mut records = BTreeMap::<RrKey, RecordSet>::new();
        for (name, record_type, record_set) in record_infos.into_iter() {
            let key = RrKey::new(name.into(), record_type);
            records.insert(key, record_set);
        }
        {
            let (key, soa_record_set) = make_soa(origin.to_owned());
            records.insert(key, soa_record_set);
        }
        return make_dns_authority_object(origin, records);

        fn make_dns_authority_object(
            origin: Name,
            records: BTreeMap<RrKey, RecordSet>,
        ) -> Result<Box<dyn AuthorityObject>, String> {
            let authority = InMemoryAuthority::new(origin, records, ZoneType::Primary, false)?;
            Ok(Box::new(Arc::new(authority)))
        }

        fn make_soa(name: Name) -> (RrKey, RecordSet) {
            let mut record_set = RecordSet::new(&name, RecordType::SOA, 0);
            let mut record = Record::new();
            record
                .set_name(name.to_owned())
                .set_ttl(3600)
                .set_rr_type(RecordType::SOA)
                .set_dns_class(DNSClass::IN)
                .set_data(Some(RData::SOA(SOA::new(
                    Name::from_str_relaxed("sns.dns.icann.org.").unwrap(),
                    Name::from_str_relaxed("noc.dns.icann.org.").unwrap(),
                    20,
                    7200,
                    600,
                    3600000,
                    60,
                ))));

            record_set.insert(record, 0);
            (RrKey::new(name.into(), RecordType::SOA), record_set)
        }
    }

    pub(crate) fn make_record_set<I: IntoIterator<Item = (Name, u32, RData)>>(
        name: Name,
        record_type: RecordType,
        ttl: u32,
        records: I,
    ) -> RecordSet {
        let mut record_set = RecordSet::with_ttl(name, record_type, ttl);
        for (name, ttl, rdata) in records.into_iter() {
            record_set.insert(Record::from_rdata(name, ttl, rdata), 0);
        }
        record_set
    }
}

#[cfg(all(feature = "async", any(feature = "c_ares", feature = "trust_dns")))]
pub(crate) use mock_dns_server::{make_record_set, make_zone, start_mock_dns_server};
