# frozen_string_literal: true

module QiniuNg
  class Client
    def initialize(access_key:, secret_key:, config:)
      raise ArgumentError, 'config must be instance of Config' unless config.is_a?(Config)
      @client = Bindings::Client.new!(access_key.to_s, secret_key.to_s, config.instance_variable_get(:@config))
    end

    def bucket(name)
      Storage::Bucket.new(client: self, bucket_name: name)
    end

    def bucket_names
      list = QiniuNg::Error.wrap_ffi_function do
               Bindings::Storage.bucket_names(@client)
             end
      (0...list.len).map { |i| list.get(i) }
    end

    def create_bucket(bucket_name, region)
      region = region.id if region.is_a?(Storage::Region)
      region_id = case region.to_sym
                  when :z0 then :qiniu_ng_region_z0
                  when :z1 then :qiniu_ng_region_z1
                  when :z2 then :qiniu_ng_region_z2
                  when :as0 then :qiniu_ng_region_as0
                  when :na0 then :qiniu_ng_region_na0
                  else
                    raise ArgumentError, "invalid region id: #{region_id.inspect}"
                  end
      QiniuNg::Error.wrap_ffi_function do
        Bindings::Storage.create_bucket(@client, bucket_name.to_s, region_id)
      end
      bucket(bucket_name.to_s)
    end

    def drop_bucket(bucket_name)
      QiniuNg::Error.wrap_ffi_function do
        Bindings::Storage.drop_bucket(@client, bucket_name.to_s)
      end
      nil
    end

    # TODO: get upload manager
    def inspect
      "#<#{self.class.name}>"
    end
  end
end
