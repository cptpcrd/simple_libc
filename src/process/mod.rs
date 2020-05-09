use std::ffi;
use std::io;
use std::path::Path;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::ffi::OsStringExt;
use libc;

pub mod sigmask;
pub mod sigaction;
pub mod priority;
pub mod resource;
pub mod exec;

#[cfg(target_os = "linux")]
pub mod signalfd;
#[cfg(target_os = "linux")]
pub mod prctl;


#[inline]
pub fn getpid() -> i32 {
    unsafe { libc::getpid() }
}


/// Returns the current real user ID.
#[inline]
pub fn getuid() -> u32 {
    unsafe { libc::getuid() }
}

/// Returns the current effective user ID.
#[inline]
pub fn geteuid() -> u32 {
    unsafe { libc::geteuid() }
}


/// Returns the current real group ID.
#[inline]
pub fn getgid() -> u32 {
    unsafe { libc::getgid() }
}

/// Returns the current effective group ID.
#[inline]
pub fn getegid() -> u32 {
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
pub fn getgroups_raw(groups: &mut Vec<u32>) -> io::Result<i32> {
    super::error::convert_neg_ret(unsafe {
        libc::getgroups(groups.len() as i32, groups.as_mut_ptr())
    })
}

/// Returns a vector containing the current supplementary
/// group IDs.
///
/// This is a higher-level wrapper that calls `getgroups_raw()` twice,
/// first to determine the number of groups and then again to actually
/// fill the list.
pub fn getgroups() -> io::Result<Vec<u32>> {
    let mut groups: Vec<u32> = Vec::new();

    let ngroups = getgroups_raw(&mut groups)?;

    groups.reserve(ngroups as usize);
    unsafe { groups.set_len(ngroups as usize) };

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
pub fn getallgroups() -> io::Result<Vec<u32>> {
    let mut groups = getgroups()?;

    let (rgid, egid) = getregid();

    groups.retain(|&x| x != rgid && x != egid);

    if rgid == egid {
        groups.insert(0, egid);
    }
    else {
        groups.splice(0..0, [rgid, egid].iter().cloned());
    }

    Ok(groups)
}


extern "C" {
    fn getlogin_r(buf: *mut libc::c_char, bufsize: libc::size_t) -> i32;
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
    let init_length = super::constrain(super::sysconf(libc::_SC_LOGIN_NAME_MAX).unwrap_or(256), 64, 1024);

    super::error::while_erange(|i| {
        let length = (init_length * (i as i64 + 1)) as usize;
        let mut buf: Vec<i8> = Vec::with_capacity(length);

        super::error::convert_nzero(unsafe {
            buf.set_len(length);
            getlogin_r(buf.as_mut_ptr(), length)
        }, buf).map(|buf| {
            ffi::OsString::from_vec(buf.iter().take_while(|x| **x > 0).map(|x| *x as u8).collect())
        })
    }, 10)
}


pub fn setuid(uid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setuid(uid)
    }, ())
}

pub fn seteuid(uid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::seteuid(uid)
    }, ())
}

pub fn setreuid(ruid: u32, euid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setreuid(ruid, euid)
    }, ())
}


pub fn setgid(gid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setgid(gid)
    }, ())
}

pub fn setegid(gid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setegid(gid)
    }, ())
}

pub fn setregid(rgid: u32, egid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setregid(rgid, egid)
    }, ())
}

pub fn setgroups(groups: &[u32]) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setgroups(groups.len(), groups.as_ptr())
    }, ())
}


cfg_if::cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "dragonfly"))] {
        pub fn getresuid() -> (u32, u32, u32) {
            let mut ruid: u32 = 0;
            let mut euid: u32 = 0;
            let mut suid: u32 = 0;

            unsafe { libc::getresuid(&mut ruid, &mut euid, &mut suid); }
            (ruid, euid, suid)
        }

        pub fn getresgid() -> (u32, u32, u32) {
            let mut rgid: u32 = 0;
            let mut egid: u32 = 0;
            let mut sgid: u32 = 0;

            unsafe { libc::getresgid(&mut rgid, &mut egid, &mut sgid); }
            (rgid, egid, sgid)
        }

        pub fn setresuid(ruid: u32, euid: u32, suid: u32) -> io::Result<()> {
            super::error::convert_nzero(unsafe {
                libc::setresuid(ruid, euid, suid)
            }, ())
        }

        pub fn setresgid(rgid: u32, egid: u32, sgid: u32) -> io::Result<()> {
            super::error::convert_nzero(unsafe {
                libc::setresgid(rgid, egid, sgid)
            }, ())
        }

        fn _getreuid() -> (u32, u32) {
            let (ruid, euid, _) = getresuid();
            (ruid, euid)
        }

        fn _getregid() -> (u32, u32) {
            let (rgid, egid, _) = getresgid();
            (rgid, egid)
        }
    }
    else {
        fn _getreuid() -> (u32, u32) {
            (getuid(), geteuid())
        }

        fn _getregid() -> (u32, u32) {
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
pub fn getreuid() -> (u32, u32) {
    _getreuid()
}

/// Gets the real and effective group IDs via the most efficient method possible.
///
/// On platforms with `getresgid()`, this function calls that function and discards
/// the saved GID. On other platforms, it combines the results of `getgid()` and
/// `getegid()`.
#[inline]
pub fn getregid() -> (u32, u32) {
    _getregid()
}


/// Attempts to change the root directory of the current process to the specified
/// path.
///
/// In addition to the normal errors, this will return an error if the given path
/// contains a null byte.
pub fn chroot<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = ffi::CString::new(path.as_ref().as_os_str().as_bytes())?;

    super::error::convert_nzero(unsafe {
        libc::chroot(path.as_ptr())
    }, ())
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
pub fn fork() -> io::Result<i32> {
    super::error::convert_neg_ret(unsafe { libc::fork() })
}
