use std::ffi;
use std::io;
use std::path::Path;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::ffi::OsStringExt;
use libc;

pub mod sigmask;
pub mod sigaction;
pub mod priority;
pub mod resource;

#[cfg(target_os = "linux")]
pub mod signalfd;
#[cfg(target_os = "linux")]
pub mod prctl;


#[inline]
pub fn getpid() -> i32 {
    unsafe { libc::getpid() }
}


#[inline]
pub fn getuid() -> u32 {
    unsafe { libc::getuid() }
}

#[inline]
pub fn geteuid() -> u32 {
    unsafe { libc::geteuid() }
}

#[cfg(target_os = "linux")]
pub fn getresuid() -> (u32, u32, u32) {
    let mut ruid: u32 = 0;
    let mut euid: u32 = 0;
    let mut suid: u32 = 0;

    unsafe { libc::getresuid(&mut ruid, &mut euid, &mut suid); }
    (ruid, euid, suid)
}


#[inline]
pub fn getgid() -> u32 {
    unsafe { libc::getgid() }
}

#[inline]
pub fn getegid() -> u32 {
    unsafe { libc::getegid() }
}

#[cfg(target_os = "linux")]
pub fn getresgid() -> (u32, u32, u32) {
    let mut rgid: u32 = 0;
    let mut egid: u32 = 0;
    let mut sgid: u32 = 0;

    unsafe { libc::getresgid(&mut rgid, &mut egid, &mut sgid); }
    (rgid, egid, sgid)
}

pub fn getgroups_raw(groups: &mut Vec<u32>) -> io::Result<i32> {
    super::error::convert_neg_ret(unsafe {
        libc::getgroups(groups.len() as i32, groups.as_mut_ptr())
    })
}

pub fn getgroups() -> io::Result<Vec<u32>> {
    let mut groups: Vec<u32> = Vec::new();

    let ngroups = getgroups_raw(&mut groups)?;

    groups.reserve(ngroups as usize);
    unsafe { groups.set_len(ngroups as usize) };

    if getgroups_raw(&mut groups)? != ngroups {
        return Err(io::Error::last_os_error());
    }

    Ok(groups)
}

pub fn getallgroups() -> io::Result<Vec<u32>> {
    let mut groups = getgroups()?;

    let (rgid, egid, _) = {
        if cfg!(target_os = "linux") {
            getresgid()
        }
        else {
            (getgid(), getegid(), 0)
        }
    };

    groups.retain(|&x| x != rgid && x != egid);

    if rgid == egid {
        groups.insert(0, egid);
    }
    else {
        groups.splice(0..0, [rgid, egid].iter().cloned());
    }

    Ok(groups)
}


extern "C" {
    fn getlogin_r(buf: *mut libc::c_char, bufsize: libc::size_t) -> i32;
}

pub fn getlogin() -> io::Result<ffi::OsString> {
    // Get the initial buffer length from sysconf(), setting some sane defaults/constraints.
    let init_length = super::constrain(super::sysconf(libc::_SC_LOGIN_NAME_MAX).unwrap_or(256), 64, 1024);

    super::error::while_erange(|i| {
        let length = (init_length * (i as i64 + 1)) as usize;
        let mut buf: Vec<i8> = Vec::with_capacity(length);

        super::error::convert_nzero(unsafe {
            buf.set_len(length);
            getlogin_r(buf.as_mut_ptr(), length)
        }, buf).map(|buf| {
            ffi::OsString::from_vec(buf.iter().take_while(|x| **x > 0).map(|x| *x as u8).collect())
        })
    }, 10)
}


pub fn setuid(uid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setuid(uid)
    }, ())
}

pub fn seteuid(uid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::seteuid(uid)
    }, ())
}

pub fn setreuid(ruid: u32, euid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setreuid(ruid, euid)
    }, ())
}

#[cfg(target_os = "linux")]
pub fn setresuid(ruid: u32, euid: u32, suid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setresuid(ruid, euid, suid)
    }, ())
}


pub fn setgid(gid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setgid(gid)
    }, ())
}

pub fn setegid(gid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setegid(gid)
    }, ())
}

pub fn setregid(rgid: u32, egid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setregid(rgid, egid)
    }, ())
}

#[cfg(target_os = "linux")]
pub fn setresgid(rgid: u32, egid: u32, sgid: u32) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setresgid(rgid, egid, sgid)
    }, ())
}


pub fn setgroups(groups: &[u32]) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setgroups(groups.len(), groups.as_ptr())
    }, ())
}


pub fn chroot<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = ffi::CString::new(path.as_ref().as_os_str().as_bytes())?;

    super::error::convert_nzero(unsafe {
        libc::chroot(path.as_ptr())
    }, ())
}

#[inline]
pub fn chdir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    std::env::set_current_dir(path)
}
