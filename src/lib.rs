use std::cmp;
use std::ffi;
use std::fs;
use std::io;
use std::os::unix::prelude::*;

mod constants;
mod externs;
mod internal;
mod types;

pub mod error;
pub mod exec;
pub mod fcntl;
pub mod grp;
pub mod ioctl;
pub mod lockf;
pub mod net;
pub mod poll;
pub mod pollers;
pub mod power;
pub mod priority;
pub mod process;
pub mod pwd;
pub mod resource;
pub mod select;
pub mod sigaction;
pub mod sigmask;
pub mod signal;
pub mod wait;

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
    target_os = "macos",
))]
pub mod flock;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod xattr;

#[macro_export]
macro_rules! attr_group {
    (#![$attr:meta] $($stmts:item)*) => {
        $(
            #[$attr]
            $stmts
        )*
    }
}

attr_group! {
    #![cfg(target_os = "linux")]

    pub mod epoll;
    pub mod inotify;
    pub mod ioprio;
    pub mod namespace;
    pub mod prctl;
    pub mod sched;
    pub mod signalfd;
}

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
pub type OffT = libc::off_t;
pub type SocklenT = libc::socklen_t;

#[cfg(target_os = "linux")]
pub type Off64T = libc::off64_t;

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
        Ok(ret) if ret >= 0 => Some(ret),
        _ => None,
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

pub fn pipe_inheritable_raw() -> io::Result<(Int, Int)> {
    let mut fds = [0; 2];

    error::convert_nzero_ret(unsafe { libc::pipe(fds.as_mut_ptr()) })?;

    Ok((fds[0], fds[1]))
}

pub fn pipe_inheritable() -> io::Result<(fs::File, fs::File)> {
    let (r, w) = pipe_inheritable_raw()?;
    unsafe { Ok((fs::File::from_raw_fd(r), fs::File::from_raw_fd(w))) }
}

#[allow(clippy::needless_return)]
pub fn pipe_raw() -> io::Result<(Int, Int)> {
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    ))]
    {
        return pipe2_raw(libc::O_CLOEXEC);
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    )))]
    {
        let fds = pipe_inheritable_raw()?;

        let res = fcntl::set_inheritable(fds.0, false)
            .and_then(|()| fcntl::set_inheritable(fds.1, false));

        if let Err(e) = res {
            unsafe {
                libc::close(fds.0);
            }
            unsafe {
                libc::close(fds.1);
            }

            return Err(e);
        }

        return Ok(fds);
    }
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
    let mut fds = [0; 2];

    error::convert_nzero_ret(unsafe { libc::pipe2(fds.as_mut_ptr(), flags) })?;

    Ok((fds[0], fds[1]))
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

pub fn dup_inheritable(oldfd: Int) -> io::Result<Int> {
    error::convert_neg_ret(unsafe { libc::dup(oldfd) })
}

#[inline]
pub fn dup(oldfd: Int) -> io::Result<Int> {
    fcntl::dupfd(oldfd, 0)
}

pub fn dup2_inheritable(oldfd: Int, newfd: Int) -> io::Result<Int> {
    if oldfd == newfd {
        fcntl::set_inheritable(newfd, true)?;
        Ok(newfd)
    } else {
        error::convert_neg_ret(unsafe { libc::dup2(oldfd, newfd) })
    }
}

pub fn dup2(oldfd: Int, newfd: Int) -> io::Result<Int> {
    let fd;

    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    ))]
    {
        if oldfd == newfd {
            // dup3() fails if oldfd == newfd.
            // Since we're emulating dup2(), let's just ignore this.
            fd = newfd;

            // However, let's match the behavior of the alternate dup2()-based
            // code below and make the file descriptor non-inheritable.
            fcntl::set_inheritable(fd, false)?;
        } else {
            fd = dup3(oldfd, newfd, libc::O_CLOEXEC)?;
        }
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    )))]
    {
        fd = dup2_inheritable(oldfd, newfd)?;

        if let Err(e) = fcntl::set_inheritable(fd, false) {
            if fd != oldfd {
                unsafe {
                    libc::close(fd);
                }
            }

            return Err(e);
        }
    }

    Ok(fd)
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
))]
pub fn dup3(oldfd: Int, newfd: Int, flags: Int) -> io::Result<Int> {
    error::convert_neg_ret(unsafe { libc::dup3(oldfd, newfd, flags) })
}

