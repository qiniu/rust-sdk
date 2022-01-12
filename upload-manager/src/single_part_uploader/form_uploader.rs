use super::{
    super::{
        callbacks::{Callbacks, UploaderWithCallbacks, UploadingProgressInfo},
        DataCheck, ObjectParams, UploadManager,
    },
    SinglePartUploader,
};
use qiniu_apis::{
    credential::AccessKey,
    http::{ResponseErrorKind as HttpResponseErrorKind, ResponseParts},
    http_client::{
        ApiResult, CallbackResult, EndpointsProvider, FileName, PartMetadata, RegionProvider,
        RegionProviderEndpoints, RequestBuilderParts, ResponseError,
    },
    storage::put_object::{sync_part::RequestBody as SyncRequestBody, SyncRequestBuilder},
};
use qiniu_upload_token::{BucketName, ObjectName};
use serde_json::Value;
use std::{
    fmt::Debug,
    fs::File,
    io::{BufReader, Read, Result as IoResult, Seek, SeekFrom},
    path::Path,
};

#[cfg(feature = "async")]
use {
    async_std::fs::File as AsyncFile,
    futures::{
        future::BoxFuture, io::BufReader as AsyncBufReader, AsyncRead, AsyncReadExt, AsyncSeekExt,
    },
    qiniu_apis::storage::put_object::{
        async_part::RequestBody as AsyncRequestBody, AsyncRequestBuilder,
    },
};

#[derive(Debug)]
pub struct FormUploader {
    upload_manager: UploadManager,
    callbacks: Callbacks<'static>,
}

impl UploaderWithCallbacks for FormUploader {
    #[inline]
    fn on_before_request<
        F: Fn(&mut RequestBuilderParts<'_>) -> CallbackResult + Send + Sync + 'static,
    >(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_before_request_callback(callback);
        self
    }

    #[inline]
    fn on_upload_progress<
        F: Fn(&UploadingProgressInfo) -> CallbackResult + Send + Sync + 'static,
    >(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_upload_progress_callback(callback);
        self
    }

    #[inline]
    fn on_response_ok<F: Fn(&mut ResponseParts) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_after_response_ok_callback(callback);
        self
    }

    #[inline]
    fn on_response_error<F: Fn(&ResponseError) -> CallbackResult + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks
            .insert_after_response_error_callback(callback);
        self
    }
}

impl SinglePartUploader for FormUploader {
    #[inline]
    fn new(upload_manager: UploadManager) -> Self {
        Self {
            upload_manager,
            callbacks: Default::default(),
        }
    }

    fn upload_path(&self, path: &Path, mut params: ObjectParams) -> ApiResult<Value> {
        self.upload(
            params.take_region_provider(),
            self.make_request_body_from_path(path, params)?,
        )
    }

    fn upload_reader<R: Read + 'static>(
        &self,
        reader: R,
        mut params: ObjectParams,
    ) -> ApiResult<Value> {
        self.upload(
            params.take_region_provider(),
            self.make_request_body_from_reader(reader, params)?,
        )
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_path<'a>(
        &'a self,
        path: &'a Path,
        mut params: ObjectParams,
    ) -> BoxFuture<'a, ApiResult<Value>> {
        Box::pin(async move {
            self.async_upload(
                params.take_region_provider(),
                self.make_async_request_body_from_path(path, params).await?,
            )
            .await
        })
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_reader<R: AsyncRead + Unpin + Send + Sync + 'static>(
        &self,
        reader: R,
        mut params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>> {
        Box::pin(async move {
            self.async_upload(
                params.take_region_provider(),
                self.make_async_request_body_from_async_reader(reader, params)
                    .await?,
            )
            .await
        })
    }
}

impl FormUploader {
    fn access_key(&self) -> ApiResult<AccessKey> {
        self.upload_manager.upload_token().access_key()
    }

    fn bucket_name(&self) -> ApiResult<BucketName> {
        self.upload_manager.upload_token().bucket_name()
    }

    #[cfg(feature = "async")]
    async fn async_access_key(&self) -> ApiResult<AccessKey> {
        self.upload_manager.upload_token().async_access_key().await
    }

    #[cfg(feature = "async")]
    async fn async_bucket_name(&self) -> ApiResult<BucketName> {
        self.upload_manager.upload_token().async_bucket_name().await
    }

