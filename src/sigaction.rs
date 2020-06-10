use std::io;

use bitflags::bitflags;

use crate::signal::Sigset;
use crate::Int;

bitflags! {
    #[derive(Default)]
    pub struct Flags: Int {
        const NOCLDSTOP = libc::SA_NOCLDSTOP;
        const ONSTACK = libc::SA_ONSTACK;
        const RESETHAND = libc::SA_RESETHAND;
        const RESTART = libc::SA_RESTART;
        const NOCLDWAIT = libc::SA_NOCLDWAIT;
        const NODEFER = libc::SA_NODEFER;
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum SigHandler {
    Default,
    Ignore,
    Handler(extern "C" fn(Int)),
    ActionHandler(extern "C" fn(Int, *mut libc::siginfo_t, *mut libc::c_void)),
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
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
            sa_flags: act.flags.bits
                | (match act.handler {
                    SigHandler::ActionHandler(_) => libc::SA_SIGINFO,
                    _ => 0,
                }),
            sa_sigaction: match act.handler {
                SigHandler::Default => libc::SIG_DFL,
                SigHandler::Ignore => libc::SIG_IGN,
                SigHandler::Handler(f) => f as libc::sighandler_t,
                SigHandler::ActionHandler(f) => f as libc::sighandler_t,
            },
            #[cfg(target_os = "linux")]
            sa_restorer: None,
        }
    }
}

impl From<libc::sigaction> for Sigaction {
    fn from(act: libc::sigaction) -> Sigaction {
        Sigaction {
            mask: Sigset::from(act.sa_mask),
            flags: Flags::from_bits_truncate(act.sa_flags),
            handler: match act.sa_sigaction {
                libc::SIG_DFL => SigHandler::Default,
                libc::SIG_IGN => SigHandler::Ignore,
                _ => {
                    if act.sa_flags & libc::SA_SIGINFO != 0 {
                        SigHandler::ActionHandler(unsafe { std::mem::transmute(act.sa_sigaction) })
                    } else {
                        SigHandler::Handler(unsafe { std::mem::transmute(act.sa_sigaction) })
                    }
                }
            },
        }
    }
}

fn sigaction(sig: Int, act: Option<Sigaction>) -> io::Result<Sigaction> {
    let mut oldact = unsafe { std::mem::zeroed() };

    let mut newact = std::ptr::null();
    if let Some(a) = act {
        newact = &libc::sigaction::from(a);
    }

    crate::error::convert_nzero_ret(unsafe { libc::sigaction(sig, newact, &mut oldact) })?;

    Ok(Sigaction::from(oldact))
}

pub fn sig_getaction(sig: Int) -> io::Result<Sigaction> {
    sigaction(sig, None)
}

pub fn sig_setaction(sig: Int, act: Sigaction) -> io::Result<Sigaction> {
    sigaction(sig, Some(act))
}

pub extern "C" fn empty_sighandler(_sig: Int) {}
