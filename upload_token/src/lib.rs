#![cfg_attr(feature = "docs", feature(doc_cfg))]

mod file_type;
mod upload_policy;
mod upload_token;
pub use file_type::{FileType, InvalidFileType};
pub use serde_json;
pub use upload_policy::{UploadPolicy, UploadPolicyBuilder};
use upload_token::FromUploadPolicy;
pub use upload_token::{
    BucketUploadTokenProvider, GenerateError, GenerateResult, ParseError, ParseResult,
    StaticUploadTokenProvider, UploadTokenProvider,
};
