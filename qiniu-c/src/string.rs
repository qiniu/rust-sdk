use std::{
    borrow::{Borrow, Cow, ToOwned},
    error::Error,
    ffi::{OsStr, OsString},
    fmt,
    ops::{Deref, Index, RangeFull},
    path::PathBuf,
    result::Result,
};

#[allow(non_camel_case_types, unused_imports, dead_code)]
mod unix {
    use super::*;
    use libc::c_char;
    use std::{
        ffi::{self, CStr, CString, FromBytesWithNulError},
        ptr::copy_nonoverlapping,
        str::Utf8Error,
    };

    pub type qiniu_ng_char_t = c_char;

    #[derive(Clone, PartialEq, Eq)]
    pub struct NulError(ffi::NulError);
    #[derive(Clone, PartialEq, Eq)]
    pub struct MissingNulError(FromBytesWithNulError);
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct EncodingError(Utf8Error);
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ucstr(CStr);
    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct UCString(CString);

    impl ucstr {
        pub unsafe fn from_ptr<'a>(ptr: *const qiniu_ng_char_t) -> &'a ucstr {
            CStr::from_ptr(ptr).into()
        }

        pub fn from_slice_with_nul(slice: &[u8]) -> Result<&ucstr, MissingNulError> {
            CStr::from_bytes_with_nul(slice).map(|s| s.into()).map_err(|e| e.into())
        }

        pub unsafe fn from_slice_with_nul_unchecked(slice: &[u8]) -> &ucstr {
            CStr::from_bytes_with_nul_unchecked(slice).into()
        }

        #[inline]
        pub fn as_ptr(&self) -> *const qiniu_ng_char_t {
            self.0.as_ptr()
        }

        #[inline]
        pub fn as_slice(&self) -> &[u8] {
            self.0.to_bytes()
        }

        #[inline]
        pub fn as_slice_with_nul(&self) -> &[u8] {
            self.0.to_bytes_with_nul()
        }

        pub fn to_string(&self) -> Result<String, EncodingError> {
            self.0.to_str().map(|s| s.to_owned()).map_err(|e| e.into())
        }

        pub fn to_string_lossy(&self) -> String {
            self.0.to_string_lossy().into_owned()
        }
    }

    impl UCString {
        pub fn new(slice: impl Into<Vec<u8>>) -> Result<Self, NulError> {
            CString::new(slice).map(|s| s.into()).map_err(|e| e.into())
        }

        pub unsafe fn from_vec_unchecked(slice: impl Into<Vec<u8>>) -> Self {
            CString::from_vec_unchecked(slice.into()).into()
        }

        #[inline]
        pub fn from_str(s: impl AsRef<str>) -> Result<Self, NulError> {
            Self::new(s.as_ref())
        }

        #[inline]
        pub unsafe fn from_str_unchecked(s: impl AsRef<str>) -> Self {
            Self::from_vec_unchecked(s.as_ref())
        }

        #[inline]
        pub fn from_string(s: impl Into<String>) -> Result<Self, NulError> {
            Self::new(s.into())
        }

        #[inline]
        pub unsafe fn from_string_unchecked(s: impl Into<String>) -> Self {
            Self::from_vec_unchecked(s.into())
        }

        #[cfg(not(windows))]
        #[inline]
        pub fn from_os_str(s: impl AsRef<OsStr>) -> Result<Self, NulError> {
            use std::os::unix::ffi::OsStrExt;
            Self::new(s.as_ref().as_bytes())
        }

        #[cfg(not(windows))]
        #[inline]
        pub unsafe fn from_os_str_unchecked(s: impl AsRef<OsStr>) -> Self {
            use std::os::unix::ffi::OsStrExt;
            Self::from_vec_unchecked(s.as_ref().as_bytes())
        }

        #[inline]
        pub fn as_ucstr(&self) -> &ucstr {
            self.0.as_c_str().into()
        }

        #[inline]
        pub fn into_vec(self) -> Vec<u8> {
            self.0.into_bytes()
        }

        #[inline]
        pub fn into_vec_with_nul(self) -> Vec<u8> {
            self.0.into_bytes_with_nul()
        }

