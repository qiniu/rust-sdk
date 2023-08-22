use super::{
    super::{PartSize, SourceKey},
    async_reader::{AsyncDataSource, AsyncDataSourceReader},
    reader::first_part_number,
    seekable::SourceOffset,
};
use digest::Digest;
use futures::{
    future::{BoxFuture, FutureExt},
    lock::Mutex,
    AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, Future,
};
use qiniu_apis::http::AsyncReset;
use std::{
    fmt::{self, Debug},
    io::{Error as IoError, Result as IoResult, SeekFrom},
    num::NonZeroUsize,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering::SeqCst},
        Arc,
    },
    task::{ready, Context, Poll},
};

#[derive(Debug, Clone)]
pub(crate) struct AsyncSeekableDataSource {
    source: AsyncSeekableSource,
    current: Arc<Mutex<SourceOffset>>,
    size: u64,
    original_offset: u64,
}

impl AsyncSeekableDataSource {
    pub(crate) async fn new<R: AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin + 'static>(
        mut source: R,
        size: u64,
    ) -> Result<Self, (IoError, R)> {
        match source.stream_position().await {
            Ok(offset) => Ok(AsyncSeekableDataSource {
                original_offset: offset,
                size,
                source: AsyncSeekableSource::new(source, 0, 0),
                current: Arc::new(Mutex::new(SourceOffset::new(offset, first_part_number()))),
            }),
            Err(err) => Err((err, source)),
        }
    }
}

impl<A: Digest + Send> AsyncDataSource<A> for AsyncSeekableDataSource {
    fn slice(&self, size: PartSize) -> BoxFuture<IoResult<Option<AsyncDataSourceReader>>> {
        Box::pin(async move {
            let mut cur = self.current.lock().await;
            if cur.offset() < self.size {
                let size = size.as_u64();
                let source_reader = AsyncDataSourceReader::seekable(
                    cur.part_number(),
                    self.source.clone_with_new_offset_and_length(cur.offset(), size),
                );
                *cur.offset_mut() += size;
                *cur.part_number_mut() =
                    NonZeroUsize::new(cur.part_number().get() + 1).expect("Page number is too big");
                Ok(Some(source_reader))
            } else {
                Ok(None)
            }
        })
    }

    #[inline]
    fn reset(&self) -> BoxFuture<IoResult<()>> {
        Box::pin(async move {
            let mut cur = self.current.lock().await;
            *cur.offset_mut() = self.original_offset;
            *cur.part_number_mut() = first_part_number();
            Ok(())
        })
    }

    #[inline]
    fn source_key(&self) -> BoxFuture<IoResult<Option<SourceKey<A>>>> {
        Box::pin(async move { Ok(None) })
    }

    #[inline]
    fn total_size(&self) -> BoxFuture<IoResult<Option<u64>>> {
        Box::pin(async move { Ok(Some(self.size)) })
    }
}

/// 可异步寻址的数据源
///
/// 用于表示一个分片，需要传入可异步寻址的数据源，以及分片的起始位置和长度
#[derive(Debug)]
#[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
pub struct AsyncSeekableSource {
    source: Arc<Mutex<AsyncSeekableSourceInner<dyn ReadSeek>>>,
    source_offset: Arc<AtomicU64>,
    offset: u64,
    len: u64,
    step: AsyncSeekableSourceReadStep,
}

#[derive(Debug)]
struct AsyncSeekableSourceInner<T: AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin + ?Sized> {
    pos: Option<u64>,
    source: T,
}

impl<T: AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin> AsyncSeekableSourceInner<T> {
    fn new(source: T) -> Self {
        Self { source, pos: None }
    }
}

enum AsyncSeekableSourceReadStep {
    Buffered {
        buffer: Vec<u8>,
        consumed: usize,
    },
    Waiting {
        task: Pin<Box<dyn Future<Output = IoResult<Vec<u8>>> + Send + Sync + 'static>>,
    },
    Done,
}

impl Debug for AsyncSeekableSourceReadStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buffered { buffer, consumed } => f
                .debug_struct("Buffered")
                .field("buffer", buffer)
                .field("consumed", consumed)
                .finish(),
            Self::Waiting { .. } => f.debug_struct("Waiting").finish(),
            Self::Done => write!(f, "Done"),
        }
    }
}

impl Default for AsyncSeekableSourceReadStep {
    #[inline]
    fn default() -> Self {
        Self::Buffered {
            buffer: Vec::new(),
            consumed: 0,
        }
    }
}

