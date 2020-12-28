#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![deny(
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    explicit_outlives_requirements,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    non_ascii_idents,
    indirect_structural_match,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unstable_features,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications
)]
#![warn(missing_crate_level_docs)]

pub use digest::{
    generic_array::{typenum::U28, GenericArray},
    FixedOutput, Reset, Update,
};

mod etag;
mod etag_v1;
mod etag_v2;
mod sha1;

pub use etag::{etag_of, etag_to_buf, etag_with_parts, etag_with_parts_to_buf, Etag, ETAG_SIZE};
pub use etag_v1::EtagV1;
pub use etag_v2::EtagV2;

#[cfg(feature = "async")]
mod async_etag;

#[cfg(feature = "async")]
pub use async_etag::{
    etag_of as async_etag_of, etag_to_buf as async_etag_to_buf,
    etag_with_parts as async_etag_with_parts,
    etag_with_parts_to_buf as async_etag_with_parts_to_buf,
};
