# frozen_string_literal: true

require 'ffi'
require 'json'

module QiniuNg
  module Storage
    class Uploader
      # 上传响应
      class UploadResponse
        # @!visibility private
        def initialize(upload_response_ffi)
          @upload_response = upload_response_ffi
          @cache = {}
        end
        private_class_method :new

        # 上传响应中的校验和字段
        # @return [String,nil] 返回上传响应中的校验和字段
        def hash
          core_ffi = Bindings.const_get :CoreFFI
          data = FFI::MemoryPointer.new(256)
          data_len = core_ffi::Size.new
          @upload_response.get_hash(data, data_len)
          return nil if data_len[:value].zero?
          data.read_string(data_len[:value])
        end

        # 上传响应中的对象名称字段
        # @return [String,nil] 返回上传响应中的对象名称字段
        def key
          @cache[:key] ||= @upload_response.get_key
          return nil if @cache[:key].is_null
          @cache[:key].get_ptr
        end

        # 获取 JSON 格式的上传响应
        # @return [String] 返回 JSON 格式的上传响应
        def as_json
          @cache[:json] ||= QiniuNg::Error.wrap_ffi_function do
                              @upload_response.get_string
                            end
          return nil if @cache[:json].is_null
          @cache[:json].get_ptr
        end

        # @!visibility private
        def method_missing(method_name)
          @cache[:parsed_json] ||= JSON.load(as_json)
          if @cache[:parsed_json].has_key?(method_name.to_s)
            return @cache[:parsed_json][method_name.to_s]
          end
          super
        end

        # @!visibility private
        def inspect
          "#<#{self.class.name}>"
        end
      end
    end
  end
end
