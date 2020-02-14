RSpec.describe QiniuNg::Config do
  context QiniuNg::Config do
    it 'should be ok to construct config and get attributes from it' do
      config = QiniuNg::Config.new(use_https: true,
                                   api_host: 'api.fake.com',
                                   rs_host: 'rs.fake.com',
                                   rsf_host: 'rsf.fake.com',
                                   batch_max_operation_size: 1000,
                                   http_connect_timeout: QiniuNg::Utils::Duration::new(seconds: 30),
                                   http_low_transfer_speed: 1024,
                                   http_low_transfer_speed_timeout: QiniuNg::Utils::Duration::new(minute: 1),
                                   http_request_retries: 5,
                                   http_request_retry_delay: QiniuNg::Utils::Duration::new(second: 1),
                                   http_request_timeout: QiniuNg::Utils::Duration::new(minutes: 5),
                                   tcp_keepalive_idle_timeout: QiniuNg::Utils::Duration::new(minutes: 5),
                                   tcp_keepalive_probe_interval: QiniuNg::Utils::Duration::new(seconds: 5),
                                   upload_block_size: 1 << 22,
                                   upload_threshold: 1 << 22,
                                   upload_token_lifetime: QiniuNg::Utils::Duration::new(hours: 2),
                                   upload_recorder_always_flush_records: true)
      expect(config.user_agent).to be_start_with('QiniuRust/qiniu-ng-')
      expect(config.user_agent).to be_include('/qiniu-ruby/')
      expect(config.user_agent).to be_include("/#{RUBY_ENGINE}/")
      expect(config.use_https?).to be true
      expect(config.api_host).to eq 'api.fake.com'
      expect(config.api_url).to eq 'https://api.fake.com'
      expect(config.rs_host).to eq 'rs.fake.com'
      expect(config.rs_url).to eq 'https://rs.fake.com'
      expect(config.rsf_host).to eq 'rsf.fake.com'
      expect(config.rsf_url).to eq 'https://rsf.fake.com'
      expect(config.batch_max_operation_size).to eq 1000
      expect(config.http_connect_timeout.to_i).to eq 30
      expect(config.http_low_transfer_speed).to eq 1024
      expect(config.http_low_transfer_speed_timeout.to_i).to eq 60
      expect(config.http_request_retries).to eq 5
      expect(config.http_request_retry_delay.to_i).to eq 1
      expect(config.http_request_timeout.to_i).to eq 300
      expect(config.tcp_keepalive_idle_timeout.to_i).to eq 300
      expect(config.tcp_keepalive_probe_interval.to_i).to eq 5
      expect(config.upload_block_size).to eq 1 << 22
      expect(config.upload_threshold.to_i).to eq 1 << 22
      expect(config.upload_token_lifetime.to_i).to eq 7200
      expect(config.upload_recorder_root_directory).to be_end_with('qiniu_sdk/records')
      expect(config.upload_recorder_always_flush_records?).to be true
      expect(config.upload_recorder_upload_block_lifetime.to_i).to eq 24 * 60 * 60 * 7

      expect(config.uplog_file_lock_policy).to eq(:lock_shared_duration_appending_and_lock_exclusive_duration_uploading)
      expect(config.uplog_file_upload_threshold).to eq 4096
      expect(config.uplog_file_max_size).to eq 1 << 22
    end

    it 'should accept hash directly for duration related config items' do
      config = QiniuNg::Config.new(use_https: true,
                                   api_host: 'api.fake.com',
                                   rs_host: 'rs.fake.com',
                                   rsf_host: 'rsf.fake.com',
                                   batch_max_operation_size: 1000,
                                   http_connect_timeout: { seconds: 30 },
                                   http_low_transfer_speed: 1024,
                                   http_low_transfer_speed_timeout: { minute: 1 },
                                   http_request_retries: 5,
                                   http_request_retry_delay: { second: 1 },
                                   http_request_timeout: { minutes: 5 },
                                   tcp_keepalive_idle_timeout: { minutes: 5 },
                                   tcp_keepalive_probe_interval: { seconds: 5 },
                                   upload_block_size: 1 << 22,
                                   upload_threshold: 1 << 22,
                                   upload_token_lifetime: { hours: 2 },
                                   upload_recorder_always_flush_records: true)
      expect(config.batch_max_operation_size).to eq 1000
      expect(config.http_connect_timeout.to_i).to eq 30
      expect(config.http_low_transfer_speed).to eq 1024
      expect(config.http_low_transfer_speed_timeout.to_i).to eq 60
      expect(config.http_request_retries).to eq 5
      expect(config.http_request_retry_delay.to_i).to eq 1
      expect(config.http_request_timeout.to_i).to eq 300
      expect(config.tcp_keepalive_idle_timeout.to_i).to eq 300
      expect(config.tcp_keepalive_probe_interval.to_i).to eq 5
      expect(config.upload_block_size).to eq 1 << 22
      expect(config.upload_threshold.to_i).to eq 1 << 22
      expect(config.upload_token_lifetime.to_i).to eq 7200
      expect(config.upload_recorder_root_directory).to be_end_with('qiniu_sdk/records')
      expect(config.upload_recorder_always_flush_records?).to be true
      expect(config.upload_recorder_upload_block_lifetime.to_i).to eq 24 * 60 * 60 * 7
    end

    it 'should not accept value which is out of range' do
      expect do
        QiniuNg::Config.new(batch_max_operation_size: -1)
      end.to raise_error(RangeError)

      expect do
        QiniuNg::Config.new(batch_max_operation_size: 1 << 32)
      end.to raise_error(RangeError)
    end
  end

  context QiniuNg::Config::Builder do
    it 'should set user_agent correctly' do
      builder = QiniuNg::Config::Builder.new
      builder.set_appended_user_agent('TEST_USER_AGENT')
      config = builder.build!
      expect(config.user_agent).to be_start_with('QiniuRust/qiniu-ng-')
      expect(config.user_agent).to be_include('/qiniu-ruby/')
      expect(config.user_agent).to be_end_with('TEST_USER_AGENT/')
      expect(config.user_agent).to be_include("/#{RUBY_ENGINE}/")
    end

    it 'should return uplog attributes even uplog is disabled' do
      builder = QiniuNg::Config::Builder.new
      builder.disable_uplog
      config = builder.build!

      expect(config.uplog_file_lock_policy).to be_nil
      expect(config.uplog_file_upload_threshold).to be_nil
      expect(config.uplog_file_max_size).to be_nil
    end

    it 'shoud be able to update uplog file lock policy' do
      builder = QiniuNg::Config::Builder.new
      builder.uplog_file_lock_policy = :none
      config = builder.build!

      expect(config.uplog_file_lock_policy).to eq(:none)
    end

    it 'should not accept invalid uplog_file_path' do
      builder = QiniuNg::Config::Builder.new
      builder.uplog_file_path = '/不存在的目录/不存在的文件'
      expect do
        builder.build!
      end.to raise_error(QiniuNg::Error::OSError)

      builder.uplog_file_path = '/不存在的文件'
      expect do
        builder.build!
      end.to raise_error(QiniuNg::Error::OSError)
    end

    it 'should not accept invalid domains manager persistent file path' do
      builder = QiniuNg::Config::Builder.new
      expect do
        builder.domains_manager_persistent_file_path = '/不存在的目录/不存在的文件'
      end.to raise_error(QiniuNg::Error::OSError)

      expect do
        builder.create_new_domains_manager('/不存在的目录/不存在的文件')
      end.to raise_error(QiniuNg::Error::OSError)

      expect do
        builder.load_domains_manager_from_file('/不存在的目录/不存在的文件')
      end.to raise_error(QiniuNg::Error::OSError)
    end

    it 'should not accept value which is out of range' do
      builder = QiniuNg::Config::Builder.new
      expect do
        builder.batch_max_operation_size = -1
      end.to raise_error(RangeError)

      expect do
        builder.batch_max_operation_size = 1 << 32
      end.to raise_error(RangeError)
    end

    it 'could accept value to be nil' do
      builder = QiniuNg::Config::Builder.new
      builder.api_host = nil
      builder.batch_max_operation_size = nil
      builder.http_connect_timeout = nil
      config = builder.build!
      expect(config.api_host).to be_empty
      expect(config.batch_max_operation_size).to be_zero
      expect(config.http_connect_timeout.to_i).to be_zero
    end
  end
end
