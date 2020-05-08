use std::io;
use std::os::unix;
use std::os::unix::io::AsRawFd;
use libc;


#[derive(Debug)]
#[repr(C)]
pub struct Ucred {
    pub pid: i32,
    pub uid: u32,
    pub gid: u32,
}

pub fn get_ucred_raw(sockfd: i32) -> io::Result<Ucred> {
    let mut ucred = Ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };

    let mut len = std::mem::size_of::<Ucred>() as u32;

    super::error::convert(unsafe {
        libc::getsockopt(sockfd, libc::SOL_SOCKET, libc::SO_PEERCRED, (&mut ucred as *mut Ucred) as *mut libc::c_void, &mut len)
    }, ucred)
}

pub fn get_ucred(sock: &unix::net::UnixStream) -> io::Result<Ucred> {
    get_ucred_raw(sock.as_raw_fd())
}
