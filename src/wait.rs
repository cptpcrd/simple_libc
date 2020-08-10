use std::io;

use bitflags::bitflags;

use crate::rusage::Rusage;
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

    let pid = crate::error::convert_neg_ret(unsafe { libc::wait(&mut status) })?;

    Ok((pid, ProcStatus::from_raw_status(status)))
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum WaitpidSpec {
    Pid(PidT),
    Pgid(PidT),
    Any,
    CurrentPgid,
}

impl WaitpidSpec {
    fn to_wpid(&self) -> io::Result<PidT> {
        match *self {
            Self::Pid(pid) => {
                if pid <= 0 {
                    Err(io::Error::from_raw_os_error(libc::EINVAL))
                } else {
                    Ok(pid)
                }
            }
            Self::Pgid(pgid) => {
                if pgid <= 1 {
                    Err(io::Error::from_raw_os_error(libc::EINVAL))
                } else {
                    Ok(-pgid)
                }
            }
            Self::Any => Ok(-1),
            Self::CurrentPgid => Ok(0),
        }
    }
}

bitflags! {
    #[derive(Default)]
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
    let mut status: Int = 0;

    let pid = crate::error::convert_neg_ret(unsafe {
        libc::waitpid(spec.to_wpid()?, &mut status, options.bits)
    })?;

    Ok(match pid {
        0 => None,
        _ => Some((pid, ProcStatus::from_raw_status(status))),
    })
}

pub fn wait4(
    spec: WaitpidSpec,
    options: WaitpidOptions,
) -> io::Result<Option<(PidT, ProcStatus, Rusage)>> {
    let mut status: Int = 0;
    let mut rusage: libc::rusage = unsafe { std::mem::zeroed() };

    let pid = crate::error::convert_neg_ret(unsafe {
        libc::wait4(spec.to_wpid()?, &mut status, options.bits, &mut rusage)
    })?;

    Ok(match pid {
        0 => None,
        _ => Some((
            pid,
            ProcStatus::from_raw_status(status),
            Rusage::from(&rusage),
        )),
    })
}

