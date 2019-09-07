use super::super::utils::auth::Auth;
use qiniu_http::Request;

#[derive(Debug, Copy, Clone)]
pub enum Token {
    QBox,
    Qiniu,
    V1,
    V2,
    None,
    Null,
}

impl Token {
    pub(crate) fn sign(&self, req: &mut Request, auth: &Auth) {
        match self {
            &Token::None | &Token::Null => {
                return;
            }
            _ => {}
        }

        let url = req.url();
        let method = req.method();
        let content_type = req.headers().get("Content-Type").map(|v| v.to_owned());
        let mut body = None::<&[u8]>;

        match self {
            &Token::QBox | &Token::V1 => {
                if let Some(content_type) = content_type.as_ref() {
                    if Auth::will_push_body_v1(content_type) {
                        body = req.body()
                    }
                }
                if let Ok(authorization) = auth.authorization_v1_for_request(&url, content_type, body) {
                    req.headers_mut().insert("Authorization".into(), authorization.into());
                }
            }
            &Token::Qiniu | &Token::V2 => {
                if let Some(content_type) = content_type.as_ref() {
                    if Auth::will_push_body_v2(content_type) {
                        body = req.body()
                    }
                }
                if let Ok(authorization) = auth.authorization_v2_for_request(method, &url, content_type, body) {
                    req.headers_mut().insert("Authorization".into(), authorization.into());
                }
            }
            &Token::None | &Token::Null => {
                return;
            }
        }
    }
}
