use super::super::{Authorization, Idempotent, QueryPairs};
use auto_impl::auto_impl;
use qiniu_http::{HeaderMap, Method, UserAgent, Version};
use std::fmt::Debug;

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait SimplifiedCallbackContext: Sync + Send + Debug {
    fn use_https(&self) -> bool;
    fn method(&self) -> &Method;
    fn version(&self) -> Version;
    fn path(&self) -> &str;
    fn query(&self) -> &str;
    fn query_pairs(&self) -> &QueryPairs;
    fn headers(&self) -> &HeaderMap;
    fn appended_user_agent(&self) -> &UserAgent;
    fn authorization(&self) -> Option<&Authorization>;
    fn idempotent(&self) -> Idempotent;
}
