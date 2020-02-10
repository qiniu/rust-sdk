# frozen_string_literal: true

module QiniuNg
  module Storage
    class Uploader
      autoload :UploadPolicy, 'qiniu_ng/storage/uploader/upload_policy'
      autoload :UploadToken, 'qiniu_ng/storage/uploader/upload_token'
      autoload :BucketUploader, 'qiniu_ng/storage/uploader/bucket_uploader'
      autoload :UploadResponse, 'qiniu_ng/storage/uploader/upload_response'

      def initialize(config)
        raise ArgumentError, 'config must be instance of Config' unless config.is_a?(Config)
        @upload_manager = Bindings::UploadManager.new!(config.instance_variable_get(:@config))
      end

      def bucket_uploader(bucket_name:, access_key:, thread_pool_size: 3)
        BucketUploader.send(:new_from_bucket_name, self, bucket_name.to_s, access_key.to_s, thread_pool_size.to_i)
      end

      def inspect
        "#<#{self.class.name}>"
      end
    end
  end
end
