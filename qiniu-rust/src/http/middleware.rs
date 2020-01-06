use qiniu_http::{Request, Response, Result};

pub trait HTTPBeforeAction {
    fn before_call(&self, request: &mut Request) -> Result<()>;
}

pub trait HTTPAfterAction {
    fn after_call(&self, request: &mut Request, response: &mut Response) -> Result<()>;
}

pub enum HTTPBeforeActionHandler {
    Dynamic(Box<dyn HTTPBeforeAction + Send + Sync>),
    Static(fn(request: &mut Request) -> Result<()>),
}

impl HTTPBeforeAction for HTTPBeforeActionHandler {
    fn before_call(&self, request: &mut Request) -> Result<()> {
        match self {
            HTTPBeforeActionHandler::Dynamic(dynamic) => dynamic.before_call(request),
            HTTPBeforeActionHandler::Static(f) => (f)(request),
        }
    }
}

pub enum HTTPAfterActionHandler {
    Dynamic(Box<dyn HTTPAfterAction + Send + Sync>),
    Static(fn(request: &mut Request, response: &mut Response) -> Result<()>),
}

impl HTTPAfterAction for HTTPAfterActionHandler {
    fn after_call(&self, request: &mut Request, response: &mut Response) -> Result<()> {
        match self {
            HTTPAfterActionHandler::Dynamic(dynamic) => dynamic.after_call(request, response),
            HTTPAfterActionHandler::Static(f) => (f)(request, response),
        }
    }
}
