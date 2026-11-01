//! M5 stage 1, list `framework-input`, round big-phyre2
//! (`ffx_pother_b::m5p_framework_input_bp2`): the motion blur's render
//! pass (`0x00a1eb10`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_framework_input.rs` (`bp2_*`).
//!
//! Functions that hand callees the addresses of their locals take `ebp`,
//! where the original's frame pointer is, and keep those locals at the
//! original's offsets below it; the shim passes the top of a
//! `StackFrame`, the tests the original's frame address.

/// `0x00a25410` (thiscall `(effect; a)`): the pass's setup.
pub const PASS_SETUP: u32 = 0x00a2_5410;
/// `TrophyManager_Install` (thiscall `(this; arg)`, a folded body the
/// texture parameter setters call first).
pub const TROPHY_INSTALL: u32 = 0x0059_e520;
/// `memcpy` (CRT thunk, cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// `PRenderSurface::PRenderSurface(texture, 0)` (thiscall, two words).
pub const SURFACE_CTOR: u32 = 0x005b_35c0;
/// `PRenderSurface::~PRenderSurface` (`ecx` only).
pub const SURFACE_DTOR: u32 = 0x005b_3640;
/// `PRenderer::setColorTarget(surface, slot)` (thiscall, two words).
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
/// `PRenderer::setDepthStencilTarget(target)` (thiscall, one word).
pub const SET_DEPTH_TARGET: u32 = 0x005b_b5b0;
/// `PRenderer::beginScene(word)` (thiscall, one word).
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
/// `PRenderer::endScene(word)` (thiscall, one word).
pub const END_SCENE: u32 = 0x005b_6950;
/// `PRendererBase::renderMeshInstance(instance, contexts, x, y)`
/// (thiscall, four words).
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
/// The motion blur pass's render state object (`renderer + 0x48`).
pub const BLUR_STATE: u32 = 0x0194_4d84;
/// `0x00a1eb10`'s frame below `ebp` (the surface at `ebp - 0x3c`, the two
/// texture bindings at `- 0x2c` and `- 0x1c`), in words.
pub const BLUR_PASS_FRAME_WORDS: u32 = 15;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// Where a parameter's bytes go in `buffer` (`+8` offset, `+0xa` size
    /// & 0x1fff): `None` (3) when the offset is below 4 or they don't fit
    /// the buffer's size word.
    fn place(h: &impl Host, param: u32, buffer: u32) -> Option<(u32, u32)> {
        let offset = u32::from(h.read::<u16>(param + 8));
        let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
        if offset.wrapping_add(size) > h.read::<u32>(buffer) || offset < 4 {
            None
        } else {
            Some((offset, size))
        }
    }

    /// A texture parameter (`fx + slot`) set as
    /// `Phyre_PParameterBuffer_setTextureChecked_0056e670` does: the
    /// binding `{0x42, 0, fx + owner, texture}` at `binding` (a local),
    /// 5 for a type other than 0x42, 0xd9..0xdc or 0x12,
    /// `TrophyManager_Install`'s error, 3 when it doesn't fit; 0.
    fn set_texture(
        h: &mut impl Host,
        fx: u32,
        slot: u32,
        owner: u32,
        texture: u32,
        binding: u32,
    ) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let params: u32 = h.read(fx + 0xd28);
        let buffer: u32 = h.read(params + 4);
        let kind: u8 = h.read(param + 2);
        let owner = fx.wrapping_add(owner);
        h.write(binding, [0x42u32, 0, owner, texture]);
        if !matches!(kind, 0x42 | 0xd9..=0xdc | 0x12) {
            return 5;
        }
        if owner != 0 && texture != 0 {
            let err = this_call(h, TROPHY_INSTALL, owner, &[texture]);
            if err != 0 {
                return err;
            }
        }
        match place(h, param, buffer) {
            None => 3,
            Some((offset, size)) => {
                h.call_sym(
                    MEMCPY,
                    Conv::Cdecl,
                    &[offset.wrapping_add(buffer), binding, size],
                );
                0
            }
        }
    }

    /// A parameter (`fx + slot`) taking the effect's own bytes at `fx +
    /// src`: 1 when that address is 0, 3 when they don't fit; 0.
    fn set_bytes(h: &mut impl Host, fx: u32, slot: u32, src: u32) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let params: u32 = h.read(fx + 0xd28);
        let buffer: u32 = h.read(params + 4);
        let src = fx.wrapping_add(src);
        if src == 0 {
            return 1;
        }
        match place(h, param, buffer) {
            None => 3,
            Some((offset, size)) => {
                h.call_sym(
                    MEMCPY,
                    Conv::Cdecl,
                    &[offset.wrapping_add(buffer), src, size],
                );
                0
            }
        }
    }

    /// `Phyre_PPostProcessing_PMotionBlur_renderPass_00a1eb10(effect;
    /// renderer, target, a, velocity, colour)` (thiscall, ret 0x14): 0x15
    /// without the mesh instance (`+0xcdc`); the setup `0x00a25410(effect;
    /// a)`; the two texture parameters (`+0xd2c` with `velocity`, owner
    /// `+0xd04`, binding at `ebp - 0x1c`; `+0xd30` with `colour`, owner
    /// `+0xce0`, at `ebp - 0x2c`) and two byte parameters (`+0xd38` from
    /// `+0xcd4`, `+0xd44` from `+0x94`) into the parameter buffer
    /// (`[[+0xd28] + 4]`), the first error returned; the blur state at
    /// `renderer + 0x48`; the target as colour target (a surface at
    /// `ebp - 0x3c`, destroyed after the call) or none, no depth target, a
    /// scene with the mesh instance, and the colour target cleared again
    /// (its `eax` returned); the first error.
    #[allow(clippy::too_many_arguments)] // the original's five words, the host and the frame
    pub fn blur_render_pass(
        h: &mut impl Host,
        ebp: u32,
        fx: u32,
        renderer: u32,
        target: u32,
        a: u32,
        velocity: u32,
        colour: u32,
    ) -> u32 {
        if h.read::<u32>(fx + 0xcdc) == 0 {
            return 0x15;
        }
        this_call(h, PASS_SETUP, fx, &[a]);
        let err = set_texture(h, fx, 0xd2c, 0xd04, velocity, ebp - 0x1c);
        if err != 0 {
            return err;
        }
        let err = set_texture(h, fx, 0xd30, 0xce0, colour, ebp - 0x2c);
        if err != 0 {
            return err;
        }
        let err = set_bytes(h, fx, 0xd38, 0xcd4);
        if err != 0 {
            return err;
        }
        let err = set_bytes(h, fx, 0xd44, 0x94);
        if err != 0 {
            return err;
        }
        let state = h.rebase(BLUR_STATE);
        h.write(renderer + 0x48, state);
        let err = if target == 0 {
            this_call(h, SET_COLOR_TARGET, renderer, &[0, 0])
        } else {
            let surface = ebp - 0x3c;
            this_call(h, SURFACE_CTOR, surface, &[target, 0]);
            let err = this_call(h, SET_COLOR_TARGET, renderer, &[surface, 0]);
            this_call(h, SURFACE_DTOR, surface, &[]);
            err
        };
        if err != 0 {
            return err;
        }
        let err = this_call(h, SET_DEPTH_TARGET, renderer, &[0]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, BEGIN_SCENE, renderer, &[0]);
        if err != 0 {
            return err;
        }
        let contexts: u32 = h.read(renderer + 0x13c);
        let instance: u32 = h.read(fx + 0xcdc);
        let list: u32 = h.read(instance + 0x10);
        let err = this_call(
            h,
            RENDER_MESH_INSTANCE,
            renderer,
            &[instance, list, 0, contexts],
        );
        if err != 0 {
            return err;
        }
        let err = this_call(h, END_SCENE, renderer, &[1]);
        if err != 0 {
            return err;
        }
        this_call(h, SET_COLOR_TARGET, renderer, &[0, 0])
    }
}
