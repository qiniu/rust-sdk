use async_compat::{Compat, CompatExt};
use futures::{
    future::TryFutureExt, stream::Stream, AsyncRead as AsyncStdRead, AsyncSeek as AsyncStdSeek,
    AsyncWrite as AsyncStdWrite, FutureExt,
};
use std::{
    error::Error as StdError,
    ffi::OsString,
    fmt::{self, Debug, Display},
    fs::{FileType, Metadata},
    future::Future,
    io::{IoSliceMut, Result as IoResult, SeekFrom},
    ops::{Deref, DerefMut},
    os::fd::{AsRawFd, FromRawFd, RawFd},
    path::{Path, PathBuf},
    pin::Pin,
    task::{ready, Context, Poll},
    time::Duration,
};
use tokio::{fs, runtime::Runtime as TokioRuntime, sync, task, time};
use tokio_stream::wrappers::ReadDirStream;

/// A builder for opening files with configurable options.
///
/// Files can be opened in [`read`] and/or [`write`] mode.
///
/// The [`append`] option opens files in a special writing mode that moves the file cursor to the
/// end of file before every write operation.
///
/// It is also possible to [`truncate`] the file right after opening, to [`create`] a file if it
/// doesn't exist yet, or to always create a new file with [`create_new`].
///
/// This type is an async version of [`std::fs::OpenOptions`].
///
/// [`read`]: #method.read
/// [`write`]: #method.write
/// [`append`]: #method.append
/// [`truncate`]: #method.truncate
/// [`create`]: #method.create
/// [`create_new`]: #method.create_new
/// [`std::fs::OpenOptions`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html
#[derive(Clone, Debug, Default)]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub struct OpenOptions(fs::OpenOptions);

impl OpenOptions {
    /// Creates a blank set of options.
    ///
    /// All options are initially set to `false`.
    #[inline]
    pub fn new() -> Self {
        Self(fs::OpenOptions::new())
    }

    /// Configures the option for append mode.
    ///
    /// When set to `true`, this option means the file will be writable after opening and the file
    /// cursor will be moved to the end of file before every write operation.
    #[inline]
    pub fn append(&mut self, append: bool) -> &mut OpenOptions {
        self.0.append(append);
        self
    }

    /// Configures the option for creating a new file if it doesn't exist.
    ///
    /// When set to `true`, this option means a new file will be created if it doesn't exist.
    ///
    /// The file must be opened in [`write`] or [`append`] mode for file creation to work.
    ///
    /// [`write`]: #method.write
    /// [`append`]: #method.append
    #[inline]
    pub fn create(&mut self, create: bool) -> &mut OpenOptions {
        self.0.create(create);
        self
    }

    /// Configures the option for creating a new file or failing if it already exists.
    ///
    /// When set to `true`, this option means a new file will be created, or the open operation
    /// will fail if the file already exists.
    ///
    /// The file must be opened in [`write`] or [`append`] mode for file creation to work.
    ///
    /// [`write`]: #method.write
    /// [`append`]: #method.append
    #[inline]
    pub fn create_new(&mut self, create_new: bool) -> &mut OpenOptions {
        self.0.create_new(create_new);
        self
    }

    /// Configures the option for read mode.
    ///
    /// When set to `true`, this option means the file will be readable after opening.
    #[inline]
    pub fn read(&mut self, read: bool) -> &mut OpenOptions {
        self.0.read(read);
        self
    }

    /// Configures the option for write mode.
    ///
    /// When set to `true`, this option means the file will be writable after opening.
    ///
    /// If the file already exists, write calls on it will overwrite the previous contents without
    /// truncating it.
    #[inline]
    pub fn write(&mut self, write: bool) -> &mut OpenOptions {
        self.0.write(write);
        self
    }

    /// Configures the option for truncating the previous file.
    ///
    /// When set to `true`, the file will be truncated to the length of 0 bytes.
    ///
    /// The file must be opened in [`write`] or [`append`] mode for truncation to work.
    ///
    /// [`write`]: #method.write
    /// [`append`]: #method.append
    #[inline]
    pub fn truncate(&mut self, truncate: bool) -> &mut OpenOptions {
        self.0.truncate(truncate);
        self
    }

