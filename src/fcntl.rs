use std::io;

use crate::error;
use crate::Int;

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

#[inline]
pub fn is_inheritable(fd: Int) -> io::Result<bool> {
    Ok(getflags(fd)? & libc::FD_CLOEXEC == 0)
}

pub fn set_inheritable(fd: Int, inheritable: bool) -> io::Result<()> {
    let mut flags = getflags(fd)?;

    let currently_inheritable = flags & libc::FD_CLOEXEC == 0;

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

#[inline]
pub fn set_lock(fd: Int, lock: &libc::flock) -> io::Result<()> {
    unsafe { fcntl_raw!(fd, libc::F_SETLK, lock)? };
    Ok(())
}

#[inline]
pub fn set_lock_wait(fd: Int, lock: &libc::flock) -> io::Result<()> {
    unsafe { fcntl_raw!(fd, libc::F_SETLKW, lock)? };
    Ok(())
}

#[inline]
pub fn get_lock(fd: Int, lock: &mut libc::flock) -> io::Result<()> {
    unsafe { fcntl_raw!(fd, libc::F_GETLK, lock)? };
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::unix::io::AsRawFd;

    use super::*;

    #[test]
    fn test_inheritable() {
        let f = std::fs::File::open("/dev/null").unwrap();

        set_inheritable(f.as_raw_fd(), false).unwrap();
        assert!(!is_inheritable(f.as_raw_fd()).unwrap());
        set_inheritable(f.as_raw_fd(), false).unwrap();
        assert!(!is_inheritable(f.as_raw_fd()).unwrap());

        set_inheritable(f.as_raw_fd(), true).unwrap();
        assert!(is_inheritable(f.as_raw_fd()).unwrap());
        set_inheritable(f.as_raw_fd(), true).unwrap();
        assert!(is_inheritable(f.as_raw_fd()).unwrap());

        set_inheritable(f.as_raw_fd(), false).unwrap();
        assert!(!is_inheritable(f.as_raw_fd()).unwrap());
        set_inheritable(f.as_raw_fd(), false).unwrap();
        assert!(!is_inheritable(f.as_raw_fd()).unwrap());
    }

    #[test]
    fn test_dupfd() {
        let f = std::fs::File::open("/dev/null").unwrap();

        let f2 = dupfd(f.as_raw_fd(), 0).unwrap();
        assert!(is_inheritable(f2).unwrap());
        unsafe {
            crate::close_fd(f2).unwrap();
        }

        let f2 = dupfd_cloexec(f.as_raw_fd(), 0).unwrap();
        assert!(!is_inheritable(f2).unwrap());
        unsafe {
            crate::close_fd(f2).unwrap();
        }
    }
}
