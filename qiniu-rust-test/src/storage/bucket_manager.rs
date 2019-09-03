#[cfg(test)]
mod tests {
    use chrono::offset::Utc;
    use qiniu::{storage::RegionId, Client};
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

    #[test]
    fn test_new_bucket_and_drop() {
        let client = get_client();
        let bucket_manager = client.bucket_manager();
        let bucket_name: String = format!("test-bucket-{}", Utc::now().timestamp_nanos());
        bucket_manager.create_bucket(&bucket_name, RegionId::Z2).unwrap();
        assert!(bucket_manager.bucket_names().unwrap().contains(&bucket_name));
        bucket_manager.drop_bucket(&bucket_name).unwrap();
        assert!(!bucket_manager.bucket_names().unwrap().contains(&bucket_name));
    }

    fn get_client() -> Client {
        let e = env::get();
        Client::new(e.access_key().to_owned(), e.secret_key().to_owned(), Default::default())
    }
}
