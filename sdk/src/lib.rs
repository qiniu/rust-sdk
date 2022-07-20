#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![deny(
    single_use_lifetimes,
    missing_debug_implementations,
    large_assignments,
    exported_private_dependencies,
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    explicit_outlives_requirements,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_docs,
    non_ascii_idents,
    indirect_structural_match,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]

//! # qiniu-sdk
//!
//! ## 七牛 SDK
//!
//! 作为七牛所有 Rust SDK 插件的入口，可以通过启动功能来导入其他七牛 SDK。
//!
//! ### 功能描述
//!
//! #### `utils`
//!
//! 允许通过 `qiniu_sdk::utils` 来访问 `qiniu-utils`。
//!
//! #### `etag`
//!
//! 允许通过 `qiniu_sdk::etag` 来访问 `qiniu-etag`。
//!
//! #### `credential`
//!
//! 允许通过 `qiniu_sdk::credential` 来访问 `qiniu-credential`。
//!
//! #### `upload-token`
//!
//! 允许通过 `qiniu_sdk::upload_token` 来访问 `qiniu-upload-token`。
//!
//! #### `http`
//!
//! 允许通过 `qiniu_sdk::http` 来访问 `qiniu-http`。
//!
//! #### `http-client`
//!
//! 允许通过 `qiniu_sdk::http_client` 来访问 `qiniu-http-client`。
//!
//! #### `apis`
//!
//! 允许通过 `qiniu_sdk::apis` 来访问 `qiniu-apis`。
//!
//! #### `objects`
//!
//! 允许通过 `qiniu_sdk::objects` 来访问 `qiniu-objects-manager`。
//!
//! #### `upload`
//!
//! 允许通过 `qiniu_sdk::upload` 来访问 `qiniu-upload-manager`。
//!
//! #### `download`
//!
//! 允许通过 `qiniu_sdk::download` 来访问 `qiniu-download-manager`。
//!
//! #### `async`
//!
//! 启用所有七牛 SDK 插件的异步接口。
//!
//! #### `ureq`
//!
//! 导入 `qiniu-ureq` 作为 HTTP 客户端，并允许通过 `qiniu_sdk::ureq` 来访问 `qiniu-ureq`。
//!
//! #### `isahc`
//!
//! 导入 `qiniu-isahc` 作为 HTTP 客户端，并允许通过 `qiniu_sdk::isahc` 来访问 `qiniu-isahc`。
//!
//! #### `reqwest`
//!
//! 导入 `qiniu-reqwest` 作为 HTTP 客户端，并允许通过 `qiniu_sdk::reqwest` 来访问 `qiniu-reqwest`。
//!
//! #### `c_ares`
//!
//! 启用 `c-ares` 库作为 DNS 解析器。
//!
//! #### `trust_dns`
//!
//! 启用 `trust-dns` 库作为 DNS 解析器。
//!
//! #### `dns-over-https`
//!
//! 启用 `trust-dns` 库作为 DNS 解析器，并使用 DOH 协议。
//!
//! #### `dns-over-tls`
//!
//! 启用 `trust-dns` 库作为 DNS 解析器，并使用 DOT 协议。

#[cfg(feature = "utils")]
pub use qiniu_utils as utils;

#[cfg(feature = "etag")]
pub use qiniu_etag as etag;

#[cfg(feature = "credential")]
pub use qiniu_credential as credential;

#[cfg(feature = "upload-token")]
pub use qiniu_upload_token as upload_token;

#[cfg(feature = "http")]
pub use qiniu_http as http;

#[cfg(feature = "http-client")]
pub use qiniu_http_client as http_client;

#[cfg(feature = "apis")]
pub use qiniu_apis as apis;

#[cfg(feature = "objects")]
pub use qiniu_objects_manager as objects;

#[cfg(feature = "upload")]
pub use qiniu_upload_manager as upload;

#[cfg(feature = "download")]
pub use qiniu_download_manager as download;

#[cfg(feature = "ureq")]
pub use qiniu_ureq as ureq;

#[cfg(feature = "reqwest")]
pub use qiniu_reqwest as reqwest;

#[cfg(feature = "isahc")]
pub use qiniu_isahc as isahc;

/// 将所有 Trait 全部重新导出，方便统一导入
pub mod prelude {
    #[cfg(feature = "credential")]
    pub use qiniu_credential::prelude::*;

    #[cfg(feature = "upload-token")]
    pub use qiniu_upload_token::prelude::*;

    #[cfg(feature = "http")]
    pub use qiniu_http::prelude::*;

    #[cfg(feature = "http-client")]
    pub use qiniu_http_client::prelude::*;

    #[cfg(feature = "objects")]
    pub use qiniu_objects_manager::prelude::*;

    #[cfg(feature = "upload")]
    pub use qiniu_upload_manager::prelude::*;

    #[cfg(feature = "download")]
    pub use qiniu_download_manager::prelude::*;
}
