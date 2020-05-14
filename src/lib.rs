use std::cmp;
use std::ffi;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::FromRawFd;

mod constants;
pub mod error;
mod externs;
pub mod fcntl;
pub mod grp;
pub mod net;
pub mod poll;
pub mod power;
pub mod process;
pub mod pwd;
pub mod select;
pub mod signal;

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
    target_os = "macos",
))]
pub mod flock;

#[cfg(target_os = "linux")]
pub mod epoll;
#[cfg(target_os = "linux")]
pub mod inotify;

pub type Short = libc::c_short;
pub type Ushort = libc::c_ushort;
pub type Int = libc::c_int;
pub type Uint = libc::c_uint;
pub type Long = libc::c_long;
pub type Ulong = libc::c_ulong;
pub type LongLong = libc::c_longlong;
pub type UlongLong = libc::c_ulonglong;

pub type SizeT = libc::size_t;
pub type SsizeT = libc::ssize_t;

pub type Char = libc::c_char;
pub type Schar = libc::c_schar;
pub type Uchar = libc::c_uchar;

pub type Float = libc::c_float;
pub type Double = libc::c_double;

pub type IdT = libc::id_t;
pub type PidT = libc::pid_t;
pub type UidT = libc::uid_t;
pub type GidT = libc::gid_t;

/// Flush filesystem write caches.
///
/// See the man page for sync(2) for more details.
pub fn sync() {
    unsafe { libc::sync() };
}

/// Get the value of runtime constants/limits.
///
/// Given a "name" (one of the `libc::_SC_*` constants),
/// returns the associated configuration value, unless
/// an error occurred (usually when the "name" is not
/// recognized).
///
/// Unlike the `sysconf()` wrapper, this function does not
/// treat return values < 0 specially; that is left to the
/// user.
pub fn sysconf_raw(name: Int) -> io::Result<Long> {
    error::set_errno_success();
    error::convert_if_errno_ret(unsafe { libc::sysconf(name) })
}

/// Get the value of runtime constants/limits.
///
/// Given a "name" (one of the `libc::_SC_*` constants),
/// returns the associated configuration value.
///
/// `None` is returned if an error occurs (usually when
/// the "name" is not recognized) OR if the value returned
/// by the C function `sysconf()` is < 0 (usually indicates
/// no limit). To differentiate between these two
/// possibilities, use `sysconf_raw()`.
pub fn sysconf(name: Int) -> Option<Long> {
    match sysconf_raw(name) {
        Ok(ret) => {
            if ret < 0 {
                return None;
            }
            Some(ret)
        }
        Err(_) => None,
    }
}

/// Constrain a value to a particular range.
///
/// This is included because sometimes when using `sysconf()`
/// it's helpful to constrain a value to a sane range before
/// using it as a buffer size or similar.
pub fn constrain<T: Ord + Eq>(val: T, min: T, max: T) -> T {
    // Help users who get the order wrong
    debug_assert!(max >= min);

    cmp::min(cmp::max(val, min), max)
}

pub fn pipe_raw() -> io::Result<(Int, Int)> {
    let mut fds: [Int; 2] = [0; 2];

    error::convert(unsafe { libc::pipe(fds.as_mut_ptr()) }, fds).map(|fds| (fds[0], fds[1]))
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
pub fn pipe2_raw(flags: Int) -> io::Result<(Int, Int)> {
    let mut fds: [Int; 2] = [0; 2];

    error::convert(unsafe { libc::pipe2(fds.as_mut_ptr(), flags) }, fds).map(|fds| (fds[0], fds[1]))
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
))]
pub fn pipe2(flags: Int) -> io::Result<(fs::File, fs::File)> {
    let (r, w) = pipe2_raw(flags)?;
    unsafe { Ok((fs::File::from_raw_fd(r), fs::File::from_raw_fd(w))) }
}

/// Closes the given file descriptor.
pub fn close_fd(fd: Int) -> io::Result<()> {
    error::convert_nzero(unsafe { libc::close(fd) }, ())
}

#[derive(Debug)]
pub enum KillSpec {
    /// Kill by process ID (must be > 0)
    Pid(PidT),
    /// Kill by process group ID (must be > 1)
    Pgid(PidT),
    /// All processes in this process's process group
    CurPgrp,
    /// All processes except an implementation-defined list
    ///
    /// On Linux, this kills everything except the current process and PID 1.
    All,
}

pub fn kill(spec: KillSpec, sig: Int) -> io::Result<()> {
    let pid = match spec {
        KillSpec::Pid(pid) => pid,
        KillSpec::Pgid(pgid) => -pgid,
        KillSpec::CurPgrp => 0,
        KillSpec::All => -1,
    };

    error::convert_nzero(unsafe { libc::kill(pid, sig) }, ())
}

pub fn killpg(pgid: PidT, sig: Int) -> io::Result<()> {
    error::convert_nzero(unsafe { libc::killpg(pgid, sig) }, ())
}

#[cfg(target_os = "linux")]
pub fn tgkill(tgid: Int, tid: Int, sig: Int) -> io::Result<()> {
    error::convert_nzero(
        unsafe { libc::syscall(libc::SYS_tgkill, tgid, tid, sig) },
        (),
    )
}

#[cfg(any(target_os = "linux", target_os = "openbsd", target_os = "netbsd"))]
type SetHostnameSize = SizeT;

#[cfg(any(target_os = "freebsd", target_os = "dragonfly", target_os = "macos"))]
type SetHostnameSize = Int;

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
    target_os = "macos",
))]
pub fn sethostname(name: &ffi::OsString) -> io::Result<()> {
    let name_vec: Vec<Char> = name.clone().into_vec().iter().map(|&x| x as Char).collect();
    error::convert_nzero(
        unsafe { libc::sethostname(name_vec.as_ptr(), name_vec.len() as SetHostnameSize) },
        (),
    )
}

