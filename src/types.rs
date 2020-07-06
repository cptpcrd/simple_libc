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
}
