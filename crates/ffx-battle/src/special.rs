//! The battle's special stages (`docs/rewrite-plan.md`, M2): the
//! battles fought on a moving stage (`g_BtlSpecialKind`, see
//! [`crate::free`]), whose work (`g_BtlSpecialMem`) holds one matrix per
//! actor (0x1f of 0x40 bytes), the sea's matrix at `+0x7c0` and the
//! stage's angles from `+0x800`.
//!
//! `Yn_SeaSetMatrix` (map) goes to the game's code through `call`; the
//! C runtime's trigonometry comes from the caller.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // actors, matrices and angles
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::exec::bridge::Call;
    use crate::pos::bridge::Trig;
    use crate::target::bridge::Dm;
    use crate::vector::bridge::{Axis, ms_rot_matrix};

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    /// `Yn_SeaSetMatrix(m)` (map).
    pub const YN_SEA_SET_MATRIX: u32 = 0x0091_ae60;
    /// 0.01 (`f64`), the actors' scale to the matrices' (`0x00b54cb8`).
    pub const HUNDREDTH: Global<f64> = Global::new(0x00b5_4cb8);
    /// π/2 (`f64`, `0x00b11338`).
    pub const HALF_PI: Global<f64> = Global::new(0x00b1_1338);

    /// `MsSpecialSeaMatrix_0079df90(chr, id, work)`: the sea's matrix
    /// (`work + 0x7c0`) is the model's optional position `id`'s
    /// (`Ch_OptposGetMatrix`), its first three rows normalized
    /// (`MsVectorNormal`), and set (`Yn_SeaSetMatrix`).
    pub fn ms_special_sea_matrix<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        chr: u32,
        id: u32,
        work: u32,
    ) {
        let m = work.wrapping_add(0x7c0);
        ffx_chara::attach::bridge::ch_optpos_get_matrix(mem, chr, id, 0, m);
        for row in [0, 0x10, 0x20] {
            let v = m.wrapping_add(row);
            crate::vector::bridge::ms_vector_normal(mem, v, v);
        }
        let f = code(mem, YN_SEA_SET_MATRIX);
        call(mem, f, &[m]);
    }

    /// `MsSpecialChrMatrix_0079e030(angles)` (`0x0079e030`; a second
    /// argument is pushed, never read): with a special stage's work, each
    /// of the 0x1f actors with a model, not hidden (`+0x32`) and no ride
    /// (`+0x12`) gets its matrix in the work: its scale (`+0x3a0`..,
    /// times 0.01) turned by its direction plus π/2 (`MsGetChrDir`), then
    /// by the stage's angles `angles[2]` (Z) and `angles[0]` (X), placed
    /// at the model's position turned the same way (`Ch_GetPos`,
    /// `MsApplyMatrix`), and made the model's base matrix
    /// (`Ch_SetOrgMatrix`).
    ///
    /// The position's `w` is a local the original never writes (the same
    /// garbage on every actor; 0 here): it only reaches the VU registers'
    /// `w` lanes, and `x`, `y`, `z` unless it is infinite or a NaN. The
    /// direction is a local too: an actor without one keeps the last
    /// one's.
    pub fn ms_special_chr_matrix(mem: &mut Dm<'_>, trig: &dyn Fn(Trig, u32) -> u32, angles: u32) {
        use ffx_bridge::x87::{add, load, mul, store};
        use ffx_ps2::vu0::bridge::matrix_identity;
        let work = crate::free::bridge::SPECIAL_MEM.get(mem);
        if work == 0 {
            return;
        }
        // the original's locals: the position, the turn, the direction
        mem.locals(0x60, |mem, at| {
            let (pos, turn, dir) = (at, at + 0x10, at + 0x50);
            let angle = |mem: &Dm<'_>, k: u32| store(load(mem.read(angles.wrapping_add(4 * k))));
            for k in 0..0x1f {
                let a = ms_get_chr(mem, k);
                let chr: u32 = mem.read(a);
                if chr == 0
                    || mem.read::<u8>(a.wrapping_add(0x32)) != 0
                    || mem.read::<u8>(a.wrapping_add(0x12)) != 0
                {
                    continue;
                }
                ffx_chara::chr::bridge::ch_get_pos(mem, chr, pos, pos + 4, pos + 8);
                let m = work.wrapping_add(0x40 * k);
                matrix_identity(mem, turn);
                let z = angle(mem, 2);
                ms_rot_matrix(mem, trig, Axis::Z, turn, turn, z);
                let x = angle(mem, 0);
                ms_rot_matrix(mem, trig, Axis::X, turn, turn, x);
                crate::vector::bridge::ms_apply_matrix(mem, pos, turn, pos);
                matrix_identity(mem, m);
                let hundredth = HUNDREDTH.get(mem);
                for (o, lane) in [(0x3a0, 0), (0x3a4, 5), (0x3a8, 10)] {
                    let s: u32 = mem.read(a.wrapping_add(o));
                    mem.write(m.wrapping_add(4 * lane), store(mul(load(s), hundredth)));
                }
                crate::body::bridge::ms_get_chr_dir(mem, k, dir);
                let d = store(add(load(mem.read(dir)), HALF_PI.get(mem)));
                mem.write(dir, d);
                ms_rot_matrix(mem, trig, Axis::Y, m, m, d);
                let z = angle(mem, 2);
                ms_rot_matrix(mem, trig, Axis::Z, m, m, z);
                let x = angle(mem, 0);
                ms_rot_matrix(mem, trig, Axis::X, m, m, x);
                for lane in 0..3 {
                    let v: u32 = mem.read(pos + 4 * lane);
                    mem.write(m.wrapping_add(0x30 + 4 * lane), store(load(v)));
                }
                ffx_chara::chr::bridge::ch_set_org_matrix(mem, chr, m);
            }
        });
    }
}
