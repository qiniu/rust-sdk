use std::{
    borrow::Cow,
    cmp::{Ord, Ordering},
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
};

#[derive(Clone, Eq)]
pub struct HeaderName<'n>(Cow<'n, str>);

impl<'n> HeaderName<'n> {
    pub fn new(header_name: impl Into<Cow<'n, str>>) -> HeaderName<'n> {
        HeaderName(header_name.into())
    }
}

impl PartialEq for HeaderName<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq_ignore_ascii_case(other.as_ref())
    }
}

impl AsRef<str> for HeaderName<'_> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Deref for HeaderName<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Hash for HeaderName<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().to_uppercase().hash(state)
    }
}

impl PartialOrd for HeaderName<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.to_ascii_lowercase().partial_cmp(&other.to_ascii_lowercase())
    }
}

impl Ord for HeaderName<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_ascii_lowercase().cmp(&other.to_ascii_lowercase())
    }
}

impl<'n> From<&'n str> for HeaderName<'n> {
    fn from(s: &'n str) -> Self {
        HeaderName(s.into())
    }
}

impl From<String> for HeaderName<'_> {
    fn from(s: String) -> Self {
        HeaderName(s.into())
    }
}

impl<'n> From<Cow<'n, str>> for HeaderName<'n> {
    fn from(s: Cow<'n, str>) -> Self {
        HeaderName(s)
    }
}

impl fmt::Debug for HeaderName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl fmt::Display for HeaderName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

pub type HeaderValue<'v> = Cow<'v, str>;
pub type Headers<'h> = HashMap<HeaderName<'h>, HeaderValue<'h>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::{boxed::Box, error::Error, result::Result};

    #[test]
    fn test_header_name() -> Result<(), Box<dyn Error>> {
        let mut headers = Headers::new();
        headers.insert("Authorization".into(), "Test".into());
        assert_eq!(headers.get(&"authorization".into()), Some(&"Test".into()));
        assert_eq!(headers.get(&"AUTHORIZATION".into()), Some(&"Test".into()));
        Ok(())
    }
}
