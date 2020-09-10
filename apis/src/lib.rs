mod regions;

pub use regions::{Region, RegionBuilder, RegionProvider};

// TODO: 替换成真正的 API 错误
pub type APIError = std::io::Error;
pub type APIResult<T> = std::io::Result<T>;
