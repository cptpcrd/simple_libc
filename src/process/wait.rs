use std::io;

use libc;

use bitflags::bitflags;


pub enum ProcStatus {
    Exited(i32),
    Signaled(i32),
    Stopped(i32),
    Continued,
}

impl ProcStatus {
    fn from_raw_status(status: i32) -> Self {
        unsafe {
            if libc::WIFSIGNALED(status) {
                Self::Signaled(libc::WTERMSIG(status))
            }
            else if libc::WIFSTOPPED(status) {
                Self::Stopped(libc::WSTOPSIG(status))
            }
            else if libc::WIFCONTINUED(status) {
                Self::Continued
            }
            else {
                // Assume normal exit
                Self::Exited(libc::WEXITSTATUS(status))
            }
        }
    }
}


pub fn wait() -> io::Result<(i32, ProcStatus)> {
    let mut status: i32 = 0;

    super::super::error::convert_neg_ret(unsafe {
        libc::wait(&mut status)
    }).map(|pid| (pid, ProcStatus::from_raw_status(status)))
}


pub enum WaitpidSpec {
    Pid(i32),
    Pgid(i32),
    Any,
    CurrentPgid,
}

bitflags! {
    pub struct WaitpidOptions: i32 {
        const CONTINUED = libc::WCONTINUED;
        const NOHANG = libc::WNOHANG;
        const UNTRACED = libc::WUNTRACED;
    }
}

pub fn waitpid(spec: WaitpidSpec, options: WaitpidOptions) -> io::Result<Option<(i32, ProcStatus)>> {
    let wpid = match spec {
        WaitpidSpec::Pid(pid) => pid,
        WaitpidSpec::Pgid(pgid) => -pgid,
        WaitpidSpec::Any => -1,
        WaitpidSpec::CurrentPgid => 0,
    };

    let mut status: i32 = 0;

    super::super::error::convert_neg_ret(unsafe {
        libc::waitpid(wpid, &mut status, options.bits)
    }).map(|pid| match pid {
        0 => None,
        _ => Some((pid, ProcStatus::from_raw_status(status))),
    })
}
