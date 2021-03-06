use std::io;

use crate::internal::{minus_one_signed, MinusOneSigned};

#[cfg(target_os = "linux")]
use libc::__errno_location as errno_mut_ptr;

#[cfg(any(target_os = "freebsd", target_os = "dragonfly", target_os = "macos"))]
use libc::__error as errno_mut_ptr;

#[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
use libc::__errno as errno_mut_ptr;

#[inline]
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
    T: MinusOneSigned + Default + Ord + Copy,
{
    convert_neg(ret, ret)
}

pub fn convert_neg<T, U>(ret: T, res: U) -> io::Result<U>
where
    T: MinusOneSigned + Default + Ord,
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
    T: MinusOneSigned + Default + Eq,
{
    convert_nzero(ret, ())
}

pub fn convert_nzero<T, U>(ret: T, res: U) -> io::Result<U>
where
    T: MinusOneSigned + Default + Eq,
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

pub fn is_raw(err: &io::Error, num: i32) -> bool {
    err.raw_os_error() == Some(num)
}

#[inline]
pub fn is_erange(err: &io::Error) -> bool {
    is_raw(err, libc::ERANGE)
}

#[inline]
pub fn is_eintr(err: &io::Error) -> bool {
    is_raw(err, libc::EINTR)
}

#[inline]
pub fn is_eagain(err: &io::Error) -> bool {
    is_raw(err, libc::EAGAIN)
}

#[inline]
pub fn is_einval(err: &io::Error) -> bool {
    is_raw(err, libc::EINVAL)
}

#[inline]
pub fn is_ewouldblock(err: &io::Error) -> bool {
    is_raw(err, libc::EWOULDBLOCK)
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
        assert!(is_raw(&io::Error::from_raw_os_error(0), 0));
        assert!(!is_raw(&io::Error::from_raw_os_error(0), libc::EINVAL));
        assert!(!is_raw(&io::Error::from_raw_os_error(libc::EINVAL), 0));
        assert!(is_raw(
            &io::Error::from_raw_os_error(libc::EINVAL),
            libc::EINVAL
        ));

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
    fn test_set_errno_success() {
        set_errno_success();
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(0));
    }
}
