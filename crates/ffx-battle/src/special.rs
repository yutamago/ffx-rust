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

    /// `Yn_PSimGetHandle_00919f00(i)`, `Yn_PSimSetPos(h, k, v)`,
    /// `Yn_SeaSetPos(v)`, `Yn_SeaSetAng(v)`, `Yn_SeaSetUvSpd(u, v)` and
    /// `yiSetRotTransDObj(i, rot, pos)` (map).
    pub const YN_PSIM_GET_HANDLE: u32 = 0x0091_9f00;
    pub const YN_PSIM_SET_POS: u32 = 0x0091_9f10;
    pub const YN_SEA_SET_POS: u32 = 0x0091_ae80;
    pub const YN_SEA_SET_ANG: u32 = 0x0091_ae40;
    pub const YN_SEA_SET_UV_SPD: u32 = 0x0091_aea0;
    pub const YI_SET_ROT_TRANS_DOBJ: u32 = 0x0090_a750;
    /// The current map (`g_CurrentMapNo`).
    pub const CURRENT_MAP: Global<u32> = Global::new(0x012f_bc40);
    /// How many bytes of locals [`ms_battle_special`] needs in game memory.
    pub const SPECIAL_LOCALS: u32 = 0xa8;

    fn f32k(mem: &impl Mem, sym: u32) -> u32 {
        Global::<u32>::new(sym).get(mem)
    }

    fn f64k(mem: &impl Mem, sym: u32) -> f64 {
        Global::<f64>::new(sym).get(mem)
    }

    /// `MsBattleSpecial()` (`0x0079d430`): the special stage's frame
    /// (`g_BtlSpecialKind`). Kinds 2 and 3 take the sea's matrix from actor
    /// 0x14's model ([`ms_special_sea_matrix`], optional positions 0xe and
    /// 5) and set the sea's UV speed (0, 6). Kind 1 is the boat: the boat
    /// model's (actor 0x17) heading (`MsVectorRect2Polar` of its position,
    /// its optional position 0xe the particle emitter's) drives a rocking
    /// roll (`+0x838`, damped by 0.96 and by the swing count `+0x854`,
    /// which counts zero crossings), the stage's pitch (`+0x830`) eases
    /// towards it, three swaying parts (`+0x814`/`+0x824`, map objects
    /// 0x1c-0x1e; 0x1e level on map 0x3d) and a random swell (`+0x858`,
    /// `+0x85c`, `+0x860`, `+0x84c` through `brnd(3)`) move the sea
    /// (`+0x844`); the sea's angles (`+0x800`), position, UV speed and the
    /// waves' object 2, the boat objects 0x14, 0x15, 0x18-0x1a and the
    /// particle emitter 0 are set, then each actor's matrix
    /// ([`ms_special_chr_matrix`]).
    ///
    /// `locals` is [`SPECIAL_LOCALS`] bytes of game memory (zeroed) for the
    /// original's locals, which go to map code; the local at `ebp - x` is
    /// `locals + 0xa8 - x`. Without the boat model the original reads
    /// three position words it never wrote (and the fourth of that vector
    /// always): they are the zeroes of `locals` here.
    #[allow(clippy::too_many_lines)]
    pub fn ms_battle_special<'d>(
        mem: &mut Dm<'d>,
        call: Call<'_, Dm<'d>>,
        trig: &dyn Fn(Trig, u32) -> u32,
        locals: u32,
    ) {
        use crate::vector::bridge::ms_near_rag;
        use ffx_bridge::x87::{add, div, ftol, load, mul, store, sub};
        let game = |mem: &mut Dm<'d>,
                    call: &mut dyn FnMut(&mut Dm<'d>, u32, &[u32]) -> u32,
                    sym: u32,
                    args: &[u32]| {
            let f = code(mem, sym);
            call(mem, f, args)
        };
        let kind = crate::free::bridge::SPECIAL_KIND.get(mem);
        let work = crate::free::bridge::SPECIAL_MEM.get(mem);
        if kind == 2 || kind == 3 {
            let h = crate::actor::bridge::ms_get_chr_handle(mem, 0x14);
            if h != 0 {
                let id = if kind == 2 { 0xe } else { 5 };
                ms_special_sea_matrix(mem, &mut *call, h, id, work);
            }
            let six = store(load(f32k(mem, 0x00b9_2308)));
            game(mem, &mut *call, YN_SEA_SET_UV_SPD, &[0, six]);
            return;
        }
        if kind != 1 {
            return;
        }
        let l = |x: u32| locals + 0xa8 - x;
        let rd = |mem: &Dm<'d>, at: u32| -> u32 { mem.read(at) };
        let w = |o: u32| work.wrapping_add(o);
        let near = |mem: &Dm<'d>, a: u32, b: u32| ms_near_rag(mem, a, b).to_bits();
        let boat = crate::actor::bridge::ms_get_chr_handle(mem, 0x17);
        let emitter = game(mem, &mut *call, YN_PSIM_GET_HANDLE, &[0]);
        let mut a0 = f32k(mem, 0x00b5_4cd0);
        if boat != 0 {
            for (x, k) in [
                (0x34, 0x00b5_4cf8),
                (0x30, 0x00b5_4d0c),
                (0x2c, 0x00b5_4cfc),
            ] {
                let v = store(load(f32k(mem, k)));
                mem.write(l(x), v);
            }
            let v = l(0x44);
            ffx_chara::attach::bridge::ch_optpos_get(mem, boat, 0xe, v);
            game(mem, &mut *call, YN_PSIM_SET_POS, &[emitter, 1, v]);
            ffx_chara::chr::bridge::ch_get_pos(mem, boat, v, v + 4, v + 8);
            mem.write(l(0xa0), a0);
            crate::vector::bridge::ms_vector_rect2_polar(mem, trig, v, 0, l(0xa0), 0, 0);
            a0 = near(mem, 0, rd(mem, l(0xa0)));
        }
        let a4 = a0;
        let k = ftol(
            mem,
            mul(
                sub(load(a4), f64k(mem, 0x00b5_4ce0)),
                f64k(mem, 0x00b9_22e0),
            ),
        )
        .cast_signed();
        let swings = rd(mem, w(0x854)).cast_signed();
        #[allow(clippy::cast_precision_loss)] // fild, exact for these
        let factor = store(div(
            load(store(f64::from(5i32.wrapping_sub(swings)))),
            f64k(mem, 0x00b4_1d20),
        ));
        let kf = store(f64::from(k));
        let f = store(mul(load(kf), f64k(mem, 0x00b4_1cc0)));
        let pitch = rd(mem, w(0x830));
        let step = f64k(mem, 0x00b5_4ca8);
        if load(pitch) > load(f) {
            let r = rd(mem, w(0x838));
            mem.write(w(0x838), store(sub(load(r), step)));
            a0 = store(sub(load(pitch), load(f)));
        } else if load(pitch) < load(f) {
            let r = rd(mem, w(0x838));
            mem.write(w(0x838), store(add(load(r), step)));
            a0 = store(sub(load(f), load(pitch)));
        }
        let y = store(mul(load(rd(mem, w(0x838))), f64k(mem, 0x00b5_4cd8)));
        mem.write(w(0x838), y);
        if rd(mem, w(0x850)).cast_signed() == k {
            let last = load(rd(mem, w(0x83c)));
            let yv = load(y);
            if ((last < 0.0 && yv > 0.0) || (last > 0.0 && yv < 0.0)) && swings < 1 {
                mem.write(w(0x854), swings.wrapping_add(1));
            }
        } else {
            mem.write(w(0x854), 0u32);
        }
        let v = store(mul(load(y), load(factor)));
        mem.write(w(0x838), v);
        let roll = store(add(load(v), load(rd(mem, w(0x820)))));
        mem.write(w(0x820), roll);
        let r = near(mem, rd(mem, w(0x830)), roll);
        let p = rd(mem, w(0x830));
        let t = store(add(
            mul(sub(load(r), load(p)), f64k(mem, 0x00b9_21e0)),
            load(p),
        ));
        mem.write(w(0x830), t);
        let q = near(mem, 0, t);
        let left = rd(mem, w(0x858)).cast_signed().wrapping_sub(1);
        mem.write(w(0x858), left);
        mem.write(w(0x830), q);
        let v = store(load(rd(mem, w(0x838))));
        mem.write(w(0x83c), v);
        mem.write(w(0x850), k);
        if left < 0 {
            let n = crate::rng::bridge::brnd(mem, 3);
            mem.write(w(0x858), (n & 0x1f) + 0x78);
            let n = crate::rng::bridge::brnd(mem, 3);
            mem.write(w(0x84c), 0u32);
            let len = (n & 7) + 0x14;
            mem.write(w(0x860), len);
            mem.write(w(0x85c), len);
        }
        let v = store(add(load(rd(mem, w(0x84c))), f64k(mem, 0x00b5_4ce8)));
        mem.write(w(0x84c), v);
        let u = store(add(load(rd(mem, w(0x834))), f64k(mem, 0x00b5_4cb0)));
        let u = near(mem, 0, u);
        mem.write(w(0x834), u);
        let h = store(add(load(rd(mem, w(0x848))), f64k(mem, 0x00b5_4cc8)));
        mem.write(w(0x848), h);
        let sn = trig(Trig::Sin, h);
        let swell = store(mul(add(load(sn), 1.0), f64k(mem, 0x00b2_c5c8)));
        mem.write(w(0x844), swell);
        let n = rd(mem, w(0x85c)).cast_signed();
        if n > 0 {
            let n1 = n - 1;
            mem.write(w(0x85c), n1);
            let cs = trig(Trig::Cos, rd(mem, w(0x84c)));
            let d = rd(mem, w(0x860)).cast_signed();
            let x = add(
                mul(
                    div(mul(add(load(cs), 1.0), f64::from(n1)), f64::from(d)),
                    f64k(mem, 0x00b9_21e0),
                ),
                load(swell),
            );
            mem.write(w(0x844), store(x));
        }
        let base = div(load(kf), f64k(mem, 0x00b9_22e0));
        let (rot, pos) = (l(0x24), l(0x14));
        let one = 1.0f32.to_bits();
        for i in 0..3u32 {
            let dv = store(f64::from((8 - i) * 0x20));
            let z = store(add(div(base, load(dv)), load(rd(mem, w(0x814 + 4 * i)))));
            mem.write(w(0x814 + 4 * i), z);
            let m = rd(mem, w(0x824 + 4 * i));
            let nr = near(mem, m, z);
            let t2 = store(add(
                mul(sub(load(nr), load(m)), f64k(mem, 0x00b3_4f90)),
                load(m),
            ));
            mem.write(w(0x824 + 4 * i), t2);
            let t2 = near(mem, 0, t2);
            mem.write(w(0x824 + 4 * i), t2);
            let yy = store(add(load(rd(mem, w(0x834))), load(t2)));
            let mut ry = near(mem, 0, yy);
            let p = store(load(rd(mem, w(0x830))));
            if CURRENT_MAP.get(mem) == 0x3d && i == 2 {
                ry = 0;
            }
            mem.write(rot, [0, ry, p, one]);
            mem.write(pos, [0, 0, 0, one]);
            game(
                mem,
                &mut *call,
                YI_SET_ROT_TRANS_DOBJ,
                &[i + 0x1c, rot, pos],
            );
        }
        let d = store(sub(load(rd(mem, w(0x840))), load(a0)));
        mem.write(w(0x840), store(load(a0)));
        let ad = if 0.0 > load(d) { store(-load(d)) } else { d };
        let mut sp = store(sub(
            f64k(mem, 0x00b3_eb88),
            mul(load(ad), f64k(mem, 0x00b5_4cf0)),
        ));
        if load(sp) < 0.0 {
            sp = 0;
        }
        let ang = [
            0,
            store(load(rd(mem, w(0x828)))),
            store(load(rd(mem, w(0x830)))),
            one,
        ];
        mem.write(w(0x800), ang);
        mem.write(pos, [0, 0, 0, one]);
        game(mem, &mut *call, YN_SEA_SET_POS, &[pos]);
        game(mem, &mut *call, YN_SEA_SET_ANG, &[w(0x800)]);
        let c = trig(Trig::Cos, rd(mem, w(0x804)));
        let uv1 = store(mul(-load(c), load(sp)));
        let s = trig(Trig::Sin, rd(mem, w(0x804)));
        let uv0 = store(mul(-load(s), load(sp)));
        game(mem, &mut *call, YN_SEA_SET_UV_SPD, &[uv0, uv1]);
        let wave = store(sub(
            load(rd(mem, w(0x864))),
            mul(load(sp), f64k(mem, 0x00b5_4cc0)),
        ));
        mem.write(w(0x864), wave);
        let rx = near(mem, 0, wave);
        let kd08 = store(load(f32k(mem, 0x00b5_4d08)));
        let kd14 = store(load(f32k(mem, 0x00b5_4d14)));
        mem.write(rot, [rx, 0, 0, one]);
        mem.write(pos, [0, kd08, kd14, one]);
        game(mem, &mut *call, YI_SET_ROT_TRANS_DOBJ, &[2, rot, pos]);
        let pi = store(load(f32k(mem, 0x00b9_22d0)));
        let anchor: [u32; 3] = [rd(mem, l(0x34)), rd(mem, l(0x30)), rd(mem, l(0x2c))];
        mem.write(rot, [0, pi, 0, one]);
        mem.write(
            pos,
            [
                store(load(anchor[0])),
                store(load(anchor[1])),
                store(load(anchor[2])),
                one,
            ],
        );
        game(mem, &mut *call, YI_SET_ROT_TRANS_DOBJ, &[0x15, rot, pos]);
        let ry = store(sub(f64k(mem, 0x00b3_ebb0), load(a4)));
        let ry = near(mem, 0, ry);
        mem.write(rot + 4, ry);
        game(mem, &mut *call, YI_SET_ROT_TRANS_DOBJ, &[0x14, rot, pos]);
        let kd04 = store(load(f32k(mem, 0x00b5_4d04)));
        let kd00 = store(load(f32k(mem, 0x00b5_4d00)));
        mem.write(pos, [0, kd04, kd00, one]);
        let m = l(0x84);
        ffx_ps2::vu0::bridge::matrix_identity(mem, m);
        let r: [u32; 4] = mem.read(rot);
        ms_rot_matrix(mem, trig, Axis::Y, m, m, store(-load(r[1])));
        ms_rot_matrix(mem, trig, Axis::Z, m, m, store(-load(r[2])));
        ms_rot_matrix(mem, trig, Axis::X, m, m, store(-load(r[0])));
        crate::vector::bridge::ms_apply_matrix(mem, pos, m, pos);
        crate::vector::bridge::ms_add_point(mem, pos, pos, l(0x34));
        game(mem, &mut *call, YN_PSIM_SET_POS, &[emitter, 0, pos]);
        let k10 = store(load(f32k(mem, 0x00b5_4d10)));
        let k0c = store(load(f32k(mem, 0x00b5_4d0c)));
        let kfc = store(load(f32k(mem, 0x00b5_4cfc)));
        mem.write(rot, [0, pi, 0, one]);
        mem.write(pos, [k10, k0c, kfc, one]);
        for i in [0x18, 0x19, 0x1a] {
            game(mem, &mut *call, YI_SET_ROT_TRANS_DOBJ, &[i, rot, pos]);
        }
        ms_special_chr_matrix(mem, trig, w(0x800));
    }

    /// `MsAtelUpdate0_007972e0(ctrl)`: script 0's per-frame update
    /// (`AtelBattleSetUp`'s callback, as `MsAtelUpdate1`): over script
    /// control `ctrl`'s works each one's signals, then each one's command,
    /// then the actors' motions ([`crate::motion::bridge::ms_motion_process`])
    /// and the special stage ([`ms_battle_special`], `locals` as there),
    /// then the signals again. Returns the last `AtelDecodeSignal`'s
    /// result; without works the original leaves `MsBattleSpecial`'s `eax`,
    /// the count here.
    pub fn ms_atel_update0<'d>(
        mem: &mut Dm<'d>,
        s: &mut crate::atel::bridge::Script<'_, Dm<'d>>,
        trig: &dyn Fn(Trig, u32) -> u32,
        locals: u32,
        ctrl: u32,
    ) -> u32 {
        use crate::atel::bridge::{
            ATEL_DECODE_COMMAND, ATEL_DECODE_SIGNAL, ATEL_GET_CTRLWORK_ACTOR_TOTAL,
        };
        let game = |mem: &mut Dm<'d>,
                    s: &mut crate::atel::bridge::Script<'_, Dm<'d>>,
                    sym: u32,
                    args: &[u32]| {
            let f = code(mem, sym);
            (s.call)(mem, f, args)
        };
        let n = game(mem, s, ATEL_GET_CTRLWORK_ACTOR_TOTAL, &[ctrl]);
        let works = 0..n.cast_signed().max(0).cast_unsigned();
        let mut last = n;
        for k in works.clone() {
            last = game(mem, s, ATEL_DECODE_SIGNAL, &[k, u32::MAX]);
        }
        for k in works.clone() {
            game(mem, s, ATEL_DECODE_COMMAND, &[k, 0]);
        }
        crate::motion::bridge::ms_motion_process(mem, trig, s);
        ms_battle_special(mem, &mut *s.call, trig, locals);
        for k in works {
            last = game(mem, s, ATEL_DECODE_SIGNAL, &[k, u32::MAX]);
        }
        last
    }
}
