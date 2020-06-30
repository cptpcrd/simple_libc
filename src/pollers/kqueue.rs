use std::convert::TryInto;
use std::io;
use std::os::unix::prelude::*;
use std::time::Duration;

use super::{Events, Poller};
use crate::{Int, Long, SizeT};

#[derive(Debug)]
pub struct KqueuePoller {
    fd: Int,
}

impl KqueuePoller {
    fn ctl(
        &self,
        changes: &[libc::kevent],
        events: &mut [libc::kevent],
        timeout: Option<Duration>,
    ) -> io::Result<SizeT> {
        let raw_timeout = match timeout {
            Some(t) => &libc::timespec {
                tv_sec: t.as_secs().try_into().unwrap_or(libc::time_t::MAX),
                tv_nsec: t.subsec_nanos() as Long,
            },
            None => std::ptr::null(),
        };

        let n = crate::error::convert_neg_ret(unsafe {
            libc::kevent(
                self.fd,
                changes.as_ptr(),
                changes.len().try_into().unwrap(),
                events.as_mut_ptr(),
                events.len().try_into().unwrap(),
                raw_timeout,
            )
        })?;

        Ok(n as SizeT)
    }

    fn ctl_add_single(&mut self, fd: RawFd, event: Events) -> io::Result<()> {
        let ev: libc::kevent = std::mem::zeroed();

        ev.ident = fd as libc::uintptr_t;
        ev.filter = if event.contains(Events::WRITE) {
            libc::EVFILT_WRITE
        } else {
            libc::EVFILT_READ
        };
        ev.flags = libc::EV_ADD;
        ev.fflags = 0;
        ev.data = 0;

        self.ctl(
            std::slice::from_ref(&ev),
            &mut [],
            Some(Duration::from_secs(0)),
        )?;

        Ok(())
    }

    fn ctl_del_single(&mut self, fd: RawFd, event: Events) -> io::Result<()> {
        let ev: libc::kevent = std::mem::zeroed();

        ev.ident = fd as libc::uintptr_t;
        ev.filter = if event.contains(Events::WRITE) {
            libc::EVFILT_WRITE
        } else {
            libc::EVFILT_READ
        };
        ev.flags = libc::EV_DELETE;
        ev.fflags = 0;
        ev.data = 0;

        self.ctl(
            std::slice::from_ref(&ev),
            &mut [],
            Some(Duration::from_secs(0)),
        )?;

        Ok(())
    }

    fn ctl_add(&mut self, fd: RawFd, events: Events) -> io::Result<()> {

    }
}

impl Poller for KqueuePoller {
    fn new() -> io::Result<Self> {
        let res;

        // NetBSD offers kqueue1(), which lets us specify O_CLOEXEC during
        // construction
        #[cfg(target_os = "netbsd")]
        {
            res = Self {
                fd: crate::error::convert_neg_ret(unsafe {
                    crate::externs::kqueue1(libc::O_CLOEXEC)
                })?,
            };
        }

        // On other BSDs, we have to settle for immediately fcntl()ing it to be
        // non-inheritable.
        // fork()ed children don't inherit kqueues by default, but we want to be
        // safe -- the program may call exec() without fork()ing.
        #[cfg(not(target_os = "netbsd"))]
        {
            let fd = crate::error::convert_neg_ret(unsafe { libc::kqueue() })?;

            // Construct it now so if the set_inheritable() call fails
            // drop() will be called to close it
            res = Self { fd };

            crate::fcntl::set_inheritable(fd, false)?;
        }

        Ok(res)
    }

    fn register(&mut self, fd: RawFd, events: Events) -> io::Result<()> {
        Ok(())
    }

    fn unregister(&mut self, fd: RawFd) -> io::Result<()> {
        self.ctl_del(fd)
    }

    fn modify(&mut self, fd: RawFd, events: Events) -> io::Result<()> {
        self.ctl_add(fd, events)
    }

    fn poll(&mut self, timeout: Option<Duration>) -> io::Result<Vec<(RawFd, Events)>> {
        self.ppoll(timeout, None)
    }
}

impl Drop for KqueuePoller {
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
    fn test_kqueue_poller() {
        let timeout_0 = Some(Duration::from_secs(0));

        let (r1, mut w1) = crate::pipe().unwrap();
        let (r2, mut w2) = crate::pipe().unwrap();

        let mut poller = KqueuePoller::new().unwrap();

        // Nothing to start
        assert_eq!(poller.poll(timeout_0).unwrap(), vec![]);

        // Nothing after we register a few descriptors
        poller.register(r1.as_raw_fd(), Events::READ).unwrap();
        poller.register(r2.as_raw_fd(), Events::READ).unwrap();
        assert_eq!(poller.poll(timeout_0).unwrap(), vec![]);

        // Errors raised
        assert_eq!(
            poller
                .register(r1.as_raw_fd(), Events::READ)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EEXIST),
        );
        assert_eq!(
            poller
                .modify(w1.as_raw_fd(), Events::READ)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ENOENT),
        );
        assert_eq!(
            poller
                .unregister(w1.as_raw_fd())
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ENOENT),
        );

        // Now we write some data and test again
        w1.write_all(b"a").unwrap();
        assert_eq!(
            poller.poll(timeout_0).unwrap(),
            vec![(r1.as_raw_fd(), Events::READ)],
        );

        // Now make sure reading two files works
        w2.write_all(b"a").unwrap();
        assert_eq!(
            poller.poll(timeout_0).unwrap(),
            vec![
                (r1.as_raw_fd(), Events::READ),
                (r2.as_raw_fd(), Events::READ)
            ],
        );

        // And checking if they're ready for writing
        poller.register(w1.as_raw_fd(), Events::WRITE).unwrap();
        poller.register(w2.as_raw_fd(), Events::WRITE).unwrap();
        assert_eq!(
            poller.poll(timeout_0).unwrap(),
            vec![
                (r1.as_raw_fd(), Events::READ),
                (r2.as_raw_fd(), Events::READ),
                (w1.as_raw_fd(), Events::WRITE),
                (w2.as_raw_fd(), Events::WRITE),
            ],
        );

        // Unregister
        poller.unregister(r1.as_raw_fd()).unwrap();
        poller.unregister(w2.as_raw_fd()).unwrap();
        assert_eq!(
            poller.poll(timeout_0).unwrap(),
            vec![
                (r2.as_raw_fd(), Events::READ),
                (w1.as_raw_fd(), Events::WRITE),
            ],
        );

        // Modify
        poller
            .modify(w1.as_raw_fd(), Events::READ | Events::WRITE)
            .unwrap();
        assert_eq!(
            poller.poll(timeout_0).unwrap(),
            vec![
                (r2.as_raw_fd(), Events::READ),
                (w1.as_raw_fd(), Events::WRITE),
            ],
        );

        poller.modify(w1.as_raw_fd(), Events::READ).unwrap();
        assert_eq!(
            poller.poll(timeout_0).unwrap(),
            vec![(r2.as_raw_fd(), Events::READ)],
        );
    }
}
