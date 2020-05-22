use std::io;

use super::signal::Sigset;
use super::Int;

fn sigmask(how: Int, set: Option<&Sigset>) -> io::Result<Sigset> {
    let oldset = Sigset::empty();

    let raw_set: *const libc::sigset_t = match set {
        Some(s) => &s.raw_set(),
        None => std::ptr::null(),
    };

    super::error::convert(
        unsafe { libc::pthread_sigmask(how, raw_set, &mut oldset.raw_set()) },
        oldset,
    )
}

pub fn getmask() -> io::Result<Sigset> {
    sigmask(0, None)
}

pub fn setmask(set: &Sigset) -> io::Result<Sigset> {
    sigmask(libc::SIG_SETMASK, Some(set))
}

pub fn block(set: &Sigset) -> io::Result<Sigset> {
    sigmask(libc::SIG_BLOCK, Some(set))
}

pub fn unblock(set: &Sigset) -> io::Result<Sigset> {
    sigmask(libc::SIG_UNBLOCK, Some(set))
}
