use std::io;

use crate::error;
use crate::{Int, PidT, UidT};

pub fn nice(incr: Int) -> io::Result<Int> {
    error::set_errno_success();
    error::convert_if_errno_ret(unsafe { libc::nice(incr) })
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum Target {
    Process(PidT),
    ProcGroup(PidT),
    User(UidT),
}

// Work around GNU not implementing the POSIX standard correctly
#[cfg(all(target_os = "linux", any(target_env = "", target_env = "gnu")))]
type PriorityWhich = libc::__priority_which_t;
#[cfg(not(all(target_os = "linux", any(target_env = "", target_env = "gnu"))))]
type PriorityWhich = Int;

#[cfg(any(target_os = "linux", target_os = "openbsd", target_os = "netbsd"))]
type PriorityWho = crate::IdT;
#[cfg(any(target_os = "freebsd", target_os = "dragonfly"))]
type PriorityWho = Int;
#[cfg(any(target_os = "macos"))]
type PriorityWho = crate::Uint;

impl Target {
    fn unpack(self) -> (PriorityWhich, PriorityWho) {
        match self {
            Self::Process(w) => (libc::PRIO_PROCESS as PriorityWhich, w as PriorityWho),
            Self::ProcGroup(w) => (libc::PRIO_PGRP as PriorityWhich, w as PriorityWho),
            Self::User(w) => (libc::PRIO_USER as PriorityWhich, w as PriorityWho),
        }
    }
}

pub fn get(t: Target) -> io::Result<Int> {
    let (which, who) = t.unpack();

    error::set_errno_success();
    error::convert_if_errno_ret(unsafe { libc::getpriority(which, who) })
}

pub fn set(t: Target, value: Int) -> io::Result<()> {
    let (which, who) = t.unpack();

    error::convert_nzero_ret(unsafe { libc::setpriority(which, who, value) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_equals_nice() {
        assert_eq!(nice(0).unwrap(), get(Target::Process(0)).unwrap());
    }

    #[test]
    fn test_get_cur_pgrp_user() {
        // Make sure we can get the value
        get(Target::ProcGroup(0)).unwrap();
        get(Target::User(0)).unwrap();
    }

    #[test]
    fn test_set() {
        set(Target::Process(0), get(Target::Process(0)).unwrap()).unwrap();
    }
}
