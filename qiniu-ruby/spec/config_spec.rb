RSpec.describe QiniuNg::Bindings::Config do
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

      config = QiniuNg.wrap_ffi_function do
                 QiniuNg::Bindings::Config.build(config_builder)
               end
      expect(config.get_use_https).to be false
      expect(config.get_uc_url&.get_ptr).to eq "http://uc.fake.com"
      expect(config.get_rs_url&.get_ptr).to eq "http://rs.qbox.me"
      expect(config.get_uplog_file_path&.get_ptr).to be_nil
    end
  end
end
