//! 七牛客户端配置模块
use crate::{
    http::{DomainsManager, HTTPAfterAction, HTTPBeforeAction, HTTPCaller},
    storage::uploader::{UploadLogger, UploadLoggerBuilder, UploadRecorder},
};
use assert_impl::assert_impl;
use derive_builder::Builder;
use getset::{CopyGetters, Getters};
use std::{borrow::Cow, boxed::Box, default::Default, env::consts::ARCH, fmt, ops::Deref, sync::Arc, time::Duration};
use sys_info::{linux_os_release, os_release, os_type};

#[derive(Builder, Getters, CopyGetters)]
#[builder(
    name = "ConfigBuilder",
    pattern = "owned",
    public,
    build_fn(name = "inner_build", private)
)]
/// 七牛客户端配置内容
///
/// 注意，请勿直接使用该结构体，您可以通过调用 `ConfigBuilder` 的同名方法来构建 `Config` 的实例，
/// 然后调用 `Config` 的同名方法来访问到该结构体的数据
pub struct ConfigInner {
    /// 用户代理
    #[get = "pub"]
    #[builder(setter(skip, into))]
    user_agent: String,

    /// 追加的用户代理
    ///
    /// 七牛 Rust SDK 本身会包含预定的用户代理字符串，您不能修改该字符串，但可以向该字符串追加更多内容
    #[builder(default, setter(into))]
    appended_user_agent: Option<Cow<'static, str>>,

    /// 是否使用 HTTPS 协议
    ///
    /// 默认为使用 HTTPS 协议
    #[get_copy = "pub"]
    #[builder(default = "default::use_https()")]
    use_https: bool,

    /// UC 服务器地址（仅需要指定主机地址和端口，无需包含协议）
    ///
    ///默认将会使用七牛公有云的 UC 服务器地址， 仅在使用私有云时才需要配置
    #[get = "pub"]
    #[builder(default = "default::uc_host()", setter(into))]
    uc_host: Cow<'static, str>,

    /// RS 服务器地址（仅需要指定主机地址和端口，无需包含协议）
    ///
    /// 默认将会使用七牛公有云的 RS 服务器地址，仅在使用私有云时才需要配置
    #[get = "pub"]
    #[builder(default = "default::rs_host()", setter(into))]
    rs_host: Cow<'static, str>,

    /// RSF 服务器地址（仅需要指定主机地址和端口，无需包含协议）
    ///
    /// 默认将会使用七牛公有云的 RSF 服务器地址，仅在使用私有云时才需要配置
    #[get = "pub"]
    #[builder(default = "default::rsf_host()", setter(into))]
    rsf_host: Cow<'static, str>,

    /// API 服务器地址（仅需要指定主机地址和端口，无需包含协议）
    ///
    /// 默认将会使用七牛公有云的 API 服务器地址，仅在使用私有云时才需要配置
    #[get = "pub"]
    #[builder(default = "default::api_host()", setter(into))]
    api_host: Cow<'static, str>,

    /// UpLog 服务器地址（仅需要指定主机地址和端口，无需包含协议）
    ///
    /// 默认将会使用七牛公有云的 UpLog 服务器地址，仅在使用私有云时才需要配置
    #[get = "pub"]
    #[builder(default = "default::uplog_host()", setter(into))]
    uplog_host: Cow<'static, str>,

    /// 上传凭证有效期
    ///
    /// 默认为 1 小时
    #[get_copy = "pub"]
    #[builder(default = "default::upload_token_lifetime()")]
    upload_token_lifetime: Duration,

    /// 最大批量操作数
    ///
    /// 默认为 1000
    #[get_copy = "pub"]
    #[builder(default = "default::batch_max_operation_size()")]
    batch_max_operation_size: usize,

    /// 如果上传文件尺寸大于该值，将自动使用分片上传，否则，使用表单上传
    ///
    /// 单位为字节，默认为 4 MB
    #[get_copy = "pub"]
    #[builder(default = "default::upload_threshold()")]
    upload_threshold: u32,

