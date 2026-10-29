//! Runs the player's own spell and summon overlays (`magicFiles/FFX/
//! magic_*.dll`) in Unicorn, inside a 32-bit address space the embedder
//! owns (docs/formats/magic.md, "Running the original overlays").
//!
//! The 581 overlays are not ported on M5's path (user, 2026-10-28): the
//! 64-bit program runs the original x86 code instead, which makes them
//! identical by construction. Unicorn is C; it is the one exception to
//! "pure Rust", and only for the overlays.
//!
//! How it fits together:
//!
//! - **Memory.** Guest addresses are `u32`. Guest address `a` lives at host
//!   address `base + a` ([`configure`]): `base` is 0 in the 32-bit hooked
//!   game (identity: Unicorn sees the game's own memory) and the arena's
//!   start in `ffx-host`. Unicorn maps that memory lazily, in 1 MiB or
//!   64 KiB blocks, the first time overlay code touches it, read-write and
//!   never executable. Only an overlay's `.text` is executable, and Unicorn
//!   runs it from a private relocated copy; the embedder keeps the real
//!   `.text` pages inaccessible, so a native jump into them traps (the
//!   32-bit hook turns that trap into [`call`]).
//! - **Loading.** [`Pe::parse`] reads the file as data; the embedder
//!   reserves the image's range ([`Pe::text_range`] stays inaccessible)
//!   and [`Overlay::load`] writes headers and data sections there,
//!   relocated, with every `MSVCR110`/`KERNEL32` import pointed at a trap
//!   address of the service range. `DllMain` is the embedder's to call
//!   ([`Overlay::entry_point`]), like any other overlay function.
//! - **Leaving overlay code.** A jump or call to any address outside an
//!   overlay's `.text` stops Unicorn there. An import trap runs a Rust
//!   stand-in (or [`Host::import`] for those the embedder answers: `rand`
//!   and the libm functions, whose results must be the game's own); any
//!   other address is [`Host::call_out`], with the `g_MagicImports` slot
//!   when the address is one of the import table's entries. A return to
//!   the address [`call`] was entered with ends the run.
//! - **Threads.** One Unicorn instance per thread that runs overlay code,
//!   made on first use. Calls nest: a call-out may run native code that
//!   calls back into an overlay on the same thread.

#![allow(clippy::cast_possible_truncation)] // guest addresses are u32 by design

mod crt;
mod engine;

use std::sync::{Arc, Mutex, OnceLock};

pub use engine::{call, call_with};
pub use ffx_magic::pe::{IMAGE_ALIGN, Import, Pe, Section};

/// Something went wrong loading or running overlay code.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a usable overlay image: {0}")]
    BadImage(String),
    #[error("Unicorn: {0:?}")]
    Unicorn(unicorn_engine::uc_error),
    #[error("emulation stopped at {pc:#010x} ({err:?}){detail}")]
    Emulation {
        pc: u32,
        err: Option<unicorn_engine::uc_error>,
        detail: String,
    },
    #[error("overlay code called import {0}, which has no stand-in")]
    NoStandIn(String),
    #[error("{0}")]
    Host(String),
    #[error("ffx_overlay::configure was not called")]
    NotConfigured,
}

impl From<ffx_magic::PeError> for Error {
    fn from(e: ffx_magic::PeError) -> Self {
        match e {
            ffx_magic::PeError::BadImage(s) => Error::BadImage(s),
        }
    }
}

impl From<unicorn_engine::uc_error> for Error {
    fn from(e: unicorn_engine::uc_error) -> Self {
        Error::Unicorn(e)
    }
}

/// The registers at a boundary between overlay code and the embedder:
/// at a call-out's entry (`esp` at the return address) and at its return
/// (`esp` past the return address and whatever the callee popped).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Regs {
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
    pub ebx: u32,
    pub esp: u32,
    pub ebp: u32,
    pub esi: u32,
    pub edi: u32,
    pub eflags: u32,
    /// x87 control word (the game runs at 0x027f) and SSE control.
    pub fpu_cw: u16,
    pub mxcsr: u32,
    /// `xmm0`/`xmm1` (the `_libm_sse2_*` functions' argument and result).
    pub xmm0: [u8; 16],
    pub xmm1: [u8; 16],
    /// The x87 stack, `st[0]` = ST0 (80-bit values), `st_len` of them:
    /// a float result on the way out, `_CIatan2`'s two arguments on the
    /// way in. Empty at an ordinary call.
    pub st: [[u8; 10]; 2],
    pub st_len: u8,
}

