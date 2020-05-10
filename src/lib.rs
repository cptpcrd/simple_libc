use std::cmp;
use std::ffi;
use std::fs;
use std::io;
use std::os::unix::io::FromRawFd;
use std::os::unix::ffi::OsStringExt;

use libc;

pub mod signal;
pub mod pwd;
pub mod grp;
pub mod error;
pub mod power;
pub mod process;
pub mod net;
pub mod fcntl;
pub mod flock;
mod constants;

#[cfg(target_os = "linux")]
pub mod epoll;


pub fn sync() {
    unsafe { libc::sync() };
}


pub fn sysconf_raw(name: i32) -> io::Result<i64> {
    error::set_errno_success();
    error::convert_if_errno_ret(unsafe {
        libc::sysconf(name)
    })
}

pub fn sysconf(name: i32) -> Option<i64> {
    match sysconf_raw(name) {
        Ok(ret) => {
            if ret < 0 {
                return None;
            }
            Some(ret)
        },
        Err(_) => None,
    }
}


pub fn constrain<T: Ord + Eq>(val: T, min: T, max: T) -> T {
    // Help users who get the order wrong
    debug_assert!(max >= min);

    cmp::min(cmp::max(val, min), max)
}


pub fn pipe_raw() -> io::Result<(i32, i32)> {
    let mut fds: [i32; 2] = [0; 2];

    error::convert(unsafe {
        libc::pipe(fds.as_mut_ptr())
    }, fds).map(|fds| (fds[0], fds[1]))
}

pub fn pipe() -> io::Result<(fs::File, fs::File)> {
    let (r, w) = pipe_raw()?;
    unsafe { Ok((fs::File::from_raw_fd(r), fs::File::from_raw_fd(w))) }
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
))]
pub fn pipe2_raw(flags: i32) -> io::Result<(i32, i32)> {
    let mut fds: [i32; 2] = [0; 2];

    error::convert(unsafe {
        libc::pipe2(fds.as_mut_ptr(), flags)
    }, fds).map(|fds| (fds[0], fds[1]))
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
))]
pub fn pipe2(flags: i32) -> io::Result<(fs::File, fs::File)> {
    let (r, w) = pipe2_raw(flags)?;
    unsafe { Ok((fs::File::from_raw_fd(r), fs::File::from_raw_fd(w))) }
}


pub fn close_fd(fd: i32) -> io::Result<()> {
    error::convert_nzero(unsafe { libc::close(fd) }, ())
}


#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
))]
pub fn sethostname(name: &ffi::OsString) -> io::Result<()> {
    let name_vec: Vec<i8> = name.clone().into_vec().iter().map(|&x| x as i8).collect();
    error::convert_nzero(unsafe {
        libc::sethostname(name_vec.as_ptr(), name_vec.len())
    }, ())
}


pub fn gethostname_raw(name_vec: &mut Vec<i8>) -> io::Result<()> {
    error::convert_nzero(unsafe {
        libc::gethostname(name_vec.as_mut_ptr(), name_vec.len())
    }, ())
}

pub fn gethostname() -> io::Result<ffi::OsString> {
    let mut name_vec: Vec<i8> = Vec::new();
    let orig_size = constrain(sysconf(libc::_SC_HOST_NAME_MAX).unwrap_or(255), 10, 1024) as usize;
    name_vec.resize(orig_size, 0);

    loop {
        match gethostname_raw(&mut name_vec) {
            Ok(()) => {
                let name = bytes_to_osstring(&name_vec);

                if name.len() >= name_vec.len() - 1 {
                    // Either no NULL byte was added, or it was added at the very end
                    // of the vector. The name may have been truncated; increase the size and try
                    // again.

                    if name_vec.len() < orig_size * 10 {
                        name_vec.resize(name_vec.len() * 2, 0);
                        continue;
                    }
                }

                return Ok(name);
            },
            Err(e) => {
                if let Some(raw_err) = e.raw_os_error() {
                    if raw_err == libc::EINVAL || raw_err == libc::ENAMETOOLONG {
                        if name_vec.len() < orig_size * 10 {
                            name_vec.resize(name_vec.len() * 2, 0);
                            continue;
                        }
                    }
                }

                return Err(e);
            }
        };
    }
}


/// This takes a type that implements IntoIterator<Item=&i8> and constructs an OsString.
fn bytes_to_osstring<'a, T: IntoIterator<Item=&'a i8>>(bytes: T) -> ffi::OsString {
    ffi::OsString::from_vec(bytes.into_iter().take_while(|x| **x > 0).map(|x| *x as u8).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constrain() {
        assert_eq!(constrain(-1, 0, 10), 0);
        assert_eq!(constrain(3, 0, 10), 3);
        assert_eq!(constrain(13, 0, 10), 10);
    }
}