    fn upload(
        &self,
        region_provider: Option<Box<dyn RegionProvider>>,
        body: SyncRequestBody,
    ) -> ApiResult<Value> {
        let put_object = self.upload_manager.client().storage().put_object();
        return if let Some(region_provider) = region_provider {
            _upload(
                self,
                put_object.new_request(RegionProviderEndpoints::new(region_provider)),
                body,
            )
        } else {
            let request = put_object.new_request(RegionProviderEndpoints::new(
                self.upload_manager
                    .queryer()
                    .query(self.access_key()?, self.bucket_name()?),
            ));
            _upload(self, request, body)
        };

        fn _upload<'a, E: EndpointsProvider + Clone + 'a>(
            form_uploader: &'a FormUploader,
            mut request: SyncRequestBuilder<'a, E>,
            body: SyncRequestBody,
        ) -> ApiResult<Value> {
            request.on_uploading_progress(|_, transfer| {
                form_uploader
                    .callbacks
                    .upload_progress(&UploadingProgressInfo::from(transfer))
            });
            if form_uploader
                .callbacks
                .before_request(request.parts_mut())
                .is_cancelled()
            {
                return Err(make_user_cancelled_error(
                    "Cancelled by on_before_request() callback",
                ));
            }
            let mut response_result = request.call(body);
            if form_uploader
                .callbacks
                .after_response(&mut response_result)
                .is_cancelled()
            {
                return Err(make_user_cancelled_error(
                    "Cancelled by on_after_response() callback",
                ));
            }
            Ok(response_result?.into_body().into())
        }
    }

    #[cfg(feature = "async")]
    async fn async_upload(
        &self,
        region_provider: Option<Box<dyn RegionProvider>>,
        body: AsyncRequestBody,
    ) -> ApiResult<Value> {
        let put_object = self.upload_manager.client().storage().put_object();
        return if let Some(region_provider) = region_provider {
            _async_upload(
                self,
                put_object.new_async_request(RegionProviderEndpoints::new(region_provider)),
                body,
            )
            .await
        } else {
            let request = put_object.new_async_request(RegionProviderEndpoints::new(
                self.upload_manager.queryer().query(
                    self.async_access_key().await?,
                    self.async_bucket_name().await?,
                ),
            ));
            _async_upload(self, request, body).await
        };

        async fn _async_upload<'a, E: EndpointsProvider + Clone + 'a>(
            form_uploader: &'a FormUploader,
            mut request: AsyncRequestBuilder<'a, E>,
            body: AsyncRequestBody,
        ) -> ApiResult<Value> {
            request.on_uploading_progress(|_, transfer| {
                form_uploader
                    .callbacks
                    .upload_progress(&UploadingProgressInfo::from(transfer))
            });
            if form_uploader
                .callbacks
                .before_request(request.parts_mut())
                .is_cancelled()
            {
                return Err(make_user_cancelled_error(
                    "Cancelled by on_before_request() callback",
                ));
            }
            let mut response_result = request.call(body).await;
            if form_uploader
                .callbacks
                .after_response(&mut response_result)
                .is_cancelled()
            {
                return Err(make_user_cancelled_error(
                    "Cancelled by on_after_response() callback",
                ));
            }
            Ok(response_result?.into_body().into())
        }
    }

    fn make_request_body_from_path(
        &self,
        path: &Path,
        mut params: ObjectParams,
    ) -> ApiResult<SyncRequestBody> {
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
    ) -> ApiResult<SyncRequestBody> {
        let mut file_metadata = PartMetadata::default();
        if let Some(file_name) = params.file_name() {
            file_metadata = file_metadata.file_name(file_name);
        }
        if let Some(content_type) = params.take_content_type() {
            file_metadata = file_metadata.mime(content_type);
        }
        let mut request_body = SyncRequestBody::default().set_upload_token(
            self.upload_manager
                .upload_token()
                .make_upload_token_provider(params.object_name().map(ObjectName::from))
                .as_ref(),
        )?;
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

    #[cfg(feature = "async")]
    async fn make_async_request_body_from_path(
        &self,
        path: &Path,
        mut params: ObjectParams,
    ) -> ApiResult<AsyncRequestBody> {
        let mut file = AsyncFile::open(path).await?;
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
            let crc32 = crc32_of_async_reader(&mut file).await?;
            file.seek(SeekFrom::Start(0)).await?;
            params.extensions_mut().insert(DataCheck::Const(crc32));
        }
        self.make_async_request_body_from_async_reader(file, params)
            .await
    }

