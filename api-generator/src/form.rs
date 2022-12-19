use super::{enums::StringLikeType, traits::CodeGenerator};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// URL 编码表单请求字段
struct FormUrlencodedRequestField {
    /// URL 编码表单字段名称
    field_name: String,

    /// URL 编码表单参数名称
    key: String,

    /// URL 编码表单参数文档
    documentation: String,

    /// URL 编码表单参数类型
    #[serde(rename = "type")]
    ty: StringLikeType,

    /// URL 编码表单参数是否可以有多个值
    multiple: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default, rename_all = "snake_case")]
/// URL 编码表单请求结构体
pub(super) struct FormUrlencodedRequestStruct {
    /// URL 编码表单字段列表
    fields: Vec<FormUrlencodedRequestField>,
}

impl CodeGenerator for FormUrlencodedRequestStruct {
    fn to_rust_token_stream(&self, name: &str, documentation: &str) -> TokenStream {
        self.to_rust_token_stream_inner(name, documentation)
    }
}

impl FormUrlencodedRequestStruct {
    fn to_rust_token_stream_inner(&self, name: &str, documentation: &str) -> TokenStream {
        let name = format_ident!("{}", name.to_case(Case::Pascal));
        let struct_definition_token_stream = define_new_struct(&name, documentation, &self.fields);
        let form_fields_token_stream = for_form_fields(&self.fields);

        return quote! {
            #struct_definition_token_stream
            impl #name {
                #form_fields_token_stream
            }
        };

        fn define_new_struct(name: &Ident, documentation: &str, fields: &[FormUrlencodedRequestField]) -> TokenStream {
            let field_definitions: Vec<_> = fields
                .iter()
                .map(|param| field_definition_token_stream(&field_name_to_ident(&param.field_name), param.multiple))
                .collect();
            let append_pairs: Vec<_> = fields
                .iter()
                .map(|param| append_pairs_token_stream(&format_ident!("all_pairs"), param))
                .collect();

            quote! {
                #[derive(Debug, Default)]
                #[doc = #documentation]
                pub struct #name {
                    #(#field_definitions,)*
                    extended_pairs: Vec<(std::borrow::Cow<'static, str>, Option<std::borrow::Cow<'static, str>>)>,
                }

                impl #name {
                    #[inline]
                    #[must_use]
                    #[doc = "添加新的表单项"]
                    pub fn append_pair(
                        mut self,
                        key: impl Into<std::borrow::Cow<'static, str>>,
                        value: impl Into<std::borrow::Cow<'static, str>>,
                    ) -> Self {
                        self.extended_pairs.push((key.into(), Some(value.into())));
                        self
                    }

                    fn build(
                        self,
                    ) -> Vec<(std::borrow::Cow<'static, str>, Option<std::borrow::Cow<'static, str>>)> {
                        let mut all_pairs: Vec<_> = Default::default();
                        #(#append_pairs)*
                        all_pairs.extend(self.extended_pairs);
                        all_pairs
                    }
                }

                impl IntoIterator for #name {
                    type Item = (std::borrow::Cow<'static, str>, Option<std::borrow::Cow<'static, str>>);
                    type IntoIter = std::vec::IntoIter<Self::Item>;

