//! M5 stage 1, list `phyre-scripting`, round "design-frames"
//! (`ffx_pother_a::m5p_scripting_df`): the two script-name builders with
//! a run-time frame ([`super::with_frame`]). The parity tests are in
//! `crates/ffx-oracle/tests/m5_design_frames.rs`.
//!
//! Threads: the script loader's (main or the async init job); the
//! buffers are per call.

use ffx_pother_a::m5p_scripting_df::bridge as sd;

use super::{mem, ported, with_frame};

ported! {
    /// The script loader's file name `0x005d57b0(name)`: `name.lua` for the file loader.
    PScripting_LoadModuleFile_005d57b0 @ 0x005d_57b0 => extern "C" fn load_module_file(name: u32) -> u32 {
        match sd::load_file_size(&mut mem(), name) {
            None => sd::load_file(&mut mem(), name, None),
            Some((len, bytes)) => {
                with_frame(bytes, |buf| sd::load_file(&mut mem(), name, Some((buf, len, bytes))))
            }
        }
    }
    /// A script path's module name `0x005d6650(string, path)` into a `PString`.
    PScripting_ModuleNameFromPath_005d6650 @ 0x005d_6650 => extern "C" fn module_name_from_path(string: u32, path: u32) -> u32 {
        match sd::module_name_size(&mut mem(), path) {
            None => sd::module_name(&mut mem(), string, path, None),
            Some((start, len, bytes)) => with_frame(bytes, |buf| {
                sd::module_name(&mut mem(), string, path, Some((buf, start, len)))
            }),
        }
    }
}
