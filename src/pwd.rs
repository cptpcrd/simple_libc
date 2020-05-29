use std::ffi;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::sync;

use lazy_static::lazy_static;

use crate::{GidT, Int, UidT};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Passwd {
    pub name: ffi::OsString,
    pub passwd: ffi::OsString,
    pub uid: UidT,
    pub gid: GidT,
    pub gecos_info: ffi::OsString,
    pub home_dir: ffi::OsString,
    pub shell: ffi::OsString,
}

lazy_static! {
    static ref PASSWD_LIST_MUTEX: sync::Mutex<i8> = sync::Mutex::new(0);
}

impl Passwd {
    /// List all the system password entries.
    ///
    /// This function simply locks a global lock and calls `list_single_thread()`. It
    /// is deprecated because it is impossible to confirm that this lock guarantees no
    /// conflicting function calls (for example, another library could make a call to
    /// a C function that calls `setpwent()`, or to `setpwent()` itself).
    #[deprecated(since = "0.5.0", note = "Use list_single_thread() and lock manually instead")]
    pub fn list() -> io::Result<Vec<Self>> {
        let _lock = PASSWD_LIST_MUTEX.lock();

        unsafe {
            Self::list_single_thread()
        }
    }

    /// List all the system password entries.
    ///
    /// This calls `iter_single_thread()` and collects the yielded values.
    ///
    /// # Safety
    ///
    /// This function is safe if it can be proven that no other thread (or
    /// code such as a signal handler) is:
    ///
    /// 1. Also calling this function.
    /// 2. Interacting with the value returned by a call to `iter_single_thread()`
    ///    (see the "Safety" section in `iter_single_thread()`'s documentation).
    /// 3. Making calls to any of the following C functions: `setpwent()`,
    ///    `getpwent()`, `getpwent_r()`, `endpwent()` (or C functions that call
    ///    them).
    pub unsafe fn list_single_thread() -> io::Result<Vec<Self>> {
        let passwds;
        let err;

        // Only hold onto the reference for as long as we have to
        {
            let mut passwd_iter = Self::iter_single_thread_dangerous();
            passwds = passwd_iter.by_ref().collect();
            err = passwd_iter.get_error();
        }

        match err {
            Some(e) => Err(e),
            None => Ok(passwds),
        }
    }

    /// Create an iterator over the system password entries.
    ///
    /// **WARNING: The return value of this function is difficult to use properly.
    /// For most cases, you should call `list_single_thread()`, which collects
    /// the results and returns an `std::io::Result<Vec<Passwd>>`.**
    ///
    /// # Safety
    ///
    /// This function is ONLY safe if, from the time this function is called to
    /// the time that the returned value is dropped, NONE of the following actions
    /// are performed, either by another thread or by ordinary code:
    ///
    /// 1. Calling `list_single_thread()`.
    /// 2. Calling this function. (In other words, it is only safe to have ONE
    ///    `PasswdIter` in existence at any given time.)
    /// 3. Making calls to any of the following C functions: `setpwent()`,
    ///    `getpwent()`, `getpwent_r()`, `endpwent()` (or C functions that call
    ///    them).
    ///
    /// Note: To help ensure safety, the value MUST be dropped as soon as it is
    /// no longer used! Exhausting the iterator is NOT enough (`endpwent()` 
    /// only called in `drop()`).
    ///
    /// Here is an example of recommended usage:
    ///
    /// ```
    /// use simple_libc::pwd::Passwd;
    ///
    /// let err;
    /// unsafe {
    ///     let mut passwd_iter = Passwd::iter_single_thread_dangerous();
    ///     for passwd in &mut passwd_iter {
    ///         // Process passwd
    ///     }
    ///
    ///     // Extract the error
    ///     err = passwd_iter.get_error();
    /// }
    ///
    /// // *After* dropping the PasswdIter, check the value of err
    /// assert!(err.is_none());
    /// ```
    #[inline]
    pub unsafe fn iter_single_thread_dangerous() -> PasswdIter {
        PasswdIter::new()
    }

    fn lookup<F>(getpwfunc: F) -> io::Result<Option<Self>>
        where F: Fn(
            *mut libc::passwd,
            &mut [libc::c_char],
            *mut *mut libc::passwd,
        ) -> Int {
        // Initial buffer size
        let init_size = crate::constrain(
            crate::sysconf(libc::_SC_GETPW_R_SIZE_MAX).unwrap_or(1024),
            256,
            8192,
        ) as usize;
        // Maximum buffer size
        let max_size = 32768;

        let mut buffer = Vec::new();
        buffer.resize(init_size, 0);

        let mut passwd = unsafe { std::mem::zeroed() };
        let mut result = std::ptr::null_mut();

        loop {
            let errno = getpwfunc(&mut passwd, &mut buffer, &mut result);

            if errno == libc::ERANGE && buffer.len() < max_size {
                // The buffer's too small and we're under the limit; let's enlarge it.
                buffer.resize(buffer.len() * 2, 0);
            } else if errno != 0 {
                return Err(io::Error::from_raw_os_error(errno));
            } else if result.is_null() {
                return Ok(None);
            } else {
                return Ok(Some(Self::parse(&passwd)));
            }
        }
    }

