use std::fs::File;
use std::io;
use std::path::Path;
use std::ffi::{CStr, CString, OsStr};
use std::os::unix::prelude::*;

use crate::Long;

use bitflags::bitflags;

const O_CREAT: i32 = libc::O_CREAT as i32;
const O_TMPFILE: i32 = libc::O_TMPFILE as i32;

// This is correct for every architecture except alpha, which
// Rust does not support
const SYS_OPENAT: Long = 437;

bitflags! {
    pub struct ResolveFlags: u64 {
        const NO_XDEV = 0x01;
        const NO_MAGICLINKS = 0x02;
        const NO_SYMLINKS = 0x04;
        const BENEATH = 0x08;
        const IN_ROOT = 0x10;
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct OpenHow {
    pub flags: i32,
    pub mode: Option<u32>,
    pub resolve_flags: ResolveFlags,
}

impl OpenHow {
    pub fn new(flags: i32) -> Self {
        Self {
            flags,
            mode: None,
            resolve_flags: ResolveFlags::empty(),
        }
    }

    fn raw_mode(&self) -> u64 {
        if let Some(mode) = self.mode {
            mode as u64
        } else if self.flags & O_CREAT == O_CREAT || self.flags & O_TMPFILE == O_TMPFILE {
            0o777
        } else {
            0
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
#[repr(C)]
struct RawOpenHow {
    flags: u64,
    mode: u64,
    resolve: u64,
}

impl From<&OpenHow> for RawOpenHow {
    fn from(other: &OpenHow) -> Self {
        Self {
            flags: (other.flags | libc::O_CLOEXEC) as u64,
            mode: other.raw_mode(),
            resolve: other.resolve_flags.bits(),
        }
    }
}

fn openat2_sys(
    dirfd: Option<RawFd>,
    path: &CStr,
    how: &OpenHow,
) -> io::Result<RawFd> {
    let dirfd = dirfd.unwrap_or(libc::AT_FDCWD);
    let mut raw_how: RawOpenHow = how.into();

    let fd = crate::error::convert_neg_ret(unsafe {
        libc::syscall(
            SYS_OPENAT,
            dirfd,
            path.as_ptr(),
            &mut raw_how as *mut RawOpenHow,
            std::mem::size_of::<RawOpenHow>(),
        )
    })?;


    Ok(fd as RawFd)
}

pub fn openat2_raw<P: AsRef<Path>>(
    dirfd: Option<RawFd>,
    path: P,
    how: &OpenHow,
) -> io::Result<RawFd> {
    let c_path = CString::new(OsStr::new(path.as_ref()).as_bytes())?;

    openat2_sys(dirfd, &c_path, how)
}

pub fn openat2<P: AsRef<Path>>(
    dirfd: Option<RawFd>,
    path: P,
    how: &OpenHow,
) -> io::Result<File> {
    let fd = openat2_raw(dirfd, path, how)?;

    Ok(unsafe { File::from_raw_fd(fd) })
}

const NULL_C_STR: [u8; 1] = [0];

pub fn has_openat2() -> bool {
    loop {
        // The null pointer and size of 0 should give us an EINVAL if the syscall
        // is not present.
        let fd = unsafe {
            libc::syscall(
                SYS_OPENAT,
                -1,
                NULL_C_STR.as_ptr(),
                std::ptr::null_mut::<RawOpenHow>(),
                0,
            )
        };

        debug_assert!(fd < 0);

        let errno = io::Error::last_os_error().raw_os_error().unwrap();

        // Retry on EINTR
        if errno != libc::EINTR {
            return errno != libc::ENOSYS;
        }
    }
}

pub fn openat2_supports(how: &OpenHow) -> bool {
    loop {
        match openat2_sys(
            Some(-1),
            unsafe { CStr::from_bytes_with_nul_unchecked(&NULL_C_STR) },
            how,
        ) {
            // Retry on EINTR
            Err(e) if crate::error::is_eintr(&e) => (),
            // ENOENT means the arguments are valid, but it's complaining about the
            // empty string
            Err(e) if crate::error::is_raw(&e, libc::ENOENT) => return true,
            // This should NEVER happen
            Ok(_) => panic!(),
            // Interpret all other error cases as indicating the given arguments
            // are invalid or not supported by the kernel
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_how() {
        let mut how = OpenHow::new(libc::O_RDONLY);
        assert_eq!(how, OpenHow {
            flags: libc::O_RDONLY,
            mode: None,
            resolve_flags: ResolveFlags::empty(),
        });

        // The main purpose here is to test the handling of the 'mode' value.

        assert_eq!(RawOpenHow::from(&how), RawOpenHow {
            flags: (libc::O_RDONLY | libc::O_CLOEXEC) as u64,
            mode: 0,
            resolve: 0,
        });

        how.mode = Some(0o700);
        assert_eq!(RawOpenHow::from(&how), RawOpenHow {
            flags: (libc::O_RDONLY | libc::O_CLOEXEC) as u64,
            mode: 0o700,
            resolve: 0,
        });

        how.mode = None;
        how.flags = libc::O_WRONLY | libc::O_CREAT;
        assert_eq!(RawOpenHow::from(&how), RawOpenHow {
            flags: (libc::O_WRONLY | libc::O_CREAT | libc::O_CLOEXEC) as u64,
            mode: 0o777,
            resolve: 0,
        });

        how.mode = Some(0o700);
        assert_eq!(RawOpenHow::from(&how), RawOpenHow {
            flags: (libc::O_WRONLY | libc::O_CREAT | libc::O_CLOEXEC) as u64,
            mode: 0o700,
            resolve: 0,
        });
    }

    #[test]
    fn test_openat2() {
        if has_openat2() {
            test_openat2_present();
        } else {
            test_openat2_absent();
        }
    }

    fn test_openat2_present() {
        assert!(openat2_supports(&OpenHow::new(libc::O_RDONLY)));

        openat2(None, "/", &OpenHow::new(libc::O_RDONLY)).unwrap();
    }

    fn test_openat2_absent() {
        assert!(!openat2_supports(&OpenHow::new(libc::O_RDONLY)));
        assert_eq!(
            openat2(None, "/", &OpenHow::new(libc::O_RDONLY)).unwrap_err().raw_os_error(),
            Some(libc::ENOSYS),
        );
    }
}
