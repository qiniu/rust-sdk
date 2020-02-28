# frozen_string_literal: true

module QiniuNg
  module Storage
    # 存储区域
    #
    # 区域实例负责管理七牛多个服务器的 URL，用于为存储管理器或上传管理器提供 URL。
    class Region
      # @!visibility private
      def initialize(region_ffi)
        @region = region_ffi
        @cache = {}
      end
      private_class_method :new

      # 创建新的存储区域
      # @return [Region] 返回新建的存储区域
      # @raise [Argument] 非法的区域 ID
      def self.create(region_id: nil,
                      up_http_urls: [],
                      up_https_urls: [],
                      rs_http_urls: [],
                      rs_https_urls: [],
                      rsf_http_urls: [],
                      rsf_https_urls: [],
                      io_http_urls: [],
                      io_https_urls: [],
                      api_http_urls: [],
                      api_https_urls: [])
        builder = Bindings::RegionBuilder.new!
        builder.set_region_id(convert_region_id(region_id)) unless region_id.nil?
        up_http_urls.each { |url| builder.append_up_http_url(url.to_s) }
        up_https_urls.each { |url| builder.append_up_https_url(url.to_s) }
        rs_http_urls.each { |url| builder.append_rs_http_url(url.to_s) }
        rs_https_urls.each { |url| builder.append_rs_https_url(url.to_s) }
        rsf_http_urls.each { |url| builder.append_rsf_http_url(url.to_s) }
        rsf_https_urls.each { |url| builder.append_rsf_https_url(url.to_s) }
        io_http_urls.each { |url| builder.append_io_http_url(url.to_s) }
        io_https_urls.each { |url| builder.append_io_https_url(url.to_s) }
        api_http_urls.each { |url| builder.append_api_http_url(url.to_s) }
        api_https_urls.each { |url| builder.append_api_https_url(url.to_s) }
        new(Bindings::Region.build(builder))
      end

      # 查询七牛服务器，根据存储空间名称获取区域列表
      #
      # @param [String] access_key 七牛 Access Key
      # @param [String] bucket_name 存储空间名称
      # @param [Config] config 客户端配置
      # @return [Array<Region>] 区域列表，区域列表中第一个区域是当前存储空间所在区域，之后的区域则是备用区域
      # @raise [ArgumentError] config 参数错误
      def self.query(access_key:, bucket_name:, config:)
        raise ArgumentError, 'config must be instance of Config' unless config.is_a?(Config)
        regions = QiniuNg::Error.wrap_ffi_function do
                    Bindings::Region.query(bucket_name.to_s, access_key.to_s, config.instance_variable_get(:@config))
                  end
        (0...regions.len).map { |i| new(regions.get(i)) }
      end

      # 通过区域 ID 获取区域实例
      #
      # @param [Symbol] region_id 区域 ID，参考官方文档 {https://developer.qiniu.com/kodo/manual/1671/region-endpoint}
      # @return [Region] 返回对应的区域实例
      # @raise [Argument] 非法的区域 ID
      def self.by_id(region_id)
        new(Bindings::Region.get_by_id(convert_region_id(region_id)))
      end

      # @!visibility private
      def inspect
        "#<#{self.class.name}>"
      end

      # 获取区域 ID
      #
      # 有可能返回 nil，通过七牛服务器查询获得的区域实例，区域 ID 通常不存在
      #
      # @return [String,nil] 返回区域 ID
      def id
        id_s = Bindings::CoreFFI::QiniuNgRegionIdTWrapper.new
        return nil unless @region.get_region_id(id_s)
        case id_s[:inner]
        when :qiniu_ng_region_z0 then :z0
        when :qiniu_ng_region_z1 then :z1
        when :qiniu_ng_region_z2 then :z2
        when :qiniu_ng_region_as0 then :as0
        when :qiniu_ng_region_na0 then :na0
        else
          raise RuntimeError, "unrecognized region id: #{id_s[:enum].inspect}"
        end
      end

      # 获取 API 服务器 URL 列表
      # @param [Boolean] use_https 是否使用 HTTPS 协议
      # @return [Array<String>] 返回 API 服务器 URL 列表
      def api_urls(use_https: true)
        @cache[[:api_urls, use_https]] ||= @region.get_api_urls(use_https)
        get_str_array_from_list(@cache[[:api_urls, use_https]])
      end

      # 获取 IO 服务器 URL 列表
      # @param [Boolean] use_https 是否使用 HTTPS 协议
      # @return [Array<String>] 返回 IO 服务器 URL 列表
      def io_urls(use_https: true)
        @cache[[:io_urls, use_https]] ||= @region.get_io_urls(use_https)
        get_str_array_from_list(@cache[[:io_urls, use_https]])
      end

      # 获取 RS 服务器 URL 列表
      # @param [Boolean] use_https 是否使用 HTTPS 协议
      # @return [Array<String>] 返回 RS 服务器 URL 列表
      def rs_urls(use_https: true)
        @cache[[:rs_urls, use_https]] ||= @region.get_rs_urls(use_https)
        get_str_array_from_list(@cache[[:rs_urls, use_https]])
      end

      # 获取 RSF 服务器 URL 列表
      # @param [Boolean] use_https 是否使用 HTTPS 协议
      # @return [Array<String>] 返回 RSF 服务器 URL 列表
      def rsf_urls(use_https: true)
        @cache[[:rsf_urls, use_https]] ||= @region.get_rsf_urls(use_https)
        get_str_array_from_list(@cache[[:rsf_urls, use_https]])
      end

      # 获取 UP 服务器 URL 列表
      # @param [Boolean] use_https 是否使用 HTTPS 协议
      # @return [Array<String>] 返回 UP 服务器 URL 列表
      def up_urls(use_https: true)
        @cache[[:up_urls, use_https]] ||= @region.get_up_urls(use_https)
        get_str_array_from_list(@cache[[:up_urls, use_https]])
      end

      private def get_str_array_from_list(list)
        (0...list.len).map { |i| list.get(i) }
      end

      def self.convert_region_id(ruby_region_id)
        case ruby_region_id.to_sym
        when :z0 then :qiniu_ng_region_z0
        when :z1 then :qiniu_ng_region_z1
        when :z2 then :qiniu_ng_region_z2
        when :as0 then :qiniu_ng_region_as0
        when :na0 then :qiniu_ng_region_na0
        else
          raise ArgumentError, "invalid region id: #{region_id.inspect}"
        end
      end
      private_class_method :convert_region_id
    end
  end
end
