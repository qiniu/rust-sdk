use super::traits::CodeGenerator;
use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(SmartDefault, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// JSON 字段类型
pub(super) enum JsonType {
    /// 字符串（默认）
    String,

    /// 整型数字
    Integer,

    /// 浮点型数字
    Float,

    /// 布尔值
    Boolean,

    /// 数组
    Array(Box<JsonArray>),

    /// 结构体
    #[default]
    Struct(JsonStruct),

    /// 任意数据结构
    Any,

    /// 任意字符串映射结构
    StringMap,
}

impl CodeGenerator for JsonType {
    fn to_rust_token_stream(&self, name: &str, documentation: &str) -> TokenStream {
        self.to_rust_token_stream_inner(name, documentation)
    }
}

impl JsonType {
    fn to_rust_token_stream_inner(&self, name: &str, documentation: &str) -> TokenStream {
        let name = format_ident!("{}", name.to_case(Case::Pascal));
        let type_definition_token_stream = define_new_struct(
            &name,
            documentation,
            match self {
                Self::Array(_) => Some(JsonCollectionType::Array),
                Self::Struct(_) | Self::StringMap => Some(JsonCollectionType::Object),
                _ => None,
            },
        );
        let impls_token_stream = match self {
            Self::String => Some(impls_token_stream_for_string(&name)),
            Self::Integer => Some(impls_token_stream_for_int(&name)),
            Self::Float => Some(impls_token_stream_for_float(&name)),
            Self::Boolean => Some(impls_token_stream_for_bool(&name)),
            Self::Array(info) => Some(impls_token_stream_for_array(&name, &info.ty)),
            Self::Struct(info) => Some(impls_token_stream_for_struct(&name, &info.fields)),
            Self::Any => None,
            Self::StringMap => {
                return quote! { type #name = crate::base_types::StringMap; };
            }
        };

        return quote! {
            #type_definition_token_stream
            #impls_token_stream
        };

        fn impls_token_stream_for_string(name: &Ident) -> TokenStream {
            let as_method_token_stream =
                impl_as_method_for_base_types(name, &quote!(&str), &quote!(as_str), "解析 JSON 得到 String");
            let set_method_token_stream =
                impl_set_method_for_base_types(name, &quote!(String), &quote!(set_str), "设置 JSON String 值");
            let from_trait_token_stream = impl_from_trait_for_base_types(name, &quote!(String));
            quote! {
                #as_method_token_stream
                #set_method_token_stream
                #from_trait_token_stream
            }
        }

        fn impls_token_stream_for_bool(name: &Ident) -> TokenStream {
            let as_method_token_stream =
                impl_as_method_for_base_types(name, &quote!(bool), &quote!(as_bool), "解析 JSON 得到 Boolean");
            let set_method_token_stream =
                impl_set_method_for_base_types(name, &quote!(bool), &quote!(set_bool), "设置 JSON Boolean");
            let from_trait_token_stream = impl_from_trait_for_base_types(name, &quote!(bool));
            quote! {
                #as_method_token_stream
                #set_method_token_stream
                #from_trait_token_stream
            }
        }

        fn impls_token_stream_for_int(name: &Ident) -> TokenStream {
            let as_method_token_streams = [
                impl_as_method_for_base_types(name, &quote!(i64), &quote!(as_i64), "解析 JSON 得到 i64"),
                impl_as_method_for_base_types(name, &quote!(u64), &quote!(as_u64), "解析 JSON 得到 u64"),
            ];
            let from_trait_and_set_method_token_streams = [
                (quote!(i8), quote!(set_i8), "设置 JSON i8 整型"),
                (quote!(i16), quote!(set_i16), "设置 JSON i16 整型"),
                (quote!(i32), quote!(set_i32), "设置 JSON i32 整型"),
                (quote!(i64), quote!(set_i64), "设置 JSON i64 整型"),
                (quote!(isize), quote!(set_isize), "设置 JSON isize 整型"),
                (quote!(u8), quote!(set_u8), "设置 JSON u8 整型"),
                (quote!(u16), quote!(set_u16), "设置 JSON u16 整型"),
                (quote!(u32), quote!(set_u32), "设置 JSON u32 整型"),
                (quote!(u64), quote!(set_u64), "设置 JSON u64 整型"),
                (quote!(usize), quote!(set_usize), "设置 JSON usize 整型"),
            ]
            .map(|(ty, set_method_name, set_method_documentation)| {
                let set_method_token_stream =
                    impl_set_method_for_base_types(name, &ty, &set_method_name, set_method_documentation);
                let from_trait_token_stream = impl_from_trait_for_base_types(name, &ty);
                quote! {
                    #set_method_token_stream
                    #from_trait_token_stream
                }
            });

            quote! {
                #(#as_method_token_streams)*
                #(#from_trait_and_set_method_token_streams)*
            }
        }

        fn impls_token_stream_for_float(name: &Ident) -> TokenStream {
            let as_method_token_stream =
                impl_as_method_for_base_types(name, &quote!(f64), &quote!(as_f64), "解析 JSON 得到 f64 浮点型");
            let from_trait_and_set_method_token_streams = [
                (quote!(f64), quote!(set_f64), "设置 JSON f64 浮点型"),
                (quote!(f32), quote!(set_f32), "设置 JSON f32 浮点型"),
            ]
            .map(|(ty, set_method_name, set_method_documentation)| {
                let set_method_token_stream =
                    impl_set_method_for_base_types(name, &ty, &set_method_name, set_method_documentation);
                let from_trait_token_stream = impl_from_trait_for_base_types(name, &ty);
                quote! {
                    #set_method_token_stream
                    #from_trait_token_stream
                }
            });

            quote! {
                #as_method_token_stream
                #(#from_trait_and_set_method_token_streams)*
            }
        }

        fn impl_as_method_for_base_types(
            name: &Ident,
            ty: &TokenStream,
            as_method_name: &TokenStream,
            as_method_documentation: &str,
        ) -> TokenStream {
            quote! {
                impl #name {
                    #[inline]
                    #[doc = #as_method_documentation]
                    pub fn #as_method_name(&self) -> #ty {
                        self.0.#as_method_name().unwrap()
                    }
                }
            }
        }

        fn impl_set_method_for_base_types(
            name: &Ident,
            ty: &TokenStream,
            set_method_name: &TokenStream,
            set_method_documentation: &str,
        ) -> TokenStream {
            quote! {
                impl #name {
                    #[inline]
                    #[doc = #set_method_documentation]
                    pub fn #set_method_name(&mut self, val: #ty) {
                        *self = Self::from(val);
                    }
                }
            }
        }

        fn impl_from_trait_for_base_types(name: &Ident, rust_type: &TokenStream) -> TokenStream {
            quote! {
                impl From<#rust_type> for #name {
                    #[inline]
                    fn from(val: #rust_type) -> Self {
                        Self(serde_json::Value::from(val))
                    }
                }
            }
        }

        fn impls_token_stream_for_array(name: &Ident, array_type: &JsonType) -> TokenStream {
            return match array_type {
                JsonType::String => impls_token_stream_for_array_of_string(name),
                JsonType::Integer => impls_token_stream_for_array_of_int(name),
                JsonType::Float => impls_token_stream_for_array_of_float(name),
                JsonType::Boolean => impls_token_stream_for_array_of_bool(name),
                JsonType::Array(_) => panic!("Array of Array is not supported"),
                JsonType::Struct(struct_info) => impls_token_stream_for_array_of_struct(name, struct_info),
                JsonType::Any => panic!("Array of Any is not supported"),
                JsonType::StringMap => panic!("Array of StringMap is not supported"),
            };

            fn impls_token_stream_for_array_of_string(name: &Ident) -> TokenStream {
                let as_method_token_stream = impl_as_method_for_base_types_in_array(
                    name,
                    &quote!(&str),
                    &quote!(to_str_vec),
                    &quote!(as_str),
                    "解析 JSON 得到 String 列表",
                );
                let from_trait_token_stream = impl_from_trait_for_base_types_in_array(name, &quote!(String));
                let immutable_methods_token_stream = impl_immutable_methods_for_base_types_in_array(name);
                let insert_method_token_stream = impl_insert_method_for_base_types_in_array(
                    name,
                    &quote!(String),
                    &quote!(insert_str),
                    "在列表的指定位置插入 JSON String",
                );
                let remove_method_token_stream = impl_remove_method_for_base_types_in_array(
                    name,
                    &quote!(String),
                    &quote!(serde_json::Value::String),
                    &quote!(remove_as_str),
                    "在列表的指定位置移出 JSON String",
                );
                let push_method_token_stream = impl_push_method_for_base_types_in_array(
                    name,
                    &quote!(String),
                    &quote!(push_str),
                    "在列表尾部追加 JSON String",
                );
                let pop_method_token_stream = impl_pop_method_for_base_types_in_array(
                    name,
                    &quote!(String),
                    &quote!(serde_json::Value::String),
                    &quote!(pop_as_str),
                    "在列表尾部取出 JSON String",
                );
                quote! {
                    #as_method_token_stream
                    #from_trait_token_stream
                    #immutable_methods_token_stream
                    #insert_method_token_stream
                    #remove_method_token_stream
                    #push_method_token_stream
                    #pop_method_token_stream
                }
            }

            fn impls_token_stream_for_array_of_bool(name: &Ident) -> TokenStream {
                let as_method_token_stream = impl_as_method_for_base_types_in_array(
                    name,
                    &quote!(bool),
                    &quote!(to_bool_vec),
                    &quote!(as_bool),
                    "解析 JSON 得到 Boolean 列表",
                );
                let from_trait_token_stream = impl_from_trait_for_base_types_in_array(name, &quote!(bool));
                let immutable_methods_token_stream = impl_immutable_methods_for_base_types_in_array(name);
                let insert_method_token_stream = impl_insert_method_for_base_types_in_array(
                    name,
                    &quote!(bool),
                    &quote!(insert_bool),
                    "在列表的指定位置插入 JSON Boolean",
                );
                let remove_method_token_stream = impl_remove_method_for_base_types_in_array(
                    name,
                    &quote!(bool),
                    &quote!(serde_json::Value::Bool),
                    &quote!(remove_as_bool),
                    "在列表的指定位置移出 JSON Boolean",
                );
                let push_method_token_stream = impl_push_method_for_base_types_in_array(
                    name,
                    &quote!(bool),
                    &quote!(push_bool),
                    "在列表尾部追加 JSON Boolean",
                );
                let pop_method_token_stream = impl_pop_method_for_base_types_in_array(
                    name,
                    &quote!(bool),
                    &quote!(serde_json::Value::Bool),
                    &quote!(pop_as_bool),
                    "在列表尾部取出 JSON Boolean",
                );
                quote! {
                    #as_method_token_stream
                    #from_trait_token_stream
                    #immutable_methods_token_stream
                    #insert_method_token_stream
                    #remove_method_token_stream
                    #push_method_token_stream
                    #pop_method_token_stream
                }
            }

            fn impls_token_stream_for_array_of_int(name: &Ident) -> TokenStream {
                let immutable_methods_token_stream = impl_immutable_methods_for_base_types_in_array(name);
                let as_method_token_streams = [
                    impl_as_method_for_base_types_in_array(
                        name,
                        &quote!(i64),
                        &quote!(to_i64_vec),
                        &quote!(as_i64),
                        "解析 JSON 得到整型列表",
                    ),
                    impl_as_method_for_base_types_in_array(
                        name,
                        &quote!(u64),
                        &quote!(to_u64_vec),
                        &quote!(as_u64),
                        "解析 JSON 得到无符号整型列表",
                    ),
                ];
                let remove_methods_token_streams = [
                    (
                        quote!(i64),
                        quote!(serde_json::Value::Number),
                        quote!(as_i64),
                        quote!(remove_as_i64),
                        "在列表的指定位置移出 JSON i64 整型",
                        quote!(pop_as_i64),
                        "在列表尾部取出 JSON i64 整型",
                    ),
                    (
                        quote!(u64),
                        quote!(serde_json::Value::Number),
                        quote!(as_u64),
                        quote!(remove_as_u64),
                        "在列表的指定位置移出 JSON u64 整型",
                        quote!(pop_as_u64),
                        "在列表尾部取出 JSON u64 整型",
                    ),
                ]
                .map(
                    |(
                        ty,
                        json_type,
                        as_method_name,
                        remove_method_name,
                        remove_method_documentation,
                        pop_method_name,
                        pop_method_documentation,
                    )| {
                        let remove_method_token_stream = impl_remove_as_method_for_base_types_in_array(
                            name,
                            &ty,
                            &json_type,
                            &as_method_name,
                            &remove_method_name,
                            remove_method_documentation,
                        );
                        let pop_method_token_stream = impl_pop_as_method_for_base_types_in_array(
                            name,
                            &ty,
                            &json_type,
                            &as_method_name,
                            &pop_method_name,
                            pop_method_documentation,
                        );
                        quote! {
                            #remove_method_token_stream
                            #pop_method_token_stream
                        }
                    },
                );
                let from_trait_and_insert_methods_token_streams = [
                    (
                        quote!(i8),
                        quote!(insert_i8),
                        "在列表的指定位置插入 JSON i8 整型",
                        quote!(push_i8),
                        "在列表尾部追加 JSON i8 整型",
                    ),
                    (
                        quote!(i16),
                        quote!(insert_i16),
                        "在列表的指定位置插入 JSON i16 整型",
                        quote!(push_i16),
                        "在列表尾部追加 JSON i16 整型",
                    ),
                    (
                        quote!(i32),
                        quote!(insert_i32),
                        "在列表的指定位置插入 JSON i32 整型",
                        quote!(push_i32),
                        "在列表尾部追加 JSON i32 整型",
                    ),
                    (
                        quote!(i64),
                        quote!(insert_i64),
                        "在列表的指定位置插入 JSON i64 整型",
                        quote!(push_i64),
                        "在列表尾部追加 JSON i64 整型",
                    ),
                    (
                        quote!(isize),
                        quote!(insert_isize),
                        "在列表的指定位置插入 JSON isize 整型",
                        quote!(push_isize),
                        "在列表尾部追加 JSON isize 整型",
                    ),
                    (
                        quote!(u8),
                        quote!(insert_u8),
                        "在列表的指定位置插入 JSON u8 整型",
                        quote!(push_u8),
                        "在列表尾部追加 JSON u8 整型",
                    ),
                    (
                        quote!(u16),
                        quote!(insert_u16),
                        "在列表的指定位置插入 JSON u16 整型",
                        quote!(push_u16),
                        "在列表尾部追加 JSON u16 整型",
                    ),
                    (
                        quote!(u32),
                        quote!(insert_u32),
                        "在列表的指定位置插入 JSON u32 整型",
                        quote!(push_u32),
                        "在列表尾部追加 JSON u32 整型",
                    ),
                    (
                        quote!(u64),
                        quote!(insert_u64),
                        "在列表的指定位置插入 JSON u64 整型",
                        quote!(push_u64),
                        "在列表尾部追加 JSON u64 整型",
                    ),
                    (
                        quote!(usize),
                        quote!(insert_usize),
                        "在列表的指定位置插入 JSON usize 整型",
                        quote!(push_usize),
                        "在列表尾部追加 JSON usize 整型",
                    ),
                ]
                .map(
                    |(
                        ty,
                        insert_method_name,
                        insert_method_documentation,
                        push_method_name,
                        push_method_documentation,
                    )| {
                        let from_trait_token_stream = impl_from_trait_for_base_types_in_array(name, &ty);
                        let insert_method_token_stream = impl_insert_method_for_base_types_in_array(
                            name,
                            &ty,
                            &insert_method_name,
                            insert_method_documentation,
                        );
                        let push_method_token_stream = impl_push_method_for_base_types_in_array(
                            name,
                            &ty,
                            &push_method_name,
                            push_method_documentation,
                        );
                        quote! {
                            #from_trait_token_stream
                            #insert_method_token_stream
                            #push_method_token_stream
                        }
                    },
                );

                quote! {
                    #immutable_methods_token_stream
                    #(#as_method_token_streams)*
                    #(#remove_methods_token_streams)*
                    #(#from_trait_and_insert_methods_token_streams)*
                }
            }

            fn impls_token_stream_for_array_of_float(name: &Ident) -> TokenStream {
                let immutable_methods_token_stream = impl_immutable_methods_for_base_types_in_array(name);
                let as_method_token_stream = impl_as_method_for_base_types_in_array(
                    name,
                    &quote!(f64),
                    &quote!(to_float_vec),
                    &quote!(as_f64),
                    "解析 JSON 得到浮点型列表",
                );
                let remove_method_token_stream = impl_remove_as_method_for_base_types_in_array(
                    name,
                    &quote!(f64),
                    &quote!(serde_json::Value::Number),
                    &quote!(as_f64),
                    &quote!(remove_as_float),
                    "在列表的指定位置移出 JSON 浮点型",
                );
                let pop_method_token_stream = impl_pop_as_method_for_base_types_in_array(
                    name,
                    &quote!(f64),
                    &quote!(serde_json::Value::Number),
                    &quote!(as_f64),
                    &quote!(pop_as_float),
                    "在列表尾部取出 JSON 浮点型",
                );
                let from_trait_and_methods_token_streams = [
                    (
                        quote!(f64),
                        quote!(insert_f64),
                        "在列表的指定位置插入 JSON f64 浮点型",
                        quote!(push_f64),
                        "在列表尾部追加 JSON f64 浮点型",
                    ),
                    (
                        quote!(f32),
                        quote!(insert_f32),
                        "在列表的指定位置插入 JSON f32 浮点型",
                        quote!(push_f32),
                        "在列表尾部追加 JSON f32 浮点型",
                    ),
                ]
                .map(
                    |(
                        ty,
                        insert_method_name,
                        insert_method_documentation,
                        push_method_name,
                        push_method_documentation,
                    )| {
                        let from_trait_token_stream = impl_from_trait_for_base_types_in_array(name, &ty);
                        let insert_method_token_stream = impl_insert_method_for_base_types_in_array(
                            name,
                            &ty,
                            &insert_method_name,
                            insert_method_documentation,
                        );
                        let push_method_token_stream = impl_push_method_for_base_types_in_array(
                            name,
                            &ty,
                            &push_method_name,
                            push_method_documentation,
                        );
                        quote! {
                            #from_trait_token_stream
                            #insert_method_token_stream
                            #push_method_token_stream
                        }
                    },
                );

                quote! {
                    #immutable_methods_token_stream
                    #as_method_token_stream
                    #remove_method_token_stream
                    #pop_method_token_stream
                    #(#from_trait_and_methods_token_streams)*
                }
            }

            fn impls_token_stream_for_array_of_struct(name: &Ident, struct_info: &JsonStruct) -> TokenStream {
                let struct_name = format_ident!("{}", struct_info.name.to_case(Case::Pascal));
                let struct_token_stream = {
                    let type_definition_token_stream = define_new_struct(
                        &struct_name,
                        &struct_info.documentation,
                        Some(JsonCollectionType::Object),
                    );
                    let struct_impls_token_stream = impls_token_stream_for_struct(&struct_name, &struct_info.fields);
                    quote! {
                        #type_definition_token_stream
                        #struct_impls_token_stream
                    }
                };
                let as_method_token_stream = {
                    let to_method_name = format_ident!("to_{}_vec", struct_info.name.to_case(Case::Snake));
                    let to_method_documentation =
                        format!("解析 JSON 得到 {} 列表", struct_info.name.to_case(Case::Pascal));
                    impl_as_method_for_user_defined_types_in_array(
                        name,
                        &quote!(#struct_name),
                        &quote!(#to_method_name),
                        &to_method_documentation,
                    )
                };
                let from_trait_token_stream = impl_from_trait_for_base_types_in_array(name, &quote!(#struct_name));
                let immutable_methods_token_stream = impl_immutable_methods_for_base_types_in_array(name);
                let insert_method_token_stream = {
                    let insert_method_name = format_ident!("insert_{}", struct_info.name.to_case(Case::Snake));
                    let insert_method_documentation =
                        format!("在列表的指定位置插入 JSON {}", struct_info.name.to_case(Case::Pascal));
                    impl_insert_method_for_base_types_in_array(
                        name,
                        &quote!(#struct_name),
                        &quote!(#insert_method_name),
                        &insert_method_documentation,
                    )
                };
                let remove_method_token_stream = {
                    let remove_method_name = format_ident!("remove_as_{}", struct_info.name.to_case(Case::Snake));
                    let remove_method_documentation =
                        format!("在列表的指定位置移出 JSON {}", struct_info.name.to_case(Case::Pascal));
                    impl_remove_method_for_struct_types_in_array(
                        name,
                        &quote!(#struct_name),
                        &quote!(#remove_method_name),
                        &remove_method_documentation,
                    )
                };
                let push_method_token_stream = {
                    let push_method_name = format_ident!("push_{}", struct_info.name.to_case(Case::Snake));
                    let push_method_documentation =
                        format!("在列表尾部追加 JSON {}", struct_info.name.to_case(Case::Pascal));
                    impl_push_method_for_base_types_in_array(
                        name,
                        &quote!(#struct_name),
                        &quote!(#push_method_name),
                        &push_method_documentation,
                    )
                };
                let pop_method_token_stream = {
                    let pop_method_name = format_ident!("pop_{}", struct_info.name.to_case(Case::Snake));
                    let pop_method_documentation =
                        format!("在列表尾部取出 JSON {}", struct_info.name.to_case(Case::Pascal));
                    impl_pop_method_for_struct_types_in_array(
                        name,
                        &quote!(#struct_name),
                        &quote!(#pop_method_name),
                        &pop_method_documentation,
                    )
                };

                quote! {
                    #struct_token_stream
                    #as_method_token_stream
                    #from_trait_token_stream
                    #immutable_methods_token_stream
                    #insert_method_token_stream
                    #remove_method_token_stream
                    #push_method_token_stream
                    #pop_method_token_stream
                }
            }

            fn impl_as_method_for_base_types_in_array(
                name: &Ident,
                ty: &TokenStream,
                to_method_name: &TokenStream,
                as_method_name: &TokenStream,
                to_method_documentation: &str,
            ) -> TokenStream {
                quote! {
                    impl #name {
                        #[doc = #to_method_documentation]
                        pub fn #to_method_name(&self) -> Vec<#ty> {
                            self.0
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|ele| ele.#as_method_name().unwrap())
                                .collect()
                        }
                    }
                }
            }

            fn impl_as_method_for_user_defined_types_in_array(
                name: &Ident,
                ty: &TokenStream,
                to_method_name: &TokenStream,
                to_method_documentation: &str,
            ) -> TokenStream {
                quote! {
                    impl #name {
                        #[doc = #to_method_documentation]
                        pub fn #to_method_name(&self) -> Vec<#ty> {
                            self.0
                                .as_array()
                                .unwrap()
                                .iter()
                                .cloned()
                                .map(#ty::new)
                                .collect()
                        }
                    }
                }
            }

            fn impl_from_trait_for_base_types_in_array(name: &Ident, rust_type: &TokenStream) -> TokenStream {
                quote! {
                    impl From<Vec<#rust_type>> for #name {
                        #[inline]
                        fn from(val: Vec<#rust_type>) -> Self {
                            Self(serde_json::Value::from(val))
                        }
                    }
                }
            }

            fn impl_immutable_methods_for_base_types_in_array(name: &Ident) -> TokenStream {
                quote! {
                    impl #name {
                        #[doc = "获取数组的长度"]
                        pub fn len(&self) -> usize {
                            self.0
                                .as_array()
                                .unwrap()
                                .len()
                        }

                        #[doc = "数组是否为空"]
                        pub fn is_empty(&self) -> bool {
                            self.0
                                .as_array()
                                .unwrap()
                                .is_empty()
                        }
                    }
                }
            }

            fn impl_insert_method_for_base_types_in_array(
                name: &Ident,
                rust_type: &TokenStream,
                insert_method_name: &TokenStream,
                insert_method_documentation: &str,
            ) -> TokenStream {
                quote! {
                    impl #name {
                        #[doc = #insert_method_documentation]
                        pub fn #insert_method_name(&mut self, index: usize, val: #rust_type) {
                            self.0
                                .as_array_mut()
                                .unwrap()
                                .insert(index, val.into());
                        }
                    }
                }
            }

            fn impl_remove_method_for_base_types_in_array(
                name: &Ident,
                rust_type: &TokenStream,
                json_type: &TokenStream,
                remove_method_name: &TokenStream,
                remove_method_documentation: &str,
            ) -> TokenStream {
                quote! {
                    impl #name {
                        #[doc = #remove_method_documentation]
                        pub fn #remove_method_name(&mut self, index: usize) -> Option<#rust_type> {
                            match self.0.as_array_mut().unwrap().remove(index) {
                                #json_type(s) => Some(s),
                                _ => None,
                            }
                        }
                    }
                }
            }

            fn impl_remove_method_for_struct_types_in_array(
                name: &Ident,
                rust_type: &TokenStream,
                remove_method_name: &TokenStream,
                remove_method_documentation: &str,
            ) -> TokenStream {
                quote! {
                    impl #name {
                        #[doc = #remove_method_documentation]
                        pub fn #remove_method_name(&mut self, index: usize) -> #rust_type {
                            #rust_type::new(
                                self.0.as_array_mut().unwrap().remove(index)
                            )
                        }
                    }
                }
            }

            fn impl_remove_as_method_for_base_types_in_array(
                name: &Ident,
                rust_type: &TokenStream,
                json_type: &TokenStream,
                as_method_name: &TokenStream,
                remove_method_name: &TokenStream,
                remove_method_documentation: &str,
            ) -> TokenStream {
                quote! {
                    impl #name {
                        #[doc = #remove_method_documentation]
                        pub fn #remove_method_name(&mut self, index: usize) -> Option<#rust_type> {
                            match self.0.as_array_mut().unwrap().remove(index) {
                                #json_type(s) => s.#as_method_name(),
                                _ => None,
                            }
                        }
                    }
                }
            }

            fn impl_push_method_for_base_types_in_array(
                name: &Ident,
                rust_type: &TokenStream,
                push_method_name: &TokenStream,
                push_method_documentation: &str,
            ) -> TokenStream {
                quote! {
                    impl #name {
                        #[doc = #push_method_documentation]
                        pub fn #push_method_name(&mut self, val: #rust_type) {
                            self.0
                                .as_array_mut()
                                .unwrap()
                                .push(val.into());
                        }
                    }
                }
            }

            fn impl_pop_method_for_base_types_in_array(
                name: &Ident,
                rust_type: &TokenStream,
                json_type: &TokenStream,
                pop_method_name: &TokenStream,
                pop_method_documentation: &str,
            ) -> TokenStream {
                quote! {
                    impl #name {
                        #[doc = #pop_method_documentation]
                        pub fn #pop_method_name(&mut self) -> Option<#rust_type> {
                            self.0
                                .as_array_mut()
                                .unwrap()
                                .pop()
                                .and_then(|val| match val {
                                    #json_type(s) => Some(s),
                                    _ => None,
                                })
                        }
                    }
                }
            }

            fn impl_pop_method_for_struct_types_in_array(
                name: &Ident,
                rust_type: &TokenStream,
                pop_method_name: &TokenStream,
                pop_method_documentation: &str,
            ) -> TokenStream {
                quote! {
                    impl #name {
                        #[doc = #pop_method_documentation]
                        pub fn #pop_method_name(&mut self) -> Option<#rust_type> {
                            self.0
                                .as_array_mut()
                                .unwrap()
                                .pop()
                                .map(#rust_type::new)
                        }
                    }
                }
            }

            fn impl_pop_as_method_for_base_types_in_array(
                name: &Ident,
                rust_type: &TokenStream,
                json_type: &TokenStream,
                as_method_name: &TokenStream,
                pop_method_name: &TokenStream,
                pop_method_documentation: &str,
            ) -> TokenStream {
                quote! {
                    impl #name {
                        #[doc = #pop_method_documentation]
                        pub fn #pop_method_name(&mut self) -> Option<#rust_type> {
                            self.0
                                .as_array_mut()
                                .unwrap()
                                .pop()
                                .and_then(|val| match val {
                                    #json_type(s) => s.#as_method_name(),
                                    _ => None,
                                })
                        }
                    }
                }
            }
        }

        fn impls_token_stream_for_struct(name: &Ident, fields: &[JsonField]) -> TokenStream {
            let impls_token_stream = fields
                .iter()
                .map(|struct_info| impls_token_stream_for_field_of_struct(name, struct_info))
                .collect::<Vec<_>>();
            return quote! {
                #(#impls_token_stream)*
            };

            fn impls_token_stream_for_field_of_struct(name: &Ident, field: &JsonField) -> TokenStream {
                let field_name = format_ident!("{}", field.field_name.to_case(Case::Snake));
                let json_key = field.key.as_str();
                let documentation = field.documentation.as_str();
                return match &field.ty {
                    JsonType::String => impls_token_stream_for_string_field_of_struct(
                        name,
                        &field_name,
                        json_key,
                        documentation,
                        field.optional,
                    ),
                    JsonType::Integer => {
                        let signed_impls = impls_token_stream_for_base_type_field_of_struct(
                            name,
                            &field_name,
                            "i64",
                            json_key,
                            documentation,
                            &quote!(i64),
                            &quote!(as_i64),
                            field.optional,
                        );
                        let unsigned_impls = impls_token_stream_for_base_type_field_of_struct(
                            name,
                            &field_name,
                            "u64",
                            json_key,
                            documentation,
                            &quote!(u64),
                            &quote!(as_u64),
                            field.optional,
                        );
                        quote! {
                            #signed_impls
                            #unsigned_impls
                        }
                    }
                    JsonType::Float => impls_token_stream_for_base_type_field_of_struct(
                        name,
                        &field_name,
                        "f64",
                        json_key,
                        documentation,
                        &quote!(f64),
                        &quote!(as_f64),
                        field.optional,
                    ),
                    JsonType::Boolean => impls_token_stream_for_base_type_field_of_struct(
                        name,
                        &field_name,
                        "bool",
                        json_key,
                        documentation,
                        &quote!(bool),
                        &quote!(as_bool),
                        field.optional,
                    ),
                    JsonType::Array(array_info) => impls_token_stream_for_array_field_of_struct(
                        name,
                        &field_name,
                        json_key,
                        array_info,
                        documentation,
                        field.optional,
                    ),
                    JsonType::Struct(struct_info) => impls_token_stream_for_struct_field_of_struct(
                        name,
                        &field_name,
                        json_key,
                        struct_info,
                        documentation,
                        field.optional,
                    ),
                    JsonType::Any => impls_token_stream_for_any_field_of_struct(
                        name,
                        &field_name,
                        json_key,
                        documentation,
                        field.optional,
                    ),
                    JsonType::StringMap => impls_token_stream_for_string_map_field_of_struct(
                        name,
                        &field_name,
                        json_key,
                        documentation,
                        field.optional,
                    ),
                };

                fn impls_token_stream_for_string_field_of_struct(
                    name: &Ident,
                    field_name: &Ident,
                    json_key: &str,
                    documentation: &str,
                    optional: bool,
                ) -> TokenStream {
                    let getter_method_token_stream = impl_getter_method_for_base_type_field_of_struct(
                        name,
                        field_name,
                        "str",
                        json_key,
                        documentation,
                        &quote!(&str),
                        &quote!(as_str),
                        optional,
                    );
                    let setter_method_token_stream = impl_setter_method_for_string_field_of_struct(
                        name,
                        field_name,
                        json_key,
                        documentation,
                        optional,
                    );
                    quote! {
                        #getter_method_token_stream
                        #setter_method_token_stream
                    }
                }

                #[allow(clippy::too_many_arguments)]
                fn impls_token_stream_for_base_type_field_of_struct(
                    name: &Ident,
                    field_name: &Ident,
                    type_suffix: &str,
                    json_key: &str,
                    documentation: &str,
                    rust_type: &TokenStream,
                    as_method_name: &TokenStream,
                    optional: bool,
                ) -> TokenStream {
                    let getter_method_token_stream = impl_getter_method_for_base_type_field_of_struct(
                        name,
                        field_name,
                        type_suffix,
                        json_key,
                        documentation,
                        rust_type,
                        as_method_name,
                        optional,
                    );
                    let setter_method_token_stream = impl_setter_method_for_base_type_field_of_struct(
                        name,
                        field_name,
                        type_suffix,
                        json_key,
                        documentation,
                        rust_type,
                        as_method_name,
                        optional,
                    );
                    quote! {
                        #getter_method_token_stream
                        #setter_method_token_stream
                    }
                }

                #[allow(clippy::too_many_arguments)]
                fn impl_getter_method_for_base_type_field_of_struct(
                    name: &Ident,
                    field_name: &Ident,
                    type_suffix: &str,
                    json_key: &str,
                    documentation: &str,
                    rust_type: &TokenStream,
                    as_method_name: &TokenStream,
                    optional: bool,
                ) -> TokenStream {
                    let field_getter_method_name = format_ident!("get_{field_name}_as_{type_suffix}");
                    let getter_documentation = format!("获取 {documentation}");
                    if optional {
                        quote! {
                            impl #name {
                                #[doc = #getter_documentation]
                                pub fn #field_getter_method_name(&self) -> Option<#rust_type> {
                                    self.0
                                        .as_object()
                                        .and_then(|obj| obj.get(#json_key))
                                        .and_then(|val| val.#as_method_name())
                                }
                            }
                        }
                    } else {
                        quote! {
                            impl #name {
                                #[doc = #getter_documentation]
                                pub fn #field_getter_method_name(&self) -> #rust_type {
                                    self.0
                                        .as_object()
                                        .unwrap()
                                        .get(#json_key)
                                        .unwrap()
                                        .#as_method_name()
                                        .unwrap()
                                }
                            }
                        }
                    }
                }

                #[allow(clippy::too_many_arguments)]
                fn impl_setter_method_for_base_type_field_of_struct(
                    name: &Ident,
                    field_name: &Ident,
                    type_suffix: &str,
                    json_key: &str,
                    documentation: &str,
                    rust_type: &TokenStream,
                    as_method_name: &TokenStream,
                    optional: bool,
                ) -> TokenStream {
                    let field_setter_method_name = format_ident!("set_{field_name}_as_{type_suffix}");
                    let setter_documentation = format!("设置 {documentation}");
                    if optional {
                        quote! {
                            impl #name {
                                #[doc = #setter_documentation]
                                pub fn #field_setter_method_name(&mut self, new: #rust_type) -> Option<#rust_type> {
                                    self.0.as_object_mut().and_then(|object| {
                                        object
                                            .insert(#json_key.to_owned(), new.into())
                                            .and_then(|val| val.#as_method_name())
                                    })
                                }
                            }
                        }
                    } else {
                        quote! {
                            impl #name {
                                #[doc = #setter_documentation]
                                pub fn #field_setter_method_name(&mut self, new: #rust_type) -> Option<#rust_type> {
                                    self.0
                                        .as_object_mut()
                                        .unwrap()
                                        .insert(#json_key.to_owned(), new.into())
                                        .and_then(|val| val.#as_method_name())
                                }
                            }
                        }
                    }
                }

                fn impl_setter_method_for_string_field_of_struct(
                    name: &Ident,
                    field_name: &Ident,
                    json_key: &str,
                    documentation: &str,
                    optional: bool,
                ) -> TokenStream {
                    let field_setter_method_name = format_ident!("set_{field_name}_as_str");
                    let setter_documentation = format!("设置 {documentation}");
                    if optional {
                        quote! {
                            impl #name {
                                #[doc = #setter_documentation]
                                pub fn #field_setter_method_name(&mut self, new: String) -> Option<String> {
                                    self.0.as_object_mut().and_then(|object| {
                                        object
                                            .insert(#json_key.to_owned(), new.into())
                                            .and_then(|val| match val {
                                                serde_json::Value::String(s) => Some(s),
                                                _ => None,
                                            })
                                    })
                                }
                            }
                        }
                    } else {
                        quote! {
                            impl #name {
                                #[doc = #setter_documentation]
                                pub fn #field_setter_method_name(&mut self, new: String) -> Option<String> {
                                    self.0
                                        .as_object_mut()
                                        .unwrap()
                                        .insert(#json_key.to_owned(), new.into())
                                        .and_then(|val| match val {
                                            serde_json::Value::String(s) => Some(s),
                                            _ => None,
                                        })
                                }
                            }
                        }
                    }
                }

                fn impls_token_stream_for_any_field_of_struct(
                    name: &Ident,
                    field_name: &Ident,
                    json_key: &str,
                    documentation: &str,
                    optional: bool,
                ) -> TokenStream {
                    let field_getter_method_name = format_ident!("get_{field_name}");
                    let field_setter_method_name = format_ident!("set_{field_name}");
                    let getter_documentation = format!("获取 {documentation}");
                    let setter_documentation = format!("设置 {documentation}");

                    if optional {
                        quote! {
                            impl #name {
                                #[doc = #getter_documentation]
                                pub fn #field_getter_method_name(&self) -> Option<&serde_json::Value> {
                                    self.0
                                        .as_object()
                                        .and_then(|obj| obj.get(#json_key))
                                }

                                #[doc = #setter_documentation]
                                pub fn #field_setter_method_name(&mut self, new: serde_json::Value) -> Option<serde_json::Value> {
                                    self.0
                                        .as_object_mut()
                                        .and_then(|object| object.insert(#json_key.to_owned(), new.into()))
                                }
                            }
                        }
                    } else {
                        quote! {
                            impl #name {
                                #[doc = #getter_documentation]
                                pub fn #field_getter_method_name(&self) -> &serde_json::Value {
                                    self.0.as_object().unwrap().get(#json_key).unwrap()
                                }
                                #[doc = #setter_documentation]
                                pub fn #field_setter_method_name(&mut self, new: serde_json::Value) -> Option<serde_json::Value> {
                                    self.0
                                        .as_object_mut()
                                        .unwrap()
                                        .insert(#json_key.to_owned(), new.into())
                                }
                            }
                        }
                    }
                }

                fn impls_token_stream_for_string_map_field_of_struct(
                    name: &Ident,
                    field_name: &Ident,
                    json_key: &str,
                    documentation: &str,
                    optional: bool,
                ) -> TokenStream {
                    let field_getter_method_name = format_ident!("get_{field_name}");
                    let field_setter_method_name = format_ident!("set_{field_name}");
                    let getter_documentation = format!("获取 {documentation}");
                    let setter_documentation = format!("设置 {documentation}");

                    if optional {
                        quote! {
                            impl #name {
                                #[doc = #getter_documentation]
                                pub fn #field_getter_method_name(&self) -> Option<crate::base_types::StringMap> {
                                    self.0
                                        .as_object()
                                        .and_then(|obj| obj.get(#json_key))
                                        .cloned()
                                        .map(crate::base_types::StringMap::new)
                                }

                                #[doc = #setter_documentation]
                                pub fn #field_setter_method_name(&mut self, new: crate::base_types::StringMap) -> Option<crate::base_types::StringMap> {
                                    self.0
                                        .as_object_mut()
                                        .and_then(|object| object.insert(#json_key.to_owned(), new.into()))
                                        .map(crate::base_types::StringMap::new)
                                }
                            }
                        }
                    } else {
                        quote! {
                            impl #name {
                                #[doc = #getter_documentation]
                                pub fn #field_getter_method_name(&self) -> crate::base_types::StringMap {
                                    crate::base_types::StringMap::new(
                                        self.0.as_object().unwrap().get(#json_key).cloned().unwrap(),
                                    )
                                }
                                #[doc = #setter_documentation]
                                pub fn #field_setter_method_name(&mut self, new: crate::base_types::StringMap) -> Option<crate::base_types::StringMap> {
                                    self.0
                                        .as_object_mut()
                                        .unwrap()
                                        .insert(#json_key.to_owned(), new.into())
                                        .map(crate::base_types::StringMap::new)
                                }
                            }
                        }
                    }
                }

                fn impls_token_stream_for_array_field_of_struct(
                    name: &Ident,
                    field_name: &Ident,
                    json_key: &str,
                    array_info: &JsonArray,
                    documentation: &str,
                    optional: bool,
                ) -> TokenStream {
                    let array_name = format_ident!("{}", array_info.name.to_case(Case::Pascal));
                    let type_definition_token_stream =
                        define_new_struct(&array_name, &array_info.documentation, Some(JsonCollectionType::Array));
                    let impls_token_stream = impls_token_stream_for_array(&array_name, &array_info.ty);
                    let getter_method_token_stream = impl_getter_method_for_struct_field_of_struct(
                        name,
                        field_name,
                        json_key,
                        documentation,
                        &quote!(#array_name),
                        optional,
                    );
                    let setter_method_token_stream = impl_setter_method_for_struct_field_of_struct(
                        name,
                        field_name,
                        json_key,
                        documentation,
                        &quote!(#array_name),
                        optional,
                    );
                    quote! {
                        #type_definition_token_stream
                        #impls_token_stream
                        #getter_method_token_stream
                        #setter_method_token_stream
                    }
                }

                fn impls_token_stream_for_struct_field_of_struct(
                    name: &Ident,
                    field_name: &Ident,
                    json_key: &str,
                    struct_info: &JsonStruct,
                    documentation: &str,
                    optional: bool,
                ) -> TokenStream {
                    let struct_name = format_ident!("{}", struct_info.name.to_case(Case::Pascal));
                    let struct_token_stream = {
                        let type_definition_token_stream = define_new_struct(
                            &struct_name,
                            &struct_info.documentation,
                            Some(JsonCollectionType::Object),
                        );
                        let struct_impls_token_stream =
                            impls_token_stream_for_struct(&struct_name, &struct_info.fields);
                        quote! {
                            #type_definition_token_stream
                            #struct_impls_token_stream
                        }
                    };
                    let getter_method_token_stream = impl_getter_method_for_struct_field_of_struct(
                        name,
                        field_name,
                        json_key,
                        documentation,
                        &quote!(#struct_name),
                        optional,
                    );
                    let setter_method_token_stream = impl_setter_method_for_struct_field_of_struct(
                        name,
                        field_name,
                        json_key,
                        documentation,
                        &quote!(#struct_name),
                        optional,
                    );
                    quote! {
                        #struct_token_stream
                        #getter_method_token_stream
                        #setter_method_token_stream
                    }
                }

                fn impl_getter_method_for_struct_field_of_struct(
                    name: &Ident,
                    field_name: &Ident,
                    json_key: &str,
                    documentation: &str,
                    rust_type: &TokenStream,
                    optional: bool,
                ) -> TokenStream {
                    let field_getter_method_name = format_ident!("get_{field_name}");
                    let getter_documentation = format!("获取 {documentation}");
                    if optional {
                        quote! {
                            impl #name {
                                #[doc = #getter_documentation]
                                pub fn #field_getter_method_name(&self) -> Option<#rust_type> {
                                    self.0
                                        .as_object()
                                        .and_then(|obj| obj.get(#json_key))
                                        .cloned()
                                        .map(#rust_type::new)
                                }
                            }
                        }
                    } else {
                        quote! {
                            impl #name {
                                #[doc = #getter_documentation]
                                pub fn #field_getter_method_name(&self) -> #rust_type {
                                    #rust_type::new(
                                        self.0
                                            .as_object()
                                            .unwrap()
                                            .get(#json_key)
                                            .cloned()
                                            .unwrap()
                                    )
                                }
                            }
                        }
                    }
                }

                fn impl_setter_method_for_struct_field_of_struct(
                    name: &Ident,
                    field_name: &Ident,
                    json_key: &str,
                    documentation: &str,
                    rust_type: &TokenStream,
                    optional: bool,
                ) -> TokenStream {
                    let field_setter_method_name = format_ident!("set_{field_name}");
                    let setter_documentation = format!("设置 {documentation}");
                    if optional {
                        quote! {
                            impl #name {
                                #[doc = #setter_documentation]
                                pub fn #field_setter_method_name(&mut self, new: #rust_type) -> Option<#rust_type> {
                                    self.0.as_object_mut().and_then(|object| {
                                        object
                                            .insert(#json_key.to_owned(), new.into())
                                            .map(#rust_type::new)
                                    })
                                }
                            }
                        }
                    } else {
                        quote! {
                            impl #name {
                                #[doc = #setter_documentation]
                                pub fn #field_setter_method_name(&mut self, new: #rust_type) -> Option<#rust_type> {
                                    self.0
                                        .as_object_mut()
                                        .unwrap()
                                        .insert(#json_key.to_owned(), new.into())
                                        .map(#rust_type::new)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum JsonCollectionType {
    Array,
    Object,
}

fn define_new_struct(name: &Ident, documentation: &str, coll_type: Option<JsonCollectionType>) -> TokenStream {
    let default_method = match coll_type {
        Some(JsonCollectionType::Array) => Some(quote! {
            impl Default for #name {
                #[inline]
                fn default() -> Self {
                    Self(serde_json::Value::Array(Default::default()))
                }
            }
        }),
        Some(JsonCollectionType::Object) => Some(quote! {
            impl Default for #name {
                #[inline]
                fn default() -> Self {
                    Self(serde_json::Value::Object(Default::default()))
                }
            }
        }),
        None => None,
    };
    quote! {
        #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        #[doc = #documentation]
        pub struct #name(serde_json::Value);

        impl #name {
            #[allow(dead_code)]
            pub(crate) fn new(value: serde_json::Value) -> Self {
                Self(value)
            }
        }

        #default_method

        impl From<#name> for serde_json::Value {
            #[inline]
            fn from(val: #name) -> Self {
                val.0
            }
        }

        impl AsRef<serde_json::Value> for #name {
            #[inline]
            fn as_ref(&self) -> &serde_json::Value {
                &self.0
            }
        }

        impl AsMut<serde_json::Value> for #name {
            #[inline]
            fn as_mut(&mut self) -> &mut serde_json::Value {
                &mut self.0
            }
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// JSON 数组字段信息
pub(super) struct JsonArray {
    /// JSON 数组类型
    #[serde(rename = "type")]
    ty: JsonType,

    /// JSON 数组名称
    name: String,

    /// JSON 数组参数文档
    documentation: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// JSON 结构体字段
pub(super) struct JsonField {
    /// JSON 字段类型
    #[serde(rename = "type")]
    ty: JsonType,

    /// JSON 字段参数名称
    key: String,

    /// JSON 字段名称
    field_name: String,

    /// JSON 字段参数文档
    documentation: String,

    /// JSON 字段参数是否可选
    optional: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// JSON 结构体
pub(super) struct JsonStruct {
    /// JSON 字段列表
    fields: Vec<JsonField>,

    /// JSON 结构体名称
    name: String,

    /// JSON 结构体参数文档
    documentation: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::{fs, io::Write};
    use tempfile::{Builder as TempFileBuilder, NamedTempFile};
    use trybuild::TestCases;

    #[test]
    fn test_json_types() -> Result<()> {
        let test_files = [
            write_token_stream(
                "TestString",
                &JsonType::String.to_rust_token_stream("TestString", "Fake docs"),
            )?,
            write_token_stream(
                "TestBoolean",
                &JsonType::Boolean.to_rust_token_stream("TestBoolean", "Fake docs"),
            )?,
            write_token_stream(
                "TestInteger",
                &JsonType::Integer.to_rust_token_stream("TestInteger", "Fake docs"),
            )?,
            write_token_stream(
                "TestFloat",
                &JsonType::Float.to_rust_token_stream("TestFloat", "Fake docs"),
            )?,
            write_token_stream(
                "TestStringMap",
                &JsonType::StringMap.to_rust_token_stream("TestStringMap", "Fake docs"),
            )?,
            write_token_stream("TestAny", &JsonType::Any.to_rust_token_stream("TestAny", "Fake docs"))?,
            write_token_stream(
                "TestArrayOfString",
                &JsonType::Array(Box::new(JsonArray {
                    name: "FakeSubArrayOfString".to_owned(),
                    documentation: "Fake docs".to_owned(),
                    ty: JsonType::String,
                }))
                .to_rust_token_stream("TestArrayOfString", "Fake docs"),
            )?,
            write_token_stream(
                "TestArrayOfInteger",
                &JsonType::Array(Box::new(JsonArray {
                    name: "FakeSubArrayOfInteger".to_owned(),
                    documentation: "Fake docs".to_owned(),
                    ty: JsonType::Integer,
                }))
                .to_rust_token_stream("TestArrayOfInteger", "Fake docs"),
            )?,
            write_token_stream(
                "TestArrayOfFloat",
                &JsonType::Array(Box::new(JsonArray {
                    name: "FakeSubArrayOfFloat".to_owned(),
                    documentation: "Fake docs".to_owned(),
                    ty: JsonType::Float,
                }))
                .to_rust_token_stream("TestArrayOfFloat", "Fake docs"),
            )?,
            write_token_stream(
                "TestArrayOfBoolean",
                &JsonType::Array(Box::new(JsonArray {
                    name: "FakeSubArrayOfBoolean".to_owned(),
                    documentation: "Fake docs".to_owned(),
                    ty: JsonType::Boolean,
                }))
                .to_rust_token_stream("TestArrayOfBoolean", "Fake docs"),
            )?,
            write_token_stream(
                "TestArrayOfStruct",
                &JsonType::Array(Box::new(JsonArray {
                    name: "FakeSubArrayOfStruct".to_owned(),
                    documentation: "Fake docs".to_owned(),
                    ty: JsonType::Struct(JsonStruct {
                        name: "FakeStruct".to_owned(),
                        documentation: "Fake docs".to_owned(),
                        fields: vec![JsonField {
                            ty: JsonType::String,
                            key: "test_string".to_owned(),
                            field_name: "StringTestKey".to_owned(),
                            documentation: "String Test Key".to_owned(),
                            optional: false,
                        }],
                    }),
                }))
                .to_rust_token_stream("TestArrayOfStruct", "Fake docs"),
            )?,
            write_token_stream(
                "TestStruct",
                &JsonType::Struct(JsonStruct {
                    name: "FakeStruct".to_owned(),
                    documentation: "Fake docs".to_owned(),
                    fields: vec![
                        JsonField {
                            ty: JsonType::String,
                            key: "test_string".to_owned(),
                            field_name: "StringTestKey".to_owned(),
                            documentation: "String Test Key".to_owned(),
                            optional: false,
                        },
                        JsonField {
                            ty: JsonType::String,
                            key: "optional_test_string".to_owned(),
                            field_name: "OptionalStringTestKey".to_owned(),
                            documentation: "Optional String Test Key".to_owned(),
                            optional: true,
                        },
                        JsonField {
                            ty: JsonType::Boolean,
                            key: "test_boolean".to_owned(),
                            field_name: "BooleanTestKey".to_owned(),
                            documentation: "Boolean Test Key".to_owned(),
                            optional: false,
                        },
                        JsonField {
                            ty: JsonType::Boolean,
                            key: "optional_test_boolean".to_owned(),
                            field_name: "OptionalBooleanTestKey".to_owned(),
                            documentation: "Optional Boolean Test Key".to_owned(),
                            optional: true,
                        },
                        JsonField {
                            ty: JsonType::Integer,
                            key: "test_int".to_owned(),
                            field_name: "IntegerTestKey".to_owned(),
                            documentation: "Integer Test Key".to_owned(),
                            optional: false,
                        },
                        JsonField {
                            ty: JsonType::Integer,
                            key: "optional_test_int".to_owned(),
                            field_name: "OptionalIntegerTestKey".to_owned(),
                            documentation: "Optional Integer Test Key".to_owned(),
                            optional: true,
                        },
                        JsonField {
                            ty: JsonType::Float,
                            key: "test_float".to_owned(),
                            field_name: "FloatTestKey".to_owned(),
                            documentation: "Float Test Key".to_owned(),
                            optional: false,
                        },
                        JsonField {
                            ty: JsonType::Float,
                            key: "optional_test_float".to_owned(),
                            field_name: "OptionalFloatTestKey".to_owned(),
                            documentation: "Optional Float Test Key".to_owned(),
                            optional: true,
                        },
                        JsonField {
                            ty: JsonType::Any,
                            key: "test_any".to_owned(),
                            field_name: "AnyTestKey".to_owned(),
                            documentation: "Any Test Key".to_owned(),
                            optional: false,
                        },
                        JsonField {
                            ty: JsonType::Any,
                            key: "optional_test_any".to_owned(),
                            field_name: "OptionalAnyTestKey".to_owned(),
                            documentation: "Optional Any Test Key".to_owned(),
                            optional: true,
                        },
                        JsonField {
                            ty: JsonType::StringMap,
                            key: "test_string_map".to_owned(),
                            field_name: "StringMapTestKey".to_owned(),
                            documentation: "String Map Test Key".to_owned(),
                            optional: false,
                        },
                        JsonField {
                            ty: JsonType::StringMap,
                            key: "optional_test_string_map".to_owned(),
                            field_name: "OptionalStringMapTestKey".to_owned(),
                            documentation: "Optional String Map Test Key".to_owned(),
                            optional: true,
                        },
                        JsonField {
                            ty: JsonType::Struct(JsonStruct {
                                name: "FakeNestedStruct".to_owned(),
                                documentation: "Fake docs".to_owned(),
                                fields: vec![
                                    JsonField {
                                        ty: JsonType::String,
                                        key: "test_string".to_owned(),
                                        field_name: "StringTestKey".to_owned(),
                                        documentation: "String Test Key".to_owned(),
                                        optional: false,
                                    },
                                    JsonField {
                                        ty: JsonType::String,
                                        key: "optional_test_string".to_owned(),
                                        field_name: "OptionalStringTestKey".to_owned(),
                                        documentation: "Optional String Test Key".to_owned(),
                                        optional: true,
                                    },
                                ],
                            }),
                            key: "test_nested_struct".to_owned(),
                            field_name: "NestedStructTestKey".to_owned(),
                            documentation: "Nested Struct Test Key".to_owned(),
                            optional: false,
                        },
                        JsonField {
                            ty: JsonType::Struct(JsonStruct {
                                name: "FakeNestedOptionalStruct".to_owned(),
                                documentation: "Fake docs".to_owned(),
                                fields: vec![
                                    JsonField {
                                        ty: JsonType::String,
                                        key: "test_string".to_owned(),
                                        field_name: "StringTestKey".to_owned(),
                                        documentation: "String Test Key".to_owned(),
                                        optional: false,
                                    },
                                    JsonField {
                                        ty: JsonType::String,
                                        key: "optional_test_string".to_owned(),
                                        field_name: "OptionalStringTestKey".to_owned(),
                                        documentation: "Optional String Test Key".to_owned(),
                                        optional: true,
                                    },
                                ],
                            }),
                            key: "optional_test_nested_struct".to_owned(),
                            field_name: "OptionalNestedStructTestKey".to_owned(),
                            documentation: "Optional Nested Struct Test Key".to_owned(),
                            optional: true,
                        },
                        JsonField {
                            ty: JsonType::Array(Box::new(JsonArray {
                                ty: JsonType::Struct(JsonStruct {
                                    name: "FakeSubStructNestedOptionalStructs".to_owned(),
                                    documentation: "Fake docs".to_owned(),
                                    fields: vec![
                                        JsonField {
                                            ty: JsonType::String,
                                            key: "test_string".to_owned(),
                                            field_name: "StringTestKey".to_owned(),
                                            documentation: "String Test Key".to_owned(),
                                            optional: false,
                                        },
                                        JsonField {
                                            ty: JsonType::String,
                                            key: "optional_test_string".to_owned(),
                                            field_name: "OptionalStringTestKey".to_owned(),
                                            documentation: "Optional String Test Key".to_owned(),
                                            optional: true,
                                        },
                                    ],
                                }),
                                name: "FakeNestedOptionalStructs".to_owned(),
                                documentation: "Fake docs".to_owned(),
                            })),
                            key: "optional_test_nested_structs".to_owned(),
                            field_name: "OptionalNestedStructsTestKey".to_owned(),
                            documentation: "Optional Nested Structs Test Key".to_owned(),
                            optional: true,
                        },
                    ],
                })
                .to_rust_token_stream("TestStruct", "Fake docs"),
            )?,
        ];

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

        let base_types_code = format!(
            "pub mod base_types {{ {} }}",
            String::from_utf8(fs::read("src/base_types.rs")?)?
        );
        file.write_all(base_types_code.as_bytes())?;
        Ok(file)
    }
}
