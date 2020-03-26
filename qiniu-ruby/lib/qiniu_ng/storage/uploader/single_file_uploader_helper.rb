# frozen_string_literal: true

require 'ffi'

module QiniuNg
  module Storage
    class Uploader
      module SingleFileUploaderHelper
        include UploaderHelper

        private

        def create_upload_params(key: nil,
                                 file_name: nil,
                                 mime: nil,
                                 vars: nil,
                                 metadata: nil,
                                 checksum_enabled: nil,
                                 resumable_policy: nil,
                                 on_uploading_progress: nil,
                                 upload_threshold: nil,
                                 thread_pool_size: nil,
                                 max_concurrency: nil)
          params = Bindings::CoreFFI::QiniuNgUploadParamsT.new
          params[:key] = FFI::MemoryPointer.from_string(key.to_s) unless key.nil?
          params[:file_name] = FFI::MemoryPointer.from_string(file_name.to_s) unless file_name.nil?
          params[:mime] = FFI::MemoryPointer.from_string(mime.to_s) unless mime.nil?
          params[:vars] = create_str_map(vars).instance unless vars.nil?
          params[:metadata] = create_str_map(metadata).instance unless metadata.nil?
          params[:checksum_enabled] = !!checksum_enabled unless checksum_enabled.nil?
          params[:resumable_policy] = normalize_resumable_policy(resumable_policy) unless resumable_policy.nil?
          unless on_uploading_progress.nil?
            params[:callback_data] = CallbackData.put(on_uploading_progress: on_uploading_progress)
            params[:on_uploading_progress] = OnUploadingProgressCallback
          end
          params[:upload_threshold] = upload_threshold.to_i unless upload_threshold.nil?
          unless thread_pool_size.nil?
            thread_pool_size = thread_pool_size.to_i
            raise ArgumentError, 'invalid thread_pool_size' if thread_pool_size <= 0
            params[:thread_pool_size] = thread_pool_size
          end
          unless max_concurrency.nil?
            max_concurrency = max_concurrency.to_i
            raise ArgumentError, 'invalid max_concurrency' if max_concurrency <= 0
            params[:max_concurrency] = max_concurrency
          end
          params
        end

        OnUploadingProgressCallback = proc do |uploaded, total, idx|
          begin
            context = CallbackData.get(idx)
            if context
              context[:on_uploading_progress]&.call(uploaded, total)
              CallbackData.delete(idx) if uploaded == total
            end
          rescue Exception => e
            Config::CallbackExceptionHandler.call(e)
          end
        end
        private_constant :OnUploadingProgressCallback
      end
      private_constant :SingleFileUploaderHelper
    end
  end
end
