use isahc::{
    auth::{Authentication, Credentials},
    config::{
        CaCertificate, ClientCertificate, Dialer, IpVersion, NetworkInterface, RedirectPolicy,
        SslOption, VersionNegotiation,
    },
};
use qiniu_http::Uri;
use std::time::Duration;

macro_rules! make_request_extension {
    ($extension_name:ident) => {
        #[derive(Copy, Clone, Debug)]
        pub struct $extension_name;
    };
    ($extension_name:ident, $type:ty) => {
        #[derive(Clone, Debug)]
        pub struct $extension_name($type);
        impl $extension_name {
            #[inline]
            pub fn new(value: $type) -> Self {
                Self(value)
            }

            #[inline]
            pub fn get(&self) -> &$type {
                &self.0
            }
        }
    };
    ($extension_name:ident, $type1:ty, $type2:ty) => {
        #[derive(Clone, Debug)]
        pub struct $extension_name(($type1, $type2));
        impl $extension_name {
            #[inline]
            pub fn new(value1: $type1, value2: $type2) -> Self {
                Self((value1, value2))
            }

            #[inline]
            pub fn get(&self) -> (&$type1, &$type2) {
                (&(self.0).0, &(self.0).1)
            }
        }
    };
}

make_request_extension!(TimeoutRequestExtension, Duration);
make_request_extension!(ConnectTimeoutRequestExtension, Duration);
make_request_extension!(LowSpeedTimeoutRequestExtension, u32, Duration);
make_request_extension!(VersionNegotiationRequestExtension, VersionNegotiation);
make_request_extension!(RedirectPolicyRequestExtension, RedirectPolicy);
make_request_extension!(AutoRefererRequestExtension);
make_request_extension!(AutomaticDecompressionRequestExtension, bool);
make_request_extension!(TcpKeepaliveRequestExtension, Duration);
make_request_extension!(TcpNodelayRequestExtension);
make_request_extension!(NetworkInterfaceRequestExtension, NetworkInterface);
make_request_extension!(IpVersionRequestExtension, IpVersion);
make_request_extension!(DialRequestExtension, Dialer);
make_request_extension!(ProxyRequestExtension, Option<Uri>);
make_request_extension!(ProxyBlacklistRequestExtension, Vec<String>);
make_request_extension!(ProxyAuthenticationRequestExtension, Authentication);
make_request_extension!(ProxyCredentialsRequestExtension, Credentials);
make_request_extension!(MaxUploadSpeedRequestExtension, u64);
make_request_extension!(MaxDownloadSpeedRequestExtension, u64);
make_request_extension!(SslClientCertificateRequestExtension, ClientCertificate);
make_request_extension!(SslCaCertificateRequestExtension, CaCertificate);
make_request_extension!(SslCiphersRequestExtension, Vec<String>);
make_request_extension!(SslOptionsRequestExtension, SslOption);
make_request_extension!(TitleCaseHeadersRequestExtension, bool);
