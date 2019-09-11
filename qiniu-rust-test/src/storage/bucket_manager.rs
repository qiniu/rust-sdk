#[cfg(test)]
mod tests {
    use chrono::offset::Utc;
    use qiniu::{storage::RegionId, Client};
    use qiniu_test_utils::env;
    use std::{boxed::Box, default::Default, error::Error, result::Result};

    #[test]
    fn test_storage_bucket_manager_list_buckets() -> Result<(), Box<dyn Error>> {
        let bucket_names = get_client().bucket_manager().bucket_names()?;
        assert!(bucket_names.contains(&"z0-bucket".into()));
        assert!(bucket_names.contains(&"z1-bucket".into()));
        assert!(bucket_names.contains(&"z2-bucket".into()));
        assert!(bucket_names.contains(&"na-bucket".into()));
        assert!(bucket_names.contains(&"as-bucket".into()));
        Ok(())
    }

    #[test]
    fn test_storage_bucket_manager_new_bucket_and_drop() -> Result<(), Box<dyn Error>> {
        let client = get_client();
        let bucket_manager = client.bucket_manager();
        let bucket_name: String = format!("test-bucket-{}", Utc::now().timestamp_nanos());
        bucket_manager.create_bucket(&bucket_name, RegionId::Z2)?;
        assert!(bucket_manager.bucket_names()?.contains(&bucket_name));
        bucket_manager.drop_bucket(&bucket_name)?;
        assert!(!bucket_manager.bucket_names()?.contains(&bucket_name));
        Ok(())
    }

    fn get_client() -> Client {
        let e = env::get();
        Client::new(e.access_key().to_owned(), e.secret_key().to_owned(), Default::default())
    }
}
