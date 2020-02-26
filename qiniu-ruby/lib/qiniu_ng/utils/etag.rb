# frozen_string_literal: true

require 'ffi'
require 'forwardable'

module QiniuNg
  module Utils
    # 七牛 Etag 计算工具
    class Etag
      # Etag 长度
      ETAG_SIZE = 28

      class << self
        # 计算指定二进制数据的七牛 Etag
        # @param [String] data 二进制数据
        # @return [String] 计算得到的 Etag
        def from_data(data)
          etag_result = FFI::MemoryPointer::new(ETAG_SIZE)
          Bindings::Etag.from_buffer(data, etag_result)
          etag_result.read_bytes(ETAG_SIZE)
        end
        alias from_buffer from_data

        # 计算指定文件的七牛 Etag
        # @param [String] path 文件路径
        # @return [String] 计算得到的 Etag
        def from_file_path(path)
          etag_result = FFI::MemoryPointer::new(ETAG_SIZE)
          QiniuNg::Error.wrap_ffi_function do
            Bindings::Etag.from_file_path(path, etag_result)
          end
          etag_result.read_bytes(ETAG_SIZE)
        end

        # 计算指定数据流的七牛 Etag
        # @param [IO] io 数据流
        # @return [String] 计算得到的 Etag
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

      # 创建七牛 Etag 计算器
      def initialize
        @etag = Bindings::Etag.new!
      end
      def_delegators :@etag, :update, :reset
      alias :<< :update

      # @!method <<(data)
      #   向七牛 Etag 计算器实例输入数据
      #   @param [String] data 二进制数据
      # @!method update(data)
      #   向七牛 Etag 计算器实例输入数据
      #   @param [String] data 二进制数据

      # 从七牛 Etag 计算器获取结果
      #
      # 该方法调用后，七牛 Etag 计算器实例将被自动重置，可以重新输入新的数据
      #
      # @return [String] 计算得到的 Etag
      def result
        etag_result = FFI::MemoryPointer::new(ETAG_SIZE)
        @etag.result(etag_result)
        etag_result.read_bytes(ETAG_SIZE)
      end

      # @!visibility private
      def inspect
        "#<#{self.class.name}>"
      end
    end
  end
end
