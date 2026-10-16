//! The effect functions (`ffx_vfx`) against the original, on two oracles
//! started alike: one runs `FFX.exe`'s code, the other the Rust bridge on
//! the same memory; results and every written byte must agree. Each test
//! runs on the image alone (synthetic state, needs `FFX_GAME_DIR`) and, with
//! `FFX_SNAPSHOT`, on the snapshot's state too.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use ffx_bridge::{Global, Mem, Value};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_vfx::{et, fp, heap, mat, op, opd, opus, ppp, prog, rand, x87};

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
    let by_id: [(&str, u32, ById); 6] = [
        ("Fp_Restart", FP_RESTART, f::fp_restart),
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

/// One of each `et` function on `handles` and effect `ids`.
fn et_calls(p: &mut Pair, rng: &mut Xorshift, handles: &[u32], ids: &[u32], out: u32, what: &str) {
    use et::bridge as e;
    let returning: [(&str, u32, ByHandle); 3] = [
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
    let void: [(&str, u32, ById); 4] = [
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
    for _ in 0..8 {
        let h = rng.pick(handles);
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
