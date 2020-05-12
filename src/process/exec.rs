use std::ffi;
use std::io;

use libc;

use super::super::Char;

fn build_c_string_vec<U: Into<Vec<u8>> + Clone + Sized>(vals: &[U]) -> io::Result<Vec<*mut Char>> {
    let mut c_vals: Vec<*mut Char> = Vec::with_capacity(vals.len() + 1);

    for val in vals {
        c_vals.push(ffi::CString::new(val.clone())?.into_raw())
    }

    c_vals.push(std::ptr::null_mut());

    Ok(c_vals)
}

fn cleanup_c_string_vec(c_vals: Vec<*mut libc::c_char>) {
    for val in c_vals {
        if val != std::ptr::null_mut() {
            unsafe {
                let _ = ffi::CString::from_raw(val);
            }
        }
    }
}

/// Attempts to execute the given program with the given arguments, replacing the
/// current process. This variant of `exec` does not perform a `PATH` lookup, so
/// a full path should be specified.
///
/// If this function returns, it means an error occurred.
pub fn execv<U: Into<Vec<u8>> + Clone + Sized>(prog: &str, argv: &[U]) -> io::Result<()> {
    let c_prog = ffi::CString::new(prog)?;
    let c_argv = build_c_string_vec(argv)?;

    unsafe {
        libc::execv(c_prog.as_ptr(), c_argv.as_ptr() as *const *const Char);
    }

    cleanup_c_string_vec(c_argv);

    Err(io::Error::last_os_error().into())
}

/// Attempts to execute the given program with the given arguments and the given
/// environment, replacing the current process. This variant of `exec` does not
/// perform a `PATH` lookup, so a full path should be specified.
///
/// If this function returns, it means an error occurred.
pub fn execve<U: Into<Vec<u8>> + Clone + Sized, V: Into<Vec<u8>> + Clone + Sized>(
    prog: &str,
    argv: &[U],
    env: &[V],
) -> io::Result<()> {
    let c_prog = ffi::CString::new(prog)?;
    let c_argv = build_c_string_vec(argv)?;
    let c_env = build_c_string_vec(env)?;

    unsafe {
        libc::execve(
            c_prog.as_ptr(),
            c_argv.as_ptr() as *const *const Char,
            c_env.as_ptr() as *const *const Char,
        );
    }

    cleanup_c_string_vec(c_argv);
    cleanup_c_string_vec(c_env);

    Err(io::Error::last_os_error().into())
}

/// Attempts to execute the given program with the given arguments and the given
/// environment, replacing the current process. This variant of `exec` does not
/// perform a `PATH` lookup, so a full path should be specified.
///
/// This varient of `exec`, rather than accepting a path or a program name, accepts
/// a file descriptor specifying the program to be executed.
///
/// If this function returns, it means an error occurred.
#[cfg(target_os = "linux")]
pub fn fexecve<U: Into<Vec<u8>> + Clone + Sized, V: Into<Vec<u8>> + Clone + Sized>(
    fd: super::super::Int,
    argv: &[U],
    env: &[V],
) -> io::Result<()> {
    let c_argv = build_c_string_vec(argv)?;
    let c_env = build_c_string_vec(env)?;

    unsafe {
        libc::fexecve(
            fd,
            c_argv.as_ptr() as *const *const Char,
            c_env.as_ptr() as *const *const Char,
        );
    }

    cleanup_c_string_vec(c_argv);
    cleanup_c_string_vec(c_env);

    Err(io::Error::last_os_error().into())
}

/// Attempts to execute the given program with the given arguments, replacing the
/// current process. This variant of `exec` performs a `PATH` lookup, so specifying
/// a full path is not necessary.
///
/// If this function returns, it means an error occurred.
pub fn execvp<U: Into<Vec<u8>> + Clone + Sized>(prog: &str, argv: &[U]) -> io::Result<()> {
    let c_prog = ffi::CString::new(prog)?;
    let c_argv = build_c_string_vec(argv)?;

    unsafe {
        libc::execvp(c_prog.as_ptr(), c_argv.as_ptr() as *const *const Char);
    }

    cleanup_c_string_vec(c_argv);

    Err(io::Error::last_os_error().into())
}
