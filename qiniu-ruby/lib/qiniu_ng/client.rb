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

    # TODO: get upload manager
    def inspect
      "#<#{self.class.name}>"
    end
  end
end
