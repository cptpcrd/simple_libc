use std::io;

use libc;


pub const SIGABRT: i32 = libc::SIGABRT;
pub const SIGALRM: i32 = libc::SIGALRM;
pub const SIGBUS: i32 = libc::SIGBUS;
pub const SIGCHLD: i32 = libc::SIGCHLD;
pub const SIGCONT: i32 = libc::SIGCONT;
pub const SIGFPE: i32 = libc::SIGFPE;
pub const SIGHUP: i32 = libc::SIGHUP;
pub const SIGILL: i32 = libc::SIGILL;
pub const SIGINT: i32 = libc::SIGINT;
pub const SIGKILL: i32 = libc::SIGKILL;
pub const SIGPIPE: i32 = libc::SIGPIPE;
pub const SIGQUIT: i32 = libc::SIGQUIT;
pub const SIGSEGV: i32 = libc::SIGSEGV;
pub const SIGSTOP: i32 = libc::SIGSTOP;
pub const SIGTERM: i32 = libc::SIGTERM;
pub const SIGTSTP: i32 = libc::SIGTSTP;
pub const SIGTTIN: i32 = libc::SIGTTIN;
pub const SIGTTOU: i32 = libc::SIGTTOU;
pub const SIGUSR1: i32 = libc::SIGUSR1;
pub const SIGUSR2: i32 = libc::SIGUSR2;
pub const SIGPOLL: i32 = libc::SIGPOLL;
pub const SIGPROF: i32 = libc::SIGPROF;
pub const SIGSYS: i32 = libc::SIGSYS;
pub const SIGTRAP: i32 = libc::SIGTRAP;
pub const SIGURG: i32 = libc::SIGURG;
pub const SIGVTALRM: i32 = libc::SIGVTALRM;
pub const SIGXCPU: i32 = libc::SIGXCPU;
pub const SIGXFSZ: i32 = libc::SIGXFSZ;

pub fn can_catch(sig: i32) -> bool {
    match sig {
        libc::SIGKILL => false,
        libc::SIGSTOP => false,
        _ => true,
    }
}

pub fn sig_from_name(name: &str) -> Option<i32> {
    match name {
        "SIGABRT" => Some(SIGABRT),
        "SIGALRM" => Some(SIGALRM),
        "SIGBUS" => Some(SIGBUS),
        "SIGCHLD" => Some(SIGCHLD),
        "SIGCONT" => Some(SIGCONT),
        "SIGFPE" => Some(SIGFPE),
        "SIGHUP" => Some(SIGHUP),
        "SIGILL" => Some(SIGILL),
        "SIGINT" => Some(SIGINT),
        "SIGKILL" => Some(SIGKILL),
        "SIGPIPE" => Some(SIGPIPE),
        "SIGQUIT" => Some(SIGQUIT),
        "SIGSEGV" => Some(SIGSEGV),
        "SIGSTOP" => Some(SIGSTOP),
        "SIGTERM" => Some(SIGTERM),
        "SIGTSTP" => Some(SIGTSTP),
        "SIGTTIN" => Some(SIGTTIN),
        "SIGTTOU" => Some(SIGTTOU),
        "SIGUSR1" => Some(SIGUSR1),
        "SIGUSR2" => Some(SIGUSR2),
        "SIGPOLL" => Some(SIGPOLL),
        "SIGPROF" => Some(SIGPROF),
        "SIGSYS" => Some(SIGSYS),
        "SIGTRAP" => Some(SIGTRAP),
        "SIGURG" => Some(SIGURG),
        "SIGVTALRM" => Some(SIGVTALRM),
        "SIGXCPU" => Some(SIGXCPU),
        "SIGXFSZ" => Some(SIGXFSZ),
        _ => None,
    }
}


#[repr(transparent)]
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
        return s;
    }

    pub fn full() -> Sigset {
        let mut s = Self::unsafe_new();
        s.fill();
        return s;
    }

    #[inline]
    pub fn clear(&mut self) {
        unsafe {
            libc::sigemptyset(&mut self.set)
        };
    }

    #[inline]
    pub fn fill(&mut self) {
        unsafe {
            libc::sigfillset(&mut self.set)
        };
    }

    pub fn add(&mut self, sig: i32) -> Result<(), io::Error> {
        super::error::convert(unsafe {
            libc::sigaddset(&mut self.set, sig)
        }, ())
    }

    pub fn del(&mut self, sig: i32) -> Result<(), io::Error> {
        super::error::convert(unsafe {
            libc::sigdelset(&mut self.set, sig)
        }, ())
    }

    pub fn ismember(&self, sig: i32) -> Result<bool, io::Error> {
        super::error::convert_ret(unsafe {
            libc::sigismember(&self.set, sig)
        }).map(|res| { res != 0 })
    }

    #[inline]
    pub fn raw_set(&self) -> libc::sigset_t {
        self.set
    }

    #[inline]
    pub fn to_raw_set(self) -> libc::sigset_t {
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
    fn test_sigset() {
        let mut set = Sigset::empty();
        assert!(!set.ismember(SIGTERM).unwrap());
        set.fill();
        assert!(set.ismember(SIGTERM).unwrap());

        set.clear();
        assert!(!set.ismember(SIGTERM).unwrap());
        set.add(SIGTERM).unwrap();
        assert!(set.ismember(SIGTERM).unwrap());

        set = Sigset::full();
        assert!(set.ismember(SIGTERM).unwrap());
    }
}
