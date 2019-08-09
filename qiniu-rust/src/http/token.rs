use super::super::utils::auth::Auth;
use http::{header, Request};
use std::io::{Cursor, Read};

pub trait Token {
    fn sign(req: &mut Request<Box<Read>>, auth: &Auth);
}

pub fn qbox() -> impl Token {
    QBoxTokenGenerator
}

pub fn v1() -> impl Token {
    QBoxTokenGenerator
}

pub fn qiniu() -> impl Token {
    QiniuTokenGenerator
}

pub fn v2() -> impl Token {
    QiniuTokenGenerator
}

pub fn null() -> impl Token {
    NoneTokenGenerator
}

pub fn none() -> impl Token {
    NoneTokenGenerator
}

struct QBoxTokenGenerator;
struct QiniuTokenGenerator;
struct NoneTokenGenerator;

impl Token for QBoxTokenGenerator {
    fn sign(req: &mut Request<Box<Read>>, auth: &Auth) {
        let url = req.uri().to_string();
        let content_type = req
            .headers_mut()
            .get(header::CONTENT_TYPE)
            .map(|v| v.to_str().map(|s| s.to_owned()).ok())
            .unwrap_or(None);
        let (mut body, mut body_buf) = (None::<&[u8]>, Vec::new());

        if let Some(content_type) = content_type.as_ref() {
            if Auth::will_push_body_v1(content_type) {
                if req.body_mut().read_to_end(&mut body_buf).is_ok() {
                    body = Some(body_buf.as_slice());
                    *req.body_mut() = Box::new(Cursor::new(body_buf.to_owned()));
                }
            }
        }
        if let Ok(authorization) = auth.authorization_v1_for_request(&url, content_type, body) {
            if let Ok(authorization_header_value) = header::HeaderValue::from_str(&authorization) {
                req.headers_mut()
                    .insert(header::AUTHORIZATION, authorization_header_value);
            }
        }
    }
}

impl Token for QiniuTokenGenerator {
    fn sign(req: &mut Request<Box<Read>>, auth: &Auth) {
        let url = req.uri().to_string();
        let content_type = req
            .headers_mut()
            .get(header::CONTENT_TYPE)
            .map(|v| v.to_str().map(|s| s.to_owned()).ok())
            .unwrap_or(None);
        let (mut body, mut body_buf) = (None::<&[u8]>, Vec::new());

        if let Some(content_type) = content_type.as_ref() {
            if Auth::will_push_body_v2(content_type) {
                if req.body_mut().read_to_end(&mut body_buf).is_ok() {
                    body = Some(body_buf.as_slice());
                    *req.body_mut() = Box::new(Cursor::new(body_buf.to_owned()));
                }
            }
        }
        if let Ok(authorization) =
            auth.authorization_v2_for_request(req.method(), &url, content_type, body)
        {
            if let Ok(authorization_header_value) = header::HeaderValue::from_str(&authorization) {
                req.headers_mut()
                    .insert(header::AUTHORIZATION, authorization_header_value);
            }
        }
    }
}

impl Token for NoneTokenGenerator {
    fn sign(_req: &mut Request<Box<Read>>, _auth: &Auth) {}
}
