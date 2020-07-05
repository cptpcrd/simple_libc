use std::convert::TryFrom;
use std::ffi;
use std::io;
use std::os::unix::prelude::*;
use std::path::Path;

use crate::externs;
use crate::internal::minus_one_either;
use crate::{GidT, Int, PidT, UidT};

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
    crate::error::convert_neg_ret(unsafe {
        libc::getgroups(groups.len() as Int, groups.as_mut_ptr())
    })
}

/// Returns a vector containing the current supplementary group IDs.
///
/// This is a higher-level wrapper that makes multiple calls to
/// `getgroups_raw()`, first to determine the number of groups and
/// then to actually fill the list. (Note that in most cases this
/// function will make two calls to `getgroups_raw()`, but it may
/// make more.)
pub fn getgroups() -> io::Result<Vec<GidT>> {
    let mut groups = Vec::new();

    // Call it with the empty vector to determine the number of groups.
    let init_ngroups = getgroups_raw(&mut groups)?;

    if init_ngroups == 0 {
        // Rare, but no point in calling getgroups_raw() again
        return Ok(groups);
    }

    // Expand the vector to fit
    groups.resize(init_ngroups as usize, 0);

    loop {
        match getgroups_raw(&mut groups) {
            Ok(ngroups) => {
                if ngroups as usize <= groups.len() {
                    // We got a value, and it's smaller than the length of the vector,
                    // so it makes sense.
                    // Shrink the vector to fit and return.

                    groups.resize(ngroups as usize, 0);
                    groups.shrink_to_fit();
                    return Ok(groups);
                }
            }
            Err(e) => {
                if crate::error::is_einval(&e) {
                    // If the value we passed was greater than NGROUPS_MAX, then presumably
                    // future calls will fail too. Let's propagate the error back up.
                    // We check for "> NGROUPS_MAX" instead of ">= NGROUPS_MAX" because the
                    // list returned by getgroups() can be up to NGROUPS_MAX + 1 if it
                    // includes the effective GID.

                    if groups.len()
                        > crate::sysconf(libc::_SC_NGROUPS_MAX)
                            .and_then(|n| usize::try_from(n).ok())
                            .unwrap_or(65536)
                    {
                        return Err(e);
                    }
                } else {
                    // Propagate everything else up.

                    return Err(e);
                }
            }
        }

        // For some reason, the vector wasn't large enough.
        // Let's resize and try again.
        // Make sure we make it at least 64 elements long so that if the initial value
        // was very small we don't build up too slowly.
        groups.resize(std::cmp::max(groups.len() * 2, 64), 0);
    }
}

/// Returns a vector containing the real group ID, the effective group
/// ID, and all group IDs returned by `getgroups()`.
///
/// No guarantees are made about the order of the vector, or the
/// uniqueness of its elements.
pub fn getallgroups() -> io::Result<Vec<GidT>> {
    let mut groups = getgroups()?;

    let (rgid, egid) = getregid();

    if !groups.contains(&rgid) {
        groups.push(rgid);
    }

    if egid != rgid && !groups.contains(&egid) {
        groups.push(egid);
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
    let init_length = crate::constrain(
        crate::sysconf(libc::_SC_LOGIN_NAME_MAX).unwrap_or(256),
        64,
        1024,
    ) as usize;

    let max_length = 1024;

    let mut buf = Vec::new();
    buf.resize(init_length, 0);

    loop {
        match crate::error::convert_nzero_ret(unsafe {
            externs::getlogin_r(buf.as_mut_ptr(), buf.len())
        }) {
            Ok(()) => {
                return Ok(ffi::OsString::from_vec(
                    buf.iter()
                        .take_while(|x| **x > 0)
                        .map(|x| *x as u8)
                        .collect(),
                ));
            }
            Err(e) => {
                if !crate::error::is_erange(&e) || buf.len() >= max_length {
                    return Err(e);
                }
            }
        }

        buf.resize(buf.len() * 2, 0);
    }
}

pub fn setuid(uid: UidT) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe { libc::setuid(uid) })
}

pub fn seteuid(uid: UidT) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe { libc::seteuid(uid) })
}

pub fn setreuid(ruid: UidT, euid: UidT) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe { externs::setreuid(ruid, euid) })
}

