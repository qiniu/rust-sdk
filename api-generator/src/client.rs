use super::{
    base_utils::join_path,
    enums::{Authorization, Idempotent, Method, ServiceName},
    form::FormUrlencodedRequestStruct,
    header::HeaderNames,
    json::JsonType,
    mods::mod_rs_client_definition_token_stream,
    multipart::MultipartFormDataRequestStruct,
    path::PathParams,
    query::QueryNames,
    traits::CodeGenerator,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// API 描述信息，可以通过 YAML 描述文件编辑
pub(super) struct ApiDetailedDescription {
    /// API 调用 HTTP 方法
    method: Method,

    /// 七牛服务名称，可以设置多个，表现有多个七牛服务都可以调用该 API
    #[serde(skip_serializing_if = "Vec::is_empty")]
    service_names: Vec<ServiceName>,

    /// API 文档
    pub(super) documentation: String,

    /// 七牛 API URL 基础路径
    base_path: String,

    /// 七牛 API URL 路径后缀
    path_suffix: String,

    /// 七牛 API 调用参数
    request: ApiRequestDescription,

    /// 七牛 API 响应参数
    response: ApiResponseDescription,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct ApiRequestDescription {
    /// 七牛 API 调用 URL 路径参数列表
    #[serde(skip_serializing_if = "Option::is_none")]
    path_params: Option<PathParams>,

    /// 七牛 API 调用 HTTP 头参数列表
    #[serde(skip_serializing_if = "Option::is_none")]
    header_names: Option<HeaderNames>,

    /// 七牛 API 调用 URL 查询参数列表
    #[serde(skip_serializing_if = "Option::is_none")]
    query_names: Option<QueryNames>,

    /// 七牛 API 调用请求体
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<RequestBody>,

    /// 七牛 API 调用鉴权参数
    #[serde(skip_serializing_if = "Option::is_none")]
    authorization: Option<Authorization>,

    /// 七牛 API 调用幂等性
    idempotent: Idempotent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// HTTP 调用请求体
enum RequestBody {
    /// JSON 调用
    Json(JsonType),

    /// URL 编码表单调用（无法上传二进制数据）
    FormUrlencoded(FormUrlencodedRequestStruct),

    /// 复合表单调用（可以上传二进制数据）
    MultipartFormData(MultipartFormDataRequestStruct),

    /// 二进制数据调用
    BinaryData,

    /// 文本数据调用
    PlainText,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// HTTP 响应请求体
enum ResponseBody {
    /// JSON 响应
    Json(JsonType),

    /// 二进制数据响应
    BinaryDataStream,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(super) struct ApiResponseDescription {
    /// 七牛 API 响应 HTTP 头参数列表
    #[serde(skip_serializing_if = "Option::is_none")]
    header_names: Option<HeaderNames>,

    /// 七牛 API 响应请求体
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<ResponseBody>,
}

impl ApiDetailedDescription {
    pub(super) fn to_rust_token_stream(&self) -> TokenStream {
        let api_client_definition_token_stream = api_client_definition_token_stream();
        let request_builder_definition_token_stream = request_builder_definition_token_stream();
        let new_request_method_token_stream = new_request_method_token_stream(self);
        let new_async_request_method_token_stream = new_async_request_method_token_stream(self);
        let path_params_token_stream = self
            .request
            .path_params
            .as_ref()
            .map(|path_params| path_params.to_rust_token_stream("PathParams", "调用 API 所用的路径参数"));
        let query_names_token_stream = self
            .request
            .query_names
            .as_ref()
            .map(|query_names| query_names.to_rust_token_stream("QueryParams", "调用 API 所用的 URL 查询参数"));
        let request_header_names_token_stream = self
            .request
            .header_names
            .as_ref()
            .map(|header_names| header_names.to_rust_token_stream("RequestHeaders", "调用 API 所用的 HTTP 头参数"));
        let response_header_names_token_stream =
            self.response.header_names.as_ref().map(|header_names| {
                header_names.to_rust_token_stream("ResponseHeaders", "获取 API 响应的 HTTP 头参数")
            });
        let request_body_token_stream = match &self.request.body {
            Some(RequestBody::Json(json_struct)) => {
                Some(json_struct.to_rust_token_stream("RequestBody", "调用 API 所用的请求体参数"))
            }
            Some(RequestBody::FormUrlencoded(form_struct)) => {
                Some(form_struct.to_rust_token_stream("RequestBody", "调用 API 所用的请求体参数"))
            }
            Some(RequestBody::MultipartFormData(multipart_struct)) => {
                Some(multipart_struct.to_rust_token_stream("RequestBody", "调用 API 所用的请求体参数"))
            }
            _ => None,
        };
        let response_body_token_stream = match &self.response.body {
            Some(ResponseBody::Json(json_struct)) => {
                Some(json_struct.to_rust_token_stream("ResponseBody", "获取 API 所用的响应体参数"))
            }
            None => Some(JsonType::default().to_rust_token_stream("ResponseBody", "获取 API 所用的响应体参数")),
            Some(ResponseBody::BinaryDataStream) => None,
        };
        let request_builder_methods_token_stream = request_builder_methods_token_stream();
        let sync_request_builder_methods_token_stream = sync_request_builder_methods_token_stream(self);
        let async_request_builder_methods_token_stream = async_request_builder_methods_token_stream(self);

        return quote! {
            #path_params_token_stream
            #query_names_token_stream
            #request_header_names_token_stream
            #response_header_names_token_stream
            #request_body_token_stream
            #response_body_token_stream
            #api_client_definition_token_stream

            impl<'client> Client<'client> {
                #new_request_method_token_stream
                #new_async_request_method_token_stream
            }

            #request_builder_definition_token_stream

            impl<'req, B, E> RequestBuilder<'req, B, E> {
                #request_builder_methods_token_stream
            }

            impl<'req, E: qiniu_http_client::EndpointsProvider + Clone + 'req> SyncRequestBuilder<'req, E> {
                #sync_request_builder_methods_token_stream
            }

            #[cfg(feature = "async")]
            impl<'req, E: qiniu_http_client::EndpointsProvider + Clone + 'req> AsyncRequestBuilder<'req, E> {
                #async_request_builder_methods_token_stream
            }
        };

        fn request_builder_definition_token_stream() -> TokenStream {
            quote! {
                #[derive(Debug)]
                #[doc = "API 请求构造器"]
                pub struct RequestBuilder<'req, B, E>(qiniu_http_client::RequestBuilder<'req, B, E>);

                #[doc = "API 阻塞请求构造器"]
                pub type SyncRequestBuilder<'req, E> = RequestBuilder<'req, qiniu_http_client::SyncRequestBody<'req>, E>;

                #[cfg(feature = "async")]
                #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
                #[doc = "API 异步请求构造器"]
                pub type AsyncRequestBuilder<'req, E> = RequestBuilder<'req, qiniu_http_client::AsyncRequestBody<'req>, E>;
            }
        }

        fn new_request_method_token_stream(description: &ApiDetailedDescription) -> TokenStream {
            let new_request_impl = impl_request_method_token_stream(description, &quote! {endpoints_provider}, true);
            let new_request_params_token_stream = new_request_params_token_stream(description, &quote! {'client});
            quote! {
                #[inline]
                #[doc = "创建一个新的阻塞请求，该方法的异步版本为 [`Self::new_async_request`]"]
                pub fn new_request<E: qiniu_http_client::EndpointsProvider + 'client>(
                    &self,
                    endpoints_provider: E,
                    #new_request_params_token_stream
                ) -> SyncRequestBuilder<'client, E> {
                    RequestBuilder({#new_request_impl})
                }
            }
        }

        fn new_async_request_method_token_stream(description: &ApiDetailedDescription) -> TokenStream {
            let new_async_request_impl =
                impl_request_method_token_stream(description, &quote! {endpoints_provider}, false);
            let new_request_params_token_stream = new_request_params_token_stream(description, &quote! {'client});
            quote! {
                #[inline]
                #[cfg(feature = "async")]
                #[doc = "创建一个新的异步请求"]
                pub fn new_async_request<E: qiniu_http_client::EndpointsProvider + 'client>(
                    &self,
                    endpoints_provider: E,
                    #new_request_params_token_stream
                ) -> AsyncRequestBuilder<'client, E> {
                    RequestBuilder({#new_async_request_impl})
                }
            }
        }

        fn impl_request_method_token_stream(
            description: &ApiDetailedDescription,
            into_endpoints: &TokenStream,
            sync_version: bool,
        ) -> TokenStream {
            let mut statements: Vec<TokenStream> = vec![];
            {
                let method_name = {
                    let mut method_name = match description.method {
                        Method::Get => format_ident!("get"),
                        Method::Post => format_ident!("post"),
                        Method::Put => format_ident!("put"),
                        Method::Delete => format_ident!("delete"),
                    };
                    if !sync_version {
                        method_name = format_ident!("async_{}", method_name);
                    }
                    method_name
                };
                let service_names = {
                    let service_names: Vec<_> = description
                        .service_names
                        .iter()
                        .map(|service_name| match service_name {
                            ServiceName::Up => quote! {qiniu_http_client::ServiceName::Up},
                            ServiceName::Io => quote! {qiniu_http_client::ServiceName::Io},
                            ServiceName::Uc => quote! {qiniu_http_client::ServiceName::Uc},
                            ServiceName::Rs => quote! {qiniu_http_client::ServiceName::Rs},
                            ServiceName::Rsf => quote! {qiniu_http_client::ServiceName::Rsf},
                            ServiceName::Api => quote! {qiniu_http_client::ServiceName::Api},
                            ServiceName::S3 => quote! {qiniu_http_client::ServiceName::S3},
                        })
                        .collect();
                    quote! {
                        &[#(#service_names),*]
                    }
                };
                statements.push(quote! {let mut builder = self.0.#method_name(#service_names, #into_endpoints)});
            }

            if let Some(authorization) = &description.request.authorization {
                let authorization = match authorization {
                    Authorization::Qbox => quote! {
                        qiniu_http_client::Authorization::v1(credential)
                    },
                    Authorization::Qiniu => quote! {
                        qiniu_http_client::Authorization::v2(credential)
                    },
                    Authorization::UploadToken => quote! {
                        qiniu_http_client::Authorization::uptoken(upload_token)
                    },
                };
                statements.push(quote! {builder.authorization(#authorization)});
            }

            let idempotent = match &description.request.idempotent {
                Idempotent::Always => quote! {qiniu_http_client::Idempotent::Always},
                Idempotent::Default => quote! {qiniu_http_client::Idempotent::Default},
                Idempotent::Never => quote! {qiniu_http_client::Idempotent::Never},
            };
            statements.push(quote! {builder.idempotent(#idempotent)});

            {
                let base_path = description.base_path.as_str();
                let path_suffix = description.path_suffix.as_str();
                let path_params_ident = format_ident!("path_params");
                let path_call_token_stream = if description.request.path_params.is_some() {
                    quote! { builder.path(crate::base_utils::join_path(#base_path, #path_suffix, #path_params_ident.build())) }
                } else {
                    let path = join_path(base_path, path_suffix, vec![]);
                    quote! { builder.path(#path) }
                };
                statements.push(path_call_token_stream);
            }

            if matches!(&description.response.body, Some(ResponseBody::BinaryDataStream),) {
                statements.push(quote! {builder.accept_application_octet_stream()});
            } else {
                statements.push(quote! {builder.accept_json()});
            }
            statements.push(quote! {builder});

            quote! {#(#statements);*}
        }

        fn new_request_params_token_stream(
            description: &ApiDetailedDescription,
            lifetime: &TokenStream,
        ) -> TokenStream {
            let mut params: Vec<_> = vec![];
            if description.request.path_params.is_some() {
                params.push(quote! {path_params: PathParams});
            };
            if let Some(authorization) = &description.request.authorization {
                match authorization {
                    Authorization::Qbox | Authorization::Qiniu => {
                        params.push(quote! {
                            credential: impl qiniu_http_client::credential::CredentialProvider + Clone + #lifetime
                        });
                    }
                    Authorization::UploadToken => {
                        params.push(quote! {
                            upload_token: impl qiniu_http_client::upload_token::UploadTokenProvider + Clone + #lifetime
                        });
                    }
                }
            }
            quote! {#(#params),*}
        }

        fn request_builder_methods_token_stream() -> TokenStream {
            quote! {
                #[inline]
                #[doc = "设置是否使用 HTTPS"]
                pub fn use_https(&mut self, use_https: bool) -> &mut Self {
                    self.0.use_https(use_https);
                    self
                }

                #[inline]
                #[doc = "设置 HTTP 协议版本"]
                pub fn version(&mut self, version: qiniu_http_client::http::Version) -> &mut Self {
                    self.0.version(version);
                    self
                }

                #[inline]
                #[doc = "设置 HTTP 请求头"]
                pub fn headers(&mut self, headers: impl Into<std::borrow::Cow<'req, qiniu_http_client::http::HeaderMap>>) -> &mut Self {
                    self.0.headers(headers);
                    self
                }

                #[inline]
                #[doc = "添加 HTTP 请求头"]
                pub fn set_header(
                    &mut self,
                    header_name: impl qiniu_http_client::http::header::IntoHeaderName,
                    header_value: impl Into<qiniu_http_client::http::HeaderValue>,
                ) -> &mut Self {
                    self.0.set_header(header_name, header_value);
                    self
                }

                #[inline]
                #[doc = "设置查询参数"]
                pub fn query(&mut self, query: impl Into<std::borrow::Cow<'req, str>>) -> &mut Self {
                    self.0.query(query);
                    self
                }

                #[inline]
                #[doc = "设置查询参数"]
                pub fn query_pairs(&mut self, query_pairs: impl Into<Vec<qiniu_http_client::QueryPair<'req>>>) -> &mut Self {
                    self.0.query_pairs(query_pairs);
                    self
                }

                #[inline]
                #[doc = "追加查询参数"]
                pub fn append_query_pair(
                    &mut self,
                    query_pair_key: impl Into<qiniu_http_client::QueryPairKey<'req>>,
                    query_pair_value: impl Into<qiniu_http_client::QueryPairValue<'req>>,
                ) -> &mut Self {
                    self.0.append_query_pair(query_pair_key, query_pair_value);
                    self
                }

                #[inline]
                #[doc = "设置扩展信息"]
                pub fn extensions(&mut self, extensions: qiniu_http_client::http::Extensions) -> &mut Self {
                    self.0.extensions(extensions);
                    self
                }

                #[doc = "添加扩展信息"]
                #[inline]
                pub fn add_extension<T: Send + Sync + 'static>(&mut self, val: T) -> &mut Self {
                    self.0.add_extension(val);
                    self
                }

                #[inline]
                #[doc = "上传进度回调函数"]
                pub fn on_uploading_progress(
                    &mut self,
                    callback: impl Fn(
                            &dyn qiniu_http_client::SimplifiedCallbackContext,
                            qiniu_http_client::http::TransferProgressInfo,
                        ) -> anyhow::Result<()>
                            + Send
                            + Sync
                            +'req,
                ) -> &mut Self {
                    self.0.on_uploading_progress(callback);
                    self
                }

                #[inline]
                #[doc = "设置响应状态码回调函数"]
                pub fn on_receive_response_status(
                    &mut self,
                    callback: impl Fn(
                            &dyn qiniu_http_client::SimplifiedCallbackContext,
                            qiniu_http_client::http::StatusCode,
                        ) -> anyhow::Result<()>
                        + Send
                        + Sync
                        + 'req,
                ) -> &mut Self {
                    self.0.on_receive_response_status(callback);
                    self
                }

                #[inline]
                #[doc = "设置响应 HTTP 头回调函数"]
                pub fn on_receive_response_header(
                    &mut self,
                    callback: impl Fn(
                            &dyn qiniu_http_client::SimplifiedCallbackContext,
                            &qiniu_http_client::http::HeaderName,
                            &qiniu_http_client::http::HeaderValue,
                        ) -> anyhow::Result<()>
                            + Send
                            + Sync
                            +'req,
                ) -> &mut Self {
                    self.0.on_receive_response_header(callback);
                    self
                }

                #[inline]
                #[doc = "设置域名解析前回调函数"]
                pub fn on_to_resolve_domain(
                    &mut self,
                    callback: impl Fn(
                            &mut dyn qiniu_http_client::CallbackContext,
                            &str,
                        ) -> anyhow::Result<()>
                            + Send
                            + Sync
                            + 'req,
                ) -> &mut Self {
                    self.0.on_to_resolve_domain(callback);
                    self
                }

                #[inline]
                #[doc = "设置域名解析成功回调函数"]
                pub fn on_domain_resolved(
                    &mut self,
                    callback: impl Fn(
                            &mut dyn qiniu_http_client::CallbackContext,
                            &str,
                            &qiniu_http_client::ResolveAnswers,
                        ) -> anyhow::Result<()>
                            + Send
                            + Sync
                            + 'req,
                    ) -> &mut Self {
                    self.0.on_domain_resolved(callback);
                    self
                }

                #[inline]
                #[doc = "设置 IP 地址选择前回调函数"]
                pub fn on_to_choose_ips(
                    &mut self,
                    callback: impl Fn(
                            &mut dyn qiniu_http_client::CallbackContext,
                            &[qiniu_http_client::IpAddrWithPort],
                        ) -> anyhow::Result<()>
                            + Send
                            + Sync
                            + 'req,
                ) -> &mut Self {
                    self.0.on_to_choose_ips(callback);
                    self
                }

                #[inline]
                #[doc = "设置 IP 地址选择成功回调函数"]
                pub fn on_ips_chosen(
                    &mut self,
                    callback: impl Fn(
                            &mut dyn qiniu_http_client::CallbackContext,
                            &[qiniu_http_client::IpAddrWithPort],
                            &[qiniu_http_client::IpAddrWithPort],
                        ) -> anyhow::Result<()>
                            + Send
                            + Sync
                            + 'req,
                ) -> &mut Self {
                    self.0.on_ips_chosen(callback);
                    self
                }

                #[inline]
                #[doc = "设置 HTTP 请求签名前回调函数"]
                pub fn on_before_request_signed(
                    &mut self,
                    callback: impl Fn(
                        &mut dyn qiniu_http_client::ExtendedCallbackContext,
                        ) -> anyhow::Result<()>
                            + Send
                            + Sync
                            + 'req,
                ) -> &mut Self {
                    self.0.on_before_request_signed(callback);
                    self
                }

                #[inline]
                #[doc = "设置 HTTP 请求前回调函数"]
                pub fn on_after_request_signed(
                    &mut self,
                    callback: impl Fn(
                            &mut dyn qiniu_http_client::ExtendedCallbackContext,
                        ) -> anyhow::Result<()>
                            + Send
                            + Sync
                            + 'req,
                ) -> &mut Self {
                    self.0.on_after_request_signed(callback);
                    self
                }

                #[inline]
                #[doc = "设置响应成功回调函数"]
                pub fn on_response(
                    &mut self,
                    callback: impl Fn(
                            &mut dyn qiniu_http_client::ExtendedCallbackContext,
                            &qiniu_http_client::http::ResponseParts,
                        ) -> anyhow::Result<()>
                            + Send
                            + Sync
                            + 'req,
                ) -> &mut Self {
                    self.0.on_response(callback);
                    self
                }

                #[inline]
                #[doc = "设置响应错误回调函数"]
                pub fn on_error(
                    &mut self,
                    callback: impl Fn(
                            &mut dyn qiniu_http_client::ExtendedCallbackContext,
                            &mut qiniu_http_client::ResponseError,
                        ) -> anyhow::Result<()>
                            + Send
                            + Sync
                            + 'req,
                ) -> &mut Self {
                    self.0.on_error(callback);
                    self
                }

                #[inline]
                #[doc = "设置退避前回调函数"]
                pub fn on_before_backoff(
                    &mut self,
                    callback: impl Fn(
                            &mut dyn qiniu_http_client::ExtendedCallbackContext,
                            std::time::Duration,
                        ) -> anyhow::Result<()>
                            + Send
                            + Sync
                            + 'req,
                ) -> &mut Self {
                    self.0.on_before_backoff(callback);
                    self
                }

                #[inline]
                #[doc = "设置退避后回调函数"]
                pub fn on_after_backoff(
                    &mut self,
                    callback: impl Fn(
                            &mut dyn qiniu_http_client::ExtendedCallbackContext,
                            std::time::Duration,
                        ) -> anyhow::Result<()>
                            + Send
                            + Sync
                            + 'req,
                ) -> &mut Self {
                    self.0.on_after_backoff(callback);
                    self
                }

                #[inline]
                #[doc = "获取 HTTP 请求构建器部分参数"]
                pub fn parts(&self) -> &qiniu_http_client::RequestBuilderParts<'req> {
                     self.0.parts()
                }

                #[inline]
                #[doc = "获取 HTTP 请求构建器部分参数的可变引用"]
                pub fn parts_mut(&mut self) -> &mut qiniu_http_client::RequestBuilderParts<'req> {
                     self.0.parts_mut()
                }
            }
        }

        fn sync_request_builder_methods_token_stream(description: &ApiDetailedDescription) -> TokenStream {
            let (call_params, set_body_call) = match &description.request.body {
                Some(RequestBody::Json(_)) => (Some(quote! {body: &RequestBody}), quote! {self.0.json(body)?}),
                Some(RequestBody::FormUrlencoded(_)) => {
                    (Some(quote! {body: RequestBody}), quote! {self.0.post_form(body)})
                }
                Some(RequestBody::MultipartFormData(_)) => (
                    Some(quote! {body: sync_part::RequestBody<'_>}),
                    quote! {self.0.multipart(body)?},
                ),
                Some(RequestBody::BinaryData) => (
                    Some(quote! {
                        body: impl std::io::Read
                            + qiniu_http_client::http::Reset
                            + std::fmt::Debug
                            + Send
                            + Sync
                            + 'static,
                        content_length: u64,
                    }),
                    quote! {
                        self.0.stream_as_body(body, content_length, None)
                    },
                ),
                Some(RequestBody::PlainText) => (
                    Some(quote! {
                        body: String,
                    }),
                    quote! {
                        self.0.bytes_as_body(body.into_bytes(), Some(mime::TEXT_PLAIN_UTF_8))
                    },
                ),
                None => (None, quote! {&mut self.0}),
            };
            let (response_body_type, parse_body_call) =
                if matches!(&description.response.body, Some(ResponseBody::BinaryDataStream)) {
                    (quote! {qiniu_http_client::SyncResponseBody}, quote! {response})
                } else {
                    (quote! {ResponseBody}, quote! {response.parse_json()?})
                };

            quote! {
                #[doc = "阻塞发起 HTTP 请求"]
                pub fn call(&mut self, #call_params) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<#response_body_type>> {
                    let request = #set_body_call;
                    let response = request.call()?;
                    let parsed = #parse_body_call;
                    Ok(parsed)
                }
            }
        }

        fn async_request_builder_methods_token_stream(description: &ApiDetailedDescription) -> TokenStream {
            let (call_params, set_body_call) = match &description.request.body {
                Some(RequestBody::Json(_)) => (Some(quote! {body: &RequestBody}), quote! {self.0.json(body)?}),
                Some(RequestBody::FormUrlencoded(_)) => {
                    (Some(quote! {body: RequestBody}), quote! {self.0.post_form(body)})
                }
                Some(RequestBody::MultipartFormData(_)) => (
                    Some(quote! {body: async_part::RequestBody<'_>}),
                    quote! {self.0.multipart(body).await?},
                ),
                Some(RequestBody::BinaryData) => (
                    Some(quote! {
                        body: impl futures::io::AsyncRead
                            + qiniu_http_client::http::AsyncReset
                            + Unpin
                            + std::fmt::Debug
                            + Send
                            + Sync
                            + 'static,
                        content_length: u64,
                    }),
                    quote! {
                        self.0.stream_as_body(body, content_length, None)
                    },
                ),
                Some(RequestBody::PlainText) => (
                    Some(quote! {
                        body: impl Into<String>,
                    }),
                    quote! {
                        self.0.bytes_as_body(body.into().into_bytes(), Some(mime::TEXT_PLAIN_UTF_8))
                    },
                ),
                None => (None, quote! {&mut self.0}),
            };
            let (response_body_type, parse_body_call) =
                if matches!(&description.response.body, Some(ResponseBody::BinaryDataStream)) {
                    (quote! {qiniu_http_client::AsyncResponseBody}, quote! {response})
                } else {
                    (quote! {ResponseBody}, quote! {response.parse_json().await?})
                };

            quote! {
                #[doc = "异步发起 HTTP 请求"]
                pub async fn call(&mut self, #call_params) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<#response_body_type>> {
                    let request = #set_body_call;
                    let response = request.call().await?;
                    let parsed = #parse_body_call;
                    Ok(parsed)
                }
            }
        }
    }
}

pub(super) fn api_client_definition_token_stream() -> TokenStream {
    mod_rs_client_definition_token_stream(&[])
}
