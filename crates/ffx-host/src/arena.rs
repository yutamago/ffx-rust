//! The arena: the game's 32-bit address space (docs/host.md, "The arena").
//!
//! Every game address is a `u32` offset from [`Arena::base`]: the arena is
//! one 4 GB reservation and the base is wherever the OS put it (the host is
//! 64-bit only; the i686 build, whose arena was the process's own address
//! space, was dropped on 2026-10-31). Regions are mapped explicitly ([`Arena::map`]);
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
/// `log2(PAGE)`.
const PAGE_SHIFT: u32 = 12;
/// Words of [`Arena`]'s page bitmap (4 GB of pages, 64 a word).
const PAGE_WORDS: usize = (1 << (32 - PAGE_SHIFT)) / 64;

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
    /// One bit per page, set once the page is mapped (regions are whole
    /// pages and never unmapped): [`Arena::is_mapped`] without the lock,
    /// which every game memory access checks (m5-perf1: the lock was
    /// shared by every game thread's every read and write).
    pages: Box<[AtomicU64]>,
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
            .finish_non_exhaustive()
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
            pages: (0..PAGE_WORDS).map(|_| AtomicU64::new(0)).collect(),
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

    /// Starts watching the writes to `addr..addr + len` (the OS's write
    /// watch: Windows' `MEM_WRITE_WATCH`): [`Arena::written`] then names
    /// the pages written since. False where there is none (Linux).
    #[must_use]
    pub fn watch(&self, addr: u32, len: u32) -> bool {
        let (start, size) = pages_around(addr, len);
        sys::watch_reset(self.base, start, size)
    }

    /// The byte runs of `addr..addr + len` on pages written since
    /// [`Arena::watch`], as (offset from `addr`, length), in order;
    /// `None` when unknown (no write watch: every byte may be written).
    #[must_use]
    pub fn written(&self, addr: u32, len: u32) -> Option<Vec<(u32, u32)>> {
        let (start, size) = pages_around(addr, len);
        let pages = sys::written_pages(self.base, start, size)?;
        Some(runs(addr, len, &pages))
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
        let last = u32::try_from((end - 1) >> PAGE_SHIFT).unwrap_or(u32::MAX);
        for page in start / PAGE..=last {
            self.pages[(page / 64) as usize].fetch_or(1 << (page % 64), Ordering::Release);
        }
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
    /// aligned (Windows' allocation granularity).
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
                // past what holds the address
                Err(_) => {
                    at = sys::next_probe(at)
                        .max(at + GRANULE)
                        .next_multiple_of(GRANULE);
                }
            }
        }
        Err(Error::Map(from, size, "arena full".into()))
    }

    /// Whether `addr..addr + len` lies in mapped regions.
    #[must_use]
    #[inline]
    pub fn is_mapped(&self, addr: u32, len: usize) -> bool {
        if len == 0 {
            return true;
        }
        let last = u64::from(addr) + len as u64 - 1;
        if last >= 1 << 32 {
            return false;
        }
        let first = addr >> PAGE_SHIFT;
        #[allow(clippy::cast_possible_truncation)] // below 4 GB: checked
        let last = (last >> PAGE_SHIFT) as u32;
        (first..=last).all(|page| {
            self.pages[(page / 64) as usize].load(Ordering::Acquire) & (1 << (page % 64)) != 0
        })
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

impl Arena {
    /// Reads game memory at `addr` ([`ArenaMem`]'s read, for callers that
    /// hold the arena: no `Arc` to clone on the hot path).
    #[inline]
    pub fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        if self.is_mapped(addr, out.len()) {
            let src = self.ptr(addr);
            // SAFETY: inside mapped regions; `[u8; N]` has alignment 1. The
            // word sizes as single loads (no `memcpy` call for the ports'
            // scalar reads; m5-perf1).
            unsafe {
                match out.len() {
                    1 => out[0] = src.read(),
                    2 => out.copy_from_slice(&src.cast::<[u8; 2]>().read()),
                    4 => out.copy_from_slice(&src.cast::<[u8; 4]>().read()),
                    8 => out.copy_from_slice(&src.cast::<[u8; 8]>().read()),
                    n => std::ptr::copy_nonoverlapping(src, out.as_mut_ptr(), n),
                }
            }
        } else {
            crate::log::bad_access(addr, out.len(), false);
            out.fill(0);
        }
    }

    /// The game memory at `addr..addr + len` in place, when mapped (for a
    /// reader that hands it on at once, as the D3D11 layer's unmap: no
    /// copy). The game's threads may write it meanwhile, as they may
    /// write what [`Arena::read_bytes`] copies.
    #[must_use]
    pub fn slice(&self, addr: u32, len: u32) -> Option<&[u8]> {
        // SAFETY: inside mapped regions, which stay mapped while the arena
        // lives.
        self.is_mapped(addr, len as usize)
            .then(|| unsafe { std::slice::from_raw_parts(self.ptr(addr), len as usize) })
    }

    /// Writes game memory at `addr` ([`ArenaMem`]'s write).
    #[inline]
    pub fn write_bytes(&self, addr: u32, data: &[u8]) {
        if self.is_mapped(addr, data.len()) {
            let dst = self.ptr(addr);
            // SAFETY: inside mapped regions; alignment 1, as above.
            unsafe {
                match data.len() {
                    1 => dst.write(data[0]),
                    2 => dst.cast::<[u8; 2]>().write([data[0], data[1]]),
                    4 => dst
                        .cast::<[u8; 4]>()
                        .write([data[0], data[1], data[2], data[3]]),
                    8 => dst
                        .cast::<[u8; 8]>()
                        .write(data.try_into().unwrap_or([0; 8])),
                    n => std::ptr::copy_nonoverlapping(data.as_ptr(), dst, n),
                }
            }
        } else {
            crate::log::bad_access(addr, data.len(), true);
        }
    }
}

