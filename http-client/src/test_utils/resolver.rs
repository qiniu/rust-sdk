use super::super::{ResolveOptions, ResolveResult, Resolver, ResponseError, ResponseErrorKind};
use rand::{prelude::*, thread_rng};
use std::{
    any::Any,
    net::{IpAddr, Ipv4Addr},
};

pub(crate) fn make_dumb_resolver() -> impl Resolver {
    #[derive(Debug)]
    struct FakeResolver;

    impl Resolver for FakeResolver {
        #[inline]
        fn resolve(&self, _domain: &str, _opts: &ResolveOptions) -> ResolveResult {
            Ok(Default::default())
        }

        #[inline]
        fn as_any(&self) -> &dyn Any {
            self
        }

        #[inline]
        fn as_resolver(&self) -> &dyn Resolver {
            self
        }
    }

    FakeResolver
}

pub(crate) fn make_static_resolver(ip_addrs: Box<[IpAddr]>) -> impl Resolver {
    #[derive(Debug)]
    struct StaticResolver(Box<[IpAddr]>);

    impl Resolver for StaticResolver {
        #[inline]
        fn resolve(&self, _domain: &str, _opts: &ResolveOptions) -> ResolveResult {
            Ok(self.0.to_owned().into())
        }

        #[inline]
        fn as_any(&self) -> &dyn Any {
            self
        }

        #[inline]
        fn as_resolver(&self) -> &dyn Resolver {
            self
        }
    }

    StaticResolver(ip_addrs)
}

pub(crate) fn make_random_resolver() -> impl Resolver {
    #[derive(Debug)]
    struct RandomResolver;

    impl Resolver for RandomResolver {
        #[inline]
        fn resolve(&self, _domain: &str, _opts: &ResolveOptions) -> ResolveResult {
            let ips = vec![IpAddr::V4(Ipv4Addr::from(thread_rng().gen::<u32>()))];
            Ok(ips.into())
        }

        #[inline]
        fn as_any(&self) -> &dyn Any {
            self
        }

        #[inline]
        fn as_resolver(&self) -> &dyn Resolver {
            self
        }
    }

    RandomResolver
}

pub(crate) fn make_error_resolver(
    error_kind: ResponseErrorKind,
    message: impl Into<String>,
) -> impl Resolver {
    #[derive(Debug)]
    struct ErrorResolver {
        error_kind: ResponseErrorKind,
        message: String,
    }

    impl Resolver for ErrorResolver {
        #[inline]
        fn resolve(&self, _domain: &str, _opts: &ResolveOptions) -> ResolveResult {
            Err(ResponseError::new(self.error_kind, self.message.to_owned()))
        }

        #[inline]
        fn as_any(&self) -> &dyn Any {
            self
        }

        #[inline]
        fn as_resolver(&self) -> &dyn Resolver {
            self
        }
    }

    ErrorResolver {
        error_kind,
        message: message.into(),
    }
}