    /// 上传分块尺寸，尺寸越小越适合弱网环境，必须是 4 MB 的倍数
    ///
    /// 单位为字节，默认为 4 MB
    #[get_copy = "pub"]
    #[builder(default = "default::upload_block_size()")]
    upload_block_size: u32,

    /// 上传日志记录仪
    ///
    /// 默认情况下，七牛 Rust SDK 会收集文件上传相关日志信息，并自动以异步的形式上传到 Uplog 服务器，并由七牛工作人员进行统计或定位问题
    ///
    /// 您可以手动创建上传日志记录仪以自定义日志尺寸，日志文件地址等属性，或者将其置为 `None` 以取消日志上传
    #[get = "pub"]
    #[builder(default = "default::upload_logger()")]
    upload_logger: Option<UploadLogger>,

    /// 上传进度记录仪
    ///
    /// 用于记录文件块上传进度，如果文件在上传期间发生错误，将可以在重试时避免再次上传已经成功上传的文件分块，实现断点续传
    #[get = "pub"]
    #[builder(default)]
    upload_recorder: UploadRecorder,

    /// HTTP 请求连接超时时长
    ///
    /// 对七牛 Rust SDK 所有发出的 HTTP 请求均有效
    ///
    /// 默认为 5 秒
    #[get_copy = "pub"]
    #[builder(default = "default::http_connect_timeout()")]
    http_connect_timeout: Duration,

    /// HTTP 请求超时时长
    ///
    /// 对七牛 Rust SDK 所有发出的 HTTP 请求均有效
    ///
    /// 默认为 5 分钟
    #[get_copy = "pub"]
    #[builder(default = "default::http_request_timeout()")]
    http_request_timeout: Duration,

    /// TCP KeepAlive 空闲时长
    ///
    /// 对七牛 Rust SDK 所有发出的 HTTP 请求均有效
    ///
    /// 默认为 5 分钟
    #[get_copy = "pub"]
    #[builder(default = "default::tcp_keepalive_idle_timeout()")]
    tcp_keepalive_idle_timeout: Duration,

    /// TCP KeepAlive 探测包的发送间隔
    ///
    /// 对七牛 Rust SDK 所有发出的 HTTP 请求均有效
    ///
    /// 默认为 5 秒
    #[get_copy = "pub"]
    #[builder(default = "default::tcp_keepalive_probe_interval()")]
    tcp_keepalive_probe_interval: Duration,

    /// HTTP 最低传输速度
    ///
    /// 与 `http_low_transfer_speed_timeout` 配合使用。
    /// 当 HTTP 传输速度低于最低传输速度 `http_low_transfer_speed` 并维持超过 `http_low_transfer_speed_timeout` 的时长，则出错。
    /// Rust SDK 会自动重试，或出错退出
    ///
    /// 对七牛 Rust SDK 所有发出的 HTTP 请求均有效
    ///
    /// 单位为 字节/秒
    ///
    /// 默认为 1024 字节/秒
    #[get_copy = "pub"]
    #[builder(default = "default::http_low_transfer_speed()")]
    http_low_transfer_speed: u32,

    /// HTTP 最低传输速度维持时长
    ///
    /// 与 `http_low_transfer_speed` 配合使用。
    /// 当 HTTP 传输速度低于最低传输速度 `http_low_transfer_speed` 并维持超过 `http_low_transfer_speed_timeout` 的时长，则出错。
    /// Rust SDK 会自动重试，或出错退出
    ///
    /// 对七牛 Rust SDK 所有发出的 HTTP 请求均有效
    ///
    /// 默认为 30 秒
    #[get_copy = "pub"]
    #[builder(default = "default::http_low_transfer_speed_timeout()")]
    http_low_transfer_speed_timeout: Duration,

