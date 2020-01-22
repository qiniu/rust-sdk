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

pub trait Recorder: Debug + Sync + Send {
    fn open(&self, id: &str, truncate: bool) -> Result<Arc<Mutex<dyn RecordMedium>>>;
    fn delete(&self, id: &str) -> Result<()>;
    fn as_downcastable(&self) -> &dyn Any;
}

pub trait RecordMedium: Read + Write + Send {}

#[derive(Clone, Debug)]
pub struct FileSystemRecorder {
    root_directory: Cow<'static, Path>,
}

impl FileSystemRecorder {
    pub fn default_root_directory() -> PathBuf {
        let mut default_path = cache_dir().unwrap_or_else(temp_dir);
        default_path.push("qiniu_sdk");
        default_path.push("records");
        create_dir_all(&default_path)
            .map(|_| default_path)
            .unwrap_or_else(|_| temp_dir())
    }

    pub fn from<P: Into<Cow<'static, Path>>>(root_directory: P) -> Arc<dyn Recorder> {
        let root_directory = root_directory.into();
        Arc::new(FileSystemRecorder { root_directory }) as Arc<dyn Recorder>
    }

    pub fn default() -> Arc<dyn Recorder> {
        FileSystemRecorder::from(Self::default_root_directory())
    }

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
