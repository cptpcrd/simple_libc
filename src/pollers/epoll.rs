use std::io;
use std::os::unix::io::RawFd;
use std::time::Duration;

use super::{Events, Flags, Poller, Ppoller};
use crate::epoll::{Epoll, EpollFlags, Event as EpollEvent, Events as EpollEvents};
use crate::signal::Sigset;

pub struct EpollPoller {
    epoll: Epoll,
}

impl EpollPoller {
    #[inline]
    pub fn new(flags: Flags) -> io::Result<Self> {
        let mut epoll_flags = EpollFlags::empty();
        if flags.contains(Flags::CLOEXEC) {
            epoll_flags.insert(EpollFlags::CLOEXEC);
        }

        Ok(Self {
            epoll: Epoll::new(epoll_flags)?,
        })
    }

    fn translate_events(events: Events) -> EpollEvents {
        let mut ev = EpollEvents::empty();

        if events.contains(Events::READ) {
            ev.insert(EpollEvents::IN);
        }
        if events.contains(Events::WRITE) {
            ev.insert(EpollEvents::OUT);
        }
        if events.contains(Events::ERROR) {
            ev.insert(EpollEvents::ERR);
        }

        ev
    }

    fn translate_events_rev(events: EpollEvents) -> Option<Events> {
        let mut ev = Events::empty();

        if events.contains(EpollEvents::IN) {
            ev.insert(Events::READ);
        }
        if events.contains(EpollEvents::OUT) {
            ev.insert(Events::WRITE);
        }
        if events.contains(EpollEvents::ERR) {
            ev.insert(Events::ERROR);
        }

        if ev.is_empty() {
            None
        } else {
            Some(ev)
        }
    }

    fn translate_epoll_event(e: &EpollEvent) -> Option<(RawFd, Events)> {
        match Self::translate_events_rev(e.events) {
            Some(ev) => Some((e.data as RawFd, ev)),
            None => None,
        }
    }
}

impl Poller for EpollPoller {
    fn register(&mut self, fd: RawFd, events: Events) -> io::Result<()> {
        self.epoll.add(fd, Self::translate_events(events))
    }

    fn unregister(&mut self, fd: RawFd) -> io::Result<()> {
        self.epoll.del(fd)
    }

    fn modify(&mut self, fd: RawFd, events: Events) -> io::Result<()> {
        self.epoll.modify(fd, Self::translate_events(events))
    }

    fn poll(&mut self, timeout: Option<Duration>) -> io::Result<Vec<(RawFd, Events)>> {
        self.ppoll(timeout, None)
    }
}

impl Ppoller for EpollPoller {
    fn ppoll(
        &mut self,
        timeout: Option<Duration>,
        sigmask: Option<Sigset>,
    ) -> io::Result<Vec<(RawFd, Events)>> {
        let mut events = [EpollEvent {
            events: EpollEvents::empty(),
            data: 0,
        }; 10];

        let n = self.epoll.pwait(&mut events, timeout, sigmask.as_ref())? as usize;
        Ok(events
            .iter()
            .filter_map(Self::translate_epoll_event)
            .take(n)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    use super::super::super::pipe2;

    #[test]
    fn test_epoll_poller() {
        let timeout_0 = Some(Duration::from_secs(0));

        let (r1, mut w1) = pipe2(libc::O_CLOEXEC).unwrap();
        let (r2, mut w2) = pipe2(libc::O_CLOEXEC).unwrap();

        let mut poller = EpollPoller::new(Flags::CLOEXEC).unwrap();

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
    }
}
