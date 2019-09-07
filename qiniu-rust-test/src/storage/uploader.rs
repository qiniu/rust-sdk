#[cfg(test)]
mod tests {
    use chrono::offset::Utc;
    use qiniu::{storage::UploadPolicyBuilder, utils::etag, Client};
    use qiniu_test_utils::{env, temp_file::create_temp_file};
    use std::{boxed::Box, error::Error, result::Result};

    #[test]
    fn test_upload_file_512k() -> Result<(), Box<dyn Error>> {
        let temp_path = create_temp_file(512 * 1024)?.into_temp_path();
        let etag = etag::from_file(&temp_path)?;
        let key = format!("test-512k-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object("z0-bucket", &key, &Default::default()).build();
        let result = get_client()
            .bucket_manager()
            .uploader()
            .for_upload_policy(policy)?
            .key(&key)
            .var("var_key1", "var_value1")
            .var("var_key2", "var_value2")
            .metadata("metadata_key1", "metadata_value1")
            .metadata("metadata_key2", "metadata_value2")
            .upload_file(&temp_path, Some("512k"), Some(mime::IMAGE_PNG))?;

        assert_eq!(result.key(), Some(key.as_str()));
        assert_eq!(result.hash(), Some(etag.as_str()));
        Ok(())
    }

    fn get_client() -> Client {
        let e = env::get();
        Client::new(e.access_key().to_owned(), e.secret_key().to_owned(), Default::default())
    }
}
