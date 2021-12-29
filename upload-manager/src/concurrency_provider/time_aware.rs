use super::{Concurrency, ConcurrencyProvider, ConcurrencyProviderFeedback};
use qiniu_apis::{
    http::ResponseErrorKind as HttpResponseErrorKind, http_client::ResponseErrorKind,
};
use std::{
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct TimeAwareConcurrencyProvider(Arc<TimeAwareConcurrencyProviderInner>);

#[derive(Debug)]
struct TimeAwareConcurrencyProviderInner {
    current: AtomicUsize,
    up_threshold: Duration,
    down_threshold: Duration,
    max_concurrency: NonZeroUsize,
}

impl ConcurrencyProvider for TimeAwareConcurrencyProvider {
    #[inline]
    fn concurrency(&self) -> Concurrency {
        Concurrency::new(
            self.0
                .current
                .load(Ordering::Relaxed)
                .min(self.0.max_concurrency.get()),
        )
        .unwrap_or_default()
    }

    fn feedback(&self, feedback: ConcurrencyProviderFeedback<'_>) {
        if (maybe_network_error(&feedback) || self.slow_network(&feedback))
            && feedback.concurrency().get() > 1
        {
            self.0
                .current
                .compare_exchange(
                    feedback.concurrency().get(),
                    feedback.concurrency().get() - 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .ok();
        } else if feedback.error().is_none()
            && self.fast_network(&feedback)
            && feedback.concurrency < self.0.max_concurrency
        {
            self.0
                .current
                .compare_exchange(
                    feedback.concurrency().get(),
                    feedback.concurrency().get() + 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .ok();
        }
    }
}

fn maybe_network_error(feedback: &ConcurrencyProviderFeedback<'_>) -> bool {
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

impl TimeAwareConcurrencyProvider {
    #[inline]
    #[must_use]
    pub fn new(
        &self,
        initial_concurrency: NonZeroUsize,
        max_concurrency: NonZeroUsize,
        up_threshold: Duration,
        down_threshold: Duration,
    ) -> Self {
        Self(Arc::new(TimeAwareConcurrencyProviderInner {
            current: AtomicUsize::new(initial_concurrency.get()),
            max_concurrency,
            up_threshold,
            down_threshold,
        }))
    }

    fn slow_network(&self, feedback: &ConcurrencyProviderFeedback<'_>) -> bool {
        feedback.elapsed() > self.0.down_threshold
    }

    fn fast_network(&self, feedback: &ConcurrencyProviderFeedback<'_>) -> bool {
        feedback.elapsed() < self.0.up_threshold
    }
}
