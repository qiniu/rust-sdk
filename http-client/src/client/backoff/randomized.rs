use super::{Backoff, BackoffOptions, GotBackoffDuration};
use qiniu_http::RequestParts as HttpRequestParts;
use rand::{thread_rng, Rng};
use std::{convert::TryInto, fmt::Debug, time::Duration, u64};

pub use num_rational::Ratio;

/// 均匀分布随机化退避时长提供者
///
/// 基于一个退避时长提供者并为其增加随机化范围
///
/// 默认的随机化范围是 `[1/2, 3/2]`
#[derive(Debug, Clone)]
pub struct RandomizedBackoff<P: ?Sized> {
    minification: Ratio<u8>,
    magnification: Ratio<u8>,
    base_backoff: P,
}

impl<P> RandomizedBackoff<P> {
    /// 创建均匀分布随机化退避时长提供者
    ///
    /// 需要提供随机化范围，其中随机化范围由最小随机比率和最大随机比率组成，返回的退避时长为 `random(base_backoff * minification, base_backoff * magnification)`
    ///
    /// 需要注意，提供的随机比率的分母必须大于 0。
    #[inline]
    pub const fn new(base_backoff: P, minification: Ratio<u8>, magnification: Ratio<u8>) -> Self {
        assert!(*minification.denom() > 0);
        assert!(*magnification.denom() > 0);
        Self {
            base_backoff,
            minification,
            magnification,
        }
    }

    /// 获取基础退避时长提供者
    #[inline]
    pub const fn base_backoff(&self) -> &P {
        &self.base_backoff
    }

    /// 获取最小随机比率
    #[inline]
    pub const fn minification(&self) -> Ratio<u8> {
        self.minification
    }

    /// 获取最大随机比率
    #[inline]
    pub const fn magnification(&self) -> Ratio<u8> {
        self.magnification
    }
}

impl<P: Backoff + Clone> Backoff for RandomizedBackoff<P> {
    fn time(&self, request: &mut HttpRequestParts, opts: BackoffOptions) -> GotBackoffDuration {
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
    use qiniu_http::ResponseErrorKind as HttpResponseErrorKind;
    use std::{error::Error, result::Result};

    #[test]
    fn test_randomized_backoff() -> Result<(), Box<dyn Error>> {
        let fixed = FixedBackoff::new(Duration::from_secs(1));
        let randomized = RandomizedBackoff::new(fixed, Ratio::new_raw(1, 2), Ratio::new_raw(3, 2));

        for _ in 0..10000 {
            let delay = randomized
                .time(
                    &mut HttpRequestParts::default(),
                    BackoffOptions::builder(
                        &ResponseError::new_with_msg(HttpResponseErrorKind::TimeoutError.into(), "Test Error"),
                        &RetriedStatsInfo::default(),
                    )
                    .retry_decision(RetryDecision::RetryRequest)
                    .build(),
                )
                .duration();
            assert!(delay >= Duration::from_millis(500));
            assert!(delay != Duration::from_millis(1000));
            assert!(delay < Duration::from_millis(1500));
        }

        Ok(())
    }
}