    /// HTTP 请求重试次数
    ///
    /// 当 SDK 发送 HTTP 请求时发生错误，且该错误可以通过重试来解决时，SDK 将重试的次数。
    ///
    /// 默认为 3 次
    #[get_copy = "pub"]
    #[builder(default = "default::http_request_retries()")]
    http_request_retries: usize,

    /// HTTP 请求重试前等待时间
    ///
    /// 当 SDK 发送 HTTP 请求时发生错误，且该错误可以通过重试来解决时，SDK 将等待一段时间并且重试。
    ///
    /// 每次实际等待时长为该项值的 50% - 100% 之间的随机时长。
    ///
    /// 默认为 1 秒，也就是说每次等待 500 毫秒至 1 秒间不等
    #[get_copy = "pub"]
    #[builder(default = "default::http_request_retry_delay()")]
    http_request_retry_delay: Duration,

    /// HTTP 请求前回调函数
    ///
    /// 在每次发送 HTTP 请求前将逐一回调列表中所有函数
    ///
    /// 对七牛 Rust SDK 所有发出的 HTTP 请求均有效
    ///
    /// 您可以利用该特性输出 HTTP 日志或对 HTTP 请求内容进行修改。
    /// 但注意，您必须确保不破坏请求中必要的内容，否则七牛服务器可能无法处理该请求。
    #[get = "pub"]
    #[builder(default)]
    http_request_before_action_handlers: Vec<Box<dyn HTTPBeforeAction>>,

    /// HTTP 请求响应后回调函数
    ///
    /// 在每次收到 HTTP 响应后将逐一回调列表中所有函数
    ///
    /// 对七牛 Rust SDK 所有发出的 HTTP 请求均有效
    ///
    /// 您可以利用该特性输出 HTTP 日志或对 HTTP 响应内容进行修改。
    /// 但注意，您必须确保不破坏响应中必要的内容，否则七牛 Rust SDK 可能无法处理该响应。
    #[get = "pub"]
    #[builder(default)]
    http_request_after_action_handlers: Vec<Box<dyn HTTPAfterAction>>,

    /// HTTP 请求处理函数
    ///
    /// 七牛 Rust SDK 本身并不直接包含 HTTP 请求处理逻辑，您需要为 SDK 提供一个 HTTP 请求处理逻辑实现。
    ///
    /// 对于开启了 `use-libcurl` 功能的七牛 Rust SDK，Config 会默认使用 [qiniu-with-libcurl](https://crates.io/crates/qiniu-with-libcurl) 提供的 `HTTPCaller` 来处理 HTTP 请求。
    ///
    /// 对七牛 Rust SDK 所有发出的 HTTP 请求均有效
    #[get = "pub"]
    #[builder(
        setter(name = "boxed_http_request_handler"),
        private,
        default = "default::http_request_handler()"
    )]
    http_request_handler: Box<dyn HTTPCaller>,

    /// 域名管理器
    ///
    /// 对七牛 Rust SDK 所用的所有域名及域名所用的 IP 地址进行管理。功能包含域名 IP 地址的预解析和缓存，冻结域名，并会对这些状态进行持久化存储。
    ///
    /// 您可以手动创建域名管理器以自定义域名冻结时长，域名预解析，域名解析结果缓存时长，持久化等属性
    #[get = "pub"]
    #[builder(default)]
    domains_manager: DomainsManager,
}

mod default {
    use super::*;

    #[inline]
    pub const fn use_https() -> bool {
        true
    }

