//! `snapshot <path>`: writes the game's memory to a file
//! (`ffx_bridge::snapshot`), on the game thread between two frames, so
//! `ffx-oracle` can run original and Rust functions on the same state.
//!
//! Taken: every committed, readable region that is private memory (heaps,
//! stacks, `VirtualAlloc`), plus `FFX.exe`'s own image (its `.data` is the
//! game's globals). Other images and mapped files are left out. Other game
//! threads keep running meanwhile, so what they write during the copy may be
//! torn.

use std::ffi::c_void;
use std::io::{BufWriter, Write};

use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_IMAGE, MEM_PRIVATE, MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_NOACCESS,
    PAGE_NOCACHE, PAGE_WRITECOMBINE, VirtualQuery,
};

use ffx_bridge::snapshot::{Snapshot, write_region};

/// The x87 control word of the calling thread.
#[must_use]
pub fn fpu_control() -> u32 {
    let mut cw: u16 = 0;
    // SAFETY: fnstcw stores two bytes at the given address.
    unsafe { core::arch::asm!("fnstcw [{}]", in(reg) &raw mut cw, options(nostack)) };
    u32::from(cw)
}

/// Writes the snapshot; returns the number of regions and bytes taken.
///
/// # Errors
/// When the file cannot be written.
pub fn take(path: &str) -> Result<(u32, u64), String> {
    let image = crate::rebase(crate::IMAGE_BASE);
    let image_end = image + image_size(image);
    let mut regions = Vec::new();
    let mut at: usize = 0x1_0000;
    loop {
        let mut info: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: VirtualQuery only fills `info`.
        let n = unsafe {
            VirtualQuery(
                at as *const c_void,
                &raw mut info,
                size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        if n == 0 {
            break;
        }
        let base = info.BaseAddress as usize;
        let size = info.RegionSize;
        // Write-combined and uncached pages are the GPU's, not game state,
        // and slow (or unsafe) to read.
        let readable = info.State == MEM_COMMIT
            && info.Protect & (PAGE_NOACCESS | PAGE_GUARD | PAGE_NOCACHE | PAGE_WRITECOMBINE) == 0
            && info.Protect != 0;
        let ours = info.Type == MEM_PRIVATE
            || (info.Type == MEM_IMAGE && base >= image && base < image_end);
        if readable && ours {
            regions.push((base, size, info.Protect, info.Type));
        }
        match base.checked_add(size) {
            Some(next) if next > at => at = next,
            _ => break,
        }
    }
    let file = std::fs::File::create(path).map_err(|e| format!("{path}: {e}"))?;
    let mut w = BufWriter::new(file);
    let count = u32::try_from(regions.len()).map_err(|e| e.to_string())?;
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    Snapshot::write_header(&mut w, image as u32, fpu_control(), count)
        .map_err(|e| e.to_string())?;
    let mut total = 0u64;
    let mut bytes = Vec::new();
    for (base, size, protect, kind) in regions {
        // Copied with ReadProcessMemory, which fails instead of faulting
        // when another thread frees or protects a page meanwhile; such
        // pages are left zero.
        bytes.clear();
        bytes.resize(size, 0);
        for off in (0..size).step_by(0x10_0000) {
            let n = (size - off).min(0x10_0000);
            if crate::harness::read_memory(base + off, n)
                .map(|v| bytes[off..off + n].copy_from_slice(&v))
                .is_err()
            {
                for page in (off..off + n).step_by(0x1000) {
                    if let Ok(v) = crate::harness::read_memory(base + page, 0x1000) {
                        bytes[page..page + 0x1000].copy_from_slice(&v);
                    }
                }
            }
        }
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        write_region(&mut w, base as u32, protect, kind, &bytes).map_err(|e| e.to_string())?;
        total += size as u64;
    }
    w.flush().map_err(|e| e.to_string())?;
    Ok((count, total))
}

fn image_size(image: usize) -> usize {
    // SAFETY: the image's own PE headers.
    unsafe {
        let pe = image + *((image + 0x3c) as *const u32) as usize;
        *((pe + 24 + 56) as *const u32) as usize
    }
}
