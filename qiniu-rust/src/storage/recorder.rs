use delegate::delegate;
use std::{
    borrow::Cow,
    env::temp_dir,
    fs::{self, File, OpenOptions},
    io::{Read, Result, Write},
    path::{Path, PathBuf},
};

pub trait Recorder: Clone {
    type Medium: RecordMedium;
    fn open<ID: AsRef<str>>(&self, id: ID, truncate: bool) -> Result<Self::Medium>;
    fn delete<ID: AsRef<str>>(&self, id: ID) -> Result<()>;
}

pub trait RecordMedium: Read + Write {}

#[derive(Clone)]
pub struct FileSystemRecorder<'r> {
    root_directory: Cow<'r, Path>,
}

pub struct FileRecorder {
    file: File,
}

impl<'r> FileSystemRecorder<'r> {
    pub fn new<P: Into<Cow<'r, Path>>>(root_directory: P) -> FileSystemRecorder<'r> {
        FileSystemRecorder {
            root_directory: root_directory.into(),
        }
    }
}

impl Default for FileSystemRecorder<'_> {
    fn default() -> Self {
        Self::new(temp_dir())
    }
}

impl FileSystemRecorder<'_> {
    fn get_path<ID: AsRef<str>>(&self, id: ID) -> PathBuf {
        let mut path = self.root_directory.as_ref().to_owned();
        path.push(id.as_ref());
        path
    }
}

impl Recorder for FileSystemRecorder<'_> {
    type Medium = FileRecorder;
    fn open<ID: AsRef<str>>(&self, id: ID, truncate: bool) -> Result<FileRecorder> {
        let mut options = OpenOptions::new();
        options.create(true);
        if truncate {
            options.write(true).truncate(true);
        } else {
            options.read(true).write(true).append(true);
        }
        options.open(self.get_path(id)).map(|file| FileRecorder { file: file })
    }
    fn delete<ID: AsRef<str>>(&self, id: ID) -> Result<()> {
        fs::remove_file(self.get_path(id))
    }
}

impl Read for FileRecorder {
    delegate! {
        target self.file {
            fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
        }
    }
}

impl Write for FileRecorder {
    delegate! {
        target self.file {
            fn write(&mut self, buf: &[u8]) -> Result<usize>;
            fn flush(&mut self) -> Result<()>;
        }
    }
}

impl RecordMedium for FileRecorder {}
