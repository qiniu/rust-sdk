use super::traits::CodeGenerator;
use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(SmartDefault, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 复合表单字段请求类型
enum MultipartFormDataRequestType {
    /// 字符串（默认）
    #[default]
    String,

    /// 二进制数据
    BinaryData,

    /// 使用上传凭证鉴权
    UploadToken,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// 有名复合表单请求字段
struct NamedMultipartFormDataRequestField {
    /// 复合表单字段名称
    field_name: String,

    /// 复合表单参数名称
    key: String,

    /// 复合表单参数文档
    documentation: String,

    /// 复合表单参数类型
    #[serde(rename = "type")]
    ty: MultipartFormDataRequestType,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// 自由复合表单请求字段
struct FreeMultipartFormDataRequestFields {
    /// 复合表单参数名称
    field_name: String,

    /// 复合表单参数文档
    documentation: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// 复合表单请求结构体
pub(super) struct MultipartFormDataRequestStruct {
    /// 有名复合表单字段列表
    named_fields: Vec<NamedMultipartFormDataRequestField>,

    /// 自由复合表单字段列表
    #[serde(skip_serializing_if = "Option::is_none")]
    free_fields: Option<FreeMultipartFormDataRequestFields>,
}

impl CodeGenerator for MultipartFormDataRequestStruct {
    fn to_rust_token_stream(&self, name: &str, documentation: &str) -> TokenStream {
        let sync_token_stream = self.to_rust_token_stream_inner(name, documentation, true);
        let async_token_stream = self.to_rust_token_stream_inner(name, documentation, false);
        quote! {
            #[doc = "阻塞 Multipart 表单"]
            pub mod sync_part {
                #sync_token_stream
            }

            #[cfg(feature = "async")]
            #[doc = "异步 Multipart 表单"]
            pub mod async_part {
                #async_token_stream
            }
        }
    }
}

impl MultipartFormDataRequestStruct {
    fn to_rust_token_stream_inner(&self, name: &str, documentation: &str, sync_version: bool) -> TokenStream {
        let name = format_ident!("{}", name.to_case(Case::Pascal));
        let struct_definition_token_stream = define_new_struct(&name, documentation, sync_version);
        let named_fields_methods_token_stream = for_named_fields(&name, &self.named_fields, sync_version);
        let free_fields_methods_token_stream = self
            .free_fields
            .as_ref()
            .map(|free_fields| for_free_fields(&name, free_fields, sync_version));

        return quote! {
            #struct_definition_token_stream
            impl<'a> #name<'a> {
                #named_fields_methods_token_stream
                #free_fields_methods_token_stream
            }
        };

        fn for_named_fields(
            struct_name: &Ident,
            fields: &[NamedMultipartFormDataRequestField],
            sync_version: bool,
        ) -> TokenStream {
            let token_streams_for_fields: Vec<_> = fields
                .iter()
                .map(|field| for_named_field(struct_name, field, sync_version))
                .collect();
            quote! {
                #(#token_streams_for_fields)*
            }
        }

        fn for_named_field(
            struct_name: &Ident,
            field: &NamedMultipartFormDataRequestField,
            sync_version: bool,
        ) -> TokenStream {
            let field_name = format_ident!("{}", field.field_name.to_case(Case::Snake));
            let documentation = field.documentation.as_str();
            match &field.ty {
                MultipartFormDataRequestType::String => for_named_string_field(
                    struct_name,
                    &field_name,
                    documentation,
                    field.key.as_str(),
                    sync_version,
                ),
                MultipartFormDataRequestType::UploadToken => for_named_upload_token_field(
                    struct_name,
                    &field_name,
                    documentation,
                    field.key.as_str(),
                    sync_version,
                ),
                MultipartFormDataRequestType::BinaryData => {
                    let token_streams = [
                        for_named_binary_field(
                            struct_name,
                            &field_name,
                            documentation,
                            field.key.as_str(),
                            sync_version,
                        ),
                        for_named_bytes_field(
                            struct_name,
                            &field_name,
                            documentation,
                            field.key.as_str(),
                            sync_version,
                        ),
                        for_named_file_path_field(
                            struct_name,
                            &field_name,
                            documentation,
                            field.key.as_str(),
                            sync_version,
                        ),
                    ];
                    quote! {#(#token_streams)*}
                }
            }
        }

        fn for_named_string_field(
            struct_name: &Ident,
            field_name: &Ident,
            documentation: &str,
            key: &str,
            sync_version: bool,
        ) -> TokenStream {
            let method_name = format_ident!("set_{}", field_name);
            let part_type = if sync_version {
                quote! {qiniu_http_client::SyncPart}
            } else {
                quote! {qiniu_http_client::AsyncPart}
            };
            quote! {
                #[inline]
                #[must_use]
                #[doc = #documentation]
                pub fn #method_name(self, value: impl Into<std::borrow::Cow<'a, str>>) -> #struct_name<'a> {
                    self.add_part(#key, #part_type::text(value))
                }
            }
        }

        fn for_named_upload_token_field(
            struct_name: &Ident,
            field_name: &Ident,
            documentation: &str,
            key: &str,
            sync_version: bool,
        ) -> TokenStream {
            let method_name = format_ident!("set_{}", field_name);
            if sync_version {
                quote! {
                    #[inline]
                    #[doc = #documentation]
                    pub fn #method_name(
                        self,
                        token: &'a (dyn qiniu_http_client::upload_token::UploadTokenProvider + 'a),
                        opts: qiniu_http_client::upload_token::ToStringOptions,
                    ) -> qiniu_http_client::upload_token::ToStringResult<#struct_name<'a>> {
                        Ok(self.add_part(
                            #key,
                            qiniu_http_client::SyncPart::text(token.to_token_string(opts)?),
                        ))
                    }
                }
            } else {
                quote! {
                    #[inline]
                    #[doc = #documentation]
                    pub async fn #method_name(
                        self,
                        token: &'a (dyn qiniu_http_client::upload_token::UploadTokenProvider + 'a),
                        opts: qiniu_http_client::upload_token::ToStringOptions,
                    ) -> qiniu_http_client::upload_token::ToStringResult<#struct_name<'a>> {
                        Ok(self.add_part(
                            #key,
                            qiniu_http_client::AsyncPart::text(token.async_to_token_string(opts).await?),
                        ))
                    }
                }
            }
        }

        fn for_named_binary_field(
            struct_name: &Ident,
            field_name: &Ident,
            documentation: &str,
            key: &str,
            sync_version: bool,
        ) -> TokenStream {
            let method_name = format_ident!("set_{}_as_reader", field_name);
            if sync_version {
                quote! {
                    #[inline]
                    #[must_use]
                    #[doc = #documentation]
                    pub fn #method_name(
                        self,
                        reader: impl std::io::Read + 'a,
                        metadata: qiniu_http_client::PartMetadata,
                    ) -> #struct_name<'a> {
                        self.add_part(
                            #key,
                            qiniu_http_client::SyncPart::stream(reader).metadata(metadata),
                        )
                    }
                }
            } else {
                quote! {
                    #[inline]
                    #[must_use]
                    #[doc = #documentation]
                    pub fn #method_name(
                        self,
                        reader: impl futures::io::AsyncRead + Send + Unpin + 'a,
                        metadata: qiniu_http_client::PartMetadata,
                    ) -> #struct_name<'a> {
                        self.add_part(
                            #key,
                            qiniu_http_client::AsyncPart::stream(reader).metadata(metadata),
                        )
                    }
                }
            }
        }

        fn for_named_bytes_field(
            struct_name: &Ident,
            field_name: &Ident,
            documentation: &str,
            key: &str,
            sync_version: bool,
        ) -> TokenStream {
            let method_name = format_ident!("set_{}_as_bytes", field_name);
            let part_type = if sync_version {
                quote! {qiniu_http_client::SyncPart}
            } else {
                quote! {qiniu_http_client::AsyncPart}
            };
            quote! {
                #[inline]
                #[must_use]
                #[doc = #documentation]
                pub fn #method_name(
                    self,
                    bytes: impl Into<std::borrow::Cow<'a, [u8]>>,
                    metadata: qiniu_http_client::PartMetadata,
                ) -> #struct_name<'a> {
                    self.add_part(
                        #key,
                        #part_type::bytes(bytes).metadata(metadata),
                    )
                }
            }
        }

        fn for_named_file_path_field(
            struct_name: &Ident,
            field_name: &Ident,
            documentation: &str,
            key: &str,
            sync_version: bool,
        ) -> TokenStream {
            let method_name = format_ident!("set_{}_as_file_path", field_name);
            if sync_version {
                quote! {
                    #[inline]
                    #[doc = #documentation]
                    pub fn #method_name<S: AsRef<std::ffi::OsStr> + ?Sized>(
                        self,
                        path: &S,
                    ) -> std::io::Result<#struct_name<'a>> {
                        Ok(self.add_part(
                            #key,
                            qiniu_http_client::SyncPart::file_path(std::path::Path::new(path))?,
                        ))
                    }
                }
            } else {
                quote! {
                    #[inline]
                    #[doc = #documentation]
                    pub async fn #method_name<S: AsRef<std::ffi::OsStr> + ?Sized>(
                        self,
                        path: &S,
                    ) -> std::io::Result<#struct_name<'a>> {
                        Ok(self.add_part(
                            #key,
                            qiniu_http_client::AsyncPart::file_path(async_std::path::Path::new(path)).await?,
                        ))
                    }
                }
            }
        }

        fn for_free_fields(
            struct_name: &Ident,
            fields: &FreeMultipartFormDataRequestFields,
            sync_version: bool,
        ) -> TokenStream {
            let field_name = format_ident!("{}", fields.field_name.to_case(Case::Snake));
            let method_name = format_ident!("append_{}", field_name);
            let documentation = fields.documentation.as_str();
            let part_type = if sync_version {
                quote! {qiniu_http_client::SyncPart}
            } else {
                quote! {qiniu_http_client::AsyncPart}
            };
            quote! {
                #[inline]
                #[must_use]
                #[doc = #documentation]
                pub fn #method_name(
                    self,
                    key: impl Into<qiniu_http_client::FieldName>,
                    value: impl Into<std::borrow::Cow<'a, str>>,
                ) -> #struct_name<'a> {
                    self.add_part(key, #part_type::text(value))
                }
            }
        }

        fn define_new_struct(name: &Ident, documentation: &str, sync_version: bool) -> TokenStream {
            let (multipart_type, part_type) = if sync_version {
                (
                    quote!(qiniu_http_client::SyncMultipart),
                    quote!(qiniu_http_client::SyncPart),
                )
            } else {
                (
                    quote!(qiniu_http_client::AsyncMultipart),
                    quote!(qiniu_http_client::AsyncPart),
                )
            };
            quote! {
                #[derive(Debug, Default)]
                #[doc = #documentation]
                pub struct #name<'a> {
                    multipart: #multipart_type<'a>
                }

                impl<'a> #name<'a> {
                    #[inline]
                    #[must_use]
                    #[doc = "添加新的 Multipart 表单组件"]
                    pub fn add_part(
                        mut self,
                        name: impl Into<qiniu_http_client::FieldName>,
                        part: #part_type<'a>,
                    ) -> Self {
                        self.multipart = self.multipart.add_part(name.into(), part);
                        self
                    }

                    fn build(self) -> #multipart_type<'a> {
                        self.multipart
                    }
                }

                impl<'a> From<#name<'a>> for #multipart_type<'a> {
                    #[inline]
                    fn from(parts: #name<'a>) -> Self {
                        parts.build()
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::io::Write;
    use tempfile::{Builder as TempFileBuilder, NamedTempFile};
    use trybuild::TestCases;

    #[test]
    fn test_multipart_types() -> Result<()> {
        let test_files = [write_token_stream(
            "TestMultipart",
            &MultipartFormDataRequestStruct {
                named_fields: vec![
                    NamedMultipartFormDataRequestField {
                        field_name: "TestString".to_owned(),
                        key: "named_string_field".to_owned(),
                        documentation: "Fake named string field docs".to_owned(),
                        ty: MultipartFormDataRequestType::String,
                    },
                    NamedMultipartFormDataRequestField {
                        field_name: "TestUploadToken".to_owned(),
                        key: "named_upload_token_field".to_owned(),
                        documentation: "Fake named upload token field docs".to_owned(),
                        ty: MultipartFormDataRequestType::UploadToken,
                    },
                    NamedMultipartFormDataRequestField {
                        field_name: "TestBinary".to_owned(),
                        key: "named_binary_field".to_owned(),
                        documentation: "Fake named binary field docs".to_owned(),
                        ty: MultipartFormDataRequestType::BinaryData,
                    },
                ],
                free_fields: Some(FreeMultipartFormDataRequestFields {
                    field_name: "free_field".to_owned(),
                    documentation: "Fake free field docs".to_owned(),
                }),
            }
            .to_rust_token_stream("TestMultipart", "Fake docs"),
        )?];

        let test_cases = TestCases::new();
        test_files.iter().for_each(|file| test_cases.pass(file));

        Ok(())
    }

    fn write_token_stream(name: &str, token_stream: &TokenStream) -> Result<NamedTempFile> {
        let mut file = TempFileBuilder::new()
            .prefix(&format!("{}-", name))
            .suffix(".rs")
            .tempfile()?;
        let all_token_stream = quote! {
            #token_stream
            fn main() {
            }
        };
        file.write_all(all_token_stream.to_string().as_bytes())?;

        Ok(file)
    }
}
