use std::io;

use libc;

use super::super::signal::Sigset;


fn sigmask(how: i32, set: &Sigset) ->io::Result<Sigset> {
    let oldset = Sigset::empty();

    super::super::error::convert(unsafe {
        libc::pthread_sigmask(how, &set.raw_set(), &mut oldset.raw_set())
    }, oldset)
}

pub fn get() -> io::Result<Sigset> {
    sigmask(0, &Sigset::empty())
}

pub fn setmask(set: &Sigset) -> io::Result<Sigset> {
    sigmask(libc::SIG_SETMASK, set)
}

pub fn block(set: &Sigset) -> io::Result<Sigset> {
    sigmask(libc::SIG_BLOCK, set)
}

pub fn unblock(set: &Sigset) -> io::Result<Sigset> {
    sigmask(libc::SIG_UNBLOCK, set)
}
