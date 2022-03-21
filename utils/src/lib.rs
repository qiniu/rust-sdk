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
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unstable_features,
    unsafe_code,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications
)]

//! # qiniu-etag
//!
//! ## 七牛实用工具库
//!
//! 仅供七牛 SDK 内部使用，接口不保证总是兼容变更

pub mod base64;
pub mod smallstr;

mod name;
pub use name::{BucketName, ObjectName};
