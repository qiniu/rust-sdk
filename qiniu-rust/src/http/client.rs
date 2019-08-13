use super::super::config::Config;
use super::super::utils::auth::Auth;
use super::request;
use qiniu_http::Method;
use std::sync::Arc;

pub struct Client {
    auth: Arc<Auth>,
    config: Arc<Config>,
}

impl Client {
    pub fn new(auth: Arc<Auth>, config: Arc<Config>) -> Client {
        Client {
            auth: auth,
            config: config,
        }
    }

    pub fn get<Hosts, Host, Path>(&self, path: Path, hosts: Hosts) -> request::Builder
    where
        Path: ToString,
        Host: ToString,
        Hosts: AsRef<[Host]>,
    {
        self.request_builder(Method::GET, path, hosts)
    }

    pub fn post<Hosts, Host, Path>(&self, path: Path, hosts: Hosts) -> request::Builder
    where
        Path: ToString,
        Host: ToString,
        Hosts: AsRef<[Host]>,
    {
        self.request_builder(Method::POST, path, hosts)
    }

    pub fn put<Hosts, Host, Path>(&self, path: Path, hosts: Hosts) -> request::Builder
    where
        Path: ToString,
        Host: ToString,
        Hosts: AsRef<[Host]>,
    {
        self.request_builder(Method::PUT, path, hosts)
    }

    pub fn delete<Hosts, Host, Path>(&self, path: Path, hosts: Hosts) -> request::Builder
    where
        Path: ToString,
        Host: ToString,
        Hosts: AsRef<[Host]>,
    {
        self.request_builder(Method::DELETE, path, hosts)
    }

    pub fn patch<Hosts, Host, Path>(&self, path: Path, hosts: Hosts) -> request::Builder
    where
        Path: ToString,
        Host: ToString,
        Hosts: AsRef<[Host]>,
    {
        self.request_builder(Method::PATCH, path, hosts)
    }

    pub fn head<Hosts, Host, Path>(&self, path: Path, hosts: Hosts) -> request::Builder
    where
        Path: ToString,
        Host: ToString,
        Hosts: AsRef<[Host]>,
    {
        self.request_builder(Method::HEAD, path, hosts)
    }

    fn request_builder<Hosts, Host, Path>(
        &self,
        method: Method,
        path: Path,
        hosts: Hosts,
    ) -> request::Builder
    where
        Path: ToString,
        Host: ToString,
        Hosts: AsRef<[Host]>,
    {
        request::Builder::new(self.auth.clone(), self.config.clone(), method, path, hosts)
    }
}
