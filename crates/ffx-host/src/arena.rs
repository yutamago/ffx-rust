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
use std::sync::atomic::{AtomicU64, Ordering};

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
    /// Bumped by every [`Arena::map`]: each thread's emulator maps the
    /// regions it hasn't seen when this moved (docs/host.md, "Threads").
    generation: AtomicU64,
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
            .field("generation", &self.generation())
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
            generation: AtomicU64::new(0),
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

    /// How many times a region was mapped so far.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
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
        self.generation.fetch_add(1, Ordering::Release);
        Ok(())
    }

    /// Maps `size` bytes in the first free gap at or above `from`, 64 KB
    /// aligned (Windows' allocation granularity). On i686 an address the
    /// process already uses (a DLL, the host heap) is skipped.
    ///
    /// # Errors
    /// When no gap is large enough.
    pub fn map_anywhere(&self, from: u32, size: u32, name: &str) -> Result<u32> {
        const GRANULE: u64 = 0x1_0000;
        let size = size.next_multiple_of(PAGE);
        let mut at = u64::from(from).next_multiple_of(GRANULE);
        while at + u64::from(size) <= 1 << 32 {
            for r in self.regions() {
                if u64::from(r.start) >= at + u64::from(size) {
                    break;
                }
                if r.end() > at {
                    at = r.end().next_multiple_of(GRANULE);
                }
            }
            let Ok(start) = u32::try_from(at) else { break };
            match self.map(start, size, name) {
                Ok(()) => return Ok(start),
                Err(_) => at += GRANULE,
            }
        }
        Err(Error::Map(from, size, "arena full".into()))
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

    #[allow(clippy::unnecessary_wraps)] // the same signature on every target
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
            return Ok(());
        }
        if super::in_image_reserve(start, size) {
            // inside the range `reserve_image_range` kept at start-up
            // SAFETY: committing pages of our own reservation.
            let p = unsafe {
                VirtualAlloc(
                    start as usize as *const _,
                    size as usize,
                    MEM_COMMIT,
                    PAGE_READWRITE,
                )
            };
            if p as usize == start as usize {
                return Ok(());
            }
        }
        Err(Error::Map(start, size, holder(start, size)))
    }

    /// What holds the first taken page of `start..start + size`, for the
    /// error.
    pub(super) fn holder(start: u32, size: u32) -> String {
        use windows_sys::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, VirtualQuery};
        let mut at = start as usize;
        let end = start as usize + size as usize;
        while at < end {
            // SAFETY: VirtualQuery only reads the address space's layout.
            let mut m: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
            // SAFETY: `m` is a local of the size passed.
            let n = unsafe {
                VirtualQuery(
                    at as *const _,
                    &raw mut m,
                    std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            };
            if n == 0 {
                break;
            }
            if m.State != windows_sys::Win32::System::Memory::MEM_FREE {
                return format!(
                    "address taken in this process: {:#x}+{:#x} (allocation {:#x}, state {:#x}, type {:#x})",
                    m.BaseAddress as usize,
                    m.RegionSize,
                    m.AllocationBase as usize,
                    m.State,
                    m.Type
                );
            }
            at = m.BaseAddress as usize + m.RegionSize.max(0x1000);
        }
        "address taken in this process".into()
    }

    /// Whether `start..start + size` is one reservation of this process
    /// starting at `start` (the parent's, [`crate::relaunch`]).
    pub(super) fn reserved_whole(start: u32, size: u32) -> bool {
        use windows_sys::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, VirtualQuery};
        // SAFETY: a plain query into a local.
        let mut m: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: as above.
        let n = unsafe {
            VirtualQuery(
                start as usize as *const _,
                &raw mut m,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        n != 0
            && m.AllocationBase as usize == start as usize
            && m.State == MEM_RESERVE
            && m.RegionSize >= size as usize
    }

    /// Reserves `start..start + size` (no commit); `false` when taken.
    pub(super) fn reserve_range(start: u32, size: u32) -> bool {
        // SAFETY: a reservation at a fixed address; refused if taken.
        let p = unsafe {
            VirtualAlloc(
                start as usize as *const _,
                size as usize,
                MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        p as usize == start as usize
    }

    pub(super) fn release(_: *mut u8, regions: &[super::Region]) {
        for r in regions {
            if super::in_image_reserve(r.start, r.size) {
                continue;
            }
            // SAFETY: each region was allocated by `commit`.
            unsafe {
                VirtualFree(r.start as usize as *mut _, 0, MEM_RELEASE);
            }
        }
    }
}

/// `FFX.exe`'s range in the arena: `0x00400000..0x0277d000`.
pub const IMAGE_RANGE: (u32, u32) = (0x0040_0000, 0x0277_d000);

/// Whether [`reserve_image_range`] holds `start..start + size`.
#[cfg(all(windows, target_pointer_width = "32"))]
fn in_image_reserve(start: u32, size: u32) -> bool {
    IMAGE_RESERVED.load(Ordering::Relaxed)
        && start >= IMAGE_RANGE.0
        && u64::from(start) + u64::from(size) <= u64::from(IMAGE_RANGE.1)
}

#[cfg(all(windows, target_pointer_width = "32"))]
static IMAGE_RESERVED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// On i686 Windows the arena is the process's own address space, so
/// `FFX.exe`'s range must still be free when the image is mapped: the
/// program calls this first thing in `main`, before its heap or threads
/// grow into it (docs/host.md, "The arena"). A process started by
/// [`crate::relaunch`] finds the range reserved by its parent
/// (`FFX_HOST_IMAGE_RESERVED=1`) and adopts it. Elsewhere it does nothing.
///
/// # Errors
/// When the range is already taken (the error names what holds it).
pub fn reserve_image_range() -> Result<()> {
    #[cfg(all(windows, target_pointer_width = "32"))]
    {
        let (start, end) = IMAGE_RANGE;
        let adopted = std::env::var("FFX_HOST_IMAGE_RESERVED").is_ok_and(|v| v == "1")
            && sys::reserved_whole(start, end - start);
        if !adopted && !sys::reserve_range(start, end - start) {
            return Err(Error::Map(
                start,
                end - start,
                sys::holder(start, end - start),
            ));
        }
        IMAGE_RESERVED.store(true, Ordering::Relaxed);
    }
    Ok(())
}
