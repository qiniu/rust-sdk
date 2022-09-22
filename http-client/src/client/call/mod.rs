mod domain_or_ip_addr;
mod error;
mod ip_addrs;
mod ip_addrs_set;
mod request_call;
mod send_http_request;
mod try_domain_or_ip_addr;
mod try_endpoints;
mod utils;

pub(super) use request_call::request_call;

#[cfg(feature = "async")]
pub(super) use request_call::async_request_call;

#[cfg(test)]
mod tests {
    use crate::{
        client::chooser::DirectChooser,
        credential::Credential,
        test_utils::{
            chaotic_up_domains_endpoint, make_dumb_resolver, make_error_response_client_builder,
            make_fixed_response_client_builder, make_random_resolver, single_up_domain_endpoint,
        },
        Authorization, Chooser, ChooserFeedback, ErrorRetrier, IpChooser, LimitedRetrier, ResponseError,
        ResponseErrorKind, ServiceName, NO_BACKOFF,
    };
    use anyhow::{Error as AnyError, Result as AnyResult};
    use qiniu_http::{Extensions, HeaderMap, HeaderValue, ResponseErrorKind as HttpResponseErrorKind, StatusCode};
    use std::{
        collections::{HashMap, HashSet},
        error::Error,
        net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
        result::Result,
        sync::{
            atomic::{AtomicUsize, Ordering::Relaxed},
            Arc, Mutex,
        },
    };

    const X_REQ_ID_HEADER_NAME: &str = "x-reqid";
    const X_LOG_HEADER_NAME: &str = "x-log";

    #[test]
    fn test_call_endpoints_selection() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let client =
            make_error_response_client_builder(HttpResponseErrorKind::ConnectError, "Fake Connect Error", true)
                .chooser(DirectChooser)
                .resolver(make_random_resolver())
                .request_retrier(ErrorRetrier)
                .backoff(NO_BACKOFF)
                .build();

