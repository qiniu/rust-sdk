require 'json'
require 'securerandom'
require 'tempfile'
require 'concurrent-ruby'

RSpec.describe QiniuNg::Storage::Uploader::BatchUploader do
  context '#upload_file' do
    it 'should upload files by io' do
      config = QiniuNg::Config.new
      upload_token = QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket', config).
                                                                       build_token(access_key: ENV['access_key'],
                                                                                   secret_key: ENV['secret_key'])
      batch_uploader = QiniuNg::Storage::Uploader.new(config).
                                                  batch_uploader(upload_token, config: config)
      batch_uploader.thread_pool_size = 8
      completed = Concurrent::AtomicFixnum.new
      err = Concurrent::AtomicReference.new
      8.times do |idx|
        tempfile = Tempfile.create('测试', encoding: 'ascii-8bit')
        tempfile.write(SecureRandom.random_bytes(rand(1 << 24)))
        tempfile.rewind
        etag = QiniuNg::Utils::Etag.from_io(tempfile)
        tempfile.rewind
        key = "测试-#{idx}-#{Time.now.to_i}"
        batch_uploader.upload_file(tempfile, key: key) do |response, err|
          begin
            expect(err).to be_nil
            expect(response).not_to be_nil
            expect(response.hash).to eq etag
            expect(response.key).to eq key
            completed.increment
          rescue Exception => e
            err.set(e)
          end
        end
      end

      GC.start
      batch_uploader.start
      GC.start

      expect(completed.value).to eq 8
      expect(err.get).to be_nil
    end

    it 'should upload files by path' do
      config = QiniuNg::Config.new
      upload_token = QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket', config).
                                                                       build_token(access_key: ENV['access_key'],
                                                                                   secret_key: ENV['secret_key'])
      batch_uploader = QiniuNg::Storage::Uploader.new(config).
                                                  batch_uploader(upload_token, config: config)
      batch_uploader.thread_pool_size = 8
      completed = Concurrent::AtomicFixnum.new
      err = Concurrent::AtomicReference.new
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
          begin
            expect(response).not_to be_nil
            expect(response.hash).to eq etag
            expect(response.key).to eq key
            completed.increment
          rescue Exception => e
            err.set(e)
          end
        end
      end
      GC.start
      batch_uploader.start
      GC.start
      expect(completed.value).to eq 8
      expect(err.get).to be_nil
    end
  end
end