impl AsyncSeekableSource {
    /// 创建可异步寻址的数据源
    ///
    /// 需要传入可异步寻址的数据源，以及分片的起始位置和长度
    #[inline]
    pub fn new(
        source: impl AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin + 'static,
        offset: u64,
        len: u64,
    ) -> Self {
        Self {
            step: Default::default(),
            source: Arc::new(Mutex::new(AsyncSeekableSourceInner::new(source))),
            source_offset: Arc::new(AtomicU64::new(0)),
            offset,
            len,
        }
    }

    pub(in super::super) fn clone_with_new_offset_and_length(&self, offset: u64, len: u64) -> Self {
        Self {
            step: Default::default(),
            source: self.source.to_owned(),
            source_offset: Arc::new(AtomicU64::new(0)),
            offset,
            len,
        }
    }

    pub(in super::super) fn offset(&self) -> u64 {
        self.offset
    }

    pub(in super::super) async fn len(&self) -> IoResult<u64> {
        let mut locked = self.source.lock().await;
        let new_pos = locked.source.seek(SeekFrom::End(0)).await?;
        if Some(new_pos) != locked.pos {
            locked.pos = Some(new_pos);
        }
        Ok(self.len.min(new_pos - self.offset))
    }

    fn poll_from_task(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<IoResult<usize>> {
        match &mut self.step {
            AsyncSeekableSourceReadStep::Waiting { task } => {
                let buffer = ready!(task.poll_unpin(cx))?;
                self.step = if buffer.is_empty() {
                    AsyncSeekableSourceReadStep::Done
                } else {
                    AsyncSeekableSourceReadStep::Buffered { buffer, consumed: 0 }
                };
                self.poll_read(cx, buf)
            }
            _ => unreachable!(),
        }
    }

    fn poll_from_buffer(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<IoResult<usize>> {
        match &mut self.step {
            AsyncSeekableSourceReadStep::Buffered { buffer, consumed } => {
                let rested = buf.len().min(buffer.len() - *consumed);
                if rested > 0 {
                    buf[..rested].copy_from_slice(&buffer[*consumed..(*consumed + rested)]);
                    *consumed += rested;
                    Poll::Ready(Ok(rested))
                } else {
                    let buffer_request_size = buf.len().max(1 << 22);
                    let source = self.source.to_owned();
                    let source_offset = self.source_offset.to_owned();
                    let len = self.len;
                    let offset = self.offset;
                    self.step = AsyncSeekableSourceReadStep::Waiting {
                        task: Box::pin(async move {
                            let mut locked = source.lock().await;
                            let source_offset_value = source_offset.load(SeqCst);
                            let max_read = len - source_offset_value;
                            if max_read == 0 {
                                Ok(Vec::new())
                            } else {
                                let max_read: usize = max_read.try_into().unwrap_or(usize::MAX);
                                let mut buffer = vec![0u8; buffer_request_size.min(max_read)];
                                let seek_pos = offset + source_offset_value;
                                if Some(seek_pos) != locked.pos {
                                    locked.pos = Some(locked.source.seek(SeekFrom::Start(seek_pos)).await?);
                                }
                                let have_read = locked.source.read(&mut buffer).await?;
                                buffer.truncate(have_read);
                                let have_read = have_read as u64;
                                source_offset.fetch_add(have_read, SeqCst);
                                if let Some(ref mut pos) = locked.pos {
                                    *pos += have_read;
                                }
                                Ok(buffer)
                            }
                        }),
                    };
                    self.poll_read(cx, buf)
                }
            }
            _ => unreachable!(),
        }
    }

    fn poll_done(self: Pin<&mut Self>) -> Poll<IoResult<usize>> {
        match &self.step {
            AsyncSeekableSourceReadStep::Done => Poll::Ready(Ok(0)),
            _ => unreachable!(),
        }
    }
}

impl AsyncRead for AsyncSeekableSource {
    #[inline]
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<IoResult<usize>> {
        match self.step {
            AsyncSeekableSourceReadStep::Waiting { .. } => self.poll_from_task(cx, buf),
            AsyncSeekableSourceReadStep::Buffered { .. } => self.poll_from_buffer(cx, buf),
            AsyncSeekableSourceReadStep::Done => self.poll_done(),
        }
    }
}

impl AsyncReset for AsyncSeekableSource {
    #[inline]
    fn reset(&mut self) -> BoxFuture<IoResult<()>> {
        Box::pin(async move {
            self.step = Default::default();
            self.source_offset.store(0, SeqCst);
            Ok(())
        })
    }
}

impl Clone for AsyncSeekableSource {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            source_offset: self.source_offset.clone(),
            offset: self.offset,
            len: self.len,
            step: Default::default(),
        }
    }
}

trait ReadSeek: AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin {}
impl<T: AsyncRead + AsyncSeek + Debug + Send + Sync + Unpin> ReadSeek for T {}
