use super::super::recorder::{RecordMedium, Recorder};
use crate::config::Config;
use assert_impl::assert_impl;
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Error, ErrorKind, Result},
    path::Path,
    sync::Arc,
    sync::Mutex,
    time::{Duration, SystemTime},
};

#[derive(Clone)]
pub(super) struct UploadRecorder {
    recorder: Arc<dyn Recorder>,
    key_generator: fn(name: &str, path: &Path, key: Option<&str>) -> String,
    upload_block_lifetime: Duration,
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
    pub(super) block_size: u64,
}

#[derive(Serialize, Debug, Clone)]
struct SerializableFileUploadRecordMediumBlockItem<'a> {
    etag: &'a str,
    part_number: usize,
    created_timestamp: u64,
    block_size: u64,
}

impl UploadRecorder {
    pub(super) fn new(recorder: Arc<dyn Recorder>, config: &Config) -> UploadRecorder {
        UploadRecorder {
            recorder,
            key_generator: config.recorder_key_generator(),
            upload_block_lifetime: config.upload_block_lifetime(),
            always_flush_records: config.always_flush_records(),
        }
    }

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

impl FileUploadRecordMedium {
    pub(super) fn append(&self, etag: &str, part_number: usize, block_size: u64) -> Result<()> {
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
