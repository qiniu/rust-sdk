# frozen_string_literal: true

module QiniuNg
  module Storage
    # 存储空间实例
    #
    # 封装存储空间相关数据，例如配置，区域，下载域名等
    class Bucket
      # 创建存储空间实例
      #
      # 注意，该方法仅用于在 SDK 中配置生成存储空间实例，而非在七牛云服务器上创建新的存储空间
      #
      # @param [Client] client 客户端实例
      # @param [String] bucket_name 存储空间名称
      # @param [Region,Symbol] region 存储空间区域，如果传入 nil 将使用懒加载自动检测。
      #                        可以接受 {Region} 实例或符号，对于传入符号的情况，如果是 `:auto_detect` 表示立即检测，而其他符号表示区域 ID
      # @param [Array<String>] domains 下载域名列表
      # @param [Boolean] auto_detect_domains 是否自动检测下载域名，如果是，将首先使用传入的 domains，如果无法使用，才会选择七牛存储的下载域名
      # @raise [ArgumentError] 参数错误
      def initialize(client:, bucket_name:, region: nil, domains: [], auto_detect_domains: false)
        raise ArgumentError, 'client must be instance of Client' unless client.is_a?(Client)

        builder = Bindings::BucketBuilder.new!(client.instance_variable_get(:@client), bucket_name.to_s)

        case region
        when nil
          # do nothing
        when :auto_detect
          builder.auto_detect_region
        when Symbol
          builder.set_region_id(region)
        when Region
          builder.set_region(region.instance_variable_get(:@region))
        else
          raise ArgumentError, 'region must be instance of Region or Symbol'
        end

        if auto_detect_domains
          builder.auto_detect_domains
        end
        domains.each do |domain|
          builder.prepend_domain(domain.to_s)
        end

        @client = client
        @bucket = Bindings::Bucket.build(builder)
        @uploader_manager = Uploader.new(client.config)
      end

      # 存储空间名称
      # @return [String] 存储空间名称
      def name
        @bucket_name ||= @bucket.get_name
        @bucket_name.get_ptr
      end

      # 存储空间所在区域
      # @return [Region] 返回存储空间所在区域
      def region
        regions.first
      end

      # 存储空间区域列表
      #
      # 区域列表中第一个区域是当前存储空间所在区域，之后的区域则是备用区域
      #
      # @return [Array<Region>] 返回存储空间区域列表
      def regions
        @regions ||= begin
                       regions = QiniuNg::Error.wrap_ffi_function do
                                   @bucket.get_regions
                                 end
                       (0...regions.len).map { |i| Region.send(:new, regions.get(i)) }
                     end
      end

      # 存储空间下载域名列表
      # @return [Array<String>] 返回存储空间存储空间下载域名列表
      def domains
        @domains ||= QiniuNg::Error.wrap_ffi_function do
                       @bucket.get_domains
                     end
        (0...@domains.len).map { |i| @domains.get(i) }
      end

      # 删除存储空间
      # @return [void]
      def drop
        QiniuNg::Error.wrap_ffi_function do
          Bindings::Storage.drop_bucket(@client.instance_variable_get(:@client), name)
        end
        nil
      end

      # 创建批量上传器
      # @return [BatchUploader] 返回批量上传器
      def batch_uploader
        BatchUploader.send(:new, Bindings::BatchUploader.new_for_bucket(@bucket))
      end

      # 上传文件
      #
      # @param [IO] file 要上传的文件
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
      # @return [Uploader::UploadResponse] 上传响应
      # @raise [ArgumentError] 参数错误
      def upload_file(file, key: nil,
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
                            @bucket.upload_reader(normalize_io(file),
                                                  file.respond_to?(:size) ? file.size : 0,
                                                  params)
                          end
        Uploader::UploadResponse.send(:new, upload_response)
      end
      alias upload_io upload_file

      # 根据文件路径上传文件
      #
      # @param [String] file_path 要上传的文件路径
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
      # @return [Uploader::UploadResponse] 上传响应
      # @raise [ArgumentError] 参数错误
      def upload_file_path(file_path, key: nil,
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
                            @bucket.upload_file_path(file_path.to_s, params)
                          end
        Uploader::UploadResponse.send(:new, upload_response)
      end

      # @!visibility private
      def inspect
        "#<#{self.class.name} @name=#{name.inspect}>"
      end

      public_send(:include, Uploader.const_get(:SingleFileUploaderHelper))
    end
  end
end
