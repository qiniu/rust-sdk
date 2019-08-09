use super::super::config::Config;
use super::super::utils::auth::Auth;
use std::rc::Rc;

pub(crate) struct Client {
    auth: Rc<Auth>,
    config: Rc<Config>,
}

impl Client {
    pub(crate) fn new(auth: Rc<Auth>, config: Rc<Config>) -> Client {
        Client {
            auth: auth,
            config: config,
        }
    }
}
