use super::{json::JsonType, multipart::MultipartFormDataRequestStruct};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(SmartDefault, Clone, Debug, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// HTTP 方法
enum Method {
    #[default]
    Get,
    Post,
    Put,
    Delete,
}

#[derive(SmartDefault, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 七牛服务名称
enum ServiceName {
    #[default]
    Up,
    Io,
    Uc,
    Rs,
    Rsf,
    Api,
    S3,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 鉴权方式
enum Authorization {
    /// 使用凭证鉴权
    Credential,

    /// 使用上传凭证鉴权
    UploadToken,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 字符串编码类型
enum EncodeType {
    /// 需要进行编码
    UrlSafeBase64,

    /// 需要可以将 None 编码
    UrlSafeBase64OrNone,
}

#[derive(SmartDefault, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 类字符串参数类型
enum StringLikeType {
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// URL 编码表单请求字段
struct FormUrlencodedRequestField {
    /// URL 编码表单字段名称
    field_name: String,

    /// URL 编码表单参数名称
    key: String,

    /// URL 编码表单参数文档
    documentation: String,

    /// URL 编码表单参数类型
    #[serde(rename = "type")]
    ty: StringLikeType,

    /// URL 编码表单参数是否可以有多个值
    multiple: bool,

    /// URL 编码表单参数是否可选
    optional: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default, rename_all = "snake_case")]
/// URL 编码表单请求结构体
struct FormUrlencodedRequestStruct {
    /// URL 编码表单字段列表
    fields: Vec<FormUrlencodedRequestField>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// HTTP 调用请求体
enum RequestBody {
    /// JSON 调用
    Json(JsonType),

    /// URL 编码表单调用（无法上传二进制数据）
    FormUrlencoded(FormUrlencodedRequestStruct),

    /// 复合表单调用（可以上传二进制数据）
    MultipartFormData(MultipartFormDataRequestStruct),

    /// 二进制数据调用
    BinaryData,

    /// 文本文件调用
    PlainText,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 路径请求参数列表
struct PathParams {
    /// HTTP URL 路径有名参数列表
    #[serde(skip_serializing_if = "Vec::is_empty")]
    named: Vec<NamedPathParam>,

    /// HTTP URL 路径自由参数列表
    #[serde(skip_serializing_if = "Option::is_none")]
    free: Option<FreePathParams>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 路径有名请求参数
struct NamedPathParam {
    /// HTTP URL 路径段落，如果为 None，则表示参数直接追加在 URL 路径末尾
    #[serde(skip_serializing_if = "Option::is_none")]
    path_segment: Option<String>,

    /// HTTP URL 路径参数名称
    field_name: String,

    /// HTTP URL 路径参数类型
    #[serde(rename = "type")]
    ty: StringLikeType,

    /// HTTP URL 路径参数文档
    documentation: String,

    /// HTTP URL 路径参数编码方式，如果为 None，表示直接转码成字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    encode: Option<EncodeType>,

    /// HTTP URL 路径参数是否可选
    optional: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 路径自由请求参数
struct FreePathParams {
    /// HTTP URL 路径参数名称
    field_name: String,

    /// HTTP URL 路径参数类型
    #[serde(rename = "type")]
    ty: StringLikeType,

    /// HTTP URL 路径参数文档
    documentation: String,

    /// HTTP URL 路径参数键编码方式，如果为 None，表示直接转码成字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    encode_param_key: Option<EncodeType>,

    /// HTTP URL 路径参数值编码方式，如果为 None，表示直接转码成字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    encode_param_value: Option<EncodeType>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP 头参数信息
pub struct HeaderName {
    /// HTTP 头参数名称
    field_name: String,

    /// HTTP 头名称
    header_name: String,

    /// HTTP 头参数文档
    documentation: String,

    /// HTTP 头参数是否可选
    optional: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 查询请求参数信息
pub struct QueryName {
    /// 参数名称
    field_name: String,

    /// HTTP URL 查询参数名称
    query_name: String,

    /// HTTP URL 查询参数文档
    documentation: String,

    /// HTTP URL 查询参数类型
    query_type: StringLikeType,

    /// HTTP URL 查询参数是否可选
    optional: bool,
}

#[derive(Clone, Debug)]
/// API 描述，仅在内存中存储
pub(super) struct ApiDescription {
    /// API 名称，通过 YAML 描述文件的文件名称来获取
    name: String,

    /// API 名字空间，通过 YAML 描述文件所在路径来获取
    namespace: Vec<String>,

    /// API 描述信息，通过 YAML 描述文件的内容来获取
    details: ApiDetailedDescription,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// API 描述信息，可以通过 YAML 描述文件编辑
pub(super) struct ApiDetailedDescription {
    /// API 调用 HTTP 方法
    method: Method,

    /// 七牛服务名称，可以设置多个，表现有多个七牛服务都可以调用该 API
    #[serde(skip_serializing_if = "Vec::is_empty")]
    service_names: Vec<ServiceName>,

    /// API 文档
    documentation: String,

    /// 七牛 API URL 基础路径
    base_path: String,

    /// 七牛 API URL 路径后缀
    path_suffix: String,

    /// 七牛 API 调用参数
    request: ApiRequestDescription,

    /// 七牛 API 响应参数
    response: ApiResponseDescription,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct ApiRequestDescription {
    /// 七牛 API 调用 URL 路径参数列表
    #[serde(skip_serializing_if = "Option::is_none")]
    path_params: Option<PathParams>,

    /// 七牛 API 调用 HTTP 头参数列表
    #[serde(skip_serializing_if = "Vec::is_empty")]
    header_names: Vec<HeaderName>,

    /// 七牛 API 调用 URL 查询参数列表
    #[serde(skip_serializing_if = "Vec::is_empty")]
    query_names: Vec<QueryName>,

    /// 七牛 API 调用请求体
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<RequestBody>,

    /// 七牛 API 调用鉴权参数
    #[serde(skip_serializing_if = "Option::is_none")]
    authorization: Option<Authorization>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// HTTP 响应请求体
enum ResponseBody {
    /// JSON 响应
    Json(JsonType),

    /// 二进制数据响应
    BinaryDataStream,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct ApiResponseDescription {
    /// 七牛 API 响应 HTTP 头参数列表
    #[serde(skip_serializing_if = "Vec::is_empty")]
    header_names: Vec<HeaderName>,

    /// 七牛 API 响应请求体
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<ResponseBody>,
}