        #[inline]
        pub fn into_raw(self) -> *mut qiniu_ng_char_t {
            self.0.into_raw()
        }

        #[inline]
        pub unsafe fn from_raw(ptr: *mut qiniu_ng_char_t) -> Self {
            CString::from_raw(ptr).into()
        }
    }

    impl<'a> From<&'a CStr> for &'a ucstr {
        fn from(s: &'a CStr) -> Self {
            unsafe { &*(s as *const CStr as *const ucstr) }
        }
    }

    impl<'a> From<&'a ucstr> for &'a CStr {
        fn from(s: &'a ucstr) -> Self {
            unsafe { &*(s as *const ucstr as *const CStr) }
        }
    }

    impl Deref for ucstr {
        type Target = CStr;

        #[inline]
        fn deref(&self) -> &CStr {
            self.into()
        }
    }

    impl<'a> Default for &'a ucstr {
        fn default() -> Self {
            <&CStr>::default().into()
        }
    }

    impl AsRef<[u8]> for ucstr {
        #[inline]
        fn as_ref(&self) -> &[u8] {
            self.as_slice()
        }
    }

    impl<'a> From<&'a ucstr> for Box<ucstr> {
        fn from(s: &'a ucstr) -> Self {
            Box::<CStr>::from(<&CStr>::from(s)).into()
        }
    }

    impl Default for Box<ucstr> {
        fn default() -> Self {
            Box::<CStr>::default().into()
        }
    }

    impl From<Box<CStr>> for Box<ucstr> {
        fn from(s: Box<CStr>) -> Self {
            unsafe { Box::from_raw(Box::into_raw(s) as *mut ucstr) }
        }
    }

    impl From<Box<ucstr>> for Box<CStr> {
        fn from(s: Box<ucstr>) -> Self {
            unsafe { Box::from_raw(Box::into_raw(s) as *mut CStr) }
        }
    }

    impl From<Box<ucstr>> for UCString {
        fn from(s: Box<ucstr>) -> Self {
            Box::<CStr>::from(s).into_c_string().into()
        }
    }

    impl From<UCString> for Box<ucstr> {
        #[inline]
        fn from(s: UCString) -> Self {
            s.0.into_boxed_c_str().into()
        }
    }

    #[cfg(not(windows))]
    impl From<&ucstr> for OsString {
        #[inline]
        fn from(s: &ucstr) -> Self {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(s.as_slice().into())
        }
    }

    impl fmt::Debug for ucstr {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl From<CString> for UCString {
        #[inline]
        fn from(s: CString) -> Self {
            Self(s)
        }
    }

    impl From<UCString> for CString {
        #[inline]
        fn from(s: UCString) -> Self {
            s.0
        }
    }

    #[cfg(not(windows))]
    impl From<OsString> for UCString {
        #[inline]
        fn from(s: OsString) -> Self {
            use std::os::unix::ffi::OsStringExt;
            unsafe { Self::from_vec_unchecked(s.into_vec()) }
        }
    }

    #[cfg(not(windows))]
    impl From<UCString> for OsString {
        #[inline]
        fn from(s: UCString) -> Self {
            use std::os::unix::ffi::OsStringExt;
            Self::from_vec(s.into_vec())
        }
    }

    impl fmt::Debug for UCString {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl From<FromBytesWithNulError> for MissingNulError {
        #[inline]
        fn from(e: FromBytesWithNulError) -> Self {
            Self(e)
        }
    }

    impl fmt::Debug for MissingNulError {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl fmt::Display for MissingNulError {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl Error for MissingNulError {
        #[inline]
        fn description(&self) -> &str {
            "missing terminating nul value"
        }
    }

    impl From<ffi::NulError> for NulError {
        #[inline]
        fn from(e: ffi::NulError) -> Self {
            Self(e)
        }
    }

    impl fmt::Debug for NulError {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl fmt::Display for NulError {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl Error for NulError {
        #[inline]
        fn description(&self) -> &str {
            self.0.description()
        }
    }

    impl From<Utf8Error> for EncodingError {
        #[inline]
        fn from(e: Utf8Error) -> Self {
            Self(e)
        }
    }

    impl fmt::Debug for EncodingError {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl fmt::Display for EncodingError {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl Error for EncodingError {
        #[inline]
        fn description(&self) -> &str {
            self.0.description()
        }
    }
}

#[allow(non_camel_case_types, unused_imports, dead_code)]
mod windows {
    use super::*;
    use libc::wchar_t;
    use std::string::FromUtf16Error;
    use widestring::{FromUtf32Error, WideCStr, WideCString, WideChar};

    pub type qiniu_ng_char_t = libc::wchar_t;
    #[derive(Clone, PartialEq, Eq)]
    pub struct NulError(widestring::NulError<WideChar>);
    #[derive(Clone, PartialEq, Eq)]
    pub struct MissingNulError(widestring::MissingNulError<WideChar>);
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct EncodingError;
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ucstr(WideCStr);
    #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct UCString(WideCString);

    impl ucstr {
        pub unsafe fn from_ptr<'a>(ptr: *const qiniu_ng_char_t) -> &'a ucstr {
            WideCStr::from_ptr_str(ptr.cast()).into()
        }

        pub fn from_slice_with_nul(slice: &[WideChar]) -> Result<&ucstr, MissingNulError> {
            WideCStr::from_slice_with_nul(slice)
                .map(|s| s.into())
                .map_err(|e| e.into())
        }

        pub unsafe fn from_slice_with_nul_unchecked(slice: &[WideChar]) -> &ucstr {
            WideCStr::from_slice_with_nul_unchecked(slice).into()
        }

        #[inline]
        pub fn as_ptr(&self) -> *const qiniu_ng_char_t {
            self.0.as_ptr().cast()
        }

        #[inline]
        pub fn as_slice(&self) -> &[WideChar] {
            self.0.as_slice()
        }

        #[inline]
        pub fn as_slice_with_nul(&self) -> &[WideChar] {
            self.0.as_slice_with_nul()
        }

        pub fn to_string(&self) -> Result<String, EncodingError> {
            self.0.to_string().map_err(|e| e.into())
        }

        pub fn to_string_lossy(&self) -> String {
            self.0.to_string_lossy()
        }
    }

    impl UCString {
        pub fn new(slice: impl Into<Vec<WideChar>>) -> Result<Self, NulError> {
            WideCString::new(slice.into()).map(|s| s.into()).map_err(|e| e.into())
        }

        pub unsafe fn from_vec_unchecked(slice: impl Into<Vec<WideChar>>) -> Self {
            WideCString::from_vec_unchecked(slice.into()).into()
        }

        pub fn from_str(s: impl AsRef<str>) -> Result<Self, NulError> {
            WideCString::from_str(s.as_ref())
                .map(|s| s.into())
                .map_err(|e| e.into())
        }

        pub unsafe fn from_str_unchecked(s: impl AsRef<str>) -> Self {
            WideCString::from_str_unchecked(s.as_ref()).into()
        }

        pub fn from_string(s: impl Into<String>) -> Result<Self, NulError> {
            Self::from_str(s.into())
        }

        #[inline]
        pub unsafe fn from_string_unchecked(s: impl Into<String>) -> Self {
            Self::from_str_unchecked(s.into())
        }

        #[cfg(windows)]
        #[inline]
        pub fn from_os_str(s: impl AsRef<OsStr>) -> Result<Self, NulError> {
            WideCString::from_os_str(s.as_ref())
                .map(|s| s.into())
                .map_err(|e| e.into())
        }

        #[cfg(windows)]
        #[inline]
        pub unsafe fn from_os_str_unchecked(s: impl AsRef<OsStr>) -> Self {
            WideCString::from_os_str_unchecked(s.as_ref()).into()
        }

        #[inline]
        pub fn as_ucstr(&self) -> &ucstr {
            self.0.as_ucstr().into()
        }

        #[inline]
        pub fn into_vec(self) -> Vec<WideChar> {
            self.0.into_vec()
        }

        #[inline]
        pub fn into_vec_with_nul(self) -> Vec<WideChar> {
            self.0.into_vec_with_nul()
        }

        #[inline]
        pub fn into_raw(self) -> *mut qiniu_ng_char_t {
            self.0.into_raw().cast()
        }

        #[inline]
        pub unsafe fn from_raw(ptr: *mut qiniu_ng_char_t) -> Self {
            WideCString::from_raw(ptr.cast()).into()
        }
    }

    impl<'a> From<&'a WideCStr> for &'a ucstr {
        fn from(s: &'a WideCStr) -> Self {
            unsafe { &*(s as *const WideCStr as *const ucstr) }
        }
    }

    impl<'a> From<&'a ucstr> for &'a WideCStr {
        fn from(s: &'a ucstr) -> Self {
            unsafe { &*(s as *const ucstr as *const WideCStr) }
        }
    }

    impl Deref for ucstr {
        type Target = WideCStr;

        #[inline]
        fn deref(&self) -> &WideCStr {
            self.into()
        }
    }

    impl<'a> Default for &'a ucstr {
        fn default() -> Self {
            <&WideCStr>::default().into()
        }
    }

    impl AsRef<[WideChar]> for ucstr {
        #[inline]
        fn as_ref(&self) -> &[WideChar] {
            self.as_slice()
        }
    }

    impl<'a> From<&'a ucstr> for Box<ucstr> {
        fn from(s: &'a ucstr) -> Self {
            Box::<WideCStr>::from(<&WideCStr>::from(s)).into()
        }
    }

    impl Default for Box<ucstr> {
        fn default() -> Self {
            Box::<WideCStr>::default().into()
        }
    }

    impl From<Box<WideCStr>> for Box<ucstr> {
        fn from(s: Box<WideCStr>) -> Self {
            unsafe { Box::from_raw(Box::into_raw(s) as *mut ucstr) }
        }
    }

    impl From<Box<ucstr>> for Box<WideCStr> {
        fn from(s: Box<ucstr>) -> Self {
            unsafe { Box::from_raw(Box::into_raw(s) as *mut WideCStr) }
        }
    }

    impl From<Box<ucstr>> for UCString {
        fn from(s: Box<ucstr>) -> Self {
            Box::<WideCStr>::from(s).into_ucstring().into()
        }
    }

    impl From<UCString> for Box<ucstr> {
        #[inline]
        fn from(s: UCString) -> Self {
            s.0.into_boxed_ucstr().into()
        }
    }

    #[cfg(windows)]
    impl From<&ucstr> for OsString {
        #[inline]
        fn from(s: &ucstr) -> Self {
            s.0.to_os_string()
        }
    }

    impl fmt::Debug for ucstr {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl From<WideCString> for UCString {
        #[inline]
        fn from(s: WideCString) -> Self {
            Self(s)
        }
    }

    impl From<UCString> for WideCString {
        #[inline]
        fn from(s: UCString) -> Self {
            s.0
        }
    }

    #[cfg(windows)]
    impl From<OsString> for UCString {
        fn from(s: OsString) -> Self {
            use std::os::windows::ffi::OsStrExt;
            let s = s.encode_wide().collect::<Box<[_]>>();
            unsafe { WideCString::from_vec_unchecked(s) }.into()
        }
    }

    impl From<UCString> for OsString {
        fn from(s: UCString) -> Self {
            WideCString::from(s).into()
        }
    }

    impl fmt::Debug for UCString {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl From<widestring::MissingNulError<WideChar>> for MissingNulError {
        #[inline]
        fn from(e: widestring::MissingNulError<WideChar>) -> Self {
            Self(e)
        }
    }

    impl fmt::Debug for MissingNulError {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl fmt::Display for MissingNulError {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl Error for MissingNulError {
        #[inline]
        fn description(&self) -> &str {
            self.0.description()
        }
    }

    impl From<widestring::NulError<WideChar>> for NulError {
        #[inline]
        fn from(e: widestring::NulError<WideChar>) -> Self {
            Self(e)
        }
    }

    impl fmt::Debug for NulError {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl fmt::Display for NulError {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    impl Error for NulError {
        #[inline]
        fn description(&self) -> &str {
            self.0.description()
        }
    }

    impl From<FromUtf16Error> for EncodingError {
        #[inline]
        fn from(_: FromUtf16Error) -> Self {
            EncodingError
        }
    }

    impl From<FromUtf32Error> for EncodingError {
        #[inline]
        fn from(_: FromUtf32Error) -> Self {
            EncodingError
        }
    }

    impl fmt::Display for EncodingError {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "error converting to UTF-8")
        }
    }

    impl Error for EncodingError {
        #[inline]
        fn description(&self) -> &str {
            "error converting to UTF-8"
        }
    }
}

#[cfg(not(windows))]
pub use unix::*;
#[cfg(windows)]
pub use windows::*;

impl ucstr {
    #[inline]
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    #[inline]
    pub fn to_ucstring(&self) -> UCString {
        self.into()
    }

    #[inline]
    pub fn into_ucstring(self: Box<Self>) -> UCString {
        self.into()
    }

    #[inline]
    pub fn to_os_string(&self) -> OsString {
        self.into()
    }

    #[inline]
    pub fn to_path_buf(&self) -> PathBuf {
        self.into()
    }
}

impl UCString {
    #[inline]
    pub unsafe fn from_ptr(ptr: *const qiniu_ng_char_t) -> Self {
        Self::from_vec_unchecked(ucstr::from_ptr(ptr).as_slice())
    }

    #[inline]
    pub fn from_ucstr(s: impl AsRef<ucstr>) -> Result<Self, NulError> {
        Self::new(s.as_ref().as_slice())
    }

    #[inline]
    pub fn from_ucstr_unchecked(s: impl AsRef<ucstr>) -> Self {
        s.as_ref().to_ucstring()
    }

    #[inline]
    pub fn into_os_string(self) -> OsString {
        self.into()
    }

    #[inline]
    pub fn into_path_buf(self) -> PathBuf {
        self.into()
    }

    #[inline]
    pub fn into_boxed_ucstr(self) -> Box<ucstr> {
        self.into()
    }
}

impl AsRef<ucstr> for ucstr {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<ucstr> for UCString {
    #[inline]
    fn as_ref(&self) -> &ucstr {
        self.as_ucstr()
    }
}

impl Default for UCString {
    fn default() -> Self {
        let def: &ucstr = Default::default();
        def.to_owned()
    }
}

impl Deref for UCString {
    type Target = ucstr;

    #[inline]
    fn deref(&self) -> &ucstr {
        self.as_ucstr()
    }
}

impl From<&ucstr> for UCString {
    #[inline]
    fn from(s: &ucstr) -> Self {
        unsafe { UCString::from_vec_unchecked(s.as_slice()) }
    }
}

impl Borrow<ucstr> for UCString {
    #[inline]
    fn borrow(&self) -> &ucstr {
        self.as_ucstr()
    }
}

impl Clone for Box<ucstr> {
    #[inline]
    fn clone(&self) -> Self {
        UCString::from(self.as_ref()).into()
    }
}

impl ToOwned for ucstr {
    type Owned = UCString;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        self.into()
    }
}

impl<'a> From<&'a ucstr> for Cow<'a, ucstr> {
    #[inline]
    fn from(s: &'a ucstr) -> Self {
        Cow::Borrowed(s)
    }
}

impl<'a> From<UCString> for Cow<'a, ucstr> {
    #[inline]
    fn from(s: UCString) -> Self {
        Cow::Owned(s)
    }
}

impl<'a> From<&'a UCString> for Cow<'a, ucstr> {
    #[inline]
    fn from(s: &'a UCString) -> Self {
        Cow::Borrowed(s.as_ref())
    }
}

impl<'a> From<Cow<'a, ucstr>> for UCString {
    #[inline]
    fn from(s: Cow<'a, ucstr>) -> Self {
        s.into_owned()
    }
}

impl Index<RangeFull> for UCString {
    type Output = ucstr;

    #[inline]
    fn index(&self, _index: RangeFull) -> &Self::Output {
        self
    }
}

impl From<&ucstr> for PathBuf {
    #[inline]
    fn from(s: &ucstr) -> Self {
        OsString::from(s).into()
    }
}

impl From<PathBuf> for UCString {
    #[inline]
    fn from(s: PathBuf) -> Self {
        OsString::from(s).into()
    }
}

impl From<UCString> for PathBuf {
    #[inline]
    fn from(s: UCString) -> Self {
        OsString::from(s).into()
    }
}
