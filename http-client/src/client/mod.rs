mod authorization;
mod backoff;
mod call;
mod callback;
mod callbacks;
mod chooser;
mod http_client;
mod request;
mod resolver;
mod response;
mod retried;
mod retrier;

pub use authorization::{Authorization, AuthorizationError, AuthorizationResult};
pub use backoff::{
    Backoff, BackoffDuration, BackoffOptions, ExponentialBackoff, FixedBackoff, RandomizedBackoff,
    Ratio, NO_BACKOFF,
};
pub use call::DomainOrIpAddr;
pub use callback::{
    CallbackContext, ExtendedCallbackContext, ResponseInfo, SimplifiedCallbackContext,
};
pub use callbacks::{
    Callbacks, CallbacksBuilder, OnDomainResolved, OnError, OnHeader, OnIPsChosen, OnProgress,
    OnRequest, OnRetry, OnStatusCode, OnSuccess, OnToChooseIPs, OnToResolveDomain,
};
pub use chooser::{
    Chooser, ChooserFeedback, IpChooser, IpChooserBuilder, NeverEmptyHandedChooser,
    ShuffledChooser, SubnetChooser, SubnetChooserBuilder,
};
pub use http_client::{HttpClient, HttpClientBuilder};
pub use request::{
    FieldName, FileName, Idempotent, Multipart, Part, PartMetadata, QueryPairKey, QueryPairValue,
    QueryPairs, RequestBuilder, SyncMultipart, SyncPart, SyncPartBody, SyncRequestBody,
    SyncRequestBuilder,
};
pub use resolver::{
    CachedResolver, ChainedResolver, ChainedResolverBuilder, ResolveAnswers, ResolveOptions,
    ResolveResult, Resolver, ShuffledResolver, SimpleResolver, TimeoutResolver,
};
pub use response::{ApiResult, Response, ResponseError, ResponseErrorKind, SyncResponse};
pub use retried::RetriedStatsInfo;
pub use retrier::{
    ErrorRetrier, LimitedRetrier, NeverRetrier, RequestRetrier, RequestRetrierOptions,
    RetryDecision, RetryResult,
};

#[cfg(feature = "c_ares")]
pub use resolver::{c_ares, c_ares_resolver, CAresResolver};

#[cfg(all(feature = "trust_dns", feature = "async"))]
pub use resolver::{trust_dns_resolver, TrustDnsResolver};

#[cfg(feature = "async")]
pub use {
    request::{AsyncMultipart, AsyncPart, AsyncPartBody, AsyncRequestBody, AsyncRequestBuilder},
    response::AsyncResponse,
};

use call::request_call;

#[cfg(feature = "async")]
use call::async_request_call;

use callback::{CallbackContextImpl, ExtendedCallbackContextImpl};
use request::RequestParts;
