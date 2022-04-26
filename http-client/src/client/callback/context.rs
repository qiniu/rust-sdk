use super::{
    super::{Authorization, Idempotent, InnerRequestParts, QueryPair},
    simplified::SimplifiedCallbackContext,
};
use auto_impl::auto_impl;
use qiniu_http::{Extensions, HeaderMap, Method, UserAgent, Version};

/// 回调函数上下文
///
/// 基于简化回调函数上下文，并在此基础上增加获取扩展信息的引用和可变引用的方法。
#[auto_impl(&mut, Box)]
pub trait CallbackContext: SimplifiedCallbackContext {
    /// 获取扩展信息
    fn extensions(&self) -> &Extensions;

    /// 获取扩展信息的可变引用
    fn extensions_mut(&mut self) -> &mut Extensions;
}

#[derive(Debug)]
pub(in super::super) struct CallbackContextImpl<'reqref, 'req, 'ext> {
    request: &'reqref InnerRequestParts<'req>,
    extensions: &'ext mut Extensions,
}

impl<'reqref, 'req, 'ext> CallbackContextImpl<'reqref, 'req, 'ext> {
    pub(in super::super) fn new(request: &'reqref InnerRequestParts<'req>, extensions: &'ext mut Extensions) -> Self {
        Self { request, extensions }
    }
}

impl SimplifiedCallbackContext for CallbackContextImpl<'_, '_, '_> {
    #[inline]
    fn use_https(&self) -> bool {
        self.request.use_https()
    }

    #[inline]
    fn method(&self) -> &Method {
        self.request.method()
    }

    #[inline]
    fn version(&self) -> Version {
        self.request.version()
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
        self.request.headers()
    }

    #[inline]
    fn appended_user_agent(&self) -> &UserAgent {
        self.request.appended_user_agent()
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

impl CallbackContext for CallbackContextImpl<'_, '_, '_> {
    #[inline]
    fn extensions(&self) -> &Extensions {
        self.extensions
    }

    #[inline]
    fn extensions_mut(&mut self) -> &mut Extensions {
        self.extensions
    }
}
