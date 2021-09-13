use super::super::{Authorization, Idempotent, QueryPairs};
use qiniu_http::{HeaderMap, Method, Version};

pub trait SimplifiedCallbackContext: Sync + Send {
    fn use_https(&self) -> bool;
    fn method(&self) -> &Method;
    fn version(&self) -> Version;
    fn path(&self) -> &str;
    fn query(&self) -> &str;
    fn query_pairs(&self) -> &QueryPairs;
    fn headers(&self) -> &HeaderMap;
    fn body(&self) -> &[u8];
    fn appended_user_agent(&self) -> &str;
    fn authorization(&self) -> Option<&Authorization>;
    fn idempotent(&self) -> Idempotent;
}
