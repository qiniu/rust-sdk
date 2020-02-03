require 'stringio'

RSpec.describe QiniuNg::Etag do
  context '#from_buffer' do
    it 'should get etag from given buffer' do
      expect(QiniuNg::Etag::from_buffer("Hello world\n")).to eq('FjOrVjm_2Oe5XrHY0Lh3gdT_6k1d')
    end
  end

  # TODO: context '#from_file_path'

  context '#from_io' do
    it 'should get etag from given file' do
      Tempfile.create('foo') do |tmpfile|
        3.times { tmpfile.puts("Hello world") }
        tmpfile.flush
        tmpfile.rewind
        expect(QiniuNg::Etag.from_file(tmpfile)).to eq 'FgAgNanfbszl6CSk8MEyKDDXvpgG'
      end
    end

    it 'should get etag from given buffer' do
      buf = StringIO.new
      3.times { buf.puts("Hello world") }
      buf.rewind
      expect(QiniuNg::Etag.from_io(buf)).to eq 'FgAgNanfbszl6CSk8MEyKDDXvpgG'
    end
  end

  context 'Etag instance' do
    it 'should create etag instance' do
      etag = QiniuNg::Etag.new
      3.times { etag << "Hello world\n" }
      expect(etag.result).to eq 'FgAgNanfbszl6CSk8MEyKDDXvpgG'
    end
  end
end
