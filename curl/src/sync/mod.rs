mod context;
mod pool;

use super::{
    http::{
        Request, ResponseError, ResponseErrorKind, StatusCode, SyncResponseBuilder,
        SyncResponseResult,
    },
    utils::easy::{handle, set_body, set_headers, set_method, set_options, set_url},
    CurlHTTPCaller,
};
use context::{Context, ResponseBody};
use curl::{easy::Easy2, Error as CurlError};
use std::mem::transmute;

pub(super) fn sync_http_call(
    http_client: &CurlHTTPCaller,
    request: &Request,
) -> SyncResponseResult {
    let r = &mut *pool::pull();
    let easy: &mut Easy2<Context> = unsafe { transmute(r) };

    easy.reset();
    easy.get_mut().reset(http_client, request);

    perform(http_client, easy, request)
}

fn perform(
    http_client: &CurlHTTPCaller,
    easy: &mut Easy2<Context>,
    request: &Request,
) -> SyncResponseResult {
    set_method(easy, request)?;
    set_url(easy, request)?;
    set_headers(easy, request)?;
    set_body(easy, request)?;
    set_options(easy, request)?;
    http_client
        .before_perform_callbacks()
        .iter()
        .try_for_each(|callback| callback(easy.raw()))?;
    check_perform_result(easy, easy.perform())?;
    http_client
        .after_perform_callbacks()
        .iter()
        .try_for_each(|callback| callback(easy.raw()))?;
    build_response(easy)
}

fn build_response(easy: &mut Easy2<Context>) -> SyncResponseResult {
    let status_code = handle(easy.response_code())? as StatusCode;
    let server_ip = handle(easy.primary_ip().map(|s| s.and_then(|s| s.parse().ok())))?;
    let server_port = handle(easy.primary_port())?;

    let mut builder = SyncResponseBuilder::default()
        .status_code(status_code)
        .headers(easy.get_mut().take_response_headers());
    builder = match easy.get_mut().take_response_body() {
        ResponseBody::Bytes(bytes) => builder.bytes_as_body(bytes),
        ResponseBody::File(file) => builder
            .file_as_body(file)
            .map_err(|err| ResponseError::new(ResponseErrorKind::LocalIOError, err))?,
    };
    Ok(builder
        .server_ip(server_ip)
        .server_port(server_port)
        .build())
}

