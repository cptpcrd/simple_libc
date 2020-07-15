use std::io;
use std::time::{Duration, SystemTime};

#[cfg(not(target_os = "netbsd"))]
fn clock_gettime(clockid: libc::clockid_t) -> io::Result<Duration> {
    let mut timespec = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    crate::error::convert_nzero_ret(unsafe { libc::clock_gettime(clockid, &mut timespec) })?;

    Ok(Duration::new(
        timespec.tv_sec as u64,
        timespec.tv_nsec as u32,
    ))
}

/// Returns the time when the sysetem was booted.
#[allow(clippy::needless_return)]
pub fn get_boot_time() -> io::Result<SystemTime> {
    #[cfg(any(target_os = "openbsd", target_os = "freebsd", target_os = "macos"))]
    {
        let mut timeval = libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };

        let timeval_size = unsafe {
            crate::sysctl_raw(
                &[libc::CTL_KERN, libc::KERN_BOOTTIME],
                Some(std::slice::from_mut(&mut timeval)),
                None,
            )
        }?;

        if timeval_size != std::mem::size_of::<libc::timeval>() {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }

        return Ok(SystemTime::UNIX_EPOCH
            + Duration::new(timeval.tv_sec as u64, timeval.tv_usec as u32 * 1000));
    }

    #[cfg(any(target_os = "netbsd", target_os = "dragonfly"))]
    {
        let mut timespec = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        let timespec_size = unsafe {
            crate::sysctl_raw(
                &[libc::CTL_KERN, libc::KERN_BOOTTIME],
                Some(std::slice::from_mut(&mut timespec)),
                None,
            )
        }?;

        if timespec_size != std::mem::size_of::<libc::timespec>() {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }

        return Ok(
            SystemTime::UNIX_EPOCH + Duration::new(timespec.tv_sec as u64, timespec.tv_nsec as u32)
        );
    }

    #[cfg(target_os = "linux")]
    return Ok(SystemTime::now() - get_time_since_boot()?);
}

/// Returns the time elapsed since the system was booted, counting when
/// the system is suspended.
#[allow(clippy::needless_return)]
pub fn get_time_since_boot() -> io::Result<Duration> {
    #[cfg(target_os = "linux")]
    return clock_gettime(libc::CLOCK_BOOTTIME);

    #[cfg(target_os = "openbsd")]
    return clock_gettime(crate::constants::CLOCK_BOOTTIME);

    #[cfg(any(
        target_os = "netbsd",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "macos",
    ))]
    return match SystemTime::now().duration_since(get_boot_time()?) {
        Ok(d) => Ok(d),
        Err(_) => Err(io::Error::from_raw_os_error(libc::EAGAIN)),
    };
}

/// Returns the time that the system has been up, not counting when
/// the system is suspended.
#[allow(clippy::needless_return)]
pub fn get_active_uptime() -> io::Result<Duration> {
    #[cfg(target_os = "linux")]
    return clock_gettime(libc::CLOCK_MONOTONIC);

    #[cfg(target_os = "openbsd")]
    return clock_gettime(crate::constants::CLOCK_UPTIME);

    #[cfg(any(target_os = "freebsd", target_os = "dragonfly"))]
    return clock_gettime(libc::CLOCK_UPTIME);

    #[cfg(target_os = "macos")]
    return clock_gettime(crate::constants::CLOCK_UPTIME_RAW);

    // It does not appear this is possible to get on NetBSD.
    #[cfg(target_os = "netbsd")]
    return Err(io::Error::from_raw_os_error(libc::ENOTSUP));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_times() {
        let boot_time = get_boot_time().unwrap();
        let time_since_boot = get_time_since_boot().unwrap();

        // Calculate the boot time by working backward from the time since the last
        // boot, then make sure that they line up (within 50ms; there may be some
        // variation).
        let calculated_boot_time = SystemTime::now() - time_since_boot;
        let calc_abs_diff = match boot_time.duration_since(calculated_boot_time) {
            Ok(d) => d,
            Err(e) => e.duration(),
        };

        assert!(calc_abs_diff < Duration::from_millis(50));
    }

    #[test]
    fn test_get_active_uptime() {
        #[cfg(not(target_os = "netbsd"))]
        {
            get_active_uptime().unwrap();
        }

        #[cfg(target_os = "netbsd")]
        {
            assert!(crate::error::is_raw(
                &get_active_uptime().unwrap_err(),
                libc::ENOTSUP,
            ));
        }
    }
}
