use std::convert::TryInto;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::iter::FromIterator;
use std::ops::{BitAnd, BitOr, BitXor, Not, Sub};

#[cfg(all(
    feature = "serde",
    any(all(feature = "strum", feature = "strum_macros"), test)
))]
use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::de::Deserialize;
#[cfg(all(
    feature = "serde",
    any(all(feature = "strum", feature = "strum_macros"), test)
))]
use serde::ser::SerializeSeq;

use crate::constants;
use crate::error;
use crate::externs;
use crate::types;

use crate::{Int, UidT, Ulong};

#[cfg_attr(
    any(all(feature = "strum", feature = "strum_macros"), test),
    derive(strum_macros::Display, strum_macros::EnumString)
)]
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
#[repr(isize)]
pub enum Cap {
    // POSIX
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_CHOWN")
    )]
    Chown = constants::CAP_CHOWN,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_DAC_OVERRIDE")
    )]
    DacOverride = constants::CAP_DAC_OVERRIDE,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_DAC_READ_SEARCH")
    )]
    DacReadSearch = constants::CAP_DAC_READ_SEARCH,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_FOWNER")
    )]
    Fowner = constants::CAP_FOWNER,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_FSETID")
    )]
    Fsetid = constants::CAP_FSETID,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_KILL")
    )]
    Kill = constants::CAP_KILL,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SETGID")
    )]
    Setgid = constants::CAP_SETGID,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SETUID")
    )]
    Setuid = constants::CAP_SETUID,

    // Linux
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SETPCAP")
    )]
    Setpcap = constants::CAP_SETPCAP,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_LINUX_IMMUTABLE")
    )]
    LinuxImmutable = constants::CAP_LINUX_IMMUTABLE,

    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_NET_BIND_SERVICE")
    )]
    NetBindService = constants::CAP_NET_BIND_SERVICE,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_NET_BROADCAST")
    )]
    NetBroadcast = constants::CAP_NET_BROADCAST,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_NET_ADMIN")
    )]
    NetAdmin = constants::CAP_NET_ADMIN,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_NET_RAW")
    )]
    NetRaw = constants::CAP_NET_RAW,

    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_IPC_LOCK")
    )]
    IpcLock = constants::CAP_IPC_LOCK,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_IPC_OWNER")
    )]
    IpcOwner = constants::CAP_IPC_OWNER,

    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SYS_MODULE")
    )]
    SysModule = constants::CAP_SYS_MODULE,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SYS_RAWIO")
    )]
    SysRawio = constants::CAP_SYS_RAWIO,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SYS_CHROOT")
    )]
    SysChroot = constants::CAP_SYS_CHROOT,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SYS_PTRACE")
    )]
    SysPtrace = constants::CAP_SYS_PTRACE,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SYS_PACCT")
    )]
    SysPacct = constants::CAP_SYS_PACCT,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SYS_ADMIN")
    )]
    SysAdmin = constants::CAP_SYS_ADMIN,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SYS_BOOT")
    )]
    SysBoot = constants::CAP_SYS_BOOT,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SYS_NICE")
    )]
    SysNice = constants::CAP_SYS_NICE,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SYS_RESOURCE")
    )]
    SysResource = constants::CAP_SYS_RESOURCE,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SYS_TIME")
    )]
    SysTime = constants::CAP_SYS_TIME,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SYS_TTY_CONFIG")
    )]
    SysTtyConfig = constants::CAP_SYS_TTY_CONFIG,

    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_MKNOD")
    )]
    Mknod = constants::CAP_MKNOD,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_LEASE")
    )]
    Lease = constants::CAP_LEASE,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_AUDIT_WRITE")
    )]
    AuditWrite = constants::CAP_AUDIT_WRITE,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_AUDIT_CONTROL")
    )]
    AuditControl = constants::CAP_AUDIT_CONTROL,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SETFCAP")
    )]
    Setfcap = constants::CAP_SETFCAP,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_MAC_OVERRIDE")
    )]
    MacOverride = constants::CAP_MAC_OVERRIDE,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_MAC_ADMIN")
    )]
    MacAdmin = constants::CAP_MAC_ADMIN,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_SYSLOG")
    )]
    Syslog = constants::CAP_SYSLOG,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_WAKE_ALARM")
    )]
    WakeAlarm = constants::CAP_WAKE_ALARM,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_BLOCK_SUSPEND")
    )]
    BlockSuspend = constants::CAP_BLOCK_SUSPEND,
    #[cfg_attr(
        any(all(feature = "strum", feature = "strum_macros"), test),
        strum(serialize = "CAP_AUDIT_READ")
    )]
    AuditRead = constants::CAP_AUDIT_READ,
}

