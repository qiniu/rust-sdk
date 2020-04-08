# frozen_string_literal: true

require 'ffi'

module QiniuNg
  module Storage
    class Uploader
      module SingleFileUploaderHelper
        include UploaderHelper

        private

        def check_upload_params!(method_name, bucket_name, credential, upload_policy, upload_token)
          method_name = case method_name
                        when :file then :upload_file
                        when :file_path then :upload_file_path
                        when :reader then :upload_reader
                        else
                          raise ArgumentError, 'Invalid method_name'
                        end
          case
          when upload_token
            upload_token = normalize_upload_token(upload_token)
            [:"#{method_name}_via_upload_token", upload_token.instance_variable_get(:@upload_token)]
          when bucket_name && credential
            raise ArgumentError, 'credential must be instance of Credential' unless credential.is_a?(Credential)
            [method_name, bucket_name.to_s, credential.instance_variable_get(:@credential)]
          when upload_policy && credential
            raise ArgumentError, 'upload_policy must be instance of UploadPolicy' unless upload_policy.is_a?(UploadPolicy)
            raise ArgumentError, 'credential must be instance of Credential' unless credential.is_a?(Credential)
            [:"#{method_name}_via_upload_policy", upload_policy.instance_variable_get(:@upload_policy), credential.instance_variable_get(:@credential)]
          when !credential
            raise ArgumentError, 'credential must be specified'
          else
            raise ArgumentError, 'either bucket_name or upload_policy must be specified'
          end
        end

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

        def clear_upload_params(params)
          callback_data = params[:callback_data]
          CallbackData.delete(callback_data) if callback_data
        end

        OnUploadingProgressCallback = proc do |uploaded, total, idx|
          begin
            context = CallbackData.get(idx)
            context[:on_uploading_progress]&.call(uploaded, total) if context
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
