use std::io;

use bitflags::bitflags;

use crate::{Int, PidT};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum ProcStatus {
    Exited(Int),
    Signaled(Int),
    Stopped(Int),
    Continued,
}

impl ProcStatus {
    fn from_raw_status(status: Int) -> Self {
        unsafe {
            if libc::WIFSIGNALED(status) {
                Self::Signaled(libc::WTERMSIG(status))
            } else if libc::WIFSTOPPED(status) {
                Self::Stopped(libc::WSTOPSIG(status))
            } else if libc::WIFCONTINUED(status) {
                Self::Continued
            } else {
                // Assume normal exit
                Self::Exited(libc::WEXITSTATUS(status))
            }
        }
    }
}

pub fn wait() -> io::Result<(PidT, ProcStatus)> {
    let mut status: Int = 0;

    crate::error::convert_neg_ret(unsafe { libc::wait(&mut status) })
        .map(|pid| (pid, ProcStatus::from_raw_status(status)))
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum WaitpidSpec {
    Pid(PidT),
    Pgid(PidT),
    Any,
    CurrentPgid,
}

bitflags! {
    pub struct WaitpidOptions: Int {
        const CONTINUED = libc::WCONTINUED;
        const NOHANG = libc::WNOHANG;
        const UNTRACED = libc::WUNTRACED;
    }
}

pub fn waitpid(
    spec: WaitpidSpec,
    options: WaitpidOptions,
) -> io::Result<Option<(PidT, ProcStatus)>> {
    let wpid = match spec {
        WaitpidSpec::Pid(pid) => pid,
        WaitpidSpec::Pgid(pgid) => -pgid,
        WaitpidSpec::Any => -1,
        WaitpidSpec::CurrentPgid => 0,
    };

    let mut status: Int = 0;

    crate::error::convert_neg_ret(unsafe { libc::waitpid(wpid, &mut status, options.bits) }).map(
        |pid| match pid {
            0 => None,
            _ => Some((pid, ProcStatus::from_raw_status(status))),
        },
    )
}
