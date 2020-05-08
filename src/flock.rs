use std::io;
use libc;


pub fn flock_raw(fd: i32, op: i32) -> io::Result<()> {
    super::error::convert(unsafe {
        libc::flock(fd, op)
    }, ())
}


pub fn lock(fd: i32, exclusive: bool, block: bool) -> io::Result<()> {
    let mut op;
    if exclusive {
        op = libc::LOCK_EX;
    }
    else {
        op = libc::LOCK_SH;
    }

    if !block {
        op |= libc::LOCK_NB;
    }

    flock_raw(fd, op)
}

pub fn unlock(fd: i32) -> io::Result<()> {
    flock_raw(fd, libc::LOCK_UN)
}
