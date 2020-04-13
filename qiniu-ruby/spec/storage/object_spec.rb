require 'json'
require 'securerandom'
require 'tempfile'
require 'concurrent-ruby'

RSpec.describe QiniuNg::Storage::Object do
  context '#upload_file' do
    it 'could upload file directly' do
      client = QiniuNg::Client.create access_key: ENV['access_key'], secret_key: ENV['secret_key']
      bucket = client.bucket 'z0-bucket'
      Tempfile.create('测试', encoding: 'ascii-8bit') do |file|
        4.times { file.write(SecureRandom.random_bytes(rand(1 << 25))) }
        file.rewind

        key = "测试-#{Time.now.to_i}-#{rand(2**64 - 1)}"
        object = bucket.object(key)

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
        response = object.upload_file(file, on_uploading_progress: on_uploading_progress)
        GC.start
        expect(response.hash).to eq(etag)
        expect(response.key).to eq(key)
        j = JSON.load response.as_json
        expect(j['hash']).to eq(etag)
        expect(j['key']).to eq(key)
        expect(err.get).to be_nil
        expect(last_uploaded).to eq file_size

        stat = object.stat
        expect(stat.size).to eq(file.size)
        expect(stat.hash).to eq(etag)
        expect(Time.now).to be_within(30).of(stat.uploaded_at)

        object.delete!
      end
    end
  end

  context '#upload_file_path' do
    it 'could upload file directly' do
      client = QiniuNg::Client.create access_key: ENV['access_key'], secret_key: ENV['secret_key']
      bucket = client.bucket 'z0-bucket'
      Tempfile.create('测试', encoding: 'ascii-8bit') do |file|
        4.times { file.write(SecureRandom.random_bytes(rand(1 << 25))) }
        file.rewind
        etag = QiniuNg::Utils::Etag.from_io(file)
        file.rewind
        key = "测试-#{Time.now.to_i}-#{rand(2**64 - 1)}"
        object = bucket.object(key)

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

        response = object.upload_file_path(file.path, on_uploading_progress: on_uploading_progress)
        expect(response.hash).to eq(etag)
        expect(response.key).to eq(key)
        j = JSON.load response.as_json
        expect(j['hash']).to eq(etag)
        expect(j['key']).to eq(key)
        expect(err.get).to be_nil
        expect(last_uploaded).to eq file_size

        stat = object.stat
        expect(stat.size).to eq(file.size)
        expect(stat.hash).to eq(etag)
        expect(Time.now).to be_within(30).of(stat.uploaded_at)

        object.delete!
      end
    end
  end
end