    #[cfg(feature = "async")]
    async fn make_async_request_body_from_async_reader<
        R: AsyncRead + Unpin + Send + Sync + 'static,
    >(
        &self,
        reader: R,
        mut params: ObjectParams,
    ) -> ApiResult<AsyncRequestBody> {
        let mut file_metadata = PartMetadata::default();
        if let Some(file_name) = params.file_name() {
            file_metadata = file_metadata.file_name(file_name);
        }
        if let Some(content_type) = params.take_content_type() {
            file_metadata = file_metadata.mime(content_type);
        }
        let mut request_body = AsyncRequestBody::default()
            .set_upload_token(
                self.upload_manager
                    .upload_token()
                    .make_upload_token_provider(params.object_name().map(ObjectName::from))
                    .as_ref(),
            )
            .await?;
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

#[cfg(feature = "async")]
trait AsyncReadTrait: AsyncRead + Unpin + Send + Sync {}

#[cfg(feature = "async")]
impl<T: AsyncRead + Unpin + Send + Sync> AsyncReadTrait for T {}

#[cfg(feature = "async")]
async fn crc32_of_async_reader(reader: &mut dyn AsyncReadTrait) -> IoResult<u32> {
    let mut hasher = crc32fast::Hasher::new();
    let mut reader = AsyncBufReader::new(reader);
    let mut buf = [0u8; 1024];
    loop {
        let have_read = reader.read(&mut buf).await?;
        if have_read == 0 {
            break;
        } else {
            hasher.update(&buf[..have_read]);
        }
    }
    Ok(hasher.finalize())
}

fn make_user_cancelled_error(message: &str) -> ResponseError {
    ResponseError::new(HttpResponseErrorKind::UserCanceled.into(), message)
}

#[cfg(test)]
mod tests {
    use super::{super::super::upload_token::UploadTokenSigner, *};
    use mime::{Mime, BOUNDARY, TEXT_PLAIN};
    use multipart::server::Multipart;
    use qiniu_apis::{
        credential::Credential,
        http::{
            header::CONTENT_TYPE, HeaderName, HeaderValue, HttpCaller, StatusCode, SyncRequest,
            SyncResponse, SyncResponseBody, SyncResponseResult,
        },
        http_client::{DirectChooser, HttpClient, NeverRetrier, Region, NO_BACKOFF},
    };
    use rand::{thread_rng, RngCore};
    use serde_json::{json, to_vec as json_to_vec};
    use std::time::Duration;

    #[cfg(feature = "async")]
    use qiniu_apis::http::{AsyncRequest, AsyncResponseResult};

    #[test]
    fn test_sync_form_upload() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Debug, Default)]
        struct FakeHttpCaller;

        impl HttpCaller for FakeHttpCaller {
            fn call(&self, request: &mut SyncRequest<'_>) -> SyncResponseResult {
                let content_type: Mime = request
                    .headers()
                    .get(CONTENT_TYPE)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse()
                    .unwrap();
                let boundary = content_type.get_param(BOUNDARY).unwrap();
                let mut multipart = Multipart::with_body(request.body_mut(), boundary.as_str());
                while let Some(mut entry) = multipart.read_entry().unwrap() {
                    match entry.headers.name.as_ref() {
                        "token" => {
                            let mut token = String::new();
                            entry.data.read_to_string(&mut token).unwrap();
                            assert!(token.starts_with("fakeaccesskey:"));
                        }
                        "key" => {
                            let mut key = String::new();
                            entry.data.read_to_string(&mut key).unwrap();
                            assert_eq!(key, "fakeobjectname");
                        }
                        "file" => {
                            assert_eq!(entry.headers.filename.as_deref(), Some("fakefilename"));
                            assert_eq!(entry.headers.content_type, Some(TEXT_PLAIN));
                        }
                        _ => unreachable!(),
                    }
                }

                Ok(SyncResponse::builder()
                    .status_code(StatusCode::OK)
                    .header(
                        HeaderName::from_static("x-reqid"),
                        HeaderValue::from_static("FakeReqid"),
                    )
                    .body(SyncResponseBody::from_bytes(
                        json_to_vec(&json!({
                            "hash": "fakehash",
                            "key": "fakekey",
                        }))
                        .unwrap(),
                    ))
                    .build())
            }

            #[cfg(feature = "async")]
            fn async_call(
                &self,
                _request: &mut AsyncRequest<'_>,
            ) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let rand_reader = Box::new(thread_rng()) as Box<dyn RngCore>;
        let value = get_upload_manager(FakeHttpCaller)
            .form_uploader()
            .upload_reader(
                rand_reader.take(1 << 10),
                ObjectParams::builder()
                    .object_name("fakeobjectname")
                    .file_name("fakefilename")
                    .content_type(TEXT_PLAIN)
                    .region_provider(single_up_domain_region())
                    .build(),
            )?;
        assert_eq!(value["hash"].as_str(), Some("fakehash"));
        assert_eq!(value["key"].as_str(), Some("fakekey"));

        Ok(())
    }

    fn get_upload_manager(caller: impl HttpCaller + 'static) -> UploadManager {
        UploadManager::builder(UploadTokenSigner::new_credential_provider(
            get_credential(),
            "fakebucket",
            Duration::from_secs(100),
        ))
        .http_client(
            HttpClient::builder(caller)
                .chooser(DirectChooser)
                .request_retrier(NeverRetrier)
                .backoff(NO_BACKOFF)
                .build(),
        )
        .build()
    }

    fn get_credential() -> Credential {
        Credential::new("fakeaccesskey", "fakesecretkey")
    }

    fn single_up_domain_region() -> Region {
        Region::builder("chaotic")
            .push_up_preferred_endpoint(("fakeup.example.com".to_owned(), 8080).into())
            .build()
    }
}
