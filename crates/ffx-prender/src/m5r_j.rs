//! M5 stage 1, list `render`, batch 10: the render delegations' runs
//! that hand each callback a word (flags 0x1000 and 0x8000), and the
//! scene's after-post-process VFX pass.

/// The delegation helper's mutex handle (`.bss`).
pub const DELEGATION_MUTEX: u32 = 0x00cc_caa0;
pub const WAIT_FOR_SINGLE_OBJECT: u32 = 0x00b0_c0f8;
pub const RELEASE_MUTEX: u32 = 0x00b0_c1c0;
pub const SET_DATA_TO_FREE_LIST: u32 = 0x006a_e1a0;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const COMPACT: u32 = 0x006a_d590;
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
pub const SET_CAMERA: u32 = 0x005b_abd0;
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
pub const END_SCENE: u32 = 0x005b_6950;
/// The scene render pass the VFX are drawn in (`0x00c169d4`).
pub const PASS_DEFAULT: u32 = 0x00c1_69d4;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    fn entry_at(h: &impl Mem, helper: u32, i: u32) -> u32 {
        word(h, word(h, helper + 4).wrapping_add(i.wrapping_mul(4)))
    }

    /// `RenderDelegationHelper_runFlag1000` / `_runFlag8000_006ae0c0(this;
    /// value)` (`ret 4`) -> `al` whether a delegation ran (the rest of
    /// `eax` `ReleaseMutex`'s): as `RenderDelegationHelper_runFlag200`,
    /// each entry with `flag` first gets `value` in its data's `+8`
    /// (`[[entry] + 8]`), then its callback; one-shot entries go back to
    /// the free list (or are deleted with their buffer), their slot
    /// cleared and the array compacted.
    pub fn run_with_value(h: &mut impl Host, helper: u32, flag: u32, value: u32) -> u32 {
        let mutex: u32 = h.read(h.rebase(DELEGATION_MUTEX));
        import(h, WAIT_FOR_SINGLE_OBJECT, &[mutex, u32::MAX]);
        let mut ran = false;
        let mut i = 0_u32;
        while i < word(h, helper) & 0x7fff_ffff {
            let entry = entry_at(h, helper, i);
            if entry != 0 && word(h, entry + 0xc) & flag != 0 {
                let data = word(h, entry);
                h.write(data + 8, value);
                let entry = entry_at(h, helper, i);
                let arg = word(h, entry);
                let callback = word(h, entry + 0x10);
                h.call(callback, Conv::Cdecl, &[arg]);
                ran = true;
                let entry = entry_at(h, helper, i);
                if h.read::<u8>(entry + 0xc) & 1 != 0 {
                    let r = h.call_sym(SET_DATA_TO_FREE_LIST, Conv::Thiscall(helper), &[entry]);
                    if r.cast_signed() < 0 {
                        let dead = entry_at(h, helper, i);
                        if dead != 0 {
                            let buffer = word(h, dead + 4);
                            if buffer != 0 {
                                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[buffer]);
                            }
                            h.write(dead + 8, 0_u32);
                            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[dead]);
                        }
                    }
                    let at = word(h, helper + 4).wrapping_add(i.wrapping_mul(4));
                    h.write(at, 0_u32);
                    i = i.wrapping_sub(1);
                    h.call_sym(COMPACT, Conv::Thiscall(helper), &[]);
                }
            }
            i = i.wrapping_add(1);
        }
        let mutex: u32 = h.read(h.rebase(DELEGATION_MUTEX));
        let r = import(h, RELEASE_MUTEX, &[mutex]);
        (r & !0xff) | u32::from(ran)
    }

    /// `PhyreScene_RenderVFXAfterPostProcess(scene)`: on the renderer
    /// `+0xd0`, `beginScene(0)` (an error is returned at once), the camera
    /// (`[+0xdc]` + 4, or 0), the default pass, every mesh instance of
    /// the list `+0x3888` (count `+0x388c`) rendered with its material
    /// `+0x10` and the renderer's `+0x13c`, `endScene(1)`'s `eax`.
    pub fn render_vfx_after_post(h: &mut impl Host, scene: u32) -> u32 {
        let r = word(h, scene + 0xd0);
        let e = h.call_sym(BEGIN_SCENE, Conv::Thiscall(r), &[0]);
        if e != 0 {
            return e;
        }
        let cam = word(h, scene + 0xdc);
        let cam = if cam == 0 { 0 } else { cam + 4 };
        let r = word(h, scene + 0xd0);
        h.call_sym(SET_CAMERA, Conv::Thiscall(r), &[cam]);
        let r = word(h, scene + 0xd0);
        let pass = h.rebase(PASS_DEFAULT);
        h.write(r + 0x48, pass);
        let mut k = 0_u32;
        while k.cast_signed() < word(h, scene + 0x388c).cast_signed() {
            let inst = word(h, word(h, scene + 0x3888) + 4 * k);
            if inst != 0 {
                let r = word(h, scene + 0xd0);
                let mat = word(h, inst + 0x10);
                let x = word(h, r + 0x13c);
                h.call_sym(RENDER_MESH_INSTANCE, Conv::Thiscall(r), &[inst, mat, 0, x]);
            }
            k += 1;
        }
        let r = word(h, scene + 0xd0);
        h.call_sym(END_SCENE, Conv::Thiscall(r), &[1])
    }
}
