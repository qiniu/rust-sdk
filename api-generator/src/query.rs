use super::{enums::StringLikeType, traits::CodeGenerator};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 查询请求参数信息
struct QueryName {
    /// 参数名称
    field_name: String,

    /// HTTP URL 查询参数名称
    query_name: String,

    /// HTTP URL 查询参数文档
    documentation: String,

    /// HTTP URL 查询参数类型
    query_type: StringLikeType,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, transparent, rename_all = "snake_case")]
/// HTTP URL 查询请求结构体
pub(super) struct QueryNames(
    /// 七牛 API 调用 URL 查询参数列表
    Vec<QueryName>,
);

impl CodeGenerator for QueryNames {
    fn to_rust_token_stream(&self, name: &str, documentation: &str) -> TokenStream {
        self.to_rust_token_stream_inner(name, documentation)
    }
}

impl QueryNames {
    fn to_rust_token_stream_inner(&self, name: &str, documentation: &str) -> TokenStream {
        let name = format_ident!("{}", name.to_case(Case::Pascal));
        let struct_definition_token_stream = define_new_struct(&name, documentation);
        let query_fields_token_stream = for_query_fields(&self.0);

        return quote! {
            #struct_definition_token_stream
            impl<'a> #name<'a> {
                #query_fields_token_stream
            }
        };

        fn for_query_fields(query_names: &[QueryName]) -> TokenStream {
            let token_streams_for_fields: Vec<_> =
                query_names.iter().map(for_query_field).collect();
            quote! {
                #(#token_streams_for_fields)*
            }
        }

        fn for_query_field(query_name: &QueryName) -> TokenStream {
            let field_name = field_name_to_ident(&query_name.field_name);
            let documentation = query_name.documentation.as_str();
            match &query_name.query_type {
                StringLikeType::String => {
                    for_string_field(&field_name, &query_name.query_name, documentation)
                }
                StringLikeType::Integer => for_based_field(
                    &field_name,
                    documentation,
                    &query_name.query_name,
                    &[
                        ("int", &format_ident!("i64")),
                        ("uint", &format_ident!("u64")),
                    ],
                ),
                StringLikeType::Float => for_based_field(
                    &field_name,
                    documentation,
                    &query_name.query_name,
                    &[("float", &format_ident!("f64"))],
                ),
                StringLikeType::Boolean => for_based_field(
                    &field_name,
                    documentation,
                    &query_name.query_name,
                    &[("bool", &format_ident!("bool"))],
                ),
            }
        }

        fn for_string_field(
            field_name: &Ident,
            query_name: &str,
            documentation: &str,
        ) -> TokenStream {
            let method_name = format_ident!("set_{}_as_str", field_name);
            quote! {
                #[inline]
                #[doc = #documentation]
                pub fn #method_name(
                    self,
                    value: impl Into<qiniu_http_client::QueryPairValue<'a>>,
                ) -> Self {
                    self.insert(#query_name.into(), value.into())
                }
            }
        }

        fn for_based_field(
            field_name: &Ident,
            query_name: &str,
            documentation: &str,
            pairs: &[(&str, &Ident)],
        ) -> TokenStream {
            let methods_token_streams: Vec<_> = pairs
                .iter()
                .map(|(type_name, rust_type)| {
                    let method_name = format_ident!("set_{}_as_{}", field_name, type_name);
                    quote! {
                        #[inline]
                        #[doc = #documentation]
                        pub fn #method_name(self, value: #rust_type) -> Self {
                            self.insert(#query_name.into(), value.to_string().into())
                        }
                    }
                })
                .collect();
            quote! {
                #(#methods_token_streams)*
            }
        }

        fn define_new_struct(name: &Ident, documentation: &str) -> TokenStream {
            quote! {
                #[derive(Debug, Clone)]
                #[doc = #documentation]
                pub struct #name<'a> {
                    map: std::collections::HashMap<
                        qiniu_http_client::QueryPairKey<'a>,
                        qiniu_http_client::QueryPairValue<'a>,
                    >,
                }

                impl<'a> #name<'a> {
                    #[inline]
                    fn insert(
                        mut self,
                        query_pair_key: qiniu_http_client::QueryPairKey<'a>,
                        query_pair_value: qiniu_http_client::QueryPairValue<'a>,
                     ) -> Self {
                        self.map.insert(query_pair_key, query_pair_value);
                        self
                     }

                    #[inline]
                    fn build(self) -> qiniu_http_client::QueryPairs<'a> {
                        qiniu_http_client::QueryPairs::from_iter(self.map)
                    }
                }

                impl<'a> From<#name<'a>> for qiniu_http_client::QueryPairs<'a> {
                    #[inline]
                    fn from(map: #name<'a>) -> Self {
                        map.build()
                    }
                }
            }
        }

        fn field_name_to_ident(field_name: &str) -> Ident {
            format_ident!("{}", field_name.to_case(Case::Snake))
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
            "TestQuery",
            &QueryNames(vec![
                QueryName {
                    field_name: "TestString".to_owned(),
                    query_name: "test_string".to_owned(),
                    documentation: "Fake string form field docs".to_owned(),
                    query_type: StringLikeType::String,
                },
                QueryName {
                    field_name: "TestInteger".to_owned(),
                    query_name: "test_integer".to_owned(),
                    documentation: "Fake integer form field docs".to_owned(),
                    query_type: StringLikeType::Integer,
                },
                QueryName {
                    field_name: "TestFloat".to_owned(),
                    query_name: "test_float".to_owned(),
                    documentation: "Fake float form field docs".to_owned(),
                    query_type: StringLikeType::Float,
                },
                QueryName {
                    field_name: "TestBoolean".to_owned(),
                    query_name: "test_boolean".to_owned(),
                    documentation: "Fake boolean form field docs".to_owned(),
                    query_type: StringLikeType::Boolean,
                },
            ])
            .to_rust_token_stream("TestQuery", "Fake docs"),
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
