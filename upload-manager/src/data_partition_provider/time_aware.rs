use super::{DataPartitionProvider, DataPartitionProviderFeedback, PartSize};
use qiniu_apis::{
    http::ResponseErrorKind as HttpResponseErrorKind, http_client::ResponseErrorKind,
};
use std::{
    num::NonZeroU64,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct TimeAwareDataPartitionProvider(Arc<TimeAwareDataPartitionProviderInner>);

#[derive(Debug)]
struct TimeAwareDataPartitionProviderInner {
    current_base: AtomicU64,
    max_base: NonZeroU64,
    multiply: NonZeroU64,
    up_threshold: Duration,
    down_threshold: Duration,
}

impl DataPartitionProvider for TimeAwareDataPartitionProvider {
    fn part_size(&self) -> PartSize {
        let part_size = self
            .0
            .current_base
            .load(Ordering::Relaxed)
            .min(self.0.max_base.get())
            * self.0.multiply.get();
        NonZeroU64::new(part_size).unwrap().into()
    }

    fn feedback(&self, feedback: DataPartitionProviderFeedback<'_>) {
        let feedback_base = self.feedback_base(&feedback);
        if (maybe_network_error(&feedback) || self.slow_network(&feedback)) && feedback_base > 1 {
            self.0
                .current_base
                .compare_exchange(
                    feedback_base,
                    feedback_base - 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .ok();
        } else if feedback.error().is_none()
            && self.fast_network(&feedback)
            && feedback_base < self.0.max_base.get()
        {
            self.0
                .current_base
                .compare_exchange(
                    feedback_base,
                    feedback_base + 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .ok();
        }
    }
}

impl TimeAwareDataPartitionProvider {
    #[inline]
    #[must_use]
    pub fn new(
        &self,
        initial_base: NonZeroU64,
        max_base: NonZeroU64,
        multiply: NonZeroU64,
        up_threshold: Duration,
        down_threshold: Duration,
    ) -> Self {
        Self(Arc::new(TimeAwareDataPartitionProviderInner {
            current_base: AtomicU64::new(initial_base.get()),
            max_base,
            multiply,
            up_threshold,
            down_threshold,
        }))
    }

    fn slow_network(&self, feedback: &DataPartitionProviderFeedback<'_>) -> bool {
        feedback.elapsed() > self.0.down_threshold
    }

    fn fast_network(&self, feedback: &DataPartitionProviderFeedback<'_>) -> bool {
        feedback.elapsed() < self.0.up_threshold
    }

    fn feedback_base(&self, feedback: &DataPartitionProviderFeedback<'_>) -> u64 {
        let x = feedback.part_size().get();
        let y = self.0.multiply.get();
        x / y + if x % y > 0 { 1 } else { 0 }
    }
}

fn maybe_network_error(feedback: &DataPartitionProviderFeedback<'_>) -> bool {
    matches!(
        feedback.error().map(|err| err.kind()),
        Some(ResponseErrorKind::HttpError(
            HttpResponseErrorKind::ConnectError
                | HttpResponseErrorKind::SendError
                | HttpResponseErrorKind::ReceiveError
                | HttpResponseErrorKind::TimeoutError,
        ))
    )
}
