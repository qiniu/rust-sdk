# frozen_string_literal: true

module QiniuNg
  # SDK 错误类
  #
  # 作为所有 SDK 错误类的基类
  class Error < StandardError
    # @!visibility private
    def inspect
      "#<#{self.class.name}>"
    end

    # 上传凭证内没有存储空间相关信息
    class BucketIsMissingInUploadToken < Error
    end

    # 非法的套接字地址
    class InvalidSocketAddress < Error
      attr_reader :socket_address
      def initialize(socket_address)
        @socket_address = socket_address
        super("Socket address #{socket_address.inspect} is invalid")
      end
    end

    # 非法的 IP 地址
    class InvalidIPAddress < Error
      attr_reader :ip_address
      def initialize(ip_address)
        @ip_address = ip_address
        super("IP address #{ip_address.inspect} is invalid")
      end
    end

    # HTTP 回调函数
    class HandlerError < Error
      attr_reader :cause, :retry_kind, :is_retry_safe
      alias is_retry_safe? is_retry_safe

      # @!method cause
      #   获取出错原因
      #   @return [Error] 出错原因
      # @!method retry_kind
      #   获取重试类型
      #   @return [Symbol] 重试类型
      # @!method is_retry_safe?
      #   是否重试安全
      #   @return [Boolean] 是否重试安全

      # @!visibility private
      def initialize(cause, retry_kind, is_retry_safe)
        @cause = cause
        @retry_kind = case retry_kind.to_sym
                      when :retryable_error        then :qiniu_ng_retry_kind_retryable_error
                      when :zone_unretryable_error then :qiniu_ng_retry_kind_zone_unretryable_error
                      when :host_unretryable_error then :qiniu_ng_retry_kind_host_unretryable_error
                      when :unretryable_error      then :qiniu_ng_retry_kind_unretryable_error
                      else
                        raise ArgumentError, "invalid retry kind: #{retry_kind}"
                      end
        @is_retry_safe = !!is_retry_safe
        super(cause.message)
      end
      private_class_method :new
    end

    # IO 异常
    class IOError < Error
      # 创建 IO 异常
      #
      # @param [Bindings::QiniuNgStr] reason 错误描述信息
      def initialize(reason)
        @reason = reason
        super(@reason.get_ptr)
      end
    end

    # 回调函数 IO 异常
    class IOHandlerError < HandlerError
      # 创建回调函数 IO 异常
      # @param [String] reason 错误描述信息
      # @param [Symbol] retry_kind 重试类型
      # @param [Boolean] is_retry_safe 是否重试安全
      def initialize(reason, retry_kind: :unretryable_error, is_retry_safe: false)
        super(IOError.new(Bindings::Str.new!(reason.to_s)), retry_kind, is_retry_safe)
      end
      public_class_method :new
    end

    # 系统调用异常
    class OSError < Error
      attr_reader :cause, :errno
      # @!method cause
      #  @return [SystemCallError] 系统调用错误
      # @!method errno
      #  @return [Integer] 系统调用错误代码

      # 创建系统调用异常
      #
      # @param [Integer] errno 系统调用错误代码
      def initialize(errno)
        @errno = errno
        @cause = SystemCallError.new(nil, errno)
        super(@cause.message)
      end
    end

    # 回调函数系统调用异常
    class OSHandlerError < HandlerError
      # 创建回调函数系统调用异常
      # @param [Integer] errno 系统调用错误代码
      # @param [Symbol] retry_kind 重试类型
      # @param [Boolean] is_retry_safe 是否重试安全
      def initialize(errno, retry_kind: :unretryable_error, is_retry_safe: false)
        super(OSError.new(errno), retry_kind, is_retry_safe)
      end
      public_class_method :new
    end

    # 非预期的重定向错误
    class UnexpectedRedirectError < Error
      # 创建非预期的重定向错误
      def initialize
        super('Unexpected redirect')
      end
    end

    # 回调函数非预期的重定向错误
    class UnexpectedRedirectHandlerError < HandlerError
      # 创建回调函数非预期的重定向错误
      # @param [Symbol] retry_kind 重试类型
      # @param [Boolean] is_retry_safe 是否重试安全
      def initialize(retry_kind: :unretryable_error, is_retry_safe: false)
        super(UnexpectedRedirectError.new, retry_kind, is_retry_safe)
      end
      public_class_method :new
    end

    # 用户取消异常
    class UserCancelledError < Error
      # 创建用户取消异常
      def initialize
        super('User canceled')
      end
    end

    # 回调函数用户取消异常
    class UserCancelledHandlerError < HandlerError
      # 创建回调函数用户取消异常
      # @param [Symbol] retry_kind 重试类型
      # @param [Boolean] is_retry_safe 是否重试安全
      def initialize(retry_kind: :unretryable_error, is_retry_safe: false)
        super(UserCancelledError.new, retry_kind, is_retry_safe)
      end
      public_class_method :new
    end

    # 空文件异常
    class EmptyFileError < Error
      # 创建空文件异常
      def initialize
        super('File must not be empty')
      end
    end

    # JSON 错误
    class JSONError < Error
      # 创建 JSON 错误
      #
      # @param [Bindings::QiniuNgStr] reason 错误描述信息
      def initialize(reason)
        @reason = reason
        super(@reason.get_ptr)
      end
    end

    # 回调函数 JSON 错误
    class JSONHandlerError < HandlerError
      # 创建回调函数 JSON 错误
      # @param [String] reason 错误描述信息
      # @param [Symbol] retry_kind 重试类型
      # @param [Boolean] is_retry_safe 是否重试安全
      def initialize(reason, retry_kind: :unretryable_error, is_retry_safe: false)
        super(JSONError.new(Bindings::Str.new!(reason.to_s)), retry_kind, is_retry_safe)
      end
      public_class_method :new
    end

    # 响应状态码错误
    class ResponseStatusCodeError < Error
      attr_reader :code
      # @!method code
      #  @return [Integer] 响应状态码

      # 创建响应状态码错误
      #
      # @param [Integer] code 响应状态码
      # @param [Bindings::QiniuNgStr] reason 错误描述信息
      def initialize(code, reason)
        @code = code
        @message = reason
        super(@message.get_ptr)
      end
    end

    # 回调函数响应状态码错误
    class ResponseStatusCodeHandlerError < HandlerError
      # 创建回调函数响应状态码错误
      # @param [Integer] code 响应状态码
      # @param [String] reason 错误描述信息
      # @param [Symbol] retry_kind 重试类型
      # @param [Boolean] is_retry_safe 是否重试安全
      def initialize(code, reason, retry_kind: :unretryable_error, is_retry_safe: false)
        super(ResponseStatusCodeError.new(code, Bindings::Str.new!(reason.to_s)), retry_kind, is_retry_safe)
      end
      public_class_method :new
    end

    # 未知错误
    class UnknownError < Error
      # 创建未知错误
      #
      # @param [Bindings::QiniuNgStr] reason 错误描述信息
      def initialize(reason)
        @reason = reason
        super(@reason.get_ptr)
      end
    end

    # Curl 错误
    class CurlError < Error
      attr_reader :curl_code, :error_kind, :original_error_kind
      # @!method curl_code
      #  @return [Integer] Curl 错误代码
      # @!method error_kind
      #  @return [Symbol] Curl 错误类型

      # 创建 Curl 错误
      #
      # @param [Integer] curl_code Curl 错误代码
      # @param [Symbol] error_kind Curl 错误类型，接受如下错误类型 :qiniu_ng_resolve_error, :qiniu_ng_proxy_error, :qiniu_ng_ssl_error, :qiniu_ng_connection_error, :qiniu_ng_request_error, :qiniu_ng_response_error, :qiniu_ng_timeout_error
      def initialize(curl_code, error_kind)
        @curl_code = curl_code
        @error_kind = case error_kind
                      when :qiniu_ng_resolve_error, :resolve_error
                        @original_error_kind = :qiniu_ng_resolve_error
                        :resolve_error
                      when :qiniu_ng_proxy_error, :proxy_error
                        @original_error_kind = :qiniu_ng_proxy_error
                        :proxy_error
                      when :qiniu_ng_ssl_error, :ssl_error
                        @original_error_kind = :qiniu_ng_ssl_error
                        :ssl_error
                      when :qiniu_ng_connection_error, :connection_error
                        @original_error_kind = :qiniu_ng_connection_error
                        :connection_error
                      when :qiniu_ng_request_error, :request_error
                        @original_error_kind = :qiniu_ng_request_error
                        :request_error
                      when :qiniu_ng_response_error, :response_error
                        @original_error_kind = :qiniu_ng_response_error
                        :response_error
                      when :qiniu_ng_timeout_error, :timeout_error
                        @original_error_kind = :qiniu_ng_timeout_error
                        :timeout_error
                      else
                        @original_error_kind = :qiniu_ng_unknown_error
                        :unknown_error
                      end
        super('Curl Error')
      end
    end

    # 回调函数 Curl 错误
    class CurlHandlerError < HandlerError
      # 创建回调函数响 Curl 错误
      # @param [Integer] curl_code Curl 错误代码
      # @param [Symbol] error_kind Curl 错误类型
      # @param [Symbol] retry_kind 重试类型
      # @param [Boolean] is_retry_safe 是否重试安全
      def initialize(curl_code, error_kind, retry_kind: :unretryable_error, is_retry_safe: false)
        super(CurlError.new(curl_code, error_kind), retry_kind, is_retry_safe)
      end
      public_class_method :new
    end

    # 无法删除非空存储空间
    class CannotDropNonEmptyBucketError < Error
      # 创建无法删除非空存储空间错误
      def initialize
        super('Drop non empty bucket is not allowed')
      end
    end

    # 非法的 MIME 错误
    class BadMIMEError < Error
      # 创建非法的 MIME 错误
      #
      # @param [Bindings::QiniuNgStr] reason 错误描述信息
      def initialize(reason)
        @reason = reason
        super(@reason.get_ptr)
      end
    end

    # 非法的上传凭证错误
    #
    # 总共有三种非法的上传凭证错误类，该类是他们共同的父类
    class InvalidUploadTokenError < Error
    end

    # 非法的上传凭证格式错误
    class InvalidUploadTokenFormatError < InvalidUploadTokenError
      # 创建非法的上传凭证格式错误
      def initialize
        super('Invalid upload token format')
      end
    end

    # 上传凭证 Base64 解析错误
    class InvalidUploadTokenBase64DecodeError < InvalidUploadTokenError
      # 创建上传凭证 Base64 解析错误
      #
      # @param [Bindings::QiniuNgStr] reason 错误描述信息
      def initialize(reason)
        @reason = reason
        super(@reason.get_ptr)
      end
    end

    # 上传凭证 JSON 解析错误
    class InvalidUploadTokenJSONDecodeError < InvalidUploadTokenError
      # 创建上传凭证 JSON 解析错误
      #
      # @param [Bindings::QiniuNgStr] reason 错误描述信息
      def initialize(reason)
        @reason = reason
        super(@reason.get_ptr)
      end
    end

    # @!visibility private
    def self.wrap_ffi_function
      return_values = yield
      return_values = [return_values] unless return_values.is_a?(Array)
      errs, return_values = return_values.partition { |v| v.is_a?(Bindings::CoreFFI::QiniuNgErrT) }
      errs.each do |err|
        err = normalize_error(err)
        raise err unless err.nil?
      end
      case return_values.size
      when 0 then nil
      when 1 then return_values.first
      else        return_values
      end
    end

    # @!visibility private
    def self.normalize_error(err)
      if Bindings::CoreFFI::qiniu_ng_err_any_error(err)
        code = FFI::MemoryPointer.new(:int)
        return Error::OSError.new(code.read_int) if Bindings::CoreFFI::qiniu_ng_err_os_error_extract(err, code)
        msg = Bindings::CoreFFI::QiniuNgStrT.new
        return Error::IOError.new(Bindings::Str.new(msg)) if Bindings::CoreFFI::qiniu_ng_err_io_error_extract(err, msg)
        return Error::UnexpectedRedirectError if Bindings::CoreFFI::qiniu_ng_err_unexpected_redirect_error_extract(err)
        return Error::UserCancelledError if Bindings::CoreFFI::qiniu_ng_err_user_canceled_error_extract(err)
        return Error::EmptyFileError if Bindings::CoreFFI::qiniu_ng_err_empty_file_error_extract(err)
        return Error::JSONError.new(Bindings::Str.new(msg)) if Bindings::CoreFFI::qiniu_ng_err_json_error_extract(err, msg)
        return Error::ResponseStatusCodeError.new(code.read_int, Bindings::Str.new(msg)) if Bindings::CoreFFI::qiniu_ng_err_response_status_code_error_extract(err, code, msg)
        return Error::UnknownError.new(Bindings::Str.new(msg)) if Bindings::CoreFFI::qiniu_ng_err_unknown_error_extract(err, msg)
        curl_kind = Bindings::CoreFFI::QiniuNgCurlErrorKindTWrapper.new
        return Error::CurlError.new(code.read_int, curl_kind[:inner]) if Bindings::CoreFFI::qiniu_ng_err_curl_error_extract(err, code, curl_kind)
        return Error::CannotDropNonEmptyBucketError if Bindings::CoreFFI::qiniu_ng_err_drop_non_empty_bucket_error_extract(err)
        return Error::BadMIMEError.new(Bindings::Str.new(msg)) if Bindings::CoreFFI::qiniu_ng_err_bad_mime_type_error_extract(err, msg)
        err2 = Bindings::CoreFFI::QiniuNgInvalidUploadTokenErrorT.new
        if Bindings::CoreFFI::qiniu_ng_err_invalid_upload_token_extract(err, err2)
          return Error::InvalidUploadTokenFormatError if Bindings::CoreFFI::qiniu_ng_err_invalid_upload_token_format_extract(err)
          return Error::InvalidUploadTokenJSONDecodeError.new(Bindings::Str.new(msg)) if Bindings::CoreFFI::qiniu_ng_err_invalid_upload_token_json_error_extract(err, msg)
          return Error::InvalidUploadTokenBase64DecodeError.new(Bindings::Str.new(msg)) if Bindings::CoreFFI::qiniu_ng_err_invalid_upload_token_base64_error_extract(err, msg)
          Bindings::CoreFFI::qiniu_ng_err_invalid_upload_token_error_ignore(err)
        else
          Bindings::CoreFFI::qiniu_ng_err_ignore(err)
        end

        raise RuntimeError, 'Unknown QiniuNg Library Error'
      end
    end
    private_class_method :normalize_error
  end
end
