# Change log

## v0.2.2

- 修复 `qiniu_objects_manager::ListStream` 在使用 V2 API 列举时，当无法列举到结果时会导致无限循环的问题。

## v0.2.1

- `qiniu_upload_manager::MultiPartsV1Uploader` 总是使用 4 MB 分片大小，无论 `qiniu_upload_manager::DataPartitionProvider` 返回多大的分片大小。
- `qiniu_upload_manager::SerialMultiPartsUploaderScheduler` 和 `qiniu_upload_manager::ConcurrentMultiPartsUploaderScheduler` 对空间所在区域上传对象失败后，会使用多活区域继续重试，直到其中有一个能成功为止。

## v0.2.0

- 大部分 Trait 现在都实现了 Clone，减少了泛型参数以方便被作为 Trait Object 使用
- 所有回调函数均可以返回 `anyhow::Result` 作为返回结果
- `qiniu_http::Metrics` 从 trait 改为 struct，并增加相应的 `qiniu_http:MetricsBuilder`
- `qiniu_http` 增加 `OnProgressCallback` / `OnStatusCodeCallback` / `OnHeaderCallback` 存储上传进度 / 状态码 / HTTP Header 回调函数
- 增加 `qiniu_http::RequestPartsBuilder`
- 为 `qiniu_http_client::RetriedStatsInfo` 和 `qiniu_download_manager::RetriedStatsInfo` 增加公开的修改方法
- `qiniu_http_client::ResponseError` 现在可以存储 `http::Extensions`，多个先前返回 `&qiniu_http_client::ResponseError` 的回调函数现在返回它的可变引用。
- `qiniu_upload_manager` 改进对于上传的分片有效期的判断逻辑
- `qiniu_upload_manager` 恢复分片上传时优先使用来自恢复文件中记录的上传地址
- `qiniu_upload_manager` 从 `FileDataSource` 内分离出 `AsyncFileDataSource`，以及从 `DataSource` 里分离出 `AsyncDataSource`
- `qiniu_upload_manager::MultiPartsUploader` 内 `complete_parts` 和 `async_complete_parts` 不再要求传入 `InitializedParts` 和 `UploadedPart` 本身，只需要传入引用即可
- `qiniu_upload_manager` 公开 `UnseekableDataSource` 和 `AsyncUnseekableDataSource`
- `qiniu_upload_manager::ObjectParams` / `qiniu_upload_manager::AutoUploaderObjectParams` 不再返回任何字段的可变引用
- 修复 `qiniu_download_manager` 只能下载带有 `Content-Length` 的响应的问题

### 不兼容变更

#### `qiniu-upload-token`

- `UploadTokenProvider::to_token_string` 的返回结果从 `std::io::Result` 改为 `ToStringResult`，`UploadTokenProvider::async_to_token_string` 的返回结果则改为 `ToStringResult` 的 `BoxFuture` 版本。
- `BucketUploadTokenProviderBuilder` 和 `ObjectUploadTokenProviderBuilder` 的 `on_policy_generated` 方法接受的回调函数现在需要返回 `anyhow::Result<()>`，先前不返回任何数据。

#### `qiniu-http`

- `ResponseError::builder` 接受的错误类型从 `impl Into<Box<dyn std::io::Error + Send + Sync>>` 改为 `impl Into<anyhow::Error>`，`ResponseError::into_inner` 返回的数据类型从 `Box<dyn std::io::Error + Send + Sync>` 改为 `anyhow::Error`。
- `Metrics` 类型从 Trait 改为 Struct。
- `CallbackResult` 类型被彻底移除，所有回调函数返回值从 `CallbackResult` 类型改为 `anyhow::Result`。
- `RequestParts::on_uploading_progress_mut` 现在返回 `&mut Option<OnProgressCallback>` 类型，`RequestParts::on_receive_response_status_mut` 现在返回 `&mut Option<OnStatusCodeCallback>` 类型，`RequestParts::on_receive_response_header_mut` 现在返回 `&mut Option<OnHeaderCallback>` 类型。
- `RequestBuilder::on_uploading_progress` 现在接受 `impl Into<OnProgressCallback>` 类型，`RequestBuilder::on_receive_response_status` 现在接受 `impl Into<OnStatusCodeCallback>` 类型，`RequestBuilder::on_receive_response_header` 现在接受 `impl Into<OnHeaderCallback>` 类型。

#### `qiniu-http-client`

- 所有回调函数返回值从 `CallbackResult` 类型改为 `anyhow::Result`。
- `RequestBuilder::on_uploading_progress` 接受的回调函数的参数类型从 `&TransferProgressInfo` 改为 `TransferProgressInfo`，`RequestBuilder::on_error` 接受的回调函数的参数类型从 `&ResponseError` 改为 `&mut ResponseError`。
- `ResponseError::new` 接受的错误类型从 `impl Into<Box<dyn std::io::Error + Send + Sync>>` 改为 `impl Into<anyhow::Error>`。
- `Resolver` / `Backoff` / `Chooser` / `RequestRetrier` 现在实现 `Clone`。

#### `qiniu-apis`

- 所有回调函数返回值从 `CallbackResult` 类型改为 `anyhow::Result`。
- `RequestBuilder::on_uploading_progress` 接受的回调函数的参数类型从 `&TransferProgressInfo` 改为 `TransferProgressInfo`，`RequestBuilder::on_error` 接受的回调函数的参数类型从 `&ResponseError` 改为 `&mut ResponseError`。

#### `qiniu-objects-manager`