    /// Opens a file with the configured options.
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * The file does not exist and neither [`create`] nor [`create_new`] were set.
    /// * The file's parent directory does not exist.
    /// * The current process lacks permissions to open the file in the configured mode.
    /// * The file already exists and [`create_new`] was set.
    /// * Invalid combination of options was used, like [`truncate`] was set but [`write`] wasn't,
    ///   or none of [`read`], [`write`], and [`append`] modes was set.
    /// * An OS-level occurred, like too many files are open or the file name is too long.
    /// * Some other I/O error occurred.
    ///
    /// [`read`]: #method.read
    /// [`write`]: #method.write
    /// [`append`]: #method.append
    /// [`truncate`]: #method.truncate
    /// [`create`]: #method.create
    /// [`create_new`]: #method.create_new
    #[inline]
    pub fn open<'a, P: AsRef<Path> + 'a>(&'a mut self, path: P) -> impl Future<Output = IoResult<File>> + 'a {
        self.0.open(path).map_ok(|file| file.compat()).map_ok(File)
    }
}

/// An open file on the filesystem.
///
/// Depending on what options the file was opened with, this type can be used for reading and/or
/// writing.
///
/// Files are automatically closed when they get dropped and any errors detected on closing are
/// ignored. Use the [`sync_all`] method before dropping a file if such errors need to be handled.
///
/// This type is an async version of [`std::fs::File`].
///
/// [`sync_all`]: #method.sync_all
/// [`std::fs::File`]: https://doc.rust-lang.org/std/fs/struct.File.html
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub struct File(Compat<fs::File>);

impl File {
    /// Opens a file in read-only mode.
    ///
    /// See the [`OpenOptions::open`] function for more options.
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` does not point to an existing file.
    /// * The current process lacks permissions to read the file.
    /// * Some other I/O error occurred.
    ///
    /// For more details, see the list of errors documented by [`OpenOptions::open`].
    ///
    /// [`OpenOptions::open`]: struct.OpenOptions.html#method.open
    #[inline]
    pub async fn open<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        Ok(Self(fs::File::open(path.as_ref()).await?.compat()))
    }

    /// Synchronizes OS-internal buffered contents and metadata to disk.
    ///
    /// This function will ensure that all in-memory data reaches the filesystem.
    ///
    /// This can be used to handle errors that would otherwise only be caught when the file is
    /// closed. When a file is dropped, errors in synchronizing this in-memory data are ignored.
    #[inline]
    pub async fn sync_all(&self) -> IoResult<()> {
        self.0.get_ref().sync_all().await
    }

    /// Truncates or extends the underlying file, updating the size of this file to become size.
    ///
    /// If the size is less than the current file's size, then the file will be
    /// shrunk. If it is greater than the current file's size, then the file
    /// will be extended to size and have all of the intermediate data filled in
    /// with 0s.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file is not opened for
    /// writing.
    #[inline]
    pub async fn set_len(&self, size: u64) -> IoResult<()> {
        self.0.get_ref().set_len(size).await
    }

    /// Reads the file's metadata.
    #[inline]
    pub async fn metadata(&self) -> IoResult<Metadata> {
        self.0.get_ref().metadata().await
    }

    /// Opens a file in write-only mode.
    ///
    /// This function will create a file if it does not exist, and will truncate
    /// it if it does.
    ///
    /// See [`OpenOptions`] for more details.
    ///
    /// # Errors
    ///
    /// Results in an error if called from outside of the Tokio runtime or if
    /// the underlying [`create`] call results in an error.
    ///
    /// [`create`]: std::fs::File::create
    #[inline]
    pub async fn create(path: impl AsRef<Path>) -> IoResult<File> {
        fs::File::create(path.as_ref())
            .await
            .map(|file| file.compat())
            .map(File)
    }
}

impl AsyncStdRead for File {
    #[inline]
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<IoResult<usize>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }

    #[inline]
    fn poll_read_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<IoResult<usize>> {
        Pin::new(&mut self.0).poll_read_vectored(cx, bufs)
    }
}

impl AsyncStdWrite for File {
    #[inline]
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    #[inline]
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    #[inline]
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.0).poll_close(cx)
    }
}

impl AsyncStdSeek for File {
    #[inline]
    fn poll_seek(mut self: Pin<&mut Self>, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<IoResult<u64>> {
        Pin::new(&mut self.0).poll_seek(cx, pos)
    }
}

impl Debug for File {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.get_ref().fmt(f)
    }
}

impl AsRawFd for File {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.0.get_ref().as_raw_fd()
    }
}

impl FromRawFd for File {
    #[inline]
    #[allow(unsafe_code)]
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(fs::File::from_raw_fd(fd).compat())
    }
}

/// Creates a new directory and all of its parents if they are missing.
///
/// This function is an async version of [`std::fs::create_dir_all`].
///
/// [`std::fs::create_dir_all`]: https://doc.rust-lang.org/std/fs/fn.create_dir_all.html
///
/// # Errors
///
/// An error will be returned in the following situations:
///
/// * `path` already points to an existing file or directory.
/// * The current process lacks permissions to create the directory or its missing parents.
/// * Some other I/O error occurred.
#[inline]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub async fn create_dir_all<P: AsRef<Path>>(path: P) -> IoResult<()> {
    fs::create_dir_all(path.as_ref()).await
}

/// Reads metadata for a path.
///
/// This function will traverse symbolic links to read metadata for the target file or directory.
///
/// This function is an async version of [`std::fs::metadata`].
///
/// [`std::fs::metadata`]: https://doc.rust-lang.org/std/fs/fn.metadata.html
///
/// # Errors
///
/// An error will be returned in the following situations:
///
/// * `path` does not point to an existing file or directory.
/// * The current process lacks permissions to read metadata for the path.
/// * Some other I/O error occurred.
#[inline]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub async fn metadata<P: AsRef<Path>>(path: P) -> IoResult<Metadata> {
    fs::metadata(path.as_ref()).await
}

/// Removes a file.
///
/// This function is an async version of [`std::fs::remove_file`].
///
/// [`std::fs::remove_file`]: https://doc.rust-lang.org/std/fs/fn.remove_file.html
///
/// # Errors
///
/// An error will be returned in the following situations:
///
/// * `path` does not point to an existing file.
/// * The current process lacks permissions to remove the file.
/// * Some other I/O error occurred.
#[inline]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub async fn remove_file<P: AsRef<Path>>(path: P) -> IoResult<()> {
    let path = path.as_ref();
    fs::remove_file(path).await
}

/// Returns a stream of entries in a directory.
///
/// The stream yields items of type [`std::io::Result`]`<`[`DirEntry`]`>`. Note that I/O errors can
/// occur while reading from the stream.
///
/// This function is an async version of [`std::fs::read_dir`].
///
/// [`DirEntry`]: struct.DirEntry.html
/// [`std::io::Result`]: https://doc.rust-lang.org/std/io/type.Result.html
/// [`std::fs::read_dir`]: https://doc.rust-lang.org/std/fs/fn.read_dir.html
///
/// # Errors
///
/// An error will be returned in the following situations:
///
/// * `path` does not point to an existing directory.
/// * The current process lacks permissions to read the contents of the directory.
/// * Some other I/O error occurred.
#[inline]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub async fn read_dir<P: AsRef<Path>>(path: P) -> IoResult<ReadDir> {
    let path = path.as_ref();
    fs::read_dir(path).map_ok(ReadDirStream::new).map_ok(ReadDir).await
}

/// Returns the canonical, absolute form of a path with all intermediate
/// components normalized and symbolic links resolved.
///
/// This is an async version of [`std::fs::canonicalize`][std]
///
/// [std]: std::fs::canonicalize
#[inline]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub async fn canonicalize(path: impl AsRef<Path>) -> IoResult<PathBuf> {
    fs::canonicalize(path.as_ref()).await
}

/// A stream of entries in a directory.
///
/// This stream is returned by [`read_dir`] and yields items of type
/// [`std::io::Result`]`<`[`DirEntry`]`>`. Each [`DirEntry`] can then retrieve information like entry's
/// path or metadata.
///
/// This type is an async version of [`std::fs::ReadDir`].
///
/// [`read_dir`]: fn.read_dir.html
/// [`DirEntry`]: struct.DirEntry.html
/// [`std::io::Result`]: https://doc.rust-lang.org/std/io/type.Result.html
/// [`std::fs::ReadDir`]: https://doc.rust-lang.org/std/fs/struct.ReadDir.html
#[derive(Debug)]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub struct ReadDir(ReadDirStream);

impl Stream for ReadDir {
    type Item = IoResult<DirEntry>;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next_entry = ready!(Pin::new(&mut self.0).poll_next(cx));
        Poll::Ready(next_entry.map(|result| result.map(DirEntry)))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

/// An entry in a directory.
///
/// A stream of entries in a directory is returned by [`read_dir`].
///
/// This type is an async version of [`std::fs::DirEntry`].
///
/// [`read_dir`]: fn.read_dir.html
/// [`std::fs::DirEntry`]: https://doc.rust-lang.org/std/fs/struct.DirEntry.html
#[derive(Debug)]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub struct DirEntry(fs::DirEntry);

impl DirEntry {
    /// Returns the full path to this entry.
    ///
    /// The full path is created by joining the original path passed to [`read_dir`] with the name
    /// of this entry.
    ///
    /// [`read_dir`]: fn.read_dir.html
    #[inline]
    pub fn path(&self) -> PathBuf {
        self.0.path()
    }

    /// Reads the metadata for this entry.
    ///
    /// This function will traverse symbolic links to read the metadata.
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * This entry does not point to an existing file or directory anymore.
    /// * The current process lacks permissions to read the metadata.
    /// * Some other I/O error occurred.
    #[inline]
    pub async fn metadata(&self) -> IoResult<Metadata> {
        self.0.metadata().await
    }

    /// Reads the file type for this entry.
    ///
    /// This function will not traverse symbolic links if this entry points at one.
    ///
    /// If you want to read metadata with following symbolic links, use [`metadata`] instead.
    ///
    /// [`metadata`]: #method.metadata
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * This entry does not point to an existing file or directory anymore.
    /// * The current process lacks permissions to read this entry's metadata.
    /// * Some other I/O error occurred.
    #[inline]
    pub async fn file_type(&self) -> IoResult<FileType> {
        self.0.file_type().await
    }

    /// Returns the bare name of this entry without the leading path.
    #[inline]
    pub fn file_name(&self) -> OsString {
        self.0.file_name()
    }
}

/// A builder for creating directories with configurable options.
///
/// This type is an async version of [`std::fs::DirBuilder`].
///
/// [`std::fs::DirBuilder`]: https://doc.rust-lang.org/std/fs/struct.DirBuilder.html
#[derive(Debug, Default)]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub struct DirBuilder(fs::DirBuilder);

impl DirBuilder {
    /// Creates a blank set of options.
    ///
    /// The [`recursive`] option is initially set to `false`.
    ///
    /// [`recursive`]: #method.recursive
    #[inline]
    pub fn new() -> DirBuilder {
        Self(fs::DirBuilder::new())
    }

    /// Sets the option for recursive mode.
    ///
    /// When set to `true`, this option means all parent directories should be created recursively
    /// if they don't exist. Parents are created with the same permissions as the final directory.
    ///
    /// This option is initially set to `false`.
    #[inline]
    pub fn recursive(&mut self, recursive: bool) -> &mut Self {
        self.0.recursive(recursive);
        self
    }

    /// Creates a directory with the configured options.
    ///
    /// It is considered an error if the directory already exists unless recursive mode is enabled.
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` already points to an existing file or directory.
    /// * The current process lacks permissions to create the directory or its missing parents.
    /// * Some other I/O error occurred.
    #[inline]
    pub fn create<'a, P: AsRef<Path> + 'a>(&'a self, path: P) -> impl Future<Output = IoResult<()>> + 'a {
        self.0.create(path)
    }
}

/// Spawns a task.
///
/// This function is similar to [`std::thread::spawn`], except it spawns an asynchronous task.
///
/// [`std::thread::spawn`]: https://doc.rust-lang.org/std/thread/fn.spawn.html
#[inline]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub fn spawn<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    JoinHandle(task::spawn(future))
}

/// A handle that awaits the result of a task.
///
/// Dropping a [`JoinHandle`] will detach the task, meaning that there is no longer
/// a handle to the task and no way to `join` on it.
///
/// Created when a task is [spawned].
///
/// [spawned]: fn.spawn.html
#[derive(Debug)]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub struct JoinHandle<T>(task::JoinHandle<T>);

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(ready!(self.0.poll_unpin(cx)).map_err(JoinError))
    }
}

