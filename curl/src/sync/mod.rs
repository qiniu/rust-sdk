mod context;
mod pool;

use super::CurlHTTPCaller;
use context::{Context, ResponseBody};
use curl::{
    easy::{Easy2, HttpVersion, List},
    Error as CurlError, Version,
};
use once_cell::sync::Lazy;
use qiniu_http::{
    Method, Request, ResponseBuilder, ResponseError, ResponseErrorKind, ResponseResult, StatusCode,
};
use std::{convert::TryInto, mem::ManuallyDrop, result::Result};
use url::Url;

static IPV6_SUPPORT: Lazy<bool> = Lazy::new(|| Version::get().feature_ipv6());
static MULTI_IP_ADDRS_SUPPORT: Lazy<bool> =
    Lazy::new(|| Version::get().version_num() >= 0x07_3b_00);

pub(super) fn sync_http_call(http_client: &CurlHTTPCaller, request: &Request) -> ResponseResult {
    let r = &mut *pool::pull();
    let mut easy: ManuallyDrop<Box<Easy2<Context>>> = r.into();

    easy.reset();
    easy.get_mut().reset(http_client, request);

    perform(&mut easy, request)
}

fn perform(easy: &mut Easy2<Context>, request: &Request) -> ResponseResult {
    set_method(easy, request)?;
    set_url(easy, request)?;
    set_headers(easy, request)?;
    set_body(easy, request)?;
    set_options(easy, request)?;
    handle(easy.perform())?;
    build_response(easy)
}

fn set_method(easy: &mut Easy2<Context>, request: &Request) -> Result<(), ResponseError> {
    match request.method() {
        Method::GET => handle(easy.get(true)),
        Method::HEAD => handle(easy.nobody(true)),
        Method::POST => handle(easy.post(true)),
        Method::PUT => handle(easy.upload(true)),
    }
}

#[inline]
fn set_url(easy: &mut Easy2<Context>, request: &Request) -> Result<(), ResponseError> {
    handle(easy.url(request.url()))
}

fn set_headers(easy: &mut Easy2<Context>, request: &Request) -> Result<(), ResponseError> {
    let mut header_list = List::new();
    handle(header_list.append("Expect:"))?;
    for (header_name, header_value) in request.headers().iter() {
        let line = header_name.as_ref().to_string() + ": " + header_value;
        handle(header_list.append(&line))?;
    }
    handle(easy.http_headers(header_list))?;
    Ok(())
}

#[inline]
fn set_body(easy: &mut Easy2<Context>, request: &Request) -> Result<(), ResponseError> {
    handle(easy.post_field_size(request.body().len().try_into().unwrap()))
}

fn set_options(easy: &mut Easy2<Context>, request: &Request) -> Result<(), ResponseError> {
    set_preresolved_socket_addrs(easy, request)?;
    handle(easy.useragent(&(request.user_agent() + "/libcurl-" + &Version::get().version())))?;
    handle(easy.accept_encoding(""))?;
    handle(easy.http_version(HttpVersion::Any))?;
    handle(easy.show_header(false))?;
    handle(easy.progress(
        request.on_uploading_progress().is_some() || request.on_downloading_progress().is_some(),
    ))?;
    handle(easy.transfer_encoding(true))?;
    handle(easy.follow_location(request.follow_redirection()))?;
    handle(easy.max_redirections(3))?;
    handle(easy.connect_timeout(request.connect_timeout()))?;
    handle(easy.timeout(request.request_timeout()))?;
    handle(easy.tcp_keepalive(true))?;
    handle(easy.tcp_keepidle(request.tcp_keepalive_idle_timeout()))?;
    handle(easy.tcp_keepintvl(request.tcp_keepalive_probe_interval()))?;
    {
        let (speed, timeout) = request.low_transfer_speed();
        handle(easy.low_speed_limit(speed))?;
        handle(easy.low_speed_time(timeout))?;
    }
    Ok(())
}

