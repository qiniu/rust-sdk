# frozen_string_literal: true

module QiniuNg
  module Storage
    class Uploader
      # 上传凭证
      #
      # 从这里 {https://developer.qiniu.com/kodo/manual/1208/upload-token} 了解七牛安全机制
      class UploadToken
        # @!visibility private
        def initialize(upload_token_ffi)
          @upload_token = upload_token_ffi
          @cache = {}
        end
        private_class_method :new

        # 通过上传策略生成上传凭证
        # @param [UploadPolicy] policy 上传策略
        # @param [String] access_key 七牛 Access Key
        # @param [String] secret_key 七牛 Secret Key
        # @return [UploadToken] 返回创建的上传凭证
        # @raise [ArgumentError] 参数错误
        def self.from_policy(policy, access_key:, secret_key:)
          raise ArgumentError, 'policy must be instance of UploadPolicy' unless policy.is_a?(UploadPolicy)
          new(Bindings::UploadToken.new_from_policy(
                                      policy.instance_variable_get(:@upload_policy),
                                      access_key.to_s,
                                      secret_key.to_s))
        end

        # 通过上传策略生成器生成上传凭证
        # @param [UploadPolicyBuilder] builder 上传策略生成器
        # @param [String] access_key 七牛 Access Key
        # @param [String] secret_key 七牛 Secret Key
        # @return [UploadToken] 返回创建的上传凭证
        # @raise [ArgumentError] 参数错误
        def self.from_policy_builder(builder, access_key:, secret_key:)
          raise ArgumentError, 'builder must be instance of UploadPolicy::Builder' unless builder.is_a?(UploadPolicy::Builder)
          new(Bindings::UploadToken.new_from_policy_builder(
                                      builder.instance_variable_get(:@builder),
                                      access_key.to_s,
                                      secret_key.to_s))
        end

        # 通过字符串生成上传凭证
        # @param [String] token 上传凭证字符串
        # @return [UploadToken] 返回创建的上传凭证
        def self.from(token)
          new(Bindings::UploadToken.new_from(token.to_s))
        end

        # 获取上传凭证中的 Access Key
        # @return [String] 返回 Access Key
        def access_key
          @cache[:access_key] ||= QiniuNg::Error.wrap_ffi_function do
                                    @upload_token.get_access_key
                                  end
          @cache[:access_key].get_ptr
        end

        # 获取上传凭证中的上传策略部分
        # @return [UploadPolicy] 返回上传策略
        def policy
          @cache[:policy] ||= begin
                                policy = QiniuNg::Error.wrap_ffi_function do
                                           @upload_token.get_policy
                                         end
                                UploadPolicy.send(:new, policy)
                              end
          @cache[:policy]
        end

        # 返回上传凭证字符串
        # @return [String] 返回上传凭证字符串
        def to_s
          @cache[:token] ||= QiniuNg::Error.wrap_ffi_function do
                               @upload_token.get_string
                             end
          @cache[:token].get_ptr
        end
        alias token to_s

        # @!visibility private
        def inspect
          "#<#{self.class.name}>"
        end
      end
    end
  end
end
