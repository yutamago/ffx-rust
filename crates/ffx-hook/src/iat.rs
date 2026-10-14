//! Import address table patching: replaces the address the main module
//! calls for one imported function.

use std::ffi::{CStr, c_void};

use windows_sys::Win32::System::Memory::{PAGE_READWRITE, VirtualProtect};

/// Points `module`'s import of `dll!function` at `replacement`. Returns the
/// original address, or `None` when the module does not import it.
///
/// # Safety
/// `module` must be a mapped PE32 image; no thread may be calling the
/// import while it is swapped.
pub unsafe fn patch(module: usize, dll: &str, function: &str, replacement: usize) -> Option<usize> {
    // SAFETY: the caller guarantees a mapped PE32 image: e_lfanew at 0x3c,
    // the import directory RVA at NT headers + 0x80.
    unsafe {
        let read = |at: usize| (at as *const u32).read_unaligned() as usize;
        let nt = module + read(module + 0x3c);
        let imports = read(nt + 0x80);
        if imports == 0 {
            return None;
        }
        let mut desc = module + imports;
        loop {
            let (names, name, thunks) = (read(desc), read(desc + 12), read(desc + 16));
            if name == 0 {
                return None;
            }
            let dll_name = CStr::from_ptr((module + name) as *const _).to_string_lossy();
            if dll_name.eq_ignore_ascii_case(dll) {
                let names = if names == 0 { thunks } else { names };
                let mut i = 0;
                loop {
                    let entry = read(module + names + 4 * i);
                    if entry == 0 {
                        break;
                    }
                    // Ordinal imports have the top bit set.
                    if entry & 0x8000_0000 == 0 {
                        let f = CStr::from_ptr((module + entry + 2) as *const _);
                        if f.to_bytes() == function.as_bytes() {
                            let slot = (module + thunks + 4 * i) as *mut usize;
                            let mut old = 0;
                            if VirtualProtect(
                                slot.cast::<c_void>(),
                                4,
                                PAGE_READWRITE,
                                &raw mut old,
                            ) == 0
                            {
                                return None;
                            }
                            let original = slot.read();
                            slot.write(replacement);
                            VirtualProtect(slot.cast::<c_void>(), 4, old, &raw mut old);
                            return Some(original);
                        }
                    }
                    i += 1;
                }
            }
            desc += 20;
        }
    }
}

/// Replaces entry `index` of a COM vtable; returns the original.
///
/// # Safety
/// `vtable` must point to a vtable with at least `index + 1` entries.
pub unsafe fn patch_vtable(vtable: *mut usize, index: usize, replacement: usize) -> Option<usize> {
    // SAFETY: forwarded from the caller.
    unsafe {
        let slot = vtable.add(index);
        let mut old = 0;
        if VirtualProtect(slot.cast::<c_void>(), 4, PAGE_READWRITE, &raw mut old) == 0 {
            return None;
        }
        let original = slot.read();
        slot.write(replacement);
        VirtualProtect(slot.cast::<c_void>(), 4, old, &raw mut old);
        Some(original)
    }
}
