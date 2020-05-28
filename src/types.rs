cfg_if::cfg_if! {
    if #[cfg(target_os = "dragonfly")] {
        use crate::{Uint, UidT, Short, GidT};

        #[repr(C)]
        #[derive(Clone, Copy)]
        pub struct xucred {
            pub cr_version: Uint,
            pub cr_uid: UidT,
            pub cr_ngroups: Short,
            pub cr_groups: [GidT; 16],
            _cr_unused1: *mut libc::c_void,
        }
    } else if #[cfg(target_os = "freebsd")] {
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
}
