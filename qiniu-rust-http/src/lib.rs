//! 七牛 SDK HTTP 接口
//!
//! 七牛 SDK 本身并不包含 HTTP 客户端的实现，而是利用本库实现的 HTTP 接口实现了与 HTTP 客户端的解耦合。任何依赖本库，实现了本库的接口的 HTTP 实现均可作为 SDK 的 HTTP 客户端实现。
//!
//! ### 重试策略，幂等性与重试安全之间的关系
//!
//! 当多次发送同一个 HTTP 请求具有相同效果，则认为该 HTTP 请求具有幂等性。
//! 根据 HTTP RESTful API 的设计理念，GET / PUT / HEAD / PATCH / DELETE 请求被认为总是幂等的，
//! 而对于非幂等的 HTTP 方法，可以手动设置 idempotent 属性将其设置为幂等。
//! 如果一个幂等的 HTTP 请求发生了错误，则总是被认为是重试安全的。
//!
//! 对于非幂等的 HTTP 请求发生了错误，则是否重试安全取决于该错误发生时，请求是否已经完整发出。
//! 如果错误发生在域名解析阶段，连接阶段或请求发送期间，则一般认为是重试安全的，
//! 如果请求已经完整发出，则之后发生的错误是重试不安全的。
//!
//! 对于每一种 HTTP 请求时发生的错误，都应该设置其重试策略，
//! 以便于外部 HTTP 客户端检测到错误时，决定将其重试，还是切换主机后再重试，还是不予重试直接抛出错误。
//! 重试策略共四种，已经在 [`RetryKind`](enum.RetryKind.html) 枚举类中列出。
//! 一般来说，如果是由于 HTTP 请求自身格式存在问题，或是收到了来自服务器的 4xx 响应，则被认为是不可重试的，
//! 而如果是服务器发生异常，或域名解析无法成功，或无法连接到，或 SSL 认证无法通过，则被认为只是当前主机不可重试，可以切换到其他主机再重试。
//! 如果只是连接超时，或是发送接受数据存在超时或突然中断，则被认为是可重试的。
//!
//! 即使根据重试策略需要重试的错误，客户端也未必一定会重试。
//! 这取决于两个因素，是否重试安全和是否已经达到重试次数上限。
//! 因此，对于容易发生错误的请求（例如上传下载之类的），要尽可能将其 HTTP 调用设置为幂等，
//! 否则就可能因为发生错误的时机不佳而无法重试。

mod error;
mod header;
mod method;
mod request;
mod response;
pub use error::{Error, ErrorKind, HTTPCallerError, HTTPCallerErrorKind, Result, RetryKind};
pub use header::{HeaderName, HeaderValue, Headers};
pub use method::Method;
pub use request::{Body as RequestBody, ProgressCallback, Request, RequestBuilder, URL};
pub use response::{Body as ResponseBody, Response, ResponseBuilder, StatusCode};

/// HTTP 请求处理函数
///
/// 实现该接口，即可处理所有七牛 SDK 发送的 HTTP 请求
pub trait HTTPCaller: Send + Sync {
    fn call(&self, request: &Request) -> Result<Response>;
}
