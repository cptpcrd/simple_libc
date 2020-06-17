use std::io;
use std::os::unix::prelude::*;

use crate::error;
use crate::signal::Sigset;
use crate::Int;

#[derive(Debug)]
pub struct SignalFd {
    fd: Int,
}

impl SignalFd {
    pub fn new(mask: &Sigset, nonblock: bool) -> io::Result<SignalFd> {
        let mut flags = libc::SFD_CLOEXEC;
        if nonblock {
            flags |= libc::SFD_NONBLOCK;
        }

        let fd = error::convert_ret(unsafe { libc::signalfd(-1, &mask.raw_set(), flags) })?;

        Ok(SignalFd { fd })
    }

    pub fn read_one(&self) -> io::Result<Siginfo> {
        let mut siginfo = unsafe { std::mem::zeroed() };

        error::convert_neg_ret(
            unsafe {
                libc::read(
                    self.fd,
                    (&mut siginfo as *mut Siginfo) as *mut libc::c_void,
                    std::mem::size_of::<Siginfo>(),
                )
            }
        )?;

        Ok(siginfo)
    }

    pub fn read(&self, siginfos: &mut [Siginfo]) -> io::Result<usize> {
        let n = error::convert_neg_ret(unsafe {
            libc::read(
                self.fd,
                siginfos.as_mut_ptr() as *mut libc::c_void,
                siginfos.len() * std::mem::size_of::<Siginfo>(),
            )
        })? as usize;

        Ok(n / std::mem::size_of::<Siginfo>())
    }
}

impl AsRawFd for SignalFd {
    fn as_raw_fd(&self) -> libc::c_int {
        self.fd
    }
}

impl Drop for SignalFd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, Hash, PartialEq)]
#[repr(C)]
pub struct Siginfo {
    pub sig: u32,
    pub errno: i32,
    pub code: i32,
    pub pid: u32,
    pub uid: u32,
    pub fd: i32,
    pub tid: u32,
    pub band: u32,
    pub overrun: u32,
    pub trapno: u32,
    pub status: i32,
    pub int: i32,
    pub ptr: u64,
    pub utime: u64,
    pub stime: u64,
    pub addr: u64,
    pub addr_lsb: u16,
    // WARNING: This is dependent on the size of the fields above!
    _padding: [u8; 32],
    _padding2: [u8; 14],
}
