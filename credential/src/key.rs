use serde::{
    de::{Deserialize, Deserializer, Error, Visitor},
    ser::{Serialize, Serializer},
};
use smallstr::SmallString;
use std::{borrow::Borrow, fmt, ops::Deref};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccessKey {
    inner: SmallString<[u8; 64]>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecretKey {
    inner: SmallString<[u8; 64]>,
}

macro_rules! impl_methods {
    ($name:ty) => {
        impl From<String> for $name {
            #[inline]
            fn from(key: String) -> Self {
                Self {
                    inner: SmallString::from_string(key),
                }
            }
        }

        impl<'a> From<&'a String> for $name {
            #[inline]
            fn from(key: &'a String) -> Self {
                Self {
                    inner: SmallString::from_str(key.as_str()),
                }
            }
        }

        impl From<Box<str>> for $name {
            #[inline]
            fn from(key: Box<str>) -> Self {
                Self {
                    inner: SmallString::from_string(key.into()),
                }
            }
        }

        impl<'a> From<&'a str> for $name {
            #[inline]
            fn from(key: &'a str) -> Self {
                Self {
                    inner: SmallString::from_str(key),
                }
            }
        }

        impl fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.inner.fmt(f)
            }
        }

        impl fmt::Debug for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.inner.fmt(f)
            }
        }

        impl AsRef<str> for $name {
            #[inline]
            fn as_ref(&self) -> &str {
                self.inner.as_ref()
            }
        }

        impl Borrow<str> for $name {
            #[inline]
            fn borrow(&self) -> &str {
                self.inner.borrow()
            }
        }

        impl Deref for $name {
            type Target = str;

            #[inline]
            fn deref(&self) -> &Self::Target {
                self.inner.deref()
            }
        }

        impl Serialize for $name {
            #[inline]
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.inner)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct KeyVisitor;

                impl<'de> Visitor<'de> for KeyVisitor {
                    type Value = $name;

                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("a string")
                    }

                    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                        Ok(v.into())
                    }

                    fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
                        Ok(v.into())
                    }
                }

                deserializer.deserialize_str(KeyVisitor)
            }
        }

        impl $name {
            #[inline]
            pub fn len(&self) -> usize {
                self.inner.len()
            }

            #[inline]
            pub fn is_empty(&self) -> bool {
                self.inner.is_empty()
            }

            #[inline]
            pub fn capacity(&self) -> usize {
                self.inner.capacity()
            }

            #[inline]
            pub fn as_str(&self) -> &str {
                self.inner.as_str()
            }

            #[inline]
            pub fn as_bytes(&self) -> &[u8] {
                self.inner.as_bytes()
            }
        }
    };
}

impl_methods!(AccessKey);
impl_methods!(SecretKey);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str as from_json_str, to_string as to_json_string};

    #[test]
    fn test_serialize() -> anyhow::Result<()> {
        let access_key = AccessKey::from("access_key");
        assert_eq!(to_json_string(&access_key)?, "\"access_key\"");
        Ok(())
    }

    #[test]
    fn test_deserialize() -> anyhow::Result<()> {
        let access_key: AccessKey = from_json_str("\"access_key\"")?;
        assert_eq!(access_key.as_str(), "access_key");
        Ok(())
    }
}
