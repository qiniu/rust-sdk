use super::traits::CodeGenerator;
use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP 头参数信息
struct HeaderName {
    /// HTTP 头参数名称
    field_name: String,

    /// HTTP 头名称
    header_name: String,

    /// HTTP 头参数文档
    documentation: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, transparent, rename_all = "snake_case")]
/// HTTP 头信息参数结构体
pub(super) struct HeaderNames(
    /// HTTP 请求/响应 HTTP 头信息参数列表
    Vec<HeaderName>,
);

impl CodeGenerator for HeaderNames {
    fn to_rust_token_stream(&self, name: &str, documentation: &str) -> TokenStream {
        self.to_rust_token_stream_inner(name, documentation)
    }
}

impl HeaderNames {
    fn to_rust_token_stream_inner(&self, name: &str, documentation: &str) -> TokenStream {
        let name = format_ident!("{}", name.to_case(Case::Pascal));
        let struct_definition_token_stream = define_new_struct(&name, documentation);
        let header_fields_token_stream = for_header_fields(&self.0);

        return quote! {
            #struct_definition_token_stream
            impl #name {
                #header_fields_token_stream
            }
        };

        fn for_header_fields(header_names: &[HeaderName]) -> TokenStream {
            let token_streams_for_fields: Vec<_> = header_names.iter().map(for_header_field).collect();
            quote! {
                #(#token_streams_for_fields)*
            }
        }

        fn for_header_field(header: &HeaderName) -> TokenStream {
            let field_name = field_name_to_ident(&header.field_name);
            let method_name = format_ident!("set_{}", field_name);
            let documentation = header.documentation.as_str();
            let header_name = header.header_name.as_str();
            quote! {
                #[inline]
                #[must_use]
                #[doc = #documentation]
                pub fn #method_name(
                    self,
                    value: impl Into<qiniu_http_client::http::header::HeaderValue>,
                ) -> Self {
                    self.insert(
                        qiniu_http_client::http::header::HeaderName::from_bytes(#header_name.as_bytes()).unwrap(),
                        value.into(),
                    )
                }
            }
        }

        fn define_new_struct(name: &Ident, documentation: &str) -> TokenStream {
            quote! {
                #[derive(Debug, Clone, Default)]
                #[doc = #documentation]
                pub struct #name {
                    map: qiniu_http_client::http::header::HeaderMap,
                }

                impl #name {
                    #[inline]
                    #[must_use]
                    #[doc = "插入 HTTP 头参数"]
                    pub fn insert(
                        mut self,
                        header_name: qiniu_http_client::http::header::HeaderName,
                        header_value: qiniu_http_client::http::header::HeaderValue,
                     ) -> Self {
                        self.map.insert(header_name, header_value);
                        self
                     }
                }

                impl<'a> From<#name> for std::borrow::Cow<'a, qiniu_http_client::http::header::HeaderMap> {
                    #[inline]
                    fn from(m: #name) -> Self {
                        std::borrow::Cow::Owned(m.map)
                    }
                }

                impl<'a> From<&'a #name> for std::borrow::Cow<'a, qiniu_http_client::http::header::HeaderMap> {
                    #[inline]
                    fn from(m: &'a #name) -> Self {
                        std::borrow::Cow::Borrowed(&m.map)
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
    fn test_header_field_types() -> Result<()> {
        let test_files = [write_token_stream(
            "TestHeader",
            &HeaderNames(vec![HeaderName {
                field_name: "TestString".to_owned(),
                header_name: "x-test-string".to_owned(),
                documentation: "Fake string form field docs".to_owned(),
            }])
            .to_rust_token_stream("TestHeader", "Fake docs"),
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
