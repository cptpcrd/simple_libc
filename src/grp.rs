use std::ffi;
use std::io;
use std::io::BufRead;
use std::os::unix::prelude::*;
use std::str::FromStr;

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
    static ref GROUP_LIST_MUTEX: std::sync::Mutex<i8> = std::sync::Mutex::new(0);
}

impl Group {
    /// List all the system group entries.
    ///
    /// This function simply locks a global lock and calls `list_single_thread()`. It
    /// is deprecated because it is impossible to confirm that this lock guarantees no
    /// conflicting function calls (for example, another library could make a call to
    /// a C function that calls `setgrent()`, or to `setgrent()` itself).
    #[deprecated(
        since = "0.5.0",
        note = "Use list_single_thread() and lock manually instead"
    )]
    pub fn list() -> io::Result<Vec<Self>> {
        let _lock = GROUP_LIST_MUTEX.lock();

        unsafe { Self::list_single_thread() }
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
    ///    `getgrent()`, `getgrent_r()`, `endgrent()`, `getgrgid()`, `getgrnam()`
    ///    (or C functions that call them).
    /// 4. Calling `pwd::Passwd::list_groups_single_thread()`.
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
    ///    `getgrent()`, `getgrent_r()`, `endgrent()`, `getgrgid()`, `getgrnam()`
    ///    (or C functions that call them).
    /// 4. Calling `pwd::Passwd::list_groups_single_thread()`.
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

    pub fn list_from_reader<R: io::Read>(reader: R) -> io::Result<Vec<Self>> {
        let mut reader = io::BufReader::new(reader);
        let mut line_vec = Vec::new();
        let mut groups = Vec::new();

        loop {
            if reader.read_until(b'\n', &mut line_vec)? == 0 {
                return Ok(groups);
            }

            if line_vec[line_vec.len() - 1] == b'\n' {
                line_vec.pop();
            }

            let mut it = line_vec.split(|c| *c == b':');

            let name_slice = it.next().unwrap_or(&[]);
            let passwd_slice = it.next().unwrap_or(&[]);
            let gid = Self::parse_str_from_bytes(it.next().unwrap_or(&[]))?;
            let member_slice = it.next().unwrap_or(&[]);

            if it.next() != None {
                return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
            }

            let mut members = Vec::new();
            for slice in member_slice.split(|c| *c == b',') {
                members.push(ffi::OsString::from_vec(slice.into()));
            }

            groups.push(Self {
                name: ffi::OsString::from_vec(name_slice.into()),
                passwd: ffi::OsString::from_vec(passwd_slice.into()),
                gid,
                members,
            });

            line_vec.clear();
        }
    }

    fn parse_str_from_bytes<T: FromStr>(bytes: &[u8]) -> io::Result<T> {
        if let Some(s) = ffi::OsStr::from_bytes(bytes).to_str() {
            if let Ok(val) = s.parse() {
                return Ok(val);
            }
        }

        Err(std::io::Error::from_raw_os_error(libc::EINVAL))
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

        let mut group = unsafe { std::mem::zeroed() };
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
        let mut parsed_members = Vec::new();

        for i in 0.. {
            let member = *group.gr_mem.offset(i);
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
            |grp: *mut libc::group, buf: &mut [libc::c_char], result: *mut *mut libc::group| unsafe {
                let c_name = ffi::CString::from_vec_unchecked(Vec::from(name));
                libc::getgrnam_r(c_name.as_ptr(), grp, buf.as_mut_ptr(), buf.len(), result)
            },
        )
    }

    pub fn lookup_gid(gid: GidT) -> io::Result<Option<Self>> {
        Self::lookup(
            |grp: *mut libc::group, buf: &mut [libc::c_char], result: *mut *mut libc::group| unsafe {
                libc::getgrgid_r(gid, grp, buf.as_mut_ptr(), buf.len(), result)
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

        #[cfg(not(any(target_os = "openbsd", target_os = "macos", target_env = "musl")))]
        {
            let result = Group::lookup(
                |grp: *mut libc::group, buf: &mut [libc::c_char], result: *mut *mut libc::group| unsafe {
                    libc::getgrent_r(grp, buf.as_mut_ptr(), buf.len() as libc::size_t, result)
                },
            );

            match result {
                Ok(grp) => grp,
                Err(err) => {
                    self.errno = err.raw_os_error().unwrap_or(libc::EINVAL);
                    None
                }
            }
        }

        // OpenBSD, macOS, and musl libc don't have getgrent_r()
        #[cfg(any(target_os = "openbsd", target_os = "macos", target_env = "musl"))]
        {
            crate::error::set_errno_success();

            let grp = unsafe { libc::getgrent() };

            if let Some(grp) = unsafe { grp.as_ref() } {
                Some(unsafe { Group::parse(grp) })
            } else {
                let errno = io::Error::last_os_error()
                    .raw_os_error()
                    .unwrap_or(libc::EINVAL);

                if errno == 0 {
                    self.errno = libc::ENOENT;
                } else {
                    self.errno = errno;
                }

                None
            }
        }
    }
}

impl Drop for GroupIter {
    fn drop(&mut self) {
        unsafe {
            libc::endgrent();
        }
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

    #[test]
    fn test_list_from_reader() {
        assert_eq!(
            Group::list_from_reader(b"grp:pwd:1:u1,u2".as_ref()).unwrap(),
            vec![Group {
                name: ffi::OsString::from("grp"),
                passwd: ffi::OsString::from("pwd"),
                gid: 1,
                members: vec![ffi::OsString::from("u1"), ffi::OsString::from("u2")],
            }],
        );

        assert_eq!(
            Group::list_from_reader(b"grp:pwd:1:u1,u2\n".as_ref()).unwrap(),
            vec![Group {
                name: ffi::OsString::from("grp"),
                passwd: ffi::OsString::from("pwd"),
                gid: 1,
                members: vec![ffi::OsString::from("u1"), ffi::OsString::from("u2")],
            }],
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_parse_str_from_bytes() {
        assert_eq!(
            Group::parse_str_from_bytes::<i32>(b"1".as_ref()).unwrap(),
            1,
        );
        assert_eq!(
            Group::parse_str_from_bytes::<i32>(b"-1".as_ref()).unwrap(),
            -1,
        );
        assert_eq!(
            Group::parse_str_from_bytes::<f32>(b"0.0".as_ref()).unwrap(),
            0.0,
        );

        assert_eq!(
            Group::parse_str_from_bytes::<i32>(b"".as_ref())
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EINVAL),
        );
        assert_eq!(
            Group::parse_str_from_bytes::<i32>(b"a".as_ref())
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EINVAL),
        );
        assert_eq!(
            Group::parse_str_from_bytes::<i32>(b"1a".as_ref())
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EINVAL),
        );
        assert_eq!(
            Group::parse_str_from_bytes::<i32>(b"1.".as_ref())
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EINVAL),
        );
    }
}
