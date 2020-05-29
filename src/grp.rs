use std::ffi;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::sync;

use lazy_static::lazy_static;

use crate::{GidT, Int};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Group {
    pub name: ffi::OsString,
    pub passwd: ffi::OsString,
    pub gid: GidT,
    pub members: Vec<ffi::OsString>,
}

lazy_static! {
    static ref GROUP_LIST_MUTEX: sync::Mutex<i8> = sync::Mutex::new(0);
}

impl Group {
    /// List all the system group entries.
    ///
    /// This function simply locks a global lock and calls `list_single_thread()`. It
    /// is deprecated because it is impossible to confirm that this lock guarantees no
    /// conflicting function calls (for example, another library could make a call to
    /// a C function that calls `setgrent()`, or to `setgrent()` itself).
    #[deprecated(since = "0.5.0", note = "Use list_single_thread() and lock manually instead")]
    pub fn list() -> io::Result<Vec<Self>> {
        let _lock = GROUP_LIST_MUTEX.lock();

        unsafe {
            Self::list_single_thread()
        }
    }

    /// List all the system group entries.
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
    /// 3. Making calls to any of the following C functions: `setgrent()`,
    ///    `getgrent()`, `getgrent_r()`, `endgrent()` (or C functions that call
    ///    them).
    pub unsafe fn list_single_thread() -> io::Result<Vec<Self>> {
        let groups;
        let err;

        // Only hold onto the reference for as long as we have to
        {
            let mut group_iter = Self::iter_single_thread_dangerous();
            groups = group_iter.by_ref().collect();
            err = group_iter.get_error();
        }

        match err {
            Some(e) => Err(e),
            None => Ok(groups),
        }
    }

    /// Create an iterator over the system group entries.
    ///
    /// **WARNING: The return value of this function is difficult to use properly.
    /// For most cases, you should call `list_single_thread()`, which collects
    /// the results and returns an `std::io::Result<Vec<Group>>`.**
    ///
    /// # Safety
    ///
    /// This function is ONLY safe if, from the time this function is called to
    /// the time that the returned value is dropped, NONE of the following actions
    /// are performed, either by another thread or by ordinary code:
    ///
    /// 1. Calling `list_single_thread()`.
    /// 2. Calling this function. (In other words, it is only safe to have ONE
    ///    `GroupIter` in existence at any given time.)
    /// 3. Making calls to any of the following C functions: `setgrent()`,
    ///    `getgrent()`, `getgrent_r()`, `endgrent()` (or C functions that call
    ///    them).
    ///
    /// Note: To help ensure safety, the value MUST be dropped as soon as it is
    /// no longer used! Exhausting the iterator is NOT enough (`endgrent()`
    /// only called in `drop()`).
    ///
    /// Here is an example of recommended usage:
    ///
    /// ```
    /// use simple_libc::grp::Group;
    ///
    /// let err;
    /// unsafe {
    ///     let mut group_iter = Group::iter_single_thread_dangerous();
    ///     for group in &mut group_iter {
    ///         // Process group
    ///     }
    ///
    ///     // Extract the error
    ///     err = group_iter.get_error();
    /// }
    ///
    /// // *After* dropping the GroupIter, check the value of err
    /// assert!(err.is_none());
    /// ```
    #[inline]
    pub unsafe fn iter_single_thread_dangerous() -> GroupIter {
        GroupIter::new()
    }

    fn lookup<F>(getgrfunc: F) -> io::Result<Option<Self>>
    where
        F: Fn(*mut libc::group, &mut [libc::c_char], *mut *mut libc::group) -> Int,
    {
        // Initial buffer size
        let init_size = crate::constrain(
            crate::sysconf(libc::_SC_GETGR_R_SIZE_MAX).unwrap_or(1024),
            256,
            4096,
        ) as usize;
        // Maximum buffer size
        let max_size = 32768;

        let mut buffer = Vec::new();
        buffer.resize(init_size, 0);

        let mut group: libc::group = unsafe { std::mem::zeroed() };
        let mut result = std::ptr::null_mut();

        loop {
            let errno = getgrfunc(&mut group, &mut buffer, &mut result);

            if errno == libc::ERANGE && buffer.len() < max_size {
                // The buffer's too small and we're under the limit; let's enlarge it.
                buffer.resize(buffer.len() * 2, 0);
            } else if errno != 0 {
                return Err(io::Error::from_raw_os_error(errno));
            } else if result.is_null() {
                return Ok(None);
            } else {
                return Ok(Some(unsafe { Self::parse(&group) }));
            }
        }
    }

