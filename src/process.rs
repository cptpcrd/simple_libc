use std::ffi;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;

use super::externs;
use super::{Char, GidT, Int, PidT, UidT};

#[deprecated(since = "0.4.0", note = "Moved out of the 'process' module")]
pub mod exec {
    pub use crate::exec::*;
}

#[deprecated(since = "0.4.0", note = "Moved out of the 'process' module")]
pub mod priority {
    pub use crate::priority::*;
}

#[deprecated(since = "0.4.0", note = "Moved out of the 'process' module")]
pub mod resource {
    pub use crate::resource::*;
}

#[deprecated(since = "0.4.0", note = "Moved out of the 'process' module")]
pub mod sigaction {
    pub use crate::sigaction::*;
}

#[deprecated(since = "0.4.0", note = "Moved out of the 'process' module")]
pub mod sigmask {
    pub use crate::sigmask::*;
}

#[deprecated(since = "0.4.0", note = "Moved out of the 'process' module")]
pub mod wait {
    pub use crate::wait::*;
}

#[deprecated(since = "0.4.0", note = "Moved out of the 'process' module")]
#[cfg(target_os = "linux")]
pub mod namespace {
    pub use crate::namespace::*;
}
#[deprecated(since = "0.4.0", note = "Moved out of the 'process' module")]
#[cfg(target_os = "linux")]
pub mod prctl {
    pub use crate::prctl::*;
}
#[deprecated(since = "0.4.0", note = "Moved out of the 'process' module")]
#[cfg(target_os = "linux")]
pub mod signalfd {
    pub use crate::signalfd::*;
}

#[inline]
pub fn getpid() -> PidT {
    unsafe { libc::getpid() }
}

#[cfg(target_os = "linux")]
#[inline]
pub fn gettid() -> PidT {
    unsafe { libc::syscall(libc::SYS_gettid) as PidT }
}

#[inline]
pub fn getppid() -> PidT {
    unsafe { libc::getppid() }
}

/// Returns the current real user ID.
#[inline]
pub fn getuid() -> UidT {
    unsafe { libc::getuid() }
}

/// Returns the current effective user ID.
#[inline]
pub fn geteuid() -> UidT {
    unsafe { libc::geteuid() }
}

/// Returns the current real group ID.
#[inline]
pub fn getgid() -> GidT {
    unsafe { libc::getgid() }
}

/// Returns the current effective group ID.
#[inline]
pub fn getegid() -> GidT {
    unsafe { libc::getegid() }
}

/// Low-level interface to the C `getgroups()` function.
///
/// This attempts to store the current list of supplementary
/// group IDs in the provided vector. It is a very thin wrapper
/// around C's `getgroups()` function, so the semantics are
/// almost exactly the same.
///
/// Namely:
/// 1. If the vector is empty (length 0), it will not be modified;
///    instead, the number of current supplementary group IDs
///    will be returned.
/// 2. If the vector is long enough to hold all the current
///    supplementary group IDs, it will be filled with the current
///    supplementary group IDs, and the number of supplementary
///    group IDs will be returned.
/// 3. If the vector is not empty and it is also not long enough to
///    hold all the current supplementary group IDs, an error will be
///    returned.
///
/// In most cases, the `getgroups()` wrapper should be preferred.
pub fn getgroups_raw(groups: &mut [GidT]) -> io::Result<Int> {
    super::error::convert_neg_ret(unsafe {
        libc::getgroups(groups.len() as Int, groups.as_mut_ptr())
    })
}

/// Returns a vector containing the current supplementary
/// group IDs.
///
/// This is a higher-level wrapper that calls `getgroups_raw()` twice,
/// first to determine the number of groups and then again to actually
/// fill the list.
pub fn getgroups() -> io::Result<Vec<GidT>> {
    let mut groups: Vec<GidT> = Vec::new();

    let ngroups = getgroups_raw(&mut groups)?;

    groups.resize(ngroups as usize, 0);

    if getgroups_raw(&mut groups)? != ngroups {
        return Err(io::Error::last_os_error());
    }

    Ok(groups)
}

