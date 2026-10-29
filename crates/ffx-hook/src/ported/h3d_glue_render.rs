//! `ffx_prender::h3d_*`: `PhyreScene::fieldMapInitialize` (M4, list
//! `h3-glue-render`, agent h3d).
//!
//! Threads: the main thread (the field map's load and rebuild); the shim
//! keeps no state.

use super::{StackFrame, mem, ported};
use ffx_bridge::Mem;
use ffx_prender::h3d_fmi::{self as fmi, bridge};

/// Calls `DepthBiasHooks_onSceneLoaded` (cdecl) at `f` with its
/// [`fmi::ON_SCENE_LOADED_WORDS`] words: the name and the four copies by
/// value, which it destroys (an MSVC array, `std::string` and vectors:
/// no pointers into themselves, so the copies may move).
fn on_scene_loaded(f: u32, w: &[u32; fmi::ON_SCENE_LOADED_WORDS]) {
    // `f` is the game's cdecl `DepthBiasHooks_onSceneLoaded`, which takes
    // exactly these 41 stack words; the caller pops them
    ffx_bridge::route::cdecl::<()>(f, w);
}

/// The frame: [`fmi::FRAME_LOW`] bytes below `ebp`, then `ebp` up to the
/// argument word (`ebp + 8`).
const FRAME_WORDS: usize = (fmi::FRAME_LOW as usize + 0xc) / 4;

ported! {
    // ---- batch 2 (ffx_prender::h3d_fmi) ----
    /// `PhyreScene::fieldMapInitialize(scene; rebuild)` -> 0, or 0x15 when
    /// the clusters it needs are missing (or a rebuild is refused).
    PhyreScene_fieldMapInitialize @ 0x0065_b8a0 => extern "thiscall" fn field_map_initialize(this: u32, rebuild: u32) -> u32 {
        let mut f = StackFrame::<FRAME_WORDS>::new();
        let ebp = f.at() + fmi::FRAME_LOW;
        let mut m = mem();
        let addr = m.rebase(fmi::ON_SCENE_LOADED);
        bridge::field_map_initialize(&mut m, ebp, this, rebuild, |_, w| on_scene_loaded(addr, w))
    }
}
