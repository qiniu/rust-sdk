# frozen_string_literal: true

module QiniuNg
  # 获取 SDK 动态链接库信息
  module LibInfo
    # 获取 SDK 动态链接库版本
    # @return [String] 动态链接库版本
    def self.version
      Bindings::CoreFFI::qiniu_ng_version
    end

    # 获取 SDK 动态链接库功能列表
    # @return [Array<String>] 动态链接库功能列表
    def self.features
      Bindings::CoreFFI::qiniu_ng_features.split(',')
    end
  end
end
