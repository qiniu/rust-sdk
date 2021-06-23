use super::SyncReqwestHTTPCaller;
use qiniu_http::HeaderMap;
use reqwest::{
    blocking::ClientBuilder as SyncClientBuilder, Certificate, Proxy, Result as ReqwestResult,
};
use std::{net::IpAddr, time::Duration};

#[cfg(feature = "async")]
use super::AsyncReqwestHTTPCaller;

#[cfg(feature = "async")]
use reqwest::ClientBuilder as AsyncClientBuilder;

#[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
use reqwest::Identity;

#[derive(Debug, Default)]
struct Config {
    default_headers: Option<HeaderMap>,
    gzip: Option<bool>,
    brotli: Option<bool>,
    deflate: Option<bool>,
    referer: Option<bool>,
    proxy: Option<Proxy>,
    no_proxy: Option<bool>,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    connection_verbose: Option<bool>,
    pool_idle_timeout: Option<Duration>,
    pool_max_idle_per_host: Option<usize>,
    http1_title_case_headers: bool,
    http2_prior_knowledge: bool,
    http2_initial_stream_window_size: Option<u32>,
    http2_initial_connection_window_size: Option<u32>,
    http2_adaptive_window: Option<bool>,
    http2_max_frame_size: Option<u32>,
    tcp_nodelay: Option<bool>,
    local_address: Option<IpAddr>,
    tcp_keepalive: Option<Duration>,

    #[cfg(any(
        feature = "default-tls",
        feature = "native-tls",
        feature = "rustls-tls"
    ))]
    add_root_certificate: Option<Certificate>,

    #[cfg(any(
        feature = "default-tls",
        feature = "native-tls",
        feature = "rustls-tls"
    ))]
    tls_built_in_root_certs: Option<bool>,

    #[cfg(feature = "native-tls")]
    danger_accept_invalid_hostnames: Option<bool>,

    #[cfg(any(
        feature = "default-tls",
        feature = "native-tls",
        feature = "rustls-tls"
    ))]
    danger_accept_invalid_certs: Option<bool>,

    #[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
    identity: Option<Identity>,

    #[cfg(feature = "native-tls")]
    use_native_tls: bool,

    #[cfg(feature = "rustls-tls")]
    use_rustls_tls: bool,

    #[cfg(feature = "trust-dns")]
    trust_dns: Option<bool>,

    no_trust_dns: bool,
    https_only: Option<bool>,
}

#[derive(Debug, Default)]
pub struct ReqwestHTTPCallerBuilder {
    config: Config,
}

impl ReqwestHTTPCallerBuilder {
    #[inline]
    pub fn build_sync(self) -> ReqwestResult<SyncReqwestHTTPCaller> {
        SyncReqwestHTTPCaller::new(self)
    }

    #[inline]
    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub fn build_async(self) -> ReqwestResult<AsyncReqwestHTTPCaller> {
        AsyncReqwestHTTPCaller::new(self)
    }
}

impl ReqwestHTTPCallerBuilder {
    #[inline]
    pub fn default_headers(mut self, default_headers: HeaderMap) -> Self {
        self.config.default_headers = Some(default_headers);
        self
    }

    #[inline]
    pub fn gzip(mut self, gzip: bool) -> Self {
        self.config.gzip = Some(gzip);
        self
    }

    #[inline]
    pub fn brotli(mut self, brotli: bool) -> Self {
        self.config.brotli = Some(brotli);
        self
    }

    #[inline]
    pub fn deflate(mut self, deflate: bool) -> Self {
        self.config.deflate = Some(deflate);
        self
    }

    #[inline]
    pub fn referer(mut self, referer: bool) -> Self {
        self.config.referer = Some(referer);
        self
    }

    #[inline]
    pub fn proxy(mut self, proxy: Proxy) -> Self {
        self.config.proxy = Some(proxy);
        self
    }

    #[inline]
    pub fn no_proxy(mut self, no_proxy: bool) -> Self {
        self.config.no_proxy = Some(no_proxy);
        self
    }

    #[inline]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }

    #[inline]
    pub fn connect_timeout(mut self, connect_timeout: Duration) -> Self {
        self.config.connect_timeout = Some(connect_timeout);
        self
    }

