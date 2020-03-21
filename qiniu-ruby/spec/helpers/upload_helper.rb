# frozen_string_literal: true

module Helpers
  module UploadHelper
    def upload_bucket_name
      if ENV['USE_NA_BUCKET'].nil?
        'na-bucket'
      else
        'z0-bucket'
      end
    end
  end
end
