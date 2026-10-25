//! Per mesh-instance segment, on the main thread (docs/scene-builder.md,
//! "Per instance"): the technique and context variant
//! (`PRendererBase::selectTechniquePass` `0x005ba690`, the technique
//! lookup `0x004e71d0`) and the parameter buffers
//! (`PRendererBase::allocateAndFillParameterBuffers` `0x005b5a50`).
//!
//! `selectTechniquePass(renderer, out, a, b, effect, material, pass,
//! node, segment)` (thiscall, `ret 0x20`): the material's remap
//! (`material +8 == pass`: `material +0xc`), the effect's technique for the
//! pass by its id (`pass +0xc` into `effect +0x18`, `effect +0x14` long)
//! or by pointer ([`bridge::find_technique`]); then the context variant:
//! the segment's cached one (`segment +0x18 == effect`: `segment +0x14`)
//! or `PEffect_findContextVariant` `0x004ef440`; the shader is the
//! technique's `+8` plus 32 bytes per variant. `out = (effect, shader)`;
//! 7 without a shader, else 0,
//! after counting the use when the effect statistics are on
//! (`0x00cbbf58`, `0x004e0700`, `InterlockedExchangeAdd`).
//!
//! `allocateAndFillParameterBuffers(renderer, out, flags, material, node,
//! a, b, c, dynamic)` (thiscall, `ret 0x20`): by `flags`, the scene-wide
//! buffer (1: cached at `renderer +0x58`, made by
//! `PSceneWideParameters_getBuffer` `0x004fdec0`), the material's (2,
//! `0x005b7f30`) and the node's (4, `0x005b7fe0`) capture buffers, and (8)
//! the context buffer (`0x005b6cf0`) filled with the dynamic streams
//! (`0x005b9290`, the dynamic object's virtual size and fill into a
//! preprocess-ring block) and the streams (`0x005b94b0`). Returns
//! `0x005b94b0`'s error (freeing the block), else 0.

use ffx_graphic::host::{Conv, Host};

/// `PEffect_findContextVariant` (thiscall `(technique0, a, b, c, d,
/// pass)`, `ret 0x14`).
pub const FIND_CONTEXT_VARIANT: u32 = 0x004e_f440;
/// The technique lookup by pointer (thiscall `(effect, pass)`, `ret 4`).
pub const FIND_TECHNIQUE: u32 = 0x004e_71d0;
/// The effect statistics' object (`0` when off).
pub const EFFECT_STATS: u32 = 0x00cb_bf58;
/// Its counter lookup (thiscall `(stats, technique, shader)`, `ret 8`).
pub const STATS_COUNTER: u32 = 0x004e_0700;
/// `KERNEL32!InterlockedExchangeAdd`'s import slot.
pub const INTERLOCKED_ADD: u32 = 0x00b0_c128;