    #[inline]
    pub fn connection_verbose(mut self, connection_verbose: bool) -> Self {
        self.config.connection_verbose = Some(connection_verbose);
        self
    }

    #[inline]
    pub fn pool_idle_timeout(mut self, pool_idle_timeout: Duration) -> Self {
        self.config.pool_idle_timeout = Some(pool_idle_timeout);
        self
    }

    #[inline]
    pub fn pool_max_idle_per_host(mut self, pool_max_idle_per_host: usize) -> Self {
        self.config.pool_max_idle_per_host = Some(pool_max_idle_per_host);
        self
    }

    #[inline]
    pub fn http1_title_case_headers(mut self) -> Self {
        self.config.http1_title_case_headers = true;
        self
    }

    #[inline]
    pub fn http2_prior_knowledge(mut self) -> Self {
        self.config.http1_title_case_headers = true;
        self
    }

    #[inline]
    pub fn http2_initial_stream_window_size(
        mut self,
        http2_initial_stream_window_size: u32,
    ) -> Self {
        self.config.http2_initial_stream_window_size = Some(http2_initial_stream_window_size);
        self
    }

    #[inline]
    pub fn http2_initial_connection_window_size(
        mut self,
        http2_initial_connection_window_size: u32,
    ) -> Self {
        self.config.http2_initial_connection_window_size =
            Some(http2_initial_connection_window_size);
        self
    }

    #[inline]
    pub fn http2_adaptive_window(mut self, http2_adaptive_window: bool) -> Self {
        self.config.http2_adaptive_window = Some(http2_adaptive_window);
        self
    }

    #[inline]
    pub fn http2_max_frame_size(mut self, http2_max_frame_size: u32) -> Self {
        self.config.http2_max_frame_size = Some(http2_max_frame_size);
        self
    }

    #[inline]
    pub fn tcp_nodelay(mut self, tcp_nodelay: bool) -> Self {
        self.config.tcp_nodelay = Some(tcp_nodelay);
        self
    }

    #[inline]
    pub fn local_address(mut self, local_address: IpAddr) -> Self {
        self.config.local_address = Some(local_address);
        self
    }

    #[inline]
    pub fn tcp_keepalive(mut self, tcp_keepalive: Duration) -> Self {
        self.config.tcp_keepalive = Some(tcp_keepalive);
        self
    }