    fn parse(passwd: &libc::passwd) -> Self {
        unsafe {
            Self {
                uid: passwd.pw_uid,
                gid: passwd.pw_gid,
                name: Self::from_c_str(passwd.pw_name),
                passwd: Self::from_c_str(passwd.pw_passwd),
                gecos_info: Self::from_c_str(passwd.pw_gecos),
                home_dir: Self::from_c_str(passwd.pw_dir),
                shell: Self::from_c_str(passwd.pw_shell),
            }
        }
    }

    unsafe fn from_c_str(s: *const libc::c_char) -> ffi::OsString {
        ffi::OsString::from_vec(ffi::CStr::from_ptr(s).to_bytes().into())
    }

    pub fn lookup_name(name: &str) -> io::Result<Option<Self>> {
        Self::lookup(
            |pwd: *mut libc::passwd,
             buf: &mut [libc::c_char],
             result: *mut *mut libc::passwd| {
                 unsafe {
                     let c_name = ffi::CString::from_vec_unchecked(Vec::from(name));
                     libc::getpwnam_r(c_name.as_ptr(), pwd, buf.as_mut_ptr(), buf.len() as libc::size_t, result)
                 }
            },
        )
    }

    pub fn lookup_uid(uid: UidT) -> io::Result<Option<Self>> {
        Self::lookup(
            |pwd: *mut libc::passwd,
             buf: &mut [libc::c_char],
             result: *mut *mut libc::passwd| {
                unsafe { libc::getpwuid_r(uid, pwd, buf.as_mut_ptr(), buf.len() as libc::size_t, result) }
            },
        )
    }

    #[deprecated(since = "0.5.0", note = "Use list_groups_single_thread() and lock manually instead")]
    pub fn list_groups(&self) -> io::Result<Vec<crate::grp::Group>> {
        #[allow(deprecated)]
        let mut groups = crate::grp::Group::list()?;

        groups.retain(|group| {
            for mem in &group.members {
                if mem == &self.name {
                    return true;
                }
            }

            false
        });

        Ok(groups)
    }

    pub unsafe fn list_groups_single_thread(&self) -> io::Result<Vec<crate::grp::Group>> {
        let mut groups = crate::grp::Group::list_single_thread()?;

        groups.retain(|group| {
            for mem in &group.members {
                if mem == &self.name {
                    return true;
                }
            }

            false
        });

        Ok(groups)
    }
}

/// An iterator over the system password entries.
///
/// The interface is inspired by the
/// `PasswdIter` struct from the `pwd` crate.
pub struct PasswdIter {
    errno: Int,
}

impl PasswdIter {
    unsafe fn new() -> Self {
        libc::setpwent();

        Self { errno: 0 }
    }

    /// Returns the error, if any, that occurred while iterating over the system
    /// password entries.
    ///
    /// This is only valid if the iterator has been exhausted.
    pub fn get_error(&self) -> Option<io::Error> {
        if self.errno == 0 || self.errno == libc::ENOENT {
            None
        } else {
            Some(io::Error::from_raw_os_error(self.errno))
        }
    }
}

impl Iterator for PasswdIter {
    type Item = Passwd;

    fn next(&mut self) -> Option<Self::Item> {
        if self.errno != 0 {
            return None;
        }

        let result = Passwd::lookup(
            |pwd: *mut libc::passwd,
             buf: &mut [libc::c_char],
             result: *mut *mut libc::passwd| {
                 unsafe {
                     libc::getpwent_r(pwd, buf.as_mut_ptr(), buf.len() as libc::size_t, result)
                 }
            },
        );

        match result {
            Ok(pwd) => pwd,
            Err(err) => {
                self.errno = err.raw_os_error().unwrap_or(libc::EINVAL);
                None
            }
        }
    }
}

impl Drop for PasswdIter {
    fn drop(&mut self) {
        unsafe { libc::endpwent(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_current_uid() {
        let passwd = Passwd::lookup_uid(crate::process::getuid())
            .unwrap()
            .unwrap();

        assert_eq!(
            passwd,
            Passwd::lookup_name(&passwd.name.to_string_lossy())
                .unwrap()
                .unwrap()
        );
    }

    #[test]
    fn test_list_iter() {
        // Since these are not thread-safe, they all need to be called
        // in the same test

        let passwds = unsafe { Passwd::list_single_thread() }.unwrap();
        assert_ne!(passwds, vec![]);

        #[allow(deprecated)]
        let passwds2 = Passwd::list().unwrap();
        assert_eq!(passwds, passwds2);

        let err;
        unsafe {
            let mut passwd_iter = Passwd::iter_single_thread_dangerous();
            for (pwd_a, pwd_b) in (&mut passwd_iter).zip(passwds) {
                assert_eq!(pwd_a, pwd_b);
            }

            // Make sure that repeated calls to `next()` return `None`
            assert_eq!(passwd_iter.next(), None);

            err = passwd_iter.get_error();
        }

        assert!(err.is_none());
    }
}
