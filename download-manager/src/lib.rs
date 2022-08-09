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

//! # qiniu-download-manager
//!
//! ## 七牛下载管理
//!
//! 基于 `qiniu-apis` 提供针对七牛对象的下载功能
//! （同时提供阻塞客户端和异步客户端，异步客户端则需要启用 `async` 功能）。
//!
//! ### 功能描述
//!
//! #### `async`
//!
//! 启用异步接口。
//!
//! #### `ureq`
//!
//! 导入 `qiniu-ureq` 作为 HTTP 客户端。
//!
//! #### `isahc`
//!
//! 导入 `qiniu-isahc` 作为 HTTP 客户端。
//!
//! #### `reqwest`
//!
//! 导入 `qiniu-reqwest` 作为 HTTP 客户端。
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
//! ### 代码示例
//!
//! #### 下载私有空间的对象到指定路径
//!
//! ##### 阻塞代码示例
//!
//! ```
//! use qiniu_download_manager::{apis::credential::Credential, DownloadManager, StaticDomainsUrlsGenerator, UrlsSigner};
//!
//! # fn example() -> anyhow::Result<()> {
//! let bound_domain = "my-domain.com"; // 直接设置绑定的空间域名
//! let object_name = "test-object";
//! let download_manager = DownloadManager::new(UrlsSigner::new(
//!     Credential::new("abcdefghklmnopq", "1234567890"),
//!     StaticDomainsUrlsGenerator::builder(bound_domain)
//!         .use_https(false)
//!         .build(), // 设置为 HTTP 协议
//! ));
//! download_manager
//!     .download(object_name)?
//!     .to_path("/home/qiniu/test.png")?;
//! Ok(())
//! # }
//! ```
//!
//! ##### 异步代码示例
//!
//! ```
//! use qiniu_download_manager::{
//!     apis::{credential::Credential, http_client::BucketDomainsQueryer},
//!     DownloadManager, EndpointsUrlGenerator, UrlsSigner,
//! };
//!
//! # async fn example() -> anyhow::Result<()> {
//! let bucket_name = "test-bucket"; // 查询空间绑定的域名
//! let object_name = "test-object";
//! let credential = Credential::new("abcdefghklmnopq", "1234567890");
//! let download_manager = DownloadManager::new(UrlsSigner::new(
//!     credential.to_owned(),
//!     EndpointsUrlGenerator::builder(BucketDomainsQueryer::new().query(credential, bucket_name))
//!         .use_https(false)
//!         .build(), // 设置为 HTTP 协议
//! ));
//! download_manager
//!     .async_download(object_name)
//!     .await?
//!     .async_to_path("/home/qiniu/test.png")
//!     .await?;
//! # Ok(())
//! # }
//! ```

pub use qiniu_apis as apis;

mod urls_generators;
pub use urls_generators::{
    DownloadUrlsGenerator, EndpointsUrlGenerator, EndpointsUrlGeneratorBuilder, GeneratorOptions,
    StaticDomainsUrlsGenerator, StaticDomainsUrlsGeneratorBuilder, UrlsSigner,
};

mod download_manager;
pub use download_manager::{DownloadManager, DownloadManagerBuilder};

mod downloading_object;
pub use downloading_object::{
    DownloadError, DownloadResult, DownloadingObject, DownloadingObjectReader, DownloadingProgressInfo,
};

mod download_callbacks;
mod download_retrier;
pub use download_retrier::{
    DownloadRetrier, DownloadRetrierOptions, ErrorRetrier, NeverRetrier, RetriedStatsInfo, RetryDecision, RetryResult,
};

#[cfg(feature = "async")]
pub use downloading_object::AsyncDownloadingObjectReader;

/// 将所有 Trait 全部重新导出，方便统一导入
pub mod prelude {
    pub use super::{apis::http_client::prelude::*, DownloadRetrier, DownloadUrlsGenerator};
}
