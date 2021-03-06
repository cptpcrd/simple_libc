use std::collections::HashSet;
use std::io;
use std::os::unix::prelude::*;
use std::time::Duration;

use super::{Events, Poller};
use crate::poll::{poll, Events as PollEvents, PollFd};

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
))]
use crate::poll::ppoll;

#[derive(Debug)]
pub struct PollPoller {
    pollfds: Vec<PollFd>,
    fdset: HashSet<RawFd>,
}

impl PollPoller {
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

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn translate_pollfd_event(pfd: &PollFd) -> Option<(RawFd, Events)> {
        match Self::translate_events_rev(pfd.revents) {
            Some(ev) => Some((pfd.fd, ev)),
            None => None,
        }
    }
}

impl Poller for PollPoller {
    fn new() -> io::Result<Self> {
        Ok(Self {
            pollfds: Vec::new(),
            fdset: HashSet::new(),
        })
    }

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
        for pfd in self.pollfds.iter_mut() {
            if pfd.fd == fd {
                pfd.events = Self::translate_events(events);
                return Ok(());
            }
        }

        Err(io::Error::from_raw_os_error(libc::ENOENT))
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
impl super::Ppoller for PollPoller {
    fn ppoll(
        &mut self,
        timeout: Option<Duration>,
        sigmask: Option<crate::signal::Sigset>,
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

    use std::io::Write;

    #[test]
    fn test_poll_poller() {
        let timeout_0 = Some(Duration::from_secs(0));

        let (r1, mut w1) = crate::pipe().unwrap();
        let (r2, mut w2) = crate::pipe().unwrap();

        let mut poller = PollPoller::new().unwrap();

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
        w2.write_all(b"a").unwrap();
        assert_eq!(
            poller.poll(timeout_0).unwrap(),
            vec![(r2.as_raw_fd(), Events::READ)],
        );

        // Now make sure reading two files works
        w1.write_all(b"a").unwrap();
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
