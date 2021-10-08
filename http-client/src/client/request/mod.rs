mod builder;
mod built;
mod multipart;
mod request_data;

use std::borrow::Cow;

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

pub use builder::RequestBuilder;
pub(super) use built::{Request, RequestWithoutEndpoints};
pub use multipart::{FieldName, FileName, Multipart, Part, SyncBody, SyncMultipart, SyncPart};

#[cfg(feature = "async")]
pub use multipart::{AsyncBody, AsyncMultipart, AsyncPart};
