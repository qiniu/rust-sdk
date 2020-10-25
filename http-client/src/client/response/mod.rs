use std::result;

mod error;
pub use qiniu_http::{SyncResponse, SyncResponseBuilder};

#[cfg(feature = "async")]
pub use qiniu_http::{AsyncResponse, AsyncResponseBuilder};

pub use error::{Error as ResponseError, ErrorKind as ResponseErrorKind};
pub type APIResult<T> = result::Result<T, ResponseError>;
