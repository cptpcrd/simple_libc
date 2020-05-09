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


#[inline]
pub fn getgid() -> u32 {
    unsafe { libc::getgid() }
}

#[inline]
pub fn getegid() -> u32 {
    unsafe { libc::getegid() }
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

    let (rgid, egid) = getregid();

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

pub fn setgroups(groups: &[u32]) -> io::Result<()> {
    super::error::convert_nzero(unsafe {
        libc::setgroups(groups.len(), groups.as_ptr())
    }, ())
}


cfg_if::cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "dragonfly"))] {
        pub fn getresuid() -> (u32, u32, u32) {
            let mut ruid: u32 = 0;
            let mut euid: u32 = 0;
            let mut suid: u32 = 0;

            unsafe { libc::getresuid(&mut ruid, &mut euid, &mut suid); }
            (ruid, euid, suid)
        }

        pub fn getresgid() -> (u32, u32, u32) {
            let mut rgid: u32 = 0;
            let mut egid: u32 = 0;
            let mut sgid: u32 = 0;

            unsafe { libc::getresgid(&mut rgid, &mut egid, &mut sgid); }
            (rgid, egid, sgid)
        }

        pub fn setresuid(ruid: u32, euid: u32, suid: u32) -> io::Result<()> {
            super::error::convert_nzero(unsafe {
                libc::setresuid(ruid, euid, suid)
            }, ())
        }

        pub fn setresgid(rgid: u32, egid: u32, sgid: u32) -> io::Result<()> {
            super::error::convert_nzero(unsafe {
                libc::setresgid(rgid, egid, sgid)
            }, ())
        }

        fn _getreuid() -> (u32, u32) {
            let (ruid, euid, _) = getresuid();
            (ruid, euid)
        }

        fn _getregid() -> (u32, u32) {
            let (rgid, egid, _) = getresgid();
            (rgid, egid)
        }
    }
    else {
        fn _getreuid() -> (u32, u32) {
            (getuid(), geteuid())
        }

        fn _getregid() -> (u32, u32) {
            (getgid(), getegid())
        }
    }
}

/// Gets the real and effective user IDs via the most efficient method possible.
///
/// On platforms with `getresuid()`, this function calls that function and discards
/// the saved UID. On other platforms, it combines the results of `getuid()` and
/// `geteuid()`.
#[inline]
pub fn getreuid() -> (u32, u32) {
    _getreuid()
}

/// Gets the real and effective group IDs via the most efficient method possible.
///
/// On platforms with `getresgid()`, this function calls that function and discards
/// the saved GID. On other platforms, it combines the results of `getgid()` and
/// `getegid()`.
#[inline]
pub fn getregid() -> (u32, u32) {
    _getregid()
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


pub fn fork() -> io::Result<i32> {
    super::error::convert_neg_ret(unsafe { libc::fork() })
}

pub fn execvp<U: Into<Vec<u8>> + Clone + Sized>(arg0: &str, argv: &[U]) -> io::Result<()> {
    let c_arg0 = ffi::CString::new(arg0)?;

    let mut c_argv: Vec<*mut libc::c_char> = Vec::with_capacity(argv.len() + 1);

    for arg in argv {
        c_argv.push(ffi::CString::new(arg.clone())?.into_raw())
    }

    c_argv.push(std::ptr::null_mut());

    unsafe {
        libc::execvp(c_arg0.as_ptr(), c_argv.as_ptr() as *const *const i8);
    }

    for arg in c_argv {
        if arg != std::ptr::null_mut() {
            unsafe {
                let _ = ffi::CString::from_raw(arg);
            }
        }
    }

    Err(io::Error::last_os_error().into())
}
