use super::{HeaderName, HeaderValue, Headers, Method};
use getset::{CopyGetters, Getters, MutGetters};
use std::{borrow::Cow, fmt, result::Result};

pub type URL<'b> = Cow<'b, str>;
pub type Body = [u8];

#[derive(Getters, CopyGetters, MutGetters, Clone)]
pub struct Request<'b> {
    #[get_mut = "pub"]
    url: URL<'b>,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    method: Method,

    #[get = "pub"]
    #[get_mut = "pub"]
    headers: Headers<'b>,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    body: Option<&'b Body>,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    follow_redirection: bool,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    on_uploading_progress: Option<&'b dyn Fn(usize, usize)>,

    #[get_copy = "pub"]
    #[get_mut = "pub"]
    on_downloading_progress: Option<&'b dyn Fn(usize, usize)>,
}

impl<'b> Request<'b> {
    fn new<U: Into<URL<'b>>>(
        method: Method,
        url: U,
        headers: Headers<'b>,
        body: Option<&'b Body>,
        follow_redirection: bool,
        on_uploading_progress: Option<&'b dyn Fn(usize, usize)>,
        on_downloading_progress: Option<&'b dyn Fn(usize, usize)>,
    ) -> Request<'b> {
        Request {
            url: url.into(),
            method: method,
            headers: headers,
            body: body,
            follow_redirection: follow_redirection,
            on_uploading_progress: on_uploading_progress,
            on_downloading_progress: on_downloading_progress,
        }
    }

    pub fn url(&self) -> &str {
        self.url.as_ref()
    }
}

pub struct RequestBuilder<'r> {
    request: Request<'r>,
}

impl<'r> RequestBuilder<'r> {
    pub fn default() -> RequestBuilder<'r> {
        RequestBuilder {
            request: Default::default(),
        }
    }

    pub fn method<M: Into<Method>>(mut self, method: M) -> RequestBuilder<'r> {
        self.request.method = method.into();
        self
    }

    pub fn url<U: Into<URL<'r>>>(mut self, url: U) -> RequestBuilder<'r> {
        self.request.url = url.into();
        self
    }

    pub fn header<HeaderNameT: Into<HeaderName<'r>>, HeaderValueT: Into<HeaderValue<'r>>>(
        mut self,
        header_name: HeaderNameT,
        header_value: HeaderValueT,
    ) -> RequestBuilder<'r> {
        self.request.headers.insert(header_name.into(), header_value.into());
        self
    }

    pub fn headers(mut self, headers: Headers<'r>) -> RequestBuilder<'r> {
        self.request.headers = headers;
        self
    }

    pub fn body(mut self, body: &'r [u8]) -> RequestBuilder<'r> {
        self.request.body = Some(body);
        self
    }

    pub fn follow_redirection(mut self, follow_redirection: bool) -> RequestBuilder<'r> {
        self.request.follow_redirection = follow_redirection;
        self
    }

    pub fn on_uploading_progress(mut self, callback: &'r dyn Fn(usize, usize)) -> RequestBuilder<'r> {
        self.request.on_uploading_progress = Some(callback);
        self
    }

    pub fn on_downloading_progress(mut self, callback: &'r dyn Fn(usize, usize)) -> RequestBuilder<'r> {
        self.request.on_downloading_progress = Some(callback);
        self
    }

    pub fn build(self) -> Request<'r> {
        self.request
    }
}

impl Default for Request<'_> {
    fn default() -> Self {
        Self::new(Method::GET, "http://localhost", Headers::new(), None, false, None, None)
    }
}

impl fmt::Debug for Request<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("Request")
            .field("url", &self.url)
            .field("method", &self.method)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .field("follow_redirection", &self.follow_redirection)
            .field(
                "on_uploading_progress",
                if self.on_uploading_progress.is_some() {
                    &"Installed"
                } else {
                    &"Not Installed"
                },
            )
            .field(
                "on_downloading_progress",
                if self.on_downloading_progress.is_some() {
                    &"Installed"
                } else {
                    &"Not Installed"
                },
            )
            .finish()
    }
}
