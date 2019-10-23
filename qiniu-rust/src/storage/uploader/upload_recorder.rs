use super::super::recorder::{RecordMedium, Recorder};
use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Error, ErrorKind, Result, Write},
    path::Path,
    sync::Mutex,
    time::{Duration, SystemTime},
};

#[derive(Clone)]
pub(super) struct UploadRecorder<R>
where
    R: Recorder,
{
    recorder: R,
    key_generator: fn(path: &Path, key: Option<&str>) -> String,
    upload_block_lifetime: Duration,
    always_flush_records: bool,
}

pub(super) struct FileUploadRecorder<M>
where
    M: RecordMedium,
{
    medium: Mutex<M>,
    always_flush_records: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub(super) struct FileRecord {
    file_size: u64,
    modified_timestamp: u64,
    pub(super) upload_id: Box<str>,
    pub(super) up_urls: Box<[Box<str>]>,
}

#[derive(Serialize, Debug, Clone)]
struct SerializableFileRecord<'a> {
    file_size: u64,
    modified_timestamp: u64,
    upload_id: &'a str,
    up_urls: &'a [&'a str],
}

#[derive(Deserialize, Debug, Clone)]
pub(super) struct FileBlockRecord {
    pub(super) etag: Box<str>,
    pub(super) part_number: usize,
    created_timestamp: u64,
    pub(super) block_size: u64,
}

#[derive(Serialize, Debug, Clone)]
struct SerializableFileBlockRecord<'a> {
    etag: &'a str,
    part_number: usize,
    created_timestamp: u64,
    block_size: u64,
}

impl<R> UploadRecorder<R>
where
    R: Recorder,
{
    pub(super) fn new(recorder: R, config: &Config) -> UploadRecorder<R> {
        UploadRecorder {
            recorder: recorder,
            key_generator: config.upload_file_recorder_key_generator(),
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
    ) -> Result<FileUploadRecorder<R::Medium>> {
        let metadata = path.metadata()?;
        let record = SerializableFileRecord {
            file_size: metadata.len(),
            modified_timestamp: metadata
                .modified()?
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Incorrect last modification time in file metadata")
                .as_secs(),
            upload_id: upload_id,
            up_urls: up_urls,
        };
        let mut medium = self.recorder.open((self.key_generator)(path, key), true)?;
        let mut record = serde_json::to_string(&record).map_err(|err| Error::new(ErrorKind::Other, err))?;
        record.push_str("\n");
        medium.write_all(record.as_bytes())?;
        if self.always_flush_records {
            medium.flush()?;
        }
        Ok(FileUploadRecorder {
            medium: Mutex::new(medium),
            always_flush_records: self.always_flush_records,
        })
    }

    pub(super) fn open_for_appending(&self, path: &Path, key: Option<&str>) -> Result<FileUploadRecorder<R::Medium>> {
        Ok(FileUploadRecorder {
            medium: Mutex::new(self.recorder.open((self.key_generator)(path, key), false)?),
            always_flush_records: self.always_flush_records,
        })
    }

    pub(super) fn drop_record(&self, path: &Path, key: Option<&str>) -> Result<()> {
        self.recorder.delete((self.key_generator)(path, key))
    }

    pub(super) fn load_record(
        &self,
        path: &Path,
        key: Option<&str>,
    ) -> Result<Option<(FileRecord, Box<[FileBlockRecord]>)>> {
        let metadata = path.metadata()?;
        let mut reader = BufReader::new(self.recorder.open((self.key_generator)(path, key), false)?);
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let record: FileRecord = serde_json::from_str(&line).map_err(|err| Error::new(ErrorKind::Other, err))?;
        if record.file_size != metadata.len()
            || record.modified_timestamp
                != metadata
                    .modified()?
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("Incorrect last modification time in file metadata")
                    .as_secs()
        {
            return Ok(None);
        }
        let mut block_records = Vec::<FileBlockRecord>::new();
        loop {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                return Ok(Some((record, block_records.into())));
            }
            let block_record: FileBlockRecord =
                serde_json::from_str(&line).map_err(|err| Error::new(ErrorKind::Other, err))?;
            if SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Clock may have gone backwards")
                .as_secs()
                > block_record.created_timestamp + self.upload_block_lifetime.as_secs()
            {
                return Ok(Some((record, block_records.into())));
            }
            block_records.push(block_record);
        }
    }
}

impl<M> FileUploadRecorder<M>
where
    M: RecordMedium,
{
    pub(super) fn append_record(&self, etag: &str, part_number: usize, block_size: u64) -> Result<()> {
        let mut record = serde_json::to_string(&SerializableFileBlockRecord {
            etag: etag,
            part_number: part_number,
            created_timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Clock may have gone backwards")
                .as_secs(),
            block_size: block_size,
        })
        .map_err(|err| Error::new(ErrorKind::Other, err))?;
        record.push_str("\n");
        let mut medium = self.medium.lock().unwrap();
        medium.write_all(record.as_bytes())?;
        if self.always_flush_records {
            medium.flush()?;
        }
        Ok(())
    }
}
