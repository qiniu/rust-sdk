use anyhow::Result;

#[inline]
pub(super) fn spawn<F>(task_name: String, f: F) -> Result<()>
where
    F: FnOnce(),
    F: Send + 'static,
{
    return _spawn(task_name, f);

    #[inline]
    #[cfg(not(feature = "async"))]
    fn _spawn<F>(task_name: String, f: F) -> Result<()>
    where
        F: FnOnce(),
        F: Send + 'static,
    {
        std::thread::Builder::new()
            .name(task_name)
            .spawn(f)
            .map(|_| ())
            .map_err(|err| err.into())
    }

    #[inline]
    #[cfg(feature = "async")]
    fn _spawn<F>(task_name: String, f: F) -> Result<()>
    where
        F: FnOnce(),
        F: Send + 'static,
    {
        async_std::task::Builder::new()
            .name(task_name)
            .spawn(async move { f() })
            .map(|_| ())
            .map_err(|err| err.into())
    }
}
