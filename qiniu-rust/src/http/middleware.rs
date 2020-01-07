use qiniu_http::{Request, Response, Result};

pub trait HTTPBeforeAction {
    fn before_call(&self, request: &mut Request) -> Result<()>;
}

pub trait HTTPAfterAction {
    fn after_call(&self, request: &mut Request, response: &mut Response) -> Result<()>;
}
