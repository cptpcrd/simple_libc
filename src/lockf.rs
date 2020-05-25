use std::io;

use super::{Int, OffT};

enum Cmd {
    LOCK = libc::F_LOCK as isize,
    TLOCK = libc::F_TLOCK as isize,
    ULOCK = libc::F_ULOCK as isize,
    TEST = libc::F_TEST as isize,
}

fn lockf_raw(fd: Int, cmd: Cmd, len: OffT) -> io::Result<()> {
    super::error::convert(unsafe { libc::lockf(fd, cmd as Int, len) }, ())
}

/// Lock the section of the given file starting at the current position
/// and proceeding for `len` bytes.
///
/// If `block` is `true` and part of this section is locked by another process,
/// the call blocks until the lock is released. If `block` is `false` and part
/// of this section is locked by another process, the call fails immediately with
/// either `EAGAIN` or `EACCES`.
///
/// If the section to be locked overlaps an earlier locked section, the sections
/// are merged.
pub fn lock(fd: Int, len: OffT, block: bool) -> io::Result<()> {
    let cmd = if block { Cmd::LOCK } else { Cmd::TLOCK };

    lockf_raw(fd, cmd, len)
}

/// Unlock the section of the given file starting at the current position
/// and proceeding for `len` bytes.
///
/// This may cause a locked section to be split into two locked sections.
#[inline]
pub fn unlock(fd: Int, len: OffT) -> io::Result<()> {
    lockf_raw(fd, Cmd::ULOCK, len)
}

/// Checks if part or all of the section of the given file starting at
/// the current position and proceeding for `len` bytes is locked by
/// another process.
pub fn is_locked_other(fd: Int, len: OffT) -> io::Result<bool> {
    match lockf_raw(fd, Cmd::TEST, len) {
        Ok(()) => Ok(false),
        Err(e) => match e.raw_os_error() {
            Some(libc::EAGAIN) | Some(libc::EACCES) => Ok(true),
            _ => Err(e),
        },
    }
}
