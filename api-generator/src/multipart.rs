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
            pub mod sync_part {
                #sync_token_stream
            }

            #[cfg(feature = "async")]
            pub mod async_part {
                #async_token_stream
            }
        }
    }
}

impl MultipartFormDataRequestStruct {
    fn to_rust_token_stream_inner(
        &self,
        name: &str,
        documentation: &str,
        sync_version: bool,
    ) -> TokenStream {
        let name = format_ident!("{}", name.to_case(Case::Pascal));
        let struct_definition_token_stream = define_new_struct(&name, documentation, sync_version);
        let impls_named_fields_token_stream =
            for_named_fields(&self.named_fields, &name, sync_version);
        let impls_free_fields_token_stream = if let Some(free_fields) = &self.free_fields {
            for_free_fields(free_fields, &name, sync_version)
        } else {
            quote! {}
        };

        return quote! {
            #struct_definition_token_stream
            #impls_named_fields_token_stream
            #impls_free_fields_token_stream
        };

        fn for_named_fields(
            fields: &[NamedMultipartFormDataRequestField],
            name: &Ident,
            sync_version: bool,
        ) -> TokenStream {
            let token_streams_for_fields: Vec<_> = fields
                .iter()
                .map(|field| for_named_field(field, name, sync_version))
                .collect();
            quote! {
                #(#token_streams_for_fields)*
            }
        }

        fn for_named_field(
            field: &NamedMultipartFormDataRequestField,
            name: &Ident,
            sync_version: bool,
        ) -> TokenStream {
            let field_name = format_ident!("{}", field.field_name.to_case(Case::Snake));
            let documentation = field.documentation.as_str();
            let method_token_stream = match &field.ty {
                MultipartFormDataRequestType::String => for_named_string_field(
                    &field_name,
                    documentation,
                    field.key.as_str(),
                    sync_version,
                ),
                MultipartFormDataRequestType::UploadToken => for_named_upload_token_field(
                    &field_name,
                    documentation,
                    field.key.as_str(),
                    sync_version,
                ),
                MultipartFormDataRequestType::BinaryData => for_named_binary_field(
                    &field_name,
                    documentation,
                    field.key.as_str(),
                    sync_version,
                ),
            };
            quote! {
                impl #name {
                    #method_token_stream
                }
            }
        }

        fn for_named_string_field(
            field_name: &Ident,
            documentation: &str,
            key: &str,
            _sync_version: bool,
        ) -> TokenStream {
            quote! {
                #[inline]
                #[doc = #documentation]
                pub fn #field_name(&mut self, value: impl Into<std::borrow::Cow<'static, str>>) -> &mut Self {
                    self.parts.push((#key.into(), qiniu_http_client::Part::text(value)));
                    self
                }
            }
        }

        fn for_named_upload_token_field(
            field_name: &Ident,
            documentation: &str,
            key: &str,
            sync_version: bool,
        ) -> TokenStream {
            if sync_version {
                quote! {
                    #[inline]
                    #[doc = #documentation]
                    pub fn #field_name(
                        &mut self,
                        token: &dyn qiniu_upload_token::UploadTokenProvider,
                    ) -> std::io::Result<&mut Self> {
                        self.parts.push((
                            #key.into(),
                            qiniu_http_client::Part::text(String::from(
                                token.to_token_string(&Default::default())?,
                            )),
                        ));
                        Ok(self)
                    }
                }
            } else {
                quote! {
                    #[inline]
                    #[doc = #documentation]
                    pub async fn #field_name(
                        &mut self,
                        token: &dyn qiniu_upload_token::UploadTokenProvider,
                    ) -> std::io::Result<&mut Self> {
                        self.parts.push((
                            #key.into(),
                            qiniu_http_client::Part::text(String::from(
                                token.async_to_token_string(&Default::default()).await?,
                            )),
                        ));
                        Ok(self)
                    }
                }
            }
        }

        fn for_named_binary_field(
            field_name: &Ident,
            documentation: &str,
            key: &str,
            sync_version: bool,
        ) -> TokenStream {
            if sync_version {
                quote! {
                    #[inline]
                    #[doc = #documentation]
                    pub fn #field_name(
                        &mut self,
                        reader: Box<dyn std::io::Read>,
                        metadata: qiniu_http_client::PartMetadata,
                    ) -> &mut Self {
                        self.parts.push((
                            #key.into(),
                            qiniu_http_client::Part::stream(reader).metadata(metadata),
                        ));
                        self
                    }
                }
            } else {
                quote! {
                    #[inline]
                    #[doc = #documentation]
                    pub fn #field_name(
                        &mut self,
                        reader: Box<dyn futures::io::AsyncRead + Send + Unpin>,
                        metadata: qiniu_http_client::PartMetadata,
                    ) -> &mut Self {
                        self.parts.push((
                            #key.into(),
                            qiniu_http_client::Part::stream(reader).metadata(metadata),
                        ));
                        self
                    }
                }
            }
        }

        fn for_free_fields(
            fields: &FreeMultipartFormDataRequestFields,
            name: &Ident,
            _sync_version: bool,
        ) -> TokenStream {
            let field_name = format_ident!("{}", fields.field_name.to_case(Case::Snake));
            let documentation = fields.documentation.as_str();
            let method_token_stream = quote! {
                #[inline]
                #[doc = #documentation]
                pub fn #field_name(
                    &mut self,
                    key: impl Into<qiniu_http_client::FieldName>,
                    value: impl Into<std::borrow::Cow<'static, str>>,
                ) -> &mut Self {
                    self.parts.push((key.into(), qiniu_http_client::Part::text(value)));
                    self
                }
            };
            quote! {
                impl #name {
                    #method_token_stream
                }
            }
        }

        fn define_new_struct(name: &Ident, documentation: &str, sync_version: bool) -> TokenStream {
            let (part_type, multipart_type) = if sync_version {
                (
                    quote!(qiniu_http_client::SyncPart),
                    quote!(qiniu_http_client::SyncMultipart),
                )
            } else {
                (
                    quote!(qiniu_http_client::AsyncPart),
                    quote!(qiniu_http_client::AsyncMultipart),
                )
            };
            quote! {
                #[derive(Debug)]
                #[doc = #documentation]
                pub struct #name {
                    parts: Vec<(qiniu_http_client::FieldName, #part_type)>,
                }

                impl #name {
                    pub fn build(&mut self) -> #multipart_type {
                        let mut multipart: #multipart_type = Default::default();
                        for (name, part) in std::mem::take(&mut self.parts).into_iter() {
                            multipart = multipart.add_part(name, part);
                        }
                        multipart
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
