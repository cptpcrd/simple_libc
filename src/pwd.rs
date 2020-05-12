use std::ffi;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::sync;

use lazy_static::lazy_static;

use super::{Char, GidT, Int, UidT};

#[derive(Debug, Clone)]
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
    pub fn list() -> io::Result<Vec<Self>> {
        let _lock = PASSWD_LIST_MUTEX.lock();

        unsafe {
            libc::setpwent();
        }

        let mut passwds: Vec<Self> = Vec::new();

        loop {
            super::error::set_errno_success();
            let passwd: *mut libc::passwd = unsafe { libc::getpwent() };
            if passwd.is_null() {
                break;
            }

            passwds.push(Self::parse(unsafe { *passwd }));
        }

        let err = super::error::result_or_os_error(()).err();

        unsafe {
            libc::endpwent();
        }

        match err {
            Some(e) => Err(e),
            None => Ok(passwds),
        }
    }

    fn lookup<T, F>(t: &T, getpwfunc: F) -> io::Result<Option<Self>>
    where
        T: Sized,
        F: Fn(
            &T,
            *mut libc::passwd,
            *mut libc::c_char,
            libc::size_t,
            *mut *mut libc::passwd,
        ) -> Int,
    {
        let mut passwd: libc::passwd = unsafe { std::mem::zeroed() };

        let init_size = super::constrain(
            super::sysconf(libc::_SC_GETPW_R_SIZE_MAX).unwrap_or(1024),
            256,
            4096,
        ) as usize;

        let mut buffer: Vec<Char> = Vec::new();

        let mut result: *mut libc::passwd = std::ptr::null_mut();

        super::error::while_erange(
            |i| {
                let buflen: usize = (i as usize + 1) * init_size;

                buffer.resize(buflen, 0);

                let ret = getpwfunc(&t, &mut passwd, buffer.as_mut_ptr(), buflen, &mut result);

                super::error::convert_nzero_ret(ret).and_then(|_| {
                    if result.is_null() {
                        return Ok(None);
                    }

                    Ok(Some(Self::parse(passwd)))
                })
            },
            5,
        )
    }

    fn parse(passwd: libc::passwd) -> Self {
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
            &name,
            |name: &&str,
             pwd: *mut libc::passwd,
             buf: *mut libc::c_char,
             buflen: libc::size_t,
             result: *mut *mut libc::passwd| {
                unsafe {
                    let c_name = ffi::CString::from_vec_unchecked(Vec::from(*name));
                    libc::getpwnam_r(c_name.as_ptr(), pwd, buf, buflen, result)
                }
            },
        )
    }

    pub fn lookup_uid(uid: UidT) -> io::Result<Option<Self>> {
        Self::lookup(
            &uid,
            |uid: &UidT,
             pwd: *mut libc::passwd,
             buf: *mut libc::c_char,
             buflen: libc::size_t,
             result: *mut *mut libc::passwd| {
                unsafe { libc::getpwuid_r(*uid, pwd, buf, buflen, result) }
            },
        )
    }

    pub fn list_groups(&self) -> io::Result<Vec<super::grp::Group>> {
        let mut groups = super::grp::Group::list()?;

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
