use std::io;
use libc;


pub fn set_cad_enabled_status(enabled: bool) -> io::Result<()> {
    let cmd: i32;
    if enabled {
        cmd = libc::LINUX_REBOOT_CMD_CAD_ON;
    }
    else {
        cmd = libc::LINUX_REBOOT_CMD_CAD_OFF;
    }

    super::error::convert(unsafe {libc::reboot(cmd) }, ())
}


pub fn force_final_reboot() {
    super::sync();
    unsafe { libc::reboot(libc::LINUX_REBOOT_CMD_RESTART); }
    panic!();
}

pub fn force_final_halt() {
    super::sync();
    unsafe { libc::reboot(libc::LINUX_REBOOT_CMD_HALT); }
    panic!();
}

pub fn force_final_power_off() {
    super::sync();
    unsafe { libc::reboot(libc::LINUX_REBOOT_CMD_POWER_OFF); }
    panic!();
}