/// Returns a vector containing the real group ID, the effective group
/// ID, and all group IDs returned by `getgroups()`.
///
/// No guarantees are made about the order of the vector, or the
/// uniqueness of its elements.
pub fn getallgroups() -> io::Result<Vec<GidT>> {
    let mut groups = getgroups()?;

    let (rgid, egid) = getregid();

    groups.retain(|&x| x != rgid && x != egid);

    if rgid == egid {
        groups.insert(0, egid);
    } else {
        groups.splice(0..0, [rgid, egid].iter().cloned());
    }

    Ok(groups)
}

/// [NOT RECOMMENDED] Returns the username of the currently logged-in
/// user.
///
/// WARNING: Use of this function is not recommended (see the documentation
/// of the C function `getlogin()` for details).
/// In most cases, especially when security is important,
/// you should call `getuid()` and pass the result to
/// `pwd::Passwd::lookup_uid()`.
pub fn getlogin() -> io::Result<ffi::OsString> {
    // Get the initial buffer length from sysconf(), setting some sane defaults/constraints.
    let init_length = super::constrain(
        super::sysconf(libc::_SC_LOGIN_NAME_MAX).unwrap_or(256),
        64,
        1024,
    ) as usize;

    super::error::while_erange(
        |i| {
            let length = init_length * (i as usize + 1);
            let mut buf: Vec<Char> = Vec::new();

            super::error::convert_nzero(
                unsafe {
                    buf.resize(length, 0);
                    externs::getlogin_r(buf.as_mut_ptr(), length)
                },
                buf,
            )
            .map(|buf| {
                ffi::OsString::from_vec(
                    buf.iter()
                        .take_while(|x| **x > 0)
                        .map(|x| *x as u8)
                        .collect(),
                )
            })
        },
        10,
    )
}

pub fn setuid(uid: UidT) -> io::Result<()> {
    super::error::convert_nzero(unsafe { libc::setuid(uid) }, ())
}

pub fn seteuid(uid: UidT) -> io::Result<()> {
    super::error::convert_nzero(unsafe { libc::seteuid(uid) }, ())
}

pub fn setreuid(ruid: UidT, euid: UidT) -> io::Result<()> {
    super::error::convert_nzero(unsafe { externs::setreuid(ruid, euid) }, ())
}

pub fn setgid(gid: GidT) -> io::Result<()> {
    super::error::convert_nzero(unsafe { libc::setgid(gid) }, ())
}

pub fn setegid(gid: GidT) -> io::Result<()> {
    super::error::convert_nzero(unsafe { libc::setegid(gid) }, ())
}

pub fn setregid(rgid: GidT, egid: GidT) -> io::Result<()> {
    super::error::convert_nzero(unsafe { externs::setregid(rgid, egid) }, ())
}

#[cfg(target_os = "linux")]
type SetGroupsSize = super::SizeT;

#[cfg(any(
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "macos",
))]
type SetGroupsSize = Int;

