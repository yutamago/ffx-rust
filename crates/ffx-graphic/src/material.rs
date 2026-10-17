//! The shader parameters of character mesh segments: a material's
//! `MaterialColour` and `AlphaThreshold`, written into its parameter
//! buffer the way `ShaderParamBuffer_SetValue` (`0x00638780`) does.
//!
//! A mesh instance list is a count (bit 31 a flag) and an array of
//! instances; an instance's `+0` is its mesh (count, `+4` segments of
//! `0x6c` bytes: `+0` the material index, `+0x14` a hidden byte, `+0x18`
//! the segment id) and `+0x10` its materials (count; one inline at `+4`,
//! several through the pointer at `+4`). A material's `+0` is its shader
//! (searched by `ShaderParams_FindByName`), `+4` its parameter buffer (a
//! size word, then the data); a parameter's `+8` (`u16`) is its offset in
//! the buffer and `+0xa` (`u16 & 0x1fff`) its size.

/// The originals, over game memory.
pub mod bridge {
    use crate::host::{Conv, Host};
    use ffx_bridge::{Mem, x87};

    /// `ShaderParams_FindByName(shader, name)` (`thiscall`): the parameter
    /// named `name`, or 0.
    pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
    /// `PhyreScene_ForceTransparent(material, on)` (cdecl).
    pub const FORCE_TRANSPARENT: u32 = 0x0065_1400;
    /// `"MaterialColour"` and `"AlphaThreshold"`.
    pub const NAME_MATERIAL_COLOUR: u32 = 0x00b4_1980;
    pub const NAME_ALPHA_THRESHOLD: u32 = 0x00b4_1bf0;
    /// The `0.001` (`f32`) `AlphaThreshold` is compared with.
    pub const ALPHA_THRESHOLD_MIN: u32 = 0x00b3_68a4;
    /// `TextureAnimationManager_GetInstance`,
    /// `..._getMaterialByLayNum`, `ShaderParamBuffer_SetValue`.
    pub const TEX_ANIM_GET_INSTANCE: u32 = 0x006b_02e0;
    pub const TEX_ANIM_MATERIAL_BY_LAYER: u32 = 0x006b_2070;
    pub const SHADER_PARAM_SET_VALUE: u32 = 0x0063_8780;
    /// The `1 / 128` (`f64`) a layer alpha is scaled by.
    pub const ALPHA_SCALE: u32 = 0x00b1_70b8;

    /// Where parameter `param` lies in `buf`, when it fits (offset at
    /// least 4, offset + size within the size word): `(address, size)`.
    fn slot(m: &impl Mem, buf: u32, param: u32) -> Option<(u32, u32)> {
        let off = u32::from(m.read::<u16>(param + 8));
        let size = u32::from(m.read::<u16>(param + 0xa) & 0x1fff);
        (off + size <= m.read::<u32>(buf) && off >= 4).then(|| (buf.wrapping_add(off), size))
    }

    /// Copies the first `size` bytes of `src` to `at` (the originals'
    /// `memcpy` from a 16-byte local: parameters are at most that; a
    /// larger one would copy the original's stack frame, and gets zeros
    /// here).
    fn copy(m: &mut impl Mem, at: u32, src: &[u8], size: u32) {
        for i in 0..size {
            let b = src.get(i as usize).copied().unwrap_or(0);
            m.write(at + i, b);
        }
    }

    /// Instance `i` of `list`.
    fn instance(m: &impl Mem, list: u32, i: u32) -> u32 {
        let arr: u32 = m.read(list + 4);
        m.read(arr.wrapping_add(4 * i))
    }

    /// Material `index` of `inst` (`+0x10`: one inline, several by
    /// pointer; with none the original reads from address 0).
    fn material(m: &impl Mem, inst: u32, index: u32) -> u32 {
        let mats: u32 = m.read(inst + 0x10);
        let n = m.read::<u32>(mats) & 0x7fff_ffff;
        let base = match n {
            0 => 0,
            1 => mats + 4,
            _ => m.read(mats + 4),
        };
        m.read(base.wrapping_add(index.wrapping_mul(4)))
    }