- 所有回调函数返回值从 `CallbackResult` 类型改为 `anyhow::Result`。
- `BatchOperations::after_response_error_callback` 接受的回调函数的参数类型从 `&ResponseError` 改为 `&mut ResponseError`，`ListBuilder::after_response_error_callback` 接受的回调函数的参数类型从 `&ResponseError` 改为 `&mut ResponseError`。
- `BatchSizeProvider` / `OperationProvider` 现在实现 `Clone`。
- `ObjectsManager::credential` 返回类型从 `Arc<dyn CredentialProvider>` 改为 `&dyn CredentialProvider`。

#### `qiniu-upload-manager`

- 所有回调函数返回值从 `CallbackResult` 类型改为 `anyhow::Result`。
- `UploadManager::multi_parts_uploader` / `UploadManager::multi_parts_v1_uploader` / `UploadManager::multi_parts_v2_uploader` 现在额外接受泛型参数 `H: Digest + Send + 'static`。
- `UploadManager::auto_uploader` / `UploadManager::auto_uploader_builder` 现在仅接受一个泛型参数 `H: Digest + Send + 'static`。
- `UploaderWithCallbacks::on_response_error` 接受的回调函数的参数类型从 `&ResponseError` 改为 `&mut ResponseError`。
- `AutoUploader` / `AutoUploaderBuilder` 现在仅有一个泛型参数 `H: Digest + Send + 'static`。
- `ObjectParams` / `AutoUploaderObjectParams` 不再返回任何字段的可变引用。
- `ObjectParams` / `ObjectParamsBuilder` 移除所有 `uploaded_part_ttl` 字段相关方法。
- `ConcurrencyProvider` / `DataPartitionProvider` / `ResumablePolicyProvider` / `ResumableRecorder` / `MultiPartsUploaderScheduler` / `DataSource` / `SinglePartUploader` / `MultiPartsUploader` / `InitializedParts` 现在实现 `Clone`。
- `DataSource` 移除了 `async_slice` / `async_source_key` / `async_total_size` 方法，增加了 `reset` 方法。
- `ResumablePolicyProvider` 的 `get_policy_from_reader` / `get_policy_from_async_reader` 方法原本接受的是泛型参数，现在接受它的 Trait Object 类型。
- `ResumableRecorder` 现在只有一个泛型参数 `HashAlgorithm`，所有涉及到泛型参数的方法现在都改用 Trait Object 类型。
- `MultiPartsUploaderScheduler` 现在只有一个泛型参数 `A: Digest`，移除了 `new` 方法，所有涉及到泛型参数的方法现在都改用 Trait Object 类型。
- `MultiPartsUploader` 移除了泛型参数 `ResumableRecorder`，加上了 `HashAlgorithm: Digest + Send + 'static` / `AsyncInitializedParts: InitializedParts + 'static` / `AsyncUploadedPart: UploadedPart`。
- `MultiPartsUploader` 增加了 `reinitialize_parts` / `async_reinitialize_parts` 方法。
- `MultiPartsUploader` 的 `complete_parts` / `async_complete_parts` 方法现在只接受 `initialized` 和 `parts` 的不可变引用。
- `SinglePartUploader` / `MultiPartsUploader` / `InitializedParts` / `UploadedPart` 不可被用户实现。

#### `qiniu-download-manager`

- 所有回调函数返回值从 `CallbackResult` 类型改为 `anyhow::Result`。
- `DownloadManager` / `DownloadManagerBuilder` / `EndpointsUrlGenerator` / `EndpointsUrlGeneratorBuilder` / `UrlsSigner` 现在移除了所有泛型参数。
- `DownloadManagerBuilder` 的所有构造器方法现在都仅需要 `&mut self` 即可调用。
- `DownloadingObject::on_download_progress` 接受的回调函数的参数从 `&TransferProgressInfo` 改为 `DownloadingProgressInfo`，`DownloadingObject::on_response_error` 接受的回调函数的参数从 `&ResponseError` 改为 `&mut ResponseError`。
- `DownloadRetrier` / `DownloadUrlsGenerator` 现在实现 `Clone`。

## v0.1.4

- `ObjectsManagerBuilder` / `UploadManagerBuilder` 的 `use_https` 方法也对内置的 `BucketRegionsQueryerBuilder` 生效
- `ListIter` / `ListStream` 添加 `marker` 方法，获取最近一次列举返回的位置标记

## v0.1.3

- 修复编译 `ureq` 出现 `pattern InsecureRequestHttpsOnly not covered` 的错误
- 升级 `fs4` 否则可能导致在 Linux 上编译失败
- 为 `SubnetChooserBuilder` 增加安全的设置 IP 地址子网掩码前缀长度的方法
- 导出多个遗漏的公开类型
- 解决 `qiniu_http::Request::from_parts_and_body` 无法使用的 bug

## v0.1.2
- 对象存储，管理类 API 发送请求时增加 [X-Qiniu-Date](https://developer.qiniu.com/kodo/3924/common-request-headers) （生成请求的时间） header
- 简化了 `http_client::RegionBuilder` / `http_client::EndpointsBuilder` 设置多个终端地址的方法
- 提供 `qiniu_http::set_library_user_agent()` 给第三方库设置用户代理，方便所有 User Agent 内包含第三方库信息
- 修复了部分已知问题

## v0.1.1

- 增加 `qiniu-download-manager` 插件负责对象的下载
- 修复了 `Credential::sign_download_url()` 的 `lifetime` 参数实现不正确的 bug
- 简化了设置 HTTP 协议的代码
- 优化 HTTP Header 相关接口

## v0.1.0

- 首个正式版本发布
