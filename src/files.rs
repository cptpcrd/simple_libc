use std::ffi::{CStr, CString, OsStr, OsString};
use std::io;
use std::os::unix::prelude::*;

use crate::internal::minus_one_either;
use crate::{Char, GidT, UidT};

pub fn readlinkat_raw(dirfd: Option<RawFd>, name: &CStr, buf: &mut [u8]) -> io::Result<usize> {
    let n = crate::error::convert_neg_ret(unsafe {
        libc::readlinkat(
            dirfd.unwrap_or(libc::AT_FDCWD),
            name.as_ptr(),
            buf.as_mut_ptr() as *mut Char,
            buf.len(),
        )
    })?;

    Ok(n as usize)
}

pub fn readlinkat_cstr(dirfd: Option<RawFd>, name: &CStr) -> io::Result<CString> {
    let mut buf = vec![0; 1024];

    loop {
        let n = readlinkat_raw(dirfd, name, &mut buf)?;

        if n < buf.len() - 1 {
            buf.resize(n, 0);

            // Possible cases we need to handle:
            // n=0
            // n=3, buf="abc"
            // n=4, buf="abc\0"

            if buf.last() == Some(&0) {
                buf.pop();
            }

            // Safety: We just checked for a terminating NULL and
            // added one if it wasn't present
            return Ok(unsafe { CString::from_vec_unchecked(buf) });
        } else {
            buf.resize(buf.len() * 2, 0);
        }
    }
}

pub fn readlinkat<N: AsRef<OsStr>>(dirfd: Option<RawFd>, name: N) -> io::Result<OsString> {
    let name = CString::new(name.as_ref().as_bytes())?;

    let target = readlinkat_cstr(dirfd, &name)?;

    Ok(OsString::from_vec(target.into_bytes()))
}

pub fn symlinkat_raw(target: &CStr, fd: Option<RawFd>, name: &CStr) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe {
        libc::symlinkat(target.as_ptr(), fd.unwrap_or(libc::AT_FDCWD), name.as_ptr())
    })
}

pub fn symlinkat<T: AsRef<OsStr>, N: AsRef<OsStr>>(
    target: T,
    fd: Option<RawFd>,
    name: N,
) -> io::Result<()> {
    symlinkat_raw(
        &CString::new(target.as_ref().as_bytes())?,
        fd,
        &CString::new(name.as_ref().as_bytes())?,
    )
}

pub fn unlinkat_raw(dirfd: Option<RawFd>, name: &CStr, dir: bool) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe {
        libc::unlinkat(
            dirfd.unwrap_or(libc::AT_FDCWD),
            name.as_ptr(),
            if dir { libc::AT_REMOVEDIR } else { 0 },
        )
    })
}

pub fn unlinkat<N: AsRef<OsStr>>(dirfd: Option<RawFd>, name: N, dir: bool) -> io::Result<()> {
    unlinkat_raw(dirfd, &CString::new(name.as_ref().as_bytes())?, dir)
}

pub fn fchmodat_raw(
    dirfd: Option<RawFd>,
    name: &CStr,
    mode: u32,
    follow_symlinks: bool,
) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe {
        libc::fchmodat(
            dirfd.unwrap_or(libc::AT_FDCWD),
            name.as_ptr(),
            mode as libc::mode_t,
            if follow_symlinks {
                0
            } else {
                libc::AT_SYMLINK_NOFOLLOW
            },
        )
    })
}

pub fn fchmodat<N: AsRef<OsStr>>(
    dirfd: Option<RawFd>,
    name: N,
    mode: u32,
    follow_symlinks: bool,
) -> io::Result<()> {
    fchmodat_raw(
        dirfd,
        &CString::new(name.as_ref().as_bytes())?,
        mode,
        follow_symlinks,
    )
}

pub fn fchownat_raw(
    dirfd: Option<RawFd>,
    name: &CStr,
    owner: UidT,
    group: GidT,
    follow_symlinks: bool,
) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe {
        libc::fchownat(
            dirfd.unwrap_or(libc::AT_FDCWD),
            name.as_ptr(),
            owner,
            group,
            if follow_symlinks {
                0
            } else {
                libc::AT_SYMLINK_NOFOLLOW
            },
        )
    })
}

pub fn fchownat2_raw(
    dirfd: Option<RawFd>,
    name: &CStr,
    owner: Option<UidT>,
    group: Option<GidT>,
    follow_symlinks: bool,
) -> io::Result<()> {
    fchownat_raw(
        dirfd,
        name,
        owner.unwrap_or_else(minus_one_either),
        group.unwrap_or_else(minus_one_either),
        follow_symlinks,
    )
}

