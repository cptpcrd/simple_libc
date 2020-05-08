use std::io;
use std::iter::FromIterator;
use std::ops::Not;

use libc;
use strum::IntoEnumIterator;
use strum_macros;
use lazy_static::lazy_static;

use super::super::constants;
use super::super::error;


#[derive(Copy, Clone, Debug, strum_macros::Display, strum_macros::EnumString, strum_macros::EnumIter)]
pub enum Cap {
    // POSIX
    #[strum(serialize = "CAP_CHOWN")]
    Chown = constants::CAP_CHOWN,
    #[strum(serialize = "CAP_DAC_OVERRIDE")]
    DacOverride = constants::CAP_DAC_OVERRIDE,
    #[strum(serialize = "CAP_DAC_READ_SEARCH")]
    DacReadSearch = constants::CAP_DAC_READ_SEARCH,
    #[strum(serialize = "CAP_FOWNER")]
    Fowner = constants::CAP_FOWNER,
    #[strum(serialize = "CAP_FSETID")]
    Fsetid = constants::CAP_FSETID,
    #[strum(serialize = "CAP_KILL")]
    Kill = constants::CAP_KILL,
    #[strum(serialize = "CAP_SETGID")]
    Setgid = constants::CAP_SETGID,
    #[strum(serialize = "CAP_SETUID")]
    Setuid = constants::CAP_SETUID,

    // Linux
    #[strum(serialize = "CAP_SETPCAP")]
    Setpcap = constants::CAP_SETPCAP,
    #[strum(serialize = "CAP_LINUX_IMMUTABLE")]
    LinuxImmutable = constants::CAP_LINUX_IMMUTABLE,

    #[strum(serialize = "CAP_NET_BIND_SERVICE")]
    NetBindService = constants::CAP_NET_BIND_SERVICE,
    #[strum(serialize = "CAP_NET_BROADCAST")]
    NetBroadcast = constants::CAP_NET_BROADCAST,
    #[strum(serialize = "CAP_NET_ADMIN")]
    NetAdmin = constants::CAP_NET_ADMIN,
    #[strum(serialize = "CAP_NET_RAW")]
    NetRaw = constants::CAP_NET_RAW,

    #[strum(serialize = "CAP_IPC_LOCK")]
    IpcLock = constants::CAP_IPC_LOCK,
    #[strum(serialize = "CAP_IPC_OWNER")]
    IpcOwner = constants::CAP_IPC_OWNER,

    #[strum(serialize = "CAP_SYS_MODULE")]
    SysModule = constants::CAP_SYS_MODULE,
    #[strum(serialize = "CAP_SYS_RAWIO")]
    SysRawio = constants::CAP_SYS_RAWIO,
    #[strum(serialize = "CAP_SYS_CHROOT")]
    SysChroot = constants::CAP_SYS_CHROOT,
    #[strum(serialize = "CAP_SYS_PTRACE")]
    SysPtrace = constants::CAP_SYS_PTRACE,
    #[strum(serialize = "CAP_SYS_PACCT")]
    SysPacct = constants::CAP_SYS_PACCT,
    #[strum(serialize = "CAP_SYS_ADMIN")]
    SysAdmin = constants::CAP_SYS_ADMIN,
    #[strum(serialize = "CAP_SYS_BOOT")]
    SysBoot = constants::CAP_SYS_BOOT,
    #[strum(serialize = "CAP_SYS_NICE")]
    SysNice = constants::CAP_SYS_NICE,
    #[strum(serialize = "CAP_SYS_RESOURCE")]
    SysResource = constants::CAP_SYS_RESOURCE,
    #[strum(serialize = "CAP_SYS_TIME")]
    SysTime = constants::CAP_SYS_TIME,
    #[strum(serialize = "CAP_SYS_TTY_CONFIG")]
    SysTtyConfig = constants::CAP_SYS_TTY_CONFIG,

