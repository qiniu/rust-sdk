use qiniu_http::{Request, Response, Result};

/// HTTP 请求前回调函数
pub trait HTTPBeforeAction: Sync + Send {
    /// HTTP 请求前回调函数
    ///
    /// 您可以在回调函数中对请求的数据进行修改，或是直接抛出错误
    fn before_call(&self, request: &mut Request) -> Result<()>;
}

/// HTTP 请求响应后回调函数
pub trait HTTPAfterAction: Sync + Send {
    /// HTTP 请求响应后回调函数
    ///
    /// 您可以在回调函数中对请求和响应的数据进行修改，或是直接抛出错误
    fn after_call(&self, request: &mut Request, response: &mut Response) -> Result<()>;
}
