use std::io;

use crate::Int;

pub fn flock_raw(fd: Int, op: i32) -> io::Result<()> {
    crate::error::convert(unsafe { libc::flock(fd, op) }, ())
}

pub fn lock(fd: Int, exclusive: bool, block: bool) -> io::Result<()> {
    let mut op = if exclusive {
        libc::LOCK_EX
    } else {
        libc::LOCK_SH
    };

    if !block {
        op |= libc::LOCK_NB;
    }

    flock_raw(fd, op)
}

#[inline]
pub fn unlock(fd: Int) -> io::Result<()> {
    flock_raw(fd, libc::LOCK_UN)
}
