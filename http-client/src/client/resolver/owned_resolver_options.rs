use super::{super::RetriedStatsInfo, ResolveOptions};

#[derive(Debug, Clone, Default)]
pub(super) struct OwnedResolveOptions {
    retried: Option<RetriedStatsInfo>,
}

impl OwnedResolveOptions {
    pub(super) fn retried(&self) -> Option<&RetriedStatsInfo> {
        self.retried.as_ref()
    }
}

impl<'a> From<ResolveOptions<'a>> for OwnedResolveOptions {
    fn from(opts: ResolveOptions<'a>) -> Self {
        Self {
            retried: opts.retried().cloned(),
        }
    }
}

impl<'a> From<&'a OwnedResolveOptions> for ResolveOptions<'a> {
    fn from(opts: &'a OwnedResolveOptions) -> Self {
        Self {
            retried: opts.retried(),
        }
    }
}
