mod authorization;
mod callbacks;
mod chooser;
mod client;
mod request;
mod resolver;
mod retrier;

pub use authorization::{Authorization, AuthorizationError, AuthorizationResult};
pub use callbacks::{Callbacks, CallbacksBuilder};
pub use chooser::{Chooser, ChosenResult, SimpleChooser};
pub use client::{Client, ClientBuilder};
pub use request::{Idempotent, Queries, QueryKey, QueryValue, Request, RequestBuilder};
pub use resolver::{
    CachedResolver, PersistentError, PersistentResult, ResolveError, ResolveResult, Resolver,
    SimpleResolver,
};
pub use retrier::{NeverRetry, RequestRetrier, RetryResult};

// TODO: 提供能力：Response