        let urls_visited = Arc::new(Mutex::new(Vec::new()));
        let domain_resolved = Arc::new(Mutex::new(Vec::new()));
        let err = client
            .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
            .on_to_resolve_domain({
                let domain_resolved = domain_resolved.to_owned();
                move |_, domain| {
                    domain_resolved.lock().unwrap().push(domain.to_owned());
                    Ok(())
                }
            })
            .on_after_request_signed({
                let urls_visited = urls_visited.to_owned();
                move |context| {
                    urls_visited.lock().unwrap().push(context.url().to_string());
                    Ok(())
                }
            })
            .call()
            .unwrap_err();
        assert_eq!(err.kind(), ResponseErrorKind::from(HttpResponseErrorKind::ConnectError));
        let domain_resolved = Arc::try_unwrap(domain_resolved).unwrap().into_inner().unwrap();
        assert_eq!(
            &domain_resolved,
            &[
                "fakedomain.withoutport.com".to_owned(),
                "fakedomain.withport.com".to_owned()
            ]
        );
        let urls_visited = Arc::try_unwrap(urls_visited).unwrap().into_inner().unwrap();
        assert_eq!(
            &urls_visited,
            &[
                "https://fakedomain.withoutport.com/".to_owned(),
                "https://fakedomain.withport.com:8080/".to_owned(),
                "https://192.168.1.1/".to_owned(),
                "https://[::ffff:192.10.2.255]/".to_owned(),
                "https://[::ffff:192.11.2.255]:8081/".to_owned(),
                "https://192.168.1.2:8080/".to_owned(),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_call_endpoints_selection_without_resolver() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let client =
            make_error_response_client_builder(HttpResponseErrorKind::ConnectError, "Fake Connect Error", false)
                .chooser(DirectChooser)
                .resolver(make_dumb_resolver())
                .request_retrier(ErrorRetrier)
                .backoff(NO_BACKOFF)
                .build();

        let urls_visited = Arc::new(Mutex::new(Vec::new()));
        let err = client
            .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
            .on_to_resolve_domain(|_, _| unreachable!())
            .on_domain_resolved(|_, _, _| unreachable!())
            .on_after_request_signed({
                let urls_visited = urls_visited.to_owned();
                move |context| {
                    urls_visited.lock().unwrap().push(context.url().to_string());
                    Ok(())
                }
            })
            .call()
            .unwrap_err();
        assert_eq!(err.kind(), ResponseErrorKind::from(HttpResponseErrorKind::ConnectError));
        let urls_visited = Arc::try_unwrap(urls_visited).unwrap().into_inner().unwrap();
        assert_eq!(
            &urls_visited,
            &[
                "https://fakedomain.withoutport.com/".to_owned(),
                "https://fakedomain.withport.com:8080/".to_owned(),
                "https://192.168.1.1/".to_owned(),
                "https://[::ffff:192.10.2.255]/".to_owned(),
                "https://[::ffff:192.11.2.255]:8081/".to_owned(),
                "https://192.168.1.2:8080/".to_owned(),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_call_all_frozen_endpoints_selection() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let err = ResponseError::new_with_msg(HttpResponseErrorKind::ConnectError.into(), "Fake Connect Error");
        let chooser = IpChooser::default();
        chooser.feedback(
            ChooserFeedback::builder(&[
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)).into(),
                IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00a, 0x2ff)).into(),
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 2), 8080)).into(),
                SocketAddr::V6(SocketAddrV6::new(
                    Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc00b, 0x2ff),
                    8081,
                    0,
                    0,
                ))
                .into(),
            ])
            .error(&err)
            .build(),
        );

        let client =
            make_error_response_client_builder(HttpResponseErrorKind::ConnectError, "Fake Connect Error", true)
                .backoff(NO_BACKOFF)
                .resolver(make_random_resolver())
                .chooser(chooser)
                .request_retrier(ErrorRetrier)
                .build();

        let urls_visited = Arc::new(Mutex::new(Vec::new()));
        let domain_resolved = Arc::new(Mutex::new(Vec::new()));
        let err = client
            .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
            .on_to_resolve_domain({
                let domain_resolved = domain_resolved.to_owned();
                move |_, domain| {
                    domain_resolved.lock().unwrap().push(domain.to_owned());
                    Ok(())
                }
            })
            .on_after_request_signed({
                let urls_visited = urls_visited.to_owned();
                move |context| {
                    urls_visited.lock().unwrap().push(context.url().to_string());
                    Ok(())
                }
            })
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::HttpError(HttpResponseErrorKind::ConnectError)
        );
        let domain_resolved = Arc::try_unwrap(domain_resolved).unwrap().into_inner().unwrap();
        assert_eq!(
            &domain_resolved,
            &[
                "fakedomain.withoutport.com".to_owned(),
                "fakedomain.withport.com".to_owned()
            ]
        );
        let urls_visited = Arc::try_unwrap(urls_visited).unwrap().into_inner().unwrap();
        assert_eq!(
            &urls_visited,
            &[
                "https://fakedomain.withoutport.com/".to_owned(),
                "https://fakedomain.withport.com:8080/".to_owned(),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_call_switch_to_alternative_endpoints() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let client = make_error_response_client_builder(HttpResponseErrorKind::ServerCertError, "Fake SSL Error", true)
            .resolver(make_random_resolver())
            .chooser(DirectChooser)
            .backoff(NO_BACKOFF)
            .request_retrier(ErrorRetrier)
            .build();

        let urls_visited = Arc::new(Mutex::new(Vec::new()));
        let domain_resolved = Arc::new(Mutex::new(Vec::new()));
        let err = client
            .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
            .on_before_backoff(|_, _| panic!("Should not retry"))
            .on_to_resolve_domain({
                let domain_resolved = domain_resolved.to_owned();
                move |_, domain| {
                    domain_resolved.lock().unwrap().push(domain.to_owned());
                    Ok(())
                }
            })
            .on_after_request_signed({
                let urls_visited = urls_visited.to_owned();
                let retried = Arc::new(AtomicUsize::new(0));
                move |context| {
                    let retried = retried.fetch_add(1, Relaxed);
                    urls_visited.lock().unwrap().push(context.url().to_string());
                    assert_eq!(context.retried().retried_total(), retried);
                    assert_eq!(context.retried().retried_on_current_endpoint(), 0);
                    assert_eq!(context.retried().retried_on_current_ips(), 0);
                    assert_eq!(context.retried().abandoned_endpoints(), retried);
                    if retried > 0 {
                        assert!(context.retried().switched_to_alternative_endpoints());
                    } else {
                        assert!(!context.retried().switched_to_alternative_endpoints());
                    }
                    Ok(())
                }
            })
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HttpResponseErrorKind::ServerCertError)
        );
        let domain_resolved = Arc::try_unwrap(domain_resolved).unwrap().into_inner().unwrap();
        assert_eq!(
            &domain_resolved,
            &[
                "fakedomain.withoutport.com".to_owned(),
                "alternative_fakedomain.withoutport.com".to_owned(),
                "alternative_fakedomain.withport.com".to_owned(),
            ]
        );
        let urls_visited = Arc::try_unwrap(urls_visited).unwrap().into_inner().unwrap();
        assert_eq!(
            &urls_visited,
            &[
                "https://fakedomain.withoutport.com/".to_owned(),
                "https://alternative_fakedomain.withoutport.com/".to_owned(),
                "https://alternative_fakedomain.withport.com:8080/".to_owned(),
                "https://192.168.2.1/".to_owned(),
                "https://[::ffff:208.10.2.255]/".to_owned(),
                "https://[::ffff:208.11.2.255]:8081/".to_owned(),
                "https://192.168.2.2:8080/".to_owned(),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_call_single_endpoint_retry() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let always_retry_client =
            make_error_response_client_builder(HttpResponseErrorKind::TimeoutError, "Fake Timeout Error", true)
                .resolver(make_random_resolver())
                .chooser(DirectChooser)
                .backoff(NO_BACKOFF)
                .request_retrier(LimitedRetrier::new(ErrorRetrier, 3))
                .build();

        let retried = Arc::new(AtomicUsize::new(0));
        let err = always_retry_client
            .post(&[ServiceName::Up], &single_up_domain_endpoint())
            .on_before_backoff({
                let retried = retried.to_owned();
                move |context, _| {
                    assert_eq!("https://fakedomain.withport.com:8080/", &context.url().to_string(),);
                    let retried = retried.fetch_add(1, Relaxed) + 1;
                    assert_eq!(context.retried().retried_total(), retried);
                    assert_eq!(context.retried().retried_on_current_endpoint(), retried);
                    assert_eq!(context.retried().retried_on_current_ips(), retried);
                    assert_eq!(context.retried().abandoned_endpoints(), 0);
                    Ok(())
                }
            })
            .call()
            .unwrap_err();
        assert_eq!(err.kind(), ResponseErrorKind::from(HttpResponseErrorKind::TimeoutError));

        let headers = {
            let mut headers = HeaderMap::default();
            headers.insert(X_REQ_ID_HEADER_NAME, HeaderValue::from_static("fake_req_id"));
            headers.insert(X_LOG_HEADER_NAME, HeaderValue::from_static("fake_log"));
            headers
        };
        let always_throttled_client = make_fixed_response_client_builder(
            StatusCode::from_u16(509)?,
            headers,
            b"{\"error\":\"Fake Throttled Error\"}".to_vec(),
            true,
        )
        .resolver(make_random_resolver())
        .chooser(DirectChooser)
        .backoff(NO_BACKOFF)
        .request_retrier(LimitedRetrier::new(ErrorRetrier, 3))
        .build();

        let err = always_throttled_client
            .post(&[ServiceName::Up], &single_up_domain_endpoint())
            .on_before_backoff({
                retried.store(0, Relaxed);
                move |context, _| {
                    assert_eq!("https://fakedomain.withport.com:8080/", &context.url().to_string(),);
                    let retried = retried.fetch_add(1, Relaxed) + 1;
                    assert_eq!(context.retried().retried_total(), retried);
                    assert_eq!(context.retried().retried_on_current_endpoint(), retried);
                    assert_eq!(context.retried().retried_on_current_ips(), retried);
                    assert_eq!(context.retried().abandoned_endpoints(), 0);
                    Ok(())
                }
            })
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::StatusCodeError(StatusCode::from_u16(509)?)
        );
        assert_eq!(err.x_reqid(), Some(&HeaderValue::from_static("fake_req_id")));
        assert_eq!(err.x_log(), Some(&HeaderValue::from_static("fake_log")));
        assert!(err.to_string().ends_with("Fake Throttled Error"));

        Ok(())
    }

    #[test]
    fn test_call_retry_with_extensions() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        #[derive(Clone, Default)]
        struct ExtensionCounter(Arc<AtomicUsize>);

        impl ExtensionCounter {
            fn inc(&mut self) -> usize {
                self.0.fetch_add(1, Relaxed)
            }

            fn into_inner(self) -> usize {
                Arc::try_unwrap(self.0).unwrap().into_inner()
            }
        }

        let counter = ExtensionCounter::default();
        let err = make_error_response_client_builder(HttpResponseErrorKind::TimeoutError, "Fake Timeout Error", true)
            .resolver(make_random_resolver())
            .chooser(DirectChooser)
            .backoff(NO_BACKOFF)
            .request_retrier(LimitedRetrier::new(ErrorRetrier, 3))
            .build()
            .post(&[ServiceName::Up], &single_up_domain_endpoint())
            .add_extension(counter.to_owned())
            .on_to_resolve_domain(move |context, _| inc_extensions(context.extensions_mut()))
            .on_domain_resolved(move |context, _, _| inc_extensions(context.extensions_mut()))
            .on_to_choose_ips(move |context, _| inc_extensions(context.extensions_mut()))
            .on_ips_chosen(move |context, _, _| inc_extensions(context.extensions_mut()))
            .on_before_request_signed(move |context| inc_extensions(context.extensions_mut()))
            .on_after_request_signed(move |context| inc_extensions(context.extensions_mut()))
            .on_before_backoff(move |context, _| inc_extensions(context.extensions_mut()))
            .on_after_backoff(move |context, _| inc_extensions(context.extensions_mut()))
            .on_response(move |context, _| inc_extensions(context.extensions_mut()))
            .on_error(move |context, _| inc_extensions(context.extensions_mut()))
            .call()
            .unwrap_err();
        assert_eq!(err.kind(), ResponseErrorKind::from(HttpResponseErrorKind::TimeoutError));
        assert_eq!(counter.into_inner(), 18);

        let counter = ExtensionCounter::default();
        let err =
            make_error_response_client_builder(HttpResponseErrorKind::ServerCertError, "Fake Server Cert Error", true)
                .resolver(make_random_resolver())
                .chooser(DirectChooser)
                .backoff(NO_BACKOFF)
                .request_retrier(LimitedRetrier::new(ErrorRetrier, 3))
                .build()
                .post(&[ServiceName::Up], &single_up_domain_endpoint())
                .add_extension(counter.to_owned())
                .on_to_resolve_domain(move |context, _| inc_extensions(context.extensions_mut()))
                .on_domain_resolved(move |context, _, _| inc_extensions(context.extensions_mut()))
                .on_to_choose_ips(move |context, _| inc_extensions(context.extensions_mut()))
                .on_ips_chosen(move |context, _, _| inc_extensions(context.extensions_mut()))
                .on_before_request_signed(move |context| inc_extensions(context.extensions_mut()))
                .on_after_request_signed(move |context| inc_extensions(context.extensions_mut()))
                .on_before_backoff(move |_, _| unreachable!())
                .on_after_backoff(move |_, _| unreachable!())
                .on_response(move |context, _| inc_extensions(context.extensions_mut()))
                .on_error(move |context, _| inc_extensions(context.extensions_mut()))
                .call()
                .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HttpResponseErrorKind::ServerCertError)
        );
        assert_eq!(counter.into_inner(), 7);

        return Ok(());

        fn inc_extensions(extensions: &mut Extensions) -> AnyResult<()> {
            extensions.get_mut::<ExtensionCounter>().unwrap().inc();
            Ok(())
        }
    }

    #[test]
    fn test_call_retry_next() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let always_try_next_client = make_error_response_client_builder(
            HttpResponseErrorKind::UnknownHostError,
            "Test Unknown Host Error",
            true,
        )
        .resolver(make_random_resolver())
        .chooser(DirectChooser)
        .backoff(NO_BACKOFF)
        .request_retrier(LimitedRetrier::new(ErrorRetrier, 3))
        .build();

        let retry_urls = [
            "https://fakedomain.withoutport.com/".to_owned(),
            "https://fakedomain.withport.com:8080/".to_owned(),
            "https://192.168.1.1/".to_owned(),
            "https://[::ffff:192.10.2.255]/".to_owned(),
            "https://[::ffff:192.11.2.255]:8081/".to_owned(),
            "https://192.168.1.2:8080/".to_owned(),
        ];
        let err = always_try_next_client
            .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
            .on_before_backoff({
                let retried = Arc::new(AtomicUsize::new(0));
                move |context, _| {
                    let retried = retried.fetch_add(1, Relaxed);
                    assert_eq!(&context.url().to_string(), retry_urls.get(retried).unwrap());
                    assert_eq!(context.retried().retried_total(), retried + 1);
                    assert_eq!(context.retried().retried_on_current_endpoint(), 1);
                    assert_eq!(context.retried().retried_on_current_ips(), 1);
                    assert_eq!(context.retried().abandoned_endpoints(), retried);
                    Ok(())
                }
            })
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HttpResponseErrorKind::UnknownHostError)
        );

        Ok(())
    }

    #[test]
    fn test_call_dont_retry() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let always_dont_retry_client =
            make_error_response_client_builder(HttpResponseErrorKind::LocalIoError, "Test Local IO Error", true)
                .resolver(make_random_resolver())
                .chooser(DirectChooser)
                .build();

        let err = always_dont_retry_client
            .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
            .on_before_backoff(|_, _| panic!("Should never retry"))
            .on_after_request_signed(|context| {
                assert_eq!(&context.url().to_string(), "https://fakedomain.withoutport.com/");
                assert_eq!(context.retried().retried_total(), 0);
                Ok(())
            })
            .call()
            .unwrap_err();
        assert_eq!(err.kind(), ResponseErrorKind::from(HttpResponseErrorKind::LocalIoError));

        Ok(())
    }

    #[test]
    fn test_call_request_signature() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let always_retry_client =
            make_error_response_client_builder(HttpResponseErrorKind::SendError, "Test Send Error", true)
                .resolver(make_random_resolver())
                .chooser(DirectChooser)
                .backoff(NO_BACKOFF)
                .request_retrier(LimitedRetrier::new(ErrorRetrier, 3))
                .build();
        let credential = Credential::new("abcdefghklmnopq", "012345678901234567890");
        let signed_urls = Arc::new(Mutex::new(HashSet::new()));

        {
            let err = always_retry_client
                .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
                .authorization(Authorization::v2(credential.to_owned()))
                .on_before_request_signed(|context| {
                    assert!(context.headers().get("authorization").is_none());
                    Ok(())
                })
                .on_after_request_signed({
                    let signed_urls = signed_urls.to_owned();
                    move |context| {
                        signed_urls.lock().unwrap().insert(context.url().to_string());
                        assert!(context
                            .headers()
                            .get("authorization")
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .starts_with("Qiniu "));
                        Ok(())
                    }
                })
                .call()
                .unwrap_err();
            assert_eq!(err.kind(), ResponseErrorKind::from(HttpResponseErrorKind::SendError));
        }

        {
            signed_urls.lock().unwrap().clear();
            let err = always_retry_client
                .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
                .authorization(Authorization::v1(credential))
                .on_before_request_signed(|context| {
                    assert!(context.headers().get("authorization").is_none());
                    Ok(())
                })
                .on_after_request_signed({
                    move |context| {
                        signed_urls.lock().unwrap().insert(context.url().to_string());
                        assert!(context
                            .headers()
                            .get("authorization")
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .starts_with("QBox "));
                        Ok(())
                    }
                })
                .call()
                .unwrap_err();
            assert_eq!(err.kind(), ResponseErrorKind::from(HttpResponseErrorKind::SendError));
        }

        Ok(())
    }

    #[test]
    fn test_call_malicious_response() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let always_malicious_client = make_fixed_response_client_builder(
            StatusCode::from_u16(200)?,
            Default::default(),
            b"<p>Hello world!</p>".to_vec(),
            true,
        )
        .resolver(make_random_resolver())
        .chooser(DirectChooser)
        .backoff(NO_BACKOFF)
        .request_retrier(LimitedRetrier::new(ErrorRetrier, 3))
        .build();

        let retried_times = Arc::new(Mutex::new(HashMap::<String, AtomicUsize>::new()));
        let err = always_malicious_client
            .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
            .on_before_backoff({
                let retried_times = retried_times.to_owned();
                move |context, _| {
                    retried_times
                        .lock()
                        .unwrap()
                        .entry(context.url().to_string())
                        .and_modify(|t| {
                            t.fetch_add(1, Relaxed);
                        })
                        .or_insert_with(|| AtomicUsize::new(1));
                    Ok(())
                }
            })
            .call()
            .unwrap_err();
        assert_eq!(err.kind(), ResponseErrorKind::MaliciousResponse);
        assert_eq!(err.response_body_sample(), "<p>Hello world!</p>".as_bytes());

        let urls = [
            "https://fakedomain.withoutport.com/",
            "https://fakedomain.withport.com:8080/",
            "https://192.168.1.1/",
            "https://[::ffff:192.10.2.255]/",
            "https://[::ffff:192.11.2.255]:8081/",
            "https://192.168.1.2:8080/",
        ];
        let retried_times = Arc::try_unwrap(retried_times).unwrap().into_inner().unwrap();
        for &url in urls.iter() {
            assert_eq!(retried_times.get(url).map(|e| e.load(Relaxed)).unwrap(), 4);
        }

        Ok(())
    }

    #[test]
    fn test_call_unexpected_redirection() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let headers = {
            let mut headers = HeaderMap::new();
            headers.insert(
                "location",
                HeaderValue::from_static("https://another-fakedomain.withoutport.com/"),
            );
            headers.insert(X_REQ_ID_HEADER_NAME, HeaderValue::from_static("fake_req_id"));
            headers
        };
        let always_redirected_client = make_fixed_response_client_builder(
            StatusCode::from_u16(301)?,
            headers,
            b"<p>Hello world!</p>".to_vec(),
            true,
        )
        .resolver(make_random_resolver())
        .chooser(DirectChooser)
        .backoff(NO_BACKOFF)
        .request_retrier(LimitedRetrier::new(ErrorRetrier, 3))
        .build();

        let err = always_redirected_client
            .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
            .on_before_backoff(|_, _| panic!("Should never retry"))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::UnexpectedStatusCode(StatusCode::from_u16(301)?)
        );

        Ok(())
    }

    #[test]
    fn test_cancel_in_callbacks() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let client =
            make_error_response_client_builder(HttpResponseErrorKind::ConnectError, "Fake Connect Error", true)
                .resolver(make_dumb_resolver())
                .chooser(DirectChooser)
                .request_retrier(ErrorRetrier)
                .backoff(NO_BACKOFF)
                .build();

        let err = client
            .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
            .on_before_request_signed(|_| Err(AnyError::msg("Fake error")))
            .on_before_backoff(|_, _| panic!("Should not retry"))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HttpResponseErrorKind::CallbackError)
        );

        let err = client
            .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
            .on_after_request_signed(|_| Err(AnyError::msg("Fake error")))
            .on_before_backoff(|_, _| panic!("Should not retry"))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HttpResponseErrorKind::CallbackError)
        );

        let err = client
            .post(&[ServiceName::Up], chaotic_up_domains_endpoint())
            .on_before_backoff(|_, _| Err(AnyError::msg("Fake error")))
            .on_after_backoff(|_, _| panic!("Should not retry"))
            .call()
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::from(HttpResponseErrorKind::CallbackError)
        );
        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "async")]
    async fn test_async_call_single_endpoint_retry() -> Result<(), Box<dyn Error>> {
        env_logger::builder().is_test(true).try_init().ok();

        let always_retry_client =
            make_error_response_client_builder(HttpResponseErrorKind::TimeoutError, "Fake Timeout Error", true)
                .resolver(make_random_resolver())
                .chooser(DirectChooser)
                .backoff(NO_BACKOFF)
                .request_retrier(LimitedRetrier::new(ErrorRetrier, 3))
                .build();

        let retried = Arc::new(AtomicUsize::new(0));
        let err = always_retry_client
            .async_post(&[ServiceName::Up], &single_up_domain_endpoint())
            .on_before_backoff({
                let retried = retried.to_owned();
                move |context, _| {
                    assert_eq!("https://fakedomain.withport.com:8080/", &context.url().to_string());
                    let retried = retried.fetch_add(1, Relaxed) + 1;
                    assert_eq!(context.retried().retried_total(), retried);
                    assert_eq!(context.retried().retried_on_current_endpoint(), retried);
                    assert_eq!(context.retried().retried_on_current_ips(), retried);
                    assert_eq!(context.retried().abandoned_endpoints(), 0);
                    Ok(())
                }
            })
            .call()
            .await
            .unwrap_err();
        assert_eq!(err.kind(), ResponseErrorKind::from(HttpResponseErrorKind::TimeoutError));

        let headers = {
            let mut headers = HeaderMap::default();
            headers.insert(X_REQ_ID_HEADER_NAME, HeaderValue::from_static("fake_req_id"));
            headers
        };
        let always_throttled_client = make_fixed_response_client_builder(
            StatusCode::from_u16(509)?,
            headers.to_owned(),
            b"{\"error\":\"Fake Throttled Error\"}".to_vec(),
            true,
        )
        .resolver(make_random_resolver())
        .chooser(DirectChooser)
        .backoff(NO_BACKOFF)
        .request_retrier(LimitedRetrier::new(ErrorRetrier, 3))
        .build();

        retried.store(0, Relaxed);
        let err = always_throttled_client
            .async_post(&[ServiceName::Up], &single_up_domain_endpoint())
            .on_before_backoff({
                let retried = retried.to_owned();
                move |context, _| {
                    assert_eq!("https://fakedomain.withport.com:8080/", &context.url().to_string(),);
                    let retried = retried.fetch_add(1, Relaxed) + 1;
                    assert_eq!(context.retried().retried_total(), retried);
                    assert_eq!(context.retried().retried_on_current_endpoint(), retried);
                    assert_eq!(context.retried().retried_on_current_ips(), retried);
                    assert_eq!(context.retried().abandoned_endpoints(), 0);
                    Ok(())
                }
            })
            .call()
            .await
            .unwrap_err();
        assert_eq!(
            err.kind(),
            ResponseErrorKind::StatusCodeError(StatusCode::from_u16(509)?)
        );
        assert!(err.to_string().ends_with("Fake Throttled Error"));

        Ok(())
    }
}
