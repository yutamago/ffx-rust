//! Patches `FFX.exe` functions so they jump into their Rust
//! reimplementations from `ffx-core`.
//!
//! Each [`Hook`] overwrites the first five bytes of an original function
//! with `jmp rel32` to an `extern "C"` shim. The original is never called
//! again, so no trampoline is needed. Before patching, the module is checked
//! against the exact build these addresses come from, and every patch site
//! must still hold its expected prologue.

// The patching code only runs on 32-bit x86, where `usize` is `u32`.
#![cfg_attr(target_arch = "x86", allow(clippy::cast_possible_truncation))]

/// Preferred image base of `FFX.exe`. ASLR is enabled, so addresses are
/// rebased onto the module's real base at install time.
pub const IMAGE_BASE: u32 = 0x40_0000;
/// PE `TimeDateStamp` of the supported build
/// (sha256 `0537b2a1…686da686d`).
pub const TIME_DATE_STAMP: u32 = 0x6aa2_219c;
/// PE `SizeOfImage` of the supported build.
pub const SIZE_OF_IMAGE: u32 = 0x0237_d000;

/// One function replacement.
#[derive(Debug, Clone, Copy)]
pub struct Hook {
    /// Name in `symbols/ffx.sym`.
    pub name: &'static str,
    /// Address in `FFX.exe`, relative to [`IMAGE_BASE`].
    pub address: u32,
    /// The original first five bytes, checked before patching.
    pub prologue: [u8; 5],
    /// Address of the replacement.
    pub replacement: usize,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Installed,
    /// Turned off through `FFX_HOOK_DISABLE`.
    Disabled,
    /// The site did not hold the expected bytes, so it was left alone.
    PrologueMismatch([u8; 5]),
    /// `VirtualProtect` refused to make the site writable.
    ProtectFailed,
}

#[cfg(all(windows, target_arch = "x86"))]
mod patch;
#[cfg(all(windows, target_arch = "x86"))]
pub mod replacements;

#[cfg(all(windows, target_arch = "x86"))]
pub use patch::{check_build, install};

/// The hook table.
#[cfg(all(windows, target_arch = "x86"))]
#[must_use]
pub fn hooks() -> [Hook; 2] {
    [
        Hook {
            name: "zlib_uncompress",
            address: 0x0040_5e20,
            prologue: [0x55, 0x8b, 0xec, 0x83, 0xe4],
            replacement: replacements::zlib_uncompress as *const () as usize,
        },
        Hook {
            name: "Path_ToLowerAscii",
            address: 0x0060_7d50,
            prologue: [0x55, 0x8b, 0xec, 0x8b, 0x45],
            replacement: replacements::path_to_lower_ascii as *const () as usize,
        },
    ]
}