                    #[inline]
                    fn into_iter(self) -> Self::IntoIter {
                        self.build().into_iter()
                    }
                }
            }
        }

        fn field_name_to_ident(field_name: &str) -> Ident {
            format_ident!("r#{}", field_name.to_case(Case::Snake))
        }

        fn field_definition_token_stream(field_name: &Ident, multi: bool) -> TokenStream {
            if multi {
                quote! {#field_name: Vec<std::borrow::Cow<'static, str>>}
            } else {
                quote! {#field_name: Option<std::borrow::Cow<'static, str>>}
            }
        }

        fn append_pairs_token_stream(all_pairs: &Ident, field: &FormUrlencodedRequestField) -> TokenStream {
            let field_name = field_name_to_ident(&field.field_name);
            let key = &field.key;
            if field.multiple {
                quote! {
                    for value in self.#field_name.into_iter() {
                        #all_pairs.push((#key.into(), Some(value)));
                    }
                }
            } else {
                quote! {
                    if let Some(value) = self.#field_name {
                        #all_pairs.push((#key.into(), Some(value)));
                    }
                }
            }
        }

        fn for_form_fields(fields: &[FormUrlencodedRequestField]) -> TokenStream {
            let token_streams_for_fields: Vec<_> = fields.iter().map(for_form_field).collect();
            quote! {
                #(#token_streams_for_fields)*
            }
        }

        fn for_form_field(field: &FormUrlencodedRequestField) -> TokenStream {
            let field_name = field_name_to_ident(&field.field_name);
            let documentation = field.documentation.as_str();
            match &field.ty {
                StringLikeType::String => for_string_field(&field_name, documentation, field.multiple),
                StringLikeType::Integer => for_based_field(
                    &field_name,
                    documentation,
                    field.multiple,
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
                StringLikeType::Float => for_based_field(
                    &field_name,
                    documentation,
                    field.multiple,
                    &[("f32", &quote!(f32)), ("f64", &quote!(f64))],
                ),
                StringLikeType::Boolean => {
                    for_based_field(&field_name, documentation, field.multiple, &[("bool", &quote!(bool))])
                }
            }
        }

        fn for_string_field(field_name: &Ident, documentation: &str, multi: bool) -> TokenStream {
            if multi {
                let method_name = format_ident!("append_{}_as_str", field_name);
                quote! {
                    #[inline]
                    #[must_use]
                    #[doc = #documentation]
                    pub fn #method_name(mut self, value: impl Into<std::borrow::Cow<'static, str>>) -> Self {
                        self.#field_name.push(value.into());
                        self
                    }
                }
            } else {
                let method_name = format_ident!("set_{}_as_str", field_name);
                quote! {
                    #[inline]
                    #[must_use]
                    #[doc = #documentation]
                    pub fn #method_name(mut self, value: impl Into<std::borrow::Cow<'static, str>>) -> Self {
                        self.#field_name = Some(value.into());
                        self
                    }
                }
            }
        }

        fn for_based_field(
            field_name: &Ident,
            documentation: &str,
            multi: bool,
            pairs: &[(&str, &TokenStream)],
        ) -> TokenStream {
            let methods_token_streams: Vec<_> = pairs
                .iter()
                .map(|(type_name, rust_type)| {
                    if multi {
                        let method_name = format_ident!("append_{}_as_{}", field_name, type_name);
                        quote! {
                            #[inline]
                            #[must_use]
                            #[doc = #documentation]
                            pub fn #method_name(mut self, value: #rust_type) -> Self {
                                self.#field_name.push(value.to_string().into());
                                self
                            }
                        }
                    } else {
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
                    }
                })
                .collect();
            quote! {
                #(#methods_token_streams)*
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
    fn test_form_fields_types() -> Result<()> {
        let test_files = [write_token_stream(
            "TestForm",
            &FormUrlencodedRequestStruct {
                fields: vec![
                    FormUrlencodedRequestField {
                        field_name: "TestString".to_owned(),
                        key: "test_string".to_owned(),
                        documentation: "Fake string form field docs".to_owned(),
                        ty: StringLikeType::String,
                        multiple: false,
                    },
                    FormUrlencodedRequestField {
                        field_name: "TestMultiString".to_owned(),
                        key: "test_multi_string".to_owned(),
                        documentation: "Fake multiple string form field docs".to_owned(),
                        ty: StringLikeType::String,
                        multiple: true,
                    },
                    FormUrlencodedRequestField {
                        field_name: "TestInteger".to_owned(),
                        key: "test_integer".to_owned(),
                        documentation: "Fake integer form field docs".to_owned(),
                        ty: StringLikeType::Integer,
                        multiple: false,
                    },
                    FormUrlencodedRequestField {
                        field_name: "TestMultiInteger".to_owned(),
                        key: "test_multi_integer".to_owned(),
                        documentation: "Fake multiple integer form field docs".to_owned(),
                        ty: StringLikeType::Integer,
                        multiple: true,
                    },
                    FormUrlencodedRequestField {
                        field_name: "TestFloat".to_owned(),
                        key: "test_float".to_owned(),
                        documentation: "Fake float form field docs".to_owned(),
                        ty: StringLikeType::Float,
                        multiple: false,
                    },
                    FormUrlencodedRequestField {
                        field_name: "TestMultiFloat".to_owned(),
                        key: "test_multi_float".to_owned(),
                        documentation: "Fake multiple float form field docs".to_owned(),
                        ty: StringLikeType::Float,
                        multiple: true,
                    },
                    FormUrlencodedRequestField {
                        field_name: "TestBoolean".to_owned(),
                        key: "test_boolean".to_owned(),
                        documentation: "Fake boolean form field docs".to_owned(),
                        ty: StringLikeType::Boolean,
                        multiple: false,
                    },
                    FormUrlencodedRequestField {
                        field_name: "TestMultiBoolean".to_owned(),
                        key: "test_multi_boolean".to_owned(),
                        documentation: "Fake multiple boolean form field docs".to_owned(),
                        ty: StringLikeType::Boolean,
                        multiple: true,
                    },
                ],
            }
            .to_rust_token_stream("TestForm", "Fake docs"),
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
