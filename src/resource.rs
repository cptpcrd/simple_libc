use std::io;

#[cfg(any(all(feature = "serde", feature = "strum"), test))]
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::Deserialize;

use crate::error;
use crate::Int;

#[cfg(target_os = "netbsd")]
use crate::constants;

// Work around GNU not implementing the POSIX standard correctly
#[cfg(all(target_os = "linux", any(target_env = "", target_env = "gnu")))]
type RawResourceType = libc::__rlimit_resource_t;
#[cfg(not(all(target_os = "linux", any(target_env = "", target_env = "gnu"))))]
type RawResourceType = Int;

#[cfg_attr(
    any(feature = "strum", test),
    derive(
        strum_macros::Display,
        strum_macros::EnumString,
        strum_macros::EnumIter,
    )
)]
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
#[repr(isize)]
pub enum Resource {
    // OpenBSD and macOS are missing this for some reason
    #[cfg(not(any(target_os = "openbsd", target_os = "macos", target_os = "netbsd")))]
    AS = libc::RLIMIT_AS as isize,
    #[cfg(target_os = "netbsd")]
    AS = constants::RLIMIT_AS as isize,

    // Should be present on all POSIX systems
    CORE = libc::RLIMIT_CORE as isize,
    CPU = libc::RLIMIT_CPU as isize,
    DATA = libc::RLIMIT_DATA as isize,
    NOFILE = libc::RLIMIT_NOFILE as isize,
    FSIZE = libc::RLIMIT_FSIZE as isize,
    STACK = libc::RLIMIT_STACK as isize,

    // Linux, the BSDs, and macOS
    #[cfg(any(
        target_os = "linux",
        target_os = "openbsd",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "macos",
    ))]
    NPROC = libc::RLIMIT_NPROC as isize,
    #[cfg(any(
        target_os = "linux",
        target_os = "openbsd",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "macos",
    ))]
    MEMLOCK = libc::RLIMIT_MEMLOCK as isize,
    #[cfg(any(
        target_os = "linux",
        target_os = "openbsd",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        target_os = "macos",
    ))]
    RSS = libc::RLIMIT_RSS as isize,

    // Most of the BSDs (but not OpenBSD)
    #[cfg(any(target_os = "freebsd", target_os = "dragonfly"))]
    SBSIZE = libc::RLIMIT_SBSIZE as isize,
    #[cfg(target_os = "netbsd")]
    SBSIZE = constants::RLIMIT_SBSIZE as isize,

    // FreeBSD-specific
    #[cfg(target_os = "freebsd")]
    KQUEUES = libc::RLIMIT_KQUEUES as isize,
    #[cfg(target_os = "freebsd")]
    SWAP = libc::RLIMIT_SWAP as isize,
    #[cfg(target_os = "freebsd")]
    NPTS = libc::RLIMIT_NPTS as isize,

    // NetBSD-specific
    #[cfg(target_os = "netbsd")]
    NTHR = constants::RLIMIT_NTHR as isize,

    // DragonFly BSD-specific
    #[cfg(target_os = "dragonfly")]
    POSIXLOCKS = libc::RLIMIT_POSIXLOCKS as isize,

    // Linux-specific
    #[cfg(target_os = "linux")]
    MSGQUEUE = libc::RLIMIT_MSGQUEUE as isize,
    #[cfg(target_os = "linux")]
    NICE = libc::RLIMIT_NICE as isize,
    #[cfg(target_os = "linux")]
    RTPRIO = libc::RLIMIT_RTPRIO as isize,
    #[cfg(target_os = "linux")]
    RTTIME = libc::RLIMIT_RTTIME as isize,
    #[cfg(target_os = "linux")]
    SIGPENDING = libc::RLIMIT_SIGPENDING as isize,
}

#[cfg(any(all(feature = "serde", feature = "strum"), test))]
impl serde::Serialize for Resource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string().to_lowercase())
    }
}

