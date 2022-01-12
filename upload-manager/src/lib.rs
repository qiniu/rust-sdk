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
    trivial_numeric_casts,
    unsafe_code,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]

mod callbacks;
mod concurrency_provider;
mod data_partition_provider;
mod data_source;
mod multi_parts_uploader;
mod object_params;
mod resumable_policy;
mod resumable_recorder;
mod single_part_uploader;
mod upload_manager;
mod upload_token;

pub use qiniu_apis as apis;

pub use callbacks::{UploaderWithCallbacks, UploadingProgressInfo};
pub use concurrency_provider::{
    Concurrency, ConcurrencyProvider, ConcurrencyProviderFeedback, FixedConcurrencyProvider,
    TimeAwareConcurrencyProvider,
};
pub use data_partition_provider::{
    DataPartitionProvider, DataPartitionProviderFeedback, FixedDataPartitionProvider,
    LimitedDataPartitionProvider, MultiplyDataPartitionProvider, PartSize,
    TimeAwareDataPartitionProvider,
};
pub use data_source::SourceKey;
pub use multi_parts_uploader::MultiPartsUploader;
pub use object_params::{DataCheck, ObjectParams, ObjectParamsBuilder};
pub use resumable_policy::{
    AlwaysMultiParts, AlwaysSinglePart, FixedThresholdResumablePolicy, GetPolicyOptions,
    MultiplePartitionsResumablePolicyProvider, ResumablePolicy, ResumablePolicyProvider,
};
pub use resumable_recorder::{
    AppendOnlyResumableRecorderMedium, FileSystemAppendOnlyResumableRecorderMedium,
    FileSystemReadOnlyResumableRecorderMedium, FileSystemResumableRecorder,
    ReadOnlyResumableRecorderMedium, ResumableRecorder,
};
pub use single_part_uploader::{FormUploader, SinglePartUploader};
pub use upload_manager::{UploadManager, UploadManagerBuilder};
pub use upload_token::UploadTokenSigner;

#[cfg(feature = "async")]
pub use resumable_recorder::{
    AppendOnlyAsyncResumableRecorderMedium, FileSystemAppendOnlyAsyncResumableRecorderMedium,
    FileSystemReadOnlyAsyncResumableRecorderMedium, ReadOnlyAsyncResumableRecorderMedium,
};

pub mod prelude {
    pub use super::apis::http_client::preclude::*;
    pub use super::{
        AppendOnlyResumableRecorderMedium, ConcurrencyProvider, DataPartitionProvider,
        MultiPartsUploader, ReadOnlyResumableRecorderMedium, ResumablePolicyProvider,
        ResumableRecorder, SinglePartUploader,
    };

    #[cfg(feature = "async")]
    pub use super::{AppendOnlyAsyncResumableRecorderMedium, ReadOnlyAsyncResumableRecorderMedium};
}
