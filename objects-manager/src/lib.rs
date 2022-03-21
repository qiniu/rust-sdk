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
    non_ascii_idents,
    indirect_structural_match,
    trivial_numeric_casts,
    unsafe_code,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]

mod batch_operations;
mod bucket;
mod callbacks;
mod list;
mod objects_manager;
mod operation;

pub use batch_operations::{BatchOperations, BatchOperationsIterator, BatchSizeProvider};
pub use bucket::{Bucket, ListBuilder};
pub use list::{ListIter, ListVersion};
pub use objects_manager::{ObjectsManager, ObjectsManagerBuilder};
pub use operation::{
    AfterDays, CopyObjectBuilder, DeleteObjectBuilder, ModifyObjectLifeCycleBuilder, ModifyObjectMetadataBuilder,
    ModifyObjectStatusBuilder, MoveObjectBuilder, ObjectType, OperationProvider, SetObjectTypeBuilder,
    StatObjectBuilder, UnfreezeObjectBuilder,
};
pub use qiniu_apis as apis;

#[cfg(feature = "async")]
pub use {batch_operations::BatchOperationsStream, list::ListStream};

pub mod prelude {
    pub use super::apis::http_client::prelude::*;
    pub use super::{BatchSizeProvider, OperationProvider};
}