    #[strum(serialize = "CAP_MKNOD")]
    Mknod = constants::CAP_MKNOD,
    #[strum(serialize = "CAP_LEASE")]
    Lease = constants::CAP_LEASE,
    #[strum(serialize = "CAP_AUDIT_WRITE")]
    AuditWrite = constants::CAP_AUDIT_WRITE,
    #[strum(serialize = "CAP_AUDIT_CONTROL")]
    AuditControl = constants::CAP_AUDIT_CONTROL,
    #[strum(serialize = "CAP_SETFCAP")]
    Setfcap = constants::CAP_SETFCAP,
    #[strum(serialize = "CAP_MAC_OVERRIDE")]
    MacOverride = constants::CAP_MAC_OVERRIDE,
    #[strum(serialize = "CAP_MAC_ADMIN")]
    MacAdmin = constants::CAP_MAC_ADMIN,
    #[strum(serialize = "CAP_SYSLOG")]
    Syslog = constants::CAP_SYSLOG,
    #[strum(serialize = "CAP_WAKE_ALARM")]
    WakeAlarm = constants::CAP_WAKE_ALARM,
    #[strum(serialize = "CAP_BLOCK_SUSPEND")]
    BlockSuspend = constants::CAP_BLOCK_SUSPEND,
    #[strum(serialize = "CAP_AUDIT_READ")]
    AuditRead = constants::CAP_AUDIT_READ,
}

// Shift to the left, then subtract one to get the lower bits filled with ones.
const CAP_BITMASK: u64 = ((1 as u64) << (constants::CAP_MAX as u64 + 1)) - 1;

lazy_static! {
    pub static ref ALL_CAPS: Vec<Cap> = Cap::iter().collect();
}


impl Cap {
    fn to_single_bitfield(self) -> u64 {
        // Sanity check in case CAP_MAX gets set incorrectly
        // Note that this still won't catch certain cases
        debug_assert!((self as isize) <= constants::CAP_MAX);

        (1 as u64) << (self as u64)
    }
}


#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct CapSet {
    pub bits: u64,
}

impl CapSet {
    #[inline]
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    #[inline]
    pub const fn full() -> Self {
        Self { bits: CAP_BITMASK }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.bits = 0;
    }

    #[inline]
    pub fn fill(&mut self) {
        self.bits = CAP_BITMASK;
    }

    #[inline]
    pub fn has(&self, cap: Cap) -> bool {
        self.bits & cap.to_single_bitfield() != 0
    }

    #[inline]
    pub fn add(&mut self, cap: Cap) {
        self.bits |= cap.to_single_bitfield();
    }

    #[inline]
    pub fn drop(&mut self, cap: Cap) {
        self.bits &= !cap.to_single_bitfield();
    }

    pub fn set_state(&mut self, cap: Cap, val: bool) {
        if val {
            self.add(cap);
        }
        else {
            self.drop(cap);
        }
    }

    pub fn add_multi<T: IntoIterator<Item=Cap>>(&mut self, t: T) {
        for cap in t.into_iter() {
            self.add(cap);
        }
    }

    pub fn drop_multi<T: IntoIterator<Item=Cap>>(&mut self, t: T) {
        for cap in t.into_iter() {
            self.drop(cap);
        }
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item=Cap> {
        self.into_iter()
    }

    #[inline]
    pub fn union_with(&self, other: &Self) -> Self {
        Self { bits: self.bits & other.bits }
    }

    #[inline]
    pub fn intersection_with(&self, other: &Self) -> Self {
        Self { bits: self.bits | other.bits }
    }

    pub fn union(capsets: &[Self]) -> Self {
        let mut bits: u64 = 0;

        for capset in capsets {
            bits &= capset.bits;
        }

        Self { bits }
    }

    pub fn intersection(capsets: &[Self]) -> Self {
        let mut bits: u64 = 0;

        for capset in capsets {
            bits |= capset.bits;
        }

        Self { bits }
    }

    #[inline]
    const fn from_bits_safe(bitfield: u64) -> Self {
        Self { bits: bitfield & CAP_BITMASK }
    }
}

impl Not for CapSet {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self { bits: (!self.bits) & CAP_BITMASK }
    }
}

impl FromIterator<Cap> for CapSet {
    fn from_iter<I: IntoIterator<Item=Cap>>(it: I) -> Self {
        let mut res = Self::empty();
        res.add_multi(it);
        res
    }
}

