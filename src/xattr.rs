use std::ffi::{CStr, CString, OsStr, OsString};
use std::io;
use std::os::unix::prelude::*;

use crate::error;
use crate::Int;

enum Target {
    File(CString),
    Link(CString),
    Fd(Int),
}

impl Target {
    fn build_from_path<P: AsRef<OsStr>>(path: P, follow_links: bool) -> io::Result<Self> {
        let c_path = CString::new(path.as_ref().as_bytes())?;

        Ok(if follow_links {
            Self::File(c_path)
        } else {
            Self::Link(c_path)
        })
    }

    fn getxattr_name<N: AsRef<OsStr>>(&self, name: N, value: &mut [u8]) -> io::Result<usize> {
        self.getxattr(&CString::new(name.as_ref().as_bytes())?, value)
    }

    fn getxattr(&self, name: &CStr, value: &mut [u8]) -> io::Result<usize> {
        unsafe {
            #[cfg(target_os = "linux")]
            let res = match self {
                Self::File(path) => libc::getxattr(
                    path.as_ptr(),
                    name.as_ptr(),
                    value.as_mut_ptr() as *mut libc::c_void,
                    value.len(),
                ),
                Self::Link(path) => libc::lgetxattr(
                    path.as_ptr(),
                    name.as_ptr(),
                    value.as_mut_ptr() as *mut libc::c_void,
                    value.len(),
                ),
                Self::Fd(fd) => libc::fgetxattr(
                    *fd,
                    name.as_ptr(),
                    value.as_mut_ptr() as *mut libc::c_void,
                    value.len(),
                ),
            };

            #[cfg(target_os = "macos")]
            let res = match self {
                Self::File(path) => libc::getxattr(
                    path.as_ptr(),
                    name.as_ptr(),
                    value.as_mut_ptr() as *mut libc::c_void,
                    value.len(),
                    0,
                    0,
                ),
                Self::Link(path) => libc::getxattr(
                    path.as_ptr(),
                    name.as_ptr(),
                    value.as_mut_ptr() as *mut libc::c_void,
                    value.len(),
                    0,
                    libc::XATTR_NOFOLLOW,
                ),
                Self::Fd(fd) => libc::fgetxattr(
                    *fd,
                    name.as_ptr(),
                    value.as_mut_ptr() as *mut libc::c_void,
                    value.len(),
                    0,
                    0,
                ),
            };

            let n = error::convert_neg_ret(res)?;
            Ok(n as usize)
        }
    }

    fn listxattr(&self, list: &mut [u8]) -> io::Result<usize> {
        unsafe {
            #[cfg(target_os = "linux")]
            let res = match self {
                Self::File(path) => libc::listxattr(
                    path.as_ptr(),
                    list.as_mut_ptr() as *mut crate::Char,
                    list.len(),
                ),
                Self::Link(path) => libc::llistxattr(
                    path.as_ptr(),
                    list.as_mut_ptr() as *mut crate::Char,
                    list.len(),
                ),
                Self::Fd(fd) => {
                    libc::flistxattr(*fd, list.as_mut_ptr() as *mut crate::Char, list.len())
                }
            };

            #[cfg(target_os = "macos")]
            let res = match self {
                Self::File(path) => libc::listxattr(
                    path.as_ptr(),
                    list.as_mut_ptr() as *mut crate::Char,
                    list.len(),
                    0,
                ),
                Self::Link(path) => libc::listxattr(
                    path.as_ptr(),
                    list.as_mut_ptr() as *mut crate::Char,
                    list.len(),
                    libc::XATTR_NOFOLLOW,
                ),
                Self::Fd(fd) => {
                    libc::flistxattr(*fd, list.as_mut_ptr() as *mut crate::Char, list.len(), 0)
                }
            };

            let n = error::convert_neg_ret(res)?;
            Ok(n as usize)
        }
    }
}

fn getxattr_impl(target: Target, name: &CStr) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    let init_size = target.getxattr(&name, &mut buf)?;

    if init_size == 0 {
        // Empty
        return Ok(buf);
    }

    buf.resize(init_size, 0);

    loop {
        match target.getxattr(&name, &mut buf) {
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

pub fn getxattr_raw<P: AsRef<OsStr>, N: AsRef<OsStr>>(
    path: P,
    name: N,
    value: &mut [u8],
    follow_links: bool,
) -> io::Result<usize> {
    Target::build_from_path(path, follow_links)?.getxattr_name(name, value)
}

pub fn getxattr<P: AsRef<OsStr>, N: AsRef<OsStr>>(
    path: P,
    name: N,
    follow_links: bool,
) -> io::Result<Vec<u8>> {
    let c_name = CString::new(name.as_ref().as_bytes())?;

    getxattr_impl(Target::build_from_path(path, follow_links)?, &c_name)
}

pub fn fgetxattr_raw<N: AsRef<OsStr>>(fd: Int, name: N, value: &mut [u8]) -> io::Result<usize> {
    Target::Fd(fd).getxattr_name(name, value)
}

pub fn fgetxattr<N: AsRef<OsStr>>(fd: Int, name: N) -> io::Result<Vec<u8>> {
    let c_name = CString::new(name.as_ref().as_bytes())?;

    getxattr_impl(Target::Fd(fd), &c_name)
}

fn listxattr_impl(target: Target) -> io::Result<Vec<OsString>> {
    let mut c_list = Vec::new();
    let init_size = target.listxattr(&mut c_list)?;

    if init_size == 0 {
        // Empty
        return Ok(Vec::new());
    }

    c_list.resize(init_size, 0);

    loop {
        match target.listxattr(&mut c_list) {
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
        res.push(OsString::from_vec(
            it.by_ref().take_while(|x| *x != 0).collect(),
        ));
    }

    Ok(res)
}

pub fn listxattr_raw<P: AsRef<OsStr>>(
    path: P,
    list: &mut [u8],
    follow_links: bool,
) -> io::Result<usize> {
    Target::build_from_path(path, follow_links)?.listxattr(list)
}

pub fn listxattr<P: AsRef<OsStr>>(path: P, follow_links: bool) -> io::Result<Vec<OsString>> {
    listxattr_impl(Target::build_from_path(path, follow_links)?)
}

#[inline]
pub fn flistxattr_raw(fd: Int, list: &mut [u8]) -> io::Result<usize> {
    Target::Fd(fd).listxattr(list)
}

#[inline]
pub fn flistxattr(fd: Int) -> io::Result<Vec<OsString>> {
    listxattr_impl(Target::Fd(fd))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_listxattr() {
        let mut buf: Vec<u8> = vec![0; 1024];

        let current_exe = std::env::current_exe().unwrap();

        assert_eq!(
            listxattr(&current_exe, false).unwrap(),
            Vec::<OsString>::new()
        );
        assert_eq!(listxattr_raw(&current_exe, &mut buf, false).unwrap(), 0);
        assert_eq!(
            listxattr(&current_exe, true).unwrap(),
            Vec::<OsString>::new()
        );
        assert_eq!(listxattr_raw(&current_exe, &mut buf, true).unwrap(), 0);

        let f = fs::File::open(&current_exe).unwrap();
        assert_eq!(flistxattr(f.as_raw_fd()).unwrap(), Vec::<OsString>::new());
        assert_eq!(flistxattr_raw(f.as_raw_fd(), &mut buf).unwrap(), 0);
    }
}
