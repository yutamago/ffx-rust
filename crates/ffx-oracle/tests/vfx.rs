//! The effect functions (`ffx_vfx`) against the original, on two oracles
//! started alike: one runs `FFX.exe`'s code, the other the Rust bridge on
//! the same memory; results and every written byte must agree. Each test
//! runs on the image alone (synthetic state, needs `FFX_GAME_DIR`) and, with
//! `FFX_SNAPSHOT`, on the snapshot's state too.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use ffx_bridge::{Global, Mem, Value};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_vfx::{
    draw, et, fp, heap, lns, mat, md, mdl, op, opd, opus, part, ppp, prog, rand, slots, small,
    tree, vec, wave, x87,
};

fn exe() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    Some(dir?.join("FFX.exe"))
}

fn pages() -> Option<Arc<Pages>> {
    static PAGES: OnceLock<Option<Arc<Pages>>> = OnceLock::new();
    PAGES
        .get_or_init(|| {
            let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
                eprintln!("FFX_SNAPSHOT is not set; snapshot cases skipped");
                return None;
            };
            Some(Pages::read(PathBuf::from(snap)).unwrap())
        })
        .clone()
}

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn chance(&mut self, percent: u32) -> bool {
        self.below(100) < percent
    }
    fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[self.below(u32::try_from(of.len()).unwrap()) as usize]
    }
    /// Float bits: specials (quiet NaNs, infinities, zeros, denormals) a
    /// fifth of the time, else ordinary values. No signalling NaNs: the
    /// oracle's x87 keeps them signalling where the hardware quiets them
    /// (`x87_nans_match_the_oracle`).
    fn float(&mut self) -> u32 {
        if self.chance(20) {
            let f = self.pick(&SPECIALS);
            return if x87::is_snan(f) { f | 0x0040_0000 } else { f };
        }
        let mantissa = self.next() & 0x007f_ffff;
        let exp = 100 + self.below(56); // about 2^-27 .. 2^28
        (self.next() & 0x8000_0000) | (exp << 23) | mantissa
    }
}

const SPECIALS: [u32; 14] = [
    0x0000_0000,
    0x8000_0000,
    0x7f80_0000,
    0xff80_0000,
    0x7fc0_0000,
    0xffc0_0000,
    0x7fc1_2345,
    0xffe0_0001,
    0x7f80_0001, // signalling
    0xff9f_0000, // signalling
    0x0000_0001, // denormal
    0x807f_ffff, // denormal
    0x7f7f_ffff,
    0x3f80_0000,
];

/// The two oracles.
struct Pair {
    name: &'static str,
    a: Oracle,
    b: Oracle,
}

/// Free code space in the image (the end of `.text`), for the float thunk.
const THUNK: u32 = 0x00b0_bf00;

impl Pair {
    /// On the image alone, and on the snapshot when there is one.
    fn all() -> Vec<Pair> {
        let Some(exe) = exe() else { return Vec::new() };
        // Oracle::load sets the game thread's control word (0x027f)
        let image = Pair {
            name: "image",
            a: Oracle::load(&exe).unwrap(),
            b: Oracle::load(&exe).unwrap(),
        };
        let mut v = vec![image];
        if let Some(p) = pages() {
            v.push(Pair {
                name: "snapshot",
                a: Oracle::from_snapshot(&exe, &p).unwrap(),
                b: Oracle::from_snapshot(&exe, &p).unwrap(),
            });
        }
        v
    }

    fn snapshot(&self) -> bool {
        self.name == "snapshot"
    }

    fn sym(&self, sym: u32) -> u32 {
        self.a.rebase(sym)
    }

    fn set<T: Value>(&mut self, addr: u32, v: T) {
        Mem::write(&mut self.a, addr, v);
        Mem::write(&mut self.b, addr, v);
    }

    fn put<T: Value>(&mut self, g: Global<T>, v: T) {
        let at = self.sym(g.sym);
        self.set(at, v);
    }

    fn get<T: Value>(&self, addr: u32) -> T {
        Mem::read(&self.a, addr)
    }

    fn alloc(&mut self, len: usize) -> u32 {
        let x = self.a.alloc(len).unwrap();
        assert_eq!(self.b.alloc(len).unwrap(), x);
        x
    }

    fn fill(&mut self, at: u32, len: u32, rng: &mut Xorshift) {
        for i in (0..len).step_by(4) {
            let v = rng.next();
            self.set(at + i, v);
        }
    }

    /// Runs the original (cdecl) in `a` and `rust` in `b`; checks the result
    /// (when `rust` gives one) and the memory; returns the original's.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        args: &[u32],
        rust: impl FnOnce(&mut Oracle) -> Option<u32>,
    ) -> u32 {
        let at = self.sym(sym);
        let r = self
            .a
            .call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{}: {what}: {e}", self.name));
        // a float the original leaves in ST0 is a result: the port must
        // return it (check_f32 and float_call), not only eax
        assert!(
            r.st0.is_none(),
            "{}: {what}: the original returns {:?} in ST0",
            self.name,
            r.st0
        );
        let mine = rust(&mut self.b);
        if let Some(m) = mine {
            assert_eq!(r.eax, m, "{}: {what}: result", self.name);
        }
        self.same(what);
        r.eax
    }

    fn same(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{}: {what}: memory differs (address, original, rust): {:x?}",
            self.name,
            &d[..d.len().min(16)]
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// A thunk that calls `sym(*arg)` and stores its `f32` result (`ST0`)
    /// at `out`; returns the thunk, in both oracles alike.
    fn float_thunk(&mut self, sym: u32, arg: u32, out: u32) -> u32 {
        let at = self.sym(THUNK);
        let target = self.sym(sym);
        let mut c = vec![0xff, 0x35]; // push dword [arg]
        c.extend(arg.to_le_bytes());
        c.push(0xe8); // call target
        c.extend(target.wrapping_sub(at + 11).to_le_bytes());
        c.extend([0x83, 0xc4, 0x04]); // add esp, 4
        c.extend([0xd9, 0x1d]); // fstp dword [out]
        c.extend(out.to_le_bytes());
        c.push(0xc3);
        self.a.write(at, &c).unwrap();
        self.b.write(at, &c).unwrap();
        at
    }

    /// Calls a float thunk in `a` and stores `rust`'s result at `out` in
    /// `b`; checks both.
    fn check_f32(
        &mut self,
        what: &str,
        thunk: u32,
        out: u32,
        rust: impl FnOnce(&mut Oracle) -> f32,
    ) {
        self.a
            .call(thunk, Conv::Cdecl, &[])
            .unwrap_or_else(|e| panic!("{}: {what}: {e}", self.name));
        let v = rust(&mut self.b);
        Mem::write(&mut self.b, out, v.to_bits());
        let o: u32 = self.get(out);
        assert_eq!(o, v.to_bits(), "{}: {what}: result", self.name);
        self.same(what);
    }
}

// ---- rand -------------------------------------------------------------

const OP_RANDOM: u32 = 0x007e_6700;
const OP_RANDOM_P: u32 = 0x007e_6780;

#[test]
fn random_draws_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x2545_f491);
        let cells = p.alloc(8);
        let (arg, out) = (cells, cells + 4);
        for (name, sym, f) in [
            (
                "op_random",
                OP_RANDOM,
                rand::bridge::op_random as fn(&mut Oracle, f32) -> f32,
            ),
            ("op_random_p", OP_RANDOM_P, rand::bridge::op_random_p),
        ] {
            let thunk = p.float_thunk(sym, arg, out);
            // the game's own state first
            let x = 1.5f32;
            p.set(arg, x.to_bits());
            p.check_f32(
                &format!("{name}(1.5) on the state as it is"),
                thunk,
                out,
                |m| f(m, x),
            );
            for i in 0..3000 {
                let x = rng.float();
                if i % 3 == 0 {
                    p.put(rand::bridge::R, rng.next());
                    p.put(rand::bridge::SEEDED, u32::from(rng.chance(80)));
                }
                p.set(arg, x);
                p.check_f32(&format!("{name}({x:#x})"), thunk, out, |m| {
                    f(m, f32::from_bits(x))
                });
            }
        }
    }
}

// ---- ppp programs -----------------------------------------------------

const PPP_KE_DRCT_CON: u32 = 0x0075_e470;
const PPP_KE_DRCT: u32 = 0x0075_e430;
const PPP_KE_MAT_SN_CON: u32 = 0x0073_40f0;
const PPP_VT_MIME_CON: u32 = 0x0072_ed70;
const PPP_COLOR_CON: u32 = 0x0075_d240;
const PPP_COLOR: u32 = 0x0075_d1f0;
const PPP_ANGLE: u32 = 0x0075_ce30;
const PPP_NEI_LIGHT_EIKYO_CON: u32 = 0x0075_d6c0;
const PPP_NEI_LIGHT_EIKYO: u32 = 0x0075_d5d0;
const PPP_KE_LNS_LP_SFT: u32 = 0x0075_9f30;
const PPP_KE_TH_CON2: u32 = 0x0073_64e0;
const PPP_KE_TH_SFT_CON: u32 = 0x0073_7130;
const PPP_CLEAR_WAIT: u32 = 0x0076_b200;
const PPP_GET_ROT_MATRIX_X: u32 = 0x0072_c730;
const PPP_GET_ROT_MATRIX_Y: u32 = 0x0072_c920;
const PPP_GET_ROT_MATRIX_Z: u32 = 0x0072_cb10;

type Con = fn(&mut Oracle, u32, u32);
type Step = fn(&mut Oracle, u32, u32, u32);

#[test]
fn ppp_programs_match_the_original() {
    use ppp::bridge as b;
    let cons: [(&str, u32, Con); 6] = [
        ("pppKeDrctCon", PPP_KE_DRCT_CON, b::ppp_ke_drct_con),
        ("pppKeMatSNCon", PPP_KE_MAT_SN_CON, b::ppp_ke_mat_sn_con),
        ("pppVtMimeCon", PPP_VT_MIME_CON, b::ppp_vt_mime_con),
        ("pppColorCon", PPP_COLOR_CON, b::ppp_color_con),
        (
            "pppNeiLightEikyoCon",
            PPP_NEI_LIGHT_EIKYO_CON,
            b::ppp_nei_light_eikyo_con,
        ),
        ("pppKeThCon2", PPP_KE_TH_CON2, b::ppp_ke_th_con2),
    ];
    let steps: [(&str, u32, Step); 5] = [
        ("pppKeDrct", PPP_KE_DRCT, b::ppp_ke_drct),
        ("pppColor", PPP_COLOR, b::ppp_color),
        ("pppAngle", PPP_ANGLE, b::ppp_angle),
        (
            "pppNeiLightEikyo",
            PPP_NEI_LIGHT_EIKYO,
            b::ppp_nei_light_eikyo,
        ),
        ("pppKeLnsLpSft", PPP_KE_LNS_LP_SFT, b::ppp_ke_lns_lp_sft),
    ];
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x1357_9bdf);
        let obj = p.alloc(0x400);
        let key = p.alloc(0x40);
        let ctrl = p.alloc(0x10);
        let offsets = p.alloc(8);
        p.set(ctrl + 0xc, offsets);
        if p.snapshot() {
            p.check("pppClearWait", PPP_CLEAR_WAIT, &[], |m| {
                b::ppp_clear_wait(m);
                None
            });
        }
        for round in 0..1500 {
            for i in (0..0x400).step_by(4) {
                let v = rng.float();
                p.set(obj + i, v);
            }
            for i in (0..0x40).step_by(4) {
                let v = rng.float();
                p.set(key + i, v);
            }
            p.set(offsets, rng.below(0x40) * 4);
            p.set(offsets + 4, rng.below(0x30) * 4);
            let frame = rng.pick(&[0u32, 1, 7, 0x8000_0000]);
            p.set(obj + 0xc, frame);
            let kf = if rng.chance(70) { frame } else { frame ^ 1 };
            p.set(key, kf);
            if !p.snapshot() || round > 0 {
                p.put(b::STOP, u32::from(rng.chance(15)));
                p.put(b::COLOR_CON_SHIFT, rng.below(0x10) * 4);
                p.put(b::R0, [rng.next(), rng.next(), rng.next(), rng.next()]);
            }
            let (name, sym, f) = rng.pick(&cons);
            p.check(&format!("{name} round {round}"), sym, &[obj, ctrl], |m| {
                f(m, obj, ctrl);
                None
            });
            let (name, sym, f) = rng.pick(&steps);
            p.check(
                &format!("{name} round {round}"),
                sym,
                &[obj, key, ctrl],
                |m| {
                    f(m, obj, key, ctrl);
                    None
                },
            );
        }
        p.check("pppKeThSftCon", PPP_KE_TH_SFT_CON, &[obj, ctrl], |m| {
            b::ppp_ke_th_sft_con(m, obj, ctrl);
            None
        });
        p.check("pppClearWait", PPP_CLEAR_WAIT, &[], |m| {
            b::ppp_clear_wait(m);
            None
        });
    }
}

/// The NaN rules of [`x87`] against the oracle's x87 (QEMU's), through
/// `pppKeLnsLpSft`'s `fld key; fadd m32`: every pair of special operands
/// but those with a signalling NaN, which the hardware quiets on `fld m32`
/// (Intel SDM, `FLD`, #IA masked) and the oracle keeps signalling, so that
/// the results differ; those are reported, not asserted.
#[test]
fn x87_nans_match_the_oracle() {
    for mut p in Pair::all().into_iter().take(1) {
        let obj = p.alloc(0x200);
        let key = p.alloc(0x10);
        let ctrl = p.alloc(0x10);
        let offsets = p.alloc(8);
        p.set(ctrl + 0xc, offsets);
        p.set(offsets, 0u32);
        p.set(offsets + 4, 0u32);
        p.put(ppp::bridge::STOP, 0);
        let mut differ = Vec::new();
        for st in SPECIALS {
            for mem in SPECIALS {
                p.set(obj + 0xc, 0u32);
                p.set(key, 0u32);
                p.set(key + 4, st);
                p.set(obj + 0xa0, mem);
                p.a.call(p.sym(PPP_KE_LNS_LP_SFT), Conv::Cdecl, &[obj, key, ctrl])
                    .unwrap();
                let got: u32 = p.get(obj + 0xa0);
                let want = x87::add(f32::from_bits(st), f32::from_bits(mem)).to_bits();
                if x87::is_snan(mem) || x87::is_snan(st) {
                    if got != want {
                        differ.push((st, mem, got, want));
                    }
                } else {
                    assert_eq!(got, want, "{st:#x} + {mem:#x}");
                }
            }
        }
        eprintln!(
            "signalling NaN operands where the oracle and the hardware rule differ \
             (st, m32, oracle, hardware): {differ:x?}"
        );
    }
}

type Rot = fn(&mut Oracle, u32, i32);

#[test]
fn rotation_matrices_match_the_original() {
    use ppp::bridge as b;
    let fns: [(&str, u32, Rot); 3] = [
        (
            "pppGetRotMatrixX",
            PPP_GET_ROT_MATRIX_X,
            b::ppp_get_rot_matrix_x,
        ),
        (
            "pppGetRotMatrixY",
            PPP_GET_ROT_MATRIX_Y,
            b::ppp_get_rot_matrix_y,
        ),
        (
            "pppGetRotMatrixZ",
            PPP_GET_ROT_MATRIX_Z,
            b::ppp_get_rot_matrix_z,
        ),
    ];
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0bad_cafe);
        let out = p.alloc(0x40);
        for i in 0..2000u32 {
            let angle = match i {
                0 => 0,
                1 => -1,
                2 => i32::MAX,
                3 => i32::MIN,
                4 => 0x4000,
                _ => rng.next().cast_signed(),
            };
            if i > 10 && i % 10 == 0 {
                p.put(b::R0, [rng.next(), rng.next(), rng.next(), rng.next()]);
            }
            for (name, sym, f) in fns {
                p.check(
                    &format!("{name}({angle:#x})"),
                    sym,
                    &[out, angle.cast_unsigned()],
                    |m| {
                        f(m, out, angle);
                        None
                    },
                );
            }
        }
        // a quiet NaN in the table is copied as it is
        let at = p.sym(b::SIN.sym);
        p.set(at, 0xffc1_2345u32);
        for (name, sym, f) in fns {
            p.check(&format!("{name} on a NaN"), sym, &[out, 0], |m| {
                f(m, out, 0);
                None
            });
        }
    }
}

// ---- heaps ------------------------------------------------------------

const PPP_CREATE_HEAP: u32 = 0x0072_c480;
const PPP_ALLOC: u32 = 0x0072_c3d0;
const PPP_FREE: u32 = 0x0072_c510;
const PPP_FP_GET_HEAP_SIZE: u32 = 0x0072_9290;

/// Random allocations and frees on `heap`, from the blocks in `live`.
fn churn(p: &mut Pair, rng: &mut Xorshift, heap_at: u32, live: &mut Vec<u32>, n: usize) {
    use heap::bridge as h;
    for i in 0..n {
        if live.is_empty() || rng.chance(60) {
            let size = match rng.below(6) {
                0 => rng.pick(&[0, 1, 15, 16, 17]),
                1 => rng.below(0x4000),
                _ => rng.below(0x400),
            };
            let r = p.check(
                &format!("pppAlloc({size:#x}) #{i}"),
                PPP_ALLOC,
                &[heap_at, size],
                |m| Some(h::ppp_alloc(m, heap_at, size)),
            );
            if r != 0 {
                live.push(r);
            }
        } else {
            let k = rng.below(u32::try_from(live.len()).unwrap()) as usize;
            let d = live.swap_remove(k);
            p.check(
                &format!("pppFree({d:#x}) #{i}"),
                PPP_FREE,
                &[heap_at, d],
                |m| {
                    h::ppp_free(m, heap_at, d);
                    None
                },
            );
        }
    }
}

#[test]
fn ppp_heaps_match_the_original() {
    use heap::bridge as h;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x7777_1111);
        p.check("pppFpGetHeapSize", PPP_FP_GET_HEAP_SIZE, &[], |m| {
            Some(h::ppp_fp_get_heap_size(m))
        });
        if p.snapshot() {
            // the field effects' heaps as the game left them
            for g in [h::FP_HEAP, h::FP_HEAP2] {
                let at = p.sym(g.sym);
                let [_, first, _, _] = p.get::<[u32; 4]>(at);
                eprintln!("snapshot heap {:#x}: first {first:#x}", g.sym);
                if first != 0 {
                    let mut live = Vec::new();
                    churn(&mut p, &mut rng, at, &mut live, 100);
                }
            }
        }
        for round in 0..30 {
            let size = 0x200 + rng.below(0x8000);
            let memory = p.alloc(size as usize + 0x20);
            let heap_at = p.alloc(0x10);
            p.check(
                &format!("pppCreateHeap round {round}"),
                PPP_CREATE_HEAP,
                &[heap_at, memory, size],
                |m| {
                    h::ppp_create_heap(m, heap_at, memory, size);
                    None
                },
            );
            let mut live = Vec::new();
            churn(&mut p, &mut rng, heap_at, &mut live, 300);
        }
        for _ in 0..50 {
            let v = [rng.next(), rng.next(), rng.next(), rng.next()];
            let w = [
                rng.next(),
                if rng.chance(30) { 0 } else { rng.next() },
                rng.next(),
                0,
            ];
            p.put(h::FP_HEAP, v);
            p.put(h::FP_HEAP2, w);
            p.check("pppFpGetHeapSize", PPP_FP_GET_HEAP_SIZE, &[], |m| {
                Some(h::ppp_fp_get_heap_size(m))
            });
        }
    }
}

// ---- field effects (FP) -----------------------------------------------

const FP_RESTART: u32 = 0x0072_9700;
const FP_RESTART_MODE2: u32 = 0x0072_9810;
const FP_END: u32 = 0x0072_9660;
const PPP_FP_ID_DISP_OFF: u32 = 0x0072_9570;
const PPP_FP_ID_DISP_ON: u32 = 0x0072_95b0;
const PPP_FP_GET_MNG_PTR: u32 = 0x0072_92f0;
const PPP_FP_RESET_STICK: u32 = 0x0072_a120;
const PPP_FP_SET_STICK_MAP: u32 = 0x0072_a590;
const PPP_FP_SET_STICK_MAP_DOBJ: u32 = 0x0072_a5e0;
const PPP_FP_STOP_STATUS: u32 = 0x0072_a750;
const PPP_FP_POINT_LIGHT_INIT: u32 = 0x0072_9f70;
const PPP_FP_SAVE_STAT: u32 = 0x0072_a180;

type ById = fn(&mut Oracle, u32);
type ById2 = fn(&mut Oracle, u32, u32);

fn fp_calls(p: &mut Pair, rng: &mut Xorshift, ids: &[u32], outs: u32, round: u32) {
    use fp::bridge as f;
    let by_id: [(&str, u32, ById); 10] = [
        ("pppFpGroupDispOff", 0x0072_9400, f::ppp_fp_group_disp_off),
        ("pppFpGroupDispOn", 0x0072_9440, f::ppp_fp_group_disp_on),
        ("Fp_Restart", FP_RESTART, f::fp_restart),
        ("Fp_RestartGroup", 0x0072_9530, f::fp_restart_group),
        ("Fp_EndGroup", 0x0072_94f0, f::fp_end_group),
        ("Fp_RestartMode2", FP_RESTART_MODE2, f::fp_restart_mode2),
        ("Fp_End", FP_END, f::fp_end),
        ("pppFpIDdispOff", PPP_FP_ID_DISP_OFF, f::ppp_fp_id_disp_off),
        ("pppFpIDdispOn", PPP_FP_ID_DISP_ON, f::ppp_fp_id_disp_on),
        ("pppFpResetStick", PPP_FP_RESET_STICK, f::ppp_fp_reset_stick),
    ];
    let sticks: [(&str, u32, ById2); 2] = [
        (
            "pppFpSetStickMap",
            PPP_FP_SET_STICK_MAP,
            f::ppp_fp_set_stick_map,
        ),
        (
            "pppFpSetStickMapDObj",
            PPP_FP_SET_STICK_MAP_DOBJ,
            f::ppp_fp_set_stick_map_dobj,
        ),
    ];
    for _ in 0..12 {
        let id = rng.pick(ids);
        let (name, sym, g) = rng.pick(&by_id);
        p.check(&format!("{name}({id:#x}) round {round}"), sym, &[id], |m| {
            g(m, id);
            None
        });
        let (name, sym, g) = rng.pick(&sticks);
        let target = rng.next();
        p.check(
            &format!("{name}({id:#x}) round {round}"),
            sym,
            &[id, target],
            |m| {
                g(m, id, target);
                None
            },
        );
        let o1 = if rng.chance(80) { outs } else { 0 };
        let o2 = if rng.chance(80) { outs + 4 } else { 0 };
        p.check(
            &format!("pppFpGetMngPtr({id:#x}) round {round}"),
            PPP_FP_GET_MNG_PTR,
            &[id, o1, o2],
            |m| {
                f::ppp_fp_get_mng_ptr(m, id, o1, o2);
                None
            },
        );
        let out = if rng.chance(90) { outs + 8 } else { 0 };
        p.check(
            &format!("pppFpSaveStat round {round}"),
            PPP_FP_SAVE_STAT,
            &[out],
            |m| {
                f::ppp_fp_save_stat(m, out);
                None
            },
        );
    }
}

#[test]
fn field_effects_match_the_original() {
    use fp::bridge as f;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00f0_0f00);
        let outs = p.alloc(0x100);
        if p.snapshot() {
            // the field's own table, if it has one
            let t: u32 = p.get(p.sym(f::DATA.sym));
            let mut ids = vec![0, 0x1_0000, 0xffff_ffff];
            if t != 0 {
                let n: u16 = p.get(t + 4);
                for i in 0..u32::from(n) {
                    ids.push(u32::from(p.get::<u16>(t + 0x64 + 0x50 * i)));
                }
            }
            eprintln!("snapshot FP table {t:#x}: ids {ids:x?}");
            fp_calls(&mut p, &mut rng, &ids, outs, 0);
        }
        for round in 1..200 {
            let n = rng.pick(&[0u32, 1, 3, 4, 5, 9, 40]);
            let table = p.alloc((0x64 + 0x50 * n + 0x10) as usize);
            p.set(table + 4, u16::try_from(n).unwrap());
            let pool = [0u16, 1, 2, 3, 0x8001];
            for i in 0..n {
                let id = rng.pick(&pool);
                p.set(table + 0x64 + 0x50 * i, id);
                // the group (Fp_EndGroup, Fp_RestartGroup)
                let group = rng.pick(&pool);
                p.set(table + 0x66 + 0x50 * i, group);
                let run = p.sym(f::RUN.sym) + 0x80 * i;
                p.fill(run, 0x80, &mut rng);
                if rng.chance(30) {
                    p.set(run, 0xffff_f000u32);
                }
                let status = rng.pick(&[0u8, 0, 1, 2]);
                p.set(run + 0x1d, status);
                let disp = p.sym(f::DISP.sym) + 0x4c * i;
                p.fill(disp, 0x4c, &mut rng);
            }
            let data = if rng.chance(95) { table } else { 0 };
            p.put(f::DATA, data);
            let ids = [0, 1, 2, 3, 0x8001, 0xffff_8001, 0x1_0001, 7];
            fp_calls(&mut p, &mut rng, &ids, outs, round);
        }
        for _ in 0..20 {
            let v = rng.next();
            p.check("pppFpStopStatus", PPP_FP_STOP_STATUS, &[v], |m| {
                f::ppp_fp_stop_status(m, v);
                None
            });
            let (x, y) = (rng.next(), rng.next());
            p.put(f::POINT_LIGHT_A, x);
            p.put(f::POINT_LIGHT_B, y);
            p.check("pppFpPointLightInit", PPP_FP_POINT_LIGHT_INIT, &[], |m| {
                f::ppp_fp_point_light_init(m);
                None
            });
        }
    }
}

// ---- op layer ---------------------------------------------------------

const OP_SPR_ALLOC: u32 = 0x007e_6a00;
const OP_SPR_FREE: u32 = 0x007e_6a60;
const OP_M_COPY: u32 = 0x007e_8010;
const OP_UNIT_M_SCALE: u32 = 0x007e_a700;
const OP_FUNC_OT_SET: u32 = 0x007e_b500;
const OP_FUNC_OT_FREE: u32 = 0x007e_b4a0;
const OP_WOT_TRIGER: u32 = 0x007e_c7d0;
const OP_PRAY_EFFECT_CHECK: u32 = 0x007f_ce10;
const OP_PAD_SET: u32 = 0x007e_6610;
const OP_MDC_MEM_TRANSFER_CUE_FREE: u32 = 0x0080_3730;
const OP_MDC_MEM_TRANSFER_CUE_FREE_ALL: u32 = 0x0080_3760;
const OP_OEF2_TEX_CLUT_OVL_CLEAR: u32 = 0x007e_4e40;
const OP_OEF2_TEX_CLUT_CUE_CHECK: u32 = 0x007e_46d0;
const OP_OEF2_CLUT_CHANGE: u32 = 0x007e_45c0;
const OP_OEF2_OBJ_A_GET: u32 = 0x007e_3b40;
const OP_OEF2_STREAM_CUE: u32 = 0x0080_0390;

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one case per function, in order
fn op_services_match_the_original() {
    use op::bridge as o;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x5eed_0001);
        let buf = p.alloc(0x400);
        let buf2 = p.alloc(0x400);
        for round in 0..300 {
            let state = !p.snapshot() || round > 0;
            // scratch stack
            if state {
                let top = rng.pick(&[0i8, 1, 2, 5, -1, -2, 126, 127, -128]);
                p.put(o::SPR_TOP, top);
                let base = p.sym(o::SPR_STACK.sym);
                let at = base.wrapping_add_signed(4 * i32::from(top));
                let v = rng.next();
                p.set(at, v);
            }
            let n = rng.next();
            p.check("op_spr_alloc", OP_SPR_ALLOC, &[n], |m| {
                Some(o::op_spr_alloc(m, n))
            });
            p.check("op_spr_free", OP_SPR_FREE, &[], |m| {
                o::op_spr_free(m);
                None
            });
            // matrices
            p.fill(buf, 0x80, &mut rng);
            let (dst, src) = rng.pick(&[
                (buf, buf + 0x40),
                (buf + 0x40, buf),
                (buf + 4, buf),
                (buf, buf + 8),
            ]);
            p.check("op_m_copy", OP_M_COPY, &[dst, src], |m| {
                o::op_m_copy(m, dst, src);
                None
            });
            let s = rng.float();
            p.check(
                &format!("op_unit_m_scale({s:#x})"),
                OP_UNIT_M_SCALE,
                &[buf2, s],
                |m| {
                    o::op_unit_m_scale(m, buf2, f32::from_bits(s));
                    None
                },
            );
            // function ordering table
            if state {
                let order = p.sym(o::FUNC_OT_ORDER.sym);
                for i in 0..0x80 {
                    let v = if rng.chance(40) {
                        -1i8
                    } else {
                        i8::try_from(rng.below(32)).unwrap()
                    };
                    p.set(order + i, v);
                }
                let slots = p.sym(o::FUNC_OT.sym);
                for i in 0..0x80 {
                    let f = if rng.chance(50) { 0 } else { rng.next() };
                    p.set(slots + 16 * i, f);
                }
            }
            let list = rng.pick(&[0u32, 0x20, 0x40, 0x60]);
            let args = [rng.next(), rng.next(), rng.next(), rng.next(), list];
            p.check(
                &format!("op_func_ot_set list {list:#x}"),
                OP_FUNC_OT_SET,
                &args,
                |m| {
                    Some(
                        o::op_func_ot_set(m, args[0], args[1], args[2], args[3], list)
                            .cast_unsigned(),
                    )
                },
            );
            let r = rng.below(32);
            let slot = rng.pick(&[0u32, 1, 5, 31, 0x7f, 0x80, 0xffff_ffff, r]);
            p.check(
                &format!("op_func_ot_free({slot:#x}, {list:#x})"),
                OP_FUNC_OT_FREE,
                &[slot, list],
                |m| {
                    o::op_func_ot_free(m, slot, list);
                    None
                },
            );
            // flags
            let on = rng.pick(&[0u32, 1, 0x100, 0x8000_0000]);
            p.check("op_wot_triger", OP_WOT_TRIGER, &[on], |m| {
                o::op_wot_triger(m, on);
                None
            });
            if state {
                p.put(o::PRAY_EFFECT, rng.pick(&[0u8, 1, 0x80]));
            }
            p.check("op_pray_effect_check", OP_PRAY_EFFECT_CHECK, &[], |m| {
                Some(o::op_pray_effect_check(m).cast_unsigned())
            });
            // pads
            if state {
                p.put(o::DEBUG_MODE, u8::from(rng.chance(90)));
                p.put(o::PAD_INPUT, rng.pick(&[0u8, 1, 0x80]));
                let pads = p.sym(o::PADS.sym);
                for i in 0..4 {
                    let at = pads + 0x20 * i;
                    let now = rng.below(4);
                    p.set(at, now);
                    p.set(at + 0x10, rng.pick(&[0u32, 1, 2, 12, 0xffff_ffff]));
                }
            }
            let pad = rng.below(4);
            let r = rng.next();
            let input = rng.pick(&[0u32, 1, 2, 3, 0x1_0001, r]);
            for _ in 0..3 {
                p.check(
                    &format!("op_pad_set({pad}, {input:#x})"),
                    OP_PAD_SET,
                    &[pad, 0, input],
                    |m| {
                        o::op_pad_set(m, pad, input);
                        None
                    },
                );
            }
            // mdc transfer cues
            if state {
                let cues = p.sym(o::MDC_TRANSFER_CUES.sym);
                for i in 0..8 {
                    let k = rng.pick(&[0u32, 1, 2, 3]);
                    p.set(cues + 16 * i, k);
                }
            }
            let key = rng.below(4);
            p.check(
                "op_mdc_mem_transfer_cue_free",
                OP_MDC_MEM_TRANSFER_CUE_FREE,
                &[key],
                |m| {
                    o::op_mdc_mem_transfer_cue_free(m, key);
                    None
                },
            );
            if round % 10 == 0 {
                p.check(
                    "op_mdc_mem_transfer_cue_free_all",
                    OP_MDC_MEM_TRANSFER_CUE_FREE_ALL,
                    &[],
                    |m| {
                        o::op_mdc_mem_transfer_cue_free_all(m);
                        None
                    },
                );
            }
            // oef2 CLUT lists
            if state {
                let ovl = p.sym(o::OEF2_CLUT_OVL.sym);
                for i in 0..64 {
                    let v = if rng.chance(40) {
                        0
                    } else {
                        1 << rng.below(32) | rng.next() & rng.next()
                    };
                    p.set(ovl + 4 * i, v);
                }
                let cue_ring = p.sym(o::OEF2_CLUT_CUES.sym);
                for i in 0..64 {
                    let v = 1u32 << rng.below(32);
                    p.set(cue_ring + 4 * i, v);
                }
                p.put(
                    o::OEF2_CLUT_CUE_HEAD,
                    u16::try_from(rng.below(0x1_0000)).unwrap(),
                );
                p.put(o::OEF2_CLUT_CUE_COUNT, rng.pick(&[0u16, 1, 5, 64, 70]));
            }
            let mask = rng.next() & rng.next() & rng.next();
            p.check(
                "op_oef2_tex_clut_ovl_clear",
                OP_OEF2_TEX_CLUT_OVL_CLEAR,
                &[mask],
                |m| Some(o::op_oef2_tex_clut_ovl_clear(m, mask)),
            );
            let mask = 1u32 << rng.below(32);
            p.check(
                "op_oef2_tex_clut_cue_check",
                OP_OEF2_TEX_CLUT_CUE_CHECK,
                &[mask],
                |m| Some(o::op_oef2_tex_clut_cue_check(m, mask).cast_unsigned()),
            );
            let v = rng.next();
            p.check(
                "op_oef2_clut_change",
                OP_OEF2_CLUT_CHANGE,
                &[buf + 0x100, v],
                |m| Some(o::op_oef2_clut_change(m, buf + 0x100, v)),
            );
            // oef2 object tables: base + *(base + 4) + 4 part -> table
            let t = rng.below(0x20) * 4;
            p.set(buf + 4, t);
            p.fill(buf + 8, 0x3f8, &mut rng);
            let part = rng.next();
            p.set(buf + t + (part & 0xff) * 4, buf2);
            let k = rng.below(0x80);
            let w = rng.below(0x200);
            p.set(buf2 + 4 * k, w);
            p.check(
                "op_oef2_obj_a_get",
                OP_OEF2_OBJ_A_GET,
                &[buf, part, k],
                |m| Some(o::op_oef2_obj_a_get(m, buf, part, k)),
            );
            // stream cues
            p.set(buf2, rng.pick(&[2u8, 1, 0]));
            if state {
                let cues = p.sym(o::OEF2_STREAM_CUES.sym);
                for i in 0..2 {
                    let v = if rng.chance(50) { 0 } else { rng.next() };
                    p.set(cues + 4 * i, v);
                }
            }
            let ch = rng.next();
            p.check("op_oef2_stream_cue", OP_OEF2_STREAM_CUE, &[buf2, ch], |m| {
                Some(o::op_oef2_stream_cue(m, buf2, ch).cast_unsigned())
            });
        }
    }
}

// ---- et tables --------------------------------------------------------

const OP_GET_CHR_STATUS_STONE: u32 = 0x007f_cc80;
const OP_GET_CHRHD_BY_CHRNO: u32 = 0x007f_cd10;
const OP_GET_CHRHD_ALL_LIST: u32 = 0x007f_ccc0;
const OP_ET_BATTLE_GENKO_COUNTER_GET: u32 = 0x007f_b240;
const OP_ET_BINDEFF_DRAW_ON: u32 = 0x007f_bab0;
const OP_ET_BINDEFF_DRAW_OFF: u32 = 0x007f_ba40;
const OP_ET_BINDEFF2_DRAW_ON: u32 = 0x007f_b5a0;
const OP_ET_BINDEFF2_WALK_ON: u32 = 0x007f_b830;
const OP_ET_BINDEFF2_WATER_ON: u32 = 0x007f_b8e0;
const OP_ET_BINDEFF2_BREATH_ON: u32 = 0x007f_b4c0;
const OP_ET_BINDEFF_OFF_SIGNAL: u32 = 0x007f_bcd0;
const OP_ET_EFF_OFF_SIGNAL: u32 = 0x007f_c450;
const OP_ET_WEAPON_AFTERIMAGE_FADE: u32 = 0x007f_ca40;

type ByHandle = fn(&mut Oracle, u32) -> u32;
type ByHandle2 = fn(&mut Oracle, u32, u32) -> u32;

/// One of each `et` function on `handles` and effect `ids`.
#[allow(clippy::too_many_lines)] // one call of each
fn et_calls(p: &mut Pair, rng: &mut Xorshift, handles: &[u32], ids: &[u32], out: u32, what: &str) {
    use et::bridge as e;
    let returning: [(&str, u32, ByHandle); 6] = [
        (
            "op_et_bindeff2_draw_off",
            0x007f_b530,
            e::op_et_bindeff2_draw_off,
        ),
        ("op_et_chr_draw_off", 0x007f_c000, e::op_et_chr_draw_off),
        ("op_et_chr_draw_on", 0x007f_c060, e::op_et_chr_draw_on),
        (
            "op_et_bindeff_draw_on",
            OP_ET_BINDEFF_DRAW_ON,
            e::op_et_bindeff_draw_on,
        ),
        (
            "op_et_bindeff_draw_off",
            OP_ET_BINDEFF_DRAW_OFF,
            e::op_et_bindeff_draw_off,
        ),
        (
            "op_et_bindeff2_draw_on",
            OP_ET_BINDEFF2_DRAW_ON,
            e::op_et_bindeff2_draw_on,
        ),
    ];
    let void: [(&str, u32, ById); 7] = [
        (
            "op_et_bindeff2_breath_off",
            0x007f_b470,
            e::op_et_bindeff2_breath_off,
        ),
        (
            "op_et_bindeff2_walk_off",
            0x007f_b7f0,
            e::op_et_bindeff2_walk_off,
        ),
        (
            "op_et_bindeff2_water_off",
            0x007f_b890,
            e::op_et_bindeff2_water_off,
        ),
        (
            "op_et_bindeff2_walk_on",
            OP_ET_BINDEFF2_WALK_ON,
            e::op_et_bindeff2_walk_on,
        ),
        (
            "op_et_bindeff2_water_on",
            OP_ET_BINDEFF2_WATER_ON,
            e::op_et_bindeff2_water_on,
        ),
        (
            "op_et_bindeff2_breath_on",
            OP_ET_BINDEFF2_BREATH_ON,
            e::op_et_bindeff2_breath_on,
        ),
        (
            "op_et_bindeff_off_signal",
            OP_ET_BINDEFF_OFF_SIGNAL,
            e::op_et_bindeff_off_signal,
        ),
    ];
    let signals: [(&str, u32, ById); 2] = [
        (
            "op_et_eff_off_signal",
            OP_ET_EFF_OFF_SIGNAL,
            e::op_et_eff_off_signal,
        ),
        (
            "op_et_weapon_afterimage_fade",
            OP_ET_WEAPON_AFTERIMAGE_FADE,
            e::op_et_weapon_afterimage_fade,
        ),
    ];
    let switches: [(&str, u32, ByHandle2); 4] = [
        (
            "op_et_bindeff2_draw_set",
            0x007f_b610,
            e::op_et_bindeff2_draw_set,
        ),
        (
            "op_et_bindeff_draw_set",
            0x007f_bb20,
            e::op_et_bindeff_draw_set,
        ),
        ("op_et_chr_draw_set", 0x007f_c0c0, e::op_et_chr_draw_set),
        (
            "op_et_draw_flags_switch",
            0x007f_c200,
            e::op_et_draw_flags_switch,
        ),
    ];
    let masks: [(&str, u32, ById2); 4] = [
        ("op_et_eff_signal_and", 0x007f_c490, e::op_et_eff_signal_and),
        ("op_et_eff_flags_and", 0x007f_c4d0, e::op_et_eff_flags_and),
        ("op_et_eff_flags_or", 0x007f_c510, e::op_et_eff_flags_or),
        ("op_et_eff_signal_or", 0x007f_c550, e::op_et_eff_signal_or),
    ];
    let sends: [(&str, u32, ByHandle2); 2] = [
        (
            "op_et_bindeff_send_common_signal",
            0x007f_bdf0,
            e::op_et_bindeff_send_common_signal,
        ),
        (
            "op_et_bindeff_send_signal",
            0x007f_bef0,
            e::op_et_bindeff_send_signal,
        ),
    ];
    for _ in 0..8 {
        let h = rng.pick(handles);
        let (name, sym, f) = rng.pick(&sends);
        let any = rng.next();
        let bits = rng.pick(&[0x4000u32, 0x8000, 1, any]);
        // a null handle leaves the caller's eax
        p.check(
            &format!("{name}({h:#x}, {bits:#x}) {what}"),
            sym,
            &[h, bits],
            |m| (h != 0).then_some(f(m, h, bits)),
        );
        let (name, sym, f) = rng.pick(&switches);
        let on = rng.pick(&[0u32, 1, 0x100]);
        p.check(
            &format!("{name}({h:#x}, {on}) {what}"),
            sym,
            &[h, on],
            |m| Some(f(m, h, on)),
        );
        let id = rng.pick(ids);
        let (name, sym, f) = rng.pick(&masks);
        let mask = rng.next();
        p.check(
            &format!("{name}({id:#x}, {mask:#x}) {what}"),
            sym,
            &[id, mask],
            |m| {
                f(m, id, mask);
                None
            },
        );
        p.check(
            &format!("op_get_chr_status_stone({h:#x}) {what}"),
            OP_GET_CHR_STATUS_STONE,
            &[h],
            |m| Some(e::op_get_chr_status_stone(m, h)),
        );
        let (name, sym, f) = rng.pick(&returning);
        p.check(&format!("{name}({h:#x}) {what}"), sym, &[h], |m| {
            Some(f(m, h))
        });
        let (name, sym, f) = rng.pick(&void);
        p.check(&format!("{name}({h:#x}) {what}"), sym, &[h], |m| {
            f(m, h);
            None
        });
        let id = rng.pick(ids);
        let (name, sym, f) = rng.pick(&signals);
        p.check(&format!("{name}({id:#x}) {what}"), sym, &[id], |m| {
            f(m, id);
            None
        });
        let no = rng.pick(&[0u32, 1, 2, 3, 7, 0x65, 0xffff_ffff]);
        p.check(
            &format!("op_get_chrhd_by_chrno({no:#x}) {what}"),
            OP_GET_CHRHD_BY_CHRNO,
            &[no],
            |m| Some(e::op_get_chrhd_by_chrno(m, no)),
        );
    }
    p.check(
        &format!("op_get_chrhd_all_list {what}"),
        OP_GET_CHRHD_ALL_LIST,
        &[out],
        |m| Some(e::op_get_chrhd_all_list(m, out)),
    );
}

#[test]
#[allow(clippy::too_many_lines)] // the game's tables, then synthetic ones
fn et_tables_match_the_original() {
    use et::bridge as e;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0e70_0e70);
        let out = p.alloc(4 * 240);
        let chr_at = p.sym(e::CHR.sym);
        let eff_at = p.sym(e::EFF.sym);
        if p.snapshot() {
            // the battle's own tables: handles and effect IDs that are there
            let mut handles = vec![0, 0x1234_5678];
            for i in 0..e::CHRS {
                let h: u32 = p.get(chr_at + 0x20 * i);
                if h != 0 && !handles.contains(&h) {
                    handles.push(h);
                }
            }
            let mut ids = vec![0x7777_7777];
            let mut seen = Vec::new();
            for i in 0..e::EFFS {
                let id: u32 = p.get(eff_at + 0x20 * i);
                let obj: u32 = p.get(eff_at + 0x20 * i + 4);
                if !seen.contains(&id) {
                    seen.push(id);
                    if obj != 0 && id != 0 {
                        ids.push(id);
                    }
                }
            }
            eprintln!(
                "snapshot et: {} handles, {} effect ids ({:x?}...)",
                handles.len() - 2,
                ids.len() - 1,
                &ids[..ids.len().min(8)]
            );
            // bind-off signals find effects by ID: only when every bind ID
            // that is there has an object
            if ids.len() > 1 {
                et_calls(
                    &mut p,
                    &mut rng,
                    &handles,
                    &ids,
                    out,
                    "on the game's tables",
                );
            }
            p.check(
                "op_et_battle_genko_counter_get",
                OP_ET_BATTLE_GENKO_COUNTER_GET,
                &[],
                |m| Some(e::op_et_battle_genko_counter_get(m)),
            );
        }
        // synthetic tables
        let chars = p.alloc(0x100);
        let objs = p.alloc(0x1000);
        let handles: Vec<u32> = (0..8).map(|i| chars + 0x10 * i).collect();
        for round in 0..60 {
            for &h in &handles {
                let kind = rng.pick(&[1u16, 0x65, 2, 3]);
                p.set(h, kind);
            }
            for i in 0..e::CHRS {
                let r = chr_at + 0x20 * i;
                p.fill(r, 0x20, &mut rng);
                let h = if rng.chance(70) {
                    0
                } else {
                    rng.pick(&handles)
                };
                p.set(r, h);
                p.set(r + 4, rng.below(8));
                p.set(r + 0xe, u16::try_from(rng.below(0xf0)).unwrap());
            }
            let flags = p.sym(e::DRAW_FLAGS.sym);
            let flags2 = p.sym(e::DRAW_FLAGS2.sym);
            for i in 0..0xf0 {
                let v = u8::try_from(rng.below(256)).unwrap();
                p.set(flags + i, v);
                let v = if rng.chance(50) {
                    0
                } else {
                    u8::try_from(rng.below(256)).unwrap()
                };
                p.set(flags2 + i, v);
            }
            let mut ids = vec![0x7777_7777];
            for i in 0..e::EFFS {
                let r = eff_at + 0x20 * i;
                p.fill(r, 0x20, &mut rng);
                let id = match rng.below(4) {
                    0 => 0x4000_0000 + rng.pick(&[0x400, 0x500, 0x600, 0x700]) + rng.below(240),
                    1 => rng.below(16),
                    _ => 0xdead_0000 + i,
                };
                ids.push(id);
                p.set(r, id);
                p.set(r + 4, objs + 0x100 * rng.below(16));
            }
            p.put(e::BREATH_FLAGS, rng.next());
            let mut hs = handles.clone();
            hs.extend([0, 0x1234_5678]);
            et_calls(&mut p, &mut rng, &hs, &ids, out, &format!("round {round}"));
            p.put(e::OPUS_BATTLE, objs + 0x10 * rng.below(16));
            p.check(
                "op_et_battle_genko_counter_get",
                OP_ET_BATTLE_GENKO_COUNTER_GET,
                &[],
                |m| Some(e::op_et_battle_genko_counter_get(m)),
            );
        }
    }
}

// ---- opus -------------------------------------------------------------

const OPUS_GET_WEAPON_INDEX: u32 = 0x0080_1000;
const OPUS_GET_MONSTER_INDEX: u32 = 0x0080_0e80;
const OPUS_DRAW_OFF: u32 = 0x0080_0d10;
const OPUS_GET_OSU_BY_DRAW: u32 = 0x0080_0f60;
const OPUS_FULLAUTO_NO_GET: u32 = 0x0080_0df0;
const OPU_WAIT: u32 = 0x0080_c500;
const OPU_ET_NO: u32 = 0x0081_65d0;
const OPP_ANMM_FIN_EN_BONE3X: u32 = 0x0081_0ed0;

/// An `i16` list of parts below `parts`, ended by a negative entry.
fn draw_list(p: &mut Pair, rng: &mut Xorshift, at: u32, parts: u32) {
    let n = rng.below(10);
    for i in 0..n {
        let v = i16::try_from(rng.below(parts)).unwrap();
        p.set(at + 2 * i, v);
    }
    let end = rng.pick(&[-1i16, -1, -5, i16::MIN]);
    p.set(at + 2 * n, end);
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one case per function, in order
fn opus_objects_match_the_original() {
    use opus::bridge as u;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0905_0905);
        let obj = p.alloc(0x300);
        let list = p.alloc(0x40);
        let parts = p.alloc(0x800);
        let cmd = p.alloc(0x10);
        let objs = p.alloc(0x1000);
        if p.snapshot() {
            let b: u32 = p.get(p.sym(et::bridge::OPUS_BATTLE.sym));
            eprintln!("snapshot opus battle object {b:#x}");
            if b != 0 {
                for kind in [0u32, 1, 2, 3, 0xff] {
                    p.check(
                        &format!("opus_get_osu_by_draw(0, _, {kind})"),
                        OPUS_GET_OSU_BY_DRAW,
                        &[0, 0, kind, 0],
                        |m| Some(u::opus_get_osu_by_draw(m, 0, 0, kind, 0)),
                    );
                }
            }
            let weapons = p.sym(u::WEAPONS.sym);
            for i in 0..5 {
                let o: u32 = p.get(weapons + 4 * i);
                p.check(
                    "opus_get_weapon_index on the game's",
                    OPUS_GET_WEAPON_INDEX,
                    &[o],
                    |m| Some(u::opus_get_weapon_index(m, o).cast_unsigned()),
                );
                p.check(
                    "opus_get_monster_index on the game's",
                    OPUS_GET_MONSTER_INDEX,
                    &[o],
                    |m| Some(u::opus_get_monster_index(m, o).cast_unsigned()),
                );
            }
        }
        for round in 0..500 {
            let pool = [0u32, 0x100, 0x200, 0x300, 0x400];
            for i in 0..4 {
                let v = rng.pick(&pool);
                p.set(p.sym(u::WEAPONS.sym) + 4 * i, v);
            }
            for i in 0..8 {
                let v = rng.pick(&pool);
                p.set(p.sym(u::MONSTERS.sym) + 4 * i, v);
            }
            let o = rng.pick(&pool);
            p.check("opus_get_weapon_index", OPUS_GET_WEAPON_INDEX, &[o], |m| {
                Some(u::opus_get_weapon_index(m, o).cast_unsigned())
            });
            p.check(
                "opus_get_monster_index",
                OPUS_GET_MONSTER_INDEX,
                &[o],
                |m| Some(u::opus_get_monster_index(m, o).cast_unsigned()),
            );
            // draw lists
            p.fill(obj, 0x300, &mut rng);
            p.set(obj + 0x1c, list);
            p.set(obj + 0x20, parts);
            draw_list(&mut p, &mut rng, list, 8);
            let r = rng.below(8);
            let part = rng.pick(&[0u32, 1, 2, 7, 0x1_0002, 0xffff, r]);
            p.check(
                &format!("opus_draw_off({part:#x}) round {round}"),
                OPUS_DRAW_OFF,
                &[obj, part],
                |m| {
                    u::opus_draw_off(m, obj, part);
                    None
                },
            );
            draw_list(&mut p, &mut rng, list, 8);
            for i in 0..8 {
                let at = parts + 0x100 * i;
                p.set(at + 0x1e, u8::try_from(rng.below(3)).unwrap());
                p.set(at + 0xbb, u8::try_from(rng.below(3)).unwrap());
            }
            let like = parts + 0x100 * rng.below(8);
            let kind = rng.pick(&[0u32, 1, 2, 0x101, 0xffff_ff01]);
            let any = rng.pick(&[0u32, 1, 0x100]);
            let target = if rng.chance(20) {
                p.put(et::bridge::OPUS_BATTLE, obj);
                0
            } else {
                obj
            };
            p.check(
                &format!("opus_get_osu_by_draw round {round}"),
                OPUS_GET_OSU_BY_DRAW,
                &[target, like, kind, any],
                |m| Some(u::opus_get_osu_by_draw(m, target, like, kind, any)),
            );
            // counters
            let v = rng.pick(&[0i16, -1, -16, -17, -18, 0x7fff, i16::MIN, 100]);
            p.set(obj + 2, v);
            p.check(
                &format!("opus_fullauto_no_get({v})"),
                OPUS_FULLAUTO_NO_GET,
                &[obj],
                |m| Some(u::opus_fullauto_no_get(m, obj)),
            );
            // script commands
            let c = rng.next();
            p.check("opu_wait", OPU_WAIT, &[obj, c], |m| {
                Some(u::opu_wait(m, obj, c))
            });
            p.fill(cmd, 0x10, &mut rng);
            let eff = p.sym(et::bridge::EFF.sym);
            let e = rng.below(et::bridge::EFFS);
            p.set(eff + 0x20 * e + 4, objs + 0x80 * rng.below(32));
            p.set(obj + 0x220, parts + 0x100 * rng.below(8));
            let part_at: u32 = p.get(obj + 0x220);
            p.set(part_at + 0x14, u16::try_from(e).unwrap());
            p.check("opu_et_no", OPU_ET_NO, &[obj, cmd], |m| {
                Some(u::opu_et_no(m, obj, cmd))
            });
            p.check(
                "opp_anmm_fin_en_bone3x",
                OPP_ANMM_FIN_EN_BONE3X,
                &[0, 0, cmd, part_at],
                |m| Some(u::opp_anmm_fin_en_bone3x(m, cmd, part_at)),
            );
        }
    }
}

// ---- opd --------------------------------------------------------------

const OPD_NUM_PAL_DEFAULT: u32 = 0x00a5_de50;
const OPD_NUM_PAL_SET: u32 = 0x00a5_de60;
const OPD_NUM_DISPLAY: u32 = 0x00a5_dd90;
const OPD_NUM_XY_SET: u32 = 0x00a5_e2b0;
const OPD_NUM_VDISP: u32 = 0x00a5_dfb0;
const OPD_HEAP_DISPLAY_CLEAR: u32 = 0x00a6_49c0;
const OPD_HEAP_DISPLAY_INIT: u32 = 0x00a6_4a00;
const OPD_HEAP_DISPLAY_SET: u32 = 0x00a6_4b20;
const OPD_HEAP_DISPLAY_SET_PAR: u32 = 0x00a6_4b60;

#[test]
#[allow(clippy::many_single_char_names)]
fn debug_display_matches_the_original() {
    use opd::bridge as d;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0bd0_0bd0);
        let gauges = p.sym(d::HEAP_DISPLAY.sym);
        for round in 0..300 {
            let v = rng.next();
            p.check("opd_num_pal_set", OPD_NUM_PAL_SET, &[v], |m| {
                d::opd_num_pal_set(m, v);
                None
            });
            p.check("opd_num_display", OPD_NUM_DISPLAY, &[v], |m| {
                d::opd_num_display(m, v);
                None
            });
            let (x, y) = (rng.next(), rng.next());
            p.check("opd_num_xy_set", OPD_NUM_XY_SET, &[x, y], |m| {
                d::opd_num_xy_set(m, x, y);
                None
            });
            if round % 3 == 0 {
                p.put(
                    d::NUM_HOME,
                    [u16::try_from(rng.below(0x1_0000)).unwrap(), 7],
                );
                p.check("opd_num_vdisp", OPD_NUM_VDISP, &[], |m| {
                    d::opd_num_vdisp(m);
                    None
                });
                p.check("opd_num_pal_default", OPD_NUM_PAL_DEFAULT, &[], |m| {
                    d::opd_num_pal_default(m);
                    None
                });
            }
            p.fill(gauges, 0x100, &mut rng);
            for i in 0..4 {
                let f = rng.pick(&[0xffffu32, 0, 0x100, 0x8300, 0xf00]);
                p.set(gauges + 0x40 * i, f);
            }
            let flags =
                rng.pick(&[0u32, 1, 0x102, 0x8203, 0xf00, 0x0e01]) | (rng.next() & 0xffff_0000);
            let (a, b) = (rng.next(), if rng.chance(20) { 0 } else { rng.next() });
            p.check(
                "opd_heap_display_set",
                OPD_HEAP_DISPLAY_SET,
                &[a, b, flags],
                |m| {
                    d::opd_heap_display_set(m, a, b, flags);
                    None
                },
            );
            p.check(
                "opd_heap_display_set_par",
                OPD_HEAP_DISPLAY_SET_PAR,
                &[b, flags],
                |m| {
                    d::opd_heap_display_set_par(m, b, flags);
                    None
                },
            );
            let n = rng.next();
            p.check(
                "opd_heap_display_clear",
                OPD_HEAP_DISPLAY_CLEAR,
                &[n],
                |m| {
                    d::opd_heap_display_clear(m, n);
                    None
                },
            );
            if round % 5 == 0 {
                p.check("opd_heap_display_init", OPD_HEAP_DISPLAY_INIT, &[], |m| {
                    d::opd_heap_display_init(m);
                    None
                });
            }
        }
    }
}

// ---- program table ----------------------------------------------------

/// The set-ups of the program table: `(name, address, bridge)`.
const PROG_CONS: &[(&str, u32, Con)] = &[
    (
        "pppKeMdlDttCon",
        0x0072_e730,
        prog::bridge::ppp_ke_mdl_dtt_con,
    ),
    (
        "pppKeShpTailPhtCon2",
        0x0075_4260,
        prog::bridge::ppp_ke_shp_tail_pht_con2,
    ),
    (
        "pppDrawShapeCon",
        0x0074_0eb0,
        prog::bridge::ppp_draw_shape_con,
    ),
    (
        "pppDrawShapeXCon",
        0x0074_1450,
        prog::bridge::ppp_draw_shape_x_con,
    ),
    (
        "pppNeiDrawShapePointLightCon",
        0x0074_b9a0,
        prog::bridge::ppp_nei_draw_shape_point_light_con,
    ),
    (
        "pppVertexApCon",
        0x0075_7b30,
        prog::bridge::ppp_vertex_ap_con,
    ),
    (
        "pppVertexApDisPosCon",
        0x0075_7e90,
        prog::bridge::ppp_vertex_ap_dis_pos_con,
    ),
    (
        "pppVertexApLcCon",
        0x0075_8250,
        prog::bridge::ppp_vertex_ap_lc_con,
    ),
    (
        "pppVertexApAtCon",
        0x0075_84b0,
        prog::bridge::ppp_vertex_ap_at_con,
    ),
    ("pppPointApCon", 0x0075_7470, prog::bridge::ppp_point_ap_con),
    (
        "pppPointRApCon",
        0x0075_7750,
        prog::bridge::ppp_point_r_ap_con,
    ),
    (
        "pppKeHitBornCon",
        0x0075_9990,
        prog::bridge::ppp_ke_hit_born_con,
    ),
    (
        "pppKeThHitBornCon",
        0x0075_9f10,
        prog::bridge::ppp_ke_th_hit_born_con,
    ),
    (
        "pppKeBornRndCon",
        0x0075_8790,
        prog::bridge::ppp_ke_born_rnd_con,
    ),
    (
        "pppKeBornRnd2Con",
        0x0075_8820,
        prog::bridge::ppp_ke_born_rnd2_con,
    ),
    (
        "pppKeBornRnd3Con",
        0x0075_89d0,
        prog::bridge::ppp_ke_born_rnd3_con,
    ),
    (
        "pppKeBornRnd4Con",
        0x0075_8dd0,
        prog::bridge::ppp_ke_born_rnd4_con,
    ),
    (
        "pppDrawMdlRevCon",
        0x0074_03f0,
        prog::bridge::ppp_draw_mdl_rev_con,
    ),
    (
        "pppKeLnsLpSftCon",
        0x0075_9fb0,
        prog::bridge::ppp_ke_lns_lp_sft_con,
    ),
    (
        "pppKeLnsLpCon2",
        0x0075_af60,
        prog::bridge::ppp_ke_lns_lp_con2,
    ),
    (
        "pppKeLnsLpTCon2",
        0x0075_b110,
        prog::bridge::ppp_ke_lns_lp_t_con2,
    ),
    (
        "pppKeBornRnd5Con",
        0x0075_8f90,
        prog::bridge::ppp_ke_born_rnd5_con,
    ),
    (
        "pppKeBornRnd6Con",
        0x0075_9270,
        prog::bridge::ppp_ke_born_rnd6_con,
    ),
    (
        "pppKeHitChkPxBCon",
        0x0075_9bc0,
        prog::bridge::ppp_ke_hit_chk_px_b_con,
    ),
    (
        "pppKeShpTailCon",
        0x0074_c540,
        prog::bridge::ppp_ke_shp_tail_con,
    ),
    (
        "pppKeShpTailXCon",
        0x0074_d540,
        prog::bridge::ppp_ke_shp_tail_x_con,
    ),
    (
        "pppKeShpTail2Con",
        0x0074_e540,
        prog::bridge::ppp_ke_shp_tail2_con,
    ),
    (
        "pppKeShpTail2XCon",
        0x0074_f580,
        prog::bridge::ppp_ke_shp_tail2_x_con,
    ),
    (
        "pppKeShpTailLcCon",
        0x0075_4fc0,
        prog::bridge::ppp_ke_shp_tail_lc_con,
    ),
    (
        "pppDrawShapeRevCon",
        0x0074_1570,
        prog::bridge::ppp_draw_shape_rev_con,
    ),
    ("pppVtMimeCon2", 0x0072_edb0, prog::bridge::ppp_vt_mime_con2),
    (
        "pppNeiPointLightCon",
        0x0075_d7c0,
        prog::bridge::ppp_nei_point_light_con,
    ),
    (
        "pppKeTkFadeCon",
        0x0072_ee20,
        prog::bridge::ppp_ke_tk_fade_con,
    ),
    (
        "pppKeMdlTfdCon",
        0x0074_97e0,
        prog::bridge::ppp_ke_mdl_tfd_con,
    ),
    (
        "pppKeMdlTfd2Con",
        0x0074_9b10,
        prog::bridge::ppp_ke_mdl_tfd2_con,
    ),
    (
        "pppKeMdlTfd3Con",
        0x0074_9e60,
        prog::bridge::ppp_ke_mdl_tfd3_con,
    ),
    (
        "pppMoveLoopCon",
        0x0075_c150,
        prog::bridge::ppp_move_loop_con,
    ),
    (
        "pppMoveLoopCon2",
        0x0075_c180,
        prog::bridge::ppp_move_loop_con2,
    ),
    (
        "pppSclMoveLoopCon",
        0x0075_c330,
        prog::bridge::ppp_scl_move_loop_con,
    ),
    (
        "pppSclMoveLoopCon2",
        0x0075_c360,
        prog::bridge::ppp_scl_move_loop_con2,
    ),
    (
        "pppAngMoveLoopCon",
        0x0075_c210,
        prog::bridge::ppp_ang_move_loop_con,
    ),
    (
        "pppAngMoveLoopCon2",
        0x0075_c250,
        prog::bridge::ppp_ang_move_loop_con2,
    ),
    (
        "pppKeOfsPtCon",
        0x0075_cdf0,
        prog::bridge::ppp_ke_ofs_pt_con,
    ),
    (
        "pppDrawMdlSeaCon2",
        0x0073_ab40,
        prog::bridge::ppp_draw_mdl_sea_con2,
    ),
    (
        "pppDrawMdlInfCon2",
        0x0073_b800,
        prog::bridge::ppp_draw_mdl_inf_con2,
    ),
    (
        "pppDrawMdlPSimCon2",
        0x0073_cfe0,
        prog::bridge::ppp_draw_mdl_p_sim_con2,
    ),
    (
        "pppDrawMdlLoopCon2",
        0x0073_e270,
        prog::bridge::ppp_draw_mdl_loop_con2,
    ),
    (
        "pppDrawMdlLoopZCon2",
        0x0073_ea00,
        prog::bridge::ppp_draw_mdl_loop_z_con2,
    ),
    (
        "pppDrawMdlLoopDisPosCon2",
        0x0073_f1f0,
        prog::bridge::ppp_draw_mdl_loop_dis_pos_con2,
    ),
    (
        "pppDrawMdlCameraLoopCon2",
        0x0073_fa10,
        prog::bridge::ppp_draw_mdl_camera_loop_con2,
    ),
    (
        "pppDrawMdlCameraCon2",
        0x0073_d8f0,
        prog::bridge::ppp_draw_mdl_camera_con2,
    ),
    (
        "pppKeShpDttCon",
        0x0075_5010,
        prog::bridge::ppp_ke_shp_dtt_con,
    ),
    (
        "pppEiWindFunCon",
        0x0075_d590,
        prog::bridge::ppp_ei_wind_fun_con,
    ),
    (
        "pppDrawMdlTsCon",
        0x0073_84f0,
        prog::bridge::ppp_draw_mdl_ts_con,
    ),
    (
        "pppDrawMdlTs2Con",
        0x0073_9480,
        prog::bridge::ppp_draw_mdl_ts2_con,
    ),
    (
        "pppDrawMdlTs3Con",
        0x0073_aac0,
        prog::bridge::ppp_draw_mdl_ts3_con,
    ),
    (
        "pppNeiDrawMdlTsPointLightCon",
        0x0074_b500,
        prog::bridge::ppp_nei_draw_mdl_ts_point_light_con,
    ),
    ("pppPointCon", 0x0075_c4c0, prog::bridge::ppp_point_con),
    ("pppScaleCon", 0x0075_d050, prog::bridge::ppp_scale_con),
    ("pppAngleCon", 0x0075_ce80, prog::bridge::ppp_angle_con),
    ("pppAcceleCon", 0x0075_b810, prog::bridge::ppp_accele_con),
    (
        "pppSclAcceleCon",
        0x0075_b9d0,
        prog::bridge::ppp_scl_accele_con,
    ),
    ("pppMoveCon", 0x0075_beb0, prog::bridge::ppp_move_con),
    ("pppSclMoveCon", 0x0075_c070, prog::bridge::ppp_scl_move_con),
    (
        "pppAngAcceleCon",
        0x0075_b8c0,
        prog::bridge::ppp_ang_accele_con,
    ),
    ("pppAngMoveCon", 0x0075_bf60, prog::bridge::ppp_ang_move_con),
    (
        "pppKeBornPtCmplCon",
        0x0075_9620,
        prog::bridge::ppp_ke_born_pt_cmpl_con,
    ),
    (
        "pppPointLoopCon",
        0x0075_c5a0,
        prog::bridge::ppp_point_loop_con,
    ),
    (
        "pppAngleLoopCon",
        0x0075_cf40,
        prog::bridge::ppp_angle_loop_con,
    ),
    (
        "pppScaleLoopCon",
        0x0075_d130,
        prog::bridge::ppp_scale_loop_con,
    ),
    (
        "pppDrawMdlCameraCon",
        0x0073_d8c0,
        prog::bridge::ppp_draw_mdl_camera_con,
    ),
    (
        "pppDrawMdlLoopCon",
        0x0073_e240,
        prog::bridge::ppp_draw_mdl_loop_con,
    ),
    (
        "pppDrawMdlLoopZCon",
        0x0073_e9d0,
        prog::bridge::ppp_draw_mdl_loop_z_con,
    ),
    (
        "pppDrawMdlLoopDisPosCon",
        0x0073_f1c0,
        prog::bridge::ppp_draw_mdl_loop_dis_pos_con,
    ),
    (
        "pppDrawMdlCameraLoopCon",
        0x0073_f9e0,
        prog::bridge::ppp_draw_mdl_camera_loop_con,
    ),
    (
        "pppDrawSpriteCon",
        0x0075_66a0,
        prog::bridge::ppp_draw_sprite_con,
    ),
    (
        "pppDrawFilterCon",
        0x0075_7210,
        prog::bridge::ppp_draw_filter_con,
    ),
    (
        "pppKeGrvTgtCon",
        0x0075_9800,
        prog::bridge::ppp_ke_grv_tgt_con,
    ),
    (
        "pppKeThTp2Con",
        0x0073_6e30,
        prog::bridge::ppp_ke_th_tp2_con,
    ),
    ("pppKeThTpCon", 0x0073_6e60, prog::bridge::ppp_ke_th_tp_con),
    (
        "pppDrawMdlPSimCon",
        0x0073_cfb0,
        prog::bridge::ppp_draw_mdl_p_sim_con,
    ),
    (
        "pppDrawRainCon",
        0x0075_6240,
        prog::bridge::ppp_draw_rain_con,
    ),
    (
        "pppDrawMdlSeaCon",
        0x0073_ab00,
        prog::bridge::ppp_draw_mdl_sea_con,
    ),
    (
        "pppKeThRes128Con",
        0x0073_6690,
        prog::bridge::ppp_ke_th_res128_con,
    ),
    (
        "pppKeThRes128x4Con",
        0x0073_66c0,
        prog::bridge::ppp_ke_th_res128x4_con,
    ),
    (
        "pppKeThRes128x8Con",
        0x0073_66f0,
        prog::bridge::ppp_ke_th_res128x8_con,
    ),
    (
        "pppKeThRes16Con",
        0x0073_6720,
        prog::bridge::ppp_ke_th_res16_con,
    ),
    (
        "pppKeThRes16x16Con",
        0x0073_6750,
        prog::bridge::ppp_ke_th_res16x16_con,
    ),
    (
        "pppKeThRes16x24Con",
        0x0073_6780,
        prog::bridge::ppp_ke_th_res16x24_con,
    ),
    (
        "pppKeThRes16x4Con",
        0x0073_67b0,
        prog::bridge::ppp_ke_th_res16x4_con,
    ),
    (
        "pppKeThRes16x64Con",
        0x0073_67e0,
        prog::bridge::ppp_ke_th_res16x64_con,
    ),
    (
        "pppKeThRes16x8Con",
        0x0073_6810,
        prog::bridge::ppp_ke_th_res16x8_con,
    ),
    (
        "pppKeThRes24Con",
        0x0073_6840,
        prog::bridge::ppp_ke_th_res24_con,
    ),
    (
        "pppKeThRes24x16Con",
        0x0073_6870,
        prog::bridge::ppp_ke_th_res24x16_con,
    ),
    (
        "pppKeThRes24x4Con",
        0x0073_68a0,
        prog::bridge::ppp_ke_th_res24x4_con,
    ),
    (
        "pppKeThRes24x8Con",
        0x0073_68d0,
        prog::bridge::ppp_ke_th_res24x8_con,
    ),
    (
        "pppKeThRes255Con",
        0x0073_6900,
        prog::bridge::ppp_ke_th_res255_con,
    ),
    (
        "pppKeThRes255x4Con",
        0x0073_6930,
        prog::bridge::ppp_ke_th_res255x4_con,
    ),
    (
        "pppKeThRes32Con",
        0x0073_6960,
        prog::bridge::ppp_ke_th_res32_con,
    ),
    (
        "pppKeThRes32x16Con",
        0x0073_6990,
        prog::bridge::ppp_ke_th_res32x16_con,
    ),
    (
        "pppKeThRes32x24Con",
        0x0073_69c0,
        prog::bridge::ppp_ke_th_res32x24_con,
    ),
    (
        "pppKeThRes32x32Con",
        0x0073_69f0,
        prog::bridge::ppp_ke_th_res32x32_con,
    ),
    (
        "pppKeThRes32x4Con",
        0x0073_6a20,
        prog::bridge::ppp_ke_th_res32x4_con,
    ),
    (
        "pppKeThRes32x8Con",
        0x0073_6a50,
        prog::bridge::ppp_ke_th_res32x8_con,
    ),
    (
        "pppKeThRes40Con",
        0x0073_6a80,
        prog::bridge::ppp_ke_th_res40_con,
    ),
    (
        "pppKeThRes40x16Con",
        0x0073_6ab0,
        prog::bridge::ppp_ke_th_res40x16_con,
    ),
    (
        "pppKeThRes40x4Con",
        0x0073_6ae0,
        prog::bridge::ppp_ke_th_res40x4_con,
    ),
    (
        "pppKeThRes40x8Con",
        0x0073_6b10,
        prog::bridge::ppp_ke_th_res40x8_con,
    ),
    (
        "pppKeThRes48Con",
        0x0073_6b40,
        prog::bridge::ppp_ke_th_res48_con,
    ),
    (
        "pppKeThRes48x16Con",
        0x0073_6b70,
        prog::bridge::ppp_ke_th_res48x16_con,
    ),
    (
        "pppKeThRes48x4Con",
        0x0073_6ba0,
        prog::bridge::ppp_ke_th_res48x4_con,
    ),
    (
        "pppKeThRes48x8Con",
        0x0073_6bd0,
        prog::bridge::ppp_ke_th_res48x8_con,
    ),
    (
        "pppKeThRes64Con",
        0x0073_6c00,
        prog::bridge::ppp_ke_th_res64_con,
    ),
    (
        "pppKeThRes64x16Con",
        0x0073_6c30,
        prog::bridge::ppp_ke_th_res64x16_con,
    ),
    (
        "pppKeThRes64x4Con",
        0x0073_6c60,
        prog::bridge::ppp_ke_th_res64x4_con,
    ),
    (
        "pppKeThRes64x8Con",
        0x0073_6c90,
        prog::bridge::ppp_ke_th_res64x8_con,
    ),
    (
        "pppKeThRes8Con",
        0x0073_6cc0,
        prog::bridge::ppp_ke_th_res8_con,
    ),
    (
        "pppKeThRes8x128Con",
        0x0073_6cf0,
        prog::bridge::ppp_ke_th_res8x128_con,
    ),
    (
        "pppKeThRes8x4Con",
        0x0073_6d20,
        prog::bridge::ppp_ke_th_res8x4_con,
    ),
    (
        "pppKeThResCon",
        0x0073_6d50,
        prog::bridge::ppp_ke_th_res_con,
    ),
    (
        "pppKeLnsArndCon",
        0x0075_9fd0,
        prog::bridge::ppp_ke_lns_arnd_con,
    ),
    (
        "pppKeLnsCrnCon",
        0x0075_a740,
        prog::bridge::ppp_ke_lns_crn_con,
    ),
    (
        "pppKeLnsClmCon",
        0x0075_a370,
        prog::bridge::ppp_ke_lns_clm_con,
    ),
    ("pppKeThCon", 0x0073_6460, prog::bridge::ppp_ke_th_con),
    (
        "pppKeMdlTfdUvCon",
        0x0074_98f0,
        prog::bridge::ppp_ke_mdl_tfd_uv_con,
    ),
    (
        "pppKeMdlTfdUv2Con",
        0x0074_9c30,
        prog::bridge::ppp_ke_mdl_tfd_uv2_con,
    ),
    (
        "pppKeMdlTfdUv3Con",
        0x0074_9f90,
        prog::bridge::ppp_ke_mdl_tfd_uv3_con,
    ),
    (
        "pppKeShpTail3Con2",
        0x0075_05a0,
        prog::bridge::ppp_ke_shp_tail3_con2,
    ),
    (
        "pppKeShpTail3XCon2",
        0x0075_1f40,
        prog::bridge::ppp_ke_shp_tail3_x_con2,
    ),
    (
        "pppDrawMdlInfCon",
        0x0073_b7b0,
        prog::bridge::ppp_draw_mdl_inf_con,
    ),
    (
        "pppColAcceleCon",
        0x0075_bac0,
        prog::bridge::ppp_col_accele_con,
    ),
    (
        "pppColMoveCon",
        0x0075_c410,
        prog::bridge::ppp_col_accele_con,
    ),
    (
        "pppPointLoopCon2",
        0x0075_c5d0,
        prog::bridge::ppp_point_loop_con2,
    ),
    (
        "pppScaleLoopCon2",
        0x0075_d160,
        prog::bridge::ppp_point_loop_con2,
    ),
    (
        "pppAngleLoopCon2",
        0x0075_cf70,
        prog::bridge::ppp_angle_loop_con2,
    ),
];

/// The per-frame steps.
const PROG_STEPS: &[(&str, u32, Step)] = &[
    ("pppAccele", 0x0075_b740, prog::bridge::ppp_accele),
    ("pppSclAccele", 0x0075_b900, prog::bridge::ppp_accele),
    ("pppMove", 0x0075_bde0, prog::bridge::ppp_accele),
    ("pppSclMove", 0x0075_bfa0, prog::bridge::ppp_accele),
    ("pppAngAccele", 0x0075_b850, prog::bridge::ppp_ang_accele),
    ("pppAngMove", 0x0075_bef0, prog::bridge::ppp_ang_accele),
    ("pppColAccele", 0x0075_ba40, prog::bridge::ppp_col_accele),
    ("pppColMove", 0x0075_c390, prog::bridge::ppp_col_accele),
    ("pppMoveLoop", 0x0075_c0b0, prog::bridge::ppp_move_loop),
    ("pppSclMoveLoop", 0x0075_c290, prog::bridge::ppp_move_loop),
    (
        "pppAngMoveLoop",
        0x0075_c1b0,
        prog::bridge::ppp_ang_move_loop,
    ),
    ("pppPoint", 0x0075_c450, prog::bridge::ppp_point),
    ("pppScale", 0x0075_cfe0, prog::bridge::ppp_point),
    ("pppPointLoop", 0x0075_c500, prog::bridge::ppp_point_loop),
    ("pppScaleLoop", 0x0075_d090, prog::bridge::ppp_point_loop),
    ("pppAngleLoop", 0x0075_cec0, prog::bridge::ppp_angle_loop),
    ("pppDrawSprite", 0x0075_66e0, prog::bridge::ppp_draw_sprite),
    ("pppKeHitBall", 0x0075_9830, prog::bridge::ppp_ke_hit_ball),
    (
        "pppNeiPointLight",
        0x0075_d700,
        prog::bridge::ppp_nei_point_light,
    ),
    ("pppPObjPoint", 0x0075_d280, prog::bridge::ppp_p_obj_point),
    ("pppKeThTp", 0x0073_6d80, prog::bridge::ppp_ke_th_tp),
    ("pppKeGrvTgt", 0x0075_97b0, prog::bridge::ppp_ke_grv_tgt),
    ("pppKeThSft", 0x0073_6e90, prog::bridge::ppp_ke_th_sft),
    ("pppKeShpTail", 0x0074_c490, |m, o, _, c| {
        prog::bridge::ppp_ke_shp_tail(m, o, c);
    }),
    ("pppKeShpTailX", 0x0074_d490, |m, o, _, c| {
        prog::bridge::ppp_ke_shp_tail(m, o, c);
    }),
    ("pppKeShpTail2", 0x0074_e490, |m, o, _, c| {
        prog::bridge::ppp_ke_shp_tail(m, o, c);
    }),
    ("pppKeShpTail2X", 0x0074_f4d0, |m, o, _, c| {
        prog::bridge::ppp_ke_shp_tail(m, o, c);
    }),
    ("pppKeShpTail3", 0x0075_02c0, |m, o, k, c| {
        prog::bridge::ppp_ke_shp_tail3(m, o, k, c, prog::bridge::Tail3::Plain);
    }),
    ("pppKeShpTail3X", 0x0075_1ca0, |m, o, k, c| {
        prog::bridge::ppp_ke_shp_tail3(m, o, k, c, prog::bridge::Tail3::X);
    }),
    ("pppKeShpTailPht", 0x0075_3f10, |m, o, k, c| {
        prog::bridge::ppp_ke_shp_tail3(m, o, k, c, prog::bridge::Tail3::Pht);
    }),
];

#[test]
#[allow(clippy::too_many_lines)] // one setup, the random rounds, then each in order
fn program_table_matches_the_original() {
    use prog::bridge as g;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0c0d_e0f1);
        let obj = p.alloc(0x1_0000);
        let key = p.alloc(0x40);
        let key2 = p.alloc(0x40);
        let ctrl = p.alloc(0x10);
        let offsets = p.alloc(6 * 4);
        p.set(ctrl + 0xc, offsets);
        // a current part with eight objects (pppPObjPoint, pppKeThTp)
        let part = p.alloc(0x40);
        let records = p.alloc(8 * 16);
        p.set(part + 0x3c, records);
        for i in 0..8 {
            let o = p.alloc(0x200);
            p.set(records + 16 * i + 4, o);
        }
        p.put(prog::bridge::CURRENT_PART, part);
        for round in 0..1500 {
            for i in (0..0x400).step_by(4) {
                let v = if rng.chance(70) {
                    rng.float()
                } else {
                    rng.next()
                };
                p.set(obj + i, v);
            }
            // the position the tail programs copy through the FPU: no
            // signalling NaNs (the oracle keeps them)
            for i in (0x40..0x50).step_by(4) {
                let v: u32 = p.get(obj + i);
                if x87::is_snan(v) {
                    p.set(obj + i, v | 0x0040_0000);
                }
            }
            for k in [key, key2] {
                for i in (0..0x40).step_by(4) {
                    let v = if rng.chance(70) {
                        rng.float()
                    } else {
                        rng.next()
                    };
                    p.set(k + i, v);
                }
            }
            for i in 0..6 {
                p.set(offsets + 4 * i, rng.below(0x40) * 4);
            }
            let frame = rng.pick(&[0u32, 1, 7, 0x8000_0000]);
            p.set(obj + 0xc, frame);
            let kf = if rng.chance(70) { frame } else { frame ^ 1 };
            p.set(key, kf);
            // part object targets: -1 or one of the eight, a field in it
            for (n, at) in [(4, 8), (0x10, 0x14)] {
                let o = if rng.chance(20) {
                    u32::MAX
                } else {
                    rng.below(8)
                };
                p.set(key + n, o);
                p.set(key + at, rng.below(0x40) * 4);
            }
            // the remembered key of the Loop programs: none, or a real one
            let off0: u32 = p.get(offsets);
            let remembered = rng.pick(&[0, key, key2]);
            // a set-up that takes the key back off has one
            let held = rng.pick(&[key, key2]);
            p.set(obj + off0 + 0xb0, held);
            if !p.snapshot() || round > 0 {
                p.put(ppp::bridge::STOP, u32::from(rng.chance(15)));
                p.put(ppp::bridge::COLOR_CON_SHIFT, rng.below(0x10) * 4);
                p.put(
                    ppp::bridge::R0,
                    [rng.next(), rng.next(), rng.next(), rng.next()],
                );
                p.put(g::VF00, [rng.next(), rng.next(), rng.next(), rng.next()]);
            }
            let (name, sym, f) = rng.pick(PROG_CONS);
            p.check(&format!("{name} round {round}"), sym, &[obj, ctrl], |m| {
                f(m, obj, ctrl);
                None
            });
            p.set(obj + off0 + 0xb0, remembered);
            let (name, sym, f) = rng.pick(PROG_STEPS);
            if name == "pppPObjPoint" {
                // it reads through its pointer every frame
                p.set(obj + off0 + 0xb0, held);
            }
            p.check(
                &format!("{name} round {round}"),
                sym,
                &[obj, key, ctrl],
                |m| {
                    f(m, obj, key, ctrl);
                    None
                },
            );
        }
        // every one at least once, in order
        for &(name, sym, f) in PROG_CONS {
            let off0: u32 = p.get(offsets);
            p.set(obj + off0 + 0xb0, key2);
            p.check(name, sym, &[obj, ctrl], |m| {
                f(m, obj, ctrl);
                None
            });
        }
        p.put(ppp::bridge::STOP, 0);
        for &(name, sym, f) in PROG_STEPS {
            let off0: u32 = p.get(offsets);
            let remembered = if name == "pppPObjPoint" { key2 } else { 0 };
            p.set(obj + off0 + 0xb0, remembered);
            p.check(name, sym, &[obj, key, ctrl], |m| {
                f(m, obj, key, ctrl);
                None
            });
        }
    }
}

const OP_FUNC_BEFORE_OT_SET: u32 = 0x007e_b370;
const OP_FUNC_AFTER_OT3D_SET: u32 = 0x007e_b320;
const OP_FUNC_BEFORE_WOT_SET: u32 = 0x007e_b3c0;
const OP_FUNC_BEFORE_OT_FREE: u32 = 0x007e_b350;
const OP_FUNC_AFTER_OT3D_FREE: u32 = 0x007e_b300;
const OP_FUNC_BEFORE_WOT_FREE: u32 = 0x007e_b3a0;
const OP_HEAP_DISPLAY_SET_PAR: u32 = 0x007f_f2b0;

type OtSet = fn(&mut Oracle, u32, u32, u32, u32) -> i32;

#[test]
fn op_wrappers_match_the_original() {
    use op::bridge as o;
    let sets: [(&str, u32, OtSet); 3] = [
        (
            "op_func_before_ot_set",
            OP_FUNC_BEFORE_OT_SET,
            o::op_func_before_ot_set,
        ),
        (
            "op_func_after_ot3d_set",
            OP_FUNC_AFTER_OT3D_SET,
            o::op_func_after_ot3d_set,
        ),
        (
            "op_func_before_wot_set",
            OP_FUNC_BEFORE_WOT_SET,
            o::op_func_before_wot_set,
        ),
    ];
    let frees: [(&str, u32, ById); 3] = [
        (
            "op_func_before_ot_free",
            OP_FUNC_BEFORE_OT_FREE,
            o::op_func_before_ot_free,
        ),
        (
            "op_func_after_ot3d_free",
            OP_FUNC_AFTER_OT3D_FREE,
            o::op_func_after_ot3d_free,
        ),
        (
            "op_func_before_wot_free",
            OP_FUNC_BEFORE_WOT_FREE,
            o::op_func_before_wot_free,
        ),
    ];
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0ffe_0ffe);
        for round in 0..300 {
            if !p.snapshot() || round > 0 {
                let order = p.sym(o::FUNC_OT_ORDER.sym);
                for i in 0..0x80 {
                    let v = if rng.chance(40) {
                        -1i8
                    } else {
                        i8::try_from(rng.below(32)).unwrap()
                    };
                    p.set(order + i, v);
                }
                let slots = p.sym(o::FUNC_OT.sym);
                for i in 0..0x80 {
                    let f = if rng.chance(50) { 0 } else { rng.next() };
                    p.set(slots + 16 * i, f);
                }
                p.put(o::DEBUG_MODE, u8::from(rng.chance(70)));
            }
            let args = [rng.next(), rng.next(), rng.next(), rng.next()];
            let (name, sym, f) = rng.pick(&sets);
            p.check(&format!("{name} round {round}"), sym, &args, |m| {
                Some(f(m, args[0], args[1], args[2], args[3]).cast_unsigned())
            });
            let r = rng.below(32);
            let slot = rng.pick(&[0u32, 1, 31, 0x7f, 0x80, r]);
            let (name, sym, f) = rng.pick(&frees);
            p.check(
                &format!("{name}({slot}) round {round}"),
                sym,
                &[slot],
                |m| {
                    f(m, slot);
                    None
                },
            );
            let gauges = p.sym(opd::bridge::HEAP_DISPLAY.sym);
            p.fill(gauges, 0x100, &mut rng);
            let flags = rng.pick(&[0u32, 1, 0x102, 0x8203, 0xf00]);
            let heap_at = if rng.chance(20) { 0 } else { rng.next() };
            p.check(
                "op_heap_display_set_par",
                OP_HEAP_DISPLAY_SET_PAR,
                &[heap_at, flags],
                |m| {
                    o::op_heap_display_set_par(m, heap_at, flags);
                    None
                },
            );
        }
    }
}

// ---- rand programs ----------------------------------------------------

const PPP_VU0_RNEXT: u32 = 0x0072_f420;
/// The import slot of MSVCR110's `srand`.
const SRAND_SLOT: u32 = 0x00b0_c484;
/// Free code space for the `srand` recorder (after the float thunk).
const SRAND_RECORDER: u32 = 0x00b0_bf40;

/// Points the `srand` import at a recorder that appends each seed to `log`
/// (count at `log`, seeds after it), in both oracles alike; returns `log`.
fn record_srand(p: &mut Pair) -> u32 {
    let log = p.alloc(0x1000);
    let (count, seeds) = (log, log + 4);
    let at = p.sym(SRAND_RECORDER);
    let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
    c.extend([0x8b, 0x0d]); // mov ecx, [count]
    c.extend(count.to_le_bytes());
    c.extend([0x89, 0x04, 0x8d]); // mov [seeds + ecx*4], eax
    c.extend(seeds.to_le_bytes());
    c.push(0x41); // inc ecx
    c.extend([0x89, 0x0d]); // mov [count], ecx
    c.extend(count.to_le_bytes());
    c.push(0xc3);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    let slot = p.sym(SRAND_SLOT);
    p.set(slot, at);
    log
}

/// The Rust side of the recorder.
fn srand_log(log: u32) -> impl FnMut(&mut Oracle, u32) {
    move |m, seed| {
        let n: u32 = Mem::read(m, log);
        Mem::write(m, log + 4 + 4 * (n % 0x3ff), seed);
        Mem::write(m, log, n + 1);
    }
}

type RandProg = (&'static str, u32, rand::bridge::Spread, rand::bridge::Kind);

const RAND_PROGS: [RandProg; 24] = [
    (
        "pppRandFloat",
        0x0072_f4b0,
        rand::bridge::Spread::Plain,
        rand::bridge::Kind::Float,
    ),
    (
        "pppRandUpFloat",
        0x0072_f680,
        rand::bridge::Spread::Up,
        rand::bridge::Kind::Float,
    ),
    (
        "pppRandDownFloat",
        0x0072_f850,
        rand::bridge::Spread::Down,
        rand::bridge::Kind::Float,
    ),
    (
        "pppRandChar",
        0x0072_fa20,
        rand::bridge::Spread::Plain,
        rand::bridge::Kind::Char,
    ),
    (
        "pppRandUpChar",
        0x0072_fbf0,
        rand::bridge::Spread::Up,
        rand::bridge::Kind::Char,
    ),
    (
        "pppRandDownChar",
        0x0072_fdc0,
        rand::bridge::Spread::Down,
        rand::bridge::Kind::Char,
    ),
    (
        "pppRandShort",
        0x0072_ffa0,
        rand::bridge::Spread::Plain,
        rand::bridge::Kind::Short,
    ),
    (
        "pppRandUpShort",
        0x0073_0170,
        rand::bridge::Spread::Up,
        rand::bridge::Kind::Short,
    ),
    (
        "pppRandDownShort",
        0x0073_0340,
        rand::bridge::Spread::Down,
        rand::bridge::Kind::Short,
    ),
    (
        "pppRandInt",
        0x0073_0520,
        rand::bridge::Spread::Plain,
        rand::bridge::Kind::Int,
    ),
    (
        "pppRandUpInt",
        0x0073_0700,
        rand::bridge::Spread::Up,
        rand::bridge::Kind::Int,
    ),
    (
        "pppRandDownInt",
        0x0073_08d0,
        rand::bridge::Spread::Down,
        rand::bridge::Kind::Int,
    ),
    (
        "pppRandFV",
        0x0073_0ab0,
        rand::bridge::Spread::Plain,
        rand::bridge::Kind::FloatVec,
    ),
    (
        "pppRandUpFV",
        0x0073_0ce0,
        rand::bridge::Spread::Up,
        rand::bridge::Kind::FloatVec,
    ),
    (
        "pppRandDownFV",
        0x0073_0ef0,
        rand::bridge::Spread::Down,
        rand::bridge::Kind::FloatVec,
    ),
    (
        "pppRandIV",
        0x0073_1100,
        rand::bridge::Spread::Plain,
        rand::bridge::Kind::IntVec,
    ),
    (
        "pppRandUpIV",
        0x0073_1350,
        rand::bridge::Spread::Up,
        rand::bridge::Kind::IntVec,
    ),
    (
        "pppRandDownIV",
        0x0073_1560,
        rand::bridge::Spread::Down,
        rand::bridge::Kind::IntVec,
    ),
    (
        "pppRandCV",
        0x0073_1780,
        rand::bridge::Spread::Plain,
        rand::bridge::Kind::CharVec,
    ),
    (
        "pppRandUpCV",
        0x0073_19f0,
        rand::bridge::Spread::Up,
        rand::bridge::Kind::CharVec,
    ),
    (
        "pppRandDownCV",
        0x0073_1c50,
        rand::bridge::Spread::Down,
        rand::bridge::Kind::CharVec,
    ),
    (
        "pppRandHCV",
        0x0073_1ec0,
        rand::bridge::Spread::Plain,
        rand::bridge::Kind::ShortVec,
    ),
    (
        "pppRandUpHCV",
        0x0073_2130,
        rand::bridge::Spread::Up,
        rand::bridge::Kind::ShortVec,
    ),
    (
        "pppRandDownHCV",
        0x0073_2390,
        rand::bridge::Spread::Down,
        rand::bridge::Kind::ShortVec,
    ),
];

const SRAND_PROGS: [RandProg; 9] = {
    use rand::bridge::{Kind, Spread};
    [
        ("pppSRandFV", 0x0073_25f0, Spread::Plain, Kind::FloatVec),
        ("pppSRandUpFV", 0x0073_28b0, Spread::Up, Kind::FloatVec),
        ("pppSRandDownFV", 0x0073_2b40, Spread::Down, Kind::FloatVec),
        ("pppSRandCV", 0x0073_2dd0, Spread::Plain, Kind::CharVec),
        ("pppSRandUpCV", 0x0073_30e0, Spread::Up, Kind::CharVec),
        ("pppSRandDownCV", 0x0073_33e0, Spread::Down, Kind::CharVec),
        ("pppSRandHCV", 0x0073_36e0, Spread::Plain, Kind::ShortVec),
        ("pppSRandUpHCV", 0x0073_3a00, Spread::Up, Kind::ShortVec),
        ("pppSRandDownHCV", 0x0073_3d10, Spread::Down, Kind::ShortVec),
    ]
};

#[test]
fn rand_programs_match_the_original() {
    use rand::bridge as r;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x5a4d_0001);
        let log = record_srand(&mut p);
        let obj = p.alloc(0x400);
        let key = p.alloc(0x40);
        let ctrl = p.alloc(0x10);
        let offsets = p.alloc(8);
        let out = p.alloc(4);
        p.set(ctrl + 0xc, offsets);
        for round in 0..10000 {
            // the log wraps like the Rust side's
            let n: u32 = p.get(log);
            if n >= 0x3f0 {
                p.set(log, 0u32);
            }
            let state = !p.snapshot() || round > 0;
            if state {
                // r0 - 1 never a signalling NaN (the oracle keeps those)
                let t0 = if rng.chance(50) {
                    u32::MAX
                } else {
                    rng.float()
                };
                p.put(
                    ppp::bridge::R0,
                    [t0.wrapping_add(1), rng.next(), rng.next(), rng.next()],
                );
                let t0_at = p.sym(0x00c8_f588);
                p.set(t0_at + 8, rng.next());
                p.set(t0_at + 12, rng.next());
                p.put(r::R, rng.next());
                p.put(r::SEEDED, u32::from(rng.chance(80)));
                p.put(r::TARGET, rng.float());
                p.put(ppp::bridge::STOP, u32::from(rng.chance(10)));
            }
            for i in (0..0x400).step_by(4) {
                let v = rng.float();
                p.set(obj + i, v);
            }
            for i in (0..0x40).step_by(4) {
                let v = rng.float();
                p.set(key + i, v);
            }
            let off0 = rng.below(0x40) * 4;
            p.set(offsets, off0);
            let frame = if rng.chance(50) {
                0
            } else {
                rng.pick(&[1u32, 7, 0x8000_0000])
            };
            p.set(obj + 0xc, frame);
            let kf = if rng.chance(70) { frame } else { frame ^ 1 };
            p.set(key, kf);
            let target = if rng.chance(30) {
                u32::MAX
            } else {
                rng.below(0x50) * 4
            };
            p.set(key + 4, target);
            for flag in [9, 0xa, 0xc, 0x10, 0x20] {
                p.set(key + flag, u8::from(rng.chance(50)));
            }
            let seeded = rng.chance(40);
            let (name, sym, spread, kind) = if seeded {
                rng.pick(&SRAND_PROGS)
            } else {
                rng.pick(&RAND_PROGS)
            };
            p.check(
                &format!("{name} round {round}"),
                sym,
                &[obj, key, ctrl],
                |m| {
                    if seeded {
                        r::ppp_srand(m, obj, key, ctrl, spread, kind, &mut srand_log(log));
                    } else {
                        r::ppp_rand(m, obj, key, ctrl, spread, kind, &mut srand_log(log));
                    }
                    None
                },
            );
            p.check("pppVu0RNext", PPP_VU0_RNEXT, &[out, p.sym(r::R.sym)], |m| {
                r::ppp_vu0_rnext(m, out);
                None
            });
        }
    }
}

// ---- matrix programs and the set-ups that call rand -------------------

/// The import slot of MSVCR110's `rand`.
const RAND_SLOT: u32 = 0x00b0_c420;
/// Free code space for the fake `rand` (after the `srand` recorder).
const FAKE_RAND: u32 = 0x00b0_bf80;

/// Points the `rand` import at a linear congruential generator whose state
/// is at the returned address, in both oracles alike.
fn fake_rand(p: &mut Pair) -> u32 {
    let state = p.alloc(4);
    let at = p.sym(FAKE_RAND);
    let mut c = vec![0xa1]; // mov eax, [state]
    c.extend(state.to_le_bytes());
    c.extend([0x69, 0xc0]); // imul eax, eax, 1103515245
    c.extend(1_103_515_245u32.to_le_bytes());
    c.push(0x05); // add eax, 12345
    c.extend(12345u32.to_le_bytes());
    c.push(0xa3); // mov [state], eax
    c.extend(state.to_le_bytes());
    c.extend([0xc1, 0xe8, 0x10]); // shr eax, 16
    c.extend([0x25, 0xff, 0x7f, 0, 0]); // and eax, 0x7fff
    c.push(0xc3);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    let slot = p.sym(RAND_SLOT);
    p.set(slot, at);
    state
}

/// The Rust side of [`fake_rand`].
fn rand_lcg(state: u32) -> impl FnMut(&mut Oracle) -> u32 {
    move |m| {
        let s: u32 = Mem::read(m, state);
        let s = s.wrapping_mul(1_103_515_245).wrapping_add(12345);
        Mem::write(m, state, s);
        (s >> 16) & 0x7fff
    }
}

type MatStep = fn(&mut Oracle, u32, u32, u32);

const MAT_STEPS: &[(&str, u32, MatStep)] = {
    use mat::bridge as b;
    use mat::order as o;
    &[
        ("pppMatrixLoop", 0x0072_d1b0, |m, x, _, c| {
            b::ppp_matrix(m, x, c, o::XYZ);
        }),
        ("pppMatrixXYZ", 0x0072_d390, |m, x, _, c| {
            b::ppp_matrix(m, x, c, o::XYZ);
        }),
        ("pppMatrixXZY", 0x0072_d570, |m, x, _, c| {
            b::ppp_matrix(m, x, c, o::XZY);
        }),
        ("pppMatrixYXZ", 0x0072_d750, |m, x, _, c| {
            b::ppp_matrix(m, x, c, o::YXZ);
        }),
        ("pppMatrixYZX", 0x0072_d930, |m, x, _, c| {
            b::ppp_matrix(m, x, c, o::YZX);
        }),
        ("pppMatrixZXY", 0x0072_db10, |m, x, _, c| {
            b::ppp_matrix(m, x, c, o::ZXY);
        }),
        ("pppMatrixZYX", 0x0072_dcf0, |m, x, _, c| {
            b::ppp_matrix(m, x, c, o::ZYX);
        }),
        ("pppMatrixLoc", 0x0073_4130, |m, x, _, c| {
            b::ppp_matrix_loc(m, x, c);
        }),
        ("pppMatrixScl", 0x0073_4170, |m, x, _, c| {
            b::ppp_matrix_scl(m, x, c);
        }),
        ("pppParMatrix", 0x0073_4640, |m, x, k, _| {
            b::ppp_par_matrix(m, x, k);
        }),
        ("pppKeMatSN", 0x0073_4020, |m, x, _, c| {
            b::ppp_ke_mat_sn(m, x, c);
        }),
        ("pppKeMatPht", 0x0073_41e0, |m, x, _, c| {
            b::ppp_ke_mat_pht(m, x, c);
        }),
        ("pppKeOfsMatXYZ", 0x0073_4400, |m, x, _, c| {
            b::ppp_ke_ofs_mat_xyz(m, x, c);
        }),
        ("pppDrawMatrix", 0x0073_4b70, |m, x, _, _| {
            b::ppp_draw_matrix(m, x);
        }),
        ("pppDrawMatrixLoop", 0x0073_4ba0, |m, x, _, _| {
            b::ppp_draw_matrix(m, x);
        }),
        ("pppDrawMatrixFront", 0x0073_4bd0, |m, x, _, _| {
            b::ppp_draw_matrix_front(m, x);
        }),
        ("pppDrawMatrixNoRot", 0x0073_4c50, |m, x, _, _| {
            b::ppp_draw_matrix_no_rot(m, x);
        }),
        ("pppDrawMatrixWood", 0x0073_4cf0, |m, x, _, _| {
            b::ppp_draw_matrix_wood(m, x);
        }),
        ("pppKeDMatDraw", 0x0073_4eb0, |m, x, _, c| {
            b::ppp_ke_d_mat_draw(m, x, c);
        }),
        ("pppKeDMatFrDraw", 0x0073_4ef0, |m, x, _, c| {
            b::ppp_ke_d_mat_fr_draw(m, x, c);
        }),
        ("pppKeDMatPhtDraw", 0x0073_4f80, |m, x, _, c| {
            b::ppp_ke_d_mat_pht_draw(m, x, c);
        }),
        ("pppKeDMatPhtFrDraw", 0x0073_5020, |m, x, _, c| {
            b::ppp_ke_d_mat_pht_fr_draw(m, x, c);
        }),
    ]
};

type RotCompound = (&'static str, u32, [mat::Axis; 3]);

const ROT_COMPOUNDS: [RotCompound; 6] = {
    use mat::order as o;
    [
        ("pppGetRotMatrixXYZ", 0x0072_c800, o::XYZ),
        ("pppGetRotMatrixXZY", 0x0072_c890, o::XZY),
        ("pppGetRotMatrixYXZ", 0x0072_c9f0, o::YXZ),
        ("pppGetRotMatrixYZX", 0x0072_ca80, o::YZX),
        ("pppGetRotMatrixZXY", 0x0072_cbe0, o::ZXY),
        ("pppGetRotMatrixZYX", 0x0072_cc70, o::ZYX),
    ]
};

type RandCon = fn(&mut Oracle, u32, u32, &mut dyn FnMut(&mut Oracle) -> u32);

const RAND_CONS: [(&str, u32, RandCon); 3] = [
    ("pppKeShpTail3Con", 0x0075_0510, |m, o, c, r| {
        prog::bridge::ppp_ke_shp_tail3_con(m, o, c, &mut |x: &mut Oracle| r(x));
    }),
    ("pppKeShpTail3XCon", 0x0075_1eb0, |m, o, c, r| {
        prog::bridge::ppp_ke_shp_tail3_con(m, o, c, &mut |x: &mut Oracle| r(x));
    }),
    ("pppKeShpTailPhtCon", 0x0075_4180, |m, o, c, r| {
        prog::bridge::ppp_ke_shp_tail_pht_con(m, o, c, &mut |x: &mut Oracle| r(x));
    }),
];

#[test]
#[allow(clippy::too_many_lines)] // one setup, then the random rounds
fn matrix_programs_match_the_original() {
    use mat::bridge as b;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x3a77_1c05);
        let rand_state = fake_rand(&mut p);
        let obj = p.alloc(0x800);
        let parent = p.alloc(0x60);
        let key = p.alloc(0x40);
        let ctrl = p.alloc(0x10);
        let offsets = p.alloc(4 * 4);
        let angles = p.alloc(0x10);
        let out = p.alloc(0x40);
        let part = p.alloc(0x80);
        p.set(ctrl + 0xc, offsets);
        for round in 0..3000 {
            for i in (0..0x800).step_by(4) {
                let v = rng.float();
                p.set(obj + i, v);
            }
            for i in (0..0x60).step_by(4) {
                let v = rng.float();
                p.set(parent + i, v);
            }
            // angles anywhere, some exact quarter turns
            for i in (0x0..0x800).step_by(0x40) {
                if rng.chance(30) {
                    for j in 0..4 {
                        let a = if rng.chance(30) {
                            rng.below(8) * 0x2000
                        } else {
                            rng.next()
                        };
                        p.set(obj + i + 4 * j, a);
                    }
                }
            }
            p.set(obj + 4, parent);
            // the four areas in distinct 0x80-byte slots
            let mut slots: Vec<u32> = (0..12).collect();
            for i in 0..4u32 {
                let j = i + rng.below(12 - i);
                slots.swap(i as usize, j as usize);
                p.set(offsets + 4 * i, slots[i as usize] * 0x80);
            }
            p.set(key + 4, if rng.chance(20) { u32::MAX } else { 0 });
            for i in 0..3 {
                p.set(angles + 4 * i, rng.next());
            }
            if !p.snapshot() || round > 0 {
                // r0's words fill the rotations: no signalling NaNs
                p.put(
                    ppp::bridge::R0,
                    [rng.float(), rng.float(), rng.float(), rng.float()],
                );
                for g in [b::VIEW.sym, b::VIEW_COPY.sym, b::WOOD.sym] {
                    let at = p.sym(g);
                    for i in (0..0x40).step_by(4) {
                        let v = rng.float();
                        p.set(at + i, v);
                    }
                }
                let at = p.sym(b::MAT_SN.sym);
                p.set(at, rng.next());
                p.set(at + 4, rng.next());
                for i in (0..0x80).step_by(4) {
                    let v = rng.float();
                    p.set(part + i, v);
                }
                p.put(prog::bridge::CURRENT_PART, part);
            }
            let (name, sym, f) = rng.pick(MAT_STEPS);
            p.check(
                &format!("{name} round {round}"),
                sym,
                &[obj, key, ctrl],
                |m| {
                    f(m, obj, key, ctrl);
                    None
                },
            );
            let (name, sym, order) = rng.pick(&ROT_COMPOUNDS);
            p.check(&format!("{name} round {round}"), sym, &[out, angles], |m| {
                b::ppp_get_rot_matrix(m, out, angles, order);
                None
            });
            let (name, sym, f) = rng.pick(&RAND_CONS);
            p.check(&format!("{name} round {round}"), sym, &[obj, ctrl], |m| {
                f(m, obj, ctrl, &mut rand_lcg(rand_state));
                None
            });
        }
    }
}

// ---- oef2 blocks, ppp environments, character draw flags ---------------

#[test]
#[allow(clippy::too_many_lines)] // one setup, then the random rounds
#[allow(clippy::cast_possible_truncation)] // small random values
fn oef2_env_and_draw_flags_match_the_original() {
    use et::bridge as e;
    use heap::bridge as h;
    use op::bridge as o;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0ef2_5e70);
        let oef = p.alloc(0x2000);
        let data = p.alloc(0x2000);
        let heaps = p.alloc(0x40);
        let arena = p.alloc(0x4000);
        let env = p.alloc(0x40);
        let env_slot = p.alloc(4);
        let tables = p.alloc(0x20);
        for round in 0..2000u32 {
            // an oef2 block: table offsets, the sound records' table at 0x800
            for k in 0..10 {
                let at = if k == 6 { 0x800 } else { 0x100 + 0x40 * k };
                p.set(oef + 4 * k, at);
            }
            for i in (0x100..0x2000).step_by(4) {
                let v = match rng.below(4) {
                    0 => 0,
                    1 => rng.below(0x20_0000),
                    2 => 0x8000_0000 | rng.below(0x100),
                    _ => rng.next(),
                };
                p.set(oef + i, v);
            }
            for r in 0..0x40 {
                p.set(oef + 0x800 + 0x10 * r, u8::from(rng.chance(60)));
            }
            // its data: offsets to the source tables, the counts, the mark
            let d = data + if rng.chance(10) { 4 } else { 0 };
            for k in 1..10 {
                p.set(d + 4 * k, 0x100 + 0x40 * k);
            }
            for i in (0x100..0x400).step_by(4) {
                let v = if rng.chance(30) { 0 } else { rng.below(0x1000) };
                p.set(d + i, v);
            }
            for at in (0x42..0x54).step_by(2) {
                let n = if rng.chance(20) {
                    0u16
                } else {
                    rng.below(16) as u16
                };
                p.set(d + at, n);
            }
            p.set(d + 0x40, u16::from(rng.chance(10)));
            p.set(oef + 0x3c, d.wrapping_sub(oef));
            if rng.chance(50) {
                p.check(
                    &format!("op_oef2_set round {round}"),
                    0x007e_3b70,
                    &[oef, d],
                    |m| {
                        o::op_oef2_set(m, oef, d);
                        None
                    },
                );
            } else {
                p.check(
                    &format!("op_oef2_set2 round {round}"),
                    0x007e_3e30,
                    &[oef],
                    |m| {
                        o::op_oef2_set2(m, oef);
                        None
                    },
                );
            }
            let kind = rng.pick(&[0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
            let n = rng.next();
            for k in [2u32, 3, 6] {
                p.set(oef + 4 * k, 0x100 + rng.below(0x40) * 4);
            }
            p.set(oef + 4 * kind, 0x100 + rng.below(0x100) * 4);
            p.check(
                &format!("op_oef2_a_get({kind}) round {round}"),
                0x007e_3810,
                &[oef, kind, n],
                |m| Some(o::op_oef2_a_get(m, oef, kind, n)),
            );
            let arg = if rng.chance(50) { 0 } else { oef };
            p.check("op_oef2_clut_load", 0x007e_46b0, &[arg], |_| None);

            // pppInitEnv and pppMemAlloc on two heaps
            p.put(h::ENV_SLOT, env_slot);
            let with_data = if rng.chance(70) { tables } else { 0 };
            p.fill(tables, 0x20, &mut rng);
            p.check(
                &format!("pppInitEnv round {round}"),
                0x0071_69d0,
                &[env, with_data, arena, 0x2000],
                |m| {
                    h::ppp_init_env(m, env, with_data, arena, 0x2000);
                    None
                },
            );
            let spare = heaps + 0x20;
            if rng.chance(70) {
                p.check(
                    "pppCreateHeap",
                    0x0072_c480,
                    &[spare, arena + 0x2000, 0x2000],
                    |m| {
                        h::ppp_create_heap(m, spare, arena + 0x2000, 0x2000);
                        None
                    },
                );
            } else {
                p.set(spare + 4, 0u32);
            }
            for _ in 0..8 {
                let size = rng.below(0x900);
                p.check(
                    &format!("pppMemAlloc({size:#x}) round {round}"),
                    0x0071_6a30,
                    &[env, spare, size],
                    |m| Some(h::ppp_mem_alloc(m, env, spare, size)),
                );
            }

            // character records with handles and draw flag indexes
            if !p.snapshot() || round > 0 {
                let chr = p.sym(e::CHR.sym);
                for i in 0..e::CHRS {
                    let handle = if rng.chance(30) { 0x1000 + i } else { 0 };
                    p.set(chr + 0x20 * i, handle);
                    let index = rng.below(0x100) as u16;
                    p.set(chr + 0x20 * i + 0xe, index);
                }
                for g in [e::DRAW_FLAGS.sym, e::DRAW_FLAGS2.sym] {
                    let at = p.sym(g);
                    p.fill(at, 0x100, &mut rng);
                }
            }
            let chr = p.sym(e::CHR.sym);
            let i = rng.below(e::CHRS);
            let handle = rng.pick(&[0, 0x1000 + i, 0x5555]);
            if handle == 0x1000 + i {
                p.set(chr + 0x20 * i, handle);
                p.set(chr + 0x20 * i + 0xe, rng.below(0xf0) as u16);
            }
            let save = rng.chance(50);
            let (name, sym) = if save {
                ("op_et_draw_flags_save", 0x007f_c190)
            } else {
                ("op_et_draw_flags_restore", 0x007f_c130)
            };
            let found = (0..e::CHRS).find(|&j| p.get::<u32>(chr + 0x20 * j) == handle);
            if found.is_some_and(|j| p.get::<u16>(chr + 0x20 * j + 0xe) >= 0xf0) {
                continue; // the original fast-fails
            }
            p.check(&format!("{name} round {round}"), sym, &[handle], |m| {
                Some(if save {
                    e::op_et_draw_flags_save(m, handle)
                } else {
                    e::op_et_draw_flags_restore(m, handle)
                })
            });
            let on = rng.pick(&[0u32, 1, 0x100, 0xffff_ffff]);
            p.check(
                &format!("op_et_chr_show({on}) round {round}"),
                0x007f_f090,
                &[on],
                |m| {
                    e::op_et_chr_show(m, on);
                    None
                },
            );
        }
    }
}

// ---- small helpers ----------------------------------------------------

#[test]
#[allow(clippy::cast_possible_truncation)] // small random values
#[allow(clippy::many_single_char_names)] // a, s, ...
fn small_helpers_match_the_original() {
    use et::bridge as e;
    use op::bridge as o;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x5a11_4e1f);
        let a = p.alloc(0x40);
        let s = p.alloc(0x100);
        let unit = p.alloc(0x40);
        let list = p.alloc(0x40);
        for round in 0..3000u32 {
            p.fill(a, 0x40, &mut rng);
            for i in (0..0x40).step_by(4) {
                let v = rng.float();
                p.set(a + i, v);
            }
            let (src, dst) = (a + rng.below(4) * 4, a + 0x10 + rng.below(4) * 4);
            p.check("VFX_Copy16From", 0x0071_0e30, &[dst, src], |m| {
                Some(o::copy16(m, dst, src))
            });
            p.check("VFX_Copy16To", 0x0071_0e70, &[src, dst], |m| {
                Some(o::copy16(m, dst, src))
            });
            p.check("VFX_Copy16To_0072f3c0", 0x0072_f3c0, &[src, dst], |m| {
                Some(o::copy16(m, dst, src))
            });
            let sc = rng.float();
            let v = if rng.chance(20) { dst } else { src };
            p.check("VFX_ScaleVec3", 0x0071_0ea0, &[dst, v, sc], |m| {
                Some(o::vfx_scale_vec3(m, dst, v, f32::from_bits(sc)))
            });
            // a string of random bytes
            let len = rng.below(0x40);
            for i in 0..len {
                let c = (rng.next() as u8).max(1);
                p.set(s + i, c);
            }
            p.set(s + len, 0u8);
            p.check("DJB_hash", 0x0067_b7d0, &[s], |m| Some(o::djb_hash(m, s)));
            // an id list
            let n = rng.below(12) as u16;
            p.set(unit + 0x34, n);
            p.set(unit + 0x1c, list);
            for i in 0..16 {
                let v: u16 = if rng.chance(10) {
                    0xffff
                } else {
                    rng.below(8) as u16
                };
                p.set(list + 2 * i, v);
            }
            let id = rng.below(9) | (rng.next() & 0xffff_0000);
            p.check("opu_id_list_add", 0x0080_0d60, &[unit, id], |m| {
                Some(o::opu_id_list_add(m, unit, id as u16))
            });
            // g_EtChr numbers and g_EtEff parts
            if !p.snapshot() || round > 0 {
                let chr = p.sym(e::CHR.sym);
                for i in 0..e::CHRS {
                    p.set(chr + 0x20 * i + 4, rng.below(64));
                }
                let eff = p.sym(e::EFF.sym);
                for i in 0..16 {
                    let r = eff + 0x20 * i;
                    p.set(r + 8, rng.below(10) as u8);
                    for j in 0..8 {
                        p.set(r + 0x10 + j, rng.below(6) as u8);
                    }
                }
            }
            let no = rng.below(70);
            p.check("op_et_chr_index_by_no", 0x007f_eaa0, &[no], |m| {
                Some(e::op_et_chr_index_by_no(m, no).cast_unsigned())
            });
            p.set(unit + 0x14, rng.below(16) as u16);
            p.set(unit + 0x1a, rng.next() as u16);
            p.set(unit + 0x18, rng.below(6) as u8);
            p.check("opu_eff_part_index", 0x0080_0da0, &[unit], |m| {
                Some(e::opu_eff_part_index(m, unit))
            });
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one round per helper
#[allow(clippy::many_single_char_names, clippy::cast_possible_truncation)] // v, s, k, f; small values
fn vu0_macros_and_tables_match_the_original() {
    use et::bridge as e;
    use mat::bridge as b;
    use op::bridge as o;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x7a0c_0de5);
        let v = p.alloc(0x40);
        let dst = p.alloc(0x40);
        let s = p.alloc(0x100);
        let out = p.alloc(0x200);
        let acc = p.sym(b::ACC.sym);
        for round in 0..3000u32 {
            for at in [v, dst, acc] {
                for i in (0..0x10).step_by(4) {
                    let x = rng.float();
                    p.set(at + i, x);
                }
            }
            let k = rng.float();
            let f = f32::from_bits(k);
            let junk = rng.next();
            for (name, at) in [
                ("pppVu0MaddaY", 0x0072_c590),
                ("pppVu0MaddaY_copy_0075e670", 0x0075_e670),
            ] {
                p.check(name, at, &[junk, v, junk, k], |m| {
                    b::ppp_vu0_madda(m, v, f);
                    Some(v)
                });
            }
            p.check(
                "pppVu0MaddaZ_copy_0075e6d0",
                0x0075_e6d0,
                &[junk, v, junk, junk, k],
                |m| {
                    b::ppp_vu0_madda(m, v, f);
                    Some(v)
                },
            );
            p.check(
                "pppVu0MaddaZ",
                0x0072_c5f0,
                &[junk, v, junk, junk, k],
                |m| {
                    b::ppp_vu0_madda(m, v, f);
                    Some(v)
                },
            );
            p.check(
                "pppVu0MaddW",
                0x0072_c650,
                &[dst, v, junk, junk, junk, k],
                |m| {
                    b::ppp_vu0_madd(m, dst, v, f);
                    Some(dst)
                },
            );
            p.check(
                "pppVu0MaddW_copy_0074bde0",
                0x0074_bde0,
                &[dst, v, junk, junk, junk, k],
                |m| {
                    b::ppp_vu0_madd(m, dst, v, f);
                    Some(dst)
                },
            );
            let d = if rng.chance(20) { v + 4 } else { dst };
            p.check("pppVu0MulX", 0x0072_c6a0, &[d, v, k], |m| {
                b::ppp_vu0_mul(m, d, v, f);
                Some(d)
            });
            p.check("pppVu0MulX_007500d0", 0x0075_00d0, &[d, v, k], |m| {
                b::ppp_vu0_mul(m, d, v, f);
                Some(v)
            });
            p.check("pppVu0AddY", 0x0074_b630, &[dst, v, junk, k], |m| {
                b::ppp_vu0_add_x(m, dst, v, f);
                Some(dst)
            });
            p.check("pppVu0AddZ", 0x0074_b650, &[dst, v, junk, junk, k], |m| {
                b::ppp_vu0_add_x(m, dst, v, f);
                Some(dst)
            });
            // strings and word copies
            let len = rng.below(0x40);
            for i in 0..len {
                let c = (rng.next() as u8).max(1);
                p.set(s + i, c);
            }
            p.set(s + len, 0u8);
            let str_at = if rng.chance(10) { 0 } else { s };
            p.check("op_strlen", 0x007e_2250, &[str_at], |m| {
                Some(o::op_strlen(m, str_at))
            });
            let n = rng.below(12);
            p.check("lmemcpy", 0x0076_b0f0, &[out, s, n], |m| {
                Some(o::lmemcpy(m, out, s, n))
            });
            // setters
            let x = rng.next();
            p.check("pppFpSetDisp", 0x0072_9050, &[x], |m| {
                Some(fp::bridge::ppp_fp_set_disp(m, x))
            });
            p.check("setVfxShapeEnlargeBufferFactor", 0x0071_7610, &[x], |m| {
                Some(o::set_vfx_shape_enlarge_buffer_factor(m, x))
            });
            p.check("resetVfxShapeEnlargeBufferFactor", 0x0071_7600, &[], |m| {
                o::set_vfx_shape_enlarge_buffer_factor(m, 1);
                None
            });
            // effect records and character handles
            if round % 20 == 1 {
                let eff = p.sym(e::EFF.sym);
                for i in 0..e::EFFS {
                    p.set(eff + 0x20 * i, rng.below(8));
                }
                let chr = p.sym(e::CHR.sym);
                let handles = p.sym(e::CHR_HANDLES.sym);
                for i in 0..e::CHRS {
                    let h = if rng.chance(50) {
                        0
                    } else {
                        0x100 + rng.below(300)
                    };
                    p.set(chr + 0x20 * i, h);
                    p.set(chr + 0x20 * i + 0xe, rng.below(0xf0) as u16);
                    p.set(handles + 4 * i, 0x100 + rng.below(300));
                }
            }
            let id = rng.pick(&[u32::MAX, 0, 1, 2, 3, 9]);
            p.check("opu_eff_free_by_id", 0x007f_e920, &[id], |m| {
                e::opu_eff_free_by_id(m, id);
                None
            });
            if round % 20 == 1 {
                // 240 x 240 compares in the emulator: now and then
                p.check("opus_chr_index_map", 0x007f_e450, &[out], |m| {
                    Some(e::opus_chr_index_map(m, out))
                });
            }
        }
    }
}

// ---- round 5: renderer flags, EE/VU emulation, rotations, packets -----

/// A float that is a number (no NaNs or infinities: the rotations' x87
/// sums of products would pick between two NaNs by register order).
fn finite(rng: &mut Xorshift) -> u32 {
    loop {
        let v = rng.float();
        if (v & 0x7f80_0000) != 0x7f80_0000 {
            return v;
        }
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names
)]
fn round5_helpers_match_the_original() {
    use heap::bridge as h;
    use mat::bridge as b;
    use op::bridge as o;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0005_ba5e);
        let x = p.alloc(0x40);
        let y = p.alloc(0x40);
        let dst = p.alloc(0x40);
        let angles = p.alloc(0x10);
        let packet = p.alloc(0x2000);
        let env_slot = p.alloc(4);
        let env = p.alloc(0x40);
        let arena = p.alloc(0x2000);
        let obj = p.alloc(0x400);
        let parent = p.alloc(0x60);
        let key = p.alloc(0x40);
        let ctrl = p.alloc(0x10);
        let offsets = p.alloc(0x10);
        let tables = p.alloc(0x400);
        p.set(ctrl + 0xc, offsets);
        p.put(h::ENV_SLOT, env_slot);
        p.set(env_slot, env);
        for round in 0..2000u32 {
            let f = rng.next();
            for (name, sym) in [
                ("calculateVFXSP_ZMat", 0x0071_2d00),
                ("calculateVFXSP_AlphaFunction", 0x0071_2c30),
            ] {
                let arg = if rng.chance(50) { rng.below(0x100) } else { f };
                p.check(name, sym, &[arg], |_| {
                    Some(if sym == 0x0071_2d00 {
                        o::calculate_vfx_sp_zmat(arg)
                    } else {
                        o::calculate_vfx_sp_alpha_function(arg)
                    })
                });
            }
            p.fill(x, 0x40, &mut rng);
            p.fill(y, 0x40, &mut rng);
            let g = if rng.chance(20) { 0 } else { x };
            p.check(
                "graphicVFXGetRenderDataByGeometry",
                0x0064_53b0,
                &[g],
                |m| Some(o::graphic_vfx_get_render_data_by_geometry(m, g)),
            );
            p.check("graphicVFXSetCurrentVFXType", 0x0064_5670, &[f], |m| {
                o::CURRENT_TYPE.set(m, f as u8);
                None
            });
            p.check("pppEePmulth", 0x0073_74e0, &[dst, x, y], |m| {
                b::ppp_ee_pmulth(m, dst, x, y);
                None
            });
            p.check("pppEePpacb", 0x0073_75d0, &[dst, x, y], |m| {
                b::ppp_ee_ppacb(m, dst, x, y);
                None
            });
            // rotations of a matrix of numbers
            for i in (0..0x40).step_by(4) {
                let v = finite(&mut rng);
                p.set(x + i, v);
            }
            for i in 0..4 {
                let a = if rng.chance(30) {
                    f32::from(rng.below(0x3000) as u16).to_bits()
                } else {
                    (f32::from_bits(finite(&mut rng)) % 1.0e6).to_bits()
                };
                p.set(angles + 4 * i, a);
            }
            let a: u32 = p.get(angles);
            let d = if rng.chance(30) { x } else { dst };
            for (name, sym, axis) in [
                ("op_rot_m_x", 0x007e_9580, mat::Axis::X),
                ("op_rot_m_y", 0x007e_9670, mat::Axis::Y),
                ("op_rot_m_z", 0x007e_9760, mat::Axis::Z),
            ] {
                p.check(name, sym, &[d, x, a], |m| {
                    o::op_rt_m(m, d, x, f32::from_bits(a), axis);
                    None
                });
            }
            p.check("op_rt_m_yxz", 0x007e_9a30, &[dst, x, angles, y], |m| {
                o::op_rt_m_yxz(m, dst, x, angles, y);
                None
            });
            // a model packet: tags of random types and counts
            let mut t = packet + 0x20;
            for _ in 0..rng.below(8) {
                p.set(
                    t + 1,
                    rng.below(8) as u8 | if rng.chance(30) { 0x80 } else { 0 },
                );
                p.set(t + 2, rng.below(16) as u16);
                let kind = u32::from(p.get::<u8>(t + 1) & 7);
                let size: u32 = p.get(p.sym(o::MD_TAG_SIZES.sym) + 4 * kind);
                t += 0x10 + u32::from(p.get::<u16>(t + 2)) * size;
            }
            p.set(t + 1, 0xffu8);
            if !p.snapshot() || round > 0 {
                let id = rng.pick(&[0u32, 0x94, 0x239, 0xd0, 7]);
                p.put(ffx_system::instance::bridge::MAGIC_ID, id);
            }
            let (va, vb) = (rng.next(), rng.next());
            p.check(
                "op_md_packet_set_addr",
                0x00a6_9de0,
                &[packet, va, vb],
                |m| {
                    o::op_md_packet_set_addr(m, packet, va, vb);
                    None
                },
            );
            // the part matrix
            for i in (0..0x80).step_by(4) {
                let v = finite(&mut rng);
                p.set(obj + i, v);
            }
            for i in 0..3 {
                p.set(obj + 0x50 + 4 * i, rng.next());
            }
            if !p.snapshot() || round > 0 {
                p.put(
                    ppp::bridge::R0,
                    [
                        finite(&mut rng),
                        finite(&mut rng),
                        finite(&mut rng),
                        finite(&mut rng),
                    ],
                );
            }
            p.check("pppSetMatrix", 0x0071_7410, &[obj], |m| {
                b::ppp_set_matrix(m, obj);
                None
            });
            // pppVertexAttend on a shape table of the environment
            for i in (0..0x60).step_by(4) {
                let v = finite(&mut rng);
                p.set(parent + i, v);
            }
            p.set(obj + 4, parent);
            p.set(offsets, 0u32);
            p.set(offsets + 4, 0x40u32);
            let records = tables;
            let blocks = tables + 0x80;
            let lists = tables + 0x100;
            let verts = tables + 0x200;
            p.set(env + 0x28, records);
            p.set(env + 0x20, blocks);
            for r in 0..8 {
                p.set(records + 8 * r, rng.below(4) as u16);
                p.set(records + 8 * r + 4, lists + 0x10 * r);
            }
            for k in 0..4 {
                p.set(blocks + 0x20 * k + 0x14, verts + 0x40 * k);
            }
            p.fill(lists, 0x100, &mut rng);
            for i in (0..0x80).step_by(2) {
                p.set(lists + i, rng.below(10) as u16);
            }
            p.fill(verts, 0x100, &mut rng);
            p.set(obj + 0xa0, rng.below(8) as u16);
            p.set(key + 4, if rng.chance(10) { u32::MAX } else { 0 });
            p.set(
                key + 0xc,
                if rng.chance(10) {
                    0xffffu16
                } else {
                    rng.below(8) as u16
                },
            );
            p.check("pppVertexAttend", 0x0075_d340, &[obj, key, ctrl], |m| {
                b::ppp_vertex_attend(m, obj, key, ctrl);
                None
            });
            // pppKeLnsLpCon, pppVtMimeDes and pppMemFree on the environment's heaps
            p.set(offsets, rng.below(0x40) * 4);
            p.check("pppKeLnsLpCon", 0x0075_af30, &[obj, ctrl], |m| {
                prog::bridge::ppp_ke_lns_lp_con(m, obj, ctrl);
                None
            });
            if round % 4 == 0 {
                p.check("pppCreateHeap", 0x0072_c480, &[env, arena, 0x1000], |m| {
                    h::ppp_create_heap(m, env, arena, 0x1000);
                    None
                });
                p.check(
                    "pppCreateHeap",
                    0x0072_c480,
                    &[env + 0x10, arena + 0x1000, 0x1000],
                    |m| {
                        h::ppp_create_heap(m, env + 0x10, arena + 0x1000, 0x1000);
                        None
                    },
                );
            }
            let size = rng.below(0x200);
            let block = p.check("pppMemAlloc", 0x0071_6a30, &[env, env + 0x10, size], |m| {
                Some(h::ppp_mem_alloc(m, env, env + 0x10, size))
            });
            let off0: u32 = p.get(offsets);
            if rng.chance(50) {
                p.set(obj + off0 + 0xac, block);
                p.check("pppVtMimeDes", 0x0072_ede0, &[obj, ctrl], |m| {
                    prog::bridge::ppp_vt_mime_des(m, obj, ctrl);
                    None
                });
            } else {
                p.check("pppMemFree", 0x0071_6a90, &[env, env + 0x10, block], |m| {
                    h::ppp_mem_free(m, env, env + 0x10, block);
                    None
                });
            }
        }
    }
}

// ---- parts and objects ------------------------------------------------

/// The program table (`g_pppProgramTable`).
const PROGRAM_TABLE: u32 = 0x00c3_a508;

fn call_original(m: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    m.call(f, Conv::Cdecl, args).unwrap().eax
}

#[test]
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names
)]
fn parts_and_objects_match_the_original() {
    use part::bridge as pt;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x9a27_0b1e);
        let programs = p.sym(PROGRAM_TABLE);
        let block = p.alloc(0x1000);
        let env_slot = p.alloc(4);
        let env = p.alloc(0x40);
        let arena = p.alloc(0x8000);
        let part = p.alloc(0x80);
        let defs = p.alloc(0x400);
        let list = p.alloc(0x20);
        // (defs: four 16-byte definitions, their program lists from +0x100)
        let offs = p.alloc(0x10);
        let keys = p.alloc(0x200 * 16);
        for i in (0..0x200 * 16).step_by(0x20) {
            p.set(keys + i, (i % 0x200) / 0x20 * 0x1000);
            for j in (4..0x20).step_by(4) {
                let v = rng.float();
                p.set(keys + i + j, v);
            }
        }
        let part2 = p.alloc(0x80);
        let start = p.alloc(0x200);
        let log = record_srand(&mut p);
        let hits_list = p.alloc(0x400);
        // (hits_list + 0x200: the shape tables of the pppVertexAp steps)
        for i in 0..4 {
            p.set(offs + 4 * i, 0x40 * i);
        }
        let nodes = p.alloc(0x40);
        let recs = p.alloc(0x80);
        let set = p.sym(0x0071_7610); // setVfxShapeEnlargeBufferFactor
        p.put(heap::bridge::ENV_SLOT, env_slot);
        p.set(env_slot, env);
        // graphicVFXDestroy: the renderer's side (its tables are not set up
        // in the oracle); the bridges call it like the originals
        let destroy = p.sym(0x0064_5280);
        p.a.stub_return(destroy, 0).unwrap();
        p.b.stub_return(destroy, 0).unwrap();
        for round in 0..1500u32 {
            // a part data block: three definitions and a terminator
            p.fill(block, 0x1000, &mut rng);
            p.set(block + 8, 0x800u32);
            p.set(block + 0xc, 0x900u32);
            let n8 = rng.below(5);
            let n9 = rng.below(5);
            p.set(block + 0x800, n8);
            p.set(block + 0x900, n9);
            for i in 0..n8 {
                p.set(block + 0x804 + 4 * i, rng.below(12));
            }
            for i in 0..n9 {
                p.set(block + 0x904 + 4 * i, rng.below(0x100) * 4);
            }
            // the definitions: the first at +0x10, the others 0x100 apart,
            // each linked by its first word (relative to the block)
            let chain = 1 + rng.below(4);
            for k in 0..=chain {
                let def = if k == 0 {
                    block + 0x10
                } else {
                    block + 0x100 * k
                };
                let next = if k == chain { 0 } else { 0x100 * (k + 1) };
                p.set(def, next);
                let n = rng.below(4) as u16;
                p.set(def + 0x26, n);
                for e in 0..u32::from(n) {
                    p.set(def + 0x28 + 0x10 * e, rng.below(12));
                    p.set(def + 0x30 + 0x10 * e, rng.below(0x80) * 4);
                    p.set(def + 0x34 + 0x10 * e, rng.below(0x80) * 4);
                }
            }
            if rng.chance(5) {
                p.set(block + 0x10, 0u32);
            }
            let pdt = block;
            p.check(
                &format!("pppInitPdt round {round}"),
                0x0071_20f0,
                &[pdt, programs],
                |m| {
                    pt::ppp_init_pdt(m, pdt, programs);
                    None
                },
            );

            // an effect data block: two empty part data blocks and the
            // three other tables
            let data = block + 0xa00;
            for (at, off) in [
                (0x10u32, 0x100u32),
                (0x14, 0x200),
                (0x18, 0x300),
                (0x1c, 0x380),
            ] {
                p.set(data + at, off);
            }
            let pdts = rng.below(3) as u16;
            p.set(data + 6, pdts);
            for i in 0..u32::from(pdts) {
                let pd = 0x40 + 0x40 * i;
                p.set(data + 0x100 + 4 * i, pd);
                p.set(data + pd + 8, 0x20u32);
                p.set(data + pd + 0xc, 0x24u32);
                p.set(data + pd + 0x20, 0u32);
                p.set(data + pd + 0x24, 0u32);
                p.set(data + pd + 0x10, 0u32);
            }
            for (at, n) in [(8u32, 4u32), (0xa, 8), (0xc, 8)] {
                p.set(data + at, rng.below(n) as u16);
            }
            p.check(
                &format!("pppInitData round {round}"),
                0x0071_1fa0,
                &[data, programs, 0],
                |m| {
                    pt::ppp_init_data(m, data, programs);
                    None
                },
            );

            // pppOtFunc_Reg on a list
            if round % 8 == 0 {
                p.set(list + 4, list + 0x10);
                p.set(list + 0xc, 0u16);
            }
            let node = nodes + 0x10 * rng.below(4);
            let (a, b) = (rng.next(), rng.next());
            p.check("pppOtFunc_Reg", 0x0071_6be0, &[list, node, a, b], |m| {
                pt::ppp_ot_func_reg(m, list, node, a, b);
                None
            });

            // object definitions whose programs are the first twelve
            // records of the table (pppAccele ... pppColor)
            if round % 16 == 0 {
                for (h, mem) in [(env, arena), (env + 0x10, arena + 0x4000)] {
                    p.check("pppCreateHeap", 0x0072_c480, &[h, mem, 0x4000], |m| {
                        heap::bridge::ppp_create_heap(m, h, mem, 0x4000);
                        None
                    });
                }
                p.set(part + 0x30, 0u32);
                for d in 0..4 {
                    // 16-byte definitions (the part's birth records too),
                    // their program lists after them
                    let def = defs + 0x10 * d;
                    let prog = defs + 0x100 + 0x80 * d;
                    p.set(def, prog);
                    p.set(def + 4, 0u32);
                    p.set(def + 0xc, 0u16);
                    p.set(prog + 0x20, 0x200 + 0x40 * rng.below(4));
                    p.set(prog + 0x24, rng.below(4) as u16);
                    let n = 1 + rng.below(3) as u16;
                    p.set(prog + 0x26, n);
                    p.set(prog + 0xc, u8::from(rng.chance(20)));
                    p.set(prog + 0x14, rng.pick(&[0x7000_0000u32, 0x3000, 0x8000]));
                    p.set(prog + 0x18, 0u32);
                    p.set(prog + 0x1c, rng.pick(&[0x7fff_ffffu32, 0x4000]));
                    for e in 0..u32::from(n) {
                        let entry = prog + 0x28 + 0x10 * e;
                        // the steps of pppAccele .. pppColor, or a draw
                        let k = rng.pick(&[0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 19, 20]);
                        let r = if rng.chance(10) {
                            0
                        } else {
                            programs + 0x28 * k
                        };
                        p.set(entry, r);
                        p.set(entry + 4, 0x20u16);
                        // the program's keys: frames 0x1000 apart
                        let k = keys + 0x200 * (4 * d + e);
                        p.set(entry + 8, k);
                        // the entry's +0xc: the work area offsets the
                        // set-up reads
                        p.set(entry + 0xc, offs);
                    }
                }
            }
            let def = defs + 0x10 * rng.below(4);
            p.check(
                &format!("pppCreatePObject round {round}"),
                0x0071_6770,
                &[part, def],
                |m| Some(pt::ppp_create_pobject(m, part, def, &mut call_original)),
            );
            let obj = p.get::<u32>(part + 0x30);
            if obj != 0 {
                p.check("pppPObjectCon2", 0x0071_2d20, &[obj], |m| {
                    pt::ppp_pobject_con(m, obj, 0x20, &mut call_original);
                    None
                });
            }
            // births
            p.set(part + 8, rng.below(10));
            p.set(part + 0x18, rng.below(4));
            p.set(part + 0x3c, if rng.chance(10) { 0 } else { defs });
            for d in 0..4 {
                p.set(defs + 0x10 * d + 8, rng.below(12));
            }
            p.check("pppBornPObjects", 0x0071_2b80, &[part], |m| {
                pt::ppp_born_pobjects(m, part, &mut call_original);
                None
            });
            // (a null birth list only here: pppRunPartStd would read
            // through it)
            p.set(part + 0x3c, defs);
            // a frame of the part
            p.set(part + 0x1c, u8::from(rng.chance(50)));
            p.set(part + 0x1d, u8::from(rng.chance(20)));
            p.set(part + 4, rng.below(0x20000));
            p.set(part + 0x10, 0x1000u32);
            if !p.snapshot() || round > 0 {
                p.put(ppp::bridge::STOP, u32::from(rng.chance(10)));
            }
            p.put(prog::bridge::CURRENT_PART, part);
            if rng.chance(50) {
                p.check(
                    &format!("pppRunPartStd round {round}"),
                    0x0071_6c40,
                    &[part],
                    |m| {
                        pt::ppp_run_part_std(m, part, &mut call_original);
                        None
                    },
                );
            } else {
                let flags = rng.pick(&[0u32, 0x10, 0x20, 0x30]);
                p.check(
                    &format!("pppRunPart round {round}"),
                    0x0071_2250,
                    &[part, flags],
                    |m| Some(pt::ppp_run_part(m, part, flags, &mut call_original)),
                );
            }
            let head = p.get::<u32>(part + 0x30);
            if head != 0 && rng.chance(10) {
                let next = p.get::<u32>(head);
                p.set(part + 0x30, next);
                p.check("pppDeletePObject", 0x0071_68f0, &[head, 0], |m| {
                    pt::ppp_delete_pobject(m, head, &mut call_original);
                    None
                });
            }
            if round % 16 == 15 {
                // the birth records it frees: a heap block
                let env = p.get::<u32>(env_slot);
                let births = p.check("pppMemAlloc", 0x0071_6a30, &[env, env + 0x10, 0x40], |m| {
                    Some(heap::bridge::ppp_mem_alloc(m, env, env + 0x10, 0x40))
                });
                p.set(part + 0x3c, births);
                p.check("pppAllFreePObject", 0x0071_1f20, &[part], |m| {
                    pt::ppp_all_free_pobject(m, part, &mut call_original);
                    None
                });
                p.set(part + 0x3c, defs);
            }

            // the part's program loops
            let n = rng.below(4);
            p.set(part + 0x14, n);
            p.set(part + 0x34, recs);
            for i in 0..4 {
                let r = recs + 0x40 + 0x10 * i;
                p.set(recs + 4 * i, r);
                for at in [0x10, 0x14, 0x18] {
                    p.set(r + at, if rng.chance(50) { set } else { 0 });
                }
            }
            // the user functions and the programs that make child objects
            let table = recs + 0x10;
            for i in 0..4 {
                p.set(table + 4 * i, if rng.chance(50) { set } else { 0 });
            }
            let arg = rng.next();
            p.check("pppCallUsrFunc", 0x0071_6710, &[part, table, arg], |m| {
                pt::ppp_call_usr_func(m, part, table, arg, &mut call_original);
                None
            });
            let head = p.get::<u32>(part + 0x30);
            if head != 0 {
                p.put(pt::USER_FUNCS, if rng.chance(30) { 0 } else { table });
                let key = start + 0x180;
                let ctrl = start + 0x1c0;
                p.set(ctrl + 0xc, offs);
                p.fill(key, 0x20, &mut rng);
                p.set(
                    key + 4,
                    if rng.chance(10) {
                        u32::MAX
                    } else {
                        rng.below(4)
                    },
                );
                p.set(key + 8, rng.below(0x10) * 4);
                p.set(
                    key + 0xc,
                    if rng.chance(10) {
                        u32::MAX
                    } else {
                        rng.below(4)
                    },
                );
                p.set(key + 0x10, rng.below(0x10) * 4);
                p.check("pppDrawHookDraw", 0x0075_7370, &[head, key, ctrl], |m| {
                    pt::ppp_draw_hook(m, head, key, ctrl, &mut call_original);
                    None
                });
                if !p.snapshot() || round > 0 {
                    p.put(ppp::bridge::STOP, u32::from(rng.chance(10)));
                }
                p.check("pppPointAp", 0x0075_73c0, &[head, key, ctrl], |m| {
                    pt::ppp_point_ap(m, head, key, ctrl, &mut call_original);
                    None
                });
                let v = rng.next() & 0xffff;
                p.check("pppBornShort", 0x0075_8270, &[head, key, v], |m| {
                    pt::ppp_born_short(m, head, key, v as u16, &mut call_original);
                    None
                });
                let vert = start + 0x1e0;
                p.fill(vert, 8, &mut rng);
                for i in (0x10..0x50).step_by(4) {
                    let x = finite(&mut rng);
                    p.set(head + i, x);
                }
                for (name, sym, relative) in [
                    ("pppBornVertex", 0x0075_7770, false),
                    ("pppBornVertexRel", 0x0075_7b50, true),
                ] {
                    p.check(name, sym, &[head, key, vert], |m| {
                        pt::ppp_born_vertex(m, head, key, vert, relative, &mut call_original);
                        None
                    });
                }
                // pppPointRAp: two rand-program draws for the direction
                p.set(log, 0u32);
                for at in [4u32, 8] {
                    let v = finite(&mut rng);
                    p.set(key + at, v);
                }
                p.set(key + 0x18, rng.below(0x10) * 4);
                p.set(key + 0x1c, rng.below(4) as u8);
                if !p.snapshot() || round > 0 {
                    p.put(rand::bridge::R, rng.next());
                    p.put(rand::bridge::SEEDED, u32::from(rng.chance(80)));
                    p.put(
                        ppp::bridge::R0,
                        [finite(&mut rng), rng.next(), rng.next(), rng.next()],
                    );
                }
                p.check("pppPointRAp", 0x0075_7490, &[head, key, ctrl], |m| {
                    pt::ppp_point_r_ap(m, head, key, ctrl, &mut call_original, &mut srand_log(log));
                    None
                });
                // pppKeHitBorn over a list of three hit nodes (key +8 the
                // child's field again, +0xc the flag)
                p.set(
                    key + 4,
                    if rng.chance(10) {
                        u32::MAX
                    } else {
                        rng.below(4)
                    },
                );
                p.set(key + 8, rng.below(0x10) * 4);
                p.set(key + 0xc, u8::from(rng.chance(50)));
                p.set(key, p.get::<u32>(head + 0xc));
                if !p.snapshot() || round > 0 {
                    let id = rng.pick(&[0u32, 0xcb, 0x296]);
                    p.put(ffx_system::instance::bridge::MAGIC_ID, id);
                }
                let nodes = hits_list;
                for k in 0..3u32 {
                    let n = nodes + 0x100 * k;
                    p.set(n, if k == 2 { 0 } else { n + 0x100 });
                    p.set(n + 0x3e, rng.pick(&[0u16, 0x800, 0x840, 0x40]));
                    p.set(n + 0x36, rng.below(3) as u8);
                }
                let a0 = p.get::<u32>(offs);
                p.set(head + a0 + 0xa0, if rng.chance(10) { 0 } else { hits_list });
                p.check("pppKeHitBorn", 0x0075_9d80, &[head, key, ctrl], |m| {
                    pt::ppp_ke_hit_born(m, head, key, ctrl, &mut call_original);
                    None
                });
                // the pppVertexAp steps on a shape table of the environment
                let shapes = hits_list + 0x200;
                let (records, blocks, lists, verts) =
                    (shapes, shapes + 0x40, shapes + 0x80, shapes + 0x100);
                p.set(env + 0x28, records);
                p.set(env + 0x20, blocks);
                for r in 0..4 {
                    p.set(records + 8 * r, rng.below(2) as u16);
                    p.set(records + 8 * r + 2, rng.below(6) as u16);
                    p.set(records + 8 * r + 4, lists + 0x10 * r);
                }
                for k in 0..2 {
                    p.set(blocks + 0x20 * k + 0x14, verts + 0x40 * k);
                }
                for i in (0..0x40).step_by(2) {
                    p.set(lists + i, rng.below(8) as u16);
                }
                p.fill(verts, 0x80, &mut rng);
                p.set(head + 0x90, if rng.chance(20) { verts } else { 0 });
                p.set(
                    key + 4,
                    if rng.chance(10) {
                        0xffffu16
                    } else {
                        rng.below(4) as u16
                    },
                );
                p.set(key + 6, rng.below(3) as u8);
                p.set(key + 7, rng.below(4) as u8);
                p.set(key + 8, rng.pick(&[0u8, 0, 1, 1, 2]));
                p.set(
                    key + 0xc,
                    if rng.chance(10) {
                        u32::MAX
                    } else {
                        rng.below(4)
                    },
                );
                p.set(key + 0x10, rng.below(0x10) * 4);
                let a0 = p.get::<u32>(offs);
                p.set(head + a0 + 0xa2, rng.below(2) as u16);
                p.set(head + a0 + 0xa0, rng.below(8) as u16);
                let hash = p.sym(0x0064_1da0);
                let h = rng.pick(&[0u32, 0xe6c, 0x18cd]);
                // the story and the script's effect flags, for the
                // localisation rule
                let scen = p.sym(0x0078_7360);
                let atel = p.sym(0x0086_b030);
                let (sv, av) = (rng.pick(&[0u32, 0xcc6]), u32::from(rng.chance(50)));
                for (at, v) in [(hash, h), (scen, sv), (atel, av)] {
                    p.a.stub_return(at, v).unwrap();
                    p.b.stub_return(at, v).unwrap();
                }
                // r0 is the zero register: draws in [0, 1) then, which the
                // random form scales to an index
                p.put(ppp::bridge::R0, [0, rng.next(), rng.next(), rng.next()]);
                let form = rng.pick(&[
                    ("pppVertexAp", 0x0075_7840, pt::VertexAp::Ap),
                    ("pppVertexApDisPos", 0x0075_7c30, pt::VertexAp::DisPos),
                    ("pppVertexApLc", 0x0075_7ff0, pt::VertexAp::Lc),
                    ("pppVertexApAt", 0x0075_82e0, pt::VertexAp::At),
                ]);
                p.set(log, 0u32);
                p.check(form.0, form.1, &[head, key, ctrl], |m| {
                    pt::ppp_vertex_ap(
                        m,
                        head,
                        key,
                        ctrl,
                        form.2,
                        &mut call_original,
                        &mut srand_log(log),
                    );
                    None
                });
                p.put(ppp::bridge::STOP, 0);
            }

            // pppStartPart on a data block: length, loop flag, programs
            // (the records above), births (a chain of definitions)
            if round % 4 == 3 {
                p.set(start, rng.below(0x10000));
                p.set(start + 4, u8::from(rng.chance(50)));
                p.set(start + 8, 0x100u32);
                p.set(start + 0xc, 0x140u32);
                p.set(start + 0x100, 4u32);
                for i in 0..4 {
                    p.set(start + 0x104 + 4 * i, recs + 0x40 + 0x10 * i);
                }
                let nb = 1 + rng.below(3);
                p.set(start + 0x140, nb);
                for k in 0..nb {
                    let node = start + 0x10 + 0x20 * k;
                    p.set(node, if k + 1 == nb { 0 } else { node + 0x20 });
                    p.set(node + 0x10, rng.below(8) * 0x1000);
                }
                let rate = rng.below(0x2000);
                let set_ups = u32::from(rng.chance(50));
                // TOGetFFXLang: a fixed language (its manager logs at
                // creation, which the oracle cannot run)
                let lang = p.sym(0x0064_10e0);
                let l = rng.pick(&[1u32, 0xb]);
                p.a.stub_return(lang, l).unwrap();
                p.b.stub_return(lang, l).unwrap();
                p.check(
                    "pppStartPart",
                    0x0071_23c0,
                    &[part2, rate, start, set_ups],
                    |m| {
                        pt::ppp_start_part(m, part2, rate, start, set_ups, &mut call_original);
                        None
                    },
                );
                p.put(prog::bridge::CURRENT_PART, part);
            }
            for (name, sym, at) in [
                ("pppPartCall10", 0x0071_2e40, 0x10),
                ("pppPartCall14", 0x0071_2e80, 0x14),
                ("pppPartCall18", 0x0071_2e00, 0x18),
            ] {
                p.check(name, sym, &[part], |m| {
                    pt::ppp_part_call(m, part, at, &mut call_original);
                    None
                });
            }
        }
    }
}

#[test]
#[allow(clippy::cast_possible_truncation)]
fn lns_t_programs_match_the_original() {
    use prog::bridge::{self as pb, LnsT};
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x1a5e_77c0);
        let destroy = p.sym(0x0064_5280);
        p.a.stub_return(destroy, 0).unwrap();
        p.b.stub_return(destroy, 0).unwrap();
        // the draws the nodes call (the renderer's side) replaced by a
        // recorder of their arguments
        arg_recorder(
            &mut p,
            &[
                0x0075_a250,
                0x0075_a610,
                0x0075_a9e0,
                0x0075_ade0,
                0x0075_b230,
            ],
            3,
        );
        let list = p.sym(0x0230_fff0);
        p.set(list + 4, list);
        p.set(list + 0xc, 0u16);
        // eight objects, each its own offsets; `None` while unregistered
        let objs = p.alloc(8 * 0x400);
        let ctrls = p.alloc(8 * 0x20);
        let mut live: [Option<Option<LnsT>>; 8] = [None; 8];
        for i in 0..8 {
            let ctrl = ctrls + 0x20 * i;
            p.set(ctrl + 0xc, ctrl + 0x10);
            p.set(ctrl + 0x10, rng.below(0x40) * 4);
        }
        for _ in 0..2000 {
            if rng.chance(10) {
                p.check("pppOtFunc_Control", 0x0071_6ac0, &[list], |m| {
                    part::bridge::ppp_ot_func_control(m, list, &mut call_original);
                    None
                });
            }
            let i = rng.below(8);
            let (obj, ctrl) = (objs + 0x400 * i, ctrls + 0x20 * i);
            match live[i as usize] {
                None => {
                    p.fill(obj, 0x400, &mut rng);
                    p.put(part::bridge::CURRENT_PROGRAM, rng.next());
                    p.put(pb::LNS_LP_T_COUNT, rng.next() as u8);
                    p.set(list + 0xe, rng.next() as u16);
                    let kinds = [
                        Some(LnsT::Arnd),
                        Some(LnsT::Clm),
                        Some(LnsT::Crn),
                        Some(LnsT::Fls),
                        None,
                    ];
                    let kind = kinds[rng.below(5) as usize];
                    match kind {
                        Some(k) => {
                            let at =
                                [0x0075_a0f0, 0x0075_a4a0, 0x0075_a870, 0x0075_ac60][k as usize];
                            p.check(&format!("{k:?}TCon"), at, &[obj, ctrl], |m| {
                                Some(pb::ppp_ke_lns_t_con(m, k, obj, ctrl))
                            });
                        }
                        None => {
                            p.check("pppKeLnsLpTCon", 0x0075_b090, &[obj, ctrl], |m| {
                                pb::ppp_ke_lns_lp_t_con(m, obj, ctrl);
                                None
                            });
                        }
                    }
                    live[i as usize] = Some(kind);
                }
                Some(Some(k)) => {
                    let at = [0x0075_a140, 0x0075_a4f0, 0x0075_a8b0, 0x0075_acb0][k as usize];
                    p.check(&format!("{k:?}TDes"), at, &[obj, ctrl], |m| {
                        Some(pb::ppp_ke_lns_t_des(m, k, obj, ctrl, &mut call_original))
                    });
                    live[i as usize] = None;
                }
                Some(None) => {
                    p.check("pppKeLnsLpTDes", 0x0075_b130, &[obj, ctrl], |m| {
                        Some(pb::ppp_ke_lns_lp_t_des(m, obj, ctrl))
                    });
                    live[i as usize] = None;
                }
            }
        }
    }
}

/// Replaces the functions at `syms` by a recorder of their first `n` (at
/// most eight) arguments (a log of 256 entries of 32 bytes after a count),
/// so that checks compare what callers pass them; the recorder returns the
/// low bit of the new count. Returns the log.
fn arg_recorder(p: &mut Pair, syms: &[u32], n: u8) -> u32 {
    let log = p.alloc(4 + 0x100 * 32);
    let mut c = vec![0x8b, 0x0d]; // mov ecx, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x81, 0xe1, 0xff, 0, 0, 0]); // and ecx, 0xff
    c.extend([0xc1, 0xe1, 0x05]); // shl ecx, 5
    for k in 0..n {
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp + 4 + 4k]
        c.extend([0x89, 0x81]); // mov [ecx + log + 4 + 4k], eax
        c.extend((log + 4 + 4 * u32::from(k)).to_le_bytes());
    }
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.push(0xa1); // mov eax, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x83, 0xe0, 0x01]); // and eax, 1
    c.push(0xc3);
    for &sym in syms {
        let at = p.sym(sym);
        p.a.write(at, &c).unwrap();
        p.b.write(at, &c).unwrap();
    }
    log
}

#[test]
#[allow(clippy::cast_possible_truncation)]
fn shape_runners_match_the_original() {
    use part::bridge as pt;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x5a9e_d2a1);
        arg_recorder(&mut p, &[pt::DRAW_SHAPE0, pt::DRAW_SHAPE_X0], 3);
        let objs = p.alloc(8 * 0x100);
        let keys = p.alloc(0x400);
        let (def, prog, e) = (p.alloc(0x10), p.alloc(0x40), p.alloc(0x10));
        for _ in 0..2000 {
            let n = 1 + rng.below(8);
            let c = rng.below(4);
            p.set(prog + 0xc, u8::from(rng.chance(20)));
            p.set(prog + 0x20, 0x20 + 4 * rng.below(8));
            p.set(e + 4, (4 * rng.below(8)) as u16);
            p.set(def + 4, objs);
            p.set(def + 0xc, n as u16);
            for k in 0..0x100 {
                p.set(keys + 4 * k, rng.below(4));
            }
            for k in 0..n {
                let o = objs + 0x100 * k;
                p.set(o, objs + 0x100 * (k + 1));
                p.set(o + 0xc, rng.below(4));
                let slot = o + p.get::<u32>(prog + 0x20) + 4 * c;
                p.set(slot, keys + 4 * rng.below(0x80));
            }
            let (name, at, draw) = if rng.chance(50) {
                ("pppDrawShape", 0x0074_09e0, pt::DRAW_SHAPE0)
            } else {
                ("pppDrawShapeX", 0x0074_0ed0, pt::DRAW_SHAPE_X0)
            };
            p.check(name, at, &[def, prog, e, c], |m| {
                Some(pt::ppp_draw_shape(
                    m,
                    draw,
                    def,
                    prog,
                    e,
                    c,
                    &mut call_original,
                ))
            });
        }
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::cast_possible_truncation
)]
fn et_opus_math_and_trees_match_the_original() {
    use et::bridge as e;
    use tree::bridge as t;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0e75_7ee5);
        // op_oef2_set2, opu_et_unit and opu_unit_free_by_oef2: recorded
        arg_recorder(&mut p, &[0x007e_3e30], 1);
        arg_recorder(&mut p, &[0x007f_d7f0], 4);
        arg_recorder(&mut p, &[0x007f_df70], 2);
        arg_recorder(&mut p, &[0x007f_de30], 2); // opu_unit_free_by_id
        arg_recorder(&mut p, &[0x007f_c2e0], 4); // op_et_eff
        let btl = p.sym(0x0112_a8e0); // g_BtlStatus (MsBattleGetStatus)
        let handles = p.sym(e::CHR_HANDLES.sym);
        let chr_at = p.sym(e::CHR.sym);
        let nos = p.sym(opus::bridge::WEAPON_NOS.sym);
        let weapons = p.sym(opus::bridge::WEAPONS.sym);
        let monsters = p.sym(opus::bridge::MONSTERS.sym);
        let (dst, v) = (p.alloc(0x10), p.alloc(0x10));
        let tree = p.alloc(0x10);
        let nodes = p.alloc(0x20 * 0x40);
        let it = p.alloc(4);
        let key = p.alloc(4);
        for round in 0..300 {
            // the opus handles and characters
            for k in 0..0xf0 {
                let h = if rng.chance(60) {
                    0
                } else {
                    0x100 + rng.below(8)
                };
                p.set(handles + 4 * k, h);
            }
            for i in 0..e::CHRS {
                let h = if rng.chance(80) {
                    0
                } else {
                    0x100 + rng.below(8)
                };
                p.set(chr_at + 0x20 * i, h);
                p.set(chr_at + 0x20 * i + 0xe, rng.below(0x10000) as u16);
            }
            p.set(btl, rng.pick(&[0u8, 1, 0xff]));
            p.check("opus_chr_handle_count", 0x007f_cf60, &[], |m| {
                Some(e::opus_chr_handle_count(m, &mut call_original))
            });
            let n = rng.below(0x30);
            p.check("opus_chr_handle_nth", 0x007f_cfb0, &[n], |m| {
                Some(e::opus_chr_handle_nth(m, n, &mut call_original))
            });
            p.check("opus_chr_draw_index_nth", 0x007f_cf10, &[n], |m| {
                Some(e::opus_chr_draw_index_nth(m, n, &mut call_original))
            });
            // weapons and monsters
            for i in 0..4 {
                p.set(nos + i, rng.pick(&[0u8, 1, 2, 0xff]));
                p.set(weapons + 4 * i, rng.pick(&[0u32, 0x200, 0x300]));
            }
            for i in 0..8 {
                p.set(monsters + 4 * i, rng.pick(&[0u32, 0, 0x400, 0x500]));
            }
            let no = rng.pick(&[0u32, 1, 2, 0xff, 0x101]);
            p.check("opus_get_weapon_by_no", 0x0080_0fd0, &[no], |m| {
                Some(opus::bridge::opus_get_weapon_by_no(m, no))
            });
            p.check("opus_get_weapon_index_by_no", 0x0080_1030, &[no], |m| {
                Some(opus::bridge::opus_get_weapon_index_by_no(m, no).cast_unsigned())
            });
            let o = rng.pick(&[0u32, 0x400, 0x500, 0x600]);
            p.check("op_et_monster_add", 0x007f_c8a0, &[o], |m| {
                Some(e::op_et_monster_add(m, o, &mut call_original))
            });
            p.put(e::OPUS_BATTLE, rng.next());
            p.check("op_et_oef2_free", 0x007f_c2a0, &[o], |m| {
                Some(e::op_et_oef2_free(m, o, &mut call_original))
            });
            p.check("opu_et_oef2_free", 0x007f_da60, &[o], |m| {
                Some(e::op_et_oef2_free(m, o, &mut call_original))
            });
            for at in [0x007f_c2c0, 0x007f_c970] {
                p.check("op_et_oef2_start", at, &[o], |m| {
                    Some(e::op_et_oef2_start(m, o, &mut call_original))
                });
            }
            let args = [o, rng.next(), rng.next(), rng.next()];
            p.check("opu_et_unit_ofs", 0x007f_da30, &args, |m| {
                Some(e::opu_et_unit_ofs(m, args, &mut call_original))
            });
            let id = rng.next();
            p.check("opu_et_unit_free_by_id", 0x007f_da40, &[id], |m| {
                Some(e::opu_et_unit_free_by_id(m, id, &mut call_original))
            });
            let h = rng.pick(&[0u32, 0x100, 0x101, 0x104, 0x107, 0x999]);
            p.check("op_et_bindeff2_off", 0x007f_b6d0, &[h], |m| {
                let r = e::op_et_bindeff2_off(m, h, &mut call_original);
                // (the original's eax is undefined for a null handle)
                (h != 0).then_some(r)
            });
            // op_et_bindeff on records whose oef2 objects may be monsters
            for i in 0..e::CHRS {
                let r = chr_at + 0x20 * i;
                p.set(r + 8, rng.pick(&[0u32, 0x400, 0x500, 0x700]));
                p.set(r + 0xc, rng.next() as u16);
            }
            p.put(e::BIND_OFF, u8::from(rng.chance(10)));
            p.check("op_et_bindeff", 0x007f_b290, &[h], |m| {
                e::op_et_bindeff(m, h, &mut call_original)
            });
            p.check("op_et_bindeff_off", 0x007f_bb90, &[h], |m| {
                e::op_et_bindeff_off(m, h, &mut call_original);
                None
            });
            for i in 0..e::CHRS {
                p.set(chr_at + 0x20 * i + 0xe, rng.pick(&[0u16, 1, 2, 0xff]));
            }
            p.check("op_et_weapon_unit", 0x007f_c9b0, &[h], |m| {
                Some(e::op_et_weapon_unit(m, h, &mut call_original))
            });
            p.put(e::ENCOUNT, o);
            p.check("op_et_encount_effect_free", 0x007f_c5f0, &[], |m| {
                Some(e::op_et_encount_effect_free(m, &mut call_original))
            });
            p.put(e::BIND_DEFAULT, rng.next());
            p.check("op_et_bind_default_water", 0x007f_b870, &[], |m| {
                Some(e::op_et_bind_default_water(m))
            });
            if round % 50 == 0 {
                p.check("op_et_eff_init_all", 0x007f_ef70, &[], |m| {
                    e::op_et_eff_init_all(m);
                    None
                });
            }
            // the vector helpers (no signalling NaNs)
            for i in 0..4 {
                let x = finite(&mut rng);
                p.set(v + 4 * i, x);
            }
            p.fill(dst, 0x10, &mut rng);
            let sb = finite(&mut rng);
            let s = f32::from_bits(sb);
            p.check("vfx_vec_w_add", 0x0081_7cc0, &[dst, v, sb], |m| {
                mat::bridge::vfx_vec_w_add(m, dst, v, s);
                None
            });
            p.check("vfx_vec3_scale", 0x0075_bb00, &[dst, v, sb], |m| {
                mat::bridge::vfx_vec3_scale(m, dst, v, s);
                None
            });
            p.check("vfx_vec4_min", 0x0080_8c10, &[dst, v, sb], |m| {
                mat::bridge::vfx_vec4_min(m, dst, v, s);
                None
            });
            // the keyed tree: insert a few, find some
            p.set(tree + 0xc, tree);
            let count = 1 + rng.below(0x40);
            for k in 0..count {
                let node = nodes + 0x20 * k;
                p.fill(node, 0x10, &mut rng);
                p.set(node + 0xc, rng.below(0x20).cast_signed() - 0x10);
                let r =
                    p.a.call(p.sym(0x0070_fda0), Conv::Thiscall(tree), &[node])
                        .unwrap();
                let mine = t::vfx_int_tree_insert(&mut p.b, tree, node);
                assert_eq!(r.eax, mine, "vfxIntTree_Insert");
                p.same("vfxIntTree_Insert");
            }
            for _ in 0..8 {
                p.set(key, rng.below(0x24).cast_signed() - 0x12);
                let r =
                    p.a.call(p.sym(0x0070_fd20), Conv::Thiscall(tree), &[key])
                        .unwrap();
                assert_eq!(
                    r.eax,
                    t::vfx_int_tree_find(&p.b, tree, key),
                    "vfxIntTree_Find"
                );
            }
            // the iterated tree: a random binary search tree (header node 0,
            // nil; children and the root's parent the header), walked from a
            // random node
            let m = 1 + rng.below(0x20);
            let node = |k: u32| nodes + 0x20 * k;
            let (mut left, mut right, mut parent) = ([0u32; 0x21], [0u32; 0x21], [0u32; 0x21]);
            let mut keys = [0u32; 0x21];
            for k in 1..=m {
                keys[k as usize] = rng.next();
                if k == 1 {
                    continue;
                }
                let mut c = 1usize;
                loop {
                    let next = if keys[k as usize] < keys[c] {
                        &mut left[c]
                    } else {
                        &mut right[c]
                    };
                    if *next == 0 {
                        *next = k;
                        parent[k as usize] = c as u32;
                        break;
                    }
                    c = *next as usize;
                }
            }
            for k in 0..=m {
                let n = node(k);
                let ku = k as usize;
                p.set(n, node(left[ku]));
                p.set(n + 4, node(parent[ku]));
                p.set(n + 8, node(right[ku]));
                p.set(n + 0xd, u8::from(k == 0));
            }
            p.set(it, node(rng.below(m + 1)));
            for at in [0x0080_6a50, 0x0080_6aa0, 0x0069_ebd0] {
                for _ in 0..4 {
                    let r = p.a.call(p.sym(at), Conv::Thiscall(it), &[]).unwrap();
                    let mine = t::opus_tree_next(&mut Capped::new(&mut p.b), it);
                    assert_eq!(r.eax, mine, "opusTree_Next");
                    p.same("opusTree_Next");
                }
            }
        }
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names
)]
fn model_draw_helpers_match_the_original() {
    use heap::bridge as h;
    use mat::bridge as b;
    use prog::bridge as pb;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x3d0d_e15a);
        // rcbgPSimClearOMD and 0x0091fd40 (another module's): recorded
        arg_recorder(&mut p, &[0x0091_f9b0, 0x0091_fd40], 1);
        // MSVCR110's _CIsqrt (the oracle has no stub): `fsqrt; ret` through
        // its import slot (0x00b0c39c), as the CRT computes it at 53 bits
        let sqrt = p.sym(0x00b0_bfe0); // code padding before the imports
        p.a.write(sqrt, &[0xd9, 0xfa, 0xc3]).unwrap();
        p.b.write(sqrt, &[0xd9, 0xfa, 0xc3]).unwrap();
        let slot = p.sym(0x00b0_c39c);
        p.set(slot, sqrt);
        let (x, y, dst) = (p.alloc(0x40), p.alloc(0x40), p.alloc(0x40));
        let env_slot = p.alloc(4);
        let env = p.alloc(0x40);
        let arena = p.alloc(0x4000);
        let obj = p.alloc(0x400);
        let ctrl = p.alloc(0x10);
        let offsets = p.alloc(0x10);
        let blocks = p.alloc(0x100);
        let shape = p.alloc(0x200);
        let rec = p.alloc(0x10);
        p.set(ctrl + 0xc, offsets);
        p.put(h::ENV_SLOT, env_slot);
        p.set(env_slot, env);
        p.set(env + 0x20, blocks);
        for round in 0..2000u32 {
            p.fill(x, 0x40, &mut rng);
            p.fill(y, 0x40, &mut rng);
            p.fill(dst, 0x40, &mut rng);
            // the EE helpers (dst may be a source)
            let d = rng.pick(&[dst, x, y]);
            p.check("pppEePextlh", 0x0073_7450, &[d, x, y], |m| {
                b::ppp_ee_pextlh(m, d, x, y);
                None
            });
            p.check("pppEePpach", 0x0073_76a0, &[d, y, x], |m| {
                b::ppp_ee_ppach(m, d, y, x);
                None
            });
            let any = rng.next();
            let sa = rng.pick(&[0u32, 1, 5, 16, 31, 32, 0x41, any]);
            p.check("pppEePsrlw", 0x0073_7700, &[d, x, sa], |m| {
                b::ppp_ee_psrlw(m, d, x, sa);
                None
            });
            p.check("pppVu0MoveVia", 0x0073_ce30, &[dst, x], |m| {
                b::ppp_vu0_move_via(m, dst, x);
                None
            });
            for at in [0x0073_ce80, 0x0073_e110] {
                p.check("pppVu0LoadVf00", at, &[dst], |m| {
                    b::ppp_vu0_load_vf00(m, dst);
                    None
                });
            }
            // the length macro: no NaNs in
            for i in 0..4 {
                let v = finite(&mut rng);
                p.set(x + 4 * i, v);
            }
            p.set(y, x);
            for at in [0x0073_ceb0, 0x0073_d7c0, 0x0073_e140] {
                let thunk = p.float_thunk(at, y, dst);
                p.check_f32("pppVu0Length", thunk, dst, |m| b::ppp_vu0_length(m, x));
            }
            // the colour shift
            p.put(ppp::bridge::COLOR_CON_SHIFT, 4 * rng.below(8));
            p.check("pppColorShiftLoad", 0x0073_7420, &[dst, y], |m| {
                b::ppp_color_shift_load(m, dst, y);
                None
            });
            p.put(ppp::bridge::COLOR_CON_SHIFT, 4 * rng.below(8));
            p.check("pppColorShiftStore", 0x0073_7750, &[x, dst], |m| {
                b::ppp_color_shift_store(m, x, dst);
                None
            });
            for at in [0x0073_e990, 0x0073_f180, 0x0073_f9a0] {
                p.check("pppShapeIterInit", at, &[x, dst], |m| {
                    b::ppp_shape_iter_init(m, x, dst);
                    None
                });
            }
            p.check("pppShapeRandFlip", 0x0074_19e0, &[x, dst], |m| {
                b::ppp_shape_rand_flip(m, x, dst);
                None
            });
            // the destructors on the environment's heaps
            if round % 64 == 0 {
                p.check("pppCreateHeap", 0x0072_c480, &[env, arena, 0x2000], |m| {
                    h::ppp_create_heap(m, env, arena, 0x2000);
                    None
                });
                p.check(
                    "pppCreateHeap",
                    0x0072_c480,
                    &[env + 0x10, arena + 0x2000, 0x2000],
                    |m| {
                        h::ppp_create_heap(m, env + 0x10, arena + 0x2000, 0x2000);
                        None
                    },
                );
            }
            p.set(offsets + 8, 4 * rng.below(0x20));
            let off2: u32 = p.get(offsets + 8);
            let w = obj + off2;
            let alloc = |p: &mut Pair, rng: &mut Xorshift| -> u32 {
                if rng.chance(30) {
                    return 0;
                }
                let n = 0x10 + rng.below(0x80);
                p.check("pppMemAlloc", 0x0071_6a30, &[env, env + 0x10, n], |m| {
                    Some(h::ppp_mem_alloc(m, env, env + 0x10, n))
                })
            };
            match round % 3 {
                0 => {
                    for f in [0x108, 0x10c] {
                        let a = alloc(&mut p, &mut rng);
                        p.set(w + f, a);
                    }
                    p.check("pppDrawMdlSeaDes", 0x0073_ab70, &[obj, ctrl], |m| {
                        pb::ppp_draw_mdl_sea_des(m, obj, ctrl);
                        None
                    });
                }
                1 => {
                    for f in [0xd0, 0xd4, 0xd8] {
                        let a = alloc(&mut p, &mut rng);
                        p.set(w + 0xa0 + f, a);
                        if a != 0 {
                            p.fill(a, 0x10, &mut rng);
                        }
                    }
                    let index = rng.below(8);
                    p.set(rec + 4, index);
                    p.set(w + 0xa0 + 0xdc, rec);
                    p.set(blocks + 0x20 * index + 0x1c, shape);
                    p.set(shape + 8, 0x20u32);
                    p.set(shape + 0x12, rng.pick(&[0u16, 1, 2, 0xffff]));
                    p.put(pb::MDL_INF_ACTIVE, 1);
                    p.check("pppDrawMdlInfDes", 0x0073_b830, &[obj, ctrl], |m| {
                        pb::ppp_draw_mdl_inf_des(m, obj, ctrl);
                        None
                    });
                }
                _ => {
                    let sim = rng.pick(&[0u32, x, y]);
                    p.set(w + 0xa0, sim);
                    p.check("pppDrawMdlPSimDes", 0x0073_d010, &[obj, ctrl], |m| {
                        pb::ppp_draw_mdl_psim_des(m, obj, ctrl, &mut call_original);
                        None
                    });
                }
            }
        }
    }
}

#[test]
fn born_rnd_steps_match_the_original() {
    use prog::bridge::{self as pb, BornRnd};
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00b0_7d5e);
        let _log = record_srand(&mut p);
        // KeBornRnd2 (the map module's, it makes the children): recorded
        arg_recorder(&mut p, &[0x0076_9230], 7);
        let obj = p.alloc(0x400);
        let key = p.alloc(0x40);
        let ctrl = p.alloc(0x10);
        let offsets = p.alloc(0x10);
        p.set(ctrl + 0xc, offsets);
        let stop = p.sym(ppp::bridge::STOP.sym);
        for _ in 0..4000 {
            p.set(offsets, 4 * rng.below(0x40));
            p.fill(key, 0x40, &mut rng);
            for at in [0xd, 7] {
                let any = rng.next().to_le_bytes()[0];
                p.set(key + at, rng.pick(&[0u8, 0xff, 1, 3, 0x80, any]));
            }
            for at in [0xc, 0x10, 0x14, 0x18] {
                if rng.chance(20) {
                    p.set(key + at, u32::MAX);
                }
            }
            let off: u32 = p.get(offsets);
            p.set(obj + off + 0xa2, rng.pick(&[0u8, 0, 1, 2, 9]));
            p.set(stop, u32::from(rng.chance(10)));
            // KeVu0RndI's state (g_Vu0_RSeeded, R, the EE register it
            // starts from)
            p.set(p.sym(0x00c8_f728), u32::from(rng.chance(90)));
            p.set(p.sym(0x00c0_a000), rng.next());
            p.set(p.sym(0x00c8_f508), rng.next());
            let (kind, at) = rng.pick(&[
                (BornRnd::Rnd2, 0x0075_87b0),
                (BornRnd::Rnd3, 0x0075_8960),
                (BornRnd::Rnd5, 0x0075_8f20),
                (BornRnd::Rnd6, 0x0075_9200),
            ]);
            p.check(&format!("{kind:?}"), at, &[obj, key, ctrl], |m| {
                pb::ppp_ke_born_rnd(m, kind, obj, key, ctrl, &mut call_original);
                None
            });
        }
    }
}

#[test]
#[allow(clippy::cast_possible_truncation)]
fn oef2_sound_cues_match_the_original() {
    use op::bridge as o;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x500d_c0e5);
        // the sound module's functions: recorded with their arguments
        arg_recorder(&mut p, &[0x0088_7760], 0); // SndStopBattleStream
        arg_recorder(&mut p, &[0x0088_6e50, 0x0088_75a0, 0x0088_6ff0], 3);
        arg_recorder(&mut p, &[0x0088_6f30], 2); // SndSepTransSepOnly
        arg_recorder(&mut p, &[0x0088_5ee0], 1); // SndDataSendSync
        let magic_id = p.sym(0x009d_a3b0);
        let oef = p.alloc(0x400);
        let recs = p.alloc(0x100);
        p.set(oef + 0x18, 0x100u32);
        let streams = [p.sym(0x023c_3520), p.sym(0x023c_3524)];
        for _ in 0..3000 {
            for n in 0..0x20 {
                let cue = oef + 0x100 + 0x10 * n;
                p.set(cue, rng.pick(&[0u8, 1, 1, 2, 3]));
                let rec = if rng.chance(10) {
                    0
                } else {
                    recs + 0x10 * rng.below(8)
                };
                p.set(cue + 8, rec);
            }
            p.fill(recs, 0x100, &mut rng);
            p.set(oef + 0x4c, rng.below(0x22) as u16);
            for s in streams {
                let v = oef + 0x100 + 0x10 * rng.below(0x20);
                p.set(s, if rng.chance(50) { v } else { 0 });
            }
            p.put(o::SOUND_CUE_HEAD, rng.next() as u16);
            p.put(o::SOUND_CUE_COUNT, rng.pick(&[0u16, 1, 0x1f, 0x20, 0x21]));
            for g in [o::OEF2_STREAM_CUES.sym, o::STREAM_CUE2.sym] {
                p.set(p.sym(g), rng.pick(&[0u32, 0, u32::MAX, 0x1234]));
            }
            let id = rng.pick(&[0x1fbu32, 0, 0x100]);
            p.a.stub_return(magic_id, id).unwrap();
            p.b.stub_return(magic_id, id).unwrap();
            let cue = oef + 0x100 + 0x10 * rng.below(0x20);
            let any = rng.next();
            let pan = rng.next();
            let vol = rng.pick(&[0u32, 1, 0x100, 0xffff_ff00, any]);
            // (the originals dereference a kind-1 cue's record unchecked)
            let usable = p.get::<u8>(cue) != 1 || p.get::<u32>(cue + 8) != 0;
            if usable {
                p.check("op_oef2_se_on", 0x007f_fe10, &[cue, pan, vol], |m| {
                    Some(o::op_oef2_se_on(m, cue, pan, vol, &mut call_original))
                });
                p.check("op_oef2_se_volume", 0x007f_fde0, &[cue, vol], |m| {
                    Some(o::op_oef2_se_volume(m, cue, vol, &mut call_original))
                });
            }
            p.check("op_oef2_sound_cue_push", 0x007f_feb0, &[cue, pan], |m| {
                Some(o::op_oef2_sound_cue_push(m, cue, pan, &mut call_original))
            });
            let from = rng.below(0x24);
            let any = rng.below(0x24);
            let to = rng.pick(&[any, 0xffff, 0x1_0003]);
            p.check(
                "op_oef2_sound_cue_multi",
                0x0080_02c0,
                &[oef, from, to, pan],
                |m| {
                    Some(o::op_oef2_sound_cue_multi(
                        m,
                        oef,
                        from,
                        to,
                        pan,
                        &mut call_original,
                    ))
                },
            );
            p.check("op_oef2_sound_cue_check", 0x0080_00f0, &[], |m| {
                Some(o::op_oef2_sound_cue_check(m, &mut call_original))
            });
        }
    }
}

#[test]
#[allow(clippy::cast_possible_truncation)]
fn chara_sizes_and_offset_steps_match_the_original() {
    use op::bridge as o;
    use prog::bridge as pb;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0c5a_0f57);
        // 0x0075c660 and the renderer's 0x00644be0: recorded
        arg_recorder(&mut p, &[0x0075_c660, 0x0064_4be0], 2);
        let chara = [
            0x0083_c9a0, // Ch_ClutGetNb
            0x0083_c9f0, // Ch_ClutGetSize
            0x0080_f630, // opus_mimic_check
            0x0080_f580,
            0x0083_0410, // Ch_GetNbModel
            0x0082_add0, // Ch_ModelGetHide
            0x0083_0450, // Ch_ModelCalcSizeEffectPacket
        ];
        let env_slot = p.alloc(4);
        let env = p.alloc(0x40);
        let blocks = p.alloc(0x400);
        let (obj, key, ctrl, offsets) =
            (p.alloc(0x400), p.alloc(0x40), p.alloc(0x10), p.alloc(0x10));
        p.set(ctrl + 0xc, offsets);
        p.put(heap::bridge::ENV_SLOT, env_slot);
        p.set(env_slot, env);
        p.set(env + 0x20, blocks);
        let stop = p.sym(ppp::bridge::STOP.sym);
        for _ in 0..800 {
            for &f in &chara {
                let any = rng.below(0x10000);
                let v = rng.pick(&[0u32, 0, 1, 3, 0x123, 0xffff_ffff, any]);
                let at = p.sym(f);
                p.a.stub_return(at, v).unwrap();
                p.b.stub_return(at, v).unwrap();
            }
            let chr = rng.next();
            p.check("op_chara_hsv_size_get", 0x0080_7b20, &[chr], |m| {
                Some(o::op_chara_hsv_size_get(m, chr, &mut call_original))
            });
            let (mode, visible) = (rng.pick(&[0u32, 1, 2]), rng.below(2));
            p.check(
                "op_chara_packet_size_get",
                0x0080_37c0,
                &[chr, mode, visible],
                |m| {
                    Some(o::op_chara_packet_size_get(
                        m,
                        chr,
                        mode,
                        visible,
                        &mut call_original,
                    ))
                },
            );
            // pppKeOfsPt and pppKeAcmCic
            p.set(offsets, 4 * rng.below(0x40));
            p.fill(key, 0x40, &mut rng);
            for i in 0..3 {
                let (a, b) = (finite(&mut rng), finite(&mut rng));
                p.set(key + 0x10 + 4 * i, a);
                // (no signalling NaNs: the copies go through the FPU)
                let c = finite(&mut rng);
                p.set(key + 0x20 + 4 * i, c);
                let off: u32 = p.get(offsets);
                p.set(obj + off + 0xa0 + 4 * i, b);
            }
            let frame = rng.below(3);
            p.set(obj + 0xc, frame);
            p.set(key, if rng.chance(80) { frame } else { 7 });
            p.set(key + 0x2c, rng.pick(&[0xffffu32, 0, 3]));
            p.set(stop, u32::from(rng.chance(10)));
            p.check("pppKeOfsPt", 0x0075_cd30, &[obj, key, ctrl], |m| {
                pb::ppp_ke_ofs_pt(m, obj, key, ctrl, &mut call_original);
                None
            });
            p.set(key + 4, rng.pick(&[0u8, 2, 3]));
            p.check("pppKeAcmCic", 0x0075_b690, &[obj, key], |m| {
                pb::ppp_ke_acm_cic(m, key, &mut call_original);
                None
            });
        }
    }
}

/// MSVCR110's `_CIsqrt` and `_CIatan2` (the oracle has no stubs): stand-ins
/// in both oracles through their import slots (`fsqrt`, `fpatan`; the
/// game's CRT computes them the same way at 53 bits), and a thunk that runs
/// the `_CIatan2` stand-in on two `f64` in memory, for the bridges'
/// callback. Returns the thunk's (y, x, out).
fn x87_crt_stand_ins(p: &mut Pair) -> (u32, u32, u32) {
    let sqrt = p.sym(0x00b0_bfe0);
    let atan2 = p.sym(0x00b0_bff0);
    for (at, code) in [(sqrt, [0xd9u8, 0xfa, 0xc3]), (atan2, [0xd9, 0xf3, 0xc3])] {
        p.a.write(at, &code).unwrap();
        p.b.write(at, &code).unwrap();
    }
    let (s1, s2) = (p.sym(0x00b0_c39c), p.sym(0x00b0_c3b0));
    p.set(s1, sqrt);
    p.set(s2, atan2);
    let data = p.alloc(0x18);
    let (y, x, out) = (data, data + 8, data + 0x10);
    let at = p.sym(ATAN2_THUNK);
    let mut c = vec![0xdd, 0x05]; // fld qword [y]
    c.extend(y.to_le_bytes());
    c.extend([0xdd, 0x05]); // fld qword [x]
    c.extend(x.to_le_bytes());
    c.push(0xe8); // call atan2
    c.extend(atan2.wrapping_sub(at + 17).to_le_bytes());
    c.extend([0xdd, 0x1d]); // fstp qword [out]
    c.extend(out.to_le_bytes());
    c.push(0xc3);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    (y, x, out)
}

const ATAN2_THUNK: u32 = 0x00b0_bfa0;

/// The bridges' `atan2` callback in the second oracle: the stand-in.
fn oracle_atan2(io: (u32, u32, u32)) -> impl FnMut(&mut Oracle, f64, f64) -> f64 {
    move |m, y, x| {
        Mem::write(m, io.0, y);
        Mem::write(m, io.1, x);
        let at = m.rebase(ATAN2_THUNK);
        m.call(at, Conv::Cdecl, &[]).unwrap();
        let r = Mem::read(m, io.2);
        // the scratch back to zero, as in the first oracle
        Mem::write(m, io.0, [0u32; 6]);
        r
    }
}

/// A thunk calling `sym(args)` (cdecl) and storing its `f32` result
/// (`ST0`) at `out`, at [`THUNK`] in both oracles.
fn float_call(p: &mut Pair, sym: u32, args: &[u32], out: u32) -> u32 {
    let at = p.sym(THUNK);
    let target = p.sym(sym);
    let mut c = Vec::new();
    for &a in args.iter().rev() {
        c.push(0x68); // push imm32
        c.extend(a.to_le_bytes());
    }
    c.push(0xe8);
    let next = at + u32::try_from(c.len()).unwrap() + 4;
    c.extend(target.wrapping_sub(next).to_le_bytes());
    c.extend([0x83, 0xc4, u8::try_from(4 * args.len()).unwrap()]); // add esp
    c.extend([0xd9, 0x1d]); // fstp dword [out]
    c.extend(out.to_le_bytes());
    c.push(0xc3);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    at
}

/// Runs the original in `a` and `rust` in `b`, then checks the result and
/// the memory, except the emulated `a0` (`0x00c8f548`), which holds a
/// stack address in the original and a frame address in the bridge.
fn check_but_a0(
    p: &mut Pair,
    what: &str,
    sym: u32,
    args: &[u32],
    rust: impl FnOnce(&mut Oracle) -> Option<u32>,
) {
    let at = p.sym(sym);
    let r =
        p.a.call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{}: {what}: {e:?}", p.name));
    let mine = rust(&mut p.b);
    if let Some(m) = mine {
        assert_eq!(r.eax, m, "{}: {what}: result", p.name);
    }
    let a0 = p.sym(0x00c8_f548);
    let v: u32 = Mem::read(&p.a, a0);
    Mem::write(&mut p.b, a0, v);
    p.same(what);
}

#[test]
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::cast_precision_loss
)]
fn sound_positions_match_the_original() {
    use op::bridge as o;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x3d50_0e7d);
        let io = x87_crt_stand_ins(&mut p);
        arg_recorder(&mut p, &[0x0088_7760], 0); // SndStopBattleStream
        arg_recorder(&mut p, &[0x0088_6e50, 0x0088_75a0], 3);
        arg_recorder(&mut p, &[0x0088_6df0, 0x0088_6f60], 2);
        arg_recorder(&mut p, &[0x0088_6680, 0x0088_7990], 1);
        // (0x00887970 is too close to 0x00887990 for a recorder: a stub; the
        // pan it gets is op_3d_vol_pan_get's, checked above)
        let pan = p.sym(0x0088_7970);
        p.a.stub_return(pan, 0).unwrap();
        p.b.stub_return(pan, 0).unwrap();
        let (dst, m, v, out) = (p.alloc(0x10), p.alloc(0x40), p.alloc(0x10), p.alloc(0x10));
        let listener = p.sym(o::LISTENER.sym);
        let curves = p.sym(0x023c_3668);
        let cue = p.alloc(0x10);
        let rec = p.alloc(0x10);
        let small = |rng: &mut Xorshift| -> f32 { (rng.below(2000) as f32 - 1000.0) / 7.0 };
        for _ in 0..3000 {
            for i in 0..16 {
                let x = finite(&mut rng);
                p.set(m + 4 * i, x);
                let l = small(&mut rng);
                p.set(listener + 4 * i, l);
            }
            for i in 0..4 {
                let x = finite(&mut rng);
                p.set(v + 4 * i, x);
            }
            p.check("op_apply_m_w1", 0x007e_79a0, &[dst, m, v], |mm| {
                o::op_apply_m_w1(mm, dst, m, v);
                None
            });
            let t = float_call(&mut p, 0x007e_7fc0, &[v, m], out);
            p.check_f32("op_length_get", t, out, |mm| o::op_length_get(mm, v, m));
            let q: [f32; 4] = std::array::from_fn(|_| small(&mut rng));
            let len = if rng.chance(50) { out + 4 } else { 0 };
            let args = [
                q[0].to_bits(),
                q[1].to_bits(),
                q[2].to_bits(),
                q[3].to_bits(),
                len,
            ];
            let t = float_call(&mut p, 0x007e_7ed0, &args, out);
            p.check_f32("op_deg_get", t, out, |mm| {
                o::op_deg_get(mm, q, len, &mut oracle_atan2(io))
            });
            // the curves: volume max/min, pan max/min, base, slopes
            let vals = [
                60.0 + small(&mut rng).abs(),
                small(&mut rng).abs() / 4.0,
                0.0,
                0.0,
                100.0 + small(&mut rng),
                0.0,
                120.0,
                8.0,
                127.0,
                -0.5 - small(&mut rng).abs() / 100.0,
                0.0,
                0.06 + small(&mut rng).abs() / 1000.0,
            ];
            for (i, x) in (0u32..).zip(vals) {
                p.set(curves + 4 * i, x);
            }
            for i in 0..4 {
                let x = small(&mut rng) * 4.0;
                p.set(v + 4 * i, x);
            }
            let vol = rng.pick(&[0u32, 0x80, 0x100, 0xff, 0x1000]);
            check_but_a0(
                &mut p,
                "op_3d_vol_pan_get",
                0x007f_fa90,
                &[v, vol, out],
                |mm| Some(o::op_3d_vol_pan_get(mm, v, vol, out, &mut oracle_atan2(io))),
            );
            p.set(cue, rng.pick(&[0u8, 1, 2]));
            p.set(cue + 8, rec);
            p.set(rec + 8, rng.pick(&[0u32, 5, u32::MAX]));
            for s in [0x023c_3520, 0x023c_3524] {
                let at = p.sym(s);
                p.set(at, if rng.chance(50) { cue } else { 0 });
            }
            let pos = if rng.chance(20) { 0 } else { v };
            check_but_a0(
                &mut p,
                "op_oef2_se_3d_on",
                0x007f_fd20,
                &[cue, pos, vol],
                |mm| {
                    Some(o::op_oef2_se_3d_on(
                        mm,
                        cue,
                        pos,
                        vol,
                        &mut call_original,
                        &mut oracle_atan2(io),
                    ))
                },
            );
            check_but_a0(
                &mut p,
                "op_oef2_se_3d_move",
                0x007f_fc60,
                &[cue, pos, vol],
                |mm| {
                    Some(o::op_oef2_se_3d_move(
                        mm,
                        cue,
                        pos,
                        vol,
                        &mut call_original,
                        &mut oracle_atan2(io),
                    ))
                },
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round7_small_functions_match_the_original() {
    use op::bridge as o;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0007_5a11);
        let (a, b) = (p.alloc(0x40), p.alloc(0x40));
        let nodes = p.alloc(0x20 * 0x10);
        for _ in 0..1000 {
            for (i, &g) in fp::bridge::STATS.iter().enumerate() {
                let v = rng.next();
                p.set(p.sym(g), v);
                let at = [
                    0x0072_93e0,
                    0x0072_93f0,
                    0x0072_93d0,
                    0x0072_92d0,
                    0x0072_9280,
                ][i];
                p.check("pppFpGet*", at, &[], |m| {
                    Some(fp::bridge::ppp_fp_stat(m, i))
                });
            }
            for (i, at) in [0x007b_2b40u32, 0x007b_2b50, 0x007b_2b60]
                .into_iter()
                .enumerate()
            {
                let v = rng.next();
                p.check("Ms set", at, &[v], |m| {
                    o::set_word(m, i, v);
                    Some(v)
                });
            }
            let v = rng.next();
            p.check("op_tex_vdisp_load_triger", 0x007e_5210, &[v], |m| {
                o::op_tex_vdisp_load_triger(m, v);
                None
            });
            p.fill(a, 0x40, &mut rng);
            for at in [0x0075_e4b0, 0x007e_6c60] {
                p.check("op_word_copy", at, &[b, a + 4], |m| {
                    o::op_word_copy(m, b, a + 4);
                    None
                });
            }
            let (w, s) = (finite(&mut rng), finite(&mut rng));
            p.set(a + 0xc, w);
            p.check("op_vec_w_scale", 0x007e_cec0, &[b, a, s], |m| {
                o::op_vec_w_scale(m, b, a, f32::from_bits(s));
                None
            });
            let n = rng.next();
            p.check("op_add64", 0x0081_7c30, &[b + 8, a, n], |m| {
                o::op_add64(m, b + 8, a, n);
                None
            });
            let k = rng.below(16);
            p.check("op_sum_words", 0x0076_b1e0, &[a, k], |m| {
                Some(o::op_sum_words(m, a, k))
            });
            let handles = p.sym(et::bridge::CHR_HANDLES.sym);
            p.fill(handles, 0x3c0, &mut rng);
            p.check("opus_chr_handles_clear", 0x007f_edf0, &[], |m| {
                et::bridge::opus_chr_handles_clear(m);
                None
            });
            let (mons, weps) = (
                p.sym(opus::bridge::MONSTERS.sym),
                p.sym(opus::bridge::WEAPONS.sym),
            );
            for i in 0..8 {
                p.set(mons + 4 * i, rng.pick(&[0u32, 0x100, 0x200]));
            }
            for i in 0..4 {
                p.set(weps + 4 * i, rng.pick(&[0u32, 0x100, 0x200]));
            }
            let obj = rng.pick(&[0u32, 0x100, 0x200, 0x300]);
            p.check("opus_monster_index", 0x007f_e540, &[obj], |m| {
                Some(opus::bridge::opus_monster_index(m, obj).cast_unsigned())
            });
            p.check("opus_weapon_index", 0x007f_e570, &[obj], |m| {
                Some(opus::bridge::opus_get_weapon_index(m, obj).cast_unsigned())
            });
            // the batch of 6-byte-record helpers
            let group = rng.pick(&[0u32, 1, 2, 0x8001]);
            for (at, off) in [(0x0072_9400u32, true), (0x0072_9440, false)] {
                p.check("pppFpGroupDisp", at, &[group], |m| {
                    if off {
                        fp::bridge::ppp_fp_group_disp_off(m, group);
                    } else {
                        fp::bridge::ppp_fp_group_disp_on(m, group);
                    }
                    None
                });
            }
            for i in 0..8 {
                let v = finite(&mut rng);
                p.set(a + 4 * i, v);
            }
            let sc: [u32; 3] = std::array::from_fn(|_| finite(&mut rng));
            p.check(
                "vfx_vec3_scale3",
                0x0072_8a60,
                &[b, a, sc[0], sc[1], sc[2]],
                |m| {
                    o::vfx_vec3_scale3(m, b, a, sc.map(f32::from_bits));
                    None
                },
            );
            p.check(
                "opus_vec3_scale",
                0x0080_8cb0,
                &[b, a, 0, 0, 0, sc[1]],
                |m| {
                    o::opus_vec3_scale(m, b, a, f32::from_bits(sc[1]));
                    None
                },
            );
            p.check("pppVec4Copy", 0x0075_0020, &[b + 0x10, a], |m| {
                o::ppp_vec4_copy(m, b + 0x10, a);
                None
            });
            p.put(ppp::bridge::COLOR_CON_SHIFT, 2 * rng.below(8));
            p.check("op_color_shift_load16", 0x007e_ca60, &[b, a], |m| {
                o::op_color_shift_load16(m, b, a);
                None
            });
            p.fill(a, 0x40, &mut rng);
            let pos = rng.next();
            p.check("op_oef2_clut_change_monster", 0x007e_45e0, &[a, pos], |m| {
                Some(o::op_oef2_clut_change_monster(m, a, pos))
            });
            p.check("op_oef2_tex_pos_set", 0x007e_4ff0, &[a + 0x10, pos], |m| {
                Some(o::op_oef2_tex_pos_set(m, a + 0x10, pos))
            });
            let flags = p.sym(opus::bridge::MONSTER_FLAGS.sym);
            for i in 0..8 {
                p.set(flags + i, rng.pick(&[0u8, 1, 2, 0x81]));
            }
            let slot = rng.below(8);
            p.check("opus_monster_clut_pos", 0x0080_0e20, &[slot], |m| {
                Some(opus::bridge::opus_monster_clut_pos(m, slot))
            });
            p.check("opus_monster_tex_pos", 0x0080_0e50, &[slot], |m| {
                Some(opus::bridge::opus_monster_tex_pos(m, slot))
            });
            let objs = p.sym(et::bridge::OPUS_BATTLE.sym);
            for i in 0..4 {
                p.set(objs + 4 * i, b);
            }
            p.set(b + 0x20, nodes);
            let (i, n) = (rng.below(4) | (rng.below(2) << 16), rng.below(0x10));
            p.check("opus_osu_init_group", 0x0080_1480, &[i, n], |m| {
                opus::bridge::opus_osu_init_group(m, i, n);
                None
            });
            // chains of nodes: left at +0, right at +8, nil byte +0xd
            for i in 0..0x10u32 {
                let n = nodes + 0x20 * i;
                p.set(n, nodes + 0x20 * ((i + 1) % 0x10));
                p.set(n + 8, nodes + 0x20 * ((i + 15) % 0x10));
                p.set(n + 0xd, u8::from(i == 0 || rng.chance(20)));
            }
            let n = nodes + 0x20 * (1 + rng.below(0xf));
            p.check("opusTree_Min", 0x0080_6db0, &[n], |m| {
                Some(tree::bridge::opus_tree_min(m, n))
            });
            p.check("opusTree_Max", 0x0080_6d70, &[n], |m| {
                Some(tree::bridge::opus_tree_max(m, n))
            });
        }
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::items_after_statements
)]
fn former_hidden_slots_match_the_original() {
    use slots::bridge as s;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0051_07e5);
        // the other modules' functions: recorded with their arguments
        for (syms, n) in [
            (
                &[0x0093_0770u32, 0x0092_5530, 0x0092_4c70, 0x007b_2c10][..],
                1u8,
            ),
            // opu_debug_reload's: the frees, op_oef2_set2, the loader, op_oef2_set
            (&[0x007f_c5f0, 0x007f_c650, 0x007f_ca90, 0x007e_3e30][..], 1),
            (&[0x007e_5c10, 0x007e_3b70][..], 2),
            (
                &[
                    0x0088_7570,
                    0x0079_d090,
                    0x0088_6680,
                    0x0088_6eb0,
                    0x0088_6ba0,
                    0x0088_5ee0,
                ][..],
                1,
            ),
            (
                &[
                    0x0088_7630,
                    0x0088_64b0,
                    0x0079_d050,
                    0x008e_89d0,
                    0x008a_dbd0,
                ][..],
                0,
            ),
            (
                &[
                    0x0078_7950,
                    0x008a_f4f0,
                    0x008a_e070,
                    0x008a_f1b0,
                    0x008a_f0d0,
                ][..],
                2,
            ),
            (&[0x008a_e410][..], 3),
        ] {
            arg_recorder(&mut p, syms, n);
        }
        // op_oef2_sound_cue_loop: a stand-in that clears the stream cues and
        // takes one queued cue
        let (c1, c2, count) = (p.sym(0x023c_3648), p.sym(0x023c_364c), p.sym(0x023c_3642));
        let mut c = vec![0x31, 0xc0]; // xor eax, eax
        for at in [c1, c2] {
            c.push(0xa3); // mov [at], eax
            c.extend(at.to_le_bytes());
        }
        c.extend([0x66, 0x39, 0x05]); // cmp [count], ax
        c.extend(count.to_le_bytes());
        c.extend([0x74, 0x07]); // je +7
        c.extend([0x66, 0xff, 0x0d]); // dec word [count]
        c.extend(count.to_le_bytes());
        c.push(0xc3);
        let lp = p.sym(0x0080_0170);
        p.a.write(lp, &c).unwrap();
        p.b.write(lp, &c).unwrap();
        // an environment for rcefChrAlloc
        let env_slot = p.alloc(4);
        let env = p.alloc(0x40);
        let arena = p.alloc(0x10000);
        p.put(heap::bridge::ENV_SLOT, env_slot);
        p.set(env_slot, env);
        for (h, m) in [(env, arena), (env + 0x10, arena + 0x8000)] {
            p.check("pppCreateHeap", 0x0072_c480, &[h, m, 0x8000], |mm| {
                heap::bridge::ppp_create_heap(mm, h, m, 0x8000);
                None
            });
        }
        let out = p.alloc(4);
        for round in 0..400 {
            let v = rng.next();
            p.set(p.sym(s::ET_FLAGS.sym), v as u8);
            for (at, bit, on) in [
                (0x007f_c990u32, 1u8, true),
                (0x007f_c9a0, 1, false),
                (0x007f_c900, 2, true),
                (0x007f_c910, 2, false),
            ] {
                if rng.chance(50) {
                    p.check("op_et_flag", at, &[], |m| {
                        s::op_et_flag(m, bit, on);
                        None
                    });
                }
            }
            p.set(p.sym(s::LIBRA_STATE.sym), rng.next() as u16);
            p.check("TOBtlLibraGetState", 0x0089_7770, &[], |m| {
                Some(s::to_btl_libra_get_state(m))
            });
            p.check("TOBtlLibraSetState6", 0x0089_0ec0, &[], |m| {
                Some(s::to_btl_libra_set_state6(m))
            });
            for (at, g) in [(0x0092_49a0u32, 0x025d_5ea4u32), (0x0089_7780, 0x0133_f790)] {
                let x = finite(&mut rng);
                p.set(p.sym(g), x);
                let t = float_call(&mut p, at, &[], out);
                p.check_f32("get_float", t, out, |m| s::get_float(m, g));
            }
            p.set(p.sym(0x0112_be90), rng.next());
            p.check("MsGetBattle112be90", 0x007b_7f20, &[], |m| {
                Some(s::ms_get_battle_112be90(m))
            });
            let list = p.sym(0x025d_5bbc);
            p.set(list, rng.below(0x20) as i32 - 2);
            p.fill(list + 4, 0x80, &mut rng);
            p.check("rcefReverseList", 0x0092_e7a0, &[], |m| {
                s::rcef_reverse_list(m);
                None
            });
            for (at, f) in [(0x0092_e770u32, 0x0092_5530u32), (0x0092_5130, 0x0092_4c70)] {
                p.check("rcefWorkCall", at, &[], |m| {
                    Some(s::rcef_work_call(m, f, &mut call_original))
                });
            }
            p.set(p.sym(0x025d_5bb8), rng.next());
            p.check("rcefEffectEndMotion", 0x0092_df90, &[], |m| {
                Some(s::rcef_effect_end_motion(m, &mut call_original))
            });
            if round % 8 == 0 {
                p.check("rcefChrAlloc", 0x0093_0800, &[], |m| {
                    Some(s::rcef_chr_alloc(m, &mut call_original))
                });
            }
            p.set(p.sym(0x0112_c8f0), rng.pick(&[0u32, 0, 5]));
            p.set(p.sym(0x0112_c8f4), rng.pick(&[u32::MAX, 3]));
            p.check("msVoiceCheckStanbyOn", 0x007b_2b70, &[], |m| {
                Some(s::ms_voice_check_stanby_on(m, &mut call_original))
            });
            p.check("msVoiceCheckStanbyOff", 0x007b_2ba0, &[], |m| {
                Some(s::ms_voice_check_stanby_off(m, &mut call_original))
            });
            p.check("msVoiceIsStreaming", 0x007b_2be0, &[], |m| {
                Some(s::ms_voice_is_streaming(m, &mut call_original))
            });
            p.check("TOSepStop4e", 0x0089_0e70, &[], |m| {
                Some(s::to_sep_stop_4e(m, &mut call_original))
            });
            p.set(p.sym(s::BWAKK_ON.sym), rng.pick(&[0u8, 1, 2]));
            type To = fn(&mut Oracle, &mut dyn FnMut(&mut Oracle, u32, &[u32]) -> u32) -> u32;
            let tos: [(&str, u32, To); 6] = [
                ("TOBwakkStop", 0x0089_0f10, |m, c| {
                    s::to_bwakk_stop(m, &mut |m, f, a| c(m, f, a))
                }),
                ("TOBwakkCheckTerm", 0x0089_77c0, |m, c| {
                    s::to_bwakk_check_term(m, &mut |m, f, a| c(m, f, a))
                }),
                ("TOBgoodCheckTerm", 0x0089_77e0, |m, c| {
                    s::to_bgood_check_term(m, &mut |m, f, a| c(m, f, a))
                }),
                ("TODum225GetNum", 0x0089_8230, |m, c| {
                    s::to_dum225_get_num(m, &mut |m, f, a| c(m, f, a))
                }),
                ("TOBwakkStart", 0x0089_8dc0, |m, c| {
                    s::to_bwakk_start(m, &mut |m, f, a| c(m, f, a))
                }),
                ("TOBgoodStart", 0x0089_ab80, |m, c| {
                    s::to_bgood_start(m, &mut |m, f, a| c(m, f, a))
                }),
            ];
            let (name, at, f) = rng.pick(&tos);
            p.check(name, at, &[], |m| Some(f(m, &mut call_original)));
            // opu_debug_reload over recorded loaders
            p.set(p.sym(s::DEBUG_MODE.sym), u8::from(rng.chance(80)));
            p.set(p.sym(s::DEBUG_RELOAD.sym), rng.next() as u8);
            let table = p.sym(0x0232_15c0);
            for i in 0..12 {
                p.set(table + 4 * i, rng.pick(&[0u32, 0x100, 0x200]));
            }
            p.check("opu_debug_reload", 0x007f_dc80, &[], |m| {
                s::opu_debug_reload(m, &mut call_original);
                None
            });
            // op_oef2_sound_wait with cues pending and queued
            p.set(c1, rng.pick(&[0u32, u32::MAX, 0x1234]));
            p.set(c2, rng.pick(&[0u32, u32::MAX, 0x1234]));
            p.set(count, rng.below(4) as u16);
            p.check("op_oef2_sound_wait", 0x0080_0330, &[], |m| {
                Some(op::bridge::op_oef2_sound_wait(m, &mut call_original))
            });
        }
    }
}

/// A recorder like [`arg_recorder`] that, for argument `k` with
/// `words[k]` not 0, logs that many words it points at instead of the
/// pointer (16 words per entry); returns the log.
fn deref_recorder(p: &mut Pair, sym: u32, words: &[u8]) -> u32 {
    deref_recorder_with(p, sym, words, None)
}

/// [`deref_recorder`] that returns the dword at `ret` (when given)
/// instead of the count.
fn deref_recorder_with(p: &mut Pair, sym: u32, words: &[u8], ret: Option<u32>) -> u32 {
    let log = p.alloc(4 + 0x100 * 64);
    let mut c = vec![0x8b, 0x0d]; // mov ecx, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x81, 0xe1, 0xff, 0, 0, 0]); // and ecx, 0xff
    c.extend([0xc1, 0xe1, 0x06]); // shl ecx, 6
    let mut pos = 0u32;
    for (k, &n) in (0u8..).zip(words) {
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp + 4 + 4k]
        if n == 0 {
            c.extend([0x89, 0x81]); // mov [ecx + slot], eax
            c.extend((log + 4 + 4 * pos).to_le_bytes());
            pos += 1;
        } else {
            for j in 0..n {
                c.extend([0x8b, 0x50, 4 * j]); // mov edx, [eax + 4j]
                c.extend([0x89, 0x91]); // mov [ecx + slot], edx
                c.extend((log + 4 + 4 * pos).to_le_bytes());
                pos += 1;
            }
        }
    }
    assert!(pos <= 15);
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.push(0xa1); // mov eax, [count] or [ret]
    c.extend(ret.unwrap_or(log).to_le_bytes());
    c.push(0xc3);
    let at = p.sym(sym);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    log
}

/// [`deref_recorder_with`] with 32-word entries (31 logged words), for
/// requests too large for the 16-word form.
fn wide_recorder(p: &mut Pair, sym: u32, words: &[u8], ret: Option<u32>) -> u32 {
    let log = p.alloc(4 + 0x100 * 128);
    let mut c = vec![0x8b, 0x0d]; // mov ecx, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x81, 0xe1, 0xff, 0, 0, 0]); // and ecx, 0xff
    c.extend([0xc1, 0xe1, 0x07]); // shl ecx, 7
    let mut pos = 0u32;
    for (k, &n) in (0u8..).zip(words) {
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp + 4 + 4k]
        if n == 0 {
            c.extend([0x89, 0x81]); // mov [ecx + slot], eax
            c.extend((log + 4 + 4 * pos).to_le_bytes());
            pos += 1;
        } else {
            for j in 0..n {
                c.extend([0x8b, 0x50, 4 * j]); // mov edx, [eax + 4j]
                c.extend([0x89, 0x91]); // mov [ecx + slot], edx
                c.extend((log + 4 + 4 * pos).to_le_bytes());
                pos += 1;
            }
        }
    }
    assert!(pos <= 31);
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.push(0xa1); // mov eax, [count] or [ret]
    c.extend(ret.unwrap_or(log).to_le_bytes());
    c.push(0xc3);
    let at = p.sym(sym);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    log
}

#[test]
#[allow(clippy::too_many_lines)]
fn opp_draw_part_matches_the_original() {
    use slots::bridge as s;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a7_4d30);
        // every callee recorded; the rectangle and the two corner vectors
        // by their contents (the original keeps them on its stack, the
        // bridge at the same addresses in the second oracle's stack)
        for (sym, words) in [
            (0x0090_8420u32, &[0u8, 0][..]),
            (0x007e_a700, &[0, 0]),
            (0x007e_9850, &[0, 0, 0, 0]),
            (0x0080_1f00, &[0, 0]),
            (0x007e_83b0, &[0, 0, 0]),
            (0x00a6_9300, &[0, 0, 0]),
            (0x007f_f1d0, &[0, 0]),
            (0x007f_f780, &[0, 0]),
            (0x0078_2ae0, &[0]),
            (0x007e_a850, &[0, 0, 0]),
            (0x007e_a550, &[0, 0, 0]),
            (0x007e_4540, &[4, 0]),
            (0x0064_2fe0, &[0, 0, 4, 4, 0]),
            (0x007e_4420, &[4, 0]),
            (0x0064_38c0, &[0]),
        ] {
            deref_recorder(&mut p, sym, words);
        }
        let magic = p.sym(0x009d_a3b0);
        let end = p.b.stack_range().end;
        // the original's frame pointer for a two-argument call
        let frame = (end - 0x100 - 8 - 4 - 12) & !15;
        let obj = p.alloc(0x400);
        let rec = p.alloc(0x100);
        let cmd = p.alloc(0x40);
        for _ in 0..2000 {
            p.fill(obj, 0x400, &mut rng);
            p.fill(rec, 0x100, &mut rng);
            p.fill(cmd, 0x40, &mut rng);
            p.set(obj + 0x220, rec);
            let kind = rng.pick(&[0u16, 0x1000, 0x2000, 0x3000, 0x4000, 0x2800]);
            let low: u16 = p.get(obj + 0x21a);
            p.set(obj + 0x21a, (low & 0x0fff) | kind);
            for at in [0x3c, 0x40, 0x48] {
                let x = finite(&mut rng);
                p.set(rec + at, x);
            }
            p.set(cmd + 2, u16::try_from(8 + 2 * rng.below(8)).unwrap());
            let id = rng.pick(&[0xaau32, 0x25f, 7]);
            p.a.stub_return(magic, id).unwrap();
            p.b.stub_return(magic, id).unwrap();
            p.check("opp_draw_part_a74d30", 0x00a7_4d30, &[obj, cmd], |m| {
                Some(s::opp_draw_part(m, obj, cmd, frame, &mut call_original))
            });
        }
    }
}

#[test]
fn libra_rects_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0089_7dc0);
        arg_recorder(&mut p, &[0x008a_e450], 4);
        deref_recorder(&mut p, 0x008a_e4b0, &[0, 0, 0, 0, 0, 0]);
        let block = p.sym(0x0133_f6a4);
        for _ in 0..50 {
            p.fill(block, 0xd4, &mut rng);
            p.check("TOBtlLibraGetRects", 0x0089_7dc0, &[], |m| {
                Some(slots::bridge::to_btl_libra_get_rects(m, &mut call_original))
            });
        }
    }
}

/// [`Pair::check`] with a calling convention.
fn check_conv(
    p: &mut Pair,
    what: &str,
    sym: u32,
    conv: Conv,
    args: &[u32],
    rust: impl FnOnce(&mut Oracle) -> Option<u32>,
) {
    let at = p.sym(sym);
    let r =
        p.a.call(at, conv, args)
            .unwrap_or_else(|e| panic!("{}: {what}: {e:?}", p.name));
    assert!(
        r.st0.is_none(),
        "{}: {what}: the original returns {:?} in ST0",
        p.name,
        r.st0
    );
    if let Some(m) = rust(&mut p.b) {
        assert_eq!(r.eax, m, "{}: {what}: result", p.name);
    }
    p.same(what);
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn small_op_helpers_match_the_original() {
    use small::bridge as s;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0007_e3a0);
        let (a, b, c) = (p.alloc(0x100), p.alloc(0x100), p.alloc(0x100));
        let out = p.alloc(4);
        let eff = p.sym(et::bridge::EFF.sym);
        for _ in 0..300 {
            p.fill(a, 0x100, &mut rng);
            p.fill(b, 0x100, &mut rng);
            p.fill(c, 0x100, &mut rng);
            // a string of a few letters, quotes and a terminator at b
            for i in 0..12 {
                let ch = rng.pick(&[b'a', b'\'', b'"', b'Z', 0xc1, b'q']);
                p.set(b + i, ch);
            }
            p.set(b + 12, 0u8);
            let v = rng.next();
            p.check("op_char_table_set", 0x007e_13e0, &[a + 0x80, b, v], |m| {
                s::op_char_table_set(m, a + 0x80, b, v);
                None
            });
            p.check("op_str_unquote", 0x007e_1b50, &[b], |m| {
                s::op_str_unquote(m, b);
                None
            });
            for (at, or) in [(0x007e_3040u32, false), (0x007e_6c80, true)] {
                p.check("op_logic64", at, &[c, a, a + 8], |m| {
                    s::op_logic64(m, c, a, a + 8, or);
                    None
                });
            }
            for at in [0x007e_3060u32, 0x007e_6c70] {
                p.check("op_word_copy", at, &[c + 0x10, a + 4], |m| {
                    op::bridge::op_word_copy(m, c + 0x10, a + 4);
                    None
                });
            }
            for i in 0..16 {
                let x = finite(&mut rng);
                p.set(a + 4 * i, x);
            }
            let sb = finite(&mut rng);
            let sf = f32::from_bits(sb);
            for (at, off, op) in [
                (0x007e_3070u32, 0u32, '*'),
                (0x0080_2080, 0, '*'),
                (0x007e_5950, 0, '+'),
                (0x007e_59a0, 0, '-'),
                (0x007e_31c0, 0xc, '-'),
                (0x007e_cea0, 8, '*'),
            ] {
                p.check("op_float_step", at, &[c, a, sb], |m| {
                    s::op_float_step(m, c, a, sf, off, op);
                    None
                });
            }
            p.check("op_vec_w_add2", 0x007e_cc00, &[c, a, 0, 0, 0, sb], |m| {
                s::op_float_step(m, c, a, sf, 0xc, '+');
                None
            });
            let z = finite(&mut rng);
            p.check("op_vec_xz_add", 0x007e_6ca0, &[c, a, sb, 0, z], |m| {
                s::op_vec_xz_add(m, c, a, sf, f32::from_bits(z));
                None
            });
            let w = rng.next();
            p.check("op_or16_64", 0x007e_30a0, &[c, a, w], |m| {
                s::op_or16_64(m, c, a, w);
                None
            });
            let (x, y) = (rng.next(), rng.next());
            p.check("op_mask16_64", 0x007e_ca00, &[c, x, y], |m| {
                s::op_mask16_64(m, c, x, y);
                None
            });
            p.check("op_add64_2", 0x007e_ca20, &[c, a, x], |m| {
                op::bridge::op_add64(m, c, a, x);
                None
            });
            for (at, half, load) in [
                (0x007e_30c0u32, false, false),
                (0x007e_aa60, true, false),
                (0x007e_cac0, false, true),
            ] {
                p.put(ppp::bridge::COLOR_CON_SHIFT, 2 * rng.below(8));
                p.check("op_color_shift", at, &[c + 0x40, b + 0x40], |m| {
                    s::op_color_shift(m, c + 0x40, b + 0x40, half, load);
                    None
                });
            }
            p.put(ppp::bridge::COLOR_CON_SHIFT, 2 * rng.below(8));
            p.check("op_color_shift_load16_2", 0x007e_ca90, &[c, b], |m| {
                op::bridge::op_color_shift_load16(m, c, b);
                None
            });
            let n = rng.pick(&[0u32, 1, 5, 31, 32, 33, 0xffff_ffff, 0x8000_0000]);
            p.check("op_pow2", 0x007e_31e0, &[n], |_| Some(s::op_pow2(n)));
            // (above 2^30 the original never ends)
            let n2 = rng.pick(&[0u32, 1, 2, 3, 100, 0x4000_0000, 0xffff_fff0]);
            p.check("op_log2_ceil", 0x007e_aa90, &[n2], |_| {
                Some(s::op_log2_ceil(n2))
            });
            let fx = rng.pick(&[
                sb,
                sb | 0x8000_0000,
                0,
                0x8000_0000,
                0x7fc0_0000,
                0xffc0_0001,
            ]);
            let t = float_call(&mut p, 0x007e_3360, &[fx], out);
            p.check_f32("op_fabs", t, out, |_| s::op_fabs(f32::from_bits(fx)));
            for i in 0..4 {
                p.set(c + 0x80 + 4 * i, 4 * rng.below(8));
            }
            let (i, j) = (32 + rng.below(4), rng.below(8));
            p.check("op_table2_get", 0x007e_37b0, &[c, i, j], |m| {
                Some(s::op_table2_get(m, c, i, j))
            });
            p.check("op_vec_splat", 0x007e_37f0, &[c, sb], |m| {
                s::op_vec_splat(m, c, sf);
                None
            });
            p.check("op_vec4_move", 0x007e_3f60, &[c, b], |m| {
                s::op_vec4_move(m, c, b);
                None
            });
            p.check("op_oef2_tex_pos_replace", 0x007e_4fc0, &[c, w], |m| {
                Some(s::op_oef2_tex_pos_replace(m, c, w))
            });
            p.check("op_vec_zero", 0x007e_5660, &[c + 0x20], |m| {
                s::op_vec_zero(m, c + 0x20);
                None
            });
            p.check("op_unit_m", 0x007e_a6d0, &[c + 0x40], |m| {
                s::op_unit_m(m, c + 0x40);
                None
            });
            p.check("op_word_copy_z", 0x007e_ce80, &[c, b], |m| {
                s::op_word_copy_at(m, c, b, 8);
                None
            });
            p.check("op_spr_cmd_size", 0x007f_5ed0, &[b], |m| {
                Some(s::op_spr_cmd_size(m, b))
            });
            for (at, g) in [(0x007f_e430u32, 0x0119_fe90u32), (0x007f_e4c0, 0x0119_fe98)] {
                let gv = rng.pick(&[0u32, 7]);
                p.set(p.sym(g), gv);
                let xv = rng.pick(&[0u32, 7]);
                p.check("op_is_not", at, &[xv], |m| Some(s::op_is_not(m, xv, g)));
            }
            for k in 0..16 {
                p.set(c + 0x60 + 4 * k, rng.pick(&[1u32, 2]));
            }
            p.check("opus_draw_slot_clear", 0x007f_e690, &[c, 2], |m| {
                s::opus_draw_slot_clear(m, c, 2);
                None
            });
            let id = rng.pick(&[0x55u32, 0x66, 0x77]);
            p.set(eff + 0x20 * rng.below(512), 0x66u32);
            p.check("op_et_eff_index", 0x007f_ed60, &[id], |m| {
                Some(s::op_et_eff_index(m, id).cast_unsigned())
            });
            p.set(b + 8, c);
            p.check("op_oef2_sep_number_get", 0x007f_ff80, &[b], |m| {
                Some(s::op_oef2_sep_number_get(m, b))
            });
            p.set(b + 0xc, a);
            p.check("opus_part_kind", 0x0080_0410, &[b], |m| {
                Some(s::opus_part_kind(m, b))
            });
            for at in [
                0x0080_6450u32,
                0x0080_6460,
                0x0080_6470,
                0x0080_6480,
                0x0080_6610,
                0x0080_6620,
            ] {
                p.check("opusIdentity", at, &[v], |_| Some(v));
            }
            let dst = rng.pick(&[0u32, c + 0x90]);
            for at in [0x0080_6490u32, 0x0080_64b0, 0x0080_64f0, 0x0080_6510] {
                check_conv(&mut p, "opusCopyIf", at, Conv::Stdcall, &[dst, a], |m| {
                    s::copy_if(m, dst, a);
                    None
                });
            }
            for at in [0x0080_64d0u32, 0x0080_6530] {
                p.check("opusCopyIf3", at, &[0, dst, a + 4], |m| {
                    s::copy_if(m, dst, a + 4);
                    None
                });
            }
            for at in [
                0x0080_6730u32,
                0x0080_6740,
                0x0080_6750,
                0x0080_6760,
                0x0080_6770,
                0x0080_6780,
            ] {
                check_conv(&mut p, "opusSetThis", at, Conv::Thiscall(c), &[v, 0], |m| {
                    Some(s::set_this(m, c, v))
                });
            }
            p.set(a, rng.pick(&[1u32, 2]));
            p.set(b, rng.pick(&[1u32, 2]));
            for (at, ne) in [
                (0x0080_6970u32, false),
                (0x0080_6990, false),
                (0x0080_69b0, true),
                (0x0080_69d0, true),
            ] {
                check_conv(&mut p, "opusIter", at, Conv::Thiscall(a), &[b], |m| {
                    Some(s::iter_eq(m, a, b, ne))
                });
            }
            p.check("opusPlus0c", 0x0080_6b50, &[v], |_| {
                Some(v.wrapping_add(0xc))
            });
        }
    }
}

/// Runs the original two-argument `sym` in `a` with 0x200 more bytes of
/// stack (through a trampoline at [`THUNK`], so its locals lie below the
/// stack the oracle gives the bridge's own calls) and `rust` in `b` with
/// the original's frame pointer (for a `push ebp; mov ebp, esp` frame);
/// checks the result and the memory.
fn check_deep(
    p: &mut Pair,
    what: &str,
    sym: u32,
    args: [u32; 2],
    rust: impl FnOnce(&mut Oracle, u32) -> u32,
) {
    let at = p.sym(THUNK);
    let mut c = vec![0x81, 0xec, 0, 2, 0, 0]; // sub esp, 0x200
    for _ in 0..2 {
        c.extend([0xff, 0xb4, 0x24, 0x08, 0x02, 0, 0]); // push [esp + 0x208]
    }
    c.push(0xe8); // call sym
    let call_end = at + u32::try_from(c.len()).unwrap() + 4;
    c.extend(p.sym(sym).wrapping_sub(call_end).to_le_bytes());
    c.extend([0x81, 0xc4, 0x08, 0x02, 0, 0]); // add esp, 0x208
    c.push(0xc3);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    // the trampoline's entry: 0x100 below the stack's end, the arguments
    // and the return address; then 0x200, two arguments, a return address
    // and the saved ebp
    let frame = p.b.stack_range().end - 0x100 - 8 - 4 - 0x200 - 8 - 4 - 4;
    let r =
        p.a.call(at, Conv::Cdecl, &args)
            .unwrap_or_else(|e| panic!("{}: {what}: {e}", p.name));
    assert!(
        r.st0.is_none(),
        "{}: {what}: the original returns {:?} in ST0",
        p.name,
        r.st0
    );
    let mine = rust(&mut p.b, frame);
    assert_eq!(r.eax, mine, "{}: {what}: result", p.name);
    p.same(what);
}

/// A `dbgPrintf` stand-in at `0x0062f500` that logs the format and, for
/// the format at `two` (one with a double), the two words after it;
/// returns the log (16-byte entries after the count).
fn dbg_printf_recorder(p: &mut Pair, two: u32) -> u32 {
    let log = p.alloc(4 + 0x100 * 16);
    let mut c = vec![0xa1]; // mov eax, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x25, 0xff, 0, 0, 0]); // and eax, 0xff
    c.extend([0xc1, 0xe0, 0x04]); // shl eax, 4
    c.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp + 4]
    c.extend([0x89, 0x88]); // mov [eax + slot], ecx
    c.extend((log + 4).to_le_bytes());
    c.extend([0x81, 0xf9]); // cmp ecx, two
    c.extend(two.to_le_bytes());
    c.extend([0x75, 0x14]); // jne +20
    for k in 1..3u8 {
        c.extend([0x8b, 0x4c, 0x24, 4 + 4 * k]); // mov ecx, [esp + 4 + 4k]
        c.extend([0x89, 0x88]); // mov [eax + slot], ecx
        c.extend((log + 4 + 4 * u32::from(k)).to_le_bytes());
    }
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.push(0xc3);
    let at = p.sym(0x0062_f500);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    log
}

#[test]
#[allow(clippy::too_many_lines)]
fn round9_handlers_match_the_original() {
    use slots::bridge as s;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a7_4a00);
        let unit = p.alloc(0x100);
        let entry = p.alloc(0x40);
        let buf = p.alloc(0x100);
        // what the buffer, oef2-entry, unit and mesh-break stand-ins return
        let cells = p.alloc(16);
        p.set(cells, buf);
        p.set(cells + 4, entry);
        p.set(cells + 8, unit);
        for (sym, words, ret) in [
            (0x00a7_6770u32, &[0u8, 0][..], None),
            (0x007f_f780, &[0, 0], Some(cells)),
            (0x00a7_53f0, &[0, 4], None),
            (0x00a7_5170, &[4, 0, 0, 0, 0], None),
            (0x007e_3810, &[0, 0, 0], Some(cells + 4)),
            (0x00a7_44e0, &[0, 0, 0], None),
            (0x0080_0d60, &[0, 0], None),
            (0x00a7_3c20, &[0, 0, 0, 0], Some(cells + 12)),
            (0x0064_5780, &[0, 0], None),
            (0x0080_1060, &[0, 0, 0], Some(cells + 8)),
            (0x007e_5260, &[0, 0, 0], None),
            (0x007e_a700, &[0, 0], None),
            (0x0080_1f00, &[0, 0], None),
            (0x0063_df50, &[], None),
            (0x0064_2fa0, &[], None),
            // the 10 copied bytes at frame - 0x10: two whole words
            (0x007f_65d0, &[0, 0, 0, 0, 2, 0, 0, 0], None),
            (0x0063_d810, &[0, 0], None),
            (0x0079_66a0, &[0, 0], None),
            (0x0063_a2b0, &[0, 0, 0], None),
            (0x0082_6a80, &[0, 0], None),
            (0x007e_79a0, &[0, 0, 3], None),
            (0x0063_d880, &[0, 0, 1], None),
            (0x007e_a730, &[0, 0], None),
            (0x007e_9970, &[0, 0, 0, 0], None),
            (0x007e_a850, &[0, 0, 0], None),
            (0x007e_a6d0, &[0], None),
            // stands in for the unit list's handler (obj +0x350, +0x64)
            (0x00a7_0910, &[0, 0, 0, 0], None),
        ] {
            deref_recorder_with(&mut p, sym, words, ret);
        }
        // 0x0063d870 has 16 bytes before 0x0063d880: a call counter
        let restored = p.alloc(4);
        let mut c = vec![0xff, 0x05];
        c.extend(restored.to_le_bytes());
        c.push(0xc3);
        let at = p.sym(0x0063_d870);
        p.a.write(at, &c).unwrap();
        p.b.write(at, &c).unwrap();
        let two = p.sym(0x00b7_c520);
        dbg_printf_recorder(&mut p, two);
        let chain = p.sym(0x0078_8dd0);
        let handler = p.sym(0x00a7_0910);
        let wave_off = p.sym(0x012f_b7d0);
        let obj = p.alloc(0x800);
        let rec = p.alloc(0x200);
        let cmd = p.alloc(0x200);
        let t = p.alloc(0x40);
        let pp = p.alloc(0x80);
        let pts = p.alloc(0x20);
        let vt = p.alloc(0x80);
        for _ in 0..1500 {
            for (at, len) in [(obj, 0x800), (rec, 0x200), (cmd, 0x200), (unit, 0x100)] {
                p.fill(at, len, &mut rng);
            }
            p.fill(entry, 0x40, &mut rng);
            p.fill(pts, 0x20, &mut rng);
            p.set(obj + 0x220, rec);
            let kind = rng.pick(&[0u16, 0x1000, 0x2000, 0x3000, 0x4000, 0x0800]);
            let low: u16 = p.get(obj + 0x21a);
            p.set(obj + 0x21a, (low & 0x0fff) | kind);
            let tv = if rng.chance(15) { 0 } else { t };
            p.set(obj + 0x3b4, tv);
            p.set(t, if rng.chance(15) { 0 } else { pp });
            p.set(t + 8, pts);
            p.set(t + 0x14, pts);
            p.set(t + 0x20, pts + 4 * rng.below(3));
            for i in 0..3 {
                let x = finite(&mut rng);
                p.set(pts + 4 * i, x);
            }
            let scl = if rng.chance(30) { 0 } else { finite(&mut rng) };
            p.set(pp + 0x60, scl);
            for off in [2u32, 4, 6] {
                let v = if rng.chance(20) {
                    0
                } else {
                    4 * rng.below(0x7c)
                };
                p.set(cmd + off, u16::try_from(v).unwrap());
            }
            for at in (0xd0..0xf0).step_by(4) {
                let x = finite(&mut rng);
                p.set(rec + at, x);
            }
            let v = finite(&mut rng);
            p.set(unit + 0x44, v);
            p.set(unit + 0xbb, rng.pick(&[3u8, 0x82, 0]));
            p.set(entry + 0x10, u16::try_from(rng.below(0x100)).unwrap());
            p.set(obj + 0x350, vt);
            p.set(vt + 0x64, handler);
            p.set(cells + 12, rng.pick(&[0u32, 1, 0x8000_0000]));
            p.set(wave_off, u32::from(rng.chance(30)));
            let on = rng.pick(&[0u32, 1]);
            p.a.stub_return(chain, on).unwrap();
            p.b.stub_return(chain, on).unwrap();
            p.check("opp_part_a76720", 0x00a7_6720, &[obj, cmd], |m| {
                Some(s::opp_part_a76720(m, obj, cmd, &mut call_original))
            });
            p.check("opp_map_draw_a74c60", 0x00a7_4c60, &[obj, cmd], |m| {
                Some(s::opp_map_draw_a74c60(m, obj, cmd, &mut call_original))
            });
            p.check("opp_md_break_init_a74410", 0x00a7_4410, &[obj, cmd], |m| {
                Some(s::opp_md_break_init_a74410(m, obj, cmd, &mut call_original))
            });
            check_deep(
                &mut p,
                "opus_wave_cmd_7f5f70",
                0x007f_5f70,
                [obj, cmd],
                |m, frame| s::opus_wave_cmd_7f5f70(m, obj, cmd, frame, &mut call_original),
            );
            check_deep(
                &mut p,
                "opp_chr_center_a756e0",
                0x00a7_56e0,
                [obj, cmd],
                |m, frame| s::opp_chr_center_a756e0(m, obj, cmd, frame, &mut call_original),
            );
            p.check("opu2_part", 0x00a7_4a00, &[obj, cmd], |m| {
                Some(s::opu2_part(m, obj, cmd, &mut call_original))
            });
        }
    }
}

#[test]
fn fade_centres_match_the_original() {
    use slots::bridge as s;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a7_6b60);
        let (unit, set, u, bone) = (
            p.alloc(0x100),
            p.alloc(0x100),
            p.alloc(0x100),
            p.alloc(0x40),
        );
        let entry = p.alloc(0x400);
        // what the unit, bone-set, entry, unit-by-name and bone-record
        // stand-ins return
        let cells = p.alloc(20);
        for (k, v) in [unit, set, entry, u, bone].into_iter().enumerate() {
            p.set(cells + 4 * u32::try_from(k).unwrap(), v);
        }
        for (sym, words, ret) in [
            (0x0080_1060u32, &[0u8, 0, 0][..], Some(cells)),
            (0x0080_e160, &[0, 0, 0, 0, 0], Some(cells + 4)),
            (0x0080_e900, &[0, 0, 0, 0, 0], None),
            (0x0080_1f00, &[0, 0], None),
            (0x007e_83b0, &[0, 0, 0], None),
            (0x0080_62c0, &[0, 0, 0, 0, 0, 0], None),
            (0x00a4_4650, &[0], None),
            (0x0083_3f80, &[0, 0], None),
            (0x006e_d460, &[0, 0], None),
            (0x006e_d2e0, &[4, 0, 0], None),
            (0x0063_b5d0, &[0, 0, 0], None),
            (0x0080_b520, &[0, 0, 0, 0, 0], Some(cells + 12)),
            (0x007e_3810, &[0, 0, 0], Some(cells + 8)),
            (0x0080_3200, &[0, 0], Some(cells + 16)),
            (0x007e_79a0, &[0, 0, 0], None),
            (0x007e_a850, &[0, 0, 0], None),
        ] {
            deref_recorder_with(&mut p, sym, words, ret);
        }
        let obj = p.alloc(0x800);
        let rec = p.alloc(0x200);
        let cmd = p.alloc(0x200);
        for _ in 0..1000 {
            for (at, len) in [(obj, 0x800), (rec, 0x200), (cmd, 0x200), (unit, 0x100)] {
                p.fill(at, len, &mut rng);
            }
            p.fill(bone, 0x40, &mut rng);
            p.set(obj + 0x220, rec);
            let kind = rng.pick(&[0x1000u16, 0x2000, 0x2000, 0x3000]);
            let low: u16 = p.get(obj + 0x21a);
            p.set(obj + 0x21a, (low & 0x0fff) | kind);
            p.set(set + 4, u16::try_from(rng.below(4)).unwrap());
            for k in 0..3 {
                let x = finite(&mut rng);
                p.set(unit + 0x90 + 4 * k, x);
            }
            p.set(obj + 0x212, u16::from(rng.chance(20)));
            p.set(
                rec + 0xa8,
                u16::from(!rng.chance(20)) * u16::try_from(rng.below(0x40)).unwrap(),
            );
            p.set(rec + 0xaa, u16::try_from(rng.below(0x30)).unwrap());
            p.set(cells + 12, if rng.chance(20) { 0 } else { u });
            p.set(cmd + 2, u16::try_from(rng.below(0x100)).unwrap());
            check_deep(
                &mut p,
                "opp_chr_fade_center_a76b60",
                0x00a7_6b60,
                [obj, cmd],
                |m, frame| s::opp_chr_fade_center_a76b60(m, obj, cmd, frame, &mut call_original),
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn optic_laser_matches_the_original() {
    use slots::bridge as s;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a7_3810);
        let (unit, entry, rd, geo) = (
            p.alloc(0x100),
            p.alloc(0x100),
            p.alloc(0x100),
            p.alloc(0x40),
        );
        // what the unit, entry, render-data and buffer stand-ins return
        let cells = p.alloc(16);
        p.set(cells, unit);
        p.set(cells + 4, entry);
        for (sym, words, ret) in [
            (0x0064_53d0u32, &[0u8, 0][..], Some(cells + 8)),
            (0x0080_1060, &[0, 0, 0], Some(cells)),
            (0x007e_a120, &[0, 0, 0, 0, 0], None),
            (0x007e_a700, &[0, 0], None),
            (0x007e_9850, &[0, 0, 0, 0], None),
            (0x007e_3810, &[0, 0, 0], Some(cells + 4)),
            (0x00a4_4700, &[0], None),
            (0x0071_2c30, &[0], None),
            // the set_ptr32 result through the local
            (0x0071_13d0, &[0, 0, 1, 0, 0], None),
            (0x0071_3290, &[0, 0], None),
            (0x0063_fdb0, &[0, 0, 0, 0], Some(cells + 12)),
            (0x0094_964c, &[0, 0, 0], None),
            (0x0080_1f00, &[0, 0], None),
            (0x006e_d2e0, &[0, 0, 4], None),
            (0x0071_a4e0, &[0, 0, 0, 0], None),
            (0x007e_a850, &[0, 0, 0], None),
            (0x0064_58a0, &[0, 0], None),
        ] {
            deref_recorder_with(&mut p, sym, words, ret);
        }
        let off = p.sym(0x012f_b7d0);
        let obj = p.alloc(0x300);
        let rec = p.alloc(0x100);
        let list = p.alloc(0x500);
        let data = p.alloc(0x10);
        // with no points the original reads the one before the first
        let pts = p.alloc(0x110) + 0x10;
        for _ in 0..1500 {
            for (at, len) in [(obj, 0x300), (rec, 0x100), (list, 0x500), (unit, 0x100)] {
                p.fill(at, len, &mut rng);
            }
            p.fill(entry, 0x100, &mut rng);
            p.fill(data, 0x10, &mut rng);
            for i in 0..0x44 {
                let x = finite(&mut rng);
                p.set(pts - 0x10 + 4 * i, x);
            }
            p.set(obj + 0x220, rec);
            p.set(rec + 0xa8, data);
            p.set(unit + 0xbc, unit);
            p.set(rec + 0xbc, unit);
            p.set(data, u16::from(rng.chance(70)));
            p.set(list + 0x28, pts);
            let n = rng.pick(&[-1.5f32, 0.0, 0.7, 1.0, 2.9, 4.0, 5.5, f32::NAN]);
            p.set(rec + 0x40, n);
            for at in [0x48, 0x3c, 0x94] {
                let x = finite(&mut rng);
                p.set(rec + at, x);
            }
            p.set(list + 0x20, if rng.chance(15) { unit } else { 0 });
            // the shape at entry + 0x20, its byte table 0x40 further
            p.set(entry + 0x10, 0x20u16);
            p.set(entry + 0x24, rng.below(0x40));
            p.set(rd + 0x94, geo);
            p.set(cells + 8, if rng.chance(30) { 0 } else { rd });
            p.set(cells + 12, u32::from(!rng.chance(15)));
            p.set(off, u32::from(rng.chance(25)));
            check_deep(
                &mut p,
                "opp2_optic_laser",
                0x00a7_3810,
                [obj, list],
                |m, frame| s::opp2_optic_laser(m, obj, list, frame, &mut call_original),
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn md_break_matches_the_original() {
    use slots::bridge as s;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a7_3d90);
        let (spr, rd, geo) = (p.alloc(0x40), p.alloc(0x100), p.alloc(0x40));
        let bufs = [
            p.alloc(0x100),
            p.alloc(0x100),
            p.alloc(0x100),
            p.alloc(0x100),
        ];
        // what the scratch, render-data and buffer stand-ins return
        let cells = p.alloc(12);
        p.set(cells, spr);
        for (sym, words, ret) in [
            (0x007e_a700u32, &[0u8, 0][..], None),
            (0x007e_9850, &[0, 0, 0, 0], None),
            (0x0080_1f00, &[0, 0], None),
            (0x007e_83b0, &[0, 0, 0], None),
            (0x007e_6a00, &[0], Some(cells)),
            (0x007e_9530, &[0, 0, 0], None),
            (0x0064_53d0, &[0, 0], Some(cells + 4)),
            (0x0063_fdb0, &[0, 0, 0, 0], Some(cells + 8)),
            (0x006e_d360, &[0, 0], None),
            (0x0094_964c, &[0, 0, 0], None),
            (0x006e_d9a0, &[0], None),
            (0x0064_5710, &[0, 0, 0], None),
            (0x0064_58a0, &[0, 0], None),
            (0x0080_0d10, &[0, 0], None),
            (0x0064_5280, &[0, 0], None),
            (0x007e_6a60, &[0], None),
        ] {
            deref_recorder_with(&mut p, sym, words, ret);
        }
        // FFXVu0ApplyMatrix(out, m, v) as out = v, so the corners the
        // original reads back from its stack are known
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp + 4]
        c.extend([0x8b, 0x4c, 0x24, 0x0c]); // mov ecx, [esp + 12]
        for k in 0..4u8 {
            c.extend([0x8b, 0x51, 4 * k]); // mov edx, [ecx + 4k]
            c.extend([0x89, 0x50, 4 * k]); // mov [eax + 4k], edx
        }
        c.push(0xc3);
        let at = p.sym(0x006e_d2e0);
        p.a.write(at, &c).unwrap();
        p.b.write(at, &c).unwrap();
        let obj = p.alloc(0x300);
        let rec = p.alloc(0x100);
        let list = p.alloc(0x100);
        let data = p.alloc(0x10);
        let d = p.alloc(0x520);
        for _ in 0..1500 {
            for (at, len) in [(obj, 0x300), (rec, 0x100), (list, 0x100), (data, 0x10)] {
                p.fill(at, len, &mut rng);
            }
            for i in 0..0x520 / 4 {
                let x = finite(&mut rng);
                p.set(d + 4 * i, x);
            }
            p.set(obj + 0x220, rec);
            p.set(rec + 0xbc, d);
            p.set(rec + 0xa8, data);
            let x = finite(&mut rng);
            p.set(rec + 0x3c, x);
            let n = rng.pick(&[0i32, 1, 2, 3, 5, -1]);
            p.set(d + 4, n);
            p.set(d + 0x12, rng.pick(&[0u16, 1, 2]));
            for k in 0..5 {
                let pb = d + 0x20 + 0x100 * k;
                let a = rng.pick(&[0i32, 1, -5, 4096, 0x1234_5678, -0x7fff_ffff]);
                p.set(pb + 0xb0, a);
            }
            p.set(obj + 0x212, u16::from(rng.chance(30)));
            p.set(rd + 0x94, geo);
            p.set(geo + 0x18, bufs[2]);
            p.set(geo + 0x1c, bufs[3]);
            p.set(geo + 0x14, bufs[1]);
            p.set(geo + 0xc, bufs[0]);
            p.set(cells + 4, if rng.chance(20) { 0 } else { rd });
            p.set(cells + 8, u32::from(!rng.chance(15)));
            check_deep(
                &mut p,
                "opp2_md_break",
                0x00a7_3d90,
                [obj, list],
                |m, frame| s::opp2_md_break(m, obj, list, frame, &mut call_original),
            );
        }
    }
}

#[test]
fn opus_0582_commands_match_the_original() {
    use slots::bridge as s;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a7_0910);
        let (unit, buf) = (p.alloc(0x100), p.alloc(0x400));
        // what the unit, buffer and size stand-ins return
        let cells = p.alloc(12);
        p.set(cells, unit);
        p.set(cells + 4, buf);
        for (sym, words, ret) in [
            (0x0080_1060u32, &[0u8, 0, 0][..], Some(cells)),
            (0x007f_f780, &[0, 0], Some(cells + 4)),
            (0x007e_a730, &[0, 3], None),
            (0x007e_a550, &[0, 0, 0], None),
            (0x0080_1f00, &[0, 0], None),
            (0x007e_83b0, &[0, 0, 0], None),
            (0x00a7_2b80, &[0, 0, 0, 0], None),
            (0x00a7_3740, &[0, 0], None),
            (0x0083_13c0, &[0, 0, 0], None),
            // 0x00831390 is 0x30 bytes before 0x008313c0: two words
            (0x0083_1390, &[0, 0], None),
            (0x0080_3220, &[0, 0, 0, 0], None),
            (0x00a7_2310, &[0], Some(cells + 8)),
            (0x0080_62c0, &[0, 0, 0, 0, 0, 0], None),
            (0x00a7_1780, &[0, 0, 0], None),
            (0x0083_1300, &[0, 0], None),
            (0x0083_11d0, &[0, 0], None),
            (0x00a7_0d70, &[0, 0, 0, 0, 0, 0], None),
            (0x007e_a6d0, &[0], None),
            (0x00a7_2380, &[0, 0, 0, 0, 0, 0, 0], None),
            (0x0082_cbe0, &[0], None),
        ] {
            deref_recorder_with(&mut p, sym, words, ret);
        }
        let obj = p.alloc(0x400);
        let rec = p.alloc(0x100);
        let cmd = p.alloc(0x10);
        for _ in 0..2000 {
            for (at, len) in [(obj, 0x400), (rec, 0x100), (cmd, 0x10), (unit, 0x100)] {
                p.fill(at, len, &mut rng);
            }
            p.set(obj + 0x220, rec);
            let kind = u16::try_from(rng.below(10)).unwrap() << 12;
            let low: u16 = p.get(obj + 0x21a);
            p.set(obj + 0x21a, (low & 0x0fff) | kind);
            let x = finite(&mut rng);
            p.set(rec + 0x3c, x);
            // kind 0x6000 clears every entry of the new buffer: few of them
            p.set(cmd + 2, rng.pick(&[0i16, 1, 7, 3]));
            p.set(obj + 0x212, u16::from(rng.chance(30)));
            p.set(cells + 8, rng.below(0x100));
            p.check("opp_0582_cmd_a70910", 0x00a7_0910, &[obj, cmd], |m| {
                Some(s::opp_0582_cmd_a70910(m, obj, cmd, &mut call_original))
            });
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn opus_0437_commands_match_the_original() {
    use slots::bridge as s;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_dbe0);
        let (unit, buf, entry, set, mt) = (
            p.alloc(0x100),
            p.alloc(0x100),
            p.alloc(0x40),
            p.alloc(0x100),
            p.alloc(0x40),
        );
        // what the unit, buffer, entry, size, bone-set and matrix
        // stand-ins return
        let cells = p.alloc(24);
        for (k, v) in [unit, buf, entry, 0, set, mt].into_iter().enumerate() {
            p.set(cells + 4 * u32::try_from(k).unwrap(), v);
        }
        for (sym, words, ret) in [
            (0x0080_1060u32, &[0u8, 0, 0][..], Some(cells)),
            (0x007f_f780, &[0, 0], Some(cells + 4)),
            (0x007e_3810, &[0, 0, 0], Some(cells + 8)),
            (0x00a6_a2b0, &[0], Some(cells + 12)),
            (0x0080_e160, &[0, 0, 0, 0, 0], Some(cells + 16)),
            (0x00a4_4650, &[0], Some(cells + 20)),
            (0x00a6_e540, &[0, 0], None),
            (0x007e_a850, &[0, 0, 0], None),
            (0x00a6_e5d0, &[0, 0, 0], None),
            (0x0080_1f00, &[0, 0], None),
            (0x007e_8010, &[0, 0], None),
            (0x00a6_e2e0, &[0, 0], None),
            (0x0080_62c0, &[0, 0, 0, 0, 0, 0], None),
            (0x00a6_e1b0, &[0, 0, 0, 0], None),
            (0x00a6_ed40, &[0, 0, 0, 0, 0, 0], None),
            (0x00a6_e260, &[0], None),
            (0x0094_964c, &[0, 0, 0], None),
            (0x0080_e900, &[0, 0, 0, 0, 0], None),
            (0x007e_83b0, &[0, 0, 0], None),
            (0x00a6_e650, &[0, 0, 0, 0, 0, 0, 0], None),
        ] {
            deref_recorder_with(&mut p, sym, words, ret);
        }
        let obj = p.alloc(0x400);
        let rec = p.alloc(0x100);
        let cmd = p.alloc(0x10);
        for _ in 0..1500 {
            for (at, len) in [
                (obj, 0x400),
                (rec, 0x100),
                (cmd, 0x10),
                (unit, 0x100),
                (buf, 0x100),
            ] {
                p.fill(at, len, &mut rng);
            }
            for i in 0..0x10 {
                let x = finite(&mut rng);
                p.set(mt + 4 * i, x);
            }
            for at in [0x34, 0x3c] {
                let x = finite(&mut rng);
                p.set(rec + at, x);
            }
            for at in [0x14, 0x24] {
                let x = finite(&mut rng);
                p.set(buf + at, x);
            }
            let x = finite(&mut rng);
            p.set(unit + 0x44, x);
            p.set(obj + 0x220, rec);
            p.set(rec + 0xbc, buf);
            let kind = u16::try_from(rng.below(8)).unwrap() << 12;
            let low: u16 = p.get(obj + 0x21a);
            p.set(obj + 0x21a, (low & 0x0fff) | kind);
            p.set(set + 4, u16::try_from(rng.below(4)).unwrap());
            p.set(cells + 12, rng.below(0x400));
            check_deep(
                &mut p,
                "opp_0437_cmd_a6dbe0",
                0x00a6_dbe0,
                [obj, cmd],
                |m, frame| s::opp_0437_cmd_a6dbe0(m, obj, cmd, frame, &mut call_original),
            );
        }
    }
}

/// Where a [`copy_recorder`] copies from: an argument or a fixed block.
#[derive(Clone, Copy)]
enum From {
    Arg(u8),
    At(u32),
}

/// A stand-in at `sym` that logs its first `nargs` arguments (16 per
/// entry) and copies `words` dwords from `from` to argument `to`, so the
/// locals an original reads back are known; returns the log.
fn copy_recorder(p: &mut Pair, sym: u32, nargs: u8, to: u8, from: From, words: u8) -> u32 {
    let log = p.alloc(4 + 0x100 * 64);
    let mut c = vec![0x8b, 0x0d]; // mov ecx, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x81, 0xe1, 0xff, 0, 0, 0]); // and ecx, 0xff
    c.extend([0xc1, 0xe1, 0x06]); // shl ecx, 6
    for k in 0..nargs {
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp + 4 + 4k]
        c.extend([0x89, 0x81]); // mov [ecx + slot], eax
        c.extend((log + 4 + 4 * u32::from(k)).to_le_bytes());
    }
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.extend([0x8b, 0x44, 0x24, 4 + 4 * to]); // mov eax, [esp + 4 + 4 to]
    match from {
        From::Arg(k) => c.extend([0x8b, 0x4c, 0x24, 4 + 4 * k]), // mov ecx, [esp + 4 + 4k]
        From::At(at) => {
            c.push(0xb9); // mov ecx, at
            c.extend(at.to_le_bytes());
        }
    }
    for k in 0..words {
        c.extend([0x8b, 0x51, 4 * k]); // mov edx, [ecx + 4k]
        c.extend([0x89, 0x50, 4 * k]); // mov [eax + 4k], edx
    }
    c.push(0xc3);
    let at = p.sym(sym);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    log
}

#[test]
#[allow(clippy::too_many_lines)]
fn mdc_draws_match_the_original() {
    use slots::bridge as s;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_f4d0);
        let (unit, set, mat, texrec) =
            (p.alloc(0x100), p.alloc(0x100), p.alloc(0x40), p.alloc(0x10));
        // what the unit, bone-set, matrix and texture-loaded stand-ins
        // return
        let cells = p.alloc(16);
        for (k, v) in [unit, set, mat, 1].into_iter().enumerate() {
            p.set(cells + 4 * u32::try_from(k).unwrap(), v);
        }
        for (sym, words, ret) in [
            (0x0080_1060u32, &[0u8, 0, 0][..], Some(cells)),
            (0x0080_e160, &[0, 0, 0, 0, 0], Some(cells + 4)),
            (0x0080_1f00, &[0, 0], Some(cells + 8)),
            (0x00a4_4650, &[0], Some(cells + 8)),
            (0x0064_5450, &[0, 0, 0, 0, 0, 0, 0], Some(cells + 12)),
            (0x0080_62c0, &[0, 0, 0, 0, 0, 0], None),
            (0x0082_6b70, &[0], None),
            // the colour sums through their local
            (0x0064_3770, &[0, 0, 0, 0, 4, 0, 0, 0, 0, 0], None),
            (0x0064_39b0, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0], None),
            (0x0063_b810, &[0, 0, 0], None),
            (0x00a6_8e10, &[0, 0, 0, 0, 0, 0, 0, 0], None),
            (0x007e_8010, &[0, 0], None),
            (0x0064_2b10, &[0, 0], None),
            (0x00a7_0210, &[0, 0, 0, 0, 0], None),
            (0x0080_e900, &[0, 0, 0, 0, 0], None),
        ] {
            deref_recorder_with(&mut p, sym, words, ret);
        }
        // op_mul_m(out, a, b) as out = b; the texture record from texrec
        copy_recorder(&mut p, 0x007e_83b0, 3, 0, From::Arg(2), 16);
        copy_recorder(&mut p, 0x0080_8a90, 3, 2, From::At(texrec), 4);
        let magic = p.sym(0x009d_a3b0);
        let off = p.sym(0x012f_b7d0);
        let (count_158, late_2c6) = (p.sym(0x022f_b3f4), p.sym(0x022f_b3f8));
        let obj = p.alloc(0x400);
        let rec = p.alloc(0x100);
        let cmd = p.alloc(0x10);
        for _ in 0..2000 {
            for (at, len) in [
                (obj, 0x400),
                (rec, 0x100),
                (cmd, 0x10),
                (unit, 0x100),
                (texrec, 0x10),
            ] {
                p.fill(at, len, &mut rng);
            }
            for i in 0..0x10 {
                let x = finite(&mut rng);
                p.set(mat + 4 * i, x);
            }
            for at in [0x30, 0x34, 0x3c, 0x4c, 0x90, 0x94] {
                let x = finite(&mut rng);
                p.set(unit + at, x);
            }
            for at in [0x3c, 0x4c] {
                let x = finite(&mut rng);
                p.set(rec + at, x);
            }
            // alphas in the byte range too
            #[allow(clippy::cast_precision_loss)] // small
            for at in [unit + 0x4c, rec + 0x4c] {
                if rng.chance(50) {
                    p.set(at, rng.below(300) as f32 - 20.0);
                }
            }
            // texture sizes 16..4080 (a zero one faults the original)
            p.set(texrec + 10, u8::try_from(1 + rng.below(255)).unwrap());
            p.set(texrec + 11, u8::try_from(1 + rng.below(255)).unwrap());
            p.set(obj + 0x220, rec);
            let kind = rng.pick(&[0x1000u16, 0x2000, 0x3000]) | rng.pick(&[0u16, 0x800]);
            let low: u16 = p.get(obj + 0x21a);
            p.set(obj + 0x21a, (low & 0x07ff) | kind);
            p.set(set + 4, u16::try_from(rng.below(4)).unwrap());
            p.set(cells + 12, u32::from(!rng.chance(15)));
            p.set(off, u32::from(rng.chance(30)));
            p.set(count_158, rng.pick(&[0u32, 0x7b, 0x7c, 0x100]));
            p.set(late_2c6, rng.pick(&[0u32, 0x279, 0x27a, 0x300]));
            let id = rng.pick(&[
                0x1b6u32, 0x1b5, 0x158, 0x245, 0x276, 0x27a, 0x270, 0xa1, 0x26f, 0x272, 0x144,
                0x2c6, 0x273, 0x117, 0x204, 7,
            ]);
            p.a.stub_return(magic, id).unwrap();
            p.b.stub_return(magic, id).unwrap();
            check_deep(
                &mut p,
                "opp_mdc_draw_a6f4d0",
                0x00a6_f4d0,
                [obj, cmd],
                |m, frame| s::opp_mdc_draw_a6f4d0(m, obj, cmd, frame, &mut call_original),
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round10_helpers_match_the_original() {
    use vec::bridge as v;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0080_1060);
        // opus_no_get: a list with 16 parts, three index lists
        let list = p.alloc(0x40);
        let parts = p.alloc(0x1000);
        let idx = p.alloc(0x60);
        let table = p.sym(opus::bridge::NO_TABLE.sym);
        let (a, b, c) = (p.alloc(0x40), p.alloc(0x40), p.alloc(0x40));
        let status = p.sym(ffx_ps2::ee::bridge::STATUS.sym);
        for _ in 0..3000 {
            p.fill(parts, 0x1000, &mut rng);
            p.fill(list, 0x40, &mut rng);
            p.set(list + 0x20, parts);
            for (k, at) in [0x14u32, 0x10, 0x18].into_iter().enumerate() {
                let l = idx + 0x20 * u32::try_from(k).unwrap();
                p.set(list + at, l);
                let n = rng.below(6);
                for i in 0..n {
                    p.set(l + 2 * i, u16::try_from(rng.below(16)).unwrap());
                }
                p.set(l + 2 * n, rng.pick(&[0xffffu16, 0x8000]));
            }
            for i in 0..16 {
                let u = parts + 0x100 * i;
                p.set(u + 0x1a, u16::try_from(rng.below(8)).unwrap());
                p.set(u + 0x1c, u16::try_from(rng.below(16)).unwrap());
                p.set(u + 0x1e, u8::try_from(rng.below(2)).unwrap());
                p.set(u + 0x14, u16::try_from(rng.below(4)).unwrap());
                if rng.chance(20) {
                    p.set(u, 0u32);
                }
            }
            for i in 0..4 {
                p.set(table + 32 * i, parts + 0x100 * rng.below(16));
            }
            let unit = parts + 0x100 * rng.below(16);
            let (r1, r2) = (rng.below(8), rng.below(8) | 0x7fff_0000);
            let no = rng.pick(&[
                0xff00u32, 0xffff, 0xfffe, 0xfffd, 0xfe03, 0xfe0f, 0xb001, 0xbffc, 0xcfff, 0xd000,
                0xb000, r1, r2,
            ]);
            p.check("opus_no_get", 0x0080_1060, &[list, unit, no], |m| {
                Some(opus::bridge::opus_no_get(m, list, unit, no))
            });
            // op_mul_m, also in place
            for at in [a, b, c] {
                for i in 0..16 {
                    let x = rng.float();
                    p.set(at + 4 * i, x);
                }
            }
            let dst = rng.pick(&[c, a, b]);
            p.check("op_mul_m", 0x007e_83b0, &[dst, a, b], |m| {
                op::bridge::op_mul_m(m, dst, a, b);
                None
            });
            // the vector helpers
            let k = [rng.float(), rng.float(), rng.float(), rng.float()];
            let kf = k.map(f32::from_bits);
            p.set(status, rng.next());
            p.check(
                "Vec3AddScalars",
                0x0073_5100,
                &[c, a, k[0], k[1], k[2], k[3]],
                |m| Some(v::vec3_add_scalars(m, c, a, [kf[0], kf[1], kf[2]])),
            );
            p.check(
                "Vec3SubScalars",
                0x0074_b670,
                &[c, a, k[0], k[1], k[2]],
                |m| {
                    v::vec3_sub_scalars(m, c, a, [kf[0], kf[1], kf[2]]);
                    None
                },
            );
            if rng.chance(30) {
                p.set(a, rng.pick(&[0u32, 0x8000_0000]));
                p.set(a + 4, k[1]);
            }
            p.check(
                "Vec2SubScalarsFlags",
                0x0075_e870,
                &[c, a, k[0], k[1]],
                |m| {
                    v::vec2_sub_scalars_flags(m, c, a, kf[0], kf[1]);
                    None
                },
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round10b_helpers_match_the_original() {
    use fp::bridge as f;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007f_e970);
        let cells = p.alloc(16);
        for (sym, words, ret) in [
            (0x0082_a9b0u32, &[0u8][..], Some(cells)),
            (0x0082_61b0, &[0], Some(cells + 4)),
            (0x0088_6eb0, &[0], None),
            (0x0088_6d80, &[0], None),
            (0x00a6_49c0, &[], None),
            (0x006e_d9a0, &[0], None),
            (0x0062_f500, &[0], None),
        ] {
            deref_recorder_with(&mut p, sym, words, ret);
        }
        let (chr_tab, handles) = (
            p.sym(et::bridge::CHR.sym),
            p.sym(et::bridge::CHR_HANDLES.sym),
        );
        let debug = p.sym(op::bridge::DEBUG_MODE.sym);
        let seps = p.sym(op::bridge::OEF2_SEPS.sym);
        let (src, dst) = (p.alloc(0x20), p.alloc(0x20));
        let table = p.alloc(0x64 + 0x50 * 8);
        let obj = p.alloc(0x400);
        let units = p.alloc(0x1000);
        let idx = p.alloc(0x60);
        let heap = p.alloc(0x400) + 0x300;
        for round in 0..1500 {
            // opus_get_chr_status_index: a crowded or sparse record table
            for i in 0..240u32 {
                let full = rng.chance(if round % 3 == 0 { 99 } else { 30 });
                p.set(
                    chr_tab + 0x20 * i,
                    if full { 0x100 + rng.below(8) } else { 0 },
                );
                let h = if rng.chance(if round % 5 == 0 { 99 } else { 50 }) {
                    0x100 + rng.below(8)
                } else {
                    0
                };
                p.set(handles + 4 * i, h);
            }
            p.set(debug, u8::from(rng.chance(30)));
            p.put(et::bridge::DEBUG_CHR_FLAGS, rng.next().to_le_bytes()[0]);
            p.put(et::bridge::DEBUG_CHR_RESIDENT, rng.pick(&[0u32, 0x1234]));
            p.set(cells, rng.pick(&[0u32, 0x5678]));
            let some = 0x100 + rng.below(8);
            let chr = rng.pick(&[0u32, some, 0x200]);
            let (no, add) = (rng.next(), rng.pick(&[0u32, 1]));
            p.check(
                "opus_get_chr_status_index",
                0x007f_e970,
                &[chr, no, add],
                |m| {
                    Some(
                        et::bridge::opus_get_chr_status_index(m, chr, no, add, &mut call_original)
                            .cast_unsigned(),
                    )
                },
            );
            // op_xyz_scale
            for i in 0..4 {
                let x = rng.float();
                p.set(src + 4 * i, x);
            }
            let k = rng.float();
            p.check("op_xyz_scale", 0x007e_a850, &[dst, src, k], |m| {
                Some(op::bridge::op_xyz_scale(m, dst, src, k))
            });
            p.check("op_heap_display_clear", 0x007f_f260, &[], |m| {
                op::bridge::op_heap_display_clear(m, &mut call_original);
                None
            });
            // op_oef2_sep_free_all
            for i in 0..32u32 {
                p.set(seps + 8 * i, rng.next());
                p.set(seps + 8 * i + 4, 1u32 << rng.below(4) | 1 << rng.below(4));
            }
            let mask = 1u32 << rng.below(5);
            p.check("op_oef2_sep_free_all", 0x007f_ff30, &[mask], |m| {
                Some(op::bridge::op_oef2_sep_free_all(
                    m,
                    mask,
                    &mut call_original,
                ))
            });
            // pppFpSetStickChr, pppFpSetStickPos
            let n = rng.below(8);
            p.set(table + 4, u16::try_from(n).unwrap());
            for i in 0..n {
                p.set(table + 0x64 + 0x50 * i, rng.pick(&[0u16, 1, 2, 3]));
            }
            p.put(f::DATA, if rng.chance(90) { table } else { 0 });
            p.set(cells + 4, rng.pick(&[0u32, u32::MAX, 3]));
            let id = rng.pick(&[0u32, 1, 2, 3, 0x1_0001]);
            let (c, b) = (rng.next(), rng.next());
            p.check("pppFpSetStickChr", 0x0072_a530, &[id, c, b], |m| {
                Some(f::ppp_fp_set_stick_chr(m, id, c, b, &mut call_original))
            });
            p.check("pppFpSetStickPos", 0x0072_a630, &[id, src], |m| {
                f::ppp_fp_set_stick_pos(m, id, src, &mut call_original);
                None
            });
            // opus_parent_get
            p.fill(units, 0x1000, &mut rng);
            p.set(obj + 0x320, units);
            for (k, at) in [0x314u32, 0x310, 0x318].into_iter().enumerate() {
                let l = idx + 0x20 * u32::try_from(k).unwrap();
                p.set(obj + at, l);
                let n = rng.below(6);
                for i in 0..n {
                    p.set(l + 2 * i, u16::try_from(rng.below(16)).unwrap());
                }
                p.set(l + 2 * n, 0xffffu16);
            }
            for i in 0..16 {
                let u = units + 0x100 * i;
                p.set(u + 0x1a, u16::try_from(rng.below(8)).unwrap());
                p.set(u + 0x1c, u16::try_from(rng.below(16)).unwrap());
                p.set(u + 0x14, u16::try_from(rng.below(4)).unwrap());
            }
            let ntab = p.sym(opus::bridge::NO_TABLE.sym);
            for i in 0..4 {
                p.set(ntab + 32 * i, units + 0x100 * rng.below(16));
            }
            let unit = units + 0x100 * rng.below(16);
            let r = rng.below(8);
            p.set(
                unit + 0xb8,
                rng.pick(&[
                    0u16,
                    0xff00,
                    0xffff,
                    0xfffe,
                    0xfe05,
                    u16::try_from(r).unwrap(),
                ]),
            );
            p.check("opus_parent_get", 0x0080_1f00, &[obj, unit], |m| {
                Some(opus::bridge::opus_parent_get(m, obj, unit))
            });
            // op_free over a few linked records
            let top = rng.below(8);
            p.set(heap - 4, u16::try_from(top).unwrap());
            p.set(heap - 2, u16::try_from(rng.below(8)).unwrap());
            for i in 0..10u32 {
                let e = heap - 16 * i;
                p.set(e, 0x1000 + rng.below(6));
                let prev = u16::try_from(rng.below(top + 1)).unwrap();
                let next = u16::try_from(rng.below(top + 1)).unwrap();
                p.set(
                    e + 8,
                    if rng.chance(20) {
                        0u32
                    } else {
                        u32::from(prev) | u32::from(next) << 16
                    },
                );
            }
            let some = 0x1000 + rng.below(6);
            let blk = rng.pick(&[0u32, some]);
            p.check("op_free", 0x007f_f1d0, &[heap, blk], |m| {
                op::bridge::op_free(m, heap, blk, &mut call_original);
                None
            });
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn round10c_helpers_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0080_14c0);
        let (m, ang, t) = (p.alloc(0x40), p.alloc(0x10), p.alloc(0x10));
        for _ in 0..400 {
            for i in 0..16 {
                let x = finite(&mut rng);
                p.set(m + 4 * i, x);
            }
            for i in 0..3 {
                let a = finite(&mut rng);
                p.set(ang + 4 * i, a);
                let x = rng.float();
                p.set(t + 4 * i, x);
            }
            p.check("op_rot_m", 0x007e_9530, &[m, m, ang], |o| {
                op::bridge::op_rot_m(o, m, m, ang);
                None
            });
            p.check("op_rt_m", 0x007e_9850, &[m, m, ang, t], |o| {
                op::bridge::op_rt_m_zyx(o, m, m, ang, t);
                None
            });
        }
        // op_irandom_core, op_random_per
        for _ in 0..3000 {
            let r = 0x3f80_0000 | rng.below(0x80_0000);
            p.put(rand::bridge::R, r);
            p.put(rand::bridge::SEEDED, u32::from(rng.chance(80)));
            // offsets inside the arguments' slots (others hit the frame)
            p.put(
                rand::bridge::COLOR_CON_SHIFT,
                rng.pick(&[0u32, 0, 0, 1, 2, 3, 4, 8, 0xc]),
            );
            let any = rng.next();
            let n = rng.pick(&[0u32, 1, 7, 100, 0x7fff_ffff, 0x8000_0000, any]);
            let args = [n, rng.next(), rng.next(), rng.next()];
            p.check("op_irandom_core", 0x007e_35c0, &args, |o| {
                Some(rand::bridge::op_irandom_core(o, args))
            });
            p.check("op_random_per", 0x007e_33e0, &args, |o| {
                Some(rand::bridge::op_random_per(o, args))
            });
            // the two jumps to them
            p.check("op_random_per_call", 0x007e_5d40, &args, |o| {
                Some(rand::bridge::op_random_per(o, args))
            });
            p.check("op_irandom_p", 0x007e_5d50, &args, |o| {
                Some(rand::bridge::op_irandom_core(o, args))
            });
        }
        // opus_prog_off
        let cells = p.alloc(8);
        for (sym, words, ret) in [
            (0x0064_5280u32, &[0u8, 0][..], None),
            (0x0064_53d0, &[0, 0], Some(cells)),
            (0x007f_f1d0, &[0, 0], None),
            (0x00a7_0910, &[0], None),
        ] {
            deref_recorder_with(&mut p, sym, words, ret);
        }
        let handler = p.sym(0x00a7_0910);
        let battle = p.sym(0x012a_12fc);
        let (list, other) = (p.alloc(0x100), p.alloc(0x400));
        let units = p.alloc(0x1000);
        let idx = p.alloc(0x60);
        let (buf, shapes, vt) = (p.alloc(0x40), p.alloc(0x200), p.alloc(0x80));
        for _ in 0..2000 {
            p.fill(units, 0x1000, &mut rng);
            for l in [list, other + 0x300] {
                p.set(l + 0x20, units);
                p.set(l + 0x50, vt);
                p.set(l + 0x38, u16::try_from(rng.below(9)).unwrap());
                p.set(l + 0xa8, 0x1234u32);
                for (k, at) in [0x14u32, 0x10, 0x18].into_iter().enumerate() {
                    let li = idx + 0x20 * u32::try_from(k).unwrap();
                    p.set(l + at, li);
                    let n = rng.below(6);
                    for i in 0..n {
                        p.set(li + 2 * i, u16::try_from(rng.below(16)).unwrap());
                    }
                    p.set(li + 2 * n, 0xffffu16);
                }
            }
            p.set(vt + 0x54, handler);
            p.set(battle, if rng.chance(30) { other } else { 0 });
            for i in 0..16 {
                let u = units + 0x100 * i;
                p.set(u + 0xbc, if rng.chance(20) { 0 } else { buf });
                p.set(u + 0xbb, rng.pick(&[1u8, 2, 3, 5, 9, 0xa, 0x11, 4]));
                p.set(u + 0x16, rng.pick(&[0x10u8, 0]));
                p.set(u + 0x17, u8::try_from(rng.below(4)).unwrap());
                p.set(u + 0xa8, shapes);
            }
            p.set(buf + 4, u16::try_from(rng.below(4)).unwrap());
            p.set(shapes, u16::try_from(rng.below(4)).unwrap());
            p.set(cells, rng.pick(&[0u32, 1]));
            let no = rng.below(16) | rng.pick(&[0, 0x1_0000]);
            p.check("opus_prog_off", 0x0080_14c0, &[list, no], |o| {
                op::bridge::opus_prog_off(o, list, no, &mut call_original);
                None
            });
        }
    }
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round10d_helpers_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007f_df70);
        for (sym, words) in [
            (0x007f_e920u32, &[0u8][..]),
            // the original opus_prog_off runs: it takes the unit out of
            // its index list, which the caller's loop relies on
            (0x0064_5280, &[0, 0]),
            (0x0080_0d10, &[0, 0]),
            // yonExtMapDeleteId (0x00807830) is a bare `ret`, left alone
            (0x0062_f500, &[0, 0, 0]),
        ] {
            deref_recorder(&mut p, sym, words);
        }
        let eff = p.sym(et::bridge::EFF.sym);
        let list = p.alloc(0x100);
        let units = p.alloc(0x1000);
        let idx = p.alloc(0x60);
        let (a, b, oef) = (p.alloc(0x20), p.alloc(0x20), p.alloc(0x100));
        for _ in 0..1500 {
            // opu_unit_free_by_oef2
            for i in 0..512u32 {
                p.set(eff + 0x20 * i + 0x1c, u8::try_from(rng.below(20)).unwrap());
                p.set(eff + 0x20 * i, rng.next());
            }
            p.fill(units, 0x1000, &mut rng);
            p.set(p.sym(0x012a_12fc), 0u32);
            p.set(list + 0x38, 0x20u16);
            for i in 0..16 {
                let u = units + 0x100 * i;
                p.set(u + 0xbc, 0u32);
                p.set(u + 0x16, 0u8);
                p.set(u + 0x1e, u8::try_from(rng.below(6)).unwrap());
            }
            for i in 0..16 {
                p.set(list + 0x60 + 4 * i, 0x100 + rng.below(6));
            }
            p.set(list + 0x20, units);
            for (k, at) in [0x10u32, 0x14, 0x18].into_iter().enumerate() {
                let li = idx + 0x20 * u32::try_from(k).unwrap();
                p.set(list + at, li);
                let n = rng.below(6);
                for i in 0..n {
                    p.set(li + 2 * i, u16::try_from(rng.below(16)).unwrap());
                }
                p.set(li + 2 * n, 0xffffu16);
            }
            let o = rng.pick(&[0u32, 0x100, 0x101, 0x102, 0x105, 0x200]);
            p.check("opu_unit_free_by_oef2", 0x007f_df70, &[list, o], |m| {
                et::bridge::opu_unit_free_by_oef2(m, list, o, &mut call_original);
                None
            });
            // opu_unit_free_by_id: a record's id (or none), units of that
            // record index (u16 +0x14) and others
            let j = rng.below(512);
            let id: u32 = p.get(eff + 0x20 * j);
            let id = if rng.chance(15) { rng.next() } else { id };
            // a second record with the id: the first one counts
            if j < 511 && rng.chance(30) {
                p.set(eff + 0x20 * (j + 1), id);
            }
            for i in 0..16 {
                let any = rng.below(512);
                let r = rng.pick(&[j, j, j + 1, any]);
                p.set(units + 0x100 * i + 0x14, u16::try_from(r).unwrap());
            }
            for (k, at) in [0x10u32, 0x14, 0x18].into_iter().enumerate() {
                let li = idx + 0x20 * u32::try_from(k).unwrap();
                p.set(list + at, li);
                let n = rng.below(6);
                for i in 0..n {
                    p.set(li + 2 * i, u16::try_from(rng.below(16)).unwrap());
                }
                p.set(li + 2 * n, 0xffffu16);
            }
            p.check("opu_unit_free_by_id", 0x007f_de30, &[list, id], |m| {
                et::bridge::opu_unit_free_by_id(m, list, id, &mut call_original);
                None
            });
            // Vec4Max
            let k = [rng.float(), rng.float(), rng.float(), rng.float()];
            for i in 0..4 {
                let x = if rng.chance(20) {
                    k[i as usize]
                } else {
                    rng.float()
                };
                p.set(a + 4 * i, x);
            }
            p.check(
                "Vec4Max",
                0x0080_8b70,
                &[b, a, k[0], k[1], k[2], k[3]],
                |m| {
                    vec::bridge::vec4_max(m, b, a, k);
                    None
                },
            );
            // op_oef2_a_set
            p.fill(oef, 0x100, &mut rng);
            let kind = rng.below(10);
            p.set(oef + 4 * kind, 0x80 - 4 * rng.below(4));
            p.set(
                oef + 0x40 + 2 * kind,
                rng.pick(&[0u16, 1, 4, 5, 0xfe, 0x1ff]),
            );
            let i = rng.pick(&[0u32, 1, 3, 4, 7, u32::MAX]);
            let v = rng.next();
            p.check("op_oef2_a_set", 0x007e_38a0, &[oef, kind, i, v], |m| {
                Some(op::bridge::op_oef2_a_set(
                    m,
                    oef,
                    kind,
                    i,
                    v,
                    &mut call_original,
                ))
            });
        }
    }
}

/// Runs the original (cdecl) in `a`, which leaves a float in `ST0`, and
/// `rust` in `b`; checks the float (NaN for NaN) and the memory.
fn check_st0(
    p: &mut Pair,
    what: &str,
    sym: u32,
    args: &[u32],
    rust: impl FnOnce(&mut Oracle) -> f64,
) {
    let at = p.sym(sym);
    let r =
        p.a.call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{}: {what}: {e}", p.name));
    let st0 = r
        .st0
        .unwrap_or_else(|| panic!("{}: {what}: no float in ST0", p.name));
    let mine = rust(&mut p.b);
    assert!(
        st0.to_bits() == mine.to_bits() || (st0.is_nan() && mine.is_nan()),
        "{}: {what}: {st0} vs {mine}",
        p.name
    );
    p.same(what);
}

/// A stand-in at `sym` that returns the `f32` at `at` in `ST0`
/// (`fld dword [at]; ret`).
fn float_stub(p: &mut Pair, sym: u32, at: u32) {
    let mut c = vec![0xd9, 0x05]; // fld dword [at]
    c.extend(at.to_le_bytes());
    c.push(0xc3);
    let s = p.sym(sym);
    p.a.write(s, &c).unwrap();
    p.b.write(s, &c).unwrap();
}

/// An original that returns in `ST0`, called from a bridge.
fn call_original_f(m: &mut Oracle, f: u32, args: &[u32]) -> f64 {
    m.call(f, Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{f:#x}: {e}"))
        .st0
        .unwrap_or_else(|| panic!("{f:#x}: no float in ST0"))
}

#[test]
#[allow(clippy::too_many_lines)]
fn round10e_helpers_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0080_c3f0);
        let cells = p.alloc(16);
        for (sym, words, ret) in [
            (0x0080_14c0u32, &[0u8, 0][..], None),
            (0x0080_0d10, &[0, 0], None),
            (0x0078_0cb0, &[], Some(cells)),
            (0x0082_aa30, &[0, 0], None),
            (0x00a5_e2b0, &[0, 0], None),
        ] {
            deref_recorder_with(&mut p, sym, words, ret);
        }
        float_stub(&mut p, 0x0079_5a20, cells + 4);
        float_stub(&mut p, 0x0082_ad70, cells + 8);
        let eff = p.sym(et::bridge::EFF.sym);
        let handles = p.sym(et::bridge::CHR_HANDLES.sym);
        let (obj, part, src, dst) = (p.alloc(0x400), p.alloc(0x40), p.alloc(0x10), p.alloc(0x10));
        for _ in 0..3000 {
            // opu_end
            p.fill(obj, 0x400, &mut rng);
            p.fill(part, 0x40, &mut rng);
            p.set(obj + 0x220, part);
            p.set(obj + 0x216, u16::from(rng.chance(20)));
            let i = rng.below(512);
            p.set(part + 0x14, u16::try_from(i).unwrap());
            p.set(eff + 0x20 * i + 4, if rng.chance(70) { part } else { 0 });
            p.set(
                eff + 0x20 * i,
                rng.pick(&[0x4000_0001u32, 0x5000_0000, 0x4fff_ffff]),
            );
            let cmd = rng.next();
            p.check("opu_end", 0x0080_c3f0, &[obj, cmd], |m| {
                Some(opus::bridge::opu_end(m, obj, cmd, &mut call_original))
            });
            // opus_chr_width
            for k in 1..3 {
                let x = finite(&mut rng);
                p.set(cells + 4 * k, x);
            }
            let chr = rng.below(0xf0);
            p.set(handles + 4 * chr, rng.next());
            let battle = rng.pick(&[0u32, 1, 0x100]);
            check_st0(&mut p, "opus_chr_width", 0x0080_0cd0, &[chr, battle], |m| {
                f64::from(opus::bridge::opus_chr_width(
                    m,
                    chr,
                    battle,
                    &mut call_original_f,
                ))
            });
            // opus_clip_off_all_in_battle
            p.set(cells, rng.pick(&[0u32, 1]));
            for k in 0..32 {
                p.set(handles + 4 * k, if rng.chance(50) { 0 } else { rng.next() });
            }
            p.check("opus_clip_off_all_in_battle", 0x0080_0bb0, &[], |m| {
                Some(opus::bridge::opus_clip_off_all_in_battle(
                    m,
                    &mut call_original,
                ))
            });
            // op_ivec4_to_fvec4
            p.fill(src, 0x10, &mut rng);
            let d = rng.pick(&[dst, src]);
            p.check("op_ivec4_to_fvec4", 0x007e_3100, &[d, src], |m| {
                vec::bridge::ivec4_to_fvec4(m, d, src);
                None
            });
            // op_num_xy_set
            p.put(op::bridge::NUM_SHOW, u8::from(rng.chance(50)));
            p.put(op::bridge::DEBUG_MODE, u8::from(rng.chance(50)));
            let (x, y) = (rng.next(), rng.next());
            p.check("op_num_xy_set", 0x007e_65e0, &[x, y], |m| {
                op::bridge::op_num_xy_set(m, x, y, &mut call_original);
                None
            });
        }
    }
}

#[test]
fn op_heaps_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007f_f780);
        deref_recorder(&mut p, 0x0062_f500, &[0]);
        let mem = p.alloc(0x2010);
        let (src, dst) = (p.alloc(0x10), p.alloc(0x10));
        for round in 0..60 {
            p.fill(mem, 0x2010, &mut rng);
            let size = rng.pick(&[0u32, 0x10, 0x18, 0x1000, 0x2000, 0x1ff8]);
            let base = mem + 8 * (round % 2);
            let heap = p.check("op_heap_init", 0x007f_f2d0, &[base, size], |m| {
                Some(op::bridge::op_heap_init(m, base, size))
            });
            if size < 0x1000 {
                continue;
            }
            let mut live: Vec<u32> = Vec::new();
            for _ in 0..80 {
                if live.is_empty() || (rng.chance(60) && live.len() < 40) {
                    let any = rng.below(0x100);
                    let n = rng.pick(&[0u32, 1, 0x10, 0x11, 0x40, 0x7f, any]);
                    let b = p.check("op_malloc", 0x007f_f780, &[heap, n], |m| {
                        Some(op::bridge::op_malloc(m, heap, n, &mut call_original))
                    });
                    live.push(b);
                } else {
                    let i = rng.below(u32::try_from(live.len()).unwrap()) as usize;
                    let b = live.swap_remove(i);
                    p.check("op_free", 0x007f_f1d0, &[heap, b], |m| {
                        op::bridge::op_free(m, heap, b, &mut call_original);
                        None
                    });
                }
            }
        }
        for _ in 0..500 {
            let k = [rng.float(), rng.float(), rng.float(), rng.float()];
            for i in 0..4 {
                let x = rng.float();
                p.set(src + 4 * i, x);
            }
            p.check(
                "Vec4AddScalars",
                0x0074_bc50,
                &[dst, src, k[0], k[1], k[2], k[3]],
                |m| {
                    vec::bridge::vec4_add_scalars(m, dst, src, k.map(f32::from_bits));
                    None
                },
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round11_matrices_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007e_a550);
        let (m, v, t, dst, c) = (
            p.alloc(0x40),
            p.alloc(0x10),
            p.alloc(0x10),
            p.alloc(0x40),
            p.alloc(0x40),
        );
        let regs = p.sym(0x00c8_f508);
        for round in 0..2000 {
            for at in [m, c] {
                for i in 0..16 {
                    let x = finite(&mut rng);
                    p.set(at + 4 * i, x);
                }
            }
            for i in 0..4 {
                let x = finite(&mut rng);
                p.set(v + 4 * i, x);
                let y = finite(&mut rng);
                p.set(t + 4 * i, y);
            }
            if round % 4 == 0 {
                p.set(v, 0u32);
                p.set(v + 8, rng.pick(&[0u32, 0x8000_0000]));
            }
            if round % 3 == 0 {
                p.set(m + 0xc, 1.0f32);
                p.set(m + 0x1c, 0.0f32);
                p.set(m + 0x2c, 0.0f32);
                p.set(m + 0x3c, rng.pick(&[1.0f32, 2.0, 0.5, -1.0]));
            }
            // the emulated r0 and a0/a1 words op_vector_m compares
            for k in [0u32, 1, 8, 9, 12, 13] {
                let w = if rng.chance(70) { 0 } else { rng.next() };
                p.set(regs + 4 * k, w);
            }
            let d = rng.pick(&[dst, m]);
            p.check("op_trans_m", 0x007e_a550, &[d, m, v], |o| {
                Some(op::bridge::op_trans_m(o, d, m, v))
            });
            p.check("op_rtp_w1", 0x007e_9b50, &[dst, m, v], |o| {
                Some(op::bridge::op_rtp_w1(o, dst, m, v))
            });
            check_deep3(
                &mut p,
                "op_vector_m",
                0x007e_a760,
                [dst, t, v],
                |o, frame| op::bridge::op_vector_m(o, dst, t, v, frame, &mut call_original),
            );
            check_deep3(
                &mut p,
                "op_matrix_light4_vector",
                0x00a6_cd80,
                [dst, m, c],
                |o, frame| {
                    op::bridge::op_matrix_light4_vector(o, dst, m, c, frame, &mut call_original)
                },
            );
        }
    }
}

/// [`check_deep`] for three arguments.
fn check_deep3(
    p: &mut Pair,
    what: &str,
    sym: u32,
    args: [u32; 3],
    rust: impl FnOnce(&mut Oracle, u32) -> u32,
) {
    let at = p.sym(THUNK);
    let mut c = vec![0x81, 0xec, 0, 2, 0, 0]; // sub esp, 0x200
    for _ in 0..3 {
        c.extend([0xff, 0xb4, 0x24, 0x0c, 0x02, 0, 0]); // push [esp + 0x20c]
    }
    c.push(0xe8); // call sym
    let call_end = at + u32::try_from(c.len()).unwrap() + 4;
    c.extend(p.sym(sym).wrapping_sub(call_end).to_le_bytes());
    c.extend([0x81, 0xc4, 0x0c, 0x02, 0, 0]); // add esp, 0x20c
    c.push(0xc3);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    let frame = p.b.stack_range().end - 0x100 - 12 - 4 - 0x200 - 12 - 4 - 4;
    let r =
        p.a.call(at, Conv::Cdecl, &args)
            .unwrap_or_else(|e| panic!("{}: {what}: {e}", p.name));
    assert!(
        r.st0.is_none(),
        "{}: {what}: the original returns {:?} in ST0",
        p.name,
        r.st0
    );
    let mine = rust(&mut p.b, frame);
    assert_eq!(r.eax, mine, "{}: {what}: result", p.name);
    p.same(what);
}

#[test]
#[allow(clippy::too_many_lines)]
fn round11_lns_and_cluts_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0075_a2d0);
        deref_recorder(&mut p, 0x0076_a1a0, &[0, 0]);
        let (obj, key, ctrl, data) = (p.alloc(0x200), p.alloc(0x10), p.alloc(0x10), p.alloc(0x10));
        let (env, table, shapes) = (p.alloc(0x40), p.alloc(0x40), p.alloc(0x100));
        let slot = p.alloc(4);
        p.put(heap::bridge::ENV_SLOT, slot);
        p.set(slot, env);
        p.set(env + 0x24, table);
        for i in 0..16 {
            p.set(table + 4 * i, shapes + 0x10 * i);
        }
        for _ in 0..2000 {
            p.fill(obj, 0x200, &mut rng);
            p.fill(shapes, 0x100, &mut rng);
            p.set(ctrl + 0xc, data);
            p.set(data, 4 * rng.below(8));
            p.set(data + 4, 4 * rng.below(8));
            let f: u32 = p.get(obj + 0xc);
            p.set(key, if rng.chance(80) { f } else { f ^ 1 });
            let (s1, s2) = (rng.below(16), rng.below(16));
            let n = rng.pick(&[0xffffu32, s1, s2]);
            p.set(key + 4, n);
            if rng.chance(30) {
                let w = obj + 0xa0 + p.get::<u32>(data);
                p.set(w + 4, table + 4 * (n & 0xf));
            }
            for i in 0..16 {
                p.set(shapes + 0x10 * i, shapes + 0x10 * rng.below(16));
            }
            for (name, at) in [
                ("pppKeLnsArndUpdate", 0x0075_a2d0u32),
                ("pppKeLnsFlsUpdate", 0x0075_ae90),
            ] {
                p.check(name, at, &[obj, key, ctrl], |o| {
                    Some(prog::bridge::ppp_ke_lns_arnd_update(o, obj, key, ctrl))
                });
            }
            p.check("pppKeLnsCrnUpdate", 0x0075_aab0, &[obj, key, ctrl], |o| {
                Some(prog::bridge::ppp_ke_lns_crn_update(
                    o,
                    obj,
                    key,
                    ctrl,
                    &mut call_original,
                ))
            });
        }
        // op_oef2_tex_clut_ovl_add over a crowded table
        let masks = p.sym(op::bridge::OEF2_CLUT_OVL.sym);
        let flags = p.sym(op::bridge::OEF2_CLUT_OVL_FLAGS.sym);
        let recs_at = p.sym(op::bridge::OEF2_CLUT_OVL_RECS.sym);
        let recs = p.alloc(0x20 * 16);
        for _ in 0..2000 {
            for i in 0..16u32 {
                let r = recs + 0x20 * i;
                p.set(r, rng.below(4) | rng.next() & 0xffff_c000);
                for (o, v) in [
                    (8u32, rng.below(3)),
                    (9, rng.below(3)),
                    (0xa, rng.below(3) + 1),
                    (0x18, rng.below(4)),
                ] {
                    p.set(r + o, u8::try_from(v).unwrap());
                }
                // sizes that put rectangle edges on each other's
                for o in [0xcu32, 0xe] {
                    let n = rng.pick(&[1u16, 16, 17, 32, 33, 48]);
                    p.set(r + o, n);
                }
            }
            for i in 0..64u32 {
                let full = rng.chance(if i < 60 { 60 } else { 98 });
                p.set(masks + 4 * i, if full { 1u32 << rng.below(4) } else { 0 });
                p.set(flags + i, u8::from(rng.chance(30)));
                p.set(recs_at + 4 * i, recs + 0x20 * rng.below(16));
            }
            let rec = recs + 0x20 * rng.below(16);
            let mask = 1u32 << rng.below(4);
            let flag = rng.pick(&[0u32, 1, 0x100]);
            p.check(
                "op_oef2_tex_clut_ovl_add",
                0x007e_56d0,
                &[rec, mask, flag],
                |o| Some(op::bridge::op_oef2_tex_clut_ovl_add(o, rec, mask, flag).cast_unsigned()),
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round11b_helpers_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007e_a730);
        deref_recorder(&mut p, 0x0062_f500, &[0]);
        let cells = p.alloc(16);
        for (k, sym) in [0x0079_50f0u32, 0x0079_5a20, 0x0082_aba0, 0x0082_ad70]
            .into_iter()
            .enumerate()
        {
            float_stub(&mut p, sym, cells + 4 * u32::try_from(k).unwrap());
        }
        let (m, v, sa, sb) = (p.alloc(0x40), p.alloc(0x10), p.alloc(0x20), p.alloc(0x20));
        let eff = p.sym(et::bridge::EFF.sym);
        let parts = p.alloc(0x200);
        let pairs = p.sym(op::bridge::PAIR_TABLE.sym);
        let handles = p.sym(et::bridge::CHR_HANDLES.sym);
        for _ in 0..3000 {
            p.fill(m, 0x40, &mut rng);
            for i in 0..4 {
                let x = rng.float();
                p.set(v + 4 * i, x);
            }
            p.check("op_unit_m_scale2", 0x007e_a730, &[m, v], |o| {
                Some(op::bridge::op_unit_m_scale2(o, m, v))
            });
            for (name, at, msg) in [
                ("op_fp_effect_free", 0x007f_cc40u32, 0x00b5_84e4u32),
                ("op_fp_effect_init", 0x007f_cc60, 0x00b5_84c8),
            ] {
                let ovl = p.sym(op::bridge::OEF2_CLUT_OVL.sym);
                for i in 0..64 {
                    p.set(ovl + 4 * i, rng.pick(&[0u32, 0x4000_0000, 1, 0x4000_0001]));
                }
                p.check(name, at, &[], |o| {
                    Some(op::bridge::op_fp_effect_reset(o, msg, &mut call_original))
                });
            }
            // op_et_prayeff_send_read_through
            for i in 0..512u32 {
                p.set(eff + 0x20 * i, rng.below(2000));
                p.set(eff + 0x20 * i + 4, parts + 4 * rng.below(0x60));
            }
            let id = rng.below(2000);
            p.check("op_et_prayeff_send_read_through", 0x007f_c920, &[id], |o| {
                Some(op::bridge::op_et_prayeff_send_read_through(
                    o,
                    id,
                    &mut call_original,
                ))
            });
            // op_deg_in_check
            #[allow(clippy::cast_precision_loss)] // small
            let ang = |r: &mut Xorshift| {
                f32::from_bits(if r.chance(10) {
                    r.float()
                } else {
                    (r.below(0x3000) as f32 - 0x800 as f32).to_bits()
                })
            };
            let (a, b, c) = (ang(&mut rng), ang(&mut rng), ang(&mut rng));
            p.check(
                "op_deg_in_check",
                0x007e_7f60,
                &[a.to_bits(), b.to_bits(), c.to_bits()],
                |o| Some(op::bridge::op_deg_in_check(o, a, b, c)),
            );
            // op_str_equal
            for i in 0..8 {
                let ch = rng.pick(&[b'a', b'A', b'z', b'Z', b'[', b'@', 0xc1, b'q', 0]);
                p.set(sa + i, ch);
                let other = if rng.chance(70) {
                    ch ^ (u8::from(rng.chance(50)) << 5)
                } else {
                    b'q'
                };
                p.set(sb + i, other);
            }
            p.set(sa + 8, 0u8);
            p.set(sb + 8, 0u8);
            let (x, y) = (rng.pick(&[sa, sa, 0]), rng.pick(&[sb, sb, 0]));
            let nocase = rng.pick(&[0u32, 1, 0x100]);
            p.check("op_str_equal", 0x007e_2600, &[x, y, nocase], |o| {
                Some(op::bridge::op_str_equal(o, x, y, nocase))
            });
            // op_pair_index
            let n = rng.pick(&[0u32, 1, 3, 8, u32::MAX, 0x8000_0000]);
            p.set(pairs, n);
            for i in 1..10 {
                p.set(pairs + 8 * i, rng.below(4));
                p.set(pairs + 8 * i + 4, rng.below(4));
            }
            let (a2, b2) = (rng.below(4), rng.below(4));
            if n.cast_signed() < 9 || n == u32::MAX {
                p.check("op_pair_index", 0x00a7_0ce0, &[a2, b2], |o| {
                    Some(op::bridge::op_pair_index(o, a2, b2))
                });
            }
            // opus_cscale_get
            for k in 0..4 {
                let x = finite(&mut rng);
                p.set(cells + 4 * k, x);
            }
            if rng.chance(20) {
                let x: u32 = p.get(cells);
                p.set(cells + 4, x);
                p.set(cells + 12, p.get::<u32>(cells + 8));
            }
            let chr = rng.below(0xf0);
            p.set(handles + 4 * chr, rng.next());
            let battle = rng.pick(&[0u32, 1]);
            check_st0(
                &mut p,
                "opus_cscale_get",
                0x0080_0c10,
                &[chr, battle],
                |o| {
                    f64::from(opus::bridge::opus_cscale_get(
                        o,
                        chr,
                        battle,
                        &mut call_original_f,
                    ))
                },
            );
        }
    }
}

/// [`check_deep`] for any number of arguments; `rust` gives the result to
/// compare with `eax`, if any.
#[allow(clippy::many_single_char_names)]
fn check_deep_n(
    p: &mut Pair,
    what: &str,
    sym: u32,
    args: &[u32],
    rust: impl FnOnce(&mut Oracle, u32) -> Option<u32>,
) {
    let n = u8::try_from(args.len()).unwrap();
    let at = p.sym(THUNK);
    let mut c = vec![0x81, 0xec, 0, 2, 0, 0]; // sub esp, 0x200
    for _ in 0..n {
        c.extend([0xff, 0xb4, 0x24]); // push [esp + 0x200 + 4n]
        c.extend((0x200 + 4 * u32::from(n)).to_le_bytes());
    }
    c.push(0xe8); // call sym
    let call_end = at + u32::try_from(c.len()).unwrap() + 4;
    c.extend(p.sym(sym).wrapping_sub(call_end).to_le_bytes());
    c.extend([0x81, 0xc4]); // add esp, 0x200 + 4n
    c.extend((0x200 + 4 * u32::from(n)).to_le_bytes());
    c.push(0xc3);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    let w = 4 * u32::from(n);
    let frame = p.b.stack_range().end - 0x100 - w - 4 - 0x200 - w - 4 - 4;
    let r =
        p.a.call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{}: {what}: {e}", p.name));
    assert!(
        r.st0.is_none(),
        "{}: {what}: the original returns {:?} in ST0",
        p.name,
        r.st0
    );
    if let Some(mine) = rust(&mut p.b, frame) {
        assert_eq!(r.eax, mine, "{}: {what}: result", p.name);
    }
    p.same(what);
}

#[test]
#[allow(clippy::many_single_char_names)]
fn fog_and_sep_waves_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007f_f100);
        let cells = p.alloc(0x20);
        copy_recorder(&mut p, 0x0090_8980, 1, 0, From::At(cells), 3);
        // yiGetFogParam(a, b, c): each from the cells
        let mut c = Vec::new();
        for k in 0..3u8 {
            c.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp + 4 + 4k]
            c.extend([0x8b, 0x0d]); // mov ecx, [cell]
            c.extend((cells + 0x10 + 4 * u32::from(k)).to_le_bytes());
            c.extend([0x89, 0x08]); // mov [eax], ecx
        }
        c.push(0xc3);
        let at = p.sym(0x0090_8bb0);
        p.a.write(at, &c).unwrap();
        p.b.write(at, &c).unwrap();
        deref_recorder(&mut p, 0x0088_6f30, &[0, 0]);
        deref_recorder(&mut p, 0x0088_7450, &[0, 0]);
        let seps = p.sym(op::bridge::OEF2_SEPS.sym);
        let (cue, wave) = (p.alloc(0x10), p.alloc(0x10));
        for _ in 0..3000 {
            p.fill(cells, 0x20, &mut rng);
            for k in 0..3 {
                let x = if rng.chance(15) {
                    rng.float()
                } else {
                    finite(&mut rng)
                };
                p.set(cells + 0x10 + 4 * k, x);
            }
            if rng.chance(20) {
                let x: u32 = p.get(cells + 0x10);
                p.set(cells + 0x14, x);
            }
            check_deep_n(&mut p, "op_fog_param_set", 0x007f_f100, &[], |o, frame| {
                // eax is the status word over a colour byte: no result
                op::bridge::op_fog_param_set(o, frame, &mut call_original);
                None
            });
            for i in 0..32 {
                let h = if rng.chance(50) { 0 } else { rng.below(40) };
                p.set(seps + 8 * i, h);
                p.set(seps + 8 * i + 4, rng.below(16));
            }
            p.set(cue + 8, wave);
            p.set(wave + 8, rng.below(40));
            p.set(wave + 0xc, rng.next());
            p.set(cue + 0xc, rng.pick(&[0u32, 1, 0x8000_0000, 7]));
            let mask = 1u32 << rng.below(8);
            p.check("op_oef2_sep_wave_load", 0x007f_ffa0, &[cue, mask], |o| {
                Some(op::bridge::op_oef2_sep_wave_load(
                    o,
                    cue,
                    mask,
                    &mut call_original,
                ))
            });
        }
    }
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn round12_helpers_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_90c0);
        x87_crt_stand_ins(&mut p);
        // the light getters fill their output from cells
        let cells = p.alloc(0x40);
        for sym in [0x0083_61c0u32, 0x0083_60f0] {
            copy_recorder(&mut p, sym, 2, 1, From::At(cells), 4);
        }
        // one word: code 3's message has no argument after it
        deref_recorder(&mut p, 0x0062_f500, &[0]);
        let dma = p.alloc(4);
        deref_recorder_with(&mut p, 0x0062_f6e0, &[0], Some(dma));
        deref_recorder(&mut p, 0x00a5_e4a0, &[]);
        let (a, b, v) = (p.alloc(0x40), p.alloc(0x40), p.alloc(0x10));
        let (tri, table) = (p.alloc(0x20), p.alloc(0x40 * 8));
        let (xs, ys, ds) = (p.alloc(0x40), p.alloc(0x40), p.alloc(0x40));
        let scratch = p.alloc(0x100);
        let (file, line) = (p.sym(0x0113_fc58), p.sym(0x0113_fc5c));
        for _ in 0..2000 {
            p.fill(cells, 0x40, &mut rng);
            p.fill(a, 0x40, &mut rng);
            p.fill(b, 0x40, &mut rng);
            for i in 0..4 {
                let x = rng.float();
                p.set(cells + 4 * i, x);
            }
            check_deep_n(
                &mut p,
                "op_ffxlight_to_omdlight",
                0x00a6_90c0,
                &[a, b],
                |o, frame| {
                    Some(op::bridge::op_ffxlight_to_omdlight(
                        o,
                        a,
                        b,
                        frame,
                        &mut call_original,
                    ))
                },
            );
            // op_md_vn_index_sub
            p.fill(table, 0x200, &mut rng);
            for k in 0..3u32 {
                p.set(tri + 2 * k, u16::try_from(rng.below(8)).unwrap());
                p.set(tri + 0x14 + 2 * k, rng.pick(&[0u16, 1, 2, 0x8001, 0xffff]));
            }
            for l in 0..8u32 {
                p.set(table + 0x40 * l, rng.pick(&[0u16, 1, 3, 5, 0x8000]));
                for i in 0..6 {
                    p.set(
                        table + 0x40 * l + 2 + 2 * i,
                        rng.pick(&[0u16, 1, 2, 0x8001, 0xffff]),
                    );
                }
            }
            p.check("op_md_vn_index_sub", 0x00a6_9fe0, &[tri, table], |o| {
                Some(op::bridge::op_md_vn_index_sub(o, tri, table))
            });
            // op_error
            p.set(file, rng.pick(&[0u32, 0x1234]));
            p.set(line, rng.pick(&[0u32, 7]));
            let code = rng.below(14);
            let arg = rng.next();
            p.check("op_error", 0x007e_1450, &[code, arg], |o| {
                Some(op::bridge::op_error(o, code, arg, &mut call_original))
            });
            // pppVu0Length_copy
            for i in 0..4 {
                let x = rng.float();
                p.set(v + 4 * i, x);
            }
            check_st0(&mut p, "pppVu0Length_copy", 0x0072_ab20, &[v], |o| {
                f64::from(vec::bridge::ppp_vu0_length(o, v))
            });
            // op_spline_table, then op_spline over it
            let n = rng.pick(&[2u32, 3, 4, 5, 8, 12]);
            let mut x = 0f32;
            for i in 0..16 {
                if rng.chance(85) {
                    x += f32::from_bits(finite(&mut rng)).abs().min(100.0);
                }
                p.set(xs + 4 * i, x);
                let y = finite(&mut rng);
                p.set(ys + 4 * i, y);
            }
            p.fill(ds, 0x40, &mut rng);
            // a scratch stack for its buffer
            p.put(op::bridge::SPR_TOP, 0i8);
            let spr = p.sym(op::bridge::SPR_STACK.sym);
            p.set(spr, scratch);
            p.check("op_spline_table", 0x007e_9f20, &[ds, xs, ys, n], |o| {
                op::bridge::op_spline_table(o, ds, xs, ys, n);
                None
            });
            let any = f32::from_bits(finite(&mut rng));
            let t = rng.pick(&[0.0f32, x / 2.0, x, -1.0, x * 2.0, any]);
            let nn = rng.pick(&[n, 1, 2, 0]);
            check_st0(
                &mut p,
                "op_spline",
                0x007e_9e20,
                &[t.to_bits(), ys, xs, ds, nn],
                |o| f64::from(op::bridge::op_spline(o, t, ys, xs, ds, nn)),
            );
            // opd_num_init
            p.set(dma, rng.next());
            let xv = rng.next();
            p.check("opd_num_init", 0x00a5_dda0, &[xv], |o| {
                Some(op::bridge::opd_num_init(o, xv, &mut call_original))
            });
        }
    }
}

#[test]
#[allow(clippy::many_single_char_names)]
fn round12b_helpers_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007e_a120);
        deref_recorder(&mut p, 0x00a7_0910, &[0, 0]);
        let fns = p.sym(0x00a7_0910);
        let (pts, xs, out, src) = (p.alloc(0x100), p.alloc(0x100), p.alloc(0x10), p.alloc(0x10));
        let scratch = p.alloc(0x100);
        let order = p.sym(op::bridge::FUNC_OT_ORDER.sym);
        let slots = p.sym(op::bridge::FUNC_OT.sym);
        for _ in 0..2000 {
            // op_spline_xyz over op_spline_table's slopes, here random
            let n = rng.pick(&[0u32, 1, 2, 3, 4, 5, 7, 9]);
            let mut x = 0f32;
            for i in 0..16 {
                x += f32::from_bits(finite(&mut rng)).abs().min(50.0);
                p.set(xs + 4 * i, x);
            }
            for i in 16..48 {
                let d = finite(&mut rng);
                p.set(xs + 4 * i, d);
            }
            for i in 0..0x40 {
                let v = finite(&mut rng);
                p.set(pts + 4 * i, v);
            }
            p.put(op::bridge::SPR_TOP, 0i8);
            let spr = p.sym(op::bridge::SPR_STACK.sym);
            p.set(spr, scratch);
            let any = f32::from_bits(finite(&mut rng));
            let t = rng.pick(&[0.0f32, x / 3.0, any]);
            p.check(
                "op_spline_xyz",
                0x007e_a120,
                &[out, t.to_bits(), pts, xs, n],
                |o| {
                    op::bridge::op_spline_xyz(o, out, t, pts, xs, n);
                    None
                },
            );
            // Vec4SubScalars
            let k = [rng.float(), rng.float(), rng.float(), rng.float()];
            for i in 0..4 {
                let v = rng.float();
                p.set(src + 4 * i, v);
            }
            p.check(
                "Vec4SubScalars",
                0x0074_be90,
                &[out, src, k[0], k[1], k[2], k[3]],
                |o| {
                    vec::bridge::vec4_sub_scalars(o, out, src, k.map(f32::from_bits));
                    None
                },
            );
            // op_func_ot
            let list = rng.pick(&[0u32, 0x20, 0x40]);
            for i in 0..0x80u32 {
                p.set(order + i, -1i8);
            }
            // distinct slots, as op_func_ot_set enters them (a freed slot's
            // function is cleared)
            let m = rng.below(8);
            let first = rng.below(8);
            for i in 0..m {
                p.set(order + list + i, i8::try_from((first + 3 * i) % 8).unwrap());
            }
            for i in 0..0x48u32 {
                let e = slots + 16 * i;
                p.set(e, fns);
                p.set(e + 4, rng.next());
                p.set(e + 8, rng.next());
                p.set(e + 0xc, u16::try_from(rng.below(3)).unwrap());
            }
            p.check("op_func_ot", 0x007e_b430, &[list], |o| {
                op::bridge::op_func_ot(o, list, &mut call_original);
                None
            });
        }
    }
}

/// The vertex streams a render-data record points at (positions `+0xc`,
/// texture coordinates `+0x10`, colours `+0x14`, indices `+0x1c`), as
/// bytes, from oracle `o`.
fn render_streams(o: &Oracle, rd: u32, len: u32) -> Vec<u8> {
    let mut out = Vec::new();
    for at in [0xcu32, 0x10, 0x14, 0x1c] {
        let base: u32 = Mem::read(o, rd + at);
        for i in 0..len {
            out.push(Mem::read::<u8>(o, base + i));
        }
    }
    for at in [4u32, 8] {
        out.extend(Mem::read::<u32>(o, rd + at).to_le_bytes());
    }
    out
}

#[test]
#[allow(clippy::too_many_lines)]
fn eiabm_sphe_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_8140);
        // graphicFontGetScreenWH(&w, &h) from cells; the render data
        // lookup returns `tex`
        let cells = p.alloc(0x10);
        let mut c = Vec::new();
        for k in 0..2u8 {
            c.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp + 4 + 4k]
            c.extend([0x8b, 0x0d]); // mov ecx, [cell]
            c.extend((cells + 4 * u32::from(k)).to_le_bytes());
            c.extend([0x89, 0x08]); // mov [eax], ecx
        }
        c.push(0xc3);
        let at = p.sym(0x0064_0db0);
        p.a.write(at, &c).unwrap();
        p.b.write(at, &c).unwrap();
        let tex = p.alloc(0x100);
        p.set(cells + 8, tex);
        deref_recorder_with(&mut p, 0x0063_8fd0, &[0, 0], Some(cells + 8));
        let rd = p.alloc(0x40);
        p.set(tex + 0x94, rd);
        let len = 0x2000u32;
        let streams = [
            p.alloc(len as usize),
            p.alloc(len as usize),
            p.alloc(len as usize),
            p.alloc(len as usize),
        ];
        for (s, at) in streams.into_iter().zip([0xcu32, 0x10, 0x14, 0x1c]) {
            p.set(rd + at, s);
        }
        let skip = p.sym(0x012f_b7d0);
        let (obj, part, cam, pd) = (p.alloc(0x200), p.alloc(0x80), p.alloc(0x40), p.alloc(0x40));
        for round in 0..3000 {
            p.fill(obj, 0x200, &mut rng);
            p.fill(part, 0x80, &mut rng);
            for s in streams {
                p.fill(s, len, &mut rng);
            }
            p.set(obj + 4, 0x20 + 4 * rng.below(8));
            p.set(obj + 8, 0x100u32);
            let off: u32 = p.get(obj + 4);
            for (k, field) in [0x1eu32, 0x20].into_iter().enumerate() {
                let _ = k;
                p.set(obj + off + field, u16::try_from(rng.below(16)).unwrap());
            }
            p.set(part + 0x20, cam);
            p.set(part + 0x44, pd);
            for at in [0x2cu32, 0x30, 0x34, 0x3c] {
                let x = if rng.chance(10) { 0 } else { finite(&mut rng) };
                p.set(cam + at, x);
            }
            let dpt = rng.float();
            p.set(pd + 0x38, dpt);
            p.set(part + 0x1b, rng.pick(&[0u8, 0x80, 0xff, 1]));
            let (w9, h9) = (rng.next(), rng.next());
            p.set(cells, rng.pick(&[1280u32, 1920, 640, 0, w9]));
            p.set(cells + 4, rng.pick(&[720u32, 1080, 448, 0, h9]));
            p.set(skip, u32::from(round % 17 == 0));
            p.set(rd + 4, rng.below(100));
            p.set(rd + 8, rng.below(100));
            let n = rng.below(30);
            let e = rng.pick(&[0u32, 1, 6, u32::MAX, (-3i32).cast_unsigned()]);
            check_deep_n(
                &mut p,
                "op1_md_draw_eiabm_sphe",
                0x00a6_8140,
                &[obj, part, n, e],
                |o, frame| {
                    Some(md::bridge::op1_md_draw_eiabm_sphe(
                        o,
                        obj,
                        part,
                        n,
                        e,
                        frame,
                        &mut call_original,
                    ))
                },
            );
            // the streams, compared on their own as well
            assert_eq!(
                render_streams(&p.a, rd, len),
                render_streams(&p.b, rd, len),
                "{}: streams",
                p.name
            );
        }
    }
}

/// A value for the lighting inputs: mostly moderate, sometimes any float
/// (`rng.float()`, which includes the specials).
fn light_value(rng: &mut Xorshift) -> u32 {
    match rng.below(4) {
        0 => rng.float(),
        1 => finite(rng),
        _ => {
            #[allow(clippy::cast_precision_loss)]
            let x = (rng.below(20001) as f32 - 10000.0) / 64.0;
            x.to_bits()
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn eiabm_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_57c0);
        x87_crt_stand_ins(&mut p);
        // graphicFontGetScreenWH(&w, &h) from cells; the render data
        // lookup returns `tex`
        let cells = p.alloc(0x10);
        let mut c = Vec::new();
        for k in 0..2u8 {
            c.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp + 4 + 4k]
            c.extend([0x8b, 0x0d]); // mov ecx, [cell]
            c.extend((cells + 4 * u32::from(k)).to_le_bytes());
            c.extend([0x89, 0x08]); // mov [eax], ecx
        }
        c.push(0xc3);
        let at = p.sym(0x0064_0db0);
        p.a.write(at, &c).unwrap();
        p.b.write(at, &c).unwrap();
        let tex = p.alloc(0x100);
        p.set(cells + 8, tex);
        deref_recorder_with(&mut p, 0x0063_8fd0, &[0, 0], Some(cells + 8));
        let rd = p.alloc(0x40);
        p.set(tex + 0x94, rd);
        let len = 0x2000u32;
        let streams = [
            p.alloc(len as usize),
            p.alloc(len as usize),
            p.alloc(len as usize),
            p.alloc(len as usize),
        ];
        for (s, at) in streams.into_iter().zip([0xcu32, 0x10, 0x14, 0x1c]) {
            p.set(rd + at, s);
        }
        let skip = p.sym(0x012f_b7d0);
        let (obj, part, cam, pd) = (p.alloc(0xa00), p.alloc(0x80), p.alloc(0x40), p.alloc(0x40));
        let (ma, mm, lights) = (p.alloc(0x40), p.alloc(0x40), p.alloc(0x40));
        let sizes = [0x14u32, 0x1c, 0x20, 0x28, 0x18, 0x20, 0x28, 0x30];
        for round in 0..2500 {
            p.fill(obj, 0xa00, &mut rng);
            p.fill(part, 0x80, &mut rng);
            for s in streams {
                p.fill(s, len, &mut rng);
            }
            // the blocks
            p.set(obj + 4, 0x40u32);
            p.set(obj + 8, 0x800u32);
            p.set(obj + 0xc, 0x900u32);
            let mut hdr = obj + 0x40;
            let mut total = 0i16;
            for _ in 0..=rng.below(5) {
                let kind = rng.pick(&[0u8, 1, 1, 4, 5, 5, 2, 3, 6, 7, 9, 0xff]);
                let n = rng.below(4);
                p.set(hdr + 1, kind);
                p.set(hdr + 2, i16::try_from(n).unwrap());
                let size = sizes.get(usize::from(kind)).copied().unwrap_or(0);
                let data = hdr + 0x10;
                for r in 0..n {
                    let rec = data + r * size;
                    for field in (0xcu32..0x20).step_by(2) {
                        if field + 2 <= size {
                            p.set(rec + field, u16::try_from(rng.below(16)).unwrap());
                        }
                    }
                }
                total += i16::try_from(n).unwrap();
                hdr = data + n * size;
            }
            if rng.chance(8) && total > 0 {
                total -= 1;
            }
            p.set(obj + 0x10, total);
            // the part
            p.set(part, rng.pick(&[0x40u8, 0x40, 0x41, 0, 0xbf]));
            p.set(part + 0x20, cam);
            p.set(part + 0x24, ma);
            p.set(part + 0x28, lights);
            p.set(part + 0x2c, mm);
            p.set(part + 0x44, pd);
            for i in 0..16u32 {
                let (a, m) = (light_value(&mut rng), light_value(&mut rng));
                p.set(ma + 4 * i, a);
                p.set(mm + 4 * i, m);
                let l = light_value(&mut rng);
                p.set(lights + 4 * i, l);
            }
            for at in [0x2cu32, 0x30, 0x34, 0x3c] {
                let x = if rng.chance(10) { 0 } else { finite(&mut rng) };
                p.set(cam + at, x);
            }
            let dpt = rng.float();
            p.set(pd + 0x38, dpt);
            let (w9, h9) = (rng.next(), rng.next());
            p.set(cells, rng.pick(&[1280u32, 1920, 640, 0, w9]));
            p.set(cells + 4, rng.pick(&[720u32, 1080, 448, 0, h9]));
            p.set(skip, u32::from(round % 17 == 0));
            p.set(rd + 4, rng.below(100));
            p.set(rd + 8, rng.below(100));
            let slot = rng.pick(&[4u32, 8, 0]);
            check_deep_n(
                &mut p,
                "op1_md_draw_eiabm",
                0x00a6_57c0,
                &[obj, part, slot],
                |o, frame| {
                    Some(md::bridge::op1_md_draw_eiabm(
                        o,
                        obj,
                        part,
                        slot,
                        frame,
                        &mut call_original,
                    ))
                },
            );
            // the streams, compared on their own as well
            assert_eq!(
                render_streams(&p.a, rd, len),
                render_streams(&p.b, rd, len),
                "{}: streams",
                p.name
            );
        }
    }
}

#[test]
fn md_spr_init_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_9940);
        let (obj, init, work) = (p.alloc(0x40), p.alloc(0x60), p.alloc(0x400));
        let (g1, g2) = (p.sym(0x011a_0740), p.sym(0x011a_07f0));
        for _ in 0..3000 {
            p.fill(obj, 0x40, &mut rng);
            p.fill(init, 0x60, &mut rng);
            p.fill(work, 0x400, &mut rng);
            p.fill(g1, 0x10, &mut rng);
            p.fill(g2, 0x10, &mut rng);
            for at in [0x38u32, 0x3c] {
                if rng.chance(2) {
                    p.set(init + at, 0u32);
                }
            }
            if rng.chance(3) {
                p.set(init + 8, 0u16);
            }
            p.check("op_md_spr_init_x64", 0x00a6_9940, &[obj, init, work], |o| {
                Some(md::bridge::op_md_spr_init_x64(o, obj, init, work))
            });
        }
    }
}

#[test]
fn node_requeue_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a3_9f00);
        let (pool, n, set) = (p.alloc(0x400), p.alloc(0x40), p.alloc(0x400));
        for _ in 0..3000 {
            // every word a pointer within the pool (4-aligned, room for
            // the +0x68 reads), the node's links too; now and then the node
            // itself
            for k in 0..0x100u32 {
                let x = if rng.chance(20) {
                    n
                } else {
                    pool + 4 * rng.below(0xe0)
                };
                p.set(pool + 4 * k, x);
            }
            p.fill(n, 0x40, &mut rng);
            for at in [0x18u32, 0x1c, 0x20] {
                p.set(n + at, pool + 4 * rng.below(0xe0));
            }
            // links that alias the node's own (next + 0x1c at its + 0x20)
            for at in [0x1cu32, 0x20] {
                if rng.chance(6) {
                    p.set(n + at, n + rng.pick(&[0u32, 4]));
                }
            }
            let state = rng.pick(&[0x2000_0000u32, 0x1000_0000, 0x2fff_ffff, 0x3000_0000]);
            let low = rng.next() & 0x0fff_ffff;
            p.set(n + 0x30, state | low);
            p.check("0x00a39f00", 0x00a3_9f00, &[n], |o| {
                Some(small::bridge::op_node_requeue(o, n))
            });
            // the set's initialisation
            p.fill(set, 0x400, &mut rng);
            let (count, v) = (rng.pick(&[0u32, 1, 2, 7, 9, u32::MAX]), rng.next());
            p.check("0x00a39bc0", 0x00a3_9bc0, &[set, count, v], |o| {
                Some(small::bridge::op_node_pool_init(o, set, count, v))
            });
        }
    }
}

#[test]
fn osu_and_hsv_helpers_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0080_1200);
        let (out, set, dst, src) = (
            p.alloc(0x100),
            p.alloc(0x40),
            p.alloc(0x100),
            p.alloc(0x100),
        );
        let r0 = p.sym(0x00c8_f508);
        let (hsv, cols) = (p.alloc(0x100), p.alloc(0x80));
        for _ in 0..3000 {
            p.fill(out, 0x100, &mut rng);
            p.fill(r0, 0x10, &mut rng);
            p.check("opus_osu_init", 0x0080_1200, &[out], |o| {
                Some(opus::bridge::opus_osu_init(o, out))
            });
            p.fill(dst, 0x100, &mut rng);
            p.fill(src, 0x100, &mut rng);
            p.fill(set, 0x40, &mut rng);
            if rng.chance(2) {
                p.set(set + 0x20, src - 0x100 * rng.below(8));
            }
            p.check("opus_osu_copy", 0x0080_0eb0, &[set, dst, src], |o| {
                Some(opus::bridge::opus_osu_copy(o, set, dst, src))
            });
            p.fill(hsv, 0x100, &mut rng);
            p.fill(cols, 0x80, &mut rng);
            for k in 0..0x20u32 {
                match rng.below(6) {
                    0 => p.set(cols + 4 * k, 0u32),
                    1 => {
                        let v = u8::try_from(rng.below(256)).unwrap();
                        let a = u8::try_from(rng.below(256)).unwrap();
                        p.set(cols + 4 * k, [v, v, v, a]);
                    }
                    2 => p.set(cols + 4 * k, rng.next() & 0xff00_0000),
                    _ => {}
                }
            }
            let n = rng.pick(&[0u32, 1, 5, 0x20, u32::MAX]);
            p.check("0x00807e90", 0x0080_7e90, &[hsv, cols, n], |o| {
                Some(small::bridge::op_rgb_to_hsv(o, hsv, cols, n))
            });
        }
    }
}

/// A test-only access budget for the bridges' walker loops: `M` through
/// at most `left` reads and writes, then a panic, so a mutation that walks
/// off the data fails at once instead of timing out. The ports themselves
/// carry no cap.
struct Capped<'a, M> {
    m: &'a mut M,
    left: std::cell::Cell<u32>,
}

impl<'a, M: Mem> Capped<'a, M> {
    fn new(m: &'a mut M) -> Self {
        Self {
            m,
            left: std::cell::Cell::new(100_000),
        }
    }

    fn spend(&self) {
        let n = self.left.get();
        assert!(n > 0, "the walker's access budget ran out");
        self.left.set(n - 1);
    }
}

impl<M: Mem> Mem for Capped<'_, M> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.spend();
        self.m.read_bytes(addr, out);
    }

    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        self.spend();
        self.m.write_bytes(addr, data);
    }

    fn image_base(&self) -> u32 {
        self.m.image_base()
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn round16_helpers_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0088_88e0);
        let obj = p.alloc(0x100);
        let table = p.alloc(0x300);
        let run = p.sym(0x00d2_c260);
        let packet = p.alloc(0x400);
        let (src, buf, out) = (p.alloc(0x20), p.alloc(0x40), p.alloc(0x48));
        for _ in 0..3000 {
            // 0x008888e0
            p.fill(obj, 0x100, &mut rng);
            p.check("0x008888e0", 0x0088_88e0, &[obj], |o| {
                Some(small::bridge::op_slot_bytes_reset(o, obj))
            });
            // 0x0072a760
            p.fill(table, 0x300, &mut rng);
            p.fill(run, 0x400, &mut rng);
            let count = rng.pick(&[0u16, 1, 3, 8]);
            p.set(table + 4, count);
            for k in 0..8u32 {
                p.set(
                    table + 0x64 + 0x50 * k,
                    u16::try_from(rng.below(6)).unwrap(),
                );
                if rng.chance(3) {
                    p.set(run + 0x80 * k, 0xffff_f000u32);
                }
                p.set(run + 0x80 * k + 0x1d, rng.pick(&[0u8, 1, 2]));
            }
            p.put(fp::bridge::DATA, if rng.chance(8) { 0 } else { table });
            let id = u16::try_from(rng.below(7)).unwrap();
            let idw = u32::from(id) | (rng.next() << 16);
            p.check("0x0072a760", 0x0072_a760, &[idw], |o| {
                Some(fp::bridge::fp_id_state(&Capped::new(o), id))
            });
            // 0x00a698c0
            p.fill(packet, 0x400, &mut rng);
            let mut t = packet + 0x20;
            for _ in 0..rng.below(5) {
                let kind = u8::try_from(rng.below(8)).unwrap();
                let hi = rng.pick(&[0x00u8, 0x30, 0x80, 0xe0]);
                p.set(t + 1, hi | kind);
                let n = rng.below(4);
                p.set(t + 2, u16::try_from(n).unwrap());
                let sizes = [0x14u32, 0x1c, 0x20, 0x28, 0x18, 0x20, 0x28, 0x30];
                t += 0x10 + n * sizes[usize::from(kind)];
            }
            let last = rng.pick(&[0x10u8, 0x20, 0xf0, 0x1f, 0xf7]);
            p.set(t + 1, last);
            p.check("0x00a698c0", 0x00a6_98c0, &[packet], |o| {
                Some(op::bridge::op_md_packet_size(&Capped::new(o), packet))
            });
            // 0x007e1cc0
            for i in 0..0x40u32 {
                let b = match rng.below(12) {
                    0 => 0u8,
                    1 => 0xff,
                    2 => 0xd,
                    3 => 0xa,
                    4 | 5 => u8::try_from(0x80 + rng.below(0x7f)).unwrap(),
                    _ => u8::try_from(0x20 + rng.below(0x5f)).unwrap(),
                };
                p.set(buf + i, b);
            }
            p.fill(out, 0x48, &mut rng);
            let end = rng.below(0x41);
            let pos = if rng.chance(10) {
                (-1i32).cast_unsigned()
            } else {
                rng.below(end + 1)
            };
            p.set(src, buf);
            p.set(src + 0xc, pos);
            p.set(src + 0x10, end);
            let max = rng.pick(&[0u32, 1, 2, 8, 0x40]);
            p.check("0x007e1cc0", 0x007e_1cc0, &[src, out, max], |o| {
                Some(op::bridge::op_read_line(&mut Capped::new(o), src, out, max))
            });
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn round17_helpers_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007e_2660);
        let (src, buf, lines) = (p.alloc(0x20), p.alloc(0x40), p.alloc(4));
        let counter = p.sym(0x0133_0278);
        let shp = p.alloc(0x200);
        let render = p.sym(0x01a8_58b4);
        let (obj, work, vecs) = (p.alloc(0x40), p.alloc(0x400), p.alloc(0x200));
        let clut = p.sym(0x011a_01a0);
        for _ in 0..3000 {
            // 0x007e2660
            for i in 0..0x40u32 {
                let b = match rng.below(11) {
                    0 => 0u8,
                    1 => 0xff,
                    2 | 3 => 0xa,
                    9 => 0xd,
                    4 | 5 => u8::try_from(0x80 + rng.below(0x7f)).unwrap(),
                    _ => u8::try_from(0x20 + rng.below(0x5f)).unwrap(),
                };
                p.set(buf + i, b);
            }
            let end = rng.below(0x41);
            let pos = if rng.chance(10) {
                (-1i32).cast_unsigned()
            } else {
                rng.below(end + 1)
            };
            p.set(src, buf);
            p.set(src + 0xc, pos);
            p.set(src + 0x10, end);
            let l = if rng.chance(4) { 0 } else { lines };
            p.check("0x007e2660", 0x007e_2660, &[src, l], |o| {
                Some(op::bridge::op_line_length(&mut Capped::new(o), src, l))
            });
            // 0x00888890
            let r9 = rng.next();
            let n = rng.pick(&[0u32, 1, 0x7fff_ffff, 0x8000_0000, r9]);
            p.set(counter, rng.next());
            p.check("0x00888890", 0x0088_8890, &[n], |o| {
                Some(small::bridge::op_counter_add(o, n))
            });
            // getAlphaForShpIgnoreAnFormatBaseOnSh
            p.fill(shp, 0x200, &mut rng);
            p.set(shp + 0xf, rng.pick(&[3u8, 0, 2, 4]));
            p.set(shp + 4, rng.below(0x1f0));
            p.check("getAlphaForShp", 0x0071_6550, &[shp], |o| {
                Some(small::bridge::shp_alpha(o, shp))
            });
            // 0x00a3ba10
            let base = rng.next();
            p.set(render, base);
            let r9 = rng.next();
            let node = base.wrapping_add(rng.pick(&[0x438u32, 0x470, 0x438 + 0x38 * 9 + 5, 0, r9]));
            p.check("0x00a3ba10", 0x00a3_ba10, &[node], |o| {
                Some(small::bridge::op_node_slot(o, node))
            });
            // 0x00a3c0f0
            let (r9, r8) = (rng.next(), rng.next());
            let (w, h) = (
                rng.pick(&[1u32, 2, 256, 0, (-4i32).cast_unsigned(), r9]),
                rng.pick(&[1u32, 3, 512, 0, (-1i32).cast_unsigned(), r8]),
            );
            let lv = rng.pick(&[0u32, 1, 4, 12, (-2i32).cast_unsigned()]);
            p.check("0x00a3c0f0", 0x00a3_c0f0, &[w, h, lv], |_| {
                Some(small::bridge::op_mip_bytes(w, h, lv))
            });
            // 0x00a6cf70
            p.fill(vecs, 0x200, &mut rng);
            // now and then over the count itself, which then reads 0
            let at = if rng.chance(6) { obj } else { vecs };
            p.set(work + 0x390, at);
            p.set(obj + 0x14, rng.pick(&[0i16, 1, 7, 0x20, -3]));
            let junk = rng.next();
            p.check("0x00a6cf70", 0x00a6_cf70, &[obj, junk, work], |o| {
                Some(md::bridge::op_md_white_vectors(
                    &mut Capped::new(o),
                    obj,
                    work,
                ))
            });
            // 0x007e5680
            p.fill(clut, 0x400, &mut rng);
            p.check("0x007e5680", 0x007e_5680, &[], |o| {
                Some(op::bridge::op_oef2_tex_clut_init(o))
            });
        }
    }
}

#[test]
fn round18_helpers_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0074_9050);
        let (out, v) = (p.alloc(0x10), p.alloc(0x10));
        let chr = p.sym(0x012a_22c0);
        let (set, list) = (p.alloc(0x40), p.alloc(0x40));
        for _ in 0..3000 {
            // 0x00749050
            p.fill(out, 0x10, &mut rng);
            p.fill(v, 0x10, &mut rng);
            p.check("0x00749050", 0x0074_9050, &[out, v], |o| {
                Some(vec::bridge::ivec4_shr7_to_fvec4(o, out, v))
            });
            // 0x007fead0
            for i in 0..240u32 {
                p.set(chr + 0x20 * i, if rng.chance(30) { rng.next() } else { 0 });
                let d = if rng.chance(40) {
                    rng.below(0x100)
                } else {
                    0x1234
                };
                p.set(chr + 0x20 * i + 0xe, u16::try_from(d).unwrap());
            }
            let (r1, r2, r3) = (rng.below(0x100), rng.next(), rng.next() & 0x0fff_ffff);
            let c = rng.pick(&[r1, r2, 0x1000_0005, r3]);
            p.check("0x007fead0", 0x007f_ead0, &[c], |o| {
                Some(et::bridge::op_et_chr_slot(&Capped::new(o), c))
            });
            // 0x00801780
            p.fill(list, 0x40, &mut rng);
            for i in 0..0x20u32 {
                let x: i16 = if rng.chance(8) {
                    -1
                } else {
                    i16::try_from(rng.below(0x100)).unwrap()
                };
                p.set(list + 2 * i, x);
            }
            p.fill(set, 0x40, &mut rng);
            let k = rng.below(4);
            p.set(set + 0x10 + 4 * k, list);
            p.set(set + 0x34, rng.pick(&[0u16, 1, 2, 9, 0x20, 0x21]));
            let id = rng.next();
            p.check("0x00801780", 0x0080_1780, &[set, id, k], |o| {
                #[allow(clippy::cast_possible_truncation)]
                Some(opus::bridge::opus_list_add(
                    &mut Capped::new(o),
                    set,
                    id as u16,
                    k,
                ))
            });
        }
    }
}

/// Code padding after `.text` for `opu_main_host`'s stand-in handlers.
const HOST_HANDLERS: u32 = 0x00b0_bb00;

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn opu_main_host_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0080_cc00);
        // the SPR stack: one block from the bottom
        let spr = p.alloc(0x600);
        p.put(op::bridge::SPR_TOP, 0i8);
        let stack = p.sym(op::bridge::SPR_STACK.sym);
        p.set(stack, spr);
        // MsBattleChrStart from a cell; dbgPrintf recorded
        let cells = p.alloc(0x20);
        deref_recorder_with(&mut p, 0x0078_0cb0, &[], Some(cells));
        dbg_printf_recorder(&mut p, 0);
        // the handlers: next (+2), done (+0x210 from a cell, +2), zero
        // (done, null) and the motion callback (recorded)
        let base = p.sym(HOST_HANDLERS);
        let (h_next, h_done, h_zero, h_cb) = (base, base + 0x10, base + 0x30, base + 0x50);
        let mut code = vec![0u8; 0x80];
        code[..8].copy_from_slice(&[0x8b, 0x44, 0x24, 0x08, 0x83, 0xc0, 0x02, 0xc3]);
        let mut d = vec![0x8b, 0x4c, 0x24, 0x04, 0x8a, 0x15];
        d.extend((cells + 4).to_le_bytes());
        d.extend([
            0x88, 0x91, 0x10, 0x02, 0, 0, 0x8b, 0x44, 0x24, 0x08, 0x83, 0xc0, 0x02, 0xc3,
        ]);
        code[0x10..0x10 + d.len()].copy_from_slice(&d);
        let z = [
            0x8b, 0x4c, 0x24, 0x04, 0xc6, 0x81, 0x10, 0x02, 0, 0, 1, 0x31, 0xc0, 0xc3,
        ];
        code[0x30..0x30 + z.len()].copy_from_slice(&z);
        let mut cb = vec![0x8b, 0x44, 0x24, 0x08, 0xa3];
        cb.extend((cells + 8).to_le_bytes());
        cb.extend([0xff, 0x05]);
        cb.extend((cells + 0xc).to_le_bytes());
        cb.push(0xc3);
        code[0x50..0x50 + cb.len()].copy_from_slice(&cb);
        p.a.write(base, &code).unwrap();
        p.b.write(base, &code).unwrap();
        let table = p.sym(0x00c4_8ec8);
        let overlay = p.alloc(0x400);
        for i in 0..0x100u32 {
            // a zero command ends the slot, so a walk off the programs
            // stops in zeroed memory
            let h = match i {
                1..0xa0 => h_next,
                0xa0..0xf0 => h_done,
                0 | 0xf0..0xfe => h_zero,
                _ => 0,
            };
            p.set(table + 4 * i, h);
            // the overlay table has no null check: every entry set
            p.set(overlay + 4 * i, if h == 0 { h_zero } else { h });
        }
        let cbs = p.sym(0x00c4_92c8);
        for i in 0..12u32 {
            p.set(cbs + 4 * i, h_cb);
        }
        let ob = p.sym(et::bridge::OPUS_BATTLE.sym);
        let (b, e, orders, units, progs) = (
            p.alloc(0x100),
            p.alloc(0x80),
            p.alloc(0x40),
            p.alloc(0x800),
            p.alloc(0x800),
        );
        let (g1, g2) = (p.sym(0x0113_fcd0), p.sym(0x0113_fd10));
        let (regs, vf, r0) = (p.sym(0x00c8_f528), p.sym(0x00c8_f8d0), p.sym(0x00c8_f508));
        for round in 0..1500 {
            p.fill(b, 0x100, &mut rng);
            p.fill(g1, 0x40, &mut rng);
            p.fill(g2, 0x40, &mut rng);
            p.fill(regs, 0x40, &mut rng);
            p.fill(vf, 0x70, &mut rng);
            p.fill(r0, 0x10, &mut rng);
            // the emulated zero register, now and then not: its bytes seed
            // the host's flags (+0x210 done, +0x212 still, +0x21d second
            // pass, +0x21f motion only), but the order index (+0x218) stays
            // 0
            if rng.chance(4) {
                p.fill(r0, 0x10, &mut rng);
                let w: u32 = p.get(r0 + 8);
                p.set(r0 + 8, w & 0xffff_0000);
            } else {
                p.set(r0, [0u32; 4]);
            }
            p.set(cells, u32::from(rng.chance(2)));
            p.set(cells + 4, rng.pick(&[1u8, 2, 3, 0xff]));
            let battle = rng.below(4);
            p.set(ob + 4 * battle, b);
            p.set(b + 0x50, e);
            p.set(e + 0x7c, if rng.chance(3) { 0 } else { overlay });
            // four orders of up to five units, ending at -1
            for l in 0..4u32 {
                let o = orders + 0x10 * l;
                p.set(b + 0x10 + 4 * l, o);
                let n = rng.below(6);
                for k in 0..n {
                    p.set(o + 2 * k, u16::try_from(rng.below(8)).unwrap());
                }
                p.set(o + 2 * n, 0xffffu16);
            }
            p.set(b + 0x20, units);
            p.fill(units, 0x800, &mut rng);
            p.fill(progs, 0x800, &mut rng);
            for k in 0..8u32 {
                let u = units + 0x100 * k;
                for s in 0..4u32 {
                    let at = if rng.chance(4) {
                        0
                    } else {
                        progs + 0x200 * s + 2 * rng.below(0x40)
                    };
                    p.set(u + 4 * s, at);
                    p.set(u + 0x10 + s, rng.pick(&[0u8, 0, 1, 2]));
                }
                p.set(u + 0x1f, u8::try_from(rng.below(0x100)).unwrap());
                for i in 0..28u32 {
                    let x = light_value(&mut rng);
                    p.set(u + 0x30 + 4 * i, x);
                }
                p.set(u + 0xb2, u8::try_from(rng.below(8)).unwrap());
                p.set(u + 0xba, u8::try_from(rng.below(14)).unwrap());
            }
            p.put(op::bridge::SPR_TOP, 0i8);
            let list = rng.below(4)
                | if rng.chance(4) {
                    rng.next() & 0xffff_ff00
                } else {
                    0
                };
            let bw = battle | if round % 7 == 0 { 0xabcd_0000 } else { 0 };
            p.check("opu_main_host", 0x0080_cc00, &[bw, list], |o| {
                Some(opus::bridge::opu_main_host(
                    &mut Capped::new(o),
                    bw,
                    list,
                    &mut |c: &mut Capped<Oracle>, f, a| call_original(c.m, f, a),
                ))
            });
        }
    }
}

/// A float in `[lo, hi)` with 16 fraction bits, as bits.
fn ranged(rng: &mut Xorshift, lo: f32, hi: f32) -> u32 {
    #[allow(clippy::cast_precision_loss)]
    let t = rng.below(0x1_0000) as f32 / 65536.0;
    (lo + (hi - lo) * t).to_bits()
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave_omd_make_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007f_94e0);
        // MSVCR110's own _CIatan2 (op_deg_get)
        let (obj, wv, uv) = (p.alloc(0x40), p.alloc(0x60), p.alloc(8));
        let omd = p.alloc(0x8000);
        let (view, light) = (p.alloc(0x100), p.alloc(0x60));
        let (col, lpos, lm) = (p.alloc(0x10), p.alloc(0x10), p.alloc(0x40));
        for _ in 0..40 {
            p.fill(omd, 0x8000, &mut rng);
            p.fill(obj, 0x40, &mut rng);
            p.fill(wv, 0x60, &mut rng);
            p.fill(uv, 8, &mut rng);
            let x = ranged(&mut rng, 0.5, 20.0);
            p.set(obj + 0x30, x);
            let x = ranged(&mut rng, -0.5, 1.0);
            p.set(obj + 0x34, x);
            for (o, lo, hi) in [
                (0x30u32, 0.0f32, 4096.0f32),
                (0x34, -200.0, 200.0),
                (0x38, -10.0, 10.0),
                (0x40, -30.0, 30.0),
                (0x44, -3.0, 3.0),
                (0x48, -0.5, 0.5),
            ] {
                let x = ranged(&mut rng, lo, hi);
                p.set(wv + o, x);
            }
            // the view matrices: a scaled rotation-ish and a translation
            for m in [0x20u32, 0x60, 0xa0] {
                for i in 0..16u32 {
                    let x = if i % 5 == 0 {
                        ranged(&mut rng, 0.5, 1.5)
                    } else if (12..15).contains(&i) {
                        ranged(&mut rng, -500.0, 500.0)
                    } else if i == 15 {
                        1.0f32.to_bits()
                    } else {
                        ranged(&mut rng, -0.5, 0.5)
                    };
                    p.set(view + m + 4 * i, x);
                }
            }
            p.set(light + 0x24, col);
            p.set(light + 0x28, lpos);
            p.set(light + 0x2c, lm);
            // now and then colours far past a byte (the conversion's
            // overflow)
            let hot = rng.chance(4);
            for i in 0..4u32 {
                let x = if hot {
                    ranged(&mut rng, -3e9, 3e9)
                } else {
                    ranged(&mut rng, 0.0, 2.0)
                };
                p.set(col + 4 * i, x);
                let x = if i == 3 {
                    ranged(&mut rng, 0.0, 50_000.0)
                } else {
                    ranged(&mut rng, -400.0, 400.0)
                };
                p.set(lpos + 4 * i, x);
            }
            for i in 0..16u32 {
                let x = if i % 5 == 0 {
                    1.0f32.to_bits()
                } else if (12..15).contains(&i) {
                    ranged(&mut rng, -100.0, 100.0)
                } else {
                    0
                };
                p.set(lm + 4 * i, x);
            }
            p.set(light + 0x4c, u8::try_from(rng.below(256)).unwrap());
            p.set(light + 0x4d, u8::try_from(rng.below(256)).unwrap());
            check_deep_n(
                &mut p,
                "opus_wave_omd_make",
                0x007f_94e0,
                &[obj, wv, uv, omd, view, light],
                |o, frame| {
                    Some(wave::bridge::opus_wave_omd_make(
                        o,
                        obj,
                        wv,
                        uv,
                        omd,
                        view,
                        light,
                        frame,
                        &mut call_original,
                        &mut call_original_f,
                    ))
                },
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn shape_con_des_match_the_original() {
    use draw::bridge as d;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0074_2b50);
        // graphicVFXDestroy and pppMemFree recorded; the level hash from a
        // cell
        let cells = p.alloc(0x10);
        deref_recorder(&mut p, 0x0064_5280, &[0, 0]);
        deref_recorder(&mut p, 0x0071_6a90, &[0, 0, 0]);
        deref_recorder_with(&mut p, 0x0064_1da0, &[], Some(cells));
        let (obj, prog, table, shape, sptr) = (
            p.alloc(0x200),
            p.alloc(0x20),
            p.alloc(0x10),
            p.alloc(0x40),
            p.alloc(4),
        );
        let env = p.alloc(0x40);
        let slot = p.sym(d::ENV_SLOT);
        let cur = p.sym(d::CURRENT_PROGRAM);
        let cons = [
            (0x0074_2b50u32, d::FIELD_CON),
            (0x0074_3f00, d::FIELD_REV_CON),
            (0x0074_50a0, d::FIELD_SPD_CON),
            (0x0074_6200, d::FIELD_GLOBAL_CON),
            (0x0074_7410, d::CAMERA_CON),
            (0x0074_8660, d::CAMERA_CON),
        ];
        let dess = [
            (0x0074_2bd0u32, d::FIELD_DES),
            (0x0074_3f80, d::FIELD_REV_DES),
            (0x0074_5130, d::FIELD_SPD_DES),
            (0x0074_6280, d::FIELD_GLOBAL_DES),
            (0x0074_7490, d::CAMERA_DES),
            (0x0074_86e0, d::CAMERA_DES),
            (0x0075_6270, d::RAIN_DES),
        ];
        for _ in 0..500 {
            p.fill(obj, 0x200, &mut rng);
            p.fill(shape, 0x40, &mut rng);
            p.set(prog + 0xc, table);
            p.set(table, 4 * rng.below(0x10));
            p.set(table + 4, 4 * rng.below(0x10));
            p.set(obj + 8, sptr);
            p.set(sptr, shape);
            p.set(slot, env);
            let r9 = rng.next();
            p.set(cur, rng.pick(&[0u32, 3, 0x8000_0001, r9]));
            p.set(cells, rng.pick(&[0x76fu32, 0x1139, 0, 0x100]));
            for (at, l) in cons {
                p.check("pppDrawShape*Con", at, &[obj, prog], |o| {
                    Some(d::shape_con(o, obj, prog, l, &mut call_original))
                });
            }
            // the freed pointers: null or not
            for off in [0xa4u32, 0xe0, 0xe4, 0xec, 0xf0, 0xf4] {
                let t: u32 = p.get(table);
                let x = if rng.chance(3) { 0 } else { rng.next() };
                p.set(obj + t + off, x);
                let t1: u32 = p.get(table + 4);
                let y = if rng.chance(3) { 0 } else { rng.next() };
                p.set(obj + t1 + off, y);
            }
            for (at, l) in dess {
                p.check("ppp*Des", at, &[obj, prog], |o| {
                    Some(d::ppp_des(o, obj, prog, l, &mut call_original))
                });
            }
        }
    }
}

#[test]
fn srand_draws_match_the_original() {
    use rand::bridge::{self as r, Spread};
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0073_26e0);
        let log = record_srand(&mut p);
        let (rg, seeded, r0) = (p.sym(r::R.sym), p.sym(r::SEEDED.sym), p.sym(0x00c8_f508));
        for _ in 0..2000 {
            let x = (rng.next() & 0x007f_ffff) | 0x3f80_0000;
            p.set(rg, x);
            p.set(
                seeded,
                u32::from(rng.chance(4)).wrapping_sub(1) & rng.next(),
            );
            p.fill(r0, 0x10, &mut rng);
            // r0 - 1 never a signalling NaN (the oracle keeps those)
            let t0 = if rng.chance(50) {
                u32::MAX
            } else {
                rng.float()
            };
            p.set(r0, t0.wrapping_add(1));
            let two = u32::from(rng.chance(2)) | (rng.next() & 0xffff_ff00);
            p.set(log, 0u32);
            for (at, spread) in [
                (0x0073_26e0u32, Spread::Plain),
                (0x0073_2f30, Spread::Plain),
                (0x0073_2970, Spread::Up),
                (0x0073_2c00, Spread::Down),
            ] {
                check_st0(
                    &mut p,
                    &format!("pppSRand*_Draw {at:#x}"),
                    at,
                    &[two],
                    |o| r::ppp_srand_draw(o, spread, two & 0xff != 0, &mut srand_log(log)).into(),
                );
            }
        }
    }
}

/// Points the scratch stack's top mark at `buf`.
fn spr_at(p: &mut Pair, rng: &mut Xorshift, buf: u32) {
    use op::bridge as o;
    let top = i8::try_from(rng.below(4)).unwrap();
    p.put(o::SPR_TOP, top);
    let at = p.sym(o::SPR_STACK.sym) + 4 * u32::from(top.cast_unsigned());
    p.set(at, buf);
}

#[test]
fn tk_fade_draw_matches_the_original() {
    use draw::bridge as d;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0072_ee50);
        let (obj, key, prog, table) = (p.alloc(0x100), p.alloc(0x20), p.alloc(0x10), p.alloc(4));
        let (env, env2, recs, buf) = (p.alloc(4), p.alloc(0x40), p.alloc(0x100), p.alloc(0x500));
        let (slot, stop) = (p.sym(d::ENV_SLOT), p.sym(0x0230_fd34));
        for _ in 0..1500 {
            p.fill(obj, 0x100, &mut rng);
            p.fill(key, 0x20, &mut rng);
            p.fill(recs, 0x100, &mut rng);
            p.fill(buf, 0x500, &mut rng);
            spr_at(&mut p, &mut rng, buf);
            p.set(prog + 0xc, table);
            let off = 4 * rng.below(0x10);
            p.set(table, off);
            let w = obj + 0xa0 + off;
            // the position: mostly in the CLUT's range, sometimes far off
            for (k, (lo, hi)) in (0u32..).zip([(-300.0, 300.0), (-20.0, 20.0), (-2.0, 2.0)]) {
                let x = if rng.chance(10) {
                    rng.float()
                } else {
                    ranged(&mut rng, lo, hi)
                };
                p.set(w + 4 * k, x);
            }
            for k in 2..5u32 {
                let x = if rng.chance(10) {
                    rng.float()
                } else {
                    ranged(&mut rng, -10.0, 10.0)
                };
                p.set(key + 4 * k, x);
            }
            if rng.chance(50) {
                let f: u32 = p.get(obj + 0xc);
                p.set(key, f);
            }
            p.set(key + 4, if rng.chance(10) { 0xffff } else { rng.below(8) });
            p.set(stop, u32::from(rng.chance(20)));
            p.set(slot, env);
            p.set(env, env2);
            p.set(env2 + 0x20, recs);
            check_deep_n(
                &mut p,
                "pppKeTkFadeDraw",
                0x0072_ee50,
                &[obj, key, prog],
                |o, frame| {
                    Some(d::ppp_ke_tk_fade_draw(
                        o,
                        obj,
                        key,
                        prog,
                        frame,
                        &mut call_original,
                    ))
                },
            );
        }
    }
}

#[test]
#[allow(clippy::many_single_char_names)]
fn lns_lp_t_ot_draw_matches_the_original() {
    use draw::bridge as d;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0075_b230);
        let (obj, buf) = (p.alloc(0x100), p.alloc(0x8000));
        let (view, parity, dc) = (p.sym(0x0230_fffe), p.sym(0x023c_bbf0), p.sym(0x012f_b900));
        for _ in 0..1500 {
            p.fill(obj, 0x100, &mut rng);
            p.fill(buf, 0x8000, &mut rng);
            spr_at(&mut p, &mut rng, buf);
            let v: u16 = p.get(obj + 0xcc);
            let v2 = if rng.chance(90) { v } else { v ^ 1 };
            p.set(view, v2);
            let vis = if rng.chance(10) {
                0
            } else {
                ranged(&mut rng, 0.0, 1.0)
            };
            p.set(obj + 0x7c, vis);
            let x = rng.float();
            p.set(obj + 0xac, x);
            let (a, m) = (rng.below(0x100), rng.below(4));
            p.set(obj + 0x8c, u8::try_from(a).unwrap());
            p.set(obj + 0x8d, u8::try_from(m).unwrap());
            let frame_no = if rng.chance(70) {
                a % (m + 1) + (m + 1) * rng.below(0x100)
            } else {
                rng.next()
            };
            let par = rng.next();
            p.set(parity, par);
            // the draw buffer's scissor and offset, both halves
            let mut gss = [0u32; 2];
            for (k, half) in [0x60u32, 0x190].into_iter().enumerate() {
                let gs = dc + half;
                let (ox, oy) = (rng.below(0x8000), rng.below(0x8000));
                let (sx0, sy0) = (rng.below(0x700), rng.below(0x700));
                let (sx1, sy1) = (sx0 + rng.below(0x80), sy0 + rng.below(0x80));
                p.set(gs + 0x18, ox);
                p.set(gs + 0x1c, oy);
                p.set(gs + 0x24, sx0 | sx1 << 16);
                p.set(gs + 0x28, sy0 | sy1 << 16);
                let r = rng.next();
                p.set(gs + 2, r);
                let r = rng.next();
                p.set(gs + 0xc, r);
                gss[k] = (ox + 16 * sx0) | (oy + 16 * sy0) << 16;
            }
            p.set(obj + 0xd0, u8::try_from(rng.below(0x40)).unwrap());
            p.set(obj + 0xd1, u8::try_from(rng.below(0x40)).unwrap());
            // FFXVu0FTOI4Vector is a stub: the point is the original's
            // stale stack below its 16-aligned frame, seeded on both sides
            let n = 3;
            let frame = p.b.stack_range().end - 0x100 - 4 * n - 4 - 0x200 - 4 * n - 4 - 4;
            let at = (frame - 8) & !0xf;
            let g = gss[(par & 1) as usize];
            let dx = rng.below(0x1000);
            let dy = rng.below(0x1000);
            p.set(at - 0x14, (g & 0xffff) + dx - 0x200);
            p.set(at - 0x10, (g >> 16) + dy - 0x200);
            // sometimes exactly the first texel's z (the compare is strict)
            let t: u32 = p.get(buf + 0x60);
            let z = if rng.chance(30) {
                (t & 0x00ff_ffff) << 4
            } else {
                rng.below(0x1000_0000)
            };
            p.set(at - 0xc, z);
            check_deep_n(
                &mut p,
                "pppKeLnsLpTOtDraw",
                0x0075_b230,
                &[obj, 0, frame_no],
                |o, frame| {
                    assert_eq!((frame - 8) & !0xf, at);
                    d::ppp_ke_lns_lp_t_ot_draw(o, obj, frame_no, at, &mut call_original);
                    None
                },
            );
        }
    }
}

/// One `pppDrawMdl*` draw (`sym`) against `rust(oracle, obj, key, prog,
/// frame)`: every renderer entry point recorded (the request by its 18
/// words), the level hash, `graphicGetPtrLogCount`, the render-data
/// lookup, the alpha helpers, the magic file's handle and id and the
/// language from cells, `FFXVu0MulMatrix` recorded; the port runs with the original's frame pointer,
/// so pointers to stack locals agree. The request at `frame - req_off`
/// gets the same stale values in its untouched words (`+8..+0xc`, `+0x30`)
/// on both sides.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn mdl_case(
    name: &str,
    sym: u32,
    req_off: u32,
    rust: impl Fn(&mut Oracle, u32, u32, u32, u32) -> u32,
) {
    for mut p in Pair::all() {
        let mut rng = Xorshift(sym);
        let cells = p.alloc(0x20);
        deref_recorder_with(&mut p, 0x0064_1da0, &[], Some(cells));
        deref_recorder_with(&mut p, 0x0064_53d0, &[0, 0], Some(cells + 4));
        deref_recorder_with(&mut p, 0x0071_2c30, &[0], Some(cells + 8));
        deref_recorder_with(&mut p, 0x0071_2d00, &[0], Some(cells + 0xc));
        deref_recorder_with(&mut p, 0x0064_1370, &[], Some(cells + 0x10));
        deref_recorder_with(&mut p, 0x009d_a390, &[], Some(cells + 0x14));
        deref_recorder_with(&mut p, 0x009d_a3b0, &[], Some(cells + 0x18));
        deref_recorder_with(&mut p, 0x008a_c2f0, &[], Some(cells + 0x1c));
        deref_recorder(&mut p, 0x0064_0060, &[0, 0]);
        // FFXVu0MulMatrix(dst, a, b) recorded with a's 16 words: the
        // matrices built on the stack only reach the request as pointers
        wide_recorder(&mut p, 0x006e_d5c0, &[0, 16, 0], None);
        wide_recorder(&mut p, 0x0071_8610, &[0, 0, 0, 18], None);
        wide_recorder(&mut p, 0x0071_0fd0, &[0, 0, 0, 18, 0], None);
        wide_recorder(&mut p, 0x0071_1180, &[0, 0, 0, 18, 0], None);
        deref_recorder(&mut p, 0x0064_5600, &[0, 0]);
        deref_recorder(&mut p, 0x0064_58a0, &[0, 0]);
        let (obj, key, prog, table) = (p.alloc(0x200), p.alloc(0x40), p.alloc(0x10), p.alloc(0x10));
        let (env, env2, recs, models, part) = (
            p.alloc(4),
            p.alloc(0x40),
            p.alloc(0x400),
            p.alloc(0x80),
            p.alloc(0x80),
        );
        let slot = p.sym(draw::bridge::ENV_SLOT);
        // the view, view-copy, screen, ppp screen and world matrices
        let matrices = [
            0x0230_ff20u32,
            0x0230_fe20,
            0x0231_1480,
            0x0230_fe60,
            0x0230_ffa0,
        ]
        .map(|a| p.sym(a));
        let (pass, map, fp, yiq) = (
            p.sym(0x0113_335c),
            p.sym(0x012f_bc40),
            p.sym(0x00c3_a500),
            p.sym(0x00c3_a504),
        );
        let (cur, cpart, shift, stop) = (
            p.sym(0x0230_fd20),
            p.sym(0x0230_ffe4),
            p.sym(0x00c8_b500),
            p.sym(0x0230_fd34),
        );
        let r0 = p.sym(0x00c8_f508);
        for _ in 0..1500 {
            p.fill(obj, 0x200, &mut rng);
            p.fill(key, 0x40, &mut rng);
            p.fill(recs, 0x400, &mut rng);
            if rng.chance(50) {
                for m in matrices {
                    for o in (0..0x40).step_by(4) {
                        let x = ranged(&mut rng, -2.0, 2.0);
                        p.set(m + o, x);
                    }
                }
            }
            // models: small words (flags in the low bits, part offsets),
            // then the fields the magic-file fixes look for
            for o in (0..0x24).step_by(4) {
                let v = (4 * rng.below(0x10)) | rng.below(4);
                p.set(models + o, v);
            }
            for o in (0x24..0x80).step_by(4) {
                let r = rng.next();
                let v = rng.pick(&[0x36c0u32, 0x3700, 0x37e0, 0x4000 | 0x37e0, r]);
                p.set(models + o, v);
            }
            for o in (0x10..0x90).step_by(4) {
                let x = ranged(&mut rng, -4.0, 4.0);
                p.set(obj + o, x);
            }
            p.fill(part, 0x80, &mut rng);
            p.fill(r0, 0x10, &mut rng);
            p.set(prog + 0xc, table);
            for k in 0..3 {
                let off = 4 * rng.below(0x10);
                p.set(table + 4 * k, off);
            }
            // the Ts work area: floats, the key's frame and its floats
            let w = obj + 0xa0 + p.get::<u32>(table + 8);
            for o in (0..0x1c).step_by(4) {
                let x = if rng.chance(20) {
                    ranged(&mut rng, -70000.0, 70000.0)
                } else {
                    ranged(&mut rng, -300.0, 300.0)
                };
                p.set(w + o, x);
            }
            // the Loop draws' ptr32 at +0: null sets the work area up
            if rng.chance(30) {
                p.set(w, 0u32);
            }
            for o in (8..0x20).step_by(4) {
                let x = ranged(&mut rng, -50.0, 50.0);
                p.set(key + o, x);
            }
            if rng.chance(50) {
                let frame = p.get::<u32>(obj + 0xc);
                p.set(key, frame);
            }
            // the flag bytes of the key: small values hit every branch
            for o in [8u32, 0xa, 0x20, 0x22] {
                if rng.chance(70) {
                    p.set(key + o, u8::try_from(rng.below(8)).unwrap());
                }
            }
            if rng.chance(30) {
                p.set(key + 0x20, 0xffu8);
            }
            for o in [9u32, 0xb, 0xc, 0xd, 0xe, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26] {
                if rng.chance(50) {
                    p.set(key + o, 0u8);
                }
            }
            p.set(pass, rng.below(3));
            let m = rng.next();
            let maps = [
                0x17cu32, 0x3e, 0xc6, 0xc2, 0xff, 0x102, 0x103, 0xf4, 0x110, 0x139, 0x7a, m,
            ];
            p.set(map, rng.pick(&maps));
            let f = rng.pick(&[
                0x2cu32, 0x2d, 0x2e, 0x2f, 0x3f, 0x40, 0xe, 3, 4, 5, 0x12, 0x41,
            ]);
            p.set(fp, f);
            p.set(yiq, rng.pick(&[0x19u32, 5, 0, u32::MAX]));
            p.set(cells + 0x14, rng.pick(&[0u32, u32::MAX]));
            let r = rng.next();
            let magics = [
                0x296u32, 0x241, 0x29e, 0x29f, 0x2c9, 0x17a, 0x110, 0x111, 0x231, 0x1f9, r,
            ];
            p.set(cells + 0x18, rng.pick(&magics));
            p.set(cells + 0x1c, rng.pick(&[0xbu32, 1]));
            p.set(stop, u32::from(rng.chance(20)));
            let s = if rng.chance(20) { 4 * rng.below(4) } else { 0 };
            p.set(shift, s);
            p.set(cpart, part);
            p.set(slot, env);
            p.set(env, env2);
            p.set(env2 + 0x20, recs);
            let model = if rng.chance(5) {
                0xffff
            } else if rng.chance(30) {
                rng.pick(&[0x1au32, 0x1c, 0x1e, 0x1f])
            } else {
                rng.below(8)
            };
            p.set(key + 4, model);
            // get_ptr32 of the model record's +0x1c: a pointer into models
            for k in 0..0x20u32 {
                let o = 4 * rng.below(8);
                p.set(recs + 32 * k + 0x1c, models + o);
            }
            p.set(obj + 0x90, models + rng.below(0x20));
            p.set(obj + 0x94, models + rng.below(0x20));
            let (h1, h2) = (rng.next(), rng.next());
            let h = rng.pick(&[0xec1u32, 0x169c, h1, h2]);
            p.set(cells, h);
            p.set(cells + 4, u32::from(rng.chance(50)));
            // the alpha helpers: often no bits, so added ones show
            for c in [cells + 8, cells + 0xc] {
                let x = if rng.chance(50) { 0 } else { rng.next() };
                p.set(c, x);
            }
            let n = if rng.chance(50) { 0 } else { rng.next() };
            p.set(cells + 0x10, n);
            let r9 = rng.next();
            p.set(cur, rng.pick(&[0u32, 3, 0x8000_0001, r9]));
            let words = 3;
            let frame = p.b.stack_range().end - 0x100 - 4 * words - 4 - 0x200 - 4 * words - 4 - 4;
            let at = (frame - 8) & !0xf;
            let (s1, s2) = (rng.next(), rng.next());
            p.set(at - req_off + 8, s1);
            p.set(at - req_off + 0x30, s2);
            check_deep_n(&mut p, name, sym, &[obj, key, prog], |o, frame| {
                assert_eq!((frame - 8) & !0xf, at);
                Some(rust(o, obj, key, prog, at))
            });
        }
    }
}

#[test]
fn mdl_draw_matches_the_original() {
    use mdl::bridge as d;
    mdl_case(
        "pppDrawMdlDraw",
        0x0073_7770,
        0x80,
        |o, obj, key, prog, frame| {
            d::ppp_draw_mdl_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

#[test]
fn mdl_semi_draw_matches_the_original() {
    use mdl::bridge as d;
    mdl_case(
        "pppDrawMdlSemiDraw",
        0x0073_7b20,
        0xa0,
        |o, obj, key, prog, frame| {
            d::ppp_draw_mdl_semi_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

#[test]
fn mdl_2_draw_matches_the_original() {
    use mdl::bridge as d;
    mdl_case(
        "pppDrawMdl2Draw",
        0x0073_8530,
        0xa0,
        |o, obj, key, prog, frame| {
            d::ppp_draw_mdl_2_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

#[test]
fn mdl_semi2_draw_matches_the_original() {
    use mdl::bridge as d;
    mdl_case(
        "pppDrawMdlSemi2Draw",
        0x0073_8a40,
        0xa0,
        |o, obj, key, prog, frame| {
            d::ppp_draw_mdl_semi2_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

#[test]
fn mdl_ts_draw_matches_the_original() {
    use mdl::bridge as d;
    mdl_case(
        "pppDrawMdlTsDraw",
        0x0073_7f40,
        0xa0,
        |o, obj, key, prog, frame| {
            d::ppp_draw_mdl_ts_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

#[test]
fn mdl_3_draw_matches_the_original() {
    use mdl::bridge as d;
    mdl_case(
        "pppDrawMdl3Draw",
        0x0073_94c0,
        0x120,
        |o, obj, key, prog, frame| {
            d::ppp_draw_mdl_3_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

#[test]
fn mdl_semi3_draw_matches_the_original() {
    use mdl::bridge as d;
    mdl_case(
        "pppDrawMdlSemi3Draw",
        0x0073_9d30,
        0x120,
        |o, obj, key, prog, frame| {
            d::ppp_draw_mdl_semi3_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

#[test]
fn mdl_ts3_draw_matches_the_original() {
    use mdl::bridge as d;
    mdl_case(
        "pppDrawMdlTs3Draw",
        0x0073_a370,
        0xe0,
        |o, obj, key, prog, frame| {
            d::ppp_draw_mdl_ts3_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

#[test]
fn mdl_loop_draw_matches_the_original() {
    use mdl::bridge as d;
    mdl_case(
        "pppDrawMdlLoopDraw",
        0x0073_e2b0,
        0xe0,
        |o, obj, key, prog, frame| {
            d::ppp_draw_mdl_loop_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

#[test]
fn mdl_loop_z_draw_matches_the_original() {
    use mdl::bridge as d;
    mdl_case(
        "pppDrawMdlLoopZDraw",
        0x0073_ea40,
        0xe0,
        |o, obj, key, prog, frame| {
            d::ppp_draw_mdl_loop_z_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

#[test]
fn mdl_loop_dis_pos_draw_matches_the_original() {
    use mdl::bridge as d;
    mdl_case(
        "pppDrawMdlLoopDisPosDraw",
        0x0073_f230,
        0x1a0,
        |o, obj, key, prog, frame| {
            d::ppp_draw_mdl_loop_dis_pos_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

#[test]
fn mdl_ts2_draw_matches_the_original() {
    use mdl::bridge as d;
    mdl_case(
        "pppDrawMdlTs2Draw",
        0x0073_8ec0,
        0xa0,
        |o, obj, key, prog, frame| {
            d::ppp_draw_mdl_ts2_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

/// The `pppKeLns*` draws (`docs/formats/vfx.md`): the updates and the
/// `KeLns*` drawers recorded (descriptors by their 7 words), the vector
/// helpers real; the ports run with the original's frame pointer.
#[allow(clippy::too_many_lines)]
fn lns_case(name: &str, sym: u32, args: usize, rust: impl Fn(&mut Oracle, [u32; 3], u32) -> u32) {
    for mut p in Pair::all() {
        let mut rng = Xorshift(sym);
        for u in [0x0075_a2d0u32, 0x0075_aab0, 0x0075_ae90] {
            deref_recorder(&mut p, u, &[0, 0, 0]);
        }
        for d in [0x0076_9340u32, 0x0076_9d70, 0x0076_a1f0] {
            wide_recorder(&mut p, d, &[7, 0], None);
        }
        deref_recorder(&mut p, 0x0076_a690, &[4]);
        let (obj, key, prog, table, part) = (
            p.alloc(0x200),
            p.alloc(0x20),
            p.alloc(0x10),
            p.alloc(4),
            p.alloc(0x80),
        );
        let cpart = p.sym(0x0230_ffe4);
        for _ in 0..1500 {
            p.fill(obj, 0x200, &mut rng);
            p.fill(key, 0x20, &mut rng);
            p.fill(part, 0x80, &mut rng);
            p.set(prog + 0xc, table);
            let off = 4 * rng.below(0x10);
            p.set(table, off);
            p.set(cpart, part);
            for o in [4u32, 8, 0xc] {
                let x = ranged(&mut rng, -50.0, 50.0);
                p.set(key + o, x);
            }
            if rng.chance(50) {
                let f = p.get::<u32>(obj + 0xc);
                p.set(key, f);
            }
            // the object's matrix and position, the work area's floats
            for o in (0x10..0x50).step_by(4) {
                let x = ranged(&mut rng, -4.0, 4.0);
                p.set(obj + o, x);
            }
            let w = obj + 0xa0 + off;
            for o in (0x2c..0x40).step_by(4) {
                let x = match rng.below(4) {
                    0 => ranged(&mut rng, -70000.0, 70000.0),
                    1 => 3.0e9f32.to_bits(),
                    _ => ranged(&mut rng, -300.0, 300.0),
                };
                p.set(w + o, x);
            }
            for o in [0x9cu32, 0xa8] {
                let x = ranged(&mut rng, -50.0, 50.0);
                p.set(w + o, x);
            }
            if rng.chance(50) {
                p.set(w + 0x8f, 0u8);
            }
            let n = u8::try_from(args).unwrap();
            let frame = p.b.stack_range().end
                - 0x100
                - 4 * u32::from(n)
                - 4
                - 0x200
                - 4 * u32::from(n)
                - 4
                - 4;
            let at = (frame - 8) & !0xf;
            let three = [obj, key, prog];
            let call_args = if args == 1 { vec![w] } else { three.to_vec() };
            check_deep_n(&mut p, name, sym, &call_args, |o, frame| {
                assert_eq!((frame - 8) & !0xf, at);
                Some(rust(o, if args == 1 { [w, 0, 0] } else { three }, at))
            });
        }
    }
}

#[test]
fn lns_draws_match_the_original() {
    use lns::bridge::{self as l, Lns};
    for (kind, name, sym) in [
        (Lns::Arnd, "pppKeLnsArndDraw", 0x0075_9ff0u32),
        (Lns::Crn, "pppKeLnsCrnDraw", 0x0075_a760),
        (Lns::Fls, "pppKeLnsFlsDraw", 0x0075_ab50),
    ] {
        lns_case(name, sym, 3, |o, [obj, key, prog], frame| {
            l::ppp_ke_lns_draw(o, kind, obj, key, prog, frame, &mut call_original)
        });
    }
}

#[test]
fn lns_t_draws_match_the_original() {
    use lns::bridge::{self as l, Lns};
    for (kind, name, sym) in [
        (Lns::Arnd, "pppKeLnsArndTDraw", 0x0075_a190u32),
        (Lns::Crn, "pppKeLnsCrnTDraw", 0x0075_a900),
        (Lns::Fls, "pppKeLnsFlsTDraw", 0x0075_ad00),
    ] {
        lns_case(name, sym, 3, |o, [obj, key, prog], _| {
            l::ppp_ke_lns_t_draw(o, kind, obj, key, prog, &mut call_original)
        });
    }
}

#[test]
fn lns_t_ot_draws_match_the_original() {
    use lns::bridge::{self as l, Lns};
    for (kind, name, sym) in [
        (Lns::Arnd, "pppKeLnsArndTOtDraw", 0x0075_a250u32),
        (Lns::Crn, "pppKeLnsCrnTOtDraw", 0x0075_a9e0),
        (Lns::Fls, "pppKeLnsFlsTOtDraw", 0x0075_ade0),
    ] {
        lns_case(name, sym, 1, |o, [area, _, _], frame| {
            l::ppp_ke_lns_t_ot_draw(o, kind, area, frame, &mut call_original)
        });
    }
}

#[test]
fn lns_lp_draw_matches_the_original() {
    use lns::bridge as l;
    lns_case(
        "pppKeLnsLpDraw",
        0x0075_af80,
        3,
        |o, [obj, key, prog], frame| {
            l::ppp_ke_lns_lp_draw(o, obj, key, prog, frame, &mut call_original)
        },
    );
}

/// The `pppKeMdlTfd*` draws: `KeMdl_MkTkClut` and `KeMdl_Draw` recorded
/// (all their words), the work area's triples and the key's floats in
/// range.
fn tfd_case(name: &str, sym: u32, rust: impl Fn(&mut Oracle, u32, u32, u32) -> u32) {
    for mut p in Pair::all() {
        let mut rng = Xorshift(sym);
        deref_recorder(&mut p, 0x0076_7530, &[0, 0, 0, 0]);
        deref_recorder(&mut p, 0x0076_6ff0, &[0; 10]);
        let (obj, key, prog, table, part) = (
            p.alloc(0x200),
            p.alloc(0x40),
            p.alloc(0x10),
            p.alloc(0x10),
            p.alloc(0x80),
        );
        let (cpart, stop) = (p.sym(0x0230_ffe4), p.sym(0x0230_fd34));
        for _ in 0..1500 {
            p.fill(obj, 0x200, &mut rng);
            p.fill(key, 0x40, &mut rng);
            p.fill(part, 0x80, &mut rng);
            p.set(prog + 0xc, table);
            for k in 0..4 {
                let off = 4 * rng.below(0x10);
                p.set(table + 4 * k, off);
            }
            p.set(cpart, part);
            p.set(stop, u32::from(rng.chance(20)));
            let w = obj + 0xa0 + p.get::<u32>(table + 8);
            for o in (0..0x28).step_by(4) {
                let x = ranged(&mut rng, -300.0, 300.0);
                p.set(w + o, x);
            }
            for o in (8..0x38).step_by(4) {
                if o != 0x1c {
                    let x = ranged(&mut rng, -50.0, 50.0);
                    p.set(key + o, x);
                }
            }
            if rng.chance(50) {
                let f = p.get::<u32>(obj + 0xc);
                p.set(key, f);
            }
            if rng.chance(10) {
                p.set(key + 4, 0xffffu32);
            }
            for o in (0x1d..0x22).chain(0x38..0x3d) {
                if rng.chance(60) {
                    p.set(key + o, 0u8);
                }
            }
            check_conv(&mut p, name, sym, Conv::Cdecl, &[obj, key, prog], |o| {
                Some(rust(o, obj, key, prog))
            });
        }
    }
}

#[test]
fn ke_mdl_tfd_draws_match_the_original() {
    use mdl::bridge::{self as d, Tfd};
    for (kind, uv, name, sym) in [
        (Tfd::One, false, "pppKeMdlTfdDraw", 0x0074_9810u32),
        (Tfd::Two, false, "pppKeMdlTfd2Draw", 0x0074_9b40),
        (Tfd::Three, false, "pppKeMdlTfd3Draw", 0x0074_9e90),
        (Tfd::One, true, "pppKeMdlTfdUvDraw", 0x0074_9950),
        (Tfd::Two, true, "pppKeMdlTfdUv2Draw", 0x0074_9c90),
        (Tfd::Three, true, "pppKeMdlTfdUv3Draw", 0x0074_9ff0),
    ] {
        tfd_case(name, sym, |o, obj, key, prog| {
            d::ppp_ke_mdl_tfd_draw(o, kind, uv, obj, key, prog, &mut call_original)
        });
    }
}

#[test]
fn fp_set_draw_matrix_and_wood_sta_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0072_a260);
        // view, camera, world, the field wood matrix, the view copy
        let matrices = [
            0x0230_ff20u32,
            0x0230_ff60,
            0x0230_ffa0,
            0x0230_fea0,
            0x0230_fe20,
        ]
        .map(|a| p.sym(a));
        for _ in 0..1000 {
            for m in matrices {
                for o in (0..0x40).step_by(4) {
                    let x = ranged(&mut rng, -2.0, 2.0);
                    p.set(m + o, x);
                }
            }
            let r = rng.next();
            let mode = rng.pick(&[0u32, 1, 2, 3, 0x101, 0x102, r]);
            check_conv(
                &mut p,
                "pppFpSetDrawMatrix",
                0x0072_a260,
                Conv::Cdecl,
                &[mode],
                |o| Some(fp::bridge::ppp_fp_set_draw_matrix(o, mode)),
            );
            // a plain frame: ebp is check_deep_n's frame
            check_deep_n(
                &mut p,
                "pppDrawMatrixWoodSta",
                0x0073_4d90,
                &[],
                |o, frame| Some(mat::bridge::ppp_draw_matrix_wood_sta(o, frame)),
            );
        }
    }
}

/// A thunk, `fld qword [esp + 4]; jmp [slot]`, through `_CItan`'s import
/// slot (`0x00b0c3a0`, MSVCR110's own `_CItan` in the oracle's runtime), for
/// the bridges' `tan` callback; returns the thunk.
fn tan_thunk(p: &mut Pair) -> u32 {
    let slot = p.sym(0x00b0_c3a0);
    let t = p.sym(0x00b0_bba0);
    let mut c = vec![0xdd, 0x44, 0x24, 0x04, 0xff, 0x25]; // fld qword [esp+4]; jmp [slot]
    c.extend(slot.to_le_bytes());
    p.a.write(t, &c).unwrap();
    p.b.write(t, &c).unwrap();
    t
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave_0154_omd_make_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007f_67c0);
        // MSVCR110's own _CIatan2 (op_deg_get) and _CItan
        let tan = tan_thunk(&mut p);
        let fov = p.alloc(4);
        float_stub(&mut p, 0x0063_9570, fov);
        let (obj, wv, uv) = (p.alloc(0x40), p.alloc(0x60), p.alloc(8));
        let omd = p.alloc(0x14800);
        let (view, proj) = (p.alloc(0x40), p.alloc(0x40));
        let shift = p.sym(0x00c8_b500);
        for _ in 0..30 {
            p.fill(omd, 0x14800, &mut rng);
            p.fill(obj, 0x40, &mut rng);
            p.fill(wv, 0x60, &mut rng);
            p.fill(uv, 8, &mut rng);
            let x = ranged(&mut rng, 0.5, 20.0);
            p.set(obj + 0x30, x);
            let x = ranged(&mut rng, -0.5, 1.0);
            p.set(obj + 0x34, x);
            for (o, lo, hi) in [
                (0x30u32, 0.0f32, 4096.0f32),
                (0x34, -200.0, 200.0),
                (0x38, -10.0, 10.0),
                (0x3c, 0.0, 512.0),
                (0x40, -30.0, 30.0),
                (0x44, -3.0, 3.0),
                (0x48, -0.5, 0.5),
                (0x4c, -40.0, 40.0),
            ] {
                let x = ranged(&mut rng, lo, hi);
                p.set(wv + o, x);
            }
            for m in [view, proj] {
                for i in 0..16u32 {
                    let x = if i % 5 == 0 {
                        ranged(&mut rng, 0.5, 1.5)
                    } else if i == 14 {
                        ranged(&mut rng, 200.0, 900.0)
                    } else if (12..14).contains(&i) {
                        ranged(&mut rng, -200.0, 200.0)
                    } else if i == 15 {
                        1.0f32.to_bits()
                    } else {
                        ranged(&mut rng, -0.3, 0.3)
                    };
                    p.set(m + 4 * i, x);
                }
            }
            let f = ranged(&mut rng, 0.3, 1.5);
            p.set(fov, f);
            p.set(shift, rng.pick(&[0u32, 0, 4, 0xc, 0x10]));
            check_deep_n(
                &mut p,
                "opus_0154_wave_omd_make",
                0x007f_67c0,
                &[obj, wv, uv, omd, view, proj],
                |o, frame| {
                    Some(wave::bridge::opus_0154_wave_omd_make(
                        o,
                        obj,
                        wv,
                        uv,
                        omd,
                        view,
                        proj,
                        frame,
                        &mut call_original,
                        &mut call_original_f,
                        &mut |m, x| {
                            let b = x.to_bits();
                            #[allow(clippy::cast_possible_truncation)]
                            let w = [b as u32, (b >> 32) as u32];
                            call_original_f(m, tan, &w)
                        },
                    ))
                },
            );
        }
    }
}

/// `fcos`/`fsin` stand-ins for the CRT's `_CIcos`/`_CIsin` (their import
/// slots `0x00b0c3b4`, `0x00b0c3b8` pointed at `0x00b0bfc0`, `0x00b0bfc8`)
/// and a thunk at [`THUNK`], `fld dword [esp + 4]; jmp [slot]`, for the
/// bridge's side; returns the thunk for cos and sin.
fn trig_stand_ins(p: &mut Pair) -> (u32, u32) {
    let mut thunks = [0u32; 2];
    for (k, (slot, code, at)) in [
        (0x00b0_c3b4u32, [0xd9u8, 0xff, 0xc3], 0x00b0_bfc0u32),
        (0x00b0_c3b8, [0xd9, 0xfe, 0xc3], 0x00b0_bfc8),
    ]
    .into_iter()
    .enumerate()
    {
        let at = p.sym(at);
        p.a.write(at, &code).unwrap();
        p.b.write(at, &code).unwrap();
        let s = p.sym(slot);
        p.set(s, at);
        let t = p.sym(THUNK) + 0x10 * u32::try_from(k).unwrap();
        let mut c = vec![0xd9, 0x44, 0x24, 0x04, 0xff, 0x25]; // fld dword [esp+4]; jmp [slot]
        c.extend(s.to_le_bytes());
        p.a.write(t, &c).unwrap();
        p.b.write(t, &c).unwrap();
        thunks[k] = t;
    }
    (thunks[0], thunks[1])
}

#[test]
fn rotated_points_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007e_a9d0);
        let (cos, sin) = trig_stand_ins(&mut p);
        let out = p.alloc(8);
        for _ in 0..3000 {
            let w: Vec<u32> = (0..5)
                .map(|_| {
                    if rng.chance(10) {
                        rng.float()
                    } else {
                        finite(&mut rng)
                    }
                })
                .collect();
            #[allow(clippy::cast_precision_loss)] // small
            let small = (rng.below(700) as f32 / 100.0 - 3.5).to_bits();
            let ang = rng.pick(&[w[2], small]);
            let args = [w[0], w[1], ang, w[3], w[4], out, out + 4];
            p.check("op_rot_point", 0x007e_a9d0, &args, |o| {
                Some(vec::bridge::op_rot_point(
                    o,
                    f32::from_bits(w[0]),
                    f32::from_bits(w[1]),
                    f32::from_bits(ang),
                    [f32::from_bits(w[3]), f32::from_bits(w[4])],
                    [out, out + 4],
                    &mut |o, c, x| {
                        let r = o
                            .call(if c { cos } else { sin }, Conv::Cdecl, &[x.to_bits()])
                            .unwrap();
                        st0_f32(&r)
                    },
                ))
            });
        }
    }
}

/// `ST0` stored as `fstp dword`: a NaN keeps its sign and top fraction
/// bits.
fn st0_f32(r: &ffx_oracle::Ret) -> f32 {
    let raw = r.st0_raw.expect("a float in ST0");
    let mant = u64::from_le_bytes(raw[..8].try_into().unwrap());
    let se = u16::from_le_bytes([raw[8], raw[9]]);
    if se & 0x7fff == 0x7fff && mant << 1 != 0 {
        let frac = u32::try_from((mant >> 40) & 0x7f_ffff).unwrap();
        f32::from_bits(u32::from(se >> 15) << 31 | 0x7f80_0000 | frac)
    } else {
        #[allow(clippy::cast_possible_truncation)] // fstp m32
        let v = r.st0.unwrap_or(f64::NAN) as f32;
        v
    }
}

#[test]
fn op_tokens_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007e_1f80);
        deref_recorder(&mut p, 0x007e_1450, &[0, 0]);
        let class = p.sym(op::bridge::CHAR_CLASS.sym);
        let (src, out, pos) = (p.alloc(0x80), p.alloc(0x40), p.alloc(4));
        let chars = [
            b'a', b'b', b'1', b' ', b'"', b'\'', b';', b'/', b'*', b'(', 0x82, 0xa0, b'\t', 0,
        ];
        for round in 0..5000 {
            if round % 50 == 0 {
                for c in 0..256u32 {
                    let k = match u8::try_from(c).unwrap() {
                        b'a' | b'b' | b'1' => 1,
                        0x82 | 0xa0 => rng.pick(&[1u8, 2]),
                        b'"' => 3,
                        b'\'' => 4,
                        b' ' | b'\t' => 5,
                        b'(' => rng.pick(&[6u8, 8]),
                        b';' | b'/' | b'*' => 7,
                        0 => 0,
                        _ => u8::try_from(rng.below(10)).unwrap(),
                    };
                    p.set(class + c, k);
                }
            }
            for i in 0..0x7f {
                let c = if rng.chance(3) { 0 } else { rng.pick(&chars) };
                p.set(src + i, c);
            }
            p.set(src + 0x7f, 0u8);
            p.fill(out, 0x40, &mut rng);
            p.set(pos, rng.below(0x40));
            p.put(op::bridge::COMMENT_DEPTH, rng.pick(&[0u32, 0, 1, 2]));
            p.put(op::bridge::IN_COMMENT, u32::from(rng.chance(20)));
            let max = rng.pick(&[0u32, 1, 2, 5, 0x20]);
            let eol = 0x40 + rng.below(0x30);
            p.check("op_token", 0x007e_1f80, &[src, out, pos, max, eol], |o| {
                Some(op::bridge::op_token(
                    o,
                    src,
                    out,
                    pos,
                    max,
                    eol,
                    &mut call_original,
                ))
            });
        }
    }
}

/// Writes `code` at `sym` in both oracles.
fn stub_code(p: &mut Pair, sym: u32, code: &[u8]) {
    let at = p.sym(sym);
    p.a.write(at, code).unwrap();
    p.b.write(at, code).unwrap();
}

#[test]
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::cast_possible_truncation
)]
fn dot_draw_matches_the_original() {
    use slots::bridge as s;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a7_2b80);
        // render data per instance: cells[0] for lo 0, cells[1] for 0x1000;
        // what graphicVFXSetup puts there: pend[0..2], what it returns
        let cells = p.alloc(0x20);
        let (pend, setup_ret, seg_ret) = (cells + 8, cells + 0x10, cells + 0x14);
        // graphicVFXGetRenderDataByInstance(lo, hi): [cells + (lo ? 4 : 0)]
        let mut c = vec![
            0x8b, 0x4c, 0x24, 0x04, 0xf7, 0xd9, 0x1b, 0xc9, 0x83, 0xe1, 0x04,
        ];
        c.extend([0x8b, 0x81]); // mov eax, [ecx + cells]
        c.extend(cells.to_le_bytes());
        c.push(0xc3);
        stub_code(&mut p, 0x0064_53d0, &c);
        // graphicVFXSetup(lo, hi): the cell from pend, returns [setup_ret]
        let mut c = vec![
            0x8b, 0x4c, 0x24, 0x04, 0xf7, 0xd9, 0x1b, 0xc9, 0x83, 0xe1, 0x04,
        ];
        c.extend([0x8b, 0x81]); // mov eax, [ecx + pend]
        c.extend(pend.to_le_bytes());
        c.extend([0x89, 0x81]); // mov [ecx + cells], eax
        c.extend(cells.to_le_bytes());
        c.push(0xa1); // mov eax, [setup_ret]
        c.extend(setup_ret.to_le_bytes());
        c.push(0xc3);
        stub_code(&mut p, 0x0064_5780, &c);
        // graphicVFXCreate(lo, hi, alpha): the cell from made[], then the
        // recorder; graphicVFXCreateDataBuffer: the second name (0x100 on)
        // into seen[], then the recorder
        let (made, seen) = (p.alloc(8), p.alloc(12));
        let mut c = vec![
            0x8b, 0x4c, 0x24, 0x04, 0xf7, 0xd9, 0x1b, 0xc9, 0x83, 0xe1, 0x04,
        ];
        c.extend([0x8b, 0x81]); // mov eax, [ecx + made]
        c.extend(made.to_le_bytes());
        c.extend([0x89, 0x81]); // mov [ecx + cells], eax
        c.extend(cells.to_le_bytes());
        stub_code(&mut p, 0x0064_4de0, &c);
        deref_recorder(
            &mut p,
            0x0064_4de0 + u32::try_from(c.len()).unwrap(),
            &[0, 0, 0],
        );
        // (only for the two-segment instance: the other one's name has
        // the original's saved registers 0x100 on)
        let mut c = vec![0x83, 0x7c, 0x24, 0x0c, 0x02, 0x75, 0x28]; // cmp [esp + 0xc], 2; jne
        c.extend([0x8b, 0x44, 0x24, 0x18]); // mov eax, [esp + 0x18]
        for k in 0..3u32 {
            c.extend([0x8b, 0x88]); // mov ecx, [eax + 0x100 + 4k]
            c.extend((0x100 + 4 * k).to_le_bytes());
            c.extend([0x89, 0x0d]); // mov [seen + 4k], ecx
            c.extend((seen + 4 * k).to_le_bytes());
        }
        assert_eq!(c.len(), 7 + 0x28);
        stub_code(&mut p, 0x0064_5040, &c);
        deref_recorder(
            &mut p,
            0x0064_5040 + u32::try_from(c.len()).unwrap(),
            &[0, 0, 0, 2, 2, 3, 2],
        );
        for (sym, words, ret) in [
            (0x0064_5280u32, &[0u8, 0][..], None),
            (0x0063_fdb0, &[0, 0, 0, 0], Some(seg_ret)),
            (0x0094_964c, &[0, 0, 0], None),
            (0x0064_5710, &[0, 0, 0], None),
            (0x0064_58a0, &[0, 0], None),
            (0x007e_6a00, &[0], Some(cells + 0x18)),
            (0x007e_6a60, &[0], None),
        ] {
            deref_recorder_with(&mut p, sym, words, ret);
        }
        // the scratch op_spr_alloc returns
        let spr = p.alloc(0x1200);
        p.set(cells + 0x18, spr);
        let (rd1, rd2) = (p.alloc(0x100), p.alloc(0x100));
        let (geo1, geo2) = (p.alloc(0x100), p.alloc(0x100));
        let mut bufs = Vec::new();
        for _ in 0..3 {
            bufs.push((p.alloc(0x8000), p.alloc(0x8000), p.alloc(0x4000)));
        }
        let b = p.alloc(0x10 + 0x20 * 24);
        let rec = p.alloc(0x100);
        let m = p.alloc(0x40);
        for _ in 0..400 {
            for (at, len) in [(rd1, 0x100), (rd2, 0x100), (geo1, 0x100), (geo2, 0x100)] {
                p.fill(at, len, &mut rng);
            }
            p.fill(spr, 0x1200, &mut rng);
            p.set(rd1 + 0x94, geo1);
            p.set(rd2 + 0x94, geo2);
            for (k, (g, at)) in [(geo1, 0u32), (geo1, 0x6c), (geo2, 0)]
                .into_iter()
                .enumerate()
            {
                let (pts, col, idx) = bufs[k];
                p.set(g + at + 0xc, pts);
                p.set(g + at + 0x14, col);
                p.set(g + at + 0x1c, idx);
            }
            p.fill(rec, 0x100, &mut rng);
            for k in 0..16 {
                let x = finite(&mut rng);
                p.set(m + 4 * k, x);
            }
            let n = rng.below(25);
            p.set(b, n);
            for i in 0..24 {
                let e = b + 0x10 + 0x20 * i;
                p.fill(e, 0x20, &mut rng);
                for k in 0..4 {
                    let x = finite(&mut rng);
                    p.set(e + 4 * k, x);
                }
                let spread = match rng.below(4) {
                    0 => 0i16,
                    1 => i16::try_from(rng.below(32)).unwrap() - 16,
                    2 => i16::try_from(rng.below(1100)).unwrap() - 40,
                    _ => rng.next() as i16,
                };
                p.set(e + 0x14, spread);
                if rng.chance(50) {
                    p.set(e + 0x1c, 0u8);
                }
                if rng.chance(40) {
                    p.set(e + 0x16, i16::try_from(rng.below(40)).unwrap() - 4);
                }
            }
            let pick = |rng: &mut Xorshift, v: u32| if rng.chance(25) { 0 } else { v };
            let (c1, c2) = (pick(&mut rng, rd1), pick(&mut rng, rd2));
            p.set(cells, c1);
            p.set(cells + 4, c2);
            let (q1, q2) = (pick(&mut rng, rd1), pick(&mut rng, rd2));
            p.set(made, pick(&mut rng, rd1));
            p.set(made + 4, pick(&mut rng, rd2));
            p.set(pend, q1);
            p.set(pend + 4, q2);
            p.set(setup_ret, u32::from(rng.chance(70)));
            p.set(seg_ret, u32::from(!rng.chance(10)));
            let hold = rng.pick(&[0u32, 1]);
            check_deep_n(
                &mut p,
                "opus_0582_dot_draw",
                0x00a7_2b80,
                &[b, rec, m, hold],
                |mm, frame| {
                    s::opus_0582_dot_draw(mm, b, rec, m, hold, frame, &mut call_original);
                    None
                },
            );
        }
    }
}

#[test]
#[allow(clippy::many_single_char_names)]
fn z_crct_shp_draw_matches_the_original() {
    use mat::bridge as mt;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0073_54c0);
        let (obj, key, ctrl, offs, part) = (
            p.alloc(0x200),
            p.alloc(0x40),
            p.alloc(0x10),
            p.alloc(0x10),
            p.alloc(0x100),
        );
        let part_at = p.sym(0x0230_ffe4);
        let (view, world) = (p.sym(0x0230_ff20), p.sym(0x0230_ffa0));
        // the frame check_deep_n gives a three-argument original
        let frame = p.b.stack_range().end - 0x100 - 12 - 4 - 0x200 - 12 - 4 - 4;
        for _ in 0..3000 {
            p.set(ctrl + 0xc, offs);
            p.set(offs, 4 * rng.below(0x10));
            p.set(part_at, part);
            for (at, n) in [
                (obj, 0x80u32),
                (key, 0x10),
                (part, 0x40),
                (view, 0x10),
                (world, 0x10),
            ] {
                for k in 0..n {
                    let x = finite(&mut rng);
                    p.set(at + 4 * k, x);
                }
            }
            p.set(key + 0x30, rng.pick(&[0u8, 1, 2, 0, 1, 2, 3, 0xff]));
            // the stack row 3 an unknown mode leaves (both alike)
            for k in 0..4 {
                let x = finite(&mut rng);
                p.set(frame - 0x14 + 4 * k, x);
            }
            check_deep_n(
                &mut p,
                "pppKeZCrctShpDraw",
                0x0073_54c0,
                &[obj, key, ctrl],
                |m, frame| {
                    mt::ppp_ke_z_crct_shp_draw(m, obj, key, ctrl, frame, &mut call_original);
                    None
                },
            );
        }
    }
}

#[test]
fn hash_u_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0071_6570);
        let s = p.alloc(0x200);
        for round in 0..3000u32 {
            let n = if round < 4 { round } else { rng.below(0x1ff) };
            for i in 0..n {
                // printable names, and any non-zero byte
                let c = if rng.chance(70) {
                    0x20 + rng.below(0x5f)
                } else {
                    1 + rng.below(0xff)
                };
                p.set(s + i, u8::try_from(c).unwrap());
            }
            p.set(s + n, 0u8);
            p.check("hashU", 0x0071_6570, &[s], |m| {
                Some(small::bridge::hash_u_at(m, s))
            });
        }
    }
}

#[test]
fn node_requeue_all_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a3_9f60);
        let (set, nodes, owners) = (p.alloc(0x100), p.alloc(0x40 * 8), p.alloc(0x100));
        for _ in 0..2000 {
            p.fill(set, 0x100, &mut rng);
            p.fill(nodes, 0x200, &mut rng);
            p.fill(owners, 0x100, &mut rng);
            // two owners, their lists (head at +0x48) empty or one node
            for o in [owners, owners + 0x80] {
                let h = o + 0x48;
                let (a, b) = if rng.chance(50) {
                    (h, h)
                } else {
                    let d = nodes + 0x40 * 7;
                    (d, d)
                };
                p.set(h + 0x1c, a);
                p.set(h + 0x20, b);
            }
            // list 2: the head at set + 0x80 and k nodes, +0x1c onward
            let head = set + 0x80;
            let k = rng.below(7);
            let nodes_in: Vec<u32> = std::iter::once(head)
                .chain((0..k).map(|i| nodes + 0x40 * i))
                .collect();
            for (i, &n) in nodes_in.iter().enumerate() {
                let next = nodes_in[(i + 1) % nodes_in.len()];
                let prev = nodes_in[(i + nodes_in.len() - 1) % nodes_in.len()];
                p.set(n + 0x1c, next);
                p.set(n + 0x20, prev);
                if n != head {
                    p.set(n + 0x18, rng.pick(&[owners, owners + 0x80]));
                }
            }
            p.check("0x00a39f60", 0x00a3_9f60, &[set], |o| {
                Some(small::bridge::op_node_requeue_all(o, set))
            });
        }
    }
}

#[test]
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn view_out_code_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0072_8dd0);
        let edges = [
            0.888_888_9f32,
            -0.888_888_9,
            0.888_888_8,
            -0.888_888_8,
            0.5,
            -0.5,
            0.499_999_97,
            -0.499_999_97,
            0.0,
            -0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
        ];
        for _ in 0..4000 {
            let pick = |rng: &mut Xorshift| -> f32 {
                match rng.below(3) {
                    0 => edges[rng.below(13) as usize],
                    1 => f32::from_bits(finite(rng)),
                    _ => (rng.below(4000) as f32 - 2000.0) / 1000.0,
                }
            };
            let (x, y) = (pick(&mut rng), pick(&mut rng));
            p.check(
                "pppViewOutCode",
                0x0072_8dd0,
                &[x.to_bits(), y.to_bits()],
                |_| Some(small::ppp_view_out_code(x, y)),
            );
        }
    }
}

#[test]
fn et_chr_count_same_no_matches_the_original() {
    use et::bridge as e;
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007f_eb20);
        let chr = p.sym(e::CHR.sym);
        for round in 0..1000 {
            // the snapshot's own table first
            if !p.snapshot() || round > 0 {
                for i in 0..e::CHRS {
                    let h = if rng.chance(50) {
                        0
                    } else {
                        1 + rng.below(300)
                    };
                    p.set(chr + 0x20 * i, h);
                    p.set(chr + 0x20 * i + 4, rng.below(8));
                }
            }
            let h = if rng.chance(70) {
                p.get(chr + 0x20 * rng.below(e::CHRS))
            } else {
                rng.pick(&[0u32, 0x1234, u32::MAX])
            };
            p.check("0x007feb20", 0x007f_eb20, &[h], |m| {
                Some(e::op_et_chr_count_same_no(m, h))
            });
        }
    }
}

#[test]
fn texels_halve_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a3_85f0);
        let (dst, src) = (p.alloc(0x200), p.alloc(0x400));
        for _ in 0..3000 {
            p.fill(dst, 0x200, &mut rng);
            p.fill(src, 0x400, &mut rng);
            let size = rng.pick(&[4u32, 4, 1, 1, 0, 2, 3, 8]);
            let big = rng.below(0x80);
            let n = rng.pick(&[0u32, 1, 2, big]);
            p.check("0x00a385f0", 0x00a3_85f0, &[dst, src, n, size], |m| {
                Some(small::bridge::op_texels_halve(m, dst, src, n, size))
            });
            // a column: steps of 4..0x10 bytes, 1..0x10 texels
            let (ds, ss) = (4 * (1 + rng.below(4)), 4 * (1 + rng.below(4)));
            let few = rng.below(0x10);
            let n = rng.pick(&[0u32, 1, few]);
            p.fill(dst, 0x200, &mut rng);
            p.check(
                "0x00a38560",
                0x00a3_8560,
                &[dst, ds, src, ss, n, size],
                |m| {
                    Some(small::bridge::op_texels_halve_v(
                        m, dst, ds, src, ss, n, size,
                    ))
                },
            );
        }
    }
}

#[test]
fn vfx_render_data_counts_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0071_3290);
        let (rd, parts, bufs) = (p.alloc(0x100), p.alloc(0x6c * 8), p.alloc(0x400));
        // the shape in the middle of its block: entries before and after it
        let shape = p.alloc(0x500) + 0x100;
        let (get_rd, get_id, get_ptr) =
            (p.sym(0x0064_53d0), p.sym(0x0063_ff50), p.sym(0x00a4_4650));
        for _ in 0..300 {
            p.fill(rd, 0x100, &mut rng);
            p.fill(parts, 0x6c * 8, &mut rng);
            p.fill(shape - 0x100, 0x500, &mut rng);
            p.fill(bufs, 0x400, &mut rng);
            let found = if rng.chance(15) { 0 } else { rd };
            let id = rng.below(3);
            for o in [&mut p.a, &mut p.b] {
                o.stub_return(get_rd, found).unwrap();
                o.stub_return(get_id, id).unwrap();
                o.stub_return(get_ptr, shape).unwrap();
            }
            let n = rng.pick(&[0i32, 1, 3, 5, -1]);
            p.set(rd + 0x90, n);
            p.set(rd + 0x94, parts);
            for i in 0..5 {
                let part = parts + 0x6c * i;
                p.set(part + 0x1c, bufs + 0x80 * i);
                p.set(part + 0x68, rng.below(0x40));
            }
            // shape entries: offsets to small records with an i16 count
            let k = rng.below(8);
            let off = rng.pick(&[0x100 + 8 * i16::try_from(k).unwrap(), -0x80, -2]);
            p.set(shape + 0x10 + 8 * k, off);
            let (lo, hi) = (rng.next(), rng.next());
            p.check("0x00713290", 0x0071_3290, &[lo, hi], |m| {
                Some(draw::bridge::clear_vfx_render_data_shp(
                    m,
                    lo,
                    hi,
                    &mut call_original,
                ))
            });
            p.fill(parts, 0x6c * 8, &mut rng);
            for i in 0..5 {
                let part = parts + 0x6c * i;
                p.set(part + 0x1c, bufs + 0x80 * i);
                p.set(part + 0x68, rng.below(0x40));
            }
            p.check("0x00713320", 0x0071_3320, &[lo, hi], |m| {
                Some(draw::bridge::clear_vfx_render_data_shp_with_memset(
                    m,
                    lo,
                    hi,
                    &mut call_original,
                ))
            });
            let arg = rng.next();
            p.set(bufs, arg);
            let n = k | rng.pick(&[0u32, 0xffff_0000]);
            p.check("0x00712bd0", 0x0071_2bd0, &[lo, hi, bufs, n], |m| {
                Some(
                    draw::bridge::caculate_vfx_render_data_shape_field_buffer_size(
                        m,
                        [lo, hi, bufs, n],
                        &mut call_original,
                    ),
                )
            });
        }
    }
}

/// [`check_deep`] for any number of arguments and an original that aligns
/// its frame (`push ebx; mov ebx, esp; sub esp, 8; and esp, -16; add esp,
/// 4; push ebp; ...; mov ebp, esp`): `rust` gets that `ebp`. Only memory is
/// compared (no caller reads the original's `eax`).
fn check_deep_aligned(
    p: &mut Pair,
    what: &str,
    sym: u32,
    args: &[u32],
    rust: impl FnOnce(&mut Oracle, u32),
) {
    let n = u8::try_from(args.len()).unwrap();
    let at = p.sym(THUNK);
    let mut c = vec![0x81, 0xec, 0, 2, 0, 0]; // sub esp, 0x200
    for _ in 0..n {
        c.extend([0xff, 0xb4, 0x24, 4 * n, 0x02, 0, 0]); // push [esp + 0x200 + 4n]
    }
    c.push(0xe8); // call sym
    let call_end = at + u32::try_from(c.len()).unwrap() + 4;
    c.extend(p.sym(sym).wrapping_sub(call_end).to_le_bytes());
    c.extend([0x81, 0xc4, 4 * n, 0x02, 0, 0]); // add esp, 0x200 + 4n
    c.push(0xc3);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    let words = 4 * u32::from(n);
    // the original's entry esp, then ebp = (esp - 12) & -16
    let entry = p.b.stack_range().end - 0x100 - words - 4 - 0x200 - words - 4;
    let frame = (entry - 12) & !0xf;
    let r =
        p.a.call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{}: {what}: {e}", p.name));
    assert!(
        r.st0.is_none(),
        "{}: {what}: the original returns {:?} in ST0",
        p.name,
        r.st0
    );
    rust(&mut p.b, frame);
    p.same(what);
}

#[test]
#[allow(clippy::many_single_char_names)] // the original's arguments
fn opus_rt_m_chara_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0080_62c0);
        let (m, angles, trans) = (p.alloc(0x40), p.alloc(0x10), p.alloc(0x10));
        let (vu0, vu1) = (p.sym(0x00c8_f570), p.sym(0x00c8_f960));
        // real characters on a snapshot: the g_EtChr indices with a handle
        let handles = p.sym(et::bridge::CHR_HANDLES.sym);
        let mut chrs = Vec::new();
        if p.snapshot() {
            for i in 0..0x30u32 {
                let h: u32 = p.get(handles + 4 * i);
                if h != 0 {
                    chrs.push(i);
                }
            }
        }
        let real = !chrs.is_empty();
        let cells = p.alloc(0x10);
        if !real {
            // the character calls stand in: floats from `cells`, and a
            // battle scale of 1.0 | chr in all three (mov eax, [esp + 4];
            // mov ecx, [esp + 8]; or eax, 0x3f800000; mov [ecx], eax;
            // mov [ecx + 4], eax; mov [ecx + 8], eax; ret)
            for (i, f) in [0x0082_ad30u32, 0x0082_ad40, 0x0082_ad50, 0x0082_ab60]
                .into_iter()
                .enumerate()
            {
                float_stub(&mut p, f, cells + 4 * u32::try_from(i).unwrap());
            }
            let ms = p.sym(0x0079_5720);
            let code = [
                0x8b, 0x44, 0x24, 0x04, 0x8b, 0x4c, 0x24, 0x08, 0x0d, 0, 0, 0x80, 0x3f, 0x89, 0x01,
                0x89, 0x41, 0x04, 0x89, 0x41, 0x08, 0xc3,
            ];
            for o in [&mut p.a, &mut p.b] {
                o.write(ms, &code).unwrap();
            }
            chrs.extend(0..0x30);
        }
        for _ in 0..300 {
            for i in (0..0x40).step_by(4) {
                let v = finite(&mut rng);
                p.set(m + i, v);
            }
            for i in (0..0x10).step_by(4) {
                let (a, t) = (finite(&mut rng), rng.float());
                p.set(angles + i, a);
                p.set(trans + i, t);
            }
            // no NaNs from the ST0 stand-ins: the oracle's ST0 result
            // (f80_to_f64) drops their sign and payload, the hardware keeps
            // them
            for i in (0..0x10).step_by(4) {
                let v = rng.float();
                let v = if x87::is_nan(v) { finite(&mut rng) } else { v };
                p.set(cells + i, v);
            }
            p.fill(vu0, 0x40, &mut rng);
            p.fill(vu1, 0x40, &mut rng);
            let chr = rng.pick(&chrs);
            let any = rng.float();
            let k = rng.pick(&[0x4380_0000u32, any]);
            let a = rng.pick(&[0, angles]);
            let t = rng.pick(&[0, trans]);
            let battle = rng.below(2);
            let args = [m, chr, k, a, t, battle];
            // the scale local at [ebp - 0x20]: what was on the stack before
            let entry = p.b.stack_range().end - 0x100 - 24 - 4 - 0x200 - 24 - 4;
            let local = ((entry - 12) & !0xf) - 0x20;
            for i in (0..0x10).step_by(4) {
                let v = rng.float();
                p.set(local + i, v);
            }
            check_deep_aligned(&mut p, "0x008062c0", 0x0080_62c0, &args, |o, frame| {
                opus::bridge::opus_rt_m_chara(
                    o,
                    m,
                    chr,
                    f32::from_bits(k),
                    a,
                    t,
                    battle,
                    frame,
                    &mut call_original,
                    &mut call_original_f,
                );
            });
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn opus_mdc_packets_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0080_e160);
        let (pk, mats, obj, unit) = (p.alloc(0x60), p.alloc(0x100), p.alloc(0xa0), p.alloc(0x100));
        let oef = p.alloc(0x400);
        let lights = p.alloc(0x100 * 16);
        let (col, pos, nos) = (p.alloc(0x40), p.alloc(0x40), p.alloc(0x10));
        let gpr = p.sym(0x00c8_f508);
        // opus_no_get stands in: unit `no` of `lights` (movzx eax, word
        // [esp + 0xc]; shl eax, 8; add eax, lights; ret)
        let mut code = vec![0x0f, 0xb7, 0x44, 0x24, 0x0c, 0xc1, 0xe0, 0x08, 0x05];
        code.extend(lights.to_le_bytes());
        code.push(0xc3);
        let get = p.sym(0x0080_1060);
        p.a.write(get, &code).unwrap();
        p.b.write(get, &code).unwrap();
        let float = |rng: &mut Xorshift| {
            if rng.chance(50) {
                rng.float()
            } else {
                // around the 0..0x100 the clamp sees
                let x = f32::from_bits(rng.float()).abs() % 600.0 - 100.0;
                x.to_bits()
            }
        };
        for _ in 0..500 {
            p.fill(pk, 0x60, &mut rng);
            p.fill(obj, 0xa0, &mut rng);
            p.fill(unit, 0x100, &mut rng);
            // the oef2 block: table 4 at a small offset, of small offsets
            p.fill(oef, 0x400, &mut rng);
            p.set(oef + 0x10, 0x100u32 + 4 * rng.below(16));
            p.set(unit + 0x1e, u8::try_from(rng.below(4)).unwrap());
            for g in 0..4 {
                p.set(obj + 0x60 + 4 * g, if rng.chance(10) { 0 } else { oef });
            }
            // the entry is tex & 0xff: high bits too
            let (n, high) = (rng.below(0x40), rng.next() & 0xffff_ff00);
            let tex = rng.pick(&[u32::MAX, n, high | n]);
            let o: u32 = p.get(obj + 0x60 + 4 * u32::from(p.get::<u8>(unit + 0x1e)));
            let tex = if o == 0 { u32::MAX } else { tex };
            for (name, sym, clear) in [
                ("0x0080e160", 0x0080_e160u32, true),
                ("0x0080e230", 0x0080_e230, false),
            ] {
                p.check(name, sym, &[pk, mats, obj, unit, tex], |m| {
                    Some(opus::bridge::opus_mdc_packet_set(
                        m, pk, mats, obj, unit, tex, clear,
                    ))
                });
            }
            // the lights
            p.set(pk + 0x24, col);
            p.set(pk + 0x28, pos);
            p.fill(col, 0x40, &mut rng);
            p.fill(pos, 0x40, &mut rng);
            p.fill(gpr, 0x10, &mut rng);
            p.fill(lights, 0x100 * 16, &mut rng);
            for i in 0..16 {
                let l = lights + 0x100 * i;
                for at in [0x3cu32, 0x4c, 0x90, 0x94, 0x98] {
                    let v = rng.float();
                    p.set(l + at, v);
                }
            }
            for i in 0..5 {
                let no = if rng.chance(25) { 0 } else { rng.below(16) };
                p.set(nos + 2 * i, u16::try_from(no).unwrap());
            }
            for at in [0x30u32, 0x4c] {
                let v = float(&mut rng);
                p.set(unit + at, v);
            }
            p.check(
                "0x0080e900",
                0x0080_e900,
                &[pk, mats, obj, unit, nos],
                |m| {
                    opus::bridge::opus_mdc_light_set(m, pk, obj, unit, nos, &mut call_original);
                    None
                },
            );
        }
    }
}

#[test]
fn opus_get_bone_matrix_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0080_5430);
        let out = p.alloc(0x60);
        let (gpr, vu) = (p.sym(0x00c8_f508), p.sym(0x00c8_f780));
        // real characters on a snapshot: the g_EtChr handles
        let chr_at = p.sym(et::bridge::CHR.sym);
        let mut handles = Vec::new();
        if p.snapshot() {
            for i in 0..et::bridge::CHRS {
                let h: u32 = p.get(chr_at + 0x20 * i);
                if h != 0 && !handles.contains(&h) {
                    handles.push(h);
                }
            }
        }
        let real = !handles.is_empty();
        if !real {
            // the character calls stand in: an element number, no matrix
            // (the matrix call records its element: mov eax, [esp + 8];
            // mov [seen], eax; xor eax, eax; ret)
            let (optpos, base) = (p.sym(0x0083_5660), p.sym(0x0083_3b80));
            let seen = p.alloc(4);
            let mut code = vec![0x8b, 0x44, 0x24, 0x08, 0xa3];
            code.extend(seen.to_le_bytes());
            code.extend([0x31, 0xc0, 0xc3]);
            for o in [&mut p.a, &mut p.b] {
                o.stub_return(optpos, 7).unwrap();
                o.write(base, &code).unwrap();
            }
            handles.push(p.alloc(0x40));
        }
        for _ in 0..300 {
            for i in (0..0x60).step_by(4) {
                let v = finite(&mut rng);
                p.set(out + i, v);
            }
            p.fill(gpr, 0x10, &mut rng);
            p.fill(vu, 0x1e0, &mut rng);
            let chr = rng.pick(&handles);
            // the option-position form passes the original's stack local to
            // Ch_OptposGetElement: only real characters fill it
            let bone = if real && rng.chance(50) {
                0x8000 | rng.below(4) | rng.pick(&[0u32, 0x100])
            } else if real {
                rng.below(4)
            } else {
                rng.below(4) | rng.pick(&[0u32, 0x100, 0x1_0000])
            };
            // the original's local is at [ebp - 0x14]
            check_deep3(
                &mut p,
                "0x00805430",
                0x0080_5430,
                [out, chr, bone],
                |m, frame| {
                    opus::bridge::opus_get_bone_matrix(
                        m,
                        out,
                        chr,
                        bone,
                        frame - 0x14,
                        &mut call_original,
                    )
                },
            );
        }
    }
}

#[test]
fn md_tex0_change_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_9ca0);
        let packet = p.alloc(0x2000);
        let sizes = p.sym(op::bridge::MD_TAG_SIZES.sym);
        for _ in 0..1500 {
            p.fill(packet, 0x2000, &mut rng);
            let mut t = packet + 0x20;
            for _ in 0..rng.below(8) {
                let kind = u8::try_from(rng.below(0xff)).unwrap();
                p.set(t + 1, kind);
                p.set(t + 2, u16::try_from(rng.below(12)).unwrap());
                // coordinates near the 14-bit range's ends and in between
                let any = rng.below(0x4000);
                let u = rng.pick(&[0u32, 1, 0x3fff, 0x3ffe, 0x2000, any]);
                let any = rng.below(0x4000);
                let v = rng.pick(&[0u32, 1, 0x3fff, 0x1000, any]);
                let w8: u32 = p.get(t + 8);
                let wc: u32 = p.get(t + 0xc);
                p.set(t + 8, w8 & !0x3fff | u);
                p.set(t + 0xc, wc & !(0x3fff << 5) | v << 5);
                let size: u32 = p.get(sizes + 4 * u32::from(kind & 7));
                t += 0x10 + u32::from(p.get::<u16>(t + 2)) * size;
            }
            p.set(t + 1, 0xffu8);
            let tex0 = rng.next();
            let any = rng.below(0x4000);
            let lo = rng.pick(&[0u32, 0x100, any, 0xffff_0010]);
            let any = rng.below(0x4000);
            let hi = rng.pick(&[0x4000u32, 0x3fff, any, 0x1_2000]);
            p.check("0x00a69ca0", 0x00a6_9ca0, &[packet, tex0, lo, hi], |m| {
                Some(op::bridge::op_md_tex0_change_limit(m, packet, lo, hi))
            });
            p.check("0x00a69c80", 0x00a6_9c80, &[packet, tex0], |m| {
                Some(op::bridge::op_md_tex0_change(m, packet))
            });
        }
    }
}

#[test]
fn opus_encount0_init_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x007f_8480);
        let obj = p.alloc(0x30 + 0x20 * 0x200);
        for _ in 0..200 {
            p.fill(obj, 0x30 + 0x20 * 0x200, &mut rng);
            let n = |rng: &mut Xorshift| {
                let any = rng.below(8);
                u16::try_from(rng.pick(&[0u32, 1, 2, any])).unwrap()
            };
            let (nx, ny, nz) = (n(&mut rng), n(&mut rng), n(&mut rng));
            p.set(obj + 2, nx);
            p.set(obj + 4, ny);
            p.set(obj + 6, nz);
            p.check("0x007f8480", 0x007f_8480, &[obj], |m| {
                Some(opus::bridge::opus_encount0_init(m, obj))
            });
        }
        // one long row of 0x200 particles
        p.set(obj + 2, 0x200u16);
        p.set(obj + 4, 1u16);
        p.set(obj + 6, 1u16);
        p.check("0x007f8480 long", 0x007f_8480, &[obj], |m| {
            Some(opus::bridge::opus_encount0_init(m, obj))
        });
    }
}

#[test]
fn shp_tail3x_vf_copy_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0075_3080);
        let (dst, vf) = (p.alloc(0x200), p.sym(0x00c8_f7b0));
        for _ in 0..500 {
            p.fill(dst, 0x200, &mut rng);
            p.fill(vf, 0x1c0, &mut rng);
            p.check("0x00753080", 0x0075_3080, &[dst], |m| {
                Some(small::bridge::ppp_ke_shp_tail3x_vf_copy(m, dst))
            });
        }
    }
}

#[test]
#[allow(clippy::similar_names)] // rng and ring
fn oef2_sound_init_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0080_0440);
        let ring = p.sym(0x023c_3500);
        for _ in 0..50 {
            p.fill(ring, 0x400, &mut rng);
            p.check("0x00800440", 0x0080_0440, &[], |m| {
                Some(op::bridge::op_oef2_sound_init(m))
            });
        }
    }
}

#[test]
fn texels_box_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a3_8670);
        let (dst, a, b) = (p.alloc(0x200), p.alloc(0x400), p.alloc(0x400));
        for _ in 0..3000 {
            p.fill(dst, 0x200, &mut rng);
            p.fill(a, 0x400, &mut rng);
            p.fill(b, 0x400, &mut rng);
            // all-ones and all-zero texels drive every carry
            if rng.chance(20) {
                for o in (0..0x400).step_by(4) {
                    p.set(a + o, rng.pick(&[0u32, u32::MAX, 0xff00_ff00, 0x00ff_00ff]));
                    p.set(b + o, rng.pick(&[0u32, u32::MAX, 0xfefe_fefe, 0x0101_0101]));
                }
            }
            let size = rng.pick(&[4u32, 4, 1, 1, 0, 2, 3, 8]);
            let big = rng.below(0x80);
            let n = rng.pick(&[0u32, 1, 2, big]);
            let (ra, rb) = if rng.chance(50) { (a, b) } else { (b, a) };
            p.check("0x00a38670", 0x00a3_8670, &[dst, ra, rb, n, size], |m| {
                Some(small::bridge::op_texels_box(m, dst, ra, rb, n, size))
            });
        }
    }
}

#[test]
fn opus_0437_fade_step_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_e260);
        let (b, table) = (p.alloc(0x3c + 0x60 * 12), p.alloc(0x400));
        for _ in 0..3000 {
            p.fill(b, 0x3c + 0x60 * 12, &mut rng);
            p.fill(table, 0x400, &mut rng);
            p.set(b, u16::try_from(rng.below(13)).unwrap());
            p.set(b + 0xc, table + 0x200);
            for i in 0..12 {
                let e = b + 0x3c + 0x60 * i;
                let slot = if rng.chance(25) {
                    -1 - i16::try_from(rng.below(3)).unwrap()
                } else {
                    i16::try_from(rng.below(0x200)).unwrap() - 0x100
                };
                p.set(e, slot);
                p.set(e + 2, rng.pick(&[0u8, 0x1e, 0x1f, 0x20, 0xff, 5]));
            }
            p.check("0x00a6e260", 0x00a6_e260, &[b], |m| {
                Some(small::bridge::opus_0437_fade_step(m, b))
            });
        }
    }
}

/// `pppFtoiRgba_0074c280` and its copies (the same code at each address).
const FTOI_RGBA: [(&str, u32); 7] = [
    ("pppFtoiRgba_0074c280", 0x0074_c280),
    ("pppFtoiRgba_0074d280", 0x0074_d280),
    ("pppFtoiRgba_0074e280", 0x0074_e280),
    ("pppFtoiRgba_0074f2c0", 0x0074_f2c0),
    ("pppFtoiRgba_00750130", 0x0075_0130),
    ("pppFtoiRgba_00751930", 0x0075_1930),
    ("pppFtoiRgba_007535a0", 0x0075_35a0),
];

#[test]
#[allow(clippy::cast_precision_loss)]
fn ftoi_rgba_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0074_c280);
        let (dst, src) = (p.alloc(0x10), p.alloc(0x10));
        for round in 0..3000u32 {
            p.fill(dst, 0x10, &mut rng);
            for i in 0..4 {
                let x = match rng.below(4) {
                    0 => f32::from_bits(finite(&mut rng)),
                    1 => rng.below(0x200) as f32 - 0x100 as f32 + 0.5,
                    2 => rng.pick(&[
                        -0.999f32,
                        0.999,
                        255.9,
                        256.0,
                        -1.0,
                        2.147_483_6e9,
                        2.147_483_7e9,
                        -2.147_483_6e9,
                        -2.147_483_7e9,
                        f32::INFINITY,
                        f32::NAN,
                    ]),
                    _ => rng.below(0x400) as f32 / 3.0,
                };
                p.set(src + 4 * i, x);
            }
            let (name, at) = FTOI_RGBA[(round % 7) as usize];
            p.check(name, at, &[dst, src], |m| {
                Some(small::bridge::ppp_ftoi_rgba(m, dst, src))
            });
        }
    }
}

#[test]
fn shp_tail3_vf_copy_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0075_1680);
        let (dst, vf) = (p.alloc(0x100), p.sym(0x00c8_f7b0));
        for _ in 0..500 {
            p.fill(dst, 0x100, &mut rng);
            p.fill(vf, 0x40, &mut rng);
            p.check("0x00751680", 0x0075_1680, &[dst], |m| {
                Some(small::bridge::ppp_ke_shp_tail3_vf_copy(m, dst))
            });
        }
    }
}

#[test]
fn shp_tail3_vf_load_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0075_1740);
        let (src, vf) = (p.alloc(0x200), p.sym(0x00c8_f7b0));
        for _ in 0..500 {
            p.fill(src, 0x200, &mut rng);
            p.fill(vf, 0x1c0, &mut rng);
            for (what, sym) in [("0x00751740", 0x0075_1740), ("0x00751ab0", 0x0075_1ab0)] {
                p.check(what, sym, &[src], |m| {
                    Some(small::bridge::ppp_ke_shp_tail3_vf_load(m, src))
                });
            }
        }
    }
}

#[test]
fn data_last_pair_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0076_1250);
        let (data, out) = (p.alloc(0x4000), p.alloc(8));
        for _ in 0..2000 {
            p.fill(data, 0x100, &mut rng);
            p.fill(out, 8, &mut rng);
            let start = 0x10 + 4 * rng.below(8);
            p.set(data + 4, start);
            // records of every kind with few entries, then the end
            let mut at = data + start;
            for _ in 0..rng.below(8) {
                let kind = rng.pick(&[0i8, 1, 2, 3, 4, 5, 6, 7, 8, -2, 0x7f]);
                let n = i16::try_from(rng.below(5)).unwrap();
                p.fill(at, 0x10, &mut rng);
                p.set(at + 1, kind);
                p.set(at + 2, n);
                let size = [20u32, 28, 32, 40, 24, 32, 40, 48]
                    .get(usize::try_from(kind).unwrap_or(99))
                    .copied()
                    .unwrap_or(0);
                at += 0x10 + size * u32::try_from(n).unwrap();
            }
            p.set(at + 1, -1i8);
            p.check("0x00761250", 0x0076_1250, &[data, out, out + 4], |m| {
                Some(small::bridge::ppp_data_last_pair(m, data, out, out + 4))
            });
        }
    }
}

#[test]
fn data_set_tex_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x0076_28c0);
        let (data, out) = (p.alloc(0x4000), p.alloc(8));
        let table = p.sym(0x00d4_be38);
        for _ in 0..2000 {
            p.fill(data, 0x100, &mut rng);
            p.fill(out, 8, &mut rng);
            p.fill(table, 12 * 0x40, &mut rng);
            let start = 0x10 + 4 * rng.below(8);
            p.set(data + 4, start);
            let base = rng.below(0x20);
            let mut at = data + start;
            for _ in 0..rng.below(8) {
                let kind = rng.pick(&[0i8, 1, 2, 3, 4, 5, 6, 7, 8, -2, 0x7f]);
                let n = i16::try_from(rng.below(5)).unwrap();
                p.fill(at, 0x10, &mut rng);
                p.set(at + 1, kind);
                p.set(at + 2, n);
                p.set(at + 4, u8::try_from(rng.below(0x20)).unwrap());
                let size = [20u32, 28, 32, 40, 24, 32, 40, 48]
                    .get(usize::try_from(kind).unwrap_or(99))
                    .copied()
                    .unwrap_or(0);
                at += 0x10 + size * u32::try_from(n).unwrap();
            }
            p.set(at + 1, -1i8);
            p.check(
                "0x007628c0",
                0x0076_28c0,
                &[data, base, out, out + 4],
                |m| Some(small::bridge::ppp_data_set_tex(m, data, base, out, out + 4)),
            );
        }
    }
}
