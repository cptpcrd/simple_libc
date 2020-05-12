use std::io;

use super::Int;

pub fn flock_raw(fd: Int, op: i32) -> io::Result<()> {
    super::error::convert(unsafe { libc::flock(fd, op) }, ())
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

pub fn unlock(fd: Int) -> io::Result<()> {
    flock_raw(fd, libc::LOCK_UN)
}
