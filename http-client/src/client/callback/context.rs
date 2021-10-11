use super::{
    super::{Authorization, Idempotent, QueryPairs, RequestWithoutEndpoints},
    simplified::SimplifiedCallbackContext,
};
use qiniu_http::{Extensions, HeaderMap, Method, UserAgent, Version};

pub trait CallbackContext: SimplifiedCallbackContext {
    fn extensions(&self) -> &Extensions;
    fn extensions_mut(&mut self) -> &mut Extensions;
}

#[derive(Debug)]
pub(in super::super) struct CallbackContextImpl<'reqref, 'req, 'ext> {
    request: &'reqref RequestWithoutEndpoints<'req>,
    extensions: &'ext mut Extensions,
}

impl<'reqref, 'req, 'ext> CallbackContextImpl<'reqref, 'req, 'ext> {
    pub(in super::super) fn new(
        request: &'reqref RequestWithoutEndpoints<'req>,
        extensions: &'ext mut Extensions,
    ) -> Self {
        Self {
            request,
            extensions,
        }
    }
}

impl<'reqref, 'req, 'ext> SimplifiedCallbackContext for CallbackContextImpl<'reqref, 'req, 'ext> {
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
    fn query_pairs(&self) -> &QueryPairs {
        self.request.query_pairs()
    }

    #[inline]
    fn headers(&self) -> &HeaderMap {
        self.request.headers()
    }

    #[inline]
    fn body(&self) -> &[u8] {
        self.request.body()
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

impl<'reqref, 'req, 'ext> CallbackContext for CallbackContextImpl<'reqref, 'req, 'ext> {
    #[inline]
    fn extensions(&self) -> &Extensions {
        self.extensions
    }

    #[inline]
    fn extensions_mut(&mut self) -> &mut Extensions {
        self.extensions
    }
}
