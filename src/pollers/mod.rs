use std::io;
use std::os::unix::io::RawFd;
use std::time::Duration;

use bitflags::bitflags;

#[cfg(target_os = "linux")]
mod epoll;
mod poll;
mod select;

#[cfg(target_os = "linux")]
pub use epoll::EpollPoller;
pub use poll::PollPoller;
pub use select::SelectPoller;

use crate::signal::Sigset;

bitflags! {
    pub struct Events: u32 {
        const READ  = 0b001;
        const WRITE = 0b010;
        const ERROR = 0b100;
    }
}

pub trait Poller: Sized {
    fn new() -> io::Result<Self>;

    /// Begin monitoring the given file descriptor for the given events.
    ///
    /// If the file object was already registered, this returns an `EEXIST` error.
    fn register(&mut self, fd: RawFd, events: Events) -> io::Result<()>;

    /// Stop monitoring the given file descriptor.
    ///
    /// If the file object was not already registered, this returns an `ENOENT` error.
    fn unregister(&mut self, fd: RawFd) -> io::Result<()>;

    /// Modify the events being monitored for the given file descriptor.
    ///
    /// If the file object was not already registered, this returns an `ENOENT` error.
    fn modify(&mut self, fd: RawFd, events: Events) -> io::Result<()> {
        self.unregister(fd)?;
        self.register(fd, events)?;
        Ok(())
    }

    fn poll(&mut self, timeout: Option<Duration>) -> io::Result<Vec<(RawFd, Events)>>;
}

pub trait Ppoller: Poller {
    fn ppoll(
        &mut self,
        timeout: Option<Duration>,
        sigmask: Option<Sigset>,
    ) -> io::Result<Vec<(RawFd, Events)>>;
}

crate::attr_group! {
    #![cfg(target_os = "linux")]

    pub type DefaultPoller = EpollPoller;
    pub type DefaultPpoller = EpollPoller;
}

crate::attr_group! {
    #![cfg(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    ))]

    pub type DefaultPoller = PollPoller;
    pub type DefaultPpoller = PollPoller;
}

crate::attr_group! {
    #![cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    )))]

    pub type DefaultPoller = PollPoller;
    pub type DefaultPpoller = SelectPoller;
}
