use std::cmp::Ordering;
use std::io;
use std::ops::{BitAnd, BitOr, BitXor};

use crate::PidT;

#[derive(Clone, Debug)]
pub struct CpuSet {
    bits: Vec<usize>,
}

impl CpuSet {
    fn with_len(len: usize) -> Self {
        Self { bits: vec![0; len] }
    }

    fn calc_len(ncpus: usize) -> usize {
        if ncpus == 0 {
            0
        } else if ncpus <= 1024 {
            1024 / (std::mem::size_of::<usize>() * 8)
        } else {
            div_round_up(
                div_round_up(ncpus, 8),
                std::mem::size_of::<usize>(),
            )
        }
    }

    #[inline]
    fn calc_idx_mask(cpu: usize) -> (usize, usize) {
        let (idx, shift) = divmod(cpu, 8 * std::mem::size_of::<usize>());
        (idx, 1 << shift)
    }

    /// Creates a new `CpuSet` capable of holding at least 1024 CPUs.
    #[inline]
    pub fn empty() -> Self {
        Self::with_len(1024 / (std::mem::size_of::<usize>() * 8))
    }

    /// Creates a new `CpuSet` capable of holding at least `ncpus` CPUs.
    #[inline]
    pub fn empty_ncpus(ncpus: usize) -> Self {
        Self::with_len(Self::calc_len(ncpus))
    }

    /// Resizes this `CpuSet` such that it can hold at least `ncpus` CPUs.
    /// This may result in truncation.
    ///
    /// # Truncation
    ///
    /// If the `CpuSet` was previously capable of holding more than `ncpus`
    /// CPUs, this method may or may not truncate it. The exact behavior varies
    /// on 32-bit vs 64-bit platforms.
    pub fn resize(&mut self, ncpus: usize) {
        self.bits.resize(Self::calc_len(ncpus), 0);
    }

    /// Returns the actual maximum number of CPUs that this `CpuSet` can hold.
    pub fn max_ncpus(&self) -> usize {
        self.bits.len() * 8 * std::mem::size_of::<usize>()
    }

    pub fn clear(&mut self) {
        for elem in self.bits.iter_mut() {
            *elem = 0;
        }
    }

    pub fn add(&mut self, cpu: usize) {
        let (idx, mask) = Self::calc_idx_mask(cpu);
        self.bits[idx] |= mask;
    }

    pub fn remove(&mut self, cpu: usize) {
        let (idx, mask) = Self::calc_idx_mask(cpu);
        self.bits[idx] &= !mask;
    }

    pub fn has(&self, cpu: usize) -> bool {
        let (idx, mask) = Self::calc_idx_mask(cpu);
        self.bits[idx] & mask != 0
    }

    pub fn count(&self) -> usize {
        let mut res = 0;
        for item in &self.bits {
            res += item.count_ones() as usize;
        }
        res
    }
}

impl PartialEq for CpuSet {
    fn eq(&self, other: &Self) -> bool {
        let self_len = self.bits.len();
        let other_len = other.bits.len();

        match self_len.cmp(&other_len) {
            Ordering::Greater => {
                &self.bits[..other_len] == other.bits.as_slice() && self.bits[other_len..].iter().all(|x| *x == 0)
            }
            Ordering::Less => {
                self.bits.as_slice() == &other.bits[..self_len] && other.bits[self_len..].iter().all(|x| *x == 0)
            }
            Ordering::Equal => {
                self.bits == other.bits
            }
        }
    }
}

impl Eq for CpuSet {}

impl BitAnd for CpuSet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        let len = std::cmp::min(self.bits.len(), rhs.bits.len());

        let mut res = Self::with_len(len);
        for i in 0..len {
            res.bits[i] = self.bits[i] & rhs.bits[i];
        }

        res
    }
}

impl BitOr for CpuSet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        let minlen = std::cmp::min(self.bits.len(), rhs.bits.len());
        let maxlen = std::cmp::max(self.bits.len(), rhs.bits.len());

        let mut res = Self::with_len(maxlen);
        for i in 0..minlen {
            res.bits[i] = self.bits[i] | rhs.bits[i];
        }

        if minlen != maxlen {
            res.bits[minlen..maxlen].clone_from_slice(
                if self.bits.len() > rhs.bits.len() {
                    &self.bits[minlen..maxlen]
                } else {
                    &rhs.bits[minlen..maxlen]
                }
            );
        }

        res
    }
}

impl BitXor for CpuSet {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self {
        let minlen = std::cmp::min(self.bits.len(), rhs.bits.len());
        let maxlen = std::cmp::max(self.bits.len(), rhs.bits.len());

        let mut res = Self::with_len(maxlen);
        for i in 0..minlen {
            res.bits[i] = self.bits[i] ^ rhs.bits[i];
        }

        if minlen != maxlen {
            res.bits[minlen..maxlen].clone_from_slice(
                if self.bits.len() > rhs.bits.len() {
                    &self.bits[minlen..maxlen]
                } else {
                    &rhs.bits[minlen..maxlen]
                }
            );
        }

        res
    }
}

fn divmod(a: usize, b: usize) -> (usize, usize) {
    (a / b, a % b)
}

fn div_round_up(a: usize, b: usize) -> usize {
    let (quotient, remainder) = divmod(a, b);
    if remainder == 0 { quotient } else { quotient + 1 }
}