/// Task failed to execute to completion.
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub struct JoinError(task::JoinError);

impl Display for JoinError {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self, fmt)
    }
}

impl Debug for JoinError {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self, fmt)
    }
}

impl StdError for JoinError {}

/// Spawns a blocking task.
///
/// The task will be spawned onto a thread pool specifically dedicated to blocking tasks. This
/// is useful to prevent long-running synchronous operations from blocking the main futures
/// executor.
///
/// See also: [`task::block_on`], [`task::spawn`].
///
/// [`task::block_on`]: fn.block_on.html
/// [`task::spawn`]: fn.spawn.html
#[inline]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub fn spawn_blocking<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    JoinHandle(task::spawn_blocking(f))
}

/// Spawns a task and blocks the current thread on its result.
///
/// Calling this function is similar to [spawning] a thread and immediately [joining] it, except an
/// asynchronous task will be spawned.
///
/// See also: [`task::spawn_blocking`].
///
/// [`task::spawn_blocking`]: fn.spawn_blocking.html
///
/// [spawning]: https://doc.rust-lang.org/std/thread/fn.spawn.html
/// [joining]: https://doc.rust-lang.org/std/thread/struct.JoinHandle.html#method.join
#[inline]
#[cfg_attr(
    feature = "docs",
    doc(cfg(any(feature = "async_std_runtime", feature = "tokio_runtime")))
)]
pub fn block_on<F, T>(future: F) -> IoResult<T>
where
    F: Future<Output = T>,
{
    let rt = new_tokio_runtime()?;
    Ok(rt.block_on(future))
}

