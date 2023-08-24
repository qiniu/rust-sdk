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
//! ## 七牛实用工具库
//!
//! 仅供七牛 SDK 内部使用，接口不保证总是兼容变更

pub mod base64;
pub mod smallstr;

mod name;
pub use name::{BucketName, ObjectName};

cfg_if::cfg_if! {
    if #[cfg(feature = "tokio_runtime")] {
        use tokio as _;
        use tokio_stream as _;
        use async_compat as _;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "async_std_runtime")] {
        use async_std as _;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "tokio_runtime")] {
        mod tokio_runtime;
    } else if #[cfg(feature = "async_std_runtime")] {
        mod async_std_runtime;
    }
}

/// Filesystem manipulation operations.
///
/// This module is an async version of [`std::fs`].
///
/// [`std::fs`]: https://doc.rust-lang.org/std/fs/index.html
#[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub mod async_fs {
    cfg_if::cfg_if! {
        if #[cfg(feature = "tokio_runtime")] {
            pub use super::tokio_runtime::{OpenOptions, File, create_dir_all, metadata, remove_file, read_dir, ReadDir, DirEntry, DirBuilder};
        } else if #[cfg(feature = "async_std_runtime")] {
            pub use super::async_std_runtime::{OpenOptions, File, create_dir_all, metadata, remove_file, read_dir, ReadDir, DirEntry, DirBuilder};
        }
    }
}

/// Types and traits for working with asynchronous tasks.
///
/// This module is similar to [`std::thread`], except it uses asynchronous tasks in place of
/// threads.
///
/// [`std::thread`]: https://doc.rust-lang.org/std/thread
#[cfg(any(feature = "async_std_runtime", feature = "tokio_runtime"))]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub mod async_task {
    cfg_if::cfg_if! {
        if #[cfg(feature = "tokio_runtime")] {
            pub use super::tokio_runtime::{spawn, JoinHandle, JoinError, spawn_blocking, block_on, sleep};
        } else if #[cfg(feature = "async_std_runtime")] {
            pub use super::async_std_runtime::{spawn, JoinHandle, JoinError, spawn_blocking, block_on, sleep};
        }
    }
}
