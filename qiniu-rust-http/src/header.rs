use std::{borrow::Cow, collections::HashMap};

pub type HeaderName = Cow<'static, str>;
pub type HeaderValue = Cow<'static, str>;
pub type Headers = HashMap<HeaderName, HeaderValue>;