#[inline]
fn check_perform_result<T>(
    easy: &Easy2<Context>,
    result: Result<T, CurlError>,
) -> Result<T, ResponseError> {
    if easy.get_ref().canceled() {
        Err(ResponseError::new(
            ResponseErrorKind::UserCanceled,
            "User Canceled",
        ))
    } else {
        handle(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use futures::channel::oneshot::channel;
    use qiniu_http::Method;
    use rand::{thread_rng, RngCore};
    use std::{
        io::Read,
        sync::{
            atomic::{AtomicUsize, Ordering::Relaxed},
            Arc, Mutex,
        },
        thread::sleep,
        time::Duration,
    };
    use tokio::task::{spawn, spawn_blocking};
    use warp::{
        body, header,
        http::{HeaderValue, StatusCode, Uri},
        hyper::body::Bytes,
        path, redirect,
        reply::Response,
        Filter,
    };

    macro_rules! starts_with_server {
        ($addr:ident, $routes:ident, $code:block) => {{
            let (tx, rx) = channel();
            let ($addr, server) =
                warp::serve($routes).bind_with_graceful_shutdown(([127, 0, 0, 1], 0), async move {
                    rx.await.ok();
                });
            let handler = spawn(server);
            $code;
            tx.send(()).ok();
            handler.await.ok();
        }};
    }

    #[tokio::test]
    async fn test_get_content() -> Result<()> {
        let buffer = generate_buffer(1 << 20);
        let routes = {
            let buffer = buffer.to_owned();
            path!("file" / "content").map(move || Response::new(buffer.to_owned().into()))
        };

        starts_with_server!(addr, routes, {
            let mut response = spawn_blocking(move || {
                sync_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/file/content", addr))
                        .build()?,
                )
            })
            .await??;
            assert_eq!(response.status_code(), StatusCode::OK);
            {
                let mut bytes = Vec::new();
                response.body_mut().read_to_end(&mut bytes)?;
                assert!(bytes == buffer);
            }
            assert_eq!(response.server_ip(), Some(addr.ip()));
            assert_eq!(response.server_port(), addr.port());
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_get_large_content() -> Result<()> {
        let buffer = generate_buffer(10 * (1 << 20));
        let routes = {
            let buffer = buffer.to_owned();
            path!("file" / "content").map(move || Response::new(buffer.to_owned().into()))
        };

        starts_with_server!(addr, routes, {
            let response_body_size_cnt = Arc::new(AtomicUsize::new(0));
            let mut response = {
                let response_body_size_cnt = response_body_size_cnt.to_owned();
                spawn_blocking(move || {
                    sync_http_call(
                        &CurlHTTPCaller::default(),
                        &Request::builder()
                            .url(format!("http://{}/file/content", addr))
                            .on_uploading_progress(Some(&|_uploaded, _total| unreachable!()))
                            .on_downloading_progress(Some(&|downloaded, total| {
                                assert_eq!(total, 10 * (1 << 20));
                                assert!(downloaded <= total);
                                true
                            }))
                            .on_receive_response_body(Some(&|data| {
                                response_body_size_cnt.fetch_add(data.len(), Relaxed);
                                true
                            }))
                            .build()?,
                    )
                })
                .await??
            };
            assert_eq!(response.status_code(), StatusCode::OK);
            {
                let mut bytes = Vec::with_capacity(buffer.len());
                response.body_mut().read_to_end(&mut bytes)?;
                assert!(bytes == buffer);
            }
            assert_eq!(response.server_ip(), Some(addr.ip()));
            assert_eq!(response.server_port(), addr.port());
            assert_eq!(response_body_size_cnt.load(Relaxed), 10 * (1 << 20));
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_abort_downloading() -> Result<()> {
        let routes = {
            let buffer = generate_buffer(1 << 20);
            path!("file" / "content").map(move || Response::new(buffer.to_owned().into()))
        };

        starts_with_server!(addr, routes, {
            let err = spawn_blocking(move || {
                sync_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/file/content", addr))
                        .on_downloading_progress(Some(&|_downloaded, _total| false))
                        .build()?,
                )
            })
            .await?
            .unwrap_err();
            assert_eq!(err.kind(), ResponseErrorKind::UserCanceled);

            let err = spawn_blocking(move || {
                sync_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/file/content", addr))
                        .on_receive_response_body(Some(&|_body| false))
                        .build()?,
                )
            })
            .await?
            .unwrap_err();
            assert_eq!(err.kind(), ResponseErrorKind::UserCanceled);
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_redirection() -> Result<()> {
        let buffer = generate_buffer(1 << 20);
        let routes = {
            let buffer = buffer.to_owned();
            path!("redirect" / "1")
                .map(|| redirect(Uri::from_static("/redirect/2")))
                .or(path!("redirect" / "2").map(|| redirect(Uri::from_static("/redirect/3"))))
                .or(path!("redirect" / "3").map(|| redirect(Uri::from_static("/file/content"))))
                .or(path!("file" / "content").map(move || Response::new(buffer.to_owned().into())))
        };

        starts_with_server!(addr, routes, {
            let status_codes = Arc::new(Mutex::new(Vec::<u16>::new()));
            let mut response = {
                let status_codes = status_codes.clone();
                spawn_blocking(move || {
                    sync_http_call(
                        &CurlHTTPCaller::default(),
                        &Request::builder()
                            .url(format!("http://{}/redirect/1", addr))
                            .on_receive_response_status(Some(&|status| {
                                status_codes.lock().unwrap().push(status);
                                true
                            }))
                            .build()?,
                    )
                })
                .await??
            };
            assert_eq!(response.status_code(), StatusCode::MOVED_PERMANENTLY);
            assert_eq!(status_codes.lock().unwrap().as_slice(), &[301]);
            status_codes.lock().unwrap().clear();

            response = {
                let status_codes = status_codes.clone();
                spawn_blocking(move || {
                    sync_http_call(
                        &CurlHTTPCaller::default(),
                        &Request::builder()
                            .url(format!("http://{}/redirect/1", addr))
                            .follow_redirection(true)
                            .on_receive_response_status(Some(&|status| {
                                status_codes.lock().unwrap().push(status);
                                true
                            }))
                            .build()?,
                    )
                })
                .await??
            };
            assert_eq!(response.status_code(), StatusCode::OK);
            {
                let mut bytes = Vec::new();
                response.body_mut().read_to_end(&mut bytes)?;
                assert!(bytes == buffer);
            }
            assert_eq!(response.server_ip(), Some(addr.ip()));
            assert_eq!(response.server_port(), addr.port());
            assert_eq!(
                status_codes.lock().unwrap().as_slice(),
                &[301, 301, 301, 200]
            );
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_too_many_redirection() -> Result<()> {
        let buffer = generate_buffer(1 << 20);
        let routes = {
            let buffer = buffer.to_owned();
            path!("redirect" / "1")
                .map(|| redirect(Uri::from_static("/redirect/2")))
                .or(path!("redirect" / "2").map(|| redirect(Uri::from_static("/redirect/3"))))
                .or(path!("redirect" / "3").map(|| redirect(Uri::from_static("/redirect/4"))))
                .or(path!("redirect" / "4").map(|| redirect(Uri::from_static("/file/content"))))
                .or(path!("file" / "content").map(move || Response::new(buffer.to_owned().into())))
        };

        starts_with_server!(addr, routes, {
            let err = spawn_blocking(move || {
                sync_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/redirect/1", addr))
                        .follow_redirection(true)
                        .build()?,
                )
            })
            .await?
            .unwrap_err();
            assert_eq!(err.kind(), ResponseErrorKind::TooManyRedirect);
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_upload_content() -> Result<()> {
        let recv_req_body = Arc::new(Mutex::new(Vec::new()));
        let routes = {
            let recv_req_body = recv_req_body.to_owned();
            path!("upload").and(body::bytes()).map(move |bytes: Bytes| {
                let mut recv_req_body = recv_req_body.lock().unwrap();
                recv_req_body.clear();
                recv_req_body.extend_from_slice(&bytes);
                StatusCode::OK
            })
        };
        starts_with_server!(addr, routes, {
            let req_body = generate_buffer(1 << 20);
            let req_body_size_cnt = Arc::new(AtomicUsize::new(0));
            let response = {
                let req_body = req_body.to_owned();
                let req_body_size_cnt = req_body_size_cnt.to_owned();
                spawn_blocking(move || {
                    sync_http_call(
                        &CurlHTTPCaller::default(),
                        &Request::builder()
                            .method(Method::PUT)
                            .url(format!("http://{}/upload", addr))
                            .body(&req_body)
                            .on_uploading_progress(Some(&|uploaded, total| {
                                assert_eq!(total, 1 << 20);
                                assert!(uploaded <= total);
                                true
                            }))
                            .on_downloading_progress(Some(&|_downloaded, _total| unreachable!()))
                            .on_send_request_body(Some(&|data| {
                                req_body_size_cnt.fetch_add(data.len(), Relaxed);
                                true
                            }))
                            .build()?,
                    )
                })
                .await??
            };
            assert_eq!(response.status_code(), StatusCode::OK);
            assert!(req_body == *recv_req_body.lock().unwrap());
            assert_eq!(response.server_ip(), Some(addr.ip()));
            assert_eq!(response.server_port(), addr.port());
            assert_eq!(req_body_size_cnt.load(Relaxed), 1 << 20);
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_abort_uploading() -> Result<()> {
        let routes = path!("upload").map(|| StatusCode::OK);

        starts_with_server!(addr, routes, {
            let req_body = generate_buffer(1 << 20);
            let err = {
                let req_body = req_body.to_owned();
                spawn_blocking(move || {
                    sync_http_call(
                        &CurlHTTPCaller::default(),
                        &Request::builder()
                            .method(Method::POST)
                            .url(format!("http://{}/upload", addr))
                            .body(&req_body)
                            .on_uploading_progress(Some(&|_downloaded, _total| false))
                            .build()?,
                    )
                })
                .await?
                .unwrap_err()
            };
            assert_eq!(err.kind(), ResponseErrorKind::UserCanceled);

            let err = {
                spawn_blocking(move || {
                    let req_body = req_body.to_owned();
                    sync_http_call(
                        &CurlHTTPCaller::default(),
                        &Request::builder()
                            .url(format!("http://{}/upload", addr))
                            .body(&req_body)
                            .on_send_request_body(Some(&|_body| false))
                            .build()?,
                    )
                })
                .await?
                .unwrap_err()
            };
            assert_eq!(err.kind(), ResponseErrorKind::UserCanceled);
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_resolved_addr() -> Result<()> {
        let routes = path!("file" / "content").map(move || Response::new("hello".into()));

        starts_with_server!(addr, routes, {
            let mut response = spawn_blocking(move || {
                sync_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://qiniu.com:{}/file/content", addr.port()))
                        .resolved_ip_addrs([addr.ip()].as_ref())
                        .build()?,
                )
            })
            .await??;
            assert_eq!(response.status_code(), StatusCode::OK);
            {
                let mut bytes = Vec::new();
                response.body_mut().read_to_end(&mut bytes)?;
                assert_eq!(&bytes, b"hello");
            }
            assert_eq!(response.server_ip(), Some(addr.ip()));
            assert_eq!(response.server_port(), addr.port());
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_timeout() -> Result<()> {
        let routes = path!("no" / "response").map(move || {
            sleep(Duration::from_secs(5));
            StatusCode::OK
        });

        starts_with_server!(addr, routes, {
            let err = spawn_blocking(move || {
                sync_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/no/response", addr))
                        .request_timeout(Duration::from_secs(3))
                        .build()?,
                )
            })
            .await?
            .unwrap_err();
            assert_eq!(err.kind(), ResponseErrorKind::TimeoutError);
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_user_agent() -> Result<()> {
        let user_agent = Arc::new(Mutex::new(String::new()));
        let routes = {
            let user_agent = user_agent.clone();
            path!("get" / "useragent")
                .and(header::value("User-Agent"))
                .map(move |agent: HeaderValue| {
                    user_agent.lock().unwrap().push_str(agent.to_str().unwrap());
                    StatusCode::OK
                })
        };

        starts_with_server!(addr, routes, {
            let response = spawn_blocking(move || {
                sync_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/get/useragent", addr))
                        .build()?,
                )
            })
            .await??;
            assert_eq!(response.status_code(), StatusCode::OK);
            assert!(user_agent
                .lock()
                .unwrap()
                .as_str()
                .starts_with("QiniuRust/qiniu-http-"));

            let response = spawn_blocking(move || {
                sync_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/get/useragent", addr))
                        .appended_user_agent("/user-agent-test")
                        .build()?,
                )
            })
            .await??;
            assert_eq!(response.status_code(), StatusCode::OK);
            assert!(user_agent
                .lock()
                .unwrap()
                .as_str()
                .starts_with("QiniuRust/qiniu-http-"));
            assert!(user_agent
                .lock()
                .unwrap()
                .as_str()
                .contains("/user-agent-test/libcurl-"));
        });
        Ok(())
    }

    #[inline]
    fn generate_buffer(size: usize) -> Vec<u8> {
        let mut buffer = vec![0; size];
        thread_rng().fill_bytes(&mut buffer);
        buffer
    }
}
