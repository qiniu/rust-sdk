# frozen_string_literal: true

module QiniuNg
  module Storage
    class Uploader
      # 存储空间上传器
      #
      # 为指定存储空间的上传准备初始化数据，可以反复使用以上传多个文件
      class BucketUploader
        include UploaderHelper

        # @!visibility private
        def initialize(bucket_uploader_ffi)
          @bucket_uploader = bucket_uploader_ffi
        end
        private_class_method :new

        def self.new_from_bucket(upload_manager, bucket, thread_pool_size: nil)
          raise ArgumentError, 'upload_manager must be instance of Uploader' unless upload_manager.is_a?(Uploader)
          raise ArgumentError, 'bucket must be instance of Bucket' unless bucket.is_a?(Bucket)
          raise ArgumentError, 'invalid thread_pool_size' if !thread_pool_size.nil? && thread_pool_size <= 0
          upload_manager = upload_manager.instance_variable_get(:@upload_manager)
          bucket = bucket.instance_variable_get(:@bucket)
          new(Bindings::BucketUploader.new_from_bucket(upload_manager, bucket, thread_pool_size&.to_i || 0))
        end
        private_class_method :new_from_bucket

        def self.new_from_bucket_name(upload_manager, bucket_name, access_key, thread_pool_size: nil)
          raise ArgumentError, 'upload_manager must be instance of Uploader' unless upload_manager.is_a?(Uploader)
          raise ArgumentError, 'invalid thread_pool_size' if !thread_pool_size.nil? && thread_pool_size <= 0
          upload_manager = upload_manager.instance_variable_get(:@upload_manager)
          new(Bindings::BucketUploader.new_from_bucket_name(upload_manager, bucket_name.to_s, access_key.to_s, thread_pool_size&.to_i || 0))
        end
        private_class_method :new_from_bucket_name

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
        # @param [Lambda] on_uploading_progress 上传进度回调，需要提供一个带有两个参数的闭包函数，其中第一个参数为已经上传的数据量，单位为字节，第二个参数为需要上传的数据总量，单位为字节。如果无法预期需要上传的数据总量，则第二个参数将总是传入 0。该函数无需返回任何值。需要注意的是，该回调函数可能会被多个线程并发调用，因此需要保证实现的函数具有可重入性
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
                              @bucket_uploader.upload_reader(
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
        # @param [Lambda] on_uploading_progress 上传进度回调，需要提供一个带有两个参数的闭包函数，其中第一个参数为已经上传的数据量，单位为字节，第二个参数为需要上传的数据总量，单位为字节。如果无法预期需要上传的数据总量，则第二个参数将总是传入 0。该函数无需返回任何值。需要注意的是，该回调函数可能会被多个线程并发调用，因此需要保证实现的函数具有可重入性
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
                              @bucket_uploader.upload_file_path(
                                upload_token.instance_variable_get(:@upload_token),
                                file_path.to_s,
                                params)
                            end
          UploadResponse.send(:new, upload_response)
        end

        # 创建批量上传器
        #
        # @param [UploadToken, String] upload_token 默认上传凭证
        # @return [BatchUploader] 批量上传器
        def batch(upload_token:)
          BatchUploader.send(:new_from_bucket_uploader, self, normalize_upload_token(upload_token))
        end

        # @!visibility private
        def inspect
          "#<#{self.class.name}>"
        end

        private

        def create_upload_params(key: nil,
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
          params = Bindings::CoreFFI::QiniuNgUploadParamsT.new
          params[:key] = FFI::MemoryPointer.from_string(key.to_s) unless key.nil?
          params[:file_name] = FFI::MemoryPointer.from_string(file_name.to_s) unless file_name.nil?
          params[:mime] = FFI::MemoryPointer.from_string(mime.to_s) unless mime.nil?
          params[:vars] = create_str_map(vars).instance unless vars.nil?
          params[:metadata] = create_str_map(metadata).instance unless metadata.nil?
          params[:checksum_enabled] = !!checksum_enabled unless checksum_enabled.nil?
          params[:resumable_policy] = normalize_resumable_policy(resumable_policy) unless resumable_policy.nil?
          unless on_uploading_progress.nil?
            params[:callback_data] = CallbackData.put(on_uploading_progress: on_uploading_progress)
            params[:on_uploading_progress] = OnUploadingProgressCallback
          end
          params[:upload_threshold] = upload_threshold.to_i unless upload_threshold.nil?
          unless thread_pool_size.nil?
            thread_pool_size = thread_pool_size.to_i
            raise ArgumentError, 'invalid thread_pool_size' if thread_pool_size <= 0
            params[:thread_pool_size] = thread_pool_size
          end
          unless max_concurrency.nil?
            max_concurrency = max_concurrency.to_i
            raise ArgumentError, 'invalid max_concurrency' if max_concurrency <= 0
            params[:max_concurrency] = max_concurrency
          end
          params
        end

        OnUploadingProgressCallback = proc do |uploaded, total, idx|
          context = CallbackData.get(idx)
          context[:on_uploading_progress]&.call(uploaded, total)
          CallbackData.delete(idx) if uploaded == total
        end
        private_constant :OnUploadingProgressCallback
      end
    end
  end
end
