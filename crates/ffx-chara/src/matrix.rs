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
}
