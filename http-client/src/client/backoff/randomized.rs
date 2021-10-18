use super::{Backoff, BackoffDuration, BackoffOptions};
use qiniu_http::Request as HTTPRequest;
use rand::{thread_rng, Rng};
use std::{any::Any, convert::TryInto, time::Duration, u64};

pub use num_rational::Ratio;

#[derive(Debug, Clone)]
pub struct RandomizedBackoff<P> {
    base_backoff: P,
    minification: Ratio<u8>,
    magnification: Ratio<u8>,
}

impl<P> RandomizedBackoff<P> {
    #[inline]
    pub fn new(base_backoff: P, minification: Ratio<u8>, magnification: Ratio<u8>) -> Self {
        Self {
            base_backoff,
            minification,
            magnification,
        }
    }

    #[inline]
    pub fn base_backoff(&self) -> &P {
        &self.base_backoff
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

impl<P: Backoff> Backoff for RandomizedBackoff<P> {
    #[inline]
    fn time(&self, request: &mut HTTPRequest, opts: &BackoffOptions) -> BackoffDuration {
        let duration = self.base_backoff().time(request, opts).duration();
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
        Duration::from_nanos(randomized).into()
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_backoff(&self) -> &dyn Backoff {
        self
    }
}

impl<P: Default> Default for RandomizedBackoff<P> {
    #[inline]
    fn default() -> Self {
        RandomizedBackoff::new(P::default(), Ratio::new_raw(1, 2), Ratio::new_raw(3, 2))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{FixedBackoff, ResponseError, RetriedStatsInfo, RetryDecision},
        *,
    };
    use qiniu_http::ResponseErrorKind as HTTPResponseErrorKind;
    use std::{error::Error, result::Result};

    #[test]
    fn test_randomized_backoff() -> Result<(), Box<dyn Error>> {
        let fixed = FixedBackoff::new(Duration::from_secs(1));
        let randomized = RandomizedBackoff::new(fixed, Ratio::new_raw(1, 2), Ratio::new_raw(3, 2));

        for _ in 0..10000 {
            let delay = randomized
                .time(
                    &mut HTTPRequest::builder().build(),
                    &BackoffOptions::new(
                        RetryDecision::RetryRequest,
                        &ResponseError::new(
                            HTTPResponseErrorKind::TimeoutError.into(),
                            "Test Error",
                        ),
                        &RetriedStatsInfo::default(),
                    ),
                )
                .duration();
            assert!(delay >= Duration::from_millis(500));
            assert!(delay != Duration::from_millis(1000));
            assert!(delay < Duration::from_millis(1500));
        }

        Ok(())
    }
}
