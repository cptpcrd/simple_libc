use std::convert::TryInto;
use std::io;
use std::time;
use std::os::unix::io::{AsRawFd, RawFd};

use bitflags::bitflags;

use crate::Int;

#[derive(Debug, Copy, Clone)]
enum CtlOp {
    Add = libc::EPOLL_CTL_ADD as isize,
    Mod = libc::EPOLL_CTL_MOD as isize,
    Del = libc::EPOLL_CTL_DEL as isize,
}

bitflags! {
    #[derive(Default)]
    #[repr(transparent)]
    pub struct Events: u32 {
        const IN = libc::EPOLLIN as u32;
        const OUT = libc::EPOLLOUT as u32;
        const ERR = libc::EPOLLERR as u32;
        const ET = libc::EPOLLET as u32;
        const HUP = libc::EPOLLHUP as u32;
        const RDHUP = libc::EPOLLRDHUP as u32;
        const ONESHOT = libc::EPOLLONESHOT as u32;
        const WAKEUP = libc::EPOLLWAKEUP as u32;
        const EXCLUSIVE = libc::EPOLLEXCLUSIVE as u32;
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
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

bitflags! {
    #[derive(Default)]
    pub struct EpollFlags: Int {
        const CLOEXEC = libc::EPOLL_CLOEXEC;
    }
}

#[derive(Debug)]
pub struct Epoll {
    fd: Int,
}

impl Epoll {
    pub fn new(flags: EpollFlags) -> io::Result<Epoll> {
        let fd = unsafe { libc::epoll_create1(flags.bits) };

        crate::error::convert_neg_ret(fd).map(|fd| Epoll { fd })
    }

    #[deprecated(since = "0.5.0", note = "Use `as_raw_fd()` instead")]
    #[inline]
    pub fn fd(&self) -> Int {
        self.as_raw_fd()
    }

    fn ctl(&mut self, op: CtlOp, fd: Int, events: Events, data: u64) -> io::Result<()> {
        let mut ep_event = libc::epoll_event {
            events: events.bits as u32,
            u64: data,
        };

        crate::error::convert(
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
        sigmask: Option<crate::signal::Sigset>,
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

        let mut ep_events = Vec::new();

        ep_events.resize(maxevents, libc::epoll_event { events: 0, u64: 0 });

        crate::error::convert_neg_ret(unsafe {
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
                events[i] = Event {
                    events: Events::from_bits_truncate(ep_events[i].events),
                    data: ep_events[i].u64,
                };
            }
            res
        })
    }

    #[inline]
    pub fn wait(&self, events: &mut [Event], timeout: Option<time::Duration>) -> io::Result<Int> {
        self.pwait(events, timeout, None)
    }
}

impl AsRawFd for Epoll {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.fd
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    use crate::pipe2;

    #[test]
    fn test_epoll() {
        let mut poller = Epoll::new(EpollFlags::CLOEXEC).unwrap();
        let mut events = [Event::default(); 3];

        assert_eq!(poller.fd, poller.as_raw_fd());
        #[allow(deprecated)]
        {
            assert_eq!(poller.fd(), poller.as_raw_fd());
        }

        let (r1, mut w1) = pipe2(libc::O_CLOEXEC).unwrap();
        let (r2, mut w2) = pipe2(libc::O_CLOEXEC).unwrap();

        poller.add(r1.as_raw_fd(), Events::IN).unwrap();
        poller
            .add2(
                r2.as_raw_fd(),
                Event {
                    events: Events::IN,
                    data: w2.as_raw_fd() as u64,
                },
            )
            .unwrap();

        // Nothing to start
        assert_eq!(
            poller
                .wait(&mut events, Some(time::Duration::from_secs(0)))
                .unwrap(),
            0,
        );

        // Now we write some data and test again
        w1.write_all(b"a").unwrap();
        assert_eq!(
            poller
                .wait(&mut events, Some(time::Duration::from_secs(0)))
                .unwrap(),
            1,
        );
        assert_eq!(events[0].data, r1.as_raw_fd() as u64);
        assert_eq!(events[0].events, Events::IN);

        // Now make sure reading two files works
        w2.write_all(b"a").unwrap();
        assert_eq!(
            poller
                .wait(&mut events, Some(time::Duration::from_secs(0)))
                .unwrap(),
            2,
        );
        assert_eq!(events[0].data, r1.as_raw_fd() as u64);
        assert_eq!(events[0].events, Events::IN);
        assert_eq!(events[1].data, w2.as_raw_fd() as u64);
        assert_eq!(events[1].events, Events::IN);

        // Now remove one of the files
        poller.del(r1.as_raw_fd()).unwrap();
        assert_eq!(
            poller
                .wait(&mut events, Some(time::Duration::from_secs(0)))
                .unwrap(),
            1,
        );
        assert_eq!(events[1].data, w2.as_raw_fd() as u64);
        assert_eq!(events[1].events, Events::IN);
    }
}