    #[inline]
    pub const fn uc_host() -> Cow<'static, str> {
        Cow::Borrowed("uc.qbox.me")
    }

    #[inline]
    pub const fn rs_host() -> Cow<'static, str> {
        Cow::Borrowed("rs.qbox.me")
    }

    #[inline]
    pub const fn rsf_host() -> Cow<'static, str> {
        Cow::Borrowed("rsf.qbox.me")
    }

    #[inline]
    pub const fn api_host() -> Cow<'static, str> {
        Cow::Borrowed("api.qiniu.com")
    }

    #[inline]
    pub const fn uplog_host() -> Cow<'static, str> {
        Cow::Borrowed("uplog.qbox.me")
    }

    #[inline]
    pub const fn upload_token_lifetime() -> Duration {
        Duration::from_secs(60 * 60)
    }

    #[inline]
    pub const fn batch_max_operation_size() -> usize {
        1000
    }

    #[inline]
    pub const fn upload_threshold() -> u32 {
        1 << 22
    }

    #[inline]
    pub const fn upload_block_size() -> u32 {
        1 << 22
    }

    #[inline]
    pub fn upload_logger() -> Option<UploadLogger> {
        UploadLoggerBuilder::default().build().map(Some).unwrap_or(None)
    }

    #[inline]
    pub fn http_connect_timeout() -> Duration {
        Duration::from_secs(5)
    }

    #[inline]
    pub fn http_request_timeout() -> Duration {
        Duration::from_secs(300)
    }

    #[inline]
    pub fn tcp_keepalive_idle_timeout() -> Duration {
        Duration::from_secs(300)
    }

    #[inline]
    pub fn tcp_keepalive_probe_interval() -> Duration {
        Duration::from_secs(5)
    }

    #[inline]
    pub fn http_low_transfer_speed() -> u32 {
        1024
    }

    #[inline]
    pub fn http_low_transfer_speed_timeout() -> Duration {
        Duration::from_secs(30)
    }

    #[inline]
    pub const fn http_request_retries() -> usize {
        3
    }

    #[inline]
    pub const fn http_request_retry_delay() -> Duration {
        Duration::from_secs(1)
    }

    #[inline]
    pub fn http_request_handler() -> Box<dyn HTTPCaller> {
        #[cfg(any(feature = "use-libcurl"))]
        {
            Box::new(qiniu_with_libcurl::CurlClient::default())
        }
        #[cfg(not(feature = "use-libcurl"))]
        {
            use crate::http::PanickedHTTPCaller;
            Box::new(PanickedHTTPCaller("Must define config.http_request_call"))
        }
    }
}

impl fmt::Debug for ConfigInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Config")
            .field("use_https", &self.use_https)
            .field("uc_host", &self.uc_host)
            .field("rs_host", &self.rs_host)
            .field("rsf_host", &self.rsf_host)
            .field("api_host", &self.api_host)
            .field("uplog_host", &self.uplog_host)
            .field("upload_token_lifetime", &self.upload_token_lifetime)
            .field("batch_max_operation_size", &self.batch_max_operation_size)
            .field("upload_threshold", &self.upload_threshold)
            .field("upload_block_size", &self.upload_block_size)
            .field("upload_recorder", &self.upload_recorder)
            .field("upload_logger", &self.upload_logger)
            .field("http_request_retries", &self.http_request_retries)
            .field("http_request_retry_delay", &self.http_request_retry_delay)
            .field("domains_manager", &self.domains_manager)
            .finish()
    }
}

impl ConfigInner {
    /// 追加的用户代理
    ///
    /// 七牛 Rust SDK 本身会包含预定的用户代理字符串，您不能修改该字符串，但可以向该字符串追加更多内容
    pub fn appended_user_agent(&self) -> Option<&str> {
        self.appended_user_agent.as_ref().map(|ua| ua.as_ref())
    }

    /// UC 服务器 URL
    pub fn uc_url(&self) -> String {
        if self.use_https {
            "https://".to_owned() + self.uc_host.as_ref()
        } else {
            "http://".to_owned() + self.uc_host.as_ref()
        }
    }

    /// RS 服务器 URL
    pub fn rs_url(&self) -> String {
        if self.use_https {
            "https://".to_owned() + self.rs_host.as_ref()
        } else {
            "http://".to_owned() + self.rs_host.as_ref()
        }
    }

    /// RSF 服务器 URL
    pub fn rsf_url(&self) -> String {
        if self.use_https {
            "https://".to_owned() + self.rsf_host.as_ref()
        } else {
            "http://".to_owned() + self.rsf_host.as_ref()
        }
    }

