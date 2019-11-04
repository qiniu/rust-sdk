use super::super::credential::Credential;
use qiniu_http::Request;
use std::borrow::Cow;

// TODO: Think about reference credential here
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Token<'t> {
    V1(Cow<'t, Credential>),
    V2(Cow<'t, Credential>),
    None,
}

impl<'t> Token<'t> {
    pub(crate) fn sign(&self, req: &mut Request) {
        if self == &Token::None {
            return;
        }

        let url = req.url();
        let method = req.method();

        match self {
            Token::V1(credential) => {
                if let Ok(authorization) =
                    credential.authorization_v1_for_request(&url, req.headers().get(&"Content-Type".into()), req.body())
                {
                    req.headers_mut().insert("Authorization".into(), authorization.into());
                }
            }
            Token::V2(credential) => {
                if let Ok(authorization) =
                    credential.authorization_v2_for_request(method, &url, req.headers(), req.body())
                {
                    req.headers_mut().insert("Authorization".into(), authorization.into());
                }
            }
            Token::None => {}
        }
    }
}
