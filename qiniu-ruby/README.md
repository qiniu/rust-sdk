# Qiniu SDK for Ruby

[![License](https://img.shields.io/badge/license-Apache%202-blue)](https://github.com/bachue/rust-sdk/blob/master/LICENSE)[![Build Status](https://api.travis-ci.com/bachue/rust-sdk.svg?branch=master)](https://travis-ci.org/bachue/rust-sdk)

## 关于

此 Ruby SDK 基于 [七牛云官方 API](http://developer.qiniu.com/) 构建。
使用此 SDK 构建您的网络应用程序，能让您以非常便捷地方式将数据安全地存储到七牛云上。
无论您的网络应用是一个网站程序，还是包括从云端（服务端程序）到终端（手持设备应用）的架构的服务或应用，通过七牛云及其 SDK，都能让您应用程序的终端用户高速上传和下载，同时也让您的服务端更加轻盈。

## 兼容平台

- Linux
- MacOS

## 依赖环境

- Linux: Ruby 2.4.0+ 或 JRuby 9.2.0.0+, Rust 1.38+, cbindgen, GCC, libcurl
- MacOS: Ruby 2.4.0+ 或 JRuby 9.2.0.0+, Rust 1.38+, cbindgen, Clang（from XCode）, libcurl

## 接口文档

[点击进入](https://bachue.github.io/rust-sdk/doc/qiniu_ng_ruby/)

## 安装指南

添加这行代码到应用的 Gemfile:

```ruby
gem 'qiniu_ng'
```

然后执行

```bash
bundle install
```

或采用如下安装命令

```bash
gem install qiniu_ng
```

## 构建指南

### 构建 gem

```bash
make
```

### 生成 API 文档

```bash
make doc
```

### 执行集成测试（需要配置七牛账户，具体做法参见 [TEST.md](https://github.com/bachue/rust-sdk/blob/master/TEST.md)）

```bash
make test
```

### 删除构建结果

```bash
make clean
```

## 贡献代码

1. Fork
2. 创建您的特性分支 (`git checkout -b my-new-feature`)
3. 提交您的改动 (`git commit -am 'Added some feature'`)
4. 将您的修改记录提交到远程 `git` 仓库 (`git push origin my-new-feature`)
5. 然后到 github 网站的该 `git` 远程仓库的 `my-new-feature` 分支下发起 Pull Request

## 许可证

Copyright (c) 2012-2020 qiniu.com

基于 Apache 2.0 协议发布:

* [opensource.org/licenses/Apache-2.0](https://opensource.org/licenses/Apache-2.0)
