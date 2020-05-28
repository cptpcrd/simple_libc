use std::collections::hash_map;
use std::collections::HashMap;
use std::io;
use std::os::unix::io::RawFd;
use std::time::Duration;

use super::{Events, Flags, Poller, Ppoller};
use crate::select::{build_fdset_opt, pselect_raw, FdSet};
use crate::signal::Sigset;

#[derive(Debug)]
pub struct SelectPoller {
    files: HashMap<RawFd, Events>,
}

impl SelectPoller {
    #[inline]
    pub fn new(_flags: Flags) -> io::Result<Self> {
        Ok(Self {
            files: HashMap::new(),
        })
    }

    fn build_fdset(&self, events: Events, nfds: RawFd) -> (Option<FdSet>, RawFd) {
        build_fdset_opt(
            self.files.iter().filter_map(|(fd, mon_ev)| {
                if mon_ev.contains(events) {
                    Some(*fd)
                } else {
                    None
                }
            }),
            nfds,
        )
    }
}

impl Poller for SelectPoller {
    fn register(&mut self, fd: RawFd, events: Events) -> io::Result<()> {
        match self.files.entry(fd) {
            hash_map::Entry::Vacant(e) => {
                e.insert(events);
                Ok(())
            }
            hash_map::Entry::Occupied(_) => Err(io::Error::from_raw_os_error(libc::EEXIST)),
        }
    }

    fn unregister(&mut self, fd: RawFd) -> io::Result<()> {
        if self.files.remove(&fd).is_some() {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(libc::ENOENT))
        }
    }

    fn modify(&mut self, fd: RawFd, events: Events) -> io::Result<()> {
        match self.files.entry(fd) {
            hash_map::Entry::Occupied(mut e) => {
                e.insert(events);
                Ok(())
            }
            hash_map::Entry::Vacant(_) => Err(io::Error::from_raw_os_error(libc::ENOENT)),
        }
    }

    fn poll(&mut self, timeout: Option<Duration>) -> io::Result<Vec<(RawFd, Events)>> {
        self.ppoll(timeout, None)
    }
}

impl Ppoller for SelectPoller {
    fn ppoll(
        &mut self,
        timeout: Option<Duration>,
        sigmask: Option<Sigset>,
    ) -> io::Result<Vec<(RawFd, Events)>> {
        let (mut read_fdset, nfds) = self.build_fdset(Events::READ, 0);
        let (mut write_fdset, nfds) = self.build_fdset(Events::WRITE, nfds);
        let (mut error_fdset, nfds) = self.build_fdset(Events::ERROR, nfds);

        let n = pselect_raw(
            nfds,
            read_fdset.as_mut(),
            write_fdset.as_mut(),
            error_fdset.as_mut(),
            timeout,
            sigmask,
        )?;

        let mut res: Vec<(RawFd, Events)> = Vec::with_capacity(n);

        for fd in self.files.keys() {
            if res.len() >= n {
                break;
            }

            let mut triggered_events = Events::empty();

            if let Some(mut s) = read_fdset {
                if s.contains(*fd) {
                    triggered_events |= Events::READ;
                }
            }

            if let Some(mut s) = write_fdset {
                if s.contains(*fd) {
                    triggered_events |= Events::WRITE;
                }
            }

            if let Some(mut s) = error_fdset {
                if s.contains(*fd) {
                    triggered_events |= Events::ERROR;
                }
            }

            if !triggered_events.is_empty() {
                res.push((*fd, triggered_events));
            }
        }

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashSet;
    use std::fs;
    use std::io::Write;
    use std::iter::FromIterator;
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
        fcntl::set_inheritable(r.as_raw_fd(), false).unwrap();
        fcntl::set_inheritable(w.as_raw_fd(), false).unwrap();
        Ok((r, w))
    }

    #[test]
    fn test_select_poller() {
        let timeout_0 = Some(Duration::from_secs(0));

        let (r1, mut w1) = pipe_cloexec().unwrap();
        let (r2, mut w2) = pipe_cloexec().unwrap();

        let mut poller = SelectPoller::new(Flags::CLOEXEC).unwrap();

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
            poller
                .poll(timeout_0)
                .unwrap()
                .into_iter()
                .collect::<HashSet<(RawFd, Events)>>(),
            HashSet::from_iter(vec![
                (r1.as_raw_fd(), Events::READ),
                (r2.as_raw_fd(), Events::READ)
            ]),
        );

        // And checking if they're ready for writing
        poller.register(w1.as_raw_fd(), Events::WRITE).unwrap();
        poller.register(w2.as_raw_fd(), Events::WRITE).unwrap();
        assert_eq!(
            poller
                .poll(timeout_0)
                .unwrap()
                .into_iter()
                .collect::<HashSet<(RawFd, Events)>>(),
            HashSet::from_iter(vec![
                (r1.as_raw_fd(), Events::READ),
                (r2.as_raw_fd(), Events::READ),
                (w1.as_raw_fd(), Events::WRITE),
                (w2.as_raw_fd(), Events::WRITE),
            ]),
        );

        // Unregister
        poller.unregister(r1.as_raw_fd()).unwrap();
        poller.unregister(w2.as_raw_fd()).unwrap();
        assert_eq!(
            poller
                .poll(timeout_0)
                .unwrap()
                .into_iter()
                .collect::<HashSet<(RawFd, Events)>>(),
            HashSet::from_iter(vec![
                (r2.as_raw_fd(), Events::READ),
                (w1.as_raw_fd(), Events::WRITE),
            ]),
        );

        // Modify
        poller
            .modify(w1.as_raw_fd(), Events::READ | Events::WRITE)
            .unwrap();
        assert_eq!(
            poller
                .poll(timeout_0)
                .unwrap()
                .into_iter()
                .collect::<HashSet<(RawFd, Events)>>(),
            HashSet::from_iter(vec![
                (r2.as_raw_fd(), Events::READ),
                (w1.as_raw_fd(), Events::WRITE),
            ]),
        );
    }
}