crate::attr_group! {
    #![cfg(any(
        target_os = "linux",
        target_os = "netbsd",
        target_os = "freebsd",
        target_os = "dragonfly",
    ))]

    use crate::constants;
    use crate::{IdT, UidT};

    #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
    pub enum WaitidSpec {
        Pid(PidT),
        Pgid(PidT),
        Any,
        #[cfg(any(
            target_os = "netbsd",
            target_os = "freebsd",
            target_os = "dragonfly",
        ))]
        Uid(UidT),
        #[cfg(any(
            target_os = "netbsd",
            target_os = "freebsd",
            target_os = "dragonfly",
        ))]
        Gid(crate::GidT),
        #[cfg(any(
            target_os = "netbsd",
            target_os = "freebsd",
            target_os = "dragonfly",
        ))]
        Sid(PidT),
        #[cfg(any(
            target_os = "freebsd",
            target_os = "dragonfly",
        ))]
        Jailid(IdT),
    }

    impl WaitidSpec {
        fn unpack(&self) -> (libc::idtype_t, IdT) {
            match *self {
                Self::Pid(pid) => (libc::P_PID, pid as IdT),
                Self::Pgid(pgid) => (libc::P_PGID, pgid as IdT),
                Self::Any => (libc::P_ALL, 0),
                #[cfg(any(
                    target_os = "netbsd",
                    target_os = "freebsd",
                    target_os = "dragonfly",
                ))]
                Self::Uid(uid) => (constants::P_UID, uid as IdT),
                #[cfg(any(
                    target_os = "netbsd",
                    target_os = "freebsd",
                    target_os = "dragonfly",
                ))]
                Self::Gid(gid) => (constants::P_GID, gid as IdT),
                #[cfg(any(
                    target_os = "netbsd",
                    target_os = "freebsd",
                    target_os = "dragonfly",
                ))]
                Self::Sid(sid) => (constants::P_SID, sid as IdT),
                #[cfg(any(
                    target_os = "freebsd",
                    target_os = "dragonfly",
                ))]
                Self::Jailid(jailid) => (constants::P_JAILID, jailid),
            }
        }
    }

    bitflags! {
        #[derive(Default)]
        pub struct WaitidOptions: Int {
            const CONTINUED = libc::WCONTINUED;
            const EXITED = libc::WEXITED;
            const NOHANG = libc::WNOHANG;
            const NOWAIT = libc::WNOWAIT;
            const STOPPED = libc::WSTOPPED;
        }
    }

    #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
    pub enum WaitidStatus {
        Exited(Int),
        Killed(Int),
        Dumped(Int),
        Stopped,
        Trapped,
        Continued,
    }

    impl WaitidStatus {
        fn from_raw_code_status(code: Int, status: Int) -> io::Result<Self> {
            match code {
                constants::CLD_EXITED => Ok(WaitidStatus::Exited(status)),
                constants::CLD_KILLED => Ok(WaitidStatus::Killed(status)),
                constants::CLD_DUMPED => Ok(WaitidStatus::Dumped(status)),
                constants::CLD_STOPPED => Ok(WaitidStatus::Stopped),
                constants::CLD_TRAPPED => Ok(WaitidStatus::Trapped),
                constants::CLD_CONTINUED => Ok(WaitidStatus::Continued),
                _ => Err(io::Error::from_raw_os_error(libc::EINVAL)),
            }
        }
    }

    #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
    pub struct WaitidInfo {
        pub pid: PidT,
        pub uid: UidT,
        pub status: WaitidStatus,
    }

    impl WaitidInfo {
        fn from_raw_siginfo(raw_info: &libc::siginfo_t) -> io::Result<Option<Self>> {
            // On FreeBSD and DragonflyBSD, the siginfo_t struct defined in the libc crate
            // exposes the si_pid/si_uid/si_status fields.
            #[cfg(any(target_os = "freebsd", target_os = "dragonfly"))]
            let wait_info = raw_info;
            // On Linux/NetBSD, we have to pull them out with the help of a special struct.
            #[cfg(any(target_os = "linux", target_os = "netbsd"))]
            let wait_info = unsafe {
                &*(raw_info as *const libc::siginfo_t as *const crate::types::waitpid_siginfo)
            };

            if raw_info.si_signo == libc::SIGCHLD && wait_info.si_pid != 0 {
                Ok(Some(WaitidInfo {
                    pid: wait_info.si_pid,
                    uid: wait_info.si_uid,
                    status: WaitidStatus::from_raw_code_status(raw_info.si_code, wait_info.si_status)?,
                }))
            } else {
                Ok(None)
            }
        }
    }

    pub fn waitid(
        spec: WaitidSpec,
        options: WaitidOptions,
    ) -> io::Result<Option<WaitidInfo>> {
        let mut raw_info: libc::siginfo_t = unsafe { std::mem::zeroed() };

        let (idtype, id) = spec.unpack();

        #[cfg(target_os = "netbsd")]
        let waitid_res = unsafe {
            crate::externs::waitid(
                idtype, id, &mut raw_info, options.bits(),
            )
        };
        #[cfg(not(target_os = "netbsd"))]
        let waitid_res = unsafe {
            libc::waitid(
                idtype, id, &mut raw_info, options.bits(),
            )
        };

        crate::error::convert_nzero_ret(waitid_res)?;

        WaitidInfo::from_raw_siginfo(&raw_info)
    }
}

crate::attr_group! {
    #![cfg(any(
        target_os = "netbsd",
        target_os = "freebsd",
        target_os = "dragonfly",
    ))]

    pub fn wait6(
        spec: WaitidSpec,
        options: WaitidOptions,
    ) -> io::Result<Option<(ProcStatus, WaitidInfo, Rusage, Rusage)>> {
        let mut status = 0;
        let mut raw_info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let mut wrusage: crate::types::wrusage = unsafe { std::mem::zeroed() };

        let (idtype, id) = spec.unpack();

        let pid = crate::error::convert_neg_ret(unsafe {
            crate::externs::wait6(
                idtype, id, &mut status, options.bits(), &mut wrusage, &mut raw_info
            )
        })?;

        if pid != 0 {
            let info = WaitidInfo::from_raw_siginfo(&raw_info)?.unwrap();

            debug_assert_eq!(pid, info.pid);

            Ok(Some((
                ProcStatus::from_raw_status(status),
                info,
                Rusage::from(&wrusage.wru_self),
                Rusage::from(&wrusage.wru_children),
            )))
        } else {
            Ok(None)
        }
    }
}