fn new_tokio_runtime() -> IoResult<TokioRuntime> {
    TokioRuntime::new()
}

/// Sleeps for the specified amount of time.
///
/// This function might sleep for slightly longer than the specified duration but never less.
///
/// This function is an async version of [`std::thread::sleep`].
///
/// [`std::thread::sleep`]: https://doc.rust-lang.org/std/thread/fn.sleep.html
#[inline]
#[cfg_attr(
    feature = "docs",
    doc(cfg(all(any(feature = "async_std_runtime", feature = "tokio_runtime"), feature = "macros")))
)]
pub async fn sleep(dur: Duration) {
    time::sleep(dur).await
}

/// An async reader-writer lock.
///
/// This type of lock allows multiple readers or one writer at any point in time.
///
/// The locking strategy is write-preferring, which means writers are never starved.
/// Releasing a write lock wakes the next blocked reader and the next blocked writer.
#[derive(Debug)]
#[cfg_attr(
    feature = "docs",
    doc(cfg(all(any(feature = "async_std_runtime", feature = "tokio_runtime"), feature = "macros")))
)]
pub struct RwLock<T: ?Sized>(sync::RwLock<T>);

impl<T> RwLock<T> {
    /// Creates a new reader-writer lock.
    #[inline]
    #[must_use]
    pub const fn new(t: T) -> Self {
        Self(sync::RwLock::const_new(t))
    }

