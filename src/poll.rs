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

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    use super::super::pipe2;

    #[test]
    fn test_poll() {
        let (r1, mut w1) = pipe2(libc::O_CLOEXEC).unwrap();
        let (r2, mut w2) = pipe2(libc::O_CLOEXEC).unwrap();

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
}
