use std::io;
use libc;

use super::super::error;

pub fn nice(incr: i32) -> io::Result<i32> {
    error::set_errno_success();
    error::convert_if_errno_ret(unsafe { libc::nice(incr) })
}


pub enum Target {
    Process(u32),
    ProcGroup(u32),
    User(u32),
}

// Work around GNU not implementing the POSIX standard correctly
#[cfg(any(target_env = "", target_env = "gnu"))]
type PriorityWhich = libc::__priority_which_t;

#[cfg(not(any(target_env = "", target_env = "gnu")))]
type PriorityWhich = i32;

impl Target {
    fn unpack(&self) -> (PriorityWhich, u32) {
        match self {
            Self::Process(w) => (libc::PRIO_PROCESS as PriorityWhich, *w),
            Self::ProcGroup(w) => (libc::PRIO_PGRP as PriorityWhich, *w),
            Self::User(w) => (libc::PRIO_USER as PriorityWhich, *w),
        }
    }
}

pub fn get(t: Target) -> io::Result<i32> {
    let (which, who) = t.unpack();

    error::set_errno_success();
    error::convert_if_errno_ret(unsafe {
        libc::getpriority(which, who)
    })
}

pub fn set(t: Target, value: i32) -> io::Result<()> {
    let (which, who) = t.unpack();

    error::set_errno_success();
    error::convert_nzero(unsafe {
        libc::setpriority(which, who, value)
    }, ())
}
