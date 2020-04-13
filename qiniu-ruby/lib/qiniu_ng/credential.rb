# frozen_string_literal: true

module QiniuNg
  # 七牛认证信息
  #
  # 七牛认证信息仅包含 Access Key 和 Secret Key。主要用于签名七牛的凭证
  class Credential
    # @!visibility private
    def initialize(credential_ffi)
      @credential = credential_ffi
      @cache = {}
    end
    private_class_method :new

    # 创建认证信息
    # @param [String] access_key 七牛 Access Key
    # @param [String] secret_key 七牛 Secret Key
    # @return [Credential] 返回认证信息实例
    def self.create(access_key, secret_key)
      new(Bindings::Credential.new!(access_key.to_s, secret_key.to_s))
    end

    # 获取 Access Key
    # @return [String] 返回 Access Key
    def access_key
      @cache[:access_key] ||= @credential.get_access_key
      @cache[:access_key].get_ptr
    end

    # 获取 Secret Key
    # @return [String] 返回 Secret Key
    def secret_key
      @cache[:secret_key] ||= @credential.get_secret_key
      @cache[:secret_key].get_ptr
    end

    # 使用七牛签名算法对数据进行签名
    # @param [String] data 输入数据
    # @return [String] 返回签名结果
    def sign(data)
      Signature.new(@credential.sign(data))
    end

    # 使用七牛签名算法对数据进行签名，并同时给出签名和原数据
    # @param [String] data 输入数据
    # @return [String] 返回签名结果，并附带原数据
    def sign_with_data(data)
      Signature.new(@credential.sign_with_data(data))
    end

    # 验证七牛回调请求
    # @param [String] url 请求 URL
    # @param [String] authorization 请求 Header 中 `Authorization` 的值
    # @param [String] content_type 请求 Header 中 `Content-Type` 的值
    # @param [String] body 请求体内容
    # @return [Boolean] 是否确实是七牛回调请求
    def validate_qiniu_callback_request(url:, authorization:, content_type:, body:)
      @credential.validate_qiniu_callback_request(url.to_s, authorization.to_s, content_type.to_s, body.to_s)
    end

    # @!visibility private
    # TODO: 考虑将该类通用化
    class Signature < String
      def initialize(str)
        @str = str
        super(@str.get_ptr)
      end
    end

    # @!visibility private
    def inspect
      "#<#{self.class.name}>"
    end
  end
end
