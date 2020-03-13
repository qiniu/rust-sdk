# frozen_string_literal: true

require 'ffi'
require 'stringio'
require 'tempfile'

module QiniuNg
  module HTTP
    # HTTP 响应实例
    #
    # 封装 HTTP 响应相关数据
    class Response
      # @!visibility private
      def initialize(response_ffi)
        @response = response_ffi
        @cache = {}
      end
      private_class_method :new

      # 获取响应状态码
      # @return [Integer] 响应状态码
      def status_code
        @response.get_status_code
      end

      # 设置响应状态码
      # @param [Integer] status_code 响应状态码
      def status_code=(status_code)
        @response.set_status_code(status_code.to_i)
      end

      # 获取响应 HTTP Headers
      # @return [String] 响应 HTTP Headers
      def headers
        headers = {}
        handler = ->(name, value, _) do
                    headers[name] = value
                    true
                  end
        @response.get_headers.each_entry(handler, nil)
        Headers.send(:new, headers, @response)
      end

      # 获取服务器 IP 地址
      # @return [String] 服务器 IP 地址
      def server_ip
        @response.get_server_ip_as_str
      end

      # 设置服务器 IP 地址
      # @param [String] server_ip 服务器 IP 地址
      def server_ip=(server_ip)
        if server_port.nil?
          @response.unset_server_ip
        else
          @response.set_server_ip_as_str = server_ip.to_s
        end
      end

      # 获取服务器端口
      # @return [Integer] 服务器端口
      def server_port
        @response.get_server_port
      end

      # 设置服务器端口
      # @param [Integer] server_port 服务器端口
      def server_port=(server_port)
        @response.set_server_port(server_port.to_i)
      end

      # 获取响应体内容
      #
      # 响应体如果较小，将会以 `StringIO` 的形式返回。如果较大，将会使用临时文件存储响应体内容，然后将临时文件实例返回
      #
      # @return [#read] 响应体内容
      def body
        body_size = self.body_size
        if body_size > 1 << 22
          tempfile = Tempfile.new(encoding: 'ascii-8bit')
          Error.wrap_ffi_function do
            @response.dump_body_to_file(tempfile.path)
          end
          tempfile.rewind
          tempfile
        else
          body = FFI::MemoryPointer.new(body_size)
          Error.wrap_ffi_function do
            @response.dump_body(body_size, body, nil)
          end
          StringIO.new(body.read_string(body_size))
        end
      end

      # 设置响应体内容
      #
      # 响应体可以以阅读器的方式提供，要求参数必须可以被调用 `read` 方法。
      # 此外，也可以以字符串的形式提供
      #
      # @param [#read,#to_s] body 响应体内容
      def body=(body)
        if body.respond_to?(:read)
          reader = Bindings::CoreFFI::QiniuNgReadableT.new
          reader[:context] = CallbackData.put(body)
          reader[:read_func] = QiniuNgReadFunc
          @response.set_body_to_reader(reader)
        elsif body.respond_to?(:to_s)
          @response.set_body(body.to_s)
        else
          raise ArgumentError, 'invalid body, only string or readable instance is acceptable'
        end
      end

      QiniuNgReadFunc = proc do |idx, data, size, have_read|
        begin
          body = CallbackData.get(idx)
          c = FFI::IO.native_read(body, data, size)
          if c > 0
            have_read.write_ulong(c)
          else
            have_read.write_ulong(0)
            CallbackData.delete(idx)
          end
          0
        rescue => e
          # TODO: use error handler instead
          STDERR.puts e.message
          e.backtrace.each { |trace| STDERR.puts "\t#{trace}" }
          Errno::EIO::Errno
        end
      end
      private_constant :QiniuNgReadFunc

      # 获取响应体尺寸
      # @return [Integer] 响应体尺寸
      def body_length
        length = Bindings::CoreFFI::U64.new
        Error.wrap_ffi_function do
          @response.get_body_length(length)
        end
        length[:value]
      end
      alias body_size body_length
    end
  end
end
