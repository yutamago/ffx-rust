//! Windows: commits Unicorn's translation buffers as they fill, in place of
//! Unicorn's own handler, which fails near a buffer's end (docs/host.md,
//! "Status on Windows").
//!
//! On Windows Unicorn (2.1.5, `qemu/accel/tcg/translate-all.c`) only
//! reserves each instance's translation buffer (`MEM_RESERVE`,
//! `PAGE_EXECUTE_READWRITE`) and commits it lazily: a vectored exception
//! handler commits 4 MB from the faulting address on each write fault in
//! the buffer. Within the buffer's last 4 MB its size is wrong
//! (`size = base_end - base`, still 4 MB, where `right - base` was meant),
//! so the commit reaches past the reservation, `VirtualAlloc` refuses it,
//! and the fault ends the process. Translated code is written in
//! translation-block order, so the faults land at unaligned addresses and
//! the 4 MB steps drift off the buffer's 4 MB grid: every thread whose
//! buffer filled that far crashed there (64-bit `ffx-rust.exe` on PC2, about
//! 200 s into the boot, inside `tcg_gen_code`'s copy of a constant pool).
//! And every commit is a fault that goes through the process's vectored
//! handlers in order: on the main machine an endpoint-security product's
//! handler, ahead of Unicorn's, crashed on the very first one
//! (`tcg_prologue_init`'s).
//!
//! This handler runs first (`AddVectoredExceptionHandler(1, …)`): a write
//! fault in a reserved, uncommitted run of a `PAGE_EXECUTE_READWRITE`
//! reservation (Unicorn's buffers are the only ones; the arena is
//! `PAGE_NOACCESS` or committed `PAGE_READWRITE`) commits at most 4 MB
//! from the faulting page, never past the run, and the write goes on.
//! Anything else goes to the next handler.

use std::sync::Once;

use windows_sys::Win32::Foundation::EXCEPTION_ACCESS_VIOLATION;
use windows_sys::Win32::System::Diagnostics::Debug::{
    AddVectoredExceptionHandler, EXCEPTION_POINTERS,
};
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RESERVE, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READWRITE, VirtualAlloc,
    VirtualQuery,
};

/// Unicorn's step.
const STEP: usize = 4 << 20;
const PAGE: usize = 4096;
/// `EXCEPTION_CONTINUE_EXECUTION`, `EXCEPTION_CONTINUE_SEARCH`.
const CONTINUE_EXECUTION: i32 = -1;
const CONTINUE_SEARCH: i32 = 0;

static INSTALL: Once = Once::new();

/// Installs the handler (once per process), before the first Unicorn
/// instance.
pub fn install() {
    INSTALL.call_once(|| {
        // SAFETY: registers a handler that only touches memory through
        // VirtualQuery and VirtualAlloc; it is never removed.
        unsafe {
            AddVectoredExceptionHandler(1, Some(handler));
        }
    });
}

unsafe extern "system" fn handler(info: *mut EXCEPTION_POINTERS) -> i32 {
    // SAFETY: Windows passes valid exception pointers.
    let Some(record) =
        (unsafe { info.as_ref() }).and_then(|i| unsafe { i.ExceptionRecord.as_ref() })
    else {
        return CONTINUE_SEARCH;
    };
    if record.ExceptionCode != EXCEPTION_ACCESS_VIOLATION || record.NumberParameters < 2 {
        return CONTINUE_SEARCH;
    }
    let at = record.ExceptionInformation[1] & !(PAGE - 1);
    let mut mbi = MEMORY_BASIC_INFORMATION::default();
    // SAFETY: a query of any address is allowed.
    let n = unsafe {
        VirtualQuery(
            at as *const _,
            &raw mut mbi,
            size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    if n == 0 || mbi.State != MEM_RESERVE || mbi.AllocationProtect != PAGE_EXECUTE_READWRITE {
        return CONTINUE_SEARCH;
    }
    // mbi describes the uncommitted run from `at`'s page on
    let size = STEP.min(mbi.RegionSize);
    // SAFETY: commits pages of a reservation, inside it.
    let ok = unsafe { VirtualAlloc(at as *const _, size, MEM_COMMIT, PAGE_EXECUTE_READWRITE) };
    if ok.is_null() {
        CONTINUE_SEARCH
    } else {
        CONTINUE_EXECUTION
    }
}
