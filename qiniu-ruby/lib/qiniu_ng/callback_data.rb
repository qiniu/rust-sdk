# frozen_string_literal: true

require 'ffi'
require 'concurrent-ruby'
require 'thread'

module QiniuNg
  module CallbackData
    Map = Concurrent::Map.new
    private_constant :Map

    def self.put(object)
      FFI::Pointer.new((Time.now.to_i..(2**64-1)).find { |idx| Map.put_if_absent(idx, object).nil? })
    end

    def self.get(ptr)
      Map[ptr.to_i]
    end

    def self.delete(ptr)
      Map.delete(ptr.to_i)
    end
  end

  private_constant :CallbackData
end