    unsafe fn parse(group: &libc::group) -> Self {
        let mut parsed_members: Vec<ffi::OsString> = Vec::new();

        for i in 0.. {
            let member: *mut libc::c_char = *group.gr_mem.offset(i);
            if member.is_null() {
                break;
            }

            parsed_members.push(Self::from_c_str(member));
        }

        Self {
            gid: group.gr_gid,
            name: Self::from_c_str(group.gr_name),
            passwd: Self::from_c_str(group.gr_passwd),
            members: parsed_members,
        }
    }

    unsafe fn from_c_str(s: *const libc::c_char) -> ffi::OsString {
        ffi::OsString::from_vec(ffi::CStr::from_ptr(s).to_bytes().into())
    }

    pub fn lookup_name(name: &str) -> io::Result<Option<Self>> {
        Self::lookup(
            |grp: *mut libc::group,
             buf: &mut [libc::c_char],
             result: *mut *mut libc::group| {
                unsafe {
                    let c_name = ffi::CString::from_vec_unchecked(Vec::from(name));
                    libc::getgrnam_r(c_name.as_ptr(), grp, buf.as_mut_ptr(), buf.len(), result)
                }
            },
        )
    }

    pub fn lookup_gid(gid: GidT) -> io::Result<Option<Self>> {
        Self::lookup(
            |grp: *mut libc::group,
             buf: &mut [libc::c_char],
             result: *mut *mut libc::group| {
                unsafe { libc::getgrgid_r(gid, grp, buf.as_mut_ptr(), buf.len(), result) }
            },
        )
    }
}

/// An iterator over the system group entries.
///
/// The interface is inspired by the
/// `GroupIter` struct from the `pwd` crate.
pub struct GroupIter {
    errno: Int,
}

impl GroupIter {
    unsafe fn new() -> Self {
        libc::setgrent();

        Self { errno: 0 }
    }

    /// Returns the error, if any, that occurred while iterating over the system
    /// group entries.
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

impl Iterator for GroupIter {
    type Item = Group;

    fn next(&mut self) -> Option<Self::Item> {
        if self.errno != 0 {
            return None;
        }

        let result = Group::lookup(
            |pwd: *mut libc::group,
             buf: &mut [libc::c_char],
             result: *mut *mut libc::group| {
                 unsafe {
                     libc::getgrent_r(pwd, buf.as_mut_ptr(), buf.len() as libc::size_t, result)
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

impl Drop for GroupIter {
    fn drop(&mut self) {
        unsafe { libc::endgrent(); }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    
    use crate::pwd::Passwd;

    #[test]
    fn test_lookup_current_gid() {
        let group = Group::lookup_gid(crate::process::getgid())
            .unwrap()
            .unwrap();

        assert_eq!(
            group,
            Group::lookup_name(&group.name.to_string_lossy())
                .unwrap()
                .unwrap()
        );
    }

    #[test]
    fn test_list_iter() {
        // Since these are not thread-safe, they all need to be called
        // in the same test

        let groups = unsafe { Group::list_single_thread() }.unwrap();
        assert_ne!(groups, vec![]);

        #[allow(deprecated)]
        let groups2 = Group::list().unwrap();
        assert_eq!(groups, groups2);

        let err;
        unsafe {
            let mut group_iter = Group::iter_single_thread_dangerous();
            for (pwd_a, pwd_b) in (&mut group_iter).zip(groups) {
                assert_eq!(pwd_a, pwd_b);
            }

            // Make sure that repeated calls to `next()` return `None`
            assert_eq!(group_iter.next(), None);

            err = group_iter.get_error();
        }

        assert!(err.is_none());

        // Now test listing the current user's groups
        let passwd = Passwd::lookup_uid(crate::process::getuid())
            .unwrap()
            .unwrap();

        let user_groups = unsafe { passwd.list_groups_single_thread() }.unwrap();

        #[allow(deprecated)]
        let user_groups2 = passwd.list_groups().unwrap();

        assert_eq!(user_groups, user_groups2);
    }
}
