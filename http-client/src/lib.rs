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
    trivial_numeric_casts,
    unsafe_code,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]

mod cache;
mod client;
mod regions;
mod spawn;

#[cfg(test)]
mod test_utils;

#[cfg(feature = "ureq")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "ureq")))]
pub use qiniu_ureq as ureq;

#[cfg(feature = "isahc")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "isahc")))]
pub use qiniu_isahc as isahc;

#[cfg(feature = "reqwest")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "reqwest")))]
pub use qiniu_reqwest as reqwest;

pub use qiniu_credential as credential;
pub use qiniu_http as http;
pub use qiniu_upload_token as upload_token;

#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub use futures::io::AsyncRead;

pub use cache::{CacheController, PersistentError, PersistentResult};
pub use client::{
    APIResult, Authorization, AuthorizationError, AuthorizationResult, Backoff, BackoffDuration,
    BackoffOptions, CachedResolver, CallbackContext, Callbacks, CallbacksBuilder, ChainedResolver,
    ChainedResolverBuilder, Chooser, ChooserFeedback, DomainOrIpAddr, ErrorRetrier,
    ExponentialBackoff, ExtendedCallbackContext, FieldName, FileName, FixedBackoff, HTTPClient,
    HTTPClientBuilder, Idempotent, IpChooser, IpChooserBuilder, LimitedRetrier, Multipart,
    NeverRetrier, Part, QueryPairKey, QueryPairValue, QueryPairs, RandomizedBackoff, Ratio,
    RequestBuilder, RequestRetrier, RequestRetrierOptions, ResolveAnswers, ResolveOptions,
    ResolveResult, Resolver, ResponseError, ResponseErrorKind, ResponseInfo, RetryDecision,
    RetryResult, ShuffledChooser, ShuffledResolver, SimpleResolver, SimplifiedCallbackContext,
    SubnetChooser, SubnetChooserBuilder, SyncBody, SyncMultipart, SyncPart, SyncResponse,
    TimeoutResolver, NO_BACKOFF,
};
pub use regions::{
    BucketRegionsProvider, BucketRegionsQueryer, BucketRegionsQueryerBuilder,
    CachedRegionsProvider, DomainWithPort, DomainWithPortParseError, Endpoint, EndpointParseError,
    Endpoints, EndpointsBuilder, GetOptions, GotRegion, GotRegions, IntoEndpoints,
    InvalidServiceName, IpAddrWithPort, IpAddrWithPortParseError, Region, RegionBuilder,
    RegionProvider, RegionsProvider, ServiceName, StaticRegionProvider,
};
pub use upload_token::{BucketName, ObjectName};

#[cfg(feature = "c_ares")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "c_ares")))]
pub use client::{c_ares, c_ares_resolver, CAresResolver};

#[cfg(all(feature = "trust_dns", feature = "async"))]
#[cfg_attr(
    feature = "docs",
    doc(cfg(all(feature = "trust_dns", feature = "async")))
)]
pub use client::{trust_dns_resolver, TrustDnsResolver};

#[cfg(feature = "async")]
pub use client::{AsyncBody, AsyncMultipart, AsyncPart, AsyncResponse};

pub mod preclude {
    pub use super::{
        client::{
            Backoff, CallbackContext, Chooser, ExtendedCallbackContext, RequestRetrier, Resolver,
            SimplifiedCallbackContext,
        },
        credential::CredentialProvider,
        http::{HTTPCaller, Metrics},
        regions::RegionProvider,
        upload_token::UploadTokenProvider,
    };
}
