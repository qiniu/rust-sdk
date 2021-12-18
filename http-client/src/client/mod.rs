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

pub use authorization::{
    Authorization, AuthorizationError, AuthorizationProvider, AuthorizationResult,
    CredentialAuthorizationV1, CredentialAuthorizationV2, UploadTokenAuthorization,
};
pub use backoff::{
    Backoff, BackoffDuration, BackoffOptions, ExponentialBackoff, FixedBackoff, LimitedBackoff,
    RandomizedBackoff, Ratio, NO_BACKOFF,
};
pub use call::DomainOrIpAddr;
pub use callback::{CallbackContext, ExtendedCallbackContext, SimplifiedCallbackContext};
pub use callbacks::{Callbacks, CallbacksBuilder};
pub use chooser::{
    Chooser, ChooserFeedback, DirectChooser, IpChooser, IpChooserBuilder, NeverEmptyHandedChooser,
    ShuffledChooser, SubnetChooser, SubnetChooserBuilder,
};
pub use http_client::{HttpClient, HttpClientBuilder};
pub use request::{
    FieldName, FileName, Idempotent, Multipart, Part, PartMetadata, QueryPairKey, QueryPairValue,
    QueryPairs, RequestBuilder, RequestBuilderParts, SyncMultipart, SyncPart, SyncPartBody,
    SyncRequestBody, SyncRequestBuilder,
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
