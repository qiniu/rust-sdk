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

    /// 文本文件调用
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
        let sync_request_builder_definition_token_stream =
            sync_request_builder_definition_token_stream();
        let async_request_builder_definition_token_stream =
            async_request_builder_definition_token_stream();
        let new_request_method_token_stream = new_request_method_token_stream(self);
        let new_async_request_method_token_stream = new_async_request_method_token_stream(self);
        let path_params_token_stream = self.request.path_params.as_ref().map(|path_params| {
            path_params.to_rust_token_stream("PathParams", "调用 API 所用的路径参数")
        });
        let query_names_token_stream = self.request.query_names.as_ref().map(|query_names| {
            query_names.to_rust_token_stream("QueryParams", "调用 API 所用的 URL 查询参数")
        });
        let request_header_names_token_stream =
            self.request.header_names.as_ref().map(|header_names| {
                header_names.to_rust_token_stream("RequestHeaders", "调用 API 所用的 HTTP 头参数")
            });
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
            Some(RequestBody::MultipartFormData(multipart_struct)) => Some(
                multipart_struct.to_rust_token_stream("RequestBody", "调用 API 所用的请求体参数"),
            ),
            _ => None,
        };
        let response_body_token_stream = match &self.response.body {
            Some(ResponseBody::Json(json_struct)) => {
                Some(json_struct.to_rust_token_stream("ResponseBody", "获取 API 所用的响应体参数"))
            }
            None => Some(
                JsonType::default()
                    .to_rust_token_stream("ResponseBody", "获取 API 所用的响应体参数"),
            ),
            Some(ResponseBody::BinaryDataStream) => None,
        };
        let request_builder_methods_token_stream = request_builder_methods_token_stream(self);
        let sync_request_builder_methods_token_stream =
            sync_request_builder_methods_token_stream(self);
        let async_request_builder_methods_token_stream =
            async_request_builder_methods_token_stream(self);

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

            #sync_request_builder_definition_token_stream
            #async_request_builder_definition_token_stream

            impl<'req> SyncRequestBuilder<'req> {
                #request_builder_methods_token_stream
                #sync_request_builder_methods_token_stream
            }

            #[cfg(feature = "async")]
            impl<'req> AsyncRequestBuilder<'req> {
                #request_builder_methods_token_stream
                #async_request_builder_methods_token_stream
            }
        };

        fn sync_request_builder_definition_token_stream() -> TokenStream {
            quote! {
                #[derive(Debug)]
                pub struct SyncRequestBuilder<'req>(qiniu_http_client::SyncRequestBuilder<'req>);
            }
        }

        fn async_request_builder_definition_token_stream() -> TokenStream {
            quote! {
                #[derive(Debug)]
                #[cfg(feature = "async")]
                #[cfg_attr(feature = "docs", doc(cfg(feature = "async")))]
                pub struct AsyncRequestBuilder<'req>(qiniu_http_client::AsyncRequestBuilder<'req>);
            }
        }

        fn new_request_method_token_stream(description: &ApiDetailedDescription) -> TokenStream {
            let new_request_impl = impl_request_method_token_stream(
                description,
                &quote! {into_endpoints.into()},
                true,
            );
            let new_request_params_token_stream = new_request_params_token_stream(description);
            quote! {
                #[inline]
                pub fn new_request(
                    &self,
                    into_endpoints: impl Into<qiniu_http_client::IntoEndpoints<'client>>,
                    #new_request_params_token_stream
                ) -> SyncRequestBuilder {
                    SyncRequestBuilder(#new_request_impl)
                }
            }
        }

        fn new_async_request_method_token_stream(
            description: &ApiDetailedDescription,
        ) -> TokenStream {
            let new_async_request_impl = impl_request_method_token_stream(
                description,
                &quote! {into_endpoints.into()},
                false,
            );
            let new_request_params_token_stream = new_request_params_token_stream(description);
            quote! {
                #[inline]
                #[cfg(feature = "async")]
                pub fn new_async_request(
                    &self,
                    into_endpoints: impl Into<qiniu_http_client::IntoEndpoints<'client>>,
                    #new_request_params_token_stream
                ) -> AsyncRequestBuilder {
                    AsyncRequestBuilder(#new_async_request_impl)
                }
            }
        }

        fn impl_request_method_token_stream(
            description: &ApiDetailedDescription,
            into_endpoints: &TokenStream,
            sync_version: bool,
        ) -> TokenStream {
            let mut method_calls: Vec<TokenStream> = vec![quote!(self), quote!(0)];
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
                method_calls.push(quote! {#method_name(#service_names, #into_endpoints)});
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
                method_calls.push(quote! {authorization(#authorization)});
            }

            let idempotent = match &description.request.idempotent {
                Idempotent::Always => quote! {qiniu_http_client::Idempotent::Always},
                Idempotent::Default => quote! {qiniu_http_client::Idempotent::Default},
                Idempotent::Never => quote! {qiniu_http_client::Idempotent::Never},
            };
            method_calls.push(quote! {idempotent(#idempotent)});

            {
                let base_path = description.base_path.as_str();
                let path_suffix = description.path_suffix.as_str();
                let path_params_ident = format_ident!("path_params");
                let path_call_token_stream = if description.request.path_params.is_some() {
                    quote! {
                        path(crate::base_utils::join_path(#base_path, #path_suffix, #path_params_ident.build()))
                    }
                } else {
                    let path = join_path(base_path, path_suffix, vec![]);
                    quote! {
                        path(#path)
                    }
                };
                method_calls.push(path_call_token_stream);
            }

            if matches!(
                &description.response.body,
                Some(ResponseBody::BinaryDataStream),
            ) {
                method_calls.push(quote! {accept_application_octet_stream()});
            } else {
                method_calls.push(quote! {accept_json()});
            }

            quote! {#(#method_calls).*}
        }

        fn new_request_params_token_stream(description: &ApiDetailedDescription) -> TokenStream {
            let mut params: Vec<_> = vec![];
            if description.request.path_params.is_some() {
                params.push(quote! {path_params: PathParams});
            };
            if let Some(authorization) = &description.request.authorization {
                match authorization {
                    Authorization::Qbox | Authorization::Qiniu => {
                        params.push(quote! {
                            credential: Box<dyn qiniu_http_client::credential::CredentialProvider>
                        });
                    }
                    Authorization::UploadToken => {
                        params.push(quote! {
                            upload_token: Box<dyn qiniu_http_client::upload_token::UploadTokenProvider>
                        });
                    }
                }
            }
            quote! {#(#params),*}
        }

        fn request_builder_methods_token_stream(
            description: &ApiDetailedDescription,
        ) -> TokenStream {
            let headers_type_token_stream = description
                .request
                .header_names
                .as_ref()
                .map(|_| {
                    quote! {&'req RequestHeaders}
                })
                .unwrap_or_else(|| {
                    quote! {impl Into<std::borrow::Cow<'req, qiniu_http_client::http::HeaderMap>>}
                });
            let query_pairs_type_token_stream = description
                .request
                .query_names
                .as_ref()
                .map(|_| {
                    quote! {QueryParams<'req>}
                })
                .unwrap_or_else(|| {
                    quote! {impl Into<qiniu_http_client::QueryPairs<'req>>}
                });
            quote! {
                #[inline]
                pub fn use_https(mut self, use_https: bool) -> Self {
                    self.0 = self.0.use_https(use_https);
                    self
                }

                #[inline]
                pub fn version(mut self, version: qiniu_http_client::http::Version) -> Self {
                    self.0 = self.0.version(version);
                    self
                }

                #[inline]
                pub fn headers(mut self, headers: #headers_type_token_stream) -> Self {
                    self.0 = self.0.headers(headers);
                    self
                }

                #[inline]
                pub fn query_pairs(mut self, query_pairs: #query_pairs_type_token_stream) -> Self {
                    self.0 = self.0.query_pairs(query_pairs);
                    self
                }

                #[inline]
                pub fn extensions(mut self, extensions: qiniu_http_client::http::Extensions) -> Self {
                    self.0 = self.0.extensions(extensions);
                    self
                }

                #[inline]
                pub fn add_extension<T: Send + Sync + 'static>(mut self, val: T) -> Self {
                    self.0 = self.0.add_extension(val);
                    self
                }

                #[inline]
                pub fn on_uploading_progress(mut self, callback: qiniu_http_client::OnProgress) -> Self {
                    self.0 = self.0.on_uploading_progress(callback);
                    self
                }

                #[inline]
                pub fn on_receive_response_status(mut self, callback: qiniu_http_client::OnStatusCode) -> Self {
                    self.0 = self.0.on_receive_response_status(callback);
                    self
                }

                #[inline]
                pub fn on_receive_response_header(mut self, callback: qiniu_http_client::OnHeader) -> Self {
                    self.0 = self.0.on_receive_response_header(callback);
                    self
                }

                #[inline]
                pub fn on_to_resolve_domain(mut self, callback: qiniu_http_client::OnToResolveDomain) -> Self {
                    self.0 = self.0.on_to_resolve_domain(callback);
                    self
                }

                #[inline]
                pub fn on_domain_resolved(mut self, callback: qiniu_http_client::OnDomainResolved) -> Self {
                    self.0 = self.0.on_domain_resolved(callback);
                    self
                }

                #[inline]
                pub fn on_to_choose_ips(mut self, callback: qiniu_http_client::OnToChooseIPs) -> Self {
                    self.0 = self.0.on_to_choose_ips(callback);
                    self
                }

                #[inline]
                pub fn on_ips_chosen(mut self, callback: qiniu_http_client::OnIPsChosen) -> Self {
                    self.0 = self.0.on_ips_chosen(callback);
                    self
                }

                #[inline]
                pub fn on_before_request_signed(mut self, callback: qiniu_http_client::OnRequest) -> Self {
                    self.0 = self.0.on_before_request_signed(callback);
                    self
                }

                #[inline]
                pub fn on_after_request_signed(mut self, callback: qiniu_http_client::OnRequest) -> Self {
                    self.0 = self.0.on_after_request_signed(callback);
                    self
                }

                #[inline]
                pub fn on_success(mut self, callback: qiniu_http_client::OnSuccess) -> Self {
                    self.0 = self.0.on_success(callback);
                    self
                }

                #[inline]
                pub fn on_error(mut self, callback: qiniu_http_client::OnError) -> Self {
                    self.0 = self.0.on_error(callback);
                    self
                }

                #[inline]
                pub fn on_before_backoff(mut self, callback: qiniu_http_client::OnRetry) -> Self {
                    self.0 = self.0.on_before_backoff(callback);
                    self
                }

                #[inline]
                pub fn on_after_backoff(mut self, callback: qiniu_http_client::OnRetry) -> Self {
                    self.0 = self.0.on_after_backoff(callback);
                    self
                }
            }
        }

        fn sync_request_builder_methods_token_stream(
            description: &ApiDetailedDescription,
        ) -> TokenStream {
            let (call_params, set_body_call) = match &description.request.body {
                Some(RequestBody::Json(_)) => (
                    Some(quote! {body: &RequestBody<'_>}),
                    quote! {self.0.json(body)?},
                ),
                Some(RequestBody::FormUrlencoded(_)) => (
                    Some(quote! {body: RequestBody}),
                    quote! {self.0.post_form(body)},
                ),
                Some(RequestBody::MultipartFormData(_)) => (
                    Some(quote! {body: sync_part::RequestBody}),
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
                None => (None, quote! {self.0}),
            };
            let (response_body_type, parse_body_call) = if matches!(
                &description.response.body,
                Some(ResponseBody::BinaryDataStream)
            ) {
                (
                    quote! {qiniu_http_client::SyncResponseBody},
                    quote! {response},
                )
            } else {
                (
                    quote! {ResponseBody<'static>},
                    quote! {response.parse_json()?},
                )
            };

            quote! {
                pub fn call(self, #call_params) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<#response_body_type>> {
                    let request = #set_body_call;
                    let response = request.call()?;
                    let parsed = #parse_body_call;
                    Ok(parsed)
                }
            }
        }

        fn async_request_builder_methods_token_stream(
            description: &ApiDetailedDescription,
        ) -> TokenStream {
            let (call_params, set_body_call) = match &description.request.body {
                Some(RequestBody::Json(_)) => (
                    Some(quote! {body: &RequestBody<'_>}),
                    quote! {self.0.json(body)?},
                ),
                Some(RequestBody::FormUrlencoded(_)) => (
                    Some(quote! {body: RequestBody}),
                    quote! {self.0.post_form(body)},
                ),
                Some(RequestBody::MultipartFormData(_)) => (
                    Some(quote! {body: async_part::RequestBody}),
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
                None => (None, quote! {self.0}),
            };
            let (response_body_type, parse_body_call) = if matches!(
                &description.response.body,
                Some(ResponseBody::BinaryDataStream)
            ) {
                (
                    quote! {qiniu_http_client::AsyncResponseBody},
                    quote! {response},
                )
            } else {
                (
                    quote! {ResponseBody<'static>},
                    quote! {response.parse_json().await?},
                )
            };

            quote! {
                pub async fn call(self, #call_params) -> qiniu_http_client::ApiResult<qiniu_http_client::Response<#response_body_type>> {
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