    /// API 服务器 URL
    pub fn api_url(&self) -> String {
        if self.use_https {
            "https://".to_owned() + self.api_host.as_ref()
        } else {
            "http://".to_owned() + self.api_host.as_ref()
        }
    }

    /// UpLog 服务器 URL
    pub fn uplog_url(&self) -> String {
        if self.use_https {
            "https://".to_owned() + self.uplog_host.as_ref()
        } else {
            "http://".to_owned() + self.uplog_host.as_ref()
        }
    }
}

/// 七牛客户端配置
///
/// 需要注意的是，所有七牛客户端配置均为只读。
#[derive(Clone, Debug)]
pub struct Config(Arc<ConfigInner>);

impl ConfigBuilder {
    /// 设置 HTTP 请求处理函数
    ///
    /// 七牛 Rust SDK 本身并不直接包含 HTTP 请求处理逻辑，您需要为 SDK 提供一个 HTTP 请求处理逻辑实现。
    ///
    /// 对于开启了 `use-libcurl` 功能的七牛 Rust SDK，Config 会默认使用 [qiniu-with-libcurl](https://crates.io/crates/qiniu-with-libcurl) 提供的 `HTTPCaller` 来处理 HTTP 请求。
    ///
    /// 对七牛 Rust SDK 所有发出的 HTTP 请求均有效
    pub fn http_request_handler(self, handler: impl HTTPCaller + 'static) -> Self {
        self.boxed_http_request_handler(Box::new(handler))
    }

    /// 追加 HTTP 请求前回调函数
    ///
    /// 您可以利用该特性输出 HTTP 日志或对 HTTP 请求内容进行修改。
    /// 但注意，您必须确保不破坏请求中必要的内容，否则七牛服务器可能无法处理该请求。
    pub fn append_http_request_before_action_handler(mut self, handler: impl HTTPBeforeAction + 'static) -> Self {
        if let Some(before_action_handlers) = &mut self.http_request_before_action_handlers {
            before_action_handlers.push(Box::new(handler));
        } else {
            let mut handlers = Vec::<Box<dyn HTTPBeforeAction>>::with_capacity(1);
            handlers.push(Box::new(handler));
            self.http_request_before_action_handlers = Some(handlers);
        }
        self
    }

    /// 新增 HTTP 请求前回调函数
    ///
    /// 您可以利用该特性输出 HTTP 日志或对 HTTP 请求内容进行修改。
    /// 但注意，您必须确保不破坏请求中必要的内容，否则七牛服务器可能无法处理该请求。
    pub fn prepend_http_request_before_action_handler(mut self, handler: impl HTTPBeforeAction + 'static) -> Self {
        if let Some(before_action_handlers) = &mut self.http_request_before_action_handlers {
            before_action_handlers.insert(0, Box::new(handler));
        } else {
            let mut handlers = Vec::<Box<dyn HTTPBeforeAction>>::with_capacity(1);
            handlers.push(Box::new(handler));
            self.http_request_before_action_handlers = Some(handlers);
        }
        self
    }

    /// 追加 HTTP 请求响应后回调函数
    ///
    /// 您可以利用该特性输出 HTTP 日志或对 HTTP 响应内容进行修改。
    /// 但注意，您必须确保不破坏响应中必要的内容，否则七牛 Rust SDK 可能无法处理该响应。
    pub fn append_http_request_after_action_handler(mut self, handler: impl HTTPAfterAction + 'static) -> Self {
        if let Some(after_action_handlers) = &mut self.http_request_after_action_handlers {
            after_action_handlers.push(Box::new(handler));
        } else {
            let mut handlers = Vec::<Box<dyn HTTPAfterAction>>::with_capacity(1);
            handlers.push(Box::new(handler));
            self.http_request_after_action_handlers = Some(handlers);
        }
        self
    }

