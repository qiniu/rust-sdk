# frozen_string_literal: true

require 'ffi'

module QiniuNg
  module Storage
    module Uploader
      class UploadPolicy
        def initialize(upload_policy_ffi)
          @upload_policy = upload_policy_ffi
          @cache = {}
        end
        private_class_method :new

        def self.new_for_bucket(bucket_name, token_lifetime: Utils::Duration.new(hour: 1))
          token_lifetime = Utils::Duration.new(token_lifetime) if token_lifetime.is_a?(Hash)
          Builder.new_for_bucket(bucket_name, token_lifetime: token_lifetime).build!
        end

        def self.new_for_object(bucket_name, object_key, token_lifetime: Utils::Duration.new(hour: 1))
          token_lifetime = Utils::Duration.new(token_lifetime) if token_lifetime.is_a?(Hash)
          Builder.new_for_object(bucket_name, object_key, token_lifetime: token_lifetime).build!
        end

        def self.new_for_objects_with_prefix(bucket_name, object_key_prefix, token_lifetime: Utils::Duration.new(hour: 1))
          token_lifetime = Utils::Duration.new(token_lifetime) if token_lifetime.is_a?(Hash)
          Builder.new_for_objects_with_prefix(bucket_name, object_key_prefix, token_lifetime: token_lifetime).build!
        end

        def inspect
          "#<#{self.class.name}>"
        end

        def as_json
          @cache[:as_json] ||= @upload_policy.as_json
          return nil if @cache[:as_json].is_null
          @cache[:as_json].get_ptr
        end

        def self.from_json(json)
          policy = QiniuNg::Error.wrap_ffi_function do
                     Bindings::UploadPolicy.from_json(json)
                   end
          new(policy)
        end

        %w[object token].each do |method|
          define_method :"#{method}_deadline" do
            core_ffi = Bindings.const_get :CoreFFI
            timestamp_s = core_ffi::U64.new
            Time.at(timestamp_s[:value]) if @upload_policy.public_send(:"get_#{method}_deadline", timestamp_s)
          end

          define_method :"#{method}_lifetime" do
            core_ffi = Bindings.const_get :CoreFFI
            lifetime_s = core_ffi::U64.new
            lifetime_s[:value] if @upload_policy.public_send(:"get_#{method}_lifetime", lifetime_s)
          end
        end

        %i[bucket key callback_body callback_body_type callback_host return_body return_url save_key].each do |method|
          define_method(method) do
            @cache[method] ||= @upload_policy.public_send(:"get_#{method}")
            return nil if @cache[method].is_null
            @cache[method].get_ptr
          end
        end

        %i[callback_urls mime_types].each do |method|
          define_method(method) do
            @cache[method] ||= @upload_policy.public_send(:"get_#{method}")
            (0...@cache[method].len).map { |i| @cache[method].get(i) }
          end
        end

        %i[is_infrequent_storage_used is_normal_storage_used is_insert_only is_overwritable
           is_mime_detection_enabled is_prefixal_scope is_save_key_forced].each do |method|
          define_method :"#{method.to_s.sub(/^is_/, '')}?" do
            @upload_policy.public_send(method)
          end
        end

        def file_size_limitation
          return @cache[:file_size_limitation] if @cache.has_key?(:file_size_limitation)
          core_ffi = Bindings.const_get :CoreFFI
          min_s = core_ffi::Size.new
          max_s = core_ffi::Size.new
          value = @upload_policy.get_file_size_limitation(min_s, max_s)
          @cache[:file_size_limitation] = case value & 0b11
                                          when 0b10
                                            [min_s[:value], nil]
                                          when 0b01
                                            [nil, max_s[:value]]
                                          when 0b11
                                            [min_s[:value], max_s[:value]]
                                          else
                                            [nil, nil]
                                          end
          @cache[:file_size_limitation].freeze
        end

        class Builder
          def initialize(upload_policy_builder_ffi, constructor_name, constructor_arguments)
            @builder = upload_policy_builder_ffi
            @constructor_name = constructor_name
            @constructor_arguments = constructor_arguments
          end
          private_class_method :new

          def self.new_for_bucket(bucket_name, token_lifetime: Utils::Duration.new(hour: 1))
            new(
              Bindings::UploadPolicyBuilder.new_for_bucket(bucket_name.to_s, token_lifetime.to_i),
              :new_for_bucket,
              [bucket_name.to_s, token_lifetime.to_i],
            )
          end

          def self.new_for_object(bucket_name, object_key, token_lifetime: Utils::Duration.new(hour: 1))
            new(
              Bindings::UploadPolicyBuilder.new_for_object(bucket_name.to_s, object_key.to_s, token_lifetime.to_i),
              :new_for_object,
              [bucket_name.to_s, object_key.to_s, token_lifetime.to_i],
            )
          end

          def self.new_for_objects_with_prefix(bucket_name, object_key_prefix, token_lifetime: Utils::Duration.new(hour: 1))
            new(
              Bindings::UploadPolicyBuilder.new_for_objects_with_prefix(bucket_name.to_s, object_key_prefix.to_s, token_lifetime.to_i),
              :new_for_objects_with_prefix,
              [bucket_name.to_s, object_key_prefix.to_s, token_lifetime.to_i],
            )
          end

          def inspect
            "#<#{self.class.name}>"
          end

          def build!
            UploadPolicy.send(:new, Bindings::UploadPolicy.build(@builder))
          ensure
            @builder = Bindings::UploadPolicyBuilder.public_send(@constructor_name, *@constructor_arguments)
          end

          def token_deadline(deadline)
            @builder.set_token_deadline(deadline.to_i)
            self
          end

          def token_lifetime(lifetime)
            lifetime = Utils::Duration::new(lifetime) if lifetime.is_a?(Hash)
            @builder.set_token_lifetime(lifetime.to_i)
            self
          end

          def use_normal_storage
            @builder.use_normal_storage
            self
          end

          def use_infrequent_storage
            @builder.use_infrequent_storage
            self
          end

          def insert_only
            @builder.set_insert_only
            self
          end

          def overwritable
            @builder.set_overwritable
            self
          end

          def return_body(body)
            @builder.set_return_body(body.to_s)
            self
          end

          def return_url(url)
            @builder.set_return_url(url.to_s)
            self
          end

          def callback_urls(urls, host: nil)
            urls ||= []
            urls = [urls] unless urls.is_a?(Array)
            @builder.set_callback_urls(urls.map(&:to_s), host&.to_s)
            self
          end

          def callback_body(body, body_type: nil)
            @builder.set_callback_body(body.to_s, body_type&.to_s)
            self
          end

          def file_size_limitation(min, max)
            core_ffi = Bindings.const_get :CoreFFI
            min = if min
                    core_ffi::Size.new.tap { |ulong| ulong[:value] = min.to_i }
                  end
            max = if max
                    core_ffi::Size.new.tap { |ulong| ulong[:value] = max.to_i }
                  end
            @builder.set_file_size_limitation(min, max)
            self
          end

          def mime_types(mimes)
            mimes ||= []
            mimes = [mimes] unless mimes.is_a?(Array)
            @builder.set_mime_types(mimes.map(&:to_s))
            self
          end

          def disable_mime_detection
            @builder.disable_mime_detection
            self
          end

          def enable_mime_detection
            @builder.enable_mime_detection
            self
          end

          def save_as(key, force: false)
            @builder.set_save_as_key(key.to_s, !!force)
            self
          end

          def object_deadline(deadline)
            @builder.set_object_deadline(deadline.to_i)
            self
          end

          def object_lifetime(lifetime)
            lifetime = Utils::Duration::new(lifetime) if lifetime.is_a?(Hash)
            @builder.set_object_lifetime(lifetime.to_i)
            self
          end
        end
      end
    end
  end
end