/// `PSceneWideParameters_getBuffer` (thiscall `(params, a, b, renderer)`).
pub const SCENE_WIDE_BUFFER: u32 = 0x004f_dec0;
/// `generateMaterialParameterCaptureBuffer` (thiscall `(renderer, material)`).
pub const MATERIAL_BUFFER: u32 = 0x005b_7f30;
/// `generateNodeParameterCaptureBuffer` (thiscall `(renderer, node)`).
pub const NODE_BUFFER: u32 = 0x005b_7fe0;
/// `generateContextParameterCaptureBuffer` (thiscall `(renderer, node, b, c)`).
pub const CONTEXT_BUFFER: u32 = 0x005b_6cf0;
/// `populateContextParameterCaptureBufferWithDynamicStreams` (thiscall
/// `(renderer, buffer, node, a, dynamic, out5, out6)`).
pub const DYNAMIC_STREAMS: u32 = 0x005b_9290;
/// `populateContextParameterCaptureBufferWithStreams` (thiscall
/// `(renderer, buffer, node, a, streams)` -> error).
pub const STREAMS: u32 = 0x005b_94b0;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        CONTEXT_BUFFER, Conv, DYNAMIC_STREAMS, EFFECT_STATS, FIND_CONTEXT_VARIANT, Host,
        INTERLOCKED_ADD, MATERIAL_BUFFER, NODE_BUFFER, SCENE_WIDE_BUFFER, STATS_COUNTER, STREAMS,
    };
    use crate::world::{RING_ALLOCATE, RING_SINGLETON};

    /// `0x004e71d0(effect, pass)`: the effect's technique entry (40
    /// bytes, `effect +0x10`, `effect +0xc` of them) whose first word is
    /// `pass`, or 0.
    pub fn find_technique(h: &mut impl Host, effect: u32, pass: u32) -> u32 {
        let n = h.read::<u32>(effect + 0xc) & 0x7fff_ffff;
        let mut e: u32 = h.read(effect + 0x10);
        for _ in 0..n {
            if h.read::<u32>(e) == pass {
                return e;
            }
            e = e.wrapping_add(40);
        }
        0
    }

    /// `selectTechniquePass(renderer, out, a, b, effect, material, pass,
    /// node, segment)`: 0, or 7 without a shader.
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn select_technique_pass(
        h: &mut impl Host,
        renderer: u32,
        out: u32,
        a: u32,
        b: u32,
        effect: u32,
        material: u32,
        pass: u32,
        node: u32,
        segment: u32,
    ) -> u32 {
        let first: u32 = h.read(effect);
        let pass = if h.read::<u32>(material + 8) == pass {
            h.read(material + 0xc)
        } else {
            pass
        };
        let id = if pass == 0 {
            u32::MAX
        } else {
            h.read(pass + 0xc)
        };
        let mut tech = 0;
        if id != u32::MAX && id < h.read::<u32>(effect + 0x14) & 0x7fff_ffff {
            let table: u32 = h.read(effect + 0x18);
            tech = h.read(table.wrapping_add(id.wrapping_mul(4)));
        }
        if tech == 0 {
            tech = find_technique(h, effect, pass);
        }
        let mut shader = 0u32;
        if tech != 0 {
            let variant = if h.read::<u32>(segment + 0x18) == effect {
                h.read::<u32>(segment + 0x14)
            } else {
                let f = h.rebase(FIND_CONTEXT_VARIANT);
                let ctx: u32 = h.read(renderer + 0x18);
                h.call(f, Conv::Thiscall(first), &[b, ctx, a, node, pass])
            } as i32;
            if variant >= 0 {
                let base: u32 = h.read(tech + 8);
                shader = (variant as u32).wrapping_shl(5).wrapping_add(base);
            }
        }
        h.write(out, effect);
        h.write(out + 4, shader);
        if shader == 0 {
            return 7;
        }
        let stats: u32 = h.read(h.rebase(EFFECT_STATS));
        if stats != 0 {
            let f = h.rebase(STATS_COUNTER);
            let counter = h.call(f, Conv::Thiscall(stats), &[tech, shader]);
            if counter != 0 {
                let add: u32 = h.read(h.rebase(INTERLOCKED_ADD));
                h.call(add, Conv::Stdcall, &[counter, 1]);
            }
        }
        0
    }

    /// `allocateAndFillParameterBuffers(renderer, out, flags, material,
    /// node, a, b, c, dynamic)`: 0, or the stream fill's error.
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn allocate_and_fill(
        h: &mut impl Host,
        renderer: u32,
        out: u32,
        flags: u32,
        material: u32,
        node: u32,
        a: u32,
        b: u32,
        c: u32,
        dynamic: u32,
    ) -> u32 {
        if flags & 1 != 0 {
            let mut buffer: u32 = h.read(renderer + 0x58);
            if buffer == 0 {
                let params: u32 = h.read(renderer + 0x54);
                let source: u32 = h.read(renderer + 0x5c);
                if params != 0 && source != 0 {
                    let f = h.rebase(SCENE_WIDE_BUFFER);
                    let ctx: u32 = h.read(renderer + 0x18);
                    let made = h.call(f, Conv::Thiscall(params), &[source, ctx, renderer]);
                    h.write(renderer + 0x58, made);
                    buffer = h.read(renderer + 0x58);
                }
            }
            h.write(out, buffer);
        }
        if flags & 2 != 0 {
            let f = h.rebase(MATERIAL_BUFFER);
            let v = h.call(f, Conv::Thiscall(renderer), &[material]);
            h.write(out + 4, v);
        }
        if flags & 4 != 0 {
            let f = h.rebase(NODE_BUFFER);
            let v = h.call(f, Conv::Thiscall(renderer), &[node]);
            h.write(out + 8, v);
        }
        if flags & 8 == 0 {
            return 0;
        }
        let f = h.rebase(CONTEXT_BUFFER);
        let buffer = h.call(f, Conv::Thiscall(renderer), &[node, b, c]);
        if buffer == 0 {
            h.write(out + 0xc, 0u32);
            return 0;
        }
        let mut block = 0u32;
        let mut streams = 0u32;
        if dynamic != 0 {
            let f = h.rebase(DYNAMIC_STREAMS);
            streams = h.call(
                f,
                Conv::Thiscall(renderer),
                &[buffer, node, a, dynamic, out + 0x14, out + 0x18],
            );
            let size = h.call_virtual(dynamic, 1, &[node]);
            if size != 0 {
                let f = h.rebase(RING_SINGLETON);
                let ring = h.call(f, Conv::Cdecl, &[]);
                let f = h.rebase(RING_ALLOCATE);
                block = h.call(f, Conv::Thiscall(ring), &[size]);
                h.call_virtual(dynamic, 2, &[block, node]);
            }
        }
        let f = h.rebase(STREAMS);
        let error = h.call(f, Conv::Thiscall(renderer), &[buffer, node, a, streams]);
        if error != 0 {
            if block != 0 {
                h.write(block + 4, 0u32);
            }
            return error;
        }
        h.write(out + 0x10, block);
        h.write(out + 0xc, buffer);
        0
    }
}