impl Cap {
    pub fn iter() -> CapIter {
        CapIter { i: 0 }
    }
}

pub struct CapIter {
    i: isize,
}

impl Iterator for CapIter {
    type Item = Cap;

    fn next(&mut self) -> Option<Cap> {
        if self.i <= constants::CAP_MAX {
            let cap = unsafe { std::mem::transmute(self.i) };
            self.i += 1;
            Some(cap)
        } else {
            None
        }
    }
}

// Shift to the left, then subtract one to get the lower bits filled with ones.
const CAP_BITMASK: u64 = ((1 as u64) << (constants::CAP_MAX as u64 + 1)) - 1;

impl Cap {
    fn to_single_bitfield(self) -> u64 {
        // Sanity check in case CAP_MAX gets set incorrectly
        // Note that this still won't catch certain cases
        debug_assert!((self as isize) <= constants::CAP_MAX);

        (1 as u64) << (self as u64)
    }
}

#[cfg(all(
    feature = "serde",
    any(all(feature = "strum", feature = "strum_macros"), test)
))]
impl serde::Serialize for Cap {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(all(
    feature = "serde",
    any(all(feature = "strum", feature = "strum_macros"), test)
))]
impl<'d> serde::Deserialize<'d> for Cap {
    fn deserialize<D: serde::Deserializer<'d>>(deserializer: D) -> Result<Self, D::Error> {
        Self::from_str(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct CapSet {
    bits: u64,
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
    pub fn is_full(self) -> bool {
        self.bits == CAP_BITMASK
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.bits == 0
    }

    #[inline]
    pub fn has(self, cap: Cap) -> bool {
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
        } else {
            self.drop(cap);
        }
    }

    pub fn add_multi<T: IntoIterator<Item = Cap>>(&mut self, t: T) {
        for cap in t.into_iter() {
            self.add(cap);
        }
    }

    pub fn drop_multi<T: IntoIterator<Item = Cap>>(&mut self, t: T) {
        for cap in t.into_iter() {
            self.drop(cap);
        }
    }

    #[inline]
    pub fn iter(self) -> impl Iterator<Item = Cap> {
        self.into_iter()
    }

    #[inline]
    pub const fn union_with(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }

    #[inline]
    pub const fn intersection_with(self, other: Self) -> Self {
        Self {
            bits: self.bits & other.bits,
        }
    }

    pub fn union<'a, T: IntoIterator<Item = &'a Self>>(capsets: T) -> Self {
        let mut bits: u64 = 0;

        for capset in capsets {
            bits |= capset.bits;
        }

        Self { bits }
    }

    pub fn intersection<'a, T: IntoIterator<Item = &'a Self>>(capsets: T) -> Self {
        let mut bits: u64 = CAP_BITMASK;

        for capset in capsets {
            bits &= capset.bits;
        }

        Self { bits }
    }

    #[inline]
    pub const fn bits(self) -> u64 {
        self.bits
    }

    #[inline]
    const fn from_bits_safe(bitfield: u64) -> Self {
        Self {
            bits: bitfield & CAP_BITMASK,
        }
    }

    #[cfg(feature = "serde")]
    fn from_bits_checked(bits: u64) -> Option<Self> {
        if bits & (!CAP_BITMASK) == 0 {
            Some(Self { bits })
        } else {
            None
        }
    }
}

impl Not for CapSet {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self {
            bits: (!self.bits) & CAP_BITMASK,
        }
    }
}

impl BitAnd for CapSet {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersection_with(rhs)
    }
}

impl BitOr for CapSet {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        self.union_with(rhs)
    }
}

impl BitXor for CapSet {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self { bits: self.bits ^ rhs.bits }
    }
}

impl Sub for CapSet {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self { bits: self.bits & (!rhs.bits) }
    }
}

