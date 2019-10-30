use std::{
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
}

pub trait RecordMedium: Read + Write + Send {}

#[derive(Clone, Debug)]
pub struct FileSystemRecorder {
    root_directory: Box<Path>,
}

impl FileSystemRecorder {
    pub fn from<P: Into<Box<Path>>>(root_directory: P) -> Arc<dyn Recorder> {
        let root_directory = root_directory.into();
        Arc::new(FileSystemRecorder { root_directory }) as Arc<dyn Recorder>
    }

    pub fn default() -> Arc<dyn Recorder> {
        let mut default_path = temp_dir();
        default_path.push("qiniu_sdk");
        default_path.push("records");
        create_dir_all(&default_path).unwrap();
        FileSystemRecorder::from(default_path)
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
}

impl RecordMedium for File {}
