use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    fs::{create_dir_all, write},
    io::Result as IOResult,
    path::Path,
};

type Tree = BTreeMap<OsString, DirOrFile>;

#[derive(Clone, Debug)]
pub(super) enum FileProperty {
    Public { documentation: String },
    PublicInternal { documentation: String },
    PrivateInternal,
}

impl FileProperty {
    fn documentation(&self) -> Option<&str> {
        match self {
            FileProperty::Public { documentation } => Some(documentation.as_str()),
            FileProperty::PublicInternal { documentation } => Some(documentation.as_str()),
            FileProperty::PrivateInternal => None,
        }
    }
}

#[derive(Clone, Debug)]
enum DirOrFile {
    Dir(Box<Tree>),
    File(FileProperty),
}

#[derive(Default, Clone, Debug)]
pub(super) struct Mods {
    root: Tree,
}

impl Mods {
    pub(super) fn add(
        &mut self,
        base_name: OsString,
        namespace: VecDeque<OsString>,
        file_property: FileProperty,
    ) {
        return add(base_name, namespace, file_property, &mut self.root);

        fn add(
            base_name: OsString,
            mut namespace: VecDeque<OsString>,
            file_property: FileProperty,
            tree: &mut Tree,
        ) {
            if let Some(namespace_root) = namespace.pop_front() {
                let entry = tree
                    .entry(namespace_root)
                    .or_insert_with(|| DirOrFile::Dir(Box::new(Tree::new())));
                match entry {
                    DirOrFile::Dir(sub_tree) => add(base_name, namespace, file_property, sub_tree),
                    DirOrFile::File { .. } => unreachable!("Cannot insert entry into File"),
                };
            } else {
                tree.insert(base_name, DirOrFile::File(file_property));
            }
        }
    }

    pub(super) fn write_to_rust_mod(&self, src_dir_path: &Path) -> IOResult<()> {
        let lib_rs_path = src_dir_path.join("lib.rs");
        return write_to_rust_mod(src_dir_path, &lib_rs_path, &self.root, true);

        fn write_to_rust_mod(
            dir_path: &Path,
            mod_file_path: &Path,
            tree: &Tree,
            is_lib_rs: bool,
        ) -> IOResult<()> {
            let mut mods = Vec::new();
            for (mod_name, item) in tree.iter() {
                if let DirOrFile::Dir(subtree) = item {
                    let mod_dir_path = dir_path.join(mod_name);
                    let mod_rs_path = mod_dir_path.join("mod.rs");
                    write_to_rust_mod(&mod_dir_path, &mod_rs_path, subtree, false)?;
                }

                let mod_name = format_ident!("{}", mod_name.to_str().unwrap());
                let file_property = match item {
                    DirOrFile::Dir(_) => None,
                    DirOrFile::File(file_property) => Some(file_property),
                };
                mods.push((mod_name, file_property.to_owned()));
            }
            let lib_rs_header = is_lib_rs.then(|| {
                quote! {
                    #![cfg_attr(feature = "docs", feature(doc_cfg))]

                    pub use qiniu_http_client as http_client;
                    pub use qiniu_http_client::credential as credential;
                    pub use qiniu_http_client::http as http;
                    pub use qiniu_http_client::upload_token as upload_token;

                    #[cfg(feature = "ureq")]
                    #[cfg_attr(feature = "docs", doc(cfg(feature = "ureq")))]
                    pub use qiniu_http_client::ureq as ureq;

                    #[cfg(feature = "isahc")]
                    #[cfg_attr(feature = "docs", doc(cfg(feature = "isahc")))]
                    pub use qiniu_http_client::isahc as isahc;

                    #[cfg(feature = "reqwest")]
                    #[cfg_attr(feature = "docs", doc(cfg(feature = "reqwest")))]
                    pub use qiniu_http_client::reqwest as reqwest;
                }
            });
            let public_api_mods: Vec<_> = mods
                .iter()
                .filter(|(_, file_property)| {
                    matches!(file_property, Some(FileProperty::Public { .. }) | None)
                })
                .cloned()
                .collect();
            let client_declaration_token_streams = if is_lib_rs {
                lib_rs_client_definition_token_stream(&public_api_mods)
            } else {
                mod_rs_client_definition_token_stream(&public_api_mods)
            };
            let mod_token_streams: Vec<_> = mods
                .iter()
                .map(|(mod_name, file_property)| match file_property {
                    Some(FileProperty::Public { documentation }) => quote! {
                        #[doc = #documentation]
                        pub mod #mod_name;
                    },
                    Some(FileProperty::PublicInternal { documentation }) => quote! {
                        #[doc = #documentation]
                        pub mod #mod_name;
                    },
                    Some(FileProperty::PrivateInternal) => quote! {
                        pub(crate) mod #mod_name;
                    },
                    None => quote! {
                        pub mod #mod_name;
                    },
                })
                .collect();
            let token_streams = quote! {
                #lib_rs_header
                #(#mod_token_streams)*
                #client_declaration_token_streams
            };
            create_dir_all(dir_path)?;
            let auto_generated_code =
                "// THIS FILE IS GENERATED BY api-generator, DO NOT EDIT DIRECTLY!\n//\n"
                    .to_owned()
                    + &token_streams.to_string();
            write(mod_file_path, auto_generated_code.as_bytes())?;
            Ok(())
        }
    }
}

pub(super) fn mod_rs_client_definition_token_stream(
    mods: &[(Ident, Option<&FileProperty>)],
) -> TokenStream {
    let methods_token_stream: Vec<_> = mods
        .iter()
        .map(|(mod_name, file_property)| {
            let documentation = file_property
                .and_then(|p| p.documentation())
                .map(|doc| quote! {#[doc = #doc]});
            quote! {
                #[inline]
                #documentation
                pub fn #mod_name(&self) -> #mod_name::Client {
                    #mod_name::Client::new(self.0)
                }
            }
        })
        .collect();
    quote! {
        #[derive(Debug, Clone)]
        pub struct Client<'client>(&'client qiniu_http_client::HttpClient);

        impl<'client> Client<'client> {
            pub(super) fn new(http_client: &'client qiniu_http_client::HttpClient) -> Self {
                Self(http_client)
            }
            #(#methods_token_stream)*
        }
    }
}

fn lib_rs_client_definition_token_stream(mods: &[(Ident, Option<&FileProperty>)]) -> TokenStream {
    let methods_token_stream: Vec<_> = mods
        .iter()
        .map(|(mod_name, file_property)| {
            let documentation = file_property
                .and_then(|p| p.documentation())
                .map(|doc| quote! {#[doc = #doc]});
            quote! {
                #[inline]
                #documentation
                pub fn #mod_name(&self) -> #mod_name::Client {
                    #mod_name::Client::new(&self.0)
                }
            }
        })
        .collect();
    quote! {
        #[derive(Debug, Clone, Default)]
        pub struct Client(qiniu_http_client::HttpClient);

        impl Client {
            #[inline]
            pub fn new(client: qiniu_http_client::HttpClient) -> Self {
                Self(client)
            }

            #(#methods_token_stream)*
        }

        impl From<qiniu_http_client::HttpClient> for Client {
            #[inline]
            fn from(client: qiniu_http_client::HttpClient) -> Self {
                Self(client)
            }
        }
    }
}
