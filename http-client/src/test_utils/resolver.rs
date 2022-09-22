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