impl Regs {
    /// Fresh registers for a call made by the embedder: `esp` at the
    /// return address, the game's x87 control word, default MXCSR.
    #[must_use]
    pub fn at(esp: u32) -> Self {
        Regs {
            esp,
            eflags: 0x202,
            fpu_cw: 0x027f,
            mxcsr: 0x1f80,
            ..Regs::default()
        }
    }
}

/// What the embedder provides while overlay code runs.
pub trait Host {
    /// Overlay code jumped to `target`, outside every overlay's code.
    /// `regs.esp` points at the return address (a `call`'s, or for a tail
    /// jump the caller's), the arguments above it. Run the function and
    /// leave in `regs` the state after its `ret`: `esp` past the return
    /// address and the bytes it popped, `eax`/`edx` (and `st[0]` with
    /// `st_len` 1 for a float result). `slot` is the `g_MagicImports`
    /// entry (index) holding `target`, when one does.
    ///
    /// # Errors
    /// The call can't be made.
    fn call_out(&mut self, target: u32, slot: Option<u32>, regs: &mut Regs) -> Result<(), String>;

    /// An import the embedder answers itself, before the built-in
    /// stand-ins: `None` leaves it to them. The same contract as
    /// [`Host::call_out`]. Those without a built-in stand-in (`rand`,
    /// `_CIatan2`, `_libm_sse2_{sin,cos,atan,acos}_precise`) must be
    /// answered here: their results come from the game's `MSVCR110`.
    fn import(&mut self, name: &str, regs: &mut Regs) -> Option<Result<(), String>> {
        let _ = (name, regs);
        None
    }

    /// Zeroed guest memory for the overlays' CRT (`_malloc_crt`, ...) and
    /// each thread's Unicorn bookkeeping (its thread block): `None` when
    /// there is no room.
    fn alloc(&mut self, size: u32) -> Option<u32>;

    /// Gives back what [`Host::alloc`] returned.
    fn free(&mut self, addr: u32);
}

/// Process-wide settings, set once by the embedder.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Host address of guest address 0 (0 in the 32-bit game).
    pub base: usize,
    /// A 64 KiB guest range nothing else uses (reserved, never touched):
    /// the import traps live there. 64 KiB-aligned.
    pub service: u32,
    /// `g_MagicImports` (address, entries): call-outs to one of its
    /// entries carry the slot index. `None` without one.
    pub import_table: Option<(u32, u32)>,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

/// Sets the process-wide [`Config`]; the first call wins.
pub fn configure(config: Config) {
    let _ = CONFIG.set(config);
}

pub(crate) fn config() -> Result<&'static Config, Error> {
    CONFIG.get().ok_or(Error::NotConfigured)
}

/// One loaded overlay, as the engines see it.
#[derive(Debug)]
pub(crate) struct Loaded {
    pub id: u64,
    pub name: String,
    pub base: u32,
    /// `base .. base + span`, 64 KiB-rounded.
    pub span: u32,
    /// The executable range (page-rounded) and its relocated bytes.
    pub text_start: u32,
    pub text_end: u32,
    pub text: crt::PageBuf,
    pub entry: u32,
    pub exports: Vec<(String, u32)>,
}

#[derive(Debug, Default)]
pub(crate) struct Registry {
    pub generation: u64,
    pub next_id: u64,
    pub images: Vec<Arc<Loaded>>,
}

pub(crate) static REGISTRY: Mutex<Registry> = Mutex::new(Registry {
    generation: 0,
    next_id: 1,
    images: Vec::new(),
});

pub(crate) fn registry() -> std::sync::MutexGuard<'static, Registry> {
    REGISTRY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The loaded overlay whose `.text` holds `addr`.
#[must_use]
pub fn text_owner(addr: u32) -> Option<Overlay> {
    registry()
        .images
        .iter()
        .find(|l| addr >= l.text_start && addr < l.text_end)
        .map(|l| Overlay(Arc::clone(l)))
}

/// The loaded overlay whose image starts at `base`.
#[must_use]
pub fn by_base(base: u32) -> Option<Overlay> {
    registry()
        .images
        .iter()
        .find(|l| l.base == base)
        .map(|l| Overlay(Arc::clone(l)))
}

/// A loaded overlay.
#[derive(Debug, Clone)]
pub struct Overlay(Arc<Loaded>);

