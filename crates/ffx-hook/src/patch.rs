//! Writing the patches.

use std::ffi::c_void;
use std::sync::atomic::Ordering;

use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualAlloc, VirtualProtect,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::{GAME_BASE, Hook, IMAGE_BASE, Outcome, SIZE_OF_IMAGE, TIME_DATE_STAMP, WrongBuild};

/// Checks that the PE image at `base` is the supported `FFX.exe` build.
///
/// # Errors
/// Returns the build's identifiers when they do not match.
///
/// # Safety
/// `base` must be the base address of a mapped PE32 image.
pub unsafe fn check_build(base: usize) -> Result<(), WrongBuild> {
    // SAFETY: the caller guarantees a mapped PE image; e_lfanew is at 0x3c,
    // and TimeDateStamp and SizeOfImage sit at fixed offsets in the NT headers.
    let (time_date_stamp, size_of_image) = unsafe {
        let nt = base + (base as *const u8).add(0x3c).cast::<u32>().read_unaligned() as usize;
        (
            (nt as *const u8).add(0x08).cast::<u32>().read_unaligned(),
            (nt as *const u8).add(0x50).cast::<u32>().read_unaligned(),
        )
    };
    if time_date_stamp == TIME_DATE_STAMP && size_of_image == SIZE_OF_IMAGE {
        Ok(())
    } else {
        Err(WrongBuild {
            time_date_stamp,
            size_of_image,
        })
    }
}

/// Installs every hook not named in `disabled` into the module at `base`,
/// and remembers `base` for [`crate::rebase`].
///
/// # Errors
/// Refuses to patch anything if the module is not the supported build.
///
/// # Safety
/// `base` must be a mapped `FFX.exe` image, and no other thread may be
/// executing the patched functions while this runs.
pub unsafe fn install(
    base: usize,
    hooks: &[Hook],
    disabled: &[&str],
) -> Result<Vec<(&'static str, Outcome)>, WrongBuild> {
    // SAFETY: forwarded from the caller.
    unsafe { check_build(base)? };
    GAME_BASE.store(base, Ordering::Release);
    Ok(hooks
        .iter()
        .map(|hook| {
            let outcome = if disabled.contains(&hook.name) {
                Outcome::Disabled
            } else {
                // SAFETY: the build check passed, so the site is inside the
                // image; forwarded thread-safety requirement.
                unsafe { patch(base, hook) }
            };
            (hook.name, outcome)
        })
        .collect())
}

/// Builds `stolen bytes; jmp site + len` in fresh executable memory.
unsafe fn trampoline(site: usize, stolen: &[u8]) -> Option<usize> {
    // SAFETY: plain allocation call.
    let mem = unsafe {
        VirtualAlloc(
            std::ptr::null(),
            stolen.len() + 5,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        )
    };
    if mem.is_null() {
        return None;
    }
    let tramp = mem as usize;
    let back = (site + stolen.len()).wrapping_sub(tramp + stolen.len() + 5) as u32;
    // SAFETY: `mem` is writable for `stolen.len() + 5` bytes.
    unsafe {
        let p = mem.cast::<u8>();
        stolen.as_ptr().copy_to_nonoverlapping(p, stolen.len());
        p.add(stolen.len()).write(0xe9);
        let back = back.to_le_bytes();
        back.as_ptr()
            .copy_to_nonoverlapping(p.add(stolen.len() + 1), 4);
    }
    Some(tramp)
}

unsafe fn patch(base: usize, hook: &Hook) -> Outcome {
    let site = base + (hook.address - IMAGE_BASE) as usize;
    let site_ptr = site as *mut u8;
    let len = hook.prologue.len();
    // SAFETY: the site is inside the mapped, readable image.
    let current = unsafe { std::slice::from_raw_parts(site_ptr, len) }.to_vec();
    if current != hook.prologue {
        return Outcome::PrologueMismatch(current);
    }

    if let Some(slot) = hook.original {
        // SAFETY: the prologue was just verified; hooks with an `original`
        // list only position-independent instructions there.
        match unsafe { trampoline(site, hook.prologue) } {
            Some(t) => slot.store(t, Ordering::Release),
            None => return Outcome::ProtectFailed,
        }
    }

    let rel = hook.replacement.wrapping_sub(site + 5) as u32;
    let mut jmp = [0xe9, 0, 0, 0, 0];
    jmp[1..].copy_from_slice(&rel.to_le_bytes());

    let mut old = 0;
    // SAFETY: changing protection of five bytes inside our own image.
    if unsafe {
        VirtualProtect(
            site_ptr.cast::<c_void>(),
            5,
            PAGE_EXECUTE_READWRITE,
            &raw mut old,
        )
    } == 0
    {
        return Outcome::ProtectFailed;
    }
    // SAFETY: the range is now writable; the caller guarantees no thread is
    // running it.
    unsafe {
        jmp.as_ptr().copy_to_nonoverlapping(site_ptr, 5);
        VirtualProtect(site_ptr.cast::<c_void>(), 5, old, &raw mut old);
        FlushInstructionCache(GetCurrentProcess(), site_ptr.cast::<c_void>(), 5);
    }
    Outcome::Installed
}
