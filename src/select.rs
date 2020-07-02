use std::convert::TryInto;
use std::io;
use std::iter::FromIterator;
use std::time::Duration;

use crate::signal::Sigset;
use crate::{Int, Long};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct FdSet {
    raw: libc::fd_set,
}

impl FdSet {
    pub fn empty() -> Self {
        let mut res: Self = unsafe { std::mem::zeroed() };
        res.fd_zero();
        res
    }

    #[inline]
    pub fn fd_zero(&mut self) {
        unsafe { libc::FD_ZERO(&mut self.raw) }
    }

    #[inline]
    pub fn fd_isset(&mut self, fd: Int) -> bool {
        unsafe { libc::FD_ISSET(fd, &mut self.raw) }
    }

    #[inline]
    pub fn fd_set(&mut self, fd: Int) {
        unsafe { libc::FD_SET(fd, &mut self.raw) }
    }

    #[inline]
    pub fn fd_clr(&mut self, fd: Int) {
        unsafe { libc::FD_CLR(fd, &mut self.raw) }
    }

    // More understandable
    #[inline(always)]
    pub fn clear(&mut self) {
        self.fd_zero()
    }

    #[inline(always)]
    pub fn contains(&mut self, fd: Int) -> bool {
        self.fd_isset(fd)
    }

    #[inline(always)]
    pub fn add(&mut self, fd: Int) {
        self.fd_set(fd)
    }

    #[inline(always)]
    pub fn remove(&mut self, fd: Int) {
        self.fd_clr(fd)
    }
}

