use super::super::{InitializedParts, ReinitializeOptions};
use qiniu_apis::http_client::{Region, RegionsProvider, ResponseError, ResponseErrorKind, RetryDecision};

pub(super) fn keep_original_region_options() -> ReinitializeOptions {
    ReinitializeOptions::builder().keep_original_region().build()
}

pub(super) fn specify_region_options(regions_provider: impl RegionsProvider + 'static) -> ReinitializeOptions {
    ReinitializeOptions::builder()
        .regions_provider(regions_provider)
        .build()
}

pub(super) fn need_to_retry(err: &ResponseError) -> bool {
    matches!(
        err.retry_decision(),
        Some(RetryDecision::TryNextServer | RetryDecision::RetryRequest | RetryDecision::Throttled)
    )
}

pub(super) fn no_region_tried_error() -> ResponseError {
    ResponseError::new_with_msg(ResponseErrorKind::NoTry, "None region is tried")
}

pub(super) fn remove_used_region_from_regions<I: InitializedParts>(regions: &mut Vec<Region>, initialized: &I) {
    if let Some(found_idx) = regions.iter().position(|r| r.up().similar(initialized.up_endpoints())) {
        regions.remove(found_idx);
    }
}

#[derive(Debug)]
pub(super) struct UploadResumedPartsError<I: InitializedParts> {
    pub(super) err: ResponseError,
    pub(super) resumed: bool,
    pub(super) initialized: Option<I>,
}

impl<I: InitializedParts> UploadResumedPartsError<I> {
    pub(super) fn new(err: ResponseError, resumed: bool, initialized: Option<I>) -> Self {
        Self {
            err,
            resumed,
            initialized,
        }
    }
}

pub(super) struct UploadPartsError<I: InitializedParts> {
    pub(super) err: ResponseError,
    pub(super) initialized: Option<I>,
}

impl<I: InitializedParts> UploadPartsError<I> {
    pub(super) fn new(err: ResponseError, initialized: Option<I>) -> Self {
        Self { err, initialized }
    }
}
