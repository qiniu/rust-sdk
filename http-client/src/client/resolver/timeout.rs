use super::{super::ResponseError, ResolveOptions, ResolveResult, Resolver};
use qiniu_http::ResponseErrorKind as HttpResponseErrorKind;
use std::time::Duration;

#[cfg(feature = "async")]
use {
    futures::future::{BoxFuture, FutureExt},
    futures_timer::Delay as AsyncDelay,
};

const DEFAULT_RESOLVE_TIMEOUT: Duration = Duration::from_secs(5);

/// 超时域名解析器
///
/// 为一个域名解析器实例提供超时功能
///
/// 默认超时时间为 5 秒
#[derive(Debug, Clone)]
pub struct TimeoutResolver<R: ?Sized> {
    timeout: Duration,
    resolver: R,
}

impl<R> TimeoutResolver<R> {
    /// 创建超时解析器
    #[inline]
    pub fn new(resolver: R, timeout: Duration) -> Self {
        Self { timeout, resolver }
    }
}

impl<R: Default> Default for TimeoutResolver<R> {
    #[inline]
    fn default() -> Self {
        Self::new(Default::default(), DEFAULT_RESOLVE_TIMEOUT)
    }
}

impl<R: Resolver + Clone + 'static> Resolver for TimeoutResolver<R> {
    #[inline]
    fn resolve(&self, domain: &str, opts: ResolveOptions) -> ResolveResult {
        return _resolve(self, domain, opts);

        #[cfg(feature = "async")]
        fn _resolve<R: Resolver + Clone + 'static>(
            resolver: &TimeoutResolver<R>,
            domain: &str,
            opts: ResolveOptions,
        ) -> ResolveResult {
            async_std::task::block_on(async move { resolver.async_resolve(domain, opts).await })
        }

        #[cfg(not(feature = "async"))]
        fn _resolve<R: Resolver + Clone + 'static>(
            resolver: &TimeoutResolver<R>,
            domain: &str,
            opts: ResolveOptions,
        ) -> ResolveResult {
            use super::{super::super::spawn::spawn, owned_resolver_options::OwnedResolveOptions};
            use crossbeam_channel::{bounded, Select, SelectTimeoutError};
            use log::warn;

            let (sender, receiver) = bounded(0);
            {
                let inner = resolver.to_owned();
                let domain = domain.to_owned();
                let opts = OwnedResolveOptions::from(opts);
                if let Err(err) = spawn(
                    "qiniu.rust-sdk.http-client.resolver.TimeoutResolver.resolve".into(),
                    move || {
                        let opts = ResolveOptions::from(&opts);
                        sender.send(inner.resolver.resolve(&domain, opts)).ok();
                    },
                ) {
                    warn!("Timeout Resolver was failed to spawn thread to resolve domain: {}", err);
                }
            }
            let mut sel = Select::new();
            let op1 = sel.recv(&receiver);
            let oper = sel.select_timeout(resolver.timeout);
            return match oper {
                Ok(op) => match op.index() {
                    i if i == op1 => op.recv(&receiver).unwrap(),
                    _ => unreachable!(),
                },
                Err(err) => Err(make_timeout_error(err, opts)),
            };

            fn make_timeout_error(err: SelectTimeoutError, opts: ResolveOptions) -> ResponseError {
                let mut err = ResponseError::new(HttpResponseErrorKind::TimeoutError.into(), err);
                if let Some(retried) = opts.retried() {
                    err = err.retried(retried);
                }
                err
            }
        }
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_resolve<'a>(&'a self, domain: &'a str, opts: ResolveOptions<'a>) -> BoxFuture<'a, ResolveResult> {
        use futures::{pin_mut, select};

        return Box::pin(async move {
            let resolve_task = self.resolver.async_resolve(domain, opts).fuse();
            let timeout_task = AsyncDelay::new(self.timeout).fuse();
            pin_mut!(resolve_task);
            pin_mut!(timeout_task);
            select! {
                resolve_result = resolve_task => resolve_result,
                _ = timeout_task => Err(make_timeout_error(self.timeout, opts)),
            }
        });

        fn make_timeout_error(timeout: Duration, opts: ResolveOptions) -> ResponseError {
            let mut err = ResponseError::new_with_msg(
                HttpResponseErrorKind::TimeoutError.into(),
                format!("Failed to resolve domain in {:?}", timeout),
            );
            if let Some(retried) = opts.retried() {
                err = err.retried(retried);
            }
            err
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        error::Error,
        net::{IpAddr, Ipv4Addr},
        thread::sleep,
    };

    const IPS: &[IpAddr] = &[IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))];

    #[derive(Clone, Copy, Debug)]
    struct WaitResolver(Duration);

    impl Resolver for WaitResolver {
        #[inline]
        fn resolve(&self, _domain: &str, _opts: ResolveOptions) -> ResolveResult {
            sleep(self.0);
            Ok(IPS.to_owned().into_boxed_slice().into())
        }

        #[inline]
        #[cfg(feature = "async")]
        fn async_resolve<'a>(&'a self, _domain: &'a str, _opts: ResolveOptions) -> BoxFuture<'a, ResolveResult> {
            Box::pin(async move {
                AsyncDelay::new(self.0).await;
                Ok(IPS.to_owned().into_boxed_slice().into())
            })
        }
    }

    #[test]
    fn test_timeout_resolver() -> Result<(), Box<dyn Error>> {
        let resolver = TimeoutResolver::new(WaitResolver(Duration::from_secs(1)), Duration::from_secs(2));

        let answers = resolver.resolve("fake.domain", Default::default())?;
        assert_eq!(answers.ip_addrs(), IPS);

        let resolver = TimeoutResolver::new(WaitResolver(Duration::from_secs(2)), Duration::from_secs(1));
        resolver.resolve("fake.domain", Default::default()).unwrap_err();

        Ok(())
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_async_timeout_resolver() -> Result<(), Box<dyn Error>> {
        let resolver = TimeoutResolver::new(WaitResolver(Duration::from_millis(100)), Duration::from_millis(200));

        let answers = resolver.async_resolve("fake.domain", Default::default()).await?;
        assert_eq!(answers.ip_addrs(), IPS);

        let resolver = TimeoutResolver::new(WaitResolver(Duration::from_millis(200)), Duration::from_millis(100));
        resolver
            .async_resolve("fake.domain", Default::default())
            .await
            .unwrap_err();

        Ok(())
    }
}
