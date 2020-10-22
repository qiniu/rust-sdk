use super::{
    super::http::ResponseError,
    single_request_context::{handle_response, SingleRequestContext},
    waker::WakerExt,
};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use curl::{
    easy::Easy2,
    multi::{Easy2Handle, Multi, WaitFd},
    Error as CurlError, MultiError,
};
use log::info;
use slab::Slab;
use std::{fmt, net::UdpSocket, result::Result, task::Waker, time::Duration};
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub(super) struct RequestID(usize);

impl fmt::Display for RequestID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<usize> for RequestID {
    #[inline]
    fn from(id: usize) -> Self {
        Self(id)
    }
}

impl From<RequestID> for usize {
    #[inline]
    fn from(id: RequestID) -> Self {
        id.0
    }
}

#[derive(Debug)]
pub(super) enum Message {
    Close,
    Execute(Easy2<SingleRequestContext<'static>>),
    UnpauseRead(RequestID),
    UnpauseWrite(RequestID),
}

#[derive(Debug)]
pub(super) struct MultiMessage {
    id: RequestID,
    error: Option<CurlError>,
}

#[derive(Error, Debug)]
pub(super) enum AgentError {
    #[error("Curl API error: {0}")]
    Curl(#[from] CurlError),
    #[error("Curl Multi API error: {0}")]
    Multi(#[from] MultiError),
    #[error("Response error: {0}")]
    Response(#[from] ResponseError),
}
pub(super) type AgentResult<T> = Result<T, AgentError>;

pub(super) struct AgentContext<'ctx> {
    multi: Multi,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    wake_socket: UdpSocket,
    requests: Slab<Easy2Handle<SingleRequestContext<'ctx>>>,
    close_requested: bool,
    waker: Waker,
}

impl<'ctx> AgentContext<'ctx> {
    #[inline]
    pub(super) fn new(
        multi: Multi,
        tx: Sender<Message>,
        rx: Receiver<Message>,
        wake_socket: UdpSocket,
        waker: Waker,
    ) -> Self {
        AgentContext {
            multi,
            tx,
            rx,
            wake_socket,
            waker,
            requests: Default::default(),
            close_requested: false,
        }
    }

    pub(super) fn run(mut self) -> AgentResult<()> {
        let wait_fd = get_wait_fd(&self.wake_socket);
        let mut wait_fds = [wait_fd];
        let mut wait_fd_buf = [0; 1024];

        wait_fds[0].poll_on_read(true);

        loop {
            self.poll_messages()?;

            if self.close_requested {
                break;
            }

            self.perform()?;
            self.multi.wait(&mut wait_fds, Duration::from_millis(100))?;
            if wait_fds[0].received_read() {
                self.wake_socket.recv_from(&mut wait_fd_buf).ok();
            }
        }
        self.requests.clear();
        return Ok(());

        #[inline]
        fn get_wait_fd(socket: &UdpSocket) -> WaitFd {
            let mut wait_fd = WaitFd::new();

            #[cfg(unix)]
            {
                use std::os::unix::io::AsRawFd;
                wait_fd.set_fd(socket.as_raw_fd());
            }

            #[cfg(windows)]
            {
                use std::os::windows::io::AsRawSocket;
                wait_fd.set_fd(socket.as_raw_socket());
            }

            wait_fd
        }
    }

    fn poll_messages(&mut self) -> AgentResult<()> {
        while !self.close_requested {
            if self.requests.is_empty() {
                match self.rx.recv() {
                    Ok(message) => self.handle_message(message)?,
                    Err(err) => {
                        info!("AgentContext is failed to poll messages: {}", err);
                        self.close_requested = true;
                        break;
                    }
                }
            } else {
                match self.rx.try_recv() {
                    Ok(message) => self.handle_message(message)?,
                    Err(TryRecvError::Empty) => break,
                    Err(err) => {
                        info!("AgentContext is failed to try to poll messages: {}", err);
                        self.close_requested = true;
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_message(&mut self, message: Message) -> AgentResult<()> {
        match message {
            Message::Execute(request) => self.begin_request(request)?,
            Message::Close => {
                self.close_requested = true;
            }
            Message::UnpauseRead(request_id) => {
                if let Some(request) = self.requests.get(request_id.into()) {
                    info!("Request {} unpause read", request_id);
                    request.unpause_read()?;
                }
            }
            Message::UnpauseWrite(request_id) => {
                if let Some(request) = self.requests.get(request_id.into()) {
                    info!("Request {} unpause write", request_id);
                    request.unpause_write()?;
                }
            }
        }
        Ok(())
    }

    fn perform(&mut self) -> AgentResult<()> {
        self.multi.perform()?;

        let (tx, rx) = unbounded();

        self.multi.messages(move |message| {
            if let (Some(result), Ok(token)) = (message.result(), message.token()) {
                tx.send(MultiMessage {
                    id: token.into(),
                    error: result.err(),
                })
                .unwrap();
            }
        });

        while let Ok(multi_message) = rx.recv() {
            self.complete_request(multi_message)?;
        }

        Ok(())
    }

    fn begin_request(&mut self, mut request: Easy2<SingleRequestContext<'ctx>>) -> AgentResult<()> {
        let entry = self.requests.vacant_entry();
        let id = RequestID::from(entry.key());
        request.get_mut().set_wakers(
            {
                let tx = self.tx.clone();
                self.waker.chain(move |inner| {
                    if tx.send(Message::UnpauseRead(id)).is_ok() {
                        inner.wake_by_ref()
                    }
                })
            },
            {
                let tx = self.tx.clone();
                self.waker.chain(move |inner| {
                    if tx.send(Message::UnpauseWrite(id)).is_ok() {
                        inner.wake_by_ref()
                    }
                })
            },
        );

        let mut request = self.multi.add2(request)?;
        request.set_token(id.into())?;
        entry.insert(request);
        info!("Begin request, new request id: {}", id);
        Ok(())
    }

    fn complete_request(&mut self, message: MultiMessage) -> AgentResult<()> {
        info!("Request is completed, to remove request id: {}", message.id);
        let handle = self.requests.remove(message.id.into());
        let mut handle = self.multi.remove2(handle)?;
        handle_response(&mut handle, message.error);
        Ok(())
    }
}
