# frozen_string_literal: true

require 'ffi'

module QiniuNg
  module Storage
    class Uploader
      class BucketUploader
        def initialize(bucket_uploader_ffi)
          @bucket_uploader = bucket_uploader_ffi
        end
        private_class_method :new

        def self.new_from_bucket(upload_manager, bucket, thread_pool_size)
          raise ArgumentError, 'upload_manager must be instance of Uploader' unless upload_manager.is_a?(Uploader)
          raise ArgumentError, 'bucket must be instance of Bucket' unless bucket.is_a?(Bucket)
          upload_manager = upload_manager.instance_variable_get(:@upload_manager)
          bucket = bucket.instance_variable_get(:@bucket)
          new(upload_manager.new_bucket_uploader_from_bucket(bucket, thread_pool_size.to_i))
        end
        private_class_method :new_from_bucket

        def self.new_from_bucket_name(upload_manager, bucket_name, access_key, thread_pool_size)
          raise ArgumentError, 'upload_manager must be instance of Uploader' unless upload_manager.is_a?(Uploader)
          upload_manager = upload_manager.instance_variable_get(:@upload_manager)
          new(upload_manager.new_bucket_uploader_from_bucket_name(bucket_name.to_s, access_key.to_s, thread_pool_size.to_i))
        end
        private_class_method :new_from_bucket_name

        def upload_file(file, upload_token:, key: nil,
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
          upload_token = normalize_upload_token(upload_token)
          params = create_upload_params(key: key,
                                        file_name: file_name,
                                        mime: mime,
                                        vars: vars,
                                        metadata: metadata,
                                        checksum_enabled: checksum_enabled,
                                        resumable_policy: resumable_policy,
                                        on_uploading_progress: on_uploading_progress,
                                        upload_threshold: upload_threshold,
                                        thread_pool_size: thread_pool_size,
                                        max_concurrency: max_concurrency)
          reader = Bindings.const_get(:CoreFFI)::QiniuNgReadableT.new
          reader[:context] = nil
          reader[:read_func] = proc do |_, data, size, have_read|
                                 content = file.read(size)
                                 if content.nil?
                                   have_read.write_ulong(0)
                                 else
                                   data.write_string(content)
                                   have_read.write_ulong(content.bytesize)
                                 end
                                 true
                               end
          upload_response = QiniuNg::Error.wrap_ffi_function do
                              @bucket_uploader.upload_reader(
                                upload_token.instance_variable_get(:@upload_token),
                                reader,
                                params)
                            end
          UploadResponse.send(:new, upload_response)
        end
        alias upload_io upload_file

        def upload_file_path(file_path, upload_token:, key: nil,
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
          upload_token = normalize_upload_token(upload_token)
          params = create_upload_params(key: key,
                                        file_name: file_name,
                                        mime: mime,
                                        vars: vars,
                                        metadata: metadata,
                                        checksum_enabled: checksum_enabled,
                                        resumable_policy: resumable_policy,
                                        on_uploading_progress: on_uploading_progress,
                                        upload_threshold: upload_threshold,
                                        thread_pool_size: thread_pool_size,
                                        max_concurrency: max_concurrency)
          upload_response = QiniuNg::Error.wrap_ffi_function do
                              @bucket_uploader.upload_file_path(
                                upload_token.instance_variable_get(:@upload_token),
                                file_path,
                                params)
                            end
          UploadResponse.send(:new, upload_response)
        end

        private

        def normalize_upload_token(upload_token)
          upload_token = UploadToken.from_token(upload_token) if upload_token.is_a?(String)
          raise ArgumentError, 'upload_token must be instance of UploadToken' unless upload_token.is_a?(UploadToken)
          upload_token
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
          core_ffi = Bindings.const_get(:CoreFFI)
          params = core_ffi::QiniuNgUploadParamsT.new
          params[:key] = FFI::MemoryPointer.from_string(key.to_s) unless key.nil?
          params[:file_name] = FFI::MemoryPointer.from_string(file_name.to_s) unless file_name.nil?
          params[:mime] = FFI::MemoryPointer.from_string(mime.to_s) unless mime.nil?
          params[:vars] = create_str_map(vars).instance unless vars.nil?
          params[:metadata] = create_str_map(metadata).instance unless metadata.nil?
          params[:checksum_enabled] = !!checksum_enabled unless checksum_enabled.nil?
          params[:resumable_policy] = normalize_resumable_policy(resumable_policy) unless resumable_policy.nil?
          unless on_uploading_progress.nil?
            params[:on_uploading_progress] = proc do |uploaded, total|
                                               on_uploading_progress.(uploaded, total)
                                             end
          end
          params[:upload_threshold] = upload_threshold.to_i unless upload_threshold.nil?
          params[:thread_pool_size] = thread_pool_size.to_i unless thread_pool_size.nil?
          params[:max_concurrency] = max_concurrency.to_i unless max_concurrency.nil?
          params
        end

        def create_str_map(hash)
          hash.each_with_object(Bindings::StrMap.new!(hash.size)) do |(key, value), strmap|
            strmap.set(key.to_s, value.to_s)
          end
        end

        def normalize_resumable_policy(resumable_policy)
          case resumable_policy
          when :default then :qiniu_ng_resumable_policy_default
          when :threshold then :qiniu_ng_resumable_policy_threshold
          when :always_be_resumeable then :qiniu_ng_resumable_policy_always_be_resumeable
          when :never_be_resumeable then :qiniu_ng_resumable_policy_never_be_resumeable
          else
            raise ArgumentError, "invalid resumable policy: #{resumable_policy.inspect}"
          end
        end

        def inspect
          "#<#{self.class.name}>"
        end
      end
    end
  end
end
