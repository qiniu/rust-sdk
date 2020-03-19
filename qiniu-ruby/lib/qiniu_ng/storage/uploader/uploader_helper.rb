# frozen_string_literal: true

require 'ffi'
require 'concurrent-ruby'

module QiniuNg
  module Storage
    class Uploader
      module UploaderHelper
        private

        def normalize_upload_token(upload_token)
          upload_token = UploadToken.from_token(upload_token) if !upload_token.is_a?(UploadToken) && upload_token.is_a?(String)
          raise ArgumentError, 'upload_token must be instance of UploadToken' unless upload_token.is_a?(UploadToken)
          upload_token
        end

        def create_str_map(hash)
          hash.each_with_object(Bindings::StrMap.new!(hash.size)) do |(key, value), strmap|
            strmap.set(key.to_s, value.to_s)
          end
        end

        def normalize_resumable_policy(resumable_policy)
          case resumable_policy
          when :default then :qiniu_ng_resumable_policy_default
          when :threshold then :qiniu_ng_resumable_policy_threshold
          when :always_be_resumeable then :qiniu_ng_resumable_policy_always_be_resumeable
          when :never_be_resumeable then :qiniu_ng_resumable_policy_never_be_resumeable
          else
            raise ArgumentError, "invalid resumable policy: #{resumable_policy.inspect}"
          end
        end

        def normalize_io(io)
          reader = Bindings::CoreFFI::QiniuNgReadableT.new
          reader[:context] = CallbackData.put(io)
          reader[:read_func] = QiniuNgReadFunc
          reader
        end

        QiniuNgReadFunc = proc do |idx, data, size, have_read|
          begin
            io = CallbackData.get(idx)
            c = if io.is_a?(IO)
                  FFI::IO.native_read(io, data, size)
                else
                  io.binmode unless !io.respond_to?(:binmode) || io.respond_to?(:binmode?) && io.binmode?
                  io_data = io.read(size)
                  data.write_string(io_data)
                  io_data.size
                end
            if c > 0
              have_read.write_ulong(c)
            else
              have_read.write_ulong(0)
              CallbackData.delete(idx)
            end
            0
          rescue => e
            Config::CallbackExceptionHandler.call(e)
            Errno::EIO::Errno
          end
        end
        private_constant :QiniuNgReadFunc
      end
      private_constant :UploaderHelper
    end
  end
end
