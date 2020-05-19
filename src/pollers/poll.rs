use std::collections::HashSet;
use std::io;
use std::os::unix::io::RawFd;
use std::time::Duration;

use super::{Events, Flags, Poller, Ppoller};
use crate::poll::{poll, ppoll, Events as PollEvents, PollFd};
use crate::signal::Sigset;

pub struct PollPoller {
    pollfds: Vec<PollFd>,
    fdset: HashSet<RawFd>,
}

impl PollPoller {
    #[inline]
    pub fn new(_flags: Flags) -> io::Result<Self> {
        Ok(Self {
            pollfds: Vec::new(),
            fdset: HashSet::new(),
        })
    }

    fn translate_events(events: Events) -> PollEvents {
        let mut ev = PollEvents::empty();

        if events.contains(Events::READ) {
            ev.insert(PollEvents::IN);
        }
        if events.contains(Events::WRITE) {
            ev.insert(PollEvents::OUT);
        }
        if events.contains(Events::ERROR) {
            ev.insert(PollEvents::ERR);
        }

        ev
    }

    fn translate_events_rev(events: PollEvents) -> Option<Events> {
        let mut ev = Events::empty();

        if events.contains(PollEvents::IN) {
            ev.insert(Events::READ);
        }
        if events.contains(PollEvents::OUT) {
            ev.insert(Events::WRITE);
        }
        if events.contains(PollEvents::ERR) {
            ev.insert(Events::ERROR);
        }

        if ev.is_empty() {
            None
        } else {
            Some(ev)
        }
    }

    fn translate_pollfd_event(pfd: &PollFd) -> Option<(RawFd, Events)> {
        match Self::translate_events_rev(pfd.revents) {
            Some(ev) => Some((pfd.fd, ev)),
            None => None,
        }
    }
}

impl Poller for PollPoller {
    fn register(&mut self, fd: RawFd, events: Events) -> io::Result<()> {
        if self.fdset.contains(&fd) {
            Err(io::Error::from_raw_os_error(libc::EEXIST))
        } else {
            self.pollfds.push(PollFd {
                fd,
                events: Self::translate_events(events),
                revents: PollEvents::empty(),
            });

            self.fdset.insert(fd);

            Ok(())
        }
    }

    fn unregister(&mut self, fd: RawFd) -> io::Result<()> {
        if self.fdset.contains(&fd) {
            if let Some(index) = self.pollfds.iter().position(|pfd| pfd.fd == fd) {
                self.pollfds.remove(index);
            }

            self.fdset.remove(&fd);

            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(libc::ENOENT))
        }
    }

    fn modify(&mut self, fd: RawFd, events: Events) -> io::Result<()> {
        if let Some(index) = self.pollfds.iter().position(|pfd| pfd.fd == fd) {
            self.pollfds[index] = PollFd {
                fd,
                events: Self::translate_events(events),
                revents: PollEvents::empty(),
            };

            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(libc::ENOENT))
        }
    }

    fn poll(&mut self, timeout: Option<Duration>) -> io::Result<Vec<(RawFd, Events)>> {
        let n = poll(&mut self.pollfds, timeout)?;
        Ok(self
            .pollfds
            .iter()
            .filter_map(Self::translate_pollfd_event)
            .take(n)
            .collect())
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
))]
impl Ppoller for PollPoller {
    fn ppoll(
        &mut self,
        timeout: Option<Duration>,
        sigmask: Option<Sigset>,
    ) -> io::Result<Vec<(RawFd, Events)>> {
        let n = ppoll(&mut self.pollfds, timeout, sigmask)?;
        Ok(self
            .pollfds
            .iter()
            .filter_map(Self::translate_pollfd_event)
            .take(n)
            .collect())
    }
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
        use crate::pipe2;
        pipe2(libc::O_CLOEXEC)
    }

    #[cfg(target_os = "macos")]
    fn pipe_cloexec() -> io::Result<(fs::File, fs::File)> {
        use crate::fcntl;
        use crate::pipe;

        let (r, w) = pipe()?;
        fcntl::set_inheritable(r.as_raw_fd(), false);
        fcntl::set_inheritable(w.as_raw_fd(), false);
        Ok((r, w))
    }

    #[test]
    fn test_poll_poller() {
        let timeout_0 = Some(Duration::from_secs(0));

        let (r1, mut w1) = pipe_cloexec().unwrap();
        let (r2, mut w2) = pipe_cloexec().unwrap();

        let mut poller = PollPoller::new(Flags::CLOEXEC).unwrap();

        // Nothing to start
        assert_eq!(poller.poll(timeout_0).unwrap(), vec![]);

        // Nothing after we register a few descriptors
        poller.register(r1.as_raw_fd(), Events::READ).unwrap();
        poller.register(r2.as_raw_fd(), Events::READ).unwrap();
        assert_eq!(poller.poll(timeout_0).unwrap(), vec![]);

        // Now we write some data and test again
        w1.write(b"a").unwrap();
        assert_eq!(
            poller.poll(timeout_0).unwrap(),
            vec![(r1.as_raw_fd(), Events::READ)],
        );

        // Now make sure reading two files works
        w2.write(b"a").unwrap();
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
    }
}
