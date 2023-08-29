use super::{
    super::{
        callbacks::{Callbacks, UploaderWithCallbacks, UploadingProgressInfo},
        upload_token::OwnedUploadTokenProviderOrReferenced,
        ObjectParams, UploadManager,
    },
    SinglePartUploader,
};
use anyhow::{Error as AnyError, Result as AnyResult};
use qiniu_apis::{
    credential::AccessKey,
    http::{ResponseErrorKind as HttpResponseErrorKind, ResponseParts},
    http_client::{
        ApiResult, BucketRegionsProvider, EndpointsProvider, PartMetadata, RegionsProvider, RegionsProviderEndpoints,
        RequestBuilderParts, Response, ResponseError,
    },
    storage::put_object::{self, sync_part::RequestBody as SyncRequestBody, SyncRequestBuilder},
};
use qiniu_upload_token::{BucketName, ObjectName, UploadTokenProvider};
use serde_json::Value;
use std::{
    fmt::Debug,
    fs::File,
    io::{Read, Seek},
    path::Path,
};

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
use {
    futures::{future::BoxFuture, AsyncRead, AsyncSeek, AsyncSeekExt},
    qiniu_apis::storage::put_object::{async_part::RequestBody as AsyncRequestBody, AsyncRequestBuilder},
    qiniu_utils::async_fs::File as AsyncFile,
    std::io::SeekFrom,
};

/// 表单上传器
///
/// 通过七牛表单上传 API 一次上传整个数据流
///
/// ### 用表单上传器上传文件
///
/// ##### 阻塞代码示例
///
/// ```
/// use qiniu_upload_manager::{
///     apis::credential::Credential, prelude::*, ObjectParams, UploadManager, UploadTokenSigner,
/// };
/// use std::time::Duration;
///
/// # fn example() -> anyhow::Result<()> {
/// let bucket_name = "test-bucket";
/// let object_name = "test-object";
/// let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
///     Credential::new("abcdefghklmnopq", "1234567890"),
///     bucket_name,
///     Duration::from_secs(3600),
/// ))
/// .build();
/// let params = ObjectParams::builder().object_name(object_name).file_name(object_name).build();
/// let mut uploader = upload_manager.form_uploader();
/// uploader.upload_path("/home/qiniu/test.png", params)?;
/// # Ok(())
/// # }
/// ```
///
/// ##### 异步代码示例
///
/// ```
/// use qiniu_upload_manager::{
///     apis::credential::Credential, prelude::*, ObjectParams, UploadManager, UploadTokenSigner,
/// };
/// use std::time::Duration;
///
/// # async fn example() -> anyhow::Result<()> {
/// let bucket_name = "test-bucket";
/// let object_name = "test-object";
/// let upload_manager = UploadManager::builder(UploadTokenSigner::new_credential_provider(
///     Credential::new("abcdefghklmnopq", "1234567890"),
///     bucket_name,
///     Duration::from_secs(3600),
/// ))
/// .build();
/// let params = ObjectParams::builder().object_name(object_name).file_name(object_name).build();
/// let mut uploader = upload_manager.form_uploader();
/// uploader.async_upload_path("/home/qiniu/test.png", params).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct FormUploader {
    upload_manager: UploadManager,
    callbacks: Callbacks<'static>,
}

impl FormUploader {
    #[inline]
    pub(crate) fn new_with_callbacks(upload_manager: UploadManager, callbacks: Callbacks<'static>) -> Self {
        Self {
            upload_manager,
            callbacks,
        }
    }
}

impl UploaderWithCallbacks for FormUploader {
    #[inline]
    fn on_before_request<F: Fn(&mut RequestBuilderParts<'_>) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_before_request_callback(callback);
        self
    }

