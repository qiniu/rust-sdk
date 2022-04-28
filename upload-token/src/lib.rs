#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![deny(
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
    unreachable_pub,
    unsafe_code,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications
)]

//! # qiniu-upload-token
//!
//! ## 七牛 上传策略 / 上传凭证 库
//!
//! 负责配置七牛对象上传所需要的上传策略，并提供生成上传凭证的库函数，同时提供 [`UploadTokenProvider`] 方便扩展获取上传凭证的方式。
//! 同时提供阻塞接口和异步接口（异步接口需要启用 `async` 功能）。
//! 提供 [`UploadTokenProvider`] 的多个实现方式，例如：
//!
//! - [`StaticUploadTokenProvider`] : 根据其他服务计算得到的上传凭证字符串生成上传凭证
//! - [`FromUploadPolicy`] : 根据给出的上传策略和认证信息生成上传凭证
//! - [`BucketUploadTokenProvider`] : 基于存储空间和认证信息即时生成上传凭证
//! - [`ObjectUploadTokenProvider`] : 基于存储空间，对象名称和认证信息即时生成上传凭证
//! - [`CachedUploadTokenProvider`] : 缓存生成的上传凭证，不必每次都即时生成
//!
//! ### 代码示例
//!
//! #### 创建上传策略，并基于该策略创建凭证
//!
//! ```
//! use qiniu_upload_token::{FileType, UploadPolicy, credential::Credential, prelude::*};
//! use std::time::Duration;
//!
//! # fn main() -> anyhow::Result<()> {
//! let upload_policy = UploadPolicy::new_for_object("your-bucket", "your-key", Duration::from_secs(3600))
//!     .file_type(FileType::InfrequentAccess)
//!     .build();
//! let credential = Credential::new("your-access-key", "your-secret-key");
//! let upload_token = upload_policy.into_static_upload_token_provider(credential, Default::default());
//! println!("{}", upload_token);
//! # Ok(())
//! # }
//! ```
//!
//! #### 从其他应用程序生成的上传凭证解析出上传策略
//!
//! ```
//! use qiniu_upload_token::{StaticUploadTokenProvider, prelude::*};
//!
//! # fn main() -> anyhow::Result<()> {
//! let upload_token: StaticUploadTokenProvider = "your-access-key:qRD-BSf_XGtovGsuOePTc1EKJo8=:eyJkZWFkbGluZSI6MTY0NzgyODY3NCwic2NvcGUiOiJ5b3VyLWJ1Y2tldC1uYW1lIn0=".parse()?;
//! let access_key = upload_token.access_key(Default::default())?;
//! let bucket_name = upload_token.bucket_name(Default::default())?;
//! let upload_policy = upload_token.policy(Default::default())?;
//! # Ok(())
//! # }
//! ```
mod file_type;
mod upload_policy;
mod upload_token;
pub use file_type::FileType;
pub use qiniu_credential::{self as credential, Extensions};
pub use qiniu_utils::{BucketName, ObjectName};
pub use serde_json;
pub use upload_policy::{UploadPolicy, UploadPolicyBuilder};
pub use upload_token::{
    BucketUploadTokenProvider, BucketUploadTokenProviderBuilder, CachedUploadTokenProvider, FromUploadPolicy,
    GetAccessKeyOptions, GetPolicyOptions, GotAccessKey, GotUploadPolicy, ObjectUploadTokenProvider,
    ObjectUploadTokenProviderBuilder, ParseError, ParseResult, StaticUploadTokenProvider, ToStringError,
    ToStringOptions, ToStringResult, UploadTokenProvider, UploadTokenProviderExt,
};

/// 将所有 Trait 全部重新导出，方便统一导入
pub mod prelude {
    pub use super::{UploadTokenProvider, UploadTokenProviderExt};
}
