use std::convert::TryInto;
use std::io;
use std::time;

use bitflags::bitflags;

use super::Int;

enum CtlOp {
    Add = libc::EPOLL_CTL_ADD as isize,
    Mod = libc::EPOLL_CTL_MOD as isize,
    Del = libc::EPOLL_CTL_DEL as isize,
}

bitflags! {
    #[derive(Default)]
    pub struct Events: i32 {
        const IN = libc::EPOLLIN;
        const OUT = libc::EPOLLOUT;
        const ERR = libc::EPOLLERR;
        const ET = libc::EPOLLET;
        const HUP = libc::EPOLLHUP;
        const RDHUP = libc::EPOLLRDHUP;
        const ONESHOT = libc::EPOLLONESHOT;
        const WAKEUP = libc::EPOLLWAKEUP;
        const EXCLUSIVE = libc::EPOLLEXCLUSIVE;
    }
}

impl From<u32> for Events {
    #[inline]
    fn from(i: u32) -> Events {
        Events::from_bits_truncate(i as i32)
    }
}

impl From<Events> for u32 {
    #[inline]
    fn from(m: Events) -> u32 {
        m.bits() as u32
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Event {
    pub events: Events,
    pub data: u64,
}

impl Default for Event {
    #[inline]
    fn default() -> Event {
        Event {
            events: Events::empty(),
            data: 0,
        }
    }
}

impl From<libc::epoll_event> for Event {
    #[inline]
    fn from(ev: libc::epoll_event) -> Event {
        Event {
            events: Events::from(ev.events),
            data: ev.u64,
        }
    }
}

impl From<Event> for libc::epoll_event {
    #[inline]
    fn from(ev: Event) -> libc::epoll_event {
        libc::epoll_event {
            events: u32::from(ev.events),
            u64: ev.data,
        }
    }
}

bitflags! {
    #[derive(Default)]
    pub struct EpollFlags: Int {
        const CLOEXEC = libc::EPOLL_CLOEXEC;
    }
}

pub struct Epoll {
    fd: Int,
}

impl Epoll {
    pub fn new(flags: EpollFlags) -> io::Result<Epoll> {
        let fd = unsafe { libc::epoll_create1(flags.bits) };

        super::error::convert_neg_ret(fd).map(|fd| Epoll { fd })
    }

    #[inline]
    pub fn fd(&self) -> Int {
        self.fd
    }

    fn ctl(&mut self, op: CtlOp, fd: Int, events: Events, data: u64) -> io::Result<()> {
        let mut ep_event = libc::epoll_event {
            events: u32::from(events),
            u64: data,
        };

        super::error::convert(
            unsafe { libc::epoll_ctl(self.fd, op as Int, fd, &mut ep_event) },
            (),
        )
    }

    #[inline]
    pub fn del(&mut self, fd: Int) -> io::Result<()> {
        self.ctl(CtlOp::Del, fd, Events::empty(), 0)
    }

    #[inline]
    pub fn add(&mut self, fd: Int, events: Events) -> io::Result<()> {
        self.add3(fd, events, fd as u64)
    }

    #[inline]
    pub fn modify(&mut self, fd: Int, events: Events) -> io::Result<()> {
        self.modify3(fd, events, fd as u64)
    }

    #[inline]
    pub fn add2(&mut self, fd: Int, event: Event) -> io::Result<()> {
        self.add3(fd, event.events, event.data)
    }

    #[inline]
    pub fn modify2(&mut self, fd: Int, event: Event) -> io::Result<()> {
        self.modify3(fd, event.events, event.data)
    }

    #[inline]
    pub fn add3(&mut self, fd: Int, events: Events, data: u64) -> io::Result<()> {
        self.ctl(CtlOp::Add, fd, events, data)
    }

    #[inline]
    pub fn modify3(&mut self, fd: Int, events: Events, data: u64) -> io::Result<()> {
        self.ctl(CtlOp::Mod, fd, events, data)
    }

    pub fn pwait(
        &self,
        events: &mut [Event],
        timeout: Option<time::Duration>,
        sigmask: Option<&super::signal::Sigset>,
    ) -> io::Result<Int> {
        let maxevents = events.len();

        let raw_timeout: Int = match timeout {
            Some(t) => t.as_millis().try_into().unwrap_or(Int::MAX),
            None => -1,
        };

        let raw_sigmask = match sigmask {
            Some(s) => &s.raw_set(),
            None => std::ptr::null(),
        };

        let mut ep_events: Vec<libc::epoll_event> = Vec::new();

        ep_events.resize(maxevents, libc::epoll_event { events: 0, u64: 0 });

        super::error::convert_neg_ret(unsafe {
            libc::epoll_pwait(
                self.fd,
                ep_events.as_mut_ptr(),
                maxevents as Int,
                raw_timeout,
                raw_sigmask,
            )
        })
        .map(|res| {
            for i in 0..(res as usize) {
                events[i] = Event::from(ep_events[i]);
            }
            res
        })
    }

    #[inline]
    pub fn wait(&self, events: &mut [Event], timeout: Option<time::Duration>) -> io::Result<Int> {
        self.pwait(events, timeout, None)
    }
}

impl Drop for Epoll {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}
