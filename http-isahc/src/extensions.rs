use isahc::{
    auth::{Authentication, Credentials},
    config::{
        CaCertificate, ClientCertificate, Dialer, IpVersion, NetworkInterface, RedirectPolicy, SslOption,
        VersionNegotiation,
    },
};
use qiniu_http::Uri;
use std::time::Duration;

macro_rules! make_request_extension {
    ($extension_name:ident, $doc:tt) => {
        #[derive(Copy, Clone, Debug)]
        #[doc = $doc]
        pub struct $extension_name;
    };
    ($extension_name:ident, $type:ty, $doc:tt) => {
        #[derive(Clone, Debug)]
        #[doc = $doc]
        pub struct $extension_name($type);
        impl $extension_name {
            /// 创建扩展类型
            #[inline]
            pub fn new(value: $type) -> Self {
                Self(value)
            }

            /// 获取扩展类型的值
            #[inline]
            pub fn get(&self) -> &$type {
                &self.0
            }
        }
    };
    ($extension_name:ident, $type1:ty, $type2:ty, $doc:tt) => {
        #[derive(Clone, Debug)]
        #[doc = $doc]
        pub struct $extension_name(($type1, $type2));
        impl $extension_name {
            /// 创建扩展类型
            #[inline]
            pub fn new(value1: $type1, value2: $type2) -> Self {
                Self((value1, value2))
            }

            /// 获取扩展类型的值
            #[inline]
            pub fn get(&self) -> (&$type1, &$type2) {
                (&(self.0).0, &(self.0).1)
            }
        }
    };
}

make_request_extension!(TimeoutRequestExtension, Duration, "请求超时时长扩展");
make_request_extension!(ConnectTimeoutRequestExtension, Duration, "连接请求超时时长扩展");
make_request_extension!(LowSpeedTimeoutRequestExtension, u32, Duration, "低速超时扩展");
make_request_extension!(VersionNegotiationRequestExtension, VersionNegotiation, "版本协商扩展");
make_request_extension!(RedirectPolicyRequestExtension, RedirectPolicy, "请求重定向策略扩展");
make_request_extension!(AutoRefererRequestExtension, "自动更新 Referer 头扩展");
make_request_extension!(AutomaticDecompressionRequestExtension, bool, "自动解压缩响应体扩展");
make_request_extension!(TcpKeepaliveRequestExtension, Duration, "TCP 长连接扩展");
make_request_extension!(TcpNodelayRequestExtension, "TCP Nagle 算法设置扩展");
make_request_extension!(NetworkInterfaceRequestExtension, NetworkInterface, "网络接口设置扩展");
make_request_extension!(IpVersionRequestExtension, IpVersion, "IP 地址版本扩展");
make_request_extension!(DialRequestExtension, Dialer, "连接套接字扩展");
make_request_extension!(ProxyRequestExtension, Option<Uri>, "请求代理扩展");
make_request_extension!(ProxyBlacklistRequestExtension, Vec<String>, "请求代理黑名单扩展");
make_request_extension!(ProxyAuthenticationRequestExtension, Authentication, "请求代理认证扩展");
make_request_extension!(ProxyCredentialsRequestExtension, Credentials, "请求代理认证信息扩展");
make_request_extension!(MaxUploadSpeedRequestExtension, u64, "上传限速扩展");
make_request_extension!(MaxDownloadSpeedRequestExtension, u64, "下载限速扩展");
make_request_extension!(
    SslClientCertificateRequestExtension,
    ClientCertificate,
    "SSL 客户端认证扩展"
);
make_request_extension!(SslCaCertificateRequestExtension, CaCertificate, "SSL CA 认证扩展");
make_request_extension!(SslCiphersRequestExtension, Vec<String>, "SSL 加密算法扩展");
make_request_extension!(SslOptionsRequestExtension, SslOption, "SSL 选项扩展");
make_request_extension!(TitleCaseHeadersRequestExtension, bool, "HTTP 头大小写敏感扩展");
