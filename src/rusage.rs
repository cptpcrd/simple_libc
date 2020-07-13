use std::io;
use std::time::Duration;

use crate::Int;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(isize)]
pub enum Target {
    Children = libc::RUSAGE_CHILDREN as isize,
    CurProc = libc::RUSAGE_SELF as isize,
    #[cfg(any(target_os = "linux", target_os = "openbsd", target_os = "freebsd"))]
    CurThread = libc::RUSAGE_THREAD as isize,
}

pub struct Rusage {
    pub utime: Duration,
    pub stime: Duration,
    pub maxrss: u64,
    pub ixrss: u64,
    pub idrss: u64,
    pub isrss: u64,
    pub minflt: u64,
    pub majflt: u64,
    pub nswap: u64,
    pub inblock: u64,
    pub oublock: u64,
    pub msgsnd: u64,
    pub msgrcv: u64,
    pub nsignals: u64,
    pub nvcsw: u64,
    pub nivcsw: u64,
}

impl From<&libc::rusage> for Rusage {
    fn from(r: &libc::rusage) -> Self {
        Self {
            utime: Duration::new(
                r.ru_utime.tv_sec as u64,
                (r.ru_utime.tv_usec % 1_000_000) as u32 * 1000,
            ),
            stime: Duration::new(
                r.ru_stime.tv_sec as u64,
                (r.ru_stime.tv_usec % 1_000_000) as u32 * 1000,
            ),
            maxrss: r.ru_maxrss as u64,
            ixrss: r.ru_ixrss as u64,
            idrss: r.ru_idrss as u64,
            isrss: r.ru_isrss as u64,
            minflt: r.ru_minflt as u64,
            majflt: r.ru_majflt as u64,
            nswap: r.ru_nswap as u64,
            inblock: r.ru_inblock as u64,
            oublock: r.ru_oublock as u64,
            msgsnd: r.ru_msgsnd as u64,
            msgrcv: r.ru_msgrcv as u64,
            nsignals: r.ru_nsignals as u64,
            nvcsw: r.ru_nvcsw as u64,
            nivcsw: r.ru_nivcsw as u64,
        }
    }
}

fn get_raw(target: Target) -> libc::rusage {
    let mut rusage: libc::rusage = unsafe { std::mem::zeroed() };

    let retval = unsafe { libc::getrusage(target as Int, &mut rusage) };

    // This should NEVER fail
    assert_eq!(retval, 0, "{}", io::Error::last_os_error());

    rusage
}

pub fn get(target: Target) -> Rusage {
    let raw_rusage = get_raw(target);

    Rusage::from(&raw_rusage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get() {
        get(Target::CurProc);
        get(Target::CurThread);
        get(Target::Children);
    }
}
