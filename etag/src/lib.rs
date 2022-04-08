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
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications
)]

//! # qiniu-etag
//!
//! ## 七牛 Etag 计算器
//!
//! 负责根据输入的数据计算七牛 Etag，适配 V1 和 V2 版本，同时提供阻塞接口和异步接口（异步接口需要启用 `async` 功能）
//!
//! 七牛 Etag 文档：<https://developer.qiniu.com/kodo/1231/appendix>
//!
//! ### 代码示例
//!
//! #### Etag V1 计算示例
//! ```
//! use qiniu_etag::{EtagV1, prelude::*};
//!
//! let mut etag_v1 = EtagV1::new();
//! etag_v1.update(b"etag");
//! assert_eq!(etag_v1.finalize_fixed().as_slice(), b"FpLiADEaVoALPkdb8tJEJyRTXoe_");
//! ```
//!
//! #### Etag V1 计算输入流示例
//!
//! ##### 阻塞代码示例
//!
//! ```
//! use std::io::{copy, Cursor};
//! use qiniu_etag::{EtagV1, prelude::*};
//!
//! # fn main() -> std::io::Result<()> {
//! let mut etag_v1 = EtagV1::new();
//! copy(&mut Cursor::new(b"etag"), &mut etag_v1)?;
//! assert_eq!(etag_v1.finalize_fixed().as_slice(), b"FpLiADEaVoALPkdb8tJEJyRTXoe_");
//! # Ok(())
//! # }
//! ```
//!
//! ##### 异步代码示例
//!
//! ```
//! use futures_lite::io::{copy, Cursor};
//! use qiniu_etag::{EtagV1, prelude::*};
//!
//! # async fn example() -> std::io::Result<()> {
//! let mut etag_v1 = EtagV1::new();
//! copy(&mut Cursor::new(b"etag"), &mut etag_v1).await?;
//! assert_eq!(etag_v1.finalize_fixed().as_slice(), b"FpLiADEaVoALPkdb8tJEJyRTXoe_");
//! # Ok(())
//! # }
//! ```
//!
//! #### Etag V2 计算示例
//!
//! ```
//! use qiniu_etag::{EtagV2, prelude::*};
//!
//! let mut etag_v2 = EtagV2::new();
//! etag_v2.update(b"hello");
//! etag_v2.update(b"world");
//! assert_eq!(etag_v2.finalize_fixed().as_slice(), b"ns56DcSIfBFUENXjdhsJTIvl3Rcu");
//! ```
//!
//! ### Etag V2 计算输入流示例
//!
//! 与 V1 不同的是，Etag V2 要求传入数据的分块方式
//!
//! ##### 阻塞代码示例
//!
//! ```
//! use qiniu_etag::etag_with_parts;
//! use std::io::Cursor;
//!
//! # fn main() -> std::io::Result<()> {
//! assert_eq!(
//!     etag_with_parts(
//!         &mut Cursor::new(data_of_size(9 << 20)),
//!         &[1 << 22, 1 << 22, 1 << 20]
//!     )?,
//!     "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
//! );
//! # Ok(())
//! # }
//! # const FAKE_DATA: [u8; 4096] = make_fake_data();
//! # fn data_of_size(size: usize) -> Vec<u8> {
//! #     let mut buffer = Vec::with_capacity(size);
//! #     let mut rest = size;
//! #     while rest > 0 {
//! #         let add_size = rest.min(FAKE_DATA.len());
//! #         buffer.extend_from_slice(&FAKE_DATA[..add_size]);
//! #         rest -= add_size;
//! #     }
//! #     buffer
//! # }
//! # const fn make_fake_data() -> [u8; 4096] {
//! #     let mut buf = [b'b'; 4096];
//! #     buf[0] = b'A';
//! #     buf[4094] = b'\r';
//! #     buf[4095] = b'\n';
//! #     buf
//! # }
//! ```
//!
//! ##### 异步代码示例
//!
//! ```
//! use qiniu_etag::async_etag_with_parts;
//! use futures_lite::io::Cursor;
//!
//! # async fn example() -> std::io::Result<()> {
//! assert_eq!(
//!     async_etag_with_parts(
//!         &mut Cursor::new(data_of_size(9 << 20)),
//!         &[1 << 22, 1 << 22, 1 << 20]
//!     ).await?,
//!     "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
//! );
//! # Ok(())
//! # }
//! # const FAKE_DATA: [u8; 4096] = make_fake_data();
//! # fn data_of_size(size: usize) -> Vec<u8> {
//! #     let mut buffer = Vec::with_capacity(size);
//! #     let mut rest = size;
//! #     while rest > 0 {
//! #         let add_size = rest.min(FAKE_DATA.len());
//! #         buffer.extend_from_slice(&FAKE_DATA[..add_size]);
//! #         rest -= add_size;
//! #     }
//! #     buffer
//! # }
//! # const fn make_fake_data() -> [u8; 4096] {
//! #     let mut buf = [b'b'; 4096];
//! #     buf[0] = b'A';
//! #     buf[4094] = b'\r';
//! #     buf[4095] = b'\n';
//! #     buf
//! # }
//! ```

pub use digest::{
    generic_array::{typenum::U28, GenericArray},
    FixedOutput, Reset, Update,
};

mod etag;
mod etag_v1;
mod etag_v2;
mod sha1;

pub use etag::{etag_of, etag_to_buf, etag_with_parts, etag_with_parts_to_buf, Etag, EtagVersion, ETAG_SIZE};
pub use etag_v1::EtagV1;
pub use etag_v2::EtagV2;

#[cfg(feature = "async")]
mod async_etag;

#[cfg(feature = "async")]
pub use async_etag::{
    etag_of as async_etag_of, etag_to_buf as async_etag_to_buf, etag_with_parts as async_etag_with_parts,
    etag_with_parts_to_buf as async_etag_with_parts_to_buf,
};

/// 将所有 Trait 全部重新导出，方便统一导入
pub mod prelude {
    pub use super::{FixedOutput, Reset, Update};
}
