mod client;
mod regions;

pub use client::{
    APIResult, Authorization, AuthorizationError, AuthorizationResult, CachedResolver, Callbacks,
    CallbacksBuilder, Chooser, ChosenResult, Client, ClientBuilder, DefaultRetrier,
    DefaultRetrierBuilder, Idempotent, NeverRetrier, PersistentError, PersistentResult, Queries,
    QueryKey, QueryValue, Request, RequestBuilder, RequestRetrier, ResolveError, ResolveResult,
    Resolver, Response, ResponseBody, ResponseBuilder, ResponseError, ResponseErrorKind,
    RetryResult, SimpleChooser, SimpleResolver,
};
pub use regions::{Region, RegionBuilder, RegionProvider};
