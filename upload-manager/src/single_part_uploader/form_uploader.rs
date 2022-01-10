use super::{
    super::{callbacks::Callbacks, DataCheck, ObjectParams, UploadManager},
    SinglePartUploader,
};
use qiniu_apis::{
    credential::AccessKey,
    http::{ResponseErrorKind as HttpResponseErrorKind, ResponseParts, TransferProgressInfo},
    http_client::{
        ApiResult, CallbackResult, FileName, PartMetadata, RequestBuilderParts, ResponseError,
    },
    storage::put_object::sync_part::RequestBody as SyncRequestBody,
};
use qiniu_upload_token::{BucketName, UploadTokenProviderExt};
use serde_json::Value;
use std::{
    fmt::Debug,
    fs::File,
    io::{BufReader, Read, Result as IoResult, Seek, SeekFrom},
    path::Path,
};

#[cfg(feature = "async")]
use futures::{future::BoxFuture, AsyncRead};

#[derive(Debug)]
pub struct FormUploader {
    upload_manager: UploadManager,
    callbacks: Callbacks<'static>,
}

impl SinglePartUploader for FormUploader {
    #[inline]
    fn new(upload_manager: UploadManager) -> Self {
        Self {
            upload_manager,
            callbacks: Default::default(),
        }
    }

    #[inline]
    fn on_before_request<
        F: FnMut(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'static,
    >(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_before_request_callback(callback);
        self
    }

    #[inline]
    fn on_upload_progress<
        F: Fn(&TransferProgressInfo) -> CallbackResult + Send + Sync + 'static,
    >(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_upload_progress_callback(callback);
        self
    }

    #[inline]
    fn on_response_ok<F: FnMut(&mut ResponseParts) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_after_response_ok_callback(callback);
        self
    }

    #[inline]
    fn on_response_error<F: FnMut(&ResponseError) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks
            .insert_after_response_error_callback(callback);
        self
    }

    fn upload_path(&self, path: &Path, params: ObjectParams) -> ApiResult<Value> {
        let response = self
            .upload_manager
            .client()
            .storage()
            .put_object()
            .new_request(
                self.upload_manager
                    .queryer()
                    .query(self.access_key()?, self.bucket_name()?),
            )
            .call(self.make_request_body_from_path(path, params)?)?;
        Ok(response.into_body().into())
    }

    fn upload_reader<R: Read + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> ApiResult<Value> {
        let response = self
            .upload_manager
            .client()
            .storage()
            .put_object()
            .new_request(
                self.upload_manager
                    .queryer()
                    .query(self.access_key()?, self.bucket_name()?),
            )
            .call(self.make_request_body_from_reader(reader, params)?)?;
        Ok(response.into_body().into())
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_path(&self, path: &Path, params: ObjectParams) -> BoxFuture<ApiResult<Value>> {
        todo!()
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_reader<R: AsyncRead + Unpin + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>> {
        todo!()
    }
}

impl FormUploader {
    fn access_key(&self) -> ApiResult<AccessKey> {
        self.upload_manager
            .upload_token()
            .access_key(&Default::default())
            .map(|ak| ak.into())
            .map_err(|err| {
                ResponseError::new(HttpResponseErrorKind::InvalidRequestResponse.into(), err)
            })
    }

    fn bucket_name(&self) -> ApiResult<BucketName> {
        self.upload_manager
            .upload_token()
            .bucket_name(&Default::default())
            .map_err(|err| {
                ResponseError::new(HttpResponseErrorKind::InvalidRequestResponse.into(), err)
            })
    }

    fn make_request_body_from_path(
        &self,
        path: &Path,
        mut params: ObjectParams,
    ) -> IoResult<SyncRequestBody> {
        let mut file = File::open(path)?;
        if params.file_name().is_none() {
            *params.file_name_mut() = path
                .file_name()
                .map(Path::new)
                .map(|file_name| FileName::from(file_name.display().to_string()));
        }
        if matches!(
            params.extensions().get::<DataCheck<u32>>(),
            Some(DataCheck::AutoCheck)
        ) {
            let crc32 = crc32_of_reader(&mut file)?;
            file.seek(SeekFrom::Start(0))?;
            params.extensions_mut().insert(DataCheck::Const(crc32));
        }
        self.make_request_body_from_reader(file, params)
    }

    fn make_request_body_from_reader<R: Read + 'static>(
        &self,
        reader: R,
        mut params: ObjectParams,
    ) -> IoResult<SyncRequestBody> {
        let mut file_metadata = PartMetadata::default();
        if let Some(file_name) = params.file_name() {
            file_metadata = file_metadata.file_name(file_name);
        }
        if let Some(content_type) = params.take_content_type() {
            file_metadata = file_metadata.mime(content_type);
        }
        let mut request_body =
            SyncRequestBody::default().set_upload_token(self.upload_manager.upload_token())?;
        if let Some(object_name) = params.take_object_name() {
            request_body = request_body.set_object_name(object_name.to_string());
        }
        if let Some(DataCheck::Const(crc32)) = params.extensions().get::<DataCheck<u32>>() {
            request_body = request_body.set_crc_32(crc32.to_string());
        }
        for (key, value) in params.take_metadata().into_iter() {
            request_body = request_body.append_custom_data("x-qn-meta-".to_owned() + &key, value);
        }
        for (key, value) in params.take_custom_vars().into_iter() {
            request_body = request_body.append_custom_data("x:".to_owned() + &key, value);
        }
        request_body = request_body.set_file_as_reader(reader, file_metadata);
        Ok(request_body)
    }
}

fn crc32_of_reader(reader: &mut dyn Read) -> IoResult<u32> {
    let mut hasher = crc32fast::Hasher::new();
    let mut reader = BufReader::new(reader);
    let mut buf = [0u8; 1024];
    loop {
        let have_read = reader.read(&mut buf)?;
        if have_read == 0 {
            break;
        } else {
            hasher.update(&buf[..have_read]);
        }
    }
    Ok(hasher.finalize())
}
