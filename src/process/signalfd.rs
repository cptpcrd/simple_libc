use std::io;
use std::os::unix::io::AsRawFd;
use libc;

use super::super::signal::Sigset;
use super::super::error;
use super::super::Int;


#[derive(Debug)]
pub struct SignalFd {
    fd: Int,
}

impl SignalFd {
    pub fn new(mask: &Sigset, flags: Int) -> io::Result<SignalFd> {
        return error::convert_ret(unsafe {
            libc::signalfd(-1, &mask.raw_set(), flags)
        }).map(|fd| SignalFd { fd })
    }

    pub fn read_one(&self) -> io::Result<Siginfo> {
        let mut siginfo: libc::signalfd_siginfo = unsafe { std::mem::zeroed() };

        return error::convert_neg(unsafe {
            libc::read(self.fd, (&mut siginfo as *mut libc::signalfd_siginfo) as *mut libc::c_void, std::mem::size_of::<libc::signalfd_siginfo>())
        }, &siginfo).map(Siginfo::from)
    }

    pub fn read(&self, siginfos: &mut [Siginfo]) -> io::Result<usize> {
        let length = siginfos.len();
        let size = std::mem::size_of::<libc::signalfd_siginfo>();

        let mut raw_siginfos: Vec<libc::signalfd_siginfo> = Vec::new();
        raw_siginfos.reserve(length);
        unsafe { raw_siginfos.set_len(length); }

        return error::convert_neg_ret(unsafe {
            libc::read(
                self.fd,
                raw_siginfos.as_mut_ptr() as *mut libc::c_void,
                size * length,
            )
        }).and_then(|n| {
            let n = (n as usize) / size;
            for (i, raw_siginfo) in raw_siginfos.iter().take(n).enumerate() {
                siginfos[i] = Siginfo::from(raw_siginfo);
            }
            Ok(n)
        })
    }
}

impl AsRawFd for SignalFd {
    fn as_raw_fd(&self) -> libc::c_int {
        return self.fd;
    }
}

impl Drop for SignalFd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Siginfo {
    pub sig: Int,
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
}

impl From<&libc::signalfd_siginfo> for Siginfo {
    fn from(siginfo: &libc::signalfd_siginfo) -> Siginfo {
        Siginfo {
            sig: siginfo.ssi_signo as Int,
            errno: siginfo.ssi_errno,
            code: siginfo.ssi_code,
            pid: siginfo.ssi_pid,
            uid: siginfo.ssi_uid,
            fd: siginfo.ssi_fd,
            tid: siginfo.ssi_tid,
            band: siginfo.ssi_band,
            overrun: siginfo.ssi_overrun,
            trapno: siginfo.ssi_trapno,
            status: siginfo.ssi_status,
            int: siginfo.ssi_int,
            ptr: siginfo.ssi_ptr,
            utime: siginfo.ssi_utime,
            stime: siginfo.ssi_stime,
            addr: siginfo.ssi_addr,
            addr_lsb: siginfo.ssi_addr_lsb,
        }
    }
}

impl From<Siginfo> for libc::signalfd_siginfo {
    fn from(siginfo: Siginfo) -> libc::signalfd_siginfo {
        let mut sinfo: libc::signalfd_siginfo = unsafe { std::mem::zeroed() };

        sinfo.ssi_signo = siginfo.sig as u32;
        sinfo.ssi_errno = siginfo.errno;
        sinfo.ssi_code = siginfo.code;
        sinfo.ssi_pid = siginfo.pid;
        sinfo.ssi_uid = siginfo.uid;
        sinfo.ssi_fd = siginfo.fd;
        sinfo.ssi_tid = siginfo.tid;
        sinfo.ssi_band = siginfo.band;
        sinfo.ssi_overrun = siginfo.overrun;
        sinfo.ssi_trapno = siginfo.trapno;
        sinfo.ssi_status = siginfo.status;
        sinfo.ssi_int = siginfo.int;
        sinfo.ssi_ptr = siginfo.ptr;
        sinfo.ssi_utime = siginfo.utime;
        sinfo.ssi_stime = siginfo.stime;
        sinfo.ssi_addr = siginfo.addr;
        sinfo.ssi_addr_lsb = siginfo.addr_lsb;

        sinfo
    }

}
