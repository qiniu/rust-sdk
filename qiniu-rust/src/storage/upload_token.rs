use super::UploadPolicy;
use crate::utils::{auth::Auth, base64};
use error_chain::error_chain;
use std::{convert::From, fmt, sync::Arc};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UploadToken {
    Token(String),
    Policy(UploadPolicy, Arc<Auth>),
}

impl UploadToken {
    pub fn from_token<T: Into<String>>(t: T) -> UploadToken {
        UploadToken::Token(t.into())
    }

    pub fn from_policy(policy: UploadPolicy, auth: Arc<Auth>) -> UploadToken {
        UploadToken::Policy(policy, auth)
    }

    pub fn policy(self) -> Result<UploadPolicy> {
        match self {
            UploadToken::Token(token) => {
                let encoded_policy = token
                    .splitn(3, ':')
                    .last()
                    .ok_or_else(|| ErrorKind::InvalidUploadTokenFormat)?;
                let decoded_policy =
                    base64::decode(encoded_policy.as_bytes()).map_err(|err| ErrorKind::Base64DecodeError(err))?;
                Ok(UploadPolicy::from_json_slice(&decoded_policy).map_err(|err| ErrorKind::JSONDecodeError(err))?)
            }
            UploadToken::Policy(policy, _) => Ok(policy),
        }
    }

    pub fn token(self) -> String {
        match self {
            UploadToken::Token(token) => token,
            UploadToken::Policy(policy, auth) => auth.sign_upload_policy(&policy),
        }
    }
}

impl fmt::Display for UploadToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UploadToken::Token(ref token) => fmt::Display::fmt(token, f),
            UploadToken::Policy(ref policy, ref auth) => fmt::Display::fmt(&auth.sign_upload_policy(policy), f),
        }
    }
}

impl From<String> for UploadToken {
    fn from(s: String) -> Self {
        Self::from_token(s)
    }
}

impl From<&str> for UploadToken {
    fn from(s: &str) -> Self {
        Self::from_token(s)
    }
}

impl From<UploadToken> for String {
    fn from(upload_token: UploadToken) -> Self {
        upload_token.token()
    }
}

error_chain! {
    errors {
        InvalidUploadTokenFormat {
            description("Invalid upload token format")
            display("Invalid upload token format")
        }
        Base64DecodeError(err: base64::DecodeError) {
            description("Base64 decode error")
            display("Base64 decode error: {}", err)
        }
        JSONDecodeError(err: serde_json::Error) {
            description("JSON decode error")
            display("JSON decode error: {}", err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::UploadPolicyBuilder, *};
    use crate::config::Config;

    #[test]
    fn test_build_upload_token_from_upload_policy() {
        let policy = UploadPolicyBuilder::new_policy_for_file("test_bucket", "test:file", &Config::default()).build();
        let token = UploadToken::from_policy(policy, Arc::new(get_auth())).token();
        assert!(token.starts_with(get_auth().access_key()));
        let token = UploadToken::from_token(token);
        let policy = token.to_owned().policy().unwrap();
        assert_eq!(policy.bucket(), Some("test_bucket"));
        assert_eq!(policy.key(), Some("test:file"));
        accept_string(token.to_owned().into());
        accept_upload_token(&token.to_string().into());
        accept_upload_token(&token.to_string().as_str().into());
    }

    fn accept_string(_: String) {}
    fn accept_upload_token(_: &UploadToken) {}

    fn get_auth() -> Auth {
        Auth::new("abcdefghklmnopq", "1234567890")
    }
}
