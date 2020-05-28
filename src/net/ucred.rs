use std::io;
use std::os::unix;
use std::os::unix::io::AsRawFd;

use super::super::{GidT, Int, UidT};

// Linux, NetBSD, and OpenBSD use almost exactly the same interface.
// The only difference is the order of the fields in the
// credentials struct, and we can special-case that.
//
// Note that on OpenBSD this is called 'sockpeercred', and on NetBSD
// it is called 'unpcbid'. But it's still pretty much the same
// interface.

use crate::PidT;

/// Stores the received credentials.
#[derive(Debug)]
#[repr(C)]
pub struct Ucred {
    #[cfg(any(target_os = "linux", target_os = "netbsd"))]
    pub pid: PidT,

    pub uid: UidT,
    pub gid: GidT,

    #[cfg(target_os = "openbsd")]
    pub pid: PidT,
}

#[cfg(target_os = "netbsd")]
pub const SO_PEERCRED: Int = super::super::constants::LOCAL_PEEREID;
#[cfg(not(target_os = "netbsd"))]
pub const SO_PEERCRED: Int = libc::SO_PEERCRED;

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

    let mut len = std::mem::size_of::<Ucred>() as libc::socklen_t;

    super::super::error::convert(
        unsafe {
            libc::getsockopt(
                sockfd,
                libc::SOL_SOCKET,
                SO_PEERCRED,
                (&mut ucred as *mut Ucred) as *mut libc::c_void,
                &mut len,
            )
        },
        ucred,
    )
}

/// Attempts to read credentials from the given Unix stream socket.
pub fn get_ucred(sock: &unix::net::UnixStream) -> io::Result<Ucred> {
    get_ucred_raw(sock.as_raw_fd())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::os::unix::net::UnixStream;

    use super::super::super::process;

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
