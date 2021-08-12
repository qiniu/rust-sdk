use super::client::Client;
use isahc::{
    auth::{Authentication, Credentials},
    config::{
        CaCertificate, ClientCertificate, Configurable, DnsCache, IpVersion, NetworkInterface,
        RedirectPolicy, ResolveMap, SslOption, VersionNegotiation,
    },
    http::Error as IsahcHttpError,
    Error as IsahcError, HttpClientBuilder as IsahcHttpClientBuilder,
};
use qiniu_http::{HeaderName, HeaderValue, Uri};
use std::{convert::TryFrom, time::Duration};

#[derive(Debug, Default)]
pub struct ClientBuilder {
    client_builder: IsahcHttpClientBuilder,
}

impl ClientBuilder {
    #[inline]
    pub fn connection_cache_ttl(mut self, ttl: Duration) -> Self {
        self.client_builder = self.client_builder.connection_cache_ttl(ttl);
        self
    }

    #[inline]
    pub fn max_connections(mut self, max: usize) -> Self {
        self.client_builder = self.client_builder.max_connections(max);
        self
    }

    #[inline]
    pub fn max_connections_per_host(mut self, max: usize) -> Self {
        self.client_builder = self.client_builder.max_connections_per_host(max);
        self
    }

    #[inline]
    pub fn connection_cache_size(mut self, size: usize) -> Self {
        self.client_builder = self.client_builder.connection_cache_size(size);
        self
    }

    #[inline]
    pub fn dns_cache(mut self, cache: impl Into<DnsCache>) -> Self {
        self.client_builder = self.client_builder.dns_cache(cache);
        self
    }

    #[inline]
    pub fn dns_resolve(mut self, map: ResolveMap) -> Self {
        self.client_builder = self.client_builder.dns_resolve(map);
        self
    }

    #[inline]
    pub fn default_header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<IsahcHttpError>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<IsahcHttpError>,
    {
        self.client_builder = self.client_builder.default_header(key, value);
        self
    }

    #[inline]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.client_builder = self.client_builder.timeout(timeout);
        self
    }

    #[inline]
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.client_builder = self.client_builder.connect_timeout(timeout);
        self
    }

    #[inline]
    pub fn low_speed_timeout(mut self, low_speed: u32, timeout: Duration) -> Self {
        self.client_builder = self.client_builder.low_speed_timeout(low_speed, timeout);
        self
    }

    #[inline]
    pub fn version_negotiation(mut self, negotiation: VersionNegotiation) -> Self {
        self.client_builder = self.client_builder.version_negotiation(negotiation);
        self
    }

    #[inline]
    pub fn redirect_policy(mut self, policy: RedirectPolicy) -> Self {
        self.client_builder = self.client_builder.redirect_policy(policy);
        self
    }

    #[inline]
    pub fn auto_referer(mut self) -> Self {
        self.client_builder = self.client_builder.auto_referer();
        self
    }

    #[inline]
    pub fn automatic_decompression(mut self, decompress: bool) -> Self {
        self.client_builder = self.client_builder.automatic_decompression(decompress);
        self
    }

    #[inline]
    pub fn authentication(mut self, authentication: Authentication) -> Self {
        self.client_builder = self.client_builder.authentication(authentication);
        self
    }

    #[inline]
    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.client_builder = self.client_builder.credentials(credentials);
        self
    }

    #[inline]
    pub fn tcp_keepalive(mut self, interval: Duration) -> Self {
        self.client_builder = self.client_builder.tcp_keepalive(interval);
        self
    }

    #[inline]
    pub fn tcp_nodelay(mut self) -> Self {
        self.client_builder = self.client_builder.tcp_nodelay();
        self
    }

    #[inline]
    pub fn interface(mut self, interface: impl Into<NetworkInterface>) -> Self {
        self.client_builder = self.client_builder.interface(interface);
        self
    }

    #[inline]
    pub fn ip_version(mut self, version: IpVersion) -> Self {
        self.client_builder = self.client_builder.ip_version(version);
        self
    }

    #[inline]
    pub fn proxy(mut self, proxy: impl Into<Option<Uri>>) -> Self {
        self.client_builder = self.client_builder.proxy(proxy);
        self
    }

    #[inline]
    pub fn proxy_blacklist(mut self, hosts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.client_builder = self.client_builder.proxy_blacklist(hosts);
        self
    }

    #[inline]
    pub fn proxy_authentication(mut self, authentication: Authentication) -> Self {
        self.client_builder = self.client_builder.proxy_authentication(authentication);
        self
    }

    #[inline]
    pub fn proxy_credentials(mut self, credentials: Credentials) -> Self {
        self.client_builder = self.client_builder.proxy_credentials(credentials);
        self
    }

    #[inline]
    pub fn max_upload_speed(mut self, max: u64) -> Self {
        self.client_builder = self.client_builder.max_upload_speed(max);
        self
    }

    #[inline]
    pub fn max_download_speed(mut self, max: u64) -> Self {
        self.client_builder = self.client_builder.max_download_speed(max);
        self
    }

    #[inline]
    pub fn ssl_client_certificate(mut self, certificate: ClientCertificate) -> Self {
        self.client_builder = self.client_builder.ssl_client_certificate(certificate);
        self
    }

    #[inline]
    pub fn ssl_ca_certificate(mut self, certificate: CaCertificate) -> Self {
        self.client_builder = self.client_builder.ssl_ca_certificate(certificate);
        self
    }

    #[inline]
    pub fn ssl_ciphers<I: IntoIterator<Item = T>, T: Into<String>>(mut self, ciphers: I) -> Self {
        self.client_builder = self.client_builder.ssl_ciphers(ciphers);
        self
    }

    #[inline]
    pub fn ssl_options(mut self, options: SslOption) -> Self {
        self.client_builder = self.client_builder.ssl_options(options);
        self
    }

    #[inline]
    pub fn title_case_headers(mut self, enable: bool) -> Self {
        self.client_builder = self.client_builder.title_case_headers(enable);
        self
    }

    #[inline]
    pub fn metrics(mut self, enable: bool) -> Self {
        self.client_builder = self.client_builder.metrics(enable);
        self
    }

    #[inline]
    pub fn build(self) -> Result<Client, IsahcError> {
        Ok(Client::new(self.client_builder.build()?))
    }
}
