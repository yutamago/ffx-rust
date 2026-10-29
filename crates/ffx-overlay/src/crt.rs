//! The overlays' imports: 25 `MSVCR110`/`KERNEL32` functions every
//! overlay's CRT start-up uses, plus `memset`, `memcpy`, `rand` and seven
//! math functions some use (581 DLLs; docs/formats/magic.md). Each import
//! slot points at a trap address in the service range; a jump there runs
//! the Rust stand-in here, or the embedder's ([`crate::Host::import`]).
//!
//! The `KERNEL32` answers are fixed: they only feed the security cookie
//! and the CRT's start-up, whose results nothing reads back.

use std::sync::Mutex;

use crate::{Config, Error, Host, Regs};

/// Page-aligned, page-rounded bytes Unicorn can map as executable code.
#[derive(Debug)]
pub struct PageBuf {
    ptr: *mut u8,
    len: usize,
}

// SAFETY: written once at creation, read-only afterwards (Unicorn maps it
// read-execute in every engine).
unsafe impl Send for PageBuf {}
// SAFETY: as above.
unsafe impl Sync for PageBuf {}

impl PageBuf {
    fn layout(len: usize) -> std::alloc::Layout {
        std::alloc::Layout::from_size_align(len.max(0x1000), 0x1000).expect("page layout")
    }

    /// `bytes`, zero-padded to `len` (a multiple of 4 KiB).
    #[must_use]
    pub fn from(bytes: &[u8], len: u32) -> PageBuf {
        let len = len as usize;
        // SAFETY: a non-zero, page-aligned layout.
        let ptr = unsafe { std::alloc::alloc_zeroed(Self::layout(len)) };
        assert!(!ptr.is_null(), "out of memory for overlay code");
        let n = bytes.len().min(len);
        // SAFETY: `ptr` holds `len >= n` bytes.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, n) };
        PageBuf { ptr, len }
    }

    #[must_use]
    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }
}

impl Drop for PageBuf {
    fn drop(&mut self) {
        // SAFETY: allocated in `from` with this layout.
        unsafe { std::alloc::dealloc(self.ptr, Self::layout(self.len)) };
    }
}

/// Guest memory through the configured base.
///
/// # Safety
/// `addr .. addr + 4` must be accessible guest memory.
pub unsafe fn read_u32(cfg: &Config, addr: u32) -> u32 {
    // SAFETY: forwarded.
    unsafe { ((cfg.base + addr as usize) as *const u32).read_unaligned() }
}

/// # Safety
/// `addr .. addr + 4` must be writable guest memory.
pub unsafe fn write_u32(cfg: &Config, addr: u32, v: u32) {
    // SAFETY: forwarded.
    unsafe { ((cfg.base + addr as usize) as *mut u32).write_unaligned(v) }
}

/// # Safety
/// `addr .. addr + bytes.len()` must be writable guest memory.
pub unsafe fn write_bytes(cfg: &Config, addr: u32, bytes: &[u8]) {
    // SAFETY: forwarded.
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            (cfg.base + addr as usize) as *mut u8,
            bytes.len(),
        );
    }
}

/// Import names by trap index (`service + 16 * index`).
static TRAPS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The last 16 bytes of the service range: where a stand-in's nested call
/// into overlay code returns.
pub const SENTINEL_OFFSET: u32 = 0xfff0;

/// The trap address for `dll!name` (one per name, shared by every
/// overlay).
pub fn trap_address(cfg: &Config, dll: &str, name: &str) -> u32 {
    let key = format!("{}!{name}", dll.to_ascii_uppercase());
    let mut t = TRAPS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let i = t.iter().position(|n| *n == key).unwrap_or_else(|| {
        t.push(key);
        t.len() - 1
    });
    cfg.service + 16 * i as u32
}

/// The import a trap address stands for: `(dll, name)`.
pub fn trap_name(cfg: &Config, addr: u32) -> Option<String> {
    let off = addr.checked_sub(cfg.service)?;
    if off >= SENTINEL_OFFSET || off % 16 != 0 {
        return None;
    }
    let t = TRAPS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    t.get((off / 16) as usize).cloned()
}

fn f64_of(x: [u8; 16]) -> f64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&x[..8]);
    f64::from_le_bytes(b)
}

