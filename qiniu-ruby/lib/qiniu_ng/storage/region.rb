# frozen_string_literal: true

module QiniuNg
  module Storage
    class Region
      def initialize(region_ffi)
        @region = region_ffi
        @cache = {}
      end
      private_class_method :new

      def self.query(access_key:, bucket_name:, config:)
        raise ArgumentError, 'config must be instance of Config' unless config.is_a?(Config)
        regions = QiniuNg::Error.wrap_ffi_function do
                    Bindings::Region.query(bucket_name.to_s, access_key.to_s, config.instance_variable_get(:@config))
                  end
        (0...regions.len).map { |i| new(regions.get(i)) }
      end

      def self.by_id(region_id)
        region_id = case region_id.to_sym
                    when :z0 then :qiniu_ng_region_z0
                    when :z1 then :qiniu_ng_region_z1
                    when :z2 then :qiniu_ng_region_z2
                    when :as0 then :qiniu_ng_region_as0
                    when :na0 then :qiniu_ng_region_na0
                    else
                      raise ArgumentError, "invalid region id: #{region_id.inspect}"
                    end
        new(Bindings::Region.get_region_by_id(region_id))
      end

      def inspect
        "#<#{self.class.name}>"
      end

      def id
        core_ffi = Bindings.const_get :@CoreFFI
        id_s = core_ffi::QiniuNgRegionIdTWrapper.new
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

      def api_urls(use_https: true)
        @cache[[:api_urls, use_https]] ||= @region.get_api_urls(use_https)
        get_str_array_from_list(@cache[[:api_urls, use_https]])
      end

      def io_urls(use_https: true)
        @cache[[:io_urls, use_https]] ||= @region.get_io_urls(use_https)
        get_str_array_from_list(@cache[[:io_urls, use_https]])
      end

      def rs_urls(use_https: true)
        @cache[[:rs_urls, use_https]] ||= @region.get_rs_urls(use_https)
        get_str_array_from_list(@cache[[:rs_urls, use_https]])
      end

      def rsf_urls(use_https: true)
        @cache[[:rsf_urls, use_https]] ||= @region.get_rsf_urls(use_https)
        get_str_array_from_list(@cache[[:rsf_urls, use_https]])
      end

      def up_urls(use_https: true)
        @cache[[:up_urls, use_https]] ||= @region.get_up_urls(use_https)
        get_str_array_from_list(@cache[[:up_urls, use_https]])
      end

      private def get_str_array_from_list(list)
        (0...list.len).map { |i| list.get(i) }
      end
    end
  end
end
