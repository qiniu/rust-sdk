use super::{enums::StringLikeType, traits::CodeGenerator};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 路径请求参数列表
pub(super) struct PathParams {
    /// HTTP URL 路径有名参数列表
    #[serde(skip_serializing_if = "Vec::is_empty")]
    named: Vec<NamedPathParam>,

    /// HTTP URL 路径自由参数列表
    #[serde(skip_serializing_if = "Option::is_none")]
    free: Option<FreePathParams>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 路径有名请求参数
struct NamedPathParam {
    /// HTTP URL 路径段落，如果为 None，则表示参数直接追加在 URL 路径末尾
    #[serde(skip_serializing_if = "Option::is_none")]
    path_segment: Option<String>,

    /// HTTP URL 路径参数名称
    field_name: String,

    /// HTTP URL 路径参数类型
    #[serde(rename = "type")]
    ty: StringLikeType,

    /// HTTP URL 路径参数文档
    documentation: String,

    /// HTTP URL 路径参数编码方式，如果为 None，表示直接转码成字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    encode: Option<EncodeType>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 路径自由请求参数
struct FreePathParams {
    /// HTTP URL 路径参数名称
    field_name: String,

    /// HTTP URL 路径参数文档
    documentation: String,

    /// HTTP URL 路径参数键编码方式，如果为 None，表示直接转码成字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    encode_param_key: Option<EncodeType>,

    /// HTTP URL 路径参数值编码方式，如果为 None，表示直接转码成字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    encode_param_value: Option<EncodeType>,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 字符串编码类型
enum EncodeType {
    /// 需要进行编码
    UrlSafeBase64,

    /// 需要可以将 None 编码
    UrlSafeBase64OrNone,
}

impl CodeGenerator for PathParams {
    fn to_rust_token_stream(&self, name: &str, documentation: &str) -> TokenStream {
        self.to_rust_token_stream_inner(name, documentation)
    }
}

impl PathParams {
    fn to_rust_token_stream_inner(&self, name: &str, documentation: &str) -> TokenStream {
        let name = format_ident!("{}", name.to_case(Case::Pascal));
        let struct_definition_token_stream = define_new_struct(&name, documentation, &self.named);

        let named_path_params_methods_token_stream = for_named_path_params(&self.named);
        let free_path_params_methods_token_stream = self.free.as_ref().map(for_free_path_params);
        return quote! {
            #struct_definition_token_stream
            impl #name {
                #named_path_params_methods_token_stream
                #free_path_params_methods_token_stream
            }
        };

        fn define_new_struct(name: &Ident, documentation: &str, named_path_params: &[NamedPathParam]) -> TokenStream {
            let named_fields: Vec<_> = named_path_params
                .iter()
                .map(|param| field_name_to_ident(&param.field_name))
                .collect();
            let concat_segments: Vec<_> = named_path_params
                .iter()
                .map(|param| concat_segments_token_stream(&format_ident!("all_segments"), param))
                .collect();

            quote! {
                #[derive(Debug, Clone, Default)]
                #[doc = #documentation]
                pub struct #name {
                    #(#named_fields: Option<std::borrow::Cow<'static, str>>,)*
                    extended_segments: Vec<std::borrow::Cow<'static, str>>,
                }

                impl #name {
                    #[inline]
                    #[must_use]
                    #[doc = "追加新的路径段"]
                    pub fn push_segment(mut self, segment: impl Into<std::borrow::Cow<'static, str>>) -> Self {
                        self.extended_segments.push(segment.into());
                        self
                    }

                    fn build(self) -> Vec<std::borrow::Cow<'static, str>> {
                        let mut all_segments: Vec<_> = Default::default();
                        #(#concat_segments)*
                        all_segments.extend(self.extended_segments);
                        all_segments
                    }
                }
            }
        }

        fn field_name_to_ident(field_name: &str) -> Ident {
            format_ident!("r#{}", field_name.to_case(Case::Snake))
        }

