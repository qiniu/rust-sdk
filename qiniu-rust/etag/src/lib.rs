pub use digest::{FixedOutput, Reset, Update};

mod etag;

pub use etag::{
    etag_of, etag_to_buf, etag_with_parts, etag_with_parts_to_buf, EtagV1, EtagV2, ETAG_SIZE,
};
