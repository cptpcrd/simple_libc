use std::io;
use std::os::unix;
use std::os::unix::io::AsRawFd;

use crate::{GidT, Int, SocklenT, UidT};

// Linux, NetBSD, and OpenBSD use almost exactly the same interface.
// The only difference is the order of the fields in the
// credentials struct, and we can special-case that.
//
// Note that on OpenBSD this is called 'sockpeercred', and on NetBSD
// it is called 'unpcbid'. But it's still pretty much the same
// interface.

use crate::PidT;

/// Stores the received credentials.
#[derive(Debug, Copy, Clone)]
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
const SO_PEERCRED: Int = crate::constants::LOCAL_PEEREID;
#[cfg(not(target_os = "netbsd"))]
const SO_PEERCRED: Int = libc::SO_PEERCRED;

/// Reads credentials for a `SOCK_STREAM` socket.
///
/// On Linux, this can also be used with `SOCK_DGRAM` sockets created using
/// `socketpair()`.
pub fn get_ucred_raw(sockfd: Int) -> io::Result<Ucred> {
    let mut ucred_arr = [Ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    }];

    unsafe { super::getsockopt_raw(sockfd, libc::SOL_SOCKET, SO_PEERCRED, &mut ucred_arr) }
        .and_then(|len| {
            if len == std::mem::size_of::<Ucred>() as SocklenT {
                Ok(ucred_arr[0])
            } else {
                Err(io::Error::from_raw_os_error(libc::EINVAL))
            }
        })
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
