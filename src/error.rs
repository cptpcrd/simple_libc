use std::io;

use crate::internal::{minus_one_signed, MinusOneSigned};
use crate::Int;

#[cfg(target_os = "linux")]
#[inline]
unsafe fn errno_mut_ptr() -> *mut Int {
    libc::__errno_location()
}

#[cfg(any(target_os = "freebsd", target_os = "dragonfly", target_os = "macos"))]
#[inline]
unsafe fn errno_mut_ptr() -> *mut Int {
    libc::__error()
}

#[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
#[inline]
unsafe fn errno_mut_ptr() -> *mut Int {
    libc::__errno()
}

pub fn set_errno_success() {
    unsafe {
        *errno_mut_ptr() = 0;
    }
}

#[inline]
pub fn convert_ret<T>(ret: T) -> io::Result<T>
where
    T: MinusOneSigned + Eq + Copy,
{
    convert(ret, ret)
}

pub fn convert<T, U>(ret: T, res: U) -> io::Result<U>
where
    T: MinusOneSigned + Eq,
{
    if ret == minus_one_signed() {
        Err(io::Error::last_os_error())
    } else {
        Ok(res)
    }
}

#[inline]
pub fn convert_if_errno_ret<T>(ret: T) -> io::Result<T>
where
    T: MinusOneSigned + Eq + Copy,
{
    convert_if_errno(ret, ret)
}

pub fn convert_if_errno<T, U>(ret: T, res: U) -> io::Result<U>
where
    T: MinusOneSigned + Eq,
{
    if ret == minus_one_signed() {
        result_or_os_error(res)
    } else {
        Ok(res)
    }
}

#[inline]
pub fn convert_neg_ret<T>(ret: T) -> io::Result<T>
where
    T: Default + Ord + Copy,
{
    convert_neg(ret, ret)
}

pub fn convert_neg<T, U>(ret: T, res: U) -> io::Result<U>
where
    T: Default + Ord,
{
    if ret < T::default() {
        Err(io::Error::last_os_error())
    } else {
        Ok(res)
    }
}

#[inline]
pub fn convert_nzero_ret<T>(ret: T) -> io::Result<()>
where
    T: Default + Eq,
{
    convert_nzero(ret, ())
}

pub fn convert_nzero<T, U>(ret: T, res: U) -> io::Result<U>
where
    T: Default + Eq,
{
    if ret != T::default() {
        Err(io::Error::last_os_error())
    } else {
        Ok(res)
    }
}

pub fn result_or_os_error<T>(res: T) -> io::Result<T> {
    let err = io::Error::last_os_error();

    // Success
    if let Some(0) = err.raw_os_error() {
        Ok(res)
    } else {
        Err(err)
    }
}

pub fn is_erange(err: &io::Error) -> bool {
    err.raw_os_error() == Some(libc::ERANGE)
}

pub fn is_eintr(err: &io::Error) -> bool {
    err.raw_os_error() == Some(libc::EINTR)
}

pub fn is_eagain(err: &io::Error) -> bool {
    err.raw_os_error() == Some(libc::EAGAIN)
}

pub fn is_einval(err: &io::Error) -> bool {
    err.raw_os_error() == Some(libc::EINVAL)
}

pub fn is_ewouldblock(err: &io::Error) -> bool {
    err.raw_os_error() == Some(libc::EWOULDBLOCK)
}