#[cfg(any(all(feature = "serde", feature = "strum"), test))]
impl<'d> serde::Deserialize<'d> for Resource {
    fn deserialize<D: serde::Deserializer<'d>>(deserializer: D) -> Result<Self, D::Error> {
        Self::from_str(&String::deserialize(deserializer)?.to_uppercase())
            .map_err(serde::de::Error::custom)
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
#[cfg(feature = "serde")]
pub fn serialize_limit<S: serde::Serializer>(
    limit: &Limit,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match *limit {
        LIMIT_INFINITY => serializer.serialize_none(),
        _ => serializer.serialize_some(&limit),
    }
}

#[cfg(feature = "serde")]
pub fn deserialize_limit<'a, D: serde::Deserializer<'a>>(
    deserializer: D,
) -> Result<Limit, D::Error> {
    Ok(Option::<Limit>::deserialize(deserializer)?.unwrap_or(LIMIT_INFINITY))
}

pub fn compare_limits(val1: &Limit, val2: &Limit) -> std::cmp::Ordering {
    if *val1 == LIMIT_INFINITY {
        if *val2 == LIMIT_INFINITY {
            std::cmp::Ordering::Equal
        } else {
            std::cmp::Ordering::Greater
        }
    } else if *val2 == LIMIT_INFINITY {
        std::cmp::Ordering::Less
    } else {
        val1.cmp(val2)
    }
}

pub fn min_limit(val1: Limit, val2: Limit) -> Limit {
    // If either value is infinity, use the other one.
    // Otherwise, just take the minimum.
    if val1 == LIMIT_INFINITY {
        val2
    } else if val2 == LIMIT_INFINITY {
        val1
    } else {
        std::cmp::min(val1, val2)
    }
}

pub fn max_limit(val1: Limit, val2: Limit) -> Limit {
    // If either value is infinity, return infinity.
    // Otherwise, just take the maximum.
    if val1 == LIMIT_INFINITY || val2 == LIMIT_INFINITY {
        LIMIT_INFINITY
    } else {
        std::cmp::max(val1, val2)
    }
}

pub type Limit = libc::rlim_t;
pub const LIMIT_INFINITY: Limit = libc::RLIM_INFINITY;

pub fn getrlimit(resource: Resource) -> io::Result<(Limit, Limit)> {
    let mut rlim = libc::rlimit {
        rlim_cur: LIMIT_INFINITY,
        rlim_max: LIMIT_INFINITY,
    };

    error::convert_nzero_ret(unsafe { libc::getrlimit(resource as RawResourceType, &mut rlim) })?;

    Ok((rlim.rlim_cur, rlim.rlim_max))
}

pub fn setrlimit(resource: Resource, new_limits: (Limit, Limit)) -> io::Result<()> {
    let rlim = libc::rlimit {
        rlim_cur: new_limits.0,
        rlim_max: new_limits.1,
    };

    error::convert_nzero_ret(unsafe { libc::setrlimit(resource as RawResourceType, &rlim) })
}

#[cfg(target_os = "linux")]
pub fn prlimit(
    pid: crate::PidT,
    resource: Resource,
    new_limits: Option<(Limit, Limit)>,
) -> io::Result<(Limit, Limit)> {
    let mut new_rlim = libc::rlimit {
        rlim_cur: LIMIT_INFINITY,
        rlim_max: LIMIT_INFINITY,
    };
    let mut new_rlim_ptr: *const libc::rlimit = std::ptr::null();

    if let Some(new_lims) = new_limits {
        new_rlim.rlim_cur = new_lims.0;
        new_rlim.rlim_max = new_lims.1;
        new_rlim_ptr = &new_rlim;
    }

    let mut old_rlim = libc::rlimit {
        rlim_cur: LIMIT_INFINITY,
        rlim_max: LIMIT_INFINITY,
    };

    error::convert_nzero_ret(unsafe {
        libc::prlimit(
            pid,
            resource as RawResourceType,
            new_rlim_ptr,
            &mut old_rlim,
        )
    })?;

    Ok((old_rlim.rlim_cur, old_rlim.rlim_max))
}

/// A generic version of Linux's `prlimit()` that is also implemented for some other
/// platforms. (WARNING: the semantics vary slightly.)
///
/// Note that this function provides fewer guarantees than Linux's `prlimit()`. Namely:
///
/// 1. On some platforms, it may only be possible to *get* resource limits for other
///    processes, not set new ones. In that case, an `ENOTSUP` error will be returned if
///    new limits are passed. (This is true even if `pid` is 0 or the current process's
///    PID).
/// 2. Getting the original limits and setting the new limits, as well as
///    getting/setting the soft limit and getting/setting the hard limit, may be
///    performed as separate operations. Besides the implications of this for performance
///    and creation of race conditions, if new limits are passed but an error is
///    returned, the soft and/or hard limits may or may not have been changed.
/// 3. The exact errors returned for different error conditions may vary slightly
///    across platforms, though an attempt is made to standardize them on
///    `prlimit()`-like errors.
///
/// This function will accept pid=0 to refer to the current process.
#[cfg(any(
    target_os = "linux",
    target_os = "netbsd",
    target_os = "freebsd",
    target_os = "dragonfly"
))]
#[inline]
#[allow(clippy::needless_return)]
pub fn proc_rlimit(
    pid: crate::PidT,
    resource: Resource,
    new_limits: Option<(Limit, Limit)>,
) -> io::Result<(Limit, Limit)> {
    #[cfg(target_os = "linux")]
    return prlimit(pid, resource, new_limits);

    #[cfg(not(target_os = "linux"))]
    return proc_rlimit_impl(pid, resource, new_limits);
}

