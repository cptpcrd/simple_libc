use std::io;
use std::os::unix;
use std::os::unix::io::AsRawFd;

use crate::{GidT, Int, SocklenT, UidT};

#[cfg(target_os = "linux")]
mod abstract_unix;
#[cfg(target_os = "linux")]
pub use abstract_unix::{unix_stream_abstract_bind, unix_stream_abstract_connect};

#[cfg(any(target_os = "linux", target_os = "openbsd", target_os = "netbsd"))]
pub mod ucred;

#[cfg(any(target_os = "freebsd", target_os = "netbsd"))]
pub mod sockcred;

#[cfg(any(target_os = "macos", target_os = "openbsd", target_os = "freebsd"))]
pub mod xucred;

#[cfg(any(target_os = "linux", target_os = "openbsd"))]
#[deprecated(since = "0.5.0", note = "Moved into the 'ucred' submodule")]
pub use ucred::{get_ucred, get_ucred_raw, Ucred};

#[cfg(any(
    target_os = "macos",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
))]
pub fn getpeereid_raw(sockfd: Int) -> io::Result<(UidT, GidT)> {
    let mut uid = 0;
    let mut gid = 0;

    crate::error::convert_nzero_ret(unsafe { libc::getpeereid(sockfd, &mut uid, &mut gid) })
        .map(|()| (uid, gid))
}

#[cfg(any(
    target_os = "macos",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
))]
pub fn getpeereid(sock: &unix::net::UnixStream) -> io::Result<(UidT, GidT)> {
    getpeereid_raw(sock.as_raw_fd())
}

#[cfg(target_os = "linux")]
pub fn get_peer_ids_raw(sockfd: Int) -> io::Result<(UidT, GidT)> {
    ucred::get_ucred_raw(sockfd).map(|ucred| (ucred.uid, ucred.gid))
}

#[cfg(not(target_os = "linux"))]
pub fn get_peer_ids_raw(sockfd: Int) -> io::Result<(UidT, GidT)> {
    getpeereid_raw(sockfd)
}

pub fn get_peer_ids(sock: &unix::net::UnixStream) -> io::Result<(UidT, GidT)> {
    get_peer_ids_raw(sock.as_raw_fd())
}

/// Obtain the value of the given socket option.
///
/// This function is a simple wrapper around `libc::getsockopt()` that reads
/// the value of the socket option into a generic slice. It returns the
/// number of bytes read.
///
/// # Safety
///
/// 1. This function has no way to verify that the slice into which the data
///    is being read is the correct format for representing the given socket
///    option.
/// 2. No checking is performed for partial reads that could lead to partially
///    filled out data in the slice.
///
/// If it can be verified that neither of these is the case (the data structure
/// is correct for the given option AND the amount of data read is correct for
/// the given structure), then this function should be safe to use.
pub unsafe fn getsockopt_raw<T: Sized>(
    sockfd: Int,
    level: Int,
    optname: Int,
    data: &mut [T],
) -> io::Result<SocklenT> {
    let mut len = (data.len() * std::mem::size_of::<T>()) as SocklenT;

    crate::error::convert_nzero_ret(
        libc::getsockopt(
            sockfd,
            level,
            optname,
            data.as_mut_ptr() as *mut libc::c_void,
            &mut len,
        ),
    )?;

    Ok(len)
}

/// Sets the value of the given socket option.
///
/// This function is a simple wrapper around `libc::getsockopt()` that sets
/// the value of the socket option to the contents of a generic slice.
///
/// # Safety
///
/// This function has no way to verify that the slice containing the data is
/// the correct format for representing the given socket option. If that can
/// be verified, then this function should be safe for use.
pub unsafe fn setsockopt_raw<T: Sized>(
    sockfd: Int,
    level: Int,
    optname: Int,
    data: &[T],
) -> io::Result<()> {
    crate::error::convert_nzero_ret(
        libc::setsockopt(
            sockfd,
            level,
            optname,
            data.as_ptr() as *mut libc::c_void,
            (data.len() * std::mem::size_of::<T>()) as SocklenT,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::os::unix::net::UnixStream;

    use crate::process;

    #[test]
    fn test_get_peer_ids() {
        let (a, b) = UnixStream::pair().unwrap();

        let (auid, agid) = get_peer_ids(&a).unwrap();
        assert_eq!(auid, process::getuid());
        assert_eq!(agid, process::getgid());

        let (buid, bgid) = get_peer_ids(&b).unwrap();
        assert_eq!(buid, process::getuid());
        assert_eq!(bgid, process::getgid());
    }
}
