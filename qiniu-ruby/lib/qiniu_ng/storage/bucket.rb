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
      #                        可以接受 {Region} 实例或符号，对于传入符号的情况，如果是 :auto_detect 表示立即检测，而其他符号表示区域 ID
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
          builder.set_region(region)
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
      def drop
        QiniuNg::Error.wrap_ffi_function do
          Bindings::Storage.drop_bucket(@client.instance_variable_get(:@client), name)
        end
        nil
      end

      # 获取存储空间上传器
      #
      # @param [Integer] thread_pool_size 上传线程池尺寸，默认使用默认的线程池策略
      # @return [BucketUploader] 返回存储空间上传器
      # @raise [ArgumentError] 线程池参数错误
      def uploader(thread_pool_size: nil)
        BucketUploader.send(:new_from_bucket, @uploader_manager, self, thread_pool_size: thread_pool_size)
      end

      # @!visibility private
      def inspect
        "#<#{self.class.name} @name=#{name.inspect}>"
      end
    end
  end
end
