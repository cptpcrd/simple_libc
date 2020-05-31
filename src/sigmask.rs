use std::io;

use crate::signal::Sigset;
use crate::Int;

fn sigmask(how: Int, set: Option<&Sigset>) -> io::Result<Sigset> {
    let oldset = Sigset::empty();

    let raw_set = match set {
        Some(s) => &s.raw_set(),
        None => std::ptr::null(),
    };

    match unsafe { libc::pthread_sigmask(how, raw_set, &mut oldset.raw_set()) } {
        0 => Ok(oldset),
        errno => Err(io::Error::from_raw_os_error(errno)),
    }
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
