use super::{
    form_uploader, resumeable_uploader,
    {super::upload_token, UploadResult},
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
pub struct BucketUploader<'b> {
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
}

impl<'b> BucketUploader<'b> {
    pub(super) fn new<B: Into<Cow<'b, str>>, U: Into<Box<[Box<[Box<str>]>]>>>(
        bucket_name: B,
        up_urls_list: U,
        credential: Credential,
        config: Config,
    ) -> BucketUploader<'b> {
        BucketUploader {
            bucket_name: bucket_name.into(),
            up_urls_list: up_urls_list.into(),
            client: Client::new(config.clone()),
            credential: credential,
            config: config,
        }
    }
}

impl Clone for BucketUploader<'_> {
    fn clone(&self) -> Self {
        BucketUploader {
            bucket_name: self.bucket_name.clone(),
            up_urls_list: self.up_urls_list.clone(),
            credential: self.credential.clone(),
            config: self.config.clone(),
            client: Client::new(self.config.clone()),
        }
    }
}

impl<'b> BucketUploader<'b> {
    pub fn upload_token<T: Into<upload_token::UploadToken<'b>>>(&'b self, upload_token: T) -> FileUploaderBuilder<'b> {
        FileUploaderBuilder::new(Cow::Borrowed(self), upload_token)
    }
}

pub enum ResumeablePolicy {
    Threshold(u64),
    Never,
    Always,
}

pub struct FileUploaderBuilder<'b> {
    bucket_uploader: Cow<'b, BucketUploader<'b>>,
    upload_token: upload_token::UploadToken<'b>,
    key: Option<Cow<'b, str>>,
    vars: Option<HashMap<Cow<'b, str>, Cow<'b, str>>>,
    metadata: Option<HashMap<Cow<'b, str>, Cow<'b, str>>>,
    checksum_enabled: bool,
    resumeable_policy: ResumeablePolicy,
}

impl<'b> FileUploaderBuilder<'b> {
    pub(super) fn new<B: Into<Cow<'b, BucketUploader<'b>>>, T: Into<upload_token::UploadToken<'b>>>(
        bucket_uploader: B,
        upload_token: T,
    ) -> FileUploaderBuilder<'b> {
        let bucket_uploader = bucket_uploader.into();
        FileUploaderBuilder {
            upload_token: upload_token.into(),
            key: None,
            vars: None,
            metadata: None,
            checksum_enabled: true,
            resumeable_policy: ResumeablePolicy::Threshold(bucket_uploader.config.upload_threshold()),
            bucket_uploader: bucket_uploader,
        }
    }

    pub fn key<K: Into<Cow<'b, str>>>(mut self, key: K) -> FileUploaderBuilder<'b> {
        self.key = Some(key.into());
        self
    }

    pub fn var<K: Into<Cow<'b, str>>, V: Into<Cow<'b, str>>>(mut self, key: K, value: V) -> FileUploaderBuilder<'b> {
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
    ) -> FileUploaderBuilder<'b> {
        if let Some(metadata) = &mut self.metadata {
            metadata.insert(key.into(), value.into());
        } else {
            let mut metadata = HashMap::with_capacity(1);
            metadata.insert(key.into(), value.into());
            self.metadata = Some(metadata);
        }
        self
    }

    pub fn disable_checksum(mut self) -> FileUploaderBuilder<'b> {
        self.checksum_enabled = false;
        self
    }

    pub fn enable_checksum(mut self) -> FileUploaderBuilder<'b> {
        self.checksum_enabled = true;
        self
    }

    pub fn upload_threshold(mut self, threshold: u64) -> FileUploaderBuilder<'b> {
        self.resumeable_policy = ResumeablePolicy::Threshold(threshold);
        self
    }

    pub fn always_be_resumeable(mut self) -> FileUploaderBuilder<'b> {
        self.resumeable_policy = ResumeablePolicy::Always;
        self
    }

    pub fn never_be_resumeable(mut self) -> FileUploaderBuilder<'b> {
        self.resumeable_policy = ResumeablePolicy::Never;
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
                if file_path.as_ref().metadata()?.len() >= threshold {
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
        let mut uploader = form_uploader::FormUploaderBuilder::new(&self.bucket_uploader, self.upload_token)?;
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
            resumeable_uploader::ResumeableUploaderBuilder::new(&self.bucket_uploader, self.upload_token)?;
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        if let Some(vars) = self.vars {
            uploader = uploader.vars(vars);
        }
        if let Some(metadata) = self.metadata {
            uploader = uploader.metadata(metadata);
        }
        Ok(uploader
            .seekable_stream(
                File::open(file_path.as_ref())?,
                Self::guess_filename(file_path.as_ref(), file_name),
                file_path.as_ref().metadata()?.len(),
                Self::guess_mime_from_file_path(mime, file_path.as_ref()),
                self.checksum_enabled,
            )?
            .send()?)
    }

    fn upload_stream_by_form<'n, R: Read, N: Into<Cow<'n, str>>>(
        self,
        stream: R,
        file_name: Option<N>,
        mime: Option<Mime>,
    ) -> Result<UploadResult> {
        let mut uploader = form_uploader::FormUploaderBuilder::new(&self.bucket_uploader, self.upload_token)?;
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
        let file_name = file_name.map(|name| name.into());
        Ok(uploader
            .stream(
                stream,
                Self::guess_mime_from_file_name(mime, file_name.as_ref().map(|name| name.as_ref())),
                file_name.unwrap_or_else(|| "streamName".into()),
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
            resumeable_uploader::ResumeableUploaderBuilder::new(&self.bucket_uploader, self.upload_token)?;
        if let Some(key) = self.key {
            uploader = uploader.key(key);
        }
        if let Some(vars) = self.vars {
            uploader = uploader.vars(vars);
        }
        if let Some(metadata) = self.metadata {
            uploader = uploader.metadata(metadata);
        }
        let file_name = file_name.map(|name| name.into());
        Ok(uploader
            .stream(
                stream,
                Self::guess_mime_from_file_name(mime, file_name.as_ref().map(|name| name.as_ref())),
                file_name.unwrap_or_else(|| "streamName".into()),
                true,
            )?
            .send()?)
    }

    fn guess_filename<'n, P: AsRef<Path>, N: Into<Cow<'n, str>>>(file_path: P, file_name: Option<N>) -> Cow<'n, str> {
        file_name
            .map(|name| name.into())
            .or_else(|| {
                file_path
                    .as_ref()
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.to_owned().into())
            })
            .unwrap_or_else(|| "fileName".into())
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