    /// The colour `(r, g, b, a)` (through the x87) as the 16 bytes the
    /// originals copy.
    fn colour_bytes(c: [f32; 4]) -> [u8; 16] {
        let mut out = [0u8; 16];
        for (k, f) in c.into_iter().enumerate() {
            out[4 * k..4 * k + 4].copy_from_slice(&x87::round_trip(f).to_le_bytes());
        }
        out
    }

    /// `MaterialColour` of `mat` = `c`, then
    /// `PhyreScene_ForceTransparent(mat, a < 1)` (ordered), when the
    /// shader has the parameter.
    fn set_colour(h: &mut impl Host, mat: u32, c: [f32; 4]) {
        let shader: u32 = h.read(mat);
        let buf: u32 = h.read(mat + 4);
        let name = h.rebase(NAME_MATERIAL_COLOUR);
        let param = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if param == 0 {
            return;
        }
        if let Some((at, size)) = slot(h, buf, param) {
            copy(h, at, &colour_bytes(c), size);
        }
        let transparent = u32::from(x87::load(c[3].to_bits()) < 1.0);
        h.call_sym(FORCE_TRANSPARENT, Conv::Cdecl, &[mat, transparent]);
    }

    /// `graphicCharacterInstanceSetMeshColor(list, _, r, g, b, a)`
    /// (`0x0063bb30`): every segment of every instance gets the colour.
    pub fn set_mesh_colour(h: &mut impl Host, list: u32, c: [f32; 4]) {
        if list == 0 {
            return;
        }
        let mut i = 0;
        while i < h.read::<u32>(list) & 0x7fff_ffff {
            let mesh: u32 = h.read(instance(h, list, i));
            let n = h.read::<u32>(mesh) & 0x7fff_ffff;
            for k in 0..n {
                let segs: u32 = h.read(mesh + 4);
                let index: u32 = h.read(segs.wrapping_add(0x6c * k));
                let inst = instance(h, list, i);
                let mat = material(h, inst, index);
                set_colour(h, mat, c);
            }
            i += 1;
        }
    }

    /// `graphicCharacterInstanceSetMeshSegmentColor(list, id, r, g, b, a,
    /// hide)` (`0x0063bc80`): the segments whose id (`+0x18`) is `id` get
    /// the colour; with `hide`, an alpha of 0 instead hides the segment
    /// (byte `+0x14` = 1, colour left) and a positive one shows it first.
    pub fn set_mesh_segment_colour(h: &mut impl Host, list: u32, id: u32, c: [f32; 4], hide: u32) {
        if list == 0 {
            return;
        }
        let alpha = x87::load(c[3].to_bits());
        let mut i = 0;
        while i < h.read::<u32>(list) & 0x7fff_ffff {
            let mesh: u32 = h.read(instance(h, list, i));
            let n = h.read::<u32>(mesh) & 0x7fff_ffff;
            for k in 0..n {
                let segs: u32 = h.read(mesh + 4);
                let seg = segs.wrapping_add(0x6c * k);
                if h.read::<u32>(seg + 0x18) != id {
                    continue;
                }
                if hide != 0 {
                    if alpha == 0.0 {
                        h.write(seg + 0x14, 1u8);
                        continue;
                    }
                    if alpha > 0.0 {
                        h.write(seg + 0x14, 0u8);
                    }
                }
                let index: u32 = h.read(seg);
                let inst = instance(h, list, i);
                let mat = material(h, inst, index);
                set_colour(h, mat, c);
            }
            i += 1;
        }
    }

