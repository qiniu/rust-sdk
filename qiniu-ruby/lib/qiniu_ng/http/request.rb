# frozen_string_literal: true

module QiniuNg
  module HTTP
    # HTTP 请求实例
    #
    # 封装 HTTP 请求相关数据
    class Request
      # @!visibility private
      def initialize(request_ffi)
        @request = request_ffi
        @cache = {}
      end
      private_class_method :new

      # 获取请求 URL
      # @return [String] 请求 URL
      def url
        @cache[:url] ||= @request.get_url
        @cache[:url].get_ptr
      end

      # 设置请求 URL
      # @param [String] url 请求 URL
      # @return [void]
      def url=(url)
        @cache.delete(:url)
        @request.set_url(url.to_s)
      end

      # 获取请求 HTTP 方法
      # @return [String] 请求 HTTP 方法
      def method
        case @request.get_method
        when :qiniu_ng_http_method_get  then :GET
        when :qiniu_ng_http_method_post then :POST
        when :qiniu_ng_http_method_head then :HEAD
        when :qiniu_ng_http_method_put  then :PUT
        else
          raise ArgumentError, "invalid http method: #{@request.get_method.inspect}"
        end
      end

      # 设置请求 HTTP 方法
      # @param [Symbol] method 请求 HTTP 方法
      # @return [void]
      def method=(method)
        case method.to_sym
        when :GET  then :qiniu_ng_http_method_get
        when :POST then :qiniu_ng_http_method_post
        when :HEAD then :qiniu_ng_http_method_head
        when :PUT  then :qiniu_ng_http_method_put
        else
          raise ArgumentError, "invalid http method: #{method.inspect}"
        end
        @request.set_method(method.to_sym)
      end

      # 获取请求 HTTP Headers
      # @return [String] 请求 HTTP Headers
      def headers
        headers = {}
        handler = ->(name, value, _) do
                    headers[name.force_encoding(Encoding::UTF_8)] = value.force_encoding(Encoding::UTF_8)
                    true
                  end
        @request.get_headers.each_entry(handler, nil)
        Headers.send(:new, headers, @request)
      end

      # 获取请求体内容
      # @return [String] 请求体内容
      def body
        body_ptr = Bindings::CoreFFI::Pointer.new
        body_size = Bindings::CoreFFI::Size.new
        @request.get_body(body_ptr, body_size)
        return nil unless body_size[:value] > 0
        body_ptr[:value].read_string(body_size[:value])
      end

      # 设置请求体内容
      # @param [String] body 请求体内容
      # @return [void]
      def body=(body)
        @request.set_body(body.to_s)
      end

      # 获取自定义数据
      # @return [Object] 自定义数据
      def custom_data
        idx = @request.get_custom_data
        CallbackData.get(idx) if idx
      end

      # 设置请求体内容
      # @param [Object] custom_data 自定义数据
      # @return [void]
      def custom_data=(custom_data)
        idx = @request.get_custom_data
        CallbackData.delete(idx) if idx
        @request.set_custom_data(CallbackData.put(custom_data)) if !custom_data.nil?
      end

      # 是否自动追踪重定向
      # @return [Boolean] 是否自动追踪重定向
      def follow_redirection?
        @request.will_follow_redirection
      end

      # 设置自动追踪重定向
      # @param [Boolean] yes 是否自动追踪重定向
      # @return [void]
      def set_follow_redirection(yes = true)
        @request.set_follow_redirection(yes)
      end

      # 获取预解析的套接字地址
      # @return [Array<String>] 返回预解析的套接字地址
      def resolved_socket_addrs
        @cache[:addrs] ||= @request.get_resolved_socket_addrs_as_str_list
        (0...@cache[:addrs].len).map { |i| @cache[:addrs].get(i) }
      end

      # 设置预解析的套接字地址
      # @param [Array<String>,nil] addrs 设置预解析的套接字地址，如果传入 `nil` 表示置空
      # @return [void]
      def resolved_socket_addrs=(addrs)
        addrs = [] if addrs.nil?
        addrs = [addrs] unless addrs.is_a?(Array)
        @request.clear_resolved_socket_addrs
        addrs.each do |addr|
          unless @request.append_resolved_socket_addr_as_str(addr.to_s)
            raise Error::InvalidSocketAddress, addr.to_s
          end
        end
      end
    end
  end
end
