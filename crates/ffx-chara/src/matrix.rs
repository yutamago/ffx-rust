//! The character's matrices: the root (global and light) matrices at
//! `+0x1d0` and `+0x210`, each model's base matrix (its first element's
//! matrix), and setting up an element's matrix block.

/// The originals, on the characters.
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_ps2::vu0::bridge as vu0;
    use ffx_system::math::bridge::mul4x4;

    use crate::layout as l;

    /// The root global matrix (64 bytes).
    pub const ROOT_GLOBAL: u32 = 0x1d0;
    /// The root light matrix (64 bytes).
    pub const ROOT_LIGHT: u32 = 0x210;
    /// Element: its global matrix (64 bytes).
    pub const ELEM_GLOBAL: u32 = 0x88;
    /// Element: its local matrix (64 bytes).
    pub const ELEM_LOCAL: u32 = 0x8;

    /// `Ch_GetRootGlobalMatrix(chr, out)` (`0x00833f80`): copies the
    /// matrix to `out` when given; returns where it is.
    pub fn ch_get_root_global_matrix(mem: &mut impl Mem, chr: u32, out: u32) -> u32 {
        let m = chr + ROOT_GLOBAL;
        if out != 0 {
            vu0::matrix_copy(mem, out, m);
        }
        m
    }

    /// `Ch_GetRootLightMatrix(chr, out)` (`0x00833fc0`).
    pub fn ch_get_root_light_matrix(mem: &mut impl Mem, chr: u32, out: u32) -> u32 {
        let m = chr + ROOT_LIGHT;
        if out != 0 {
            vu0::matrix_copy(mem, out, m);
        }
        m
    }

    /// Model `i`'s base matrix: the global matrix of the element its data
    /// starts with (`u16` at the model data's `+0`).
    fn base(mem: &impl Mem, chr: u32, i: u32) -> u32 {
        let models: u32 = mem.read(chr + l::MODELS);
        let data: u32 = mem.read(models.wrapping_add(i.wrapping_mul(l::MODEL_SIZE)));
        let e = u32::from(mem.read::<u16>(data));
        mem.read::<u32>(chr + l::ELEMS)
            .wrapping_add(ELEM_GLOBAL)
            .wrapping_add(e * l::ELEM_SIZE)
    }

    /// `Ch_ModelGetBaseMatrix(chr, i, out)` (`0x00830500`): copies it to
    /// `out` when given; returns where it is.
    pub fn ch_model_get_base_matrix(mem: &mut impl Mem, chr: u32, i: u32, out: u32) -> u32 {
        let m = base(mem, chr, i);
        if out != 0 {
            vu0::matrix_copy(mem, out, m);
        }
        m
    }

    /// `Ch_ModelGetMatrix(chr, i, out)` (`0x00830550`): the root global
    /// matrix times model `i`'s base matrix, into `out` when given.
    pub fn ch_model_get_matrix(mem: &mut impl Mem, chr: u32, i: u32, out: u32) {
        if out != 0 {
            let m = base(mem, chr, i);
            mul4x4(mem, out, chr + ROOT_GLOBAL, m);
        }
    }

    /// `Ch_GetRootScaledMatrix(out, chr)` (`0x00826a80`): the root global
    /// matrix times a scale of 1000 on x, y and z.
    pub fn ch_get_root_scaled_matrix(mem: &mut impl Mem, out: u32, chr: u32) {
        // the identity's ones × 1000.0 (exact)
        let mut s = ffx_ps2::vu0::identity();
        for i in [0, 5, 10] {
            s[i] = 1000.0;
        }
        let root: [f32; 16] = mem.read(chr + ROOT_GLOBAL);
        let r = ffx_system::math::mul4x4(&root, &s);
        mem.write(out, r);
    }

    /// `Ch_ElemInit(e)` (`0x00828b60`): an element: no data (word 0),
    /// the vectors at `+0xe4..+0x100` cleared, `+0x104..+0x110` ones,
    /// both matrices the identity, `+0x144` 1. Returns 1.0's bits (left
    /// by the identity).
    pub fn ch_elem_init(mem: &mut impl Mem, e: u32) -> u32 {
        mem.write(e, 0u32);
        mem.write(e + 0x144, 1u8);
        vu0::matrix_identity(mem, e + ELEM_GLOBAL);
        let r = vu0::matrix_identity(mem, e + ELEM_LOCAL);
        mem.write(e + 0xe4, [0.0f32; 8]);
        mem.write(e + 0x104, [1.0f32; 4]);
        r
    }

    /// `1000.0` (`double`): the element translations' scale.
    pub const K_1000: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b0_dba0);

    /// The translation of `m` × 1000, as the originals scale an animated
    /// element's matrix.
    fn scale_translation(mem: &impl Mem, m: &mut [f32; 16]) {
        let k = crate::x87::Reg::constant(K_1000.get(mem));
        m[12] = crate::x87::Reg::load(m[12]).mul_st(k).store();
        m[13] = crate::x87::Reg::load(m[13]).mul_st(k).store();
        m[14] = k.mul_m32(m[14]).store();
    }

    fn elem_global(mem: &impl Mem, chr: u32, e: u32) -> u32 {
        mem.read::<u32>(chr + l::ELEMS)
            .wrapping_add(e.wrapping_mul(l::ELEM_SIZE))
            .wrapping_add(ELEM_GLOBAL)
    }

    /// The element's world matrix: the root global matrix times its
    /// global matrix, the translation × 1000 first when the motion
    /// animates it.
    fn elem_world(mem: &impl Mem, chr: u32, e: u32) -> [f32; 16] {
        let mut g: [f32; 16] = mem.read(elem_global(mem, chr, e));
        if crate::parts::bridge::ch_elem_in_motion(mem, chr, e) != 0 {
            scale_translation(mem, &mut g);
        }
        let root: [f32; 16] = mem.read(chr + ROOT_GLOBAL);
        ffx_system::math::mul4x4(&root, &g)
    }

    /// `Ch_ElemGetBaseMatrix(chr, e, out)` (`0x00833b80`): copies the
    /// element's global matrix to `out` when given (translation × 1000
    /// when the motion animates it); returns where it is.
    pub fn ch_elem_get_base_matrix(mem: &mut impl Mem, chr: u32, e: u32, out: u32) -> u32 {
        let g = elem_global(mem, chr, e);
        if out != 0 {
            vu0::matrix_copy(mem, out, g);
            if crate::parts::bridge::ch_elem_in_motion(mem, chr, e) != 0 {
                let mut m: [f32; 16] = mem.read(out);
                scale_translation(mem, &mut m);
                mem.write(out + 0x30, [m[12], m[13], m[14]]);
            }
        }
        g
    }

    /// `Ch_ElemGetMatrix(chr, e, out)` (`0x00833c30`): the element's world
    /// matrix into `out`, for an active character and a given `out`.
    pub fn ch_elem_get_matrix(mem: &mut impl Mem, chr: u32, e: u32, out: u32) {
        if out == 0 || mem.read::<u8>(chr + l::ACTIVE) == 0 {
            return;
        }
        let m = elem_world(mem, chr, e);
        mem.write(out, m);
    }

    /// `Ch_ElemGetPos(chr, e, v, out)` (`0x00833d10`): `v` through the
    /// element's world matrix into `out` (a vector of four), or without
    /// `v` the matrix's translation (x, y, z).
    pub fn ch_elem_get_pos(mem: &mut impl Mem, chr: u32, e: u32, v: u32, out: u32) {
        let m = elem_world(mem, chr, e);
        if v == 0 {
            mem.write(out, [12, 13, 14].map(|i| crate::x87::load(m[i])));
            return;
        }
        let x: [f32; 4] = mem.read(v);
        let r: [f32; 4] = std::array::from_fn(|i| {
            #[allow(clippy::cast_possible_truncation)] // 0..4
            let i = i as u32;
            let col = [0, 4, 8, 12].map(|row| m[(row + i) as usize]);
            ffx_system::math::mul4x4_component(i, x, col)
        });
        mem.write(out, r);
    }
}
