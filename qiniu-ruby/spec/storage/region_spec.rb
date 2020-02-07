RSpec.describe QiniuNg::Storage::Region do
  context '#query' do
    it 'should query region by access key and region id' do
      config = QiniuNg::Config.new
      regions = QiniuNg::Storage::Region.query(access_key: ENV['access_key'], bucket_name: 'z0-bucket', config: config)
      expect(regions.size).to eq 2

      expect(regions[0].id).to be_nil
      expect(regions[0].api_urls).to be_empty
      expect(regions[0].rs_urls).to be_empty
      expect(regions[0].rsf_urls).to be_empty
      expect(regions[0].up_urls.size >= 2).to be true
      expect(regions[0].up_urls).to include('https://up.qbox.me', 'https://upload.qbox.me')
      expect(regions[0].io_urls).to contain_exactly('https://iovip.qbox.me')
      expect(regions[0].up_urls(use_https: false).size >= 2).to be true
      expect(regions[0].up_urls(use_https: false)).to include('http://up.qiniup.com', 'http://upload.qiniup.com')
      expect(regions[0].io_urls(use_https: false)).to include('http://iovip.qbox.me')

      expect(regions[1].id).to be_nil
      expect(regions[1].id).to be_nil
      expect(regions[1].api_urls).to be_empty
      expect(regions[1].rs_urls).to be_empty
      expect(regions[1].rsf_urls).to be_empty
      expect(regions[1].up_urls.size >= 2).to be true
      expect(regions[1].up_urls).to include('https://up-z1.qbox.me', 'https://upload-z1.qbox.me')
      expect(regions[1].io_urls).to contain_exactly('https://iovip-z1.qbox.me')
      expect(regions[1].up_urls(use_https: false).size >= 2).to be true
      expect(regions[1].up_urls(use_https: false)).to include('http://up-z1.qiniup.com', 'http://upload-z1.qiniup.com')
      expect(regions[1].io_urls(use_https: false)).to contain_exactly('http://iovip-z1.qbox.me')
    end
  end

  context '#by_id' do
    it 'should get region by id' do
      region = QiniuNg::Storage::Region.by_id :z0
      expect(region.id).to eq :z0
      expect(region.api_urls).to contain_exactly('https://api.qiniu.com')
      expect(region.rs_urls).to contain_exactly('https://rs.qbox.me')
      expect(region.rsf_urls).to contain_exactly('https://rsf.qbox.me')
      expect(region.up_urls.size >= 2).to be true
      expect(region.up_urls).to include('https://up.qbox.me', 'https://upload.qbox.me')
      expect(region.io_urls).to contain_exactly('https://iovip.qbox.me')

      expect(region.api_urls(use_https: false)).to contain_exactly('http://api.qiniu.com')
      expect(region.rs_urls(use_https: false)).to contain_exactly('http://rs.qiniu.com')
      expect(region.rsf_urls(use_https: false)).to contain_exactly('http://rsf.qiniu.com')
      expect(region.up_urls(use_https: false).size >= 2).to be true
      expect(region.up_urls(use_https: false)).to include('http://up.qiniup.com', 'http://upload.qiniup.com')
      expect(region.io_urls(use_https: false)).to contain_exactly('http://iovip.qbox.me')
    end
  end
end