pub fn getaffinity_raw(pid: PidT, mask: &mut [usize]) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe {
        libc::sched_getaffinity(
            pid,
            mask.len(),
            mask.as_mut_ptr() as *mut libc::c_void as *mut libc::cpu_set_t,
        )
    })
}

pub fn getaffinity(pid: PidT) -> io::Result<CpuSet> {
    let mut cpuset = CpuSet::empty();

    loop {
        match getaffinity_raw(pid, cpuset.bits.as_mut_slice()) {
            Ok(()) => return Ok(cpuset),
            Err(e) if crate::error::is_einval(&e) => {
                cpuset.resize(cpuset.max_ncpus() * 2);
            }
            Err(e) => return Err(e),
        }
    }
}

pub fn setaffinity_raw(pid: PidT, mask: &[usize]) -> io::Result<()> {
    crate::error::convert_nzero_ret(unsafe {
        libc::sched_setaffinity(
            pid,
            mask.len(),
            mask.as_ptr() as *const libc::c_void as *const libc::cpu_set_t,
        )
    })
}

pub fn setaffinity(pid: PidT, cpuset: &CpuSet) -> io::Result<()> {
    setaffinity_raw(pid, cpuset.bits.as_slice())
}

#[cfg(test)]
#[allow(clippy::redundant_clone)]
mod tests {
    use super::*;
    
    #[test]
    fn test_calc_len() {
        let s = std::mem::size_of::<usize>();

        assert_eq!(CpuSet::calc_len(0), 0);
        assert_eq!(CpuSet::calc_len(1), 128 / s);
        assert_eq!(CpuSet::calc_len(1024), 128 / s);

        if s == 8 {
            assert_eq!(CpuSet::calc_len(1025), 136 / s);
            assert_eq!(CpuSet::calc_len(1032), 136 / s);
            assert_eq!(CpuSet::calc_len(1088), 136 / s);
            assert_eq!(CpuSet::calc_len(1089), 144 / s);
        } else {
            assert_eq!(CpuSet::calc_len(1025), 132 / s);
            assert_eq!(CpuSet::calc_len(1032), 132 / s);
            assert_eq!(CpuSet::calc_len(1088), 136 / s);
            assert_eq!(CpuSet::calc_len(1089), 140 / s);
        }
    }

    #[test]
    fn test_cpuset_basic() {
        let mut set = CpuSet::empty();
        assert!(!set.has(0));
        assert_eq!(set.count(), 0);

        set.add(0);
        assert!(set.has(0));
        assert_eq!(set.count(), 1);

        set.add(1);
        assert!(set.has(0));
        assert!(set.has(1));
        assert_eq!(set.count(), 2);

        set.add(8);
        assert!(set.has(0));
        assert!(set.has(1));
        assert!(set.has(8));
        assert_eq!(set.count(), 3);

        set.add(64);
        assert!(set.has(0));
        assert!(set.has(1));
        assert!(set.has(8));
        assert!(set.has(64));
        assert_eq!(set.count(), 4);

        set.remove(8);
        assert!(!set.has(8));
        assert_eq!(set.count(), 3);

        set.clear();
        assert!(!set.has(0));
        assert_eq!(set.count(), 0);
    }

    #[test]
    fn test_cpuset_eq() {
        assert_eq!(CpuSet::empty(), CpuSet::empty());
        assert_eq!(CpuSet::empty_ncpus(0), CpuSet::empty());
        assert_eq!(CpuSet::empty(), CpuSet::empty_ncpus(0));
    }

    #[test]
    fn test_cpuset_bitand() {
        let mut a = CpuSet::empty();
        let mut b = CpuSet::empty();
        let mut c = CpuSet::empty();
        assert_eq!(a.clone() & b.clone(), c.clone());

        a.add(1);
        assert_eq!(a.clone() & b.clone(), c.clone());

        b.add(1);
        c.add(1);
        assert_eq!(a.clone() & b.clone(), c.clone());
    }

    #[test]
    fn test_cpuset_bitor() {
        let mut a = CpuSet::empty();
        let mut b = CpuSet::empty();
        let mut c = CpuSet::empty();
        assert_eq!(a.clone() | b.clone(), c.clone());

        a.add(1);
        c.add(1);
        assert_eq!(a.clone() | b.clone(), c.clone());

        b.add(1);
        assert_eq!(a.clone() | b.clone(), c.clone());

        b.add(2);
        c.add(2);
        assert_eq!(a.clone() | b.clone(), c.clone());
    }

    #[test]
    fn test_cpuset_bitxor() {
        let mut a = CpuSet::empty();
        let mut b = CpuSet::empty();
        let mut c = CpuSet::empty();
        assert_eq!(a.clone() ^ b.clone(), c.clone());

        a.add(1);
        c.add(1);
        assert_eq!(a.clone() ^ b.clone(), c.clone());

        b.add(1);
        c.remove(1);
        assert_eq!(a.clone() ^ b.clone(), c.clone());

        b.add(2);
        c.add(2);
        assert_eq!(a.clone() ^ b.clone(), c.clone());
    }

    #[test]
    fn test_get_set_affinity() {
        let affinity = getaffinity(0).unwrap();
        setaffinity(0, &affinity).unwrap();
    }
}
