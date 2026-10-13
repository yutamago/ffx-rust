//! Reimplementations of `FFX.exe` functions, written so they can be tested on
//! any platform. Each function documents the original it replaces; the FFI
//! shims that patch them into the game live in `ffx-hook`, and `ffx-oracle`
//! checks them against the original machine code.

pub mod path;
pub mod zlib;
