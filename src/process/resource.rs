use std::io;
use std::str::FromStr;

use serde::Deserialize;

use super::super::error;
use super::super::Int;

#[cfg(target_os = "netbsd")]
use super::super::constants;

// Work around GNU not implementing the POSIX standard correctly
#[cfg(all(target_os = "linux", any(target_env = "", target_env = "gnu")))]
type RawResourceType = libc::__rlimit_resource_t;

#[cfg(not(all(target_os = "linux", any(target_env = "", target_env = "gnu"))))]
type RawResourceType = Int;

#[derive(
    Copy,
    Clone,
    Debug,
    Eq,
    PartialEq,
    strum_macros::Display,
    strum_macros::EnumString,
    strum_macros::EnumIter,
)]
#[repr(isize)]
pub enum Resource {
    // OpenBSD is missing this for some reason
    #[cfg(not(any(target_os = "openbsd", target_os = "netbsd")))]
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

    // Linux and the BSDs
    #[cfg(any(
        target_os = "linux",
        target_os = "openbsd",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    NPROC = libc::RLIMIT_NPROC as isize,
    #[cfg(any(
        target_os = "linux",
        target_os = "openbsd",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    MEMLOCK = libc::RLIMIT_MEMLOCK as isize,
    #[cfg(any(
        target_os = "linux",
        target_os = "openbsd",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "dragonfly"
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

impl serde::Serialize for Resource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string().to_lowercase())
    }
}

impl<'d> serde::Deserialize<'d> for Resource {
    fn deserialize<D: serde::Deserializer<'d>>(deserializer: D) -> Result<Self, D::Error> {
        Self::from_str(&String::deserialize(deserializer)?.to_uppercase())
            .map_err(serde::de::Error::custom)
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn serialize_limit<S: serde::Serializer>(
    limit: &Limit,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match *limit {
        LIMIT_INFINITY => serializer.serialize_none(),
        _ => serializer.serialize_some(&limit),
    }
}

pub fn deserialize_limit<'a, D: serde::Deserializer<'a>>(
    deserializer: D,
) -> Result<Limit, D::Error> {
    Ok(match Option::<Limit>::deserialize(deserializer)? {
        Some(limit) => limit,
        None => LIMIT_INFINITY,
    })
}

pub type Limit = libc::rlim_t;
pub const LIMIT_INFINITY: Limit = libc::RLIM_INFINITY;

pub fn getrlimit(resource: Resource) -> io::Result<(Limit, Limit)> {
    let mut rlim = libc::rlimit {
        rlim_cur: LIMIT_INFINITY,
        rlim_max: LIMIT_INFINITY,
    };

    error::convert_nzero(
        unsafe { libc::getrlimit(resource as RawResourceType, &mut rlim) },
        rlim,
    )
    .map(|rlim| (rlim.rlim_cur, rlim.rlim_max))
}

pub fn setrlimit(resource: Resource, new_limits: (Limit, Limit)) -> io::Result<()> {
    let rlim = libc::rlimit {
        rlim_cur: new_limits.0,
        rlim_max: new_limits.1,
    };

    error::convert_nzero(
        unsafe { libc::setrlimit(resource as RawResourceType, &rlim) },
        (),
    )
}

#[cfg(target_os = "linux")]
pub fn prlimit(
    pid: super::super::PidT,
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

    error::convert_nzero(
        unsafe {
            libc::prlimit(
                pid,
                resource as RawResourceType,
                new_rlim_ptr,
                &mut old_rlim,
            )
        },
        old_rlim,
    )
    .map(|old_rlim| (old_rlim.rlim_cur, old_rlim.rlim_max))
}

#[cfg(target_os = "linux")]
pub fn nice_rlimit_to_thresh(nice_rlim: Limit) -> Int {
    if nice_rlim == LIMIT_INFINITY {
        return -20;
    }

    20 - (super::super::constrain(nice_rlim, 1, 40) as Int)
}

#[cfg(target_os = "linux")]
pub fn nice_thresh_to_rlimit(nice_thresh: Int) -> Limit {
    (20 - super::super::constrain(nice_thresh, -20, 19)) as Limit
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_test::{assert_de_tokens, assert_ser_tokens, assert_tokens, Token};

    #[test]
    fn test_resource_serde() {
        assert_ser_tokens(&Resource::NOFILE, &[Token::String("nofile")]);

        // Deserializing is case-insensitive
        assert_de_tokens(&Resource::NOFILE, &[Token::String("nofile")]);
        assert_de_tokens(&Resource::NOFILE, &[Token::String("nofile")]);
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
            ()
        });
    }

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
}
