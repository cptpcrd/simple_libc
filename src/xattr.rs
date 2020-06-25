use std::ffi;
use std::io;
use std::os::unix::prelude::*;

use crate::error;
use crate::{Char, Int};

fn getxattr_raw_internal(
    path: &ffi::CStr,
    name: &ffi::CStr,
    value: &mut [u8],
    follow_links: bool,
) -> io::Result<usize> {
    #[cfg(target_os = "linux")]
    let callback_fn = if follow_links {
        libc::getxattr
    } else {
        libc::lgetxattr
    };

    #[cfg(target_os = "macos")]
    let callback_fn = libc::getxattr;

    #[cfg(target_os = "macos")]
    let flags = if follow_links {
        0
    } else {
        libc::XATTR_NOFOLLOW
    };

    let n = error::convert_neg_ret(unsafe {
        callback_fn(
            path.as_ptr(),
            name.as_ptr(),
            value.as_mut_ptr() as *mut libc::c_void,
            value.len(),
            #[cfg(target_os = "macos")]
            0,
            #[cfg(target_os = "macos")]
            flags,
        )
    })?;

    Ok(n as usize)
}

pub fn getxattr_raw<P: AsRef<ffi::OsStr>, N: AsRef<ffi::OsStr>>(
    path: P,
    name: N,
    value: &mut [u8],
    follow_links: bool,
) -> io::Result<usize> {
    let c_path = ffi::CString::new(path.as_ref().as_bytes())?;
    let c_name = ffi::CString::new(name.as_ref().as_bytes())?;

    getxattr_raw_internal(&c_path, &c_name, value, follow_links)
}

pub fn getxattr<P: AsRef<ffi::OsStr>, N: AsRef<ffi::OsStr>>(
    path: P,
    name: N,
    follow_links: bool,
) -> io::Result<Vec<u8>> {
    let c_path = ffi::CString::new(path.as_ref().as_bytes())?;
    let c_name = ffi::CString::new(name.as_ref().as_bytes())?;

    let mut buf = Vec::new();
    let init_size = getxattr_raw_internal(&c_path, &c_name, &mut buf, follow_links)?;
    buf.resize(init_size, 0);

    loop {
        match getxattr_raw_internal(&c_path, &c_name, &mut buf, follow_links) {
            Ok(n) => {
                buf.resize(n as usize, 0);

                return Ok(buf);
            }
            Err(e) => {
                if !error::is_erange(&e) || buf.len() > init_size * 4 {
                    return Err(e);
                }
            }
        }

        buf.resize(buf.len() * 2, 0);
    }
}

fn fgetxattr_raw_internal(fd: Int, name: &ffi::CStr, value: &mut [u8]) -> io::Result<usize> {
    let n = error::convert_neg_ret(unsafe {
        libc::fgetxattr(
            fd,
            name.as_ptr(),
            value.as_mut_ptr() as *mut libc::c_void,
            value.len(),
            #[cfg(target_os = "macos")]
            0,
            #[cfg(target_os = "macos")]
            0,
        )
    })?;

    Ok(n as usize)
}

pub fn fgetxattr_raw<N: AsRef<ffi::OsStr>>(
    fd: Int,
    name: N,
    value: &mut [u8],
) -> io::Result<usize> {
    let c_name = ffi::CString::new(name.as_ref().as_bytes())?;

    fgetxattr_raw_internal(fd, &c_name, value)
}

pub fn fgetxattr<N: AsRef<ffi::OsStr>>(fd: Int, name: N) -> io::Result<Vec<u8>> {
    let c_name = ffi::CString::new(name.as_ref().as_bytes())?;

    let mut buf = Vec::new();
    let init_size = fgetxattr_raw_internal(fd, &c_name, &mut buf)?;
    buf.resize(init_size, 0);

    loop {
        match fgetxattr_raw_internal(fd, &c_name, &mut buf) {
            Ok(n) => {
                buf.resize(n, 0);

                return Ok(buf);
            }
            Err(e) => {
                if !error::is_erange(&e) || buf.len() > init_size * 4 {
                    return Err(e);
                }
            }
        }

        buf.resize(buf.len() * 2, 0);
    }
}

pub fn listxattr_raw(path: &ffi::CStr, list: &mut [u8], follow_links: bool) -> io::Result<usize> {
    #[cfg(target_os = "linux")]
    let callback_fn = if follow_links {
        libc::listxattr
    } else {
        libc::llistxattr
    };

    #[cfg(target_os = "macos")]
    let callback_fn = libc::listxattr;

    #[cfg(target_os = "macos")]
    let flags = if follow_links {
        0
    } else {
        libc::XATTR_NOFOLLOW
    };

    let n = error::convert_neg_ret(unsafe {
        callback_fn(
            path.as_ptr(),
            list.as_mut_ptr() as *mut Char,
            list.len(),
            #[cfg(target_os = "macos")]
            flags,
        )
    })?;

    Ok(n as usize)
}

pub fn listxattr<P: AsRef<ffi::OsStr>>(
    path: P,
    follow_links: bool,
) -> io::Result<Vec<ffi::OsString>> {
    let c_path = ffi::CString::new(path.as_ref().as_bytes())?;

    let mut c_list = Vec::new();
    let init_size = listxattr_raw(&c_path, &mut c_list, follow_links)?;
    c_list.resize(init_size, 0);

    loop {
        match listxattr_raw(&c_path, &mut c_list, follow_links) {
            Ok(n) => {
                c_list.resize(n as usize, 0);
                break;
            }
            Err(e) => {
                if !error::is_erange(&e) || c_list.len() > init_size * 4 {
                    return Err(e);
                }
            }
        }

        c_list.resize(c_list.len() * 2, 0);
    }

    let mut res = Vec::new();

    let mut it = c_list.into_iter().peekable();
    while it.peek().is_some() {
        res.push(ffi::OsString::from_vec(
            it.by_ref().take_while(|x| *x != 0).collect(),
        ));
    }

    Ok(res)
}

pub fn flistxattr_raw(fd: Int, list: &mut [u8]) -> io::Result<usize> {
    let n = error::convert_neg_ret(unsafe {
        libc::flistxattr(
            fd,
            list.as_mut_ptr() as *mut Char,
            list.len(),
            #[cfg(target_os = "macos")]
            0,
        )
    })?;

    Ok(n as usize)
}

pub fn flistxattr(fd: Int) -> io::Result<Vec<ffi::OsString>> {
    let mut c_list = Vec::new();
    let init_size = flistxattr_raw(fd, &mut c_list)?;
    c_list.resize(init_size, 0);

    loop {
        match flistxattr_raw(fd, &mut c_list) {
            Ok(n) => {
                c_list.resize(n as usize, 0);
                break;
            }
            Err(e) => {
                if !error::is_erange(&e) || c_list.len() > init_size * 4 {
                    return Err(e);
                }
            }
        }

        c_list.resize(c_list.len() * 2, 0);
    }

    let mut res = Vec::new();

    let mut it = c_list.into_iter().peekable();
    while it.peek().is_some() {
        res.push(ffi::OsString::from_vec(
            it.by_ref().take_while(|x| *x != 0).collect(),
        ));
    }

    Ok(res)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_listxattr() {
        let current_exe = std::env::current_exe().unwrap();

        listxattr(&current_exe, false).unwrap();
        listxattr(&current_exe, true).unwrap();

        let f = fs::File::open(&current_exe).unwrap();
        flistxattr(f.as_raw_fd()).unwrap();
    }
}
