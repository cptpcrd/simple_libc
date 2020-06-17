use std::convert::TryInto;
use std::io;
use std::time::Duration;

use bitflags::bitflags;

use crate::{Int, Short};

bitflags! {
    #[repr(transparent)]
    pub struct Events: Short {
        const IN = libc::POLLIN;
        const RDNORM = libc::POLLRDNORM;
        const RDBAND = libc::POLLRDBAND;
        const PRI = libc::POLLPRI;
        const OUT = libc::POLLOUT;
        const WRNORM  = libc::POLLWRNORM;
        const WRBAND = libc::POLLWRBAND;
        const ERR = libc::POLLERR;
        const HUP = libc::POLLHUP;
        const NVAL = libc::POLLNVAL;
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
#[repr(C)]
pub struct PollFd {
    pub fd: Int,
    pub events: Events,
    pub revents: Events,
}

impl PollFd {
    pub const fn new(fd: Int, events: Events) -> Self {
        Self {
            fd,
            events,
            revents: Events::empty(),
        }
    }
}

pub fn poll(fds: &mut [PollFd], timeout: Option<Duration>) -> io::Result<usize> {
    let raw_timeout = match timeout {
        Some(t) => t.as_millis().try_into().unwrap_or(Int::MAX),
        None => -1,
    };

    let n = crate::error::convert_neg_ret(unsafe {
        libc::poll(
            fds.as_mut_ptr() as *mut libc::pollfd,
            fds.len() as libc::nfds_t,
            raw_timeout,
        )
    })?;

    Ok(n as usize)
}

#[cfg(target_os = "netbsd")]
const LIBC_PPOLL: unsafe extern "C" fn(
    *mut libc::pollfd,
    libc::nfds_t,
    *const libc::timespec,
    *const libc::sigset_t,
) -> libc::c_int = crate::externs::pollts;
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "dragonfly",
))]
const LIBC_PPOLL: unsafe extern "C" fn(
    *mut libc::pollfd,
    libc::nfds_t,
    *const libc::timespec,
    *const libc::sigset_t,
) -> libc::c_int = libc::ppoll;

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
))]
pub fn ppoll(
    fds: &mut [PollFd],
    timeout: Option<Duration>,
    sigmask: Option<crate::signal::Sigset>,
) -> io::Result<usize> {
    let raw_timeout = match timeout {
        Some(t) => &libc::timespec {
            tv_sec: t.as_secs().try_into().unwrap_or(libc::time_t::MAX),
            tv_nsec: t.subsec_nanos() as crate::Long,
        },
        None => std::ptr::null(),
    };

    let raw_sigmask = match sigmask {
        Some(s) => &s.raw_set(),
        None => std::ptr::null(),
    };

    let n = crate::error::convert_neg_ret(unsafe {
        LIBC_PPOLL(
            fds.as_mut_ptr() as *mut libc::pollfd,
            fds.len() as libc::nfds_t,
            raw_timeout,
            raw_sigmask,
        )
    })?;

    Ok(n as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    #[test]
    fn test_poll() {
        let (r1, mut w1) = crate::pipe().unwrap();
        let (r2, mut w2) = crate::pipe().unwrap();

        let mut fds = [
            PollFd {
                fd: r1.as_raw_fd(),
                events: Events::IN,
                revents: Events::empty(),
            },
            PollFd {
                fd: r2.as_raw_fd(),
                events: Events::IN,
                revents: Events::empty(),
            },
        ];

        // Nothing to start
        assert_eq!(poll(&mut fds, Some(Duration::from_secs(0))).unwrap(), 0);

        // Now we write some data and test again
        w1.write_all(b"a").unwrap();
        assert_eq!(poll(&mut fds, Some(Duration::from_secs(0))).unwrap(), 1);
        assert_eq!(fds[0].fd, r1.as_raw_fd());
        assert_eq!(fds[0].revents, Events::IN);

        // Now make sure reading two files works
        w2.write_all(b"a").unwrap();
        assert_eq!(poll(&mut fds, Some(Duration::from_secs(0))).unwrap(), 2);
        assert_eq!(fds[0].fd, r1.as_raw_fd());
        assert_eq!(fds[0].revents, Events::IN);
        assert_eq!(fds[1].fd, r2.as_raw_fd());
        assert_eq!(fds[1].revents, Events::IN);
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    ))]
    #[test]
    fn test_ppoll() {
        let (r1, mut w1) = crate::pipe().unwrap();
        let (r2, mut w2) = crate::pipe().unwrap();

        let mut fds = [
            PollFd {
                fd: r1.as_raw_fd(),
                events: Events::IN,
                revents: Events::empty(),
            },
            PollFd {
                fd: r2.as_raw_fd(),
                events: Events::IN,
                revents: Events::empty(),
            },
        ];

        // Nothing to start
        assert_eq!(
            ppoll(&mut fds, Some(Duration::from_secs(0)), None).unwrap(),
            0,
        );

        // Now we write some data and test again
        w1.write_all(b"a").unwrap();
        assert_eq!(
            ppoll(&mut fds, Some(Duration::from_secs(0)), None).unwrap(),
            1,
        );
        assert_eq!(fds[0].fd, r1.as_raw_fd());
        assert_eq!(fds[0].revents, Events::IN);

        // Now make sure reading two files works
        w2.write_all(b"a").unwrap();
        assert_eq!(
            ppoll(&mut fds, Some(Duration::from_secs(0)), None).unwrap(),
            2,
        );
        assert_eq!(fds[0].fd, r1.as_raw_fd());
        assert_eq!(fds[0].revents, Events::IN);
        assert_eq!(fds[1].fd, r2.as_raw_fd());
        assert_eq!(fds[1].revents, Events::IN);
    }
}
