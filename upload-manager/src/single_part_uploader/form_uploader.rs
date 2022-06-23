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
        ApiResult, BucketRegionsProvider, EndpointsProvider, FileName, PartMetadata, RegionsProvider,
        RegionsProviderEndpoints, RequestBuilderParts, Response, ResponseError,
    },
    storage::put_object::{self, sync_part::RequestBody as SyncRequestBody, SyncRequestBuilder},
};
use qiniu_upload_token::{BucketName, ObjectName, UploadTokenProvider};
use serde_json::Value;
use std::{fmt::Debug, fs::File, io::Read, mem::take, path::Path};

#[cfg(feature = "async")]
use {
    async_std::fs::File as AsyncFile,
    futures::{future::BoxFuture, AsyncRead},
    qiniu_apis::storage::put_object::{async_part::RequestBody as AsyncRequestBody, AsyncRequestBuilder},
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
    fn on_response_error<F: Fn(&ResponseError) -> AnyResult<()> + Send + Sync + 'static>(
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

    fn upload_path(&self, path: impl AsRef<Path>, mut params: ObjectParams) -> ApiResult<Value> {
        self.upload(
            take(params.region_provider_mut()),
            self.make_request_body_from_path(path.as_ref(), self.make_upload_token_signer(&params).as_ref(), params)?,
        )
    }

    fn upload_reader<R: Read + 'static>(&self, reader: R, mut params: ObjectParams) -> ApiResult<Value> {
        self.upload(
            take(params.region_provider_mut()),
            self.make_request_body_from_reader(reader, self.make_upload_token_signer(&params).as_ref(), params)?,
        )
    }

    #[cfg(feature = "async")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
    fn async_upload_path<'a>(
        &'a self,
        path: impl AsRef<Path> + Send + Sync + 'a,
        mut params: ObjectParams,
    ) -> BoxFuture<'a, ApiResult<Value>> {
        Box::pin(async move {
            self.async_upload(
                take(params.region_provider_mut()),
                self.make_async_request_body_from_path(
                    path.as_ref(),
                    self.make_upload_token_signer(&params).as_ref(),
                    params,
                )
                .await?,
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
                take(params.region_provider_mut()),
                self.make_async_request_body_from_async_reader(
                    reader,
                    self.make_upload_token_signer(&params).as_ref(),
                    params,
                )
                .await?,
            )
            .await
        })
    }
}

impl FormUploader {
    fn upload(&self, region_provider: Option<Box<dyn RegionsProvider>>, body: SyncRequestBody<'_>) -> ApiResult<Value> {
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
            body: SyncRequestBody<'_>,
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

    #[cfg(feature = "async")]
    async fn async_upload<'a>(
        &'a self,
        region_provider: Option<Box<dyn RegionsProvider>>,
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
        &'a self,
        path: &Path,
        token: &'a (dyn UploadTokenProvider + 'a),
        mut params: ObjectParams,
    ) -> ApiResult<SyncRequestBody<'a>> {
        let file = File::open(path)?;
        if params.file_name().is_none() {
            *params.file_name_mut() = path
                .file_name()
                .map(Path::new)
                .map(|file_name| FileName::from(file_name.display().to_string()));
        }
        self.make_request_body_from_reader(file, token, params)
    }

    fn make_request_body_from_reader<'a, R: Read + 'static>(
        &'a self,
        reader: R,
        token: &'a (dyn UploadTokenProvider + 'a),
        mut params: ObjectParams,
    ) -> ApiResult<SyncRequestBody<'a>> {
        let mut file_metadata = PartMetadata::default();
        if let Some(file_name) = params.file_name() {
            file_metadata = file_metadata.file_name(file_name);
        }
        if let Some(content_type) = take(params.content_type_mut()) {
            file_metadata = file_metadata.mime(content_type);
        }
        let mut request_body = SyncRequestBody::default().set_upload_token(token, Default::default())?;
        if let Some(object_name) = take(params.object_name_mut()) {
            request_body = request_body.set_object_name(object_name.to_string());
        }
        for (key, value) in take(params.metadata_mut()).into_iter() {
            request_body = request_body.append_custom_data("x-qn-meta-".to_owned() + &key, value);
        }
        for (key, value) in take(params.custom_vars_mut()).into_iter() {
            request_body = request_body.append_custom_data("x:".to_owned() + &key, value);
        }
        request_body = request_body.set_file_as_reader(reader, file_metadata);
        Ok(request_body)
    }

    #[cfg(feature = "async")]
    async fn make_async_request_body_from_path<'a>(
        &'a self,
        path: &'a Path,
        token: &'a (dyn UploadTokenProvider + 'a),
        mut params: ObjectParams,
    ) -> ApiResult<AsyncRequestBody<'a>> {
        let file = AsyncFile::open(path).await?;
        if params.file_name().is_none() {
            *params.file_name_mut() = path
                .file_name()
                .map(Path::new)
                .map(|file_name| FileName::from(file_name.display().to_string()));
        }
        self.make_async_request_body_from_async_reader(file, token, params)
            .await
    }

    #[cfg(feature = "async")]
    async fn make_async_request_body_from_async_reader<'a, R: AsyncRead + Unpin + Send + Sync + 'static>(
        &'a self,
        reader: R,
        token: &'a (dyn UploadTokenProvider + 'a),
        mut params: ObjectParams,
    ) -> ApiResult<AsyncRequestBody<'a>> {
        let mut file_metadata = PartMetadata::default();
        if let Some(file_name) = params.file_name() {
            file_metadata = file_metadata.file_name(file_name);
        } else {
            file_metadata = file_metadata.file_name("untitled");
        }
        if let Some(content_type) = take(params.content_type_mut()) {
            file_metadata = file_metadata.mime(content_type);
        }
        let mut request_body = AsyncRequestBody::default()
            .set_upload_token(token, Default::default())
            .await?;
        if let Some(object_name) = take(params.object_name_mut()) {
            request_body = request_body.set_object_name(object_name.to_string());
        }
        for (key, value) in take(params.metadata_mut()).into_iter() {
            request_body = request_body.append_custom_data("x-qn-meta-".to_owned() + &key, value);
        }
        for (key, value) in take(params.custom_vars_mut()).into_iter() {
            request_body = request_body.append_custom_data("x:".to_owned() + &key, value);
        }
        request_body = request_body.set_file_as_reader(reader, file_metadata);
        Ok(request_body)
    }

    fn get_bucket_region(&self) -> ApiResult<BucketRegionsProvider> {
        Ok(self
            .upload_manager
            .queryer()
            .query(self.access_key()?, self.bucket_name()?))
    }

    #[cfg(feature = "async")]
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

    #[cfg(feature = "async")]
    async fn async_access_key(&self) -> ApiResult<AccessKey> {
        self.upload_manager.upload_token().async_access_key().await
    }

    #[cfg(feature = "async")]
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

#[cfg(feature = "async")]
trait AsyncReadTrait: AsyncRead + Unpin + Send + Sync {}

#[cfg(feature = "async")]
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

            #[cfg(feature = "async")]
            fn async_call(&self, _request: &mut AsyncRequest<'_>) -> BoxFuture<AsyncResponseResult> {
                unreachable!()
            }
        }

        let rand_reader = Box::new(thread_rng()) as Box<dyn RngCore>;
        let value = get_upload_manager(FakeHttpCaller).form_uploader().upload_reader(
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
            .add_up_preferred_endpoint(("fakeup.example.com".to_owned(), 8080).into())
            .build()
    }
}
