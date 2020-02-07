#[cfg(test)]
mod tests {
    use chrono::offset::Utc;
    use qiniu_ng::{storage::region::RegionId, Client, Config};
    use qiniu_test_utils::env;
    use std::{boxed::Box, default::Default, error::Error, result::Result};

    #[test]
    fn test_storage_list_buckets() -> Result<(), Box<dyn Error>> {
        let bucket_names = get_client(Config::default()).storage().bucket_names()?;
        assert!(bucket_names.contains(&"z0-bucket".into()));
        assert!(bucket_names.contains(&"z1-bucket".into()));
        assert!(bucket_names.contains(&"z2-bucket".into()));
        assert!(bucket_names.contains(&"na-bucket".into()));
        assert!(bucket_names.contains(&"as-bucket".into()));
        Ok(())
    }

    #[test]
    fn test_storage_new_bucket_and_drop() -> Result<(), Box<dyn Error>> {
        let client = get_client(Config::default());
        let storage_manager = client.storage();
        let bucket_name: String = format!("test-bucket-{}", Utc::now().timestamp_nanos());
        storage_manager.create_bucket(&bucket_name, RegionId::Z2)?;
        assert!(storage_manager.bucket_names()?.contains(&bucket_name));
        storage_manager.drop_bucket(&bucket_name)?;
        assert!(!storage_manager.bucket_names()?.contains(&bucket_name));
        Ok(())
    }

    #[test]
    fn test_storage_get_bucket() -> Result<(), Box<dyn Error>> {
        let client = get_client(Config::default());
        let bucket = client.storage().bucket("z0-bucket").build();
        assert_eq!(bucket.regions()?.count(), 2);
        let domains = bucket.domains()?;
        assert_eq!(domains.len(), 2);
        Ok(())
    }

    fn get_client(config: Config) -> Client {
        let e = env::get();
        Client::new(e.access_key().to_owned(), e.secret_key().to_owned(), config)
    }
}
