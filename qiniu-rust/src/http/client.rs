use super::request::Builder as RequestBuilder;
use crate::{config::Config, http::Method};
use assert_impl::assert_impl;
use getset::Getters;

#[derive(Clone, Getters)]
#[get = "pub(crate)"]
pub(crate) struct Client {
    config: Config,
}

impl Client {
    pub(crate) fn new(config: Config) -> Client {
        Client { config }
    }

    pub(crate) fn get<'a>(&self, path: &'a str, base_urls: &'a [&'a str]) -> RequestBuilder<'a> {
        self.request_builder(Method::GET, path, base_urls)
    }

    pub(crate) fn post<'a>(&self, path: &'a str, base_urls: &'a [&'a str]) -> RequestBuilder<'a> {
        self.request_builder(Method::POST, path, base_urls)
    }

    pub(crate) fn put<'a>(&self, path: &'a str, base_urls: &'a [&'a str]) -> RequestBuilder<'a> {
        self.request_builder(Method::PUT, path, base_urls)
    }

    pub(crate) fn head<'a>(&self, path: &'a str, base_urls: &'a [&'a str]) -> RequestBuilder<'a> {
        self.request_builder(Method::HEAD, path, base_urls)
    }

    fn request_builder<'a>(&self, method: Method, path: &'a str, base_urls: &'a [&'a str]) -> RequestBuilder<'a> {
        RequestBuilder::new(self.config.clone(), method, path, base_urls)
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}
