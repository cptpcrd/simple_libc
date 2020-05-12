use std::ffi;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::sync;

use lazy_static::lazy_static;

use super::{Char, Int};

#[derive(Debug, Clone)]
pub struct Group {
    pub name: ffi::OsString,
    pub passwd: ffi::OsString,
    pub gid: u32,
    pub members: Vec<ffi::OsString>,
}

lazy_static! {
    static ref GROUP_LIST_MUTEX: sync::Mutex<i8> = sync::Mutex::new(0);
}

impl Group {
    pub fn list() -> io::Result<Vec<Self>> {
        let _lock = GROUP_LIST_MUTEX.lock();

        unsafe {
            libc::setgrent();
        }

        let mut groups: Vec<Self> = Vec::new();

        loop {
            super::error::set_errno_success();
            let group: *mut libc::group = unsafe { libc::getgrent() };
            if group.is_null() {
                break;
            }

            groups.push(unsafe { Self::parse(*group) });
        }

        let err = super::error::result_or_os_error(()).err();

        unsafe {
            libc::endgrent();
        }

        match err {
            Some(e) => Err(e),
            None => Ok(groups),
        }
    }

    fn lookup<T, F>(t: &T, getgrfunc: F) -> io::Result<Option<Self>>
    where
        T: Sized,
        F: Fn(&T, *mut libc::group, *mut libc::c_char, libc::size_t, *mut *mut libc::group) -> Int,
    {
        let mut group: libc::group = unsafe { std::mem::zeroed() };

        let init_size = super::constrain(
            super::sysconf(libc::_SC_GETPW_R_SIZE_MAX).unwrap_or(1024),
            256,
            4096,
        ) as usize;

        let mut buffer: Vec<Char> = Vec::new();

        let mut result: *mut libc::group = std::ptr::null_mut();

        super::error::while_erange(
            |i| {
                let buflen: usize = (i as usize + 1) * init_size;

                buffer.resize(buflen, 0);

                let ret = getgrfunc(&t, &mut group, buffer.as_mut_ptr(), buflen, &mut result);

                super::error::convert_nzero_ret(ret).and_then(|_| {
                    if result.is_null() {
                        return Ok(None);
                    }

                    Ok(Some(unsafe { Self::parse(group) }))
                })
            },
            5,
        )
    }

    unsafe fn parse(group: libc::group) -> Self {
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
            &name,
            |name: &&str,
             grp: *mut libc::group,
             buf: *mut libc::c_char,
             buflen: libc::size_t,
             result: *mut *mut libc::group| {
                unsafe {
                    let c_name = ffi::CString::from_vec_unchecked(Vec::from(*name));
                    libc::getgrnam_r(c_name.as_ptr(), grp, buf, buflen, result)
                }
            },
        )
    }

    pub fn lookup_gid(gid: u32) -> io::Result<Option<Self>> {
        Self::lookup(
            &gid,
            |gid: &u32,
             grp: *mut libc::group,
             buf: *mut libc::c_char,
             buflen: libc::size_t,
             result: *mut *mut libc::group| {
                unsafe { libc::getgrgid_r(*gid, grp, buf, buflen, result) }
            },
        )
    }
}
