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
    unsafe_code,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]

//! # qiniu-sdk
//!
//! ## 七牛 SDK

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

#[cfg(feature = "objects-manager")]
pub use qiniu_objects_manager as objects;

#[cfg(feature = "upload-manager")]
pub use qiniu_upload_manager as upload;

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

    #[cfg(feature = "objects-manager")]
    pub use qiniu_objects_manager::prelude::*;

    #[cfg(feature = "upload-manager")]
    pub use qiniu_upload_manager::prelude::*;
}
