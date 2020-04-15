use lazy_static::lazy_static;
use std::{
    borrow::Cow,
    cmp::{Ord, Ordering},
    collections::{HashMap, HashSet},
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
};

/// HTTP Header 名称
#[derive(Clone, Eq)]
pub struct HeaderName<'n>(Cow<'n, str>);

/// HTTP Header 名称
#[derive(Clone, Eq)]
pub struct HeaderNameOwned(String);

impl<'n> HeaderName<'n> {
    /// 创建 HTTP Header 名称
    pub fn new(header_name: impl Into<Cow<'n, str>>) -> HeaderName<'n> {
        make_header_name(header_name.into())
    }
}

impl HeaderNameOwned {
    pub fn new<'a>(header_name: impl Into<Cow<'a, str>>) -> HeaderNameOwned {
        HeaderName::new(header_name).into()
    }
}

fn make_header_name(header_name: Cow<str>) -> HeaderName<'_> {
    let mut need_not_clone = header_name
        .chars()
        .any(|header_char| !HEADER_NAME_TOKEN.contains(&header_char));
    if need_not_clone {
        let mut upper = true;
        need_not_clone = header_name.chars().all(|header_char| {
            if (upper && header_char.is_lowercase()) || (!upper && header_char.is_uppercase()) {
                false
            } else {
                upper = header_char == '-';
                true
            }
        })
    };
    if need_not_clone {
        return HeaderName(header_name);
    }

    let mut upper = true;
    let mut new_header_name = String::with_capacity(header_name.len());
    for header_char in header_name.chars() {
        if upper && header_char.is_lowercase() {
            new_header_name.push(header_char.to_ascii_uppercase());
        } else if !upper && header_char.is_uppercase() {
            new_header_name.push(header_char.to_ascii_lowercase());
        } else {
            new_header_name.push(header_char);
        }
        upper = header_char == '-';
    }
    HeaderName(new_header_name.into())
}

impl PartialEq for HeaderName<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq_ignore_ascii_case(other.as_ref())
    }
}

impl PartialEq for HeaderNameOwned {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq_ignore_ascii_case(other.as_ref())
    }
}

impl AsRef<str> for HeaderName<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl AsRef<str> for HeaderNameOwned {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for HeaderName<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Deref for HeaderNameOwned {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Hash for HeaderName<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().to_uppercase().hash(state)
    }
}

impl Hash for HeaderNameOwned {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().to_uppercase().hash(state)
    }
}

impl PartialOrd for HeaderName<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.to_ascii_lowercase().partial_cmp(&other.to_ascii_lowercase())
    }
}

impl PartialOrd for HeaderNameOwned {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.to_ascii_lowercase().partial_cmp(&other.to_ascii_lowercase())
    }
}

impl Ord for HeaderName<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_ascii_lowercase().cmp(&other.to_ascii_lowercase())
    }
}

impl Ord for HeaderNameOwned {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_ascii_lowercase().cmp(&other.to_ascii_lowercase())
    }
}

impl<'n> From<&'n str> for HeaderName<'n> {
    fn from(s: &'n str) -> Self {
        make_header_name(s.into())
    }
}

impl<'n> From<&'n str> for HeaderNameOwned {
    fn from(s: &'n str) -> Self {
        HeaderName::from(s).into()
    }
}

impl From<String> for HeaderName<'_> {
    fn from(s: String) -> Self {
        make_header_name(s.into())
    }
}

impl From<String> for HeaderNameOwned {
    fn from(s: String) -> Self {
        HeaderName::from(s).into()
    }
}

impl<'n> From<Cow<'n, str>> for HeaderName<'n> {
    fn from(s: Cow<'n, str>) -> Self {
        make_header_name(s)
    }
}

impl<'n> From<Cow<'n, str>> for HeaderNameOwned {
    fn from(s: Cow<'n, str>) -> Self {
        HeaderName::from(s).into()
    }
}

impl<'n> From<HeaderName<'n>> for HeaderNameOwned {
    #[inline]
    fn from(s: HeaderName<'n>) -> Self {
        Self(s.0.into_owned())
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

impl fmt::Debug for HeaderNameOwned {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl fmt::Display for HeaderNameOwned {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

lazy_static! {
    static ref HEADER_NAME_TOKEN: HashSet<char> = {
        let mut set = HashSet::with_capacity(127);
        set.insert('!');
        set.insert('#');
        set.insert('$');
        set.insert('%');
        set.insert('&');
        set.insert('\'');
        set.insert('*');
        set.insert('+');
        set.insert('-');
        set.insert('.');
        set.insert('0');
        set.insert('1');
        set.insert('2');
        set.insert('3');
        set.insert('4');
        set.insert('5');
        set.insert('6');
        set.insert('7');
        set.insert('8');
        set.insert('9');
        set.insert('A');
        set.insert('B');
        set.insert('C');
        set.insert('D');
        set.insert('E');
        set.insert('F');
        set.insert('G');
        set.insert('H');
        set.insert('I');
        set.insert('J');
        set.insert('K');
        set.insert('L');
        set.insert('M');
        set.insert('N');
        set.insert('O');
        set.insert('P');
        set.insert('Q');
        set.insert('R');
        set.insert('S');
        set.insert('T');
        set.insert('U');
        set.insert('W');
        set.insert('V');
        set.insert('X');
        set.insert('Y');
        set.insert('Z');
        set.insert('^');
        set.insert('_');
        set.insert('`');
        set.insert('a');
        set.insert('b');
        set.insert('c');
        set.insert('d');
        set.insert('e');
        set.insert('f');
        set.insert('g');
        set.insert('h');
        set.insert('i');
        set.insert('j');
        set.insert('k');
        set.insert('l');
        set.insert('m');
        set.insert('n');
        set.insert('o');
        set.insert('p');
        set.insert('q');
        set.insert('r');
        set.insert('s');
        set.insert('t');
        set.insert('u');
        set.insert('v');
        set.insert('w');
        set.insert('x');
        set.insert('y');
        set.insert('z');
        set.insert('|');
        set.insert('~');
        set
    };
}

/// HTTP Header 值
pub type HeaderValue<'v> = Cow<'v, str>;

/// HTTP Header 值
pub type HeaderValueOwned = String;

/// HTTP Header
pub type Headers<'h> = HashMap<HeaderName<'h>, HeaderValue<'h>>;

/// HTTP Header
pub type HeadersOwned = HashMap<HeaderNameOwned, HeaderValueOwned>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::{boxed::Box, error::Error, result::Result};

    #[test]
    fn test_header_name() -> Result<(), Box<dyn Error>> {
        let mut headers = Headers::new();
        headers.insert("Authorization".into(), "Test".into());
        headers.insert("X-Qiniu-aXXXX".into(), "Testa".into());
        headers.insert("X-Qiniu-Bxxxx".into(), "Testb".into());
        headers.insert("X-Qiniu-CXXXX".into(), "Testc".into());
        assert_eq!(headers.get(&"authorization".into()), Some(&"Test".into()));
        assert_eq!(headers.get(&"AUTHORIZATION".into()), Some(&"Test".into()));
        assert_eq!(headers.get(&"X-Qiniu-Axxxx".into()), Some(&"Testa".into()));
        assert_eq!(headers.get(&"X-Qiniu-Bxxxx".into()), Some(&"Testb".into()));
        assert_eq!(headers.get(&"X-Qiniu-Cxxxx".into()), Some(&"Testc".into()));
        assert_eq!(headers.get(&"X-Qiniu-cXXXX".into()), Some(&"Testc".into()));
        Ok(())
    }
}
