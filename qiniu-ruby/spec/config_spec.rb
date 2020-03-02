require 'uri'
require 'stringio'
require 'securerandom'
require 'webrick/httputils'
require 'json'

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
      config = QiniuNg::Config::Builder.new.set_appended_user_agent('TEST_USER_AGENT').build!
      expect(config.user_agent).to be_start_with('QiniuRust/qiniu-ng-')
      expect(config.user_agent).to be_include('/qiniu-ruby/')
      expect(config.user_agent).to be_end_with('TEST_USER_AGENT/')
      expect(config.user_agent).to be_include("/#{RUBY_ENGINE}/")
    end

    it 'should return uplog attributes even uplog is disabled' do
      config = QiniuNg::Config::Builder.new.disable_uplog.build!

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

    context 'Handler' do
      it 'could modify request before http call' do
        builder = QiniuNg::Config::Builder.new
        builder.append_http_request_before_action_handler do |request|
          url = URI.parse(request.url)
          expect(url.scheme).to eq 'https'
          expect(url.host).to eq 'uc.qbox.me'
          expect(url.path).to eq '/v3/query'
          query = WEBrick::HTTPUtils.parse_query(url.query)
          expect(query['bucket']).to eq 'z0-bucket'
          expect(query['ak']).to eq ENV['access_key']
          url.query = "bucket=z1-bucket&ak=#{ENV['access_key']}"
          request.url = url

          expect(request.method).to eq :GET
          headers = request.headers
          expect(headers['Accept']).to eq 'application/json'
          expect(headers['Content-Type']).to eq 'application/x-www-form-urlencoded'
          expect(request.body).to be_nil
          expect(request).not_to be_follow_redirection
          expect(request.resolved_socket_addrs.is_a?(Array)).to be true
          expect(request.resolved_socket_addrs.size >= 0).to be true
        end
        config = builder.build!
        regions = QiniuNg::Storage::Region.query(access_key: ENV['access_key'], bucket_name: 'z0-bucket', config: config)
        expect(regions.size).to eq 1
        expect(regions[0].io_urls).to eq %w[https://iovip-z1.qbox.me]
      end

      it 'could modify request by io error before http call' do
        builder = QiniuNg::Config::Builder.new
        builder.append_http_request_before_action_handler do |request|
          raise QiniuNg::Error::IOHandlerError.new('test error')
        end
        config = builder.build!
        expect do
          QiniuNg::Storage::Region.query(access_key: ENV['access_key'], bucket_name: 'z0-bucket', config: config)
        end.to raise_error(QiniuNg::Error::IOError, 'test error')
      end

      it 'could modify request by os error before http call' do
        builder = QiniuNg::Config::Builder.new
        builder.append_http_request_before_action_handler do |request|
          raise QiniuNg::Error::OSHandlerError.new(Errno::EPERM::Errno)
        end
        config = builder.build!
        begin
          QiniuNg::Storage::Region.query(access_key: ENV['access_key'], bucket_name: 'z0-bucket', config: config)
          fail 'expect to raise error here'
        rescue QiniuNg::Error::OSError => e
          expect(e.errno).to eq Errno::EPERM::Errno
        end
      end

      it 'could modify request by status code error before http call' do
        builder = QiniuNg::Config::Builder.new
        builder.append_http_request_before_action_handler do |request|
          raise QiniuNg::Error::ResponseStatusCodeHandlerError.new(503, 'Gateway Timeout')
        end
        config = builder.build!
        begin
          QiniuNg::Storage::Region.query(access_key: ENV['access_key'], bucket_name: 'z0-bucket', config: config)
          fail 'expect to raise error here'
        rescue QiniuNg::Error::ResponseStatusCodeError => e
          expect(e.code).to eq 503
          expect(e.message).to eq 'Gateway Timeout'
        end
      end

      it 'could modify headers before http call' do
        builder = QiniuNg::Config::Builder.new
        builder.append_http_request_before_action_handler do |request|
          unless request.url.start_with?('https://uc.qbox.me/')
            expect(request.headers['Authorization']).to be_start_with('UpToken ')
            request.headers['Authorization'] = nil
          end
        end
        config = builder.build!

        upload_token = QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket', config).
                                                                         build!.
                                                                         build_token(access_key: ENV['access_key'],
                                                                                     secret_key: ENV['secret_key'])
        io = StringIO.new(SecureRandom.random_bytes(1 << 23))
        expect do
          QiniuNg::Storage::Uploader.new(config).
                                     bucket_uploader(bucket_name: 'z0-bucket',
                                                     access_key: ENV['access_key']).
                                     upload_io(io, upload_token: upload_token, key: "测试-#{Time.now.to_i}")
        end.to raise_error(an_instance_of(QiniuNg::Error::ResponseStatusCodeError).and(having_attributes(code: 401)))
      end

      it 'could modify body before http call' do
        builder = QiniuNg::Config::Builder.new
        builder.append_http_request_before_action_handler do |request|
          unless request.url.start_with?('https://uc.qbox.me/')
            request.body = 'hello world'
          end
        end
        config = builder.build!

        upload_token = QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket', config).
                                                                         build!.
                                                                         build_token(access_key: ENV['access_key'],
                                                                                     secret_key: ENV['secret_key'])
        Tempfile.create('测试', encoding: 'ascii-8bit') do |file|
          file.write(SecureRandom.random_bytes(1 << 21))
          expect do
            QiniuNg::Storage::Uploader.new(config).
                                       bucket_uploader(bucket_name: 'z0-bucket',
                                                       access_key: ENV['access_key']).
                                       upload_file_path(file.path, upload_token: upload_token, key: "测试-#{Time.now.to_i}")
          end.to raise_error(an_instance_of(QiniuNg::Error::ResponseStatusCodeError).and(having_attributes(code: 400)))
        end
      end

      it 'could modify response after http call' do
        builder = QiniuNg::Config::Builder.new
        builder.append_http_request_after_action_handler do |request, response|
          expect(response.status_code).to eq 200
          expect(response.server_port).to eq 443
          expect(response.headers['Content-Length'].to_i > 2).to be true
          expect(JSON.parse(response.body.read)['hosts'].is_a?(Array)).to be true

          response.headers['Content-Length'] = 2
          response.body = '{}'
        end
        config = builder.build!
        expect do
          QiniuNg::Storage::Region.query(access_key: ENV['access_key'], bucket_name: 'z0-bucket', config: config)
        end.to raise_error(QiniuNg::Error::JSONError, /missing field `hosts`/)
      end

      it 'could make response for http call handler' do
        builder = QiniuNg::Config::Builder.new
        builder.http_request_handler do |request, response|
          # do nothing
        end
        config = builder.build!
        client = QiniuNg::Client.new access_key: ENV['access_key'],
                                     secret_key: ENV['secret_key'],
                                     config: config
        client.create_bucket('test-bucket', :z1)

        client = QiniuNg::Client.new access_key: ENV['access_key'],
                                     secret_key: ENV['secret_key']
        expect(client.bucket_names).not_to include('test-bucket')
      end

      it 'could make response error for http call handler' do
        builder = QiniuNg::Config::Builder.new
        builder.http_request_handler do |request, response|
          raise QiniuNg::Error::UserCancelledHandlerError
        end
        config = builder.build!
        client = QiniuNg::Client.new access_key: ENV['access_key'],
                                     secret_key: ENV['secret_key'],
                                     config: config
        expect do
          client.create_bucket('test-bucket', :z1)
        end.to raise_error(QiniuNg::Error::UserCancelledError)
      end
    end
  end
end
