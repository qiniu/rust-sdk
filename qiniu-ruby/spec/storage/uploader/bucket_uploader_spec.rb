require 'json'
require 'securerandom'

RSpec.describe QiniuNg::Storage::Uploader::BucketUploader do
  context '#upload_file' do
    it 'should upload file by io' do
      config = QiniuNg::Config.new
      upload_token = QiniuNg::Storage::Uploader::UploadToken.from_policy_builder(
                       QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket', config),
                       access_key: ENV['access_key'],
                       secret_key: ENV['secret_key'])
      bucket_uploader = QiniuNg::Storage::Uploader.new(config).
                                                   bucket_uploader(bucket_name: 'z0-bucket',
                                                                   access_key: ENV['access_key'])
      file_path = '/etc/services'
      key = "测试-#{Time.now.to_i}"

      last_uploaded = -1
      on_uploading_progress = ->(uploaded, total) do
                                expect(total).to be_zero
                                expect(uploaded >= last_uploaded).to be true
                                last_uploaded = uploaded
                              end

      etag = File.open(file_path, 'rb') do |file|
               QiniuNg::Utils::Etag.from_io(file)
             end
      response = File.open(file_path, 'rb') do |file|
                  bucket_uploader.upload_file(file, upload_token: upload_token,
                                                    key: key,
                                                    vars: { 'key_1': 'value_1', 'key_2': 'value_2' },
                                                    metadata: { 'k_1': 'v_1', 'k_2': 'v_2' },
                                                    on_uploading_progress: on_uploading_progress)
                 end
      expect(response.hash).to eq(etag)
      expect(response.key).to eq(key)
      j = JSON.load response.as_json
      expect(j['hash']).to eq(etag)
      expect(j['key']).to eq(key)
    end

    it 'should upload customized io' do
      config = QiniuNg::Config.new
      upload_token = QiniuNg::Storage::Uploader::UploadToken.from_policy_builder(
                       QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket', config)
                                                                        .return_body(%[{"key":"$(key)","hash":"$(etag)","fsize":$(fsize),"bucket":"$(bucket)","name":"$(x:name)"}]),
                       access_key: ENV['access_key'],
                       secret_key: ENV['secret_key'])
      bucket_uploader = QiniuNg::Storage::Uploader.new(config).
                                                   bucket_uploader(bucket_name: 'z0-bucket',
                                                                   access_key: ENV['access_key'])
      key = "测试-#{Time.now.to_i}"

      last_uploaded = -1
      on_uploading_progress = ->(uploaded, total) do
                                expect(total).to be_zero
                                expect(uploaded >= last_uploaded).to be true
                                last_uploaded = uploaded
                              end
      io = StringIO.new SecureRandom.random_bytes(1 << 24)
      etag = QiniuNg::Utils::Etag.from_io(io)
      io.rewind
      response = bucket_uploader.upload_io(io, upload_token: upload_token,
                                               key: key,
                                               file_name: key,
                                               vars: { 'name': key },
                                               on_uploading_progress: on_uploading_progress)
      expect(response.hash).to eq(etag)
      expect(response.key).to eq(key)
      expect(response.fsize).to eq(1 << 24)
      expect(response.bucket).to eq('z0-bucket')
      expect(response.name).to eq(key)
      expect(response.name).to eq(key)
      j = JSON.load response.as_json
      expect(j['hash']).to eq(etag)
      expect(j['key']).to eq(key)
      expect(j['fsize']).to eq(1 << 24)
      expect(j['bucket']).to eq('z0-bucket')
      expect(j['name']).to eq(key)
    end
  end

  context '#upload_file_path' do
    it 'should upload file by path' do
      config = QiniuNg::Config.new
      upload_token = QiniuNg::Storage::Uploader::UploadToken.from_policy_builder(
                       QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket', config),
                       access_key: ENV['access_key'],
                       secret_key: ENV['secret_key'])
      bucket_uploader = QiniuNg::Storage::Uploader.new(config).
                                                   bucket_uploader(bucket_name: 'z0-bucket',
                                                                   access_key: ENV['access_key'])
      file_path = '/etc/services'
      etag = File.open(file_path, 'rb') do |file|
        QiniuNg::Utils::Etag.from_io(file)
      end
      key = "测试-#{Time.now.to_i}"
      last_uploaded, file_size = -1, File.size(file_path)
      on_uploading_progress = ->(uploaded, total) do
                                expect(total >= file_size).to be true
                                expect(uploaded >= last_uploaded).to be true
                                last_uploaded = uploaded
                              end

      response = bucket_uploader.upload_file_path('/etc/services', upload_token: upload_token,
                                                                   key: key,
                                                                   on_uploading_progress: on_uploading_progress)
      expect(response.hash).to eq(etag)
      expect(response.key).to eq(key)
      j = JSON.load response.as_json
      expect(j['hash']).to eq(etag)
      expect(j['key']).to eq(key)
    end
  end
end
