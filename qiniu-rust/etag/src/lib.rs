pub use digest::{FixedOutput, Reset, Update};

mod etag;

pub use etag::{
    etag_of_file, etag_of_file_to_array, etag_of_file_with_parts, etag_of_file_with_parts_to_array,
    etag_of_reader, etag_of_reader_to_array, etag_of_reader_with_parts,
    etag_of_reader_with_parts_to_array, EtagV1, EtagV2, ETAG_SIZE,
};
