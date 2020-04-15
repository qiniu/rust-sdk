# frozen_string_literal: true

module QiniuNg
  module Storage
    # 上传管理器
    #
    # 上传管理器可以用于构建批量上传器，或直接上传单个文件
    class Uploader
      autoload :UploadPolicy, 'qiniu_ng/storage/uploader/upload_policy'
      autoload :UploadToken, 'qiniu_ng/storage/uploader/upload_token'
      autoload :UploaderHelper, 'qiniu_ng/storage/uploader/uploader_helper'
      autoload :SingleFileUploaderHelper, 'qiniu_ng/storage/uploader/single_file_uploader_helper'
      autoload :BatchUploader, 'qiniu_ng/storage/uploader/batch_uploader'
      autoload :UploadResponse, 'qiniu_ng/storage/uploader/upload_response'
      include SingleFileUploaderHelper

      # @!visibility private
      def initialize(uploader_ffi)
        @upload_manager = uploader_ffi
      end
      private_class_method :new

      # 创建上传管理器
      # @param [Config] config 客户端配置，默认为默认客户端配置
      # @raise [ArgumentError] 参数错误
      def self.create(config = nil)
        uploader_ffi = if config.nil?
                         Bindings::UploadManager.new_default
                       else
                         raise ArgumentError, 'config must be instance of Config' unless config.is_a?(Config)
                         Bindings::UploadManager.new!(config.instance_variable_get(:@config))
                       end
        new(uploader_ffi)
      end

      # 创建批量上传器
      # @param [UploadToken, String] upload_token 默认上传凭证，如果指定，则无需传其他参数
      # @param [String] bucket_name 存储空间名称，如果指定，则必须传入 `credential` 参数
      # @param [UploadPolicy] upload_policy 上传策略，如果指定，则必须传入 `credential` 参数
      # @param [Credential] credential 认证信息，如果指定，则必须传入 `bucket_name` 或 `upload_policy` 参数
      # @return [BatchUploader] 返回批量上传器
      # @raise [ArgumentError] 参数错误
      def batch_uploader(bucket_name: nil, credential: nil, upload_policy: nil, upload_token: nil)
        uploader = case
                   when upload_token
                     batch_uploader_from_upload_token(upload_token)
                   when bucket_name && credential
                     batch_uploader_from_bucket_name_and_credential(bucket_name, credential)
                   when upload_policy && credential
                     batch_uploader_from_upload_policy_and_credential(upload_policy, credential)
                   when !credential
                     raise ArgumentError, 'credential must be specified'
                   else
                     raise ArgumentError, 'either bucket_name or upload_policy must be specified'
                   end
        BatchUploader.send(:new, uploader)
      end

      private def batch_uploader_from_bucket_name_and_credential(bucket_name, credential)
        raise ArgumentError, 'credential must be instance of Credential' unless credential.is_a?(Credential)
        Bindings::BatchUploader.new!(@upload_manager, bucket_name.to_s, credential.instance_variable_get(:@credential))
      end

      private def batch_uploader_from_upload_policy_and_credential(upload_policy, credential)
        raise ArgumentError, 'upload_policy must be instance of UploadPolicy' unless upload_policy.is_a?(UploadPolicy)
        raise ArgumentError, 'credential must be instance of Credential' unless credential.is_a?(Credential)
        QiniuNg::Error.wrap_ffi_function do
          Bindings::BatchUploader.new_for_upload_policy(@upload_manager, upload_policy.instance_variable_get(:@upload_policy), credential.instance_variable_get(:@credential))
        end
      end

      private def batch_uploader_from_upload_token(upload_token)
        QiniuNg::Error.wrap_ffi_function do
          Bindings::BatchUploader.new_for_upload_token(@upload_manager, normalize_upload_token(upload_token))
        end
      end

      # @!visibility private
      def inspect
        "#<#{self.class.name}>"
      end

      # 上传文件
      #
      # @param [IO] file 要上传的文件
      # @param [UploadToken, String] upload_token 上传凭证，如果指定，则无需传入 `bucket_name`，`credential` 和 `upload_policy`
      # @param [String] bucket_name 存储空间名称，如果指定，则必须传入 `credential` 参数
      # @param [UploadPolicy] upload_policy 上传策略，如果指定，则必须传入 `credential` 参数
      # @param [Credential] credential 认证信息，如果指定，则必须传入 `bucket_name` 或 `upload_policy` 参数
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
      def upload_file(file, bucket_name: nil,
                            credential: nil,
                            upload_policy: nil,
                            upload_token: nil,
                            key: nil,
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
        args = check_upload_params!(:reader, bucket_name, credential, upload_policy, upload_token)
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
        reader = normalize_io(file)
        file_size = file.respond_to?(:size) ? file.size : 0
        args += [reader, file_size, params]
        begin
          upload_response = QiniuNg::Error.wrap_ffi_function do
                              @upload_manager.public_send(*args)
                            end
          UploadResponse.send(:new, upload_response)
        ensure
          clear_upload_params(params)
        end
      end
      alias upload_io upload_file

      # 根据文件路径上传文件
      #
      # @param [String] file_path 要上传的文件路径
      # @param [UploadToken, String] upload_token 上传凭证，如果指定，则无需传入 `bucket_name`，`credential` 和 `upload_policy`
      # @param [String] bucket_name 存储空间名称，如果指定，则必须传入 `credential` 参数
      # @param [UploadPolicy] upload_policy 上传策略，如果指定，则必须传入 `credential` 参数
      # @param [Credential] credential 认证信息，如果指定，则必须传入 `bucket_name` 或 `upload_policy` 参数
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
      def upload_file_path(file_path, bucket_name: nil,
                                      credential: nil,
                                      upload_policy: nil,
                                      upload_token: nil,
                                      key: nil,
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
        args = check_upload_params!(:file_path, bucket_name, credential, upload_policy, upload_token)
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
        args += [file_path.to_s, params]
        upload_response = QiniuNg::Error.wrap_ffi_function do
                            @upload_manager.public_send(*args)
                          end
        UploadResponse.send(:new, upload_response)
      end
    end
  end
end
