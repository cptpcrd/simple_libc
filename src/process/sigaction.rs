use std::io;

use libc;
use bitflags::bitflags;

use super::super::signal::Sigset;


bitflags! {
    #[derive(Default)]
    pub struct Flags: i32 {
        const SA_NOCLDSTOP = libc::SA_NOCLDSTOP;
        const SA_ONSTACK = libc::SA_ONSTACK;
        const SA_RESETHAND = libc::SA_RESETHAND;
        const SA_RESTART = libc::SA_RESTART;
        const SA_NOCLDWAIT = libc::SA_NOCLDWAIT;
        const SA_NODEFER = libc::SA_NODEFER;
    }
}

impl From<i32> for Flags {
    fn from(f: i32) -> Flags {
        Flags::from_bits_truncate(f)
    }
}


pub enum SigHandler {
    Default,
    Ignore,
    Handler(extern "C" fn(i32)),
}

pub struct Sigaction {
    pub handler: SigHandler,
    pub mask: Sigset,
    pub flags: Flags,
}

impl Sigaction {
    pub fn ignore() -> Sigaction {
        Sigaction {
            handler: SigHandler::Ignore,
            mask: Sigset::empty(),
            flags: Flags::empty(),
        }
    }

    pub fn ignoreflags(flags: Flags) -> Sigaction {
        Sigaction {
            handler: SigHandler::Ignore,
            mask: Sigset::empty(),
            flags,
        }
    }

    pub fn default() -> Sigaction {
        Sigaction {
            handler: SigHandler::Default,
            mask: Sigset::empty(),
            flags: Flags::empty(),
        }
    }

    pub fn empty_handler() -> Sigaction {
        Sigaction {
            handler: SigHandler::Handler(empty_sighandler),
            mask: Sigset::empty(),
            flags: Flags::empty(),
        }
    }
}

impl From<Sigaction> for libc::sigaction {
    fn from(act: Sigaction) -> libc::sigaction {
        libc::sigaction {
            sa_mask: act.mask.raw_set(),
            sa_flags: act.flags.bits(),
            sa_sigaction: match act.handler {
                SigHandler::Default => libc::SIG_DFL,
                SigHandler::Ignore => libc::SIG_IGN,
                SigHandler::Handler(f) => f as libc::sighandler_t,
            },
            sa_restorer: None,
        }
    }
}

impl From<libc::sigaction> for Sigaction {
    fn from(act: libc::sigaction) -> Sigaction {
        Sigaction {
            mask: Sigset::from(act.sa_mask),
            flags: Flags::from(act.sa_flags),
            handler: match act.sa_sigaction {
                libc::SIG_DFL => SigHandler::Default,
                libc::SIG_IGN => SigHandler::Ignore,
                _ => SigHandler::Handler(unsafe {
                    std::mem::transmute(act.sa_sigaction)
                }),
            },
        }
    }
}


fn sigaction(sig: i32, act: Option<Sigaction>) ->io::Result<Sigaction> {
    let mut oldact: libc::sigaction = unsafe { std::mem::zeroed() };

    let mut newact: *const libc::sigaction = std::ptr::null();
    if let Some(a) = act {
        newact = &libc::sigaction::from(a);
    }

    super::super::error::convert(unsafe {
        libc::sigaction(sig, newact, &mut oldact)
    }, oldact).map(|oldact| Sigaction::from(oldact))
}

pub fn sig_getaction(sig: i32) ->io::Result<Sigaction> {
    sigaction(sig, None)
}

pub fn sig_setaction(sig: i32, act: Sigaction) ->io::Result<Sigaction> {
    sigaction(sig, Some(act))
}

pub extern "C" fn empty_sighandler(_sig: i32) {
}