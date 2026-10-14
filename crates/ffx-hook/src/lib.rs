//! Patches `FFX.exe` functions so they jump into their Rust
//! reimplementations from `ffx-core`.
//!
//! Each [`Hook`] overwrites the first five bytes of an original function
//! with `jmp rel32` to a Rust shim. A hook that still needs the original
//! (for example to fall back to it) gets a trampoline: the stolen prologue
//! instructions followed by a jump back past them. Before patching, the
//! module is checked against the exact build these addresses come from, and
//! every patch site must still hold its expected prologue.

// The patching code only runs on 32-bit x86, where `usize` is `u32`.
#![cfg_attr(target_arch = "x86", allow(clippy::cast_possible_truncation))]

use std::sync::atomic::{AtomicUsize, Ordering};

/// Preferred image base of `FFX.exe`. ASLR is enabled, so addresses are
/// rebased onto the module's real base at install time.
pub const IMAGE_BASE: u32 = 0x40_0000;
/// PE `TimeDateStamp` of the supported build
/// (sha256 `0537b2a1…686da686d`).
pub const TIME_DATE_STAMP: u32 = 0x6aa2_219c;
/// PE `SizeOfImage` of the supported build.
pub const SIZE_OF_IMAGE: u32 = 0x0237_d000;

/// Base address of the patched `FFX.exe`, set by `install`.
pub static GAME_BASE: AtomicUsize = AtomicUsize::new(0);

/// Turns an `FFX.exe` address into its address in the running process.
#[must_use]
pub fn rebase(address: u32) -> usize {
    GAME_BASE.load(Ordering::Acquire) + (address - IMAGE_BASE) as usize
}

/// One function replacement.
#[derive(Debug, Clone, Copy)]
pub struct Hook {
    /// Name in `symbols/ffx.sym`.
    pub name: &'static str,
    /// Address in `FFX.exe`, relative to [`IMAGE_BASE`].
    pub address: u32,
    /// The original first bytes, checked before patching: at least five.
    /// With [`Hook::original`], exactly the whole, position-independent
    /// instructions copied into the trampoline.
    pub prologue: &'static [u8],
    /// Address of the replacement.
    pub replacement: usize,
    /// Receives the trampoline address, for replacements that call the
    /// original.
    pub original: Option<&'static AtomicUsize>,
}

/// The module is not the `FFX.exe` build the hook table was made for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrongBuild {
    pub time_date_stamp: u32,
    pub size_of_image: u32,
}

impl std::fmt::Display for WrongBuild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unsupported FFX.exe build (TimeDateStamp {:#010x}, SizeOfImage {:#x}; expected {TIME_DATE_STAMP:#010x}, {SIZE_OF_IMAGE:#x})",
            self.time_date_stamp, self.size_of_image
        )
    }
}

impl std::error::Error for WrongBuild {}

/// What happened to one hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Installed,
    /// Turned off through `FFX_HOOK_DISABLE`.
    Disabled,
    /// The site did not hold the expected bytes, so it was left alone.
    PrologueMismatch(Vec<u8>),
    /// `VirtualProtect` or `VirtualAlloc` failed.
    ProtectFailed,
}

#[cfg(all(windows, target_arch = "x86"))]
pub mod harness;
#[cfg(all(windows, target_arch = "x86"))]
mod iat;
pub mod log;
#[cfg(all(windows, target_arch = "x86"))]
pub mod mods;
#[cfg(all(windows, target_arch = "x86"))]
mod patch;
#[cfg(all(windows, target_arch = "x86"))]
pub mod replacements;

#[cfg(all(windows, target_arch = "x86"))]
pub use patch::{check_build, install};

/// The hook table.
#[cfg(all(windows, target_arch = "x86"))]
#[must_use]
pub fn hooks() -> [Hook; 4] {
    [
        Hook {
            name: "zlib_uncompress",
            address: 0x0040_5e20,
            prologue: &[0x55, 0x8b, 0xec, 0x83, 0xe4],
            replacement: replacements::zlib_uncompress as *const () as usize,
            original: None,
        },
        Hook {
            name: "Path_ToLowerAscii",
            address: 0x0060_7d50,
            prologue: &[0x55, 0x8b, 0xec, 0x8b, 0x45],
            replacement: replacements::path_to_lower_ascii as *const () as usize,
            original: None,
        },
        Hook {
            name: "File_Open",
            address: 0x0060_7f40,
            // push ebp; mov ebp, esp; sub esp, 8
            prologue: &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08],
            replacement: mods::file_open as *const () as usize,
            original: Some(&mods::FILE_OPEN_ORIGINAL),
        },
        Hook {
            name: "Input_UpdatePad",
            address: 0x0063_2a30,
            // push ebp; mov ebp, esp; push -1 (the next push has a relocated
            // immediate, so it is not copied)
            prologue: &[0x55, 0x8b, 0xec, 0x6a, 0xff],
            replacement: harness::update_pad as *const () as usize,
            original: Some(&harness::UPDATE_PAD_ORIGINAL),
        },
    ]
}
