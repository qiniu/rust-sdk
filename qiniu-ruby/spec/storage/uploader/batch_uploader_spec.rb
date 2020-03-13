require 'json'
require 'securerandom'
require 'tempfile'
require 'concurrent-ruby'

RSpec.describe QiniuNg::Storage::Uploader::BatchUploader do
  context '#upload_file' do
    it 'should upload files by io' do
      config = QiniuNg::Config.new
      upload_token = QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket', config).
                                                                       build!.
                                                                       build_token(access_key: ENV['access_key'],
                                                                                   secret_key: ENV['secret_key'])
      batch_uploader = QiniuNg::Storage::Uploader.new(config).
                                                  bucket_uploader(bucket_name: 'z0-bucket',
                                                                  access_key: ENV['access_key']).
                                                  batch(upload_token: upload_token)
      batch_uploader.thread_pool_size = 8
      completed = Concurrent::AtomicFixnum.new
      8.times do |idx|
        tempfile = Tempfile.create('测试', encoding: 'ascii-8bit')
        tempfile.write(SecureRandom.random_bytes(rand(1 << 24)))
        tempfile.rewind
        etag = QiniuNg::Utils::Etag.from_io(tempfile)
        tempfile.rewind
        key = "测试-#{idx}-#{Time.now.to_i}"
        batch_uploader.upload_file(tempfile, key: key) do |response, err|
          expect(response).not_to be_nil
          expect(response.hash).to eq etag
          expect(response.key).to eq key
          completed.increment
        end
      end
      batch_uploader.start
      expect(completed.value).to eq 8
    end

    it 'should upload files by path' do
      config = QiniuNg::Config.new
      upload_token = QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket', config).
                                                                       build!.
                                                                       build_token(access_key: ENV['access_key'],
                                                                                   secret_key: ENV['secret_key'])
      batch_uploader = QiniuNg::Storage::Uploader.new(config).
                                                  bucket_uploader(bucket_name: 'z0-bucket',
                                                                  access_key: ENV['access_key']).
                                                  batch(upload_token: upload_token)
      batch_uploader.thread_pool_size = 8
      completed = Concurrent::AtomicFixnum.new
      8.times do |idx|
        tempfile = Tempfile.create('测试', encoding: 'ascii-8bit')
        tempfile.write(SecureRandom.random_bytes(rand(1 << 24)))
        tempfile.rewind
        etag = QiniuNg::Utils::Etag.from_io(tempfile)
        tempfile.rewind
        key = "测试-#{idx}-#{Time.now.to_i}"
        last_uploaded = -1
        on_uploading_progress = ->(uploaded, total) do
                                  expect(total).to be_zero
                                  expect(uploaded >= last_uploaded).to be true
                                  last_uploaded = uploaded
                                end
        batch_uploader.upload_file_path(tempfile.path, key: key, on_uploading_progress: on_uploading_progress) do |response, err|
          expect(response).not_to be_nil
          expect(response.hash).to eq etag
          expect(response.key).to eq key
          completed.increment
        end
      end
      batch_uploader.start
      expect(completed.value).to eq 8
    end
  end
end
