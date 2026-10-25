//! The audio platform census (M4, `docs/formats/audio-platform.md`): the
//! FMOD boundary of `FFX.exe`, read from the image (`boundary`) and from
//! call logs of the running game (`fmodlog`, written by `ffx-hook` with
//! `FFX_FMOD_LOG`), and summed up per function (`summary`). It is the spec
//! input for the Rust audio platform that replaces FMOD.

pub mod boundary;
pub mod demangle;
pub mod fmodlog;
pub mod summary;
