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
    unstable_features,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]

mod client;
mod regions;

#[cfg(test)]
mod test_utils;

#[cfg(any(feature = "curl"))]
pub extern crate qiniu_curl as curl;

pub extern crate qiniu_credential as credential;
pub extern crate qiniu_http as http;
pub extern crate qiniu_upload_token as upload_token;

pub use client::{
    APIResult, Authorization, AuthorizationError, AuthorizationResult, CachedResolver,
    CallbackContext, Callbacks, CallbacksBuilder, ChainedResolver, ChainedResolverBuilder, Chooser,
    ChooserFeedback, ChosenResult, DefaultRetrier, DefaultRetrierBuilder, DomainOrIpAddr,
    ExponentialRetryDelayPolicy, FixedRetryDelayPolicy, HTTPClient, HTTPClientBuilder, Idempotent,
    NeverRetrier, PersistentError, PersistentResult, QueryPairKey, QueryPairValue, QueryPairs,
    RandomizedRetryDelayPolicy, Ratio, RequestBuilder, RequestInfo, RequestRetrier, ResolveResult,
    Resolver, ResponseError, ResponseErrorKind, ResponseMetrics, RetryDelayPolicy, RetryResult,
    ShuffledChooser, ShuffledChooserBuilder, ShuffledResolver, SimpleChooser, SimpleResolver,
    SyncResponse, NO_DELAY_POLICY,
};
pub use regions::{
    BucketRegionsProvider, BucketRegionsQueryer, BucketRegionsQueryerBuilder, DomainWithPort,
    DomainWithPortParseError, Endpoint, EndpointParseError, IntoEndpoints, InvalidServiceName,
    IpAddrWithPort, IpAddrWithPortParseError, Region, RegionBuilder, RegionProvider,
    RegionsProvider, RegionsProviderBuilder, ServiceName, StaticRegionProvider,
};

#[cfg(any(feature = "c_ares"))]
pub use client::{c_ares, c_ares_resolver, CAresResolver};

#[cfg(any(feature = "async"))]
pub use client::AsyncResponse;

pub mod preclude {
    pub use super::{
        client::{Chooser, RequestRetrier, Resolver, RetryDelayPolicy},
        credential::CredentialProvider,
        http::{HTTPCaller, ReadDebug},
        regions::RegionProvider,
        upload_token::UploadTokenProvider,
    };

    #[cfg(any(feature = "async"))]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub use super::http::{AsyncReadDebug, AsyncReadSeekDebug};
}
