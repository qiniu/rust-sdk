# 如何运行集成测试

## 1. 设置七牛账户

### 方法一

在当前目录中放置 `.env` 文件，内容如下：

```bash
access_key=[access_key]
secret_key=[secret_key]
z2_encrypt_key=[z2_encrypt_key]
```

### 方法二

设置环境变量 `access_key`，`secret_key` 和 `z2_encrypt_key`。

## 2. 配置七牛账户

1. 创建以下存储空间

- 华东区 `z0-bucket`
- 华北区 `z1-bucket`
- 华北区 `z0-bucket-bind`。
- 华南区 `z2-bucket`
- 新加坡地区 `as-bucket`
- 北美地区 `na0-bucket`

2. 将存储空间 `z0-bucket-bind` 配置为 `z0-bucket` 的双活空间。
