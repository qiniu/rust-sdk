mod authorization;
mod callbacks;
mod client;
mod request_retrier;
mod resolver;

pub use authorization::{Authorization, AuthorizationError, AuthorizationResult};
pub use callbacks::{Callbacks, CallbacksBuilder};
pub use client::{Client, ClientBuilder};
pub use request_retrier::{NeverRetry, RequestRetrier, RetryResult};
pub use resolver::{
    CachedResolver, PersistentError, PersistentResult, ResolveError, ResolveResult, Resolver,
    SimpleResolver,
};

// TODO: 提供能力：ClientBuilder
// TODO: 提供能力：Request
