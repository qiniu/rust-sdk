mod builder;
mod built;
mod multipart;
mod request_metadata;

pub use qiniu_http::SyncRequestBody;
use smart_default::SmartDefault;
use std::borrow::Cow;

#[cfg(feature = "async")]
pub use qiniu_http::AsyncRequestBody;

/// HTTP 查询参数名
pub type QueryPairKey<'q> = Cow<'q, str>;

/// HTTP 查询参数值
pub type QueryPairValue<'q> = Cow<'q, str>;

/// HTTP 查询参数对
pub type QueryPair<'q> = (QueryPairKey<'q>, QueryPairValue<'q>);

/// API 幂等性
#[derive(Clone, Copy, Debug, SmartDefault)]
#[non_exhaustive]
pub enum Idempotent {
    /// 总是幂等
    Always,

    /// 根据 HTTP 方法自动判定
    ///
    /// 参考 <https://datatracker.ietf.org/doc/html/rfc7231#section-4.2.2>
    #[default]
    Default,

    /// 不幂等
    Never,
}

pub use builder::{RequestBuilder, RequestBuilderParts, RequestParts, SyncRequestBuilder};
pub(super) use built::{InnerRequest, InnerRequestParts};
pub use multipart::{FieldName, FileName, Multipart, Part, PartMetadata, SyncMultipart, SyncPart, SyncPartBody};

#[cfg(feature = "async")]
pub use {
    builder::AsyncRequestBuilder,
    multipart::{AsyncMultipart, AsyncPart, AsyncPartBody},
};

pub(super) type SyncInnerRequest<'r, E> = InnerRequest<'r, SyncRequestBody<'r>, E>;

#[cfg(feature = "async")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub(super) type AsyncInnerRequest<'r, E> = InnerRequest<'r, AsyncRequestBody<'r>, E>;
