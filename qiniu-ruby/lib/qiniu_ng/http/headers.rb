# frozen_string_literal: true

require 'forwardable'

module QiniuNg
  module HTTP
     # 请求 / 响应 HTTP Headers
    class Headers
      include Enumerable
      extend Forwardable

      # @!visibility private
      def initialize(headers, req_resp)
        @headers = headers
        @req_resp = req_resp
      end
      private_class_method :new

      # 根据 HTTP Header 名称获取 HTTP Header 值
      # @param [String] key HTTP Header 名称
      # @return [String] HTTP Header 值
      def [](key)
        @headers[key.to_s]
      end

      # 设置 HTTP Header
      # @param [String] key HTTP Header 名称
      # @param [String] value HTTP Header 值
      # @return [void]
      def []=(key, value)
        @headers[key.to_s] = value.to_s
        @req_resp.set_header(key.to_s, value.to_s)
      end

      # 删除 HTTP Header
      # @param [String] key HTTP Header 名称
      # @return [void]
      def delete(key)
        @headers.delete(key.to_s)
        @req_resp.set_header(key.to_s, nil)
      end

      def_delegator :@headers, :each

      # 以 Hash 形式获取 HTTP Header
      # @return [Hash] 返回 Hash 形式的 HTTP Header
      def to_hash
        @headers
      end

      # @!visibility private
      def inspect
        @headers.inspect
      end
    end
  end
end
