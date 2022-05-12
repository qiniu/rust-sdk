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

//! # qiniu-isahc
//!
//! ## 七牛 Isahc HTTP 客户端实现
//!
//! 基于 Isahc 库提供 HTTP 客户端接口实现（同时实现阻塞接口和异步接口，异步实现则需要启用 `async` 功能）
//! 不过由于 Isahc 库本身核心是使用异步接口实现，所以即使不启用 `async` 功能，也会引入异步相关库。

mod client;
mod extensions;

pub use client::Client;
pub use extensions::*;

pub use isahc;
pub use qiniu_http as http;

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use futures::channel::oneshot::channel;
    use isahc::http::header::{CONTENT_LENGTH, USER_AGENT};
    use md5::{Digest, Md5};
    use qiniu_http::{HttpCaller, Method, OnProgressCallback, SyncRequest, SyncRequestBody, TransferProgressInfo};
    use rand::{thread_rng, RngCore};
    use std::{
        io::{copy as io_copy, Read},
        sync::{
            atomic::{AtomicU64, Ordering::Relaxed},
            Arc,
        },
        time::Duration,
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

    #[cfg(feature = "async")]
    use {
        futures::io::{copy as async_io_copy, AsyncReadExt},
        qiniu_http::{AsyncRequest, AsyncRequestBody},
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
                assert!(user_agent.to_str().unwrap().contains("/qiniu-isahc/"));

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
                    let resolved_ip_addrs = [addr.ip()];
                    let mut request = SyncRequest::builder()
                        .method(Method::POST)
                        .url(
                            format!("http://fakehost:{}/dir1/dir2/file", addr.port())
                                .parse()
                                .expect("invalid uri"),
                        )
                        .body(SyncRequestBody::from_referenced_bytes(&request_body))
                        .resolved_ip_addrs(resolved_ip_addrs.as_ref())
                        .on_uploading_progress(OnProgressCallback::reference(&callback))
                        .add_extension(TimeoutRequestExtension::new(Duration::from_secs(1)))
                        .build();
                    Client::default_client()?.call(&mut request)?
                };
                assert_eq!(
                    response.header(&CONTENT_LENGTH).map(|h| h.as_bytes()),
                    Some(format!("{}", BUF_LEN + MD5_LEN).as_bytes())
                );
                assert_eq!(last_uploaded.load(Relaxed), request_body.len() as u64);
                assert_eq!(last_total.load(Relaxed), request_body.len() as u64);
                assert_eq!(
                    response.extensions().get::<TimeoutRequestExtension>().unwrap().get(),
                    &Duration::from_secs(1)
                );

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

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn async_http_test() -> anyhow::Result<()> {
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
                assert!(user_agent.to_str().unwrap().contains("/qiniu-isahc/"));

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
                let resolved_ip_addrs = [addr.ip()];
                let mut request = AsyncRequest::builder()
                    .method(Method::POST)
                    .url(
                        format!("http://fakehost:{}/dir1/dir2/file", addr.port())
                            .parse()
                            .expect("invalid uri"),
                    )
                    .body(AsyncRequestBody::from_referenced_bytes(&request_body))
                    .resolved_ip_addrs(resolved_ip_addrs.as_ref())
                    .on_uploading_progress(OnProgressCallback::reference(&callback))
                    .add_extension(TimeoutRequestExtension::new(Duration::from_secs(1)))
                    .build();
                Client::default_client()?.async_call(&mut request).await?
            };
            assert_eq!(
                response.header(&CONTENT_LENGTH).map(|h| h.as_bytes()),
                Some(format!("{}", BUF_LEN + MD5_LEN).as_bytes())
            );
            assert_eq!(last_uploaded.load(Relaxed), request_body.len() as u64);
            assert_eq!(last_total.load(Relaxed), request_body.len() as u64);
            assert_eq!(
                response.extensions().get::<TimeoutRequestExtension>().unwrap().get(),
                &Duration::from_secs(1)
            );

            {
                let mut body_part = Vec::new();
                let mut checksum_part = Vec::new();

                assert_eq!(
                    async_io_copy(&mut response.body_mut().take(BUF_LEN as u64), &mut body_part).await?,
                    BUF_LEN as u64
                );
                assert_eq!(
                    async_io_copy(&mut response.body_mut().take(MD5_LEN as u64), &mut checksum_part).await?,
                    MD5_LEN as u64
                );

                let mut hasher = Md5::new();
                hasher.update(&body_part);
                assert_eq!(hasher.finalize().as_slice(), checksum_part.as_slice());
            }
            Ok::<_, anyhow::Error>(())
        });

        Ok(())
    }
}
