require "qiniu_ng/version"

module QiniuNg
  class Error < StandardError; end

  autoload :Bindings, 'qiniu_ng/bindings'
  autoload :VERSION, 'qiniu_ng/version'
end
