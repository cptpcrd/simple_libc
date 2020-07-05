use std::convert::TryInto;
use std::io;
use std::time;
use std::os::unix::prelude::*;

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

crate::attr_group! {
    #![cfg(target_arch = "x86_64")]

    /// The raw event representation. Read the documentation of `Event`
    /// for details.
    #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
    #[repr(C)]
    #[repr(packed)]
    pub struct RawEvent {
        pub events: Events,
        pub data: u64,
    }

    #[cfg(target_arch = "x86_64")]
    impl RawEvent {
        // WARNING: Do not use this!
        // Its purpose is to add a compile-time check that the size of a
        // RawEvent matches the size of a libc::epoll_event.
        unsafe fn _into_raw(&self) -> libc::epoll_event {
            std::mem::transmute(*self)
        }
    }

    #[cfg(target_arch = "x86_64")]
    impl Default for RawEvent {
        #[inline]
        fn default() -> Self {
            Self {
                events: Events::empty(),
                data: 0,
            }
        }
    }
}

/// An event that occurred on a file descriptor.
///
/// # `Event` vs `RawEvent`
///
/// **TL;DR**: If you want a quick 99% solution, use `Event` and
/// `Epoll::wait()`/`Epoll::pwait()`. If you've read this carefully, you understand
/// how to interact with an unpacked struct properly, and you want the *slight*
/// performance boost that `RawEvent` provides on x86_64, use `RawEvent` and
/// `Epoll::wait_raw()`/`Epoll::pwait_raw()`.
///
/// On x86_64, the `epoll_event` structure is *packed* to make 32-bit compatibility
/// easier. As a result, the version of this structure used to interact with the kernel
/// must be packed. However, since Rust is moving towards making borrows of packed
/// fields unsafe (see [issue 46043](https://github.com/rust-lang/rust/issues/46043)
/// for details), this struct is difficult to use directly in a safe manner.
///
/// As a result, this module exposes two structures, `RawEvent` and `Event`.
/// `RawEvent` is the structure passed directly to the kernel, which may be packed (depending
/// on the architecture) and can be used with `Epoll::wait_raw()` and `Epoll::pwait_raw()`.
/// `Event`, meanwhile, is guaranteed *not* to be packed and can be used with `Epoll::wait()`
/// and `Epoll::pwait()`. (Note that on non-x86_64 platforms the "raw" and non-"raw"
/// types/functions are identical; meanwhile, on x86_64 `wait()`/`pwait()` simply copies
/// data from `RawEvent`s to `Event`s.)
///
/// The data copying that is necessary for `Event` on x86_64 results in a slight
/// slowdown. As a result, if your application has a lot of events on file descriptors
/// watched by an `Epoll`, it may be possible to improve performance slightly by switching
/// to `RawEvent` and the `*wait_raw()` methods -- just be sure to read the issue linked
/// above for information on how to properly handle packed structs.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(not(target_arch = "x86_64"), repr(C))]
pub struct Event {
    pub events: Events,
    pub data: u64,
}

impl Default for Event {
    #[inline]
    fn default() -> Self {
        Self {
            events: Events::empty(),
            data: 0,
        }
    }
}

/// The raw event representation. Read the documentation of `Event`
/// for details.
#[cfg(not(target_arch = "x86_64"))]
pub type RawEvent = Event;

#[derive(Debug)]
pub struct Epoll {
    fd: Int,
}

impl Epoll {
    pub fn new() -> io::Result<Self> {
        let fd = crate::error::convert_neg_ret(unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) })?;

        Ok(Self { fd })
    }

    fn ctl(&mut self, op: CtlOp, fd: Int, events: Events, data: u64) -> io::Result<()> {
        let mut ep_event = libc::epoll_event {
            events: events.bits as u32,
            u64: data,
        };

        crate::error::convert_nzero_ret(
            unsafe { libc::epoll_ctl(self.fd, op as Int, fd, &mut ep_event) }
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

    #[cfg(target_arch = "x86_64")]
    pub fn pwait(
        &self,
        events: &mut [Event],
        timeout: Option<time::Duration>,
        sigmask: Option<crate::signal::Sigset>,
    ) -> io::Result<usize> {
        let mut ep_events = Vec::new();
        ep_events.resize(events.len(), RawEvent { events: Events::empty(), data: 0 });

        let res = self.pwait_raw(&mut ep_events, timeout, sigmask)?;

        for i in 0..res {
            events[i] = Event {
                events: ep_events[i].events,
                data: ep_events[i].data,
            };
        }

        Ok(res)
    }

    #[cfg(not(target_arch = "x86_64"))]
    #[inline]
    pub fn pwait(
        &self,
        events: &mut [Event],
        timeout: Option<time::Duration>,
        sigmask: Option<crate::signal::Sigset>,
    ) -> io::Result<usize> {
        self.pwait_raw(events, timeout, sigmask)
    }

    pub fn pwait_raw(
        &self,
        events: &mut [RawEvent],
        timeout: Option<time::Duration>,
        sigmask: Option<crate::signal::Sigset>,
    ) -> io::Result<usize> {
        let raw_timeout: Int = match timeout {
            Some(t) => t.as_millis().try_into().unwrap_or(Int::MAX),
            None => -1,
        };

        let raw_sigmask = match sigmask {
            Some(s) => &s.raw_set(),
            None => std::ptr::null(),
        };

        let n = crate::error::convert_neg_ret(unsafe {
            libc::epoll_pwait(
                self.fd,
                events.as_mut_ptr() as *mut libc::epoll_event,
                events.len() as Int,
                raw_timeout,
                raw_sigmask,
            )
        })?;

        Ok(n as usize)
    }

    #[inline]
    pub fn wait(&self, events: &mut [Event], timeout: Option<time::Duration>) -> io::Result<usize> {
        self.pwait(events, timeout, None)
    }

    #[inline]
    pub fn wait_raw(&self, events: &mut [RawEvent], timeout: Option<time::Duration>) -> io::Result<usize> {
        self.pwait_raw(events, timeout, None)
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

    #[test]
    fn test_default() {
        assert_eq!(Events::empty(), Events::default());
    }

    #[test]
    fn test_epoll() {
        assert_eq!(std::mem::size_of::<RawEvent>(), std::mem::size_of::<libc::epoll_event>());

        let mut poller = Epoll::new().unwrap();
        let mut events = [Event::default(); 3];

        assert_eq!(poller.fd, poller.as_raw_fd());

        let (r1, mut w1) = crate::pipe().unwrap();
        let (r2, mut w2) = crate::pipe().unwrap();

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
        assert_eq!(events[0].data, w2.as_raw_fd() as u64);
        assert_eq!(events[0].events, Events::IN);

        // Now test wait_raw(), and also omit the timeout so we
        // can cover that case too.
        let mut raw_events = [RawEvent::default(); 3];
        assert_eq!(
            poller
                .wait_raw(&mut raw_events, None)
                .unwrap(),
            1,
        );
        assert_eq!({raw_events[0].data}, w2.as_raw_fd() as u64);
        assert_eq!({raw_events[0].events}, Events::IN);
    }
}
