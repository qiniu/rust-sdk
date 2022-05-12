#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![deny(
    single_use_lifetimes,
    missing_debug_implementations,
    large_assignments,
    exported_private_dependencies,
    absolute_paths_not_starting_with_crate,
    anonymous_parameters,
    explicit_outlives_requirements,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_docs,
    non_ascii_idents,
    indirect_structural_match,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications
)]

//! # qiniu-ureq
//!
//! ## 七牛 Ureq HTTP 客户端实现
//!
//! 基于 Ureq 库提供 HTTP 客户端接口实现，仅提供阻塞接口的实现，不提供异步接口的实现。

mod client;
mod extensions;

pub use client::Client;
pub use extensions::*;

pub use qiniu_http as http;
pub use ureq;

#[cfg(feature = "async")]
use std::{future::Future, pin::Pin};

#[cfg(feature = "async")]
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a + Send>>;

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use futures::channel::oneshot::channel;
    use md5::{Digest, Md5};
    use qiniu_http::HttpCaller;
    use qiniu_http::{
        header::{CONTENT_LENGTH, USER_AGENT},
        Method, OnProgressCallback, SyncRequest, SyncRequestBody, TransferProgressInfo,
    };
    use rand::{thread_rng, RngCore};
    use std::{
        io::{copy as io_copy, Read},
        sync::{
            atomic::{AtomicU64, Ordering::Relaxed},
            Arc,
        },
    };
    use tokio::task::spawn_blocking;
    use warp::{
        filters::{body::bytes, method::post},
        header::value as header_value,
        http::header::HeaderValue,
        path,
        reply::Response,
        Filter,
    };

    macro_rules! starts_with_server {
        ($addr:ident, $routes:ident, $code:block) => {{
            let (tx, rx) = channel();
            let ($addr, server) = warp::serve($routes).bind_with_graceful_shutdown(([127, 0, 0, 1], 0), async move {
                rx.await.ok();
            });
            let handler = tokio::spawn(server);
            $code?;
            tx.send(()).ok();
            handler.await.ok();
        }};
    }

    const BUF_LEN: usize = 1 << 20;
    const MD5_LEN: usize = 16;

    #[tokio::test]
    async fn sync_http_test() -> anyhow::Result<()> {
        env_logger::builder().is_test(true).try_init().ok();

        let routes = path!("dir1" / "dir2" / "file")
            .and(post())
            .and(header_value(USER_AGENT.as_str()))
            .and(bytes())
            .map(|user_agent: HeaderValue, req_body: Bytes| {
                assert_eq!(req_body.len(), BUF_LEN + MD5_LEN);
                {
                    let mut hasher = Md5::new();
                    hasher.update(&req_body[..BUF_LEN]);
                    assert_eq!(hasher.finalize().as_slice(), &req_body[BUF_LEN..]);
                }

                assert!(user_agent.to_str().unwrap().starts_with("QiniuRust/"));
                assert!(user_agent.to_str().unwrap().ends_with("/qiniu-ureq"));

                let mut resp_body = vec![0u8; BUF_LEN + MD5_LEN];
                thread_rng().fill_bytes(&mut resp_body[..BUF_LEN]);
                {
                    let mut hasher = Md5::new();
                    hasher.update(&resp_body[..BUF_LEN]);
                    resp_body[BUF_LEN..].copy_from_slice(hasher.finalize().as_slice());
                }
                Response::new(resp_body.into())
            });
        starts_with_server!(addr, routes, {
            spawn_blocking(move || {
                let mut request_body = vec![0u8; BUF_LEN + MD5_LEN];
                thread_rng().fill_bytes(&mut request_body[..BUF_LEN]);
                {
                    let mut hasher = Md5::new();
                    hasher.update(&request_body[..BUF_LEN]);
                    request_body[BUF_LEN..].copy_from_slice(hasher.finalize().as_slice());
                }

                let last_uploaded = Arc::new(AtomicU64::new(0));
                let last_total = Arc::new(AtomicU64::new(0));
                let mut response = {
                    let last_uploaded = last_uploaded.to_owned();
                    let last_total = last_total.to_owned();
                    let callback = move |info: TransferProgressInfo| {
                        last_uploaded.store(info.transferred_bytes(), Relaxed);
                        last_total.store(info.total_bytes(), Relaxed);
                        Ok(())
                    };
                    let mut request = SyncRequest::builder()
                        .method(Method::POST)
                        .url(format!("http://{}/dir1/dir2/file", addr).parse().expect("invalid uri"))
                        .body(SyncRequestBody::from_referenced_bytes(&request_body))
                        .on_uploading_progress(OnProgressCallback::reference(&callback))
                        .build();
                    Client::default().call(&mut request)?
                };
                assert_eq!(
                    response.header(&CONTENT_LENGTH).map(|h| h.as_bytes()),
                    Some(format!("{}", BUF_LEN + MD5_LEN).as_bytes())
                );
                assert_eq!(last_uploaded.load(Relaxed), request_body.len() as u64);
                assert_eq!(last_total.load(Relaxed), request_body.len() as u64);

                {
                    let mut body_part = Vec::new();
                    let mut checksum_part = Vec::new();

                    assert_eq!(
                        io_copy(&mut response.body_mut().take(BUF_LEN as u64), &mut body_part)?,
                        BUF_LEN as u64
                    );
                    assert_eq!(
                        io_copy(&mut response.body_mut().take(MD5_LEN as u64), &mut checksum_part)?,
                        MD5_LEN as u64
                    );

                    let mut hasher = Md5::new();
                    hasher.update(&body_part);
                    assert_eq!(hasher.finalize().as_slice(), checksum_part.as_slice());
                }
                Ok::<_, anyhow::Error>(())
            })
            .await?
        });

        Ok(())
    }
}
