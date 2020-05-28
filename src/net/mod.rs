use std::io;
use std::os::unix;
use std::os::unix::io::AsRawFd;

use crate::{GidT, Int, UidT};

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

    super::error::convert_nzero(unsafe { libc::getpeereid(sockfd, &mut uid, &mut gid) }, ())
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