#[cfg(target_os = "freebsd")]
fn proc_rlimit_impl(
    mut pid: crate::PidT,
    resource: Resource,
    new_limits: Option<(Limit, Limit)>,
) -> io::Result<(Limit, Limit)> {
    if pid == 0 {
        pid = crate::process::getpid();
    }

    let mut new_rlim_opt = if let Some(lims) = new_limits {
        Some(libc::rlimit {
            rlim_cur: lims.0,
            rlim_max: lims.1,
        })
    } else {
        None
    };

    let new_rlim_slice_opt = if let Some(ref mut rlim) = new_rlim_opt {
        Some(std::slice::from_mut(&mut *rlim))
    } else {
        None
    };

    let mut old_rlim = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };

    // Construct the MIB path
    let mib = [
        libc::CTL_KERN,
        libc::KERN_PROC,
        libc::KERN_PROC_RLIMIT,
        pid as Int,
        resource as Int,
    ];

    let nbytes = match unsafe {
        crate::sysctl_raw(
            &mib,
            Some(std::slice::from_mut(&mut old_rlim)),
            new_rlim_slice_opt,
        )
    } {
        Ok(n) => n,
        Err(e) => {
            // ENOENT means the node doesn't exist. Probably this means the process
            // doesn't exist, so we return ESRCH instead.
            return Err(if e.raw_os_error() == Some(libc::ENOENT) {
                io::Error::from_raw_os_error(libc::ESRCH)
            } else {
                e
            });
        }
    };

    // Sanity check
    if nbytes != std::mem::size_of::<libc::rlimit>() {
        return Err(io::Error::from_raw_os_error(libc::EINVAL));
    }

    Ok((old_rlim.rlim_cur, old_rlim.rlim_max))
}

#[cfg(target_os = "dragonfly")]
fn proc_rlimit_impl(
    pid: crate::PidT,
    resource: Resource,
    new_limits: Option<(Limit, Limit)>,
) -> io::Result<(Limit, Limit)> {
    use std::io::BufRead;

    if new_limits.is_some() {
        // Can't set rlimits
        return Err(io::Error::from_raw_os_error(libc::ENOTSUP));
    }

    let prefix = match resource {
        Resource::CPU => "cpu ",
        Resource::FSIZE => "fsize ",
        Resource::DATA => "data ",
        Resource::STACK => "stack ",
        Resource::CORE => "core ",
        Resource::RSS => "rss ",
        Resource::MEMLOCK => "memlock ",
        Resource::NPROC => "nproc ",
        Resource::NOFILE => "nofile ",
        Resource::SBSIZE => "sbsize ",
        Resource::AS => "vmem ",
        Resource::POSIXLOCKS => "posixlock ",
    };

    let rlim_path = std::path::Path::new("/proc/")
        .join(if pid == 0 {
            "curproc".to_string()
        } else {
            pid.to_string()
        })
        .join("rlimit");

    fn parse_rlim_str(lim_str: &str) -> Option<Limit> {
        if lim_str == "-1" {
            Some(LIMIT_INFINITY)
        } else {
            lim_str.parse().ok()
        }
    }

    match std::fs::File::open(rlim_path) {
        Ok(f) => {
            let mut reader = io::BufReader::new(f);
            let mut line = String::new();

            while reader.read_line(&mut line)? > 0 {
                if line.starts_with(prefix) {
                    let remainder = line[prefix.len()..].trim();

                    if let Some(index) = remainder.find(' ') {
                        let (cur_lim_str, max_lim_str) = remainder.split_at(index);
                        let max_lim_str = &max_lim_str[1..];

                        if let Some(cur_lim) = parse_rlim_str(cur_lim_str) {
                            if let Some(max_lim) = parse_rlim_str(max_lim_str) {
                                return Ok((cur_lim, max_lim));
                            }
                        }
                    }
                }

                line.clear();
            }

            Err(io::Error::from_raw_os_error(libc::EINVAL))
        }
        Err(e) if crate::error::is_raw(&e, libc::ENOENT) => {
            Err(io::Error::from_raw_os_error(libc::ESRCH))
        }
        Err(e) => Err(e),
    }
}

