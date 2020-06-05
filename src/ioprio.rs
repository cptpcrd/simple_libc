use std::io;

use crate::Int;
use crate::constants;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum Target {
    Process(Int),
    ProcGroup(Int),
    User(Int),
}

impl Target {
    fn unpack(self) -> (Int, Int) {
        match self {
            Self::Process(w) => (constants::IOPRIO_WHO_PROCESS, w),
            Self::ProcGroup(w) => (constants::IOPRIO_WHO_PGRP, w),
            Self::User(w) => (constants::IOPRIO_WHO_USER, w),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum Priority {
    None,
    RealTime(Int),
    BestEffort(Int),
    Idle,
}

impl Priority {
    fn to_ioprio(self) -> Int {
        let (class, data) = match self {
            Self::None => (constants::IOPRIO_CLASS_NONE, 0),
            Self::RealTime(dat) => (constants::IOPRIO_CLASS_RT, dat),
            Self::BestEffort(dat) => (constants::IOPRIO_CLASS_BE, dat),
            Self::Idle => (constants::IOPRIO_CLASS_IDLE, 0),
        };

        (class << constants::IOPRIO_CLASS_SHIFT) | (data & constants::IOPRIO_PRIO_MASK)
    }

    fn from_ioprio(ioprio: Int) -> Option<Self> {
        let class = ioprio >> constants::IOPRIO_CLASS_SHIFT;
        let data = ioprio & constants::IOPRIO_PRIO_MASK;

        match class {
            constants::IOPRIO_CLASS_NONE => Some(Self::None),
            constants::IOPRIO_CLASS_RT => Some(Self::RealTime(data)),
            constants::IOPRIO_CLASS_BE => Some(Self::BestEffort(data)),
            constants::IOPRIO_CLASS_IDLE => Some(Self::Idle),
            _ => None,
        }
    }
}

fn ioprio_get_raw(which: Int, who: Int) -> io::Result<Int> {
    crate::error::set_errno_success();

    crate::error::convert_if_errno_ret(unsafe {
        libc::syscall(libc::SYS_ioprio_get, which, who, 0, 0, 0, 0) as i32
    })
}

fn ioprio_set_raw(which: Int, who: Int, ioprio: Int) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe {
        libc::syscall(libc::SYS_ioprio_set, which, who, ioprio, 0, 0, 0)
    })
}

pub fn get(target: Target) -> io::Result<Priority> {
    let (which, who) = target.unpack();

    let ioprio = ioprio_get_raw(which, who)?;

    if let Some(prio) = Priority::from_ioprio(ioprio) {
        Ok(prio)
    } else {
        Err(io::Error::from_raw_os_error(libc::EINVAL))
    }
}

pub fn set(target: Target, prio: Priority) -> io::Result<()> {
    let (which, who) = target.unpack();

    ioprio_set_raw(which, who, prio.to_ioprio())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_ioprio() {
        assert_eq!(
            Priority::None.to_ioprio(),
            constants::IOPRIO_CLASS_NONE << constants::IOPRIO_CLASS_SHIFT,
        );

        assert_eq!(
            Priority::RealTime(0).to_ioprio(),
            constants::IOPRIO_CLASS_RT << constants::IOPRIO_CLASS_SHIFT,
        );
        assert_eq!(
            Priority::RealTime(7).to_ioprio(),
            (constants::IOPRIO_CLASS_RT << constants::IOPRIO_CLASS_SHIFT) + 7,
        );

        assert_eq!(
            Priority::BestEffort(0).to_ioprio(),
            constants::IOPRIO_CLASS_BE << constants::IOPRIO_CLASS_SHIFT,
        );
        assert_eq!(
            Priority::BestEffort(7).to_ioprio(),
            (constants::IOPRIO_CLASS_BE << constants::IOPRIO_CLASS_SHIFT) + 7,
        );

        assert_eq!(
            Priority::Idle.to_ioprio(),
            constants::IOPRIO_CLASS_IDLE << constants::IOPRIO_CLASS_SHIFT,
        );
    }

    #[test]
    fn test_get_set() {
        let prio = get(Target::Process(0)).unwrap();
        set(Target::Process(0), prio).unwrap();
    }
}
