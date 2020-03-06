require 'stringio'
require 'tempfile'

RSpec.describe QiniuNg::Utils do
  context QiniuNg::Utils::Etag do
    context '#from_data' do
      it 'should get etag from given data' do
        expect(QiniuNg::Utils::Etag::from_data("Hello world\n")).to eq('FjOrVjm_2Oe5XrHY0Lh3gdT_6k1d')
      end
    end

    context '#from_file_path' do
      it 'should get etag from given file' do
        Tempfile.create('临时文件') do |tmpfile|
          3.times { tmpfile.puts("Hello world") }
          tmpfile.flush
          tmpfile.rewind
          expect(QiniuNg::Utils::Etag.from_file_path(tmpfile.path)).to eq 'FgAgNanfbszl6CSk8MEyKDDXvpgG'
        end
      end
    end

    context '#from_io' do
      it 'should get etag from given file' do
        Tempfile.create('临时文件') do |tmpfile|
          3.times { tmpfile.puts("Hello world") }
          tmpfile.flush
          tmpfile.rewind
          expect(QiniuNg::Utils::Etag.from_file(tmpfile)).to eq 'FgAgNanfbszl6CSk8MEyKDDXvpgG'
        end
      end

      it 'should get etag from given data' do
        buf = StringIO.new
        3.times { buf.puts("Hello world") }
        buf.rewind
        expect(QiniuNg::Utils::Etag.from_io(buf)).to eq 'FgAgNanfbszl6CSk8MEyKDDXvpgG'
      end
    end

    context 'Etag instance' do
      it 'should create etag instance' do
        etag = QiniuNg::Utils::Etag.new
        3.times { etag << "Hello world\n" }
        expect(etag.result).to eq 'FgAgNanfbszl6CSk8MEyKDDXvpgG'
      end
    end
  end
end
