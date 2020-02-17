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

/// 上传进度记录仪
///
/// 用于记录文件块上传进度，如果文件在上传期间发生错误，将可以在重试时避免再次上传已经成功上传的文件分块，实现断点续传
#[derive(Builder, Clone)]
#[builder(pattern = "mutable", public, build_fn(name = "inner_build", private))]
pub struct UploadRecorder {
    /// 设置记录仪
    #[builder(default = "default::recorder()")]
    recorder: Arc<dyn Recorder>,

    /// 记录仪 ID 生成器
    ///
    /// 默认将使用基于 SHA1 的策略生成 ID
    #[builder(default = "default::id_generator")]
    id_generator: fn(name: &str, path: &Path, key: Option<&str>) -> String,

    /// 文件分块有效期
    ///
    /// 对于超过有效期的分块，SDK 将重新上传，确保所有分块在创建文件时均有效。
    ///
    /// 默认为 7 天，这是七牛公有云默认的配置。对于私有云的情况，需要参照私有云的配置来设置。
    #[builder(default = "default::upload_block_lifetime()")]
    upload_block_lifetime: Duration,

    /// 始终刷新
    ///
    /// 当记录上传进度后，是否始终刷新 IO 确保数据已经被持久化，默认为否
    #[builder(default = "default::always_flush_records()")]
    always_flush_records: bool,
}

pub(super) struct FileUploadRecordMedium {
    medium: Arc<Mutex<dyn RecordMedium>>,
    always_flush_records: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub(super) struct FileUploadRecordMediumMetadata {
    pub(super) file_size: u64,
    pub(super) modified_timestamp: u64,
    pub(super) upload_id: Box<str>,
    pub(super) up_urls: Box<[Box<str>]>,
    pub(super) block_size: u32,
}

#[derive(Serialize, Debug, Clone)]
struct SerializableFileUploadRecordMediumMetadata<'a> {
    file_size: u64,
    modified_timestamp: u64,
    upload_id: &'a str,
    up_urls: &'a [&'a str],
    block_size: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub(super) struct FileUploadRecordMediumBlockItem {
    pub(super) etag: Box<str>,
    pub(super) part_number: usize,
    pub(super) created_timestamp: u64,
}

#[derive(Serialize, Debug, Clone)]
struct SerializableFileUploadRecordMediumBlockItem<'a> {
    etag: &'a str,
    part_number: usize,
    created_timestamp: u64,
}

impl UploadRecorderBuilder {
    pub fn build(&self) -> UploadRecorder {
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
        block_size: u32,
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
            block_size,
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
        (self.id_generator)("upload", path, key)
    }

    /// 获取记录仪的引用
    pub fn recorder(&self) -> &dyn Recorder {
        self.recorder.as_ref()
    }

    /// 获取文件分块的有效期
    pub fn upload_block_lifetime(&self) -> Duration {
        self.upload_block_lifetime
    }

    /// 是否总是刷新
    pub fn always_flush_records(&self) -> bool {
        self.always_flush_records
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

mod default {
    use super::*;
    use digest::{FixedOutput, Input};
    use sha1::Sha1;

    #[inline]
    pub fn recorder() -> Arc<dyn Recorder> {
        FileSystemRecorder::default()
    }

    #[inline]
    pub const fn upload_block_lifetime() -> Duration {
        Duration::from_secs(60 * 60 * 24 * 7)
    }

    #[inline]
    pub const fn always_flush_records() -> bool {
        false
    }

    pub fn id_generator(name: &str, path: &Path, key: Option<&str>) -> String {
        let mut sha1 = Sha1::default();
        if let Some(key) = key {
            sha1.input(key.as_bytes());
            sha1.input(b"_._");
        }
        sha1.input(name.as_bytes());
        sha1.input(b"_._");
        sha1.input(path.to_string_lossy().as_ref().as_bytes());
        hex::encode(sha1.fixed_result())
    }
}

impl FileUploadRecordMedium {
    pub(super) fn append(&self, etag: &str, part_number: usize) -> Result<()> {
        let mut item = serde_json::to_string(&SerializableFileUploadRecordMediumBlockItem {
            etag,
            part_number,
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
