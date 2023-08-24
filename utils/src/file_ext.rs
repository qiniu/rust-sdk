use cfg_if::cfg_if;
use std::io::{Error as IoError, Result as IoResult};

/// Extension trait for `File` which provides locking methods.
#[cfg_attr(feature = "docs", doc(cfg(feature = "file_ext")))]
pub trait FileExt {
    /// Locks the file for shared usage, blocking if the file is currently
    /// locked exclusively.
    fn lock_shared(&self) -> IoResult<()>;

    /// Locks the file for exclusive usage, blocking if the file is currently
    /// locked.
    fn lock_exclusive(&self) -> IoResult<()>;

    /// Locks the file for shared usage, or returns a an error if the file is
    /// currently locked (see `lock_contended_error`).
    fn try_lock_shared(&self) -> IoResult<()>;

    /// Locks the file for shared usage, or returns a an error if the file is
    /// currently locked (see `lock_contended_error`).
    fn try_lock_exclusive(&self) -> IoResult<()>;

    /// Unlocks the file.
    fn unlock(&self) -> IoResult<()>;
}

cfg_if! {
    if #[cfg(unix)] {
        use rustix::{fd::{AsRawFd, BorrowedFd}, fs::{flock, FlockOperation}};

        #[allow(unsafe_code)]
        fn _unix_flock<F: AsRawFd>(file: &F, operation: FlockOperation) -> IoResult<()> {
            let borrowed_fd = unsafe { BorrowedFd::borrow_raw(file.as_raw_fd()) };
            match flock(borrowed_fd, operation) {
                Ok(_) => Ok(()),
                Err(e) => Err(IoError::from_raw_os_error(e.raw_os_error())),
            }
        }

        impl<F: AsRawFd> FileExt for F {
            #[inline]
            fn lock_shared(&self) -> IoResult<()> {
                _unix_flock(self, FlockOperation::LockShared)
            }

            #[inline]
            fn lock_exclusive(&self) -> IoResult<()> {
                _unix_flock(self, FlockOperation::LockExclusive)
            }

            #[inline]
            fn try_lock_shared(&self) -> IoResult<()> {
                _unix_flock(self, FlockOperation::NonBlockingLockShared)
            }

            #[inline]
            fn try_lock_exclusive(&self) -> IoResult<()> {
                _unix_flock(self, FlockOperation::NonBlockingLockExclusive)
            }

            #[inline]
            fn unlock(&self) -> IoResult<()> {
                _unix_flock(self, FlockOperation::Unlock)
            }
        }
    }
}

cfg_if! {
    if #[cfg(windows)] {
        use std::{mem, os::windows::io::AsRawHandle};
        use windows_sys::Win32::Foundation::{HANDLE, FileSystem::{LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, UnlockFile, LockFileEx}};

        #[allow(unsafe_code)]
        fn _windows_lock_file<F: AsRawHandle>(file: &F, flags: u32) -> IoResult<()> {
            unsafe {
                let mut overlapped = mem::zeroed();
                let ret = LockFileEx(
                    file.as_raw_handle() as HANDLE,
                    flags,
                    0,
                    !0,
                    !0,
                    &mut overlapped,
                );
                if ret == 0 {
                    Err(Error::last_os_error())
                } else {
                    Ok(())
                }
            }
        }

        #[allow(unsafe_code)]
        fn _windows_unlock<F: AsRawHandle>(file: &F) -> IoResult<()> {
            unsafe {
                let ret = UnlockFile(file.as_raw_handle() as HANDLE, 0, 0, !0, !0);
                if ret == 0 {
                    Err(Error::last_os_error())
                } else {
                    Ok(())
                }
            }
        }

        impl<F: AsRawHandle> FileExt for F {
            #[inline]
            fn lock_shared(&self) -> IoResult<()> {
                _windows_lock_file(self, 0)
            }

            #[inline]
            fn lock_exclusive(&self) -> IoResult<()> {
                _windows_lock_file(self, LOCKFILE_EXCLUSIVE_LOCK)
            }

            #[inline]
            fn try_lock_shared(&self) -> IoResult<()> {
                _windows_lock_file(self, LOCKFILE_FAIL_IMMEDIATELY)
            }

            #[inline]
            fn try_lock_exclusive(&self) -> IoResult<()> {
                _windows_lock_file(self, LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY)
            }

            #[inline]
            fn unlock(&self) -> IoResult<()> {
                _windows_unlock(self)
            }
        }
    }
}
