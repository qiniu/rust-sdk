use super::{
    form_uploader, resumeable_uploader, upload_recorder,
    {
        super::{recorder, upload_policy, upload_token},
        UploadResult,
    },
};
use crate::{config::Config, credential::Credential, http::Client};
use error_chain::error_chain;
use getset::Getters;
use mime::Mime;
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::{self, Read},
    path::Path,
};

#[derive(Getters)]
pub struct BucketUploader<'b, REC: recorder::Recorder> {
    #[get = "pub(super)"]
    bucket_name: Cow<'b, str>,

    #[get = "pub(super)"]
    up_urls_list: Box<[Box<[Box<str>]>]>,

    #[get = "pub(super)"]
    credential: Credential,

    #[get = "pub(super)"]
    config: Config,

    #[get = "pub(super)"]
    client: Client,

    #[get = "pub(super)"]
    recorder: upload_recorder::UploadRecorder<REC>,
}

impl<'b> BucketUploader<'b, recorder::FileSystemRecorder<'b>> {
    pub(super) fn new<B: Into<Cow<'b, str>>, U: Into<Box<[Box<[Box<str>]>]>>>(
        bucket_name: B,
        up_urls_list: U,
        credential: Credential,
        config: Config,
    ) -> BucketUploader<'b, recorder::FileSystemRecorder<'b>> {
        let uploader = BucketUploader {
            bucket_name: bucket_name.into(),
            up_urls_list: up_urls_list.into(),
            client: Client::new(config.clone()),
            credential: credential,
            recorder: upload_recorder::UploadRecorder::new(recorder::FileSystemRecorder::default(), &config),
            config: config,
        };
        assert!(!uploader.up_urls_list.is_empty());
        uploader
    }
    // TODO: ADD CUSTOMIZED RECORDER METHOD
}

impl<REC: recorder::Recorder> Clone for BucketUploader<'_, REC> {
    fn clone(&self) -> Self {
        BucketUploader {
            bucket_name: self.bucket_name.clone(),
            up_urls_list: self.up_urls_list.clone(),
            credential: self.credential.clone(),
            config: self.config.clone(),
            client: Client::new(self.config.clone()),
            recorder: self.recorder.clone(),
        }
    }
}

impl<'b, REC: recorder::Recorder> BucketUploader<'b, REC> {
    pub fn upload_token<T: Into<upload_token::UploadToken<'b>>>(
        &'b self,
        upload_token: T,
    ) -> FileUploaderBuilder<'b, REC> {
        FileUploaderBuilder::new(Cow::Borrowed(self), upload_token)
    }

    pub fn upload_policy(&'b self, upload_policy: upload_policy::UploadPolicy<'b>) -> FileUploaderBuilder<'b, REC> {
        FileUploaderBuilder::new(
            Cow::Borrowed(self),
            upload_token::UploadToken::from_policy(upload_policy, self.credential.clone()),
        )
    }
}

pub enum ResumeablePolicy {
    Threshold(u64),
    Never,
    Always,
}

// TODO: 加强 UploadToken 复用性，使 FileUploaderBuilder 的 upload_token 可以引用 BucketUploader 的属性

pub struct FileUploaderBuilder<'b, REC: recorder::Recorder> {
    bucket_uploader: Cow<'b, BucketUploader<'b, REC>>,
    upload_token: upload_token::UploadToken<'b>,
    key: Option<Cow<'b, str>>,
    vars: Option<HashMap<Cow<'b, str>, Cow<'b, str>>>,
    metadata: Option<HashMap<Cow<'b, str>, Cow<'b, str>>>,
    checksum_enabled: bool,
    resumeable_policy: ResumeablePolicy,
    on_uploading_progress: Option<&'b dyn Fn(usize, usize)>,
}

