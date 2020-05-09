use std::cmp;
use std::env;
use std::ffi;
use std::fs;
use std::io;
use std::path::Path;
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

pub fn pipe2_raw(flags: i32) -> io::Result<(i32, i32)> {
    let mut fds: [i32; 2] = [0; 2];

    error::convert(unsafe {
        libc::pipe2(fds.as_mut_ptr(), flags)
    }, fds).map(|fds| (fds[0], fds[1]))
}

pub fn pipe2(flags: i32) -> io::Result<(fs::File, fs::File)> {
    let (r, w) = pipe2_raw(flags)?;
    unsafe { Ok((fs::File::from_raw_fd(r), fs::File::from_raw_fd(w))) }
}


pub fn fork() -> io::Result<i32> {
    error::convert_neg_ret(unsafe { libc::fork() })
}

pub fn execvp<U: Into<Vec<u8>> + Clone + Sized>(arg0: &str, argv: &[U]) -> io::Result<()> {
    let c_arg0 = ffi::CString::new(arg0)?;

    let mut c_argv: Vec<*mut libc::c_char> = Vec::with_capacity(argv.len() + 1);

    for arg in argv {
        c_argv.push(ffi::CString::new(arg.clone())?.into_raw())
    }

    c_argv.push(std::ptr::null_mut());

    unsafe {
        libc::execvp(c_arg0.as_ptr(), c_argv.as_ptr() as *const *const i8);
    }

    for arg in c_argv {
        if arg != std::ptr::null_mut() {
            unsafe {
                let _ = ffi::CString::from_raw(arg);
            }
        }
    }

    Err(io::Error::last_os_error().into())
}



pub fn close_fd(fd: i32) -> io::Result<()> {
    error::convert_nzero(unsafe { libc::close(fd) }, ())
}


#[cfg(target_os = "linux")]
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


/// The POSIX standard is unclear on some of the exact semantics, so for now this is Linux-only.
#[cfg(target_os = "linux")]
pub fn gethostname() -> io::Result<ffi::OsString> {
    let mut name_vec: Vec<i8> = Vec::new();
    name_vec.resize(constrain(sysconf(libc::_SC_HOST_NAME_MAX).unwrap_or(255), 10, 1024) as usize, 0);

    loop {
        match gethostname_raw(&mut name_vec) {
            Ok(()) => {
                return Ok(ffi::OsString::from_vec(name_vec.iter().take_while(|&x| *x > 0).map(|&x| x as u8).collect()))
            },
            Err(e) => {
                if let Some(raw_err) = e.raw_os_error() {
                    if raw_err == libc::EINVAL || raw_err == libc::ENAMETOOLONG {
                        name_vec.resize(name_vec.len() * 2, 0);
                        continue;
                    }
                }

                return Err(e);
            }
        };
    }
}