/// Attempts to read the current system hostname into the given vector.
///
/// The result is null-terminated. Behavior in the case that the vector
/// is not long enough is system-dependent.
pub fn gethostname_raw(name_vec: &mut Vec<Char>) -> io::Result<()> {
    error::convert_nzero(
        unsafe { libc::gethostname(name_vec.as_mut_ptr(), name_vec.len()) },
        (),
    )
}

/// Attempts to determine the current system hostname.
pub fn gethostname() -> io::Result<ffi::OsString> {
    let mut name_vec: Vec<Char> = Vec::new();
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
            }
            Err(e) => {
                if let Some(raw_err) = e.raw_os_error() {
                    if (raw_err == libc::EINVAL || raw_err == libc::ENAMETOOLONG)
                        && name_vec.len() < orig_size * 10
                    {
                        name_vec.resize(name_vec.len() * 2, 0);
                        continue;
                    }
                }

                return Err(e);
            }
        };
    }
}

#[cfg(target_os = "linux")]
pub fn setdomainname(name: &ffi::OsString) -> io::Result<()> {
    let name_vec: Vec<Char> = name.clone().into_vec().iter().map(|&x| x as Char).collect();
    error::convert_nzero(
        unsafe { libc::setdomainname(name_vec.as_ptr(), name_vec.len()) },
        (),
    )
}

#[cfg(target_os = "linux")]
pub fn getdomainname_raw(name_vec: &mut Vec<Char>) -> io::Result<()> {
    error::convert_nzero(
        unsafe { libc::getdomainname(name_vec.as_mut_ptr(), name_vec.len()) },
        (),
    )
}

#[cfg(target_os = "linux")]
pub fn getdomainname() -> io::Result<ffi::OsString> {
    let mut name_vec: Vec<Char> = Vec::new();
    let orig_size = 128;
    name_vec.resize(orig_size, 0);

    loop {
        match getdomainname_raw(&mut name_vec) {
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
            }
            Err(e) => {
                if error::is_einval(&e) && name_vec.len() < orig_size * 10 {
                    name_vec.resize(name_vec.len() * 2, 0);
                    continue;
                }

                return Err(e);
            }
        };
    }
}

#[derive(Debug)]
pub struct Utsname {
    pub sysname: ffi::OsString,
    pub nodename: ffi::OsString,
    pub release: ffi::OsString,
    pub version: ffi::OsString,
    pub machine: ffi::OsString,
    #[cfg(target_os = "linux")]
    pub domainname: ffi::OsString,
}

/// Returns a `Utsname` struct containing information about the
/// current system.
pub fn uname() -> io::Result<Utsname> {
    let mut utsname = unsafe { std::mem::zeroed::<libc::utsname>() };

    error::convert_nzero(unsafe { libc::uname(&mut utsname) }, ())?;

    Ok(Utsname {
        sysname: bytes_to_osstring(utsname.sysname.iter()),
        nodename: bytes_to_osstring(utsname.nodename.iter()),
        release: bytes_to_osstring(utsname.release.iter()),
        version: bytes_to_osstring(utsname.version.iter()),
        machine: bytes_to_osstring(utsname.machine.iter()),
        #[cfg(target_os = "linux")]
        domainname: bytes_to_osstring(utsname.domainname.iter()),
    })
}

/// This takes a type that implements IntoIterator<Item=&Char> and constructs an OsString.
fn bytes_to_osstring<'a, T: IntoIterator<Item = &'a Char>>(bytes: T) -> ffi::OsString {
    ffi::OsString::from_vec(
        bytes
            .into_iter()
            .take_while(|x| **x > 0)
            .map(|x| *x as u8)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use super::*;

    #[test]
    fn test_sync() {
        sync();
    }

    #[test]
    fn test_constrain() {
        assert_eq!(constrain(-1, 0, 10), 0);
        assert_eq!(constrain(3, 0, 10), 3);
        assert_eq!(constrain(13, 0, 10), 10);
    }

    #[test]
    fn test_pipe() {
        let (mut r, mut w) = pipe().unwrap();
        let mut buf: Vec<u8> = Vec::new();

        buf.resize(5, 10);
        w.write(&[0, 1, 2, 3]).unwrap();
        assert_eq!(r.read(&mut buf).unwrap(), 4);
        assert_eq!(buf, &[0, 1, 2, 3, 10]);

        w.write(&[4, 5, 6, 7]).unwrap();
        drop(w);
        buf.clear();
        r.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, &[4, 5, 6, 7]);

        let (r, w) = pipe_raw().unwrap();
        close_fd(r).unwrap();
        close_fd(w).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_pipe2() {
        let (mut r, mut w) = pipe2(0).unwrap();
        let mut buf: Vec<u8> = Vec::new();

        buf.resize(5, 10);
        w.write(&[0, 1, 2, 3]).unwrap();
        assert_eq!(r.read(&mut buf).unwrap(), 4);
        assert_eq!(buf, &[0, 1, 2, 3, 10]);

        w.write(&[4, 5, 6, 7]).unwrap();
        drop(w);
        buf.clear();
        r.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, &[4, 5, 6, 7]);

        let (r, w) = pipe2_raw(0).unwrap();
        close_fd(r).unwrap();
        close_fd(w).unwrap();
    }

    #[test]
    fn test_uname_hostname() {
        uname().unwrap();
        gethostname().unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_domainname() {
        getdomainname().unwrap();
    }
}
