//! The render-side glue, sixty-first batch (M4 port round 17, list
//! `h3-glue-render`): a VFX dynamic geometry's shader parameters.

/// `ClassDynamicGeometry_updateRenderData(geometry; a)` (`ret 4`).
pub const UPDATE_RENDER_DATA: u32 = 0x006a_4a50;
/// `ShaderParams_FindByName(shader; name)` (`ret 4`).
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
/// `memcpy` (the import thunk, cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// `g_PhyreScene` and `PhyreScene_getYIQParam(scene; &scale x3, &shift
/// x3)` (`ret 0x18`).
pub const SCENE: u32 = 0x00cc_b9d8;
pub const GET_YIQ_PARAM: u32 = 0x0065_e9e0;
/// The parameter names: `colBs`, `colSs`, `lv`, `LvMatrix`, `LcMatrix`,
/// `MdiLight`, `YIQscale`, `YIQshift`, `uvOffset`, `rgbaPower`.
pub const COL_BS: u32 = 0x00b4_7f7c;
pub const COL_SS: u32 = 0x00b4_7f84;
pub const LV: u32 = 0x00b4_7f8c;
pub const LV_MATRIX: u32 = 0x00b4_1c00;
pub const LC_MATRIX: u32 = 0x00b4_1c0c;
pub const MDI_LIGHT: u32 = 0x00b4_7f90;
pub const YIQ_SCALE: u32 = 0x00b4_3c98;
pub const YIQ_SHIFT: u32 = 0x00b4_3ca4;
pub const UV_OFFSET: u32 = 0x00b4_7f9c;
pub const RGBA_POWER: u32 = 0x00b4_7fa8;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    const COUNT_MASK: u32 = 0x7fff_ffff;
    const ONE: u32 = 0x3f80_0000;
    /// 255.0 and -256.0 (f32).
    const MAX_LV: u32 = 0x437f_0000;
    const MIN_LV: u32 = 0xc380_0000;

    /// The frame's records (`ebp` offsets).
    const COL_SS_REC: u32 = 0x5c;
    const COL_BS_REC: u32 = 0x4c;
    const LV_REC: u32 = 0x3c;
    const SHIFT_REC: u32 = 0x24;
    const SCALE_REC: u32 = 0x14;

    /// The geometry's render elements (`+0x14`, a counted array): the
    /// count.
    fn count(h: &impl Host, this: u32) -> u32 {
        let list: u32 = h.read(this + 0x14);
        h.read::<u32>(list) & COUNT_MASK
    }

    /// Element `i` of the list (its data inline for one, none for none).
    fn element(h: &impl Host, this: u32, i: u32) -> u32 {
        let list: u32 = h.read(this + 0x14);
        let data = match h.read::<u32>(list) & COUNT_MASK {
            0 => 0,
            1 => list.wrapping_add(4),
            _ => h.read(list.wrapping_add(4)),
        };
        h.read(data.wrapping_add(4 * i))
    }

    /// The shader's (`+0x28`) parameter `name`.
    fn find(h: &mut impl Host, this: u32, name: u32) -> u32 {
        let shader: u32 = h.read(this + 0x28);
        let name = h.rebase(name);
        h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name])
    }

    /// `memcpy(buffer + offset, from, size)` with the parameter's offset
    /// (`+8`, 16 bits) and size (`+0xa`, 13 bits) when it fits the
    /// buffer (its size the first word) past the size word.
    fn put(h: &mut impl Host, parameter: u32, buffer: u32, from: u32) {
        let offset = u32::from(h.read::<u16>(parameter.wrapping_add(8)));
        let size = u32::from(h.read::<u16>(parameter.wrapping_add(0xa)) & 0x1fff);
        if offset + size > h.read::<u32>(buffer) || offset < 4 {
            return;
        }
        h.call_sym(
            MEMCPY,
            Conv::Cdecl,
            &[buffer.wrapping_add(offset), from, size],
        );
    }

    /// Four colour bytes of the geometry as f32 into the record.
    fn colour(h: &mut impl Host, this: u32, at: u32, record: u32) {
        for k in 0..4 {
            let v = h.read::<u8>(this + at + k);
            h.write(record + 4 * k, f32::from(v).to_bits());
        }
    }

    /// `VFXDynamicGeometry_updateRenderDataWithoutAddingToArray(geometry;
    /// a)` (`ret 4`; frame from `ebp - 0x5c`, the security cookie left
    /// out) -> what `ClassDynamicGeometry_updateRenderData` returns. Then
    /// for each render element (the shader `+0x28`, the element's
    /// parameter buffer `+4`): with `+0x40` the colours `+0x44` and
    /// `+0x48` (bytes as f32) into "colBs"/"colSs" and `+0x4c` clamped to
    /// -256..255 (a NaN gives 255) into "lv"; with `+0x52` the matrices
    /// `+0x54`, `+0x94` and the light `+0xd4`; with `+0xd8` the scene's
    /// YIQ scale and shift; always `+0xe0` "uvOffset" and `+0xe8`
    /// "rgbaPower". Records copy the parameter's own size: a size past a
    /// frame record reads the frame beyond it (junk read: the original
    /// reads its other locals and the caller's frame there).
    #[allow(clippy::too_many_lines)]
    pub fn update_render_data(h: &mut impl Host, ebp: u32, this: u32, a: u32) -> u32 {
        let result = h.call_sym(UPDATE_RENDER_DATA, Conv::Thiscall(this), &[a]);
        if h.read::<u8>(this + 0x40) != 0 && h.read::<u32>(this + 0x14) != 0 {
            let n = count(h, this);
            for i in 0..n {
                let element = element(h, this, i);
                let p = find(h, this, COL_BS);
                if p != 0 {
                    let buffer: u32 = h.read(element.wrapping_add(4));
                    colour(h, this, 0x44, ebp - COL_BS_REC);
                    put(h, p, buffer, ebp - COL_BS_REC);
                }
                let p = find(h, this, COL_SS);
                if p != 0 {
                    let buffer: u32 = h.read(element.wrapping_add(4));
                    colour(h, this, 0x48, ebp - COL_SS_REC);
                    put(h, p, buffer, ebp - COL_SS_REC);
                }
                let p = find(h, this, LV);
                if p != 0 {
                    let v = x87::load(x87::store(x87::load(h.read(this + 0x4c))));
                    // the original's fcomp: anything not below 255 (a NaN too) is 255
                    let below = matches!(v.partial_cmp(&255.0), Some(std::cmp::Ordering::Less));
                    let v = if !below {
                        MAX_LV
                    } else if -256.0 > v {
                        MIN_LV
                    } else {
                        x87::store(v)
                    };
                    h.write(ebp - LV_REC, v);
                    let buffer: u32 = h.read(element.wrapping_add(4));
                    put(h, p, buffer, ebp - LV_REC);
                }
            }
        }
        if h.read::<u8>(this + 0x52) != 0 && h.read::<u32>(this + 0x14) != 0 {
            let n = count(h, this);
            for i in 0..n {
                let element = element(h, this, i);
                for (name, at) in [(LV_MATRIX, 0x54), (LC_MATRIX, 0x94), (MDI_LIGHT, 0xd4)] {
                    let p = find(h, this, name);
                    if p != 0 && this.wrapping_add(at) != 0 {
                        let buffer: u32 = h.read(element.wrapping_add(4));
                        put(h, p, buffer, this.wrapping_add(at));
                    }
                }
            }
        }
        if h.read::<u8>(this + 0xd8) != 0 && h.read::<u32>(this + 0x14) != 0 {
            for k in 0..4 {
                h.write(ebp - SCALE_REC + 4 * k, ONE);
                h.write(ebp - SHIFT_REC + 4 * k, 0u32);
            }
            let n = count(h, this);
            let scene: u32 = h.read(h.rebase(SCENE));
            let (scale, shift) = (ebp - SCALE_REC, ebp - SHIFT_REC);
            h.call_sym(
                GET_YIQ_PARAM,
                Conv::Thiscall(scene),
                &[scale, scale + 4, scale + 8, shift, shift + 4, shift + 8],
            );
            for i in 0..n {
                let element = element(h, this, i);
                for (name, record) in [(YIQ_SCALE, scale), (YIQ_SHIFT, shift)] {
                    let p = find(h, this, name);
                    if p != 0 {
                        let buffer: u32 = h.read(element.wrapping_add(4));
                        put(h, p, buffer, record);
                    }
                }
            }
        }
        if h.read::<u32>(this + 0x14) != 0 {
            let n = count(h, this);
            for i in 0..n {
                let element = element(h, this, i);
                for (name, at) in [(UV_OFFSET, 0xe0), (RGBA_POWER, 0xe8)] {
                    let p = find(h, this, name);
                    if p != 0 && this.wrapping_add(at) != 0 {
                        let buffer: u32 = h.read(element.wrapping_add(4));
                        put(h, p, buffer, this.wrapping_add(at));
                    }
                }
            }
        }
        result
    }
}
