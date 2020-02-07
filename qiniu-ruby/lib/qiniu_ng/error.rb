# frozen_string_literal: true

module QiniuNg
  class Error < StandardError
    def inspect
      "#<#{self.class.name}>"
    end

    class IOError < Error
      def initialize(qiniu_str)
        @reason = qiniu_str
        super(@reason.get_ptr)
      end
    end

    class OSError < Error
      attr_reader :cause, :errno
      def initialize(code)
        @errno = code
        @cause = SystemCallError.new(nil, code)
        super(@cause.message)
      end
    end

    class UnexpectedRedirectError < Error
      def initialize
        super('Unexpected redirect')
      end
    end

    class UserCancelledError < Error
      def initialize
        super('User canceled')
      end
    end

    class JSONError < Error
      def initialize(qiniu_str)
        @reason = qiniu_str
        super(@reason.get_ptr)
      end
    end

    class ResponseStatusCodeError < Error
      attr_reader :code
      def initialize(code, qiniu_str)
        @code = code
        @message = qiniu_str
        super(@message.get_ptr)
      end
    end

    class UnknownError < Error
      def initialize(qiniu_str)
        @reason = qiniu_str
        super(@reason.get_ptr)
      end
    end

    class CurlError < Error
      attr_reader :curl_code
      def initialize(curl_code)
        @curl_code = curl_code
        super('Curl Error')
      end
    end

    class CannotDropNonEmptyBucketError < Error
      def initialize
        super('Drop non empty bucket is not allowed')
      end
    end

    class BadMIMEError < Error
      def initialize(qiniu_str)
        @reason = qiniu_str
        super(@reason.get_ptr)
      end
    end

    class InvalidUploadTokenError < Error
    end

    class InvalidUploadTokenFormatError < InvalidUploadTokenError
      def initialize
        super('Invalid upload token format')
      end
    end

    class InvalidUploadTokenBase64DecodeError < InvalidUploadTokenError
      def initialize(qiniu_str)
        @reason = qiniu_str
        super(@reason.get_ptr)
      end
    end

    class InvalidUploadTokenJSONDecodeError < InvalidUploadTokenError
      def initialize(qiniu_str)
        @reason = qiniu_str
        super(@reason.get_ptr)
      end
    end
  end

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
        raise Error::IOError, Bindings::Str.new(msg) if core_ffi::qiniu_ng_err_os_error_extract(err, msg)
        raise Error::UnexpectedRedirectError if core_ffi::qiniu_ng_err_unexpected_redirect_error_extract(err)
        raise Error::UserCancelledError if core_ffi::qiniu_ng_err_user_canceled_error_extract(err)
        raise Error::JSONError, Bindings::Str.new(msg) if core_ffi::qiniu_ng_err_json_error_extract(err, msg)
        raise Error::ResponseStatusCodeError.new(code.read_int, Bindings::Str.new(msg)) if core_ffi::qiniu_ng_err_response_status_code_error_extract(err, code, msg)
        raise Error::UnknownError, Bindings::Str.new(msg) if core_ffi::qiniu_ng_err_unknown_error_extract(err, msg)
        raise Error::CurlError, code.read_int if core_ffi::qiniu_ng_err_curl_error_extract(err, code)
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
