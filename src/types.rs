crate::attr_group! {
    #![cfg(target_os = "linux")]

    #[repr(C)]
    pub struct cap_user_header_t {
        pub version: u32,
        pub pid: libc::c_int,
    }

    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct cap_user_data_t {
        pub effective: u32,
        pub permitted: u32,
        pub inheritable: u32,
    }

    #[repr(C)]
    pub struct waitpid_siginfo {
        _pad1: libc::c_int,
        _pad2: libc::c_int,
        _pad3: libc::c_int,
        #[cfg(target_pointer_width = "64")]
        _pad4: libc::c_int,
        pub si_pid: libc::pid_t,
        pub si_uid: libc::uid_t,
        pub si_status: libc::c_int,
    }
}

crate::attr_group! {
    #![cfg(target_os = "dragonfly")]

    use crate::{Uint, UidT, Short, GidT};

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub struct xucred {
        pub cr_version: Uint,
        pub cr_uid: UidT,
        pub cr_ngroups: Short,
        pub cr_groups: [GidT; crate::constants::XU_NGROUPS as usize],
        _cr_unused1: *mut libc::c_void,
    }
}

crate::attr_group! {
    #![cfg(target_os = "freebsd")]

    use crate::{Uint, UidT, Short, GidT, PidT};

    #[repr(C)]
    #[derive(Clone, Copy)]
    union xucred_cr {
        pid: PidT,
        _cr_unused1: *const libc::c_void,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct xucred {
        pub cr_version: Uint,
        pub cr_uid: UidT,
        pub cr_ngroups: Short,
        pub cr_groups: [GidT; libc::XU_NGROUPS as usize],
        _cr: xucred_cr,
    }

    impl xucred {
        pub unsafe fn cr_pid(&self) -> PidT {
            self._cr.pid
        }
    }
}

crate::attr_group! {
    #![cfg(target_os = "netbsd")]

    use crate::{UidT, GidT, PidT};

    #[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
    #[repr(C)]
    pub struct unpcbid {
        pub pid: PidT,
        pub uid: UidT,
        pub gid: GidT,
    }

    #[repr(C)]
    pub struct waitpid_siginfo {
        _pad1: libc::c_int,
        _pad2: libc::c_int,
        _pad3: libc::c_int,
        #[cfg(target_pointer_width = "64")]
        _pad4: libc::c_int,
        pub si_pid: libc::pid_t,
        pub si_uid: libc::uid_t,
        pub si_status: libc::c_int,
    }
}

#[cfg(any(target_os = "netbsd", target_os = "freebsd", target_os = "dragonfly"))]
#[repr(C)]
pub struct wrusage {
    pub wru_self: libc::rusage,
    pub wru_children: libc::rusage,
}
