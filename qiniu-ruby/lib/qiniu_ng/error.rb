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

    # 操作系统异常
    class OSError < Error
      attr_reader :cause, :errno
      # @!method cause
      #  @return [SystemCallError] 系统调用错误
      # @!method errno
      #  @return [Integer] 操作系统错误代码

      # 创建操作系统异常
      #
      # @param [Integer] code 操作系统错误代码
      def initialize(code)
        @errno = code
        @cause = SystemCallError.new(nil, code)
        super(@cause.message)
      end
    end

    # 非预期的重定向错误
    class UnexpectedRedirectError < Error
      # 创建非预期的重定向错误
      def initialize
        super('Unexpected redirect')
      end
    end

    # 用户取消异常
    class UserCancelledError < Error
      # 创建用户取消异常
      def initialize
        super('User canceled')
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

    # 响应状态码错误
    class ResponseStatusCodeError < Error
      attr_reader :code
      # @!method code
      #  @return [Integer] 操作系统错误代码

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
      attr_reader :curl_code, :error_kind
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
                      when :qiniu_ng_resolve_error then :resolve_error
                      when :qiniu_ng_proxy_error then :proxy_error
                      when :qiniu_ng_ssl_error then :ssl_error
                      when :qiniu_ng_connection_error then :connection_error
                      when :qiniu_ng_request_error then :request_error
                      when :qiniu_ng_response_error then :response_error
                      when :qiniu_ng_timeout_error then :timeout_error
                      else
                        :unknown_error
                      end
        super('Curl Error')
      end
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
  end

  # @!visibility private
  def Error.wrap_ffi_function
    core_ffi = Bindings.const_get(:CoreFFI)
    return_values = yield
    return_values = [return_values] unless return_values.is_a?(Array)
    errs, return_values = return_values.partition { |v| v.is_a?(core_ffi::QiniuNgErrT) }
    errs.each do |err|
      if core_ffi::qiniu_ng_err_any_error(err)
        code = FFI::MemoryPointer.new(:int)
        raise Error::OSError, code.read_int if core_ffi::qiniu_ng_err_os_error_extract(err, code)
        msg = core_ffi::QiniuNgStrT.new
        raise Error::IOError, Bindings::Str.new(msg) if core_ffi::qiniu_ng_err_io_error_extract(err, msg)
        raise Error::UnexpectedRedirectError if core_ffi::qiniu_ng_err_unexpected_redirect_error_extract(err)
        raise Error::UserCancelledError if core_ffi::qiniu_ng_err_user_canceled_error_extract(err)
        raise Error::JSONError, Bindings::Str.new(msg) if core_ffi::qiniu_ng_err_json_error_extract(err, msg)
        raise Error::ResponseStatusCodeError.new(code.read_int, Bindings::Str.new(msg)) if core_ffi::qiniu_ng_err_response_status_code_error_extract(err, code, msg)
        raise Error::UnknownError, Bindings::Str.new(msg) if core_ffi::qiniu_ng_err_unknown_error_extract(err, msg)
        curl_kind = Bindings::QiniuNgCurlErrorKindTWrapper.new
        raise Error::CurlError, code.read_int, curl_kind.inner if core_ffi::qiniu_ng_err_curl_error_extract(err, code, curl_kind)
        raise Error::CannotDropNonEmptyBucketError if core_ffi::qiniu_ng_err_drop_non_empty_bucket_error_extract(err)
        raise Error::BadMIMEError, Bindings::Str.new(msg) if core_ffi::qiniu_ng_err_bad_mime_type_error_extract(err, msg)
        err2 = core_ffi::QiniuNgInvalidUploadTokenErrorT.new
        if core_ffi::qiniu_ng_err_invalid_upload_token_extract(err, err2)
          raise Error::InvalidUploadTokenFormatError if core_ffi::qiniu_ng_err_invalid_upload_token_format_extract(err)
          raise Error::InvalidUploadTokenJSONDecodeError, Bindings::Str.new(msg) if core_ffi::qiniu_ng_err_invalid_upload_token_json_error_extract(err, msg)
          raise Error::InvalidUploadTokenBase64DecodeError, Bindings::Str.new(msg) if core_ffi::qiniu_ng_err_invalid_upload_token_base64_error_extract(err, msg)
          core_ffi::qiniu_ng_err_invalid_upload_token_error_ignore(err)
        end
        core_ffi::qiniu_ng_err_ignore(err)

        raise RuntimeError, 'Unknown QiniuNg Library Error'
      end
    end
    case return_values.size
    when 0 then nil
    when 1 then return_values.first
    else        return_values
    end
  end
end
