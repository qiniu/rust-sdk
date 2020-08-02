use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident, Meta, MetaList, NestedMeta};

#[proc_macro_derive(FFIStruct, attributes(ffi_wrap))]
pub fn ffi_struct_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    impl_ffi_struct(&ast)
}

fn impl_ffi_struct(ast: &DeriveInput) -> TokenStream {
    let (ffi_struct, container) = extract_ffi_wrap(ast);

    let name = &ast.ident;
    let gen = quote! {
        impl Default for #name {
            #[inline]
            fn default() -> Self {
                Self(::core::ptr::null_mut())
            }
        }

        impl #name {
            #[inline]
            pub fn is_null(self) -> bool {
                self == Self::default()
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

fn extract_ffi_wrap(ast: &DeriveInput) -> (Ident, Ident) {
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
                    if idents.len() != 2 {
                        panic!("ffi_wrap must contains 2 idents");
                    }
                    return (idents.pop().unwrap(), idents.pop().unwrap());
                }
            }
        }
    }
    panic!("ffi_wrap contains invalid syntax")
}