impl FromIterator<Cap> for CapSet {
    fn from_iter<I: IntoIterator<Item = Cap>>(it: I) -> Self {
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

impl fmt::Debug for CapSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
#[cfg(feature = "serde")]
pub fn serialize_capset_raw<S: serde::Serializer>(
    set: &CapSet,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_u64(set.bits)
}

#[cfg(feature = "serde")]
pub fn deserialize_capset_raw<'d, D: serde::Deserializer<'d>>(
    deserializer: D,
) -> Result<CapSet, D::Error> {
    CapSet::from_bits_checked(u64::deserialize(deserializer)?)
        .ok_or_else(|| serde::de::Error::custom("Invalid bits"))
}

#[allow(clippy::trivially_copy_pass_by_ref)]
#[cfg(all(
    feature = "serde",
    any(all(feature = "strum", feature = "strum_macros"), test)
))]
pub fn serialize_capset_seq<S: serde::Serializer>(
    set: &CapSet,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if set.is_full() {
        let mut seq = serializer.serialize_seq(Some(1))?;
        seq.serialize_element(&"ALL")?;
        return seq.end();
    }

    let values: Vec<Cap> = set.iter().collect();

    let mut seq = serializer.serialize_seq(Some(values.len()))?;
    for cap in values {
        seq.serialize_element(&cap)?;
    }
    seq.end()
}

#[cfg(all(
    feature = "serde",
    any(all(feature = "strum", feature = "strum_macros"), test)
))]
pub fn deserialize_capset_seq<'d, D: serde::Deserializer<'d>>(
    deserializer: D,
) -> Result<CapSet, D::Error> {
    let mut values: Vec<String> = Vec::deserialize(deserializer)?;
    if values.is_empty() {
        return Ok(CapSet::empty());
    }

    let mut set = CapSet::empty();
    let mut inverted = false;
    if values[0] == "!" {
        inverted = true;
        values.remove(0);
        set.fill();
    }

    for cap_name in values {
        if cap_name == "ALL" {
            if inverted {
                set.clear();
            } else {
                set.fill();
            }

            continue;
        }

        let cap = Cap::from_str(&cap_name).map_err(serde::de::Error::custom)?;
        if inverted {
            set.drop(cap);
        } else {
            set.add(cap);
        }
    }

    Ok(set)
}

pub struct CapSetIterator {
    bits: u64,
    i: isize,
}

impl Iterator for CapSetIterator {
    type Item = Cap;

