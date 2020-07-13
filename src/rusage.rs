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

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
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

impl Rusage {
    fn checked_sub(self, rhs: Self) -> Option<Self> {
        Some(Self {
            utime: self.utime.checked_sub(rhs.utime)?,
            stime: self.stime.checked_sub(rhs.stime)?,
            maxrss: self.maxrss.checked_sub(rhs.maxrss)?,
            ixrss: self.ixrss.checked_sub(rhs.ixrss)?,
            idrss: self.idrss.checked_sub(rhs.idrss)?,
            isrss: self.isrss.checked_sub(rhs.isrss)?,
            minflt: self.minflt.checked_sub(rhs.minflt)?,
            majflt: self.majflt.checked_sub(rhs.majflt)?,
            nswap: self.nswap.checked_sub(rhs.nswap)?,
            inblock: self.inblock.checked_sub(rhs.inblock)?,
            oublock: self.oublock.checked_sub(rhs.oublock)?,
            msgsnd: self.msgsnd.checked_sub(rhs.msgsnd)?,
            msgrcv: self.msgrcv.checked_sub(rhs.msgrcv)?,
            nsignals: self.nsignals.checked_sub(rhs.nsignals)?,
            nvcsw: self.nvcsw.checked_sub(rhs.nvcsw)?,
            nivcsw: self.nivcsw.checked_sub(rhs.nivcsw)?,
        })
    }
}

impl std::ops::Sub for Rusage {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        self.checked_sub(rhs).unwrap()
    }
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
    Rusage::from(&get_raw(target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get() {
        get(Target::Children);
        get(Target::CurProc);
        #[cfg(any(target_os = "linux", target_os = "openbsd", target_os = "freebsd"))]
        get(Target::CurThread);
    }

    #[test]
    fn test_sub() {
        let rusage_a = get(Target::CurProc);
        let rusage_b = get(Target::CurProc);

        assert!(rusage_b.checked_sub(rusage_a).is_some());
        let _ = rusage_b - rusage_a;

        assert!(rusage_a.checked_sub(rusage_b).is_none());
        std::panic::catch_unwind(|| {
            let _ = rusage_a - rusage_b;
        })
        .unwrap_err();
    }
}
