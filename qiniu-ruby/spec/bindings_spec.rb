require 'tempfile'

RSpec.describe QiniuNg::Bindings do
  context QiniuNg::Bindings::Str do
    it 'should be ok to initialize string' do
      str1 = QiniuNg::Bindings::Str.new! '你好'
      str2 = QiniuNg::Bindings::Str.new! '七牛'
      expect(str1.get_ptr).to eq('你好')
      expect(str2.get_ptr).to eq('七牛')
      expect(str1.get_len).to eq('你好'.bytesize)
      expect(str2.get_len).to eq('七牛'.bytesize)
      expect(str1.is_freed).to be false
      expect(str2.is_freed).to be false
      expect(str1.is_null).to be false
      expect(str2.is_null).to be false
    end
  end

  context QiniuNg::Bindings::StrList do
    it 'should be ok to initialize string list' do
      list1 = QiniuNg::Bindings::StrList.new!(['七牛', '你好', '武汉', '加油'])
      list2 = QiniuNg::Bindings::StrList.new!(['科多兽', '多啦A梦', '潘多拉'])
      expect(list1.len).to eq(4)
      expect(list2.len).to eq(3)
      expect(list1.get(0)).to eq('七牛')
      expect(list1.get(1)).to eq('你好')
      expect(list1.get(2)).to eq('武汉')
      expect(list1.get(3)).to eq('加油')
      expect(list2.get(0)).to eq('科多兽')
      expect(list2.get(1)).to eq('多啦A梦')
      expect(list2.get(2)).to eq('潘多拉')
      expect(list1.is_freed).to be false
      expect(list2.is_freed).to be false
    end
  end

  context QiniuNg::Bindings::StrMap do
    it 'should be ok to initialize string map' do
      map1 = QiniuNg::Bindings::StrMap.new! 5
      map1.set('KODO', '科多兽')
      map1.set('多啦A梦', 'DORA')
      map1.set('PANDORA', '潘多拉')

      map2 = QiniuNg::Bindings::StrMap.new! 10
      map2.set('科多兽', 'KODO')
      map2.set('DORA', '多啦A梦')
      map2.set('潘多拉', 'PANDORA')

      expect(map1.len).to eq(3)
      expect(map1.get('KODO')).to eq('科多兽')
      expect(map1.get('多啦A梦')).to eq('DORA')
      expect(map1.get('PANDORA')).to eq('潘多拉')

      expect(map2.len).to eq(3)
      expect(map2.get('科多兽')).to eq('KODO')
      expect(map2.get('DORA')).to eq('多啦A梦')
      expect(map2.get('潘多拉')).to eq('PANDORA')

      looped = 0
      map1.each_entry(->(key, value, _) do
        case key.force_encoding(Encoding::UTF_8)
        when 'KODO' then
          expect(value.force_encoding(Encoding::UTF_8)).to eq('科多兽')
        when '多啦A梦' then
          expect(value.force_encoding(Encoding::UTF_8)).to eq('DORA')
        when 'PANDORA' then
          expect(value.force_encoding(Encoding::UTF_8)).to eq('潘多拉')
        else
          fail "Unrecognized key: #{key}"
        end
        looped += 1
        true
      end, FFI::MemoryPointer.new(:pointer))
      expect(looped).to eq 3
    end
  end

  context QiniuNg::Bindings::Etag do
    ETAG_SIZE = 28

    it 'should get etag from buffer' do
      FFI::MemoryPointer::new(ETAG_SIZE) do |etag_result|
        QiniuNg::Bindings::Etag.from_data("Hello world\n", etag_result)
        expect(etag_result.read_bytes(ETAG_SIZE)).to eq('FjOrVjm_2Oe5XrHY0Lh3gdT_6k1d')
      end
    end

    it 'should get etag from file' do
      FFI::MemoryPointer::new(ETAG_SIZE) do |etag_result|
        Tempfile.create('foo') do |tmpfile|
          tmpfile.puts "Hello world\n"
          tmpfile.flush
          QiniuNg::Error.wrap_ffi_function do
            QiniuNg::Bindings::Etag.from_file_path(tmpfile.path, etag_result)
          end
        end
        expect(etag_result.read_bytes(ETAG_SIZE)).to eq('FjOrVjm_2Oe5XrHY0Lh3gdT_6k1d')
      end
    end

    it 'should get etag from Etag instance' do
      FFI::MemoryPointer::new(ETAG_SIZE) do |etag_result|
        etag = QiniuNg::Bindings::Etag.new!
        3.times { etag.update("Hello world\n") }
        etag.result(etag_result)
        expect(etag_result.read_bytes(ETAG_SIZE)).to eq('FgAgNanfbszl6CSk8MEyKDDXvpgG')
        4.times { etag.update("Hello world\n") }
        etag.result(etag_result)
        expect(etag_result.read_bytes(ETAG_SIZE)).to eq('FhV9_jRUUi8lQ9eL_AbKIZj5pWXx')
      end
    end
  end

  context QiniuNg::Bindings::Config do
    context '#new_default' do
      it 'should be ok to build default config' do
        config = QiniuNg::Bindings::Config.new_default
        expect(config.get_use_https).to be true
        expect(config.get_uc_url&.get_ptr).to eq "https://uc.qbox.me"
        expect(config.get_rs_url&.get_ptr).to eq "https://rs.qbox.me"
        expect(config.get_uplog_file_path&.get_ptr).to be_end_with('qiniu_sdk/upload.log')
      end
    end

    context '#build' do
      it 'should be ok to build config' do
        config_builder = QiniuNg::Bindings::ConfigBuilder.new!
        config_builder.use_https false
        config_builder.uc_host('uc.fake.com')
        config_builder.disable_uplog

        config = QiniuNg::Error.wrap_ffi_function do
                   QiniuNg::Bindings::Config.build(config_builder)
                 end
        expect(config.get_use_https).to be false
        expect(config.get_uc_url&.get_ptr).to eq "http://uc.fake.com"
        expect(config.get_rs_url&.get_ptr).to eq "http://rs.qbox.me"
        expect(config.get_uplog_file_path&.get_ptr).to be_nil
      end
    end
  end

  context QiniuNg::Bindings::Region do
    context '#query' do
      it 'should get region by id' do
        region = QiniuNg::Bindings::Region.get_by_id(:qiniu_ng_region_z0)
        expect(region.is_freed).to be false
        up_urls = region.get_up_urls(true)
        expect(up_urls.len > 2).to be true
        io_urls = region.get_io_urls(true)
        expect(io_urls.len).to eq 1
        expect(region.get_rs_urls(true).len).to eq 1
        expect(region.get_rsf_urls(true).len).to eq 1
        expect(region.get_api_urls(true).len).to eq 1
      end

      it 'should not accept invalid region id' do
        expect do
          QiniuNg::Bindings::Region.get_by_id(:qiniu_ng_region_z3)
        end.to raise_error(ArgumentError)
      end

      it 'should query regions by access_key and bucket name' do
        regions = QiniuNg::Error.wrap_ffi_function do
                    QiniuNg::Bindings::Region.query('z0-bucket', ENV['access_key'], QiniuNg::Bindings::Config.new_default)
                  end
        expect(regions.len).to eq 2
        region = regions.get(0)
        expect(region.is_freed).to be false
        up_urls = region.get_up_urls(true)
        expect(up_urls.len > 2).to be true
        io_urls = region.get_io_urls(true)
        expect(io_urls.len).to eq 1
        expect(region.get_rs_urls(true).len).to eq 0
        expect(region.get_rsf_urls(true).len).to eq 0
        expect(region.get_api_urls(true).len).to eq 0

        region = regions.get(1)
        expect(region.is_freed).to be false
        up_urls = region.get_up_urls(true)
        expect(up_urls.len > 2).to be true
        io_urls = region.get_io_urls(true)
        expect(io_urls.len).to eq 1
        expect(region.get_rs_urls(true).len).to eq 0
        expect(region.get_rsf_urls(true).len).to eq 0
        expect(region.get_api_urls(true).len).to eq 0

        region = regions.get(2)
        expect(region.is_freed).to be true
      end
    end
  end
end
