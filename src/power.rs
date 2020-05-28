use std::io;

use bitflags::bitflags;

#[derive(Debug, Copy, Clone)]
pub enum Action {
    /// Reboot the system
    ForceReboot,
    /// Halt the system
    ForceHalt,
    /// Halt the system and attempt to power it down
    ForcePowerOff,
}

bitflags! {
    /// Extra modifiers for the action to be performed.
    ///
    /// Note: The values of these bitmasks have NO MEANING to the OS.
    /// Do NOT pass them directly to `libc::reboot()`.
    #[derive(Default)]
    pub struct ActionFlags: u32 {
        /// Do not sync the disks before halting/rebooting.
        ///
        /// WARNING: Use of this option will almost certainly result in data loss!
        const NOSYNC = 0b00001;
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        pub fn set_cad_enabled_status(enabled: bool) -> io::Result<()> {
            let cmd = if enabled {
                libc::LINUX_REBOOT_CMD_CAD_ON
            } else {
                libc::LINUX_REBOOT_CMD_CAD_OFF
            };

            crate::error::convert(unsafe { libc::reboot(cmd) }, ())
        }

        pub fn perform_action(action: Action, flags: ActionFlags) -> io::Result<()> {
            let reboot_flags = match action {
                Action::ForceReboot => libc::LINUX_REBOOT_CMD_RESTART,
                Action::ForceHalt => libc::LINUX_REBOOT_CMD_HALT,
                Action::ForcePowerOff => libc::LINUX_REBOOT_CMD_POWER_OFF,
            };

            // Linux does not sync() by default, so we need to do it manually
            if !flags.contains(ActionFlags::NOSYNC) {
                crate::sync();
            }

            unsafe { libc::reboot(reboot_flags); }

            Err(io::Error::last_os_error())
        }
    }
    else if #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "dragonfly", target_os = "netbsd"))] {
        use crate::externs;
        use crate::constants;

        pub fn perform_action(action: Action, flags: ActionFlags) -> io::Result<()> {
            let mut reboot_flags = match action {
                Action::ForceReboot => constants::RB_AUTOBOOT,
                Action::ForceHalt => constants::RB_HALT,
                Action::ForcePowerOff => constants::RB_HALT | constants::RB_POWERDOWN,
            };

            if flags.contains(ActionFlags::NOSYNC) {
                reboot_flags |= constants::RB_NOSYNC;
            }

            #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "dragonfly"))]
            unsafe { externs::reboot(reboot_flags); }

            #[cfg(target_os = "netbsd")]
            unsafe {
                use std::ffi;
                let empty_str = ffi::CString::new("").unwrap().into_raw();
                externs::reboot(reboot_flags, empty_str);
                ffi::CString::from_raw(empty_str);
            }

            Err(io::Error::last_os_error())
        }
    }
}
