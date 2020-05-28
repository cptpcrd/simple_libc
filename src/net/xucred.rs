use std::io;
use std::os::unix;
use std::os::unix::io::AsRawFd;

use crate::error;
use crate::{GidT, Int, UidT};

#[derive(Debug)]
pub struct Xucred {
    pub uid: UidT,
    pub gid: GidT,
    pub groups: Vec<GidT>,
}

#[cfg(target_os = "dragonfly")]
type RawXucred = super::super::types::xucred;
#[cfg(not(target_os = "dragonfly"))]
type RawXucred = libc::xucred;

pub fn get_xucred_raw(sockfd: Int) -> io::Result<Xucred> {
    let mut raw_xucred: RawXucred = unsafe { std::mem::zeroed() };
    #[cfg(not(target_os = "openbsd"))]
    {
        raw_xucred.cr_version = libc::XUCRED_VERSION;
    }

    let mut len = std::mem::size_of::<RawXucred>() as u32;

    error::convert(
        unsafe {
            libc::getsockopt(
                sockfd,
                libc::SOL_SOCKET,
                libc::LOCAL_PEERCRED,
                (&mut raw_xucred as *mut RawXucred) as *mut libc::c_void,
                &mut len,
            )
        },
        raw_xucred,
    )
    .and_then(|raw_xucred| {
        #[cfg(not(target_os = "openbsd"))]
        if raw_xucred.cr_version != libc::XUCRED_VERSION {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }

        Ok(Xucred {
            uid: raw_xucred.cr_uid,
            // OpenBSD has a separate field for the GID.
            #[cfg(target_os = "openbsd")]
            gid: raw_xucred.cr_gid,
            // FreeBSD, DragonflyBSD, and macOS just use the first
            // ID from `cr_groups`.
            #[cfg(not(target_os = "openbsd"))]
            gid: raw_xucred.cr_groups[0],
            groups: raw_xucred.cr_groups[..raw_xucred.cr_ngroups as usize].into(),
        })
    })
}

#[inline]
pub fn get_xucred(sock: &unix::net::UnixStream) -> io::Result<Xucred> {
    get_xucred_raw(sock.as_raw_fd())
}

#[inline]
pub fn get_xucred_dgram(sock: &unix::net::UnixDatagram) -> io::Result<Xucred> {
    get_xucred_raw(sock.as_raw_fd())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::os::unix::net::UnixStream;

    use super::super::super::process;

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

        let mut bcred = get_xucred(&b).unwrap();
        assert_eq!(bcred.uid, process::geteuid());
        assert_eq!(bcred.gid, process::getegid());

        bcred.groups.sort();
        assert_eq!(bcred.groups, groups);
    }
}
