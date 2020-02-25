# frozen_string_literal: true

module QiniuNg
  module Storage
    class Bucket
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

      def name
        @bucket_name ||= @bucket.get_name
        @bucket_name.get_ptr
      end

      def region
        regions.first
      end

      def regions
        @regions ||= begin
                       regions = QiniuNg::Error.wrap_ffi_function do
                                   @bucket.get_regions
                                 end
                       (0...regions.len).map { |i| Region.send(:new, regions.get(i)) }
                     end
      end

      def domains
        @domains ||= QiniuNg::Error.wrap_ffi_function do
                       @bucket.get_domains
                     end
        (0...@domains.len).map { |i| @domains.get(i) }
      end

      def drop
        QiniuNg::Error.wrap_ffi_function do
          Bindings::Storage.drop_bucket(@client.instance_variable_get(:@client), name)
        end
        nil
      end

      def uploader(thread_pool_size: 3)
        BucketUploader.send(:new_from_bucket, @uploader_manager, self, thread_pool_size.to_i)
      end

      def inspect
        "#<#{self.class.name} @name=#{name.inspect}>"
      end
    end
  end
end
