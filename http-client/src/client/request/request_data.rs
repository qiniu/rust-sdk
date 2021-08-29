use super::{super::authorization::Authorization, Idempotent, QueryPairs};
use qiniu_http::{Extensions, HeaderMap, Method, RequestBody, Version};
use std::{borrow::Cow, fmt};

pub(super) struct RequestData<'r> {
    pub(super) use_https: Option<bool>,
    pub(super) method: Method,
    pub(super) version: Version,
    pub(super) path: Cow<'r, str>,
    pub(super) query: Cow<'r, str>,
    pub(super) query_pairs: QueryPairs<'r>,
    pub(super) headers: Cow<'r, HeaderMap>,
    pub(super) body: RequestBody<'r>,
    pub(super) authorization: Option<Authorization>,
    pub(super) idempotent: Idempotent,
    pub(super) extensions: Extensions,
}

impl fmt::Debug for RequestData<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        macro_rules! field {
            ($ctx:ident,$method:ident) => {
                $ctx.field("$method", &self.$method)
            };
        }
        let s = &mut f.debug_struct("RequestData");
        field!(s, use_https);
        field!(s, method);
        field!(s, version);
        field!(s, path);
        field!(s, query_pairs);
        field!(s, headers);
        field!(s, body);
        field!(s, authorization);
        field!(s, idempotent);
        field!(s, extensions);
        s.finish()
    }
}
