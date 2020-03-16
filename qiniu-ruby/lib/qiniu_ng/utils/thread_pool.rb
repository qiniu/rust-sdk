# frozen_string_literal: true

module QiniuNg
  module Utils
    # SDK 线程池
    module ThreadPool
      # 重新创建 SDK 使用的全局线程池
      #
      # 在每次 Fork 新进程后，应该在子进程内调用该方法以重建全局线程池，否则部分 SDK 功能在子进程内可能无法正常使用。
      # 使用该方法也可以用于调整全局线程池线程数量。
      #
      # @param [Integer] num_threads 调整后的线程池数量，默认为不调整
      # @return [void]
      def recreate_thread_pool(num_threads: 0)
        Bindings::CoreFFI::qiniu_ng_recreate_global_thread_pool(num_threads || 0)
      end

      extend self
    end
  end
end