    #[inline]
    fn on_upload_progress<F: Fn(&UploadingProgressInfo) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_upload_progress_callback(callback);
        self
    }

    #[inline]
    fn on_response_ok<F: Fn(&mut ResponseParts) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_after_response_ok_callback(callback);
        self
    }

    #[inline]
    fn on_response_error<F: Fn(&mut ResponseError) -> AnyResult<()> + Send + Sync + 'static>(
        &mut self,
        callback: F,
    ) -> &mut Self {
        self.callbacks.insert_after_response_error_callback(callback);
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

    fn upload_path(&self, path: impl AsRef<Path>, params: ObjectParams) -> ApiResult<Value> {
        self.upload(
            params.region_provider(),
            Self::make_request_body_from_path(path.as_ref(), self.make_upload_token_signer(&params).as_ref(), &params)?,
        )
    }

    fn upload_reader<R: Read + Send + Sync>(&self, reader: R, params: ObjectParams) -> ApiResult<Value> {
        self.upload(
            params.region_provider(),
            Self::make_request_body_from_reader(
                reader,
                None,
                self.make_upload_token_signer(&params).as_ref(),
                &params,
            )?,
        )
    }

    fn upload_seekable_reader<R: Read + Seek + Send + Sync>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> ApiResult<Value> {
        self.upload(
            params.region_provider(),
            Self::make_request_body_from_seekable_reader(
                reader,
                None,
                self.make_upload_token_signer(&params).as_ref(),
                &params,
            )?,
        )
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
    )]
    fn async_upload_path<'a>(
        &'a self,
        path: impl AsRef<Path> + Send + Sync + 'a,
        params: ObjectParams,
    ) -> BoxFuture<'a, ApiResult<Value>> {
        Box::pin(async move {
            self.async_upload(
                params.region_provider(),
                self.make_async_request_body_from_path(
                    path.as_ref(),
                    self.make_upload_token_signer(&params).as_ref(),
                    &params,
                )
                .await?,
            )
            .await
        })
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
    )]
    fn async_upload_reader<R: AsyncRead + Unpin + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>> {
        Box::pin(async move {
            self.async_upload(
                params.region_provider(),
                Self::make_async_request_body_from_async_reader(
                    reader,
                    None,
                    self.make_upload_token_signer(&params).as_ref(),
                    &params,
                )
                .await?,
            )
            .await
        })
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    #[cfg_attr(
        feature = "docs",
        doc(cfg(any(feature = "async-std-runtime", feature = "tokio-runtime")))
    )]
    fn async_upload_seekable_reader<R: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static>(
        &self,
        reader: R,
        params: ObjectParams,
    ) -> BoxFuture<ApiResult<Value>> {
        Box::pin(async move {
            self.async_upload(
                params.region_provider(),
                Self::make_async_request_body_from_async_seekable_reader(
                    reader,
                    None,
                    self.make_upload_token_signer(&params).as_ref(),
                    &params,
                )
                .await?,
            )
            .await
        })
    }
}

impl super::__private::Sealed for FormUploader {}

impl FormUploader {
    fn upload(&self, region_provider: Option<&dyn RegionsProvider>, body: SyncRequestBody<'_>) -> ApiResult<Value> {
        let put_object = self.put_object();
        return if let Some(region_provider) = region_provider {
            _upload(
                self,
                put_object.new_request(RegionsProviderEndpoints::new(region_provider)),
                body,
            )
        } else {
            let request = put_object.new_request(RegionsProviderEndpoints::new(self.get_bucket_region()?));
            _upload(self, request, body)
        };

        fn _upload<'a, E: EndpointsProvider + Clone + 'a>(
            form_uploader: &'a FormUploader,
            mut request: SyncRequestBuilder<'a, E>,
            body: SyncRequestBody<'a>,
        ) -> ApiResult<Value> {
            request.on_uploading_progress(|_, transfer| {
                form_uploader
                    .callbacks
                    .upload_progress(&UploadingProgressInfo::from(transfer))
            });
            form_uploader.before_request_call(request.parts_mut())?;
            let mut response_result = request.call(body);
            form_uploader.after_response_call(&mut response_result)?;
            Ok(response_result?.into_body().into())
        }
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    async fn async_upload<'a>(
        &'a self,
        region_provider: Option<&'a dyn RegionsProvider>,
        body: AsyncRequestBody<'a>,
    ) -> ApiResult<Value> {
        let put_object = self.put_object();
        return if let Some(region_provider) = region_provider {
            _async_upload(
                self,
                put_object.new_async_request(RegionsProviderEndpoints::new(region_provider)),
                body,
            )
            .await
        } else {
            let request =
                put_object.new_async_request(RegionsProviderEndpoints::new(self.async_get_bucket_region().await?));
            _async_upload(self, request, body).await
        };

        async fn _async_upload<'a, E: EndpointsProvider + Clone + 'a>(
            form_uploader: &'a FormUploader,
            mut request: AsyncRequestBuilder<'a, E>,
            body: AsyncRequestBody<'a>,
        ) -> ApiResult<Value> {
            request.on_uploading_progress(|_, transfer| {
                form_uploader
                    .callbacks
                    .upload_progress(&UploadingProgressInfo::from(transfer))
            });
            form_uploader.before_request_call(request.parts_mut())?;
            let mut response_result = request.call(body).await;
            form_uploader.after_response_call(&mut response_result)?;
            Ok(response_result?.into_body().into())
        }
    }

