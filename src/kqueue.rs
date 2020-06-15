use std::convert::TryInto;
use std::io;
use std::os::unix::prelude::*;
use std::time::Duration;

use crate::{Int, Long, PidT};

crate::attr_group! {
    #![cfg(target_os = "netbsd")]

    type RawFilterType = u32;
    type RawFlagType = u32;
    type RawFflagType = u32;
}

crate::attr_group! {
    #![cfg(not(target_os = "netbsd"))]

    type RawFilterType = crate::Short;
    type RawFlagType = crate::Ushort;
    type RawFflagType = crate::Uint;
}

#[cfg(any(target_os = "openbsd", target_os = "dragonfly"))]
type RawDataType = libc::intptr_t;
#[cfg(not(any(target_os = "openbsd", target_os = "dragonfly")))]
type RawDataType = i64;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct RawKevent {
    raw: libc::kevent,
}

impl RawKevent {
    pub fn new(filter: EventFilter, action: EventAction, udata: *mut libc::c_void) -> Self {
        let mut res: Self = unsafe { std::mem::zeroed() };
        res.raw.flags = action.bits;
        res.raw.udata = udata;

        res
    }
}

pub enum EventFilter {
    Read(RawFd),
    Write(RawFd),
    Empty(RawFd),
    Vnode(RawFd, FileEvents),
    Proc(PidT, ProcEvents),
    ProcDesc(PidT, ProcEvents),
    Signal(Int),
}

bitflags::bitflags! {
    pub struct FileEvents: RawFflagType {
        #[cfg(target_os = "freebsd")]
        const CLOSE_NOWRITE = libc::NOTE_CLOSE as RawFlagType;
        #[cfg(target_os = "freebsd")]
        const CLOSE_WRITE = libc::NOTE_CLOSE_WRITE as RawFlagType;
        #[cfg(target_os = "freebsd")]
        const OPEN = libc::NOTE_OPEN as RawFlagType;
        #[cfg(target_os = "freebsd")]
        const READ = libc::NOTE_READ as RawFlagType;

        #[cfg(target_os = "openbsd")]
        const TRUNCATE = libc::NOTE_TRUNCATE as RawFlagType;

        const ATTRIB = libc::NOTE_ATTRIB as RawFlagType;
        const DELETE = libc::NOTE_DELETE as RawFlagType;
        const EXTEND = libc::NOTE_EXTEND as RawFlagType;
        const LINK = libc::NOTE_LINK as RawFlagType;
        const RENAME = libc::NOTE_RENAME as RawFlagType;
        const REVOKE = libc::NOTE_REVOKE as RawFlagType;
        const WRITE = libc::NOTE_WRITE as RawFlagType;
    }
}

bitflags::bitflags! {
    pub struct ProcEvents: RawFflagType {
        const EXIT = libc::NOTE_EXIT as RawFlagType;
        const FORK = libc::NOTE_FORK as RawFlagType;
        const EXEC = libc::NOTE_EXEC as RawFlagType;

        #[cfg(target_os = "macos")]
        const SIGNAL = libc::NOTE_SIGNAL as RawFlagType;
        #[cfg(target_os = "macos")]
        const REAP = libc::NOTE_REAP as RawFlagType;

        #[cfg(not(target_os = "macos"))]
        const TRACK = libc::NOTE_TRACK as RawFlagType;

        #[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
        const TRACK = libc::NOTE_TRACK as RawFlagType;
    }
}

bitflags::bitflags! {
    pub struct EventAction: RawFlagType {
        const ADD = libc::EV_ADD as RawFlagType;
        const ENABLE = libc::EV_ENABLE as RawFlagType;
        const DISABLE = libc::EV_DISABLE as RawFlagType;
        const DISPATCH = libc::EV_DISPATCH as RawFlagType;
        const DELETE = libc::EV_DELETE as RawFlagType;
        const RECEIPT = libc::EV_RECEIPT as RawFlagType;
        const ONESHOT = libc::EV_ONESHOT as RawFlagType;
        const CLEAR = libc::EV_CLEAR as RawFlagType;
        const EOF = libc::EV_EOF as RawFlagType;
        const ERROR = libc::EV_ERROR as RawFlagType;
    }
}

pub struct Kqueue {
    fd: RawFd,
}

impl Kqueue {
    pub fn new() -> io::Result<Self> {
        // NetBSD offers kqueue1(), which lets us specify O_CLOEXEC during
        // construction
        #[cfg(target_os = "netbsd")]
        return Ok(Self { fd: crate::error::convert_neg_ret(unsafe { crate::externs::kqueue1(libc::O_CLOEXEC) })? });

        // On other BSDs, we have to settle for immediately fcntl()ing it to be
        // non-inheritable.
        // fork()ed children don't inherit kqueues by default, but we want to be
        // safe -- the program may call exec() without fork()ing.
        #[cfg(not(target_os = "netbsd"))]
        {
            let fd = crate::error::convert_neg_ret(unsafe { libc::kqueue() })?;
            // Construct it now so if the set_inheritable() call fails
            // drop() will be called to close it
            let kqueue = Self { fd };

            crate::fcntl::set_inheritable(kqueue.as_raw_fd(), false)?;

            Ok(kqueue)
        }
    }

    pub fn kevent(&self, changes: &[RawKevent], events: &mut [RawKevent], timeout: Option<Duration>) -> io::Result<usize> {
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
                changes.as_ptr() as *const libc::kevent,
                changes.len() as Int,
                events.as_mut_ptr() as *mut libc::kevent,
                events.len() as Int,
                raw_timeout,
            )
        })?;

        Ok(n as usize)
    }
}

impl AsRawFd for Kqueue {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl IntoRawFd for Kqueue {
    #[inline]
    fn into_raw_fd(self) -> RawFd {
        self.fd
    }
}

impl Drop for Kqueue {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}
