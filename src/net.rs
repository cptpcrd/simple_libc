use std::io;
use std::os::unix;
use std::os::unix::io::AsRawFd;

use libc;

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "openbsd"))] {
        // Linux and OpenBSD use almost exactly the same interface.
        // The only difference is the order of the fields in the
        // credentials struct, and we can special-case that.

        #[cfg(target_os = "linux")]
        #[derive(Debug)]
        #[repr(C)]
        pub struct Ucred {
            pub pid: i32,
            pub uid: u32,
            pub gid: u32,
        }

        #[cfg(not(target_os = "linux"))]
        #[derive(Debug)]
        #[repr(C)]
        pub struct Ucred {
            pub uid: u32,
            pub gid: u32,
            pub pid: i32,
        }

        /// Reads credentials for a `SOCK_STREAM` socket. Behavior when given a
        /// `SOCK_DGRAM` socket is unspecified and system-dependent.
        pub fn get_ucred_raw(sockfd: i32) -> io::Result<Ucred> {
            let mut ucred = Ucred {
                pid: 0,
                uid: 0,
                gid: 0,
            };

            let mut len = std::mem::size_of::<Ucred>() as u32;

            super::error::convert(unsafe {
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

/// Attempts to read credentials from the given Unix socket.
///
/// Only supported on specific platforms.
#[cfg(any(target_os = "linux", target_os = "openbsd"))]
pub fn get_ucred(sock: &unix::net::UnixStream) -> io::Result<Ucred> {
    get_ucred_raw(sock.as_raw_fd())
}
