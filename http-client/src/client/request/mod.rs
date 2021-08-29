mod builder;
mod built;
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