    /// 新增 HTTP 请求响应后回调函数
    ///
    /// 您可以利用该特性输出 HTTP 日志或对 HTTP 响应内容进行修改。
    /// 但注意，您必须确保不破坏响应中必要的内容，否则七牛 Rust SDK 可能无法处理该响应。
    pub fn prepend_http_request_after_action_handler(mut self, handler: impl HTTPAfterAction + 'static) -> Self {
        if let Some(after_action_handlers) = &mut self.http_request_after_action_handlers {
            after_action_handlers.insert(0, Box::new(handler));
        } else {
            let mut handlers = Vec::<Box<dyn HTTPAfterAction>>::with_capacity(1);
            handlers.push(Box::new(handler));
            self.http_request_after_action_handlers = Some(handlers);
        }
        self
    }

    /// 生成客户端配置
    pub fn build(self) -> Config {
        let mut config = self.inner_build().unwrap();
        config.user_agent = format!(
            "QiniuRust/qiniu-ng-{}/{};{};{};{}/rust-{}{}",
            env!("CARGO_PKG_VERSION"),
            os_type().ok().unwrap_or_else(String::new),
            os_release().ok().unwrap_or_else(String::new),
            linux_os_release()
                .ok()
                .and_then(|info| info.pretty_name)
                .unwrap_or_else(String::new),
            ARCH,
            rustc_version_runtime::version(),
            config
                .appended_user_agent()
                .map_or(Cow::Borrowed("/"), |ua| Cow::Owned("/".to_owned() + ua + "/"))
        );
        Config(Arc::new(config))
    }
}

impl Default for Config {
    fn default() -> Self {
        ConfigBuilder::default().build()
    }
}

impl Deref for Config {
    type Target = ConfigInner;

    #[inline]
    fn deref(&self) -> &ConfigInner {
        self.0.deref()
    }
}

impl Config {
    #[doc(hidden)]
    pub fn into_raw(self) -> *const ConfigInner {
        Arc::into_raw(self.0)
    }

    #[doc(hidden)]
    pub unsafe fn from_raw(ptr: *const ConfigInner) -> Config {
        Config(Arc::from_raw(ptr))
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qiniu_http::{HTTPCaller, Request, RequestBuilder, Response, ResponseBody, ResponseBuilder, Result};
    use regex::Regex;
    use std::{error::Error, result::Result as StdResult};

    struct FakeHTTPRequester;
    impl HTTPCaller for FakeHTTPRequester {
        fn call(&self, _: &Request) -> Result<Response> {
            Ok(ResponseBuilder::default()
                .status_code(612u16)
                .bytes_as_body(b"It's HTTP Body".as_ref())
                .build())
        }
    }

    #[test]
    fn test_config_with_set_user_agent() -> StdResult<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .appended_user_agent(Some("fake_for_test".into()))
            .build();
        assert!(Regex::new("QiniuRust/qiniu-ng-[^/]+/[^/]+/rust-[^/]+/fake_for_test/")
            .unwrap()
            .is_match(config.user_agent()));
        Ok(())
    }

    #[test]
    fn test_config_with_set_dynamic_http_request_call() -> StdResult<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .http_request_retries(5)
            .http_request_retry_delay(Duration::from_secs(1))
            .http_request_handler(FakeHTTPRequester)
            .build();

        let http_response = config
            .http_request_handler()
            .call(&RequestBuilder::default().url("http://fake.qiniu.com").build())?;

        assert_eq!(http_response.status_code(), 612);
        match http_response.into_body().unwrap() {
            ResponseBody::Bytes(http_body) => {
                assert_eq!(http_body, b"It's HTTP Body");
            }
            _ => {
                panic!("Unexpected response type");
            }
        }
        Ok(())
    }

    #[test]
    fn test_config_with_getters() -> StdResult<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .http_request_retries(5)
            .http_request_retry_delay(Duration::from_secs(1))
            .http_request_handler(FakeHTTPRequester)
            .build();
        assert_eq!(config.http_request_retries(), 5);
        assert_eq!(config.http_request_retry_delay(), Duration::from_secs(1));
        Ok(())
    }
}
