use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// HTTP 方法
pub(super) enum Method {
    #[default]
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 七牛服务名称
pub(super) enum ServiceName {
    #[default]
    Up,
    Io,
    Uc,
    Rs,
    Rsf,
    Api,
    S3,
}

#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Idempotent {
    Always,

    #[default]
    Default,

    Never,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 鉴权方式
pub(super) enum Authorization {
    /// 使用 QBox 凭证鉴权
    Qbox,

    /// 使用 Qiniu 凭证鉴权
    Qiniu,

    /// 使用上传凭证鉴权
    UploadToken,
}

#[derive(Default, Copy, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 类字符串参数类型
pub(super) enum StringLikeType {
    /// 字符串（默认）
    #[default]
    String,

    /// 整型数字
    Integer,

    /// 浮点型数字
    Float,

    /// 布尔值
    Boolean,
}
