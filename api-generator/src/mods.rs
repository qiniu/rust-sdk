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
    pub(super) fn add(&mut self, base_name: OsString, namespace: VecDeque<OsString>, file_property: FileProperty) {
        return add(base_name, namespace, file_property, &mut self.root);

        fn add(base_name: OsString, mut namespace: VecDeque<OsString>, file_property: FileProperty, tree: &mut Tree) {
            if let Some(namespace_root) = namespace.pop_front() {
                let entry = tree
                    .entry(namespace_root)
                    .or_insert_with(|| DirOrFile::Dir(Box::<Tree>::default()));
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

        fn write_to_rust_mod(dir_path: &Path, mod_file_path: &Path, tree: &Tree, is_lib_rs: bool) -> IOResult<()> {
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
                    #![deny(
                        missing_debug_implementations,
                        large_assignments,
                        exported_private_dependencies,
                        absolute_paths_not_starting_with_crate,
                        anonymous_parameters,
                        explicit_outlives_requirements,
                        keyword_idents,
                        macro_use_extern_crate,
                        meta_variable_misuse,
                        missing_docs,
                        non_ascii_idents,
                        indirect_structural_match,
                        trivial_numeric_casts,
                        unsafe_code,
                        unused_extern_crates,
                        unused_import_braces,
                        unused_qualifications
                    )]
                    //! # qiniu-apis
                    //!
                    //! ## 七牛 HTTP API 库
                    //!
                    //! 这是一个基于 `qiniu-apis-specs` 自动生成的 Rust 库，基于 `qiniu-http-client`，用于调用七牛 HTTP API。
                    //! 该库同时提供阻塞客户端和异步客户端，异步客户端则需要启用 `async` 功能。
                    //! 该库致力于根据 [`qiniu-apis-specs`](https://github.com/qiniu/api-specs.git) 提供的 YAML 描述文件，在不理解业务逻辑的前提下，提供简单的封装方法方便用户正确调用 API。
                    //!
                    //! 该库可以通过启用不同的功能来选择不同的 HTTP 客户端实现，
                    //! 例如可以通过启用 `ureq` 功能导入 `qiniu-ureq` 库作为 HTTP 客户端，
                    //! 通过启用 `reqwest` 功能导入 `qiniu-reqwest` 库作为 HTTP 客户端，
                    //! 通过启用 `isahc` 功能导入 `qiniu-isahc` 库作为 HTTP 客户端。
                    //! 您也可以显式传入任何基于 `qiniu-http` 接口的 HTTP 客户端实现来提供给 `qiniu-apis` 使用。
                    //!
                    //! 由于是自动生成库，无法为每个接口提供代码示例，下面选择几个典型的场景来讲解如何使用该库：
                    //!
                    //! ### 功能描述
                    //!
                    //! #### `async`
                    //!
                    //! 启用异步接口。
                    //!
                    //! #### `ureq`
                    //!
                    //! 导入 `qiniu-ureq` 作为 HTTP 客户端。
                    //!
                    //! #### `isahc`
                    //!
                    //! 导入 `qiniu-isahc` 作为 HTTP 客户端。
                    //!
                    //! #### `reqwest`
                    //!
                    //! 导入 `qiniu-reqwest` 作为 HTTP 客户端。
                    //!
                    //! #### `c_ares`
                    //!
                    //! 启用 `c-ares` 库作为 DNS 解析器。
                    //!
                    //! #### `trust_dns`
                    //!
                    //! 启用 `trust-dns` 库作为 DNS 解析器。
                    //!
                    //! #### `dns-over-https`
                    //!
                    //! 启用 `trust-dns` 库作为 DNS 解析器，并使用 DOH 协议。
                    //!
                    //! #### `dns-over-tls`
                    //!
                    //! 启用 `trust-dns` 库作为 DNS 解析器，并使用 DOT 协议。
                    //!
                    //! ### 代码示例
                    //!
                    //! #### 创建存储空间
                    //!
                    //! API 参考文档：<https://developer.qiniu.com/kodo/1382/mkbucketv3>
                    //!
                    //! 通过该参考文档可知，创建存储空间需要通过 URL 路径提供参数，因此 `qiniu-apis` 代码如下：
                    //!
                    //! ##### 阻塞代码示例
                    //!
                    //! ```
                    //! use qiniu_apis::{
                    //!     credential::Credential,
                    //!     http_client::{AllRegionsProvider, RegionsProvider, RegionsProviderEndpoints},
                    //!     storage::create_bucket::PathParams,
                    //!     Client,
                    //! };
                    //! # fn example() -> anyhow::Result<()> {
                    //! let credential = Credential::new("abcdefghklmnopq", "1234567890");
                    //! let region = AllRegionsProvider::new(credential.to_owned())
                    //!     .get(Default::default())?;
                    //! Client::default()
                    //!     .storage()
                    //!     .create_bucket()
                    //!     .new_request(
                    //!         RegionsProviderEndpoints::new(&region),
                    //!         PathParams::default()
                    //!             .set_bucket_as_str("new-bucket-name")
                    //!             .set_region_as_str("z1"),
                    //!         credential,
                    //!     )
                    //!     .call()?;
                    //! # Ok(())
                    //! # }
                    //! ```
                    //!
                    //! ##### 异步代码示例
                    //!
                    //! ```
                    //! use qiniu_apis::{
                    //!     credential::Credential,
                    //!     http_client::{AllRegionsProvider, RegionsProvider, RegionsProviderEndpoints},
                    //!     storage::create_bucket::PathParams,
                    //!     Client,
                    //! };
                    //! # async fn example() -> anyhow::Result<()> {
                    //! let credential = Credential::new("abcdefghklmnopq", "1234567890");
                    //! let region = AllRegionsProvider::new(credential.to_owned())
                    //!     .async_get(Default::default())
                    //!     .await?;
                    //! Client::default()
                    //!     .storage()
                    //!     .create_bucket()
                    //!     .new_async_request(
                    //!         RegionsProviderEndpoints::new(&region),
                    //!         PathParams::default()
                    //!             .set_bucket_as_str("new-bucket-name")
                    //!             .set_region_as_str("z1"),
                    //!         credential,
                    //!     )
                    //!     .call()
                    //!     .await?;
                    //! # Ok(())
                    //! # }
                    //! ```
                    //!
                    //! 这里的 [`storage::create_bucket::PathParams`] 提供了设置路径参数的方法。
                    //!
                    //! #### 设置存储空间标签
                    //!
                    //! API 参考文档：<https://developer.qiniu.com/kodo/6314/put-bucket-tagging>
                    //!
                    //! 通过该参考文档可知，设置存储空间标签需要提供 URL 查询参数作为设置目标，并且通过 JSON 参数传输标签列表，因此 `qiniu-apis` 代码如下：
                    //!
                    //! ##### 阻塞代码示例
                    //!
                    //! ```
                    //! use qiniu_apis::{
                    //!     credential::Credential,
                    //!     http_client::{BucketRegionsQueryer, RegionsProviderEndpoints},
                    //!     storage::set_bucket_taggings::{QueryParams, RequestBody, TagInfo, Tags},
                    //!     Client,
                    //! };
                    //! # fn example() -> anyhow::Result<()> {
                    //! let credential = Credential::new("abcdefghklmnopq", "1234567890");
                    //! let bucket_name = "test-bucket";
                    //! let region = BucketRegionsQueryer::new().query(credential.access_key().to_owned(), bucket_name);
                    //! let mut tag1 = TagInfo::default();
                    //! tag1.set_key_as_str("tag_key1".to_owned());
                    //! tag1.set_value_as_str("tag_val1".to_owned());
                    //! let mut tag2 = TagInfo::default();
                    //! tag2.set_key_as_str("tag_key2".to_owned());
                    //! tag2.set_value_as_str("tag_val2".to_owned());
                    //! let mut tags = Tags::default();
                    //! tags.push_tag_info(tag1);
                    //! tags.push_tag_info(tag2);
                    //! let mut req_body = RequestBody::default();
                    //! req_body.set_tags(tags);
                    //! Client::default()
                    //!     .storage()
                    //!     .set_bucket_taggings()
                    //!     .new_request(RegionsProviderEndpoints::new(&region), credential)
                    //!     .query_pairs(QueryParams::default().set_bucket_as_str(bucket_name))
                    //!     .call(&req_body)?;
                    //! # Ok(())
                    //! # }
                    //! ```
                    //!
                    //! ##### 异步代码示例
                    //!
                    //! ```
                    //! use qiniu_apis::{
                    //!     credential::Credential,
                    //!     http_client::{BucketRegionsQueryer, RegionsProviderEndpoints},
                    //!     storage::set_bucket_taggings::{QueryParams, RequestBody, TagInfo, Tags},
                    //!     Client,
                    //! };
                    //! # async fn example() -> anyhow::Result<()> {
                    //! let credential = Credential::new("abcdefghklmnopq", "1234567890");
                    //! let bucket_name = "test-bucket";
                    //! let region = BucketRegionsQueryer::new().query(credential.access_key().to_owned(), bucket_name);
                    //! let mut tag1 = TagInfo::default();
                    //! tag1.set_key_as_str("tag_key1".to_owned());
                    //! tag1.set_value_as_str("tag_val1".to_owned());
                    //! let mut tag2 = TagInfo::default();
                    //! tag2.set_key_as_str("tag_key2".to_owned());
                    //! tag2.set_value_as_str("tag_val2".to_owned());
                    //! let mut tags = Tags::default();
                    //! tags.push_tag_info(tag1);
                    //! tags.push_tag_info(tag2);
                    //! let mut req_body = RequestBody::default();
                    //! req_body.set_tags(tags);
                    //! Client::default()
                    //!     .storage()
                    //!     .set_bucket_taggings()
                    //!     .new_async_request(RegionsProviderEndpoints::new(&region), credential)
                    //!     .query_pairs(QueryParams::default().set_bucket_as_str(bucket_name))
                    //!     .call(&req_body)
                    //!     .await?;
                    //! # Ok(())
                    //! # }
                    //! ```
                    //!
                    //! 这里的 [`storage::set_bucket_taggings::QueryParams`] 提供了设置查询参数的方法，
                    //! 而 [`storage::set_bucket_taggings::RequestBody`] 提供了设置请求体参数的方法 。
                    //!
                    //! #### 列出存储空间标签
                    //!
                    //! API 参考文档：<https://developer.qiniu.com/kodo/6315/get-bucket-tagging>
                    //!
                    //! 通过该参考文档可知，该 API 通过 JSON 响应体返回标签列表，因此 `qiniu-apis` 代码如下：
                    //!
                    //! ##### 阻塞代码示例
                    //!
                    //! ```
                    //! use qiniu_apis::{
                    //!     credential::Credential,
                    //!     http_client::{BucketRegionsQueryer, RegionsProviderEndpoints},
                    //!     storage::get_bucket_taggings::QueryParams,
                    //!     Client,
                    //! };
                    //! # fn example() -> anyhow::Result<()> {
                    //! let credential = Credential::new("abcdefghklmnopq", "1234567890");
                    //! let bucket_name = "test-bucket";
                    //! let region = BucketRegionsQueryer::new().query(credential.access_key().to_owned(), bucket_name);
                    //! let tags = Client::default()
                    //!     .storage()
                    //!     .get_bucket_taggings()
                    //!     .new_request(RegionsProviderEndpoints::new(&region), credential)
                    //!     .query_pairs(QueryParams::default().set_bucket_name_as_str(bucket_name))
                    //!     .call()?
                    //!     .into_body()
                    //!     .get_tags()
                    //!     .to_tag_info_vec();
                    //! for tag in tags {
                    //!     println!("{}: {}", tag.get_key_as_str(), tag.get_value_as_str());
                    //! }
                    //! # Ok(())
                    //! # }
                    //! ```
                    //!
                    //! ##### 异步代码示例
                    //!
                    //! ```
                    //! use qiniu_apis::{
                    //!     credential::Credential,
                    //!     http_client::{BucketRegionsQueryer, RegionsProviderEndpoints},
                    //!     storage::get_bucket_taggings::QueryParams,
                    //!     Client,
                    //! };
                    //! # async fn example() -> anyhow::Result<()> {
                    //! let credential = Credential::new("abcdefghklmnopq", "1234567890");
                    //! let bucket_name = "test-bucket";
                    //! let region = BucketRegionsQueryer::new().query(credential.access_key().to_owned(), bucket_name);
                    //! let tags = Client::default()
                    //!     .storage()
                    //!     .get_bucket_taggings()
                    //!     .new_async_request(RegionsProviderEndpoints::new(&region), credential)
                    //!     .query_pairs(QueryParams::default().set_bucket_name_as_str(bucket_name))
                    //!     .call()
                    //!     .await?
                    //!     .into_body()
                    //!     .get_tags()
                    //!     .to_tag_info_vec();
                    //! for tag in tags {
                    //!     println!("{}: {}", tag.get_key_as_str(), tag.get_value_as_str());
                    //! }
                    //! # Ok(())
                    //! # }
                    //! ```

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
                .filter(|(_, file_property)| matches!(file_property, Some(FileProperty::Public { .. }) | None))
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
                        #[allow(missing_docs)]
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
            let auto_generated_code = "// THIS FILE IS GENERATED BY api-generator, DO NOT EDIT DIRECTLY!\n//\n"
                .to_owned()
                + &token_streams.to_string();
            write(mod_file_path, auto_generated_code.as_bytes())?;
            Ok(())
        }
    }
}

pub(super) fn mod_rs_client_definition_token_stream(mods: &[(Ident, Option<&FileProperty>)]) -> TokenStream {
    let methods_token_stream: Vec<_> = mods
        .iter()
        .map(|(mod_name, file_property)| {
            let documentation = file_property
                .and_then(|p| p.documentation())
                .map(|doc| quote! {#[doc = #doc]});
            quote! {
                #[inline]
                #documentation
                pub fn #mod_name(&self) -> #mod_name::Client<'client> {
                    #mod_name::Client::new(self.0)
                }
            }
        })
        .collect();
    quote! {
        #[doc = "API 调用客户端"]
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
                .map(|doc| quote! {#[doc = #doc]})
                .unwrap_or_else(|| quote! {#[allow(missing_docs)]});
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
        #[doc = "七牛 API 调用客户端"]
        #[derive(Debug, Clone, Default)]
        pub struct Client(qiniu_http_client::HttpClient);

        impl Client {
            #[inline]
            #[must_use]
            #[doc = "创建七牛 API 调用客户端"]
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