/// Game memory borrowed from the arena ([`crate::exec::Exec::mem`]): an
/// [`ArenaMem`] without the `Arc`, whose clone at every access was a
/// cache line all the game threads fought over (m5-perf1).
#[derive(Debug, Clone, Copy)]
pub struct ArenaRef<'a>(pub &'a Arena);

impl ffx_bridge::Mem for ArenaRef<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        self.0.write_bytes(addr, data);
    }
    fn image_base(&self) -> u32 {
        ffx_bridge::IMAGE_BASE
    }
}

impl ffx_bridge::Mem for ArenaMem {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.arena.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        self.arena.write_bytes(addr, data);
    }
    fn image_base(&self) -> u32 {
        ffx_bridge::IMAGE_BASE
    }
}

/// The whole pages around `addr..addr + len`: (start, size).
fn pages_around(addr: u32, len: u32) -> (u32, u32) {
    let page = 1u64 << PAGE_SHIFT;
    let start = u64::from(addr) & !(page - 1);
    let end = (u64::from(addr) + u64::from(len)).div_ceil(page) * page;
    #[allow(clippy::cast_possible_truncation)] // below 4 GB
    let r = (start as u32, (end - start).min(u64::from(u32::MAX)) as u32);
    r
}

/// The runs of `addr..addr + len` (offsets from `addr`) that the written
/// pages (their addresses, ascending) cover, adjacent pages joined.
fn runs(addr: u32, len: u32, pages: &[u32]) -> Vec<(u32, u32)> {
    let page = 1u64 << PAGE_SHIFT;
    let (lo, hi) = (u64::from(addr), u64::from(addr) + u64::from(len));
    let mut out: Vec<(u32, u32)> = Vec::new();
    for &p in pages {
        let a = u64::from(p).max(lo);
        let b = (u64::from(p) + page).min(hi);
        if a >= b {
            continue;
        }
        #[allow(clippy::cast_possible_truncation)] // offsets within `len`
        let (off, n) = ((a - lo) as u32, (b - a) as u32);
        match out.last_mut() {
            Some(last) if last.0 + last.1 == off => last.1 += n,
            _ => out.push((off, n)),
        }
    }
    out
}

