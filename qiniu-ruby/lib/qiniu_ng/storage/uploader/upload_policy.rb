# frozen_string_literal: true

require 'ffi'

module QiniuNg
  module Storage
    class Uploader
      # 上传策略
      #
      # 从 [这里](https://developer.qiniu.com/kodo/manual/1206/put-policy) 了解七牛安全机制
      class UploadPolicy
        # @!visibility private
        def initialize(upload_policy_ffi)
          @upload_policy = upload_policy_ffi
          @cache = {}
        end
        private_class_method :new

        # 为指定的存储空间生成的上传策略
        #
        # 允许用户上传文件到指定的存储空间，不限制上传客户端指定对象名称。
        # 且这种模式下生成的上传策略将被自动设置 {Builder#insert_only}，且不允许设置 {Builder#overwritable}，
        # 因此上传时不能通过覆盖的方式修改同名对象。
        #
        # 上传策略根据给出的客户端配置指定上传凭证有效期
        #
        # @param [String] bucket_name 存储空间名称
        # @param [Config] config 客户端配置
        # @return [UploadPolicy] 返回创建的上传策略
        def self.new_for_bucket(bucket_name, config)
          Builder.new_for_bucket(bucket_name, config).build!
        end

        # 为指定的存储空间和对象名称生成的上传策略
        #
        # 允许用户以指定的对象名称上传文件到指定的存储空间。
        # 上传客户端不能指定与上传策略冲突的对象名称。
        # 且这种模式下生成的上传策略将被自动指定 {Builder#overwritable}，
        # 如果不希望允许同名对象被覆盖和修改，则应该调用 {Builder#insert_only}。
        #
        # 上传策略根据给出的客户端配置指定上传凭证有效期
        #
        # @param [String] bucket_name 存储空间名称
        # @param [String] object_key 对象名称
        # @param [Config] config 客户端配置
        # @return [UploadPolicy] 返回创建的上传策略
        def self.new_for_object(bucket_name, object_key, config)
          Builder.new_for_object(bucket_name, object_key, config).build!
        end

        # 为指定的存储空间和对象名称前缀生成的上传策略
        #
        # 允许用户以指定的对象名称前缀上传文件到指定的存储空间。
        # 上传客户端指定包含该前缀的对象名称。
        # 且这种模式下生成的上传策略将被自动指定 {Builder#overwritable}，
        # 如果不希望允许同名对象被覆盖和修改，则应该调用 {Builder#insert_only}。
        #
        # 上传策略根据给出的客户端配置指定上传凭证有效期
        #
        # @param [String] bucket_name 存储空间名称
        # @param [String] object_key_prefix 对象名称前缀
        # @param [Config] config 客户端配置
        # @return [UploadPolicy] 返回创建的上传策略
        def self.new_for_objects_with_prefix(bucket_name, object_key_prefix, config)
          Builder.new_for_objects_with_prefix(bucket_name, object_key_prefix, config).build!
        end

        # @!visibility private
        def inspect
          "#<#{self.class.name}>"
        end

        # 生成上传凭证
        # @param [String] access_key 七牛 Access Key
        # @param [String] secret_key 七牛 Secret Key
        # @return [UploadToken] 返回创建的上传凭证
        def build_token(access_key:, secret_key:)
          UploadToken.from_policy(self, access_key: access_key, secret_key: secret_key)
        end

        # 将上传凭证转换为 JSON 格式
        # @return [String] JSON 格式的上传凭证
        def as_json
          @cache[:as_json] ||= @upload_policy.as_json
          return nil if @cache[:as_json].is_null
          @cache[:as_json].get_ptr
        end

        # 从 JSON 中解析出上传凭证
        # @param [String] json JSON 格式的上传凭证
        # @return [UploadToken] 上传凭证
        def self.from_json(json)
          policy = QiniuNg::Error.wrap_ffi_function do
                     Bindings::UploadPolicy.from_json(json)
                   end
          new(policy)
        end

        # @!method object_deadline
        #   对象生命结束时间
        #   @return [Time,nil] 对象生命结束时间，精确到天
        # @!method object_lifetime
        #   对象生命周期
        #   @return [Time,nil] 对象生命周期，精确到天
        # @!method token_deadline
        #   上传凭证过期时间
        #   @return [Time,nil] 上传凭证过期时间
        # @!method token_lifetime
        #   上传凭证有效期
        #   @return [Time,nil] 上传凭证有效期

        %w[object token].each do |method|
          define_method :"#{method}_deadline" do
            timestamp_s = Bindings::CoreFFI::U64.new
            Time.at(timestamp_s[:value]) if @upload_policy.public_send(:"get_#{method}_deadline", timestamp_s)
          end

          define_method :"#{method}_lifetime" do
            lifetime_s = Bindings::CoreFFI::U64.new
            Utils::Duration::new(seconds: lifetime_s[:value]) if @upload_policy.public_send(:"get_#{method}_lifetime", lifetime_s)
          end
        end

        # @!method bucket
        #   存储空间名称约束
        #   @return [String,nil] 存储空间名称约束
        # @!method key
        #   对象名称约束
        #   @return [String,nil] 对象名称约束
        # @!method callback_body
        #   上传成功后，七牛云向业务服务器发送回调请求时的内容
        #   @return [String,nil] 回调请求的请求体
        # @!method callback_body_type
        #   上传成功后，七牛云向业务服务器发送回调请求时的 `Content-Type`
        #   @return [String,nil] 回调请求的 `Content-Type`
        # @!method callback_host
        #   上传成功后，七牛云向业务服务器发送回调请求时的 `Host`
        #   @return [String,nil] 回调请求的 `Host`
        # @!method return_body
        #   上传成功后，自定义七牛云最终返回给上传端的数据
        #   @return [String,nil] 给上传端的数据
        # @!method return_url
        #   Web 端文件上传成功后，浏览器执行 303 跳转的 URL
        #   @return [String,nil] 浏览器执行 303 跳转的 URL
        # @!method save_key
        #   上传策略中的自定义对象名称
        #   @return [String,nil] 自定义对象名称
        %i[bucket key callback_body callback_body_type callback_host return_body return_url save_key].each do |method|
          define_method(method) do
            @cache[method] ||= @upload_policy.public_send(:"get_#{method}")
            return nil if @cache[method].is_null
            @cache[method].get_ptr
          end
        end

        # @!method callback_urls
        #   上传成功后，七牛云向业务服务器发送 POST 请求的 URL 列表
        #   @return [Array<String>] 回调请求的 URL 列表
        # @!method mime_types
        #   上传策略中的 MIME 类型限定
        #   @return [Array<String>] MIME 类型限定
        %i[callback_urls mime_types].each do |method|
          define_method(method) do
            @cache[method] ||= @upload_policy.public_send(:"get_#{method}")
            (0...@cache[method].len).map { |i| @cache[method].get(i) }
          end
        end

        # @!method infrequent_storage_used?
        #   是否会使用低频存储
        #   @return [Boolean] 是否会使用低频存储
        # @!method normal_storage_used?
        #   是否会使用标准存储
        #   @return [Boolean] 是否会使用标准存储
        # @!method insert_only?
        #   是否仅允许新增对象，不允许覆盖对象
        #   @return [Boolean] 是否仅允许新增对象，不允许覆盖对象
        # @!method overwritable?
        #   是否允许覆盖对象
        #   @return [Boolean] 是否允许覆盖对象
        # @!method mime_detection_enabled?
        #   是否启用 MIME 类型自动检测
        #   @return [Boolean] 是否启用 MIME 类型自动检测
        # @!method prefixal_object_key?
        #   上传策略是否是对象名称前缀约束
        #   @return [Boolean] 上传策略是否是对象名称前缀约束
        # @!method infrequent_storage_used?
        #   是否忽略客户端指定的对象名称
        #   @return [Boolean] 是否忽略客户端指定的对象名称
        %i[is_infrequent_storage_used is_normal_storage_used is_insert_only is_overwritable
           is_mime_detection_enabled use_prefixal_object_key is_save_key_forced].each do |method|
          define_method :"#{method.to_s.sub(/^(is|use)_/, '')}?" do
            @upload_policy.public_send(method)
          end
        end

        # 获取上传策略中的上传文件尺寸的范围限定
        # @return [Array<Integer,nil>] 返回两个整型，第一个表示文件尺寸下限，第二个表示文件尺寸上限。如果其中一项不存在，则对应位置上返回 nil
        def file_size_limitation
          return @cache[:file_size_limitation] if @cache.has_key?(:file_size_limitation)
          core_ffi = Bindings.const_get :CoreFFI
          min_s = core_ffi::Size.new
          max_s = core_ffi::Size.new
          value = @upload_policy.get_file_size_limitation(min_s, max_s)
          min_s = min_s[:value].zero? ? nil : min_s[:value]
          max_s = max_s[:value].zero? ? nil : max_s[:value]
          [min_s, max_s]
        end

        # 上传策略生成器
        #
        # 通过多次调用方法修改上传策略，将具有比 UploadPolicy 的构造方法有更强大的功能
        class Builder
          # @!visibility private
          def initialize(upload_policy_builder_ffi, constructor_name, constructor_arguments)
            @builder = upload_policy_builder_ffi
            @constructor_name = constructor_name
            @constructor_arguments = constructor_arguments
          end
          private_class_method :new

          def self.new!(constructor_name, *constructor_arguments)
            new(
              Bindings::UploadPolicyBuilder.public_send(constructor_name, *constructor_arguments),
              constructor_name,
              constructor_arguments
            )
          end
          private_class_method :new!

          # 为指定的存储空间创建的上传策略生成器
          #
          # 允许用户上传文件到指定的存储空间，不限制上传客户端指定对象名称。
          # 且这种模式下生成的上传策略将被自动设置 {#insert_only}，且不允许设置 {#overwritable}，
          # 因此上传时不能通过覆盖的方式修改同名对象。
          #
          # 上传策略根据给出的客户端配置指定上传凭证有效期
          #
          # @param [String] bucket_name 存储空间名称
          # @param [Config] config 客户端配置
          # @return [Builder] 返回创建的上传策略生成器
          def self.new_for_bucket(bucket_name, config)
            raise ArgumentError, 'config must be instance of Config' unless config.is_a?(Config)
            new!(:new_for_bucket, bucket_name.to_s, config.instance_variable_get(:@config))
          end

          # 为指定的存储空间和对象名称生成的上传策略
          #
          # 允许用户以指定的对象名称上传文件到指定的存储空间。
          # 上传客户端不能指定与上传策略冲突的对象名称。
          # 且这种模式下生成的上传策略将被自动指定 {#overwritable}，
          # 如果不希望允许同名对象被覆盖和修改，则应该调用 {#insert_only}。
          #
          # 上传策略根据给出的客户端配置指定上传凭证有效期
          #
          # @param [String] bucket_name 存储空间名称
          # @param [String] object_key 对象名称
          # @param [Config] config 客户端配置
          # @return [Builder] 返回创建的上传策略生成器
          def self.new_for_object(bucket_name, object_key, config)
            raise ArgumentError, 'config must be instance of Config' unless config.is_a?(Config)
            new!(:new_for_object, bucket_name.to_s, object_key.to_s, config.instance_variable_get(:@config))
          end

          # 为指定的存储空间和对象名称前缀生成的上传策略
          #
          # 允许用户以指定的对象名称前缀上传文件到指定的存储空间。
          # 上传客户端指定包含该前缀的对象名称。
          # 且这种模式下生成的上传策略将被自动指定 {#overwritable}，
          # 如果不希望允许同名对象被覆盖和修改，则应该调用 {#insert_only}。
          #
          # 上传策略根据给出的客户端配置指定上传凭证有效期
          #
          # @param [String] bucket_name 存储空间名称
          # @param [String] object_key_prefix 对象名称前缀
          # @param [Config] config 客户端配置
          # @return [Builder] 返回创建的上传策略生成器
          def self.new_for_objects_with_prefix(bucket_name, object_key_prefix, config)
            raise ArgumentError, 'config must be instance of Config' unless config.is_a?(Config)
            new!(:new_for_objects_with_prefix, bucket_name.to_s, object_key_prefix.to_s, config.instance_variable_get(:@config))
          end

          # @!visibility private
          def inspect
            "#<#{self.class.name}>"
          end

          # 生成上传凭证
          #
          # @return [UploadPolicy] 返回生成的上传策略
          def build!
            UploadPolicy.send(:new, Bindings::UploadPolicy.build(@builder))
          end

          # 生成上传凭证
          # @param [String] access_key 七牛 Access Key
          # @param [String] secret_key 七牛 Secret Key
          # @return [UploadToken] 返回创建的上传凭证
          def build_token(access_key:, secret_key:)
            UploadToken.from_policy_builder(self, access_key: access_key, secret_key: secret_key)
          end

          # 指定上传凭证过期时间
          # @param [Time] deadline 过期时间
          # @return [Builder] 返回自身，可以形成链式调用
          def token_deadline(deadline)
            @builder.set_token_deadline(deadline.to_i)
            self
          end

          # 指定上传凭证有效期
          # @param [Utils::Duration,Hash] lifetime 有效期，可以直接传入 Hash 时间参数
          # @return [Builder] 返回自身，可以形成链式调用
          # @example
          #   builder.token_lifetime(hours: 3)
          def token_lifetime(lifetime)
            lifetime = Utils::Duration::new(lifetime) if lifetime.is_a?(Hash)
            @builder.set_token_lifetime(lifetime.to_i)
            self
          end

          # 使用标准存储
          # @return [Builder] 返回自身，可以形成链式调用
          def use_normal_storage
            @builder.use_normal_storage
            self
          end

          # 使用低频存储
          # @return [Builder] 返回自身，可以形成链式调用
          def use_infrequent_storage
            @builder.use_infrequent_storage
            self
          end

          # 仅允许创建新的对象，不允许覆盖和修改同名对象
          # @return [Builder] 返回自身，可以形成链式调用
          def insert_only
            @builder.set_insert_only
            self
          end

          # 允许覆盖和修改同名对象
          # @return [Builder] 返回自身，可以形成链式调用
          def overwritable
            @builder.set_overwritable
            self
          end

          # 上传成功后，自定义七牛云最终返回给上传端（在指定 `#return_url` 时是携带在跳转路径参数中）的数据
          # @param [String] body 返回数据，必须是合法的 JSON 文本
          # @return [Builder] 返回自身，可以形成链式调用
          def return_body(body)
            @builder.set_return_body(body.to_s)
            self
          end

          # 指定 Web 端文件上传成功后，浏览器执行 303 跳转的 URL
          # @param [String] url 跳转 URL
          # @return [Builder] 返回自身，可以形成链式调用
          def return_url(url)
            @builder.set_return_url(url.to_s)
            self
          end

          # 上传成功后，七牛云向业务服务器发送 POST 请求的 URL 列表，`Host`，回调请求的内容以及其 `Content-Type`
          # @param [Array<String>,String] urls 回调 URL 列表
          # @param [String,nil] host 回调时的 `Host`
          # @param [String] body 回调请求体
          # @param [String,nil] body_type 回调请求体的 `Content-Type`
          # @return [Builder] 返回自身，可以形成链式调用
          def callback(urls, host: nil, body:, body_type: nil)
            urls ||= []
            urls = [urls] unless urls.is_a?(Array)
            @builder.set_callback(urls.map(&:to_s), host&.to_s, body.to_s, body_type&.to_s)
            self
          end

          # 限定上传文件尺寸的范围
          # @param [Integer,nil] min 上传文件尺寸下限，单位为字节。可以传入 nil 表示没有下限
          # @param [Integer,nil] max 上传文件尺寸上限，单位为字节。可以传入 nil 表示没有上限
          # @return [Builder] 返回自身，可以形成链式调用
          def file_size_limitation(min, max)
            @builder.set_file_size_limitation(min&.to_i || 0, max&.to_i || 0)
            self
          end

          # 限定用户上传的文件类型
          # @param [Array<String>,String] mimes MIME 类型列表
          # @return [Builder] 返回自身，可以形成链式调用
          def mime_types(mimes)
            mimes ||= []
            mimes = [mimes] unless mimes.is_a?(Array)
            @builder.set_mime_types(mimes.map(&:to_s))
            self
          end

          # 禁用 MIME 类型自动检测
          # @return [Builder] 返回自身，可以形成链式调用
          def disable_mime_detection
            @builder.disable_mime_detection
            self
          end

          # 启用 MIME 类型自动检测
          # @return [Builder] 返回自身，可以形成链式调用
          def enable_mime_detection
            @builder.enable_mime_detection
            self
          end

          # 设置自定义对象名称
          # @param [String] key 对象名称
          # @param [Boolean] force 设置为 false 时，key 字段在仅当用户上传时没有注定指定对象名称时起作用。如果为 true，将始终按 key 字段的内容命名
          # @return [Builder] 返回自身，可以形成链式调用
          def save_as(key, force: false)
            @builder.set_save_as_key(key.to_s, !!force)
            self
          end

          # 指定对象生命到期时间
          # @param [Time] deadline 过期时间，精确到天
          def object_deadline(deadline)
            @builder.set_object_deadline(deadline.to_i)
            self
          end

          # 指定对象生命周期
          # @param [Time] lifetime 生命周期，精确到天，可以直接传入 Hash 时间参数
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