    fn make_request_body_from_path<'a>(
        path: &'a Path,
        token: &'a (dyn UploadTokenProvider + 'a),
        params: &'a ObjectParams,
    ) -> ApiResult<SyncRequestBody<'a>> {
        let mut file = File::open(path)?;
        if file.stream_position().is_ok() {
            Self::make_request_body_from_seekable_reader(file, Some(path), token, params)
        } else {
            Self::make_request_body_from_reader(file, Some(path), token, params)
        }
    }

    fn make_part_metadata<'a>(path: Option<&'a Path>, params: &'a ObjectParams) -> PartMetadata {
        let mut file_metadata = PartMetadata::default();
        let mut file_name_set = false;
        if let Some(file_name) = params.file_name() {
            file_metadata = file_metadata.file_name(file_name);
            file_name_set = true;
        } else if let Some(path) = path {
            if let Some(file_name) = path.file_name() {
                file_metadata = file_metadata.file_name(Path::new(file_name).display().to_string());
                file_name_set = true;
            }
        }
        if !file_name_set {
            file_metadata = file_metadata.file_name("untitled");
        }
        if let Some(content_type) = params.content_type() {
            file_metadata = file_metadata.mime(content_type.to_owned());
        }
        file_metadata
    }

    fn make_request_body_from_token_and_params<'a>(
        token: &'a (dyn UploadTokenProvider + 'a),
        params: &'a ObjectParams,
    ) -> ApiResult<SyncRequestBody<'a>> {
        let mut request_body = SyncRequestBody::default().set_upload_token(token, Default::default())?;
        if let Some(object_name) = params.object_name() {
            request_body = request_body.set_object_name(object_name.to_string());
        }
        for (key, value) in params.metadata() {
            request_body = request_body.append_custom_data("x-qn-meta-".to_owned() + key, value);
        }
        for (key, value) in params.custom_vars() {
            request_body = request_body.append_custom_data("x:".to_owned() + key, value);
        }
        Ok(request_body)
    }

    fn make_request_body_from_reader<'a, R: Read + Send + Sync + 'a>(
        reader: R,
        path: Option<&'a Path>,
        token: &'a (dyn UploadTokenProvider + 'a),
        params: &'a ObjectParams,
    ) -> ApiResult<SyncRequestBody<'a>> {
        let file_metadata = Self::make_part_metadata(path, params);
        Ok(Self::make_request_body_from_token_and_params(token, params)?.set_file_as_reader(reader, file_metadata))
    }

    fn make_request_body_from_seekable_reader<'a, R: Read + Seek + Send + Sync + 'a>(
        reader: R,
        path: Option<&'a Path>,
        token: &'a (dyn UploadTokenProvider + 'a),
        params: &'a ObjectParams,
    ) -> ApiResult<SyncRequestBody<'a>> {
        let file_metadata = Self::make_part_metadata(path, params);
        Ok(Self::make_request_body_from_token_and_params(token, params)?
            .set_file_as_seekable_reader(reader, file_metadata))
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    async fn make_async_request_body_from_path<'a>(
        &'a self,
        path: &'a Path,
        token: &'a (dyn UploadTokenProvider + 'a),
        params: &'a ObjectParams,
    ) -> ApiResult<AsyncRequestBody<'a>> {
        let mut file = AsyncFile::open(path).await?;
        if file.seek(SeekFrom::Current(0)).await.is_ok() {
            Self::make_async_request_body_from_async_seekable_reader(file, Some(path), token, params).await
        } else {
            Self::make_async_request_body_from_async_reader(file, Some(path), token, params).await
        }
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    async fn make_async_request_body_from_token_and_params<'a>(
        token: &'a (dyn UploadTokenProvider + 'a),
        params: &'a ObjectParams,
    ) -> ApiResult<AsyncRequestBody<'a>> {
        let mut request_body = AsyncRequestBody::default()
            .set_upload_token(token, Default::default())
            .await?;
        if let Some(object_name) = params.object_name() {
            request_body = request_body.set_object_name(object_name.to_string());
        }
        for (key, value) in params.metadata() {
            request_body = request_body.append_custom_data("x-qn-meta-".to_owned() + key, value);
        }
        for (key, value) in params.custom_vars() {
            request_body = request_body.append_custom_data("x:".to_owned() + key, value);
        }
        Ok(request_body)
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    async fn make_async_request_body_from_async_reader<'a, R: AsyncRead + Unpin + Send + Sync + 'static>(
        reader: R,
        path: Option<&'a Path>,
        token: &'a (dyn UploadTokenProvider + 'a),
        params: &'a ObjectParams,
    ) -> ApiResult<AsyncRequestBody<'a>> {
        let file_metadata = Self::make_part_metadata(path, params);
        Ok(Self::make_async_request_body_from_token_and_params(token, params)
            .await?
            .set_file_as_reader(reader, file_metadata))
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    async fn make_async_request_body_from_async_seekable_reader<
        'a,
        R: AsyncRead + AsyncSeek + Unpin + Send + Sync + 'static,
    >(
        reader: R,
        path: Option<&'a Path>,
        token: &'a (dyn UploadTokenProvider + 'a),
        params: &'a ObjectParams,
    ) -> ApiResult<AsyncRequestBody<'a>> {
        let file_metadata = Self::make_part_metadata(path, params);
        Ok(Self::make_async_request_body_from_token_and_params(token, params)
            .await?
            .set_file_as_seekable_reader(reader, file_metadata))
    }

    fn get_bucket_region(&self) -> ApiResult<BucketRegionsProvider> {
        Ok(self
            .upload_manager
            .queryer()
            .query(self.access_key()?, self.bucket_name()?))
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    async fn async_get_bucket_region(&self) -> ApiResult<BucketRegionsProvider> {
        Ok(self
            .upload_manager
            .queryer()
            .query(self.async_access_key().await?, self.async_bucket_name().await?))
    }

    fn make_upload_token_signer(&self, params: &ObjectParams) -> OwnedUploadTokenProviderOrReferenced<'_> {
        let object_name = params.object_name().map(ObjectName::from);
        self.upload_manager
            .upload_token()
            .make_upload_token_provider(object_name)
    }

    fn put_object(&self) -> put_object::Client {
        self.upload_manager.client().storage().put_object()
    }

    fn access_key(&self) -> ApiResult<AccessKey> {
        self.upload_manager.upload_token().access_key()
    }

    fn bucket_name(&self) -> ApiResult<BucketName> {
        self.upload_manager.upload_token().bucket_name()
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    async fn async_access_key(&self) -> ApiResult<AccessKey> {
        self.upload_manager.upload_token().async_access_key().await
    }

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
    async fn async_bucket_name(&self) -> ApiResult<BucketName> {
        self.upload_manager.upload_token().async_bucket_name().await
    }

    fn before_request_call(&self, request: &mut RequestBuilderParts<'_>) -> ApiResult<()> {
        self.callbacks.before_request(request).map_err(make_callback_error)
    }

    fn after_response_call<B>(&self, response: &mut ApiResult<Response<B>>) -> ApiResult<()> {
        self.callbacks.after_response(response).map_err(make_callback_error)
    }
}

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
trait AsyncReadTrait: AsyncRead + Unpin + Send + Sync {}

