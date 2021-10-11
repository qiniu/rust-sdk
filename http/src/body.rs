use std::{
    default::Default,
    fmt::Debug,
    io::{Cursor, Read, Result as IOResult},
};

trait ReadDebug: Read + Debug + Send {}
impl<T: Read + Debug + Send> ReadDebug for T {}

/// HTTP 响应体
#[derive(Debug)]
pub struct Body(BodyInner);

#[derive(Debug)]
enum BodyInner {
    Reader(Box<dyn ReadDebug>),
    Bytes(Cursor<Vec<u8>>),
}

impl Body {
    #[inline]
    pub(super) fn from_reader(reader: impl Read + Debug + Send + 'static) -> Self {
        Self(BodyInner::Reader(Box::new(reader)))
    }

    #[inline]
    pub(super) fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(BodyInner::Bytes(Cursor::new(bytes)))
    }
}

impl Default for Body {
    #[inline]
    fn default() -> Self {
        Self::from_bytes(Default::default())
    }
}

impl Read for Body {
    fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
        match &mut self.0 {
            BodyInner::Reader(reader) => reader.read(buf),
            BodyInner::Bytes(bytes) => bytes.read(buf),
        }
    }
}

/// HTTP 响应体引用
#[derive(Debug)]
pub struct MaybeOwnedBody<'a>(MaybeOwnedBodyInner<'a>);

#[derive(Debug)]
enum MaybeOwnedBodyInner<'a> {
    ReaderRef(&'a mut dyn ReadDebug),
    BytesRef(Cursor<&'a [u8]>),
    Owned(Body),
}

impl<'a> MaybeOwnedBody<'a> {
    #[inline]
    pub(super) fn from_referenced_reader<T: Read + Debug + Send>(reader: &'a mut T) -> Self {
        Self(MaybeOwnedBodyInner::ReaderRef(reader))
    }

    #[inline]
    pub(super) fn from_referenced_bytes(bytes: &'a [u8]) -> Self {
        Self(MaybeOwnedBodyInner::BytesRef(Cursor::new(bytes)))
    }

    #[inline]
    pub(super) fn from_reader(reader: impl Read + Debug + Send + 'static) -> Self {
        Self(MaybeOwnedBodyInner::Owned(Body::from_reader(reader)))
    }

    #[inline]
    pub(super) fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(MaybeOwnedBodyInner::Owned(Body::from_bytes(bytes)))
    }
}

impl Default for MaybeOwnedBody<'_> {
    #[inline]
    fn default() -> Self {
        Self::from_bytes(Default::default())
    }
}

impl Read for MaybeOwnedBody<'_> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
        match &mut self.0 {
            MaybeOwnedBodyInner::ReaderRef(reader) => reader.read(buf),
            MaybeOwnedBodyInner::BytesRef(bytes) => bytes.read(buf),
            MaybeOwnedBodyInner::Owned(owned) => owned.read(buf),
        }
    }
}

#[cfg(feature = "async")]
mod async_body {
    use futures_lite::{
        io::{AsyncRead, Cursor, Result as IOResult},
        pin,
    };
    use std::{
        fmt::Debug,
        pin::Pin,
        task::{Context, Poll},
    };

    trait AsyncReadDebug: AsyncRead + Unpin + Debug + Send + Sync {}
    impl<T: AsyncRead + Unpin + Debug + Send + Sync> AsyncReadDebug for T {}

    /// 异步 HTTP 响应体
    #[derive(Debug)]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub struct AsyncBody(AsyncBodyInner);

    #[derive(Debug)]
    enum AsyncBodyInner {
        Reader(Box<dyn AsyncReadDebug>),
        Bytes(Cursor<Vec<u8>>),
    }

    impl AsyncBody {
        #[inline]
        pub(in super::super) fn from_reader(
            reader: impl AsyncRead + Unpin + Debug + Send + Sync + 'static,
        ) -> Self {
            Self(AsyncBodyInner::Reader(Box::new(reader)))
        }

        #[inline]
        pub(in super::super) fn from_bytes(bytes: Vec<u8>) -> Self {
            Self(AsyncBodyInner::Bytes(Cursor::new(bytes)))
        }
    }

    impl Default for AsyncBody {
        #[inline]
        fn default() -> Self {
            Self::from_bytes(Default::default())
        }
    }

    impl AsyncRead for AsyncBody {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut [u8],
        ) -> Poll<IOResult<usize>> {
            match &mut self.as_mut().0 {
                AsyncBodyInner::Reader(reader) => {
                    pin!(reader);
                    reader.poll_read(cx, buf)
                }
                AsyncBodyInner::Bytes(bytes) => {
                    pin!(bytes);
                    bytes.poll_read(cx, buf)
                }
            }
        }
    }

    /// 异步 HTTP 响应体
    #[derive(Debug)]
    #[cfg_attr(feature = "docs", doc(cfg(r#async)))]
    pub struct MaybeOwnedAsyncBody<'a>(MaybeOwnedAsyncBodyInner<'a>);

    #[derive(Debug)]
    enum MaybeOwnedAsyncBodyInner<'a> {
        ReaderRef(&'a mut dyn AsyncReadDebug),
        BytesRef(Cursor<&'a [u8]>),
        Owned(AsyncBody),
    }

    impl<'a> MaybeOwnedAsyncBody<'a> {
        #[inline]
        pub(super) fn from_referenced_reader<T: AsyncRead + Unpin + Debug + Send + Sync>(
            reader: &'a mut T,
        ) -> Self {
            Self(MaybeOwnedAsyncBodyInner::ReaderRef(reader))
        }

        #[inline]
        pub(super) fn from_referenced_bytes(bytes: &'a [u8]) -> Self {
            Self(MaybeOwnedAsyncBodyInner::BytesRef(Cursor::new(bytes)))
        }

        #[inline]
        pub(super) fn from_reader(
            reader: impl AsyncRead + Unpin + Debug + Send + Sync + 'static,
        ) -> Self {
            Self(MaybeOwnedAsyncBodyInner::Owned(AsyncBody::from_reader(
                reader,
            )))
        }

        #[inline]
        pub(super) fn from_bytes(bytes: Vec<u8>) -> Self {
            Self(MaybeOwnedAsyncBodyInner::Owned(AsyncBody::from_bytes(
                bytes,
            )))
        }
    }

    impl Default for MaybeOwnedAsyncBody<'_> {
        #[inline]
        fn default() -> Self {
            Self::from_bytes(Default::default())
        }
    }

    impl AsyncRead for MaybeOwnedAsyncBody<'_> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut [u8],
        ) -> Poll<IOResult<usize>> {
            match &mut self.as_mut().0 {
                MaybeOwnedAsyncBodyInner::ReaderRef(reader) => {
                    pin!(reader);
                    reader.poll_read(cx, buf)
                }
                MaybeOwnedAsyncBodyInner::BytesRef(bytes) => {
                    pin!(bytes);
                    bytes.poll_read(cx, buf)
                }
                MaybeOwnedAsyncBodyInner::Owned(owned) => {
                    pin!(owned);
                    owned.poll_read(cx, buf)
                }
            }
        }
    }
}

#[cfg(feature = "async")]
pub use async_body::*;
