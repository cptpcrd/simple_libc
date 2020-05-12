use std::io;

use super::error;
use super::Int;

macro_rules! fcntl_raw {
    ($fd:expr, $cmd:expr$(, $args:expr)*) => {
        error::convert_ret(libc::fcntl($fd, $cmd$(, $args)*));
    };
}

#[inline]
pub fn dupfd(fd: Int, min_fd: Int) -> io::Result<Int> {
    unsafe { fcntl_raw!(fd, libc::F_DUPFD, min_fd) }
}

#[inline]
pub fn dupfd_cloexec(fd: Int, min_fd: Int) -> io::Result<Int> {
    unsafe { fcntl_raw!(fd, libc::F_DUPFD_CLOEXEC, min_fd) }
}

#[inline]
pub fn getflags(fd: Int) -> io::Result<Int> {
    unsafe { fcntl_raw!(fd, libc::F_GETFD) }
}

#[inline]
pub fn setflags(fd: Int, flags: Int) -> io::Result<()> {
    unsafe { fcntl_raw!(fd, libc::F_SETFD, flags)? };
    Ok(())
}

pub fn is_inheritable(fd: Int) -> io::Result<bool> {
    Ok(getflags(fd)? & libc::FD_CLOEXEC == 0)
}

pub fn set_inheritable(fd: Int, inheritable: bool) -> io::Result<()> {
    let mut flags = getflags(fd)?;

    let currently_inheritable = flags & libc::FD_CLOEXEC != 0;
    
    if inheritable == currently_inheritable {
        return Ok(());
    }

    if inheritable {
        flags &= !(libc::FD_CLOEXEC as Int);
    } else {
        flags |= libc::FD_CLOEXEC;
    }

    setflags(fd, flags)
}

pub fn set_lock(fd: Int, lock: &libc::flock) -> io::Result<()> {
    unsafe { fcntl_raw!(fd, libc::F_SETLK, lock)? };
    Ok(())
}

pub fn set_lock_wait(fd: Int, lock: &libc::flock) -> io::Result<()> {
    unsafe { fcntl_raw!(fd, libc::F_SETLKW, lock)? };
    Ok(())
}

pub fn get_lock(fd: Int, lock: &mut libc::flock) -> io::Result<()> {
    unsafe { fcntl_raw!(fd, libc::F_GETLK, lock)? };
    Ok(())
}
