use std::io;
use libc;

use bitflags::bitflags;


#[derive(Debug)]
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
            let cmd = match enabled {
                true => libc::LINUX_REBOOT_CMD_CAD_ON,
                false => libc::LINUX_REBOOT_CMD_CAD_OFF,
            };

            super::error::convert(unsafe { libc::reboot(cmd) }, ())
        }

        pub fn perform_action(action: &Action, flags: ActionFlags) -> io::Result<()> {
            let reboot_flags = match action {
                Action::ForceReboot => libc::LINUX_REBOOT_CMD_RESTART,
                Action::ForceHalt => libc::LINUX_REBOOT_CMD_HALT,
                Action::ForcePowerOff => libc::LINUX_REBOOT_CMD_POWER_OFF,
            };

            // Linux does not sync() by default, so we need to do it manually
            if !flags.contains(ActionFlags::NOSYNC) {
                super::sync();
            }

            unsafe { libc::reboot(reboot_flags); }

            Err(io::Error::last_os_error())
        }
    }
    else if #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "dragonfly", target_os = "netbsd"))] {
        pub fn perform_action(action: Action, flags: ActionFlags) -> io::Result<()> {
            let mut reboot_flags = match action {
                Action::ForceReboot => libc::RB_AUTOBOOT,
                Action::ForceHalt => libc::RB_HALT,
                Action::ForcePowerOff => libc::RB_HALT | libc::RB_POWERDOWN,
            };

            if flags.contains(ActionFlags::NOSYNC) {
                reboot_flags |= libc::RB_NOSYNC;
            }

            unsafe { libc::reboot(reboot_flags); }

            Err(io::Error::last_os_error())
        }
    }
}
