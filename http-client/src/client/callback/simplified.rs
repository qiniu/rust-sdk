use super::super::{Authorization, Idempotent, QueryPair};
use auto_impl::auto_impl;
use qiniu_http::{HeaderMap, Method, UserAgent, Version};
use std::fmt::Debug;

/// 简化回调函数上下文
///
/// 用于在回调函数中获取请求相关信息，如请求路径、请求方法、查询参数、请求头等。
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait SimplifiedCallbackContext: Sync + Send + Debug {
    /// 是否使用 HTTPS 协议
    fn use_https(&self) -> bool;

    /// 获取请求 HTTP 方法
    fn method(&self) -> &Method;

    /// 获取请求 HTTP 版本
    fn version(&self) -> Version;

    /// 获取请求路径
    fn path(&self) -> &str;

    /// 获取请求查询参数
    fn query(&self) -> &str;

    /// 获取请求查询对
    fn query_pairs(&self) -> &[QueryPair];

    /// 获取请求 HTTP Headers
    fn headers(&self) -> &HeaderMap;

    /// 获取追加的 UserAgent
    fn appended_user_agent(&self) -> &UserAgent;

    /// 获取七牛鉴权签名
    fn authorization(&self) -> Option<&Authorization>;

    /// 获取请求幂等性
    fn idempotent(&self) -> Idempotent;
}
