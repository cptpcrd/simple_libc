use std::io;
use libc;

use super::super::error;


// Work around GNU not implementing the POSIX standard correctly
#[cfg(any(target_env = "", target_env = "gnu"))]
pub type Resource = libc::__rlimit_resource_t;

#[cfg(not(any(target_env = "", target_env = "gnu")))]
pub type Resource = i32;

pub const RLIMIT_AS: Resource = libc::RLIMIT_AS;
pub const RLIMIT_CORE: Resource = libc::RLIMIT_CORE;
pub const RLIMIT_CPU: Resource = libc::RLIMIT_CPU;
pub const RLIMIT_DATA: Resource = libc::RLIMIT_DATA;
pub const RLIMIT_NOFILE: Resource = libc::RLIMIT_NOFILE;
pub const RLIMIT_FSIZE: Resource = libc::RLIMIT_FSIZE;
pub const RLIMIT_STACK: Resource = libc::RLIMIT_STACK;

#[cfg(target_os = "linux")]
pub const RLIMIT_MEMLOCK: Resource = libc::RLIMIT_MEMLOCK;
#[cfg(target_os = "linux")]
pub const RLIMIT_MSGQUEUE: Resource = libc::RLIMIT_MSGQUEUE;
#[cfg(target_os = "linux")]
pub const RLIMIT_NICE: Resource = libc::RLIMIT_NICE;
#[cfg(target_os = "linux")]
pub const RLIMIT_NPROC: Resource = libc::RLIMIT_NPROC;
#[cfg(target_os = "linux")]
pub const RLIMIT_RSS: Resource = libc::RLIMIT_RSS;
#[cfg(target_os = "linux")]
pub const RLIMIT_RTPRIO: Resource = libc::RLIMIT_RTPRIO;
#[cfg(target_os = "linux")]
pub const RLIMIT_RTTIME: Resource = libc::RLIMIT_RTTIME;
#[cfg(target_os = "linux")]
pub const RLIMIT_SIGPENDING: Resource = libc::RLIMIT_SIGPENDING;


pub type Limit = libc::rlim_t;
pub const LIMIT_INFINITY: Limit = libc::RLIM_INFINITY;


pub fn getrlimit(resource: Resource) -> io::Result<(Limit, Limit)> {
    let mut rlim = libc::rlimit { rlim_cur: LIMIT_INFINITY, rlim_max: LIMIT_INFINITY };

    error::convert_nzero(unsafe {
        libc::getrlimit(resource, &mut rlim);
    }, rlim).map(|rlim| (rlim.rlim_cur, rlim.rlim_max))
}

pub fn setrlimit(resource: Resource, new_limits: (Limit, Limit)) -> io::Result<()> {
    let rlim = libc::rlimit { rlim_cur: new_limits.0, rlim_max: new_limits.1 };

    error::convert_nzero(unsafe {
        libc::setrlimit(resource, &rlim);
    }, ())
}

#[cfg(target_os = "linux")]
pub fn prlimit(pid: i32, resource: Resource, new_limits: Option<(Limit, Limit)>) -> io::Result<(Limit, Limit)> {
    let mut new_rlim = libc::rlimit { rlim_cur: LIMIT_INFINITY, rlim_max: LIMIT_INFINITY };
    let mut new_rlim_ptr: *const libc::rlimit = std::ptr::null();

    if let Some(new_lims) = new_limits {
        new_rlim.rlim_cur = new_lims.0;
        new_rlim.rlim_max = new_lims.1;
        new_rlim_ptr = &new_rlim;
    }

    let mut old_rlim = libc::rlimit { rlim_cur: LIMIT_INFINITY, rlim_max: LIMIT_INFINITY };

    error::convert_nzero(unsafe {
        libc::prlimit(pid, resource, new_rlim_ptr, &mut old_rlim);
    }, old_rlim).map(|old_rlim| (old_rlim.rlim_cur, old_rlim.rlim_max))
}

#[cfg(target_os = "linux")]
pub fn nice_rlimit_to_thresh(nice_rlim: Limit) -> i32 {
    20 - (super::super::constrain(nice_rlim, 1, 40) as i32)
}

#[cfg(target_os = "linux")]
pub fn nice_thresh_to_rlimit(nice_thresh: i32) -> Limit {
    (20 - super::super::constrain(nice_thresh, -20, 19)) as Limit
}
