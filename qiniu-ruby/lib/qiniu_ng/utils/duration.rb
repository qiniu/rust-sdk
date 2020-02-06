# frozen_string_literal: true

require 'digest'

module QiniuNg
  module Utils
    # 持续时间工具
    #
    # @example
    #   QiniuNg::Duration.new(days: 2, hours: 6)
    #
    # @!attribute [r] total
    #   @return [Integer] 总持续时间，单位为秒
    # @!attribute [r] seconds
    #   @return [Integer] 秒
    # @!attribute [r] minutes
    #   @return [Integer] 分钟
    # @!attribute [r] hours
    #   @return [Integer] 小时
    # @!attribute [r] days
    #   @return [Integer] 天
    # @!attribute [r] weeks
    #   @return [Integer] 周
    class Duration
      # @!visibility private
      MULTIPLES = {
        seconds: 1,
        minutes: 60,
        hours: 3_600,
        days: 86_400,
        weeks: 604_800,
        second: 1,
        minute: 60,
        hour: 3_600,
        day: 86_400,
        week: 604_800
      }.freeze
      # @!visibility private
      UNITS = %i[seconds minutes hours days weeks].freeze
      # @!visibility private
      attr_reader :total, :seconds, :minutes, :hours, :days, :weeks
      alias to_i total

      # 构造方法
      #
      # @example
      #   QiniuNg::Duration.new(days: 2, hours: 6)
      #
      # @param [Hash, Integer] args 支持语义化 Hash 参数或直接输入数字，单位为秒
      # @option args [Integer] :seconds 秒
      # @option args [Integer] :second 秒
      # @option args [Integer] :minutes 分钟
      # @option args [Integer] :minute 分钟
      # @option args [Integer] :hours 小时
      # @option args [Integer] :hour 小时
      # @option args [Integer] :days 天
      # @option args [Integer] :day 天
      # @option args [Integer] :weeks 周
      # @option args [Integer] :week 周
      def initialize(args = 0)
        if args.is_a?(Hash)
          @seconds = 0
          MULTIPLES.each do |unit, multiple|
            unit = unit.to_sym
            @seconds += args[unit] * multiple if args.key?(unit)
          end
        else
          @seconds = args.to_i
        end
        calculate!
      end

      # @!visibility private
      def inspect
        "#<#{self.class}: #{@total} seconds>"
      end

      # 持续时间相加
      #
      # @param [Integer, QiniuNg::Duration] other 增加的持续时间
      def +(other)
        Duration.new(@total + other.to_i)
      end

      # 持续时间相减
      #
      # @param [Integer, QiniuNg::Duration] other 减少的持续时间
      def -(other)
        Duration.new(@total - other.to_i)
      end

      # 持续时间相乘
      #
      # @param [Integer, QiniuNg::Duration] other 倍数
      def *(other)
        Duration.new(@total * other.to_i)
      end

      # 持续时间相除
      #
      # @param [Integer, QiniuNg::Duration] other 除数
      def /(other)
        Duration.new(@total / other.to_i)
      end

      # 持续时间取模
      #
      # @param [Integer, QiniuNg::Duration] other 除数
      def %(other)
        Duration.new(@total % other.to_i)
      end

      private

      def calculate!
        multiples = [MULTIPLES[:weeks], MULTIPLES[:days], MULTIPLES[:hours], MULTIPLES[:minutes], MULTIPLES[:seconds]]
        units     = []
        @total    = @seconds.to_f.round
        multiples.inject(@total) do |total, multiple|
          units << total / multiple
          total % multiple
        end

        @weeks, @days, @hours, @minutes, @seconds = units
      end
    end
  end
end
