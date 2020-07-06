use std::io;
use std::os::unix::net::UnixStream;
use std::os::unix::prelude::*;

use crate::{GidT, Int, SocklenT, UidT};

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Xucred {
    /// Note: Only FreeBSD 13+ passes the PID. On FreeBSD 12 and earlier,
    /// this will always be 0.
    #[cfg(target_os = "freebsd")]
    pub pid: crate::PidT,
    pub uid: UidT,
    pub gid: GidT,
    pub groups: Vec<GidT>,
}

// The libc crate doesn't define the xucred struct for DragonflyBSD.
//
// FreeBSD passes the PID in a private union field. However, libc makes that
// field private, so we use a custom struct to get access to it.
#[cfg(any(target_os = "dragonfly", target_os = "freebsd"))]
type RawXucred = crate::types::xucred;
#[cfg(not(any(target_os = "dragonfly", target_os = "freebsd")))]
type RawXucred = libc::xucred;

#[cfg(target_os = "freebsd")]
const XU_NGROUPS: usize = libc::XU_NGROUPS as usize;
#[cfg(not(target_os = "freebsd"))]
const XU_NGROUPS: usize = crate::constants::XU_NGROUPS as usize;

pub fn get_xucred_raw(sockfd: Int) -> io::Result<Xucred> {
    let mut raw_xucred: RawXucred = unsafe { std::mem::zeroed() };

    raw_xucred.cr_version = libc::XUCRED_VERSION;

    let len = unsafe {
        super::getsockopt_raw(
            sockfd,
            0,
            libc::LOCAL_PEERCRED,
            std::slice::from_mut(&mut raw_xucred),
        )
    }?;

    // We want to make sure that 1) the length matches, 2) the version number
    // matches, 3) we have at least one GID to pull out as the primary GID, and
    // 4) cr_ngroups isn't greater than XU_NGROUPS.
    //
    // Most of this is just paranoid sanity checks that should never actually
    // happen.
    if len != std::mem::size_of::<RawXucred>() as SocklenT
        || raw_xucred.cr_version != libc::XUCRED_VERSION
        || raw_xucred.cr_ngroups < 1
        || raw_xucred.cr_ngroups as usize > XU_NGROUPS
    {
        return Err(io::Error::from_raw_os_error(libc::EINVAL));
    }

    Ok(Xucred {
        #[cfg(target_os = "freebsd")]
        pid: unsafe { raw_xucred.cr_pid() },
        uid: raw_xucred.cr_uid,
        // FreeBSD, DragonflyBSD, and macOS just use the first
        // ID in `cr_groups` to store the primary GID.
        gid: raw_xucred.cr_groups[0],
        groups: raw_xucred.cr_groups[..raw_xucred.cr_ngroups as usize].into(),
    })
}

#[inline]
pub fn get_xucred(sock: &UnixStream) -> io::Result<Xucred> {
    get_xucred_raw(sock.as_raw_fd())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::process;

    #[cfg(target_os = "freebsd")]
    fn get_expected_pid() -> crate::PidT {
        if super::super::has_cr_pid().unwrap() {
            process::getpid()
        } else {
            0
        }
    }

    #[test]
    fn test_get_xucred() {
        let (a, b) = UnixStream::pair().unwrap();

        let mut groups = process::getgroups().unwrap();
        groups.sort();

        let mut acred = get_xucred(&a).unwrap();
        assert_eq!(acred.uid, process::geteuid());
        assert_eq!(acred.gid, process::getegid());

        acred.groups.sort();
        assert_eq!(acred.groups, groups);

        #[cfg(target_os = "freebsd")]
        assert_eq!(acred.pid, get_expected_pid());

        let mut bcred = get_xucred(&b).unwrap();
        assert_eq!(bcred.uid, process::geteuid());
        assert_eq!(bcred.gid, process::getegid());

        bcred.groups.sort();
        assert_eq!(bcred.groups, groups);

        #[cfg(target_os = "freebsd")]
        assert_eq!(bcred.pid, get_expected_pid());
    }
}
