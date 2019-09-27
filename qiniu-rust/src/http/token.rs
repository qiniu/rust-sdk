use super::super::credential::Credential;
use qiniu_http::Request;

#[derive(Debug, Clone)]
pub enum Token {
    QBox(Credential),
    Qiniu(Credential),
    V1(Credential),
    V2(Credential),
    None,
    Null,
}

impl Token {
    pub(crate) fn sign(&self, req: &mut Request) {
        match self {
            Token::None | Token::Null => {
                return;
            }
            _ => {}
        }

        let url = req.url();
        let method = req.method();
        let content_type = req.headers().get("Content-Type").map(|v| v.to_owned());

        match self {
            Token::QBox(credential) | Token::V1(credential) => {
                if let Ok(authorization) = credential.authorization_v1_for_request(&url, content_type, req.body()) {
                    req.headers_mut().insert("Authorization".into(), authorization.into());
                }
            }
            Token::Qiniu(credential) | Token::V2(credential) => {
                if let Ok(authorization) =
                    credential.authorization_v2_for_request(method, &url, content_type, req.body())
                {
                    req.headers_mut().insert("Authorization".into(), authorization.into());
                }
            }
            &Token::None | &Token::Null => {
                return;
            }
        }
    }
}
