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
    Backoff, ExponentialBackoff, FixedBackoff, RandomizedBackoff, Ratio, NO_BACKOFF,
};
pub use call::DomainOrIpAddr;
pub use callback::{
    CallbackContext, ExtendedCallbackContext, ResponseInfo, SimplifiedCallbackContext,
};
pub use callbacks::{Callbacks, CallbacksBuilder};
pub use chooser::{
    Chooser, ChooserFeedback, IpChooser, IpChooserBuilder, NeverEmptyHandedChooser,
    ShuffledChooser, SubnetChooser, SubnetChooserBuilder,
};
pub use http_client::{HTTPClient, HTTPClientBuilder};
pub use request::{
    FieldName, FileName, Idempotent, Multipart, Part, QueryPairKey, QueryPairValue, QueryPairs,
    RequestBuilder, SyncBody, SyncMultipart, SyncPart,
};
pub use resolver::{
    CachedResolver, ChainedResolver, ChainedResolverBuilder, ResolveAnswers, ResolveResult,
    Resolver, ShuffledResolver, SimpleResolver, TimeoutResolver,
};
pub use response::{APIResult, Response, ResponseError, ResponseErrorKind, SyncResponse};
pub use retried::RetriedStatsInfo;
pub use retrier::{ErrorRetrier, LimitedRetrier, NeverRetrier, RequestRetrier, RetryResult};

#[cfg(feature = "c_ares")]
pub use resolver::{c_ares, c_ares_resolver, CAresResolver};

#[cfg(all(feature = "trust_dns", feature = "async"))]
pub use resolver::{trust_dns_resolver, TrustDnsResolver};

#[cfg(feature = "async")]
pub use {
    request::{AsyncBody, AsyncMultipart, AsyncPart},
    response::AsyncResponse,
};

use call::request_call;

#[cfg(feature = "async")]
use call::async_request_call;

use callback::{CallbackContextImpl, ExtendedCallbackContextImpl};
use request::{Request, RequestWithoutEndpoints};
