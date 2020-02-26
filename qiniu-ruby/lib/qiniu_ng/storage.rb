# frozen_string_literal: true

module QiniuNg
  # 存储相关模块
  module Storage
    autoload :Region, 'qiniu_ng/storage/region'
    autoload :Bucket, 'qiniu_ng/storage/bucket'
    autoload :Uploader, 'qiniu_ng/storage/uploader'
  end
end

