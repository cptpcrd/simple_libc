use std::io;
use libc;


pub fn set_errno_success() {
    unsafe {
        *libc::__errno_location() = 0;
    }
}

pub fn convert_ret<T>(ret: T) -> io::Result<T> where
    T: From<i32> + Eq + Copy {
    convert(ret, ret)
}

pub fn convert<T, U>(ret: T, res: U) -> io::Result<U> where
    T: From<i32> + Eq {
    if ret == T::from(-1) {
        return Err(io::Error::last_os_error());
    }

    Ok(res)
}

pub fn convert_if_errno_ret<T>(ret: T) -> io::Result<T> where
    T: From<i32> + Eq + Copy {
    convert_if_errno(ret, ret)
}

pub fn convert_if_errno<T, U>(ret: T, res: U) -> io::Result<U> where
    T: From<i32> + Eq {
    if ret == T::from(-1) {
        return result_or_os_error(res)
    }

    Ok(res)
}


pub fn convert_neg_ret<T>(ret: T) -> io::Result<T> where
    T: Default + Ord + Copy {
    convert_neg(ret, ret)
}

pub fn convert_neg<T, U>(ret: T, res: U) -> io::Result<U> where
    T: Default + Ord {
    if ret < T::default() {
        return Err(io::Error::last_os_error());
    }

    Ok(res)
}


pub fn convert_nzero_ret<T>(ret: T) -> io::Result<T> where
    T: Default + Eq + Copy {
    convert_nzero(ret, ret)
}

pub fn convert_nzero<T, U>(ret: T, res: U) -> io::Result<U> where
    T: Default + Eq {
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
    return false;
}

pub fn is_eintr(err: &io::Error) -> bool {
    if let Some(libc::EINTR) = err.raw_os_error() {
        return true;
    }
    return false;
}

pub fn is_eagain(err: &io::Error) -> bool {
    if let Some(libc::EAGAIN) = err.raw_os_error() {
        return true;
    }
    return false;
}

pub fn is_einval(err: &io::Error) -> bool {
    if let Some(libc::EINVAL) = err.raw_os_error() {
        return true;
    }
    return false;
}

pub fn is_ewouldblock(err: &io::Error) -> bool {
    if let Some(libc::EWOULDBLOCK) = err.raw_os_error() {
        return true;
    }
    return false;
}

pub fn while_erange<F: FnMut(i32) -> io::Result<T>, T>(mut callback: F, max_n: i32) -> io::Result<T> {
    let mut i: i32 = 0;

    loop {
        match callback(i) {
            Ok(t) => return Ok(t),
            Err(e) => {
                if i >= max_n || !is_erange(&e) {
                    return Err(e);
                }
            },
        };

        i += 1;
    }
}
