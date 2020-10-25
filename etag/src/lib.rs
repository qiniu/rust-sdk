#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![deny(unsafe_code)]

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