        fn concat_segments_token_stream(all_segments: &Ident, param: &NamedPathParam) -> TokenStream {
            let path_segment_push_token_stream = param.path_segment.as_ref().map(|path_segment| {
                quote! {
                    #all_segments.push(std::borrow::Cow::Borrowed(#path_segment));
                }
            });
            let field_name = field_name_to_ident(&param.field_name);
            match (param.ty, param.encode) {
                (StringLikeType::String, Some(EncodeType::UrlSafeBase64OrNone)) => {
                    quote! {
                        #path_segment_push_token_stream
                        #all_segments.push(
                            self.#field_name.unwrap_or(std::borrow::Cow::Borrowed("~")),
                        );
                    }
                }
                _ => {
                    quote! {
                        if let Some(segment) = self.#field_name {
                            #path_segment_push_token_stream
                            #all_segments.push(segment);
                        }
                    }
                }
            }
        }

        fn for_named_path_params(params: &[NamedPathParam]) -> TokenStream {
            let token_streams_for_fields: Vec<_> = params.iter().map(for_named_path_param).collect();
            quote! {
                #(#token_streams_for_fields)*
            }
        }

        fn for_named_path_param(param: &NamedPathParam) -> TokenStream {
            let field_name = field_name_to_ident(&param.field_name);
            let documentation = param.documentation.as_str();
            match &param.ty {
                StringLikeType::String => for_named_string_field(&field_name, documentation, param.encode),
                StringLikeType::Integer => for_named_based_field(
                    &field_name,
                    documentation,
                    &[
                        ("i8", &quote!(i8)),
                        ("i16", &quote!(i16)),
                        ("i32", &quote!(i32)),
                        ("i64", &quote!(i64)),
                        ("isize", &quote!(isize)),
                        ("u8", &quote!(u8)),
                        ("u16", &quote!(u16)),
                        ("u32", &quote!(u32)),
                        ("u64", &quote!(u64)),
                        ("usize", &quote!(usize)),
                    ],
                ),
                StringLikeType::Float => for_named_based_field(
                    &field_name,
                    documentation,
                    &[("f32", &quote!(f32)), ("f64", &quote!(f64))],
                ),
                StringLikeType::Boolean => {
                    for_named_based_field(&field_name, documentation, &[("bool", &quote!(bool))])
                }
            }
        }

        fn for_named_string_field(field_name: &Ident, documentation: &str, encode: Option<EncodeType>) -> TokenStream {
            let method_name = format_ident!("set_{}_as_str", field_name);
            let value_token_stream = if encode.is_some() {
                quote! {qiniu_utils::base64::urlsafe(value.into().as_bytes()).into()}
            } else {
                quote! {value.into()}
            };
            quote! {
                #[inline]
                #[must_use]
                #[doc = #documentation]
                pub fn #method_name(
                    mut self,
                    value: impl Into<std::borrow::Cow<'static, str>>,
                ) -> Self {
                    self.#field_name = Some(#value_token_stream);
                    self
                }
            }
        }

        fn for_named_based_field(
            field_name: &Ident,
            documentation: &str,
            pairs: &[(&str, &TokenStream)],
        ) -> TokenStream {
            let methods_token_streams: Vec<_> = pairs
                .iter()
                .map(|(type_name, rust_type)| {
                    let method_name = format_ident!("set_{}_as_{}", field_name, type_name);
                    quote! {
                        #[inline]
                        #[must_use]
                        #[doc = #documentation]
                        pub fn #method_name(mut self, value: #rust_type) -> Self {
                            self.#field_name = Some(value.to_string().into());
                            self
                        }
                    }
                })
                .collect();
            quote! {
                #(#methods_token_streams)*
            }
        }

        fn for_free_path_params(params: &FreePathParams) -> TokenStream {
            let field_name = field_name_to_ident(&params.field_name);
            let documentation = params.documentation.as_str();
            let encode_key_token_stream =
                encode_path_segment_token_stream(&format_ident!("{}", "key"), params.encode_param_key);
            let key_type_token_stream = path_segment_type_token_stream(params.encode_param_key);
            let encode_value_token_stream =
                encode_path_segment_token_stream(&format_ident!("{}", "value"), params.encode_param_value);
            let value_type_token_stream = path_segment_type_token_stream(params.encode_param_value);
            let method_name = format_ident!("append_{}_as_str", field_name);
            return quote! {
                #[inline]
                #[must_use]
                #[doc = #documentation]
                pub fn #method_name(
                    mut self,
                    key: #key_type_token_stream,
                    value: #value_type_token_stream,
                ) -> Self {
                    self.extended_segments.push(#encode_key_token_stream);
                    self.extended_segments.push(#encode_value_token_stream);
                    self
                }
            };

            fn path_segment_type_token_stream(encode: Option<EncodeType>) -> TokenStream {
                match encode {
                    Some(EncodeType::UrlSafeBase64OrNone) => {
                        quote! {Option<impl Into<std::borrow::Cow<'static, str>>>}
                    }
                    _ => {
                        quote! {impl Into<std::borrow::Cow<'static, str>>}
                    }
                }
            }

            fn encode_path_segment_token_stream(ident: &Ident, encode: Option<EncodeType>) -> TokenStream {
                match encode {
                    None => {
                        quote! { #ident.into() }
                    }
                    Some(EncodeType::UrlSafeBase64) => {
                        quote! {
                            qiniu_utils::base64::urlsafe(#ident.into().as_bytes()).into()
                        }
                    }
                    Some(EncodeType::UrlSafeBase64OrNone) => {
                        quote! {
                            if let Some(value) = #ident {
                                std::borrow::Cow::Owned(
                                    qiniu_utils::base64::urlsafe(value.into().as_bytes()),
                                )
                            } else {
                                std::borrow::Cow::Borrowed("~")
                            }
                        }
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
    fn test_path_param_types() -> Result<()> {
        let test_files = [write_token_stream(
            "TestPath",
            &PathParams {
                named: vec![
                    NamedPathParam {
                        field_name: "TestStringWithoutPathSegment".to_owned(),
                        path_segment: None,
                        documentation: "Fake named string field docs".to_owned(),
                        ty: StringLikeType::String,
                        encode: None,
                    },
                    NamedPathParam {
                        field_name: "TestString".to_owned(),
                        path_segment: Some("named_string_field".to_owned()),
                        documentation: "Fake named string field docs".to_owned(),
                        ty: StringLikeType::String,
                        encode: None,
                    },
                    NamedPathParam {
                        field_name: "TestEncodedString".to_owned(),
                        path_segment: Some("encoded_named_string_field".to_owned()),
                        documentation: "Fake encoded named string field docs".to_owned(),
                        ty: StringLikeType::String,
                        encode: Some(EncodeType::UrlSafeBase64),
                    },
                    NamedPathParam {
                        field_name: "TestOptionalEncodedString".to_owned(),
                        path_segment: Some("optional_encoded_named_string_field".to_owned()),
                        documentation: "Fake optional encoded named string field docs".to_owned(),
                        ty: StringLikeType::String,
                        encode: Some(EncodeType::UrlSafeBase64OrNone),
                    },
                    NamedPathParam {
                        field_name: "TestInteger".to_owned(),
                        path_segment: Some("named_integer_field".to_owned()),
                        documentation: "Fake named integer field docs".to_owned(),
                        ty: StringLikeType::Integer,
                        encode: None,
                    },
                    NamedPathParam {
                        field_name: "TestFloat".to_owned(),
                        path_segment: Some("named_float_field".to_owned()),
                        documentation: "Fake named float field docs".to_owned(),
                        ty: StringLikeType::Float,
                        encode: None,
                    },
                    NamedPathParam {
                        field_name: "TestBoolean".to_owned(),
                        path_segment: Some("named_boolean_field".to_owned()),
                        documentation: "Fake named boolean field docs".to_owned(),
                        ty: StringLikeType::Boolean,
                        encode: None,
                    },
                ],
                free: Some(FreePathParams {
                    field_name: "free_field".to_owned(),
                    documentation: "Fake free field docs".to_owned(),
                    encode_param_key: None,
                    encode_param_value: Some(EncodeType::UrlSafeBase64),
                }),
            }
            .to_rust_token_stream("TestPath", "Fake docs"),
        )?];

        let test_cases = TestCases::new();
        test_files.iter().for_each(|file| test_cases.pass(file));

        Ok(())
    }

    fn write_token_stream(name: &str, token_stream: &TokenStream) -> Result<NamedTempFile> {
        let mut file = TempFileBuilder::new()
            .prefix(&format!("{name}-"))
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