impl<'b, REC: recorder::Recorder> FileUploaderBuilder<'b, REC> {
    pub(super) fn new<B: Into<Cow<'b, BucketUploader<'b, REC>>>, T: Into<upload_token::UploadToken<'b>>>(
        bucket_uploader: B,
        upload_token: T,
    ) -> FileUploaderBuilder<'b, REC> {
        let bucket_uploader = bucket_uploader.into();
        FileUploaderBuilder {
            upload_token: upload_token.into(),
            key: None,
            vars: None,
            metadata: None,
            checksum_enabled: true,
            resumeable_policy: ResumeablePolicy::Threshold(bucket_uploader.config.upload_threshold()),
            bucket_uploader: bucket_uploader,
            on_uploading_progress: None,
        }
    }

    pub fn key<K: Into<Cow<'b, str>>>(mut self, key: K) -> FileUploaderBuilder<'b, REC> {
        self.key = Some(key.into());
        self
    }

    pub fn var<K: Into<Cow<'b, str>>, V: Into<Cow<'b, str>>>(
        mut self,
        key: K,
        value: V,
    ) -> FileUploaderBuilder<'b, REC> {
        if let Some(vars) = &mut self.vars {
            vars.insert(key.into(), value.into());
        } else {
            let mut vars = HashMap::with_capacity(1);
            vars.insert(key.into(), value.into());
            self.vars = Some(vars);
        }
        self
    }

    pub fn metadata<K: Into<Cow<'b, str>>, V: Into<Cow<'b, str>>>(
        mut self,
        key: K,
        value: V,
    ) -> FileUploaderBuilder<'b, REC> {
        if let Some(metadata) = &mut self.metadata {
            metadata.insert(key.into(), value.into());
        } else {
            let mut metadata = HashMap::with_capacity(1);
            metadata.insert(key.into(), value.into());
            self.metadata = Some(metadata);
        }
        self
    }

    pub fn disable_checksum(mut self) -> FileUploaderBuilder<'b, REC> {
        self.checksum_enabled = false;
        self
    }

    pub fn enable_checksum(mut self) -> FileUploaderBuilder<'b, REC> {
        self.checksum_enabled = true;
        self
    }

    pub fn upload_threshold(mut self, threshold: u64) -> FileUploaderBuilder<'b, REC> {
        self.resumeable_policy = ResumeablePolicy::Threshold(threshold);
        self
    }

    pub fn always_be_resumeable(mut self) -> FileUploaderBuilder<'b, REC> {
        self.resumeable_policy = ResumeablePolicy::Always;
        self
    }

    pub fn never_be_resumeable(mut self) -> FileUploaderBuilder<'b, REC> {
        self.resumeable_policy = ResumeablePolicy::Never;
        self
    }

    pub fn on_progress(mut self, callback: &'b dyn Fn(usize, usize)) -> FileUploaderBuilder<'b, REC> {
        self.on_uploading_progress = Some(callback);
        self
    }

    pub fn upload_file<'n, P: AsRef<Path>, N: Into<Cow<'n, str>>>(
        self,
        file_path: P,
        file_name: Option<N>,
        mime: Option<Mime>,
    ) -> Result<UploadResult> {
        match self.resumeable_policy {
            ResumeablePolicy::Threshold(threshold) => {
                if file_path.as_ref().metadata()?.len() > threshold {
                    self.upload_file_by_blocks(file_path, file_name, mime)
                } else {
                    self.upload_file_by_form(file_path, file_name, mime)
                }
            }
            ResumeablePolicy::Always => self.upload_file_by_blocks(file_path, file_name, mime),
            ResumeablePolicy::Never => self.upload_file_by_form(file_path, file_name, mime),
        }
    }

    pub fn upload_stream<'n, R: Read, N: Into<Cow<'n, str>>>(
        self,
        stream: R,
        file_name: Option<N>,
        mime: Option<Mime>,
    ) -> Result<UploadResult> {
        match self.resumeable_policy {
            ResumeablePolicy::Threshold(_) | ResumeablePolicy::Always => {
                self.upload_stream_by_blocks(stream, file_name, mime)
            }
            ResumeablePolicy::Never => self.upload_stream_by_form(stream, file_name, mime),
        }
    }

    fn upload_file_by_form<'n, P: AsRef<Path>, N: Into<Cow<'n, str>>>(
        self,
        file_path: P,
        file_name: Option<N>,
        mime: Option<Mime>,
    ) -> Result<UploadResult> {
        let mut uploader = form_uploader::FormUploaderBuilder::new(&self.bucket_uploader, &self.upload_token)?;
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        if let Some(vars) = self.vars {
            for (k, v) in vars.into_iter() {
                uploader = uploader.var(k, v);
            }
        }
        if let Some(metadata) = self.metadata {
            for (k, v) in metadata.into_iter() {
                uploader = uploader.metadata(k, v);
            }
        }
        if let Some(callback) = self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback);
        }
        Ok(uploader
            .seekable_stream(
                File::open(file_path.as_ref())?,
                Self::guess_filename(file_path.as_ref(), file_name),
                Self::guess_mime_from_file_path(mime, file_path.as_ref()),
                self.checksum_enabled,
            )?
            .send()?)
    }

    fn upload_file_by_blocks<'n, P: AsRef<Path>, N: Into<Cow<'n, str>>>(
        self,
        file_path: P,
        file_name: Option<N>,
        mime: Option<Mime>,
    ) -> Result<UploadResult> {
        let mut uploader =
            resumeable_uploader::ResumeableUploaderBuilder::new(&self.bucket_uploader, &self.upload_token)?;
        if let Some(key) = &self.key {
            uploader = uploader.key(key.to_owned());
        }
        if let Some(vars) = self.vars {
            uploader = uploader.vars(vars);
        }
        if let Some(metadata) = self.metadata {
            uploader = uploader.metadata(metadata);
        }
        if let Some(callback) = self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback);
        }
        let mut uploader = uploader.file(
            File::open(file_path.as_ref())?,
            file_path.as_ref(),
            Self::guess_filename(file_path.as_ref(), file_name),
            file_path.as_ref().metadata()?.len(),
            Self::guess_mime_from_file_path(mime, file_path.as_ref()),
            self.checksum_enabled,
        );
        Self::prepare_for_resuming(
            self.key.as_ref().map(|key| key.as_ref()),
            &self.bucket_uploader.recorder,
            &mut uploader,
            file_path.as_ref(),
        )?;
        Ok(uploader.send()?)
    }

    fn prepare_for_resuming(
        key: Option<&str>,
        recorder: &upload_recorder::UploadRecorder<REC>,
        uploader: &mut resumeable_uploader::ResumeableUploader<'_, File, REC>,
        file_path: &Path,
    ) -> Result<()> {
        if let Some((file_record, block_records)) = recorder.load_record(file_path, key)? {
            uploader.prepare_for_resuming(file_record, block_records, recorder.open_for_appending(file_path, key)?);
        }
        Ok(())
    }

    fn upload_stream_by_form<'n, R: Read, N: Into<Cow<'n, str>>>(
        self,
        stream: R,
        file_name: Option<N>,
        mime: Option<Mime>,
    ) -> Result<UploadResult> {
        let mut uploader = form_uploader::FormUploaderBuilder::new(&self.bucket_uploader, &self.upload_token)?;
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        if let Some(vars) = self.vars {
            for (k, v) in vars.into_iter() {
                uploader = uploader.var(k, v);
            }
        }
        if let Some(metadata) = self.metadata {
            for (k, v) in metadata.into_iter() {
                uploader = uploader.metadata(k, v);
            }
        }
        if let Some(callback) = self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback);
        }
        let file_name = file_name.map(|name| name.into());
        Ok(uploader
            .stream(
                stream,
                Self::guess_mime_from_file_name(mime, file_name.as_ref().map(|name| name.as_ref())),
                file_name,
                None,
            )?
            .send()?)
    }

    fn upload_stream_by_blocks<'n, R: Read, N: Into<Cow<'n, str>>>(
        self,
        stream: R,
        file_name: Option<N>,
        mime: Option<Mime>,
    ) -> Result<UploadResult> {
        let mut uploader =
            resumeable_uploader::ResumeableUploaderBuilder::new(&self.bucket_uploader, &self.upload_token)?;
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        if let Some(vars) = self.vars {
            uploader = uploader.vars(vars);
        }
        if let Some(metadata) = self.metadata {
            uploader = uploader.metadata(metadata);
        }
        if let Some(callback) = self.on_uploading_progress {
            uploader = uploader.on_uploading_progress(callback);
        }
        let file_name = file_name.map(|name| name.into());
        Ok(uploader
            .stream(
                stream,
                Self::guess_mime_from_file_name(mime, file_name.as_ref().map(|name| name.as_ref())),
                file_name,
                true,
            )
            .send()?)
    }

    fn guess_filename<'n, P: AsRef<Path>, N: Into<Cow<'n, str>>>(
        file_path: P,
        file_name: Option<N>,
    ) -> Option<Cow<'n, str>> {
        file_name.map(|name| name.into()).or_else(|| {
            file_path
                .as_ref()
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_owned().into())
        })
    }

    fn guess_mime_from_file_path<P: AsRef<Path>>(mime: Option<Mime>, file_path: P) -> Option<Mime> {
        mime.or_else(|| mime_guess::from_path(file_path).first())
    }

    fn guess_mime_from_file_name(mime: Option<Mime>, file_name: Option<&str>) -> Option<Mime> {
        mime.or_else(|| file_name.and_then(|file_name| mime_guess::from_path(file_name).first()))
    }
}

error_chain! {
    links {
        InvalidUploadToken(upload_token::Error, upload_token::ErrorKind);
    }

    foreign_links {
        IOError(io::Error);
        QiniuError(qiniu_http::Error);
    }
}
