use std::io;
use std::os::unix::net::UnixStream;
use std::os::unix::prelude::*;

use crate::error;
use crate::{GidT, Int, UidT};

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Sockcred {
    #[cfg(target_os = "netbsd")]
    pub pid: crate::PidT,
    pub ruid: UidT,
    pub euid: UidT,
    pub rgid: GidT,
    pub egid: GidT,
    pub groups: Vec<GidT>,
}

pub fn recv_sockcred_raw(sockfd: Int, block: bool) -> io::Result<Sockcred> {
    let flags = if block { 0 } else { libc::MSG_DONTWAIT };

    let mut cmsg_dat: Vec<u8> = Vec::new();

    let sockcred_size;

    #[cfg(target_os = "freebsd")]
    {
        sockcred_size = unsafe { libc::SOCKCREDSIZE(libc::CMGROUP_MAX) };
    }

    #[cfg(target_os = "netbsd")]
    {
        sockcred_size = std::cmp::max(crate::ioctl::get_readbuf_length(sockfd)?, unsafe {
            libc::SOCKCREDSIZE(1)
        });
    }

    cmsg_dat.resize(std::mem::size_of::<libc::cmsghdr>() + sockcred_size, 0);

    let cmsg = libc::cmsghdr {
        cmsg_len: cmsg_dat.len() as libc::socklen_t,
        cmsg_level: libc::SOL_SOCKET,
        cmsg_type: libc::SCM_CREDS,
    };

    unsafe {
        cmsg_dat.as_mut_ptr().copy_from_nonoverlapping(
            &cmsg as *const libc::cmsghdr as *const u8,
            std::mem::size_of::<libc::cmsghdr>(),
        );
    }

    let mut msg = libc::msghdr {
        msg_name: std::ptr::null_mut(),
        msg_namelen: 0,
        msg_iov: std::ptr::null_mut(),
        msg_iovlen: 0,
        msg_control: cmsg_dat.as_mut_ptr() as *mut libc::c_void,
        msg_controllen: cmsg_dat.len() as libc::socklen_t,
        msg_flags: 0,
    };

    let nbytes =
        error::convert_neg_ret(unsafe { libc::recvmsg(sockfd, &mut msg, flags) })? as usize;

    if nbytes < std::mem::size_of::<libc::sockcred>() || nbytes > cmsg_dat.len() {
        Err(io::Error::from_raw_os_error(libc::EIO))
    } else {
        #[allow(clippy::cast_ptr_alignment)]
        let raw_sockcred = unsafe {
            &*(libc::CMSG_DATA(cmsg_dat.as_ptr() as *const libc::cmsghdr) as *const libc::sockcred)
        };

        Ok(Sockcred {
            #[cfg(target_os = "netbsd")]
            pid: raw_sockcred.sc_pid,
            ruid: raw_sockcred.sc_uid,
            euid: raw_sockcred.sc_euid,
            rgid: raw_sockcred.sc_gid,
            egid: raw_sockcred.sc_egid,
            groups: read_sockcred_groups(&raw_sockcred),
        })
    }
}

fn read_sockcred_groups(cred: &libc::sockcred) -> Vec<GidT> {
    if cred.sc_ngroups == 0 {
        Vec::new()
    } else {
        unsafe {
            std::slice::from_raw_parts(&cred.sc_groups as *const GidT, cred.sc_ngroups as usize)
        }
        .into()
    }
}

#[inline]
pub fn recv_sockcred(sock: &mut UnixStream, block: bool) -> io::Result<Sockcred> {
    recv_sockcred_raw(sock.as_raw_fd(), block)
}
