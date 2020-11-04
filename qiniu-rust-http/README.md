# Qiniu Rust SDK HTTP Interfaces

[![License](https://img.shields.io/badge/license-Apache%202-blue)](https://github.com/bachue/rust-sdk/blob/master/LICENSE)
[![Build Status](https://api.travis-ci.com/bachue/rust-sdk.svg?branch=master)](https://travis-ci.org/bachue/rust-sdk)

## 关于

本模块定义了 `qiniu-rust` 所使用的 HTTP 客户端接口，并提供 HTTP 请求 / 响应 / 错误类的封装。

## 接口文档

[点击进入](https://bachue.github.io/rust-sdk/qiniu_http/)

## 构建指南

### 构建库的开发版

```bash
make
```

### 执行单元测试（不依赖七牛服务器即可执行，不需要配置七牛账户）

```bash
make test
```

### 检查 Rust 代码质量

```bash
make clippy
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
