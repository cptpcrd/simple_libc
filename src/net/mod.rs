use std::io;
use std::os::unix::net::UnixStream;
use std::os::unix::prelude::*;

#[cfg(target_os = "linux")]
use std::ffi::OsString;
#[cfg(target_os = "linux")]
use std::os::unix::net::UnixListener;

use crate::{GidT, Int, SocklenT, UidT};

#[cfg(target_os = "linux")]
pub mod abstract_unix;
#[cfg(target_os = "linux")]
#[deprecated(since = "0.5.0", note = "Moved into the 'abstract_unix' submodule")]
pub use abstract_unix::{unix_stream_abstract_bind, unix_stream_abstract_connect};

#[cfg(any(target_os = "linux", target_os = "openbsd", target_os = "netbsd"))]
pub mod ucred;

#[cfg(any(target_os = "freebsd", target_os = "netbsd"))]
pub mod sockcred;

#[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "dragonfly"))]
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

    crate::error::convert_nzero_ret(unsafe { libc::getpeereid(sockfd, &mut uid, &mut gid) })?;

    Ok((uid, gid))
}

#[cfg(any(
    target_os = "macos",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
))]
pub fn getpeereid(sock: &UnixStream) -> io::Result<(UidT, GidT)> {
    getpeereid_raw(sock.as_raw_fd())
}

#[allow(clippy::needless_return)]
pub fn get_peer_ids_raw(sockfd: Int) -> io::Result<(UidT, GidT)> {
    #[cfg(target_os = "linux")]
    {
        let cred = ucred::get_ucred_raw(sockfd)?;
        return Ok((cred.uid, cred.gid));
    }

    #[cfg(not(target_os = "linux"))]
    return getpeereid_raw(sockfd);
}

pub fn get_peer_ids(sock: &UnixStream) -> io::Result<(UidT, GidT)> {
    get_peer_ids_raw(sock.as_raw_fd())
}

#[cfg(any(
    target_os = "linux",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "freebsd",
))]
pub fn get_peer_pid_ids_raw(sockfd: Int) -> io::Result<(crate::PidT, UidT, GidT)> {
    #[cfg(any(target_os = "linux", target_os = "openbsd", target_os = "netbsd"))]
    let cred = ucred::get_ucred_raw(sockfd)?;

    #[cfg(target_os = "freebsd")]
    let cred = xucred::get_xucred_raw(sockfd)?;

    Ok((cred.pid, cred.uid, cred.gid))
}

#[cfg(any(
    target_os = "linux",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "freebsd",
))]
pub fn get_peer_pid_ids(sock: &UnixStream) -> io::Result<(crate::PidT, UidT, GidT)> {
    get_peer_pid_ids_raw(sock.as_raw_fd())
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

    crate::error::convert_nzero_ret(libc::getsockopt(
        sockfd,
        level,
        optname,
        data.as_mut_ptr() as *mut libc::c_void,
        &mut len,
    ))?;

    Ok(len)
}

/// Sets the value of the given socket option.
///
/// This function is a simple wrapper around `libc::setsockopt()` that sets
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
    crate::error::convert_nzero_ret(libc::setsockopt(
        sockfd,
        level,
        optname,
        data.as_ptr() as *mut libc::c_void,
        (data.len() * std::mem::size_of::<T>()) as SocklenT,
    ))
}

#[cfg(target_os = "linux")]
fn get_unix_raw_sockname(sockfd: Int) -> io::Result<OsString> {
    let mut addr = libc::sockaddr_un {
        sun_family: libc::AF_UNIX as libc::sa_family_t,
        sun_path: unsafe { std::mem::zeroed() },
    };

    let mut addrlen = std::mem::size_of::<libc::sockaddr_un>() as SocklenT;

    crate::error::convert_nzero_ret(unsafe {
        libc::getsockname(
            sockfd,
            &mut addr as *mut libc::sockaddr_un as *mut libc::sockaddr,
            &mut addrlen,
        )
    })?;

    let len = addrlen as usize - std::mem::size_of::<libc::sa_family_t>();

    Ok(OsString::from_vec(
        addr.sun_path[..len].iter().map(|c| *c as u8).collect(),
    ))
}

#[cfg(target_os = "linux")]
fn get_unix_raw_peername(sockfd: Int) -> io::Result<OsString> {
    let mut addr = libc::sockaddr_un {
        sun_family: libc::AF_UNIX as libc::sa_family_t,
        sun_path: unsafe { std::mem::zeroed() },
    };

    let mut addrlen = std::mem::size_of::<libc::sockaddr_un>() as SocklenT;

    crate::error::convert_nzero_ret(unsafe {
        libc::getpeername(
            sockfd,
            &mut addr as *mut libc::sockaddr_un as *mut libc::sockaddr,
            &mut addrlen,
        )
    })?;

    let len = addrlen as usize - std::mem::size_of::<libc::sa_family_t>();

    Ok(OsString::from_vec(
        addr.sun_path[..len].iter().map(|c| *c as u8).collect(),
    ))
}

#[cfg(target_os = "linux")]
pub fn get_unix_stream_raw_sockname(sock: &UnixStream) -> io::Result<OsString> {
    get_unix_raw_sockname(sock.as_raw_fd())
}

#[cfg(target_os = "linux")]
pub fn get_unix_listener_raw_sockname(sock: &UnixListener) -> io::Result<OsString> {
    get_unix_raw_sockname(sock.as_raw_fd())
}

#[cfg(target_os = "linux")]
pub fn get_unix_stream_raw_peername(sock: &UnixStream) -> io::Result<OsString> {
    get_unix_raw_peername(sock.as_raw_fd())
}

#[cfg(target_os = "freebsd")]
#[allow(dead_code)]
fn has_cr_pid() -> io::Result<bool> {
    let release = crate::uname()?.release;

    // Only FreeBSD 13+ sets cr_pid to a nonzero value

    let release_major_str = release.to_str().unwrap().split('.').next().unwrap();

    if let Ok(release_major) = release_major_str.parse::<i32>() {
        Ok(release_major >= 13)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[cfg(any(
        target_os = "macos",
        target_os = "openbsd",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
    ))]
    #[test]
    fn test_getpeereid() {
        let (a, b) = UnixStream::pair().unwrap();

        let (auid, agid) = getpeereid(&a).unwrap();
        assert_eq!(auid, process::geteuid());
        assert_eq!(agid, process::getegid());

        let (buid, bgid) = getpeereid(&b).unwrap();
        assert_eq!(buid, process::geteuid());
        assert_eq!(bgid, process::getegid());
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "freebsd",
    ))]
    #[test]
    fn test_get_peer_pid_ids() {
        let (a, b) = UnixStream::pair().unwrap();

        let (apid, auid, agid) = get_peer_pid_ids(&a).unwrap();
        assert_eq!(apid, get_expected_pid());
        assert_eq!(auid, process::getuid());
        assert_eq!(agid, process::getgid());

        let (bpid, buid, bgid) = get_peer_pid_ids(&b).unwrap();
        assert_eq!(bpid, get_expected_pid());
        assert_eq!(buid, process::getuid());
        assert_eq!(bgid, process::getgid());
    }

    #[allow(clippy::needless_return)]
    fn get_expected_pid() -> crate::PidT {
        #[cfg(target_os = "freebsd")]
        {
            return if has_cr_pid().unwrap() {
                process::getpid()
            } else {
                0
            };
        }

        #[cfg(not(target_os = "freebsd"))]
        {
            return process::getpid();
        }
    }
}
