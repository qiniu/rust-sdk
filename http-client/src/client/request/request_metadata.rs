use super::{super::authorization::Authorization, Idempotent, QueryPair};
use qiniu_http::{HeaderMap, Method, Version};
use std::borrow::Cow;

#[derive(Default, Debug)]
pub(super) struct RequestMetadata<'r> {
    pub(super) use_https: Option<bool>,
    pub(super) method: Method,
    pub(super) version: Version,
    pub(super) path: Cow<'r, str>,
    pub(super) query: Cow<'r, str>,
    pub(super) query_pairs: Vec<QueryPair<'r>>,
    pub(super) headers: Cow<'r, HeaderMap>,
    pub(super) authorization: Option<Authorization<'r>>,
    pub(super) idempotent: Idempotent,
}
