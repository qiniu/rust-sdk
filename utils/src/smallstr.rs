//! smallstr
//!
//! Implements SmallString, a String-like container for small strings

pub use smallstr::SmallString;

#[macro_export]
/// 创建一个新的数据结构，并将其作为 SmallString 的封装类型
macro_rules! wrap_smallstr {
    ($name:ty) => {
        impl From<String> for $name {
            #[inline]
            fn from(key: String) -> Self {
                Self {
                    inner: SmallString::from(key),
                }
            }
        }

        impl<'a> From<&'a String> for $name {
            #[inline]
            fn from(key: &'a String) -> Self {
                Self {
                    inner: SmallString::from(key.as_str()),
                }
            }
        }

        impl From<Box<str>> for $name {
            #[inline]
            fn from(key: Box<str>) -> Self {
                Self {
                    inner: SmallString::from(key),
                }
            }
        }

        impl<'a> From<&'a str> for $name {
            #[inline]
            fn from(key: &'a str) -> Self {
                Self {
                    inner: SmallString::from(key),
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    inner: SmallString::new(),
                }
            }
        }

        impl fmt::Display for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.inner, f)
            }
        }

        impl fmt::Debug for $name {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.inner, f)
            }
        }

        impl AsRef<str> for $name {
            #[inline]
            fn as_ref(&self) -> &str {
                self.inner.as_ref()
            }
        }

        impl AsMut<str> for $name {
            #[inline]
            fn as_mut(&mut self) -> &mut str {
                self.inner.as_mut()
            }
        }

        impl Borrow<str> for $name {
            #[inline]
            fn borrow(&self) -> &str {
                self.inner.borrow()
            }
        }

        impl BorrowMut<str> for $name {
            #[inline]
            fn borrow_mut(&mut self) -> &mut str {
                self.inner.borrow_mut()
            }
        }

        impl AsRef<[u8]> for $name {
            #[inline]
            fn as_ref(&self) -> &[u8] {
                self.inner.as_ref()
            }
        }

        impl Borrow<[u8]> for $name {
            #[inline]
            fn borrow(&self) -> &[u8] {
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

        impl DerefMut for $name {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.inner.deref_mut()
            }
        }

        impl Serialize for $name {
            #[inline]
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.inner)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            #[inline]
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct KeyVisitor;

                impl Visitor<'_> for KeyVisitor {
                    type Value = $name;

                    #[inline]
                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("a string")
                    }

                    #[inline]
                    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                        Ok(v.into())
                    }

                    #[inline]
                    fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
                        Ok(v.into())
                    }
                }

                deserializer.deserialize_str(KeyVisitor)
            }
        }

        impl Extend<char> for $name {
            #[inline]
            fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) {
                let iter = iter.into_iter();
                let (lo, _) = iter.size_hint();

                self.reserve(lo);

                for ch in iter {
                    self.push(ch);
                }
            }
        }

        impl<'a> Extend<&'a char> for $name {
            #[inline]
            fn extend<I: IntoIterator<Item = &'a char>>(&mut self, iter: I) {
                self.extend(iter.into_iter().cloned());
            }
        }

        impl<'a> Extend<Cow<'a, str>> for $name {
            #[inline]
            fn extend<I: IntoIterator<Item = Cow<'a, str>>>(&mut self, iter: I) {
                for s in iter {
                    self.push_str(&s);
                }
            }
        }

        impl<'a> Extend<&'a str> for $name {
            #[inline]
            fn extend<I: IntoIterator<Item = &'a str>>(&mut self, iter: I) {
                for s in iter {
                    self.push_str(s);
                }
            }
        }

        impl Extend<String> for $name {
            #[inline]
            fn extend<I: IntoIterator<Item = String>>(&mut self, iter: I) {
                for s in iter {
                    self.push_str(&s);
                }
            }
        }

        impl FromIterator<char> for $name {
            #[inline]
            fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
                let mut s = SmallString::new();
                s.extend(iter);
                Self { inner: s }
            }
        }

        impl<'a> FromIterator<&'a char> for $name {
            #[inline]
            fn from_iter<I: IntoIterator<Item = &'a char>>(iter: I) -> Self {
                let mut s = SmallString::new();
                s.extend(iter.into_iter().cloned());
                Self { inner: s }
            }
        }

        impl<'a> FromIterator<Cow<'a, str>> for $name {
            #[inline]
            fn from_iter<I: IntoIterator<Item = Cow<'a, str>>>(iter: I) -> Self {
                let mut s = SmallString::new();
                s.extend(iter);
                Self { inner: s }
            }
        }

        impl<'a> FromIterator<&'a str> for $name {
            #[inline]
            fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> Self {
                let mut s = SmallString::new();
                s.extend(iter);
                Self { inner: s }
            }
        }

        impl FromIterator<String> for $name {
            #[inline]
            fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
                let mut s = SmallString::new();
                s.extend(iter);
                Self { inner: s }
            }
        }

        impl $name {
            /// Construct an empty string.
            #[inline]
            pub fn new() -> Self {
                Self {
                    inner: SmallString::new(),
                }
            }

            /// Construct an empty string with enough capacity pre-allocated to store at least n bytes.
            ///
            /// Will create a heap allocation only if n is larger than the inline capacity.
            #[inline]
            pub fn with_capacity(n: usize) -> Self {
                Self {
                    inner: SmallString::with_capacity(n),
                }
            }

            /// Returns the length of this string, in bytes.
            #[inline]
            pub fn len(&self) -> usize {
                self.inner.len()
            }

            /// Returns `true` if this string is empty.
            #[inline]
            pub fn is_empty(&self) -> bool {
                self.inner.is_empty()
            }

            /// Returns the number of bytes this string can hold without reallocating.
            #[inline]
            pub fn capacity(&self) -> usize {
                self.inner.capacity()
            }

            /// Appends the given `char` to the end of this string.
            #[inline]
            pub fn push(&mut self, ch: char) {
                self.inner.push(ch)
            }

            /// Appends the given string slice to the end of this string.
            #[inline]
            pub fn push_str(&mut self, s: &str) {
                self.inner.push_str(s)
            }

            /// Removes the last character from this string and returns it.
            ///
            /// Returns `None` if the string is empty.
            #[inline]
            pub fn pop(&mut self) -> Option<char> {
                self.inner.pop()
            }

            /// Shorten the string, keeping the first len bytes.
            ///
            /// This does not reallocate. If you want to shrink the string’s capacity, use shrink_to_fit after truncating.
            ///
            /// # Panics
            ///
            /// If `len` does not lie on a `char` boundary.
            #[inline]
            pub fn truncate(&mut self, len: usize) {
                self.inner.truncate(len)
            }

            /// Extracts a string slice containing the entire string.
            #[inline]
            pub fn as_str(&self) -> &str {
                self.inner.as_str()
            }

            /// Extracts a string slice containing the entire string.
            #[inline]
            pub fn as_mut_str(&mut self) -> &mut str {
                self.inner.as_mut_str()
            }

            /// Removes all contents of the string.
            #[inline]
            pub fn clear(&mut self) {
                self.inner.clear()
            }

            /// Removes a `char` from this string at a byte position and returns it.
            ///
            /// # Panics
            ///
            /// If `idx` does not lie on a `char` boundary.
            #[inline]
            pub fn remove(&mut self, idx: usize) -> char {
                self.inner.remove(idx)
            }

            /// Inserts a `char` into this string at the given byte position.
            ///
            /// # Panics
            ///
            /// If `idx` does not lie on `char` boundaries.
            #[inline]
            pub fn insert(&mut self, idx: usize, ch: char) {
                self.inner.insert(idx, ch)
            }

            /// Inserts a `&str` into this string at the given byte position.
            ///
            /// # Panics
            ///
            /// If `idx` does not lie on `char` boundaries.
            #[inline]
            pub fn insert_str(&mut self, idx: usize, s: &str) {
                self.inner.insert_str(idx, s)
            }

            /// Retains only the characters specified by the predicate.
            ///
            /// In other words, removes all characters `c` such that `f(c)` returns `false`.
            /// This method operates in place and preserves the order of retained
            /// characters.
            #[inline]
            pub fn retain<F: FnMut(char) -> bool>(&mut self, f: F) {
                self.inner.retain(f)
            }

            /// Ensures that this string's capacity is at least `additional` bytes larger
            /// than its length.
            ///
            /// The capacity may be increased by more than `additional` bytes in order to
            /// prevent frequent reallocations.
            #[inline]
            pub fn reserve(&mut self, additional: usize) {
                self.inner.reserve(additional)
            }

            /// Ensures that this string's capacity is `additional` bytes larger than
            /// its length.
            #[inline]
            pub fn reserve_exact(&mut self, additional: usize) {
                self.inner.reserve_exact(additional)
            }

            /// Shrink the capacity of the string as much as possible.
            ///
            /// When possible, this will move the data from an external heap buffer
            /// to the string's inline storage.
            #[inline]
            pub fn shrink_to_fit(&mut self) {
                self.inner.shrink_to_fit()
            }
        }

        impl Index<Range<usize>> for $name {
            type Output = str;

            #[inline]
            fn index(&self, index: Range<usize>) -> &str {
                &self.as_str()[index]
            }
        }

        impl IndexMut<Range<usize>> for $name {
            #[inline]
            fn index_mut(&mut self, index: Range<usize>) -> &mut str {
                &mut self.as_mut_str()[index]
            }
        }

        impl Index<RangeFrom<usize>> for $name {
            type Output = str;

            #[inline]
            fn index(&self, index: RangeFrom<usize>) -> &str {
                &self.as_str()[index]
            }
        }

        impl IndexMut<RangeFrom<usize>> for $name {
            #[inline]
            fn index_mut(&mut self, index: RangeFrom<usize>) -> &mut str {
                &mut self.as_mut_str()[index]
            }
        }

        impl Index<RangeTo<usize>> for $name {
            type Output = str;

            #[inline]
            fn index(&self, index: RangeTo<usize>) -> &str {
                &self.as_str()[index]
            }
        }

        impl IndexMut<RangeTo<usize>> for $name {
            #[inline]
            fn index_mut(&mut self, index: RangeTo<usize>) -> &mut str {
                &mut self.as_mut_str()[index]
            }
        }

        impl Index<RangeFull> for $name {
            type Output = str;

            #[inline]
            fn index(&self, index: RangeFull) -> &str {
                &self.as_str()[index]
            }
        }

        impl IndexMut<RangeFull> for $name {
            #[inline]
            fn index_mut(&mut self, index: RangeFull) -> &mut str {
                &mut self.as_mut_str()[index]
            }
        }

        impl std::fmt::Write for $name {
            #[inline]
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                self.inner.write_str(s)
            }

            #[inline]
            fn write_char(&mut self, c: char) -> std::fmt::Result {
                self.inner.write_char(c)
            }

            #[inline]
            fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::fmt::Result {
                self.inner.write_fmt(args)
            }
        }
    };
}