impl Default for FdSet {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl FromIterator<Int> for FdSet {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Int>>(fds: T) -> Self {
        build_fdset(fds).0
    }
}

pub fn build_fdset<T: IntoIterator<Item = Int>>(fds: T) -> (FdSet, Int) {
    let mut fdset = FdSet::empty();
    let mut nfds: Int = 0;

    for fd in fds {
        fdset.add(fd);
        nfds = std::cmp::max(nfds, fd + 1);
    }

    (fdset, nfds)
}

pub fn build_fdset_slice(fds: &[Int]) -> (FdSet, Int) {
    let mut fdset = FdSet::empty();
    let mut nfds = 0;

    for fd in fds {
        let fd = *fd;
        fdset.add(fd);
        nfds = std::cmp::max(nfds, fd + 1);
    }

    (fdset, nfds)
}

pub fn build_fdset_opt<T: IntoIterator<Item = Int>>(fds: T, mut nfds: Int) -> (Option<FdSet>, Int) {
    let mut it = fds.into_iter();

    if let Some(fd) = it.next() {
        let mut fdset = FdSet::empty();
        fdset.add(fd);
        nfds = std::cmp::max(nfds, fd + 1);

        for fd in it {
            fdset.add(fd);
            nfds = std::cmp::max(nfds, fd + 1);
        }

        (Some(fdset), nfds)
    } else {
        (None, nfds)
    }
}

pub fn build_fdset_opt_slice(fds: &[Int], mut nfds: Int) -> (Option<FdSet>, Int) {
    if fds.is_empty() {
        return (None, nfds);
    }

    let mut fdset = FdSet::empty();
    for fd in fds {
        let fd = *fd;
        fdset.add(fd);
        nfds = std::cmp::max(nfds, fd + 1);
    }

    (Some(fdset), nfds)
}

#[inline]
fn raw_opt_fdset(set: Option<&mut FdSet>) -> *mut libc::fd_set {
    match set {
        Some(s) => &mut s.raw,
        None => std::ptr::null_mut(),
    }
}

pub fn pselect_raw(
    nfds: Int,
    readfds: Option<&mut FdSet>,
    writefds: Option<&mut FdSet>,
    errorfds: Option<&mut FdSet>,
    timeout: Option<Duration>,
    sigmask: Option<Sigset>,
) -> io::Result<usize> {
    let raw_timeout = match timeout {
        Some(t) => &libc::timespec {
            tv_sec: t.as_secs().try_into().unwrap_or(libc::time_t::MAX),
            tv_nsec: t.subsec_nanos() as Long,
        },
        None => std::ptr::null(),
    };

    let raw_sigmask = match sigmask {
        Some(s) => &s.raw_set(),
        None => std::ptr::null(),
    };

    let n = crate::error::convert_neg_ret(unsafe {
        libc::pselect(
            nfds,
            raw_opt_fdset(readfds),
            raw_opt_fdset(writefds),
            raw_opt_fdset(errorfds),
            raw_timeout,
            raw_sigmask,
        )
    })?;

    Ok(n as usize)
}

pub fn select_raw(
    nfds: Int,
    readfds: Option<&mut FdSet>,
    writefds: Option<&mut FdSet>,
    errorfds: Option<&mut FdSet>,
    timeout: Option<Duration>,
) -> io::Result<usize> {
    let raw_timeout = match timeout {
        Some(t) => &mut libc::timeval {
            tv_sec: t.as_secs().try_into().unwrap_or(libc::time_t::MAX),
            tv_usec: t.subsec_micros() as libc::suseconds_t,
        },
        None => std::ptr::null_mut(),
    };

    let n = crate::error::convert_neg_ret(unsafe {
        libc::select(
            nfds,
            raw_opt_fdset(readfds),
            raw_opt_fdset(writefds),
            raw_opt_fdset(errorfds),
            raw_timeout,
        )
    })?;

    Ok(n as usize)
}

fn build_raw_setup(
    readfds: &[Int],
    writefds: &[Int],
    errorfds: &[Int],
) -> (Int, Option<FdSet>, Option<FdSet>, Option<FdSet>) {
    let (readfdset, nfds) = build_fdset_opt_slice(readfds, 0);
    let (writefdset, nfds) = build_fdset_opt_slice(writefds, nfds);
    let (errorfdset, nfds) = build_fdset_opt_slice(errorfds, nfds);

    (nfds, readfdset, writefdset, errorfdset)
}

fn build_return_vec(
    mut n: usize,
    orig_fds: &[Int],
    fdset: Option<&mut FdSet>,
) -> (usize, Vec<Int>) {
    if n == 0 {
        return (n, Vec::new());
    }

    match fdset {
        Some(s) => {
            let mut res = Vec::with_capacity(orig_fds.len());

            for fd in orig_fds {
                if s.contains(*fd) {
                    res.push(*fd);
                    n -= 1;

                    if n == 0 {
                        break;
                    }
                }
            }

            res.shrink_to_fit();
            (n, res)
        }
        None => (n, Vec::new()),
    }
}

pub fn select_simple(
    readfds: &[Int],
    writefds: &[Int],
    errorfds: &[Int],
    timeout: Option<Duration>,
) -> io::Result<(Vec<Int>, Vec<Int>, Vec<Int>)> {
    let (nfds, mut readfdset, mut writefdset, mut errorfdset) =
        build_raw_setup(readfds, writefds, errorfds);

    let n = select_raw(
        nfds,
        readfdset.as_mut(),
        writefdset.as_mut(),
        errorfdset.as_mut(),
        timeout,
    )?;

    let (n, ready_readfds) = build_return_vec(n, readfds, readfdset.as_mut());
    let (n, ready_writefds) = build_return_vec(n, writefds, writefdset.as_mut());
    let (n, ready_errorfds) = build_return_vec(n, errorfds, errorfdset.as_mut());

    debug_assert_eq!(n, 0);

    Ok((ready_readfds, ready_writefds, ready_errorfds))
}

pub fn pselect_simple(
    readfds: &[Int],
    writefds: &[Int],
    errorfds: &[Int],
    timeout: Option<Duration>,
    sigmask: Option<Sigset>,
) -> io::Result<(Vec<Int>, Vec<Int>, Vec<Int>)> {
    let (nfds, mut readfdset, mut writefdset, mut errorfdset) =
        build_raw_setup(readfds, writefds, errorfds);

    let n = pselect_raw(
        nfds,
        readfdset.as_mut(),
        writefdset.as_mut(),
        errorfdset.as_mut(),
        timeout,
        sigmask,
    )?;

    let (n, ready_readfds) = build_return_vec(n, readfds, readfdset.as_mut());
    let (n, ready_writefds) = build_return_vec(n, writefds, writefdset.as_mut());
    let (n, ready_errorfds) = build_return_vec(n, errorfds, errorfdset.as_mut());

    debug_assert_eq!(n, 0);

    Ok((ready_readfds, ready_writefds, ready_errorfds))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    #[test]
    fn test_fdset() {
        let mut fdset = FdSet::default();

        assert!(!fdset.contains(1));
        fdset.add(1);
        assert!(fdset.contains(1));
        fdset.remove(1);
        assert!(!fdset.contains(1));

        fdset.add(1);
        assert!(fdset.contains(1));
        fdset.clear();
        assert!(!fdset.contains(1));
    }

    #[test]
    fn test_fdset_from_iter() {
        let mut fdset = FdSet::empty();
        assert_eq!(FdSet::from_iter([].iter().copied()), fdset);

        fdset.add(0);
        assert_eq!(FdSet::from_iter([0].iter().copied()), fdset);

        fdset.add(5);
        assert_eq!(FdSet::from_iter([0, 5].iter().copied()), fdset);
    }

    #[test]
    fn test_build_fdset() {
        let mut fdset = FdSet::empty();
        assert_eq!(build_fdset([].iter().copied()), (fdset, 0));

        fdset.add(0);
        assert_eq!(build_fdset([0].iter().copied()), (fdset, 1));

        fdset.add(5);
        assert_eq!(build_fdset([0, 5].iter().copied()), (fdset, 6));
    }

    #[test]
    fn test_build_fdset_slice() {
        let mut fdset = FdSet::empty();
        assert_eq!(build_fdset_slice(&[]), (fdset, 0));

        fdset.add(0);
        assert_eq!(build_fdset_slice(&[0]), (fdset, 1));

        fdset.add(5);
        assert_eq!(build_fdset_slice(&[0, 5]), (fdset, 6));
    }

    #[test]
    fn test_select() {
        let timeout_0 = Some(Duration::from_secs(0));

        let (r1, mut w1) = crate::pipe().unwrap();
        let (r2, mut w2) = crate::pipe().unwrap();

        let maxfd = [&r1, &w1, &r2, &w2]
            .iter()
            .cloned()
            .map(AsRawFd::as_raw_fd)
            .max()
            .unwrap();

        let mut readfds = FdSet::empty();
        let mut writefds = FdSet::empty();

        // Nothing to start
        assert_eq!(
            select_raw(
                maxfd + 1,
                Some(&mut readfds),
                Some(&mut writefds),
                None,
                timeout_0,
            )
            .unwrap(),
            0,
        );

        // Now we write some data and test again
        w1.write_all(b"a").unwrap();
        readfds.clear();
        readfds.add(r1.as_raw_fd());
        readfds.add(r2.as_raw_fd());
        writefds.clear();
        assert_eq!(
            select_raw(
                maxfd + 1,
                Some(&mut readfds),
                Some(&mut writefds),
                None,
                timeout_0,
            )
            .unwrap(),
            1,
        );
        assert!(readfds.contains(r1.as_raw_fd()));

        // Now make sure reading two files works
        w2.write_all(b"a").unwrap();
        readfds.clear();
        readfds.add(r1.as_raw_fd());
        readfds.add(r2.as_raw_fd());
        writefds.clear();
        assert_eq!(
            select_raw(
                maxfd + 1,
                Some(&mut readfds),
                Some(&mut writefds),
                None,
                timeout_0,
            )
            .unwrap(),
            2,
        );
        assert!(readfds.contains(r1.as_raw_fd()));
        assert!(readfds.contains(r2.as_raw_fd()));

        // And checking if they're ready for writing
        readfds.clear();
        readfds.add(r1.as_raw_fd());
        readfds.add(r2.as_raw_fd());
        writefds.clear();
        writefds.add(w1.as_raw_fd());
        writefds.add(w2.as_raw_fd());
        assert_eq!(
            select_raw(
                maxfd + 1,
                Some(&mut readfds),
                Some(&mut writefds),
                None,
                timeout_0,
            )
            .unwrap(),
            4,
        );
        assert!(readfds.contains(r1.as_raw_fd()));
        assert!(readfds.contains(r2.as_raw_fd()));
        assert!(writefds.contains(w1.as_raw_fd()));
        assert!(writefds.contains(w2.as_raw_fd()));
    }

    #[test]
    fn test_pselect() {
        let timeout_0 = Some(Duration::from_secs(0));

        let (r1, mut w1) = crate::pipe().unwrap();
        let (r2, mut w2) = crate::pipe().unwrap();

        let maxfd = [&r1, &w1, &r2, &w2]
            .iter()
            .cloned()
            .map(AsRawFd::as_raw_fd)
            .max()
            .unwrap();

        let mut readfds = FdSet::empty();
        let mut writefds = FdSet::empty();

        // Nothing to start
        assert_eq!(
            pselect_raw(
                maxfd + 1,
                Some(&mut readfds),
                Some(&mut writefds),
                None,
                timeout_0,
                None,
            )
            .unwrap(),
            0,
        );

        // Now we write some data and test again
        w1.write_all(b"a").unwrap();
        readfds.clear();
        readfds.add(r1.as_raw_fd());
        readfds.add(r2.as_raw_fd());
        writefds.clear();
        assert_eq!(
            pselect_raw(
                maxfd + 1,
                Some(&mut readfds),
                Some(&mut writefds),
                None,
                timeout_0,
                None,
            )
            .unwrap(),
            1,
        );
        assert!(readfds.contains(r1.as_raw_fd()));

        // Now make sure reading two files works
        w2.write_all(b"a").unwrap();
        readfds.clear();
        readfds.add(r1.as_raw_fd());
        readfds.add(r2.as_raw_fd());
        writefds.clear();
        assert_eq!(
            pselect_raw(
                maxfd + 1,
                Some(&mut readfds),
                Some(&mut writefds),
                None,
                timeout_0,
                None,
            )
            .unwrap(),
            2,
        );
        assert!(readfds.contains(r1.as_raw_fd()));
        assert!(readfds.contains(r2.as_raw_fd()));

        // And checking if they're ready for writing
        readfds.clear();
        readfds.add(r1.as_raw_fd());
        readfds.add(r2.as_raw_fd());
        writefds.clear();
        writefds.add(w1.as_raw_fd());
        writefds.add(w2.as_raw_fd());
        assert_eq!(
            pselect_raw(
                maxfd + 1,
                Some(&mut readfds),
                Some(&mut writefds),
                None,
                timeout_0,
                None,
            )
            .unwrap(),
            4,
        );
        assert!(readfds.contains(r1.as_raw_fd()));
        assert!(readfds.contains(r2.as_raw_fd()));
        assert!(writefds.contains(w1.as_raw_fd()));
        assert!(writefds.contains(w2.as_raw_fd()));
    }

    #[test]
    fn test_select_simple() {
        let timeout_0 = Some(Duration::from_secs(0));

        let (r1, mut w1) = crate::pipe().unwrap();
        let (r2, mut w2) = crate::pipe().unwrap();

        // Nothing to start
        assert_eq!(
            select_simple(&[], &[], &[], timeout_0).unwrap(),
            (vec![], vec![], vec![]),
        );

        // Now we write some data and test again
        w1.write_all(b"a").unwrap();
        assert_eq!(
            select_simple(&[r1.as_raw_fd(), r2.as_raw_fd()], &[], &[], timeout_0).unwrap(),
            (vec![r1.as_raw_fd()], vec![], vec![]),
        );

        // Now make sure reading two files works
        w2.write_all(b"a").unwrap();
        assert_eq!(
            select_simple(&[r1.as_raw_fd(), r2.as_raw_fd()], &[], &[], timeout_0).unwrap(),
            (vec![r1.as_raw_fd(), r2.as_raw_fd()], vec![], vec![]),
        );

        // And checking if they're ready for writing
        w2.write_all(b"a").unwrap();
        assert_eq!(
            select_simple(
                &[r1.as_raw_fd(), r2.as_raw_fd()],
                &[w1.as_raw_fd(), w2.as_raw_fd()],
                &[],
                timeout_0,
            )
            .unwrap(),
            (
                vec![r1.as_raw_fd(), r2.as_raw_fd()],
                vec![w1.as_raw_fd(), w2.as_raw_fd()],
                vec![],
            ),
        );

        // Just check if they're ready for writing
        assert_eq!(
            select_simple(
                &[],
                &[w1.as_raw_fd(), w2.as_raw_fd()],
                &[],
                timeout_0,
            )
            .unwrap(),
            (
                vec![],
                vec![w1.as_raw_fd(), w2.as_raw_fd()],
                vec![],
            ),
        );

        // No timeout
        assert_eq!(
            select_simple(
                &[],
                &[w1.as_raw_fd(), w2.as_raw_fd()],
                &[],
                None,
            )
            .unwrap(),
            (
                vec![],
                vec![w1.as_raw_fd(), w2.as_raw_fd()],
                vec![],
            ),
        );
    }

    #[test]
    fn test_pselect_simple() {
        let timeout_0 = Some(Duration::from_secs(0));

        let (r1, mut w1) = crate::pipe().unwrap();
        let (r2, mut w2) = crate::pipe().unwrap();

        // Nothing to start
        assert_eq!(
            pselect_simple(&[], &[], &[], timeout_0, None).unwrap(),
            (vec![], vec![], vec![]),
        );

        // Now we write some data and test again
        w1.write_all(b"a").unwrap();
        assert_eq!(
            pselect_simple(&[r1.as_raw_fd(), r2.as_raw_fd()], &[], &[], timeout_0, None).unwrap(),
            (vec![r1.as_raw_fd()], vec![], vec![]),
        );

        // Now make sure reading two files works
        w2.write_all(b"a").unwrap();
        assert_eq!(
            pselect_simple(&[r1.as_raw_fd(), r2.as_raw_fd()], &[], &[], timeout_0, None).unwrap(),
            (vec![r1.as_raw_fd(), r2.as_raw_fd()], vec![], vec![]),
        );

        // And checking if they're ready for writing
        w2.write_all(b"a").unwrap();
        assert_eq!(
            pselect_simple(
                &[r1.as_raw_fd(), r2.as_raw_fd()],
                &[w1.as_raw_fd(), w2.as_raw_fd()],
                &[],
                timeout_0,
                None,
            )
            .unwrap(),
            (
                vec![r1.as_raw_fd(), r2.as_raw_fd()],
                vec![w1.as_raw_fd(), w2.as_raw_fd()],
                vec![],
            ),
        );

        // Just check if they're ready for writing
        assert_eq!(
            pselect_simple(
                &[],
                &[w1.as_raw_fd(), w2.as_raw_fd()],
                &[],
                timeout_0,
                None,
            )
            .unwrap(),
            (
                vec![],
                vec![w1.as_raw_fd(), w2.as_raw_fd()],
                vec![],
            ),
        );

        // No timeout
        assert_eq!(
            pselect_simple(
                &[],
                &[w1.as_raw_fd(), w2.as_raw_fd()],
                &[],
                None,
                None,
            )
            .unwrap(),
            (
                vec![],
                vec![w1.as_raw_fd(), w2.as_raw_fd()],
                vec![],
            ),
        );
    }
}
