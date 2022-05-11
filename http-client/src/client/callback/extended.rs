use super::{
    super::{Authorization, Idempotent, InnerRequestParts, QueryPair, RetriedStatsInfo},
    context::CallbackContext,
    simplified::SimplifiedCallbackContext,
};
use auto_impl::auto_impl;
use qiniu_http::{
    uri::Scheme, Extensions, HeaderMap, Method, RequestParts as HttpRequestParts, Uri, UserAgent, Version,
};
use std::{borrow::Cow, net::IpAddr};

/// 扩展的回调函数上下文
///
/// 基于回调函数上下文，并在此基础上增加返回部分请求信息的可变引用，以及 UserAgent 和经过解析的 IP 地址列表的获取和设置方法。
#[auto_impl(&mut, Box)]
pub trait ExtendedCallbackContext: CallbackContext {
    /// 获取 HTTP 请求 URL
    fn url(&self) -> &Uri;

    /// 获取请求 HTTP 版本的可变引用
    fn version_mut(&mut self) -> &mut Version;

    /// 获取请求 HTTP Headers 的可变引用
    fn headers_mut(&mut self) -> &mut HeaderMap;

    /// 获取 UserAgent
    fn user_agent(&self) -> UserAgent;

    /// 设置追加的 UserAgent
    fn set_appended_user_agent(&mut self, appended_user_agent: UserAgent);

    /// 获取经过解析的 IP 地址列表
    fn resolved_ip_addrs(&self) -> Option<&[IpAddr]>;

    /// 设置经过解析的 IP 地址列表
    fn set_resolved_ip_addrs(&mut self, resolved_ip_addrs: Vec<IpAddr>);

    /// 获取重试统计信息
    fn retried(&self) -> &RetriedStatsInfo;
}

#[derive(Debug)]
pub(in super::super) struct ExtendedCallbackContextImpl<'reqref, 'req, 'retried, 'httpreqref, 'httpreq> {
    request: &'reqref InnerRequestParts<'req>,
    http_request: &'httpreqref mut HttpRequestParts<'httpreq>,
    retried: &'retried RetriedStatsInfo,
}

impl<'reqref, 'req, 'retried, 'httpreqref, 'httpreq>
    ExtendedCallbackContextImpl<'reqref, 'req, 'retried, 'httpreqref, 'httpreq>
{
    pub(in super::super) fn new(
        request: &'reqref InnerRequestParts<'req>,
        http_request: &'httpreqref mut HttpRequestParts<'httpreq>,
        retried: &'retried RetriedStatsInfo,
    ) -> Self {
        Self {
            request,
            http_request,
            retried,
        }
    }
}

impl SimplifiedCallbackContext for ExtendedCallbackContextImpl<'_, '_, '_, '_, '_> {
    #[inline]
    fn use_https(&self) -> bool {
        self.http_request.url().scheme() == Some(&Scheme::HTTPS)
    }

    #[inline]
    fn method(&self) -> &Method {
        self.http_request.method()
    }

    #[inline]
    fn version(&self) -> Version {
        self.http_request.version()
    }

    #[inline]
    fn path(&self) -> &str {
        self.request.path()
    }

    #[inline]
    fn query(&self) -> &str {
        self.request.query()
    }

    #[inline]
    fn query_pairs(&self) -> &[QueryPair] {
        self.request.query_pairs()
    }

    #[inline]
    fn headers(&self) -> &HeaderMap {
        self.http_request.headers()
    }

    #[inline]
    fn appended_user_agent(&self) -> &UserAgent {
        self.http_request.appended_user_agent()
    }

    #[inline]
    fn authorization(&self) -> Option<&Authorization> {
        self.request.authorization()
    }

    #[inline]
    fn idempotent(&self) -> Idempotent {
        self.request.idempotent()
    }
}

impl CallbackContext for ExtendedCallbackContextImpl<'_, '_, '_, '_, '_> {
    #[inline]
    fn extensions(&self) -> &Extensions {
        self.http_request.extensions()
    }

    #[inline]
    fn extensions_mut(&mut self) -> &mut Extensions {
        self.http_request.extensions_mut()
    }
}

impl ExtendedCallbackContext for ExtendedCallbackContextImpl<'_, '_, '_, '_, '_> {
    #[inline]
    fn url(&self) -> &Uri {
        self.http_request.url()
    }

    #[inline]
    fn version_mut(&mut self) -> &mut Version {
        self.http_request.version_mut()
    }

    #[inline]
    fn headers_mut(&mut self) -> &mut HeaderMap {
        self.http_request.headers_mut()
    }

    #[inline]
    fn user_agent(&self) -> UserAgent {
        self.http_request.user_agent()
    }

    #[inline]
    fn set_appended_user_agent(&mut self, appended_user_agent: UserAgent) {
        *self.http_request.appended_user_agent_mut() = appended_user_agent;
    }

    #[inline]
    fn resolved_ip_addrs(&self) -> Option<&[IpAddr]> {
        self.http_request.resolved_ip_addrs()
    }

    #[inline]
    fn set_resolved_ip_addrs(&mut self, resolved_ip_addrs: Vec<IpAddr>) {
        *self.http_request.resolved_ip_addrs_mut() = Some(Cow::Owned(resolved_ip_addrs));
    }

    #[inline]
    fn retried(&self) -> &RetriedStatsInfo {
        self.retried
    }
}
