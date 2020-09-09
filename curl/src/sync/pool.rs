use super::context::Context;
use curl::{easy::Easy2, init as curl_init};
use object_pool::Pool;
use once_cell::sync::Lazy;
use std::ops::DerefMut;

static POOL: Lazy<Pool<Easy2<Context>>> = Lazy::new(|| Pool::new(1, default));

#[inline]
pub(super) fn pull() -> impl DerefMut<Target = Easy2<Context<'static>>> {
    POOL.pull(default)
}

#[inline]
fn default<'ctx>() -> Easy2<Context<'ctx>> {
    curl_init();
    Easy2::new(Context::default())
}
