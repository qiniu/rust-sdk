#[cfg(test)]
mod tests {
    use chrono::offset::Utc;
    use qiniu::{
        storage::{upload_policy::UploadPolicyBuilder, uploader::UploadErrorKind},
        utils::etag,
        Client, Config, ConfigBuilder,
    };
    use qiniu_http::ErrorKind as HTTPErrorKind;
    use qiniu_test_utils::{env, temp_file::create_temp_file};
    use serde_json::json;
    use std::{
        boxed::Box,
        error::Error,
        io::{Seek, SeekFrom},
        result::Result,
        sync::atomic::{
            AtomicUsize,
            Ordering::{AcqRel, Acquire},
        },
    };

    #[test]
    fn test_storage_uploader_upload_file_with_return_url() -> Result<(), Box<dyn Error>> {
        let config = Config::default();
        let temp_path = create_temp_file(1 << 19)?.into_temp_path();
        let key = format!("test-512k-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object("z0-bucket", &key, &config)
            .return_url("http://www.qiniu.com")
            .build();
        let err = get_client(config)
            .upload()
            .for_upload_policy(policy)?
            .key(&key)
            .upload_file(&temp_path, Some("512k"), Some(mime::IMAGE_PNG))
            .unwrap_err();
        match err.kind() {
            UploadErrorKind::QiniuError(e) => match e.error_kind() {
                HTTPErrorKind::UnexpectedRedirect => {
                    return Ok(());
                }
                _ => {}
            },
            _ => {}
        }
        panic!();
    }

    #[test]
    fn test_storage_uploader_upload_file_with_key() -> Result<(), Box<dyn Error>> {
        let config = Config::default();
        let temp_path = create_temp_file(1 << 19)?.into_temp_path();
        let etag = etag::from_file(&temp_path)?;
        let key = format!("test-512k-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object("z0-bucket", &key, &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fname\":$(fname),\"var_key1\":$(x:var_key1),\"var_key2\":$(x:var_key2)}")
            .build();
        let last_uploaded = AtomicUsize::new(0);
        let result = get_client(config.clone())
            .upload()
            .for_upload_policy(policy)?
            .key(&key)
            .var("var_key1", "var_value1")
            .var("var_key2", "var_value2")
            .metadata("metadata_key1", "metadata_value1")
            .metadata("metadata_key2", "metadata_value2")
            .on_progress(&|uploaded, total| {
                assert!(total > (1 << 19));
                assert!(uploaded + (1 << 16) >= last_uploaded.swap(uploaded, AcqRel));
            })
            .upload_file(&temp_path, Some("512k"), Some(mime::IMAGE_PNG))?;

        assert!(last_uploaded.load(Acquire) > (1 << 19));
        assert_eq!(result.key(), Some(key.as_str()));
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("var_key1"), Some(&json!("var_value1")));
        assert_eq!(result.get("var_key2"), Some(&json!("var_value2")));
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE

        let key = format!("test-512k-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object("z0-bucket", &key, &Config::default())
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fname\":$(fname),\"var_key1\":$(x:var_key1),\"var_key2\":$(x:var_key2)}")
            .build();
        let last_uploaded = AtomicUsize::new(0);
        let result = get_client(config)
            .upload()
            .for_upload_policy(policy)?
            .always_be_resumeable()
            .key(&key)
            .var("var_key1", "var_value1")
            .var("var_key2", "var_value2")
            .metadata("metadata_key1", "metadata_value1")
            .metadata("metadata_key2", "metadata_value2")
            .on_progress(&|uploaded, total| {
                assert_eq!(total, 1 << 19);
                assert!(uploaded + (1 << 16) >= last_uploaded.swap(uploaded, AcqRel));
            })
            .upload_file(&temp_path, Some("512k"), Some(mime::IMAGE_PNG))?;

        assert_eq!(last_uploaded.load(Acquire), 1 << 19);
        assert_eq!(result.key(), Some(key.as_str()));
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("var_key1"), Some(&json!("var_value1")));
        assert_eq!(result.get("var_key2"), Some(&json!("var_value2")));
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_large_file_with_key() -> Result<(), Box<dyn Error>> {
        const FILE_SIZE: usize = (1 << 28) + (1 << 20);
        let config = Config::default();
        let temp_path = create_temp_file(FILE_SIZE)?.into_temp_path();
        let etag = etag::from_file(&temp_path)?;
        let key = format!("test-257m-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object("z0-bucket", &key, &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fsize\":$(fsize)}")
            .build();
        let last_uploaded = AtomicUsize::new(0);
        let result = get_client(config)
            .upload()
            .for_upload_policy(policy)?
            .key(&key)
            .var("var_key1", "var_value1")
            .var("var_key2", "var_value2")
            .metadata("metadata_key1", "metadata_value1")
            .metadata("metadata_key2", "metadata_value2")
            .on_progress(&|uploaded, total| {
                assert_eq!(total, FILE_SIZE);
                assert!(uploaded + (1 << 16) >= last_uploaded.swap(uploaded, AcqRel));
            })
            .upload_file(&temp_path, Some("257m"), Some(mime::IMAGE_PNG))?;

        assert_eq!(last_uploaded.load(Acquire), FILE_SIZE);
        assert_eq!(result.key(), Some(key.as_str()));
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("fsize"), Some(&json!(FILE_SIZE)));
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_file_with_only_one_part() -> Result<(), Box<dyn Error>> {
        const FILE_SIZE: usize = (1 << 22) + (1 << 20) + (1 << 10) + 1;
        let config = ConfigBuilder::default().upload_block_size(1 << 30).build().unwrap();
        let temp_path = create_temp_file(FILE_SIZE)?.into_temp_path();
        let etag = etag::from_file(&temp_path)?;
        let key = format!("test-5m-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object("z0-bucket", &key, &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fsize\":$(fsize)}")
            .build();
        let last_uploaded = AtomicUsize::new(0);
        let result = get_client(config)
            .upload()
            .for_upload_policy(policy)?
            .always_be_resumeable()
            .key(&key)
            .var("var_key1", "var_value1")
            .var("var_key2", "var_value2")
            .metadata("metadata_key1", "metadata_value1")
            .metadata("metadata_key2", "metadata_value2")
            .on_progress(&|uploaded, total| {
                assert_eq!(total, FILE_SIZE);
                assert!(uploaded + (1 << 16) >= last_uploaded.swap(uploaded, AcqRel));
            })
            .upload_file(&temp_path, Some("5m"), Some(mime::IMAGE_PNG))?;

        assert_eq!(last_uploaded.load(Acquire), FILE_SIZE);
        assert_eq!(result.key(), Some(key.as_str()));
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("fsize"), Some(&json!(FILE_SIZE)));
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_file_without_key() -> Result<(), Box<dyn Error>> {
        let config = Config::default();
        let temp_path = create_temp_file(1 << 20)?.into_temp_path();
        let etag = etag::from_file(&temp_path)?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("z0-bucket", &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fname\":$(fname),\"var_key1\":$(x:var_key1)}")
            .build();
        let last_uploaded = AtomicUsize::new(0);
        let result = get_client(config.clone())
            .upload()
            .for_upload_policy(policy)?
            .var("var_key1", "var_value1")
            .metadata("metadata_key1", "metadata_value1")
            .on_progress(&|uploaded, total| {
                assert!(total > (1 << 20));
                assert!(uploaded + (1 << 16) >= last_uploaded.swap(uploaded, AcqRel));
            })
            .upload_file(&temp_path, Some("1m"), Some(mime::IMAGE_PNG))?;

        assert!(last_uploaded.load(Acquire) > (1 << 20));
        assert!(result.key().is_some());
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("var_key1"), Some(&json!("var_value1")));
        assert_eq!(result.get("var_key2"), None);

