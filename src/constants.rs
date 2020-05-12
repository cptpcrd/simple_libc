use cfg_if::cfg_if;

// This module contains all the constants that are not exposed by
// libc and must instead be hardcoded.

cfg_if! {
    if #[cfg(target_os = "linux")] {
        use super::Ulong;

        // BEGIN USED BY process/capabilities.rs

        pub const SECBIT_KEEP_CAPS: Ulong = 0x10;
        pub const SECBIT_KEEP_CAPS_LOCKED: Ulong = 0x20;
        pub const SECBIT_NOROOT: Ulong = 0x1;
        pub const SECBIT_NOROOT_LOCKED: Ulong = 0x2;
        pub const SECBIT_NO_SETUID_FIXUP: Ulong = 0x4;
        pub const SECBIT_NO_SETUID_FIXUP_LOCKED: Ulong = 0x8;
        pub const SECBIT_NO_CAP_AMBIENT_RAISE: Ulong = 0x40;
        pub const SECBIT_NO_CAP_AMBIENT_RAISE_LOCKED: Ulong = 0x80;

        pub const CAP_CHOWN: isize = 0;
        pub const CAP_DAC_OVERRIDE: isize = 1;
        pub const CAP_DAC_READ_SEARCH: isize = 2;
        pub const CAP_FOWNER: isize = 3;
        pub const CAP_FSETID: isize = 4;
        pub const CAP_KILL: isize = 5;
        pub const CAP_SETGID: isize = 6;
        pub const CAP_SETUID: isize = 7;
        pub const CAP_SETPCAP: isize = 8;
        pub const CAP_LINUX_IMMUTABLE: isize = 9;
        pub const CAP_NET_BIND_SERVICE: isize = 10;
        pub const CAP_NET_BROADCAST: isize = 11;
        pub const CAP_NET_ADMIN: isize = 12;
        pub const CAP_NET_RAW: isize = 13;
        pub const CAP_IPC_LOCK: isize = 14;
        pub const CAP_IPC_OWNER: isize = 15;
        pub const CAP_SYS_MODULE: isize = 16;
        pub const CAP_SYS_RAWIO: isize = 17;
        pub const CAP_SYS_CHROOT: isize = 18;
        pub const CAP_SYS_PTRACE: isize = 19;
        pub const CAP_SYS_PACCT: isize = 20;
        pub const CAP_SYS_ADMIN: isize = 21;
        pub const CAP_SYS_BOOT: isize = 22;
        pub const CAP_SYS_NICE: isize = 23;
        pub const CAP_SYS_RESOURCE: isize = 24;
        pub const CAP_SYS_TIME: isize = 25;
        pub const CAP_SYS_TTY_CONFIG: isize = 26;
        pub const CAP_MKNOD: isize = 27;
        pub const CAP_LEASE: isize = 28;
        pub const CAP_AUDIT_WRITE: isize = 29;
        pub const CAP_AUDIT_CONTROL: isize = 30;
        pub const CAP_SETFCAP: isize = 31;
        pub const CAP_MAC_OVERRIDE: isize = 32;
        pub const CAP_MAC_ADMIN: isize = 33;
        pub const CAP_SYSLOG: isize = 34;
        pub const CAP_WAKE_ALARM: isize = 35;
        pub const CAP_BLOCK_SUSPEND: isize = 36;
        pub const CAP_AUDIT_READ: isize = 37;

        // *** WARNING WARNING WARNING ***
        // This MUST be set to the last capability from the above list!
        // This assumption is used to perform shortcuts in several places.
        pub const CAP_MAX: isize = CAP_AUDIT_READ;

        // WARNING: Updating to newer versions may require significant
        // code changes to process/capabilities.rs
        pub const _LINUX_CAPABILITY_VERSION_3: u32 = 0x2008_0522;

        // END USED BY process/capabilities.rs


        // BEGIN USED by inotify.rs
        pub const IN_EXCL_UNLINK: u32 = 0x0400_0000;
        pub const IN_MASK_ADD: u32 = 0x2000_0000;
        pub const IN_MASK_CREATE: u32 = 0x1000_0000;
        // END USED by inotify.rs
    }
    else if #[cfg(target_os = "openbsd")] {
        use super::Int;

        // BEGIN USED by power.rs
        pub const RB_AUTOBOOT: Int = 0;
        pub const RB_HALT: Int = 0x0008;
        pub const RB_POWERDOWN: Int = 0x1000;
        pub const RB_NOSYNC: Int = 0x0004;
        // END USED by power.rs
    }
    else if #[cfg(target_os = "netbsd")] {
        use super::Int;

        // BEGIN USED by process/resource.rs
        pub const RLIMIT_SBSIZE: Int = 9;
        pub const RLIMIT_AS: Int = 10;
        pub const RLIMIT_NTHR: Int = 11;
        // END USED by process/resource.rs

        // BEGIN USED by power.rs
        pub const RB_AUTOBOOT: Int = 0;
        pub const RB_HALT: Int = 0x0008;
        pub const RB_POWERDOWN: Int = 0x0808;
        pub const RB_NOSYNC: Int = 0x0004;
        // END USED by power.rs
    }
    else if #[cfg(any(target_os = "freebsd", target_os = "dragonfly"))] {
        use super::Int;

        // BEGIN USED by power.rs
        pub const RB_AUTOBOOT: Int = 0;
        pub const RB_HALT: Int = 0x0008;
        pub const RB_POWEROFF: Int = 0x4000;
        pub const RB_POWERDOWN: Int = RB_POWEROFF;  // For compatibility
        pub const RB_NOSYNC: Int = 0x0004;
        // END USED by power.rs
    }
}