/// The OS side: reserve, commit, release. The host's pages may be larger
/// than the game's 4 KB (16 KB on Apple silicon and some ARM64 Linux
/// kernels): a commit makes every host page it touches accessible
/// ([`host_span`]), so neighbouring game regions may share one; the arena's
/// own page bitmap still says which game pages are mapped.
///
/// No write watch off Windows: the D3D11 layer's discarding maps then
/// upload whole buffers instead of the pages written
/// (`Arena::written` answers `None`; docs/host.md, "Native platform").
/// Linux's soft-dirty bits are per process (clearing them for one buffer
/// clears them for all), and `userfaultfd`'s write protection costs a
/// fault per page, so neither replaces it.
#[cfg(all(unix, target_pointer_width = "64"))]
mod sys {
    use crate::{Error, Result};

    /// The host's page size.
    fn host_page() -> usize {
        static PAGE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        // SAFETY: a plain query.
        *PAGE.get_or_init(|| {
            usize::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) })
                .unwrap_or(0x1000)
                .max(0x1000)
        })
    }

    /// `start..start + size` widened to whole host pages of `page` bytes:
    /// the offset and length to `mprotect`.
    pub(super) fn host_span(start: u32, size: u32, page: usize) -> (usize, usize) {
        let lo = (start as usize) / page * page;
        let hi = (start as usize + size as usize).next_multiple_of(page);
        (lo, hi - lo)
    }

    /// No write watch here.
    pub(super) fn watch_reset(_: *mut u8, _: u32, _: u32) -> bool {
        false
    }

    pub(super) fn written_pages(_: *mut u8, _: u32, _: u32) -> Option<Vec<u32>> {
        None
    }

    /// Where to look next after a refused mapping at `at`.
    pub(super) fn next_probe(at: u64) -> u64 {
        at
    }

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
        let (at, len) = host_span(start, size, host_page());
        // SAFETY: inside the reservation (4 GB, a multiple of every page
        // size); anonymous pages read as zero, and pages already
        // accessible keep their bytes.
        let r =
            unsafe { libc::mprotect(base.add(at).cast(), len, libc::PROT_READ | libc::PROT_WRITE) };
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
    use std::sync::atomic::{AtomicBool, Ordering};

    use windows_sys::Win32::System::Memory::{
        GetWriteWatch, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_NOACCESS, PAGE_READWRITE,
        ResetWriteWatch, VirtualAlloc, VirtualFree,
    };

    use crate::{Error, Result};

    /// `MEM_WRITE_WATCH` (windows-sys has it under `SystemServices`).
    const MEM_WRITE_WATCH: u32 = 0x0020_0000;

    /// The reservation has the write watch (`MEM_WRITE_WATCH`).
    static WATCHED: AtomicBool = AtomicBool::new(false);

    pub(super) fn watch_reset(base: *mut u8, start: u32, size: u32) -> bool {
        // SAFETY: inside the reservation, which has the write watch.
        WATCHED.load(Ordering::Relaxed)
            && unsafe { ResetWriteWatch(base.add(start as usize).cast(), size as usize) } == 0
    }

    /// The pages of `start..start + size` written since the last reset,
    /// as arena addresses.
    pub(super) fn written_pages(base: *mut u8, start: u32, size: u32) -> Option<Vec<u32>> {
        if !WATCHED.load(Ordering::Relaxed) {
            return None;
        }
        let mut addrs =
            vec![std::ptr::null_mut::<core::ffi::c_void>(); (size as usize).div_ceil(0x1000)];
        let mut count = addrs.len();
        let mut granularity = 0u32;
        // SAFETY: inside the reservation; the array holds `count` entries.
        let r = unsafe {
            GetWriteWatch(
                0,
                base.add(start as usize).cast(),
                size as usize,
                addrs.as_mut_ptr(),
                &raw mut count,
                &raw mut granularity,
            )
        };
        if r != 0 || granularity != 0x1000 || count > addrs.len() {
            return None;
        }
        #[allow(clippy::cast_possible_truncation)] // offsets into the 4 GB arena
        let pages = addrs[..count]
            .iter()
            .map(|&p| (p as usize - base as usize) as u32)
            .collect();
        Some(pages)
    }

    /// Where to look next after a refused mapping at `at`.
    pub(super) fn next_probe(at: u64) -> u64 {
        at
    }

    pub(super) fn reserve() -> Result<*mut u8> {
        // SAFETY: a fresh reservation, with the write watch (the D3D11
        // layer's discarding maps upload only the pages the game wrote),
        // else without it.
        let mut p = unsafe {
            VirtualAlloc(
                std::ptr::null(),
                1 << 32,
                MEM_RESERVE | MEM_WRITE_WATCH,
                PAGE_NOACCESS,
            )
        };
        if p.is_null() {
            // SAFETY: as above.
            p = unsafe { VirtualAlloc(std::ptr::null(), 1 << 32, MEM_RESERVE, PAGE_NOACCESS) };
        } else {
            WATCHED.store(true, Ordering::Relaxed);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_pages_become_runs_inside_the_range() {
        assert_eq!(pages_around(0x1234, 0x10), (0x1000, 0x1000));
        // 16 KB host pages: a 4 KB region at 0x5000 opens 0x4000..0x8000
        #[cfg(all(unix, target_pointer_width = "64"))]
        {
            assert_eq!(sys::host_span(0x5000, 0x1000, 0x4000), (0x4000, 0x4000));
            assert_eq!(sys::host_span(0x3000, 0x2000, 0x4000), (0, 0x8000));
            assert_eq!(sys::host_span(0x5000, 0x1000, 0x1000), (0x5000, 0x1000));
            assert_eq!(
                sys::host_span(0xffff_f000, 0x1000, 0x4000),
                (0xffff_c000, 0x4000)
            );
        }
        assert_eq!(pages_around(0x1ff0, 0x20), (0x1000, 0x2000));
        // the range 0x1800..0x5800; pages 0x1000, 0x2000 (joined), 0x4000, 0x5000
        let r = runs(0x1800, 0x4000, &[0x1000, 0x2000, 0x4000, 0x5000]);
        assert_eq!(r, vec![(0, 0x1800), (0x2800, 0x1800)]);
        assert_eq!(runs(0x1800, 0x100, &[0x3000]), []);
    }

    #[test]
    fn mapped_pages_are_seen_without_the_lock() {
        let a = Arena::new().unwrap();
        a.map(0x0040_0000, 0x2000, "a").unwrap();
        a.map(0x0040_2000, 0x1000, "b").unwrap();
        a.map(0xffff_f000, 0x1000, "top").unwrap();
        assert!(a.is_mapped(0x0040_0000, 0x3000));
        assert!(a.is_mapped(0x0040_1ffe, 4), "across two regions");
        assert!(a.is_mapped(0x0040_2ffc, 4));
        assert!(!a.is_mapped(0x0040_2ffe, 4), "past the end");
        assert!(!a.is_mapped(0x003f_fffe, 4), "before the start");
        assert!(a.is_mapped(0x0050_0000, 0), "nothing to read");
        assert!(a.is_mapped(0xffff_fffc, 4), "the last word");
        assert!(!a.is_mapped(0xffff_fffe, 4), "past 4 GB");
        let mut w = [0u8; 4];
        a.write_bytes(0x0040_1ffe, &[1, 2, 3, 4]);
        a.read_bytes(0x0040_1ffe, &mut w);
        assert_eq!(w, [1, 2, 3, 4]);
        a.read_bytes(0x0040_2ffe, &mut w);
        assert_eq!(w, [0; 4], "an unmapped read is zeros");
    }
}
