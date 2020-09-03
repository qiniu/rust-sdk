use super::context::Context;
use curl::{easy::Easy2, init as curl_init};
use object_pool::Pool;
use once_cell::sync::Lazy;
use std::{
    mem::{size_of, transmute, transmute_copy, ManuallyDrop},
    ops::DerefMut,
};

static POOL: Lazy<Pool<Easy2ContextRef>> = Lazy::new(|| Pool::new(16, Easy2ContextRef::default));

pub(super) fn pull<'a>() -> impl DerefMut<Target = Easy2ContextRef> {
    POOL.pull(Easy2ContextRef::default)
}

pub(super) struct Easy2ContextRef([u8; size_of::<*mut Easy2<Context<'static>>>()]);

impl Default for Easy2ContextRef {
    fn default() -> Self {
        curl_init();
        Box::new(Easy2::new(Context::default())).into()
    }
}

impl<'r> From<&'r mut Easy2ContextRef> for ManuallyDrop<Box<Easy2<Context<'r>>>> {
    fn from(r: &'r mut Easy2ContextRef) -> Self {
        let boxed = unsafe {
            let ptr: *mut Easy2<Context<'r>> = transmute_copy(r);
            Box::from_raw(ptr)
        };
        ManuallyDrop::new(boxed)
    }
}

impl<'r> From<Box<Easy2<Context<'r>>>> for Easy2ContextRef {
    fn from(context: Box<Easy2<Context<'r>>>) -> Self {
        unsafe { transmute(Box::into_raw(context)) }
    }
}