#[cfg(target_os = "netbsd")]
fn proc_rlimit_impl(
    pid: crate::PidT,
    resource: Resource,
    new_limits: Option<(Limit, Limit)>,
) -> io::Result<(Limit, Limit)> {
    // Split the new limits into two Options
    let (new_soft, new_hard) = if let Some((soft, hard)) = new_limits {
        // Return EINVAL if soft > hard
        if soft > hard {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }

        (Some(soft), Some(hard))
    } else {
        (None, None)
    };

    // If both the soft and hard limits are being raised, then we need to set the
    // hard limit first, since the new soft limit may be greater than the old hard
    // limit (but less than the new hard limit).
    //
    // If both the soft and hard limits are being lowered, then we need to set the
    // soft limit first. That way, when we set the hard limit we won't be trying
    // to set it to a value below the soft limit.
    //
    // If we're moving the soft and hard limits in opposite directions (raising one
    // and lowering the other), then we can set the soft/hard limits in either order.
    //
    // So here's what we do:
    // 1. We try to get/set the soft limit.
    //    If this succeeds, we skip step 3.
    //    If we get EINVAL, we ignore it (but we don't skip step 3).
    //    If we get any other error, we return it up to the caller.
    // 2. We try to get/set the hard limit, returning any errors up to the caller.
    // 3. If step 1 failed with EINVAL, we try to get/set the soft limit again,
    //    returning any errors up to the caller.

    let old_soft = match proc_limit_getset(pid, resource, new_soft, false) {
        Ok(old_soft) => Some(old_soft),
        Err(e) => {
            if e.raw_os_error() == Some(libc::EINVAL) {
                None
            } else {
                return Err(e);
            }
        }
    };

    let old_hard = proc_limit_getset(pid, resource, new_hard, true)?;

    let old_soft = if let Some(val) = old_soft {
        val
    } else {
        // The original call failed with EINVAL. Try again.
        proc_limit_getset(pid, resource, new_soft, false)?
    };

    Ok((old_soft, old_hard))
}

#[cfg(target_os = "netbsd")]
fn proc_limit_getset(
    pid: crate::PidT,
    resource: Resource,
    mut new_limit: Option<Limit>,
    hard: bool,
) -> io::Result<Limit> {
    // Extract the pointer to the new limit
    let new_lim_slice_opt = if let Some(ref mut new_lim) = new_limit {
        Some(std::slice::from_mut(new_lim))
    } else {
        None
    };

    // Get the raw value for representing the resource.
    let raw_level = match resource {
        Resource::AS => constants::PROC_PID_LIMIT_AS,
        Resource::CPU => constants::PROC_PID_LIMIT_CPU,
        Resource::FSIZE => constants::PROC_PID_LIMIT_FSIZE,
        Resource::STACK => constants::PROC_PID_LIMIT_STACK,
        Resource::CORE => constants::PROC_PID_LIMIT_CORE,
        Resource::RSS => constants::PROC_PID_LIMIT_RSS,
        Resource::MEMLOCK => constants::PROC_PID_LIMIT_MEMLOCK,
        Resource::NPROC => constants::PROC_PID_LIMIT_NPROC,
        Resource::NOFILE => constants::PROC_PID_LIMIT_NOFILE,
        Resource::DATA => constants::PROC_PID_LIMIT_DATA,
        Resource::SBSIZE => constants::PROC_PID_LIMIT_SBSIZE,
        Resource::NTHR => constants::PROC_PID_LIMIT_NTHR,
    };

    // Construct the MIB path
    let mib = [
        libc::CTL_PROC,
        if pid == 0 {
            constants::PROC_CURPROC
        } else {
            pid as Int
        },
        constants::PROC_PID_LIMIT,
        raw_level,
        if hard {
            constants::PROC_PID_LIMIT_TYPE_HARD
        } else {
            constants::PROC_PID_LIMIT_TYPE_SOFT
        },
    ];

    let mut old_lim: Limit = LIMIT_INFINITY;

    let nbytes = match unsafe {
        crate::sysctl_raw(
            &mib,
            Some(std::slice::from_mut(&mut old_lim)),
            new_lim_slice_opt,
        )
    } {
        Ok(n) => n,
        Err(e) => {
            // ENOENT means the node doesn't exist. Probably this means the process
            // doesn't exist, so we return ESRCH instead.
            return Err(if e.raw_os_error() == Some(libc::ENOENT) {
                io::Error::from_raw_os_error(libc::ESRCH)
            } else {
                e
            });
        }
    };

    // Sanity check
    if nbytes != std::mem::size_of::<Limit>() {
        return Err(io::Error::from_raw_os_error(libc::EINVAL));
    }

    Ok(old_lim)
}

