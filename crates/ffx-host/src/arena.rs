//! The arena: the game's 32-bit address space (docs/host.md, "The arena").
//!
//! Every game address is a `u32` offset from [`Arena::base`]. On `x86_64` the
//! arena is one 4 GB reservation and the base is wherever the OS put it; on
//! i686 the arena is the process's own address space (base 0) and each
//! region is allocated at its game address, so a game address is also a
//! host pointer there. Regions are mapped explicitly ([`Arena::map`]);
//! anything else stays unmapped, so a stray access is caught (by
//! [`Arena::read_bytes`]'s check, or by Unicorn's unmapped-memory hook).
//!
//! The same host memory backs the Unicorn fallback (`mem_map_ptr`), so the
//! Rust ports and the original code see one memory.

use std::sync::Mutex;

use crate::{Error, Result};

/// One page.
pub const PAGE: u32 = 0x1000;

/// A mapped range of the arena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// First address.
    pub start: u32,
    /// Size in bytes (whole pages).
    pub size: u32,
    /// What it holds, for logs.
    pub name: String,
}

impl Region {
    /// One past the last address, as `u64` (a region may end at 4 GB).
    #[must_use]
    pub fn end(&self) -> u64 {
        u64::from(self.start) + u64::from(self.size)
    }
}

/// The game's address space.
pub struct Arena {
    base: *mut u8,
    regions: Mutex<Vec<Region>>,
}

// SAFETY: the arena's memory is plain bytes shared by the game's threads,
// as the original process's memory is; synchronisation is the game's own.
unsafe impl Send for Arena {}
// SAFETY: as above.
unsafe impl Sync for Arena {}

impl std::fmt::Debug for Arena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arena")
            .field("base", &self.base)
            .field("regions", &self.regions())
            .finish()
    }
}

impl Arena {
    /// Reserves the arena (nothing mapped yet).
    ///
    /// # Errors
    /// When the OS refuses the 4 GB reservation (`x86_64`).
    pub fn new() -> Result<Self> {
        Ok(Self {
            base: sys::reserve()?,
            regions: Mutex::new(Vec::new()),
        })
    }

    /// The host address of game address 0.
    #[must_use]
    pub fn base(&self) -> *mut u8 {
        self.base
    }

    /// The host pointer of game address `addr`.
    #[must_use]
    pub fn ptr(&self, addr: u32) -> *mut u8 {
        self.base.wrapping_add(addr as usize)
    }

    /// The mapped regions, sorted by address.
    #[must_use]
    pub fn regions(&self) -> Vec<Region> {
        self.regions.lock().map(|r| r.clone()).unwrap_or_default()
    }

    /// Maps `size` bytes (rounded up to pages) at `start`, zeroed.
    ///
    /// # Errors
    /// When the range overlaps a mapped region, is not page-aligned or the
    /// OS refuses it.
    pub fn map(&self, start: u32, size: u32, name: &str) -> Result<()> {
        if !start.is_multiple_of(PAGE) || size == 0 {
            return Err(Error::Map(start, size, "not page-aligned".into()));
        }
        let size = size.next_multiple_of(PAGE);
        let end = u64::from(start) + u64::from(size);
        if end > 1 << 32 {
            return Err(Error::Map(start, size, "past 4 GB".into()));
        }
        let mut regions = self
            .regions
            .lock()
            .map_err(|_| Error::Map(start, size, "lock".into()))?;
        if let Some(r) = regions
            .iter()
            .find(|r| u64::from(r.start) < end && u64::from(start) < r.end())
        {
            return Err(Error::Map(start, size, format!("overlaps {}", r.name)));
        }
        sys::commit(self.base, start, size)?;
        regions.push(Region {
            start,
            size,
            name: name.to_string(),
        });
        regions.sort_by_key(|r| r.start);
        Ok(())
    }

    /// Maps `size` bytes in the first free gap at or above `from`.
    ///
    /// # Errors
    /// When no gap is large enough.
    pub fn map_anywhere(&self, from: u32, size: u32, name: &str) -> Result<u32> {
        let size = size.next_multiple_of(PAGE);
        let mut at = u64::from(from.next_multiple_of(PAGE));
        for r in self.regions() {
            if u64::from(r.start) >= at + u64::from(size) {
                break;
            }
            if r.end() > at {
                at = r.end();
            }
        }
        let start = u32::try_from(at).map_err(|_| Error::Map(from, size, "arena full".into()))?;
        self.map(start, size, name)?;
        Ok(start)
    }

    /// Whether `addr..addr + len` lies in mapped regions.
    #[must_use]
    pub fn is_mapped(&self, addr: u32, len: usize) -> bool {
        let Ok(regions) = self.regions.lock() else {
            return false;
        };
        let mut at = u64::from(addr);
        let end = at + len as u64;
        while at < end {
            let i = regions.partition_point(|r| r.end() <= at);
            match regions.get(i) {
                Some(r) if u64::from(r.start) <= at => at = r.end(),
                _ => return false,
            }
        }
        true
    }

    /// Copies `data` into the arena at `addr` (no mapping check: the
    /// loader's own writes).
    pub(crate) fn fill(&self, addr: u32, data: &[u8]) {
        debug_assert!(self.is_mapped(addr, data.len()));
        // SAFETY: the loader writes inside regions it mapped itself.
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr(addr), data.len()) }
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        let regions = self.regions();
        sys::release(self.base, &regions);
    }
}

