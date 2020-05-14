use std::io;
use std::os::unix;
use std::os::unix::io::AsRawFd;

use super::super::{GidT, Int, UidT};

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "openbsd"))] {
        // Linux and OpenBSD use almost exactly the same interface.
        // The only difference is the order of the fields in the
        // credentials struct, and we can special-case that.

        /// Stores the received credentials. WARNING: Several aspects of this struct
        /// are system-dependent!
        #[derive(Debug)]
        #[repr(C)]
        pub struct Ucred {
            #[cfg(target_os = "linux")]
            pub pid: super::super::PidT,

            pub uid: UidT,
            pub gid: GidT,

            #[cfg(not(target_os = "linux"))]
            pub pid: super::super::PidT,
        }

        fn get_ucred_raw_impl(sockfd: Int) -> io::Result<Ucred> {
            let mut ucred = Ucred {
                pid: 0,
                uid: 0,
                gid: 0,
            };

            let mut len = std::mem::size_of::<Ucred>() as u32;

            super::super::error::convert(unsafe {
                libc::getsockopt(
                    sockfd,
                    libc::SOL_SOCKET,
                    libc::SO_PEERCRED,
                    (&mut ucred as *mut Ucred) as *mut libc::c_void,
                    &mut len,
                )
            }, ucred)
        }
    }
}

/// Attempts to read credentials from the given Unix stream socket.
///
/// Only supported on specific platforms.
#[cfg(any(target_os = "linux", target_os = "openbsd"))]
pub fn get_ucred(sock: &unix::net::UnixStream) -> io::Result<Ucred> {
    get_ucred_raw(sock.as_raw_fd())
}

/// Reads credentials for a `SOCK_STREAM` socket. Behavior when given a
/// `SOCK_DGRAM` socket is unspecified and system-dependent.
///
/// Only supported on specific platforms.
#[cfg(any(target_os = "linux", target_os = "openbsd"))]
#[inline(always)]
pub fn get_ucred_raw(sockfd: Int) -> io::Result<Ucred> {
    get_ucred_raw_impl(sockfd)
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
        #[cfg(any(target_os = "linux", target_os = "openbsd"))]
        assert_eq!(acred.pid, process::getpid());

        let bcred = get_ucred(&b).unwrap();
        assert_eq!(bcred.uid, process::getuid());
        assert_eq!(bcred.gid, process::getgid());
        #[cfg(any(target_os = "linux", target_os = "openbsd"))]
        assert_eq!(bcred.pid, process::getpid());
    }
}
