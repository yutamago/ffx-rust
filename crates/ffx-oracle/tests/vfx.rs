//! The effect functions (`ffx_vfx`) against the original, on two oracles
//! started alike: one runs `FFX.exe`'s code, the other the Rust bridge on
//! the same memory; results and every written byte must agree. Each test
//! runs on the image alone (synthetic state, needs `FFX_GAME_DIR`) and, with
//! `FFX_SNAPSHOT`, on the snapshot's state too.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use ffx_bridge::{Global, Mem, Value};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_vfx::{et, fp, heap, mat, op, opd, opus, part, ppp, prog, rand, slots, small, tree, x87};

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
    for _ in 0..8 {
        let h = rng.pick(handles);
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
            p.check("pppVu0MaddaY", 0x0072_c590, &[junk, v, junk, k], |m| {
                b::ppp_vu0_madda(m, v, f);
                Some(v)
            });
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
                ("op_rt_m_x", 0x007e_9580, mat::Axis::X),
                ("op_rt_m_y", 0x007e_9670, mat::Axis::Y),
                ("op_rt_m_z", 0x007e_9760, mat::Axis::Z),
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
            for at in [0x0080_6a50, 0x0080_6aa0] {
                for _ in 0..4 {
                    let r = p.a.call(p.sym(at), Conv::Thiscall(it), &[]).unwrap();
                    let mine = t::opus_tree_next(&mut p.b, it);
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
            p.check("op_oef2_clut_pos_set", 0x007e_45e0, &[a, pos], |m| {
                Some(o::op_oef2_clut_pos_set(m, a, pos))
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
            p.check("opus_parts_clear", 0x0080_1480, &[i, n], |m| {
                opus::bridge::opus_parts_clear(m, i, n);
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
