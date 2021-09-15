#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![deny(
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    explicit_outlives_requirements,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    non_ascii_idents,
    indirect_structural_match,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]

mod client;
mod regions;

#[cfg(test)]
mod test_utils;

#[cfg(any(feature = "isahc"))]
pub extern crate qiniu_isahc as isahc;
#[cfg(any(feature = "reqwest"))]
pub extern crate qiniu_reqwest as reqwest;

pub extern crate qiniu_credential as credential;
pub extern crate qiniu_http as http;
pub extern crate qiniu_upload_token as upload_token;

pub use client::{
    APIResult, Authorization, AuthorizationError, AuthorizationResult, CachedResolver,
    CallbackContext, Callbacks, CallbacksBuilder, ChainedResolver, ChainedResolverBuilder, Chooser,
    ChooserFeedback, DomainOrIpAddr, ErrorRetrier, ExponentialRetryDelayPolicy,
    ExtendedCallbackContext, FixedRetryDelayPolicy, HTTPClient, HTTPClientBuilder, Idempotent,
    IpChooser, IpChooserBuilder, LimitedRetrier, NeverRetrier, PersistentError, PersistentResult,
    QueryPairKey, QueryPairValue, QueryPairs, RandomizedRetryDelayPolicy, Ratio, RequestBuilder,
    RequestRetrier, ResolveAnswers, ResolveResult, Resolver, ResponseError, ResponseErrorKind,
    ResponseInfo, RetryDelayPolicy, RetryResult, ShuffledChooser, ShuffledResolver, SimpleResolver,
    SimplifiedCallbackContext, SubnetChooser, SubnetChooserBuilder, SyncResponse, TimeoutResolver,
    NO_DELAY_POLICY,
};
pub use regions::{
    BucketRegionsProvider, BucketRegionsQueryer, BucketRegionsQueryerBuilder,
    CachedRegionsProvider, DomainWithPort, DomainWithPortParseError, Endpoint, EndpointParseError,
    Endpoints, EndpointsBuilder, IntoEndpoints, InvalidServiceName, IpAddrWithPort,
    IpAddrWithPortParseError, Region, RegionBuilder, RegionProvider, RegionsProvider, ServiceName,
    StaticRegionProvider,
};

#[cfg(any(feature = "c_ares"))]
pub use client::{c_ares, c_ares_resolver, CAresResolver};

#[cfg(all(feature = "trust_dns", feature = "async"))]
pub use client::{trust_dns_resolver, TrustDnsResolver};

#[cfg(any(feature = "async"))]
pub use client::AsyncResponse;

pub mod preclude {
    pub use super::{
        client::{
            CallbackContext, Chooser, ExtendedCallbackContext, RequestRetrier, Resolver,
            RetryDelayPolicy, SimplifiedCallbackContext,
        },
        credential::CredentialProvider,
        http::{HTTPCaller, Metrics, ReadDebug},
        regions::RegionProvider,
        upload_token::UploadTokenProvider,
    };

    #[cfg(any(feature = "async"))]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub use super::http::{AsyncReadDebug, AsyncReadSeekDebug};
}
