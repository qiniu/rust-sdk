use qiniu_http::{Request, Response, Result};

/// HTTP 请求前回调函数
pub trait HTTPBeforeAction {
    fn before_call(&self, request: &mut Request) -> Result<()>;
}

/// HTTP 请求响应后回调函数
pub trait HTTPAfterAction {
    fn after_call(&self, request: &mut Request, response: &mut Response) -> Result<()>;
}