/// Game memory for the ports ([`ffx_bridge::Mem`]): every access goes
/// through the arena's base. Unmapped accesses read zeros and drop writes,
/// and are counted, instead of faulting the host process.
#[derive(Debug, Clone)]
pub struct ArenaMem {
    /// The arena.
    pub arena: std::sync::Arc<Arena>,
}

impl ffx_bridge::Mem for ArenaMem {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        if self.arena.is_mapped(addr, out.len()) {
            // SAFETY: inside mapped regions.
            unsafe {
                std::ptr::copy_nonoverlapping(self.arena.ptr(addr), out.as_mut_ptr(), out.len());
            }
        } else {
            crate::log::bad_access(addr, out.len(), false);
            out.fill(0);
        }
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        if self.arena.is_mapped(addr, data.len()) {
            // SAFETY: inside mapped regions.
            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr(), self.arena.ptr(addr), data.len());
            }
        } else {
            crate::log::bad_access(addr, data.len(), true);
        }
    }
    fn image_base(&self) -> u32 {
        ffx_bridge::IMAGE_BASE
    }
}

/// The OS side: reserve, commit, release.
#[cfg(all(unix, target_pointer_width = "64"))]
mod sys {
    use crate::{Error, Result};

    const SIZE: usize = 1 << 32;

    pub(super) fn reserve() -> Result<*mut u8> {
        // SAFETY: a fresh anonymous reservation, inaccessible until committed.
        let p = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                SIZE,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            )
        };
        if p == libc::MAP_FAILED {
            return Err(Error::Map(0, 0, "4 GB reservation refused".into()));
        }
        Ok(p.cast())
    }

    pub(super) fn commit(base: *mut u8, start: u32, size: u32) -> Result<()> {
        // SAFETY: inside the reservation; anonymous pages read as zero.
        let r = unsafe {
            libc::mprotect(
                base.add(start as usize).cast(),
                size as usize,
                libc::PROT_READ | libc::PROT_WRITE,
            )
        };
        if r == 0 {
            Ok(())
        } else {
            Err(Error::Map(start, size, "mprotect refused".into()))
        }
    }

    pub(super) fn release(base: *mut u8, _: &[super::Region]) {
        // SAFETY: the whole reservation, mapped once by `reserve`.
        unsafe {
            libc::munmap(base.cast(), SIZE);
        }
    }
}

/// The OS side on 64-bit Windows: one 4 GB reservation.
#[cfg(all(windows, target_pointer_width = "64"))]
mod sys {
    use windows_sys::Win32::System::Memory::{
        MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_NOACCESS, PAGE_READWRITE, VirtualAlloc,
        VirtualFree,
    };

    use crate::{Error, Result};

    pub(super) fn reserve() -> Result<*mut u8> {
        // SAFETY: a fresh reservation.
        let p = unsafe { VirtualAlloc(std::ptr::null(), 1 << 32, MEM_RESERVE, PAGE_NOACCESS) };
        if p.is_null() {
            return Err(Error::Map(0, 0, "4 GB reservation refused".into()));
        }
        Ok(p.cast())
    }

    pub(super) fn commit(base: *mut u8, start: u32, size: u32) -> Result<()> {
        // SAFETY: inside the reservation.
        let p = unsafe {
            VirtualAlloc(
                base.add(start as usize).cast(),
                size as usize,
                MEM_COMMIT,
                PAGE_READWRITE,
            )
        };
        if p.is_null() {
            Err(Error::Map(start, size, "commit refused".into()))
        } else {
            Ok(())
        }
    }

    pub(super) fn release(base: *mut u8, _: &[super::Region]) {
        // SAFETY: the reservation made by `reserve`.
        unsafe {
            VirtualFree(base.cast(), 0, MEM_RELEASE);
        }
    }
}

/// The OS side on 32-bit Windows: no reservation; the arena is the
/// process's address space (base 0) and each region is allocated at its
/// game address. `ffx-rust.exe` links at a high base so the image range
/// (0x00400000..0x0277d000) stays free (docs/host.md).
#[cfg(all(windows, target_pointer_width = "32"))]
mod sys {
    use windows_sys::Win32::System::Memory::{
        MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc, VirtualFree,
    };

    use crate::{Error, Result};

    pub(super) fn reserve() -> Result<*mut u8> {
        Ok(std::ptr::null_mut())
    }

    pub(super) fn commit(_: *mut u8, start: u32, size: u32) -> Result<()> {
        // SAFETY: a fresh allocation at a fixed address; refused if taken.
        let p = unsafe {
            VirtualAlloc(
                start as usize as *const _,
                size as usize,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_READWRITE,
            )
        };
        if p as usize == start as usize {
            Ok(())
        } else {
            Err(Error::Map(
                start,
                size,
                "address taken in this process".into(),
            ))
        }
    }

    pub(super) fn release(_: *mut u8, regions: &[super::Region]) {
        for r in regions {
            // SAFETY: each region was allocated by `commit`.
            unsafe {
                VirtualFree(r.start as usize as *mut _, 0, MEM_RELEASE);
            }
        }
    }
}
