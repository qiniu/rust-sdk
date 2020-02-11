require 'json'

RSpec.describe QiniuNg::Storage::Uploader::BucketUploader do
  context '#upload_file' do
    it 'should upload file by io' do
      upload_token = QiniuNg::Storage::Uploader::UploadToken.from_policy_builder(
                       QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket'),
                       access_key: ENV['access_key'],
                       secret_key: ENV['secret_key'])
      bucket_uploader = QiniuNg::Storage::Uploader.new(QiniuNg::Config.new).
                                                   bucket_uploader(bucket_name: 'z0-bucket',
                                                                   access_key: ENV['access_key'])
      file_path = '/etc/services'
      key = "测试-#{Time.now.to_i}"

      etag = File.open(file_path, 'r') do |file|
               QiniuNg::Utils::Etag.from_io(file)
             end
      response = File.open(file_path, 'r') do |file|
                  bucket_uploader.upload_file(file, upload_token: upload_token,
                                                    key: key,
                                                    vars: { 'key_1': 'value_1', 'key_2': 'value_2' },
                                                    metadata: { 'k_1': 'v_1', 'k_2': 'v_2' })
                 end
      expect(response.hash).to eq(etag)
      expect(response.key).to eq(key)
      j = JSON.load response.as_json
      expect(j['hash']).to eq(etag)
      expect(j['key']).to eq(key)
    end
  end

  context '#upload_file_path' do
    it 'should upload file by path' do
      upload_token = QiniuNg::Storage::Uploader::UploadToken.from_policy_builder(
                       QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket'),
                       access_key: ENV['access_key'],
                       secret_key: ENV['secret_key'])
      bucket_uploader = QiniuNg::Storage::Uploader.new(QiniuNg::Config.new).
                                                   bucket_uploader(bucket_name: 'z0-bucket',
                                                                   access_key: ENV['access_key'])
      file_path = '/etc/services'
      etag = File.open(file_path, 'r') do |file|
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
