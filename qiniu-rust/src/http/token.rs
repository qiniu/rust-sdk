use super::super::credential::Credential;
use qiniu_http::Request;
use std::borrow::Cow;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Token<'t> {
    version: Version,
    credential: Cow<'t, Credential>,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum Version {
    V1,
    V2,
}

impl<'t> Token<'t> {
    pub(crate) fn new(version: Version, credential: Cow<'t, Credential>) -> Self {
        Self { version, credential }
    }

    pub(crate) fn sign(&self, req: &mut Request) {
        let url = req.url();
        let method = req.method();

        match self.version {
            Version::V1 => {
                if let Ok(authorization) = self.credential.authorization_v1_for_request(
                    &url,
                    req.headers().get(&"Content-Type".into()),
                    req.body().as_ref().map(|body| body.as_ref()),
                ) {
                    req.headers_mut().insert("Authorization".into(), authorization.into());
                }
            }
            Version::V2 => {
                if let Ok(authorization) = self.credential.authorization_v2_for_request(
                    method,
                    &url,
                    req.headers(),
                    req.body().as_ref().map(|body| body.as_ref()),
                ) {
                    req.headers_mut().insert("Authorization".into(), authorization.into());
                }
            }
        }
    }
}