#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
impl<T: AsyncRead + Unpin + Send + Sync> AsyncReadTrait for T {}

fn make_callback_error(err: AnyError) -> ResponseError {
    ResponseError::new_with_msg(HttpResponseErrorKind::CallbackError.into(), err)
}

#[cfg(test)]
mod tests {
    use super::{
        super::super::{
            mime::{Mime, BOUNDARY, TEXT_PLAIN},
            upload_token::UploadTokenSigner,
        },
        *,
    };
    use multipart::server::Multipart;
    use qiniu_apis::{
        credential::Credential,
        http::{
            header::CONTENT_TYPE, HeaderValue, HttpCaller, StatusCode, SyncRequest, SyncResponse, SyncResponseBody,
            SyncResponseResult,
        },
        http_client::{DirectChooser, HttpClient, NeverRetrier, Region, NO_BACKOFF},
    };
    use rand::{thread_rng, RngCore};
    use serde_json::{json, to_vec as json_to_vec};
    use std::{
        io::{Read, Result as IoResult},
        time::Duration,
    };

    #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
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
                    .header("x-reqid", HeaderValue::from_static("FakeReqid"))
                    .body(SyncResponseBody::from_bytes(
                        json_to_vec(&json!({
                            "hash": "fakehash",
                            "key": "fakekey",
                        }))
                        .unwrap(),
                    ))
                    .build())
            }

            #[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
            fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let value = get_upload_manager(FakeHttpCaller).form_uploader().upload_reader(
            RandReader.take(1 << 10),
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
            .add_up_preferred_endpoint(("fakeup.example.com".to_owned(), 8080).into())
            .build()
    }

    struct RandReader;

    impl Read for RandReader {
        fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
            let mut rng = thread_rng();
            rng.fill_bytes(buf);
            Ok(buf.len())
        }
    }
}
