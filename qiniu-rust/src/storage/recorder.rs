use crate::config::Config;
use std::{
    env::temp_dir,
    fs::{create_dir_all, remove_file, File, OpenOptions},
    io::{Read, Result, Write},
    path::{Path, PathBuf},
};

pub trait Recorder: Clone {
    type Medium: RecordMedium;
    fn open<ID: AsRef<str>>(&self, id: ID, truncate: bool) -> Result<Self::Medium>;
    fn delete<ID: AsRef<str>>(&self, id: ID) -> Result<()>;
}

pub trait RecordMedium: Read + Write + Send {}

#[derive(Clone)]
pub struct FileSystemRecorder {
    root_directory: Box<Path>,
}

impl FileSystemRecorder {
    pub fn new<P: Into<Box<Path>>>(root_directory: P) -> FileSystemRecorder {
        FileSystemRecorder {
            root_directory: root_directory.into(),
        }
    }

    pub fn configure_by(config: &Config) -> Result<FileSystemRecorder> {
        let mut root_directory = config.records_dir().to_path_buf();
        root_directory.push("upload_records");
        create_dir_all(&root_directory)?;
        Ok(FileSystemRecorder {
            root_directory: root_directory.into(),
        })
    }
}

impl Default for FileSystemRecorder {
    fn default() -> Self {
        let mut temp_dir = temp_dir();
        temp_dir.push("upload_records");
        Self::new(temp_dir)
    }
}

impl FileSystemRecorder {
    fn get_path<ID: AsRef<str>>(&self, id: ID) -> PathBuf {
        let mut path = self.root_directory.as_ref().to_owned();
        path.push(id.as_ref());
        path
    }
}

impl Recorder for FileSystemRecorder {
    type Medium = File;
    fn open<ID: AsRef<str>>(&self, id: ID, truncate: bool) -> Result<Self::Medium> {
        let mut options = OpenOptions::new();
        options.create(true);
        if truncate {
            options.write(true).truncate(true);
        } else {
            options.read(true).append(true);
        }
        options.open(self.get_path(id))
    }
    fn delete<ID: AsRef<str>>(&self, id: ID) -> Result<()> {
        remove_file(self.get_path(id))
    }
}

impl RecordMedium for File {}
