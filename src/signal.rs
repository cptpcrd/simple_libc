use std::collections::HashMap;
use std::io;

use lazy_static::lazy_static;
pub use libc::{
    SIGABRT, SIGALRM, SIGBUS, SIGCHLD, SIGCONT, SIGFPE, SIGHUP, SIGILL, SIGINT, SIGKILL, SIGPIPE,
    SIGPROF, SIGQUIT, SIGSEGV, SIGSTOP, SIGSYS, SIGTERM, SIGTRAP, SIGTSTP, SIGTTIN, SIGTTOU,
    SIGURG, SIGUSR1, SIGUSR2, SIGVTALRM, SIGXCPU, SIGXFSZ,
};

#[cfg(target_os = "linux")]
pub use libc::SIGPOLL;

use crate::Int;

pub fn can_catch(sig: Int) -> bool {
    match sig {
        SIGKILL => false,
        SIGSTOP => false,
        _ => true,
    }
}

lazy_static! {
    static ref SIGNALS_BY_NAME: HashMap<&'static str, Int> = {
        let mut m = HashMap::new();
        m.insert("SIGABRT", SIGABRT);
        m.insert("SIGALRM", SIGALRM);
        m.insert("SIGBUS", SIGBUS);
        m.insert("SIGCHLD", SIGCHLD);
        m.insert("SIGCONT", SIGCONT);
        m.insert("SIGFPE", SIGFPE);
        m.insert("SIGHUP", SIGHUP);
        m.insert("SIGILL", SIGILL);
        m.insert("SIGINT", SIGINT);
        m.insert("SIGKILL", SIGKILL);
        m.insert("SIGPIPE", SIGPIPE);
        m.insert("SIGPROF", SIGPROF);
        m.insert("SIGQUIT", SIGQUIT);
        m.insert("SIGSEGV", SIGSEGV);
        m.insert("SIGSTOP", SIGSTOP);
        m.insert("SIGSYS", SIGSYS);
        m.insert("SIGTERM", SIGTERM);
        m.insert("SIGTRAP", SIGTRAP);
        m.insert("SIGTSTP", SIGTSTP);
        m.insert("SIGTTIN", SIGTTIN);
        m.insert("SIGTTOU", SIGTTOU);
        m.insert("SIGURG", SIGURG);
        m.insert("SIGUSR1", SIGUSR1);
        m.insert("SIGUSR2", SIGUSR2);
        m.insert("SIGVTALRM", SIGVTALRM);
        m.insert("SIGXCPU", SIGXCPU);
        m.insert("SIGXFSZ", SIGXFSZ);

        #[cfg(target_os = "linux")]
        m.insert("SIGPOLL", SIGPOLL);

        m
    };
}

pub fn sig_from_name(name: &str) -> Option<Int> {
    SIGNALS_BY_NAME.get(name).copied()
}

#[cfg(target_os = "linux")]
pub fn get_rtsig_minmax() -> io::Result<(Int, Int)> {
    let res = unsafe {
        (
            crate::externs::__libc_current_sigrtmin(),
            crate::externs::__libc_current_sigrtmax(),
        )
    };

    Ok(res)
}

#[cfg(target_os = "linux")]
pub fn get_rtsig_range() -> io::Result<std::ops::RangeInclusive<Int>> {
    let (sigrtmin, sigrtmax) = get_rtsig_minmax()?;
    Ok(sigrtmin..=sigrtmax)
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct Sigset {
    set: libc::sigset_t,
}

impl Sigset {
    fn unsafe_new() -> Sigset {
        Sigset {
            set: unsafe { std::mem::zeroed() },
        }
    }

    pub fn empty() -> Sigset {
        let mut s = Self::unsafe_new();
        s.clear();
        s
    }

    pub fn full() -> Sigset {
        let mut s = Self::unsafe_new();
        s.fill();
        s
    }

    #[inline]
    pub fn clear(&mut self) {
        unsafe { libc::sigemptyset(&mut self.set) };
    }

    #[inline]
    pub fn fill(&mut self) {
        unsafe { libc::sigfillset(&mut self.set) };
    }

    pub fn add(&mut self, sig: i32) -> io::Result<()> {
        crate::error::convert(unsafe { libc::sigaddset(&mut self.set, sig) }, ())
    }

    pub fn del(&mut self, sig: i32) -> io::Result<()> {
        crate::error::convert(unsafe { libc::sigdelset(&mut self.set, sig) }, ())
    }

    pub fn ismember(&self, sig: i32) -> io::Result<bool> {
        let res = crate::error::convert_ret(unsafe { libc::sigismember(&self.set, sig) })?;

        Ok(res != 0)
    }

    #[inline]
    pub fn raw_set(&self) -> libc::sigset_t {
        self.set
    }

    #[inline]
    pub fn into_raw_set(self) -> libc::sigset_t {
        self.set
    }
}

impl From<libc::sigset_t> for Sigset {
    #[inline]
    fn from(set: libc::sigset_t) -> Sigset {
        Sigset { set }
    }
}

impl Default for Sigset {
    #[inline]
    fn default() -> Sigset {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_catch() {
        assert!(can_catch(SIGTERM));
        assert!(!can_catch(SIGKILL));
        assert!(!can_catch(SIGSTOP));
    }

    #[test]
    fn test_sig_from_name() {
        assert_eq!(sig_from_name("SIGALRM"), Some(SIGALRM));
        assert_eq!(sig_from_name("SIGALRM_BAD"), None);

        #[cfg(target_os = "linux")]
        assert_eq!(sig_from_name("SIGPOLL"), Some(SIGPOLL));
    }

    #[test]
    fn test_sigset() {
        let mut set = Sigset::default();
        assert!(!set.ismember(SIGTERM).unwrap());
        set.fill();
        assert!(set.ismember(SIGTERM).unwrap());

        set.clear();
        assert!(!set.ismember(SIGTERM).unwrap());
        set.add(SIGTERM).unwrap();
        assert!(set.ismember(SIGTERM).unwrap());
        set.del(SIGTERM).unwrap();
        assert!(!set.ismember(SIGTERM).unwrap());

        set = Sigset::full();
        assert!(set.ismember(SIGTERM).unwrap());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_get_rtsig_minmax_range() {
        get_rtsig_minmax().unwrap();
        get_rtsig_range().unwrap();
    }
}
