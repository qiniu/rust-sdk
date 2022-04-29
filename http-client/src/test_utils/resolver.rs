use super::super::{ResolveOptions, ResolveResult, Resolver, ResponseError, ResponseErrorKind};
use rand::{prelude::*, thread_rng};
use std::net::{IpAddr, Ipv4Addr};

pub(crate) fn make_dumb_resolver() -> impl Resolver {
    #[derive(Debug)]
    struct FakeResolver;

    impl Resolver for FakeResolver {
        fn resolve(&self, _domain: &str, _opts: ResolveOptions) -> ResolveResult {
            Ok(vec![].into())
        }
    }

    FakeResolver
}

pub(crate) fn make_static_resolver(ip_addrs: Box<[IpAddr]>) -> impl Resolver {
    #[derive(Debug)]
    struct StaticResolver(Box<[IpAddr]>);

    impl Resolver for StaticResolver {
        fn resolve(&self, _domain: &str, _opts: ResolveOptions) -> ResolveResult {
            Ok(self.0.to_owned().into())
        }
    }

    StaticResolver(ip_addrs)
}

pub(crate) fn make_random_resolver() -> impl Resolver {
    #[derive(Debug)]
    struct RandomResolver;

    impl Resolver for RandomResolver {
        fn resolve(&self, _domain: &str, _opts: ResolveOptions) -> ResolveResult {
            let ips = vec![IpAddr::V4(Ipv4Addr::from(thread_rng().gen::<u32>()))];
            Ok(ips.into())
        }
    }

    RandomResolver
}

pub(crate) fn make_error_resolver(error_kind: ResponseErrorKind, message: impl Into<String>) -> impl Resolver {
    #[derive(Debug)]
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
