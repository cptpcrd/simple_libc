use std::ffi::OsStr;
use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::prelude::*;

use crate::SocklenT;

fn build_abstract_addr(name: &OsStr) -> io::Result<(libc::sockaddr_un, SocklenT)> {
    // Allow a leading NULL, but just ignore it since we add our own NULL anyway.
    let mut name = name.as_bytes();
    if !name.is_empty() && name[0] == 0 {
        name = &name[1..];
    }

    let mut addr = libc::sockaddr_un {
        sun_family: libc::AF_UNIX as libc::sa_family_t,
        sun_path: unsafe { std::mem::zeroed() },
    };

    // + 2 -- 1 for the leading null byte and 1 for the trailing null byte
    if name.len() + 2 > addr.sun_path.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Abstract socket name is too long",
        ));
    }

    for (ch, addr_dest) in name.iter().zip(addr.sun_path.iter_mut().skip(1)) {
        let ch = *ch;

        if ch == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Abstract socket name cannot contain null bytes",
            ));
        }

        *addr_dest = ch as libc::c_char;
    }

    let addrlen = (std::mem::size_of::<libc::sa_family_t>() + name.len() + 1) as SocklenT;

    Ok((addr, addrlen))
}

fn unix_stream_abstract_bind_impl(name: &OsStr) -> io::Result<UnixListener> {
    let fd = crate::error::convert_neg_ret(unsafe {
        libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0)
    })?;

    let (addr, addrlen) = build_abstract_addr(name)?;

    crate::error::convert_nzero_ret(unsafe {
        libc::bind(
            fd,
            &addr as *const libc::sockaddr_un as *const libc::sockaddr,
            addrlen,
        )
    })?;

    crate::error::convert_nzero_ret(unsafe { libc::listen(fd, 128) })?;

    Ok(unsafe { UnixListener::from_raw_fd(fd) })
}

fn unix_stream_abstract_connect_impl(name: &OsStr) -> io::Result<UnixStream> {
    let fd = crate::error::convert_neg_ret(unsafe {
        libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0)
    })?;

    let (addr, addrlen) = build_abstract_addr(name)?;

    crate::error::convert_nzero_ret(unsafe {
        libc::connect(
            fd,
            &addr as *const libc::sockaddr_un as *const libc::sockaddr,
            addrlen,
        )
    })?;

    Ok(unsafe { UnixStream::from_raw_fd(fd) })
}

pub fn unix_stream_abstract_bind<N: AsRef<OsStr>>(name: N) -> io::Result<UnixListener> {
    unix_stream_abstract_bind_impl(name.as_ref())
}

pub fn unix_stream_abstract_connect<N: AsRef<OsStr>>(name: N) -> io::Result<UnixStream> {
    unix_stream_abstract_connect_impl(name.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::{
        get_unix_listener_raw_sockname, get_unix_stream_raw_peername, get_unix_stream_raw_sockname,
    };

    use std::ffi::OsString;
    use std::io::{Read, Write};

    use getrandom::getrandom;

    #[test]
    fn test_build_abstract_addr() {
        build_abstract_addr(&OsString::from_vec(vec![0, 1])).unwrap();
        build_abstract_addr(&OsString::from_vec(vec![1])).unwrap();

        build_abstract_addr(&OsString::from_vec([1].repeat(106))).unwrap();

        let err = build_abstract_addr(&OsString::from_vec([1].repeat(107))).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);

        let err = build_abstract_addr(&OsString::from_vec(vec![0, 1, 0])).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);

        let err = build_abstract_addr(&OsString::from_vec(vec![1, 0])).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);

        let err = build_abstract_addr(&OsString::from_vec(vec![0, 0])).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn test_abstract_unix_stream() {
        // Generate a name by taking "SIMPLE_LIBC" and adding some random bytes
        let mut name_vec = OsString::from("SIMPLE_LIBC").into_vec();
        let old_len = name_vec.len();
        name_vec.resize(old_len + 11, 0);

        // We omit the last element so that the `if *item == 0` case
        // below gets executed consistently and the coverage doesn't
        // vary between runs.
        getrandom(&mut name_vec[old_len..old_len + 10]).unwrap();

        // Replace any NULL bytes
        for item in name_vec.iter_mut().skip(1) {
            if *item == 0 {
                *item = 1;
            }
        }

        let name = OsString::from_vec(name_vec);

        let listener = unix_stream_abstract_bind(&name).unwrap();

        let mut remote = unix_stream_abstract_connect(&name).unwrap();
        let (mut client, _addr) = listener.accept().unwrap();

        let mut prefixed_name = OsString::from("\0");
        prefixed_name.push(name);

        assert_eq!(
            get_unix_listener_raw_sockname(&listener).unwrap(),
            prefixed_name,
        );

        assert_eq!(
            get_unix_stream_raw_sockname(&remote).unwrap(),
            OsString::new(),
        );
        assert_eq!(
            get_unix_stream_raw_peername(&remote).unwrap(),
            prefixed_name,
        );

        assert_eq!(
            get_unix_stream_raw_sockname(&client).unwrap(),
            prefixed_name,
        );
        assert_eq!(
            get_unix_stream_raw_peername(&client).unwrap(),
            OsString::new(),
        );

        let mut data = Vec::new();
        data.resize(10, 0);

        client.write_all(&[0, 1, 2, 3]).unwrap();
        assert_eq!(remote.read(&mut data).unwrap(), 4);
        assert_eq!(data[..4], [0, 1, 2, 3]);

        remote.write_all(&[0, 1, 2, 3]).unwrap();
        assert_eq!(client.read(&mut data).unwrap(), 4);
        assert_eq!(data[..4], [0, 1, 2, 3]);
    }
}
