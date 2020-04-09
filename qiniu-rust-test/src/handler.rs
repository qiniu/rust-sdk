#[cfg(test)]
mod tests {
    use chrono::offset::Utc;
    use qiniu_http::{Method, Request, Response, ResponseBody, Result};
    use qiniu_ng::{
        http::{HTTPAfterAction, HTTPBeforeAction},
        storage::uploader::UploadPolicyBuilder,
        Client, Config, ConfigBuilder, Credential,
    };
    use qiniu_test_utils::{env, temp_file::create_temp_file};
    use std::{error::Error, result::Result as StdResult};

    struct HTTPBeforeActionTester;

    impl HTTPBeforeAction for HTTPBeforeActionTester {
        fn before_call(&self, request: &mut Request) -> Result<()> {
            if request.url() == "https://upload.qiniup.com/" {
                assert_eq!(request.method(), Method::POST);
                assert!(request
                    .headers()
                    .get(&"Content-Type".into())
                    .unwrap()
                    .starts_with("multipart/form-data"));
                assert_eq!(
                    request.headers().get(&"Accept".into()),
                    Some(&"application/json".into())
                );
                assert!(!request.body().is_empty());
            }
            Ok(())
        }
    }

    #[test]
    fn test_config_http_request_before_action_handlers() -> StdResult<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .append_http_request_before_action_handler(HTTPBeforeActionTester)
            .build();
        let temp_path = create_temp_file(1)?.into_temp_path();
        let key = format!("test-1b-{}", Utc::now().timestamp_nanos());
        let bucket = get_client(config).storage().bucket("z0-bucket").build();
        bucket.uploader().key(&key).upload_file(&temp_path, "1b", None)?;
        bucket.object(key).delete()?;

        Ok(())
    }

    struct HTTPAfterActionTester;

    impl HTTPAfterAction for HTTPAfterActionTester {
        fn after_call(&self, request: &mut Request, response: &mut Response) -> Result<()> {
            if request.url() == "https://upload.qiniup.com/" {
                assert_eq!(response.status_code(), 200);
                assert!(response.headers().get(&"X-Reqid".into()).is_some());
                assert!(response.server_ip().is_some());
                assert_eq!(response.server_port(), 443);
                assert!(response.body_len().unwrap() > 0);

                *response.body_mut() = Some(ResponseBody::Bytes(b"[]".to_vec()));
                response.headers_mut().insert("Content-Length".into(), "2".into());
            }

            Ok(())
        }
    }

    #[test]
    fn test_config_http_request_after_action_handlers() -> StdResult<(), Box<dyn Error>> {
        let config = ConfigBuilder::default()
            .append_http_request_after_action_handler(HTTPAfterActionTester)
            .build();
        let temp_path = create_temp_file(1)?.into_temp_path();
        let key = format!("test-1b-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object("z0-bucket", &key, &config).build();
        let response = get_client(config)
            .upload()
            .upload_for_upload_policy(policy, get_credential())?
            .key(&key)
            .upload_file(&temp_path, "1b", None)
            .unwrap();
        assert_eq!(response.into_bytes(), b"[]");
        Ok(())
    }

    fn get_client(config: Config) -> Client {
        let e = env::get();
        Client::new(e.access_key().to_owned(), e.secret_key().to_owned(), config)
    }

    fn get_credential() -> Credential {
        let e = env::get();
        Credential::new(e.access_key().to_owned(), e.secret_key().to_owned())
    }
}