fn set_preresolved_socket_addrs(
    easy: &mut Easy2<Context>,
    request: &Request,
) -> Result<(), ResponseError> {
    if !request.resolved_ip_addrs().is_empty() {
        let url = Url::parse(request.url()).unwrap();
        let mut addr = url.host_str().unwrap().to_owned()
            + ":"
            + &url.port_or_known_default().unwrap().to_string()
            + ":";
        for (i, ip_addr) in request.resolved_ip_addrs().iter().enumerate() {
            if !*IPV6_SUPPORT && ip_addr.is_ipv6() {
                continue;
            }
            if i > 0 {
                addr.push_str(",");
            }
            addr.push_str(&ip_addr.to_string());
            if !*MULTI_IP_ADDRS_SUPPORT {
                break;
            }
        }
        if !addr.ends_with(':') {
            let mut list = List::new();
            handle(list.append(&addr))?;
            handle(easy.resolve(list))?;
        }
    }
    Ok(())
}

fn build_response(easy: &mut Easy2<Context>) -> ResponseResult {
    let status_code = handle(easy.response_code())? as StatusCode;
    let server_ip = handle(easy.primary_ip().map(|s| s.and_then(|s| s.parse().ok())))?;
    let server_port = handle(easy.primary_port())?;

    let mut builder = ResponseBuilder::default()
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

fn handle<T>(result: Result<T, CurlError>) -> Result<T, ResponseError> {
    result.map_err(|err| {
        if err.is_unsupported_protocol()
            || err.is_bad_content_encoding()
            || err.is_filesize_exceeded()
            || err.is_http2_error()
            || err.is_http2_stream_error()
        {
            ResponseError::new(ResponseErrorKind::ProtocolError, err)
        } else if err.is_url_malformed() {
            ResponseError::new(ResponseErrorKind::InvalidURLError, err)
        } else if err.is_couldnt_resolve_proxy() || err.is_couldnt_resolve_host() {
            ResponseError::new(ResponseErrorKind::UnknownHostError, err)
        } else if err.is_couldnt_connect() {
            ResponseError::new(ResponseErrorKind::ConnectError, err)
        } else if err.is_send_error() {
            ResponseError::new(ResponseErrorKind::SendError, err)
        } else if err.is_recv_error() {
            ResponseError::new(ResponseErrorKind::ReceiveError, err)
        } else if err.is_read_error() || err.is_write_error() || err.is_send_fail_rewind() {
            ResponseError::new(ResponseErrorKind::LocalIOError, err)
        } else if err.is_aborted_by_callback() {
            ResponseError::new(ResponseErrorKind::UserCancelled, err)
        } else if err.is_operation_timedout() {
            ResponseError::new(ResponseErrorKind::TimeoutError, err)
        } else if err.is_too_many_redirects() {
            ResponseError::new(ResponseErrorKind::TooManyRedirect, err)
        } else if err.is_ssl_connect_error()
            || err.is_peer_failed_verification()
            || err.is_ssl_engine_initfailed()
            || err.is_ssl_engine_notfound()
            || err.is_ssl_engine_setfailed()
            || err.is_ssl_certproblem()
            || err.is_ssl_cipher()
            || err.is_use_ssl_failed()
            || err.is_ssl_cacert()
            || err.is_ssl_cacert_badfile()
            || err.is_ssl_crl_badfile()
            || err.is_ssl_shutdown_failed()
            || err.is_ssl_issuer_error()
        {
            ResponseError::new(ResponseErrorKind::SSLError, err)
        } else {
            ResponseError::new(ResponseErrorKind::UnknownError, err)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use futures::channel::oneshot;
    use qiniu_http::ResponseBody;
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
            let (tx, rx) = oneshot::channel();
            let ($addr, server) =
                warp::serve($routes).bind_with_graceful_shutdown(([127, 0, 0, 1], 0), async move {
                    rx.await.ok();
                });
            let handler = spawn(server);
            {
                $code;
            }
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
            let response = spawn_blocking(move || {
                sync_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/file/content", addr))
                        .build(),
                )
            })
            .await??;
            assert_eq!(response.status_code(), StatusCode::OK);
            if let ResponseBody::Bytes(bytes) = response.body() {
                assert!(bytes == &buffer);
            } else {
                panic!("Response body is not bytes: {:?}", response.body());
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
                            .build(),
                    )
                })
                .await??
            };
            assert_eq!(response.status_code(), StatusCode::OK);
            if let ResponseBody::File(file) = response.body_mut() {
                let mut bytes = Vec::with_capacity(buffer.len());
                file.read_to_end(&mut bytes)?;
                assert!(&bytes == &buffer);
            } else {
                panic!("Response body is not file: {:?}", response.body());
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
                        .build(),
                )
            })
            .await?
            .unwrap_err();
            assert_eq!(err.kind(), ResponseErrorKind::UserCancelled);

            let err = spawn_blocking(move || {
                sync_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://{}/file/content", addr))
                        .on_receive_response_body(Some(&|_body| false))
                        .build(),
                )
            })
            .await?
            .unwrap_err();
            assert_eq!(err.kind(), ResponseErrorKind::LocalIOError);
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
            let response = {
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
                            .build(),
                    )
                })
                .await??
            };
            assert_eq!(response.status_code(), StatusCode::OK);
            if let ResponseBody::Bytes(bytes) = response.body() {
                assert!(bytes == &buffer);
            } else {
                panic!("Response body is not bytes: {:?}", response.body());
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
                        .build(),
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
        let resp_body = Arc::new(Mutex::new(Vec::new()));
        let routes = {
            let resp_body = resp_body.to_owned();
            path!("upload").and(body::bytes()).map(move |bytes: Bytes| {
                let mut resp_body = resp_body.lock().unwrap();
                resp_body.clear();
                resp_body.extend_from_slice(&bytes);
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
                })
                .await??
            };
            assert_eq!(response.status_code(), StatusCode::OK);
            assert!(&req_body == &*resp_body.lock().unwrap());
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
                            .url(format!("http://{}/upload", addr))
                            .body(&req_body)
                            .on_uploading_progress(Some(&|_downloaded, _total| false))
                            .build(),
                    )
                })
                .await?
                .unwrap_err()
            };
            assert_eq!(err.kind(), ResponseErrorKind::UserCancelled);

            let err = {
                spawn_blocking(move || {
                    let req_body = req_body.to_owned();
                    sync_http_call(
                        &CurlHTTPCaller::default(),
                        &Request::builder()
                            .url(format!("http://{}/upload", addr))
                            .body(&req_body)
                            .on_send_request_body(Some(&|_body| false))
                            .build(),
                    )
                })
                .await?
                .unwrap_err()
            };
            assert_eq!(err.kind(), ResponseErrorKind::UserCancelled);
        });
        Ok(())
    }

    #[tokio::test]
    async fn test_resolved_addr() -> Result<()> {
        let routes = path!("file" / "content").map(move || Response::new("hello".into()));

        starts_with_server!(addr, routes, {
            let response = spawn_blocking(move || {
                sync_http_call(
                    &CurlHTTPCaller::default(),
                    &Request::builder()
                        .url(format!("http://qiniu.com:{}/file/content", addr.port()))
                        .resolved_ip_addrs([addr.ip()].as_ref())
                        .build(),
                )
            })
            .await??;
            assert_eq!(response.status_code(), StatusCode::OK);
            if let ResponseBody::Bytes(bytes) = response.body() {
                assert_eq!(bytes.as_slice(), b"hello");
            } else {
                panic!("Response body is not bytes: {:?}", response.body());
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
                        .build(),
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
                        .request_timeout(Duration::from_secs(3))
                        .build(),
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
                        .request_timeout(Duration::from_secs(3))
                        .appended_user_agent("/user-agent-test")
                        .build(),
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
