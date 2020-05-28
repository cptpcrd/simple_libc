use std::io;

use crate::Int;

#[cfg(target_os = "linux")]
pub fn set_errno_success() {
    unsafe {
        *libc::__errno_location() = 0;
    }
}

#[cfg(any(target_os = "freebsd", target_os = "dragonfly", target_os = "macos"))]
pub fn set_errno_success() {
    unsafe {
        *libc::__error() = 0;
    }
}

#[cfg(any(target_os = "netbsd", target_os = "openbsd"))]
pub fn set_errno_success() {
    unsafe {
        *libc::__errno() = 0;
    }
}

#[inline]
pub fn convert_ret<T>(ret: T) -> io::Result<T>
where
    T: From<Int> + Eq + Copy,
{
    convert(ret, ret)
}

pub fn convert<T, U>(ret: T, res: U) -> io::Result<U>
where
    T: From<Int> + Eq,
{
    if ret == T::from(-1) {
        return Err(io::Error::last_os_error());
    }

    Ok(res)
}

#[inline]
pub fn convert_if_errno_ret<T>(ret: T) -> io::Result<T>
where
    T: From<Int> + Eq + Copy,
{
    convert_if_errno(ret, ret)
}

pub fn convert_if_errno<T, U>(ret: T, res: U) -> io::Result<U>
where
    T: From<Int> + Eq,
{
    if ret == T::from(-1) {
        return result_or_os_error(res);
    }

    Ok(res)
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
        return Err(io::Error::last_os_error());
    }

    Ok(res)
}

#[inline]
pub fn convert_nzero_ret<T>(ret: T) -> io::Result<T>
where
    T: Default + Eq + Copy,
{
    convert_nzero(ret, ret)
}

pub fn convert_nzero<T, U>(ret: T, res: U) -> io::Result<U>
where
    T: Default + Eq,
{
    if ret != T::default() {
        return Err(io::Error::last_os_error());
    }

    Ok(res)
}

pub fn result_or_os_error<T>(res: T) -> io::Result<T> {
    let err = io::Error::last_os_error();

    // Success
    if let Some(0) = err.raw_os_error() {
        return Ok(res);
    }

    Err(err)
}

pub fn is_erange(err: &io::Error) -> bool {
    if let Some(libc::ERANGE) = err.raw_os_error() {
        return true;
    }
    false
}

pub fn is_eintr(err: &io::Error) -> bool {
    if let Some(libc::EINTR) = err.raw_os_error() {
        return true;
    }
    false
}

pub fn is_eagain(err: &io::Error) -> bool {
    if let Some(libc::EAGAIN) = err.raw_os_error() {
        return true;
    }
    false
}

pub fn is_einval(err: &io::Error) -> bool {
    if let Some(libc::EINVAL) = err.raw_os_error() {
        return true;
    }
    false
}

pub fn is_ewouldblock(err: &io::Error) -> bool {
    if let Some(libc::EWOULDBLOCK) = err.raw_os_error() {
        return true;
    }
    false
}

pub fn while_erange<F: FnMut(i32) -> io::Result<T>, T>(
    mut callback: F,
    max_n: i32,
) -> io::Result<T> {
    let mut i: i32 = 0;

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
        assert_eq!(convert_nzero_ret(0).unwrap(), 0);
        assert_eq!(
            convert_nzero_ret(1).unwrap_err().raw_os_error(),
            io::Error::last_os_error().raw_os_error()
        );
    }

    #[test]
    fn test_set_errno_success() {
        set_errno_success();
    }
}
