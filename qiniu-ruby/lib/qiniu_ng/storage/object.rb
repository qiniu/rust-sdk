# frozen_string_literal: true

module QiniuNg
  module Storage
    # 对象实例
    #
    # 用于表示存储空间中的一个对象，可用来获取对象信息或对对象进行操作
    class Object
      # @!visibility private
      def initialize(object_ffi)
        @object = object_ffi
      end
      private_class_method :new

      # @!visibility private
      def self.init(bucket, key)
        raise ArgumentError, 'bucket must be instance of Bucket' unless bucket.is_a?(Bucket)
        object = Bindings::Object.new! bucket.instance_variable_get(:@bucket), key.to_s
        new(object)
      end
      private_class_method :init

      # 获得对象所在的存储空间
      # @return [Bucket] 返回对象所在的存储空间实例
      def bucket
        @bucket ||= Bucket.send(:new, @object.get_bucket)
      end

      # 获得对象名称
      # @return [String] 返回对象名称
      def key
        @key ||= @object.get_key
        @key.get_ptr
      end

      # 获得对象详细信息
      # @return [Object::Info] 返回对象详细信息
      def stat
        info = QiniuNg::Error.wrap_ffi_function do
                 @object.get_info
               end
        Info.send(:new, info)
      end

      # 删除对象
      # @return [void]
      def delete!
        QiniuNg::Error.wrap_ffi_function do
          @object.delete
        end
        nil
      end

      # 对象详细信息
      class Info
        # @!visibility private
        def initialize(info_ffi)
          @info = info_ffi
          @cache = {}
        end
        private_class_method :new

        # 获取对象尺寸
        # @return [Integer] 返回对象尺寸
        def size
          @info.get_size
        end

        # 获取对象校验和
        # @return [String] 返回对象校验和
        def hash
          @cache[:hash] ||= begin
                              data = FFI::MemoryPointer.new(256)
                              data_len = Bindings::CoreFFI::Size.new
                              @info.get_hash(data, data_len)
                              data.read_string(data_len[:value]) unless data_len[:value].zero?
                            end
        end

        # 获取对象 MIME 类型
        # @return [String] 返回对象 MIME 类型
        def mime_type
          @cache[:mime_type] ||= @info.get_mime_type
          @cache[:mime_type].get_ptr
        end

        # 获取对象上传时间
        # @return [Time] 返回对象上传时间
        def uploaded_at
          Time.at @info.get_put_time
        end
        alias put_time uploaded_at

        # @!visibility private
        def inspect
          "#<#{self.class.name} @size=#{size.inspect} @hash=#{hash.inspect} @mime_type=#{mime_type.inspect} @uploaded_at=#{uploaded_at.inspect}>"
        end
      end

      # 上传文件
      #
      # @param [IO] file 要上传的文件
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
      # @return [Uploader::UploadResponse] 上传响应
      # @raise [ArgumentError] 参数错误
      def upload_file(file, file_name: nil,
                            mime: nil,
                            vars: nil,
                            metadata: nil,
                            checksum_enabled: nil,
                            resumable_policy: nil,
                            on_uploading_progress: nil,
                            upload_threshold: nil,
                            thread_pool_size: nil,
                            max_concurrency: nil)
        params = create_upload_params(file_name: file_name,
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
                            @object.upload_reader(normalize_io(file),
                                                  file.respond_to?(:size) ? file.size : 0,
                                                  params)
                          end
        Uploader::UploadResponse.send(:new, upload_response)
      end
      alias upload_io upload_file

      # 根据文件路径上传文件
      #
      # @param [String] file_path 要上传的文件路径
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
      # @return [Uploader::UploadResponse] 上传响应
      # @raise [ArgumentError] 参数错误
      def upload_file_path(file_path, file_name: nil,
                                      mime: nil,
                                      vars: nil,
                                      metadata: nil,
                                      checksum_enabled: nil,
                                      resumable_policy: nil,
                                      on_uploading_progress: nil,
                                      upload_threshold: nil,
                                      thread_pool_size: nil,
                                      max_concurrency: nil)
        params = create_upload_params(file_name: file_name,
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
                            @object.upload_file_path(file_path.to_s, params)
                          end
        Uploader::UploadResponse.send(:new, upload_response)
      end

      # @!visibility private
      def inspect
        "#<#{self.class.name} @bucket=#{bucket.inspect} @key=#{key.inspect}>"
      end

      public_send(:include, Uploader.const_get(:SingleFileUploaderHelper))
    end
  end
end