/// Closes the given file descriptor.
///
/// # Safety
///
/// If used to close file descriptors opened by builtin types (such as `File`
/// or `TcpStream`), this could violate assumptions made by those types, or
/// code that uses them.
pub unsafe fn close_fd(fd: Int) -> io::Result<()> {
    error::convert_nzero_ret(libc::close(fd))
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
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
        KillSpec::Pid(pid) => {
            if pid <= 0 {
                return Err(io::Error::from_raw_os_error(libc::EINVAL));
            }
            pid
        }
        KillSpec::Pgid(pgid) => {
            if pgid <= 1 {
                return Err(io::Error::from_raw_os_error(libc::EINVAL));
            }
            -pgid
        }
        KillSpec::CurPgrp => 0,
        KillSpec::All => -1,
    };

    error::convert_nzero_ret(unsafe { libc::kill(pid, sig) })
}

pub fn killpg(pgid: PidT, sig: Int) -> io::Result<()> {
    error::convert_nzero_ret(unsafe { libc::killpg(pgid, sig) })
}

#[cfg(target_os = "linux")]
pub fn tgkill(tgid: Int, tid: Int, sig: Int) -> io::Result<()> {
    error::convert_nzero_ret(unsafe { libc::syscall(libc::SYS_tgkill, tgid, tid, sig) })
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
pub fn sethostname<N: AsRef<ffi::OsStr>>(name: N) -> io::Result<()> {
    let name_vec: Vec<Char> = name
        .as_ref()
        .as_bytes()
        .iter()
        .map(|&x| x as Char)
        .collect();
    error::convert_nzero_ret(unsafe {
        libc::sethostname(name_vec.as_ptr(), name_vec.len() as SetHostnameSize)
    })
}

/// Attempts to read the current system hostname into the given slice.
///
/// The result is null-terminated. Behavior in the case that the vector
/// is not long enough is system-dependent.
pub fn gethostname_raw(name_vec: &mut [Char]) -> io::Result<()> {
    error::convert_nzero_ret(unsafe { libc::gethostname(name_vec.as_mut_ptr(), name_vec.len()) })
}

/// Attempts to determine the current system hostname.
pub fn gethostname() -> io::Result<ffi::OsString> {
    let mut name_vec = Vec::new();
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
pub fn setdomainname<N: AsRef<ffi::OsStr>>(name: N) -> io::Result<()> {
    let name_vec: Vec<Char> = name
        .as_ref()
        .as_bytes()
        .iter()
        .map(|&x| x as Char)
        .collect();
    error::convert_nzero_ret(unsafe { libc::setdomainname(name_vec.as_ptr(), name_vec.len()) })
}

#[cfg(target_os = "linux")]
pub fn getdomainname_raw(name_slice: &mut [Char]) -> io::Result<()> {
    error::convert_nzero_ret(unsafe {
        libc::getdomainname(name_slice.as_mut_ptr(), name_slice.len())
    })
}

#[cfg(target_os = "linux")]
pub fn getdomainname() -> io::Result<ffi::OsString> {
    let mut name_vec = Vec::new();
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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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

    error::convert_nzero_ret(unsafe { libc::uname(&mut utsname) })?;

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
            .take_while(|x| **x != 0)
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
        let (r, w) = pipe().unwrap();

        assert!(!fcntl::is_inheritable(r.as_raw_fd()).unwrap());
        assert!(!fcntl::is_inheritable(w.as_raw_fd()).unwrap());
    }

    #[test]
    fn test_kill_bounds() {
        assert_eq!(
            kill(KillSpec::Pid(0), libc::SIGTERM)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EINVAL)
        );
        assert_eq!(
            kill(KillSpec::Pid(-1), libc::SIGTERM)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EINVAL)
        );

        assert_eq!(
            kill(KillSpec::Pgid(1), libc::SIGTERM)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EINVAL)
        );
        assert_eq!(
            kill(KillSpec::Pgid(0), libc::SIGTERM)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EINVAL)
        );
        assert_eq!(
            kill(KillSpec::Pgid(-1), libc::SIGTERM)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EINVAL)
        );
    }

    #[test]
    fn test_pipe_inheritable() {
        let (mut r, mut w) = pipe_inheritable().unwrap();

        assert!(fcntl::is_inheritable(r.as_raw_fd()).unwrap());
        assert!(fcntl::is_inheritable(w.as_raw_fd()).unwrap());

        fcntl::set_inheritable(r.as_raw_fd(), false).unwrap();
        fcntl::set_inheritable(w.as_raw_fd(), false).unwrap();

        let mut buf: Vec<u8> = Vec::new();

        buf.resize(5, 10);
        w.write_all(&[0, 1, 2, 3]).unwrap();
        assert_eq!(r.read(&mut buf).unwrap(), 4);
        assert_eq!(buf, &[0, 1, 2, 3, 10]);

        w.write_all(&[4, 5, 6, 7]).unwrap();
        drop(w);
        buf.clear();
        r.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, &[4, 5, 6, 7]);

        let (r, w) = pipe_raw().unwrap();
        unsafe {
            close_fd(r).unwrap();
            close_fd(w).unwrap();
        }
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
    ))]
    #[test]
    fn test_pipe2() {
        let (r, w) = pipe2(0).unwrap();

        assert!(fcntl::is_inheritable(r.as_raw_fd()).unwrap());
        assert!(fcntl::is_inheritable(w.as_raw_fd()).unwrap());

        drop(r);
        drop(w);

        let (mut r, mut w) = pipe2(libc::O_CLOEXEC).unwrap();

        assert!(!fcntl::is_inheritable(r.as_raw_fd()).unwrap());
        assert!(!fcntl::is_inheritable(w.as_raw_fd()).unwrap());

        let mut buf: Vec<u8> = Vec::new();

        buf.resize(5, 10);
        w.write_all(&[0, 1, 2, 3]).unwrap();
        assert_eq!(r.read(&mut buf).unwrap(), 4);
        assert_eq!(buf, &[0, 1, 2, 3, 10]);

        w.write_all(&[4, 5, 6, 7]).unwrap();
        drop(w);
        buf.clear();
        r.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, &[4, 5, 6, 7]);

        let (r, w) = pipe2_raw(0).unwrap();
        unsafe {
            close_fd(r).unwrap();
            close_fd(w).unwrap();
        }
    }

    #[test]
    fn test_dup() {
        let (r, _w) = pipe().unwrap();

        let fd = dup(r.as_raw_fd()).unwrap();
        assert!(!fcntl::is_inheritable(fd).unwrap());
        unsafe {
            close_fd(fd).unwrap();
        }

        let fd = dup_inheritable(r.as_raw_fd()).unwrap();
        assert!(fcntl::is_inheritable(fd).unwrap());
        unsafe {
            close_fd(fd).unwrap();
        }
    }

    #[test]
    fn test_dup2() {
        let (r, w) = pipe().unwrap();

        dup2(r.as_raw_fd(), w.as_raw_fd()).unwrap();
        assert!(!fcntl::is_inheritable(w.as_raw_fd()).unwrap());

        dup2_inheritable(r.as_raw_fd(), w.as_raw_fd()).unwrap();
        assert!(fcntl::is_inheritable(w.as_raw_fd()).unwrap());

        // Now duplicate into the same file descriptor.

        // dup2() will always make it non-inheritable.
        dup2(r.as_raw_fd(), r.as_raw_fd()).unwrap();
        assert!(!fcntl::is_inheritable(r.as_raw_fd()).unwrap());

        // dup2_inheritable() will always make it inheritable.
        dup2_inheritable(r.as_raw_fd(), r.as_raw_fd()).unwrap();
        assert!(fcntl::is_inheritable(r.as_raw_fd()).unwrap());
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
