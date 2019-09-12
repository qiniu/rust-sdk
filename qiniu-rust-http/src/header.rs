use std::{borrow::Cow, collections::HashMap};

pub type HeaderName<'n> = Cow<'n, str>;
pub type HeaderValue<'v> = Cow<'v, str>;
pub type Headers<'h> = HashMap<HeaderName<'h>, HeaderValue<'h>>;
