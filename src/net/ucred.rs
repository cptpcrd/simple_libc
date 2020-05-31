use std::io;
use std::os::unix;
use std::os::unix::io::AsRawFd;

use crate::{Int, SocklenT};

// Linux, NetBSD, and OpenBSD use almost exactly the same interface.
// The only differences are 1) the name of the socket option and 2) the order
// the fields in the resulting struct. We can special-case both of those.
//
// Note that on OpenBSD this is called 'sockpeercred', and on NetBSD
// it is called 'unpcbid'. But it's still pretty much the same
// interface.

#[cfg(target_os = "linux")]
pub type Ucred = libc::ucred;

#[cfg(target_os = "openbsd")]
pub type Ucred = libc::sockpeercred;

#[cfg(target_os = "netbsd")]
pub type Ucred = crate::types::unpcbid;

#[cfg(target_os = "netbsd")]
const SO_PEERCRED: Int = crate::constants::LOCAL_PEEREID;
#[cfg(not(target_os = "netbsd"))]
const SO_PEERCRED: Int = libc::SO_PEERCRED;

/// Reads credentials for a `SOCK_STREAM` socket.
///
/// On Linux, this can also be used with `SOCK_DGRAM` sockets created using
/// `socketpair()`.
pub fn get_ucred_raw(sockfd: Int) -> io::Result<Ucred> {
    let mut ucred = Ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };

    let len = unsafe {
        super::getsockopt_raw(
            sockfd,
            libc::SOL_SOCKET,
            SO_PEERCRED,
            std::slice::from_mut(&mut ucred),
        )
    }?;

    if len == std::mem::size_of::<Ucred>() as SocklenT {
        Ok(ucred)
    } else {
        Err(io::Error::from_raw_os_error(libc::EINVAL))
    }
}

/// Attempts to read credentials from the given Unix stream socket.
pub fn get_ucred(sock: &unix::net::UnixStream) -> io::Result<Ucred> {
    get_ucred_raw(sock.as_raw_fd())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::os::unix::net::UnixStream;

    use crate::process;

    #[test]
    fn test_get_ucred() {
        let (a, b) = UnixStream::pair().unwrap();

        let acred = get_ucred(&a).unwrap();
        assert_eq!(acred.uid, process::getuid());
        assert_eq!(acred.gid, process::getgid());
        assert_eq!(acred.pid, process::getpid());

        let bcred = get_ucred(&b).unwrap();
        assert_eq!(bcred.uid, process::getuid());
        assert_eq!(bcred.gid, process::getgid());
        assert_eq!(bcred.pid, process::getpid());
    }
}