    #[inline]
    #[cfg(any(
        feature = "default-tls",
        feature = "native-tls",
        feature = "rustls-tls"
    ))]
    pub fn add_root_certificate(mut self, add_root_certificate: Certificate) -> Self {
        self.config.add_root_certificate = Some(add_root_certificate);
        self
    }

    #[inline]
    #[cfg(any(
        feature = "default-tls",
        feature = "native-tls",
        feature = "rustls-tls"
    ))]
    pub fn tls_built_in_root_certs(mut self, tls_built_in_root_certs: bool) -> Self {
        self.config.tls_built_in_root_certs = Some(tls_built_in_root_certs);
        self
    }

    #[inline]
    #[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
    pub fn identity(mut self, identity: Identity) -> Self {
        self.config.identity = Some(identity);
        self
    }

    #[inline]
    #[cfg(feature = "native-tls")]
    pub fn danger_accept_invalid_hostnames(
        mut self,
        danger_accept_invalid_hostnames: bool,
    ) -> Self {
        self.config.danger_accept_invalid_hostnames = Some(danger_accept_invalid_hostnames);
        self
    }

    #[inline]
    #[cfg(any(
        feature = "default-tls",
        feature = "native-tls",
        feature = "rustls-tls"
    ))]
    pub fn danger_accept_invalid_certs(mut self, danger_accept_invalid_certs: bool) -> Self {
        self.config.danger_accept_invalid_certs = Some(danger_accept_invalid_certs);
        self
    }

    #[inline]
    #[cfg(feature = "native-tls")]
    pub fn use_native_tls(mut self) -> Self {
        self.config.use_native_tls = true;
        self
    }

    #[inline]
    #[cfg(feature = "rustls-tls")]
    pub fn use_rustls_tls(mut self) -> Self {
        self.config.use_rustls_tls = true;
        self
    }

    #[inline]
    #[cfg(feature = "trust-dns")]
    pub fn trust_dns(mut self, trust_dns: bool) -> Self {
        self.config.trust_dns = Some(trust_dns);
        self
    }

    #[inline]
    pub fn no_trust_dns(mut self) -> Self {
        self.config.no_trust_dns = true;
        self
    }

    #[inline]
    pub fn https_only(mut self, https_only: bool) -> Self {
        self.config.https_only = Some(https_only);
        self
    }

    pub fn build_sync_client_builder(self) -> SyncClientBuilder {
        let mut builder = SyncClientBuilder::new();
        if let Some(enable) = self.config.gzip {
            builder = builder.gzip(enable);
        }
        if let Some(enable) = self.config.brotli {
            builder = builder.brotli(enable);
        }
        if let Some(enable) = self.config.deflate {
            builder = builder.deflate(enable);
        }
        if let Some(enable) = self.config.referer {
            builder = builder.referer(enable);
        }
        if let Some(proxy) = self.config.proxy {
            builder = builder.proxy(proxy);
        }
        if let Some(timeout) = self.config.timeout {
            builder = builder.timeout(timeout);
        }
        if let Some(connect_timeout) = self.config.connect_timeout {
            builder = builder.connect_timeout(connect_timeout);
        }
        if let Some(enable) = self.config.connection_verbose {
            builder = builder.connection_verbose(enable);
        }
        if let Some(timeout) = self.config.pool_idle_timeout {
            builder = builder.pool_idle_timeout(timeout);
        }
        if let Some(value) = self.config.pool_max_idle_per_host {
            builder = builder.pool_max_idle_per_host(value);
        }
        if self.config.http1_title_case_headers {
            builder = builder.http1_title_case_headers();
        }
        if self.config.http2_prior_knowledge {
            builder = builder.http2_prior_knowledge();
        }
        if let Some(window_size) = self.config.http2_initial_stream_window_size {
            builder = builder.http2_initial_stream_window_size(window_size);
        }
        if let Some(window_size) = self.config.http2_initial_connection_window_size {
            builder = builder.http2_initial_connection_window_size(window_size);
        }
        if let Some(enable) = self.config.http2_adaptive_window {
            builder = builder.http2_adaptive_window(enable);
        }
        if let Some(size) = self.config.http2_max_frame_size {
            builder = builder.http2_max_frame_size(size);
        }
        if let Some(enable) = self.config.tcp_nodelay {
            builder = builder.tcp_nodelay(enable);
        }
        if let Some(local_address) = self.config.local_address {
            builder = builder.local_address(local_address);
        }
        if let Some(tcp_keepalive) = self.config.tcp_keepalive {
            builder = builder.tcp_keepalive(tcp_keepalive);
        }

        #[cfg(any(
            feature = "default-tls",
            feature = "native-tls",
            feature = "rustls-tls"
        ))]
        if let Some(add_root_certificate) = self.config.add_root_certificate {
            builder = builder.add_root_certificate(add_root_certificate);
        }

        #[cfg(any(
            feature = "default-tls",
            feature = "native-tls",
            feature = "rustls-tls"
        ))]
        if let Some(tls_built_in_root_certs) = self.config.tls_built_in_root_certs {
            builder = builder.tls_built_in_root_certs(tls_built_in_root_certs);
        }

        #[cfg(feature = "native-tls")]
        if let Some(danger_accept_invalid_hostnames) = self.config.danger_accept_invalid_hostnames {
            builder = builder.danger_accept_invalid_hostnames(danger_accept_invalid_hostnames);
        }

        #[cfg(any(
            feature = "default-tls",
            feature = "native-tls",
            feature = "rustls-tls"
        ))]
        if let Some(danger_accept_invalid_certs) = self.config.danger_accept_invalid_certs {
            builder = builder.danger_accept_invalid_certs(danger_accept_invalid_certs);
        }

        #[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
        if let Some(identity) = self.config.identity {
            builder = builder.identity(identity);
        }

        #[cfg(feature = "native-tls")]
        if self.config.use_native_tls {
            builder = builder.use_native_tls();
        }

        #[cfg(feature = "rustls-tls")]
        if self.config.use_rustls_tls {
            builder = builder.use_rustls_tls();
        }

        #[cfg(feature = "trust-dns")]
        if let Some(enable) = self.config.trust_dns {
            builder = builder.trust_dns(enable);
        }

        if self.config.no_trust_dns {
            builder = builder.no_trust_dns();
        }
        if let Some(enable) = self.config.https_only {
            builder = builder.https_only(enable);
        }
        builder
    }

    #[cfg(feature = "async")]
    pub fn build_async_client_builder(self) -> AsyncClientBuilder {
        let mut builder = AsyncClientBuilder::new();
        if let Some(enable) = self.config.gzip {
            builder = builder.gzip(enable);
        }
        if let Some(enable) = self.config.brotli {
            builder = builder.brotli(enable);
        }
        if let Some(enable) = self.config.deflate {
            builder = builder.deflate(enable);
        }
        if let Some(enable) = self.config.referer {
            builder = builder.referer(enable);
        }
        if let Some(proxy) = self.config.proxy {
            builder = builder.proxy(proxy);
        }
        if let Some(timeout) = self.config.timeout {
            builder = builder.timeout(timeout);
        }
        if let Some(connect_timeout) = self.config.connect_timeout {
            builder = builder.connect_timeout(connect_timeout);
        }
        if let Some(enable) = self.config.connection_verbose {
            builder = builder.connection_verbose(enable);
        }
        if let Some(timeout) = self.config.pool_idle_timeout {
            builder = builder.pool_idle_timeout(timeout);
        }
        if let Some(value) = self.config.pool_max_idle_per_host {
            builder = builder.pool_max_idle_per_host(value);
        }
        if self.config.http1_title_case_headers {
            builder = builder.http1_title_case_headers();
        }
        if self.config.http2_prior_knowledge {
            builder = builder.http2_prior_knowledge();
        }
        if let Some(window_size) = self.config.http2_initial_stream_window_size {
            builder = builder.http2_initial_stream_window_size(window_size);
        }
        if let Some(window_size) = self.config.http2_initial_connection_window_size {
            builder = builder.http2_initial_connection_window_size(window_size);
        }
        if let Some(enable) = self.config.http2_adaptive_window {
            builder = builder.http2_adaptive_window(enable);
        }
        if let Some(size) = self.config.http2_max_frame_size {
            builder = builder.http2_max_frame_size(size);
        }
        if let Some(enable) = self.config.tcp_nodelay {
            builder = builder.tcp_nodelay(enable);
        }
        if let Some(local_address) = self.config.local_address {
            builder = builder.local_address(local_address);
        }
        if let Some(tcp_keepalive) = self.config.tcp_keepalive {
            builder = builder.tcp_keepalive(tcp_keepalive);
        }

        #[cfg(any(
            feature = "default-tls",
            feature = "native-tls",
            feature = "rustls-tls"
        ))]
        if let Some(add_root_certificate) = self.config.add_root_certificate {
            builder = builder.add_root_certificate(add_root_certificate);
        }

        #[cfg(any(
            feature = "default-tls",
            feature = "native-tls",
            feature = "rustls-tls"
        ))]
        if let Some(tls_built_in_root_certs) = self.config.tls_built_in_root_certs {
            builder = builder.tls_built_in_root_certs(tls_built_in_root_certs);
        }

        #[cfg(feature = "native-tls")]
        if let Some(danger_accept_invalid_hostnames) = self.config.danger_accept_invalid_hostnames {
            builder = builder.danger_accept_invalid_hostnames(danger_accept_invalid_hostnames);
        }

        #[cfg(any(
            feature = "default-tls",
            feature = "native-tls",
            feature = "rustls-tls"
        ))]
        if let Some(danger_accept_invalid_certs) = self.config.danger_accept_invalid_certs {
            builder = builder.danger_accept_invalid_certs(danger_accept_invalid_certs);
        }

        #[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
        if let Some(identity) = self.config.identity {
            builder = builder.identity(identity);
        }

        #[cfg(feature = "native-tls")]
        if self.config.use_native_tls {
            builder = builder.use_native_tls();
        }

        #[cfg(feature = "rustls-tls")]
        if self.config.use_rustls_tls {
            builder = builder.use_rustls_tls();
        }

        #[cfg(feature = "trust-dns")]
        if let Some(enable) = self.config.trust_dns {
            builder = builder.trust_dns(enable);
        }

        if self.config.no_trust_dns {
            builder = builder.no_trust_dns();
        }
        if let Some(enable) = self.config.https_only {
            builder = builder.https_only(enable);
        }
        builder
    }
}
