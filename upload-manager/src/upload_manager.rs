use super::{ConcurrencyProvider, DataPartitionProvider, ResumablePolicyProvider};
use qiniu_upload_token::UploadTokenProvider;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct UploadManager(Arc<UploadManagerInner>);

#[derive(Debug)]
struct UploadManagerInner {
    upload_token_provider: Box<dyn UploadTokenProvider>,
    data_partition_provider: Box<dyn DataPartitionProvider>,
    concurrency_provider: Box<dyn ConcurrencyProvider>,
    resumable_policy_provider: Box<dyn ResumablePolicyProvider>,
}
