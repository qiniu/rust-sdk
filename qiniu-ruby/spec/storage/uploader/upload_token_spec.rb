RSpec.describe QiniuNg::Storage::Uploader::UploadToken do
  context '#from_policy' do
    it 'should be ok to create upload token from upload policy' do
      config = QiniuNg::Config.new
      upload_policy = QiniuNg::Storage::Uploader::UploadPolicy.new_for_bucket('z0-bucket', config)
      upload_token = QiniuNg::Storage::Uploader::UploadToken.from_policy(upload_policy, access_key: ENV['access_key'], secret_key: ENV['secret_key'])
      expect(upload_token.access_key).to eq(ENV['access_key'])
      expect(upload_token.token).to be_start_with("#{ENV['access_key']}:")
      expect(upload_token.policy.bucket).to eq('z0-bucket')
      expect(upload_token.policy.key).to be_nil
    end
  end

  context '#from_policy_builder' do
    it 'should be ok to create upload token from upload policy builder' do
      config = QiniuNg::Config.new
      policy_builder = QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket', config)
      upload_token = QiniuNg::Storage::Uploader::UploadToken.from_policy_builder(policy_builder, access_key: ENV['access_key'], secret_key: ENV['secret_key'])
      expect(upload_token.access_key).to eq(ENV['access_key'])
      expect(upload_token.token).to be_start_with("#{ENV['access_key']}:")
      expect(upload_token.policy.bucket).to eq('z0-bucket')
      expect(upload_token.policy.key).to be_nil
    end
  end

  context '#from_token' do
    it 'should be ok to create upload token from upload policy' do
      config = QiniuNg::Config.new
      upload_policy = QiniuNg::Storage::Uploader::UploadPolicy.new_for_bucket('z0-bucket', config)
      upload_token = QiniuNg::Storage::Uploader::UploadToken.from_policy(upload_policy, access_key: ENV['access_key'], secret_key: ENV['secret_key']).
                                                             token
      upload_token_2 = QiniuNg::Storage::Uploader::UploadToken.from_token(upload_token)
      expect(upload_token_2.policy.bucket).to eq('z0-bucket')
      expect(upload_token_2.policy.key).to be_nil
    end
  end
end
