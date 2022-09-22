use super::{
    super::{ResponseError, ResponseErrorKind},
    ResolveOptions, ResolveResult, Resolver,
};
use std::{collections::VecDeque, mem::take, sync::Arc};

#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// 域名解析串
///
/// 将多个域名解析器串联起来，遍历并找寻第一个可用的解析结果
#[derive(Debug, Clone)]
pub struct ChainedResolver {
    resolvers: Arc<[Box<dyn Resolver>]>,
}

impl ChainedResolver {
    /// 创建域名解析串构建器
    #[inline]
    pub fn builder(first_resolver: impl Resolver + 'static) -> ChainedResolverBuilder {
        ChainedResolverBuilder::new(first_resolver)
    }
}

impl Resolver for ChainedResolver {
    fn resolve(&self, domain: &str, opts: ResolveOptions) -> ResolveResult {
        let mut last_result: Option<ResolveResult> = None;
        for resolver in self.resolvers.iter() {
            match resolver.resolve(domain, opts) {
                Ok(answers) if !answers.ip_addrs().is_empty() => return Ok(answers),
                result => last_result = Some(result),
            }
        }
        last_result.unwrap_or_else(|| Err(no_try_error(opts)))
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_resolve<'a>(&'a self, domain: &'a str, opts: ResolveOptions<'a>) -> BoxFuture<'a, ResolveResult> {
        Box::pin(async move {
            let mut last_result: Option<ResolveResult> = None;
            for resolver in self.resolvers.iter() {
                match resolver.async_resolve(domain, opts).await {
                    Ok(answers) if !answers.ip_addrs().is_empty() => return Ok(answers),
                    result => last_result = Some(result),
                }
            }
            last_result.unwrap_or_else(|| Err(no_try_error(opts)))
        })
    }
}

fn no_try_error(opts: ResolveOptions) -> ResponseError {
    let mut err = ResponseError::new_with_msg(ResponseErrorKind::NoTry, "None resolver is tried");
    if let Some(retried) = opts.retried() {
        err = err.retried(retried);
    }
    err
}

impl FromIterator<Box<dyn Resolver>> for ChainedResolver {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Box<dyn Resolver>>>(iter: T) -> Self {
        ChainedResolverBuilder::from_iter(iter).build()
    }
}

impl<'a> IntoIterator for &'a ChainedResolver {
    type Item = &'a Box<dyn Resolver>;
    type IntoIter = std::slice::Iter<'a, Box<dyn Resolver>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.resolvers.iter()
    }
}

/// 域名解析串构建器
#[derive(Debug, Default)]
pub struct ChainedResolverBuilder {
    resolvers: VecDeque<Box<dyn Resolver>>,
}

impl ChainedResolverBuilder {
    /// 创建域名解析串构建器
    #[inline]
    pub fn new(first_resolver: impl Resolver + 'static) -> Self {
        let mut builder = Self::default();
        builder.append_resolver(first_resolver);
        builder
    }

    /// 追加域名解析器
    #[inline]
    pub fn append_resolver(&mut self, resolver: impl Resolver + 'static) -> &mut Self {
        self.resolvers.push_back(Box::new(resolver));
        self
    }

    /// 前置域名解析器
    #[inline]
    pub fn prepend_resolver(&mut self, resolver: impl Resolver + 'static) -> &mut Self {
        self.resolvers.push_front(Box::new(resolver));
        self
    }

    /// 构建域名解析串
    #[inline]
    pub fn build(&mut self) -> ChainedResolver {
        assert!(
            !self.resolvers.is_empty(),
            "ChainedResolverBuilder must owns at least one Resolver"
        );
        ChainedResolver {
            resolvers: Vec::from(take(&mut self.resolvers)).into_boxed_slice().into(),
        }
    }
}

impl FromIterator<Box<dyn Resolver>> for ChainedResolverBuilder {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Box<dyn Resolver>>>(iter: T) -> Self {
        ChainedResolverBuilder {
            resolvers: VecDeque::from_iter(iter),
        }
    }
}

impl Extend<Box<dyn Resolver>> for ChainedResolverBuilder {
    #[inline]
    fn extend<T: IntoIterator<Item = Box<dyn Resolver>>>(&mut self, iter: T) {
        self.resolvers.extend(iter)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{make_dumb_resolver, make_error_resolver, make_static_resolver};
    use qiniu_http::ResponseErrorKind;
    use std::{
        error::Error,
        net::{IpAddr, Ipv4Addr},
        result::Result,
    };

    const IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
        IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2)),
    ];

    #[test]
    fn test_chained_resolver() -> Result<(), Box<dyn Error>> {
        let resolver = ChainedResolver::builder(make_static_resolver(IPS.to_vec().into()))
            .prepend_resolver(make_dumb_resolver())
            .prepend_resolver(make_error_resolver(
                ResponseErrorKind::LocalIoError.into(),
                "Test Local IO Error",
            ))
            .build();

        let ips = resolver.resolve("testdomain.com", Default::default())?;
        assert_eq!(ips.ip_addrs(), IPS);

        let resolver = ChainedResolver::builder(make_dumb_resolver())
            .prepend_resolver(make_static_resolver(IPS.to_vec().into()))
            .prepend_resolver(make_error_resolver(
                ResponseErrorKind::LocalIoError.into(),
                "Test Local IO Error",
            ))
            .build();

        let ips = resolver.resolve("testdomain.com", Default::default())?;
        assert_eq!(ips.ip_addrs(), IPS,);

        let resolver = ChainedResolver::builder(make_error_resolver(
            ResponseErrorKind::LocalIoError.into(),
            "Test Local IO Error",
        ))
        .prepend_resolver(make_dumb_resolver())
        .prepend_resolver(make_static_resolver(IPS.to_vec().into()))
        .build();

        let ips = resolver.resolve("testdomain.com", Default::default())?;
        assert_eq!(ips.ip_addrs(), IPS,);

        Ok(())
    }
}
