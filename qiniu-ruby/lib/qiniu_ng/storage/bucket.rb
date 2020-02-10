# frozen_string_literal: true

module QiniuNg
  module Storage
    class Bucket
      def initialize(client:, bucket_name:, region: nil, domains: [])
        raise ArgumentError, 'client must be instance of Client' unless client.is_a?(Client)
        raise ArgumentError, 'region must be instance of Region' unless region.nil? || region.is_a?(Region)

        region = region.instance_variable_get(:@region) unless region.nil?
        domains ||= []
        domains = [domains] unless domains.is_a?(Array)

        @client = client
        @bucket = Bindings::Bucket.new2(client.instance_variable_get(:@client), bucket_name.to_s, region, domains.map(&:to_s))
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
