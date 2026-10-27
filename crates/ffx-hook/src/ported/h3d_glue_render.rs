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
    #[rustfmt::skip]
    type F41 = unsafe extern "C" fn(
        u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32,
        u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32,
        u32, u32, u32, u32, u32, u32, u32, u32, u32,
    );
    // SAFETY: `f` is the game's cdecl `DepthBiasHooks_onSceneLoaded`,
    // which takes exactly these 41 stack words; the caller pops them.
    unsafe {
        let g = std::mem::transmute::<usize, F41>(f as usize);
        g(
            w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7], w[8], w[9], w[10], w[11], w[12], w[13],
            w[14], w[15], w[16], w[17], w[18], w[19], w[20], w[21], w[22], w[23], w[24], w[25],
            w[26], w[27], w[28], w[29], w[30], w[31], w[32], w[33], w[34], w[35], w[36], w[37],
            w[38], w[39], w[40],
        );
    }
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
