# frozen_string_literal: true

# 下一代七牛 Ruby SDK
#
# 这是 QiniuNg-Ruby 的主要名字空间。
module QiniuNg
  autoload :Bindings, 'qiniu_ng/bindings'
  autoload :LibInfo, 'qiniu_ng/lib_info'
  autoload :VERSION, 'qiniu_ng/version'
  autoload :Config, 'qiniu_ng/config'
  autoload :Client, 'qiniu_ng/client'
  autoload :Credential, 'qiniu_ng/credential'
  autoload :Storage, 'qiniu_ng/storage'
  autoload :Utils, 'qiniu_ng/utils'
  autoload :Error, 'qiniu_ng/error'
  autoload :HTTP, 'qiniu_ng/http'
  autoload :CallbackData, 'qiniu_ng/callback_data'
end
