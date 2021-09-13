mod authorization;
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
mod retry_delay_policy;
mod spawn;

pub use authorization::{Authorization, AuthorizationError, AuthorizationResult};
pub use call::DomainOrIpAddr;
pub use callback::{
    CallbackContext, ExtendedCallbackContext, ResponseInfo, SimplifiedCallbackContext,
};
pub use callbacks::{Callbacks, CallbacksBuilder};
pub use chooser::{
    Chooser, ChooserFeedback, IpChooser, IpChooserBuilder, NeverChooseNoneChooser, ShuffledChooser,
    SubnetChooser, SubnetChooserBuilder,
};
pub use http_client::{HTTPClient, HTTPClientBuilder};
pub use request::{Idempotent, QueryPairKey, QueryPairValue, QueryPairs, RequestBuilder};
pub use resolver::{
    CachedResolver, ChainedResolver, ChainedResolverBuilder, PersistentError, PersistentResult,
    ResolveAnswers, ResolveResult, Resolver, ShuffledResolver, SimpleResolver, TimeoutResolver,
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

use callback::{CallbackContextImpl, ExtendedCallbackContextImpl};
use request::{Request, RequestWithoutEndpoints};
