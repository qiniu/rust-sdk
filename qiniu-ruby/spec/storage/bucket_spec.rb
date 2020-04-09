require 'json'
require 'securerandom'
require 'tempfile'
require 'concurrent-ruby'

RSpec.describe QiniuNg::Storage::Bucket do
  context '#new' do
    it 'could query regions from bucket' do
      client = QiniuNg::Client.new access_key: ENV['access_key'], secret_key: ENV['secret_key']
      bucket = QiniuNg::Storage::Bucket.new client: client, bucket_name: 'z0-bucket'
      expect(bucket.regions.size).to eq 2
      expect(bucket.regions[0].io_urls).to contain_exactly('https://iovip.qbox.me')
    end

    it 'could set regions for bucket' do
      client = QiniuNg::Client.new access_key: ENV['access_key'], secret_key: ENV['secret_key']
      bucket = QiniuNg::Storage::Bucket.new client: client, bucket_name: 'z0-bucket', region: QiniuNg::Storage::Region.by_id(:as0)
      expect(bucket.regions.size).to eq 1
      expect(bucket.regions[0].id).to eq :as0
    end

    it 'could query domains from bucket' do
      client = QiniuNg::Client.new access_key: ENV['access_key'], secret_key: ENV['secret_key']
      bucket = QiniuNg::Storage::Bucket.new client: client, bucket_name: 'z0-bucket'
      expect(bucket.domains.size).to eq 2
    end

    it 'could set domains for bucket' do
      client = QiniuNg::Client.new access_key: ENV['access_key'], secret_key: ENV['secret_key']
      bucket = QiniuNg::Storage::Bucket.new client: client, bucket_name: 'z0-bucket', domains: (0...2).map { |i| "https://domain#{i}.com" }
      expect(bucket.domains).to contain_exactly(*(0...2).map { |i| "https://domain#{i}.com" })
    end
  end

  context '#upload_file' do
    it 'could upload file directly' do
      client = QiniuNg::Client.new access_key: ENV['access_key'], secret_key: ENV['secret_key']
      bucket = QiniuNg::Storage::Bucket.new client: client, bucket_name: 'z0-bucket'
      Tempfile.create('测试', encoding: 'ascii-8bit') do |file|
        4.times { file.write(SecureRandom.random_bytes(rand(1 << 25))) }
        file.rewind

        key = "测试-#{Time.now.to_i}-#{rand(2**64 - 1)}"

        err = Concurrent::AtomicReference.new
        last_uploaded, mutex, file_size = -1, Mutex.new, file.size
        on_uploading_progress = ->(uploaded, total) do
                                  begin
                                    expect(total).to eq file_size
                                    expect(uploaded <= total).to be true
                                    mutex.synchronize do
                                      last_uploaded = [last_uploaded, uploaded].max
                                    end
                                  rescue Exception => e
                                    err.set(e)
                                  end
                                end

        etag = QiniuNg::Utils::Etag.from_io(file)
        file.rewind

        GC.start
        response = bucket.upload_file(file, key: key,
                                            on_uploading_progress: on_uploading_progress)
        GC.start
        expect(response.hash).to eq(etag)
        expect(response.key).to eq(key)
        j = JSON.load response.as_json
        expect(j['hash']).to eq(etag)
        expect(j['key']).to eq(key)
        expect(err.get).to be_nil
        expect(last_uploaded).to eq file_size
      end
    end
  end

  context '#upload_file_path' do
    it 'could upload file directly' do
      client = QiniuNg::Client.new access_key: ENV['access_key'], secret_key: ENV['secret_key']
      bucket = QiniuNg::Storage::Bucket.new client: client, bucket_name: 'z0-bucket'
      Tempfile.create('测试', encoding: 'ascii-8bit') do |file|
        4.times { file.write(SecureRandom.random_bytes(rand(1 << 25))) }
        file.rewind
        etag = QiniuNg::Utils::Etag.from_io(file)
        file.rewind
        key = "测试-#{Time.now.to_i}-#{rand(2**64 - 1)}"
        err = Concurrent::AtomicReference.new
        last_uploaded, mutex, file_size = -1, Mutex.new, file.size
        on_uploading_progress = ->(uploaded, total) do
                                  begin
                                    expect(total).to eq(file_size)
                                    expect(uploaded <= total).to be true
                                    mutex.synchronize do
                                      last_uploaded = [last_uploaded, uploaded].max
                                    end
                                  rescue Exception => e
                                    err.set(e)
                                  end
                                end

        response = bucket.upload_file_path(file.path, key: key,
                                                      on_uploading_progress: on_uploading_progress)
        expect(response.hash).to eq(etag)
        expect(response.key).to eq(key)
        j = JSON.load response.as_json
        expect(j['hash']).to eq(etag)
        expect(j['key']).to eq(key)
        expect(err.get).to be_nil
        expect(last_uploaded).to eq file_size
      end
    end
  end
end
