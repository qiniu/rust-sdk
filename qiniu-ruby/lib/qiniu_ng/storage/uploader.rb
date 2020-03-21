# frozen_string_literal: true

module QiniuNg
  module Storage
    # 上传管理器
    #
    # 上传管理器可以用于构建存储空间上传器，或直接上传单个文件
    class Uploader
      autoload :UploadPolicy, 'qiniu_ng/storage/uploader/upload_policy'
      autoload :UploadToken, 'qiniu_ng/storage/uploader/upload_token'
      autoload :UploaderHelper, 'qiniu_ng/storage/uploader/uploader_helper'
      autoload :SingleFileUploaderHelper, 'qiniu_ng/storage/uploader/single_file_uploader_helper'
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

      # 创建批量上传器
      # @param [UploadToken, String] upload_token 默认上传凭证
      # @param [Config] config 客户端配置实例
      # @return [BatchUploader] 返回批量上传器
      # @raise [Error::BucketIsMissingInUploadToken] 上传凭证内没有存储空间相关信息
      def batch_uploader(upload_token, config:)
        Storage::Uploader::BatchUploader.send(:new_from_config, upload_token, config)
      end

      # @!visibility private
      def inspect
        "#<#{self.class.name}>"
      end

      include SingleFileUploaderHelper

      # 上传文件
      #
      # @param [IO] file 要上传的文件
      # @param [UploadToken,String] upload_token 上传凭证
      # @param [String] key 对象名称
      # @param [String] file_name 原始文件名称
      # @param [Hash] vars [自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
      # @param [Hash] metadata 元数据
      # @param [Boolean] checksum_enabled 是否启用文件校验，默认总是启用，且不推荐禁用
      # @param [Symbol] resumable_policy 分片上传策略，可以接受 `:default`, `:threshold`, `:always_be_resumeable`, `:never_be_resumeable` 四种取值
      #                                  默认且推荐使用 default 策略
      # @param [Lambda] on_uploading_progress 上传进度回调，需要提供一个带有两个参数的闭包函数，其中第一个参数为已经上传的数据量，单位为字节，第二个参数为需要上传的数据总量，单位为字节。如果无法预期需要上传的数据总量，则第二个参数将总是传入 0。该函数无需返回任何值。需要注意的是，该回调函数可能会被多个线程并发调用，因此需要保证实现的函数线程安全
      # @param [Integer] upload_threshold 分片上传策略阙值，仅当 resumable_policy 为 `:threshold` 时起效，为其设置分片上传的阙值
      # @param [Ingeger] thread_pool_size 上传线程池尺寸，默认使用默认的线程池策略
      # @param [Ingeger] max_concurrency 最大并发度，默认与线程池大小相同
      # @return [UploadResponse] 上传响应
      # @raise [ArgumentError] 参数错误
      def upload_file(file, upload_token:, key: nil,
                                           file_name: nil,
                                           mime: nil,
                                           vars: nil,
                                           metadata: nil,
                                           checksum_enabled: nil,
                                           resumable_policy: nil,
                                           on_uploading_progress: nil,
                                           upload_threshold: nil,
                                           thread_pool_size: nil,
                                           max_concurrency: nil)
        upload_token = normalize_upload_token(upload_token)
        params = create_upload_params(key: key,
                                      file_name: file_name,
                                      mime: mime,
                                      vars: vars,
                                      metadata: metadata,
                                      checksum_enabled: checksum_enabled,
                                      resumable_policy: resumable_policy,
                                      on_uploading_progress: on_uploading_progress,
                                      upload_threshold: upload_threshold,
                                      thread_pool_size: thread_pool_size,
                                      max_concurrency: max_concurrency)
        upload_response = QiniuNg::Error.wrap_ffi_function do
                            @upload_manager.upload_reader(
                              upload_token.instance_variable_get(:@upload_token),
                              normalize_io(file),
                              file.respond_to?(:size) ? file.size : 0,
                              params)
                          end
        UploadResponse.send(:new, upload_response)
      end
      alias upload_io upload_file

      # 根据文件路径上传文件
      #
      # @param [String] file_path 要上传的文件路径
      # @param [UploadToken,String] upload_token 上传凭证
      # @param [String] key 对象名称
      # @param [String] file_name 原始文件名称
      # @param [Hash] vars [自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
      # @param [Hash] metadata 元数据
      # @param [Boolean] checksum_enabled 是否启用文件校验，默认总是启用，且不推荐禁用
      # @param [Symbol] resumable_policy 分片上传策略，可以接受 `:default`, `:threshold`, `:always_be_resumeable`, `:never_be_resumeable` 四种取值
      #                                  默认且推荐使用 default 策略
      # @param [Lambda] on_uploading_progress 上传进度回调，需要提供一个带有两个参数的闭包函数，其中第一个参数为已经上传的数据量，单位为字节，第二个参数为需要上传的数据总量，单位为字节。如果无法预期需要上传的数据总量，则第二个参数将总是传入 0。该函数无需返回任何值。需要注意的是，该回调函数可能会被多个线程并发调用，因此需要保证实现的函数线程安全
      # @param [Integer] upload_threshold 分片上传策略阙值，仅当 resumable_policy 为 `:threshold` 时起效，为其设置分片上传的阙值
      # @param [Ingeger] thread_pool_size 上传线程池尺寸，默认使用默认的线程池策略
      # @param [Ingeger] max_concurrency 最大并发度，默认与线程池大小相同
      # @return [UploadResponse] 上传响应
      # @raise [ArgumentError] 参数错误
      def upload_file_path(file_path, upload_token:, key: nil,
                                                     file_name: nil,
                                                     mime: nil,
                                                     vars: nil,
                                                     metadata: nil,
                                                     checksum_enabled: nil,
                                                     resumable_policy: nil,
                                                     on_uploading_progress: nil,
                                                     upload_threshold: nil,
                                                     thread_pool_size: nil,
                                                     max_concurrency: nil)
        upload_token = normalize_upload_token(upload_token)
        params = create_upload_params(key: key,
                                      file_name: file_name,
                                      mime: mime,
                                      vars: vars,
                                      metadata: metadata,
                                      checksum_enabled: checksum_enabled,
                                      resumable_policy: resumable_policy,
                                      on_uploading_progress: on_uploading_progress,
                                      upload_threshold: upload_threshold,
                                      thread_pool_size: thread_pool_size,
                                      max_concurrency: max_concurrency)
        upload_response = QiniuNg::Error.wrap_ffi_function do
                            @upload_manager.upload_file_path(
                              upload_token.instance_variable_get(:@upload_token),
                              file_path.to_s,
                              params)
                          end
        UploadResponse.send(:new, upload_response)
      end
    end
  end
end
