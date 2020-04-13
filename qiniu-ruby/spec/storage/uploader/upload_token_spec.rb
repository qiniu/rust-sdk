RSpec.describe QiniuNg::Storage::Uploader::UploadToken do
  context '#from_policy' do
    it 'should be ok to create upload token from upload policy' do
      upload_policy = QiniuNg::Storage::Uploader::UploadPolicy.new_for_bucket('z0-bucket')
      upload_token = upload_policy.build_token(access_key: ENV['access_key'], secret_key: ENV['secret_key'])
      expect(upload_token).to be_start_with("#{ENV['access_key']}:")
      expect(upload_token.access_key).to eq(ENV['access_key'])
      expect(upload_token.policy.bucket).to eq('z0-bucket')
      expect(upload_token.policy.key).to be_nil
    end
  end

  context '#from_policy_builder' do
    it 'should be ok to create upload token from upload policy builder' do
      policy_builder = QiniuNg::Storage::Uploader::UploadPolicy::Builder.new_for_bucket('z0-bucket')
      upload_token = policy_builder.build_token(access_key: ENV['access_key'], secret_key: ENV['secret_key'])
      expect(upload_token).to be_start_with("#{ENV['access_key']}:")
      expect(upload_token.access_key).to eq(ENV['access_key'])
      expect(upload_token.policy.bucket).to eq('z0-bucket')
      expect(upload_token.policy.key).to be_nil
    end
  end

  context '#from_token' do
    it 'should be ok to create upload token from upload policy' do
      upload_policy = QiniuNg::Storage::Uploader::UploadPolicy.new_for_bucket('z0-bucket')
      upload_token = upload_policy.build_token(access_key: ENV['access_key'], secret_key: ENV['secret_key'])
      upload_token_2 = QiniuNg::Storage::Uploader::UploadToken.from(upload_token)
      expect(upload_token_2.policy.bucket).to eq('z0-bucket')
      expect(upload_token_2.policy.key).to be_nil
    end
  end
end
