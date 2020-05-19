use std::io;
use std::os::unix::io::RawFd;
use std::time::Duration;

use bitflags::bitflags;
use cfg_if::cfg_if;

#[cfg(target_os = "linux")]
mod epoll;
mod poll;
mod select;

#[cfg(target_os = "linux")]
pub use epoll::EpollPoller;
pub use poll::PollPoller;
pub use select::SelectPoller;

use super::signal::Sigset;

bitflags! {
    pub struct Events: u32 {
        const READ  = 0b001;
        const WRITE = 0b010;
        const ERROR = 0b100;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Flags: u32 {
        const CLOEXEC = 0b1;
    }
}

pub trait Poller {
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

cfg_if! {
    if #[cfg(target_os = "linux")] {
        pub type DefaultPoller = EpollPoller;
        pub type DefaultPpoller = EpollPoller;
    } else if #[cfg(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    ))] {
        pub type DefaultPoller = PollPoller;
        pub type DefaultPpoller = PollPoller;
    } else {
        pub type DefaultPoller = PollPoller;
        pub type DefaultPpoller = SelectPoller;
    }
}