#[cfg(target_os = "linux")]
pub fn nice_rlimit_to_thresh(nice_rlim: Limit) -> Int {
    if nice_rlim == LIMIT_INFINITY {
        return -20;
    }

    20 - (crate::constrain(nice_rlim, 1, 40) as Int)
}

#[cfg(target_os = "linux")]
pub fn nice_thresh_to_rlimit(nice_thresh: Int) -> Limit {
    (20 - crate::constrain(nice_thresh, -20, 19)) as Limit
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_test::{
        assert_de_tokens, assert_de_tokens_error, assert_ser_tokens, assert_tokens, Token,
    };
    use strum::IntoEnumIterator;

    #[test]
    fn test_get_set_rlimits() {
        for res in Resource::iter() {
            let limits = getrlimit(res).unwrap();
            setrlimit(res, limits).unwrap();
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_prlimit() {
        for res in Resource::iter() {
            let limits = prlimit(0, res, None).unwrap();
            assert_eq!(prlimit(0, res, Some(limits)).unwrap(), limits);
            assert_eq!(prlimit(0, res, None).unwrap(), limits);
        }
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "netbsd",
        target_os = "freebsd",
        target_os = "dragonfly"
    ))]
    #[test]
    fn test_proc_rlimit() {
        let pid = crate::process::getpid();

        for res in Resource::iter() {
            let limits = proc_rlimit(0, res, None).unwrap();

            assert_eq!(getrlimit(res).unwrap(), limits);

            #[cfg(not(target_os = "dragonfly"))]
            {
                assert_eq!(proc_rlimit(0, res, Some(limits)).unwrap(), limits);
                assert_eq!(proc_rlimit(0, res, None).unwrap(), limits);

                assert_eq!(proc_rlimit(pid, res, Some(limits)).unwrap(), limits);
                assert_eq!(proc_rlimit(pid, res, None).unwrap(), limits);
            }
        }

        #[cfg(target_os = "dragonfly")]
        {
            assert_eq!(
                proc_rlimit(0, Resource::DATA, Some(limits))
                    .unwrap_err()
                    .raw_os_error(),
                Some(libc::ENOTSUP),
            );

            assert_eq!(
                proc_rlimit(pid, Resource::DATA, Some(limits))
                    .unwrap_err()
                    .raw_os_error(),
                Some(libc::ENOTSUP),
            );
        }

        assert_eq!(
            proc_rlimit(-1, Resource::DATA, None)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::ESRCH),
        );
    }

    #[test]
    fn test_resource_serde() {
        assert_ser_tokens(&Resource::NOFILE, &[Token::String("nofile")]);

        assert_de_tokens_error::<Resource>(&[Token::String("")], "Matching variant not found");
        assert_de_tokens_error::<Resource>(
            &[Token::String("no_file")],
            "Matching variant not found",
        );

        // Deserializing is case-insensitive
        assert_de_tokens(&Resource::NOFILE, &[Token::String("nofile")]);
        assert_de_tokens(&Resource::NOFILE, &[Token::String("NoFile")]);
        assert_de_tokens(&Resource::NOFILE, &[Token::String("NOFILE")]);
    }

    #[test]
    fn test_limit_serde() {
        // A quick struct so we can use our custom serializer and deserializer
        #[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
        struct SerializeLimit {
            #[serde(
                serialize_with = "serialize_limit",
                deserialize_with = "deserialize_limit"
            )]
            limit: Limit,
        }

        assert_tokens(
            &SerializeLimit {
                limit: LIMIT_INFINITY,
            },
            &[
                Token::Struct {
                    name: "SerializeLimit",
                    len: 1,
                },
                Token::Str("limit"),
                Token::None,
                Token::StructEnd,
            ],
        );

        std::panic::catch_unwind(|| {
            assert_tokens(
                &SerializeLimit { limit: 1 },
                &[
                    Token::Struct {
                        name: "SerializeLimit",
                        len: 1,
                    },
                    Token::Str("limit"),
                    Token::Some,
                    Token::U64(1),
                    Token::StructEnd,
                ],
            );
        })
        .unwrap_or_else(|_| {
            std::panic::catch_unwind(|| {
                assert_tokens(
                    &SerializeLimit { limit: 1 },
                    &[
                        Token::Struct {
                            name: "SerializeLimit",
                            len: 1,
                        },
                        Token::Str("limit"),
                        Token::Some,
                        Token::I64(1),
                        Token::StructEnd,
                    ],
                );
            })
            .unwrap_or_else(|_| {
                assert_tokens(
                    &SerializeLimit { limit: 1 },
                    &[
                        Token::Struct {
                            name: "SerializeLimit",
                            len: 1,
                        },
                        Token::Str("limit"),
                        Token::Some,
                        Token::U32(1),
                        Token::StructEnd,
                    ],
                );
            });
        });
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_nice_rlimit_thresh() {
        assert_eq!(nice_rlimit_to_thresh(LIMIT_INFINITY), -20);

        assert_eq!(nice_rlimit_to_thresh(40), -20);
        assert_eq!(nice_rlimit_to_thresh(30), -10);
        assert_eq!(nice_rlimit_to_thresh(20), 0);
        assert_eq!(nice_rlimit_to_thresh(10), 10);
        assert_eq!(nice_rlimit_to_thresh(1), 19);

        assert_eq!(nice_rlimit_to_thresh(100), -20);
        assert_eq!(nice_rlimit_to_thresh(0), 19);

        assert_eq!(nice_thresh_to_rlimit(-20), 40);
        assert_eq!(nice_thresh_to_rlimit(-10), 30);
        assert_eq!(nice_thresh_to_rlimit(0), 20);
        assert_eq!(nice_thresh_to_rlimit(10), 10);
        assert_eq!(nice_thresh_to_rlimit(19), 1);

        assert_eq!(nice_thresh_to_rlimit(-100), 40);
        assert_eq!(nice_thresh_to_rlimit(100), 1);
    }

    #[test]
    fn test_compare_limits() {
        use std::cmp::Ordering;

        assert_eq!(compare_limits(&0, &0), Ordering::Equal);
        assert_eq!(compare_limits(&1, &0), Ordering::Greater);
        assert_eq!(compare_limits(&0, &1), Ordering::Less);

        assert_eq!(
            compare_limits(&LIMIT_INFINITY, &LIMIT_INFINITY),
            Ordering::Equal
        );
        assert_eq!(compare_limits(&LIMIT_INFINITY, &0), Ordering::Greater);
        assert_eq!(compare_limits(&0, &LIMIT_INFINITY), Ordering::Less);
    }

    #[test]
    fn test_min_max_limit() {
        assert_eq!(min_limit(1, 2), 1);
        assert_eq!(min_limit(1, LIMIT_INFINITY), 1);
        assert_eq!(min_limit(LIMIT_INFINITY, 1), 1);
        assert_eq!(min_limit(LIMIT_INFINITY, LIMIT_INFINITY), LIMIT_INFINITY);

        assert_eq!(max_limit(1, 2), 2);
        assert_eq!(max_limit(1, LIMIT_INFINITY), LIMIT_INFINITY);
        assert_eq!(max_limit(LIMIT_INFINITY, 1), LIMIT_INFINITY);
        assert_eq!(max_limit(LIMIT_INFINITY, LIMIT_INFINITY), LIMIT_INFINITY);
    }
}
