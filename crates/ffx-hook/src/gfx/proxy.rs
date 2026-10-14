//! A stand-in for the immediate context. The D3D11 runtime keeps the
//! immediate context's vtable inside the object and swaps its entries as
//! state changes (draws, clears and copies get other implementations every
//! frame), so patching that vtable loses calls. The game gets a proxy
//! instead: an object with a vtable of our own whose entries are either the
//! layer's hooks or small stubs that put the real context in place of
//! `this` and jump to its current method.
//!
//! Layout of a proxy: `[vtable, real context]`.

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_READWRITE, VirtualAlloc, VirtualProtect,
};

/// Entries in the proxy vtable: `ID3D11DeviceContext4` has 151.
pub const SLOTS: usize = 160;

/// The proxy vtable (writable until `seal`), or 0.
static VTABLE: AtomicUsize = AtomicUsize::new(0);
/// Proxies made so far: (real, proxy).
static PROXIES: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());

/// The start of every forwarding stub, then `jmp [ecx + slot*4]`:
///
/// ```text
/// mov eax, [esp+4]      ; the proxy
/// mov eax, [eax+4]      ; the real context
/// mov [esp+4], eax
/// mov ecx, [eax]        ; its vtable, as it is now
/// ```
const HEAD: [u8; 13] = [
    0x8b, 0x44, 0x24, 0x04, 0x8b, 0x40, 0x04, 0x89, 0x44, 0x24, 0x04, 0x8b, 0x08,
];

/// Makes the stubs (19 bytes each, padded to 32 with `int3`) and the vtable pointing
/// at them.
unsafe fn create_vtable() -> usize {
    const SIZE: usize = 32;
    // SAFETY: fresh allocations, written before use.
    unsafe {
        let code = VirtualAlloc(
            std::ptr::null(),
            SLOTS * SIZE,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        )
        .cast::<u8>();
        let table = VirtualAlloc(
            std::ptr::null(),
            SLOTS * 4,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        )
        .cast::<usize>();
        if code.is_null() || table.is_null() {
            return 0;
        }
        for slot in 0..SLOTS {
            let at = code.add(slot * SIZE);
            std::ptr::copy_nonoverlapping(HEAD.as_ptr(), at, HEAD.len());
            // jmp dword ptr [ecx + disp32]
            *at.add(13) = 0xff;
            *at.add(14) = 0xa1;
            std::ptr::copy_nonoverlapping(
                ((slot * 4) as u32).to_le_bytes().as_ptr(),
                at.add(15),
                4,
            );
            for k in 19..SIZE {
                *at.add(k) = 0xcc;
            }
            *table.add(slot) = at as usize;
        }
        let mut old = 0;
        VirtualProtect(
            code.cast::<c_void>(),
            SLOTS * SIZE,
            PAGE_EXECUTE_READ,
            &raw mut old,
        );
        table as usize
    }
}

/// The proxy vtable, made on first use; the layer writes its hooks into it.
pub fn vtable() -> *mut usize {
    let v = VTABLE.load(Ordering::Acquire);
    if v != 0 {
        return v as *mut usize;
    }
    // SAFETY: allocates and fills fresh memory.
    let v = unsafe { create_vtable() };
    match VTABLE.compare_exchange(0, v, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => {
            // SAFETY: slot 0 of our own table.
            unsafe { *(v as *mut usize) = query_interface as *const () as usize };
            v as *mut usize
        }
        Err(existing) => existing as *mut usize,
    }
}

/// The proxy for `real` (made once per real context).
pub fn wrap(real: *mut c_void) -> *mut c_void {
    let Ok(mut list) = PROXIES.lock() else {
        return real;
    };
    if let Some(&(_, p)) = list.iter().find(|(r, _)| *r == real as usize) {
        return p as *mut c_void;
    }
    let object = Box::leak(Box::new([vtable() as usize, real as usize]));
    let p = object.as_mut_ptr() as usize;
    list.push((real as usize, p));
    p as *mut c_void
}

/// The proxy for `real` if there is one.
pub fn proxy_of(real: *mut c_void) -> Option<*mut c_void> {
    PROXIES.lock().ok().and_then(|l| {
        l.iter()
            .find(|(r, _)| *r == real as usize)
            .map(|&(_, p)| p as *mut c_void)
    })
}

/// The real context behind `this`, or `this` when it is not a proxy.
pub fn real(this: *mut c_void) -> *mut c_void {
    // SAFETY: a live COM object or proxy starts with its vtable.
    unsafe {
        if *(this as *const usize) == VTABLE.load(Ordering::Acquire) {
            *(this as *const *mut c_void).add(1)
        } else {
            this
        }
    }
}

/// `QueryInterface` on a proxy: asking for a context interface gives the
/// proxy again (so the game never sees the real context); anything else is
/// the real object's answer.
unsafe extern "system" fn query_interface(
    this: *mut c_void,
    iid: *const [u8; 16],
    out: *mut *mut c_void,
) -> i32 {
    type Qi = unsafe extern "system" fn(*mut c_void, *const [u8; 16], *mut *mut c_void) -> i32;
    let r = real(this);
    // SAFETY: the real object's QueryInterface, then Release of its answer
    // when the proxy stands in for it.
    unsafe {
        let qi: Qi = std::mem::transmute(super::real_method(r, 0));
        let hr = qi(r, iid, out);
        if hr >= 0 && !iid.is_null() && super::CONTEXT_IIDS.contains(&*iid) && !out.is_null() {
            // The real object holds the reference the caller now owns; the
            // proxy forwards AddRef and Release to it.
            *out = this;
        }
        hr
    }
}
