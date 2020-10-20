use std::result;

mod error;
pub use qiniu_http::{Response, ResponseBody, ResponseBuilder};

pub use error::{Error as ResponseError, ErrorKind as ResponseErrorKind};
pub type APIResult<T> = result::Result<T, ResponseError>;
