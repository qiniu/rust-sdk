use super::{super::ResponseError, ResolveAnswers, ResolveResult, Resolver};
use async_std::task::block_on;
use async_std_resolver::AsyncStdResolver as AsyncResolver;
use async_std_resolver::{resolver, resolver_from_system_conf};
use futures::future::BoxFuture;
use qiniu_http::ResponseErrorKind as HTTPResponseErrorKind;
use std::{any::Any, fmt};
pub use trust_dns_resolver;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    error::ResolveError,
};

#[cfg_attr(feature = "docs", doc(all(cfg(trust_dns), cfg(r#async))))]
pub struct TrustDnsResolver {
    #[cfg(feature = "async")]
    resolver: AsyncResolver,
}

type TrustDnsResolveResult<T> = Result<T, ResolveError>;

impl TrustDnsResolver {
    #[inline]
    pub async fn new(config: ResolverConfig, options: ResolverOpts) -> TrustDnsResolveResult<Self> {
        Ok(Self {
            resolver: resolver(config, options).await?,
        })
    }

    #[inline]
    pub async fn default() -> TrustDnsResolveResult<Self> {
        Ok(Self {
            resolver: resolver(Default::default(), Default::default()).await?,
        })
    }

    #[inline]
    pub async fn from_system_conf() -> TrustDnsResolveResult<Self> {
        Ok(Self {
            resolver: resolver_from_system_conf().await?,
        })
    }
}

impl Resolver for TrustDnsResolver {
    fn resolve(&self, domain: &str) -> ResolveResult {
        block_on(async move { self.async_resolve(domain).await })
    }

    fn async_resolve<'a>(&'a self, domain: &'a str) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            Ok(ResolveAnswers::new(
                self.resolver
                    .lookup_ip(domain)
                    .await
                    .map_err(convert_trust_dns_error_to_response_error)?
                    .iter()
                    .collect(),
            ))
        })
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

impl fmt::Debug for TrustDnsResolver {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrustDnsResolver").finish()
    }
}

#[inline]
fn convert_trust_dns_error_to_response_error(err: ResolveError) -> ResponseError {
    ResponseError::new(HTTPResponseErrorKind::DNSServerError.into(), err)
}
