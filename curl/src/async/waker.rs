use futures::task::{waker, ArcWake};
use std::{
    io::Result as IOResult,
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    sync::Arc,
    task::Waker,
};

fn waker_fn(f: impl Fn() + Send + Sync + 'static) -> Waker {
    struct Impl<F>(F);

    impl<F: Fn() + Send + Sync + 'static> ArcWake for Impl<F> {
        #[inline]
        fn wake_by_ref(arc_self: &Arc<Self>) {
            (&arc_self.0)()
        }
    }

    waker(Arc::new(Impl(f)))
}

pub(super) trait WakerExt {
    fn chain(&self, f: impl Fn(&Waker) + Send + Sync + 'static) -> Waker;
}

impl WakerExt for Waker {
    #[inline]
    fn chain(&self, f: impl Fn(&Waker) + Send + Sync + 'static) -> Waker {
        let inner = self.clone();
        waker_fn(move || (f)(&inner))
    }
}

pub(super) struct UdpWaker {
    socket: UdpSocket,
}

impl UdpWaker {
    #[inline]
    pub(super) fn connect(addr: SocketAddr) -> IOResult<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::new(127, 0, 0, 1), 0))?;
        socket.set_nonblocking(true)?;
        socket.connect(addr)?;

        Ok(Self { socket })
    }
}

impl ArcWake for UdpWaker {
    #[inline]
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.socket.send(&[1]).ok();
    }
}