    /// `graphicCharacterInstanceSetMeshAlphaThreshold(list)`
    /// (`0x0063b9e0`): every segment's material whose `AlphaThreshold` is
    /// not below `0.001` (or is NaN) gets it zeroed. A parameter outside
    /// its buffer is compared at address 0 in the original (a fault).
    pub fn set_mesh_alpha_threshold(h: &mut impl Host, list: u32) {
        if list == 0 {
            return;
        }
        let min = x87::load(h.read(h.rebase(ALPHA_THRESHOLD_MIN)));
        let mut i = 0;
        while i < h.read::<u32>(list) & 0x7fff_ffff {
            let mesh: u32 = h.read(instance(h, list, i));
            let n = h.read::<u32>(mesh) & 0x7fff_ffff;
            for k in 0..n {
                let segs: u32 = h.read(mesh + 4);
                let index: u32 = h.read(segs.wrapping_add(0x6c * k));
                let inst = instance(h, list, i);
                let mat = material(h, inst, index);
                let shader: u32 = h.read(mat);
                let buf: u32 = h.read(mat + 4);
                let name = h.rebase(NAME_ALPHA_THRESHOLD);
                let param = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
                if param == 0 {
                    continue;
                }
                let at = slot(h, buf, param).map_or(0, |(at, _)| at);
                let v = x87::load(h.read(at));
                // `fcomp; test ah, 0x41; je`: skipped only when min > v
                if min > v {
                    continue;
                }
                if let Some((at, size)) = slot(h, buf, param) {
                    copy(h, at, &[0; 4], size);
                }
            }
            i += 1;
        }
    }

    /// `PhyreScene_SetLayerMaterialAlpha(scene, layer, alpha)`
    /// (`0x006582e0`): when the scene draws layers (byte `+0x93c`, word
    /// `+0x1128`), the texture-animation material of `layer` gets
    /// `MaterialColour` `(1, 1, 1, alpha / 128)` (`ShaderParamBuffer_SetValue`
    /// on its buffer `+4`, from a 16-byte value in [`Host::scratch`]).
    pub fn scene_set_layer_material_alpha(h: &mut impl Host, scene: u32, layer: u32, alpha: u32) {
        if h.read::<u8>(scene + 0x93c) == 0 || h.read::<u32>(scene + 0x1128) == 0 {
            return;
        }
        let mgr = h.call_sym(TEX_ANIM_GET_INSTANCE, Conv::Cdecl, &[]);
        let mat = h.call_sym(TEX_ANIM_MATERIAL_BY_LAYER, Conv::Thiscall(mgr), &[layer]);
        if mat == 0 {
            return;
        }
        let shader: u32 = h.read(mat);
        let name = h.rebase(NAME_MATERIAL_COLOUR);
        let param = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if param == 0 {
            return;
        }
        #[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)] // fild, fstp m32
        let a = (alpha as i32) as f32;
        let scale = f64::from_bits(h.read(h.rebase(ALPHA_SCALE)));
        let a = x87::store(x87::mul(f64::from(a), scale));
        let v = h.scratch();
        let one = 1f32.to_bits();
        for (k, w) in (0u32..).zip([one, one, one, a]) {
            h.write(v + 4 * k, w);
        }
        let buf: u32 = h.read(mat + 4);
        h.call_sym(SHADER_PARAM_SET_VALUE, Conv::Thiscall(buf), &[param, v]);
    }

    /// `graphicCharacterInstanceSetMeshSegmentAlpha(list, _, id, a)`
    /// (`0x0063bc50`): `SetMeshSegmentColor(list, id, 1, 1, 1, a, 1)`.
    pub fn set_mesh_segment_alpha(h: &mut impl Host, list: u32, id: u32, a: f32) {
        set_mesh_segment_colour(h, list, id, [1.0, 1.0, 1.0, a], 1);
    }

    /// `graphicChangeLayerAlpha(layer, alpha)` (`0x00639d20`):
    /// [`scene_set_layer_material_alpha`] on `g_PhyreScene`.
    pub fn change_layer_alpha(h: &mut impl Host, layer: u32, alpha: u32) {
        let scene = crate::scene::bridge::SCENE.get(h);
        scene_set_layer_material_alpha(h, scene, layer, alpha);
    }
}
