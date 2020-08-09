use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Ident, Index, Meta, MetaList, NestedMeta};

#[proc_macro_derive(FFIStruct, attributes(ffi_wrap))]
pub fn ffi_struct_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    impl_ffi_struct(&ast)
}

fn impl_ffi_struct(ast: &DeriveInput) -> TokenStream {
    let (container, ffi_struct) = extract_ffi_wrap(ast);
    let fields_num = get_fields_num(ast);
    let null_muts = (0usize..fields_num).map(|_| quote! {::core::ptr::null_mut()});
    let is_nulls = (0usize..fields_num)
        .map(Index::from)
        .map(|i| quote! { self.#i.is_null() });

    let name = &ast.ident;
    let gen = quote! {
        impl Default for #name {
            #[inline]
            fn default() -> Self {
                Self(#(#null_muts),*)
            }
        }

        impl #name {
            #[inline]
            pub fn is_null(self) -> bool {
                #(#is_nulls)&&*
            }
        }

        impl From<#name> for Option<#container<#ffi_struct>> {
            fn from(n: #name) -> Self {
                if n.is_null() {
                    None
                } else {
                    Some(unsafe { #container::from_raw(::core::mem::transmute(n)) })
                }
            }
        }

        impl From<#container<#ffi_struct>> for #name {
            #[inline]
            fn from(n: #container<#ffi_struct>) -> Self {
                unsafe { ::core::mem::transmute(#container::into_raw(n)) }
            }
        }

        impl From<Option<#container<#ffi_struct>>> for #name {
            #[inline]
            fn from(n: Option<#container<#ffi_struct>>) -> Self {
                n.map(|n| n.into()).unwrap_or_default()
            }
        }
    };
    gen.into()
}

fn extract_ffi_wrap(ast: &DeriveInput) -> (TokenStream2, TokenStream2) {
    for attr in ast.attrs.iter() {
        let meta = attr.parse_meta().unwrap();
        if let Meta::List(MetaList {
            ref path,
            ref nested,
            ..
        }) = meta
        {
            if let Some(ident) = path.get_ident() {
                if ident == "ffi_wrap" {
                    let mut idents: Vec<Ident> = nested
                        .iter()
                        .map(|n| {
                            if let NestedMeta::Meta(meta) = n {
                                if let Some(ident) = meta.path().get_ident() {
                                    ident.clone()
                                } else {
                                    panic!("ffi_wrap must contain Ident",);
                                }
                            } else {
                                panic!("ffi_wrap must contain Meta")
                            }
                        })
                        .collect();
                    if idents.len() < 2 {
                        panic!("ffi_wrap must contain at least 2 identities");
                    }
                    let container = idents.remove(0);
                    let ffi_struct = idents;
                    let container = quote! {#container};
                    let ffi_struct = quote! {#(#ffi_struct) *};
                    return (container, ffi_struct);
                }
            }
        }
    }
    panic!("ffi_wrap contains invalid syntax")
}

fn get_fields_num(ast: &DeriveInput) -> usize {
    match &ast.data {
        Data::Struct(data_struct) => data_struct.fields.len(),
        _ => panic!("FFIStruct can only derive on Struct"),
    }
}
