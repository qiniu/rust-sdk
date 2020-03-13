# frozen_string_literal: true

module QiniuNg
  module Storage
    # 上传管理器
    #
    # 上传管理器更接近于一个上传入口，帮助构建存储空间上传器或文件上传器，而本身并不具有实质管理功能
    class Uploader
      autoload :UploadPolicy, 'qiniu_ng/storage/uploader/upload_policy'
      autoload :UploadToken, 'qiniu_ng/storage/uploader/upload_token'
      autoload :UploaderHelper, 'qiniu_ng/storage/uploader/uploader_helper'
      autoload :BucketUploader, 'qiniu_ng/storage/uploader/bucket_uploader'
      autoload :BatchUploader, 'qiniu_ng/storage/uploader/batch_uploader'
      autoload :UploadResponse, 'qiniu_ng/storage/uploader/upload_response'

      # 创建上传管理器
      # @param [Config] config 客户端配置
      # @raise [ArgumentError] 参数错误
      def initialize(config)
        raise ArgumentError, 'config must be instance of Config' unless config.is_a?(Config)
        @upload_manager = Bindings::UploadManager.new!(config.instance_variable_get(:@config))
      end

      # 创建存储空间上传器
      # @param [String] bucket_name 存储空间名称
      # @param [String] access_key 七牛 Access Key
      # @param [Integer] thread_pool_size 上传线程池尺寸，默认使用默认的线程池策略
      # @return [BucketUploader] 返回存储空间上传器
      # @raise [ArgumentError] 线程池参数错误
      def bucket_uploader(bucket_name:, access_key:, thread_pool_size: nil)
        BucketUploader.send(:new_from_bucket_name, self,
                            bucket_name.to_s, access_key.to_s,
                            thread_pool_size: thread_pool_size&.to_i)
      end

      # @!visibility private
      def inspect
        "#<#{self.class.name}>"
      end
    end
  end
end
