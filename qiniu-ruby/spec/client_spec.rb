RSpec.describe QiniuNg::Client do
  context '#new' do
    it 'can create new client and get attributes' do
      client = QiniuNg::Client.create access_key: ENV['access_key'],
                                      secret_key: ENV['secret_key']
      expect(client.access_key).to eq ENV['access_key']
      expect(client.secret_key).to eq ENV['secret_key']
      expect(client.config.is_a?(QiniuNg::Config)).to be true
    end
  end

  context '#bucket_names' do
    it 'should list all bucket_names' do
      client = QiniuNg::Client.create access_key: ENV['access_key'],
                                      secret_key: ENV['secret_key']
      expect(client.bucket_names).to include('z0-bucket', 'z1-bucket', 'z0-bucket-bind')
    end
  end

  context '#create_bucket' do
    it 'should create bucket and then drop it via client' do
      client = QiniuNg::Client.create access_key: ENV['access_key'],
                                      secret_key: ENV['secret_key']
      bucket_name = "test-bucket-#{Time.now.to_i}"
      bucket = client.create_bucket(bucket_name, :z1)
      begin
        expect(bucket.is_a?(QiniuNg::Storage::Bucket)).to be true
        expect(client.bucket_names).to include(bucket_name)
      ensure
        client.drop_bucket(bucket_name)
        expect(client.bucket_names).not_to include(bucket_name)
      end
    end

    it 'should create bucket and then drop it via bucket' do
      client = QiniuNg::Client.create access_key: ENV['access_key'],
                                      secret_key: ENV['secret_key']
      bucket_name = "test-bucket-#{Time.now.to_i}"
      bucket = client.create_bucket(bucket_name, :z2)
      begin
        expect(client.bucket_names).to include(bucket_name)
      ensure
        bucket.drop!
        expect(client.bucket_names).not_to include(bucket_name)
      end
    end
  end
end
