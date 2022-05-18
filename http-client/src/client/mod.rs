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
    global_disable_timestamp_signature, global_enable_timestamp_signature, Authorization, AuthorizationError,
    AuthorizationProvider, AuthorizationResult, CredentialAuthorizationV1, CredentialAuthorizationV2,
    DownloadUrlCredentialAuthorization, UploadTokenAuthorization,
};
pub use backoff::{
    Backoff, BackoffOptions, BackoffOptionsBuilder, ExponentialBackoff, FixedBackoff, GotBackoffDuration,
    LimitedBackoff, RandomizedBackoff, Ratio, NO_BACKOFF,
};
pub use callback::{CallbackContext, ExtendedCallbackContext, SimplifiedCallbackContext};
pub use chooser::{
    ChooseOptions, ChooseOptionsBuilder, Chooser, ChooserFeedback, ChosenResults, DirectChooser, IpChooser,
    IpChooserBuilder, NeverEmptyHandedChooser, ShuffledChooser, SubnetChooser, SubnetChooserBuilder,
};
pub use http_client::{HttpClient, HttpClientBuilder};
pub use request::{
    FieldName, FileName, Idempotent, Multipart, Part, PartMetadata, QueryPair, QueryPairKey, QueryPairValue,
    RequestBuilder, RequestBuilderParts, RequestParts, SyncMultipart, SyncPart, SyncPartBody, SyncRequestBody,
    SyncRequestBuilder,
};
pub use resolver::{
    CachedResolver, CachedResolverBuilder, ChainedResolver, ChainedResolverBuilder, ResolveAnswers, ResolveOptions,
    ResolveResult, Resolver, ShuffledResolver, SimpleResolver, TimeoutResolver,
};
pub use response::{ApiResult, Response, ResponseError, ResponseErrorKind, SyncResponse};
pub use retried::RetriedStatsInfo;
pub use retrier::{
    ErrorRetrier, LimitedRetrier, NeverRetrier, RequestRetrier, RequestRetrierOptions, RequestRetrierOptionsBuilder,
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
use request::InnerRequestParts;
