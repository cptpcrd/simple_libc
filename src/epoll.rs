use std::io;
use std::time;
use libc;
use bitflags::bitflags;


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
    fn from(i: u32) -> Events {
        Events::from_bits_truncate(i as i32)
    }
}

impl From<Events> for u32 {
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
    fn default() -> Event {
        Event {
            events: Events::empty(),
            data: 0,
        }
    }
}

impl From<libc::epoll_event> for Event {
    fn from(ev: libc::epoll_event) -> Event {
        Event {
            events: Events::from(ev.events),
            data: ev.u64,
        }
    }
}

impl From<Event> for libc::epoll_event {
    fn from(ev: Event) -> libc::epoll_event {
        libc::epoll_event {
            events: u32::from(ev.events),
            u64: ev.data,
        }
    }
}

pub struct Epoll {
    fd: libc::c_int,
}

impl Epoll {
    pub fn new(close_on_exec: bool) -> io::Result<Epoll> {
        let mut flags: libc::c_int = 0;
        if close_on_exec {
            flags |= libc::EPOLL_CLOEXEC;
        }

        let fd = unsafe {
            libc::epoll_create1(flags)
        };

        super::error::convert_neg_ret(fd).map(|fd| {
            Epoll { fd: fd }
        })
    }

    #[inline]
    pub fn fd(&self) -> i32 {
        self.fd as i32
    }


    fn ctl(&self, op: CtlOp, fd: i32, events: Events, data: u64) -> io::Result<()> {
        let mut ep_event = libc::epoll_event{events: u32::from(events), u64: data};

        super::error::convert(unsafe {
            libc::epoll_ctl(self.fd, op as libc::c_int, fd as libc::c_int, &mut ep_event)
        }, ())
    }

    #[inline]
    pub fn del(&self, fd: i32) -> io::Result<()> {
        self.ctl(CtlOp::Del, fd, Events::empty(), 0)
    }


    #[inline]
    pub fn add(&self, fd: i32, events: Events) -> io::Result<()> {
        self.add3(fd, events, fd as u64)
    }

    #[inline]
    pub fn modify(&self, fd: i32, events: Events) -> io::Result<()> {
        self.modify3(fd, events, fd as u64)
    }


    #[inline]
    pub fn add2(&self, fd: i32, event: Event) -> io::Result<()> {
        self.add3(fd, event.events, event.data)
    }

    #[inline]
    pub fn modify2(&self, fd: i32, event: Event) -> io::Result<()> {
        self.modify3(fd, event.events, event.data)
    }


    #[inline]
    pub fn add3(&self, fd: i32, events: Events, data: u64) -> io::Result<()> {
        self.ctl(CtlOp::Add, fd, events, data)
    }

    #[inline]
    pub fn modify3(&self, fd: i32, events: Events, data: u64) -> io::Result<()> {
        self.ctl(CtlOp::Mod, fd, events, data)
    }


    pub fn pwait(&self, events: &mut [Event], timeout: Option<time::Duration>, sigmask: Option<&super::signal::Sigset>) -> io::Result<i32> {
        let maxevents = events.len();

        let raw_timeout = match timeout {
            Some(t) => t.as_millis() as i32,
            None => -1,
        };

        let raw_sigmask = match sigmask {
            Some(s) => &s.raw_set(),
            None => std::ptr::null(),
        };

        let mut ep_events: Vec<libc::epoll_event> = Vec::with_capacity(maxevents);

        unsafe { ep_events.set_len(maxevents) };

        super::error::convert_neg_ret(unsafe {
            libc::epoll_pwait(self.fd, ep_events.as_mut_ptr(), maxevents as i32, raw_timeout, raw_sigmask)
        }).map(|res| {
            for i in 0..(res as usize) {
                events[i] = Event::from(ep_events[i]);
            }
            return res
        })
    }

    pub fn wait(&self, events: &mut [Event], timeout: Option<time::Duration>) -> io::Result<i32> {
        self.pwait(events, timeout, None)
    }

    pub fn close(&self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        self.close();
    }
}