/// Runs the stand-in for `key` (`DLL!name`). `regs.esp` is at the return
/// address; on success `regs` is the state after its `ret`.
///
/// # Errors
/// No stand-in, the embedder's failed, or a nested call failed.
#[allow(clippy::too_many_lines)] // one arm per import
pub fn stand_in(
    host: &mut impl Host,
    cfg: &Config,
    key: &str,
    regs: &mut Regs,
) -> Result<(), Error> {
    let name = key.split_once('!').map_or(key, |(_, n)| n);
    if let Some(r) = host.import(name, regs) {
        return r.map_err(Error::Host);
    }
    let esp = regs.esp;
    // SAFETY: the overlay's own stack, mapped (it just pushed onto it).
    let arg = |i: u32| unsafe { read_u32(cfg, esp + 4 + 4 * i) };
    // bytes the callee pops (stdcall); cdecl pops none
    let mut pops = 0;
    match name {
        // cdecl CRT
        "memset" => {
            let (dst, c, n) = (arg(0), arg(1), arg(2));
            // SAFETY: the overlay's own buffer.
            unsafe {
                std::ptr::write_bytes((cfg.base + dst as usize) as *mut u8, c as u8, n as usize);
            }
            regs.eax = dst;
        }
        "memcpy" | "memmove" => {
            let (dst, src, n) = (arg(0), arg(1), arg(2));
            // SAFETY: the overlay's own buffers (MSVCR110's memcpy copies
            // overlapping ranges like memmove).
            unsafe {
                std::ptr::copy(
                    (cfg.base + src as usize) as *const u8,
                    (cfg.base + dst as usize) as *mut u8,
                    n as usize,
                );
            }
            regs.eax = dst;
        }
        "_libm_sse2_sqrt_precise" => {
            // correctly rounded on both sides (sqrtsd)
            let r = f64_of(regs.xmm0).sqrt();
            regs.xmm0[..8].copy_from_slice(&r.to_le_bytes());
        }
        "_lock" | "_unlock" | "__clean_type_info_names_internal" => {}
        // the process-wide list; the DLL's start-up gives it its own list
        // first, so this is only reached without one
        "_onexit" => regs.eax = arg(0),
        "_malloc_crt" => regs.eax = host.alloc(arg(0)).unwrap_or(0),
        "_calloc_crt" => regs.eax = host.alloc(arg(0).saturating_mul(arg(1))).unwrap_or(0),
        "free" => {
            if arg(0) != 0 {
                host.free(arg(0));
            }
        }
        "_initterm" | "_initterm_e" => {
            let (mut p, end) = (arg(0), arg(1));
            regs.eax = 0;
            while p < end {
                // SAFETY: the overlay's initializer table.
                let f = unsafe { read_u32(cfg, p) };
                if f != 0 {
                    let r = crate::engine::call_nested(host, f, regs)?;
                    if name == "_initterm_e" && r.eax != 0 {
                        regs.eax = r.eax;
                        break;
                    }
                }
                p += 4;
            }
        }
        // the DLL's atexit list: `__dllonexit(f, &begin, &end)` grows the
        // block (encoded pointers; EncodePointer is the identity here)
        "__dllonexit" => {
            let (f, pbegin, pend) = (arg(0), arg(1), arg(2));
            // SAFETY: the overlay's own globals and its list block.
            unsafe {
                let (begin, end) = (read_u32(cfg, pbegin), read_u32(cfg, pend));
                let n = end.wrapping_sub(begin) / 4;
                let Some(block) = host.alloc(4 * (n + 1)) else {
                    regs.eax = 0;
                    finish(regs, esp, pops);
                    return Ok(());
                };
                for i in 0..n {
                    write_u32(cfg, block + 4 * i, read_u32(cfg, begin + 4 * i));
                }
                write_u32(cfg, block + 4 * n, f);
                if begin != 0 {
                    host.free(begin);
                }
                write_u32(cfg, pbegin, block);
                write_u32(cfg, pend, block + 4 * (n + 1));
            }
            regs.eax = f;
        }
        // stdcall KERNEL32
        "GetTickCount64" => {
            regs.eax = 0x0010_0000;
            regs.edx = 0;
        }
        "GetSystemTimeAsFileTime" | "QueryPerformanceCounter" => {
            let out = arg(0);
            // SAFETY: the caller's 8-byte out-parameter.
            unsafe {
                write_u32(cfg, out, 0x1234_5678);
                write_u32(cfg, out + 4, 0x01d0_0000);
            }
            regs.eax = 1;
            pops = 4;
        }
        "GetCurrentThreadId" => regs.eax = 1,
        "IsDebuggerPresent" => regs.eax = 0,
        // SSE, SSE2, SSE3 (PF 6, 10, 13); not the fail-fast int 0x29 (23)
        "IsProcessorFeaturePresent" => {
            regs.eax = u32::from(matches!(arg(0), 6 | 10 | 13));
            pops = 4;
        }
        "EncodePointer" | "DecodePointer" => {
            regs.eax = arg(0);
            pops = 4;
        }
        "DisableThreadLibraryCalls" => {
            regs.eax = 1;
            pops = 4;
        }
        _ => return Err(Error::NoStandIn(key.to_owned())),
    }
    finish(regs, esp, pops);
    Ok(())
}

fn finish(regs: &mut Regs, esp: u32, pops: u32) {
    regs.esp = esp + 4 + pops;
}