pub fn setgroups(groups: &[GidT]) -> io::Result<()> {
    super::error::convert_nzero(
        unsafe { libc::setgroups(groups.len() as SetGroupsSize, groups.as_ptr()) },
        (),
    )
}

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "dragonfly"))] {
        pub fn getresuid() -> (UidT, UidT, UidT) {
            let mut ruid: UidT = 0;
            let mut euid: UidT = 0;
            let mut suid: UidT = 0;

            unsafe { externs::getresuid(&mut ruid, &mut euid, &mut suid); }
            (ruid, euid, suid)
        }

        pub fn getresgid() -> (GidT, GidT, GidT) {
            let mut rgid: GidT = 0;
            let mut egid: GidT = 0;
            let mut sgid: GidT = 0;

            unsafe { externs::getresgid(&mut rgid, &mut egid, &mut sgid); }
            (rgid, egid, sgid)
        }

        pub fn setresuid(ruid: UidT, euid: UidT, suid: UidT) -> io::Result<()> {
            super::error::convert_nzero(unsafe {
                externs::setresuid(ruid, euid, suid)
            }, ())
        }

        pub fn setresgid(rgid: GidT, egid: GidT, sgid: GidT) -> io::Result<()> {
            super::error::convert_nzero(unsafe {
                externs::setresgid(rgid, egid, sgid)
            }, ())
        }

        fn getreuid_impl() -> (UidT, UidT) {
            let (ruid, euid, _) = getresuid();
            (ruid, euid)
        }

        fn getregid_impl() -> (GidT, GidT) {
            let (rgid, egid, _) = getresgid();
            (rgid, egid)
        }
    }
    else {
        fn getreuid_impl() -> (UidT, UidT) {
            (getuid(), geteuid())
        }

        fn getregid_impl() -> (GidT, GidT) {
            (getgid(), getegid())
        }
    }
}

/// Gets the real and effective user IDs via the most efficient method possible.
///
/// On platforms with `getresuid()`, this function calls that function and discards
/// the saved UID. On other platforms, it combines the results of `getuid()` and
/// `geteuid()`.
#[inline]
pub fn getreuid() -> (UidT, UidT) {
    getreuid_impl()
}

/// Gets the real and effective group IDs via the most efficient method possible.
///
/// On platforms with `getresgid()`, this function calls that function and discards
/// the saved GID. On other platforms, it combines the results of `getgid()` and
/// `getegid()`.
#[inline]
pub fn getregid() -> (GidT, GidT) {
    getregid_impl()
}

/// Attempts to change the root directory of the current process to the specified
/// path.
///
/// In addition to the normal errors, this will return an error if the given path
/// contains a null byte.
pub fn chroot<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = ffi::CString::new(path.as_ref().as_os_str().as_bytes())?;

    super::error::convert_nzero(unsafe { libc::chroot(path.as_ptr()) }, ())
}

/// Change the current working directory to the specified path.
///
/// This is a thin wrapper around std::env::set_current_dir(), and only
/// present for consistency.
#[inline]
pub fn chdir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    std::env::set_current_dir(path)
}

/// Forks the current process.
///
/// If an error occurred, the Result returned represents the error encountered.
/// Otherwise, the Ok value of the Result is 0 in the child, and the child's PID
/// in the parent.
pub fn fork() -> io::Result<Int> {
    super::error::convert_neg_ret(unsafe { libc::fork() })
}

pub fn setpgid(pid: PidT, pgid: PidT) -> io::Result<()> {
    super::error::convert_nzero(unsafe { libc::setpgid(pid, pgid) }, ())
}

pub fn setsid() -> io::Result<PidT> {
    super::error::convert_neg_ret(unsafe { libc::setsid() })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Most of these are really just to check that the calls succeed without crashing.
    // Which is about all we can do for a lot of them.

    #[test]
    fn test_getpids_tid() {
        getpid();
        getppid();

        #[cfg(target_os = "linux")]
        gettid();
    }

    #[test]
    fn test_getuidgid() {
        assert_eq!((getuid(), geteuid()), getreuid());
        assert_eq!((getgid(), getegid()), getregid());
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    #[test]
    fn test_resuidgid() {
        let (ruid, euid, _suid) = getresuid();
        assert_eq!((getuid(), geteuid()), (ruid, euid));

        let (rgid, egid, _sgid) = getresgid();
        assert_eq!((getgid(), getegid()), (rgid, egid));
    }

    #[test]
    fn test_getgroups() {
        getgroups().unwrap();
        getallgroups().unwrap();
    }

    #[test]
    fn test_chdir() {
        chdir("/").unwrap();
    }
}