#[deprecated(since = "0.5.0", note = "Please loop manually instead")]
pub fn while_erange<F: FnMut(i32) -> io::Result<T>, T>(
    mut callback: F,
    max_n: i32,
) -> io::Result<T> {
    let mut i = 0;

    loop {
        match callback(i) {
            Ok(t) => return Ok(t),
            Err(e) => {
                if i >= max_n || !is_erange(&e) {
                    return Err(e);
                }
            }
        };

        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert() {
        assert_eq!(convert(-2, 19).unwrap(), 19);
        assert_eq!(
            convert(-1, ()).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );
        assert_eq!(convert(0, 19).unwrap(), 19);
        assert_eq!(convert(1, 19).unwrap(), 19);

        assert_eq!(convert_ret(-2).unwrap(), -2);
        assert_eq!(
            convert_ret(-1).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );
        assert_eq!(convert_ret(0).unwrap(), 0);
        assert_eq!(convert_ret(1).unwrap(), 1);
    }

    #[test]
    fn test_convert_neg() {
        assert_eq!(
            convert_neg(-1, ()).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );
        assert_eq!(
            convert_neg(-2, ()).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );
        assert_eq!(convert_neg(0, 19).unwrap(), 19);
        assert_eq!(convert_neg(1, 19).unwrap(), 19);

        assert_eq!(
            convert_neg_ret(-1).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );
        assert_eq!(
            convert_neg_ret(-2).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );
        assert_eq!(convert_neg_ret(0).unwrap(), 0);
        assert_eq!(convert_neg_ret(1).unwrap(), 1);
    }

    #[test]
    fn test_convert_nzero() {
        assert_eq!(
            convert_nzero(-1, ()).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );
        assert_eq!(
            convert_nzero(-2, ()).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );
        assert_eq!(convert_nzero(0, 19).unwrap(), 19);
        assert_eq!(
            convert_nzero(1, ()).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );

        assert_eq!(
            convert_nzero_ret(-1).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );
        assert_eq!(
            convert_nzero_ret(-2).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );
        convert_nzero_ret(0).unwrap();
        assert_eq!(
            convert_nzero_ret(1).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );
    }

    #[test]
    fn test_convert_if_errno() {
        set_errno_success();

        assert_eq!(convert_if_errno_ret(-1).unwrap(), -1);
        convert_if_errno(-1, ()).unwrap();

        unsafe {
            *errno_mut_ptr() = libc::EINVAL;
        }

        assert_eq!(
            convert_if_errno_ret(-1).unwrap_err().raw_os_error(),
            Some(libc::EINVAL),
        );
        assert_eq!(
            convert_if_errno(-1, ()).unwrap_err().raw_os_error(),
            Some(libc::EINVAL),
        );

        set_errno_success();
    }

    #[test]
    fn test_is_e() {
        assert!(!is_erange(&io::Error::from_raw_os_error(0)));
        assert!(!is_erange(&io::Error::from_raw_os_error(libc::EINVAL)));
        assert!(is_erange(&io::Error::from_raw_os_error(libc::ERANGE)));

        assert!(!is_einval(&io::Error::from_raw_os_error(0)));
        assert!(!is_einval(&io::Error::from_raw_os_error(libc::ERANGE)));
        assert!(is_einval(&io::Error::from_raw_os_error(libc::EINVAL)));

        assert!(!is_eagain(&io::Error::from_raw_os_error(0)));
        assert!(!is_eagain(&io::Error::from_raw_os_error(libc::ERANGE)));
        assert!(is_eagain(&io::Error::from_raw_os_error(libc::EAGAIN)));

        assert!(!is_ewouldblock(&io::Error::from_raw_os_error(0)));
        assert!(!is_ewouldblock(&io::Error::from_raw_os_error(libc::ERANGE)));
        assert!(is_ewouldblock(&io::Error::from_raw_os_error(
            libc::EWOULDBLOCK
        )));

        assert!(!is_eintr(&io::Error::from_raw_os_error(0)));
        assert!(!is_eintr(&io::Error::from_raw_os_error(libc::ERANGE)));
        assert!(is_eintr(&io::Error::from_raw_os_error(libc::EINTR)));
    }

    #[test]
    #[allow(deprecated)]
    fn test_while_erange() {
        while_erange(
            |i| match i {
                0 => Err(io::Error::from_raw_os_error(libc::ERANGE)),
                1 => Ok(()),
                _ => panic!(),
            },
            10,
        )
        .unwrap();

        while_erange(
            |i| match i {
                0 => Err(io::Error::from_raw_os_error(libc::ERANGE)),
                1 => Ok(()),
                _ => panic!(),
            },
            10,
        )
        .unwrap();

        assert_eq!(
            while_erange(
                |i| -> io::Result<()> {
                    match i {
                        0 => Err(io::Error::from_raw_os_error(libc::ERANGE)),
                        1 => Err(io::Error::from_raw_os_error(libc::ERANGE)),
                        _ => panic!(),
                    }
                },
                1
            )
            .unwrap_err()
            .raw_os_error(),
            Some(libc::ERANGE),
        );
    }

    #[test]
    fn test_set_errno_success() {
        set_errno_success();
    }
}