impl IntoIterator for CapSet {
    type Item = Cap;
    type IntoIter = CapSetIterator;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        CapSetIterator {
            bits: self.bits,
            i: 0,
        }
    }
}


pub struct CapSetIterator {
    bits: u64,
    i: usize,
}

impl Iterator for CapSetIterator {
    type Item = Cap;

    fn next(&mut self) -> Option<Cap> {
        while self.i < ALL_CAPS.len() {
            let cap = ALL_CAPS[self.i];
            self.i += 1;

            if self.bits & cap.to_single_bitfield() != 0 {
                return Some(cap);
            }
        }

        None
    }
}


#[repr(C)]
struct c_cap_user_header {
    version: u32,
    pid: libc::c_int,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct c_cap_data_struct {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}

extern "C" {
    fn capget(hdrp: &mut c_cap_user_header, datap: &mut c_cap_data_struct) -> libc::c_int;

    fn capset(hdrp: &mut c_cap_user_header, datap: &c_cap_data_struct) -> libc::c_int;
}


#[derive(Copy, Clone, Debug)]
pub struct CapState {
    pub effective: CapSet,
    pub permitted: CapSet,
    pub inheritable: CapSet,
}

impl CapState {
    #[inline]
    pub fn get_current() -> io::Result<Self> {
        Self::get_for_pid(0)
    }

    pub fn get_for_pid(pid: i32) -> io::Result<Self> {
        let mut header = c_cap_user_header {
            version: constants::_LINUX_CAPABILITY_VERSION_3,
            pid: pid,
        };

        let mut raw_dat = [c_cap_data_struct {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        }; 2];

        error::convert_nzero(unsafe {
            capget(&mut header, &mut raw_dat[0])
        }, raw_dat).map(|raw_dat| {
            Self {
                effective: CapSet::from_bits_safe(
                    Self::combine_raw(raw_dat[0].effective, raw_dat[1].effective)
                ),
                permitted: CapSet::from_bits_safe(
                    Self::combine_raw(raw_dat[0].permitted, raw_dat[1].permitted)
                ),
                inheritable: CapSet::from_bits_safe(
                    Self::combine_raw(raw_dat[0].inheritable, raw_dat[1].inheritable)
                ),
            }
        })
    }

    #[inline]
    const fn combine_raw(lower: u32, upper: u32) -> u64 {
        ((upper as u64) << 32) + (lower as u64)
    }

    pub fn set_current(&self) -> io::Result<()> {
        let mut header = c_cap_user_header {
            version: constants::_LINUX_CAPABILITY_VERSION_3,
            pid: 0,
        };

        let effective = self.effective.bits;
        let permitted = self.permitted.bits;
        let inheritable = self.inheritable.bits;

        let raw_dat = [
            c_cap_data_struct {
                effective: effective as u32,
                permitted: permitted as u32,
                inheritable: inheritable as u32,
            },
            c_cap_data_struct {
                effective: (effective >> 32) as u32,
                permitted: (permitted >> 32) as u32,
                inheritable: (inheritable >> 32) as u32,
            },
        ];

        error::convert_nzero(unsafe {
            capset(&mut header, &raw_dat[0])
        }, ())
    }
}


fn prctl(option: i32, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> io::Result<i32> {
    error::convert_neg_ret(unsafe {
        libc::prctl(option, arg2, arg3, arg4, arg5)
    })
}


#[inline]
pub fn get_no_new_privs() -> io::Result<bool> {
    prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0).map(|x| x != 0)
}

#[inline]
pub fn set_no_new_privs() -> io::Result<()> {
    prctl(libc::PR_GET_NO_NEW_PRIVS, 1, 0, 0, 0).and(Ok(()))
}


#[inline]
pub fn get_keepcaps() ->  io::Result<bool> {
    prctl(libc::PR_GET_KEEPCAPS, 0, 0, 0, 0).map(|x| x != 0)
}

#[inline]
pub fn set_keepcaps(keep: bool) ->  io::Result<()> {
    prctl(libc::PR_SET_KEEPCAPS, keep as u64, 0, 0, 0).and(Ok(()))
}


pub mod ambient {
    use std::io;
    use libc;

