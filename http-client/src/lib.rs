#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![deny(unsafe_code)]

mod client;
mod regions;

#[cfg(test)]
mod test_utils;

pub extern crate qiniu_credential as credential;
pub extern crate qiniu_http as http;
pub extern crate qiniu_upload_token as upload_token;

pub use client::{
    APIResult, Authorization, AuthorizationError, AuthorizationResult, CachedResolver,
    CallbackContext, Callbacks, CallbacksBuilder, ChainedResolver, ChainedResolverBuilder, Chooser,
    ChooserFeedback, ChosenResult, Client, ClientBuilder, DefaultRetrier, DefaultRetrierBuilder,
    DomainOrIpAddr, ExponentialRetryDelayPolicy, FixedRetryDelayPolicy, Idempotent, NeverRetrier,
    PersistentError, PersistentResult, QueryPairKey, QueryPairValue, QueryPairs,
    RandomizedRetryDelayPolicy, Ratio, RequestBuilder, RequestInfo, RequestRetrier, ResolveResult,
    Resolver, ResponseError, ResponseErrorKind, RetryDelayPolicy, RetryResult, ShuffledChooser,
    ShuffledChooserBuilder, ShuffledResolver, SimpleChooser, SimpleResolver, SyncResponse,
    NO_DELAY_POLICY,
};
pub use regions::{
    BucketRegionsProvider, BucketRegionsQueryer, BucketRegionsQueryerBuilder, DomainWithPort,
    DomainWithPortParseError, Endpoint, EndpointParseError, IntoEndpoints, IpAddrWithPort,
    IpAddrWithPortParseError, Region, RegionBuilder, RegionProvider, RegionsProvider,
    RegionsProviderBuilder, ServiceName,
};

#[cfg(any(feature = "c_ares"))]
pub use client::{c_ares, c_ares_resolver, CAresResolver};

#[cfg(any(feature = "async"))]
pub use client::AsyncResponse;