    /// Unwraps the lock and returns the inner value.
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Acquires a read lock.
    ///
    /// Returns a guard that releases the lock when dropped.
    ///
    /// Note that attempts to acquire a read lock will block if there are also concurrent attempts
    /// to acquire a write lock.
    #[inline]
    #[must_use]
    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        RwLockReadGuard(self.0.read().await)
    }

    /// Acquires a write lock.
    ///
    /// Returns a guard that releases the lock when dropped.
    #[inline]
    #[must_use]
    pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
        RwLockWriteGuard(self.0.write().await)
    }

    /// Returns a mutable reference to the inner value.
    ///
    /// Since this call borrows the lock mutably, no actual locking takes place. The mutable borrow
    /// statically guarantees no locks exist.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.0.get_mut()
    }
}

impl<T> From<T> for RwLock<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl<T: Default + ?Sized> Default for RwLock<T> {
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

/// A guard that releases the read lock when dropped.
#[derive(Debug)]
#[cfg_attr(
    feature = "docs",
    doc(cfg(all(any(feature = "async_std_runtime", feature = "tokio_runtime"), feature = "macros")))
)]
pub struct RwLockReadGuard<'a, T: ?Sized>(sync::RwLockReadGuard<'a, T>);

impl<T: ?Sized> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

/// A guard that releases the write lock when dropped.
#[derive(Debug)]
#[cfg_attr(
    feature = "docs",
    doc(cfg(all(any(feature = "async_std_runtime", feature = "tokio_runtime"), feature = "macros")))
)]
pub struct RwLockWriteGuard<'a, T: ?Sized>(sync::RwLockWriteGuard<'a, T>);

impl<T: ?Sized> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
