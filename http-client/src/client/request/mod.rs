mod builder;
mod built;
mod multipart;
mod request_metadata;

pub use qiniu_http::SyncRequestBody;
use std::borrow::Cow;

#[cfg(feature = "async")]
pub use qiniu_http::AsyncRequestBody;

pub type QueryPairKey<'q> = Cow<'q, str>;
pub type QueryPairValue<'q> = Cow<'q, str>;
pub type QueryPairs<'q> = Vec<(QueryPairKey<'q>, QueryPairValue<'q>)>;

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Idempotent {
    Always,
    Default,
    Never,
}

impl Default for Idempotent {
    #[inline]
    fn default() -> Self {
        Self::Default
    }
}

pub use builder::{RequestBuilder, SyncRequestBuilder};
pub(super) use built::{Request, RequestParts};
pub use multipart::{FieldName, FileName, Multipart, Part, SyncMultipart, SyncPart, SyncPartBody};

#[cfg(feature = "async")]
pub use {
    builder::AsyncRequestBuilder,
    multipart::{AsyncMultipart, AsyncPart, AsyncPartBody},
};

/// 同步 HTTP 请求
pub(super) type SyncRequest<'r> = Request<'r, SyncRequestBody<'r>>;

/// 异步 HTTP 请求
#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub(super) type AsyncRequest<'r> = Request<'r, AsyncRequestBody<'r>>;
