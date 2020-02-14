#![doc(html_favicon_url = "https://developer.qiniu.com/favicon.ico")]
#![doc(html_logo_url = "https://dn-mars-assets.qbox.me/qiniulogo/img-slogon-horizontal-blue-cn.jpg")]
//! 七牛云新一代 Rust SDK
//!
//! 此 Rust SDK 适用于 Rust 1.38 及其以上版本。基于 [七牛云官方 API](http://developer.qiniu.com/) 构建。
//! 使用此 SDK 构建您的网络应用程序，能让您以非常便捷地方式将数据安全地存储到七牛云上。
//! 无论您的网络应用是一个网站程序，还是包括从云端（服务端程序）到终端（手持设备应用）的架构的服务或应用，通过七牛云及其 SDK，都能让您应用程序的终端用户高速上传和下载，同时也让您的服务端更加轻盈。
//!
//! Rust SDK 属于七牛服务端 SDK 之一，主要有如下功能：
//! 1. 提供生成客户端上传所需的上传凭证的功能
//! 2. 提供文件从服务端直接上传七牛的功能
//! 3. 提供文件从七牛直接下载到本地的功能 【开发中】
//! 4. 提供对七牛空间中文件进行管理的功能 【开发中】
//! 5. 提供对七牛空间中文件进行处理的功能 【开发中】
//! 6. 提供七牛 CDN 相关的刷新，预取，日志功能 【开发中】
//!
//! # 鉴权
//!
//! 七牛 Rust SDK 的所有的功能，都需要合法的授权。授权凭证的签算需要七牛账号下的一对有效的 `Access Key` 和 `Secret Key` ，这对密钥可以通过如下步骤获得：
//!
//! 1. 点击[注册](https://portal.qiniu.com/signup)开通七牛开发者帐号
//! 2. 如果已有账号，直接登录七牛开发者后台，点击[这里](https://portal.qiniu.com/user/key)查看 Access Key 和 Secret Key
//!
//! # 存储空间
//!
//! 七牛对象存储的文件需要存储在存储空间中，存储空间即可以通过[七牛开发者后台](https://portal.qiniu.com/kodo/bucket)进行管理，也可以调用 Rust SDK 提供的 API 进行管理。
//!
//! ## 创建存储空间
//!
//! 在创建存储空间时，需要注意存储空间的名称必须遵守以下规则：
//! - 存储空间名称不允许重复，遇到冲突请更换名称。
//! - 名称由 3 ~ 63 个字符组成 ，可包含小写字母、数字和短划线，且必须以小写字母或者数字开头和结尾。
//!
//! ```rust,no_run
//! use qiniu_ng::{Config, Client, storage::region::RegionId};
//! # use std::{result::Result, error::Error};
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! let access_key = "[Qiniu Access Key]";
//! let secret_key = "[Qiniu Secret Key]";
//! let new_bucket_name = "[New Bucket Name, Must be Unique]";
//! let client = Client::new(access_key, secret_key, Config::default());
//! client.storage().create_bucket(new_bucket_name, RegionId::Z0)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## 删除存储空间
//!
//! 删除存储空间前务必保证存储空间里已经没有任何文件，否则删除将会失败。
//!
//! ```rust,no_run
//! use qiniu_ng::{Config, Client};
//! # use std::{result::Result, error::Error};
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! let access_key = "[Qiniu Access Key]";
//! let secret_key = "[Qiniu Secret Key]";
//! let bucket_name_to_drop = "[Bucket Name]";
//! let client = Client::new(access_key, secret_key, Config::default());
//! client.storage().drop_bucket(bucket_name_to_drop)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## 列出存储空间
//!
//! ```rust,no_run
//! use qiniu_ng::{Config, Client};
//! # use std::{result::Result, error::Error};
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! let access_key = "[Qiniu Access Key]";
//! let secret_key = "[Qiniu Secret Key]";
//! let client = Client::new(access_key, secret_key, Config::default());
//! let bucket_names = client.storage().bucket_names()?;
//! # Ok(())
//! # }
//! ```
//!
//! # 上传
//! ## 上传流程
//!
//! 七牛文件上传分为客户端上传（主要是指网页端和移动端等面向终端用户的场景）和服务端上传两种场景，具体可以参考文档[七牛业务流程](https://developer.qiniu.com/kodo/manual/programming-model)。
//!
//! 服务端 SDK 在上传方面主要提供两种功能，一种是生成客户端上传所需要的上传凭证，另外一种是直接上传文件到云端。
//!
//! ## 客户端上传凭证
//!
//! 客户端（移动端或者 Web 端）上传文件的时候，需要从客户自己的业务服务器获取上传凭证，而这些上传凭证是通过服务端的 SDK 来生成的，然后通过客户自己的业务 API 分发给客户端使用。根据上传的业务需求不同，七牛云 Rust SDK 支持丰富的上传凭证生成方式。
//!
//! ### 简单上传凭证
//!
//! 最简单的上传凭证只需要 `AccessKey`，`SecretKey` 和 `Bucket` 就可以。
//!
//! ```rust
//! use qiniu_ng::{Credential, Config};
//! use qiniu_ng::storage::upload_policy::{UploadPolicyBuilder, UploadPolicy};
//! use qiniu_ng::storage::upload_token::UploadToken;
//!
//! let config = Config::default();
//! let upload_policy = UploadPolicyBuilder::new_policy_for_bucket("[Bucket Name]", &config)
//!                                         .build();
//!
//! let access_key = "[Qiniu Access Key]";
//! let secret_key = "[Qiniu Secret Key]";
//! let credential = Credential::new(access_key, secret_key);
//! let upload_token = UploadToken::from_policy(upload_policy, &credential);
//! ```
//!
//! 默认情况下，在不指定上传凭证的有效时间情况下，默认有效期为 1 个小时。也可以自行指定上传凭证的有效期，例如：
//!
//! ```rust
//! # use qiniu_ng::{
//! #     Credential, Config, ConfigBuilder,
//! #     storage::{
//! #         upload_policy::{
//! #             UploadPolicyBuilder,
//! #             UploadPolicy
//! #         },
//! #         upload_token::UploadToken,
//! #     },
//! # };
//! # use std::time::Duration;
//! #
//! let access_key = "[Qiniu Access Key]";
//! let secret_key = "[Qiniu Secret Key]";
//! let bucket = "[Bucket Name]";
//!
//! let credential = Credential::new(access_key, secret_key);
//! let config = ConfigBuilder::default()
//!                            .upload_token_lifetime(Duration::from_secs(7200))
//!                            .build();
//! let upload_policy = UploadPolicyBuilder::new_policy_for_bucket(bucket, &config)
//!                                         .build();
//!
//! let upload_token = UploadToken::from_policy(upload_policy, &credential);
//! ```
//!
//! ### 覆盖上传凭证
//!
//! 覆盖上传除了需要 `简单上传` 所需要的信息之外，还需要想进行覆盖的文件名称，这个文件名称同时可是客户端上传代码中指定的文件名，两者必须一致。
//!
//! ```rust
//! # use qiniu_ng::{
//! #     Credential, Config,
//! #     storage::{
//! #         upload_policy::{
//! #             UploadPolicyBuilder,
//! #             UploadPolicy
//! #         },
//! #         upload_token::UploadToken,
//! #     },
//! # };
//! # use std::time::Duration;
//! #
//! let access_key = "[Qiniu Access Key]";
//! let secret_key = "[Qiniu Secret Key]";
//! let bucket = "[Bucket Name]";
//! let key_to_overwrite = "qiniu.mp4";
//!
//! let credential = Credential::new(access_key, secret_key);
//! let config = Config::default();
//! let upload_policy = UploadPolicyBuilder::new_policy_for_object(bucket, key_to_overwrite, &config)
//!                                         .build();
//!
//! let upload_token = UploadToken::from_policy(upload_policy, &credential);
//! ```
//!
//! ### 自定义上传回复凭证
//!
//! 默认情况下，文件上传到七牛之后，在没有设置 `returnBody` 或者回调相关的参数情况下，七牛返回给上传端的回复格式为 `hash`和 `key`，例如：
//!
//! ```json
//! {"hash":"Ftgm-CkWePC9fzMBTRNmPMhGBcSV","key":"qiniu.jpg"}
//! ```
//!
//! 有时候我们希望能自定义这个返回的 JSON 格式的内容，可以通过设置 `returnBody` 参数来实现，在 `returnBody` 中，我们可以使用七牛支持的[魔法变量](https://developer.qiniu.com/kodo/manual/vars#magicvar)和[自定义变量](https://developer.qiniu.com/kodo/manual/vars#xvar)。
//!
//! ```rust
//! # use qiniu_ng::{
//! #     Credential, Config,
//! #     storage::{
//! #         upload_policy::{
//! #             UploadPolicyBuilder,
//! #             UploadPolicy
//! #         },
//! #         upload_token::UploadToken,
//! #     },
//! # };
//! # use std::time::Duration;
//! #
//! let access_key = "[Qiniu Access Key]";
//! let secret_key = "[Qiniu Secret Key]";
//! let bucket = "[Bucket Name]";
//! let key_to_overwrite = "qiniu.mp4";
//!
//! let credential = Credential::new(access_key, secret_key);
//! let config = Config::default();
//! let upload_policy = UploadPolicyBuilder::new_policy_for_object(bucket, key_to_overwrite, &config)
//!                                         .return_body("{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"fsize\":$(fsize),\"bucket\":\"$(bucket)\",\"name\":\"$(x:name)\"}")
//!                                         .build();
//!
//! let upload_token = UploadToken::from_policy(upload_policy, &credential);
//! ```
//!
//! 则文件上传到七牛之后，收到的回复内容格式如下：
//!
//! ```json
//! {"key":"github-x.png","hash":"FqKXVdTvIx_mPjOYdjDyUSy_H1jr","fsize":6091,"bucket":"if-pbl","name":"github logo"}
//! ```
//!
//! 对于上面的自定义返回值，我们可以调用 UploadResponse 的方法解析这个结果，例如下面提供了一个解析结果的方法：
//!
//! ```rust,no_run
//! # use qiniu_ng::{
//! #     Credential, Client, Config,
//! #     storage::{
//! #         upload_policy::{
//! #             UploadPolicyBuilder,
//! #             UploadPolicy
//! #         },
//! #         upload_token::UploadToken,
//! #         uploader::UploadManager,
//! #     },
//! # };
//! # use std::{path::Path, time::Duration, result::Result, error::Error};
//! #
//! # fn main() -> Result<(), Box<dyn Error>> {
//! # let access_key = "[Qiniu Access Key]";
//! # let secret_key = "[Qiniu Secret Key]";
//! # let bucket = "[Bucket Name]";
//! # let key_to_overwrite = "qiniu.mp4";
//! #
//! # let credential = Credential::new(access_key, secret_key);
//! # let config = Config::default();
//! # let upload_policy = UploadPolicyBuilder::new_policy_for_object(bucket, key_to_overwrite, &config)
//! #                                         .return_body("{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"fsize\":$(fsize),\"bucket\":\"$(bucket)\",\"name\":\"$(x:name)\"}")
//! #                                         .build();
//! #
//! # let upload_token = UploadToken::from_policy(upload_policy, &credential);
//! let local_file_path = Path::new("local file path");
//! let upload_manager = UploadManager::new(config);
//! let upload_response = upload_manager.for_upload_token(upload_token)?
//!                                     .var("name", "七牛云视频")
//!                                     .upload_file(local_file_path, "local file name", None)?;
//! assert_eq!(Some(bucket), upload_response.get("bucket").and_then(|v| v.as_str()));
//! assert_eq!(Some("七牛云视频"), upload_response.get("name").and_then(|v| v.as_str()));
//! # Ok(())
//! # }
//! ```
//!
//! ### 带回调业务服务器的凭证
//!
//! 上面生成的 `自定义上传回复` 的上传凭证适用于上传端（无论是客户端还是服务端）和七牛服务器之间进行直接交互的情况下。在客户端上传的场景之下，有时候客户端需要在文件上传到七牛之后，从业务服务器获取相关的信息，这个时候就要用到七牛的上传回调及相关回调参数的设置。
//!
//! ```rust
//! # use qiniu_ng::{
//! #     Credential, Config, ConfigBuilder,
//! #     storage::{
//! #         upload_policy::{
//! #             UploadPolicyBuilder,
//! #             UploadPolicy
//! #         },
//! #         upload_token::UploadToken,
//! #     },
//! # };
//! # use std::time::Duration;
//! #
//! let access_key = "[Qiniu Access Key]";
//! let secret_key = "[Qiniu Secret Key]";
//! let bucket = "[Bucket Name]";
//! let key_to_overwrite = "qiniu.mp4";
//!
//! let credential = Credential::new(access_key, secret_key);
//! let config = Config::default();
//! let upload_policy = UploadPolicyBuilder::new_policy_for_object(bucket, key_to_overwrite, &config)
//!                                         .callback_urls(["http://api.example.com/qiniu/upload/callback"], "")
//!                                         .callback_body("{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"fsize\":$(fsize),\"bucket\":\"$(bucket)\",\"name\":\"$(x:name)\"}", "application/json")
//!                                         .build();
//!
//! let upload_token = UploadToken::from_policy(upload_policy, &credential);
//! ```
//!
//! 在使用了上传回调的情况下，客户端收到的回复就是业务服务器响应七牛的 JSON 格式内容，业务服务器收到回调之后必须响应 JSON 格式的回复給七牛，这个回复会被七牛传递给客户端。
//!
//! 例如上面的 `CallbackBody` 的设置会在文件上传到七牛之后，触发七牛回调如下内容給业务服务器：
//!
//! ```json
//! {"key":"github-x.png","hash":"FqKXVdTvIx_mPjOYdjDyUSy_H1jr","fsize":6091,"bucket":"if-pbl","name":"github logo"}
//! ```
//!
//! 通常情况下，我们建议使用 `application/json` 格式来设置 `callbackBody` ，保持数据格式的统一性。实际情况下， `callbackBody` 也支持 `application/x-www-form-urlencoded` 格式来组织内容，这个主要看业务服务器在接收到 `callbackBody` 的内容时如何解析。例如：
//!
//! ```rust
//! # use qiniu_ng::{
//! #     Credential, Config, ConfigBuilder,
//! #     storage::{
//! #         upload_policy::{
//! #             UploadPolicyBuilder,
//! #             UploadPolicy
//! #         },
//! #         upload_token::UploadToken,
//! #     },
//! # };
//! # use std::time::Duration;
//! #
//! let access_key = "[Qiniu Access Key]";
//! let secret_key = "[Qiniu Secret Key]";
//! let bucket = "[Bucket Name]";
//! let key_to_overwrite = "qiniu.mp4";
//!
//! let credential = Credential::new(access_key, secret_key);
//! let config = Config::default();
//! let upload_policy = UploadPolicyBuilder::new_policy_for_object(bucket, key_to_overwrite, &config)
//!                                         .callback_urls(["http://api.example.com/qiniu/upload/callback"], "")
//!                                         .callback_body("key=$(key)&hash=$(etag)&bucket=$(bucket)&fsize=$(fsize)&name=$(x:name)", "application/x-www-form-urlencoded")
//!                                         .build();
//!
//! let upload_token = UploadToken::from_policy(upload_policy, &credential);
//! ```
//!
//! ### 带自定义参数的凭证
//!
//! 七牛支持客户端上传文件的时候定义一些自定义参数，这些参数可以在 `returnBody` 和 `callbackBody` 里面和七牛内置支持的魔法变量（即系统变量）通过相同的方式来引用。这些自定义的参数名称必须以 `x:` 开头。例如客户端上传的时候指定了自定义的参数 `x:name` 和 `x:age` 分别是 string 和 int 类型。那么可以通过下面的方式引用：
//!
//! ```rust
//! # use qiniu_ng::{
//! #     Credential, Config,
//! #     storage::{
//! #         upload_policy::{
//! #             UploadPolicyBuilder,
//! #             UploadPolicy
//! #         },
//! #         upload_token::UploadToken,
//! #     },
//! # };
//! # use std::time::Duration;
//! #
//! # let access_key = "[Qiniu Access Key]";
//! # let secret_key = "[Qiniu Secret Key]";
//! # let bucket = "[Bucket Name]";
//! # let key_to_overwrite = "qiniu.mp4";
//! #
//! # let credential = Credential::new(access_key, secret_key);
//! # let config = Config::default();
//! let upload_policy = UploadPolicyBuilder::new_policy_for_object(bucket, key_to_overwrite, &config)
//!                                         .return_body("{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"fsize\":$(fsize),\"bucket\":\"$(bucket)\",\"name\":\"$(x:name)\",\"age\":$(x:age)}")
//!                                         .build();
//!
//! let upload_token = UploadToken::from_policy(upload_policy, &credential);
//! ```
//!
//! 或者
//!
//! ```rust
//! # use qiniu_ng::{
//! #     Credential, Config, ConfigBuilder,
//! #     storage::{
//! #         upload_policy::{
//! #             UploadPolicyBuilder,
//! #             UploadPolicy
//! #         },
//! #         upload_token::UploadToken,
//! #     },
//! # };
//! # use std::time::Duration;
//! #
//! # let access_key = "[Qiniu Access Key]";
//! # let secret_key = "[Qiniu Secret Key]";
//! # let bucket = "[Bucket Name]";
//! # let key_to_overwrite = "qiniu.mp4";
//! #
//! # let credential = Credential::new(access_key, secret_key);
//! # let config = Config::default();
//! let upload_policy = UploadPolicyBuilder::new_policy_for_object(bucket, key_to_overwrite, &config)
//!                                         .callback_urls(["http://api.example.com/qiniu/upload/callback"], "")
//!                                         .callback_body("{\"key\":\"$(key)\",\"hash\":\"$(etag)\",\"fsize\":$(fsize),\"bucket\":\"$(bucket)\",\"name\":\"$(x:name)\",\"age\":$(x:age)}", "application/json")
//!                                         .build();
//!
//! let upload_token = UploadToken::from_policy(upload_policy, &credential);
//! ```
//!
//! ### 综合上传凭证
//!
//! 上面的生成上传凭证的方法，都是通过设置[上传策略](https://developer.qiniu.com/kodo/manual/put-policy)相关的参数来支持的，这些参数可以通过不同的组合方式来满足不同的业务需求，可以灵活地组织你所需要的上传凭证。
//!
//! ## 服务器直传
//!
//! 服务端直传是指客户利用七牛服务端 SDK 从服务端直接上传文件到七牛云，交互的双方一般都在机房里面，所以服务端可以自己生成上传凭证，然后利用 SDK 中的上传逻辑进行上传，最后从七牛云获取上传的结果，这个过程中由于双方都是业务服务器，所以很少利用到上传回调的功能，而是直接自定义 `returnBody` 来获取自定义的回复内容。
//!
//! ### 文件上传
//!
//! 最简单的就是上传本地文件，直接指定文件的完整路径即可上传。
//!
//! ```rust,no_run
//! # use qiniu_ng::{
//! #     Credential, Client, Config,
//! #     storage::{
//! #         upload_policy::{
//! #             UploadPolicyBuilder,
//! #             UploadPolicy
//! #         },
//! #         upload_token::UploadToken,
//! #         uploader::UploadManager,
//! #     },
//! # };
//! # use std::{path::Path, time::Duration, result::Result, error::Error};
//! #
//! # fn main() -> Result<(), Box<dyn Error>> {
//! # let access_key = "[Qiniu Access Key]";
//! # let secret_key = "[Qiniu Secret Key]";
//! # let bucket = "[Bucket Name]";
//! #
//! # let credential = Credential::new(access_key, secret_key);
//! # let config = Config::default();
//! # let upload_policy = UploadPolicyBuilder::new_policy_for_bucket(bucket, &config)
//! #                                         .build();
//! #
//! # let upload_token = UploadToken::from_policy(upload_policy, &credential);
//! let local_file_path = Path::new("local file path");
//! let upload_manager = UploadManager::new(config);
//! let upload_response = upload_manager.for_upload_token(upload_token)?
//!                                     .upload_file(local_file_path, "local file name", None)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### 数据流上传
//!
//! 如果要上传的数据存在于内存中或输入流中，可以使用基于 `std::io::Read` 的特性上传数据。这里给出一个将内存中的字节数组上传的例子：
//!
//! ```rust,no_run
//! # use qiniu_ng::{
//! #     Credential, Client, Config,
//! #     storage::{
//! #         upload_policy::{
//! #             UploadPolicyBuilder,
//! #             UploadPolicy
//! #         },
//! #         upload_token::UploadToken,
//! #         uploader::UploadManager,
//! #     },
//! # };
//! # use std::{path::Path, time::Duration, io::{Read, Cursor}, fs::File, result::Result, error::Error};
//! #
//! # fn main() -> Result<(), Box<dyn Error>> {
//! # let access_key = "[Qiniu Access Key]";
//! # let secret_key = "[Qiniu Secret Key]";
//! # let bucket = "[Bucket Name]";
//! # let mut bytes = Vec::new();
//! #
//! # File::open("/etc/services")?.read_to_end(&mut bytes)?;
//! #
//! # let credential = Credential::new(access_key, secret_key);
//! # let config = Config::default();
//! # let upload_policy = UploadPolicyBuilder::new_policy_for_bucket(bucket, &config)
//! #                                         .build();
//! #
//! # let upload_token = UploadToken::from_policy(upload_policy, &credential);
//! let stream = Cursor::new(bytes);
//! let upload_manager = UploadManager::new(config);
//! let upload_response = upload_manager.for_upload_token(upload_token)?
//!                                     .upload_stream(stream, "file name", None)?;
//! # Ok(())
//! # }
//! ```
//!
//! 再给出一个将 STDIN 输入的数据上传的例子：
//!
//! ```rust,no_run
//! # use qiniu_ng::{
//! #     Credential, Client, Config,
//! #     storage::{
//! #         upload_policy::{
//! #             UploadPolicyBuilder,
//! #             UploadPolicy
//! #         },
//! #         upload_token::UploadToken,
//! #         uploader::UploadManager,
//! #     },
//! # };
//! # use std::{path::Path, time::Duration, io::stdin, result::Result, error::Error};
//! #
//! # fn main() -> Result<(), Box<dyn Error>> {
//! # let access_key = "[Qiniu Access Key]";
//! # let secret_key = "[Qiniu Secret Key]";
//! # let bucket = "[Bucket Name]";
//! #
//! # let credential = Credential::new(access_key, secret_key);
//! # let config = Config::default();
//! # let upload_policy = UploadPolicyBuilder::new_policy_for_bucket(bucket, &config)
//! #                                         .build();
//! #
//! # let upload_token = UploadToken::from_policy(upload_policy, &credential);
//! let upload_manager = UploadManager::new(config);
//! let upload_response = upload_manager.for_upload_token(upload_token)?
//!                                     .upload_stream(stdin(), "file name", None)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### 文件上传策略
//!
//! 默认情况下，对于尺寸大于 4 MB 的文件，SDK 默认自动使用分片上传的方式来上传，分片上传通过将一个文件切割为标准的块（默认的固定大小为 4 MB，可以通过修改配置增加尺寸，但必须是 4 MB 的倍数），然后通过上传块的方式来进行文件的上传。一个块中的片和另外一个块中的片是可以并发的。分片上传不等于断点续传，但是分片上传可以支持断点续传。
//!
//! 断点续传是将每个块上传完毕的返回的 `context` 保存到本地的文件中持久化，如果本次上传被中断，下次可以从这个进度文件中读取每个块上传的状态，然后继续上传完毕没有完成的块，最后完成文件的拼接。
//!
//! 这里需要注意，只有在块上传完毕之后，才向本地的进度文件写入 `context` 内容。
//!
//! 另外需要注意，每个 `context` 的有效期最长默认是 `7` 天，过期的 `context` 会触发 `701` 的错误，默认情况下，如果 `context` 超过 7 天，SDK 会自动重新上传 `context` 对应的分块。
//!
//! 上述策略中不少参数可以在 `Config` 中配置，这里给出一个修改配置参数的例子：
//!
//! ```rust
//! # use qiniu_ng::{
//! #   Config, ConfigBuilder,
//! #   storage::{
//! #     recorder::FileSystemRecorder,
//! #     uploader::UploadRecorderBuilder,
//! #   },
//! # };
//! # use std::{time::Duration, path::Path};
//! let config = ConfigBuilder::default()
//!                            .upload_threshold(16 * 1024 * 1024) // 修改成尺寸大于 16 MB 的文件才使用分片上传
//!                            .upload_block_size(8 * 1024 * 1024) // 每个分块尺寸修改为 8 MB
//!                            .upload_recorder(
//!                               UploadRecorderBuilder::default()
//!                                 .recorder(FileSystemRecorder::from(Path::new("/recorder/data"))) // 修改上传进度记录文件的存储目录
//!                                 .upload_block_lifetime(Duration::from_secs(5 * 24 * 60 * 60)) // 每个分块的有效期减少为 5 天
//!                                 .build()
//!                            )
//!                            .build();
//! ```
//!
//! ### 业务服务器验证七牛回调
//!
//! 在上传策略里面设置了上传回调相关参数的时候，七牛在文件上传到服务器之后，会主动地向 `callbackUrl` 发送 `POST` 请求的回调，回调的内容为 `callbackBody` 模版所定义的内容，如果这个模版里面引用了魔法变量或者自定义变量，那么这些变量会被自动填充对应的值，然后在发送给业务服务器。
//!
//! 业务服务器在收到来自七牛的回调请求的时候，可以根据请求头部的 `Authorization` 字段来进行验证，查看该请求是否是来自七牛的未经篡改的请求。
//!
//! Rust SDK 提供了一个方法 `Credential::is_valid_request()` 用于验证回调的请求：
//!
//! ```rust,no_run
//! use qiniu_http::{Method, RequestBuilder};
//! use qiniu_ng::Credential;
//!
//! let access_key = "[Qiniu Access Key]";
//! let secret_key = "[Qiniu Secret Key]";
//! let credential = Credential::new(access_key, secret_key);
//! # let request_method = Method::POST;
//! # let request_url = "http://api.example.com/qiniu/upload/callback";
//! # let request_header_name = "";
//! # let request_header_value = "";
//! # let request_body = "{\"key\":\"github-x.png\",\"hash\":\"FqKXVdTvIx_mPjOYdjDyUSy_H1jr\",\"fsize\":6091,\"bucket\":\"if-pbl\",\"name\":\"github logo\"}".as_bytes();
//!
//! let is_valid = credential.is_valid_request(&RequestBuilder::default()
//!                                                            .method(request_method)
//!                                                            .url(request_url)
//!                                                            .header(request_header_name, request_header_value)
//!                                                            .body(request_body)
//!                                                            .build());
//! ```
//!
//! ## 私有云配置
//!
//! 默认情况下，Rust SDK 内置了七牛公有云存储的配置。如果需要使用七牛私有云，则需要对 `Config` 中的配置作出必要的调整，这里给出一个例子：
//!
//! ```rust
//! # use qiniu_ng::{
//! #   Config, ConfigBuilder,
//! #   storage::{
//! #     recorder::FileSystemRecorder,
//! #     uploader::UploadRecorderBuilder,
//! #   },
//! # };
//! # use std::{time::Duration, path::Path};
//! let config = ConfigBuilder::default()
//!                            .use_https(true) // 设置为使用 HTTPS 协议
//!                            .uc_host("uc.example.com") // 设置 UC 服务器地址
//!                            .rs_host("rs.example.com") // 设置 RS 服务器地址
//!                            .rsf_host("rsf.example.com") // 设置 RSF 服务器地址
//!                            .api_host("api.example.com") // 设置 API 服务器地址
//!                            .build();
//! ```

#![recursion_limit = "256"]

mod client;
pub use client::Client;

mod credential;
pub use credential::Credential;

pub mod config;
pub use config::{Config, ConfigBuilder};

pub mod http;
pub mod storage;
pub mod utils;
