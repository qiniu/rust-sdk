use super::{
    super::{CurlHTTPCaller, MultiOptions},
    agent_context::{AgentContext, AgentError, AgentResult, Message},
    single_request_context::SingleRequestContext,
    waker::UdpWaker,
};
use crossbeam_channel::{unbounded, Sender};
use crossbeam_utils::sync::WaitGroup;
use curl::{easy::Easy2, init as curl_init, multi::Multi, MultiError};
use futures::task::waker;
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    io::Result as IOResult,
    mem::{drop, transmute},
    net::{Ipv4Addr, UdpSocket},
    result::Result,
    sync::{Arc, Mutex},
    task::Waker,
    thread::{Builder as ThreadBuilder, JoinHandle},
};

static GLOBAL_HANDLERS: Lazy<Mutex<HashMap<MultiOptions, Arc<Handler>>>> =
    Lazy::new(Default::default);

pub(super) struct Handler {
    tx: Sender<Message>,
    waker: Waker,
    join_handle: Mutex<Option<JoinHandle<AgentResult<()>>>>,
}

pub(super) fn spawn(client: &CurlHTTPCaller) -> IOResult<Arc<Handler>> {
    let multi_options = client.clone_multi_options();
    let mut cache = GLOBAL_HANDLERS.lock().unwrap();
    if let Some(handler) = cache.get(&multi_options).cloned() {
        Ok(handler)
    } else {
        let handler = spawn_new(client)?;
        cache.insert(multi_options, handler.to_owned());
        Ok(handler)
    }
}

fn spawn_new(client: &CurlHTTPCaller) -> IOResult<Arc<Handler>> {
    curl_init();

    let multi_options = client.clone_multi_options();
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
            let wait_group = WaitGroup::new();
            ThreadBuilder::new()
                .name(format!("qiniu-curl/{}", port))
                .spawn(move || {
                    let mut multi = Multi::new();
                    set_multi_options(&mut multi, &multi_options)?;
                    let agent = AgentContext::new(multi, tx, rx, wake_socket, waker);
                    drop(wait_group);
                    agent.run()
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
