use std::ops::{BitAnd, BitOr, Not};

use http::{header::HeaderName, HeaderValue, StatusCode};
use smart_default::SmartDefault;

pub(super) type OnProgress<'r> =
    &'r (dyn Fn(&TransferProgressInfo) -> CallbackResult + Send + Sync);
pub(super) type OnStatusCode<'r> = &'r (dyn Fn(StatusCode) -> CallbackResult + Send + Sync);
pub(super) type OnHeader<'r> =
    &'r (dyn Fn(&HeaderName, &HeaderValue) -> CallbackResult + Send + Sync);

/// 上传进度信息
pub struct TransferProgressInfo<'b> {
    transferred_bytes: u64,
    total_bytes: u64,
    body: &'b [u8],
}

impl<'b> TransferProgressInfo<'b> {
    #[inline]
    pub fn new(transferred_bytes: u64, total_bytes: u64, body: &'b [u8]) -> Self {
        Self {
            transferred_bytes,
            total_bytes,
            body,
        }
    }

    #[inline]
    pub fn transferred_bytes(&self) -> u64 {
        self.transferred_bytes
    }

    #[inline]
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    #[inline]
    pub fn body(&self) -> &[u8] {
        self.body
    }
}

#[must_use]
#[derive(SmartDefault, Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum CallbackResult {
    #[default]
    Continue,

    Cancel,
}

impl CallbackResult {
    #[inline]
    pub fn is_continue(self) -> bool {
        self == Self::Continue
    }

    #[inline]
    pub fn is_cancelled(self) -> bool {
        self == Self::Cancel
    }
}

impl FromIterator<CallbackResult> for CallbackResult {
    #[inline]
    fn from_iter<T: IntoIterator<Item = CallbackResult>>(iter: T) -> Self {
        if iter.into_iter().any(|result| result.is_cancelled()) {
            Self::Cancel
        } else {
            Self::Continue
        }
    }
}

impl Not for CallbackResult {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        if self.is_cancelled() {
            Self::Continue
        } else {
            Self::Cancel
        }
    }
}

impl BitAnd for CallbackResult {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        if self.is_cancelled() || rhs.is_cancelled() {
            Self::Cancel
        } else {
            Self::Continue
        }
    }
}

impl BitOr for CallbackResult {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        if self.is_continue() && rhs.is_continue() {
            Self::Continue
        } else {
            Self::Cancel
        }
    }
}
