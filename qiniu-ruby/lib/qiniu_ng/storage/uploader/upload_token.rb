# frozen_string_literal: true

module QiniuNg
  module Storage
    module Uploader
      class UploadToken
        def initialize(upload_token_ffi)
          @upload_token = upload_token_ffi
          @cache = {}
        end
        private_class_method :new

        def self.from_policy(policy, access_key:, secret_key:)
          raise ArgumentError, 'policy must be instance of UploadPolicy' unless policy.is_a?(UploadPolicy)
          new(Bindings::UploadToken.new_from_policy(
                                      policy.instance_variable_get(:@upload_policy),
                                      access_key.to_s,
                                      secret_key.to_s))
        end

        def self.from_policy_builder(policy_builder, access_key:, secret_key:)
          raise ArgumentError, 'policy_builder must be instance of UploadPolicyBuilder' unless policy_builder.is_a?(UploadPolicy::Builder)
          new(Bindings::UploadToken.new_from_policy_builder(
                                      policy_builder.instance_variable_get(:@builder),
                                      access_key.to_s,
                                      secret_key.to_s)).tap do
            policy_builder.send(:reset!)
          end
        end

        def self.from_token(token)
          new(Bindings::UploadToken.new_from_token(token.to_s))
        end

        def access_key
          @cache[:access_key] ||= QiniuNg::Error.wrap_ffi_function do
                                    @upload_token.get_access_key
                                  end
          @cache[:access_key].get_ptr
        end

        def policy
          @cache[:policy] ||= begin
                                policy = QiniuNg::Error.wrap_ffi_function do
                                           @upload_token.get_policy
                                         end
                                UploadPolicy.send(:new, policy)
                              end
          @cache[:policy]
        end

        def token
          @cache[:token] ||= QiniuNg::Error.wrap_ffi_function do
                               @upload_token.get_token
                             end
          @cache[:token].get_ptr
        end
        alias to_s token

        def inspect
          "#<#{self.class.name}>"
        end
      end
    end
  end
end
