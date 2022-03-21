#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![deny(
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

mod auto_uploader;
mod callbacks;
mod concurrency_provider;
mod data_partition_provider;
mod data_source;
mod multi_parts_uploader;
mod object_params;
mod resumable_policy;
mod resumable_recorder;
mod scheduler;
mod single_part_uploader;
mod upload_manager;
mod upload_token;

pub use qiniu_apis as apis;

pub use auto_uploader::{
    AutoUploader, AutoUploaderBuilder, AutoUploaderObjectParams, AutoUploaderObjectParamsBuilder,
    MultiPartsUploaderPrefer, MultiPartsUploaderSchedulerPrefer, SinglePartUploaderPrefer,
};
pub use callbacks::{MultiPartsUploaderWithCallbacks, UploaderWithCallbacks, UploadingProgressInfo};
pub use concurrency_provider::{
    Concurrency, ConcurrencyProvider, ConcurrencyProviderFeedback, FixedConcurrencyProvider,
};
pub use data_partition_provider::{
    DataPartitionProvider, DataPartitionProviderFeedback, FixedDataPartitionProvider, LimitedDataPartitionProvider,
    MultiplyDataPartitionProvider, PartSize,
};
pub use data_source::{DataSource, DataSourceReader, FileDataSource, SeekableSource, SourceKey, UnseekableDataSource};
pub use multi_parts_uploader::{
    InitializedParts, MultiPartsUploader, MultiPartsV1Uploader, MultiPartsV1UploaderInitializedObject,
    MultiPartsV1UploaderUploadedPart, MultiPartsV2Uploader, MultiPartsV2UploaderInitializedObject,
    MultiPartsV2UploaderUploadedPart, UploadedPart,
};
pub use object_params::{ObjectParams, ObjectParamsBuilder};
pub use resumable_policy::{
    AlwaysMultiParts, AlwaysSinglePart, DynRead, FixedThresholdResumablePolicy, GetPolicyOptions,
    MultiplePartitionsResumablePolicyProvider, ResumablePolicy, ResumablePolicyProvider,
};
pub use resumable_recorder::{
    AppendOnlyResumableRecorderMedium, DummyResumableRecorder, DummyResumableRecorderMedium,
    FileSystemResumableRecorder, ReadOnlyResumableRecorderMedium, ResumableRecorder,
};
pub use scheduler::{
    ConcurrentMultiPartsUploaderScheduler, MultiPartsUploaderScheduler, MultiPartsUploaderSchedulerExt,
    SerialMultiPartsUploaderScheduler,
};
pub use single_part_uploader::{FormUploader, SinglePartUploader};
pub use upload_manager::{UploadManager, UploadManagerBuilder};
pub use upload_token::UploadTokenSigner;

#[cfg(feature = "async")]
pub use {
    data_source::{AsyncDataSourceReader, AsyncSeekableSource, AsyncUnseekableDataSource},
    resumable_policy::DynAsyncRead,
    resumable_recorder::{AppendOnlyAsyncResumableRecorderMedium, ReadOnlyAsyncResumableRecorderMedium},
};

pub mod prelude {
    pub use super::apis::http_client::prelude::*;
    pub use super::{
        AppendOnlyResumableRecorderMedium, ConcurrencyProvider, DataPartitionProvider, DataSource, InitializedParts,
        MultiPartsUploader, MultiPartsUploaderScheduler, MultiPartsUploaderSchedulerExt,
        MultiPartsUploaderWithCallbacks, ReadOnlyResumableRecorderMedium, ResumablePolicyProvider, ResumableRecorder,
        SinglePartUploader, UploadedPart, UploaderWithCallbacks,
    };

    #[cfg(feature = "async")]
    pub use super::{AppendOnlyAsyncResumableRecorderMedium, DynAsyncRead, ReadOnlyAsyncResumableRecorderMedium};
}