/// Optionally set the real and effective UIDs of the current process.
///
/// The `setreuid()` C function allows specifying `(uid_t)-1` for the new
/// real/effective UIDs to indicate that the corresponding UID should
/// remain unchanged. However, `uid_t` is usually unsigned, and because of
/// the way Rust handles casting integers this can make it difficult to
/// reliably get the value of `(uid_t)-1`.
///
/// This wrapper around `setreuid()` makes it easy to specify this special
/// value, by simply passing `None` for the corresponding UID.
pub fn setreuid2(ruid: Option<UidT>, euid: Option<UidT>) -> io::Result<()> {
    setreuid(
        ruid.unwrap_or_else(minus_one_either),
        euid.unwrap_or_else(minus_one_either),
    )
}

pub fn setgid(gid: GidT) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe { libc::setgid(gid) })
}

pub fn setegid(gid: GidT) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe { libc::setegid(gid) })
}

pub fn setregid(rgid: GidT, egid: GidT) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe { externs::setregid(rgid, egid) })
}

/// Optionally set the real and effective GIDs of the current process.
///
/// See the documentation of [`setreuid2`] for an explanation of why this
/// is useful.
///
/// [`setreuid2`]: ./fn.setreuid2.html
pub fn setregid2(rgid: Option<GidT>, egid: Option<GidT>) -> io::Result<()> {
    setregid(
        rgid.unwrap_or_else(minus_one_either),
        egid.unwrap_or_else(minus_one_either),
    )
}

#[cfg(target_os = "linux")]
type SetGroupsSize = crate::SizeT;

#[cfg(any(
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "macos",
))]
type SetGroupsSize = Int;

pub fn setgroups(groups: &[GidT]) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe {
        libc::setgroups(groups.len() as SetGroupsSize, groups.as_ptr())
    })
}

pub fn build_grouplist(gid: GidT, groups: &[GidT]) -> Vec<GidT> {
    if groups.is_empty() {
        vec![gid]
    } else if groups[0] == gid {
        groups.into()
    } else {
        let mut res = Vec::with_capacity(groups.len() + 1);

        res.push(gid);
        res.extend(groups.iter().filter(|g| **g != gid).copied());
        res.shrink_to_fit();

        res
    }
}

pub fn build_grouplist_inplace(gid: GidT, groups: &mut Vec<GidT>) {
    if groups.is_empty() {
        groups.push(gid);
    } else if let Some(index) = groups.iter().position(|g| *g == gid) {
        groups.swap(0, index);
    } else {
        groups.push(gid);
        let index = groups.len() - 1;
        groups.swap(0, index);
    }
}