impl Overlay {
    /// Lays `file` (parsed as `pe`) out at `base` and registers it. The
    /// embedder has made `base .. base + pe.span()` writable through the
    /// configured `base` pointer except [`Pe::text_range`], which is only
    /// reserved. Writes the headers and every non-executable section,
    /// relocated, the import address table pointed at the import traps.
    ///
    /// # Errors
    /// A bad image, `base` not 64 KiB-aligned, or no [`configure`].
    pub fn load(name: &str, file: &[u8], pe: &Pe, base: u32) -> Result<Overlay, Error> {
        let cfg = config()?;
        if !base.is_multiple_of(IMAGE_ALIGN) {
            return Err(Error::BadImage(format!(
                "base {base:#x} not 64 KiB-aligned"
            )));
        }
        let mut img = pe.layout(file, base)?;
        for imp in &pe.imports {
            let trap = crt::trap_address(cfg, &imp.dll, &imp.name);
            let at = imp.iat as usize;
            img.get_mut(at..at + 4)
                .ok_or_else(|| Error::BadImage(format!("IAT slot {:#x} outside", imp.iat)))?
                .copy_from_slice(&trap.to_le_bytes());
        }
        let (t0, t1) = pe.text_range()?;
        let text = crt::PageBuf::from(&img[t0 as usize..(t1 as usize).min(img.len())], t1 - t0);
        // everything but the code, through the embedder's memory
        // SAFETY: the embedder made these ranges writable (the contract).
        unsafe {
            crt::write_bytes(cfg, base, &img[..t0 as usize]);
            if (t1 as usize) < img.len() {
                crt::write_bytes(cfg, base + t1, &img[t1 as usize..]);
            }
        }
        let mut reg = registry();
        let id = reg.next_id;
        reg.next_id += 1;
        let loaded = Arc::new(Loaded {
            id,
            name: name.to_owned(),
            base,
            span: pe.span(),
            text_start: base + t0,
            text_end: base + t1,
            text,
            entry: base + pe.entry,
            exports: pe
                .exports
                .iter()
                .map(|(n, rva)| (n.clone(), base + rva))
                .collect(),
        });
        reg.images.push(Arc::clone(&loaded));
        reg.generation += 1;
        Ok(Overlay(loaded))
    }

    /// Forgets the overlay: every engine drops its mappings and
    /// translated code the next time it runs. The embedder frees the
    /// memory after `DllMain(DLL_PROCESS_DETACH)`.
    pub fn unload(&self) {
        let mut reg = registry();
        reg.images.retain(|l| l.id != self.0.id);
        reg.generation += 1;
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.0.name
    }

    #[must_use]
    pub fn base(&self) -> u32 {
        self.0.base
    }

    #[must_use]
    pub fn span(&self) -> u32 {
        self.0.span
    }

    /// `DllMain`'s address (`_DllMainCRTStartup`).
    #[must_use]
    pub fn entry_point(&self) -> u32 {
        self.0.entry
    }

    /// An export's address (`InitMagicPRX`, `GetEffectOverlayTable`).
    #[must_use]
    pub fn export(&self, name: &str) -> Option<u32> {
        self.0.exports.iter().find(|(n, _)| n == name).map(|e| e.1)
    }

    /// The executable range.
    #[must_use]
    pub fn text(&self) -> (u32, u32) {
        (self.0.text_start, self.0.text_end)
    }

    /// The function pointers of the table `GetEffectOverlayTable` returned
    /// (`table`): the consecutive words from its start that point into
    /// this overlay's code, as `(offset, address)`. The battle code calls
    /// them by offset (`+0x4`, `+0xc`, `+0x10`, ...).
    ///
    /// # Errors
    /// No [`configure`].
    pub fn table_entries(&self, table: u32) -> Result<Vec<(u32, OverlayFn)>, Error> {
        let cfg = config()?;
        let mut out = Vec::new();
        for i in 0..64u32 {
            // SAFETY: the table is in this overlay's data, which the
            // embedder keeps mapped while it is loaded.
            let f = unsafe { crt::read_u32(cfg, table + 4 * i) };
            if f < self.0.text_start || f >= self.0.text_end {
                break;
            }
            out.push((4 * i, OverlayFn(f)));
        }
        Ok(out)
    }
}

/// A function of overlay code, callable from Rust: the arguments are
/// pushed on a stack the caller provides and the code runs in Unicorn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayFn(pub u32);

impl OverlayFn {
    /// Calls it as cdecl or stdcall (`ecx` for thiscall) with `args`, on
    /// the stack ending at `stack_top` (16-byte aligned room the caller
    /// owns). Returns the registers after its `ret`.
    ///
    /// # Errors
    /// The emulation failed.
    pub fn call(
        self,
        host: &mut impl Host,
        stack_top: u32,
        ecx: u32,
        args: &[u32],
    ) -> Result<Regs, Error> {
        engine::call_rust(host, self.0, stack_top, ecx, args)
    }
}
