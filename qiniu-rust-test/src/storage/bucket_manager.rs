#[cfg(test)]
mod tests {
    use qiniu::Client;
    use qiniu_test_utils::env;
    use std::default::Default;

    #[test]
    fn test_list_buckets() {
        let bucket_names = get_client().bucket_manager().bucket_names().unwrap();
        assert!(bucket_names.contains(&"z0-bucket".into()));
        assert!(bucket_names.contains(&"z1-bucket".into()));
        assert!(bucket_names.contains(&"z2-bucket".into()));
        assert!(bucket_names.contains(&"na-bucket".into()));
        assert!(bucket_names.contains(&"as-bucket".into()));
    }

    fn get_client() -> Client {
        let e = env::get();
        Client::new(e.access_key().to_owned(), e.secret_key().to_owned(), Default::default())
    }
}