pub fn fchownat<N: AsRef<OsStr>>(
    dirfd: Option<RawFd>,
    name: N,
    owner: UidT,
    group: GidT,
    follow_symlinks: bool,
) -> io::Result<()> {
    fchownat_raw(
        dirfd,
        &CString::new(name.as_ref().as_bytes())?,
        owner,
        group,
        follow_symlinks,
    )
}

pub fn fchownat2<N: AsRef<OsStr>>(
    dirfd: Option<RawFd>,
    name: N,
    owner: Option<UidT>,
    group: Option<GidT>,
    follow_symlinks: bool,
) -> io::Result<()> {
    fchownat(
        dirfd,
        name,
        owner.unwrap_or_else(minus_one_either),
        group.unwrap_or_else(minus_one_either),
        follow_symlinks,
    )
}

pub fn fstatat_raw(
    fd: Option<RawFd>,
    path: &CStr,
    follow_symlinks: bool,
) -> io::Result<libc::stat> {
    let mut stat_buf = unsafe { std::mem::zeroed() };

    crate::error::convert_nzero_ret(unsafe {
        libc::fstatat(
            fd.unwrap_or(libc::AT_FDCWD),
            path.as_ptr(),
            &mut stat_buf,
            if follow_symlinks {
                0
            } else {
                libc::AT_SYMLINK_NOFOLLOW
            },
        )
    })?;

    Ok(stat_buf)
}

pub fn fstatat<P: AsRef<OsStr>>(
    fd: Option<RawFd>,
    path: P,
    follow_symlinks: bool,
) -> io::Result<libc::stat> {
    fstatat_raw(
        fd,
        &CString::new(path.as_ref().as_bytes())?,
        follow_symlinks,
    )
}

#[cfg(test)]
mod tests {
    use tempfile::{NamedTempFile, TempDir};

    use super::*;

    #[test]
    fn test_symlinks_no_fd() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        let uid = crate::process::geteuid();
        let gid = crate::process::getegid();

        assert_eq!(
            readlinkat(None, path.join("link"))
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ENOENT),
        );

        symlinkat("TARGET", None, path.join("link")).unwrap();

        assert_eq!(
            readlinkat(None, path.join("link")).unwrap(),
            OsString::from("TARGET"),
        );

        fstatat(None, path.join("link"), false).unwrap();

        fchownat(None, path.join("link"), uid, gid, false).unwrap();

        fchownat2(None, path.join("link"), None, Some(gid), false).unwrap();
        fchownat2(None, path.join("link"), Some(uid), None, false).unwrap();

        assert_eq!(
            fstatat(None, path.join("link"), true)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ENOENT),
        );

        assert_eq!(
            fchownat(None, path.join("link"), uid, gid, true)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ENOENT),
        );

        assert_eq!(
            fchmodat(None, path.join("link"), 0o600, true)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ENOENT),
        );

        unlinkat(None, path.join("link"), false).unwrap();
    }

    #[test]
    fn test_symlinks_fd() {
        let dir = TempDir::new().unwrap();
        let f = std::fs::File::open(dir.path()).unwrap();
        let fd = f.as_raw_fd();

        let uid = crate::process::geteuid();
        let gid = crate::process::getegid();

        assert_eq!(
            readlinkat(Some(fd), "link").unwrap_err().raw_os_error(),
            Some(libc::ENOENT),
        );

        symlinkat("TARGET", Some(fd), "link").unwrap();

        assert_eq!(
            readlinkat(Some(fd), "link").unwrap(),
            OsString::from("TARGET"),
        );

        fstatat(Some(fd), "link", false).unwrap();

        fchownat(Some(fd), "link", uid, gid, false).unwrap();
        fchownat2(Some(fd), "link", None, Some(gid), false).unwrap();
        fchownat2(Some(fd), "link", Some(uid), None, false).unwrap();

        assert_eq!(
            fstatat(Some(fd), "link", true).unwrap_err().raw_os_error(),
            Some(libc::ENOENT),
        );

        assert_eq!(
            fchownat(Some(fd), "link", uid, gid, true)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ENOENT),
        );

        assert_eq!(
            fchmodat(Some(fd), "link", 0o600, true)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ENOENT),
        );

        unlinkat(Some(fd), "link", false).unwrap();
    }

    #[test]
    fn test_file_no_fd() {
        let tmpf = NamedTempFile::new().unwrap();
        let path = tmpf.path();

        let uid = crate::process::geteuid();
        let gid = crate::process::getegid();

        fstatat(None, &path, false).unwrap();
        fstatat(None, &path, true).unwrap();

        fchmodat(None, &path, 0o600, true).unwrap();

        fchownat(None, &path, uid, gid, false).unwrap();
        fchownat(None, &path, uid, gid, true).unwrap();
        fchownat2(None, &path, None, Some(gid), false).unwrap();
        fchownat2(None, &path, Some(uid), None, false).unwrap();
    }
}
