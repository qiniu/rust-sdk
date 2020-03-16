# frozen_string_literal: true

require 'ffi'
require 'concurrent-ruby'
require 'thread'

module QiniuNg
  module CallbackData
    Map = Concurrent::Map.new
    # LastCleanTime = Concurrent::AtomicReference.new(Time.now)
    private_constant :Map#, :LastCleanTime

    def self.put(object)
      # try_clean
      (Time.now.to_i..(2**64-1)).find { |idx| Map.put_if_absent(idx, object).nil? }
    end

    def self.get(ptr)
      Map[ptr.to_i]
    end

    def self.delete(ptr)
      Map.delete(ptr.to_i)
    end

    # # 由于回调数据有可能因各种原因无法及时清理，当数量达到 10000 且距离上次检查时间超过 1 小时，则异步清理 24 小时前创建的数据
    # def self.try_clean
    #   if Map.size > 10000 && Time.now - LastCleanTime.get > 3600
    #     LastCleanTime.update do |last_clean_time|
    #       if Time.now - last_clean_time > 3600
    #         Thread.start do
    #           to_delete_keys = []
    #           Map.each_key do |key|
    #             to_delete_keys.push(key) if Time.now - Time.at(key) > 86400
    #           end
    #           to_delete_keys.each do |key|
    #             Map.delete(key)
    #           end
    #         end
    #         Time.now
    #       else
    #         last_clean_time
    #       end
    #     end
    #   end
    # end
    # private_class_method :try_clean
  end

  private_constant :CallbackData
end
