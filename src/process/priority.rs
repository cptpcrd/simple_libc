use std::io;
use libc;

use super::super::error;
use super::super::{Int, IdT};


pub fn nice(incr: Int) -> io::Result<Int> {
    error::set_errno_success();
    error::convert_if_errno_ret(unsafe { libc::nice(incr) })
}


#[derive(Debug)]
pub enum Target {
    Process(IdT),
    ProcGroup(IdT),
    User(IdT),
}

// Work around GNU not implementing the POSIX standard correctly
#[cfg(any(target_env = "", target_env = "gnu"))]
type PriorityWhich = libc::__priority_which_t;

#[cfg(not(any(target_env = "", target_env = "gnu")))]
type PriorityWhich = Int;

impl Target {
    fn unpack(&self) -> (PriorityWhich, u32) {
        match self {
            Self::Process(w) => (libc::PRIO_PROCESS as PriorityWhich, *w),
            Self::ProcGroup(w) => (libc::PRIO_PGRP as PriorityWhich, *w),
            Self::User(w) => (libc::PRIO_USER as PriorityWhich, *w),
        }
    }
}

pub fn get(t: Target) -> io::Result<Int> {
    let (which, who) = t.unpack();

    error::set_errno_success();
    error::convert_if_errno_ret(unsafe {
        libc::getpriority(which, who)
    })
}

pub fn set(t: Target, value: Int) -> io::Result<()> {
    let (which, who) = t.unpack();

    error::convert_nzero(unsafe {
        libc::setpriority(which, who, value)
    }, ())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_equals_nice() {
        assert_eq!(super::nice(0).unwrap(), super::get(super::Target::Process(0)).unwrap());
    }
}
