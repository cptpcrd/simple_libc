use std::convert::TryInto;
use std::io;
use std::time::Duration;

use bitflags::bitflags;

use super::{Int, Short};

bitflags! {
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

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct PollFd {
    pub fd: Int,
    pub events: Events,
    pub revents: Events,
}

pub fn poll(fds: &mut [PollFd], timeout: Option<Duration>) -> io::Result<usize> {
    let raw_timeout: Int = match timeout {
        Some(t) => t.as_millis().try_into().unwrap_or(Int::MAX),
        None => -1,
    };

    super::error::convert_neg_ret(unsafe {
        libc::poll(
            fds.as_mut_ptr() as *mut libc::pollfd,
            fds.len() as libc::nfds_t,
            raw_timeout,
        )
    })
    .map(|n| n as usize)
}

#[cfg(target_os = "netbsd")]
const LIBC_PPOLL: unsafe extern "C" fn(
    *mut libc::pollfd,
    libc::nfds_t,
    *const libc::timespec,
    *const libc::sigset_t,
) -> libc::c_int = super::externs::pollts;
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
))]
pub fn ppoll(
    fds: &mut [PollFd],
    timeout: Option<Duration>,
    sigmask: Option<super::signal::Sigset>,
) -> io::Result<usize> {
    let raw_timeout = match timeout {
        Some(t) => &libc::timespec {
            tv_sec: t.as_secs().try_into().unwrap_or(libc::time_t::MAX),
            tv_nsec: t.subsec_nanos() as super::Long,
        },
        None => std::ptr::null(),
    };

    let raw_sigmask = match sigmask {
        Some(s) => &s.raw_set(),
        None => std::ptr::null(),
    };

    super::error::convert_neg_ret(unsafe {
        LIBC_PPOLL(
            fds.as_mut_ptr() as *mut libc::pollfd,
            fds.len() as libc::nfds_t,
            raw_timeout,
            raw_sigmask,
        )
    })
    .map(|n| n as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    ))]
    fn pipe_cloexec() -> io::Result<(fs::File, fs::File)> {
        super::super::pipe2(libc::O_CLOEXEC)
    }

    #[cfg(target_os = "macos")]
    fn pipe_cloexec() -> io::Result<(fs::File, fs::File)> {
        let (r, w) = super::super::pipe()?;
        super::super::fcntl::set_inheritable(r.as_raw_fd(), false);
        super::super::fcntl::set_inheritable(w.as_raw_fd(), false);
        Ok((r, w))
    }

    #[test]
    fn test_poll() {
        let (r1, mut w1) = pipe_cloexec().unwrap();
        let (r2, mut w2) = pipe_cloexec().unwrap();

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
        w1.write(b"a").unwrap();
        assert_eq!(poll(&mut fds, Some(Duration::from_secs(0))).unwrap(), 1);
        assert_eq!(fds[0].fd, r1.as_raw_fd());
        assert_eq!(fds[0].revents, Events::IN);

        // Now make sure reading two files works
        w2.write(b"a").unwrap();
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
    ))]
    #[test]
    fn test_ppoll() {
        let (r1, mut w1) = pipe_cloexec().unwrap();
        let (r2, mut w2) = pipe_cloexec().unwrap();

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
        w1.write(b"a").unwrap();
        assert_eq!(
            ppoll(&mut fds, Some(Duration::from_secs(0)), None).unwrap(),
            1,
        );
        assert_eq!(fds[0].fd, r1.as_raw_fd());
        assert_eq!(fds[0].revents, Events::IN);

        // Now make sure reading two files works
        w2.write(b"a").unwrap();
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
