mod builder;
mod built;
mod request_data;
use std::{borrow::Cow, collections::HashMap};

pub type QueryKey<'q> = Cow<'q, str>;
pub type QueryValue<'q> = Cow<'q, str>;
pub type Queries<'q> = HashMap<QueryKey<'q>, QueryValue<'q>>;

#[derive(Clone, Copy, Debug)]
pub enum Idempotent {
    Always,
    Default,
    Never,
}

impl Default for Idempotent {
    #[inline]
    fn default() -> Self {
        Self::Default
    }
}

pub use builder::RequestBuilder;
pub use built::Request;
