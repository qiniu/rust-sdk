# 如何运行集成测试

## 1. 设置七牛账户

### 方法一

在当前目录中放置 `.env` 文件，内容如下：

```bash
access_key=[access_key]
secret_key=[secret_key]
z2_encrypt_key=[z2_encrypt_key]
```

其中 `z2_encrypt_key` 为 z2-bucket 的时间戳鉴权密钥（见 2.5）

### 方法二

设置环境变量 `access_key`，`secret_key` 和 `z2_encrypt_key`。

## 2. 配置七牛账户

1. 创建以下存储空间

- 华东区 `z0-bucket`，公开空间。
- 华北区 `z1-bucket`，私有空间。
- 华北区 `z0-bucket-bind`，公开空间。
- 华南区 `z2-bucket`，公开空间。
- 新加坡地区 `as-bucket`，公开空间。
- 北美地区 `na-bucket`，公开空间。

2. 将存储空间 `z0-bucket-bind` 配置为 `z0-bucket` 的双活空间。
3. 为 z0-bucket 绑定至少一个 CDN 域名
4. 为 z1-bucket 绑定至少一个 CDN 域名，并且设置回源鉴权
5. 为 z2-bucket 绑定至少一个 CDN 域名，并且设置时间戳鉴权
