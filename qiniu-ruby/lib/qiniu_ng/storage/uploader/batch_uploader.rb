# frozen_string_literal: true

require 'ffi'

module QiniuNg
  module Storage
    class Uploader
      # 批量上传器
      #
      # 准备批量上传多个文件或数据流，可以反复使用以上传多个批次的文件或数据
      class BatchUploader
        include UploaderHelper

        # @!visibility private
        def initialize(batch_uploader_ffi)
          raise NotImplementedError, 'BatchUploader is unavailable for JRuby' if RUBY_ENGINE == 'jruby'
          @batch_uploader = batch_uploader_ffi
        end
        private_class_method :new

        def self.new_from_bucket_uploader(bucket_uploader, upload_token)
          raise ArgumentError, 'bucket_uploader must be instance of BucketUploader' unless bucket_uploader.is_a?(BucketUploader)
          raise ArgumentError, 'upload_token must be instance of UploadToken' unless upload_token.is_a?(UploadToken)
          bucket_uploader = bucket_uploader.instance_variable_get(:@bucket_uploader)
          upload_token = upload_token.instance_variable_get(:@upload_token)
          new(Bindings::BatchUploader.new_from_bucket_uploader(bucket_uploader, upload_token))
        end
        private_class_method :new_from_bucket_uploader

        def self.new_from_config(upload_token, config)
          raise ArgumentError, 'upload_token must be instance of UploadToken' unless upload_token.is_a?(UploadToken)
          raise ArgumentError, 'config must be instance of Config' unless config.is_a?(Config)
          upload_token = upload_token.instance_variable_get(:@upload_token)
          config = config.instance_variable_get(:@config)
          batch_uploader = Bindings::BatchUploader.new_from_config(upload_token, config)
          raise Error::BucketIsMissingInUploadToken if batch_uploader.is_freed
          new(batch_uploader)
        end
        private_class_method :new_from_config

        # 设置批量上传器预期的任务数量
        #
        # 如果预先知道上传任务的数量，可以调用该函数预分配内存空间
        #
        # @param [Integer] expected_jobs_count 预期即将推送的上传任务数量
        # @return [void]
        def expected_jobs_count=(expected_jobs_count)
          @batch_uploader.set_expected_jobs_count(expected_jobs_count.to_i)
        end

        # 设置上传文件最大并发度
        #
        # 默认情况下，上传文件时的最大并发度等于其使用的线程池大小。调用该方法可以修改最大并发度
        #
        # @param [Integer] max_concurrency 上传文件最大并发度
        # @return [void]
        def max_concurrency=(max_concurrency)
          @batch_uploader.set_max_concurrency(max_concurrency.to_i)
        end

        # 设置批量上传器线程池数量
        #
        # 批量上传器总是优先使用存储空间上传器中的线程池，如果存储空间上传器中没有创建过线程池，则自行创建专用线程池
        #
        # @param [Integer] thread_pool_size 上传线程池大小
        # @return [void]
        def thread_pool_size=(thread_pool_size)
          @batch_uploader.set_thread_pool_size(thread_pool_size.to_i)
        end

        # 推送上传文件的任务
        # @param [IO] file 要上传的文件
        # @param [UploadToken,String] upload_token 专用上传凭证，如果不传入，则默认使用批量上传器的上传凭证
        # @param [String] key 对象名称
        # @param [String] file_name 原始文件名称
        # @param [Hash] vars [自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
        # @param [Hash] metadata 元数据
        # @param [Boolean] checksum_enabled 是否启用文件校验，默认总是启用，且不推荐禁用
        # @param [Symbol] resumable_policy 分片上传策略，可以接受 `:default`, `:threshold`, `:always_be_resumeable`, `:never_be_resumeable` 四种取值
        #                                  默认且推荐使用 default 策略
        # @param [Integer] upload_threshold 分片上传策略阙值，仅当 resumable_policy 为 `:threshold` 时起效，为其设置分片上传的阙值
        # @param [Lambda] on_uploading_progress 上传进度回调，需要提供一个带有两个参数的闭包函数，其中第一个参数为已经上传的数据量，单位为字节，第二个参数为需要上传的数据总量，单位为字节。如果无法预期需要上传的数据总量，则第二个参数将总是传入 0。该函数无需返回任何值。需要注意的是，该回调函数可能会被多个线程并发调用，因此需要保证实现的函数线程安全
        # @yield [response, err] 上传完成后回调函数，用于接受上传完成后的结果。需要注意的是，该回调函数可能会被多个线程并发调用，因此需要保证实现的函数线程安全
        # @yieldparam response [UploadResponse] 上传响应，应该首先判断上传是否有错误，然后再获取上传响应中的数据
        # @yieldparam err [Error] 上传错误
        # @raise [ArgumentError] 参数错误
        # @return [void]
        def upload_file(file, upload_token: nil,
                              key: nil,
                              file_name: nil,
                              mime: nil,
                              vars: nil,
                              metadata: nil,
                              checksum_enabled: nil,
                              resumable_policy: nil,
                              on_uploading_progress: nil,
                              upload_threshold: nil,
                              &on_completed)
          params = create_upload_params(
                    upload_token: upload_token,
                    key: key,
                    file_name: file_name,
                    mime: mime,
                    vars: vars,
                    metadata: metadata,
                    checksum_enabled: checksum_enabled,
                    resumable_policy: resumable_policy,
                    on_uploading_progress: on_uploading_progress,
                    upload_threshold: upload_threshold,
                    on_completed: on_completed)
          Error.wrap_ffi_function do
            @batch_uploader.upload_reader(normalize_io(file),
                                          file.respond_to?(:size) ? file.size : 0,
                                          params)
          end
        end
        alias upload_io upload_file

        # 推送上传路径所在文件的任务
        # @param [String] file_path 要上传的文件路径
        # @param [UploadToken,String] upload_token 专用上传凭证，如果不传入，则默认使用批量上传器的上传凭证
        # @param [String] key 对象名称
        # @param [String] file_name 原始文件名称
        # @param [Hash] vars [自定义变量](https://developer.qiniu.com/kodo/manual/1235/vars#xvar)
        # @param [Hash] metadata 元数据
        # @param [Boolean] checksum_enabled 是否启用文件校验，默认总是启用，且不推荐禁用
        # @param [Symbol] resumable_policy 分片上传策略，可以接受 `:default`, `:threshold`, `:always_be_resumeable`, `:never_be_resumeable` 四种取值
        #                                  默认且推荐使用 default 策略
        # @param [Integer] upload_threshold 分片上传策略阙值，仅当 resumable_policy 为 `:threshold` 时起效，为其设置分片上传的阙值
        # @param [Lambda] on_uploading_progress 上传进度回调，需要提供一个带有两个参数的闭包函数，其中第一个参数为已经上传的数据量，单位为字节，第二个参数为需要上传的数据总量，单位为字节。如果无法预期需要上传的数据总量，则第二个参数将总是传入 0。该函数无需返回任何值。需要注意的是，该回调函数可能会被多个线程并发调用，因此需要保证实现的函数线程安全
        # @yield [response, err] 上传完成后回调函数，用于接受上传完成后的结果。需要注意的是，该回调函数可能会被多个线程并发调用，因此需要保证实现的函数线程安全
        # @yieldparam response [UploadResponse] 上传响应，应该首先判断上传是否有错误，然后再获取上传响应中的数据
        # @yieldparam err [Error] 上传错误
        # @raise [ArgumentError] 参数错误
        # @return [void]
        def upload_file_path(file_path, upload_token: nil,
                                        key: nil,
                                        file_name: nil,
                                        mime: nil,
                                        vars: nil,
                                        metadata: nil,
                                        checksum_enabled: nil,
                                        resumable_policy: nil,
                                        on_uploading_progress: nil,
                                        upload_threshold: nil,
                                        &on_completed)
          params = create_upload_params(
                    upload_token: upload_token,
                    key: key,
                    file_name: file_name,
                    mime: mime,
                    vars: vars,
                    metadata: metadata,
                    checksum_enabled: checksum_enabled,
                    resumable_policy: resumable_policy,
                    on_uploading_progress: on_uploading_progress,
                    upload_threshold: upload_threshold,
                    on_completed: on_completed)
          Error.wrap_ffi_function do
            @batch_uploader.upload_file_path(file_path.to_s, params)
          end
        end

        # 开始执行上传任务
        #
        # 需要注意的是，该方法会持续阻塞直到上传任务全部执行完毕（不保证执行顺序）。
        # 该方法不返回任何结果，上传结果由每个上传任务内定义的代码块负责返回。
        # 方法返回后，当前批量上传器的上传任务将被清空，但其他参数都将保留，可以重新添加任务并复用。
        # @return [void]
        def start
          @batch_uploader.start
        end

        # @!visibility private
        def inspect
          "#<#{self.class.name}>"
        end

        private

        def create_upload_params(upload_token: nil,
                                 key: nil,
                                 file_name: nil,
                                 mime: nil,
                                 vars: nil,
                                 metadata: nil,
                                 checksum_enabled: nil,
                                 resumable_policy: nil,
                                 on_uploading_progress: nil,
                                 upload_threshold: nil,
                                 on_completed: nil)
          params = Bindings::CoreFFI::QiniuNgBatchUploadParamsT.new
          params[:upload_token] = normalize_upload_token(upload_token).instance_variable_get(:@upload_token) unless upload_token.nil?
          params[:key] = FFI::MemoryPointer.from_string(key.to_s) unless key.nil?
          params[:file_name] = FFI::MemoryPointer.from_string(file_name.to_s) unless file_name.nil?
          params[:mime] = FFI::MemoryPointer.from_string(mime.to_s) unless mime.nil?
          params[:vars] = create_str_map(vars).instance unless vars.nil?
          params[:metadata] = create_str_map(metadata).instance unless metadata.nil?
          params[:checksum_enabled] = !!checksum_enabled unless checksum_enabled.nil?
          params[:resumable_policy] = normalize_resumable_policy(resumable_policy) unless resumable_policy.nil?
          unless on_uploading_progress.nil?
            params[:on_uploading_progress] = OnUploadingProgressCallback
          end
          params[:on_completed] = OnCompletedCallback
          params[:callback_data] = CallbackData.put(on_uploading_progress: on_uploading_progress, on_completed: on_completed)
          params[:upload_threshold] = upload_threshold.to_i unless upload_threshold.nil?
          params
        end

        OnUploadingProgressCallback = proc do |uploaded, total, idx|
          begin
            context = CallbackData.get(idx)
            context[:on_uploading_progress]&.call(uploaded, total) if context
          rescue Exception => e
            Config::CallbackExceptionHandler.call(e)
          end
        end

        OnCompletedCallback = proc do |response, err, idx|
          begin
            context = CallbackData.get(idx)
            err = Error.send(:normalize_error, err)
            response = UploadResponse.send(:new, Bindings::UploadResponse.new(response)) if err.nil?
            context[:on_completed]&.call(response, err)
          rescue Exception => e
            Config::CallbackExceptionHandler.call(e)
          ensure
            CallbackData.delete(idx)
          end
        end
        private_constant :OnUploadingProgressCallback, :OnCompletedCallback
      end
    end
  end
end
