mod client;
mod regions;

pub use client::{
    APIResult, Authorization, AuthorizationError, AuthorizationResult, CachedResolver, Callbacks,
    CallbacksBuilder, Chooser, ChosenResult, Client, ClientBuilder, DefaultRetrier,
    DefaultRetrierBuilder, ExponentialRetryDelayPolicy, FixedRetryDelayPolicy, Idempotent,
    NeverRetrier, PersistentError, PersistentResult, Queries, QueryKey, QueryValue, Request,
    RequestBuilder, RequestRetrier, ResolveError, ResolveResult, Resolver, Response, ResponseBody,
    ResponseBuilder, ResponseError, ResponseErrorKind, RetryDelayPolicy, RetryResult,
    SimpleChooser, SimpleResolver, NO_DELAY_POLICY,
};
pub use regions::{Region, RegionBuilder, RegionProvider};
