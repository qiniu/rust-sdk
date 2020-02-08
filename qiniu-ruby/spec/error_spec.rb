require 'tempfile'

RSpec.describe QiniuNg::Error do
  context QiniuNg::Error::OSError do
    it 'should get os error' do
      begin
        QiniuNg::Utils::Etag.from_file_path('/不存在的文件')
        fail 'Should get exception'
      rescue QiniuNg::Error::OSError => e
        expect(e.message).to eq('No such file or directory')
        expect(e.errno).to eq 2
      else
        fail 'Should get OSError'
      end
    end
  end

  context QiniuNg::Error::JSONError do
    it 'should get JSON error' do
      begin
        QiniuNg::Storage::Uploader::UploadPolicy.from_json 'invalid_json'
        fail 'Should get exception'
      rescue QiniuNg::Error::JSONError => e
        expect(e.message).to eq('expected value at line 1 column 1')
      else
        fail 'Should get JSONError'
      end
    end
  end
end
