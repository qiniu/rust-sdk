//! 记录仪模块
//!
//! 提供上传日志记录仪的持久化相关特性和基于文件系统的持久化方案实现

use dirs::cache_dir;
use std::{
    any::Any,
    borrow::Cow,
    env::temp_dir,
    fmt::Debug,
    fs::{create_dir_all, remove_file, File, OpenOptions},
    io::{Read, Result, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// 记录仪特性
///
/// 提供上传日志记录仪的持久化相关特性
pub trait Recorder: Debug + Sync + Send {
    /// 用于打开一个记录仪介质
    ///
    /// # Arguments
    ///
    /// * `id` - 介质 ID
    /// * `truncate` - 是否在打开时清空介质数据
    fn open(&self, id: &str, truncate: bool) -> Result<Arc<Mutex<dyn RecordMedium>>>;

    /// 用于删除一个记录仪介质
    ///
    /// # Arguments
    ///
    /// * `id` - 介质 ID
    fn delete(&self, id: &str) -> Result<()>;

    /// 提供面向 `std::any::Any` 的类型转换
    fn as_downcastable(&self) -> &dyn Any;
}

/// 记录仪介质特性
///
/// 提供上传日志记录仪的持久化介质相关特性
pub trait RecordMedium: Read + Write + Send {}

/// 文件系统记录仪
///
/// 提供基于文件系统的记录仪特性实现
#[derive(Clone, Debug)]
pub struct FileSystemRecorder {
    root_directory: Cow<'static, Path>,
}

impl FileSystemRecorder {
    /// 文件系统记录仪默认目录
    ///
    /// 默认的文件系统记录仪目录规则如下：
    ///   1. 尝试在[操作系统特定的缓存目录](https://docs.rs/dirs/2.0.2/dirs/fn.cache_dir.html)下创建 `qiniu_sdk/records` 目录。
    ///   2. 如果成功，则使用 `qiniu_sdk/records` 目录。
    ///   3. 如果失败，则直接使用临时目录。
    pub fn default_root_directory() -> PathBuf {
        let mut default_path = cache_dir().unwrap_or_else(temp_dir);
        default_path.push("qiniu_sdk");
        default_path.push("records");
        create_dir_all(&default_path)
            .map(|_| default_path)
            .unwrap_or_else(|_| temp_dir())
    }

    /// 基于指定目录创建文件系统记录仪
    pub fn from(root_directory: impl Into<Cow<'static, Path>>) -> Arc<dyn Recorder> {
        let root_directory = root_directory.into();
        Arc::new(FileSystemRecorder { root_directory }) as Arc<dyn Recorder>
    }

    /// 基于默认的目录创建文件系统记录仪
    pub fn default() -> Arc<dyn Recorder> {
        FileSystemRecorder::from(Self::default_root_directory())
    }

    /// 文件系统记录仪目录
    pub fn root_directory(&self) -> &Path {
        self.root_directory.as_ref()
    }

    fn get_path<ID: AsRef<str>>(&self, id: ID) -> PathBuf {
        self.root_directory.join(id.as_ref())
    }
}

impl Recorder for FileSystemRecorder {
    fn open(&self, id: &str, truncate: bool) -> Result<Arc<Mutex<dyn RecordMedium>>> {
        let mut options = OpenOptions::new();
        options.create(true);
        if truncate {
            options.write(true).truncate(true);
        } else {
            options.read(true).append(true);
        }
        options
            .open(self.get_path(id))
            .map(|file| Arc::new(Mutex::new(file)) as Arc<Mutex<dyn RecordMedium>>)
    }

    fn delete(&self, id: &str) -> Result<()> {
        remove_file(self.get_path(id))
    }

    fn as_downcastable(&self) -> &dyn Any {
        self
    }
}

impl RecordMedium for File {}
