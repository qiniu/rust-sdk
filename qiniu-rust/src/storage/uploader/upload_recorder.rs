use super::super::recorder::{FileSystemRecorder, RecordMedium, Recorder};
use assert_impl::assert_impl;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    io::{BufRead, BufReader, Error, ErrorKind, Result},
    path::Path,
    sync::Arc,
    sync::Mutex,
    time::{Duration, SystemTime},
};

#[derive(Builder, Clone)]
#[builder(pattern = "owned", public, build_fn(name = "inner_build", private))]
pub struct UploadRecorder {
    #[builder(default = "default::recorder()")]
    recorder: Arc<dyn Recorder>,

    #[builder(default = "default::key_generator")]
    key_generator: fn(name: &str, path: &Path, key: Option<&str>) -> String,

    #[builder(default = "default::upload_block_lifetime()")]
    upload_block_lifetime: Duration,

    #[builder(default = "default::always_flush_records()")]
    always_flush_records: bool,
}

pub(super) struct FileUploadRecordMedium {
    medium: Arc<Mutex<dyn RecordMedium>>,
    always_flush_records: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub(super) struct FileUploadRecordMediumMetadata {
    file_size: u64,
    modified_timestamp: u64,
    pub(super) upload_id: Box<str>,
    pub(super) up_urls: Box<[Box<str>]>,
}

#[derive(Serialize, Debug, Clone)]
struct SerializableFileUploadRecordMediumMetadata<'a> {
    file_size: u64,
    modified_timestamp: u64,
    upload_id: &'a str,
    up_urls: &'a [&'a str],
}

#[derive(Deserialize, Debug, Clone)]
pub(super) struct FileUploadRecordMediumBlockItem {
    pub(super) etag: Box<str>,
    pub(super) part_number: usize,
    created_timestamp: u64,
    pub(super) block_size: u32,
}

#[derive(Serialize, Debug, Clone)]
struct SerializableFileUploadRecordMediumBlockItem<'a> {
    etag: &'a str,
    part_number: usize,
    created_timestamp: u64,
    block_size: u32,
}

impl UploadRecorderBuilder {
    pub fn build(self) -> UploadRecorder {
        self.inner_build().unwrap()
    }
}

impl UploadRecorder {
    pub(super) fn open_and_write_metadata(
        &self,
        path: &Path,
        key: Option<&str>,
        upload_id: &str,
        up_urls: &[&str],
    ) -> Result<FileUploadRecordMedium> {
        let metadata = path.metadata()?;
        let metadata = SerializableFileUploadRecordMediumMetadata {
            file_size: metadata.len(),
            modified_timestamp: metadata
                .modified()?
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Incorrect last modification time in file metadata")
                .as_secs(),
            upload_id,
            up_urls,
        };
        let medium = self.recorder.open(&self.generate_key(path, key), true)?;
        {
            let mut medium = medium.lock().unwrap();
            let mut metadata = serde_json::to_string(&metadata).map_err(|err| Error::new(ErrorKind::Other, err))?;
            metadata.push_str("\n");
            medium.write_all(metadata.as_bytes())?;
            if self.always_flush_records {
                medium.flush()?;
            }
        }
        Ok(FileUploadRecordMedium {
            medium,
            always_flush_records: self.always_flush_records,
        })
    }

    pub(super) fn open_for_appending(&self, path: &Path, key: Option<&str>) -> Result<FileUploadRecordMedium> {
        Ok(FileUploadRecordMedium {
            medium: self.recorder.open(&self.generate_key(path, key), false)?,
            always_flush_records: self.always_flush_records,
        })
    }

    pub(super) fn drop(&self, path: &Path, key: Option<&str>) -> Result<()> {
        self.recorder.delete(&self.generate_key(path, key))
    }

    pub(super) fn load(
        &self,
        path: &Path,
        key: Option<&str>,
    ) -> Result<Option<(FileUploadRecordMediumMetadata, Box<[FileUploadRecordMediumBlockItem]>)>> {
        let file_metadata = path.metadata()?;
        let medium = self.recorder.open(&self.generate_key(path, key), false)?;
        let mut lock = medium.lock().unwrap();
        let mut reader = BufReader::new(&mut *lock);
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let metadata: FileUploadRecordMediumMetadata =
            serde_json::from_str(&line).map_err(|err| Error::new(ErrorKind::Other, err))?;
        if metadata.file_size != file_metadata.len()
            || metadata.modified_timestamp
                != file_metadata
                    .modified()?
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("Incorrect last modification time in file metadata")
                    .as_secs()
        {
            return Ok(None);
        }
        let mut block_items = Vec::<FileUploadRecordMediumBlockItem>::new();
        loop {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                return Ok(Some((metadata, block_items.into())));
            }
            let block_item: FileUploadRecordMediumBlockItem =
                serde_json::from_str(&line).map_err(|err| Error::new(ErrorKind::Other, err))?;
            if SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Clock may have gone backwards")
                .as_secs()
                > block_item.created_timestamp + self.upload_block_lifetime.as_secs()
            {
                return Ok(Some((metadata, block_items.into())));
            }
            // TODO: 验证文件内容与记录的 Etag 是否符合
            block_items.push(block_item);
        }
    }

    fn generate_key(&self, path: &Path, key: Option<&str>) -> String {
        (self.key_generator)("upload", path, key)
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}

impl fmt::Debug for UploadRecorder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("UploadRecorder")
            .field("recorder", &self.recorder)
            .field("upload_block_lifetime", &self.upload_block_lifetime)
            .field("always_flush_records", &self.always_flush_records)
            .finish()
    }
}

impl Default for UploadRecorder {
    fn default() -> Self {
        UploadRecorderBuilder::default().build()
    }
}

pub mod default {
    use super::*;
    use crypto::{digest::Digest, sha1::Sha1};

    pub fn recorder() -> Arc<dyn Recorder> {
        FileSystemRecorder::default()
    }

    pub fn upload_block_lifetime() -> Duration {
        Duration::from_secs(60 * 60 * 24 * 7)
    }

    pub fn always_flush_records() -> bool {
        false
    }

    pub fn key_generator(name: &str, path: &Path, key: Option<&str>) -> String {
        let mut sha1 = Sha1::new();
        if let Some(key) = key {
            sha1.input(key.as_bytes());
            sha1.input(b"_._");
        }
        sha1.input(name.as_bytes());
        sha1.input(b"_._");
        sha1.input(path.to_string_lossy().as_ref().as_bytes());
        sha1.result_str()
    }
}

impl FileUploadRecordMedium {
    pub(super) fn append(&self, etag: &str, part_number: usize, block_size: u32) -> Result<()> {
        let mut item = serde_json::to_string(&SerializableFileUploadRecordMediumBlockItem {
            etag,
            part_number,
            block_size,
            created_timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Clock may have gone backwards")
                .as_secs(),
        })
        .map_err(|err| Error::new(ErrorKind::Other, err))?;
        item.push_str("\n");
        let mut medium = self.medium.lock().unwrap();
        medium.write_all(item.as_bytes())?;
        if self.always_flush_records {
            medium.flush()?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn ignore() {
        assert_impl!(Send: Self);
        assert_impl!(Sync: Self);
    }
}