    fn next(&mut self) -> Option<Cap> {
        while self.i <= constants::CAP_MAX {
            let cap: Cap = unsafe { std::mem::transmute(self.i) };
            self.i += 1;

            if self.bits & cap.to_single_bitfield() != 0 {
                return Some(cap);
            }
        }

        None
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
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

    pub fn get_for_pid(pid: Int) -> io::Result<Self> {
        let mut header = types::cap_user_header_t {
            version: constants::_LINUX_CAPABILITY_VERSION_3,
            pid,
        };

        let mut raw_dat = [types::cap_user_data_t {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        }; 2];

        error::convert_nzero_ret(unsafe { externs::capget(&mut header, &mut raw_dat[0]) })?;

        Ok(Self {
            effective: CapSet::from_bits_safe(combine_raw_u32s(
                raw_dat[0].effective,
                raw_dat[1].effective,
            )),
            permitted: CapSet::from_bits_safe(combine_raw_u32s(
                raw_dat[0].permitted,
                raw_dat[1].permitted,
            )),
            inheritable: CapSet::from_bits_safe(combine_raw_u32s(
                raw_dat[0].inheritable,
                raw_dat[1].inheritable,
            )),
        })
    }

    pub fn set_current(&self) -> io::Result<()> {
        let mut header = types::cap_user_header_t {
            version: constants::_LINUX_CAPABILITY_VERSION_3,
            pid: 0,
        };

        let effective = self.effective.bits;
        let permitted = self.permitted.bits;
        let inheritable = self.inheritable.bits;

        let raw_dat = [
            types::cap_user_data_t {
                effective: effective as u32,
                permitted: permitted as u32,
                inheritable: inheritable as u32,
            },
            types::cap_user_data_t {
                effective: (effective >> 32) as u32,
                permitted: (permitted >> 32) as u32,
                inheritable: (inheritable >> 32) as u32,
            },
        ];

        error::convert_nzero_ret(unsafe { externs::capset(&mut header, &raw_dat[0]) })
    }
}

#[inline]
const fn combine_raw_u32s(lower: u32, upper: u32) -> u64 {
    ((upper as u64) << 32) + (lower as u64)
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FileCaps {
    pub effective: bool,
    pub permitted: CapSet,
    pub inheritable: CapSet,
    pub rootid: Option<UidT>,
}

impl FileCaps {
    pub fn empty() -> Self {
        Self {
            effective: false,
            permitted: CapSet::empty(),
            inheritable: CapSet::empty(),
            rootid: None,
        }
    }

    pub fn get_for_file<P: AsRef<OsStr>>(path: P, follow_links: bool) -> io::Result<Option<Self>> {
        Self::extract_attr_or_error(crate::getxattr(path, constants::XATTR_NAME_CAPS, follow_links))
    }

    pub fn get_for_fd(fd: Int) -> io::Result<Option<Self>> {
        Self::extract_attr_or_error(crate::fgetxattr(fd, constants::XATTR_NAME_CAPS))
    }

    fn extract_attr_or_error(attr_res: io::Result<Vec<u8>>) -> io::Result<Option<Self>> {
        match attr_res {
            Ok(attrs) => Ok(Some(Self::unpack_attrs(&attrs)?)),
            Err(e) => {
                if e.raw_os_error() == Some(libc::ENODATA) {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }

    pub fn unpack_attrs(attrs: &[u8]) -> io::Result<Self> {
        let len = attrs.len();

        if len < 4 {
            return Err(io::Error::from_raw_os_error(libc::EINVAL));
        }

        let magic = u32::from_le_bytes(attrs[0..4].try_into().unwrap());
        let effective = (magic & constants::VFS_CAP_FLAGS_EFFECTIVE) != 0;
        let version = magic & (!constants::VFS_CAP_FLAGS_EFFECTIVE);

        if version == constants::VFS_CAP_REVISION_2 && len == constants::XATTR_CAPS_SZ_2 {
            Ok(FileCaps {
                effective,
                permitted: CapSet::from_bits_safe(
                    combine_raw_u32s(
                        u32::from_le_bytes(attrs[4..8].try_into().unwrap()),
                        u32::from_le_bytes(attrs[8..12].try_into().unwrap()),
                    )
                ),
                inheritable: CapSet::from_bits_safe(
                    combine_raw_u32s(
                        u32::from_le_bytes(attrs[12..16].try_into().unwrap()),
                        u32::from_le_bytes(attrs[16..20].try_into().unwrap()),
                    )
                ),
                rootid: None,
            })
        } else if version == constants::VFS_CAP_REVISION_3 && len == constants::XATTR_CAPS_SZ_3 {
            Ok(FileCaps {
                effective,
                permitted: CapSet::from_bits_safe(
                    combine_raw_u32s(
                        u32::from_le_bytes(attrs[4..8].try_into().unwrap()),
                        u32::from_le_bytes(attrs[8..12].try_into().unwrap()),
                    )
                ),
                inheritable: CapSet::from_bits_safe(
                    combine_raw_u32s(
                        u32::from_le_bytes(attrs[12..16].try_into().unwrap()),
                        u32::from_le_bytes(attrs[16..20].try_into().unwrap()),
                    )
                ),
                rootid: Some(u32::from_le_bytes(attrs[20..24].try_into().unwrap())),
            })
        } else if version == constants::VFS_CAP_REVISION_1 && len == constants::XATTR_CAPS_SZ_1 {
            Ok(FileCaps {
                effective,
                permitted: CapSet::from_bits_safe(u32::from_le_bytes(attrs[4..8].try_into().unwrap()) as u64),
                inheritable: CapSet::from_bits_safe(u32::from_le_bytes(attrs[8..12].try_into().unwrap()) as u64),
                rootid: None,
            })
        } else {
            Err(io::Error::from_raw_os_error(libc::EINVAL))
        }
    }
}

unsafe fn prctl(option: Int, arg2: Ulong, arg3: Ulong, arg4: Ulong, arg5: Ulong) -> io::Result<Int> {
    error::convert_neg_ret(libc::prctl(option, arg2, arg3, arg4, arg5))
}

#[inline]
pub fn get_no_new_privs() -> io::Result<bool> {
    let res = unsafe { prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) }?;

    Ok(res != 0)
}

#[inline]
pub fn set_no_new_privs() -> io::Result<()> {
    unsafe { prctl(libc::PR_GET_NO_NEW_PRIVS, 1, 0, 0, 0) }?;

    Ok(())
}

#[inline]
pub fn get_keepcaps() -> io::Result<bool> {
    let res = unsafe { prctl(libc::PR_GET_KEEPCAPS, 0, 0, 0, 0) }?;

    Ok(res != 0)
}

#[inline]
pub fn set_keepcaps(keep: bool) -> io::Result<()> {
    unsafe { prctl(libc::PR_SET_KEEPCAPS, keep as Ulong, 0, 0, 0) }?;

    Ok(())
}

pub mod ambient {
    use std::io;

    use super::{Cap, CapSet};
    use crate::Ulong;

    #[inline]
    pub fn raise(cap: Cap) -> io::Result<()> {
        unsafe {
            super::prctl(
                libc::PR_CAP_AMBIENT,
                libc::PR_CAP_AMBIENT_RAISE as Ulong,
                cap as Ulong,
                0,
                0,
            )
        }?;

        Ok(())
    }

    #[inline]
    pub fn lower(cap: Cap) -> io::Result<()> {
        unsafe {
            super::prctl(
                libc::PR_CAP_AMBIENT,
                libc::PR_CAP_AMBIENT_LOWER as Ulong,
                cap as Ulong,
                0,
                0,
            )
        }?;

        Ok(())
    }

    #[inline]
    pub fn is_set(cap: Cap) -> io::Result<bool> {
        let x = unsafe {
            super::prctl(
                libc::PR_CAP_AMBIENT,
                libc::PR_CAP_AMBIENT_IS_SET as Ulong,
                cap as Ulong,
                0,
                0,
            )
        }?;

        Ok(x != 0)
    }

    #[inline]
    pub fn clear() -> io::Result<()> {
        unsafe {
            super::prctl(
                libc::PR_CAP_AMBIENT,
                libc::PR_CAP_AMBIENT_CLEAR_ALL as Ulong,
                0,
                0,
                0,
            )
        }?;

        Ok(())
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

    use super::{Cap, CapSet};
    use crate::Ulong;

    #[inline]
    pub fn drop(cap: Cap) -> io::Result<()> {
        unsafe { super::prctl(libc::PR_CAPBSET_DROP, cap as Ulong, 0, 0, 0) }?;

        Ok(())
    }

    #[inline]
    pub fn read(cap: Cap) -> io::Result<bool> {
        let res = unsafe { super::prctl(libc::PR_CAPBSET_READ, cap as Ulong, 0, 0, 0) }?;

        Ok(res != 0)
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

    use bitflags::bitflags;

    use crate::constants;
    use crate::Ulong;

    bitflags! {
        pub struct SecFlags: Ulong {
            const KEEP_CAPS = constants::SECBIT_KEEP_CAPS;
            const KEEP_CAPS_LOCKED = constants::SECBIT_KEEP_CAPS_LOCKED;

            const NO_SETUID_FIXUP = constants::SECBIT_NO_SETUID_FIXUP;
            const NO_SETUID_FIXUP_LOCKED = constants::SECBIT_NO_SETUID_FIXUP_LOCKED;

            const NOROOT = constants::SECBIT_NOROOT;
            const NOROOT_LOCKED = constants::SECBIT_NOROOT_LOCKED;

            const NO_CAP_AMBIENT_RAISE = constants::SECBIT_NO_CAP_AMBIENT_RAISE;
            const NO_CAP_AMBIENT_RAISE_LOCKED = constants::SECBIT_NO_CAP_AMBIENT_RAISE_LOCKED;


            #[deprecated(since = "0.5.0", note = "The SECBIT_ prefix has been removed")]
            const SECBIT_KEEP_CAPS = constants::SECBIT_KEEP_CAPS;
            #[deprecated(since = "0.5.0", note = "The SECBIT_ prefix has been removed")]
            const SECBIT_KEEP_CAPS_LOCKED = constants::SECBIT_KEEP_CAPS_LOCKED;

            #[deprecated(since = "0.5.0", note = "The SECBIT_ prefix has been removed")]
            const SECBIT_NO_SETUID_FIXUP = constants::SECBIT_NO_SETUID_FIXUP;
            #[deprecated(since = "0.5.0", note = "The SECBIT_ prefix has been removed")]
            const SECBIT_NO_SETUID_FIXUP_LOCKED = constants::SECBIT_NO_SETUID_FIXUP_LOCKED;

            #[deprecated(since = "0.5.0", note = "The SECBIT_ prefix has been removed")]
            const SECBIT_NOROOT = constants::SECBIT_NOROOT;
            #[deprecated(since = "0.5.0", note = "The SECBIT_ prefix has been removed")]
            const SECBIT_NOROOT_LOCKED = constants::SECBIT_NOROOT_LOCKED;

            #[deprecated(since = "0.5.0", note = "The SECBIT_ prefix has been removed")]
            const SECBIT_NO_CAP_AMBIENT_RAISE = constants::SECBIT_NO_CAP_AMBIENT_RAISE;
            #[deprecated(since = "0.5.0", note = "The SECBIT_ prefix has been removed")]
            const SECBIT_NO_CAP_AMBIENT_RAISE_LOCKED = constants::SECBIT_NO_CAP_AMBIENT_RAISE_LOCKED;
        }
    }

    #[inline]
    pub fn set(flags: SecFlags) -> io::Result<()> {
        unsafe { super::prctl(libc::PR_SET_SECUREBITS, flags.bits(), 0, 0, 0) }?;

        Ok(())
    }

    #[inline]
    pub fn get() -> io::Result<SecFlags> {
        let f = unsafe {
            super::prctl(libc::PR_GET_SECUREBITS, 0, 0, 0, 0)
        }?;

        Ok(SecFlags::from_bits_truncate(f as Ulong))
    }
}

pub fn with_effective_capset<T, F: FnOnce() -> T>(capset: CapSet, f: F) -> io::Result<T> {
    let orig_state = CapState::get_current()?;

    let mut new_state = orig_state;
    new_state.effective = capset;
    new_state.set_current()?;

    let retval = f();

    orig_state.set_current()?;

    Ok(retval)
}

pub fn with_effective_cap<T, F: FnOnce() -> T>(cap: Cap, f: F) -> io::Result<T> {
    let orig_state = CapState::get_current()?;

    let mut new_state = orig_state;
    new_state.effective.add(cap);
    new_state.set_current()?;

    let retval = f();

    orig_state.set_current()?;

    Ok(retval)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants;

    #[cfg(feature = "serde")]
    use serde_test::{assert_de_tokens, assert_de_tokens_error, assert_tokens, Token};

    #[test]
    fn test_cap_iter() {
        assert_eq!(
            Cap::iter().last().map(|x| x as isize),
            Some(constants::CAP_MAX)
        );

        assert_eq!(
            Cap::iter().map(|x| x as isize).last(),
            Some(constants::CAP_MAX)
        );
    }

    #[test]
    fn test_cap_bits() {
        let mut mask: u64 = 0;

        for cap in Cap::iter() {
            let cap_bits = cap.to_single_bitfield();
            assert_eq!(2u64.pow(cap as u32), cap_bits);
            mask |= cap_bits;
        }

        assert_eq!(mask, CAP_BITMASK);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_cap_serde() {
        assert_tokens(&Cap::Chown, &[Token::Str("CAP_CHOWN")]);
    }

    #[test]
    fn test_capset_empty() {
        let mut set = CapSet::full();
        for cap in Cap::iter() {
            set.drop(cap);
        }
        assert_eq!(set.bits, 0);
        assert!(set.is_empty());
        assert!(!set.is_full());

        set = CapSet::empty();
        assert_eq!(set.bits, 0);
        assert!(set.is_empty());
        assert!(!set.is_full());

        set = CapSet::full();
        set.clear();
        assert_eq!(set.bits, 0);
        assert!(set.is_empty());
        assert!(!set.is_full());

        assert!(!Cap::iter().any(|c| set.has(c)));
    }

    #[test]
    fn test_capset_full() {
        let mut set = CapSet::empty();
        for cap in Cap::iter() {
            set.add(cap);
        }
        assert_eq!(set.bits, CAP_BITMASK);
        assert!(set.is_full());
        assert!(!set.is_empty());

        set = CapSet::full();
        assert_eq!(set.bits, CAP_BITMASK);
        assert!(set.is_full());
        assert!(!set.is_empty());

        set = CapSet::empty();
        set.fill();
        assert_eq!(set.bits, CAP_BITMASK);
        assert!(set.is_full());
        assert!(!set.is_empty());

        assert_eq!(set.bits, set.bits());

        assert!(Cap::iter().all(|c| set.has(c)));
    }

    #[test]
    fn test_capset_add_drop() {
        let mut set = CapSet::empty();
        set.add(Cap::Chown);
        assert!(set.has(Cap::Chown));
        assert!(!set.is_empty());

        set.drop(Cap::Chown);
        assert!(!set.has(Cap::Chown));
        assert!(set.is_empty());

        set.set_state(Cap::Chown, true);
        assert!(set.has(Cap::Chown));
        assert!(!set.is_empty());

        set.set_state(Cap::Chown, false);
        assert!(!set.has(Cap::Chown));
        assert!(set.is_empty());
    }

    #[test]
    fn test_capset_add_drop_multi() {
        let mut set = CapSet::empty();
        set.add_multi(vec![Cap::Fowner, Cap::Chown, Cap::Kill]);

        // Iteration order is not preserved, but it should be consistent.
        assert_eq!(
            set.into_iter().collect::<Vec<Cap>>(),
            vec![Cap::Chown, Cap::Fowner, Cap::Kill]
        );
        assert_eq!(
            set.iter().collect::<Vec<Cap>>(),
            vec![Cap::Chown, Cap::Fowner, Cap::Kill]
        );

        set.drop_multi(vec![Cap::Fowner, Cap::Chown]);
        assert_eq!(set.iter().collect::<Vec<Cap>>(), vec![Cap::Kill]);

        set.drop_multi(vec![Cap::Kill]);
        assert_eq!(set.iter().collect::<Vec<Cap>>(), vec![]);
    }

    #[test]
    fn test_capset_from_iter() {
        let set = CapSet::from_iter(vec![Cap::Chown, Cap::Fowner]);
        assert_eq!(
            set.iter().collect::<Vec<Cap>>(),
            vec![Cap::Chown, Cap::Fowner],
        );
    }

    #[test]
    fn test_capset_union() {
        let a = CapSet::from_iter(vec![Cap::Chown, Cap::Fowner]);
        let b = CapSet::from_iter(vec![Cap::Fowner, Cap::Kill]);
        let c = CapSet::from_iter(vec![Cap::Chown, Cap::Fowner, Cap::Kill]);
        assert_eq!(a.union_with(b), c);
        assert_eq!(CapSet::union(&[a, b]), c);
        assert_eq!(CapSet::union([a, b].iter()), c);
        assert_eq!(CapSet::union(&vec![a, b]), c);
        assert_eq!(CapSet::union(vec![a, b].iter()), c);
    }

    #[test]
    fn test_capset_intersection() {
        let a = CapSet::from_iter(vec![Cap::Chown, Cap::Fowner]);
        let b = CapSet::from_iter(vec![Cap::Fowner, Cap::Kill]);
        let c = CapSet::from_iter(vec![Cap::Fowner]);
        assert_eq!(a.intersection_with(b), c);
        assert_eq!(CapSet::intersection(&[a, b]), c);
        assert_eq!(CapSet::intersection([a, b].iter()), c);
        assert_eq!(CapSet::intersection(&vec![a, b]), c);
        assert_eq!(CapSet::intersection(vec![a, b].iter()), c);
    }

    #[test]
    fn test_capset_not() {
        assert_eq!(!CapSet::full(), CapSet::empty());
        assert_eq!(CapSet::full(), !CapSet::empty());

        let mut a = CapSet::full();
        let mut b = CapSet::empty();
        a.add(Cap::Chown);
        b.drop(Cap::Chown);
        assert_eq!(!a, b);
    }

    #[test]
    fn test_capset_bitor() {
        let a = CapSet::from_iter(vec![Cap::Chown, Cap::Fowner]);
        let b = CapSet::from_iter(vec![Cap::Fowner, Cap::Kill]);
        let c = CapSet::from_iter(vec![Cap::Chown, Cap::Fowner, Cap::Kill]);
        assert_eq!(a | b , c);
    }

    #[test]
    fn test_capset_bitand() {
        let a = CapSet::from_iter(vec![Cap::Chown, Cap::Fowner]);
        let b = CapSet::from_iter(vec![Cap::Fowner, Cap::Kill]);
        let c = CapSet::from_iter(vec![Cap::Fowner]);
        assert_eq!(a & b, c);
    }

    #[test]
    fn test_capset_bitxor() {
        let a = CapSet::from_iter(vec![Cap::Chown, Cap::Fowner]);
        let b = CapSet::from_iter(vec![Cap::Fowner, Cap::Kill]);
        let c = CapSet::from_iter(vec![Cap::Chown, Cap::Kill]);
        assert_eq!(a ^ b, c);
    }

    #[test]
    fn test_capset_sub() {
        let a = CapSet::from_iter(vec![Cap::Chown, Cap::Fowner]);
        let b = CapSet::from_iter(vec![Cap::Fowner, Cap::Kill]);
        let c = CapSet::from_iter(vec![Cap::Chown]);
        assert_eq!(a - b, c);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_capset_serde_seq() {
        // A quick struct so we can use our custom serializer and deserializer
        #[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
        struct SerSet {
            #[serde(
                serialize_with = "serialize_capset_seq",
                deserialize_with = "deserialize_capset_seq"
            )]
            set: CapSet,
        }

        let mut s = SerSet {
            set: CapSet::empty(),
        };
        assert_tokens(
            &s,
            &[
                Token::Struct {
                    name: "SerSet",
                    len: 1,
                },
                Token::Str("set"),
                Token::Seq { len: Some(0) },
                Token::SeqEnd,
                Token::StructEnd,
            ],
        );

        s.set.add(Cap::Chown);
        assert_tokens(
            &s,
            &[
                Token::Struct {
                    name: "SerSet",
                    len: 1,
                },
                Token::Str("set"),
                Token::Seq { len: Some(1) },
                Token::Str("CAP_CHOWN"),
                Token::SeqEnd,
                Token::StructEnd,
            ],
        );

        s.set.fill();
        assert_tokens(
            &s,
            &[
                Token::Struct {
                    name: "SerSet",
                    len: 1,
                },
                Token::Str("set"),
                Token::Seq { len: Some(1) },
                Token::Str("ALL"),
                Token::SeqEnd,
                Token::StructEnd,
            ],
        );

        s.set.drop(Cap::Chown);
        assert_de_tokens(
            &s,
            &[
                Token::Struct {
                    name: "SerSet",
                    len: 1,
                },
                Token::Str("set"),
                Token::Seq { len: Some(2) },
                Token::Str("!"),
                Token::Str("CAP_CHOWN"),
                Token::SeqEnd,
                Token::StructEnd,
            ],
        );

        s.set.clear();
        assert_de_tokens(
            &s,
            &[
                Token::Struct {
                    name: "SerSet",
                    len: 1,
                },
                Token::Str("set"),
                Token::Seq { len: Some(2) },
                Token::Str("!"),
                Token::Str("ALL"),
                Token::SeqEnd,
                Token::StructEnd,
            ],
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_capset_serde_raw() {
        #[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
        struct SerSet {
            #[serde(
                serialize_with = "serialize_capset_raw",
                deserialize_with = "deserialize_capset_raw"
            )]
            set: CapSet,
        }

        let mut s = SerSet {
            set: CapSet::empty(),
        };
        assert_tokens(
            &s,
            &[
                Token::Struct {
                    name: "SerSet",
                    len: 1,
                },
                Token::Str("set"),
                Token::U64(0),
                Token::StructEnd,
            ],
        );

        s.set.add(Cap::Chown);
        assert_tokens(
            &s,
            &[
                Token::Struct {
                    name: "SerSet",
                    len: 1,
                },
                Token::Str("set"),
                Token::U64(Cap::Chown.to_single_bitfield()),
                Token::StructEnd,
            ],
        );

        s.set.fill();
        assert_tokens(
            &s,
            &[
                Token::Struct {
                    name: "SerSet",
                    len: 1,
                },
                Token::Str("set"),
                Token::U64(CAP_BITMASK),
                Token::StructEnd,
            ],
        );

        assert_de_tokens_error::<SerSet>(
            &[
                Token::Struct {
                    name: "SerSet",
                    len: 1,
                },
                Token::Str("set"),
                Token::U64(CAP_BITMASK + 1),
                Token::StructEnd,
            ],
            "Invalid bits",
        );
    }

    #[test]
    fn test_capstate() {
        CapState::get_current().unwrap();
    }

    #[test]
    fn test_nnp() {
        get_no_new_privs().unwrap();
    }

    #[test]
    fn test_keepcaps() {
        let old_keepcaps = get_keepcaps().unwrap();

        set_keepcaps(true).unwrap();
        assert!(get_keepcaps().unwrap());
        assert!(secbits::get().unwrap().contains(secbits::SecFlags::KEEP_CAPS));

        set_keepcaps(false).unwrap();
        assert!(!get_keepcaps().unwrap());
        assert!(!secbits::get().unwrap().contains(secbits::SecFlags::KEEP_CAPS));

        set_keepcaps(old_keepcaps).unwrap();
    }

    #[test]
    fn test_ambient() {
        ambient::probe().unwrap();
        assert!(ambient::is_supported());
    }

    #[test]
    fn test_bounding() {
        bounding::probe().unwrap();
        bounding::is_set(Cap::Chown).unwrap();
    }

    #[test]
    fn test_secbits() {
        secbits::get().unwrap();
    }
}
