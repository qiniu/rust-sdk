use std::{fmt, ops::Deref};

pub enum Ron<'t, T: 't> {
    Referenced(&'t T),
    Owned(T),
}

impl<'t, T: 't> AsRef<T> for Ron<'t, T> {
    fn as_ref(&self) -> &T {
        match self {
            Ron::Referenced(r) => r,
            Ron::Owned(r) => &r,
        }
    }
}

impl<'t, T: 't> Deref for Ron<'t, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'t, T: fmt::Debug + 't> fmt::Debug for Ron<'t, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<'t, T: fmt::Display + 't> fmt::Display for Ron<'t, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}
