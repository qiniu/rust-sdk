mod authorization;
mod call;
mod callbacks;
mod chooser;
mod client;
mod request;
mod resolver;
mod response;
mod retried;
mod retrier;
mod retry_delay_policy;

pub use authorization::{Authorization, AuthorizationError, AuthorizationResult};
pub use call::DomainOrIpAddr;
pub use callbacks::{CallbackContext, Callbacks, CallbacksBuilder, RequestInfo, ResponseInfo};
pub use chooser::{
    Chooser, ChooserFeedback, ChosenResult, ResponseMetrics, ShuffledChooser,
    ShuffledChooserBuilder, SimpleChooser,
};
pub use client::{Client, ClientBuilder};
pub use request::{Idempotent, QueryPairKey, QueryPairValue, QueryPairs, RequestBuilder};
pub use resolver::{
    CachedResolver, ChainedResolver, ChainedResolverBuilder, PersistentError, PersistentResult,
    ResolveResult, Resolver, ShuffledResolver, SimpleResolver,
};
pub use response::{APIResult, Response, ResponseError, ResponseErrorKind, SyncResponse};
pub use retried::RetriedStatsInfo;
pub use retrier::{
    DefaultRetrier, DefaultRetrierBuilder, NeverRetrier, RequestRetrier, RetryResult,
};
pub use retry_delay_policy::{
    ExponentialRetryDelayPolicy, FixedRetryDelayPolicy, RandomizedRetryDelayPolicy, Ratio,
    RetryDelayPolicy, NO_DELAY_POLICY,
};

#[cfg(any(feature = "c_ares"))]
pub use resolver::{c_ares, c_ares_resolver, CAresResolver};

#[cfg(any(feature = "async"))]
pub use response::AsyncResponse;

use call::request_call;

#[cfg(any(feature = "async"))]
use call::async_request_call;

use request::{Request, RequestWithoutEndpoints};
