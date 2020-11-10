use super::{
    super::{CurlHTTPCaller, MultiOptions},
    agent_context::{AgentContext, AgentError, AgentResult, Message},
    single_request_context::SingleRequestContext,
    waker::UdpWaker,
};
use async_std::task::spawn as async_spawn;
use crossbeam_channel::{unbounded, Sender};
use crossbeam_utils::sync::WaitGroup;
use curl::{easy::Easy2, init as curl_init, multi::Multi, MultiError};
use dashmap::DashMap;
use futures::task::waker;
use log::{error, info};
use once_cell::sync::Lazy;
use std::{
    io::Result as IOResult,
    mem::{drop, transmute},
    net::{Ipv4Addr, UdpSocket},
    result::Result,
    sync::{Arc, Mutex},
    task::Waker,
    thread::{Builder as ThreadBuilder, JoinHandle},
};
use tap::tap::TapFallible;

static GLOBAL_HANDLERS: Lazy<DashMap<MultiOptions, Arc<Handler>>> = Lazy::new(Default::default);

pub(super) struct Handler {
    tx: Sender<Message>,
    waker: Waker,
    join_handle: Mutex<Option<JoinHandle<AgentResult<()>>>>,
}

pub(super) async fn spawn(client: &CurlHTTPCaller) -> IOResult<Arc<Handler>> {
    let multi_options = client.clone_multi_options();
    let handler = async_spawn::<_, IOResult<Arc<Handler>>>(async move {
        let handler = GLOBAL_HANDLERS
            .entry(multi_options.to_owned())
            .or_try_insert_with(|| spawn_new(multi_options))?
            .to_owned();
        Ok(handler)
    })
    .await?;
    Ok(handler)
}

fn spawn_new(multi_options: MultiOptions) -> IOResult<Arc<Handler>> {
    curl_init();

    let wake_socket = UdpSocket::bind((Ipv4Addr::new(127, 0, 0, 1), 0))?;
    wake_socket.set_nonblocking(true)?;
    let wake_addr = wake_socket.local_addr()?;
    let port = wake_addr.port();
    let waker = waker(Arc::new(UdpWaker::connect(wake_addr)?));

    let (tx, rx) = unbounded();
    let wait_group = WaitGroup::new();

    let handler = Arc::new(Handler {
        tx: tx.to_owned(),
        waker: waker.to_owned(),
        join_handle: Mutex::new(Some({
            let wait_group = wait_group.to_owned();
            ThreadBuilder::new()
                .name(format!("qiniu.rust-sdk.curl.async.Handler/{}", port))
                .spawn(move || {
                    let mut multi = Multi::new();
                    set_multi_options(&mut multi, &multi_options).tap_err(|err| {
                        error!("Failed to set multi_options: {}", err);
                    })?;
                    info!("A new AgentContext started");
                    let agent = AgentContext::new(multi, tx, rx, wake_socket, waker);
                    drop(wait_group);
                    agent.run().tap_err(|err| {
                        error!("AgentContext is quit because of error: {}", err);
                    })
                })?
        })),
    });

    wait_group.wait();
    return Ok(handler);

    fn set_multi_options(multi: &mut Multi, opts: &MultiOptions) -> Result<(), MultiError> {
        if opts.max_connections > 0 {
            multi.set_max_total_connections(opts.max_connections)?;
        }
        if opts.max_connections_per_host > 0 {
            multi.set_max_host_connections(opts.max_connections_per_host)?;
        }
        if opts.connection_cache_size > 0 {
            multi.set_max_connects(opts.connection_cache_size)?;
        }
        let http_1_pipelining = opts.http_1_pipelining_length > 0;
        if http_1_pipelining {
            multi.pipelining(http_1_pipelining, opts.http_2_multiplexing)?;
            multi.set_pipeline_length(opts.http_1_pipelining_length)?;
        } else {
            multi.pipelining(http_1_pipelining, opts.http_2_multiplexing)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
enum JoinResult {
    AlreadyJoined,
    Ok,
    Err(AgentError),
    Panic,
}

impl Handler {
    #[inline]
    pub(super) fn submit_request(&self, easy: Easy2<SingleRequestContext>) {
        self.send_message(Message::Execute(unsafe { transmute(easy) }))
    }

    fn send_message(&self, message: Message) {
        match self.tx.send(message) {
            Ok(()) => self.waker.wake_by_ref(),
            Err(_) => match self.try_join() {
                JoinResult::Err(e) => panic!("agent thread terminated with error: {}", e),
                JoinResult::Panic => panic!("agent thread panicked"),
                _ => panic!("agent thread terminated prematurely"),
            },
        }
    }

    fn try_join(&self) -> JoinResult {
        let mut option = self.join_handle.lock().unwrap();

        if let Some(join_handle) = option.take() {
            match join_handle.join() {
                Ok(Ok(())) => JoinResult::Ok,
                Ok(Err(e)) => JoinResult::Err(e),
                Err(_) => JoinResult::Panic,
            }
        } else {
            JoinResult::AlreadyJoined
        }
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        self.send_message(Message::Close);
        match self.try_join() {
            JoinResult::Err(e) => panic!("agent thread terminated with error: {}", e),
            JoinResult::Panic => panic!("agent thread panicked"),
            _ => {}
        }
    }
}