crate::attr_group! {
    #![cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "dragonfly"))]

    pub fn getresuid() -> (UidT, UidT, UidT) {
        let mut ruid = 0;
        let mut euid = 0;
        let mut suid = 0;

        unsafe { externs::getresuid(&mut ruid, &mut euid, &mut suid); }
        (ruid, euid, suid)
    }

    pub fn getresgid() -> (GidT, GidT, GidT) {
        let mut rgid = 0;
        let mut egid = 0;
        let mut sgid = 0;

        unsafe { externs::getresgid(&mut rgid, &mut egid, &mut sgid); }
        (rgid, egid, sgid)
    }

    pub fn setresuid(ruid: UidT, euid: UidT, suid: UidT) -> io::Result<()> {
        crate::error::convert_nzero_ret(unsafe {
            externs::setresuid(ruid, euid, suid)
        })
    }

    /// Optionally set the real, effective, and saved UIDs of the current process.
    ///
    /// See the documentation of [`setreuid2`] for an explanation of why this
    /// is useful.
    ///
    /// [`setreuid2`]: ./fn.setreuid2.html
    pub fn setresuid2(ruid: Option<UidT>, euid: Option<UidT>, suid: Option<UidT>) -> io::Result<()> {
        setresuid(
            ruid.unwrap_or_else(minus_one_either),
            euid.unwrap_or_else(minus_one_either),
            suid.unwrap_or_else(minus_one_either),
        )
    }

    pub fn setresgid(rgid: GidT, egid: GidT, sgid: GidT) -> io::Result<()> {
        crate::error::convert_nzero_ret(unsafe {
            externs::setresgid(rgid, egid, sgid)
        })
    }

    /// Optionally set the real, effective, and saved GIDs of the current process.
    ///
    /// See the documentation of [`setreuid2`] for an explanation of why this
    /// is useful.
    ///
    /// [`setreuid2`]: ./fn.setreuid2.html
    pub fn setresgid2(rgid: Option<GidT>, egid: Option<GidT>, sgid: Option<GidT>) -> io::Result<()> {
        setresgid(
            rgid.unwrap_or_else(minus_one_either),
            egid.unwrap_or_else(minus_one_either),
            sgid.unwrap_or_else(minus_one_either),
        )
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

crate::attr_group! {
    #![cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "dragonfly")))]

    fn getreuid_impl() -> (UidT, UidT) {
        (getuid(), geteuid())
    }

    fn getregid_impl() -> (GidT, GidT) {
        (getgid(), getegid())
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

    crate::error::convert_nzero_ret(unsafe { libc::chroot(path.as_ptr()) })
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
pub fn fork() -> io::Result<PidT> {
    crate::error::convert_neg_ret(unsafe { libc::fork() })
}

pub fn setpgid(pid: PidT, pgid: PidT) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe { libc::setpgid(pid, pgid) })
}

pub fn setsid() -> io::Result<PidT> {
    crate::error::convert_neg_ret(unsafe { libc::setsid() })
}

pub fn getset_umask(new_mask: u32) -> u32 {
    unsafe { libc::umask(new_mask as libc::mode_t) as u32 }
}

/// Attempt to get the umask for the process with the given PID (0 indicates
/// the current process) without changing it. This may not succeed.
///
/// # Errors
///
/// - If `pid` does not name a valid process, ESRCH will be returned.
/// - If this functionality is not available on the current platform,
///   ENOTSUP will be returned.
/// - Other errors, such as EACCES, may be returned depending on the
///   platform.
///
/// Note that on some platforms ENOTSUP may be returned for some values but not
/// others. For example, it may be possible to determine the current process's
/// umask but not other processes' umasks; in this case, ENOTSUP will be
/// returned if `pid` is not either 0 or the current process's PID.
///
/// # Platform-specific information
///
/// - On Linux, this looks at the "Umask" field of `/proc/<pid>/status`.
/// - On FreeBSD, this calls `sysctl()`.
#[allow(unused_variables)]
#[allow(clippy::needless_return)]
pub fn try_get_umask(pid: PidT) -> io::Result<u32> {
    #[cfg(target_os = "linux")]
    {
        use std::io::BufRead;

        let stat_path = Path::new("/proc/")
            .join(if pid == 0 {
                "self".to_string()
            } else {
                pid.to_string()
            })
            .join("status");

        match std::fs::File::open(stat_path) {
            Ok(f) => {
                let mut reader = io::BufReader::new(f);
                let mut line = String::new();

                while reader.read_line(&mut line)? > 0 {
                    if line.starts_with("Umask:") {
                        if let Ok(val) = u32::from_str_radix(line[6..].trim(), 8) {
                            return Ok(val);
                        }
                    }

                    line.clear();
                }
            }
            Err(e) if crate::error::is_raw(&e, libc::ENOENT) => {
                return Err(io::Error::from_raw_os_error(libc::ESRCH))
            }
            Err(e) => return Err(e),
        }

        return Err(io::Error::from_raw_os_error(libc::ENOTSUP));
    }

    #[cfg(target_os = "freebsd")]
    {
        let mib = [
            libc::CTL_KERN,
            libc::KERN_PROC,
            libc::KERN_PROC_UMASK,
            pid as Int,
        ];

        let mut umask: crate::Ushort = 0;

        let umask_size =
            unsafe { crate::sysctl_raw(&mib, Some(std::slice::from_mut(&mut umask)), None) }?;

        if umask_size != std::mem::size_of::<crate::Ushort>() {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }

        return Ok(umask as u32);
    }

    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    return Err(io::Error::from_raw_os_error(libc::ENOTSUP));
}

/// Check if the current environment in which the process is running demands
/// "secure execution".
///
/// *WARNING: The semantics of this function vary across platforms. On some platforms,
/// if the process changes its real/effective/saved UID/GID, this function may start
/// reporting `true`. As a result, it is strongly recommended to call this function
/// once, as soon as the process is started, and then use that result to make decisions
/// later.*
///
/// On Linux, this checks `getauxval(AT_SECURE)`, which the kernel usually sets to mean
/// that the program is set-UID, is set-GID, or has file capabilities set. On the BSDs
/// and macOS, this checks `issetugid()`.
///
/// If anything goes wrong (though it shouldn't; these functions are designed not to
/// fail!), this function checks the current real/effective UID and GID, and returns
/// true if `ruid != euid || rgid != egid`.
pub fn requires_secure_execution() -> bool {
    #[cfg(target_os = "linux")]
    {
        crate::error::set_errno_success();
        let res = unsafe { externs::getauxval(crate::constants::AT_SECURE) };

        if res == 0 {
            // On error, getauxval() returns 0 and sets errno to ENOENT.
            // This *should* never happen, but let's be sure that wasn't
            // what happened.
            if io::Error::last_os_error().raw_os_error() == Some(0) {
                // Success
                return false;
            }
        } else {
            // res != 0
            return true;
        }
    }

    #[cfg(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "macos",
    ))]
    match unsafe { externs::issetugid() } {
        0 => return false,
        1 => return true,
        _ => (),
    }

    let (ruid, euid) = getreuid();
    if ruid != euid {
        return true;
    }

    let (rgid, egid) = getregid();
    rgid != egid
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
        // Get the group list
        let mut groups = getgroups().unwrap();

        // Make sure the length matches
        assert_eq!(groups.len(), getgroups_raw(&mut []).unwrap() as usize);

        // Now get the list the other way -- with a vector of size NGROUPS_MAX.
        let mut groups2 = Vec::new();
        groups2.resize(
            crate::sysconf(libc::_SC_NGROUPS_MAX).unwrap() as usize + 1,
            0,
        );
        let ngroups2 = getgroups_raw(&mut groups2).unwrap();
        groups2.resize(ngroups2 as usize, 0);

        // Sort both, remove duplicates, and make sure they match.
        groups.sort();
        groups.dedup();
        groups2.sort();
        groups2.dedup();
        assert_eq!(groups, groups2);

        let allgroups = getallgroups().unwrap();

        let (rgid, egid) = getregid();

        for gid in allgroups.iter() {
            assert!(*gid == rgid || *gid == egid || groups.contains(&gid));
        }

        for gid in groups.iter() {
            assert!(allgroups.contains(&gid));
        }
    }

    #[test]
    fn test_chdir() {
        chdir("/").unwrap();
    }

    #[test]
    fn test_build_grouplist() {
        assert_eq!(build_grouplist(0, &[]), vec![0]);
        assert_eq!(build_grouplist(0, &[0]), vec![0]);
        assert_eq!(build_grouplist(0, &[0, 0]), vec![0, 0]);

        assert_eq!(build_grouplist(0, &[0, 1, 2]), vec![0, 1, 2]);
        assert_eq!(build_grouplist(0, &[0, 1, 2, 0]), vec![0, 1, 2, 0]);
        assert_eq!(build_grouplist(0, &[1, 2, 0]), vec![0, 1, 2]);
        assert_eq!(build_grouplist(0, &[1, 2, 0, 0]), vec![0, 1, 2]);
    }

    #[test]
    fn test_build_grouplist_inplace() {
        let mut groups;

        groups = vec![];
        build_grouplist_inplace(0, &mut groups);
        assert_eq!(groups, vec![0]);

        groups = vec![0];
        build_grouplist_inplace(0, &mut groups);
        assert_eq!(groups, vec![0]);

        groups = vec![0, 0];
        build_grouplist_inplace(0, &mut groups);
        assert_eq!(groups, vec![0, 0]);

        groups = vec![0, 1, 2];
        build_grouplist_inplace(0, &mut groups);
        assert_eq!(groups, vec![0, 1, 2]);

        groups = vec![0, 1, 2, 0];
        build_grouplist_inplace(0, &mut groups);
        assert_eq!(groups, vec![0, 1, 2, 0]);

        groups = vec![1, 2];
        build_grouplist_inplace(0, &mut groups);
        assert_eq!(groups, vec![0, 2, 1]);

        groups = vec![1, 2, 0];
        build_grouplist_inplace(0, &mut groups);
        assert_eq!(groups, vec![0, 2, 1]);

        groups = vec![1, 2, 0, 0];
        build_grouplist_inplace(0, &mut groups);
        assert_eq!(groups, vec![0, 2, 1, 0]);
    }

    #[test]
    fn test_umask() {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            let umask = try_get_umask(0).unwrap();
            assert_eq!(umask, getset_umask(umask));
            assert_eq!(umask, try_get_umask(getpid()).unwrap());

            assert_eq!(
                try_get_umask(-1).unwrap_err().raw_os_error(),
                Some(libc::ESRCH)
            );
        }

        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        {
            assert_eq!(
                try_get_umask(0).unwrap_err().raw_os_error(),
                Some(libc::ENOTSUP)
            );

            assert_eq!(
                try_get_umask(-1).unwrap_err().raw_os_error(),
                Some(libc::ENOTSUP)
            );
        }
    }

    #[test]
    fn test_requires_secure_execution() {
        assert_eq!(requires_secure_execution(), false);
    }
}
