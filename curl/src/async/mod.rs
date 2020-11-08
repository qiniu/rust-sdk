mod agent;
mod agent_context;
mod single_request_context;
mod waker;
use super::{
    http::{AsyncResponseResult, Request, ResponseError, ResponseErrorKind},
    utils::easy::{set_body, set_headers, set_method, set_options, set_url},
    CurlHTTPCaller,
};
use agent::spawn;
use single_request_context::SingleRequestContext;

pub(super) async fn async_http_call(
    http_client: &CurlHTTPCaller,
    request: &Request<'_>,
) -> AsyncResponseResult {
    let (mut easy, future) = SingleRequestContext::new(http_client, request);
    set_method(&mut easy, request)?;
    set_url(&mut easy, request)?;
    set_headers(&mut easy, request)?;
    set_body(&mut easy, request)?;
    set_options(&mut easy, request)?;
    http_client
        .before_perform_callbacks()
        .iter()
        .try_for_each(|callback| callback(easy.raw()))?;

    let agent = spawn(http_client)
        .await
        .map_err(|err| ResponseError::new(ResponseErrorKind::LocalIOError, err))?;
    agent.submit_request(easy);
    future.await
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{
        channel::oneshot::channel, executor::block_on, future::try_join_all, io::AsyncReadExt,
        lock::Mutex as AsyncMutex,
    };
    use qiniu_http::Method;
    use rand::{thread_rng, RngCore};
    use std::{
        error::Error,
        io::Read,
        result::Result,
        sync::{
            atomic::{AtomicUsize, Ordering::Relaxed},
            Arc, Mutex,
        },
        thread::sleep,
        time::Duration,
    };
    use tempfile::NamedTempFile;
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
    async fn test_get_content() -> Result<(), Box<dyn Error>> {
        let buffer = generate_buffer(1 << 20);
        let routes = {
            let buffer = buffer.to_owned();
            path!("file" / "content").map(move || Response::new(buffer.to_owned().into()))
        };

        starts_with_server!(addr, routes, {
            let mut response = async_http_call(
                &CurlHTTPCaller::default(),
                &Request::builder()
                    .url(format!("http://{}/file/content", addr))
                    .build(),
            )
            .await?;
            assert_eq!(response.status_code(), StatusCode::OK);
            {
                let mut bytes = Vec::new();
                response.body_mut().read_to_end(&mut bytes).await?;
                assert!(bytes == buffer);
            }
            assert_eq!(response.server_ip(), Some(addr.ip()));
            assert_eq!(response.server_port(), addr.port());
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_get_large_content() -> Result<(), Box<dyn Error>> {
        let buffer = generate_buffer(10 * (1 << 20));
        let routes = {
            let buffer = buffer.to_owned();
            path!("file" / "content").map(move || Response::new(buffer.to_owned().into()))
        };

        starts_with_server!(addr, routes, {
            let response_body_size_cnt = Arc::new(AtomicUsize::new(0));
            let mut response = {
                let response_body_size_cnt = response_body_size_cnt.to_owned();
                async_http_call(
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
                        .build(),
                )
                .await?
            };
            assert_eq!(response.status_code(), StatusCode::OK);
            {
                let mut bytes = Vec::with_capacity(buffer.len());
                response.body_mut().read_to_end(&mut bytes).await?;
                assert!(bytes == buffer);
            }
            assert_eq!(response.server_ip(), Some(addr.ip()));
            assert_eq!(response.server_port(), addr.port());
            assert_eq!(response_body_size_cnt.load(Relaxed), 10 * (1 << 20));

            response_body_size_cnt.store(0, Relaxed);
            let mut tempfile = spawn_blocking(|| NamedTempFile::new()).await??;
            response = {
                let response_body_size_cnt = response_body_size_cnt.to_owned();
                async_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/file/content", addr))
                        .response_body_buffer_path(tempfile.path())
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
                        .build(),
                )
                .await?
            };
            assert_eq!(response.status_code(), StatusCode::OK);
            {
                let mut bytes = Vec::with_capacity(buffer.len());
                response.body_mut().read_to_end(&mut bytes).await?;
                assert!(bytes == buffer);
            }
            assert_eq!(response.server_ip(), Some(addr.ip()));
            assert_eq!(response.server_port(), addr.port());
            assert_eq!(response_body_size_cnt.load(Relaxed), 10 * (1 << 20));
            {
                let buffer_len = buffer.len();
                let bytes = spawn_blocking(move || {
                    let mut bytes = Vec::with_capacity(buffer_len);
                    tempfile
                        .as_file_mut()
                        .read_to_end(&mut bytes)
                        .map(|_| bytes)
                })
                .await??;
                assert!(bytes == buffer);
            }
        });

        Ok(())
    }

    #[tokio::test]
    async fn test_get_contents() -> Result<(), Box<dyn Error>> {
        let buffers = Arc::new(
            (0..20)
                .map(|_| generate_buffer(1 << 20))
                .collect::<Vec<_>>(),
        );
        let routes = {
            let buffers = buffers.to_owned();
            path!("file" / usize)
                .map(move |i: usize| Response::new(buffers.get(i).unwrap().to_owned().into()))
        };

        starts_with_server!(addr, routes, {
            let futures = (0..20).map(|i| async move {
                async_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/file/{}", addr, i))
                        .connect_timeout(Duration::from_secs(60))
                        .request_timeout(Duration::from_secs(600))
                        .build(),
                )
                .await
            });
            for (i, mut response) in try_join_all(futures).await?.into_iter().enumerate() {
                assert_eq!(response.status_code(), StatusCode::OK);
                {
                    let mut bytes = Vec::new();
                    response.body_mut().read_to_end(&mut bytes).await?;
                    assert!(bytes == buffers[i]);
                }
                assert_eq!(response.server_ip(), Some(addr.ip()));
                assert_eq!(response.server_port(), addr.port());
            }
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_abort_downloading() -> Result<(), Box<dyn Error>> {
        let routes = {
            let buffer = generate_buffer(1 << 20);
            path!("file" / "content").map(move || Response::new(buffer.to_owned().into()))
        };

        starts_with_server!(addr, routes, {
            let err = async_http_call(
                &CurlHTTPCaller::default(),
                &Request::builder()
                    .url(format!("http://{}/file/content", addr))
                    .on_downloading_progress(Some(&|_downloaded, _total| false))
                    .build(),
            )
            .await
            .unwrap_err();
            assert_eq!(err.kind(), ResponseErrorKind::UserCanceled);

            let err = async_http_call(
                &CurlHTTPCaller::default(),
                &Request::builder()
                    .url(format!("http://{}/file/content", addr))
                    .on_receive_response_body(Some(&|_body| false))
                    .build(),
            )
            .await
            .unwrap_err();
            assert_eq!(err.kind(), ResponseErrorKind::UserCanceled);
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_redirection() -> Result<(), Box<dyn Error>> {
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
            let status_codes = Arc::new(AsyncMutex::new(Vec::<u16>::new()));
            let mut response = {
                let status_codes = status_codes.clone();
                async_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/redirect/1", addr))
                        .on_receive_response_status(Some(&|status| {
                            block_on(async { status_codes.lock().await }).push(status);
                            true
                        }))
                        .build(),
                )
                .await?
            };
            assert_eq!(response.status_code(), StatusCode::MOVED_PERMANENTLY);
            assert_eq!(status_codes.lock().await.as_slice(), &[301]);
            status_codes.lock().await.clear();

            response = {
                let status_codes = status_codes.clone();
                async_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/redirect/1", addr))
                        .follow_redirection(true)
                        .on_receive_response_status(Some(&|status| {
                            block_on(async { status_codes.lock().await }).push(status);
                            true
                        }))
                        .build(),
                )
                .await?
            };
            assert_eq!(response.status_code(), StatusCode::OK);
            {
                let mut bytes = Vec::new();
                response.body_mut().read_to_end(&mut bytes).await?;
                assert!(bytes == buffer);
            }
            assert_eq!(response.server_ip(), Some(addr.ip()));
            assert_eq!(response.server_port(), addr.port());
            assert_eq!(status_codes.lock().await.as_slice(), &[301, 301, 301, 200]);
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_too_many_redirection() -> Result<(), Box<dyn Error>> {
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
            let err = async_http_call(
                &CurlHTTPCaller::default(),
                &Request::builder()
                    .url(format!("http://{}/redirect/1", addr))
                    .follow_redirection(true)
                    .build(),
            )
            .await
            .unwrap_err();
            assert_eq!(err.kind(), ResponseErrorKind::TooManyRedirect);
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_upload_content() -> Result<(), Box<dyn Error>> {
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
                async_http_call(
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
                        .build(),
                )
                .await?
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
    async fn test_upload_contents() -> Result<(), Box<dyn Error>> {
        let recv_req_bodies = Arc::new((0..20).map(|_| Mutex::new(Vec::new())).collect::<Vec<_>>());
        let routes = {
            let recv_req_bodies = recv_req_bodies.to_owned();
            path!("upload" / usize)
                .and(body::bytes())
                .map(move |i: usize, bytes: Bytes| {
                    let mut recv_req_body = recv_req_bodies.get(i).unwrap().lock().unwrap();
                    recv_req_body.clear();
                    recv_req_body.extend_from_slice(&bytes);
                    StatusCode::OK
                })
        };
        starts_with_server!(addr, routes, {
            let req_bodies = Arc::new(
                (0..20)
                    .map(|_| generate_buffer(1 << 20))
                    .collect::<Vec<_>>(),
            );
            let futures = (0..20).map(|i| {
                let req_body = req_bodies.get(i).unwrap().to_owned();
                async move {
                    async_http_call(
                        &CurlHTTPCaller::default(),
                        &Request::builder()
                            .method(Method::PUT)
                            .url(format!("http://{}/upload/{}", addr, i))
                            .body(req_body)
                            .build(),
                    )
                    .await
                }
            });
            for (i, response) in try_join_all(futures).await?.into_iter().enumerate() {
                assert_eq!(response.status_code(), StatusCode::OK);
                {
                    assert!(
                        req_bodies.get(i).unwrap()
                            == &*recv_req_bodies.get(i).unwrap().lock().unwrap()
                    );
                }
                assert_eq!(response.server_ip(), Some(addr.ip()));
                assert_eq!(response.server_port(), addr.port());
            }
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_abort_uploading() -> Result<(), Box<dyn Error>> {
        let routes = path!("upload").map(|| StatusCode::OK);

        starts_with_server!(addr, routes, {
            let req_body = generate_buffer(1 << 20);
            let err = {
                let req_body = req_body.to_owned();
                async_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .method(Method::POST)
                        .url(format!("http://{}/upload", addr))
                        .body(&req_body)
                        .on_uploading_progress(Some(&|_downloaded, _total| false))
                        .build(),
                )
                .await
                .unwrap_err()
            };
            assert_eq!(err.kind(), ResponseErrorKind::UserCanceled);

            let err = {
                let req_body = req_body.to_owned();
                async_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/upload", addr))
                        .body(&req_body)
                        .on_send_request_body(Some(&|_body| false))
                        .build(),
                )
                .await
                .unwrap_err()
            };
            assert_eq!(err.kind(), ResponseErrorKind::UserCanceled);
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_resolved_addr() -> Result<(), Box<dyn Error>> {
        let routes = path!("file" / "content").map(move || Response::new("hello".into()));

        starts_with_server!(addr, routes, {
            let mut response = async_http_call(
                &CurlHTTPCaller::default(),
                &Request::builder()
                    .url(format!("http://qiniu.com:{}/file/content", addr.port()))
                    .resolved_ip_addrs([addr.ip()].as_ref())
                    .build(),
            )
            .await?;
            assert_eq!(response.status_code(), StatusCode::OK);
            {
                let mut bytes = Vec::new();
                response.body_mut().read_to_end(&mut bytes).await?;
                assert_eq!(&bytes, b"hello");
            }
            assert_eq!(response.server_ip(), Some(addr.ip()));
            assert_eq!(response.server_port(), addr.port());
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_timeout() -> Result<(), Box<dyn Error>> {
        let routes = path!("no" / "response").map(move || {
            sleep(Duration::from_secs(5));
            StatusCode::OK
        });

        starts_with_server!(addr, routes, {
            let err = async_http_call(
                &CurlHTTPCaller::default(),
                &Request::builder()
                    .url(format!("http://{}/no/response", addr))
                    .request_timeout(Duration::from_secs(3))
                    .build(),
            )
            .await
            .unwrap_err();
            assert_eq!(err.kind(), ResponseErrorKind::TimeoutError);
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_user_agent() -> Result<(), Box<dyn Error>> {
        let user_agent = Arc::new(AsyncMutex::new(String::new()));
        let routes = {
            let user_agent = user_agent.to_owned();
            path!("get" / "useragent")
                .and(header::value("User-Agent"))
                .map(move |agent: HeaderValue| {
                    block_on(async { user_agent.lock().await }).push_str(agent.to_str().unwrap());
                    StatusCode::OK
                })
        };

        starts_with_server!(addr, routes, {
            let response = async_http_call(
                &CurlHTTPCaller::default(),
                &Request::builder()
                    .url(format!("http://{}/get/useragent", addr))
                    .build(),
            )
            .await?;
            assert_eq!(response.status_code(), StatusCode::OK);
            assert!(user_agent
                .lock()
                .await
                .as_str()
                .starts_with("QiniuRust/qiniu-http-"));

            let response = async_http_call(
                &CurlHTTPCaller::default(),
                &Request::builder()
                    .url(format!("http://{}/get/useragent", addr))
                    .request_timeout(Duration::from_secs(3))
                    .appended_user_agent("/user-agent-test")
                    .build(),
            )
            .await?;
            assert_eq!(response.status_code(), StatusCode::OK);
            assert!(user_agent
                .lock()
                .await
                .as_str()
                .starts_with("QiniuRust/qiniu-http-"));
            assert!(user_agent
                .lock()
                .await
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
