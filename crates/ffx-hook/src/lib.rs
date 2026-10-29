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

/// The hook names `FFX_HOOK_ONLY` keeps, `None` when it is unset: a list
/// separated by commas or lines, or `@<file>` holding one (the M4 port
/// lists outgrew the 128 KB a single environment string may hold).
#[must_use]
pub fn hook_only() -> Option<Vec<String>> {
    let value = std::env::var("FFX_HOOK_ONLY").ok()?;
    let list = match value.strip_prefix('@') {
        Some(path) => std::fs::read_to_string(path.trim()).unwrap_or_default(),
        None => value,
    };
    Some(
        list.split([',', '\n', '\r'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect(),
    )
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
    /// Bytes to copy into the trampoline when more than the prologue: whole
    /// instructions whose only image-dependent part is an absolute operand
    /// (copied from the relocated, live code, so still correct). 0 = the
    /// prologue.
    pub steal: usize,
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
pub mod audio_wav;
#[cfg(all(windows, target_arch = "x86"))]
pub mod coverage;
#[cfg(all(windows, target_arch = "x86"))]
pub mod fmod_log;
#[cfg(all(windows, target_arch = "x86"))]
pub mod gfx;
#[cfg(all(windows, target_arch = "x86"))]
pub mod harness;
#[cfg(all(windows, target_arch = "x86"))]
pub mod hook_hits;
#[cfg(all(windows, target_arch = "x86"))]
mod iat;
#[cfg(all(windows, target_arch = "x86"))]
pub mod iggy_log;
#[cfg(all(windows, target_arch = "x86"))]
pub mod loader_turn;
#[cfg(all(windows, target_arch = "x86"))]
pub mod loadsave;
pub mod log;
#[cfg(all(windows, target_arch = "x86"))]
pub mod minimap;
#[cfg(all(windows, target_arch = "x86"))]
pub mod mods;
#[cfg(all(windows, target_arch = "x86"))]
pub mod overlay_emu;
#[cfg(all(windows, target_arch = "x86"))]
mod patch;
#[cfg(all(windows, target_arch = "x86"))]
pub mod ported;
#[cfg(all(windows, target_arch = "x86"))]
pub mod replacements;
#[cfg(all(windows, target_arch = "x86"))]
pub mod rust_audio;
#[cfg(all(windows, target_arch = "x86"))]
pub mod rust_audio_out;
#[cfg(all(windows, target_arch = "x86"))]
pub mod snapshot;
#[cfg(all(windows, target_arch = "x86"))]
pub mod world;

#[cfg(all(windows, target_arch = "x86"))]
pub use patch::{check_build, install};

/// The hook table.
#[cfg(all(windows, target_arch = "x86"))]
#[must_use]
pub fn hooks() -> Vec<Hook> {
    let mut all = vec![
        Hook {
            name: "zlib_uncompress",
            address: 0x0040_5e20,
            prologue: &[0x55, 0x8b, 0xec, 0x83, 0xe4],
            replacement: replacements::zlib_uncompress as *const () as usize,
            original: None,
            steal: 0,
        },
        Hook {
            name: "Path_ToLowerAscii",
            address: 0x0060_7d50,
            prologue: &[0x55, 0x8b, 0xec, 0x8b, 0x45],
            replacement: replacements::path_to_lower_ascii as *const () as usize,
            original: None,
            steal: 0,
        },
        Hook {
            name: "Phyre_PSerialization_PStreamFileWin32_openFile",
            address: 0x0060_7f40,
            // push ebp; mov ebp, esp; sub esp, 8
            prologue: &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08],
            replacement: mods::file_open as *const () as usize,
            original: Some(&mods::FILE_OPEN_ORIGINAL),
            steal: 0,
        },
        Hook {
            name: "Input_KeyHeld",
            address: 0x0062_9d50,
            // push ebp; mov ebp, esp; mov eax, [ecx + 0x2b8]
            prologue: &[0x55, 0x8b, 0xec, 0x8b, 0x81, 0xb8, 0x02, 0x00, 0x00],
            replacement: harness::key_held as *const () as usize,
            original: Some(&harness::KEY_HELD_ORIGINAL),
            steal: 0,
        },
        Hook {
            name: "Phyre_PTimerWin32_GetTime",
            address: 0x0042_fd80,
            // push ebp; mov ebp, esp; then mov eax, [abs32] (relocated: only
            // its opcode is checked; 8 bytes are copied)
            prologue: &[0x55, 0x8b, 0xec, 0xa1],
            replacement: harness::time_now as *const () as usize,
            original: Some(&harness::TIME_NOW_ORIGINAL),
            steal: 8,
        },
        Hook {
            name: "Rng_ClockSeed",
            address: 0x0079_8950,
            // push ebp; mov ebp, esp; sub esp, 8
            prologue: &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08],
            replacement: harness::clock_seed as *const () as usize,
            original: Some(&harness::CLOCK_SEED_ORIGINAL),
            steal: 0,
        },
        Hook {
            name: "MsScreenProcess",
            address: 0x007b_c110,
            // push ebp; mov ebp, esp; sub esp, 0x20
            prologue: &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x20],
            replacement: gfx::live::cam_update as *const () as usize,
            original: None,
            steal: 0,
        },
        Hook {
            name: "rcbgGuideUpdateEx",
            address: 0x0091_da80,
            // push ebx; mov ebx, esp; sub esp, 8
            prologue: &[0x53, 0x8b, 0xdc, 0x83, 0xec, 0x08],
            replacement: minimap::guide_update as *const () as usize,
            original: None,
            steal: 0,
        },
        Hook {
            name: "CDfileSize_CD",
            address: 0x0076_c320,
            // push ebp; mov ebp, esp; push ecx; then mov eax, [abs32]
            // (relocated, so only its opcode is checked; 9 bytes are copied)
            prologue: &[0x55, 0x8b, 0xec, 0x51, 0xa1],
            replacement: mods::cd_file_size as *const () as usize,
            original: Some(&mods::CD_FILE_SIZE_ORIGINAL),
            steal: 9,
        },
        Hook {
            name: "CDfileSize_PC",
            address: 0x00a4_28a0,
            // push ebp; mov ebp, esp; push ebx; push edi
            prologue: &[0x55, 0x8b, 0xec, 0x53, 0x57],
            replacement: mods::cd_file_size_pc as *const () as usize,
            original: Some(&mods::CD_FILE_SIZE_PC_ORIGINAL),
            steal: 0,
        },
        Hook {
            name: "Phyre_PRendering_PRenderer_renderWorldCulledSorted",
            address: 0x009f_3c30,
            // push ebp; mov ebp, esp; push -1
            prologue: &[0x55, 0x8b, 0xec, 0x6a, 0xff],
            replacement: world::render_world as *const () as usize,
            original: Some(&world::RENDER_WORLD_ORIGINAL),
            steal: 0,
        },
        // The loader thread: notes its id for the harness's loader turn
        // (`loader_turn.rs`), then runs the port (`ported/m5_system_b.rs`)
        // or, when that is off, the original through the trampoline.
        Hook {
            name: "AsyncLoadManager_threadMain_00691360",
            address: 0x0069_1360,
            // push ebp; mov ebp, esp; mov eax, [abs32] (relocated: only its
            // opcode is checked; 8 bytes are copied)
            prologue: &[0x55, 0x8b, 0xec, 0xa1],
            replacement: loader_turn::loader_thread as *const () as usize,
            original: Some(&loader_turn::LOADER_ORIGINAL),
            steal: 8,
        },
    ];
    all.extend(ported::hooks());
    all
}
