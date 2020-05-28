cfg_if::cfg_if! {
    if #[cfg(target_os = "dragonfly")] {
        use super::{Uint, UidT, Short, GidT};

        #[repr(C)]
        pub struct xucred {
            pub cr_version: Uint,
            pub cr_uid: UidT,
            pub cr_ngroups: Short,
            pub cr_groups: [GidT; 16],
            _cr_unused1: *mut libc::c_void,
        }
    }
}
