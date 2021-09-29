use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(SmartDefault, Clone, Debug, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// HTTP 方法
pub(crate) enum Method {
    #[default]
    Get,
    Post,
    Put,
    Delete,
}

#[derive(SmartDefault, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 七牛服务名称
pub(crate) enum ServiceName {
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
pub(crate) enum Authorization {
    /// 使用凭证鉴权
    Credential,

    /// 使用上传凭证鉴权
    UploadToken,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 字符串编码类型
pub(crate) enum EncodeType {
    /// 需要进行编码
    UrlSafeBase64,

    /// 需要可以将 None 编码
    UrlSafeBase64OrNone,
}

#[derive(SmartDefault, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// JSON 字段类型
pub(crate) enum JsonType {
    /// 字符串（默认）
    #[default]
    String,

    /// 整型数字
    Integer,

    /// 浮点型数字
    Float,

    /// 布尔值
    Boolean,

    /// 数组
    Array(Box<JsonArray>),

    /// 结构体
    Struct(JsonStruct),

    /// 任意数据结构
    Any,

    /// 任意字符串映射结构
    StringMap,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// JSON 数组字段信息
pub(crate) struct JsonArray {
    /// JSON 数组类型
    #[serde(rename = "type")]
    pub(crate) ty: JsonType,

    /// JSON 数组参数是否可选
    pub(crate) optional: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// JSON 结构体字段
pub(crate) struct JsonField {
    /// JSON 字段类型
    #[serde(rename = "type")]
    pub(crate) ty: JsonType,

    /// JSON 字段参数名称
    pub(crate) key: String,

    /// JSON 字段名称
    pub(crate) field_name: String,

    /// JSON 字段参数文档
    pub(crate) documentation: String,

    /// JSON 字段参数是否可选
    pub(crate) optional: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// JSON 结构体
pub(crate) struct JsonStruct {
    /// JSON 字段列表
    pub(crate) fields: Vec<JsonField>,

    /// JSON 结构体参数文档
    pub(crate) documentation: String,

    /// JSON 结构体参数是否可选
    pub(crate) optional: bool,
}

#[derive(SmartDefault, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 复合表单字段请求类型
pub(crate) enum MultipartFormDataRequestType {
    /// 字符串（默认）
    #[default]
    String,

    /// 二进制数据
    BinaryData,

    /// 使用上传凭证鉴权
    UploadToken,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// 有名复合表单请求字段
pub(crate) struct NamedMultipartFormDataRequestField {
    /// 复合表单字段名称
    pub(crate) field_name: String,

    /// 支持传入复合表单文件名称
    pub(crate) file_name: bool,

    /// 支持传入复合表单内容 MIME 类型
    pub(crate) content_type: bool,

    /// 复合表单参数名称
    pub(crate) key: String,

    /// 复合表单参数文档
    pub(crate) documentation: String,

    /// 复合表单参数类型
    #[serde(rename = "type")]
    pub(crate) ty: MultipartFormDataRequestType,

    /// 复合表单参数是否可选
    pub(crate) optional: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// 自由复合表单请求字段
pub(crate) struct FreeMultipartFormDataRequestFields {
    /// 复合表单参数名称
    pub(crate) field_name: String,

    /// 复合表单参数文档
    pub(crate) documentation: String,

    /// 复合表单参数类型
    #[serde(rename = "type")]
    pub(crate) ty: MultipartFormDataRequestType,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// 复合表单请求结构体
pub(crate) struct MultipartFormDataRequestStruct {
    /// 有名复合表单字段列表
    pub(crate) named_fields: Vec<NamedMultipartFormDataRequestField>,

    /// 自由复合表单字段列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) free_fields: Option<FreeMultipartFormDataRequestFields>,
}

#[derive(SmartDefault, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 类字符串参数类型
pub(crate) enum StringLikeType {
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
pub(crate) struct FormUrlencodedRequestField {
    /// URL 编码表单字段名称
    pub(crate) field_name: String,

    /// URL 编码表单参数名称
    pub(crate) key: String,

    /// URL 编码表单参数文档
    pub(crate) documentation: String,

    /// URL 编码表单参数类型
    #[serde(rename = "type")]
    pub(crate) ty: StringLikeType,

    /// URL 编码表单参数是否可以有多个值
    pub(crate) multiple: bool,

    /// URL 编码表单参数是否可选
    pub(crate) optional: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default, rename_all = "snake_case")]
/// URL 编码表单请求结构体
pub(crate) struct FormUrlencodedRequestStruct {
    /// URL 编码表单字段列表
    pub(crate) fields: Vec<FormUrlencodedRequestField>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// HTTP 调用请求体
pub(crate) enum RequestBody {
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
pub(crate) struct PathParams {
    /// HTTP URL 路径有名参数列表
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) named: Vec<NamedPathParam>,

    /// HTTP URL 路径自由参数列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) free: Option<FreePathParams>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 路径有名请求参数
pub(crate) struct NamedPathParam {
    /// HTTP URL 路径段落，如果为 None，则表示参数直接追加在 URL 路径末尾
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path_segment: Option<String>,

    /// HTTP URL 路径参数名称
    pub(crate) field_name: String,

    /// HTTP URL 路径参数类型
    #[serde(rename = "type")]
    pub(crate) ty: StringLikeType,

    /// HTTP URL 路径参数文档
    pub(crate) documentation: String,

    /// HTTP URL 路径参数编码方式，如果为 None，表示直接转码成字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) encode: Option<EncodeType>,

    /// HTTP URL 路径参数是否可选
    pub(crate) optional: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 路径自由请求参数
pub(crate) struct FreePathParams {
    /// HTTP URL 路径参数名称
    pub(crate) field_name: String,

    /// HTTP URL 路径参数类型
    #[serde(rename = "type")]
    pub(crate) ty: StringLikeType,

    /// HTTP URL 路径参数文档
    pub(crate) documentation: String,

    /// HTTP URL 路径参数键编码方式，如果为 None，表示直接转码成字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) encode_param_key: Option<EncodeType>,

    /// HTTP URL 路径参数值编码方式，如果为 None，表示直接转码成字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) encode_param_value: Option<EncodeType>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP 头参数信息
pub struct HeaderName {
    /// HTTP 头参数名称
    pub(crate) field_name: String,

    /// HTTP 头名称
    pub(crate) header_name: String,

    /// HTTP 头参数文档
    pub(crate) documentation: String,

    /// HTTP 头参数是否可选
    pub(crate) optional: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// HTTP URL 查询请求参数信息
pub struct QueryName {
    /// 参数名称
    pub(crate) field_name: String,

    /// HTTP URL 查询参数名称
    pub(crate) query_name: String,

    /// HTTP URL 查询参数文档
    pub(crate) documentation: String,

    /// HTTP URL 查询参数类型
    pub(crate) query_type: StringLikeType,

    /// HTTP URL 查询参数是否可选
    pub(crate) optional: bool,
}

#[derive(Clone, Debug)]
/// API 描述，仅在内存中存储
pub(crate) struct ApiDescription {
    /// API 名称，通过 YAML 描述文件的文件名称来获取
    pub(crate) name: String,

    /// API 名字空间，通过 YAML 描述文件所在路径来获取
    pub(crate) namespace: Vec<String>,

    /// API 描述信息，通过 YAML 描述文件的内容来获取
    pub(crate) details: ApiDetailedDescription,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// API 描述信息，可以通过 YAML 描述文件编辑
pub(crate) struct ApiDetailedDescription {
    /// API 调用 HTTP 方法
    pub(crate) method: Method,

    /// 七牛服务名称，可以设置多个，表现有多个七牛服务都可以调用该 API
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) service_names: Vec<ServiceName>,

    /// API 文档
    pub(crate) documentation: String,

    /// 七牛 API URL 基础路径
    pub(crate) base_path: String,

    /// 七牛 API URL 路径后缀
    pub(crate) path_suffix: String,

    /// 七牛 API 调用参数
    pub(crate) request: ApiRequestDescription,

    /// 七牛 API 响应参数
    pub(crate) response: ApiResponseDescription,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct ApiRequestDescription {
    /// 七牛 API 调用 URL 路径参数列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path_params: Option<PathParams>,

    /// 七牛 API 调用 HTTP 头参数列表
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) header_names: Vec<HeaderName>,

    /// 七牛 API 调用 URL 查询参数列表
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) query_names: Vec<QueryName>,

    /// 七牛 API 调用请求体
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) body: Option<RequestBody>,

    /// 七牛 API 调用鉴权参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) authorization: Option<Authorization>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// HTTP 响应请求体
pub(crate) enum ResponseBody {
    /// JSON 响应
    Json(JsonType),

    /// 二进制数据响应
    BinaryDataStream,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// JSON 响应结构体
pub(crate) struct JsonResponseStruct {
    /// JSON 字段列表
    pub(crate) fields: Vec<JsonResponseField>,

    /// JSON 结构体文档
    pub(crate) documentation: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// JSON 响应字段基础信息
pub(crate) struct JsonResponseBaseField {
    /// JSON 字段名称
    pub(crate) field_name: String,

    /// JSON 字段文档
    pub(crate) documentation: String,

    /// JSON 字段是否可选
    pub(crate) optional: bool,

    /// JSON 字段类型
    #[serde(rename = "type")]
    pub(crate) ty: JsonResponseType,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
/// JSON 响应字段
pub(crate) struct JsonResponseField {
    /// JSON 字段路径
    pub(crate) key_path: Vec<String>,

    /// JSON 字段基础信息
    #[serde(flatten)]
    pub(crate) info: JsonResponseBaseField,
}

#[derive(SmartDefault, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// JSON 响应类型
pub(crate) enum JsonResponseType {
    /// 字符串（默认）
    #[default]
    String,

    /// 整型数字
    Integer,

    /// 浮点型数字
    Float,

    /// 布尔值
    Boolean,

    /// 对象
    Object,

    /// 整型数组
    IntegerArray,

    /// 字符串数组
    StringArray,

    /// 对象数组
    ObjectArray,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct ApiResponseDescription {
    /// 七牛 API 响应 HTTP 头参数列表
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) header_names: Vec<HeaderName>,

    /// 七牛 API 响应请求体
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) body: Option<ResponseBody>,
}
