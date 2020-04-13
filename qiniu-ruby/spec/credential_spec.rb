RSpec.describe QiniuNg::Credential do
  context QiniuNg::Credential do
    it 'could create credential' do
      credential = QiniuNg::Credential.create('abcdefghklmnopq', '1234567890')
      expect(credential.access_key).to eq 'abcdefghklmnopq'
      expect(credential.secret_key).to eq '1234567890'
    end

    it 'could sign data' do
      credential = QiniuNg::Credential.create('abcdefghklmnopq', '1234567890')
      expect(credential.sign('hello')).to eq 'abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0='
      expect(credential.sign('world')).to eq 'abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ='
      expect(credential.sign('-test')).to eq 'abcdefghklmnopq:vYKRLUoXRlNHfpMEQeewG0zylaw='
      expect(credential.sign('ba#a-')).to eq 'abcdefghklmnopq:2d_Yr6H1GdTKg3RvMtpHOhi047M='
    end

    it 'could sign with data' do
      credential = QiniuNg::Credential.create('abcdefghklmnopq', '1234567890')
      expect(credential.sign_with_data('hello')).to eq 'abcdefghklmnopq:BZYt5uVRy1RVt5ZTXbaIt2ROVMA=:aGVsbG8='
      expect(credential.sign_with_data('world')).to eq 'abcdefghklmnopq:Wpe04qzPphiSZb1u6I0nFn6KpZg=:d29ybGQ='
      expect(credential.sign_with_data('-test')).to eq 'abcdefghklmnopq:HlxenSSP_6BbaYNzx1fyeyw8v1Y=:LXRlc3Q='
      expect(credential.sign_with_data('ba#a-')).to eq 'abcdefghklmnopq:kwzeJrFziPDMO4jv3DKVLDyqud0=:YmEjYS0='
    end
  end
end
