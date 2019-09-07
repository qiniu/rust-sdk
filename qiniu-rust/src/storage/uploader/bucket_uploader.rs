use super::{
    form_uploader,
    {super::upload_token, UploadResult},
};
use crate::{config::Config, http::Client, utils::auth::Auth};
use error_chain::error_chain;
use getset::Getters;
use mime::Mime;
use std::{borrow::Cow, collections::HashMap, io, path::Path};

#[derive(Getters)]
pub struct BucketUploader<'b> {
    #[get = "pub(super)"]
    bucket_name: Cow<'b, str>,

    #[get = "pub(super)"]
    up_urls_list: Box<[Box<[Box<str>]>]>,

    #[get = "pub(super)"]
    auth: Auth,

    #[get = "pub(super)"]
    config: Config,

    #[get = "pub(super)"]
    client: Client,
}

impl<'b> BucketUploader<'b> {
    pub(super) fn new<B: Into<Cow<'b, str>>, U: Into<Box<[Box<[Box<str>]>]>>>(
        bucket_name: B,
        up_urls_list: U,
        auth: Auth,
        config: Config,
    ) -> BucketUploader<'b> {
        BucketUploader {
            bucket_name: bucket_name.into(),
            up_urls_list: up_urls_list.into(),
            client: Client::new(auth.clone(), config.clone()),
            auth: auth,
            config: config,
        }
    }
}

impl Clone for BucketUploader<'_> {
    fn clone(&self) -> Self {
        BucketUploader {
            bucket_name: self.bucket_name.clone(),
            up_urls_list: self.up_urls_list.clone(),
            auth: self.auth.clone(),
            config: self.config.clone(),
            client: Client::new(self.auth.clone(), self.config.clone()),
        }
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
                    self.chunked_upload(file_path, file_name, mime)
                } else {
                    self.form_upload(file_path, file_name, mime)
                }
            }
            ResumeablePolicy::Always => self.chunked_upload(file_path, file_name, mime),
            ResumeablePolicy::Never => self.form_upload(file_path, file_name, mime),
        }
    }

    fn form_upload<'n, P: AsRef<Path>, N: Into<Cow<'n, str>>>(
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
            .file_path(file_path, file_name, mime, self.checksum_enabled)?
            .send()?)
    }

    fn chunked_upload<'n, P: AsRef<Path>, N: Into<Cow<'n, str>>>(
        self,
        _file_path: P,
        _file_name: Option<N>,
        _mime: Option<Mime>,
    ) -> Result<UploadResult> {
        panic!("NOT IMPLETEMENT");
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
