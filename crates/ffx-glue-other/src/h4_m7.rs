//! Continuation round 11 of list `h4-glue-other`, batch 2: the dynamic
//! geometry's render data update.

use ffx_bridge::x87::{load, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4_m1::{
    CLUSTER_BY_NAME, CLUSTER_MANAGER, FIND_PARAM, LINEAR_SAMPLER, PHYRE_SCENE, SAMPLER_PARAM,
    SET_VALUE_DESC, SURFACE_FORMAT, TEXTURE_ITERATOR, TEXTURE_PARAM,
};
use crate::h4_m6::{FADE_FACTOR_PARAM, FADE_PARAM, SET_VALUE2, SOFT_A, SOFT_B, SOFT_PARTICLE};

/// `ClassDynamicMesh_fillDrawTable_006732a0(mesh; geometry)` (thiscall,
/// ret 4).
pub const FILL_DRAW_TABLE: u32 = 0x0067_32a0;
/// `PhyreScene_setMeshInstanceWorldMatrix(instance, geometry)` (stdcall).
pub const SET_WORLD_MATRIX: u32 = 0x0066_d810;
/// `Phyre_PRendering_PMeshInstanceBounds_setMinAndMax(bounds; &min,
/// &max)` (thiscall, ret 8).
pub const SET_MIN_MAX: u32 = 0x0050_0bd0;
/// `memcpy` through its thunk (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// Parameter names: the depth range (`0x00b47f5c`), the z matrix
/// (`0x00b47f64`), the screen parameters (`0x00b47f6c`).
pub const Z_RANGE_PARAM: u32 = 0x00b4_7f5c;
/// See [`Z_RANGE_PARAM`].
pub const Z_MATRIX_PARAM: u32 = 0x00b4_7f64;
/// See [`Z_RANGE_PARAM`].
pub const SCREEN_PARAM: u32 = 0x00b4_7f6c;
/// Bytes of [`update_render_data`]'s frame, from the original's
/// `ebp - 0x100`: a copy of the segment record (`+0`, 0x6c), the z matrix
/// (`+0x6c`, 16 floats), the fade factor and flag (`+0xb0`, `+0xb4`), the
/// texture iterator (`+0xb8`), the bounds' minimum (`+0xc8`), the
/// maximum, later the texture record (`+0xd8`), and the depth range
/// value (`+0xfc`).
pub const RENDER_FRAME: u32 = 0x100;

/// The `i`th element of the `PArray` at `this + 0x14`.
fn element(h: &impl Host, this: u32, i: u32) -> u32 {
    let array: u32 = h.read(this + 0x14);
    let elements = crate::h4_m2::element(h, array).unwrap_or(0);
    h.read(elements.wrapping_add(4 * i))
}

/// `f32` bits copied through `fld`/`fstp`.
fn copy(bits: u32) -> u32 {
    store(load(bits))
}

/// `f32` bits negated through `fld`/`fchs`/`fstp`.
fn negate(bits: u32) -> u32 {
    store(-load(bits))
}

/// A parameter's slice of the buffer (`[param+8]` the offset, `[param+0xa]
/// & 0x1fff` the size) filled from `src` with `memcpy` when it fits
/// (`offset + size <= [buffer]`, `offset >= 4`).
fn copy_param(h: &mut impl Host, buffer: u32, param: u32, src: u32) {
    let offset = u32::from(h.read::<u16>(param + 8));
    let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
    if offset + size <= h.read::<u32>(buffer) && offset >= 4 {
        h.call_sym(
            MEMCPY,
            Conv::Cdecl,
            &[buffer.wrapping_add(offset), src, size],
        );
    }
}

/// `ClassDynamicGeometry_updateRenderData(this; unused)` (`0x006a4a50`,
/// thiscall, ret 4, the word never read; `frame` see [`RENDER_FRAME`]):
/// with the geometry (`+0x24`), a mesh instance (`+4`) and materials
/// (`+0x14`): fills the dynamic mesh's draw table (`+8`), sets the
/// instance's world matrix and bounds (`+0xc`), binds the segments'
/// texture clusters once (`+0x30` set, cleared again when one is
/// missing), then per material the depth range, the z matrix (the
/// geometry's rows with x and y negated), the screen parameters, the soft
/// particle setup and the camera fade. Returns 0, 1 (no instance or
/// materials) or a binding's error.
#[allow(clippy::too_many_lines)] // the original's passes
pub fn update_render_data(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let record = frame;
    let rows = frame + 0x6c;
    let factor = frame + 0xb0;
    let fade = frame + 0xb4;
    let iter = frame + 0xb8;
    let min = frame + 0xc8;
    let max = frame + 0xd8;
    let z_range = frame + 0xfc;
    let geometry: u32 = h.read(this + 0x24);
    if geometry == 0 {
        return 0;
    }
    if h.read::<u32>(this + 4) == 0 {
        return 1;
    }
    let array: u32 = h.read(this + 0x14);
    if array == 0 || h.read::<u32>(array).trailing_zeros() >= 31 {
        return 1;
    }
    let mesh: u32 = h.read(this + 8);
    if mesh != 0 {
        h.call_sym(FILL_DRAW_TABLE, Conv::Thiscall(mesh), &[geometry]);
    }
    let instance: u32 = h.read(this + 0xc);
    if instance != 0 {
        let geometry: u32 = h.read(this + 0x24);
        h.call_sym(SET_WORLD_MATRIX, Conv::Stdcall, &[instance, geometry]);
        let instance: u32 = h.read(this + 0xc);
        let bounds: u32 = h.read(instance + 0x1c);
        if bounds != 0 {
            let g: u32 = h.read(this + 0x24);
            for k in 0..3 {
                let v = copy(h.read(g + 0xa4 + 4 * k));
                h.write(max + 4 * k, v);
                let v = copy(h.read(g + 0x98 + 4 * k));
                h.write(min + 4 * k, v);
            }
            h.call_sym(SET_MIN_MAX, Conv::Thiscall(bounds), &[min, max]);
        }
    }
    // the segments' texture clusters, once
    if h.read::<u8>(this + 0x30) == 0 {
        h.write(this + 0x30, 1u8);
        let mut i = 0u32;
        while i.cast_signed()
            < h.read::<u32>(h.read::<u32>(this + 0x24) + 0x90)
                .cast_signed()
        {
            let offset = 0x6c * i;
            let g: u32 = h.read(this + 0x24);
            let records: u32 = h.read(g + 0x94);
            let mut copy_bytes = [0u8; 0x6c];
            h.read_bytes(records.wrapping_add(offset), &mut copy_bytes);
            h.write_bytes(record, &copy_bytes);
            let name: u32 = h.read(record + 0x20);
            let material = if name == 0 { 0 } else { element(h, this, i) };
            if material != 0 {
                let shader: u32 = h.read(this + 0x28);
                let p_texture = {
                    let n = h.rebase(TEXTURE_PARAM);
                    h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[n])
                };
                let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
                let cluster = h.call_sym(CLUSTER_BY_NAME, Conv::Thiscall(manager), &[name]);
                let shader: u32 = h.read(this + 0x28);
                let p_sampler = {
                    let n = h.rebase(SAMPLER_PARAM);
                    h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[n])
                };
                if p_sampler != 0 {
                    let buffer: u32 = h.read(material + 4);
                    let s = sampler(h, this, offset);
                    h.call_sym(SET_VALUE_DESC, Conv::Thiscall(buffer), &[p_sampler, s]);
                }
                if cluster != 0 && p_texture != 0 {
                    h.call_sym(TEXTURE_ITERATOR, Conv::Thiscall(iter), &[cluster]);
                    let base: u32 = h.read(iter);
                    let tex = base.wrapping_add(h.read(iter + 0x10));
                    h.write(max + 0xc, tex);
                    h.write(max, 0xffu32);
                    h.write(max + 4, 0u32);
                    let s = sampler(h, this, offset);
                    h.write(max + 8, s);
                    let buffer: u32 = h.read(material + 4);
                    let r = h.call_sym(SURFACE_FORMAT, Conv::Thiscall(buffer), &[p_texture, max]);
                    if r != 0 {
                        return r;
                    }
                } else {
                    h.write(this + 0x30, 0u8);
                }
            }
            i = i.wrapping_add(1);
        }
    }
    let array: u32 = h.read(this + 0x14);
    if array == 0 {
        return 0;
    }
    let count = h.read::<u32>(array) & 0x7fff_ffff;
    // the depth range and the z matrix
    if h.read::<u32>(h.read::<u32>(this + 0x24) + 0x80) != 0 {
        for k in 0..count {
            let material = element(h, this, k);
            let p = find(h, this, Z_RANGE_PARAM);
            if p != 0 {
                let g: u32 = h.read(this + 0x24);
                let range: u32 = h.read(g + 0x80);
                h.write(z_range, store(f64::from(range.cast_signed())));
                let buffer: u32 = h.read(material + 4);
                h.call_sym(SET_VALUE2, Conv::Thiscall(buffer), &[p, z_range]);
            }
            let q = find(h, this, Z_MATRIX_PARAM);
            if q != 0 {
                let g: u32 = h.read(this + 0x24);
                let buffer: u32 = h.read(material + 4);
                for (row, at) in [(0x40, min), (0x50, min), (0x60, min), (0x70, max)] {
                    let x = negate(h.read(g + row));
                    h.write(at, x);
                    let y = negate(h.read(g + row + 4));
                    h.write(at + 4, y);
                    let z = copy(h.read(g + row + 8));
                    h.write(at + 8, z);
                    let r = rows + (row - 0x40);
                    for c in 0..3 {
                        let v = copy(h.read(at + 4 * c));
                        h.write(r + 4 * c, v);
                    }
                    let w = if row == 0x70 { 1f32.to_bits() } else { 0 };
                    h.write(r + 0xc, w);
                }
                copy_param(h, buffer, q, rows);
            }
        }
    }
    // the screen parameters
    if h.read::<u32>(h.read::<u32>(this + 0x24) + 0x84) != 0 {
        let count = h.read::<u32>(h.read::<u32>(this + 0x14)) & 0x7fff_ffff;
        for k in 0..count {
            let p = find(h, this, SCREEN_PARAM);
            if p != 0 {
                let g: u32 = h.read(this + 0x24);
                let material = element(h, this, k);
                let buffer: u32 = h.read(material + 4);
                h.call_sym(SET_VALUE2, Conv::Thiscall(buffer), &[p, g + 0x88]);
            }
        }
    }
    // the soft particles
    let depth = h.read::<u8>(h.read::<u32>(this + 0x24) + 0xb6);
    if depth != 0 {
        let count = h.read::<u32>(h.read::<u32>(this + 0x14)) & 0x7fff_ffff;
        for k in 0..count {
            let material = element(h, this, k);
            let g: u32 = h.read(this + 0x24);
            let depth = u32::from(h.read::<u8>(g + 0xb6));
            let shader: u32 = h.read(this + 0x28);
            let a = copy(h.read(h.rebase(SOFT_A)));
            let b = copy(h.read(h.rebase(SOFT_B)));
            h.call_sym(SOFT_PARTICLE, Conv::Cdecl, &[material, shader, depth, a, b]);
        }
    }
    // the camera fade
    if h.read::<u8>(h.read::<u32>(this + 0x24) + 0xb7) != 0 {
        let count = h.read::<u32>(h.read::<u32>(this + 0x14)) & 0x7fff_ffff;
        for k in 0..count {
            let material = element(h, this, k);
            let p_fade = find(h, this, FADE_PARAM);
            let p_factor = find(h, this, FADE_FACTOR_PARAM);
            if p_factor == 0 || p_fade == 0 {
                continue;
            }
            h.write(fade, 0u32);
            h.write(factor, 1f32.to_bits());
            let g: u32 = h.read(this + 0x24);
            if g != 0 {
                let depth: u8 = h.read(g + 0xb7);
                if depth != 0 {
                    h.write(fade, 1f32.to_bits());
                    h.write(factor, store(f64::from(depth)));
                }
            }
            let buffer: u32 = h.read(material + 4);
            copy_param(h, buffer, p_fade, fade);
            let buffer: u32 = h.read(material + 4);
            copy_param(h, buffer, p_factor, factor);
        }
    }
    0
}

/// `ShaderParams_FindByName(this's shader (+0x28); name)` of the image
/// string `name`.
fn find(h: &mut impl Host, this: u32, name: u32) -> u32 {
    let shader: u32 = h.read(this + 0x28);
    let name = h.rebase(name);
    h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[name])
}

/// The scene's linear sampler for segment record `offset`.
fn sampler(h: &mut impl Host, this: u32, offset: u32) -> u32 {
    let g: u32 = h.read(this + 0x24);
    let records: u32 = h.read(g + 0x94);
    let which: u32 = h.read(records.wrapping_add(offset + 0x24));
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[which])
}