    use strum::IntoEnumIterator;

    use super::{Cap, CapSet};


    #[inline]
    pub fn raise(cap: Cap) -> io::Result<()> {
        super::prctl(libc::PR_CAP_AMBIENT, libc::PR_CAP_AMBIENT_RAISE as u64, cap as u64, 0, 0).and(Ok(()))
    }

    #[inline]
    pub fn lower(cap: Cap) -> io::Result<()> {
        super::prctl(libc::PR_CAP_AMBIENT, libc::PR_CAP_AMBIENT_LOWER as u64, cap as u64, 0, 0).and(Ok(()))
    }

    #[inline]
    pub fn is_set(cap: Cap) -> io::Result<bool> {
        super::prctl(libc::PR_CAP_AMBIENT, libc::PR_CAP_AMBIENT_IS_SET as u64, cap as u64, 0, 0).map(|x| x != 0)
    }

    #[inline]
    pub fn clear() -> io::Result<()> {
        super::prctl(libc::PR_CAP_AMBIENT, libc::PR_CAP_AMBIENT_CLEAR_ALL as u64, 0, 0, 0).and(Ok(()))
    }

    #[inline]
    pub fn is_supported() -> bool {
        is_set(Cap::Chown).is_ok()
    }

    pub fn probe() -> io::Result<CapSet> {
        let mut set = CapSet::empty();

        for cap in Cap::iter() {
            if is_set(cap)? {
                set.add(cap);
            }
        }

        Ok(set)
    }
}

pub mod bounding {
    use std::io;
    use libc;

    use strum::IntoEnumIterator;

    use super::{Cap, CapSet};


    #[inline]
    pub fn drop(cap: Cap) -> io::Result<()> {
        super::prctl(libc::PR_CAPBSET_DROP, cap as u64, 0, 0, 0).and(Ok(()))
    }

    #[inline]
    pub fn read(cap: Cap) -> io::Result<bool> {
        super::prctl(libc::PR_CAPBSET_READ, cap as u64, 0, 0, 0).map(|x| x != 0)
    }

    // Slightly easier to understand than read()
    #[inline]
    pub fn is_set(cap: Cap) -> io::Result<bool> {
        read(cap)
    }

    pub fn probe() -> io::Result<CapSet> {
        let mut set = CapSet::empty();

        for cap in Cap::iter() {
            if read(cap)? {
                set.add(cap);
            }
        }

        Ok(set)
    }
}

pub mod secbits {
    use std::io;
    use libc;

    use bitflags::bitflags;


    bitflags! {
        pub struct SecFlags: u64 {
            const SECBIT_KEEP_CAPS = super::super::super::constants::SECBIT_KEEP_CAPS;
            const SECBIT_KEEP_CAPS_LOCKED = super::super::super::constants::SECBIT_KEEP_CAPS_LOCKED;

            const SECBIT_NO_SETUID_FIXUP = super::super::super::constants::SECBIT_NO_SETUID_FIXUP;
            const SECBIT_NO_SETUID_FIXUP_LOCKED = super::super::super::constants::SECBIT_NO_SETUID_FIXUP_LOCKED;

            const SECBIT_NOROOT = super::super::super::constants::SECBIT_NOROOT;
            const SECBIT_NOROOT_LOCKED = super::super::super::constants::SECBIT_NOROOT_LOCKED;

            const SECBIT_NO_CAP_AMBIENT_RAISE = super::super::super::constants::SECBIT_NO_CAP_AMBIENT_RAISE;
            const SECBIT_NO_CAP_AMBIENT_RAISE_LOCKED = super::super::super::constants::SECBIT_NO_CAP_AMBIENT_RAISE_LOCKED;
        }
    }

    #[inline]
    pub fn set(flags: SecFlags) -> io::Result<()> {
        super::prctl(libc::PR_SET_SECUREBITS, flags.bits(), 0, 0, 0).and(Ok(()))
    }

    #[inline]
    pub fn get() -> io::Result<SecFlags> {
        super::prctl(libc::PR_GET_SECUREBITS, 0, 0, 0, 0).map(|f| SecFlags::from_bits_truncate(f as u64))
    }
}