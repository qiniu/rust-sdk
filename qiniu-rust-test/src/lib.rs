#[cfg(test)]
mod tests {
    use qiniu::{http::error::ErrorKind as QiniuErrorKind, http::Client, utils::auth::Auth};
    use qiniu_test_utils::env;
    use std::error::Error;
    use std::{default::Default, sync::Arc};

    #[test]
    fn test_call() {
        let err = Client::new(Arc::new(get_auth()), Arc::new(Default::default()))
            .get("", &["http://up.qiniup.com"])
            .no_body()
            .send()
            .unwrap_err();
        assert_eq!(
            err.description(),
            QiniuErrorKind::MethodNotAllowedError(405, "only allow POST method".into()).description(),
        );
    }

    fn get_auth() -> Auth {
        let e = env::get();
        Auth::new(e.access_key().to_owned(), e.secret_key().to_owned())
    }
}
