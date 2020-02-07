RSpec.describe QiniuNg::Storage::Bucket do
  context '#new' do
    it 'could query regions from bucket' do
      client = QiniuNg::Client.new access_key: ENV['access_key'], secret_key: ENV['secret_key'], config: QiniuNg::Config.new
      bucket = QiniuNg::Storage::Bucket.new client: client, bucket_name: 'z0-bucket'
      expect(bucket.regions.size).to eq 2
      expect(bucket.regions[0].io_urls).to contain_exactly('https://iovip.qbox.me')
    end

    it 'could set regions for bucket' do
      client = QiniuNg::Client.new access_key: ENV['access_key'], secret_key: ENV['secret_key'], config: QiniuNg::Config.new
      bucket = QiniuNg::Storage::Bucket.new client: client, bucket_name: 'z0-bucket', region: QiniuNg::Storage::Region.by_id(:as0)
      expect(bucket.regions.size).to eq 1
      expect(bucket.regions[0].id).to eq :as0
    end

    it 'could query domains from bucket' do
      client = QiniuNg::Client.new access_key: ENV['access_key'], secret_key: ENV['secret_key'], config: QiniuNg::Config.new
      bucket = QiniuNg::Storage::Bucket.new client: client, bucket_name: 'z0-bucket'
      expect(bucket.domains.size).to eq 2
    end

    it 'could set domains for bucket' do
      client = QiniuNg::Client.new access_key: ENV['access_key'], secret_key: ENV['secret_key'], config: QiniuNg::Config.new
      bucket = QiniuNg::Storage::Bucket.new client: client, bucket_name: 'z0-bucket', domains: (0...2).map { |i| "https://domain#{i}.com" }
      expect(bucket.domains).to contain_exactly(*(0...2).map { |i| "https://domain#{i}.com" })
    end
  end
end
