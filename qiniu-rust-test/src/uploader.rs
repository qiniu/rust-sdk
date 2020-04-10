#[cfg(test)]
mod tests {
    use chrono::offset::Utc;
    use matches::matches;
    use qiniu_ng::{
        http::ErrorKind as HTTPErrorKind,
        storage::uploader::{BatchUploadJobBuilder, UploadError, UploadPolicyBuilder},
        utils::etag,
        Client, Config, ConfigBuilder, Credential,
    };
    use qiniu_test_utils::{env, temp_file::create_temp_file};
    use serde_json::json;
    use std::{
        boxed::Box,
        convert::TryInto,
        error::Error,
        io::{Error as IOError, Seek, SeekFrom},
        result::Result,
        sync::{
            atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed},
            Arc, Mutex, RwLock,
        },
        thread::{current, ThreadId},
    };

    #[test]
    fn test_storage_uploader_upload_file_with_return_url() -> Result<(), Box<dyn Error>> {
        let config = Config::default();
        let temp_path = create_temp_file(1 << 19)?.into_temp_path();
        let key = format!("test-512k-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object(bucket_name(), &key, &config)
            .return_url("http://www.qiniu.com")
            .build();
        let err = get_client(config)
            .upload()
            .upload_for_upload_policy(policy, get_credential())?
            .key(&key)
            .upload_file(&temp_path, "512k", Some(mime::IMAGE_PNG))
            .unwrap_err();

        if let UploadError::QiniuError(e) = &err {
            if matches!(e.error_kind(), HTTPErrorKind::UnexpectedRedirect) {
                return Ok(());
            }
        }
        panic!("Unexpected error: {:?}", err);
    }

    #[test]
    fn test_storage_uploader_upload_file_with_non_json_return_body() -> Result<(), Box<dyn Error>> {
        let config = Config::default();
        let temp_path = create_temp_file(1 << 19)?.into_temp_path();
        let key = format!("test-512k-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object(bucket_name(), &key, &config)
            .return_body("$(fname)/$(key)")
            .build();
        let result = get_client(config)
            .upload()
            .upload_for_upload_policy(policy, get_credential())?
            .key(&key)
            .upload_file(&temp_path, "512k", Some(mime::IMAGE_PNG))?;

        assert_eq!(result.key(), None);
        assert_eq!(result.hash(), None);
        assert!(!result.is_json_value());
        assert_eq!(String::from_utf8(result.into_bytes())?, format!("\"512k\"/\"{}\"", key));
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_file_with_key() -> Result<(), Box<dyn Error>> {
        let config = Config::default();
        let temp_path = create_temp_file(1 << 19)?.into_temp_path();
        let etag = etag::from_file(&temp_path)?;
        let key = format!("test-512k-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object(bucket_name(), &key, &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fname\":$(fname),\"var_key1\":$(x:var_key1),\"var_key2\":$(x:var_key2)}")
            .build();
        let last_uploaded = AtomicU64::new(0);
        let result = get_client(config.clone())
            .upload()
            .upload_for_upload_policy(policy, get_credential())?
            .key(&key)
            .var("var_key1", "var_value1")
            .var("var_key2", "var_value2")
            .metadata("metadata_key1", "metadata_value1")
            .metadata("metadata_key2", "metadata_value2")
            .on_progress_ref(&|uploaded, total| {
                assert!(total.unwrap() > (1 << 19));
                last_uploaded.store(uploaded, Relaxed);
            })
            .upload_file(&temp_path, "512k", Some(mime::IMAGE_PNG))?;

        assert!(last_uploaded.load(Relaxed) > (1 << 19));
        assert_eq!(result.key(), Some(key.as_str()));
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("var_key1"), Some(&json!("var_value1")));
        assert_eq!(result.get("var_key2"), Some(&json!("var_value2")));
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_file_with_always_be_resumable_policy() -> Result<(), Box<dyn Error>> {
        let config = Config::default();
        let temp_path = create_temp_file(1 << 19)?.into_temp_path();
        let etag = etag::from_file(&temp_path)?;
        let key = format!("test-512k-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object(bucket_name(), &key, &Config::default())
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fname\":$(fname),\"var_key1\":$(x:var_key1),\"var_key2\":$(x:var_key2)}")
            .build();
        let last_uploaded = AtomicU64::new(0);
        let result = get_client(config)
            .upload()
            .upload_for_upload_policy(policy, get_credential())?
            .always_be_resumable()
            .key(&key)
            .var("var_key1", "var_value1")
            .var("var_key2", "var_value2")
            .metadata("metadata_key1", "metadata_value1")
            .metadata("metadata_key2", "metadata_value2")
            .on_progress_ref(&|uploaded, total| {
                assert_eq!(total.unwrap(), 1 << 19);
                last_uploaded.store(uploaded, Relaxed);
            })
            .upload_file(&temp_path, "512k", Some(mime::IMAGE_PNG))?;

        assert_eq!(last_uploaded.load(Relaxed), 1 << 19);
        assert_eq!(result.key(), Some(key.as_str()));
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("var_key1"), Some(&json!("var_value1")));
        assert_eq!(result.get("var_key2"), Some(&json!("var_value2")));
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_large_file_with_key() -> Result<(), Box<dyn Error>> {
        const FILE_SIZE: u64 = (1 << 23) + (1 << 20);
        let config = Config::default();
        let temp_path = create_temp_file(FILE_SIZE.try_into().unwrap())?.into_temp_path();
        let etag = etag::from_file(&temp_path)?;
        let key = format!("test-9m-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object(bucket_name(), &key, &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fsize\":$(fsize)}")
            .build();
        let last_uploaded = AtomicU64::new(0);
        let result = get_client(config)
            .upload()
            .upload_for_upload_policy(policy, get_credential())?
            .key(&key)
            .var("var_key1", "var_value1")
            .var("var_key2", "var_value2")
            .metadata("metadata_key1", "metadata_value1")
            .metadata("metadata_key2", "metadata_value2")
            .on_progress_ref(&|uploaded, total| {
                assert_eq!(total.unwrap(), FILE_SIZE);
                last_uploaded.store(uploaded, Relaxed);
            })
            .upload_file(&temp_path, "9m", Some(mime::IMAGE_PNG))?;

        assert_eq!(last_uploaded.load(Relaxed), FILE_SIZE);
        assert_eq!(result.key(), Some(key.as_str()));
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("fsize"), Some(&json!(FILE_SIZE)));
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_file_with_only_one_part() -> Result<(), Box<dyn Error>> {
        const FILE_SIZE: u64 = (1 << 20) + (1 << 10) + 1;
        let config = ConfigBuilder::default().build();
        let temp_path = create_temp_file(FILE_SIZE.try_into().unwrap())?.into_temp_path();
        let etag = etag::from_file(&temp_path)?;
        let key = format!("test-5m-{}", Utc::now().timestamp_nanos());
        let policy = UploadPolicyBuilder::new_policy_for_object(bucket_name(), &key, &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fsize\":$(fsize)}")
            .build();
        let last_uploaded = AtomicU64::new(0);
        let thread_id: Mutex<Option<ThreadId>> = Mutex::new(None);
        let result = get_client(config)
            .upload()
            .upload_for_upload_policy(policy, get_credential())?
            .always_be_resumable()
            .key(&key)
            .var("var_key1", "var_value1")
            .var("var_key2", "var_value2")
            .metadata("metadata_key1", "metadata_value1")
            .metadata("metadata_key2", "metadata_value2")
            .on_progress_ref(&|uploaded, total| {
                let mut thread_id = thread_id.lock().unwrap();
                if let Some(thread_id) = *thread_id {
                    assert_eq!(thread_id, current().id());
                } else {
                    *thread_id = Some(current().id());
                }
                assert_eq!(total.unwrap(), FILE_SIZE);
                last_uploaded.store(uploaded, Relaxed);
            })
            .upload_file(&temp_path, "5m", Some(mime::IMAGE_PNG))?;

        assert_eq!(last_uploaded.load(Relaxed), FILE_SIZE);
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
        let policy = UploadPolicyBuilder::new_policy_for_bucket(bucket_name(), &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fname\":$(fname),\"var_key1\":$(x:var_key1)}")
            .build();
        let last_uploaded = AtomicU64::new(0);
        let result = get_client(config.clone())
            .upload()
            .upload_for_upload_policy(policy, get_credential())?
            .var("var_key1", "var_value1")
            .metadata("metadata_key1", "metadata_value1")
            .on_progress_ref(&|uploaded, total| {
                assert!(total.unwrap() > (1 << 20));
                last_uploaded.store(uploaded, Relaxed);
            })
            .upload_file(&temp_path, "1m", Some(mime::IMAGE_PNG))?;

        assert!(last_uploaded.load(Relaxed) > (1 << 20));
        assert!(result.key().is_some());
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("var_key1"), Some(&json!("var_value1")));
        assert_eq!(result.get("var_key2"), None);

        let policy = UploadPolicyBuilder::new_policy_for_bucket(bucket_name(), &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fname\":$(fname),\"var_key1\":$(x:var_key1)}")
            .build();
        let last_uploaded = AtomicU64::new(0);
        let result = get_client(config)
            .upload()
            .upload_for_upload_policy(policy, get_credential())?
            .always_be_resumable()
            .var("var_key1", "var_value1")
            .metadata("metadata_key1", "metadata_value1")
            .on_progress_ref(&|uploaded, total| {
                assert_eq!(total.unwrap(), 1 << 20);
                last_uploaded.store(uploaded, Relaxed);
            })
            .upload_file(&temp_path, "1m", Some(mime::IMAGE_PNG))?;

        assert_eq!(last_uploaded.load(Relaxed), 1 << 20);
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
        let (mut file, temp_path) = create_temp_file(1 << 23)?.into_parts();
        file.seek(SeekFrom::Start(0))?;

        let etag = etag::from_file(&temp_path)?;
        let policy = UploadPolicyBuilder::new_policy_for_bucket(bucket_name(), &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fname\":$(fname),\"var_key1\":$(x:var_key1)}")
            .build();
        let last_uploaded = AtomicU64::new(0);
        let result = get_client(config.clone())
            .upload()
            .upload_for_upload_policy(policy, get_credential())?
            .var("var_key1", "var_value1")
            .metadata("metadata_key1", "metadata_value1")
            .never_be_resumable()
            .on_progress_ref(&|uploaded, total| {
                assert!(total.unwrap() > (1 << 23));
                last_uploaded.store(uploaded, Relaxed);
            })
            .upload_stream(&file, 1 << 23, "", None)?;

        assert!(last_uploaded.load(Relaxed) > (1 << 23));
        assert!(result.key().is_some());
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("fname"), Some(&json!("")));
        assert_eq!(result.get("var_key1"), Some(&json!("var_value1")));
        assert_eq!(result.get("var_key2"), None);
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_stream_without_vars_and_expected_size() -> Result<(), Box<dyn Error>> {
        let config = Config::default();
        let (mut file, temp_path) = create_temp_file(1 << 23)?.into_parts();
        file.seek(SeekFrom::Start(0))?;

        let etag = etag::from_file(&temp_path)?;
        let last_uploaded = AtomicU64::new(0);
        let result = get_client(config.clone())
            .upload()
            .upload_for_bucket(bucket_name(), get_credential())
            .var("var_key1", "var_value1")
            .metadata("metadata_key1", "metadata_value1")
            .on_progress_ref(&|uploaded, total| {
                assert!(total.is_none());
                last_uploaded.store(uploaded, Relaxed);
            })
            .upload_stream(&file, 0, "8m", None)?;

        assert_eq!(last_uploaded.load(Relaxed), 1 << 23);
        assert!(result.key().is_some());
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("var_key1"), None);
        assert_eq!(result.get("var_key2"), None);
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_stream_without_4m_multiples_size() -> Result<(), Box<dyn Error>> {
        let config = Config::default();
        let (mut file, temp_path) = create_temp_file((1 << 23) + 1)?.into_parts();
        file.seek(SeekFrom::Start(0))?;
        let etag = etag::from_file(&temp_path)?;
        let last_uploaded = AtomicU64::new(0);
        let result = get_client(config)
            .upload()
            .upload_for_bucket(bucket_name(), get_credential())
            .var("var_key1", "var_value1")
            .metadata("metadata_key1", "metadata_value1")
            .on_progress_ref(&|uploaded, total| {
                assert_eq!(total.unwrap(), (1 << 23) + 1);
                last_uploaded.store(uploaded, Relaxed);
            })
            .upload_stream(&file, (1 << 23) + 1, "8m+1", None)?;

        assert_eq!(last_uploaded.load(Relaxed), (1 << 23) + 1);
        assert!(result.key().is_some());
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("var_key1"), None);
        assert_eq!(result.get("var_key2"), None);
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_stream_with_never_be_resumable_policy() -> Result<(), Box<dyn Error>> {
        let config = Config::default();
        let (mut file, temp_path) = create_temp_file(1 << 21)?.into_parts();
        file.seek(SeekFrom::Start(0))?;
        let etag = etag::from_file(&temp_path)?;
        let last_uploaded = AtomicU64::new(0);
        let policy = UploadPolicyBuilder::new_policy_for_bucket(bucket_name(), &config).build();
        let result = get_client(config.clone())
            .upload()
            .upload_for_upload_policy(policy, get_credential())?
            .var("var_key1", "var_value1")
            .metadata("metadata_key1", "metadata_value1")
            .never_be_resumable()
            .on_progress_ref(&|uploaded, total| {
                assert!(total.unwrap() > (1 << 21));
                last_uploaded.store(uploaded, Relaxed);
            })
            .upload_stream(&file, 1 << 21, "2m", None)?;

        assert!(last_uploaded.load(Relaxed) > (1 << 21));
        assert!(result.key().is_some());
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("var_key1"), None);
        assert_eq!(result.get("var_key2"), None);
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_small_stream() -> Result<(), Box<dyn Error>> {
        let config = Config::default();
        let (mut file, temp_path) = create_temp_file((1 << 22) - 3)?.into_parts();
        file.seek(SeekFrom::Start(0))?;
        let etag = etag::from_file(&temp_path)?;
        let last_uploaded = AtomicU64::new(0);
        let policy = UploadPolicyBuilder::new_policy_for_bucket(bucket_name(), &config).build();
        let result = get_client(config.clone())
            .upload()
            .upload_for_upload_policy(policy, get_credential())?
            .var("var_key1", "var_value1")
            .metadata("metadata_key1", "metadata_value1")
            .on_progress_ref(&|uploaded, total| {
                assert!(total.unwrap() > (1 << 22) - 3);
                last_uploaded.store(uploaded, Relaxed);
            })
            .upload_stream(&file, (1 << 22) - 3, "2m-3", None)?;

        assert!(last_uploaded.load(Relaxed) > (1 << 22) - 3);
        assert!(result.key().is_some());
        assert_eq!(result.hash(), Some(etag.as_str()));
        assert_eq!(result.get("var_key1"), None);
        assert_eq!(result.get("var_key2"), None);
        // TODO: Verify METADATA & FILE_SIZE & CONTENT_TYPE
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_files_by_batch_uploader() -> Result<(), Box<dyn Error>> {
        const FILE_SIZES: [u64; 12] = [
            (1 << 22) + (1 << 20) + (1 << 10) + 1,
            (1 << 20) + (1 << 20) + (1 << 10) + 1,
            (1 << 21) + (1 << 20) + (1 << 10) + 1,
            (1 << 23) + (1 << 20) + (1 << 10) + 1,
            (1 << 22) + (1 << 20) + 1,
            (1 << 20) + (1 << 20) + 1,
            (1 << 21) + (1 << 20) + 1,
            (1 << 23) + (1 << 20) + 1,
            (1 << 22) + 1,
            (1 << 20) + 1,
            (1 << 21) + 1,
            (1 << 23) + 1,
        ];
        let config = Config::default();
        let temp_paths = FILE_SIZES
            .iter()
            .map(|&size| Ok(create_temp_file(size.try_into().unwrap())?.into_temp_path()))
            .collect::<Result<Vec<_>, IOError>>()?;
        let etags = Arc::new(
            temp_paths
                .iter()
                .map(etag::from_file)
                .collect::<Result<Vec<_>, IOError>>()?,
        );
        let policy = UploadPolicyBuilder::new_policy_for_bucket(bucket_name(), &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fname\":$(fname),\"var_key1\":$(x:var_key1)}")
            .build();
        let mut batch_uploader = get_client(config)
            .upload()
            .batch_uploader_for_upload_policy(policy, get_credential())?;
        batch_uploader.expected_jobs_count(FILE_SIZES.len()).thread_pool_size(4);
        let thread_ids = Arc::new(RwLock::new(Vec::with_capacity(12)));
        let completed = Arc::new(AtomicUsize::new(0));
        for (idx, temp_path) in temp_paths.iter().enumerate() {
            let etags = etags.clone();
            let thread_ids_1 = thread_ids.clone();
            let thread_ids_2 = thread_ids.clone();
            let completed = completed.clone();
            batch_uploader.push_job(
                BatchUploadJobBuilder::default()
                    .key(format!("test-batch-{}-{}", idx, Utc::now().timestamp_nanos()))
                    .var("var_key1", format!("var_value_{}", idx))
                    .on_progress(move |_, _| {
                        let cur_thread_id = current().id();
                        if !thread_ids_1.read().unwrap().contains(&cur_thread_id) {
                            assert!(thread_ids_1.read().unwrap().len() <= 4);
                            thread_ids_1.write().unwrap().push(cur_thread_id);
                        }
                    })
                    .on_completed(move |result| {
                        let cur_thread_id = current().id();
                        if !thread_ids_2.read().unwrap().contains(&cur_thread_id) {
                            assert!(thread_ids_2.read().unwrap().len() <= 4);
                            thread_ids_2.write().unwrap().push(cur_thread_id);
                        }
                        let response = result.unwrap();
                        assert_eq!(response.hash(), etags.get(idx).map(|s| s.as_ref()));
                        assert_eq!(
                            response.get("fname").and_then(|f| f.as_str()),
                            Some(format!("filename-{}", idx).as_str())
                        );
                        assert_eq!(
                            response.get("var_key1").and_then(|f| f.as_str()),
                            Some(format!("var_value_{}", idx).as_str())
                        );
                        completed.fetch_add(1, Relaxed);
                    })
                    .upload_file(temp_path, format!("filename-{}", idx), None)?,
            );
        }

        batch_uploader.start();
        assert_eq!(completed.load(Relaxed), FILE_SIZES.len());
        Ok(())
    }
    #[test]
    fn test_storage_uploader_upload_files_by_batch_uploader_with_one_thread_pool() -> Result<(), Box<dyn Error>> {
        const FILE_SIZES: [u64; 12] = [
            (1 << 22) + (1 << 20) + (1 << 10) + 1,
            (1 << 20) + (1 << 20) + (1 << 10) + 1,
            (1 << 21) + (1 << 20) + (1 << 10) + 1,
            (1 << 23) + (1 << 20) + (1 << 10) + 1,
            (1 << 22) + (1 << 20) + 1,
            (1 << 20) + (1 << 20) + 1,
            (1 << 21) + (1 << 20) + 1,
            (1 << 23) + (1 << 20) + 1,
            (1 << 22) + 1,
            (1 << 20) + 1,
            (1 << 21) + 1,
            (1 << 23) + 1,
        ];
        let config = Config::default();
        let temp_paths = FILE_SIZES
            .iter()
            .map(|&size| Ok(create_temp_file(size.try_into().unwrap())?.into_temp_path()))
            .collect::<Result<Vec<_>, IOError>>()?;
        let etags = Arc::new(
            temp_paths
                .iter()
                .map(etag::from_file)
                .collect::<Result<Vec<_>, IOError>>()?,
        );
        let policy = UploadPolicyBuilder::new_policy_for_bucket(bucket_name(), &config).build();
        let mut batch_uploader = get_client(config)
            .upload()
            .batch_uploader_for_upload_policy(policy, get_credential())?;
        batch_uploader.expected_jobs_count(FILE_SIZES.len()).thread_pool_size(1);
        let thread_ids = Arc::new(RwLock::new(Vec::with_capacity(12)));
        let completed = Arc::new(AtomicUsize::new(0));
        for (idx, temp_path) in temp_paths.iter().enumerate() {
            let etags = etags.clone();
            let thread_ids_1 = thread_ids.clone();
            let thread_ids_2 = thread_ids.clone();
            let completed = completed.clone();
            batch_uploader.push_job(
                BatchUploadJobBuilder::default()
                    .key(format!("test-batch-{}-{}", idx, Utc::now().timestamp_nanos()))
                    .on_progress(move |_, _| {
                        let cur_thread_id = current().id();
                        if !thread_ids_1.read().unwrap().contains(&cur_thread_id) {
                            assert!(thread_ids_1.read().unwrap().len() <= 4);
                            thread_ids_1.write().unwrap().push(cur_thread_id);
                        }
                    })
                    .on_completed(move |result| {
                        let cur_thread_id = current().id();
                        if !thread_ids_2.read().unwrap().contains(&cur_thread_id) {
                            assert!(thread_ids_2.read().unwrap().len() <= 4);
                            thread_ids_2.write().unwrap().push(cur_thread_id);
                        }
                        let response = result.unwrap();
                        assert_eq!(response.hash(), etags.get(idx).map(|s| s.as_ref()));
                        completed.fetch_add(1, Relaxed);
                    })
                    .upload_file(temp_path, "", None)?,
            );
        }

        batch_uploader.start();
        assert_eq!(completed.load(Relaxed), FILE_SIZES.len());
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_large_files_by_batch_uploader() -> Result<(), Box<dyn Error>> {
        const FILE_SIZES: [u64; 12] = [
            (1 << 24) + (1 << 20) + (1 << 10) + 1,
            (1 << 22) + (1 << 20) + (1 << 10) + 1,
            (1 << 23) + (1 << 20) + (1 << 10) + 1,
            (1 << 25) + (1 << 20) + (1 << 10) + 1,
            (1 << 24) + (1 << 20) + 1,
            (1 << 22) + (1 << 20) + 1,
            (1 << 23) + (1 << 20) + 1,
            (1 << 25) + (1 << 20) + 1,
            (1 << 24) + 1,
            (1 << 22) + 1,
            (1 << 23) + 1,
            (1 << 25) + 1,
        ];
        let config = Config::default();
        let temp_paths = FILE_SIZES
            .iter()
            .map(|&size| Ok(create_temp_file(size.try_into().unwrap())?.into_temp_path()))
            .collect::<Result<Vec<_>, IOError>>()?;
        let etags = Arc::new(
            temp_paths
                .iter()
                .map(etag::from_file)
                .collect::<Result<Vec<_>, IOError>>()?,
        );
        let policy = UploadPolicyBuilder::new_policy_for_bucket(bucket_name(), &config).build();
        let mut batch_uploader = get_client(config)
            .upload()
            .batch_uploader_for_upload_policy(policy, get_credential())?;
        batch_uploader.expected_jobs_count(FILE_SIZES.len()).thread_pool_size(3);
        let thread_ids = Arc::new(RwLock::new(Vec::with_capacity(12)));
        let completed = Arc::new(AtomicUsize::new(0));
        for (idx, temp_path) in temp_paths.iter().enumerate() {
            let etags = etags.clone();
            let thread_ids_1 = thread_ids.clone();
            let thread_ids_2 = thread_ids.clone();
            let completed = completed.clone();
            batch_uploader.push_job(
                BatchUploadJobBuilder::default()
                    .key(format!("test-batch-{}-{}", idx, Utc::now().timestamp_nanos()))
                    .on_progress(move |_, _| {
                        let cur_thread_id = current().id();
                        if !thread_ids_1.read().unwrap().contains(&cur_thread_id) {
                            assert!(thread_ids_1.read().unwrap().len() <= 4);
                            thread_ids_1.write().unwrap().push(cur_thread_id);
                        }
                    })
                    .on_completed(move |result| {
                        let cur_thread_id = current().id();
                        if !thread_ids_2.read().unwrap().contains(&cur_thread_id) {
                            assert!(thread_ids_2.read().unwrap().len() <= 4);
                            thread_ids_2.write().unwrap().push(cur_thread_id);
                        }
                        let response = result.unwrap();
                        assert_eq!(response.hash(), etags.get(idx).map(|s| s.as_ref()));
                        completed.fetch_add(1, Relaxed);
                    })
                    .upload_file(temp_path, "", None)?,
            );
        }

        batch_uploader.start();
        assert_eq!(completed.load(Relaxed), FILE_SIZES.len());
        Ok(())
    }

    #[test]
    fn test_storage_uploader_upload_streams_by_batch_uploader() -> Result<(), Box<dyn Error>> {
        const FILE_SIZES: [u64; 12] = [
            (1 << 22) + (1 << 20) + (1 << 10) + 1,
            (1 << 20) + (1 << 20) + (1 << 10) + 1,
            (1 << 21) + (1 << 20) + (1 << 10) + 1,
            (1 << 23) + (1 << 20) + (1 << 10) + 1,
            (1 << 22) + (1 << 20) + 1,
            (1 << 20) + (1 << 20) + 1,
            (1 << 21) + (1 << 20) + 1,
            (1 << 23) + (1 << 20) + 1,
            (1 << 22) + 1,
            (1 << 20) + 1,
            (1 << 21) + 1,
            (1 << 23) + 1,
        ];
        let config = Config::default();
        let parts = FILE_SIZES
            .iter()
            .map(|&size| Ok(create_temp_file(size.try_into().unwrap())?.into_parts()))
            .collect::<Result<Vec<_>, IOError>>()?;
        let etags = Arc::new(
            parts
                .iter()
                .map(|part| etag::from_file(&part.1))
                .collect::<Result<Vec<_>, IOError>>()?,
        );
        let policy = UploadPolicyBuilder::new_policy_for_bucket(bucket_name(), &config)
            .return_body("{\"hash\":$(etag),\"key\":$(key),\"fname\":$(fname),\"var_key1\":$(x:var_key1)}")
            .build();
        let mut batch_uploader = get_client(config)
            .upload()
            .batch_uploader_for_upload_policy(policy, get_credential())?;
        batch_uploader.expected_jobs_count(FILE_SIZES.len()).thread_pool_size(5);
        let thread_ids = Arc::new(RwLock::new(Vec::with_capacity(12)));
        let completed = Arc::new(AtomicUsize::new(0));
        for (idx, (mut file, _)) in parts.into_iter().enumerate() {
            file.seek(SeekFrom::Start(0))?;
            let etags = etags.clone();
            let thread_ids_1 = thread_ids.clone();
            let thread_ids_2 = thread_ids.clone();
            let completed = completed.clone();
            batch_uploader.push_job(
                BatchUploadJobBuilder::default()
                    .key(format!("test-batch-{}-{}", idx, Utc::now().timestamp_nanos()))
                    .var("var_key1", format!("var_value_{}", idx))
                    .on_progress(move |_, _| {
                        let cur_thread_id = current().id();
                        if !thread_ids_1.read().unwrap().contains(&cur_thread_id) {
                            assert!(thread_ids_1.read().unwrap().len() <= 4);
                            thread_ids_1.write().unwrap().push(cur_thread_id);
                        }
                    })
                    .on_completed(move |result| {
                        let cur_thread_id = current().id();
                        if !thread_ids_2.read().unwrap().contains(&cur_thread_id) {
                            assert!(thread_ids_2.read().unwrap().len() <= 4);
                            thread_ids_2.write().unwrap().push(cur_thread_id);
                        }
                        let response = result.unwrap();
                        assert_eq!(response.hash(), etags.get(idx).map(|s| s.as_ref()));
                        assert_eq!(
                            response.get("fname").and_then(|f| f.as_str()),
                            Some(format!("filename-{}", idx).as_str())
                        );
                        assert_eq!(
                            response.get("var_key1").and_then(|f| f.as_str()),
                            Some(format!("var_value_{}", idx).as_str())
                        );
                        completed.fetch_add(1, Relaxed);
                    })
                    .upload_stream(file, FILE_SIZES[idx], format!("filename-{}", idx), None),
            );
        }

        batch_uploader.start();
        assert_eq!(completed.load(Relaxed), FILE_SIZES.len());
        Ok(())
    }

    fn bucket_name() -> &'static str {
        if option_env!("USE_NA_BUCKET").is_some() {
            "na-bucket"
        } else {
            "z0-bucket"
        }
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
