use std::ffi::OsString;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::FromRawFd;
use std::os::unix::net::{UnixListener, UnixStream};

fn build_abstract_addr(name: &OsString) -> io::Result<(libc::sockaddr_un, libc::socklen_t)> {
    let mut addr = libc::sockaddr_un {
        sun_family: libc::AF_UNIX as libc::sa_family_t,
        sun_path: unsafe { std::mem::zeroed() },
    };

    if name.len() + 2 > addr.sun_path.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Abstract socket name is too long",
        ));
    }

    let name_vec = name.clone().into_vec();

    let mut i = 0;
    while i < name_vec.len() {
        addr.sun_path[i + 1] = name_vec[i] as libc::c_char;
        i += 1;
    }

    let addrlen =
        (std::mem::size_of::<libc::sa_family_t>() + name_vec.len() + 1) as libc::socklen_t;

    Ok((addr, addrlen))
}

pub fn unix_stream_abstract_bind(name: &OsString) -> io::Result<UnixListener> {
    let fd = crate::error::convert_neg_ret(unsafe {
        libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0)
    })?;

    let (addr, addrlen) = build_abstract_addr(name)?;

    crate::error::convert_nzero(
        unsafe {
            libc::bind(
                fd,
                &addr as *const libc::sockaddr_un as *const libc::sockaddr,
                addrlen,
            )
        },
        (),
    )?;

    crate::error::convert_nzero(unsafe { libc::listen(fd, 128) }, ())?;

    Ok(unsafe { UnixListener::from_raw_fd(fd) })
}

pub fn unix_stream_abstract_connect(name: &OsString) -> io::Result<UnixStream> {
    let fd = crate::error::convert_neg_ret(unsafe {
        libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0)
    })?;

    let (addr, addrlen) = build_abstract_addr(name)?;

    crate::error::convert_nzero(
        unsafe {
            libc::connect(
                fd,
                &addr as *const libc::sockaddr_un as *const libc::sockaddr,
                addrlen,
            )
        },
        (),
    )?;

    Ok(unsafe { UnixStream::from_raw_fd(fd) })
}
