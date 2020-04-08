require 'json'
require 'securerandom'
require 'tempfile'
require 'concurrent-ruby'

RSpec.describe QiniuNg::Storage::Uploader do
  context '#upload_file' do
    it 'should upload file by io' do
      uploader = QiniuNg::Storage::Uploader.new
      credential = QiniuNg::Credential.new(ENV['access_key'], ENV['secret_key'])
      Tempfile.create('测试', encoding: 'ascii-8bit') do |file|
        4.times { file.write(SecureRandom.random_bytes(rand(1 << 25))) }
        file.rewind

        key = "测试-#{Time.now.to_i}-#{rand(2**64 - 1)}"

        err = Concurrent::AtomicReference.new
        last_uploaded, file_size = Concurrent::AtomicFixnum.new(-1), file.size
        on_uploading_progress = ->(uploaded, total) do
                                  begin
                                    expect(total).to eq file_size
                                    last_uploaded.value = uploaded
                                  rescue Exception => e
                                    err.set(e)
                                  end
                                end

        etag = QiniuNg::Utils::Etag.from_io(file)
        file.rewind

        GC.start
        response = uploader.upload_file(file, credential: credential,
                                              bucket_name: upload_bucket_name,
                                              key: key,
                                              on_uploading_progress: on_uploading_progress)
        GC.start
        expect(response.hash).to eq(etag)
        expect(response.key).to eq(key)
        j = JSON.load response.as_json
        expect(j['hash']).to eq(etag)
        expect(j['key']).to eq(key)
        expect(err.get).to be_nil
        expect(last_uploaded.value).to eq file_size
      end
    end

    it 'should upload customized io' do
      upload_token = QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket(upload_bucket_name, QiniuNg::Config.new)
                                                                      .return_body(%[{"key":"$(key)","hash":"$(etag)","fsize":$(fsize),"bucket":"$(bucket)","name":"$(x:name)"}])
                                                                      .build_token(access_key: ENV['access_key'], secret_key: ENV['secret_key'])
      uploader = QiniuNg::Storage::Uploader.new
      key = "测试-#{Time.now.to_i}-#{rand(2**64 - 1)}"

      io = StringIO.new SecureRandom.random_bytes(1 << 24)
      etag = QiniuNg::Utils::Etag.from_io(io)
      io.rewind

      err = Concurrent::AtomicReference.new
      last_uploaded, io_size = Concurrent::AtomicFixnum.new(-1), io.size
      on_uploading_progress = ->(uploaded, total) do
                                begin
                                  expect(total).to eq io_size
                                  last_uploaded.value = uploaded
                                rescue Exception => e
                                  err.set(e)
                                end
                              end
      GC.start
      response = uploader.upload_io(io, upload_token: upload_token,
                                        key: key,
                                        file_name: key,
                                        vars: { 'name': key },
                                        on_uploading_progress: on_uploading_progress)
      GC.start
      expect(response.hash).to eq(etag)
      expect(response.key).to eq(key)
      expect(response.fsize).to eq(1 << 24)
      expect(response.bucket).to eq(upload_bucket_name)
      expect(response.name).to eq(key)
      j = JSON.load response.as_json
      expect(j['hash']).to eq(etag)
      expect(j['key']).to eq(key)
      expect(j['fsize']).to eq(1 << 24)
      expect(j['bucket']).to eq(upload_bucket_name)
      expect(j['name']).to eq(key)
      expect(err.get).to be_nil
      expect(last_uploaded.value).to eq io_size
    end
  end

  context '#upload_file_path' do
    it 'should upload file by path' do
      uploader = QiniuNg::Storage::Uploader.new
      credential = QiniuNg::Credential.new(ENV['access_key'], ENV['secret_key'])
      Tempfile.create('测试', encoding: 'ascii-8bit') do |file|
        4.times { file.write(SecureRandom.random_bytes(rand(1 << 25))) }
        file.rewind
        etag = QiniuNg::Utils::Etag.from_io(file)
        key = "测试-#{Time.now.to_i}-#{rand(2**64 - 1)}"
        err = Concurrent::AtomicReference.new
        last_uploaded, file_size = Concurrent::AtomicFixnum.new(-1), file.size
        on_uploading_progress = ->(uploaded, total) do
                                  begin
                                    expect(total >= file_size).to be true
                                    last_uploaded.value = uploaded
                                  rescue Exception => e
                                    err.set(e)
                                  end
                                end

        response = uploader.upload_file_path(file.path, bucket_name: upload_bucket_name,
                                                        credential: credential,
                                                        key: key,
                                                        on_uploading_progress: on_uploading_progress)
        expect(response.hash).to eq(etag)
        expect(response.key).to eq(key)
        j = JSON.load response.as_json
        expect(j['hash']).to eq(etag)
        expect(j['key']).to eq(key)
        expect(err.get).to be_nil
        expect(last_uploaded.value).to eq file_size
      end
    end
  end
end
