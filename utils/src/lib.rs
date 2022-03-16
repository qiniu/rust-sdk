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
    unstable_features,
    unsafe_code,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications
)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod base64;
pub mod smallstr;

mod name;
pub use name::{BucketName, ObjectName};
