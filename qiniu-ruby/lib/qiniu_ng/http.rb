# frozen_string_literal: true

module QiniuNg
  # HTTP 协议功能模块
  module HTTP
    autoload :Request, 'qiniu_ng/http/request'
    autoload :Response, 'qiniu_ng/http/response'
    autoload :Headers, 'qiniu_ng/http/headers'
  end
end
