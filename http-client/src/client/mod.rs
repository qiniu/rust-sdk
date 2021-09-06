mod authorization;
mod call;
mod callbacks;
mod chooser;
mod http_client;
mod request;
mod resolver;
mod response;
mod retried;
mod retrier;
mod retry_delay_policy;
mod spawn;

pub use authorization::{Authorization, AuthorizationError, AuthorizationResult};
pub use call::DomainOrIpAddr;
pub use callbacks::{CallbackContext, Callbacks, CallbacksBuilder, RequestInfo, ResponseInfo};
pub use chooser::{
    Chooser, ChooserFeedback, DefaultChooser, NeverChooseNoneChooser, ShuffledChooser,
    SimpleChooser, DEFAULT_IPV4_NETMASK_PREFIX_LENGTH, DEFAULT_IPV6_NETMASK_PREFIX_LENGTH,
};
pub use http_client::{HTTPClient, HTTPClientBuilder};
pub use request::{Idempotent, QueryPairKey, QueryPairValue, QueryPairs, RequestBuilder};
pub use resolver::{
    CachedResolver, ChainedResolver, ChainedResolverBuilder, PersistentError, PersistentResult,
    ResolveAnswers, ResolveResult, Resolver, ShuffledResolver, SimpleResolver,
};
pub use response::{APIResult, Response, ResponseError, ResponseErrorKind, SyncResponse};
pub use retried::RetriedStatsInfo;
pub use retrier::{ErrorRetrier, LimitedRetrier, NeverRetrier, RequestRetrier, RetryResult};
pub use retry_delay_policy::{
    ExponentialRetryDelayPolicy, FixedRetryDelayPolicy, RandomizedRetryDelayPolicy, Ratio,
    RetryDelayPolicy, NO_DELAY_POLICY,
};

#[cfg(any(feature = "c_ares"))]
pub use resolver::{c_ares, c_ares_resolver, CAresResolver};

#[cfg(all(feature = "trust_dns", feature = "async"))]
pub use resolver::{trust_dns_resolver, TrustDnsResolver};

#[cfg(any(feature = "async"))]
pub use response::AsyncResponse;

use call::request_call;

#[cfg(any(feature = "async"))]
use call::async_request_call;

use request::{Request, RequestWithoutEndpoints};
