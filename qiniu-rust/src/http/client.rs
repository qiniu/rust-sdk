use super::request;
use crate::config::Config;
use assert_impl::assert_impl;
use qiniu_http::Method;

#[derive(Clone)]
pub(crate) struct Client {
    config: Config,
}

impl Client {
    pub(crate) fn new(config: Config) -> Client {
        Client { config }
    }

    pub(crate) fn get<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::GET, path, hosts)
    }

    pub(crate) fn post<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::POST, path, hosts)
    }

    pub(crate) fn put<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::PUT, path, hosts)
    }

    pub(crate) fn delete<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::DELETE, path, hosts)
    }

    pub(crate) fn patch<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::PATCH, path, hosts)
    }

    pub(crate) fn head<'a>(&self, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        self.request_builder(Method::HEAD, path, hosts)
    }

    fn request_builder<'a>(&self, method: Method, path: &'a str, hosts: &'a [&'a str]) -> request::Builder<'a> {
        request::Builder::new(self.config.clone(), method, path, hosts)
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}
