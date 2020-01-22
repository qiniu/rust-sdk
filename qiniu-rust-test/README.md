# Qiniu Rust SDK Test

[![License](https://img.shields.io/badge/license-Apache%202-blue)](https://github.com/bachue/rust-sdk/blob/master/LICENSE)[![Build Status](https://api.travis-ci.com/bachue/rust-sdk.svg?branch=master)](https://travis-ci.org/bachue/rust-sdk)

## 关于

本模块针对 `qiniu-rust` 的 `use-libcurl` 功能进行集成测试。
在测试前，应安装 libcurl，根据 [../TEST.md](../TEST.md) 配置七牛账户，并保证网络正常。

## 构建指南

### 构建集成测试程序

```bash
make build_test
```

### 执行集成测试（需要配置七牛账户，具体做法参见 [../TEST.md](../TEST.md)）

```bash
make test
```

### 检查 Rust 代码质量

```bash
make clippy
```
