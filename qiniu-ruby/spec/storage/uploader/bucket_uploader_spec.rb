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
      key = "测试-#{Time.now.to_i}"
      etag = File.open('/etc/services', 'r') do |file|
        QiniuNg::Utils::Etag.from_io(file)
        file.rewind
        response = bucket_uploader.upload_file(file, upload_token: upload_token,
                                                     key: key)
        p JSON.load response.as_json
      end
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
      etag = File.open('/etc/services', 'r') do |file|
        QiniuNg::Utils::Etag.from_io(file)
      end
      key = "测试-#{Time.now.to_i}"
      response = bucket_uploader.upload_file_path('/etc/services', upload_token: upload_token,
                                                                   key: key)
      expect(response.hash).to eq(etag)
      expect(response.key).to eq(key)
      j = JSON.load response.as_json
      expect(j['hash']).to eq(etag)
      expect(j['key']).to eq(key)
    end
  end
end
