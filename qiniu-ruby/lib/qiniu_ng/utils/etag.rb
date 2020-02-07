# frozen_string_literal: true

require 'ffi'
require 'forwardable'

module QiniuNg
  module Utils
    class Etag
      ETAG_SIZE = 28

      class << self
        def from_data(data)
          etag_result = FFI::MemoryPointer::new(ETAG_SIZE)
          Bindings::Etag.from_buffer(data, etag_result)
          etag_result.read_bytes(ETAG_SIZE)
        end
        alias from_buffer from_data

        def from_file_path(path)
          etag_result = FFI::MemoryPointer::new(ETAG_SIZE)
          QiniuNg::Error.wrap_ffi_function do
            Bindings::Etag.from_file_path(path, etag_result)
          end
          etag_result.read_bytes(ETAG_SIZE)
        end

        def from_io(io)
          io.binmode
          e = Bindings::Etag.new!
          e.update(io.read(1 << 22)) until io.eof?
          etag_result = FFI::MemoryPointer::new(ETAG_SIZE)
          e.result(etag_result)
          etag_result.read_bytes(ETAG_SIZE)
        end
        alias from_file from_io
      end

      extend Forwardable

      def initialize
        @etag = Bindings::Etag.new!
      end
      def_delegators :@etag, :update, :reset
      alias :<< :update

      # 调用 #result 方法将会自动重置缓冲区中的数据
      def result
        etag_result = FFI::MemoryPointer::new(ETAG_SIZE)
        @etag.result(etag_result)
        etag_result.read_bytes(ETAG_SIZE)
      end

      def inspect
        "#<#{self.class.name}>"
      end
    end
  end
end
