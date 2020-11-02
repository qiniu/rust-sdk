use super::{ResponseError, RetriedStatsInfo, RetryDelayPolicy, RetryResult};
use qiniu_http::Request as HTTPRequest;
use rand::{thread_rng, Rng};
use std::{any::Any, convert::TryInto, time::Duration, u64};

pub use num_rational::Ratio;

#[derive(Debug, Clone, Copy)]
pub struct RandomizedRetryDelayPolicy<P: RetryDelayPolicy> {
    base_policy: P,
    minification: Ratio<u8>,
    magnification: Ratio<u8>,
}

impl<P: RetryDelayPolicy> RandomizedRetryDelayPolicy<P> {
    #[inline]
    pub fn new(base_policy: P, minification: Ratio<u8>, magnification: Ratio<u8>) -> Self {
        Self {
            base_policy,
            minification,
            magnification,
        }
    }

    #[inline]
    pub fn base_policy(&self) -> &P {
        &self.base_policy
    }

    #[inline]
    pub fn minification(&self) -> Ratio<u8> {
        self.minification
    }

    #[inline]
    pub fn magnification(&self) -> Ratio<u8> {
        self.magnification
    }
}

impl<P: RetryDelayPolicy> RetryDelayPolicy for RandomizedRetryDelayPolicy<P> {
    #[inline]
    fn delay_before_next_retry(
        &self,
        request: &mut HTTPRequest,
        retry_result: RetryResult,
        response_error: &ResponseError,
        retried: &RetriedStatsInfo,
    ) -> Duration {
        let duration = self.base_policy().delay_before_next_retry(
            request,
            retry_result,
            response_error,
            retried,
        );
        let minification: Ratio<u128> = Ratio::new_raw(
            self.minification().numer().to_owned().into(),
            self.minification().denom().to_owned().into(),
        );
        let magnification: Ratio<u128> = Ratio::new_raw(
            self.magnification().numer().to_owned().into(),
            self.magnification().denom().to_owned().into(),
        );
        let minified: u64 = (minification * duration.as_nanos())
            .to_integer()
            .try_into()
            .unwrap_or(u64::MAX);
        let magnified: u64 = (magnification * duration.as_nanos())
            .to_integer()
            .try_into()
            .unwrap_or(u64::MAX);

        let randomized = thread_rng().gen_range(minified, magnified);
        Duration::from_nanos(randomized)
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_retry_delay_policy(&self) -> &dyn RetryDelayPolicy {
        self
    }
}

impl<P: RetryDelayPolicy + Default> Default for RandomizedRetryDelayPolicy<P> {
    #[inline]
    fn default() -> Self {
        RandomizedRetryDelayPolicy::new(P::default(), Ratio::new_raw(1, 2), Ratio::new_raw(3, 2))
    }
}

#[cfg(test)]
mod tests {
    use super::{super::FixedRetryDelayPolicy, *};
    use qiniu_http::ResponseErrorKind as HTTPResponseErrorKind;
    use std::{error::Error, result::Result};

    #[test]
    fn test_randomized_retry_delay_policy() -> Result<(), Box<dyn Error>> {
        let fixed = FixedRetryDelayPolicy::new(Duration::from_secs(1));
        let randomized =
            RandomizedRetryDelayPolicy::new(fixed, Ratio::new_raw(1, 2), Ratio::new_raw(3, 2));

        for _ in 0..10000 {
            let delay = randomized.delay_before_next_retry(
                &mut HTTPRequest::builder().build(),
                RetryResult::RetryRequest,
                &ResponseError::new(HTTPResponseErrorKind::TimeoutError.into(), "Test Error"),
                &RetriedStatsInfo::default(),
            );
            assert!(delay >= Duration::from_millis(500));
            assert!(delay != Duration::from_millis(1000));
            assert!(delay < Duration::from_millis(1500));
        }

        Ok(())
    }
}