        let policy = UploadPolicyBuilder::new_policy_for_bucket("z0-bucket", &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fname\":$(fname),\"var_key1\":$(x:var_key1)}")
            .build();
        let last_uploaded = AtomicUsize::new(0);
        let result = get_client(config)
            .upload()
            .for_upload_policy(policy)?
            .always_be_resumeable()
            .var("var_key1", "var_value1")
            .metadata("metadata_key1", "metadata_value1")
            .on_progress(&|uploaded, total| {
                assert_eq!(total, 1 << 20);
                assert!(uploaded + (1 << 16) >= last_uploaded.swap(uploaded, AcqRel));
            })
            .upload_file(&temp_path, Some("1m"), Some(mime::IMAGE_PNG))?;

        assert_eq!(last_uploaded.load(Acquire), (1 << 20));
        assert!(result.key().is_some());
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("var_key1"), Some(&json!("var_value1")));
        assert_eq!(result.get("var_key2"), None);
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_stream() -> Result<(), Box<dyn Error>> {
        let config = Config::default();
        let (mut file, temp_path) = create_temp_file(1 << 21)?.into_parts();
        file.seek(SeekFrom::Start(0))?;

        let etag = etag::from_file(&temp_path)?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("z0-bucket", &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fname\":$(fname),\"var_key1\":$(x:var_key1)}")
            .build();
        let last_uploaded = AtomicUsize::new(0);
        let result = get_client(config.clone())
            .upload()
            .for_upload_policy(policy)?
            .var("var_key1", "var_value1")
            .metadata("metadata_key1", "metadata_value1")
            .never_be_resumeable()
            .on_progress(&|uploaded, total| {
                assert!(total > (1 << 21));
                assert!(uploaded + (1 << 16) >= last_uploaded.swap(uploaded, AcqRel));
            })
            .upload_stream(&file, None::<String>, None)?;

        assert!(last_uploaded.load(Acquire) > (1 << 21));
        assert!(result.key().is_some());
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("fname"), Some(&json!("")));
        assert_eq!(result.get("var_key1"), Some(&json!("var_value1")));
        assert_eq!(result.get("var_key2"), None);
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE

        file.seek(SeekFrom::Start(0))?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket("z0-bucket", &config).build();
        let result = get_client(config)
            .upload()
            .for_upload_policy(policy)?
            .var("var_key1", "var_value1")
            .metadata("metadata_key1", "metadata_value1")
            .upload_stream(&file, Some("2m"), None)?;

        assert!(result.key().is_some());
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("var_key1"), None);
        assert_eq!(result.get("var_key2"), None);
        Ok(())
    }

    fn get_client(config: Config) -> Client {
        let e = env::get();
        Client::new(e.access_key().to_owned(), e.secret_key().to_owned(), config)
    }
}
