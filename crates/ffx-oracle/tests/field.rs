//! The field (`ffx_field`) against the original, on two oracles started
//! alike: one runs `FFX.exe`'s code, the other the Rust bridge on the same
//! memory; results and every written byte (the emulated VU0 registers
//! included) must agree. Floats compare bit for bit, at the game thread's
//! x87 precision (control word `0x027f`).
//!
//! Synthetic cases run on the image alone (`FFX_GAME_DIR`); the real
//! state (guide map, display objects, the minimap mesh) on `FFX_SNAPSHOT`
//! (field or battle), skipped without one.
//!
//! Two bytes of code are patched alike in both oracles: the import thunk
//! of `_CIsqrt` (`0x009497b8`, which has no oracle stub) becomes
//! `fsqrt; ret`, what the CRT computes (the correctly rounded root of
//! `ST0` at the current precision), and `fstp st(0); ret` goes in the
//! padding after `rcdtChunkInit` (`0x0092b856`) to pop a float result.

#![allow(clippy::many_single_char_names)] // pointers, vectors and operands

use std::fmt::Debug;
use std::path::PathBuf;

use ffx_bridge::{Global, Mem};
use ffx_field::{geom, guide, ke, model, nkz, packet, rcdt, rcui, scene, task, vu0, yi};
use ffx_oracle::{Conv, Oracle, Pages};

const SQRT_THUNK: u32 = 0x0094_97b8;
const POP_STUB: u32 = 0x0092_b856;

// rcdt
const RCDT_CHUNK_INIT: u32 = 0x0092_b840;
const RCDT_CHUNK_GET_BEGIN: u32 = 0x0092_b810;
const RCDT_CHUNK_SET_BIN: u32 = 0x0092_b940;
const RCDT_PARSE_ADVANCE: u32 = 0x0092_b960;
const RCDT_PARSE_GET_BEGIN: u32 = 0x0092_ba70;
const RCDT_PARSE_SET: u32 = 0x0092_c0a0;
const RCDT_PARSE_SET_POS: u32 = 0x0092_c0d0;
const RCDT_PARSE_GET_BIN: u32 = 0x0092_ba90;
const RCDT_PARSE_INIT: u32 = 0x0092_bf50;
const RCDT_PARSE_INIT_SEPARATORS: u32 = 0x0092_c000;
// rcPacketData
const RC_PACKET_DATA_INIT: u32 = 0x0092_7a20;
const RC_PACKET_DATA_OPEN_ADR: u32 = 0x0092_7a70;
const RC_PACKET_DATA_CLOSE_ADR: u32 = 0x0092_75b0;
// geometry
const RC_BBOX_INIT: u32 = 0x0092_14e0;
const RC_BBOX_GET_CENTER: u32 = 0x0092_13d0;
const RC_BBOX_GET_SHORT_CONVERT_SCALE: u32 = 0x0092_1410;
const RC_VEC3F_MUL: u32 = 0x0092_12c0;
const RC_VEC3F_LENGTH: u32 = 0x0092_1250;
const RC_VEC3F_NORMALIZE: u32 = 0x0092_1360;
const RC_MAT4F_SET_TRANSLATE: u32 = 0x0092_5850;
const RC_MAT4F_SET_TRANSLATE_VEC3F: u32 = 0x0092_5870;
// scene objects
const RCBG_MODEL_CREATE_FROM_OMD: u32 = 0x0092_7d70;
const RCBG_MODEL_GET_PRIM_HEADER: u32 = 0x0092_82c0;
const RCBG_MODEL_GET_PRIM: u32 = 0x0092_82a0;
const RCBG_MODEL_SET_ALPHA: u32 = 0x0092_8460;
const RCBG_LAYER_LOAD_CALC_CENTER: u32 = 0x0092_b270;
const RCBG_TRX_INIT: u32 = 0x0092_d510;
const RCBG_TRX_SET_NO: u32 = 0x0092_d670;
const RCBG_PAL_INIT: u32 = 0x0092_99b0;
const RCBG_PAL_SET_NO: u32 = 0x0092_9ab0;
const RCBG_ANIME_INIT: u32 = 0x0093_08a0;
const RCBG_PSIM_SET_INIT_CNT: u32 = 0x0091_fef0;
const RCBG_PSIM_CLEAR_OMD: u32 = 0x0091_f9b0;
// guide map
const GUIDE_MESH_OBJECT: u32 = 0x0091_d460;
const GUIDE_FADE_STEP: u32 = 0x0091_d280;
const GUIDE_FADE_OFF: u32 = 0x0091_d3a0;
const RCBG_GUIDE_SET_DISP_PROG: u32 = 0x0091_d560;
const RCBG_GUIDE_BATTLE_BEGIN: u32 = 0x0091_ccc0;
const RCBG_GUIDE_BATTLE_END: u32 = 0x0091_cce0;
const YN_BATTLE_BEGIN: u32 = 0x0091_9a20;
const YN_BATTLE_END: u32 = 0x0091_9a40;
const YN_GUIDE_MAP_SET_DISP_PROG: u32 = 0x0091_9d70;
const YN_GUIDE_MAP_GET_MAP_NO: u32 = 0x0091_9c70;
// rcef tasks
const RCEF_TASK_INIT: u32 = 0x0092_ed10;
const RCEF_TASK_SET_SEQ: u32 = 0x0092_ee20;
const RCEF_TASK_SET_SEQ_CONT: u32 = 0x0092_ee40;
const RCEF_TASK_CALL_SEQ: u32 = 0x0092_ecb0;
const RCEF_TASK_RET_SEQ_CONT_INNER: u32 = 0x0092_ede0;
const RCEF_TASK_RET_SEQ_CONT: u32 = 0x0092_ee00;
const RCEF_TASK_WAIT: u32 = 0x0092_ee60;
const RCEF_TASK_WAIT_SEQ: u32 = 0x0092_ee90;
const RCEF_TASK_CTRL_INIT: u32 = 0x0093_0290;
const RCEF_TASK_CTRL_IS_END: u32 = 0x0093_02e0;
const RCEF_TASK_CTRL_ADD: u32 = 0x0093_0220;
const RCEF_OBJ_IS_PAR_END: u32 = 0x0093_0010;
const RCEF_OBJ_IS_ANIME_END: u32 = 0x0092_fff0;
const RCEF_OBJ_ATTACH_CHR: u32 = 0x0092_f970;
const RCEF_OBJ_CLEAR_MODE: u32 = 0x0093_01e0;
const RCEF_OBJ_SET_MODE: u32 = 0x0093_0200;
const RCEF_PAR_INIT: u32 = 0x0092_efd0;
const RCEF_ANIME_INIT: u32 = 0x0092_f4c0;
const RCEF_ANIME_START: u32 = 0x0092_f470;
const RC_EFFECT_TASK_PAR_WAIT: u32 = 0x0093_0490;
const RC_EFFECT_TASK_ANIME_WAIT: u32 = 0x0093_0350;
const RC_EFFECT_TASK_TASK_WAIT: u32 = 0x0093_0520;
const RC_EFFECT_TASK_FLAG_WAIT: u32 = 0x0093_03c0;
// yi
const YI_GET_RENDER_STATUS: u32 = 0x0090_8f10;
const YI_RENDER_ENABLE: u32 = 0x0090_9e20;
const YI_RENDER_DISABLE: u32 = 0x0090_9e10;
const YI_GET_DRAW_MASK: u32 = 0x0090_8960;
const YI_SET_DRAW_MASK: u32 = 0x0090_a2b0;
const YI_GET_VCOUNT: u32 = 0x0090_8f70;
const YI_GET_FCOUNT: u32 = 0x0090_8970;
const YI_GET_MAP_STATUS: u32 = 0x0090_8dd0;
const YI_GET_MATRIX_DOBJ: u32 = 0x0090_8df0;
const YI_GET_MATRIX_GROUP: u32 = 0x0090_8e30;
const YI_SET_SCALE_FACTOR: u32 = 0x0090_a950;
const YI_RENDER_RESETTEX: u32 = 0x0090_cea0;
const YI_SCREFF_INIT: u32 = 0x0091_3f30;
// VU0 helpers
const KE_VU0_COPY_VEC_XYZ: u32 = 0x0075_ede0;
const KE_VU0_ADD_VEC_XYZ: u32 = 0x0075_eca0;
const KE_VU0_SCALE_VEC_XYZ: u32 = 0x0075_fb50;
const KE_VU0_VEC_LEN: u32 = 0x0075_fda0;
const KE_VU0_VEC_LEN_XY: u32 = 0x0075_fde0;
const KE_VU0_SQRT: u32 = 0x0075_fbf0;
const KE_MDL_MK_TK_CLUT: u32 = 0x0076_7530;
// Nkz, Sqs
const NKZ_VERSION: u32 = 0x00a5_cf00;
const NKZ_GET_LULU_LIMIT_TIME: u32 = 0x00a5_cb10;
const NKZ_GET_AURON_LIMIT_TIME: u32 = 0x00a5_ca70;
const NKZ_GET_TIDUS_LIMIT_TIME: u32 = 0x00a5_cbd0;
const NKZ_GET_AURON_LIMIT_BUTTON_DATA: u32 = 0x00a5_ca60;
const SQS_REPORT_IOP_SOUND_STATUS: u32 = 0x0081_e250;
const SQS_IOP_SOUND_SYSTEM_STATUS: u32 = 0x0081_d710;
const SQS_IOP_SOUND_ERROR_STATUS: u32 = 0x0081_d700;

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

    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        let span = u32::try_from(hi - lo + 1).unwrap();
        lo + i32::try_from(self.below(span)).unwrap()
    }

    /// Float bits: specials, small and large values, denormals; no NaN.
    fn float(&mut self) -> u32 {
        const SPECIAL: [u32; 10] = [
            0,
            0x8000_0000,
            0x3f80_0000,
            0xbf80_0000,
            0x7f80_0000,
            0xff80_0000,
            0x7f7f_ffff,
            0x0000_0001,
            0x807f_ffff,
            0x0080_0000,
        ];
        match self.below(8) {
            0 => SPECIAL[self.below(10) as usize],
            1 => self.next() & 0x807f_ffff, // denormal
            2 => {
                let v = self.next();
                if v & 0x7f80_0000 == 0x7f80_0000 {
                    v & 0xbfff_ffff
                } else {
                    v
                }
            }
            _ => {
                #[allow(clippy::cast_precision_loss)]
                let v = (self.range(-100_000, 100_000) as f32) / 64.0;
                v.to_bits()
            }
        }
    }

    /// Float bits for copies: anything, quiet NaNs included. Not signalling
    /// NaNs: the emulator (QEMU 5's softfloat) keeps their signalling bit
    /// through `fld m32` / `fstp m32`, where an x87 CPU quiets them (the
    /// masked response to #IA); the port follows the CPU (`ffx_field::x87`).
    fn any_float(&mut self) -> u32 {
        match self.below(4) {
            0 => 0x7fc0_0000 | (self.next() & 0x803f_ffff),
            _ => self.float(),
        }
    }
}

/// Two oracles on the same memory: `a` runs the original, `b` the bridge.
struct Twin {
    a: Oracle,
    b: Oracle,
}

impl Twin {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        for o in [&mut a, &mut b] {
            o.set_fpu_control(0x027f).unwrap();
            let at = o.rebase(SQRT_THUNK);
            o.write(at, &[0xd9, 0xfa, 0xc3]).unwrap(); // fsqrt; ret
            let at = o.rebase(POP_STUB);
            o.write(at, &[0xdd, 0xd8, 0xc3]).unwrap(); // fstp st(0); ret
        }
        Self { a, b }
    }

    fn image() -> Option<Self> {
        let exe = exe()?;
        Some(Self::new(
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
        ))
    }

    fn snapshot() -> Option<Self> {
        let exe = exe()?;
        let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
            eprintln!("FFX_SNAPSHOT is not set; skipping");
            return None;
        };
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        Some(Self::new(
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            Oracle::from_snapshot(&exe, &pages).unwrap(),
        ))
    }

    /// Zeroed scratch memory at the same address in both.
    fn alloc(&mut self, len: usize) -> u32 {
        let x = self.a.alloc(len).unwrap();
        assert_eq!(self.b.alloc(len).unwrap(), x);
        x
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn put<T: ffx_bridge::Value>(&mut self, at: u32, v: T) {
        Mem::write(&mut self.a, at, v);
        Mem::write(&mut self.b, at, v);
    }

    fn set<T: ffx_bridge::Value>(&mut self, g: Global<T>, v: T) {
        let at = g.ptr(&self.b).addr;
        self.put(at, v);
    }

    /// Random bytes at `at` in both.
    fn fill(&mut self, at: u32, len: usize, rng: &mut Xorshift) {
        let bytes: Vec<u8> = (0..len).map(|_| rng.next().to_le_bytes()[0]).collect();
        self.write(at, &bytes);
    }

    /// Runs the original and the bridge; their results and memory agree.
    fn same<R: PartialEq + Debug>(
        &mut self,
        what: &str,
        original: impl FnOnce(&mut Orig) -> R,
        rust: impl FnOnce(&mut Oracle) -> R,
    ) {
        let ra = original(&mut Orig(&mut self.a));
        let rb = rust(&mut self.b);
        assert_eq!(ra, rb, "{what}: results");
        assert_eq!(self.a.diff(&mut self.b).unwrap(), [], "{what}: memory");
    }
}

/// The original's side: cdecl calls by `ffx.sym` address.
struct Orig<'a>(&'a mut Oracle);

impl Orig<'_> {
    fn call(&mut self, f: u32, args: &[u32]) -> ffx_oracle::Ret {
        let at = self.0.rebase(f);
        let r = self
            .0
            .call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{f:#x}: {e}"));
        if r.st0.is_some() {
            let pop = self.0.rebase(POP_STUB);
            self.0.call(pop, Conv::Cdecl, &[]).unwrap();
        }
        r
    }

    fn eax(&mut self, f: u32, args: &[u32]) -> u32 {
        self.call(f, args).eax
    }

    fn void(&mut self, f: u32, args: &[u32]) {
        self.call(f, args);
    }

    /// A float result, as bits (NaNs as one).
    fn float(&mut self, f: u32, args: &[u32]) -> u32 {
        let st0 = self.call(f, args).st0.expect("a float result");
        #[allow(clippy::cast_possible_truncation)] // the value is an f32
        bits(st0 as f32)
    }
}

/// Float bits, NaNs as one (the oracle reads `ST0` through an `f64`).
fn bits(x: f32) -> u32 {
    if x.is_nan() { 0x7fc0_0000 } else { x.to_bits() }
}

fn exe() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    Some(dir?.join("FFX.exe"))
}

#[allow(clippy::cast_sign_loss)]
const fn u(x: i32) -> u32 {
    x as u32
}

#[test]
fn rcdt_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x1357_9bdf);
    let (c, p, data, out) = (t.alloc(8), t.alloc(0x90), t.alloc(0x100), t.alloc(0x40));
    t.fill(data, 0x100, &mut rng);
    for _ in 0..20 {
        t.fill(c, 8, &mut rng);
        t.fill(p, 0x90, &mut rng);
        t.same(
            "rcdtChunkInit",
            |o| o.void(RCDT_CHUNK_INIT, &[c]),
            |m| rcdt::bridge::rcdt_chunk_init(m, c),
        );
        t.same(
            "rcdtParseInit",
            |o| o.void(RCDT_PARSE_INIT, &[p]),
            |m| rcdt::bridge::rcdt_parse_init(m, p),
        );
        t.fill(p, 0x90, &mut rng);
        t.same(
            "rcdtParseInitSeparators",
            |o| o.void(RCDT_PARSE_INIT_SEPARATORS, &[p]),
            |m| rcdt::bridge::rcdt_parse_init_separators(m, p),
        );
        let at = data + rng.below(0x40);
        t.same(
            "rcdtChunkSetBin",
            |o| o.void(RCDT_CHUNK_SET_BIN, &[c, at]),
            |m| rcdt::bridge::rcdt_chunk_set_bin(m, c, at),
        );
        t.same(
            "rcdtChunkGetBegin",
            |o| o.eax(RCDT_CHUNK_GET_BEGIN, &[c]),
            |m| rcdt::bridge::rcdt_chunk_get_begin(m, c),
        );
        t.same(
            "rcdtParseSet",
            |o| o.void(RCDT_PARSE_SET, &[p, c]),
            |m| rcdt::bridge::rcdt_parse_set(m, p, c),
        );
        t.same(
            "rcdtParseGetBegin",
            |o| o.eax(RCDT_PARSE_GET_BEGIN, &[p]),
            |m| rcdt::bridge::rcdt_parse_get_begin(m, p),
        );
        let n = u(rng.range(-8, 0x20));
        t.same(
            "rcdtParseAdvance",
            |o| o.void(RCDT_PARSE_ADVANCE, &[p, n]),
            |m| rcdt::bridge::rcdt_parse_advance(m, p, n),
        );
        let off = rng.below(0x40);
        t.same(
            "rcdtParseSetPos",
            |o| o.void(RCDT_PARSE_SET_POS, &[p, off]),
            |m| rcdt::bridge::rcdt_parse_set_pos(m, p, off),
        );
        let n = rng.below(0x40);
        t.same(
            "rcdtParseGetBin",
            |o| o.void(RCDT_PARSE_GET_BIN, &[p, out, n]),
            |m| rcdt::bridge::rcdt_parse_get_bin(m, p, out, n),
        );
    }
}

#[test]
fn packets_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x2468_ace0);
    let (p, buf) = (t.alloc(8), t.alloc(0x200));
    for _ in 0..50 {
        t.fill(p, 8, &mut rng);
        t.same(
            "rcPacketDataInit",
            |o| o.void(RC_PACKET_DATA_INIT, &[p]),
            |m| packet::bridge::rc_packet_data_init(m, p),
        );
        let start = buf + 4 * rng.below(8);
        t.same(
            "rcPacketDataOpenAdr",
            |o| o.void(RC_PACKET_DATA_OPEN_ADR, &[p, start]),
            |m| packet::bridge::rc_packet_data_open_adr(m, p, start),
        );
        // some data written, sometimes not a whole quadword
        let cursor = start + 0x10 + 4 * rng.below(0x30);
        t.put(p + packet::CURSOR, cursor);
        t.same(
            "rcPacketDataCloseAdr",
            |o| o.void(RC_PACKET_DATA_CLOSE_ADR, &[p]),
            |m| packet::bridge::rc_packet_data_close_adr(m, p),
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn geometry_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0bad_cafe);
    let (bx, out, va, vb, m) = (
        t.alloc(0x20),
        t.alloc(0x10),
        t.alloc(0x10),
        t.alloc(0x10),
        t.alloc(0x40),
    );
    t.same(
        "rcBBoxInit",
        |o| o.void(RC_BBOX_INIT, &[bx]),
        |mm| geom::bridge::rc_bbox_init(mm, bx),
    );
    for i in 0..400 {
        let corners: [u32; 8] = std::array::from_fn(|_| rng.float());
        t.put(bx, corners);
        t.same(
            &format!("rcBBoxGetCenter {corners:x?}"),
            |o| o.void(RC_BBOX_GET_CENTER, &[bx, out]),
            |mm| geom::bridge::rc_bbox_get_center(mm, bx, out),
        );
        t.same(
            &format!("rcBBoxGetShortConvertScale {corners:x?}"),
            |o| o.float(RC_BBOX_GET_SHORT_CONVERT_SCALE, &[bx]),
            |mm| bits(geom::bridge::rc_bbox_get_short_convert_scale(mm, bx)),
        );
        // the centre written over the box itself
        if i % 10 == 0 {
            t.same(
                "rcBBoxGetCenter in place",
                |o| o.void(RC_BBOX_GET_CENTER, &[bx, bx + 4]),
                |mm| geom::bridge::rc_bbox_get_center(mm, bx, bx + 4),
            );
        }
        let (a, b): ([u32; 4], [u32; 4]) = (
            std::array::from_fn(|_| rng.float()),
            std::array::from_fn(|_| rng.float()),
        );
        t.put(va, a);
        t.put(vb, b);
        let target = [out, va, vb][rng.below(3) as usize];
        t.same(
            &format!("rcVec3fMul {a:x?} {b:x?} into {target:#x}"),
            |o| o.void(RC_VEC3F_MUL, &[target, va, vb]),
            |mm| geom::bridge::rc_vec3f_mul(mm, target, va, vb),
        );
        let v: [u32; 4] = if i % 7 == 0 {
            [0, 0x8000_0000, 0, rng.float()]
        } else {
            std::array::from_fn(|_| rng.float())
        };
        t.put(va, v);
        t.same(
            &format!("rcVec3fLength {v:x?}"),
            |o| o.float(RC_VEC3F_LENGTH, &[va]),
            |mm| bits(geom::bridge::rc_vec3f_length(mm, va)),
        );
        t.same(
            &format!("rcVec3fNormalize {v:x?}"),
            |o| o.void(RC_VEC3F_NORMALIZE, &[va]),
            |mm| geom::bridge::rc_vec3f_normalize(mm, va),
        );
        t.fill(m, 0x40, &mut rng);
        let (x, y, z) = (rng.any_float(), rng.any_float(), rng.any_float());
        t.same(
            &format!("rcMat4fSetTranslate {x:#x} {y:#x} {z:#x}"),
            |o| o.void(RC_MAT4F_SET_TRANSLATE, &[m, x, y, z]),
            |mm| geom::bridge::rc_mat4f_set_translate(mm, m, x, y, z),
        );
        t.put(vb, [rng.any_float(), rng.any_float(), rng.any_float(), 0]);
        t.same(
            "rcMat4fSetTranslateVec3f",
            |o| o.void(RC_MAT4F_SET_TRANSLATE_VEC3F, &[m, vb]),
            |mm| geom::bridge::rc_mat4f_set_translate_vec3f(mm, m, vb),
        );
    }
}

/// A synthetic OMD blob: packets of random types and counts from `+0x20`
/// (its `+4` says so too), `+0x10` their total. Returns the packet count.
fn omd(t: &mut Twin, blob: u32, rng: &mut Xorshift) -> i32 {
    let strides = [0x14u32, 0x1c, 0x20, 0x28, 0x18, 0x20, 0x28, 0x30];
    let n = rng.range(1, 6);
    let mut at = blob + 0x20;
    let mut total = 0i16;
    for _ in 0..n {
        let kind = rng.below(8);
        let count = rng.range(1, 5);
        t.fill(at, 0x10, rng);
        t.put(at + model::packet::TYPE, u8::try_from(kind).unwrap());
        t.put(at + model::packet::COUNT, i16::try_from(count).unwrap());
        at += 0x10 + strides[kind as usize] * u(count);
        total += i16::try_from(count).unwrap();
    }
    t.put(blob + model::omd::PACKETS, 0x20u32);
    t.put(blob + model::omd::PRIMS, total);
    n
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn scene_objects_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x7777_1234);
    let (mdl, blob, obj) = (t.alloc(0x100), t.alloc(0x1000), t.alloc(0x100));
    for _ in 0..50 {
        t.fill(blob, 0x1000, &mut rng);
        let n = omd(&mut t, blob, &mut rng);
        t.same(
            "rcbgModelCreateFromOMD",
            |o| o.void(RCBG_MODEL_CREATE_FROM_OMD, &[mdl, blob]),
            |m| model::bridge::rcbg_model_create_from_omd(m, mdl, blob),
        );
        for k in -1..=n {
            t.same(
                &format!("rcbgModelGetPrimHeader {k}"),
                |o| o.eax(RCBG_MODEL_GET_PRIM_HEADER, &[mdl, u(k)]),
                |m| model::bridge::rcbg_model_get_prim_header(m, mdl, k),
            );
            t.same(
                &format!("rcbgModelGetPrim {k}"),
                |o| o.eax(RCBG_MODEL_GET_PRIM, &[mdl, u(k)]),
                |m| model::bridge::rcbg_model_get_prim(m, mdl, k),
            );
        }
        let alpha = rng.next();
        t.same(
            "rcbgModelSetAlpha",
            |o| o.void(RCBG_MODEL_SET_ALPHA, &[mdl, alpha]),
            |m| model::bridge::rcbg_model_set_alpha(m, mdl, alpha.to_le_bytes()[0]),
        );
        t.fill(obj, 0x100, &mut rng);
        let (w, h) = (rng.next() >> rng.below(32), rng.next() >> rng.below(32));
        t.put(obj + model::layer::WIDTH, w);
        t.put(obj + model::layer::HEIGHT, h);
        t.same(
            "rcbgLayerLoadCalcCenter",
            |o| o.void(RCBG_LAYER_LOAD_CALC_CENTER, &[obj]),
            |m| model::bridge::rcbg_layer_load_calc_center(m, obj),
        );
        t.same(
            "rcbgTrxInit",
            |o| o.void(RCBG_TRX_INIT, &[obj]),
            |m| model::bridge::rcbg_trx_init(m, obj),
        );
        let no = rng.next();
        t.same(
            "rcbgTrxSetNo",
            |o| o.void(RCBG_TRX_SET_NO, &[obj, no]),
            |m| model::bridge::rcbg_trx_set_no(m, obj, no),
        );
        t.same(
            "rcbgPalInit",
            |o| o.void(RCBG_PAL_INIT, &[obj]),
            |m| model::bridge::rcbg_pal_init(m, obj),
        );
        let no = rng.next();
        t.same(
            "rcbgPalSetNo",
            |o| o.void(RCBG_PAL_SET_NO, &[obj, no]),
            |m| model::bridge::rcbg_pal_set_no(m, obj, u16::try_from(no & 0xffff).unwrap()),
        );
        t.same(
            "rcbgAnimeInit",
            |o| o.void(RCBG_ANIME_INIT, &[obj]),
            |m| model::bridge::rcbg_anime_init(m, obj),
        );
        let cnt = rng.next();
        t.same(
            "rcbgPSimSetInitCnt",
            |o| o.void(RCBG_PSIM_SET_INIT_CNT, &[obj, cnt]),
            |m| model::bridge::rcbg_psim_set_init_cnt(m, obj, cnt),
        );
        t.put(
            obj + model::psim::MODEL,
            if rng.below(2) == 0 { 0 } else { mdl },
        );
        t.same(
            "rcbgPSimClearOMD",
            |o| o.void(RCBG_PSIM_CLEAR_OMD, &[obj]),
            |m| model::bridge::rcbg_psim_clear_omd(m, obj),
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn guide_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x5eed_0001);
    let (sys, map, list, g) = (t.alloc(8), t.alloc(0x10), t.alloc(0x40), t.alloc(0x80));
    for _ in 0..300 {
        t.set(guide::FADE_STATE, rng.range(-1, 4));
        t.set(guide::FADE_WAIT, rng.range(-1, 4));
        t.set(guide::FADE_LEVEL, rng.range(-1, 8));
        if rng.below(4) == 0 {
            t.set(guide::FADE_DELAY, rng.range(0, 4));
            t.set(guide::FADE_STEPS, rng.range(1, 8));
        }
        let show = [0, 1, 5][rng.below(3) as usize];
        t.same(
            &format!("Guide_FadeStep({show})"),
            |o| o.void(GUIDE_FADE_STEP, &[show]),
            |m| guide::bridge::guide_fade_step(m, i32::try_from(show).unwrap()),
        );
    }
    t.set(guide::FADE_DELAY, 2);
    t.set(guide::FADE_STEPS, 6);
    t.same(
        "Guide_FadeOff",
        |o| o.void(GUIDE_FADE_OFF, &[]),
        guide::bridge::guide_fade_off,
    );
    for _ in 0..50 {
        t.fill(g, 0x80, &mut rng);
        if rng.below(2) == 0 {
            t.put(g + guide::object::MODEL, 0u32);
        }
        t.same(
            "Guide_MeshObject",
            |o| o.eax(GUIDE_MESH_OBJECT, &[g]),
            |m| guide::bridge::guide_mesh_object(m, g),
        );
        let on = [0, 1, 0x100][rng.below(3) as usize];
        t.same(
            "rcbgGuideSetDispProg",
            |o| o.void(RCBG_GUIDE_SET_DISP_PROG, &[g, on]),
            |m| guide::bridge::rcbg_guide_set_disp_prog(m, g, i32::try_from(on).unwrap()),
        );
        t.put(g + guide::object::LIMIT, rng.any_float());
        t.same(
            "rcbgGuideBattleBegin",
            |o| o.void(RCBG_GUIDE_BATTLE_BEGIN, &[g]),
            |m| guide::bridge::rcbg_guide_battle_begin(m, g),
        );
        t.set(guide::BATTLE_SAVE, rng.any_float());
        t.same(
            "rcbgGuideBattleEnd",
            |o| o.void(RCBG_GUIDE_BATTLE_END, &[g]),
            |m| guide::bridge::rcbg_guide_battle_end(m, g),
        );
        // Yn's field state, with each link missing in turn
        let case = rng.below(5);
        t.set(guide::YN_MAP_SYSTEM, if case == 0 { 0 } else { sys });
        t.put(sys + guide::system::GUIDE, if case == 1 { 0 } else { g });
        t.put(sys + guide::system::MAP, if case == 2 { 0 } else { map });
        t.put(map + guide::map::SCENE, if case == 3 { 0 } else { list });
        t.put(
            list + ffx_field::scene::layout::GUIDE_CURRENT,
            u16::try_from(rng.below(0x10000)).unwrap(),
        );
        t.same(
            &format!("Yn_GuideMapGetMapNo case {case}"),
            |o| o.eax(YN_GUIDE_MAP_GET_MAP_NO, &[]),
            |m| u(guide::bridge::yn_guide_map_get_map_no(m)),
        );
        let on = rng.below(2);
        t.same(
            &format!("Yn_GuideMapSetDispProg case {case}"),
            |o| o.void(YN_GUIDE_MAP_SET_DISP_PROG, &[on]),
            |m| guide::bridge::yn_guide_map_set_disp_prog(m, i32::try_from(on).unwrap()),
        );
        if case != 1 {
            t.same(
                "Yn_BattleBegin",
                |o| o.void(YN_BATTLE_BEGIN, &[]),
                guide::bridge::yn_battle_begin,
            );
            t.set(guide::BATTLE_SAVE, rng.any_float());
            t.same(
                "Yn_BattleEnd",
                |o| o.void(YN_BATTLE_END, &[]),
                guide::bridge::yn_battle_end,
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn tasks_match_the_original() {
    use task::{bridge as tb, header};
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0xfeed_beef);
    // room on both sides of the object for out-of-range levels
    let area = t.alloc(0x800);
    let o = area + 0x100;
    for _ in 0..200 {
        t.fill(area, 0x800, &mut rng);
        t.same(
            "rcefTaskInit",
            |x| x.void(RCEF_TASK_INIT, &[o]),
            |m| tb::rcef_task_init(m, o),
        );
        t.put(o + header::LEVEL, rng.range(-1, 8));
        let seq = rng.next();
        t.same(
            "rcefTaskSetSeq",
            |x| x.void(RCEF_TASK_SET_SEQ, &[o, seq]),
            |m| tb::rcef_task_set_seq(m, o, seq),
        );
        t.same(
            "rcefTaskSetSeqCont",
            |x| x.void(RCEF_TASK_SET_SEQ_CONT, &[o, seq]),
            |m| tb::rcef_task_set_seq_cont(m, o, seq),
        );
        t.same(
            "rcefTaskCallSeq",
            |x| x.void(RCEF_TASK_CALL_SEQ, &[o, seq]),
            |m| tb::rcef_task_call_seq(m, o, seq),
        );
        t.put(o + header::LEVEL, rng.range(-1, 8));
        t.same(
            "rcefTaskRetSeqCont_Inner",
            |x| x.void(RCEF_TASK_RET_SEQ_CONT_INNER, &[o]),
            |m| tb::rcef_task_ret_seq_cont_inner(m, o),
        );
        t.same(
            "rcefTaskRetSeqCont",
            |x| x.void(RCEF_TASK_RET_SEQ_CONT, &[o]),
            |m| tb::rcef_task_ret_seq_cont(m, o),
        );
        t.put(o + header::LEVEL, rng.range(-1, 8));
        let frames = rng.range(-1, 3);
        t.same(
            &format!("rcefTaskWait {frames}"),
            |x| x.void(RCEF_TASK_WAIT, &[o, u(frames)]),
            |m| tb::rcef_task_wait(m, o, frames),
        );
        for _ in 0..3 {
            t.same(
                "rcefTaskWaitSeq",
                |x| x.void(RCEF_TASK_WAIT_SEQ, &[o]),
                |m| tb::rcef_task_wait_seq(m, o),
            );
        }
        t.same(
            "rcefObjIsParEnd",
            |x| x.eax(RCEF_OBJ_IS_PAR_END, &[o]),
            |m| u32::from(tb::rcef_obj_is_par_end(m, o)),
        );
        t.same(
            "rcefObjIsAnimeEnd",
            |x| x.eax(RCEF_OBJ_IS_ANIME_END, &[o]),
            |m| u32::from(tb::rcef_obj_is_anime_end(m, o)),
        );
        t.put(o + header::LEVEL, rng.range(0, 7));
        t.same(
            "rcEffectTaskParWait",
            |x| x.void(RC_EFFECT_TASK_PAR_WAIT, &[o]),
            |m| tb::rc_effect_task_par_wait(m, o),
        );
        t.same(
            "rcEffectTaskAnimeWait",
            |x| x.void(RC_EFFECT_TASK_ANIME_WAIT, &[o]),
            |m| tb::rc_effect_task_anime_wait(m, o),
        );
        let (wa, wb) = match rng.below(4) {
            0 => (o, o),
            1 => (o, 0),
            2 => (0, o),
            _ => (rng.next(), rng.next()),
        };
        t.set(tb::TASK_WAIT_A, wa);
        t.set(tb::TASK_WAIT_B, wb);
        t.same(
            "rcEffectTaskTaskWait",
            |x| x.void(RC_EFFECT_TASK_TASK_WAIT, &[o]),
            |m| tb::rc_effect_task_task_wait(m, o),
        );
        t.set(tb::FLAG_WAIT, rng.below(2));
        t.same(
            "rcEffectTaskFlagWait",
            |x| x.void(RC_EFFECT_TASK_FLAG_WAIT, &[o]),
            |m| tb::rc_effect_task_flag_wait(m, o),
        );
        let (chr, el) = (rng.next(), rng.next());
        t.same(
            "rcefObjAttachChr",
            |x| x.void(RCEF_OBJ_ATTACH_CHR, &[o, chr, el]),
            |m| tb::rcef_obj_attach_chr(m, o, chr, el),
        );
        t.same(
            "rcefObjClearMode",
            |x| x.void(RCEF_OBJ_CLEAR_MODE, &[o]),
            |m| tb::rcef_obj_clear_mode(m, o),
        );
        let mode = rng.next();
        t.same(
            "rcefObjSetMode",
            |x| x.void(RCEF_OBJ_SET_MODE, &[o, mode]),
            |m| tb::rcef_obj_set_mode(m, o, mode),
        );
        let par = o + task::obj::PAR;
        t.same(
            "rcefParInit",
            |x| x.void(RCEF_PAR_INIT, &[par]),
            |m| tb::rcef_par_init(m, par),
        );
        let an = o + task::obj::ANIME;
        t.same(
            "rcefAnimeInit",
            |x| x.void(RCEF_ANIME_INIT, &[an]),
            |m| tb::rcef_anime_init(m, an),
        );
        let (d, o0, o1) = (rng.next(), rng.next(), rng.next());
        t.same(
            "rcefAnimeStart",
            |x| x.void(RCEF_ANIME_START, &[an, d, o0, o1]),
            |m| tb::rcef_anime_start(m, an, d, o0, o1),
        );
    }
    // a controller and tasks sorted into it
    let ctrl = t.alloc(0x180);
    let tasks: Vec<u32> = (0..8)
        .map(|_| t.alloc(task::header::SIZE as usize))
        .collect();
    for round in 0..20 {
        t.fill(ctrl, 0x180, &mut rng);
        t.same(
            "rcefTaskCtrlInit",
            |x| x.void(RCEF_TASK_CTRL_INIT, &[ctrl]),
            |m| tb::rcef_task_ctrl_init(m, ctrl),
        );
        t.same(
            "rcefTaskCtrlIsEnd (empty)",
            |x| x.eax(RCEF_TASK_CTRL_IS_END, &[ctrl]),
            |m| u32::from(tb::rcef_task_ctrl_is_end(m, ctrl)),
        );
        for &k in &tasks {
            t.same(
                "rcefTaskInit",
                |x| x.void(RCEF_TASK_INIT, &[k]),
                |m| tb::rcef_task_init(m, k),
            );
            let p = [0, 0x10, 0x8000, 0xffff, 0x1_0000][rng.below(5) as usize];
            t.put(k + header::PRIORITY, p);
            let first = if round == 0 && k == tasks[0] { 0 } else { ctrl };
            t.same(
                &format!("rcefTaskCtrlAdd priority {p:#x}"),
                |x| x.void(RCEF_TASK_CTRL_ADD, &[first, k]),
                |m| tb::rcef_task_ctrl_add(m, first, k),
            );
            t.same(
                "rcefTaskCtrlIsEnd",
                |x| x.eax(RCEF_TASK_CTRL_IS_END, &[ctrl]),
                |m| u32::from(tb::rcef_task_ctrl_is_end(m, ctrl)),
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn yi_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0042_4242);
    let (out, v, map) = (t.alloc(0x40), t.alloc(0x10), t.alloc(0x10));
    let render = t.alloc(0xc_0000);
    for _ in 0..100 {
        t.set(yi::RENDER_STATUS, rng.next());
        t.same(
            "yiGetRenderStatus",
            |o| o.eax(YI_GET_RENDER_STATUS, &[]),
            |m| yi::bridge::yi_get_render_status(m),
        );
        let bitsv = rng.next();
        t.same(
            "yiRenderEnable",
            |o| o.void(YI_RENDER_ENABLE, &[bitsv]),
            |m| yi::bridge::yi_render_enable(m, bitsv),
        );
        let bitsv = rng.next();
        t.same(
            "yiRenderDisable",
            |o| o.void(YI_RENDER_DISABLE, &[bitsv]),
            |m| yi::bridge::yi_render_disable(m, bitsv),
        );
        let mask = rng.next();
        t.same(
            "yiSetDrawMask",
            |o| o.void(YI_SET_DRAW_MASK, &[mask]),
            |m| yi::bridge::yi_set_draw_mask(m, mask),
        );
        t.same(
            "yiGetDrawMask",
            |o| o.eax(YI_GET_DRAW_MASK, &[]),
            |m| yi::bridge::yi_get_draw_mask(m),
        );
        t.set(yi::SG_VCOUNT, rng.next());
        t.set(yi::SG_FRAME, rng.next());
        t.same(
            "yiGetVCount",
            |o| o.eax(YI_GET_VCOUNT, &[]),
            |m| yi::bridge::yi_get_vcount(m),
        );
        t.same(
            "yiGetFCount",
            |o| o.eax(YI_GET_FCOUNT, &[]),
            |m| yi::bridge::yi_get_fcount(m),
        );
        t.fill(map, 0x10, &mut rng);
        t.set(yi::MAP_DATA, if rng.below(3) == 0 { 0 } else { map });
        t.same(
            "yiGetMapStatus",
            |o| o.eax(YI_GET_MAP_STATUS, &[]),
            |m| yi::bridge::yi_get_map_status(m),
        );
        // display objects
        let count = rng.below(12);
        t.set(yi::DOBJ_COUNT, count);
        let base = yi::DOBJS.ptr(&t.b).addr;
        t.fill(base, 12 * yi::dobj::SIZE as usize, &mut rng);
        for i in 0..12 {
            t.put(
                base + i * yi::dobj::SIZE + yi::dobj::GROUP,
                u16::try_from(rng.below(6)).unwrap(),
            );
        }
        let index = rng.below(14);
        t.same(
            &format!("yiGetMatrixDObj {index} of {count}"),
            |o| o.eax(YI_GET_MATRIX_DOBJ, &[index, out]),
            |m| u(yi::bridge::yi_get_matrix_dobj(m, index, out)),
        );
        let group = [rng.below(7), rng.next()][rng.below(2) as usize];
        t.same(
            &format!("yiGetMatrixGroup {group}"),
            |o| o.eax(YI_GET_MATRIX_GROUP, &[group, out]),
            |m| u(yi::bridge::yi_get_matrix_group(m, group, out)),
        );
        t.put(v, [rng.any_float(), rng.any_float(), rng.any_float()]);
        t.same(
            "yiSetScaleFactor",
            |o| o.void(YI_SET_SCALE_FACTOR, &[v]),
            |m| yi::bridge::yi_set_scale_factor(m, v),
        );
    }
    t.same(
        "yiRender_resettex",
        |o| o.void(YI_RENDER_RESETTEX, &[render]),
        |m| yi::bridge::yi_render_resettex(m, render),
    );
    t.same(
        "yiScreff_init",
        |o| o.void(YI_SCREFF_INIT, &[render]),
        |m| yi::bridge::yi_screff_init(m, render),
    );
}

#[test]
fn vu0_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x00c8_f7a0);
    let vs: Vec<u32> = (0..3).map(|_| t.alloc(0x10)).collect();
    for i in 0..400 {
        for &v in &vs {
            let x: [u32; 4] = if i % 2 == 0 {
                std::array::from_fn(|_| rng.any_float())
            } else {
                std::array::from_fn(|_| rng.float())
            };
            t.put(v, x);
        }
        let pick = |rng: &mut Xorshift| vs[rng.below(3) as usize];
        let (d, s) = (pick(&mut rng), pick(&mut rng));
        t.same(
            "KeVu0CopyVecXYZ",
            |o| o.void(KE_VU0_COPY_VEC_XYZ, &[d, s]),
            |m| vu0::bridge::ke_vu0_copy_vec_xyz(m, d, s),
        );
        if i % 2 == 0 {
            continue; // NaN operands left out of the arithmetic
        }
        let (d, a, b) = (pick(&mut rng), pick(&mut rng), pick(&mut rng));
        t.same(
            "KeVu0AddVecXYZ",
            |o| o.void(KE_VU0_ADD_VEC_XYZ, &[d, a, b]),
            |m| vu0::bridge::ke_vu0_add_vec_xyz(m, d, a, b),
        );
        let sc = [
            rng.float(),
            rng.any_float(),
            rng.any_float(),
            rng.any_float(),
        ];
        let (d, s) = (pick(&mut rng), pick(&mut rng));
        t.same(
            "KeVu0ScaleVecXYZ",
            |o| o.void(KE_VU0_SCALE_VEC_XYZ, &[d, s, sc[0], sc[1], sc[2], sc[3]]),
            |m| vu0::bridge::ke_vu0_scale_vec_xyz(m, d, s, sc),
        );
        let v = pick(&mut rng);
        t.same(
            "KeVu0VecLen",
            |o| o.float(KE_VU0_VEC_LEN, &[v]),
            |m| bits(vu0::bridge::ke_vu0_vec_len(m, v)),
        );
        t.same(
            "KeVu0VecLenXY",
            |o| o.float(KE_VU0_VEC_LEN_XY, &[v]),
            |m| bits(vu0::bridge::ke_vu0_vec_len_xy(m, v)),
        );
        let x = rng.float() & 0x7fff_ffff; // the CRT's domain error left out
        t.same(
            &format!("KeVu0Sqrt {x:#x}"),
            |o| o.float(KE_VU0_SQRT, &[x]),
            |_| bits(vu0::bridge::ke_vu0_sqrt(x)),
        );
        let (a, b, c) = (rng.next(), rng.next(), rng.any_float());
        t.same(
            "KeMdl_MkTkClut",
            |o| o.void(KE_MDL_MK_TK_CLUT, &[a, b, c]),
            |m| vu0::bridge::ke_mdl_mk_tk_clut(m, a, b, c),
        );
    }
}

#[test]
fn nkz_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    t.same(
        "NkzVersion",
        |o| o.eax(NKZ_VERSION, &[]),
        |m| nkz::bridge::nkz_version(m),
    );
    t.same(
        "NkzGetLuluLimitTime",
        |o| o.float(NKZ_GET_LULU_LIMIT_TIME, &[]),
        |m| bits(nkz::bridge::nkz_get_lulu_limit_time(m)),
    );
    for i in 0..8 {
        t.same(
            "NkzGetAuronLimitTime",
            |o| o.float(NKZ_GET_AURON_LIMIT_TIME, &[i]),
            |m| {
                bits(nkz::bridge::nkz_get_auron_limit_time(
                    m,
                    i32::try_from(i).unwrap(),
                ))
            },
        );
        t.same(
            "NkzGetTidusLimitTime",
            |o| o.float(NKZ_GET_TIDUS_LIMIT_TIME, &[i]),
            |m| {
                bits(nkz::bridge::nkz_get_tidus_limit_time(
                    m,
                    i32::try_from(i).unwrap(),
                ))
            },
        );
        t.same(
            "NkzGetAuronLimitButtonData",
            |o| o.eax(NKZ_GET_AURON_LIMIT_BUTTON_DATA, &[i]),
            |m| nkz::bridge::nkz_get_auron_limit_button_data(m, i),
        );
    }
    let mut rng = Xorshift(0x5a5a_5a5a);
    for _ in 0..10 {
        let at = nkz::SQS_IOP_STATUS.ptr(&t.b).addr;
        t.fill(at, 0x10, &mut rng);
        for i in 0..4 {
            t.same(
                "SqsReportIopSoundStatus",
                |o| o.eax(SQS_REPORT_IOP_SOUND_STATUS, &[i]),
                |m| nkz::bridge::sqs_report_iop_sound_status(m, i),
            );
        }
        t.same(
            "SqsIopSoundSystemStatus",
            |o| o.eax(SQS_IOP_SOUND_SYSTEM_STATUS, &[]),
            |m| nkz::bridge::sqs_iop_sound_system_status(m),
        );
        t.same(
            "SqsIopSoundErrorStatus",
            |o| o.eax(SQS_IOP_SOUND_ERROR_STATUS, &[]),
            |m| nkz::bridge::sqs_iop_sound_error_status(m),
        );
    }
}

/// The game's own state: the guide map and its minimap mesh, the display
/// objects, the renderer's flags.
#[test]
#[allow(clippy::too_many_lines)] // one walk over the state
fn field_state_matches_the_original_on_a_snapshot() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    t.same(
        "Yn_GuideMapGetMapNo",
        |o| o.eax(YN_GUIDE_MAP_GET_MAP_NO, &[]),
        |m| u(guide::bridge::yn_guide_map_get_map_no(m)),
    );
    t.same(
        "yiGetMapStatus",
        |o| o.eax(YI_GET_MAP_STATUS, &[]),
        |m| yi::bridge::yi_get_map_status(m),
    );
    for (name, f, rust) in [
        (
            "yiGetRenderStatus",
            YI_GET_RENDER_STATUS,
            yi::bridge::yi_get_render_status as fn(&Oracle) -> u32,
        ),
        (
            "yiGetDrawMask",
            YI_GET_DRAW_MASK,
            yi::bridge::yi_get_draw_mask,
        ),
        ("yiGetVCount", YI_GET_VCOUNT, yi::bridge::yi_get_vcount),
        ("yiGetFCount", YI_GET_FCOUNT, yi::bridge::yi_get_fcount),
    ] {
        t.same(name, |o| o.eax(f, &[]), |m| rust(m));
    }
    let out = t.alloc(0x40);
    let count = yi::DOBJ_COUNT.get(&t.b);
    eprintln!("{count} display objects");
    for i in 0..=count.min(64) {
        t.same(
            &format!("yiGetMatrixDObj {i}"),
            |o| o.eax(YI_GET_MATRIX_DOBJ, &[i, out]),
            |m| u(yi::bridge::yi_get_matrix_dobj(m, i, out)),
        );
    }
    for g in 0..16 {
        t.same(
            &format!("yiGetMatrixGroup {g}"),
            |o| o.eax(YI_GET_MATRIX_GROUP, &[g, out]),
            |m| u(yi::bridge::yi_get_matrix_group(m, g, out)),
        );
    }
    // the fade from where the game left it, shown then hidden
    for show in [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] {
        t.same(
            &format!("Guide_FadeStep({show})"),
            |o| o.void(GUIDE_FADE_STEP, &[show]),
            |m| guide::bridge::guide_fade_step(m, i32::try_from(show).unwrap()),
        );
    }
    let sys = guide::YN_MAP_SYSTEM.get(&t.b);
    let g = if sys == 0 {
        0
    } else {
        Mem::read::<u32>(&t.b, sys + 4)
    };
    if g == 0 {
        eprintln!("no guide map in this snapshot; skipping the minimap mesh");
        return;
    }
    t.same(
        "Guide_MeshObject",
        |o| o.eax(GUIDE_MESH_OBJECT, &[g]),
        |m| guide::bridge::guide_mesh_object(m, g),
    );
    let mesh = guide::bridge::guide_mesh_object(&t.b, g);
    let blob = Mem::read::<u32>(&t.b, mesh + model::obj::OMD);
    if blob == 0 {
        eprintln!("the minimap mesh has no blob; skipping it");
        return;
    }
    // packets up to the end marker (0xffff in the first two bytes)
    let mut n = 0;
    loop {
        let h = model::bridge::rcbg_model_get_prim_header(&t.b, mesh, n);
        t.same(
            &format!("rcbgModelGetPrimHeader {n}"),
            |o| o.eax(RCBG_MODEL_GET_PRIM_HEADER, &[mesh, u(n)]),
            |m| model::bridge::rcbg_model_get_prim_header(m, mesh, n),
        );
        t.same(
            &format!("rcbgModelGetPrim {n}"),
            |o| o.eax(RCBG_MODEL_GET_PRIM, &[mesh, u(n)]),
            |m| model::bridge::rcbg_model_get_prim(m, mesh, n),
        );
        if Mem::read::<u16>(&t.b, h) == 0xffff || n > 256 {
            break;
        }
        n += 1;
    }
    let h = model::bridge::rcbg_model_get_prim_header(&t.b, mesh, 0);
    eprintln!(
        "minimap mesh: {n} packets (blob +4 {:#x}, +8 {:#x}, {} prims, {} vertices; first type {} x {})",
        Mem::read::<u32>(&t.b, blob + model::omd::PACKETS),
        Mem::read::<u32>(&t.b, blob + model::omd::VERTICES),
        Mem::read::<i16>(&t.b, blob + model::omd::PRIMS),
        Mem::read::<u16>(&t.b, blob + model::omd::VERTEX_COUNT),
        Mem::read::<i8>(&t.b, h + model::packet::TYPE),
        Mem::read::<i16>(&t.b, h + model::packet::COUNT),
    );
    if Mem::read::<u32>(&t.b, blob + model::omd::PACKETS) == model::omd::FIRST_PACKET {
        for alpha in [0x40u32, 0x80] {
            t.same(
                "rcbgModelSetAlpha",
                |o| o.void(RCBG_MODEL_SET_ALPHA, &[mesh, alpha]),
                |m| model::bridge::rcbg_model_set_alpha(m, mesh, alpha.to_le_bytes()[0]),
            );
        }
    }
    t.same(
        "rcbgGuideBattleBegin",
        |o| o.void(RCBG_GUIDE_BATTLE_BEGIN, &[g]),
        |m| guide::bridge::rcbg_guide_battle_begin(m, g),
    );
    t.same(
        "Yn_BattleEnd",
        |o| o.void(YN_BATTLE_END, &[]),
        guide::bridge::yn_battle_end,
    );
}

// Round 2: the guide map's settings, scenes, more rcdt/rcbg/rcef/yi, rcui, Ke.
const RCBG_GUIDE_SET_POS: u32 = 0x0091_d900;
const RCBG_GUIDE_SET_SIZE: u32 = 0x0091_d920;
const RCBG_GUIDE_SET_TARGET: u32 = 0x0091_d930;
const RCBG_GUIDE_GET_TARGET: u32 = 0x0091_d480;
const RCBG_GUIDE_SET_FLAGS: u32 = 0x0091_d430;
const RCBG_GUIDE_CLEAR_FLAGS: u32 = 0x0091_d420;
const RCBG_GUIDE_SET_FLAG3: u32 = 0x0091_d510;
const RCBG_GUIDE_GET_LIMIT: u32 = 0x0091_d450;
const RCBG_GUIDE_SET_LIMIT: u32 = 0x0091_d580;
const GUIDE_SET_ENABLED: u32 = 0x0091_f640;
const YN_GUIDE_MAP_SET_MODE: u32 = 0x0091_9e20;
const YN_GUIDE_MAP_SET_FLAGS: u32 = 0x0091_9c30;
const YN_GUIDE_MAP_CLEAR_FLAGS: u32 = 0x0091_9c00;
const YN_GUIDE_MAP_SET_FLAG3: u32 = 0x0091_9d40;
const YN_GUIDE_MAP_GET_LIMIT: u32 = 0x0091_9ca0;
const YN_GUIDE_MAP_SET_LIMIT: u32 = 0x0091_9df0;
const YN_GUIDE_MAP_SET_POS: u32 = 0x0091_9e50;
const YN_GUIDE_MAP_SET_SIZE: u32 = 0x0091_9e80;
const YN_MAP_PART_COUNT: u32 = 0x0091_a3d0;
const YN_MAP_PART_ANIME_ON: u32 = 0x0091_9f90;
const YN_MAP_PART_ANIME_OFF: u32 = 0x0091_9f40;
const YN_PSIM_GET_CNT: u32 = 0x0091_9ef0;
const YN_SEA_GET_MATRIX: u32 = 0x0091_ae00;
const RCBG_SCENE_INIT: u32 = 0x0092_09e0;
const RCBG_SCENE_GET_OBJECT: u32 = 0x0092_0950;
const RCBG_SCENE_COUNT_OBJECTS: u32 = 0x0092_09a0;
const RCBG_CHUNK_TAG: u32 = 0x0092_1b90;
const RCBG_LAYER_INIT: u32 = 0x0092_ad20;
const RCDT_PARSE_SET_MODE: u32 = 0x0092_c0c0;
const RCDT_PARSE_ADD_SEPARATORS: u32 = 0x0092_c0f0;
const RCDT_CHUNK_SET: u32 = 0x0092_b920;
const RCDT_CHUNK_GET_END: u32 = 0x0092_b820;
const RCDT_CHUNK_IS_EMPTY: u32 = 0x0092_b860;
const RCDT_TM2_INIT: u32 = 0x0092_37e0;
const RCBG_MODEL_GET_SCALE: u32 = 0x0092_8300;
const RCBG_TRX_GET_CNT: u32 = 0x0092_d4c0;
const RCBG_TRX_GET_ENTRY: u32 = 0x0092_d4d0;
const RCBG_TRX_GET_ENTRY_DATA: u32 = 0x0092_d4f0;
const RCBG_PSIM_INIT: u32 = 0x0091_fe00;
const RCBG_PSIM_GET_OMD_IN: u32 = 0x0091_fdc0;
const RCBG_FPM173_END: u32 = 0x0092_2b50;
const RC_PAD16: u32 = 0x0092_b2c0;
const RC_LOG2_CEIL: u32 = 0x0092_9910;
const RCEF_MODEL_INIT: u32 = 0x0092_f750;
const RCEF_IS_BATTLE: u32 = 0x0092_e620;
const RCEF_DIR_TO_ANGLE: u32 = 0x0092_de30;
const RC_VEC3F_DOT: u32 = 0x0092_1220;
const RC_BBOX_SET: u32 = 0x0092_1530;
const RC_CALC_PERS_MATRIX: u32 = 0x0092_0020;
const RCBG_UPDATE_MATRIX: u32 = 0x0092_b160;
const YI_SET_RENDER_PARAM: u32 = 0x0090_8580;
const YI_CHECK_DOBJ: u32 = 0x0090_86b0;
const YI_SET_COLOR3: u32 = 0x0090_a400;
const RCUI_MOUSE_MOVE_END: u32 = 0x0093_c050;
const RCUI_MOUSE_MOVE_RESET: u32 = 0x0093_c110;
const RCUI_MOUSE_MOVE_GET_ID: u32 = 0x0093_c100;
const RCUI_MOUSE_MOVE_IS_ID: u32 = 0x0093_c120;
const RCUI_NUMBER_IS_OK: u32 = 0x0093_c3d0;
const RCUI_COPY_VECTOR: u32 = 0x0093_ce50;
const RCUI_COPY_MATRIX: u32 = 0x0093_ce10;
const KE_TH_RES_HD_INIT: u32 = 0x0076_4bb0;
const KE_TH_CP_INIT: u32 = 0x0076_4800;
const KE_LNS_FLS_INIT: u32 = 0x0076_9690;
const KE_LNS_CRN_INIT: u32 = 0x0076_a170;
const KE_LNS_CLM_INIT: u32 = 0x0076_9ca0;
const KE_LNS_FLR_INIT: u32 = 0x0076_a1d0;

/// Yn's field state with a guide and a scene of `parts` layers; `case`
/// leaves out one link (1 system, 2 guide, 3 map, 4 scene; 0 none).
fn yn_state(t: &mut Twin, rng: &mut Xorshift, case: u32, parts: i32) -> u32 {
    let (sys, map, sc, g) = (t.alloc(8), t.alloc(0x10), t.alloc(0x110), t.alloc(0x80));
    let layers = t.alloc(0xc0 * 8);
    t.fill(g, 0x80, rng);
    t.fill(layers, 0xc0 * 8, rng);
    t.set(guide::YN_MAP_SYSTEM, if case == 1 { 0 } else { sys });
    t.put(sys + guide::system::GUIDE, if case == 2 { 0 } else { g });
    t.put(sys + guide::system::MAP, if case == 3 { 0 } else { map });
    t.put(map + guide::map::SCENE, if case == 4 { 0 } else { sc });
    t.put(sc + scene::layout::LAYER_COUNT, parts);
    t.put(sc + scene::layout::LAYERS, layers);
    g
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn guide_settings_match_the_original() {
    use guide::bridge as gb;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x6d61_7032);
    for _ in 0..100 {
        let case = rng.below(5);
        let parts = rng.range(-1, 8);
        let g = yn_state(&mut t, &mut rng, case, parts);
        let target = u(rng.range(-1, 6));
        t.same(
            &format!("rcbgGuideSetTarget {target}"),
            |o| o.void(RCBG_GUIDE_SET_TARGET, &[g, target]),
            |m| gb::rcbg_guide_set_target(m, g, target),
        );
        t.same(
            "rcbgGuideGetTarget",
            |o| o.eax(RCBG_GUIDE_GET_TARGET, &[]),
            |m| gb::rcbg_guide_get_target(m),
        );
        let (x, y, s) = (rng.next(), rng.next(), rng.next());
        t.same(
            "rcbgGuideSetPos",
            |o| o.void(RCBG_GUIDE_SET_POS, &[g, x, y]),
            |m| gb::rcbg_guide_set_pos(m, g, x, y),
        );
        t.same(
            "rcbgGuideSetSize",
            |o| o.void(RCBG_GUIDE_SET_SIZE, &[g, s]),
            |m| gb::rcbg_guide_set_size(m, g, s),
        );
        let mask = rng.next();
        t.same(
            "rcbgGuideSetFlags",
            |o| o.void(RCBG_GUIDE_SET_FLAGS, &[g, mask]),
            |m| gb::rcbg_guide_set_flags(m, g, mask),
        );
        let mask = rng.next();
        t.same(
            "rcbgGuideClearFlags",
            |o| o.void(RCBG_GUIDE_CLEAR_FLAGS, &[g, mask]),
            |m| gb::rcbg_guide_clear_flags(m, g, mask),
        );
        let on = [0, 1, 0x100][rng.below(3) as usize];
        t.same(
            "rcbgGuideSetFlag3",
            |o| o.void(RCBG_GUIDE_SET_FLAG3, &[g, on]),
            |m| gb::rcbg_guide_set_flag3(m, g, i32::try_from(on).unwrap()),
        );
        let limit = rng.any_float();
        t.same(
            "rcbgGuideSetLimit",
            |o| o.void(RCBG_GUIDE_SET_LIMIT, &[g, limit]),
            |m| gb::rcbg_guide_set_limit(m, g, limit),
        );
        t.same(
            "rcbgGuideGetLimit",
            |o| o.float(RCBG_GUIDE_GET_LIMIT, &[g]),
            |m| bits(gb::rcbg_guide_get_limit(m, g)),
        );
        let on = rng.next();
        t.same(
            "Guide_SetEnabled",
            |o| o.void(GUIDE_SET_ENABLED, &[on]),
            |m| gb::guide_set_enabled(m, on),
        );
        // the Yn wrappers, on whatever links exist
        let target = u(rng.range(-1, 6));
        t.same(
            &format!("Yn_GuideMapSetMode {target} case {case}"),
            |o| o.void(YN_GUIDE_MAP_SET_MODE, &[target]),
            |m| gb::yn_guide_map_set_mode(m, target),
        );
        let mask = rng.next();
        t.same(
            "Yn_GuideMapSetFlags",
            |o| o.void(YN_GUIDE_MAP_SET_FLAGS, &[mask]),
            |m| gb::yn_guide_map_set_flags(m, mask),
        );
        let mask = rng.next();
        t.same(
            "Yn_GuideMapClearFlags",
            |o| o.void(YN_GUIDE_MAP_CLEAR_FLAGS, &[mask]),
            |m| gb::yn_guide_map_clear_flags(m, mask),
        );
        let on = rng.below(2);
        t.same(
            "Yn_GuideMapSetFlag3",
            |o| o.void(YN_GUIDE_MAP_SET_FLAG3, &[on]),
            |m| gb::yn_guide_map_set_flag3(m, i32::try_from(on).unwrap()),
        );
        let limit = rng.any_float();
        t.same(
            "Yn_GuideMapSetLimit",
            |o| o.void(YN_GUIDE_MAP_SET_LIMIT, &[limit]),
            |m| gb::yn_guide_map_set_limit(m, limit),
        );
        t.same(
            "Yn_GuideMapGetLimit",
            |o| o.float(YN_GUIDE_MAP_GET_LIMIT, &[]),
            |m| bits(gb::yn_guide_map_get_limit(m)),
        );
        let (x, y) = (rng.next(), rng.next());
        t.same(
            "Yn_GuideMapSetPos",
            |o| o.void(YN_GUIDE_MAP_SET_POS, &[x, y]),
            |m| gb::yn_guide_map_set_pos(m, x, y),
        );
        let s = rng.next();
        t.same(
            "Yn_GuideMapSetSize",
            |o| o.void(YN_GUIDE_MAP_SET_SIZE, &[s]),
            |m| gb::yn_guide_map_set_size(m, s),
        );
        t.same(
            &format!("Yn_MapPartCount case {case} parts {parts}"),
            |o| o.eax(YN_MAP_PART_COUNT, &[]),
            |m| u(gb::yn_map_part_count(m)),
        );
        // without a scene the count is 0: only parts >= 0 return early
        // there (the original dereferences null otherwise, as the bridge)
        let part = rng.range(-1, 9);
        if case == 0 || part >= 0 {
            t.same(
                &format!("Yn_MapPartAnimeOn {part}"),
                |o| o.void(YN_MAP_PART_ANIME_ON, &[u(part)]),
                |m| gb::yn_map_part_anime_on(m, part),
            );
            t.same(
                &format!("Yn_MapPartAnimeOff {part}"),
                |o| o.void(YN_MAP_PART_ANIME_OFF, &[u(part)]),
                |m| gb::yn_map_part_anime_off(m, part),
            );
        }
    }
    t.same(
        "Yn_PSimGetCnt",
        |o| o.eax(YN_PSIM_GET_CNT, &[]),
        |_| gb::yn_psim_get_cnt(),
    );
    t.same(
        "Yn_SeaGetMatrix",
        |o| o.eax(YN_SEA_GET_MATRIX, &[]),
        |m| gb::yn_sea_get_matrix(m),
    );
}

#[test]
fn scenes_match_the_original() {
    use scene::bridge as sb;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x5ce0_0001);
    let (sc, objs, list) = (t.alloc(0x110), t.alloc(0x400), t.alloc(0x40));
    for _ in 0..100 {
        t.fill(sc, 0x110, &mut rng);
        t.same(
            "rcbgSceneInit",
            |o| o.void(RCBG_SCENE_INIT, &[sc]),
            |m| sb::rcbg_scene_init(m, sc),
        );
        let n = rng.range(-1, 12);
        t.put(sc + scene::layout::OBJECT_COUNT, n);
        t.put(sc + scene::layout::OBJECTS, list);
        for i in 0..12 {
            let o = objs + 0x20 * i;
            t.put(list + 4 * i, o);
            t.put(o, [0x20002u32, 0x20004, 0x20008][rng.below(3) as usize]);
        }
        let kind = [0x20002u32, 0x20004, 0x20008, 0][rng.below(4) as usize];
        t.same(
            "rcbgSceneCountObjects",
            |o| o.eax(RCBG_SCENE_COUNT_OBJECTS, &[sc, kind]),
            |m| sb::rcbg_scene_count_objects(m, sc, kind),
        );
        let k = rng.below(6);
        t.same(
            &format!("rcbgSceneGetObject {kind:#x} {k} of {n}"),
            |o| o.eax(RCBG_SCENE_GET_OBJECT, &[sc, kind, k]),
            |m| sb::rcbg_scene_get_object(m, sc, kind, k),
        );
        let layer = t.alloc(0xc0);
        t.fill(layer, 0xc0, &mut rng);
        t.same(
            "rcbgLayerInit",
            |o| o.void(RCBG_LAYER_INIT, &[layer]),
            |m| sb::rcbg_layer_init(m, layer),
        );
    }
    for i in 0..16 {
        t.same(
            "rcbgChunkTag",
            |o| o.eax(RCBG_CHUNK_TAG, &[i]),
            |m| sb::rcbg_chunk_tag(m, i),
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn more_rc_objects_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0xabad_1dea);
    let (p, c, s, obj, mdl) = (
        t.alloc(0x200),
        t.alloc(8),
        t.alloc(0x40),
        t.alloc(0x100),
        t.alloc(0x100),
    );
    for _ in 0..200 {
        let mode = rng.next();
        t.same(
            "rcdtParseSetMode",
            |o| o.void(RCDT_PARSE_SET_MODE, &[p, mode]),
            |m| rcdt::bridge::rcdt_parse_set_mode(m, p, mode),
        );
        let len = rng.below(12);
        let mut chars: Vec<u8> = (0..len)
            .map(|_| u8::try_from(1 + rng.next() % 0xff).unwrap())
            .collect();
        chars.push(0);
        t.write(s, &chars);
        t.same(
            "rcdtParseAddSeparators",
            |o| o.void(RCDT_PARSE_ADD_SEPARATORS, &[p, s]),
            |m| rcdt::bridge::rcdt_parse_add_separators(m, p, s),
        );
        let data = [0, rng.next()][rng.below(2) as usize];
        let size = [0, rng.next()][rng.below(2) as usize];
        t.same(
            "rcdtChunkSet",
            |o| o.void(RCDT_CHUNK_SET, &[c, data, size]),
            |m| rcdt::bridge::rcdt_chunk_set(m, c, data, size),
        );
        t.same(
            "rcdtChunkGetEnd",
            |o| o.eax(RCDT_CHUNK_GET_END, &[c]),
            |m| rcdt::bridge::rcdt_chunk_get_end(m, c),
        );
        t.same(
            "rcdtChunkIsEmpty",
            |o| o.eax(RCDT_CHUNK_IS_EMPTY, &[c]),
            |m| u32::from(rcdt::bridge::rcdt_chunk_is_empty(m, c)),
        );
        t.fill(obj, 0x100, &mut rng);
        t.same(
            "rcdtTm2Init",
            |o| o.void(RCDT_TM2_INIT, &[obj]),
            |m| rcdt::bridge::rcdt_tm2_init(m, obj),
        );
        let scale = rng.float();
        t.put(mdl + model::obj::MATRIX, scale);
        t.same(
            &format!("rcbgModelGetScale {scale:#x}"),
            |o| o.float(RCBG_MODEL_GET_SCALE, &[mdl]),
            |m| bits(model::bridge::rcbg_model_get_scale(m, mdl)),
        );
        t.fill(obj, 0x40, &mut rng);
        let n = rng.next() % 64;
        t.same(
            "rcbgTrxGetCnt",
            |o| o.eax(RCBG_TRX_GET_CNT, &[obj]),
            |m| model::bridge::rcbg_trx_get_cnt(m, obj),
        );
        t.same(
            "rcbgTrxGetEntry",
            |o| o.eax(RCBG_TRX_GET_ENTRY, &[obj, n]),
            |m| model::bridge::rcbg_trx_get_entry(m, obj, n),
        );
        t.same(
            "rcbgTrxGetEntryData",
            |o| o.eax(RCBG_TRX_GET_ENTRY_DATA, &[obj, n]),
            |m| model::bridge::rcbg_trx_get_entry_data(m, obj, n),
        );
        t.same(
            "rcbgPSimGetOmdIn",
            |o| o.eax(RCBG_PSIM_GET_OMD_IN, &[obj]),
            |m| model::bridge::rcbg_psim_get_omd_in(m, obj),
        );
        t.fill(obj, 0x100, &mut rng);
        t.same(
            "rcbgPSimInit",
            |o| o.void(RCBG_PSIM_INIT, &[obj]),
            |m| model::bridge::rcbg_psim_init(m, obj),
        );
        t.same(
            "rcbgFPm173End",
            |o| o.void(RCBG_FPM173_END, &[obj]),
            |m| model::bridge::rcbg_fpm173_end(m, obj),
        );
        let x = rng.next() >> rng.below(32);
        t.same(
            &format!("rcPad16 {x:#x}"),
            |o| o.eax(RC_PAD16, &[x]),
            |_| u(model::pad16(x.cast_signed())),
        );
        let x = u(rng.range(-3, 0x1_0010)) >> rng.below(12);
        t.same(
            &format!("rcLog2Ceil {x:#x}"),
            |o| o.eax(RC_LOG2_CEIL, &[x]),
            |_| model::log2_ceil(x.cast_signed()),
        );
        t.fill(obj, 0x100, &mut rng);
        t.same(
            "rcefModelInit",
            |o| o.void(RCEF_MODEL_INIT, &[obj]),
            |m| task::bridge::rcef_model_init(m, obj),
        );
        t.set(
            ffx_battle::state::bridge::STATUS,
            [0i8, 1, -1, 3][rng.below(4) as usize],
        );
        t.same(
            "rcefIsBattle",
            |o| o.eax(RCEF_IS_BATTLE, &[]),
            |m| u32::from(task::bridge::rcef_is_battle(m)),
        );
        let d = rng.float();
        t.same(
            &format!("rcefDirToAngle {d:#x}"),
            |o| o.float(RCEF_DIR_TO_ANGLE, &[d]),
            |m| bits(task::bridge::rcef_dir_to_angle(m, d)),
        );
    }
}

#[test]
fn more_geometry_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0000_d07d);
    let (a, b, bx, pts, m, dst) = (
        t.alloc(0x10),
        t.alloc(0x10),
        t.alloc(0x20),
        t.alloc(0x100),
        t.alloc(0x40),
        t.alloc(0x40),
    );
    for _ in 0..300 {
        let va: [u32; 4] = std::array::from_fn(|_| rng.float());
        let vb: [u32; 4] = std::array::from_fn(|_| rng.float());
        t.put(a, va);
        t.put(b, vb);
        t.same(
            &format!("rcVec3fDot {va:x?} {vb:x?}"),
            |o| o.float(RC_VEC3F_DOT, &[a, b]),
            |mm| bits(geom::bridge::rc_vec3f_dot(mm, a, b)),
        );
        let n = rng.range(-1, 8);
        let points: [u32; 32] = std::array::from_fn(|i| {
            if i % 4 == 3 || rng.below(10) == 0 {
                0x7fc0_0000 // NaNs never win
            } else {
                rng.float()
            }
        });
        t.put(pts, points);
        t.same(
            &format!("rcBBoxSet {n}"),
            |o| o.void(RC_BBOX_SET, &[bx, pts, u(n)]),
            |mm| geom::bridge::rc_bbox_set(mm, bx, pts, n),
        );
    }
    for _ in 0..100 {
        for g in [
            geom::CAMERA_MATRIX,
            geom::SCREEN_MATRIX,
            geom::WORLD,
            geom::CLIP,
        ] {
            let at = g.ptr(&t.b).addr;
            let v: [u32; 16] = std::array::from_fn(|_| rng.float());
            t.put(at, v);
        }
        let v: [u32; 16] = std::array::from_fn(|_| rng.float());
        t.put(m, v);
        let target = [dst, m][rng.below(2) as usize];
        t.same(
            "rcCalcPersMatrix",
            |o| o.void(RC_CALC_PERS_MATRIX, &[target, m]),
            |mm| geom::bridge::rc_calc_pers_matrix(mm, target, m),
        );
        t.same(
            "rcbgUpdateMatrix",
            |o| o.void(RCBG_UPDATE_MATRIX, &[]),
            geom::bridge::rcbg_update_matrix,
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn yi_rcui_and_ke_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x7e57_0002);
    let (w, inp, v, mt) = (t.alloc(0xc00), t.alloc(0x40), t.alloc(0x40), t.alloc(0x80));
    let hd = t.alloc(0x2_0000);
    let (o, src, data) = (t.alloc(0x20), t.alloc(4), t.alloc(0x20));
    for _ in 0..100 {
        let p = rng.next();
        t.same(
            "yiSetRenderParam",
            |x| x.void(YI_SET_RENDER_PARAM, &[p]),
            |m| yi::bridge::yi_set_render_param(m, p),
        );
        t.set(yi::DOBJ_COUNT, rng.below(8));
        let i = rng.below(10);
        t.same(
            "yiCheckDObj",
            |x| x.eax(YI_CHECK_DOBJ, &[i]),
            |m| u(yi::bridge::yi_check_dobj(m, i)),
        );
        let (r, g, b) = (rng.next(), rng.next(), rng.next());
        t.same(
            "yiSetColor3",
            |x| x.eax(YI_SET_COLOR3, &[r, g, b]),
            |m| {
                let [r, g, b] = [r, g, b].map(|c| c.to_le_bytes()[0]);
                yi::bridge::yi_set_color3(m, r, g, b)
            },
        );
        t.set(rcui::WORK, w);
        t.fill(w + 0xbe0, 0x20, &mut rng);
        if rng.below(2) == 0 {
            t.put(w + rcui::work::MOVE_ID, 5u32);
        }
        let id = [5, rng.next()][rng.below(2) as usize];
        t.same(
            "rcuiMouseMoveIsID",
            |x| x.eax(RCUI_MOUSE_MOVE_IS_ID, &[id]),
            |m| u32::from(rcui::bridge::rcui_mouse_move_is_id(m, id)),
        );
        t.same(
            "rcuiMouseMoveGetID",
            |x| x.eax(RCUI_MOUSE_MOVE_GET_ID, &[]),
            |m| rcui::bridge::rcui_mouse_move_get_id(m),
        );
        let f = [RCUI_MOUSE_MOVE_END, RCUI_MOUSE_MOVE_RESET][rng.below(2) as usize];
        t.same(
            "rcuiMouseMoveEnd/Reset",
            |x| x.void(f, &[]),
            rcui::bridge::rcui_mouse_move_end,
        );
        t.fill(inp, 0x40, &mut rng);
        t.same(
            "rcuiNumberIsOK",
            |x| x.eax(RCUI_NUMBER_IS_OK, &[inp]),
            |m| rcui::bridge::rcui_number_is_ok(m, inp),
        );
        let fs: [u32; 32] = std::array::from_fn(|_| rng.any_float());
        t.put(mt, fs);
        // overlapping copies too
        let (d, s) = [(v, mt), (mt, mt + 4), (mt + 4, mt)][rng.below(3) as usize];
        t.same(
            "rcuiCopyVector",
            |x| x.void(RCUI_COPY_VECTOR, &[d, s]),
            |m| rcui::bridge::rcui_copy_vector(m, d, s),
        );
        t.same(
            "rcuiCopyMatrix",
            |x| x.void(RCUI_COPY_MATRIX, &[d, s]),
            |m| rcui::bridge::rcui_copy_matrix(m, d, s),
        );
        let (count, size) = (rng.below(9), rng.below(256));
        t.same(
            &format!("KeThResHd_Init {count} {size}"),
            |x| x.void(KE_TH_RES_HD_INIT, &[hd, count | 0xab00, size | 0xcd00]),
            |m| {
                let [c, _, _, _] = count.to_le_bytes();
                let [z, _, _, _] = size.to_le_bytes();
                ke::bridge::ke_th_res_hd_init(m, hd, c, z);
            },
        );
        t.fill(o, 0x20, &mut rng);
        t.same(
            "KeThCp_Init",
            |x| x.void(KE_TH_CP_INIT, &[o]),
            |m| ke::bridge::ke_th_cp_init(m, o),
        );
        for f in [KE_LNS_FLS_INIT, KE_LNS_CRN_INIT] {
            t.fill(o, 0x20, &mut rng);
            t.same(
                "KeLnsFls_Init/KeLnsCrn_Init",
                |x| x.void(f, &[o]),
                |m| ke::bridge::ke_lns_init(m, o),
            );
        }
        t.fill(o, 0x20, &mut rng);
        t.same(
            "KeLnsClm_Init",
            |x| x.void(KE_LNS_CLM_INIT, &[o]),
            |m| ke::bridge::ke_lns_clm_init(m, o),
        );
        t.fill(data, 0x20, &mut rng);
        t.put(src, data);
        t.same(
            "KeLnsFlr_Init",
            |x| x.void(KE_LNS_FLR_INIT, &[o, src]),
            |m| ke::bridge::ke_lns_flr_init(m, o, src),
        );
    }
}

/// Round 2 on the game's own state: the scene, the guide settings, the
/// background matrices, the battle flag.
#[test]
fn more_field_state_matches_the_original_on_a_snapshot() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    t.same(
        "Yn_MapPartCount",
        |o| o.eax(YN_MAP_PART_COUNT, &[]),
        |m| u(guide::bridge::yn_map_part_count(m)),
    );
    t.same(
        "rcbgGuideGetTarget",
        |o| o.eax(RCBG_GUIDE_GET_TARGET, &[]),
        |m| guide::bridge::rcbg_guide_get_target(m),
    );
    t.same(
        "rcefIsBattle",
        |o| o.eax(RCEF_IS_BATTLE, &[]),
        |m| u32::from(task::bridge::rcef_is_battle(m)),
    );
    t.same(
        "rcbgUpdateMatrix",
        |o| o.void(RCBG_UPDATE_MATRIX, &[]),
        geom::bridge::rcbg_update_matrix,
    );
    let dst = t.alloc(0x40);
    let world = geom::WORLD.ptr(&t.b).addr;
    t.same(
        "rcCalcPersMatrix",
        |o| o.void(RC_CALC_PERS_MATRIX, &[dst, world]),
        |m| geom::bridge::rc_calc_pers_matrix(m, dst, world),
    );
    let sys = guide::YN_MAP_SYSTEM.get(&t.b);
    if sys == 0 {
        eprintln!("no Yn field state in this snapshot; skipping the scene");
        return;
    }
    t.same(
        "Yn_GuideMapGetLimit",
        |o| o.float(YN_GUIDE_MAP_GET_LIMIT, &[]),
        |m| bits(guide::bridge::yn_guide_map_get_limit(m)),
    );
    let map = Mem::read::<u32>(&t.b, sys);
    let sc = if map == 0 {
        0
    } else {
        Mem::read::<u32>(&t.b, map + guide::map::SCENE)
    };
    if sc == 0 {
        return;
    }
    let n = Mem::read::<i32>(&t.b, sc + scene::layout::OBJECT_COUNT);
    let parts = guide::bridge::yn_map_part_count(&t.b);
    eprintln!("scene: {parts} map parts, {n} objects");
    for kind in [0x20002u32, 0x20004, 0x20008] {
        t.same(
            &format!("rcbgSceneCountObjects {kind:#x}"),
            |o| o.eax(RCBG_SCENE_COUNT_OBJECTS, &[sc, kind]),
            |m| scene::bridge::rcbg_scene_count_objects(m, sc, kind),
        );
        for k in 0..4 {
            t.same(
                &format!("rcbgSceneGetObject {kind:#x} {k}"),
                |o| o.eax(RCBG_SCENE_GET_OBJECT, &[sc, kind, k]),
                |m| scene::bridge::rcbg_scene_get_object(m, sc, kind, k),
            );
        }
    }
    for part in 0..parts.min(4) {
        t.same(
            &format!("Yn_MapPartAnimeOn {part}"),
            |o| o.void(YN_MAP_PART_ANIME_ON, &[u(part)]),
            |m| guide::bridge::yn_map_part_anime_on(m, part),
        );
        t.same(
            &format!("Yn_MapPartAnimeOff {part}"),
            |o| o.void(YN_MAP_PART_ANIME_OFF, &[u(part)]),
            |m| guide::bridge::yn_map_part_anime_off(m, part),
        );
    }
}

// Round 3: rcdt/packet getters, yi display objects and helpers, the guide
// screen and scene parameters, rcef targets, lerps, rcui values.
const RCDT_CHUNK_GET_SIZE: u32 = 0x0092_b830;
const RCDT_PARSE_GET_CHUNK: u32 = 0x0092_bb20;
const RCDT_PARSE_GET_POS: u32 = 0x0092_bda0;
const RCDT_PARSE_GET_MODE: u32 = 0x0092_bd10;
const RC_PACKET_DATA_SET_ADR: u32 = 0x0092_7a40;
const RC_PACKET_DATA_OPEN: u32 = 0x0092_7a50;
const YI_SET_MATRIX_DOBJ: u32 = 0x0090_a520;
const YI_SET_MATRIX_GROUP: u32 = 0x0090_a560;
const YI_GET_DOBJ_GROUP: u32 = 0x0090_8c50;
const YI_GET_COLOR_DOBJ: u32 = 0x0090_8e90;
const YI_SET_COLOR_DOBJ: u32 = 0x0090_a5c0;
const YI_GET_VIEW_ADJUST: u32 = 0x0090_8f20;
const YI_SET_VIEW_ADJUST: u32 = 0x0090_a730;
const YI_SET_COLOR_OFFSET: u32 = 0x0090_9ff0;
const YI_PK_ADD_GS_AD: [u32; 3] = [0x0090_ae20, 0x0090_bc40, 0x0091_0040];
const YI_POW2_CEIL: u32 = 0x0091_7460;
const YI_HIGHEST_BIT: u32 = 0x0091_0580;
const YI_LOG2_CEIL16: u32 = 0x0091_7430;
const YI_IS_POW2: u32 = 0x0091_73e0;
const YI_VRAM_ALLOC: u32 = 0x0091_8130;
const YI_VRAM_SET_LIMIT: u32 = 0x0091_9630;
const RCBG_GUIDE_SET_FLAG8: u32 = 0x0091_d530;
const RCBG_GUIDE_GET_SCREEN: u32 = 0x0091_da30;
const YN_SCENE_GET: [(u32, u32); 3] = [
    (0x0091_a6d0, 0xac),
    (0x0091_a700, 0xb8),
    (0x0091_a730, 0xbc),
];
const YN_SCENE_SET_C0: u32 = 0x0091_ad80;
const YN_SCENE_SET_DC: u32 = 0x0091_aa00;
const YN_SCENE_SET_104: u32 = 0x0091_a260;
const RC_ANGLE_DIFF: u32 = 0x0092_e400;
const RCEF_TARGET_IS_DONE: u32 = 0x0092_4ba0;
const RCEF_TARGET_CLEAR: u32 = 0x0092_4bc0;
const RCEF_TARGET_GET_DIST: u32 = 0x0092_4a00;
const RCEF_TARGET_GET_ANGLE: u32 = 0x0092_49e0;
const RCEF_TARGET_GET_HEIGHT: u32 = 0x0092_49c0;
const RCEF_TARGET_SET_DELAY: u32 = 0x0092_4be0;
const RCEF_MAIN_SET_ENV: [u32; 2] = [0x0092_54f0, 0x0092_5510];
const RCEF_OBJ_LINK_LAST: u32 = 0x0092_f920;
const RCBG_ANIME_GET_FRAME: u32 = 0x0093_0860;
const RCBG_ANIME_GET_CURRENT: u32 = 0x0093_0880;
const RC_VEC3F_APPLY_MATRIX: u32 = 0x0092_1300;
const RC_VEC3F_LERP: u32 = 0x0092_1280;
const YI_LERP_VEC4: u32 = 0x0091_48d0;
const YI_LERP_VEC2: u32 = 0x0091_48a0;
const RCUI_VALUE_GET: u32 = 0x0093_c210;
const RCUI_VALUE_SET: u32 = 0x0093_c270;
const RCUI_TAKE: [(u32, u32); 3] = [
    (0x0093_c800, 0x18),
    (0x0093_c820, 0x1c),
    (0x0093_c840, 0x14),
];
const RCUI_ITEM_INIT: u32 = 0x0093_c2c0;
const RCUI_ADD_VECTOR: u32 = 0x0093_cbc0;
const RCUI_MUL_VECTOR: u32 = 0x0093_d290;
const RCUI_ITOF: u32 = 0x0093_d030;
const RCUI_DOT: u32 = 0x0093_d060;
const RCUI_VU0_COPY_VECTOR: u32 = 0x0093_82e0;

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn round3_getters_and_yi_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0003_0001);
    let (c, p, buf, m, a, b) = (
        t.alloc(8),
        t.alloc(0x90),
        t.alloc(8),
        t.alloc(0x40),
        t.alloc(0x10),
        t.alloc(0x10),
    );
    for _ in 0..200 {
        t.fill(c, 8, &mut rng);
        t.fill(p, 0x90, &mut rng);
        t.fill(buf, 8, &mut rng);
        t.same(
            "rcdtChunkGetSize",
            |o| o.eax(RCDT_CHUNK_GET_SIZE, &[c]),
            |mm| rcdt::bridge::rcdt_chunk_get_size(mm, c),
        );
        t.same(
            "rcdtParseGetChunk",
            |o| o.eax(RCDT_PARSE_GET_CHUNK, &[p]),
            |mm| rcdt::bridge::rcdt_parse_get_chunk(mm, p),
        );
        t.same(
            "rcdtParseGetPos",
            |o| o.eax(RCDT_PARSE_GET_POS, &[p]),
            |mm| rcdt::bridge::rcdt_parse_get_pos(mm, p),
        );
        t.same(
            "rcdtParseGetMode",
            |o| o.eax(RCDT_PARSE_GET_MODE, &[p]),
            |mm| rcdt::bridge::rcdt_parse_get_mode(mm, p),
        );
        let at = rng.next();
        t.same(
            "rcPacketDataSetAdr",
            |o| o.void(RC_PACKET_DATA_SET_ADR, &[c, at]),
            |mm| packet::bridge::rc_packet_data_set_adr(mm, c, at),
        );
        t.same(
            "rcPacketDataOpen",
            |o| o.void(RC_PACKET_DATA_OPEN, &[c, buf]),
            |mm| packet::bridge::rc_packet_data_open(mm, c, buf),
        );
        // display objects
        let count = rng.below(12);
        t.set(yi::DOBJ_COUNT, count);
        let base = yi::DOBJS.ptr(&t.b).addr;
        t.fill(base, 12 * yi::dobj::SIZE as usize, &mut rng);
        for i in 0..12 {
            t.put(
                base + i * yi::dobj::SIZE + yi::dobj::GROUP,
                u16::try_from(rng.below(6)).unwrap(),
            );
        }
        t.fill(m, 0x40, &mut rng);
        let i = rng.below(14);
        t.same(
            &format!("yiSetMatrixDObj {i} of {count}"),
            |o| o.eax(YI_SET_MATRIX_DOBJ, &[i, m]),
            |mm| u(yi::bridge::yi_set_matrix_dobj(mm, i, m)),
        );
        let g = [rng.below(7), rng.next()][rng.below(2) as usize];
        t.same(
            &format!("yiSetMatrixGroup {g}"),
            |o| o.eax(YI_SET_MATRIX_GROUP, &[g, m]),
            |mm| u(yi::bridge::yi_set_matrix_group(mm, g, m)),
        );
        t.same(
            "yiGetDObjGroup",
            |o| o.eax(YI_GET_DOBJ_GROUP, &[i]),
            |mm| yi::bridge::yi_get_dobj_group(mm, i),
        );
        t.fill(a, 0x10, &mut rng);
        t.fill(b, 0x10, &mut rng);
        t.same(
            "yiSetColorDObj",
            |o| o.eax(YI_SET_COLOR_DOBJ, &[i, a, b]),
            |mm| u(yi::bridge::yi_set_color_dobj(mm, i, a, b)),
        );
        t.same(
            "yiGetColorDObj",
            |o| o.eax(YI_GET_COLOR_DOBJ, &[i, a, b]),
            |mm| u(yi::bridge::yi_get_color_dobj(mm, i, a, b)),
        );
        t.same(
            "yiSetViewAdjust",
            |o| o.void(YI_SET_VIEW_ADJUST, &[m]),
            |mm| yi::bridge::yi_set_view_adjust(mm, m),
        );
        t.fill(m, 0x40, &mut rng);
        t.same(
            "yiGetViewAdjust",
            |o| o.void(YI_GET_VIEW_ADJUST, &[m]),
            |mm| yi::bridge::yi_get_view_adjust(mm, m),
        );
        let ints: [u32; 4] = std::array::from_fn(|_| rng.next() >> rng.below(32));
        t.put(a, ints);
        t.same(
            "yiSetColorOffset",
            |o| o.void(YI_SET_COLOR_OFFSET, &[a]),
            |mm| yi::bridge::yi_set_color_offset(mm, a),
        );
        let (reg, data) = (rng.next(), rng.next());
        let f = YI_PK_ADD_GS_AD[rng.below(3) as usize];
        t.same(
            "yiPkAddGsAD",
            |o| o.void(f, &[b, reg, data]),
            |mm| yi::bridge::yi_pk_add_gs_ad(mm, b, reg, data),
        );
        let x = [
            rng.next() >> rng.below(32),
            rng.next() & 0xffff,
            1 << rng.below(17),
        ][rng.below(3) as usize];
        t.same(
            &format!("yiPow2Ceil {x:#x}"),
            |o| o.eax(YI_POW2_CEIL, &[x]),
            |_| u(yi::pow2_ceil(x.cast_signed())),
        );
        t.same(
            &format!("yiHighestBit {x:#x}"),
            |o| o.eax(YI_HIGHEST_BIT, &[x]),
            |_| u(yi::highest_bit(x)),
        );
        t.same(
            &format!("yiLog2Ceil16 {x:#x}"),
            |o| o.eax(YI_LOG2_CEIL16, &[x]),
            |_| u(yi::log2_ceil16(x)),
        );
        t.same(
            &format!("yiIsPow2 {x:#x}"),
            |o| o.eax(YI_IS_POW2, &[x]),
            |_| u32::from(yi::is_pow2(x)),
        );
        let pages = rng.below(0x40);
        t.same(
            "yiVramSetLimit",
            |o| o.eax(YI_VRAM_SET_LIMIT, &[pages]),
            |mm| yi::bridge::yi_vram_set_limit(mm, pages),
        );
        t.set(yi::VRAM_NEXT, rng.next() >> rng.below(32));
        let bytes = u(rng.range(-0x100, 0x1_0000));
        t.same(
            &format!("yiVramAlloc {bytes:#x}"),
            |o| o.eax(YI_VRAM_ALLOC, &[bytes]),
            |mm| yi::bridge::yi_vram_alloc(mm, bytes.cast_signed()),
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn round3_guide_and_rcef_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0003_0002);
    let (sys, map, sc, out) = (t.alloc(8), t.alloc(0x10), t.alloc(0x110), t.alloc(0x10));
    let (main, env_slot) = (t.alloc(0x20), t.alloc(4));
    let objs: Vec<u32> = (0..6).map(|_| t.alloc(0x100)).collect();
    let (anime, frames) = (t.alloc(0x20), t.alloc(0x40));
    for _ in 0..200 {
        let on = [0, 1, 0x100][rng.below(3) as usize];
        t.same(
            "rcbgGuideSetFlag8",
            |o| o.void(RCBG_GUIDE_SET_FLAG8, &[0, on]),
            |m| guide::bridge::rcbg_guide_set_flag8(m, 0, i32::try_from(on).unwrap()),
        );
        t.set(guide::SIZE, rng.next() >> rng.below(32));
        t.set(
            guide::POS,
            [rng.next() >> rng.below(32), rng.next() >> rng.below(32)],
        );
        t.same(
            "rcbgGuideGetScreen",
            |o| o.void(RCBG_GUIDE_GET_SCREEN, &[0, out, out + 4]),
            |m| guide::bridge::rcbg_guide_get_screen(m, 0, out, out + 4),
        );
        // Yn's scene parameters: no state, or the whole chain
        let full = rng.below(3) != 0;
        t.set(guide::YN_MAP_SYSTEM, if full { sys } else { 0 });
        t.put(sys + guide::system::MAP, map);
        t.put(map + guide::map::SCENE, sc);
        t.fill(sc + 0xa0, 0x70, &mut rng);
        for (f, offset) in YN_SCENE_GET {
            t.same(
                &format!("Yn_SceneGet {offset:#x}"),
                |o| o.float(f, &[]),
                |m| {
                    bits(match offset {
                        0xac => guide::bridge::yn_scene_get_ac(m),
                        0xb8 => guide::bridge::yn_scene_get_b8(m),
                        _ => guide::bridge::yn_scene_get_bc(m),
                    })
                },
            );
        }
        let args = [
            rng.next(),
            rng.any_float(),
            rng.next(),
            rng.next(),
            rng.next(),
        ];
        t.same(
            "Yn_SceneSetC0",
            |o| o.void(YN_SCENE_SET_C0, &args),
            |m| guide::bridge::yn_scene_set_c0(m, args),
        );
        let (x0, x1, x2, x3) = (rng.next(), rng.any_float(), rng.next(), rng.any_float());
        t.same(
            "Yn_SceneSetDC",
            |o| o.void(YN_SCENE_SET_DC, &[x0, x1, x2, x3]),
            |m| guide::bridge::yn_scene_set_dc(m, x0, x1, x2, x3),
        );
        let v = rng.next();
        t.same(
            "Yn_SceneSet104",
            |o| o.void(YN_SCENE_SET_104, &[v]),
            |m| guide::bridge::yn_scene_set_104(m, v),
        );
        // rcef
        let (x, y) = (rng.float(), rng.float());
        t.same(
            &format!("rcAngleDiff {x:#x} {y:#x}"),
            |o| o.float(RC_ANGLE_DIFF, &[x, y]),
            |m| bits(task::bridge::rc_angle_diff(m, x, y)),
        );
        let table = task::target::TABLE.ptr(&t.b).addr;
        t.fill(table, 0x24 * 16, &mut rng);
        let i = rng.below(16);
        t.same(
            "rcefTargetIsDone",
            |o| o.eax(RCEF_TARGET_IS_DONE, &[i]),
            |m| task::bridge::rcef_target_is_done(m, i),
        );
        for (f, which) in [
            (RCEF_TARGET_GET_DIST, 0),
            (RCEF_TARGET_GET_ANGLE, 1),
            (RCEF_TARGET_GET_HEIGHT, 2),
        ] {
            let at = table + 0x24 * i + [8, 0xc, 0x10][which];
            t.put(at, rng.float());
            t.same(
                "rcefTargetGet*",
                |o| o.float(f, &[i]),
                |m| {
                    bits(match which {
                        0 => task::bridge::rcef_target_get_dist(m, i),
                        1 => task::bridge::rcef_target_get_angle(m, i),
                        _ => task::bridge::rcef_target_get_height(m, i),
                    })
                },
            );
        }
        t.same(
            "rcefTargetClear",
            |o| o.void(RCEF_TARGET_CLEAR, &[i]),
            |m| task::bridge::rcef_target_clear(m, i),
        );
        let d = rng.next();
        t.same(
            "rcefTargetSetDelay",
            |o| o.void(RCEF_TARGET_SET_DELAY, &[d]),
            |m| task::bridge::rcef_target_set_delay(m, u16::try_from(d & 0xffff).unwrap()),
        );
        t.set(task::main::ENV_SLOT, env_slot);
        t.fill(main, 0x20, &mut rng);
        let f = RCEF_MAIN_SET_ENV[rng.below(2) as usize];
        t.same(
            "rcefMainSetEnv",
            |o| o.void(f, &[main]),
            |m| task::bridge::rcef_main_set_env(m, main),
        );
        // a chain of objects, then one more linked at its end
        for (k, &o) in objs.iter().enumerate() {
            t.fill(o, 0x100, &mut rng);
            let next = if k + 1 < objs.len() - 1 && rng.below(4) != 0 {
                objs[k + 1]
            } else {
                0
            };
            t.put(o + 0xcc, next);
        }
        t.same(
            "rcefObjLinkLast",
            |x| x.void(RCEF_OBJ_LINK_LAST, &[objs[0], objs[5]]),
            |m| task::bridge::rcef_obj_link_last(m, objs[0], objs[5]),
        );
        t.fill(anime, 0x20, &mut rng);
        t.put(anime + model::anime::FRAMES, frames);
        t.put(anime + model::anime::COUNT, rng.range(-1, 8));
        t.put(
            anime + model::anime::CURRENT,
            i16::try_from(rng.range(-2, 9)).unwrap(),
        );
        let n = rng.range(-2, 10);
        t.same(
            "rcbgAnimeGetFrame",
            |o| o.eax(RCBG_ANIME_GET_FRAME, &[anime, u(n)]),
            |m| model::bridge::rcbg_anime_get_frame(m, anime, n),
        );
        t.same(
            "rcbgAnimeGetCurrent",
            |o| o.eax(RCBG_ANIME_GET_CURRENT, &[anime]),
            |m| model::bridge::rcbg_anime_get_current(m, anime),
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn round3_vectors_and_rcui_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0003_0003);
    let (va, vb, mtx, out, item) = (
        t.alloc(0x10),
        t.alloc(0x10),
        t.alloc(0x40),
        t.alloc(0x10),
        t.alloc(0x40),
    );
    for _ in 0..300 {
        // quiet NaNs too: ffx_bridge::x87 picks the one the x87 propagates
        let fa: [u32; 4] = std::array::from_fn(|_| rng.any_float());
        let fb: [u32; 4] = std::array::from_fn(|_| rng.any_float());
        let fm: [u32; 16] = std::array::from_fn(|_| rng.any_float());
        t.put(va, fa);
        t.put(vb, fb);
        t.put(mtx, fm);
        let target = [out, va][rng.below(2) as usize];
        t.same(
            "rcVec3fApplyMatrix",
            |o| o.void(RC_VEC3F_APPLY_MATRIX, &[target, va, mtx]),
            |m| geom::bridge::rc_vec3f_apply_matrix(m, target, va, mtx),
        );
        let tt = rng.float();
        let target = [out, va, vb][rng.below(3) as usize];
        t.same(
            "rcVec3fLerp",
            |o| o.void(RC_VEC3F_LERP, &[target, va, vb, tt]),
            |m| geom::bridge::rc_vec3f_lerp(m, target, va, vb, tt),
        );
        t.same(
            "yiLerpVec4",
            |o| o.void(YI_LERP_VEC4, &[tt, va, vb, target]),
            |m| geom::bridge::yi_lerp_vec4(m, tt, va, vb, target),
        );
        t.same(
            "yiLerpVec2",
            |o| o.void(YI_LERP_VEC2, &[tt, va, vb, target]),
            |m| geom::bridge::yi_lerp_vec2(m, tt, va, vb, target),
        );
        t.same(
            "rcuiAddVector",
            |o| o.void(RCUI_ADD_VECTOR, &[target, va, vb]),
            |m| rcui::bridge::rcui_add_vector(m, target, va, vb),
        );
        t.same(
            "rcuiMulVector",
            |o| o.void(RCUI_MUL_VECTOR, &[target, va, vb]),
            |m| rcui::bridge::rcui_mul_vector(m, target, va, vb),
        );
        t.same(
            "rcuiDot",
            |o| o.float(RCUI_DOT, &[va, vb]),
            |m| bits(rcui::bridge::rcui_dot(m, va, vb)),
        );
        let ints: [u32; 4] = std::array::from_fn(|_| rng.next() >> rng.below(32));
        t.put(vb, ints);
        t.same(
            "rcuiItoF",
            |o| o.void(RCUI_ITOF, &[out, vb]),
            |m| rcui::bridge::rcui_itof(m, out, vb),
        );
        t.same(
            "rcuiVu0CopyVector",
            |o| o.void(RCUI_VU0_COPY_VECTOR, &[out, va]),
            |m| rcui::bridge::rcui_vu0_copy_vector(m, out, va),
        );
        t.fill(item, 0x40, &mut rng);
        let kind = rng.below(8);
        t.same(
            &format!("rcuiValueGet {kind}"),
            |o| o.eax(RCUI_VALUE_GET, &[item, kind]),
            |m| rcui::bridge::rcui_value_get(m, item, kind),
        );
        let v = rng.next();
        t.same(
            &format!("rcuiValueSet {kind}"),
            |o| o.void(RCUI_VALUE_SET, &[item, kind, v]),
            |m| rcui::bridge::rcui_value_set(m, item, kind, v),
        );
        for (f, offset) in RCUI_TAKE {
            t.same(
                &format!("rcuiTake {offset:#x}"),
                |o| o.eax(f, &[item]),
                |m| rcui::bridge::rcui_take(m, item, offset),
            );
        }
        t.same(
            "rcuiItemInit",
            |o| o.void(RCUI_ITEM_INIT, &[item]),
            |m| rcui::bridge::rcui_item_init(m, item),
        );
    }
}

const YI_RENDER_ADD_REF: u32 = 0x0090_c1f0;
const YI_RENDER_ADD_REF_DATA: u32 = 0x0090_c230;
const YI_PK_UNPACK_MATRIX: u32 = 0x0090_cd10;
const YI_PK_UNPACK_MATRIX_DMA: u32 = 0x0090_ccb0;

#[test]
fn round3_render_packets_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0003_0004);
    let render = t.alloc(0xc_0000);
    let (pk, block, m, out) = (t.alloc(0x200), t.alloc(0x20), t.alloc(0x40), t.alloc(0x80));
    for _ in 0..100 {
        t.put(render + yi::render::PACKET, pk + 0x10 * rng.below(16));
        t.put(render + yi::render::PACKET_COUNT, rng.next());
        let (addr, qwc) = (rng.next(), rng.next());
        t.same(
            "yiRender_addRef",
            |o| o.void(YI_RENDER_ADD_REF, &[render, addr, qwc]),
            |mm| {
                let [lo, hi, _, _] = qwc.to_le_bytes();
                yi::bridge::yi_render_add_ref(mm, render, addr, u16::from_le_bytes([lo, hi]));
            },
        );
        t.fill(block, 0x20, &mut rng);
        t.same(
            "yiRender_addRefData",
            |o| o.void(YI_RENDER_ADD_REF_DATA, &[render, block]),
            |mm| yi::bridge::yi_render_add_ref_data(mm, render, block),
        );
        t.fill(m, 0x40, &mut rng);
        let a = rng.next() & 0xffff;
        t.same(
            "yiPkUnpackMatrix",
            |o| o.void(YI_PK_UNPACK_MATRIX, &[m, out, a]),
            |mm| yi::bridge::yi_pk_unpack_matrix(mm, m, out, a),
        );
        t.same(
            "yiPkUnpackMatrixDma",
            |o| o.void(YI_PK_UNPACK_MATRIX_DMA, &[m, out, a]),
            |mm| yi::bridge::yi_pk_unpack_matrix_dma(mm, m, out, a),
        );
    }
}

/// Round 3 on the game's own state: display objects, the view matrix,
/// the scene's parameters, the guide screen.
#[test]
fn round3_field_state_matches_the_original_on_a_snapshot() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    let (a, b, m) = (t.alloc(0x10), t.alloc(0x10), t.alloc(0x40));
    let count = yi::DOBJ_COUNT.get(&t.b);
    for i in 0..=count.min(64) {
        t.same(
            &format!("yiGetDObjGroup {i}"),
            |o| o.eax(YI_GET_DOBJ_GROUP, &[i]),
            |mm| yi::bridge::yi_get_dobj_group(mm, i),
        );
        t.same(
            &format!("yiGetColorDObj {i}"),
            |o| o.eax(YI_GET_COLOR_DOBJ, &[i, a, b]),
            |mm| u(yi::bridge::yi_get_color_dobj(mm, i, a, b)),
        );
    }
    t.same(
        "yiGetViewAdjust",
        |o| o.void(YI_GET_VIEW_ADJUST, &[m]),
        |mm| yi::bridge::yi_get_view_adjust(mm, m),
    );
    for (f, offset) in YN_SCENE_GET {
        t.same(
            &format!("Yn_SceneGet {offset:#x}"),
            |o| o.float(f, &[]),
            |mm| {
                bits(match offset {
                    0xac => guide::bridge::yn_scene_get_ac(mm),
                    0xb8 => guide::bridge::yn_scene_get_b8(mm),
                    _ => guide::bridge::yn_scene_get_bc(mm),
                })
            },
        );
    }
    t.same(
        "rcbgGuideGetScreen",
        |o| o.void(RCBG_GUIDE_GET_SCREEN, &[0, a, b]),
        |mm| guide::bridge::rcbg_guide_get_screen(mm, 0, a, b),
    );
    for i in 0..16 {
        t.same(
            "rcefTargetIsDone",
            |o| o.eax(RCEF_TARGET_IS_DONE, &[i]),
            |mm| task::bridge::rcef_target_is_done(mm, i),
        );
    }
}

// ---- round 4 ----

/// Code space for test functions: the body of `pppFpLoop` (`0x00729ab0`),
/// which no round 4 test calls.
const CODE_AREA: u32 = 0x0072_9ab0;
/// `strncmp`'s import slot (MSVCR110), which the oracle does not stub.
const STRNCMP_SLOT: u32 = 0x00b0_c3e4;
/// Free code space after the test functions for a `strncmp`.
const STRNCMP_CODE: u32 = CODE_AREA + 0x40;

/// A sequence function: counts calls in the task's `+0x2c`, asks to run
/// again until the third, returns the count.
const SEQ_STUB: [u8; 0x15] = [
    0x8b, 0x44, 0x24, 0x04, 0xff, 0x40, 0x2c, 0x8b, 0x48, 0x2c, 0x83, 0xf9, 0x03, 0x7d, 0x03, 0x83,
    0x08, 0x02, 0x89, 0xc8, 0xc3,
];
/// An after-function: adds 5 to the task's `+0x30`, returns `0x1234`.
const AFTER_STUB: [u8; 0xe] = [
    0x8b, 0x44, 0x24, 0x04, 0x83, 0x40, 0x30, 0x05, 0xb8, 0x34, 0x12, 0x00, 0x00, 0xc3,
];
/// A two-argument function: adds its second argument to the first's
/// `+4`.
const ADD_STUB: [u8; 0xc] = [
    0x8b, 0x44, 0x24, 0x04, 0x8b, 0x4c, 0x24, 0x08, 0x01, 0x48, 0x04, 0xc3,
];
/// `strncmp(a, b, n)` (the C semantics, unsigned bytes).
const STRNCMP_STUB: [u8; 0x2a] = [
    0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c, 0x8b, 0x7c, 0x24, 0x10, 0x8b, 0x4c, 0x24, 0x14, 0x85, 0xc9,
    0x74, 0x13, 0x0f, 0xb6, 0x06, 0x0f, 0xb6, 0x17, 0x29, 0xd0, 0x75, 0x0b, 0x85, 0xd2, 0x74, 0x05,
    0x46, 0x47, 0x49, 0xeb, 0xe9, 0x31, 0xc0, 0x5f, 0x5e, 0xc3,
];

impl Twin {
    /// The round 4 test functions, in both.
    fn install_round4_code(&mut self) {
        for o in [&mut self.a, &mut self.b] {
            let at = o.rebase(CODE_AREA);
            o.write(at, &SEQ_STUB).unwrap();
            o.write(at + 0x20, &AFTER_STUB).unwrap();
            o.write(at + 0x80, &ADD_STUB).unwrap();
            let code = o.rebase(STRNCMP_CODE);
            o.write(code, &STRNCMP_STUB).unwrap();
            let slot = o.rebase(STRNCMP_SLOT);
            o.write(slot, &code.to_le_bytes()).unwrap();
        }
    }

    /// Random float words (no signalling NaN) at `at`.
    fn floats(&mut self, at: u32, words: u32, rng: &mut Xorshift) {
        for i in 0..words {
            let v = rng.any_float();
            self.put(at + 4 * i, v);
        }
    }
}

impl Orig<'_> {
    fn conv(&mut self, f: u32, conv: Conv, args: &[u32]) -> u32 {
        let at = self.0.rebase(f);
        self.0
            .call(at, conv, args)
            .unwrap_or_else(|e| panic!("{f:#x}: {e}"))
            .eax
    }
}

/// Calls a game function pointer (absolute) on the bridge's oracle.
fn call_ptr(m: &mut Oracle, f: u32, arg: u32) -> u32 {
    m.call(f, Conv::Cdecl, &[arg]).unwrap().eax
}

const YI_GET_COLOR_SCALE: u32 = 0x0090_88d0;
const YI_SET_COLOR_SCALE: u32 = 0x0090_a270;
const YI_SET_COLOR_SHIFT: u32 = 0x0090_8c30;
const YI_SET_BATTLE_CENTER: u32 = 0x0090_a0e0;
const YI_GET_BATTLE_CENTER: u32 = 0x0090_87f0;
const YI_PUSH_RENDER_STATE: u32 = 0x0090_9c60;
const YI_POP_RENDER_STATE: u32 = 0x0090_9b90;
const YI_GET_BACK_COLOR: u32 = 0x0090_86d0;
const YI_IMAGE_INIT: u32 = 0x0091_88c0;
const YI_TEX_SWAP_INIT: u32 = 0x0091_52d0;
const YI_TEX_SWAP_SET_FRAME: u32 = 0x0090_9e30;
const YI_ANIM_ATTACH_DEFAULT: u32 = 0x0091_43c0;
const YI_KEY_FRAME_ATTACH_CHR: u32 = 0x0091_4630;
const YI_SET_FIELD_VEC4I: u32 = 0x0090_a1d0;
const YI_SET_FIELD_VEC4: u32 = 0x0090_a200;
const YI_SET_FIELD_VEC6: u32 = 0x0090_a230;
const YI_SET_FIELD_PAIR: u32 = 0x0090_a430;
const YI_SET_FIELD_VEC3_INT: u32 = 0x0090_a470;
const YI_MAP_GET_SECTION: u32 = 0x0090_7f00;
const YI_VRAM_ALLOC_PSM: u32 = 0x0091_8090;
const RC_MAT4F_SET_SCALE: u32 = 0x0092_57f0;
const RCBG_SET_CLIP_MATRIX: u32 = 0x0092_b0d0;
const RCBG_SET_WORLD_MATRIX: u32 = 0x0092_b140;
const YN_SEA_RESET_MATRIX: u32 = 0x0091_ae30;
const YN_SEA_SET_MATRIX: u32 = 0x0091_ae60;
const YN_PRE_BG_LAYER_DISP_GET: u32 = 0x0091_a770;
const GUIDE_REVEAL_FOG: u32 = 0x0091_f420;
const KE_LNS_LP_INIT: u32 = 0x0076_a670;
const KE_LNS_CRN_SET_SHP: u32 = 0x0076_a1a0;
const KE_LNS_SHP_DRAW: u32 = 0x0076_ab90;
const KE_LNS_SHP_UPDATE: u32 = 0x0076_abe0;
const KE_LNS_CLM_SET_SHP: u32 = 0x0076_9cd0;
const KE_DTT_MDL_USE_SINGLE_INSTANCE: u32 = 0x0072_e6d0;
const ABMAP_ALL_LINKS_LIMIT_FLAG: u32 = 0x00a4_73d0;
const ABMAP_LOST_PANEL: u32 = 0x00a4_58c0;
const ABMAP_GET_NODE_POS_RADIUS: u32 = 0x00a5_b4c0;
const RCBG_PSIM_GET_POS: u32 = 0x0091_fdd0;
const RCBG_SCENE_SET_PAL: u32 = 0x0092_10a0;
const RCBG_CHK_ID: u32 = 0x0092_1940;
const RCBG_FPM097_INIT: u32 = 0x0092_2560;
const RCBG_FPM173_INIT: u32 = 0x0092_2b70;
const RCBG_FPM565_INIT: u32 = 0x0092_2f50;
const RCFX_DAMP_INIT: u32 = 0x0092_93d0;
const RCBG_FP_GET_WORK: u32 = 0x0092_3100;
const RCBG_SCENE_DRAW_LAYER: u32 = 0x0092_0830;
const RCBG_SCENE_WAVE_DC_STOP: u32 = 0x0092_0650;
const RCBG_SCENE_WAVE_C0_STOP: u32 = 0x0092_1200;
const RCFX_PARTICLE_CLEAR_FORCES: u32 = 0x0092_89b0;
const RCDT_TM2_GET_PSM: u32 = 0x0092_3280;
const RCDT_TM2_CLUT_ALPHA_TO_PS2: u32 = 0x0092_3240;
const RCDT_RSD2_INIT: u32 = 0x0092_4390;
const RCBG_TRX_SET_REGS: u32 = 0x0092_d360;
const RCBG_CLIP_CODE_AND: u32 = 0x0092_aed0;
const RCEF_TASK_PROC: u32 = 0x0092_ed90;
const RCEF_TASK_DESTRUCT: u32 = 0x0092_ec90;
const RC_EFFECT_TASK_PRAY_WAIT: u32 = 0x0093_04b0;
const RCEF_OBJ_INIT: u32 = 0x0092_ff20;
const RCEF_PAR_CREATE: u32 = 0x0092_eee0;
const MAP_PART_INIT_LANG_OVERRIDES: u32 = 0x0067_c620;
const MAP_PART_LANG_OVERRIDE: u32 = 0x0067_fb30;
const TEXT_UTIL_LIST_TEXT_UTIL_LIST: u32 = 0x0090_e6d0;
const RC_MAT4F_SET_SCALE_VEC3F: u32 = 0x0092_5820;
const RCBG_MODEL_SET_SCALE: u32 = 0x0092_8540;
const YI_ATTACH_CHR_DOBJ: u32 = 0x0090_8350;
const YI_CALL_FIELD_PARTICLE: u32 = 0x0090_83e0;
const RCFX_PARTICLE_RESET_FORCE: u32 = 0x0092_8970;
const RCBG_SCENE_DRAW: u32 = 0x0092_0740;
const YN_PSIM_GET_HANDLE: u32 = 0x0091_99f0;
const RCEF_MAIN_SET_PAR: u32 = 0x0092_56d0;
const PPP_FP_LOOP: u32 = 0x0072_9ab0;
const YN_PSIM_GET_HANDLE_JUMP: u32 = 0x0091_9f00;
const RCBG_PSIM_SET_READY: u32 = 0x0091_ffb0;

const MAGIC_ID: u32 = 0x00c6_4ca0;
const FP_DATA: u32 = 0x00d2_c258;
const FP_RUN: u32 = 0x00d2_c260;

#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round4_yi_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0004_0001);
    let (v, out) = (t.alloc(0x10), t.alloc(0x10));
    let scale = yi::state::COLOR_SCALE.ptr(&t.b).addr;
    let shift = yi::state::COLOR_SHIFT.ptr(&t.b).addr;
    let center = yi::state::BATTLE_CENTER.ptr(&t.b).addr;
    let factor = yi::SCALE_FACTOR.ptr(&t.b).addr;
    let offset = yi::COLOR_OFFSET.ptr(&t.b).addr;
    for _ in 0..200 {
        t.floats(v, 4, &mut rng);
        t.floats(scale, 4, &mut rng);
        t.floats(shift, 4, &mut rng);
        t.same(
            "yiGetColorScale",
            |o| o.eax(YI_GET_COLOR_SCALE, &[out]),
            |m| yi::bridge::yi_get_color_scale(m, out),
        );
        t.same(
            "yiSetColorShift",
            |o| o.eax(YI_SET_COLOR_SHIFT, &[out]),
            |m| yi::bridge::yi_set_color_shift(m, out),
        );
        t.same(
            "yiSetColorScale",
            |o| o.eax(YI_SET_COLOR_SCALE, &[v]),
            |m| yi::bridge::yi_set_color_scale(m, v),
        );
        for i in 0..4 {
            let f = rng.float();
            t.put(factor + 4 * i, f);
            let x = rng.float();
            t.put(v + 4 * i, x);
        }
        t.same(
            "yiSetBattleCenter",
            |o| o.eax(YI_SET_BATTLE_CENTER, &[v]),
            |m| yi::bridge::yi_set_battle_center(m, v),
        );
        t.floats(center, 4, &mut rng);
        t.same(
            "yiGetBattleCenter",
            |o| o.eax(YI_GET_BATTLE_CENTER, &[out]),
            |m| yi::bridge::yi_get_battle_center(m, out),
        );
        // the background colour: scale and offset in a useful range too
        let status = match rng.below(4) {
            0 => rng.next(),
            _ => rng.next() | 5,
        };
        t.set(yi::RENDER_STATUS, status);
        let c = rng.next();
        t.set(yi::state::BACK_COLOR, c.to_le_bytes());
        for i in 0..4 {
            let (s, o) = if rng.below(2) == 0 {
                (rng.float(), rng.float())
            } else {
                #[allow(clippy::cast_precision_loss)]
                (
                    (rng.range(0, 300) as f32 / 128.0).to_bits(),
                    (rng.range(-200, 200) as f32).to_bits(),
                )
            };
            t.put(scale + 4 * i, s);
            t.put(offset + 4 * i, o);
        }
        t.same(
            "yiGetBackColor",
            |o| o.eax(YI_GET_BACK_COLOR, &[out]),
            |m| yi::bridge::yi_get_back_color(m, out),
        );
        // the field vectors set by script natives
        let w: [u32; 6] = std::array::from_fn(|_| rng.any_float());
        t.same(
            "yiSetFieldVec4",
            |o| o.void(YI_SET_FIELD_VEC4, &w[..4]),
            |m| yi::bridge::yi_set_field_vec4(m, [w[0], w[1], w[2], w[3]]),
        );
        let ints: [u32; 4] = std::array::from_fn(|_| rng.next() >> rng.below(32));
        t.same(
            "yiSetFieldVec4i",
            |o| o.void(YI_SET_FIELD_VEC4I, &ints),
            |m| yi::bridge::yi_set_field_vec4i(m, ints.map(u32::cast_signed)),
        );
        t.same(
            "yiSetFieldVec6",
            |o| o.void(YI_SET_FIELD_VEC6, &w),
            |m| yi::bridge::yi_set_field_vec6(m, w),
        );
        let on = rng.below(3);
        t.same(
            "yiSetFieldPair",
            |o| o.eax(YI_SET_FIELD_PAIR, &[on, w[0], w[1]]),
            |m| yi::bridge::yi_set_field_pair(m, on, w[0], w[1]),
        );
        let n = rng.next();
        t.same(
            "yiSetFieldVec3Int",
            |o| o.eax(YI_SET_FIELD_VEC3_INT, &[w[2], w[3], w[4], n]),
            |m| yi::bridge::yi_set_field_vec3_int(m, w[2], w[3], w[4], n),
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round4_yi_state_and_textures_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0004_0002);
    // the render state: live and saved areas full of floats
    let dobjs = yi::DOBJS.ptr(&t.b).addr;
    let saved = yi::state::SAVED.ptr(&t.b).addr;
    let live = [
        yi::state::RENDER_WORD.ptr(&t.b).addr,
        yi::state::COLOR_SCALE.ptr(&t.b).addr,
        yi::COLOR_OFFSET.ptr(&t.b).addr,
    ];
    for _ in 0..3 {
        t.floats(dobjs, 128 * 0xb4 / 4, &mut rng);
        t.floats(saved, 0x44 / 4 + 128 * 25, &mut rng);
        t.floats(live[0], 0x20 / 4, &mut rng);
        t.floats(live[1], 8, &mut rng);
        t.set(yi::RENDER_STATUS, rng.next());
        t.same(
            "yiPushRenderState",
            |o| o.eax(YI_PUSH_RENDER_STATE, &[]),
            yi::bridge::yi_push_render_state,
        );
        t.floats(dobjs, 128 * 0xb4 / 4, &mut rng);
        t.floats(live[0], 0x20 / 4, &mut rng);
        t.same(
            "yiPopRenderState",
            |o| o.eax(YI_POP_RENDER_STATE, &[]),
            yi::bridge::yi_pop_render_state,
        );
    }
    // images and the renderer's texture swaps and animations
    let (im, r) = (t.alloc(0x180), t.alloc(0xc_0000));
    t.fill(im, 0x180, &mut rng);
    t.same(
        "yiImage_init",
        |o| o.eax(YI_IMAGE_INIT, &[im]),
        |m| yi::bridge::yi_image_init(m, im),
    );
    t.fill(r + 0x9_d000, 0x2_0000, &mut rng);
    t.same(
        "yiTexSwap_init",
        |o| o.eax(YI_TEX_SWAP_INIT, &[r]),
        |m| yi::bridge::yi_tex_swap_init(m, r),
    );
    let swaps = yi::state::RENDER.ptr(&t.b).addr;
    for _ in 0..200 {
        t.put(swaps + yi::render::TEX_SWAP_COUNT, rng.below(0x48));
        let (i, f) = (rng.below(0x48), rng.below(16));
        let e = swaps + yi::render::TEX_SWAP + i.min(0x3f) * yi::render::TEX_SWAP_SIZE;
        t.put(e + yi::render::tex_swap::FRAMES, rng.below(16));
        t.put(e, rng.next());
        t.same(
            "yiTexSwapSetFrame",
            |o| o.eax(YI_TEX_SWAP_SET_FRAME, &[i, f]),
            |m| yi::bridge::yi_tex_swap_set_frame(m, i, f),
        );
    }
    let kf = t.alloc(0x10);
    for _ in 0..300 {
        t.set(yi::state::USE_VCOUNT, rng.below(2));
        t.set(yi::SG_VCOUNT, rng.next());
        t.set(yi::SG_FRAME, rng.next());
        t.put(r + yi::render::ANIM_IDS, rng.below(0x20));
        t.put(r + yi::render::TEX_SWAP_COUNT, rng.below(0x20));
        t.put(r + yi::render::ANIM_COUNT, rng.next());
        for k in 0..yi::render::ANIMS {
            let s = r + yi::render::ANIM + k * yi::render::ANIM_SIZE;
            let busy = rng.below(8) != 0 || rng.below(2) == 0;
            t.put(s, u32::from(busy) | (rng.next() & 0xfe));
            t.put(s + 6, u16::try_from(rng.below(0x20)).unwrap());
        }
        for k in 0..4 {
            let p = if rng.below(3) == 0 { 0 } else { kf };
            t.put(r + yi::render::KEYFRAMES + 4 * k, p);
        }
        t.fill(kf, 0x10, &mut rng);
        let (id, a, b) = (rng.below(0x24), rng.next(), rng.next());
        t.same(
            "yiAnimAttachDefault",
            |o| o.eax(YI_ANIM_ATTACH_DEFAULT, &[r, id, a, b]),
            |m| yi::bridge::yi_anim_attach_default(m, r, id, a, b),
        );
        let k = rng.below(4);
        let arg = rng.next();
        t.same(
            "yiKeyFrame_attachChr",
            |o| o.eax(YI_KEY_FRAME_ATTACH_CHR, &[r, id, k, arg]),
            |m| {
                let [lo, hi, _, _] = arg.to_le_bytes();
                yi::bridge::yi_key_frame_attach_chr(m, r, id, k, u16::from_le_bytes([lo, hi]))
            },
        );
    }
    // map sections
    let map = t.alloc(0x40);
    for _ in 0..50 {
        t.fill(map, 0x40, &mut rng);
        if rng.below(3) != 0 {
            t.write(map, b"MAP1");
        }
        if rng.below(4) == 0 {
            t.put(map + 0x10, 0u32);
        }
        let i = rng.below(8);
        t.same(
            "yiMapGetSection",
            |o| o.eax(YI_MAP_GET_SECTION, &[map, i]),
            |m| yi::bridge::yi_map_get_section(m, map, i),
        );
    }
    // VRAM regions
    let regions = yi::state::VRAM_REGIONS.ptr(&t.b).addr;
    for _ in 0..300 {
        for i in 0..4u32 {
            let limit = rng.below(0x40_0000);
            t.put(regions + 4 * i, limit);
            t.put(
                regions + 0x10 + 4 * i,
                limit.saturating_sub(rng.below(0x8000)),
            );
        }
        let bytes = u(rng.range(-0x100, 0x2000));
        let psm = [0x13, 0x14, 0, 2][rng.below(4) as usize];
        t.same(
            "yiVramAllocPsm",
            |o| o.eax(YI_VRAM_ALLOC_PSM, &[bytes, psm]),
            |m| yi::bridge::yi_vram_alloc_psm(m, bytes.cast_signed(), psm),
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round4_matrices_guide_and_ke_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0004_0003);
    let m = t.alloc(0x40);
    for _ in 0..200 {
        t.fill(m, 0x40, &mut rng);
        let (x, y, z) = (rng.any_float(), rng.any_float(), rng.any_float());
        t.same(
            "rcMat4fSetScale",
            |o| o.eax(RC_MAT4F_SET_SCALE, &[m, x, y, z]),
            |mm| geom::bridge::rc_mat4f_set_scale(mm, m, x, y, z),
        );
        let a: [u32; 5] = std::array::from_fn(|_| rng.float());
        t.same(
            "rcbgSetClipMatrix",
            |o| o.eax(RCBG_SET_CLIP_MATRIX, &a),
            |mm| {
                let f = a.map(f32::from_bits);
                geom::bridge::rcbg_set_clip_matrix(mm, f[0], f[1], f[2], f[3], f[4])
            },
        );
        t.same(
            "rcbgSetWorldMatrix",
            |o| o.eax(RCBG_SET_WORLD_MATRIX, &[m]),
            |mm| geom::bridge::rcbg_set_world_matrix(mm, m),
        );
        t.same(
            "Yn_SeaSetMatrix",
            |o| o.eax(YN_SEA_SET_MATRIX, &[m]),
            |mm| guide::bridge::yn_sea_set_matrix(mm, m),
        );
        t.same(
            "Yn_SeaResetMatrix",
            |o| o.eax(YN_SEA_RESET_MATRIX, &[]),
            guide::bridge::yn_sea_reset_matrix,
        );
    }
    // the fog of war
    let (g, model, fog) = (t.alloc(0x100), t.alloc(0x100), t.alloc(0x200));
    t.put(g + guide::object::FOG, fog);
    for _ in 0..300 {
        let (own, walk) = if rng.below(2) == 0 {
            (model, 0)
        } else {
            (0, model)
        };
        t.put(g + guide::object::MODEL, own);
        t.put(g + guide::object::WALK_MODEL, walk);
        #[allow(clippy::cast_precision_loss)]
        let coord = |rng: &mut Xorshift| (rng.range(-300_000, 300_000) as f32 / 64.0).to_bits();
        let (ox, oz) = (coord(&mut rng), coord(&mut rng));
        t.put(model + 0x90, ox);
        t.put(model + 0xa8, oz);
        let (x, z) = if rng.below(8) == 0 {
            (rng.float(), rng.float())
        } else {
            (coord(&mut rng), coord(&mut rng))
        };
        t.same(
            "Guide_RevealFog",
            |o| o.eax(GUIDE_REVEAL_FOG, &[g, x, z]),
            |mm| guide::bridge::guide_reveal_fog(mm, g, f32::from_bits(x), f32::from_bits(z)),
        );
    }
    // lens flare shapes
    let (o, shp, frames, st) = (t.alloc(0x100), t.alloc(0x10), t.alloc(0x100), t.alloc(0x10));
    t.put(shp, frames);
    for _ in 0..300 {
        t.fill(o, 0x100, &mut rng);
        t.fill(frames, 0x100, &mut rng);
        t.put(o, shp);
        t.put(o + 4, st);
        t.put(frames + 6, u16::try_from(rng.below(12)).unwrap());
        for k in 0..12 {
            t.put(
                frames + 8 * k + 0x12,
                u16::try_from(rng.below(0x900)).unwrap(),
            );
        }
        t.fill(st, 4, &mut rng);
        t.put(st + 2, u8::try_from(rng.below(12)).unwrap());
        t.same(
            "KeLnsShp_Update",
            |oo| oo.eax(KE_LNS_SHP_UPDATE, &[o]),
            |mm| ke::bridge::ke_lns_shp_update(mm, o),
        );
        t.same(
            "KeLnsShp_Draw",
            |oo| oo.eax(KE_LNS_SHP_DRAW, &[o]),
            |mm| ke::bridge::ke_lns_shp_draw(mm, o),
        );
        t.same(
            "KeLnsCrn_SetShp",
            |oo| oo.eax(KE_LNS_CRN_SET_SHP, &[o, shp]),
            |mm| ke::bridge::ke_lns_crn_set_shp(mm, o, shp),
        );
        t.put(o + 8, u8::try_from(rng.below(24)).unwrap());
        let seed = rng.next();
        t.same(
            "KeLnsClm_SetShp",
            |oo| oo.void(KE_LNS_CLM_SET_SHP, &[o, shp, seed]),
            |mm| ke::bridge::ke_lns_clm_set_shp(mm, o, shp, seed),
        );
        t.same(
            "KeLnsLp_Init",
            |oo| oo.eax(KE_LNS_LP_INIT, &[o]),
            |mm| ke::bridge::ke_lns_lp_init(mm, o),
        );
    }
    for id in (0..0x300).chain([u32::MAX]) {
        t.put(MAGIC_ID, id);
        t.same(
            "KeDttMdl_UseSingleInstance",
            |oo| oo.eax(KE_DTT_MDL_USE_SINGLE_INSTANCE, &[]),
            |mm| ke::bridge::ke_dtt_mdl_use_single_instance(mm),
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round4_abmap_and_scene_objects_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    t.install_round4_code();
    let mut rng = Xorshift(0x0004_0004);
    // the sphere grid
    let grid = t.alloc(0x1_0000);
    t.set(ffx_field::abmap::ABMAP, grid);
    let out = t.alloc(0x10);
    for _ in 0..100 {
        t.fill(grid + 0x800, 0x400, &mut rng);
        t.fill(grid + 0xa800, 0x400, &mut rng);
        t.fill(grid + 0xf800, 0x400, &mut rng);
        t.put(grid + 4, u16::try_from(rng.below(40)).unwrap());
        let mask = rng.next() & 0xff;
        t.same(
            "AbmapAllLinksLimitFlag",
            |o| o.eax(ABMAP_ALL_LINKS_LIMIT_FLAG, &[mask]),
            |m| {
                ffx_field::abmap::bridge::abmap_all_links_limit_flag(m, u8::try_from(mask).unwrap())
            },
        );
        let (bit, node) = (rng.below(40), rng.below(20));
        t.same(
            "abmap_lost_panel",
            |o| o.eax(ABMAP_LOST_PANEL, &[bit, node]),
            |m| ffx_field::abmap::bridge::abmap_lost_panel(m, bit, node),
        );
        for k in 0..20 {
            let kind = u16::try_from(rng.below(16)).unwrap();
            t.put(grid + 0x808 + 0x28 * k + 6, kind);
        }
        let node = if rng.below(8) == 0 {
            0xffff
        } else {
            rng.below(20)
        };
        t.same(
            "AbmapGetNodePosRadius",
            |o| o.float(ABMAP_GET_NODE_POS_RADIUS, &[node | 0xabcd_0000, out]),
            |m| {
                let n = u16::try_from(node).unwrap();
                bits(ffx_field::abmap::bridge::abmap_get_node_pos_radius(
                    m, n, out,
                ))
            },
        );
    }
    // particle simulations, scenes, FP parts
    let (p, s, layers) = (t.alloc(0xb0), t.alloc(0x100), t.alloc(0xc0 * 8));
    for _ in 0..100 {
        t.floats(p, 0xb0 / 4, &mut rng);
        let which = rng.below(3);
        t.same(
            "rcbgPSimGetPos",
            |o| o.eax(RCBG_PSIM_GET_POS, &[p, which, out]),
            |m| model::bridge::rcbg_psim_get_pos(m, p, which, out),
        );
        t.fill(layers, 0xc0 * 8, &mut rng);
        t.put(s + 8, layers);
        t.put(s + 4, u(rng.range(-2, 8)));
        t.same(
            "rcbgSceneSetPal",
            |o| o.void(RCBG_SCENE_SET_PAL, &[s]),
            |m| model::bridge::rcbg_scene_set_pal(m, s),
        );
        t.fill(s, 0x100, &mut rng);
        t.same(
            "rcbgSceneWaveDCStop",
            |o| o.void(RCBG_SCENE_WAVE_DC_STOP, &[s]),
            |m| model::bridge::rcbg_scene_wave_dc_stop(m, s),
        );
        t.same(
            "rcbgSceneWaveC0Stop",
            |o| o.void(RCBG_SCENE_WAVE_C0_STOP, &[s]),
            |m| model::bridge::rcbg_scene_wave_c0_stop(m, s),
        );
        for (name, f, rust) in [
            (
                "rcbgFPm097Init",
                RCBG_FPM097_INIT,
                model::bridge::rcbg_fpm097_init as fn(&mut Oracle, u32) -> u32,
            ),
            (
                "rcbgFPm173Init",
                RCBG_FPM173_INIT,
                model::bridge::rcbg_fpm173_init,
            ),
            (
                "rcbgFPm565Init",
                RCBG_FPM565_INIT,
                model::bridge::rcbg_fpm565_init,
            ),
            (
                "rcfxDampInit",
                RCFX_DAMP_INIT,
                model::bridge::rcfx_damp_init,
            ),
        ] {
            t.fill(s, 0x100, &mut rng);
            t.same(name, |o| o.eax(f, &[s]), |m| rust(m, s));
        }
        t.fill(s, 0x100, &mut rng);
        t.same(
            "rcdtRsd2Init",
            |o| o.void(RCDT_RSD2_INIT, &[s]),
            |m| model::bridge::rcdt_rsd2_init(m, s),
        );
        t.put(s + 0x1c, u(rng.range(-1, 8)));
        t.put(s + 0x2c, layers);
        t.same(
            "rcfxParticleClearForces",
            |o| o.void(RCFX_PARTICLE_CLEAR_FORCES, &[s]),
            |m| model::bridge::rcfx_particle_clear_forces(m, s),
        );
        t.put(s + 0x13, u8::try_from(rng.below(6)).unwrap());
        t.same(
            "rcdtTm2GetPsm",
            |o| o.eax(RCDT_TM2_GET_PSM, &[s]),
            |m| model::bridge::rcdt_tm2_get_psm(m, s),
        );
        let h: [u32; 7] = std::array::from_fn(|_| rng.next());
        t.same(
            "rcbgTrxSetRegs",
            |o| {
                o.eax(
                    RCBG_TRX_SET_REGS,
                    &[s, h[0], h[1], h[2], h[3], h[4], h[5], h[6]],
                )
            },
            |m| {
                let w = h.map(|v| u16::try_from(v & 0xffff).unwrap().cast_signed());
                model::bridge::rcbg_trx_set_regs(m, s, w[0], w[1], w[2], w[3], w[4], w[5], w[6])
            },
        );
        t.same(
            "rcbgClipCodeAnd",
            |o| o.eax(RCBG_CLIP_CODE_AND, &[s, h[0], h[1], h[2], h[3]]),
            |m| model::bridge::rcbg_clip_code_and(m, s, h[0], h[1], h[2], h[3]),
        );
        t.fill(s, 0x40, &mut rng);
        t.same(
            "TextUtilList::TextUtilList",
            |o| o.eax(TEXT_UTIL_LIST_TEXT_UTIL_LIST, &[s]),
            |m| rcui::bridge::text_util_list_text_util_list(m, s),
        );
    }
    let clut = t.alloc(0x400);
    for _ in 0..10 {
        t.fill(clut, 0x400, &mut rng);
        for k in 0..64 {
            t.put(clut + 16 * k + 3, 0u8);
        }
        t.same(
            "rcdtTm2ClutAlphaToPs2",
            |o| o.void(RCDT_TM2_CLUT_ALPHA_TO_PS2, &[clut]),
            |m| model::bridge::rcdt_tm2_clut_alpha_to_ps2(m, clut),
        );
    }
    // chunk ids: the table's own tags, near misses, shorter strings
    let id = t.alloc(8);
    let table = t.b.rebase(model::CHUNK_IDS);
    let tags: Vec<u32> = (0..64)
        .map(|i| Mem::read::<u32>(&t.b, table + 4 * i))
        .take_while(|&p| p != 0)
        .collect();
    assert_ne!(tags.len(), 0, "the chunk tag table");
    for _ in 0..200 {
        let tag = tags[rng.below(u32::try_from(tags.len()).unwrap()) as usize];
        let mut b: [u8; 8] = Mem::read(&t.b, tag);
        match rng.below(4) {
            0 => b[rng.below(4) as usize] ^= 1 << rng.below(8),
            1 => b[rng.below(4) as usize] = 0,
            _ => {}
        }
        t.write(id, &b);
        t.same(
            "rcbgChkId",
            |o| o.eax(RCBG_CHK_ID, &[id]),
            |m| u(model::bridge::rcbg_chk_id(m, id)),
        );
    }
    // FP work records: a small table of ids, runs and part tables
    let (data, parts, table2, works) =
        (t.alloc(0x200), t.alloc(0x10), t.alloc(0x100), t.alloc(0x40));
    for _ in 0..100 {
        let n = rng.below(4);
        t.put(data + 4, u16::try_from(n).unwrap());
        t.set(
            Global::<u32>::new(FP_DATA),
            if rng.below(8) == 0 { 0 } else { data },
        );
        for i in 0..n {
            t.put(data + 0x64 + 0x50 * i, u16::try_from(rng.below(6)).unwrap());
            let run = t.b.rebase(FP_RUN) + 0x80 * i;
            let d = if rng.below(6) == 0 { 0 } else { data };
            t.put(run + 0x30, d);
        }
        t.put(data + 8, if rng.below(6) == 0 { 0 } else { parts });
        t.put(parts, if rng.below(6) == 0 { 0 } else { table2 });
        t.fill(table2, 0x100, &mut rng);
        for k in 0..8 {
            t.put(table2 + 0x28 + 0x10 * k + 0xc, works);
        }
        t.fill(works, 0x40, &mut rng);
        let (id, part, index) = (rng.below(6), rng.below(8), rng.below(16));
        let scratch = t.alloc(4);
        t.same(
            "rcbgFpGetWork",
            |o| o.eax(RCBG_FP_GET_WORK, &[id, part, index]),
            |m| {
                model::bridge::rcbg_fp_get_work(m, id, part, index, |mm, id| {
                    ffx_vfx::fp::bridge::ppp_fp_get_mng_ptr(mm, id, scratch, 0);
                    let run = Mem::read::<u32>(mm, scratch);
                    Mem::write(mm, scratch, 0u32);
                    run
                })
            },
        );
    }
    // draw layer: queued in the 3D "after" list
    for _ in 0..40 {
        let layer = rng.next();
        t.same(
            "rcbgSceneDrawLayer",
            |o| o.eax(RCBG_SCENE_DRAW_LAYER, &[layer]),
            |m| {
                model::bridge::rcbg_scene_draw_layer(m, layer, |mm, f, l| {
                    ffx_vfx::op::bridge::op_func_after_ot3d_set(mm, f, l, 0, 1).cast_unsigned()
                })
            },
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round4_rcef_and_map_parts_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    t.install_round4_code();
    let mut rng = Xorshift(0x0004_0005);
    let seq = t.b.rebase(CODE_AREA);
    let after = seq + 0x20;
    let task = t.alloc(0xbc);
    for _ in 0..200 {
        t.fill(task, 0xbc, &mut rng);
        let level = rng.below(8);
        t.put(task + task::header::LEVEL, level);
        t.put(task + task::header::ARGS, rng.below(4));
        t.put(
            task + task::header::SEQ + 4 * level,
            if rng.below(4) == 0 { 0 } else { seq },
        );
        t.put(
            task + task::header::AFTER,
            if rng.below(4) == 0 { 0 } else { after },
        );
        t.put(
            task + task::header::DESTRUCTOR,
            if rng.below(3) == 0 { 0 } else { after },
        );
        t.same(
            "rcefTaskProc",
            |o| o.eax(RCEF_TASK_PROC, &[task]),
            |m| task::bridge::rcef_task_proc(m, task, call_ptr),
        );
        t.same(
            "rcefTaskDestruct",
            |o| o.eax(RCEF_TASK_DESTRUCT, &[task]),
            |m| task::bridge::rcef_task_destruct(m, task, call_ptr),
        );
        t.set(task::bridge::PRAY_EFFECT, rng.below(2));
        t.set(
            Global::<u8>::new(0x011a_0061),
            u8::try_from(rng.below(2)).unwrap(),
        );
        t.same(
            "rcEffectTaskPrayWait",
            |o| o.void(RC_EFFECT_TASK_PRAY_WAIT, &[task]),
            |m| {
                task::bridge::rc_effect_task_pray_wait(m, task, |mm, _| {
                    ffx_vfx::op::bridge::op_pray_effect_check(mm)
                });
            },
        );
    }
    let o = t.alloc(0x490);
    t.fill(o, 0x490, &mut rng);
    t.same(
        "rcefObjInit",
        |oo| oo.eax(RCEF_OBJ_INIT, &[o]),
        |m| task::bridge::rcef_obj_init(m, o),
    );
    let ids = [0x20e, 0x180, 0x168, 0x217, 0xbf, 0x216, 0, 0x100];
    for _ in 0..100 {
        t.fill(o, 0xa0, &mut rng);
        t.put(MAGIC_ID, ids[rng.below(8) as usize]);
        let (data, index) = (rng.next(), rng.below(4));
        let (a, b, c) = (rng.next(), rng.next(), rng.next());
        t.same(
            "rcefParCreate",
            |oo| oo.eax(RCEF_PAR_CREATE, &[o, data, index, a, b, c]),
            |m| task::bridge::rcef_par_create(m, o, data, index, a, b, c),
        );
    }
    // the map part language overrides (languages that print a warning
    // are left out: Phyre's printf runs imports the oracle lacks)
    let this = t.alloc(0x2c60);
    t.fill(this + 0x2c00, 0x60, &mut rng);
    t.same(
        "MapPart::InitLangOverrides",
        |oo| oo.conv(MAP_PART_INIT_LANG_OVERRIDES, Conv::Fastcall, &[this, 0]),
        |m| scene::bridge::map_part_init_lang_overrides(m, this),
    );
    for _ in 0..100 {
        let lang = [0, 1, 2, 3, 4, 5, 9, 10][rng.below(8) as usize];
        let map = [0xcbc, 0x199f, 0, 0x100][rng.below(4) as usize];
        t.same(
            "MapPart::LangOverride",
            |oo| oo.conv(MAP_PART_LANG_OVERRIDE, Conv::Thiscall(this), &[lang, map]),
            |m| scene::bridge::map_part_lang_override(m, this, lang, map, |_| panic!("no warning")),
        );
    }
}

/// Round 4 on the game's own state.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round4_field_state_matches_the_original_on_a_snapshot() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    t.install_round4_code();
    let out = t.alloc(0x10);
    t.same(
        "yiGetBackColor",
        |o| o.eax(YI_GET_BACK_COLOR, &[out]),
        |m| yi::bridge::yi_get_back_color(m, out),
    );
    t.same(
        "yiGetColorScale",
        |o| o.eax(YI_GET_COLOR_SCALE, &[out]),
        |m| yi::bridge::yi_get_color_scale(m, out),
    );
    t.same(
        "yiGetBattleCenter",
        |o| o.eax(YI_GET_BATTLE_CENTER, &[out]),
        |m| yi::bridge::yi_get_battle_center(m, out),
    );
    t.same(
        "yiPushRenderState",
        |o| o.eax(YI_PUSH_RENDER_STATE, &[]),
        yi::bridge::yi_push_render_state,
    );
    t.same(
        "yiPopRenderState",
        |o| o.eax(YI_POP_RENDER_STATE, &[]),
        yi::bridge::yi_pop_render_state,
    );
    for part in 0..40 {
        t.same(
            &format!("Yn_PreBgLayerDispGet {part}"),
            |o| o.eax(YN_PRE_BG_LAYER_DISP_GET, &[u(part)]),
            |m| guide::bridge::yn_pre_bg_layer_disp_get(m, part),
        );
    }
    let sys = guide::YN_MAP_SYSTEM.get(&t.b);
    let g = if sys == 0 {
        0
    } else {
        Mem::read::<u32>(&t.b, sys + 4)
    };
    let ready = g != 0
        && guide::bridge::guide_mesh_object(&t.b, g) != 0
        && Mem::read::<u32>(&t.b, g + guide::object::FOG) != 0;
    eprintln!("guide {g:#x}, fog map ready: {ready}");
    if ready {
        let mut rng = Xorshift(0x0004_0006);
        for _ in 0..50 {
            #[allow(clippy::cast_precision_loss)]
            let (x, z) = (
                (rng.range(-2000, 2000) as f32).to_bits(),
                (rng.range(-2000, 2000) as f32).to_bits(),
            );
            t.same(
                "Guide_RevealFog",
                |o| o.eax(GUIDE_REVEAL_FOG, &[g, x, z]),
                |m| guide::bridge::guide_reveal_fog(m, g, f32::from_bits(x), f32::from_bits(z)),
            );
        }
    }
    for id in 0..8 {
        let scratch = t.alloc(4);
        for (part, index) in [(0, 0), (1, 0), (0, 1)] {
            t.same(
                "rcbgFpGetWork",
                |o| o.eax(RCBG_FP_GET_WORK, &[id, part, index]),
                |m| {
                    model::bridge::rcbg_fp_get_work(m, id, part, index, |mm, id| {
                        ffx_vfx::fp::bridge::ppp_fp_get_mng_ptr(mm, id, scratch, 0);
                        let run = Mem::read::<u32>(mm, scratch);
                        Mem::write(mm, scratch, 0u32);
                        run
                    })
                },
            );
        }
    }
    t.same(
        "KeDttMdl_UseSingleInstance",
        |o| o.eax(KE_DTT_MDL_USE_SINGLE_INSTANCE, &[]),
        |m| ke::bridge::ke_dtt_mdl_use_single_instance(m),
    );
}

/// The callers of round 4's functions whose callees are now all ported.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round4_callers_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0004_0007);
    // the field particles: the gate, and pppFpLoop itself (no FP data in
    // the image) when it opens
    for _ in 0..8 {
        t.set(yi::state::MAP_READ, rng.below(2));
        t.set(yi::RENDER_STATUS, rng.next());
        t.same(
            "yiCallFieldParticle",
            |o| o.void(YI_CALL_FIELD_PARTICLE, &[]),
            |m| {
                yi::bridge::yi_call_field_particle(m, |mm| {
                    let f = mm.rebase(PPP_FP_LOOP);
                    mm.call(f, Conv::Cdecl, &[]).unwrap();
                });
            },
        );
    }
    t.install_round4_code();
    let (m, v, model) = (t.alloc(0x40), t.alloc(0x10), t.alloc(0x100));
    for _ in 0..100 {
        t.fill(m, 0x40, &mut rng);
        t.floats(v, 4, &mut rng);
        t.same(
            "rcMat4fSetScaleVec3f",
            |o| o.eax(RC_MAT4F_SET_SCALE_VEC3F, &[m, v]),
            |mm| geom::bridge::rc_mat4f_set_scale_vec3f(mm, m, v),
        );
        let s = rng.float();
        t.same(
            "rcbgModelSetScale",
            |o| o.eax(RCBG_MODEL_SET_SCALE, &[model, s]),
            |mm| geom::bridge::rcbg_model_set_scale(mm, model, f32::from_bits(s)),
        );
    }
    let r = yi::state::RENDER.ptr(&t.b).addr;
    for _ in 0..100 {
        t.put(r + yi::render::ANIM_IDS, rng.below(0x20));
        for k in 0..yi::render::ANIMS {
            let s = r + yi::render::ANIM + k * yi::render::ANIM_SIZE;
            t.put(s, u32::from(rng.below(16) != 0));
            t.put(s + 6, u16::try_from(rng.below(0x20)).unwrap());
        }
        let (id, a, b) = (rng.below(0x24), rng.next(), rng.next());
        t.same(
            "yiAttachChrDObj",
            |o| o.eax(YI_ATTACH_CHR_DOBJ, &[id, a, b]),
            |mm| yi::bridge::yi_attach_chr_dobj(mm, id, a, b),
        );
    }
    // function tables: forces and scene objects with test functions
    let add = t.b.rebase(CODE_AREA) + 0x80;
    let after = t.b.rebase(CODE_AREA) + 0x20;
    let (p, list, items) = (t.alloc(0x40), t.alloc(0x40), t.alloc(0x400));
    for _ in 0..100 {
        t.fill(items, 0x400, &mut rng);
        for k in 0..16 {
            let item = items + 0x40 * k;
            t.put(list + 4 * k, item);
            let f = if rng.below(4) == 0 { 0 } else { add };
            t.put(item + 0x10, f);
            let g = if rng.below(4) == 0 { 0 } else { after };
            t.put(item + 0xc, g);
        }
        t.put(p + 0x34, u(rng.range(-1, 12)));
        t.put(p + 0x38, list);
        t.put(p + 0x1c, u(rng.range(-1, 12)));
        t.put(p + 0x2c, items + 0x200);
        t.same(
            "rcfxParticleResetForce",
            |o| o.void(RCFX_PARTICLE_RESET_FORCE, &[p]),
            |mm| {
                model::bridge::rcfx_particle_reset_force(mm, p, |m2, f, a, b| {
                    m2.call(f, Conv::Cdecl, &[a, b]).unwrap();
                });
            },
        );
        let psim = t.alloc(0x20);
        t.fill(psim, 0x20, &mut rng);
        t.put(psim + 0x18, p);
        t.same(
            "rcbgPSimSetReady",
            |o| o.void(RCBG_PSIM_SET_READY, &[psim]),
            |mm| {
                model::bridge::rcbg_psim_set_ready(mm, psim, |m2, f, a, b| {
                    m2.call(f, Conv::Cdecl, &[a, b]).unwrap();
                });
            },
        );
        t.put(p + 0xc, u(rng.range(-1, 12)));
        t.put(p + 0x10, list);
        t.same(
            "rcbgSceneDraw",
            |o| o.eax(RCBG_SCENE_DRAW, &[p]),
            |mm| {
                model::bridge::rcbg_scene_draw(
                    mm,
                    p,
                    |m2, f, o| {
                        call_ptr(m2, f, o);
                    },
                    |m2, f, l| {
                        ffx_vfx::op::bridge::op_func_after_ot3d_set(m2, f, l, 0, 1).cast_unsigned()
                    },
                )
            },
        );
    }
    // the particle simulation handle: FP work through a small table
    let (data, parts, table, works, handles) = (
        t.alloc(0x200),
        t.alloc(0x10),
        t.alloc(0x100),
        t.alloc(0x40),
        t.alloc(0x40),
    );
    t.set(Global::<u32>::new(FP_DATA), data);
    t.put(data + 4, 3u16);
    t.put(data + 8, parts);
    t.put(parts, table);
    t.put(table + 0x28 + 0x60 + 0xc, works);
    for _ in 0..50 {
        for i in 0..3 {
            t.put(data + 0x64 + 0x50 * i, u16::try_from(0x5b + i).unwrap());
            let run = t.b.rebase(FP_RUN) + 0x80 * i;
            t.put(run + 0x30, data);
        }
        let at = handles + 4 * rng.below(8);
        t.put(works + 8, at.wrapping_sub(0xa0).wrapping_sub(data));
        t.fill(handles, 0x40, &mut rng);
        let n = rng.below(3);
        let scratch = t.alloc(4);
        let f = [YN_PSIM_GET_HANDLE, YN_PSIM_GET_HANDLE_JUMP][rng.below(2) as usize];
        t.same(
            "Yn_PSimGetHandle",
            |o| o.eax(f, &[n]),
            |mm| {
                model::bridge::yn_psim_get_handle(mm, n, |m2, id| {
                    ffx_vfx::fp::bridge::ppp_fp_get_mng_ptr(m2, id, scratch, 0);
                    let run = Mem::read::<u32>(m2, scratch);
                    Mem::write(m2, scratch, 0u32);
                    run
                })
            },
        );
    }
    // effect objects' particle parts
    let (main, tbl, o) = (t.alloc(0x20), t.alloc(0x20), t.alloc(0x490));
    t.put(main + 0x18, tbl);
    for _ in 0..50 {
        t.fill(tbl, 0x20, &mut rng);
        t.fill(o + 0x280, 0xb0, &mut rng);
        t.put(MAGIC_ID, [0x20e, 0xbf, 0, 0x216][rng.below(4) as usize]);
        let (di, index) = (rng.below(8), rng.below(3));
        t.same(
            "rcefMainSetPar",
            |oo| oo.eax(RCEF_MAIN_SET_PAR, &[main, o, di, index]),
            |mm| task::bridge::rcef_main_set_par(mm, main, o, di, index),
        );
    }
}

// ---- round 5 ----

const YN_PART_GET_U32: [(u32, &str); 8] = [
    (0x0091_a2a0, "Yn_MapPartAnimeGetCount"),
    (0x0091_a2e0, "Yn_MapPartGetAlpha"),
    (0x0091_a3f0, "Yn_MapPartGetHeight"),
    (0x0091_a440, "Yn_MapPartGet0C"),
    (0x0091_a490, "Yn_MapPartGet10"),
    (0x0091_a580, "Yn_MapPartGet24"),
    (0x0091_a5d0, "Yn_MapPartGet28"),
    (0x0091_a620, "Yn_MapPartGetWidth"),
];
const YN_PART_GET_F32: [(u32, &str); 5] = [
    (0x0091_a330, "Yn_MapPartGetPosX"),
    (0x0091_a380, "Yn_MapPartGetPosY"),
    (0x0091_a4e0, "Yn_MapPartGetScaleY"),
    (0x0091_a530, "Yn_MapPartGetScaleX"),
    (0x0091_a670, "Yn_MapPartGet14"),
];

fn yn_part_u32(m: &mut Oracle, f: u32, part: i32) -> u32 {
    use guide::bridge as g;
    match f {
        0x0091_a2a0 => g::yn_map_part_anime_get_count(m, part),
        0x0091_a2e0 => g::yn_map_part_get_alpha(m, part),
        0x0091_a3f0 => g::yn_map_part_get_height(m, part),
        0x0091_a440 => g::yn_map_part_get_0c(m, part),
        0x0091_a490 => g::yn_map_part_get_10(m, part),
        0x0091_a580 => g::yn_map_part_get_24(m, part),
        0x0091_a5d0 => g::yn_map_part_get_28(m, part),
        _ => g::yn_map_part_get_width(m, part),
    }
}

fn yn_part_f32(m: &mut Oracle, f: u32, part: i32) -> f32 {
    use guide::bridge as g;
    match f {
        0x0091_a330 => g::yn_map_part_get_pos_x(m, part),
        0x0091_a380 => g::yn_map_part_get_pos_y(m, part),
        0x0091_a4e0 => g::yn_map_part_get_scale_y(m, part),
        0x0091_a530 => g::yn_map_part_get_scale_x(m, part),
        _ => g::yn_map_part_get_14(m, part),
    }
}

/// The script natives' map part accessors, on a synthetic Yn scene.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round5_map_parts_match_the_original() {
    use guide::bridge as g;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0005_0001);
    let (sys, map, scene, layers) = (
        t.alloc(0x20),
        t.alloc(0x20),
        t.alloc(0x110),
        t.alloc(0xc0 * 6),
    );
    t.set(guide::YN_MAP_SYSTEM, sys);
    t.put(sys, map);
    t.put(map + 8, scene);
    t.put(scene + scene::layout::LAYERS, layers);
    for round in 0..60 {
        t.floats(layers, 0xc0 * 6 / 4, &mut rng);
        t.put(scene + scene::layout::LAYER_COUNT, u(rng.range(0, 6)));
        // occasionally no scene at all (count 0)
        t.put(sys, if round % 10 == 9 { 0 } else { map });
        let part = rng.range(0, 7);
        for (f, name) in YN_PART_GET_U32 {
            let p = u(part);
            t.same(name, |o| o.eax(f, &[p]), |m| yn_part_u32(m, f, part));
        }
        for (f, name) in YN_PART_GET_F32 {
            t.same(
                name,
                |o| o.float(f, &[u(part)]),
                |m| bits(yn_part_f32(m, f, part)),
            );
        }
        let (a, b, c) = (rng.any_float(), rng.any_float(), rng.next());
        let zero = if rng.below(3) == 0 { 0 } else { a };
        let p = u(part);
        t.same(
            "Yn_MapPartAnimeSetMode",
            |o| o.void(0x0091_9fe0, &[p, c]),
            |m| g::yn_map_part_anime_set_mode(m, part, c),
        );
        t.same(
            "Yn_MapPartAnimeFlag0Off",
            |o| o.void(0x0091_a030, &[p]),
            |m| g::yn_map_part_anime_flag0_off(m, part),
        );
        t.same(
            "Yn_MapPartAnimeFlag0On",
            |o| o.void(0x0091_a080, &[p]),
            |m| g::yn_map_part_anime_flag0_on(m, part),
        );
        t.same(
            "Yn_MapPartMoveTo",
            |o| o.void(0x0091_a0f0, &[p, a, b, c]),
            |m| g::yn_map_part_move_to(m, part, a, b, c),
        );
        t.same(
            "Yn_MapPartScaleTo",
            |o| o.void(0x0091_a150, &[p, zero, b, c]),
            |m| g::yn_map_part_scale_to(m, part, zero, b, c),
        );
        t.same(
            "Yn_MapPartScaleTo 0",
            |o| o.void(0x0091_a150, &[p, a, 0x8000_0000, c]),
            |m| {
                g::yn_map_part_scale_to(m, part, a, 0x8000_0000, c);
            },
        );
        t.same(
            "Yn_MapPartAnimeSetFrame",
            |o| o.void(0x0091_a930, &[p, c]),
            |m| {
                g::yn_map_part_anime_set_frame(m, part, u16::try_from(c & 0xffff).unwrap());
            },
        );
        t.same(
            "Yn_MapPartAnimeSetLastFrame",
            |o| o.void(0x0091_a970, &[p]),
            |m| g::yn_map_part_anime_set_last_frame(m, part),
        );
        t.same(
            "Yn_MapPartAnimeSetRate",
            |o| o.void(0x0091_a9c0, &[p, c]),
            |m| g::yn_map_part_anime_set_rate(m, part, c),
        );
        t.same(
            "Yn_MapPartSetPos",
            |o| o.void(0x0091_aae0, &[p, a, b]),
            |m| g::yn_map_part_set_pos(m, part, a, b),
        );
        t.same(
            "Yn_MapPartSet24",
            |o| o.void(0x0091_ac40, &[p, a, c]),
            |m| g::yn_map_part_set_24(m, part, a, c),
        );
        t.same(
            "Yn_MapPartSetSize",
            |o| o.void(0x0091_ac80, &[p, c, a]),
            |m| g::yn_map_part_set_size(m, part, c, a),
        );
        t.same(
            "Yn_MapPartSet14",
            |o| o.void(0x0091_acc0, &[p, b]),
            |m| g::yn_map_part_set_14(m, part, b),
        );
        // scene objects through Yn
        let (kind, n) = (rng.below(3), rng.below(3));
        let objs = t.alloc(0x20);
        for k in 0..4 {
            let o = t.alloc(8);
            t.put(o, rng.below(3));
            t.put(objs + 4 * k, o);
        }
        t.put(scene + scene::layout::OBJECT_COUNT, rng.below(5));
        t.put(scene + scene::layout::OBJECTS, objs);
        t.same(
            "Yn_SceneGetObject",
            |o| o.eax(0x0092_1be0, &[kind, n]),
            |m| model::bridge::yn_scene_get_object(m, kind, n),
        );
        t.same(
            "Yn_SceneCountObjects",
            |o| o.eax(0x0092_1c20, &[kind]),
            |m| model::bridge::yn_scene_count_objects(m, kind),
        );
    }
}

/// The sphere grid's moves, tilt, homing and loading.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round5_abmap_matches_the_original() {
    use ffx_field::abmap::{self, bridge as ab};
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0005_0002);
    let g = t.alloc(0x1_2000);
    t.set(abmap::ABMAP, g);
    let pad = t.alloc(0x2800);
    t.set(abmap::TILT_PAD, pad);
    // homing, moving to a target, the homing state, the tilt
    for _ in 0..200 {
        t.floats(g + 0x11000, 0x800 / 4, &mut rng);
        t.fill(g + 0x800, 0x200, &mut rng);
        for k in 0..12 {
            t.put(
                g + 0x808 + 0x28 * k + 6,
                u16::try_from(rng.below(16)).unwrap(),
            );
        }
        t.fill(g + 0xf800, 0x400, &mut rng);
        let (node, f) = (rng.below(12), rng.any_float());
        t.same(
            "AbmapInitHoming",
            |o| o.eax(0x00a4_8d70, &[node, f]),
            |m| ab::abmap_init_homing(m, node, f),
        );
        // progress in a useful range
        #[allow(clippy::cast_precision_loss)]
        let (p, r) = (
            (rng.range(0, 1100) as f32 / 1000.0).to_bits(),
            (rng.range(0, 200) as f32 / 1000.0).to_bits(),
        );
        t.put(g + 0x11620, p);
        t.put(g + 0x11624, r);
        t.put(g + 0x11634, u16::try_from(rng.below(12)).unwrap());
        t.same(
            "AbmapState_HomeOnNode",
            |o| o.eax(0x00a5_9e80, &[]),
            ab::abmap_state_home_on_node,
        );
        let target = rng.below(7);
        let n = rng.next() & 0xffff;
        if rng.below(2) == 0 {
            t.put(g + 0x115b0, 0u32);
        }
        t.same(
            "AbmapInitMovingToTarget",
            |o| o.eax(0x00a4_8c80, &[target, n]),
            |m| ab::abmap_init_moving_to_target(m, target, u16::try_from(n).unwrap()),
        );
        t.put(pad + 0x2700, if rng.below(2) == 0 { 0xffffu16 } else { 1 });
        t.put(g + 0x1166e, u16::try_from(rng.below(4)).unwrap());
        t.put(g + 0x115cb, u8::try_from(rng.below(4)).unwrap());
        t.put(
            g + 0x115ce,
            u16::try_from(rng.range(-2, 31) & 0xffff).unwrap(),
        );
        t.put(g + 0x11340, u(rng.range(-8000, 100)));
        t.same(
            "AbmapCtrlTilt",
            |o| o.eax(0x00a4_8f50, &[]),
            ab::abmap_ctrl_tilt,
        );
    }
    // move costs: a small graph of 10 nodes with up to 5 links each
    let links = t.alloc(0x14 * 24);
    for _ in 0..40 {
        t.fill(links, 0x14 * 24, &mut rng);
        for l in 0..24 {
            let at = links + 0x14 * l;
            t.put(at, u16::try_from(rng.below(10)).unwrap());
            t.put(at + 2, u16::try_from(rng.below(10)).unwrap());
            t.put(at + 0xe, u8::try_from(rng.below(8) & 6).unwrap());
        }
        for k in 0..10 {
            let nd = g + 0x28 * k;
            for s in 0..5 {
                let l = if rng.below(4) == 0 {
                    0
                } else {
                    links + 0x14 * rng.below(24)
                };
                t.put(nd + 0x814 + 4 * s, l);
            }
            t.put(nd + 0x82c, 0xffffu16);
        }
        t.set(abmap::MOVE_STEP, rng.below(5));
        t.set(abmap::MOVE_LIMIT, rng.range(0, 40));
        t.set(abmap::MOVE_MASK, u8::try_from(rng.next() & 0xff).unwrap());
        let (node, cost) = (rng.below(10), rng.below(4));
        t.same(
            "AbmapCalcMoveCost",
            |o| o.eax(0x00a4_7560, &[node, cost]),
            |m| ab::abmap_calc_move_cost(m, u16::try_from(node).unwrap(), cost),
        );
    }
    // the saved grid, panels and joined characters
    let save = t.b.rebase(0x0112_ec7c);
    let ply = ffx_battle::save::bridge::PLY_SAVE.ptr(&t.b).addr;
    for _ in 0..30 {
        t.fill(save, 0x1000, &mut rng);
        t.put(g + 2, u16::try_from(rng.below(20)).unwrap());
        t.put(g + 4, u16::try_from(rng.below(20)).unwrap());
        for k in 0..20 {
            let kind = if rng.below(4) == 0 {
                0xffff
            } else {
                u16::try_from(rng.below(40)).unwrap()
            };
            t.put(g + 0x808 + 0x28 * k + 6, kind);
        }
        t.same(
            "LoadSaveAbmap",
            |o| o.eax(0x00a4_9590, &[]),
            ab::load_save_abmap,
        );
        for c in 0..8 {
            t.put(
                ply + c * 0x94 + 0x2c,
                u8::try_from(rng.next() & 0xff).unwrap(),
            );
        }
        let (a, node) = (rng.below(40), rng.below(20));
        t.same(
            "abmap_test_panel",
            |o| o.eax(0x00a4_57d0, &[a, node]),
            |m| ab::abmap_test_panel(m, a, node),
        );
        let chr = rng.below(8);
        t.same(
            "abmap_other_has_panel",
            |o| o.eax(0x00a4_5800, &[chr, node]),
            |m| ab::abmap_other_has_panel(m, chr, node),
        );
        t.same(
            "abmap_chr_node",
            |o| o.eax(0x00a4_5870, &[chr]),
            |m| ab::abmap_chr_node(m, chr),
        );
    }
}

/// `srand` import slot (MSVCR110).
const SRAND_SLOT: u32 = 0x00b0_c484;

impl Twin {
    /// Points `srand` at a recorder in code space that appends each seed to
    /// the log at `log` (`[log]` the write position); returns the log.
    fn record_srand(&mut self) -> u32 {
        let log = self.alloc(0x100);
        self.put(log, log + 4);
        // mov eax,[esp+4]; mov ecx,[log]; mov [ecx],eax; add dword [log],4; ret
        let l = log.to_le_bytes();
        let code = [
            0x8b, 0x44, 0x24, 0x04, 0x8b, 0x0d, l[0], l[1], l[2], l[3], 0x89, 0x01, 0x83, 0x05,
            l[0], l[1], l[2], l[3], 0x04, 0xc3,
        ];
        for o in [&mut self.a, &mut self.b] {
            let at = o.rebase(CODE_AREA + 0xa0);
            o.write(at, &code).unwrap();
            let slot = o.rebase(SRAND_SLOT);
            o.write(slot, &at.to_le_bytes()).unwrap();
        }
        log
    }
}

/// What the recorder does, on the bridge's side.
fn srand_log(log: u32) -> impl FnMut(&mut Oracle, u32) {
    move |m, seed| {
        let at = Mem::read::<u32>(m, log);
        Mem::write(m, at, seed);
        Mem::write(m, log, at + 4);
    }
}

/// The render packets, the VU0 random step, Lulu's table and the rest.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round5_render_and_ke_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    t.install_round4_code();
    let mut rng = Xorshift(0x0005_0003);
    // CLUT packet
    let (pk, cluts) = (t.alloc(0x6_8000), t.alloc(72 * 0x400));
    t.fill(cluts, 72 * 0x400, &mut rng);
    for mode in [2, 3, 4, 7] {
        t.put(pk + 0x67f20, mode);
        let n = rng.below(4);
        let src = if mode == 7 { 0 } else { cluts };
        t.same(
            "yiClutAlloc_makepacket",
            |o| o.eax(0x0091_7050, &[pk, src, n]),
            |m| yi::bridge::yi_clut_alloc_makepacket(m, pk, src, n),
        );
    }
    // GS set-up
    let r = t.alloc(0xa_0000);
    for _ in 0..100 {
        let flags = rng.next() & 0x0200_0004;
        t.put(r + yi::render::FLAGS, flags);
        t.fill(r + yi::render::BACK_COLOR, 4, &mut rng);
        t.put(r + yi::render::FRAME_BASE, rng.next());
        for i in 0..4 {
            #[allow(clippy::cast_precision_loss)]
            let (s, o) = if rng.below(2) == 0 {
                (
                    (rng.range(0, 300) as f32 / 128.0).to_bits(),
                    (rng.range(-200, 200) as f32).to_bits(),
                )
            } else {
                (
                    (rng.range(-100_000, 100_000) as f32 / 64.0).to_bits(),
                    (rng.range(-100_000, 100_000) as f32 / 64.0).to_bits(),
                )
            };
            t.put(r + yi::render::COLOR_SCALE + 4 * i, s);
            t.put(r + yi::render::COLOR_OFFSET + 4 * i, o);
        }
        t.same(
            "yiRender_setupparamGS",
            |o| o.void(0x0090_d260, &[r]),
            |m| yi::bridge::yi_render_setupparam_gs(m, r),
        );
    }
    // the VU0 random step, srand recorded
    let log = t.record_srand();
    for _ in 0..50 {
        t.put(t.b.rebase(0x00c8_f508), rng.next());
        t.set(vu0::VU0_R, 0x3f80_0000 | (rng.next() & 0x007f_ffff));
        t.set(vu0::VU0_R_SEEDED, rng.below(2));
        t.fill(t.b.rebase(0x00c8_f528), 0x10, &mut rng);
        t.fill(t.b.rebase(0x00c8_f588), 0x10, &mut rng);
        t.put(log, log + 4);
        t.same(
            "KeVu0RndI",
            |o| o.eax(0x0075_f920, &[]),
            |m| vu0::bridge::ke_vu0_rnd_i(m, srand_log(log)),
        );
    }
    // Lulu's digit tables by group and magic
    let chr = t.alloc(0x1_0000);
    t.set(ffx_battle::actor::bridge::CHR_TOP, chr);
    for group in 0..21 {
        for mag in [
            0u8, 0x19, 0x22, 0x23, 0x3b, 0x5a, 0x63, 0x95, 0xc7, 0xfe, 0xff,
        ] {
            t.put(chr + 5 * 0xf90 + 0x5aa, mag);
            t.same(
                "NkzSetLuluLimitTbl",
                |o| o.void(0x00a5_cc70, &[group]),
                |m| nkz::bridge::nkz_set_lulu_limit_tbl(m, group),
            );
        }
    }
    // battle streaming id (with a magic file loaded)
    for _ in 0..20 {
        t.put(MAGIC_ID, rng.below(0x300));
        let (a, b) = if rng.below(2) == 0 {
            (0, t.alloc(0x10))
        } else {
            (t.alloc(0x10), 0)
        };
        let (a, b) = if rng.below(4) == 0 { (0, 0) } else { (a, b) };
        if a != 0 {
            t.put(a + 8, rng.next());
        }
        if b != 0 {
            t.put(b + 8, rng.next());
        }
        t.same(
            "SqsBattleStreamingDataSend",
            |o| o.void(0x0081_cef0, &[a, b]),
            |m| {
                nkz::bridge::sqs_battle_streaming_data_send(m, a, b, |_| panic!("no error path"));
            },
        );
    }
    // KeThBP_Quit: buckets of linked objects, the owner in a list
    let (bp, objs, prev, next) = (
        t.alloc(0x58 + 0x30 * 8),
        t.alloc(0x40 * 16),
        t.alloc(0x10),
        t.alloc(0x10),
    );
    for _ in 0..30 {
        t.fill(objs, 0x40 * 16, &mut rng);
        t.fill(bp, 0x58, &mut rng);
        for k in 0..8 {
            let head = if rng.below(3) == 0 {
                0
            } else {
                objs + 0x40 * rng.below(8)
            };
            t.put(bp + 0x58 + 0x30 * k, head);
        }
        for k in 0..16 {
            let nx = if k >= 8 || rng.below(3) == 0 {
                0
            } else {
                objs + 0x40 * (k + 8)
            };
            t.put(objs + 0x40 * k + 8, nx);
        }
        t.put(bp + 0x3c, u16::try_from(rng.below(5)).unwrap());
        t.put(bp + 0x34, u16::try_from(rng.below(4)).unwrap());
        t.put(bp + 0x38, u16::try_from(rng.below(4)).unwrap());
        t.put(bp + 8, if rng.below(3) == 0 { 0 } else { next });
        t.put(bp + 0xc, if rng.below(3) == 0 { 0 } else { prev });
        t.same(
            "KeThBP_Quit",
            |o| o.void(0x0076_36c0, &[bp]),
            |m| ke::bridge::ke_th_bp_quit(m, bp),
        );
    }
    // small Ke helpers
    let (p, q) = (t.alloc(0x10), t.alloc(0x10));
    t.set(ke::PMCOM_NEXT, t.b.rebase(ke::PMCOM_BASE));
    for _ in 0..100 {
        let n = rng.below(0x40_000);
        t.same(
            "KePmcomAlloc",
            |o| o.eax(0x0076_2f50, &[n]),
            |m| ke::bridge::ke_pmcom_alloc(m, n),
        );
        let (kind, ptr) = (rng.next(), rng.next());
        t.same(
            "KePmcomQueuePush",
            |o| o.void(0x0076_2fa0, &[kind, ptr]),
            |m| ke::bridge::ke_pmcom_queue_push(m, kind, ptr),
        );
        t.floats(q, 4, &mut rng);
        let s = rng.float();
        t.same(
            "KeVu0MulX",
            |o| o.void(0x0076_76c0, &[p, q, s]),
            |m| ke::bridge::ke_vu0_mul_x(m, p, q, f32::from_bits(s)),
        );
        t.same(
            "KeVu0SubX",
            |o| o.void(0x0076_7720, &[p, q, s]),
            |m| ke::bridge::ke_vu0_sub_x(m, p, q, f32::from_bits(s)),
        );
        t.fill(p, 0x10, &mut rng);
        t.same(
            "KeLnsDefault_Init",
            |o| o.eax(0x0076_a640, &[p]),
            |m| ke::bridge::ke_lns_default_init(m, p),
        );
    }
    t.same(
        "NkzGetMonsterListNum",
        |o| o.eax(0x00a5_cb20, &[]),
        |_| nkz::bridge::nkz_get_monster_list_num(),
    );
}

/// Palettes, layers, texture sizes, FP fades, characters, matrices.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round5_scene_helpers_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    t.install_round4_code();
    let mut rng = Xorshift(0x0005_0004);
    let (pal, trx, layer, h, out) = (
        t.alloc(0x20),
        t.alloc(0x100),
        t.alloc(0xc0),
        t.alloc(4),
        t.alloc(0x10),
    );
    for _ in 0..100 {
        t.fill(trx, 0x100, &mut rng);
        t.put(trx + 0x30, rng.below(4) + 1);
        t.put(pal + 0xc, trx);
        let n = rng.below(4);
        t.same(
            "rcbgPalGetEntryData",
            |o| o.eax(0x0092_9990, &[pal, n]),
            |m| model::bridge::rcbg_pal_get_entry_data(m, pal, n),
        );
        t.same(
            "rcbgPalGetLastEntryData",
            |o| o.eax(0x0092_9930, &[pal]),
            |m| model::bridge::rcbg_pal_get_last_entry_data(m, pal),
        );
        t.put(layer + 0xbc, if rng.below(3) == 0 { 0 } else { trx });
        t.same(
            "rcbgLayerGetTexData",
            |o| o.eax(0x0092_acf0, &[layer]),
            |m| model::bridge::rcbg_layer_get_tex_data(m, layer),
        );
        t.put(h, trx);
        t.same(
            "rcdtChunkHandleGetEnd",
            |o| o.eax(0x0092_bb90, &[h]),
            |m| rcdt::bridge::rcdt_chunk_handle_get_end(m, h),
        );
        let (w, hh) = (rng.range(-70, 2000), rng.range(-70, 2000));
        let outs = [out, out + 4, out + 8].map(|a| if rng.below(4) == 0 { 0 } else { a });
        t.same(
            "rcbgTexGetLog2Size",
            |o| o.void(0x0092_0050, &[u(w), u(hh), outs[0], outs[1], outs[2]]),
            |m| {
                model::bridge::rcbg_tex_get_log2_size(m, w, hh, outs[0], outs[1], outs[2]);
            },
        );
        t.fill(trx, 0x40, &mut rng);
        t.same(
            "rcMat4fSetScale001",
            |o| o.eax(0x0092_de50, &[trx]),
            |m| geom::bridge::rc_mat4f_set_scale_001(m, trx),
        );
        t.same(
            "rcMat4fSetScale0001",
            |o| o.eax(0x0092_de80, &[trx]),
            |m| geom::bridge::rc_mat4f_set_scale_0001(m, trx),
        );
        let (id, step, frames) = (rng.below(8), rng.any_float(), rng.next());
        t.same(
            "rcbgFpFadeInit",
            |o| o.void(0x0092_2930, &[layer, id, step, frames]),
            |m| {
                model::bridge::rcbg_fp_fade_init(m, layer, id, step, frames, |mm, id| {
                    ffx_vfx::fp::bridge::ppp_fp_id_disp_off(mm, id);
                });
            },
        );
    }
    // particle simulation positions: ready after the initial count
    let add = t.b.rebase(CODE_AREA) + 0x80;
    let (p, sys, list, item, v) = (
        t.alloc(0xb0),
        t.alloc(0x40),
        t.alloc(4),
        t.alloc(0x20),
        t.alloc(0x10),
    );
    t.put(list, item);
    t.put(item + 0x10, add);
    for _ in 0..100 {
        t.fill(p, 0xb0, &mut rng);
        t.floats(v, 4, &mut rng);
        t.put(p + 0x18, sys);
        t.put(p + 0x9c, rng.below(2));
        t.put(p + 0xa0, rng.below(2));
        t.put(p + 0xa4, rng.below(3));
        t.put(sys + 0x34, rng.below(2));
        t.put(sys + 0x38, list);
        t.put(sys + 0x1c, 0u32);
        let which = rng.below(2);
        t.same(
            "rcbgPSimSetPos",
            |o| o.void(0x0091_ff10, &[p, which, v]),
            |m| {
                model::bridge::rcbg_psim_set_pos(m, p, which, v, |m2, f, a, b| {
                    m2.call(f, Conv::Cdecl, &[a, b]).unwrap();
                });
            },
        );
    }
    // Yn's battle number: none in the image, so nothing changes or the
    // number stays
    let sys = t.alloc(0x20);
    t.set(guide::YN_MAP_SYSTEM, sys);
    for v in [0u32, 5] {
        t.put(sys + 0xc, v);
        t.same(
            "Yn_BattleCaptureNum",
            |o| o.void(0x0092_1910, &[]),
            model::bridge::yn_battle_capture_num,
        );
    }
    // characters: an empty image
    for id in [0u32, 1, 0x1234] {
        t.same(
            "rcbgChrFindById",
            |o| o.eax(0x0092_e1e0, &[id]),
            |m| model::bridge::rcbg_chr_find_by_id(m, id),
        );
    }
    let work = t.alloc(0x900);
    t.set(rcui::WORK, work);
    t.same(
        "rcuiCollectChrHandles",
        |o| o.void(0x0093_2570, &[]),
        rcui::bridge::rcui_collect_chr_handles,
    );
}

/// Round 5 on the game's state: characters, the battle number, Lulu's
/// magic, map parts.
#[test]
fn round5_state_matches_the_original_on_a_snapshot() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    for id in 0..0x20 {
        t.same(
            "rcbgChrFindById",
            |o| o.eax(0x0092_e1e0, &[id]),
            |m| model::bridge::rcbg_chr_find_by_id(m, id),
        );
    }
    let battle = ffx_battle::actor::bridge::CHR_TOP.get(&t.b) != 0;
    for i in (0..0x30).filter(|_| battle) {
        let hd = ffx_battle::actor::bridge::ms_get_chr_handle(&t.b, i);
        t.same(
            "rcbgChrHandleIndex",
            |o| o.eax(0x0092_4f30, &[hd]),
            |m| model::bridge::rcbg_chr_handle_index(m, hd),
        );
    }
    let work = t.alloc(0x900);
    t.set(rcui::WORK, work);
    t.same(
        "rcuiCollectChrHandles",
        |o| o.void(0x0093_2570, &[]),
        rcui::bridge::rcui_collect_chr_handles,
    );
    let scratch = t.alloc(4);
    t.same(
        "Yn_SeaGetWork",
        |o| o.eax(0x0091_9a10, &[]),
        |m| {
            model::bridge::yn_sea_get_work(m, |mm, id| {
                ffx_vfx::fp::bridge::ppp_fp_get_mng_ptr(mm, id, scratch, 0);
                let run = Mem::read::<u32>(mm, scratch);
                Mem::write(mm, scratch, 0u32);
                run
            })
        },
    );
    if guide::YN_MAP_SYSTEM.get(&t.b) != 0 {
        t.same(
            "Yn_BattleCaptureNum",
            |o| o.void(0x0092_1910, &[]),
            model::bridge::yn_battle_capture_num,
        );
        for part in 0..8 {
            for (f, name) in YN_PART_GET_U32 {
                t.same(
                    name,
                    |o| o.eax(f, &[part]),
                    |m| yn_part_u32(m, f, i32::try_from(part).unwrap()),
                );
            }
        }
        for kind in 0..4 {
            t.same(
                "Yn_SceneCountObjects",
                |o| o.eax(0x0092_1c20, &[kind]),
                |m| model::bridge::yn_scene_count_objects(m, kind),
            );
        }
    }
    if battle {
        for group in 0..20 {
            t.same(
                "NkzSetLuluLimitTbl",
                |o| o.void(0x00a5_cc70, &[group]),
                |m| nkz::bridge::nkz_set_lulu_limit_tbl(m, group),
            );
        }
    }
}

// ---- round 6 ----

/// A born function: counts calls in `obj +0x40`, sums the vertex
/// pointers in `+0x44`, asks for more until the fifth.
const BORN_STUB: [u8; 30] = [
    0x8b, 0x44, 0x24, 0x04, 0xff, 0x40, 0x40, 0x8b, 0x4c, 0x24, 0x0c, 0x01, 0x48, 0x44, 0x31, 0xc9,
    0x83, 0x78, 0x40, 0x05, 0x0f, 0x9c, 0xc1, 0x8b, 0x54, 0x24, 0x10, 0x89, 0x0a, 0xc3,
];

/// Sea and particle simulation settings, Sqs DMA, transfers, the sprite
/// packet's VU0 registers, particle birth.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round6_ke_sqs_and_sea_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    t.install_round4_code();
    let log = t.record_srand();
    let mut rng = Xorshift(0x0006_0001);
    // transfers loaded through a parser
    let (trx, c, p, data) = (t.alloc(0x40), t.alloc(8), t.alloc(0x90), t.alloc(0x200));
    for _ in 0..30 {
        t.fill(data, 0x200, &mut rng);
        t.put(data + 0x20, rng.below(4));
        t.put(data + 0x24, rng.below(0x20));
        t.fill(trx, 0x40, &mut rng);
        t.same(
            "rcdtChunkSetBin",
            |o| o.void(RCDT_CHUNK_SET_BIN, &[c, data]),
            |m| rcdt::bridge::rcdt_chunk_set_bin(m, c, data),
        );
        t.same(
            "rcdtParseSet",
            |o| o.void(RCDT_PARSE_SET, &[p, c]),
            |m| rcdt::bridge::rcdt_parse_set(m, p, c),
        );
        t.same(
            "rcbgTrxLoad",
            |o| o.void(0x0092_b710, &[trx, p]),
            |m| model::bridge::rcbg_trx_load(m, trx, p),
        );
    }
    // the sea (no FP data in the image: nothing to set) and simulations
    let v = t.alloc(0x10);
    let scratch = t.alloc(4);
    let run_of = move |mm: &mut Oracle, id: u32| {
        ffx_vfx::fp::bridge::ppp_fp_get_mng_ptr(mm, id, scratch, 0);
        let run = Mem::read::<u32>(mm, scratch);
        Mem::write(mm, scratch, 0u32);
        run
    };
    t.same(
        "Yn_SeaSetAng",
        |o| o.eax(0x0091_ae40, &[v]),
        |m| model::bridge::yn_sea_set_ang(m, v, run_of),
    );
    t.same(
        "Yn_SeaSetPos",
        |o| o.eax(0x0091_ae80, &[v]),
        |m| model::bridge::yn_sea_set_pos(m, v, run_of),
    );
    t.same(
        "Yn_SeaSetUvSpd",
        |o| o.void(0x0091_aea0, &[1, 2]),
        |m| model::bridge::yn_sea_set_uv_spd(m, 1, 2, run_of),
    );
    let (ps, sys) = (t.alloc(0xb0), t.alloc(0x40));
    for _ in 0..30 {
        t.fill(ps, 0xb0, &mut rng);
        t.floats(v, 4, &mut rng);
        t.put(ps + 0x18, sys);
        t.put(ps + 0xa4, rng.below(2));
        t.put(ps + 0x9c, 0u32);
        t.put(ps + 0xa0, 0u32);
        t.put(sys + 0x34, 0u32);
        t.put(sys + 0x1c, 0u32);
        let which = rng.range(-1, 3);
        let p = if rng.below(4) == 0 { 0 } else { ps };
        t.same(
            "Yn_PSimSetPos",
            |o| o.void(0x0091_9f10, &[p, u(which), v]),
            |m| {
                model::bridge::yn_psim_set_pos(m, p, which, v, |_, _, _, _| panic!("no forces"));
            },
        );
    }
    // Sqs: shout DMA records
    for _ in 0..30 {
        t.set(nkz::SHOUT_SKIPPED, rng.below(2));
        t.same(
            "SqsShoutStreamingPlayState",
            |o| o.eax(0x0081_edf0, &[]),
            nkz::bridge::sqs_shout_streaming_play_state,
        );
        let (a, b, cc) = (rng.next(), rng.next(), rng.next());
        t.same(
            "SqsSifDmaStart",
            |o| o.eax(0x0081_eeb0, &[0, a, b, cc]),
            |m| nkz::bridge::sqs_sif_dma_start(m, a, b, cc),
        );
        t.set(nkz::SHOUT_BUFFER, rng.next());
        let src = if rng.below(4) == 0 { 0 } else { rng.next() };
        let size = rng.range(0, 0x14_0000);
        t.same(
            "SqsShoutStreamingDataSend",
            |o| o.void(0x0081_ed40, &[src, u(size)]),
            |m| {
                nkz::bridge::sqs_shout_streaming_data_send(m, src, size);
            },
        );
    }
    t.same(
        "yiArcPrintSize",
        |o| o.eax(0x0090_e6f0, &[]),
        |_| yi::bridge::yi_arc_print_size(),
    );
    // the sprite packet's registers
    let (spr, m20, m28) = (t.alloc(0x60), t.alloc(0x20), t.alloc(0x40));
    t.put(spr + 0x20, m20);
    t.put(spr + 0x28, m28);
    let spr_base = t.alloc(0x400);
    t.set(ffx_vfx::op::bridge::SPR_TOP, 0);
    let stack = ffx_vfx::op::bridge::SPR_STACK.ptr(&t.b).addr;
    t.put(stack, spr_base);
    for _ in 0..50 {
        t.floats(spr + 0x30, 0x30 / 4, &mut rng);
        t.floats(m20, 8, &mut rng);
        t.floats(m28, 16, &mut rng);
        t.floats(t.b.rebase(0x00c0_a000), 5, &mut rng);
        t.floats(t.b.rebase(0x00c8_f500), 0x500 / 4, &mut rng);
        // a scratchpad stack with one free block, random contents
        t.fill(spr_base, 0x100, &mut rng);
        t.same(
            "KeDttSpr_MkGifPk_Pre",
            |o| o.void(0x0076_8d30, &[spr]),
            |m| {
                ke::bridge::ke_dtt_spr_mk_gif_pk_pre(
                    m,
                    spr,
                    |mm| ffx_vfx::op::bridge::op_spr_alloc(mm, 0x90),
                    ffx_vfx::op::bridge::op_spr_free,
                );
            },
        );
    }
    // particle birth through a test function
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(CODE_AREA + 0xc0);
        o.write(at, &BORN_STUB).unwrap();
    }
    let born = t.b.rebase(CODE_AREA + 0xc0);
    let (cell, vdata, shapes, models, pts, obj, idx, more) = (
        t.alloc(4),
        t.alloc(0x40),
        t.alloc(0x40),
        t.alloc(0x40),
        t.alloc(0x40),
        t.alloc(0xa0),
        t.alloc(4),
        t.alloc(4),
    );
    t.put(t.b.rebase(ke::VFX_DATA), cell);
    t.put(cell, vdata);
    t.put(vdata + 0x28, shapes);
    t.put(vdata + 0x20, models);
    for k in 0..4 {
        t.put(
            shapes + 8 * k,
            if k == 3 {
                0x9018u16
            } else {
                u16::try_from(k).unwrap()
            },
        );
        t.put(shapes + 8 * k + 2, u16::try_from(1 + rng.below(6)).unwrap());
        t.put(shapes + 8 * k + 4, pts);
        t.put(models + 0x20 * k + 0x14, 0x1000 * (k + 1));
    }
    for _ in 0..60 {
        t.fill(pts, 0x40, &mut rng);
        t.put(obj + 0x40, u(rng.range(0, 6)));
        t.put(obj + 0x90, if rng.below(2) == 0 { 0 } else { 0x2000 });
        t.put(idx, u16::try_from(rng.below(6)).unwrap());
        let (shape, random, n) = (rng.below(4), rng.below(2), rng.below(8));
        t.put(log, log + 4);
        t.same(
            "KeBornRnd2",
            |o| o.eax(0x0076_9230, &[obj, 7, born, idx, shape, random, n]),
            |m| {
                let mut srand = srand_log(log);
                ke::bridge::ke_born_rnd2(
                    m,
                    obj,
                    7,
                    born,
                    idx,
                    i16::try_from(shape).unwrap(),
                    u8::try_from(random).unwrap(),
                    u8::try_from(n).unwrap(),
                    |mm| vu0::bridge::ke_vu0_rnd_i(mm, &mut srand),
                    |mm, f, a, b, at| {
                        mm.call(f, Conv::Cdecl, &[a, b, at, more]).unwrap();
                        let v = Mem::read::<u32>(mm, more);
                        Mem::write(mm, more, 0u32);
                        v
                    },
                )
            },
        );
    }
}

/// Parser scans, rcui vectors and matrices, palettes, TIM2 rectangles,
/// fixed point, screen depth, the view matrix.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round6_parse_rcui_and_view_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0006_0002);
    // parser scans over random text with random separators
    let (c, p, data) = (t.alloc(8), t.alloc(0x110), t.alloc(0x100));
    for _ in 0..100 {
        let text: Vec<u8> = (0..0x100)
            .map(|_| [b'a', b' ', b'\n', b',', b'x', 0][rng.below(6) as usize])
            .collect();
        t.write(data, &text);
        t.same(
            "rcdtChunkSetBin",
            |o| o.void(RCDT_CHUNK_SET_BIN, &[c, data]),
            |m| rcdt::bridge::rcdt_chunk_set_bin(m, c, data),
        );
        t.put(c + 4, 0x80 + rng.below(0x80));
        t.same(
            "rcdtParseSet",
            |o| o.void(RCDT_PARSE_SET, &[p, c]),
            |m| rcdt::bridge::rcdt_parse_set(m, p, c),
        );
        t.same(
            "rcdtParseInitSeparators",
            |o| o.void(RCDT_PARSE_INIT_SEPARATORS, &[p]),
            |m| rcdt::bridge::rcdt_parse_init_separators(m, p),
        );
        let start = Mem::read::<u32>(&t.b, p + 4);
        t.put(p + 4, start + rng.below(0x90));
        t.same(
            "rcdtParseIsEnd",
            |o| o.eax(0x0092_bfb0, &[p]),
            |m| rcdt::bridge::rcdt_parse_is_end(m, p),
        );
        t.same(
            "rcdtParseSkipSeparators",
            |o| o.eax(0x0092_bd60, &[p]),
            |m| rcdt::bridge::rcdt_parse_skip_separators(m, p),
        );
        t.same(
            "rcdtParseSkipToken",
            |o| o.eax(0x0092_bd20, &[p]),
            |m| rcdt::bridge::rcdt_parse_skip_token(m, p),
        );
        let ch = [u32::from(b','), u32::from(b'x'), 0xff, 0x41][rng.below(4) as usize];
        t.same(
            "rcdtParseFindChar",
            |o| o.eax(0x0092_bae0, &[p, ch]),
            |m| rcdt::bridge::rcdt_parse_find_char(m, p, u8::try_from(ch).unwrap().cast_signed()),
        );
    }
    // rcui vectors and matrices
    let (a, b, out) = (t.alloc(0x40), t.alloc(0x40), t.alloc(0x40));
    for _ in 0..200 {
        t.floats(a, 16, &mut rng);
        t.floats(b, 16, &mut rng);
        t.same(
            "rcuiVecToInt",
            |o| o.eax(0x0093_cff0, &[out, a]),
            |m| rcui::bridge::rcui_vec_to_int(m, out, a),
        );
        t.same(
            "rcuiApplyMatrix",
            |o| o.void(0x0093_cbf0, &[out, a, b]),
            |m| rcui::bridge::rcui_apply_matrix(m, out, a, b),
        );
        t.same(
            "rcuiMulMatrix",
            |o| o.void(0x0093_d1b0, &[out, a, b]),
            |m| rcui::bridge::rcui_mul_matrix(m, out, a, b),
        );
        t.same(
            "rcuiMulMatrix in place",
            |o| o.void(0x0093_d1b0, &[a, a, b]),
            |m| rcui::bridge::rcui_mul_matrix(m, a, a, b),
        );
        for i in 0..4 {
            let v = rng.float();
            t.put(b + 4 * i, v);
        }
        t.same(
            "rcuiNormalize",
            |o| o.void(0x0093_d3d0, &[out, b]),
            |m| rcui::bridge::rcui_normalize(m, out, b),
        );
        t.same(
            "rcVec4fToFixed",
            |o| o.eax(0x0092_5950, &[out, b]),
            |m| model::bridge::rc_vec4f_to_fixed(m, out, b),
        );
    }
    // mouse drag start
    let work = t.alloc(0xc00);
    t.set(rcui::WORK, work);
    for _ in 0..10 {
        t.put(t.b.rebase(0x0132_5240), rng.next());
        t.put(t.b.rebase(0x0132_5244), rng.next());
        let (id, arg) = (rng.next(), rng.below(4));
        t.same(
            "rcuiMouseMoveBegin",
            |o| o.void(0x0093_c150, &[id, arg]),
            |m| {
                use ffx_menu::gu::bridge as gu;
                rcui::bridge::rcui_mouse_move_begin(
                    m,
                    id,
                    arg,
                    gu::gu_mouse_set_capture,
                    |mm, _| gu::MOUSE_X.get(mm).cast_unsigned(),
                    |mm, _| gu::MOUSE_Y.get(mm).cast_unsigned(),
                );
            },
        );
    }
    // palettes and TIM2 rectangles
    let (pal, trx, entries, tm2, dst, img) = (
        t.alloc(0x20),
        t.alloc(0x40),
        t.alloc(0x1100 * 4),
        t.alloc(0x50),
        t.alloc(0x800),
        t.alloc(0x2000),
    );
    for _ in 0..20 {
        t.fill(entries, 0x4400, &mut rng);
        t.put(trx + 0x30, 2 + rng.below(3));
        t.put(trx + 0x34, 0x410u32);
        t.put(trx + 0x38, entries);
        t.put(pal + 0xc, trx);
        t.put(pal + 8, u16::try_from(rng.below(2)).unwrap());
        t.same(
            "rcbgPalCopyToLast",
            |o| o.void(0x0092_99d0, &[pal]),
            |m| model::bridge::rcbg_pal_copy_to_last(m, pal),
        );
        t.fill(img, 0x2000, &mut rng);
        t.put(tm2 + 0x40, img);
        t.put(tm2 + 0x24, u16::try_from(0x40 + rng.below(0x40)).unwrap());
        let (x, y, n, rows) = (
            rng.below(0x20),
            rng.below(8),
            rng.below(0x30),
            rng.range(-1, 8),
        );
        t.same(
            "rcdtTm2CopyRect",
            |o| o.void(0x0092_3ae0, &[tm2, dst, x, y, n, u(rows)]),
            |m| {
                model::bridge::rcdt_tm2_copy_rect(m, tm2, dst, x, y, n, rows);
            },
        );
    }
    // screen depth (with and without a scene), the view matrix
    let screen = geom::SCREEN_MATRIX.ptr(&t.b).addr;
    let (sys, map, scene) = (t.alloc(0x10), t.alloc(0x10), t.alloc(0x110));
    for k in 0..100 {
        t.floats(screen, 16, &mut rng);
        #[allow(clippy::cast_precision_loss)]
        let d = (rng.range(-5000, 5000) as f32 / 8.0).to_bits();
        t.set(guide::YN_MAP_SYSTEM, if k % 2 == 0 { 0 } else { sys });
        t.put(sys, map);
        t.put(map + 8, scene);
        t.put(scene + 0xf4, rng.float());
        t.same(
            "Yn_DepthToScreen",
            |o| o.eax(0x0092_01a0, &[d]),
            |m| guide::bridge::yn_depth_to_screen(m, f32::from_bits(d)),
        );
        t.floats(a, 16, &mut rng);
        t.set(yi::RENDER_STATUS, rng.next() & 0x800);
        let mm = if rng.below(4) == 0 { 0 } else { a };
        t.same(
            "yiSetViewMatrix",
            |o| o.void(0x0090_a980, &[mm]),
            |m| yi::bridge::yi_set_view_matrix(m, mm),
        );
    }
}

/// A sphere grid visitor `f(node, log)`: counts the call in `log[0]`,
/// records the node at `log[count]`, returns -1 when the node's `+0x20`
/// byte is at least 0xf0, else 0.
const VISIT_STUB: [u8; 29] = [
    0x8b, 0x44, 0x24, 0x04, 0x8b, 0x4c, 0x24, 0x08, 0xff, 0x01, 0x8b, 0x11, 0x89, 0x04, 0x91, 0x0f,
    0xb6, 0x40, 0x20, 0x3d, 0xf0, 0x00, 0x00, 0x00, 0x19, 0xc0, 0xf7, 0xd0, 0xc3,
];

/// Finite float words (no NaN) at `at`, for arithmetic.
fn finite_floats(t: &mut Twin, at: u32, words: u32, rng: &mut Xorshift) {
    for i in 0..words {
        let v = rng.float();
        t.put(at + 4 * i, v);
    }
}

/// Sphere grid visitors and matrices, `yi` records, rcfx initialisers,
/// rcui selection, Ke colours.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round6_grid_records_and_rcfx_match_the_original() {
    use ffx_field::abmap::{self, bridge as ab};
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0006_0003);
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(CODE_AREA + 0xe0);
        o.write(at, &VISIT_STUB).unwrap();
    }
    let visit = t.b.rebase(CODE_AREA + 0xe0);
    let g = t.alloc(0x1_2000);
    t.set(abmap::ABMAP, g);
    let (out, pos, pos2, log) = (t.alloc(0x40), t.alloc(8), t.alloc(8), t.alloc(0x400));
    let call =
        |m: &mut Oracle, f: u32, n: u32, a: u32| m.call(f, Conv::Cdecl, &[n, a]).unwrap().eax;
    for _ in 0..200 {
        // nodes, links, kinds, markers
        t.fill(g, 0x1_2000, &mut rng);
        let nodes = rng.range(-1, 40);
        t.put(g + 2, i16::try_from(nodes).unwrap());
        t.put(g + 4, i16::try_from(rng.range(-1, 30)).unwrap());
        for k in 0..64u32 {
            let kind = if rng.below(5) == 0 {
                0xffff
            } else {
                rng.below(16)
            };
            t.put(g + 0x808 + 0x28 * k + 6, u16::try_from(kind).unwrap());
            t.put(
                g + 0x808 + 0x28 * k + 0x20,
                u8::try_from(rng.below(0x100)).unwrap(),
            );
        }
        for l in 0..30u32 {
            t.put(g + 0xa808 + 0x14 * l, u16::try_from(rng.below(12)).unwrap());
            t.put(g + 0xa80a + 0x14 * l, u16::try_from(rng.below(12)).unwrap());
        }
        for c in 0..7u32 {
            t.put(
                g + 0x11088 + 0x50 * c + 0x44,
                u16::try_from(rng.below(40)).unwrap(),
            );
        }
        t.fill(pos, 8, &mut rng);
        t.fill(pos2, 8, &mut rng);
        let s = rng.any_float();
        t.same(
            "AbmapMakeNodeMatrix",
            |o| o.eax(0x00a5_ad30, &[out, pos, s]),
            |m| ab::abmap_make_node_matrix(m, out, pos, s),
        );
        t.same(
            "AbmapMakeLinkMatrix",
            |o| o.eax(0x00a5_a360, &[out, pos, pos2, s]),
            |m| ab::abmap_make_link_matrix(m, out, pos, pos2, s),
        );
        if nodes >= 0 {
            t.put(log, 0u32);
            t.same(
                "AbmapForEachNode",
                |o| o.eax(0x00a5_9710, &[visit, log]),
                |m| ab::abmap_for_each_node(m, visit, log, call),
            );
        }
        t.put(log, 0u32);
        let node = rng.below(12);
        t.same(
            "AbmapForNodeAndLinked",
            |o| o.eax(0x00a5_9760, &[node, visit, log]),
            |m| ab::abmap_for_node_and_linked(m, u16::try_from(node).unwrap(), visit, log, call),
        );
        t.same(
            "AbmapResetHelp",
            |o| o.eax(0x00a5_b930, &[]),
            ab::abmap_reset_help,
        );
        t.same(
            "AbmapPulseCursor",
            |o| o.eax(0x00a4_7440, &[]),
            ab::abmap_pulse_cursor,
        );
        let c = rng.below(7);
        t.same(
            "AbmapSetChrMarker",
            |o| o.eax(0x00a5_a990, &[c]),
            |m| ab::abmap_set_chr_marker(m, c),
        );
        let n = rng.next() >> rng.below(32);
        for f in [0x00a5_cee0, 0x00a5_cf10] {
            t.same(
                "AbmapQuarterOrOne",
                |o| o.eax(f, &[0, n]),
                |_| ab::abmap_quarter_or_one(n.cast_signed()),
            );
        }
        t.floats(out, 16, &mut rng);
        t.same(
            "AbmapVu0LoadMatrix1",
            |o| o.eax(0x00a5_be30, &[out]),
            |m| ab::abmap_vu0_load_matrix1(m, out),
        );
        t.same(
            "AbmapVu0LoadMatrix2",
            |o| o.eax(0x00a5_bec0, &[out]),
            |m| ab::abmap_vu0_load_matrix2(m, out),
        );
    }
    // yi: polygon buffers, GIF colours, records, fog
    let (buf, gif, tri, scene, rec) = (
        t.alloc(0x4090),
        t.alloc(0x200),
        t.alloc(0x20),
        t.alloc(0xba410),
        t.alloc(0x100),
    );
    t.fill(buf, 0x4090, &mut rng);
    t.same(
        "yiPolyBufClear",
        |o| o.eax(0x0091_1970, &[buf]),
        |m| yi::bridge::yi_poly_buf_clear(m, buf),
    );
    for _ in 0..100 {
        t.fill(gif, 0x200, &mut rng);
        t.fill(tri, 0x20, &mut rng);
        let (first, uv) = (rng.below(16), rng.below(16));
        let first = if rng.below(10) == 0 {
            first.wrapping_sub(2)
        } else {
            first
        };
        t.same(
            "yiTriColorsFromGif",
            |o| o.eax(0x0091_0340, &[tri, gif, first]),
            |m| yi::bridge::yi_tri_colors_from_gif(m, tri, gif, first),
        );
        t.same(
            "yiTriColorsUvFromGif",
            |o| o.eax(0x0091_03e0, &[tri, gif, first, uv]),
            |m| yi::bridge::yi_tri_colors_uv_from_gif(m, tri, gif, first, uv),
        );
        t.floats(rec, 0x40, &mut rng);
        t.same(
            "yiLoadMatrixChunk",
            |o| o.void(0x0090_ed10, &[scene, rec]),
            |m| yi::bridge::yi_load_matrix_chunk(m, scene, rec),
        );
        t.same(
            "yiObjSave",
            |o| o.eax(0x0090_ee60, &[rec, scene]),
            |m| yi::bridge::yi_obj_save(m, rec, scene),
        );
        t.floats(rec, 0x40, &mut rng);
        t.same(
            "yiObjLoad",
            |o| o.eax(0x0090_edb0, &[scene, rec]),
            |m| yi::bridge::yi_obj_load(m, scene, rec),
        );
        t.same(
            "yiCameraLoad",
            |o| o.eax(0x0090_ef10, &[scene, rec]),
            |m| yi::bridge::yi_camera_load(m, scene, rec),
        );
        t.same(
            "yiFogLoad",
            |o| o.eax(0x0090_f090, &[scene, rec]),
            |m| yi::bridge::yi_fog_load(m, scene, rec),
        );
        finite_floats(&mut t, scene + 0x681f8, 1, &mut rng);
        finite_floats(&mut t, scene + 0x68344, 3, &mut rng);
        if rng.below(4) == 0 {
            let v = Mem::read::<u32>(&t.b, scene + 0x68348);
            t.put(scene + 0x6834c, v);
        }
        t.put(scene + 0x67f08, u8::try_from(rng.below(0x100)).unwrap());
        t.same(
            "yiRender_setupFogParam",
            |o| o.void(0x0090_d1b0, &[scene]),
            |m| yi::bridge::yi_render_setup_fog_param(m, scene),
        );
        t.put(rec + 4, i16::try_from(rng.range(-2, 12)).unwrap());
        t.same(
            "yiWindowSetHandlers",
            |o| o.eax(0x0090_7d80, &[rec]),
            |m| yi::bridge::yi_window_set_handlers(m, rec),
        );
    }
    // nearest point on a line, cross product, Ke colours
    let (a, b, p, q) = (t.alloc(0x10), t.alloc(0x10), t.alloc(0x10), t.alloc(0x10));
    for k in 0..300u32 {
        for v in [a, b, p, q] {
            finite_floats(&mut t, v, 4, &mut rng);
        }
        if k % 5 == 0 {
            let x = Mem::read::<u64>(&t.b, a);
            t.put(b, x);
        }
        let clamp = rng.below(2);
        t.same(
            "Yn_NearestOnLine2D",
            |o| o.eax(0x0091_cc00, &[out, a, b, p, clamp]),
            |m| guide::bridge::yn_nearest_on_line_2d(m, out, a, b, p, clamp),
        );
        t.same(
            "rcuiCrossProduct",
            |o| o.eax(0x0093_d440, &[out, a, b]),
            |m| rcui::bridge::rcui_cross_product(m, out, a, b),
        );
        for i in 0..4 {
            #[allow(clippy::cast_precision_loss)]
            let (x, y) = (
                rng.range(-300, 600) as f32 / 2.0,
                rng.range(-300, 300) as f32 / 4.0,
            );
            t.put(a + 4 * i, x.to_bits());
            t.put(b + 4 * i, y.to_bits());
        }
        let kk = if k % 3 == 0 {
            rng.float()
        } else {
            (f32::from_bits(rng.below(0x100)) + f32::from(i16::try_from(rng.range(-2, 3)).unwrap()))
                .to_bits()
        };
        t.same(
            "KeAddScaledColor",
            |o| o.void(0x0076_4c40, &[out, a, b, kk]),
            |m| ke::bridge::ke_add_scaled_color(m, out, a, b, kk),
        );
    }
    // rc meshes, rcfx initialisers, rcef animation and draw records
    let (obj, mesh, fx, anim, keys) = (
        t.alloc(0x20),
        t.alloc(0x20),
        t.alloc(0x60),
        t.alloc(0x50),
        t.alloc(0x100),
    );
    for _ in 0..200 {
        t.fill(mesh, 0x20, &mut rng);
        t.put(obj + 0x14, mesh);
        t.same(
            "rcMeshPastIndices",
            |o| o.eax(0x0092_8270, &[obj]),
            |m| model::bridge::rc_mesh_past_indices(m, obj),
        );
        let x = rng.next();
        t.same(
            "rcbgIdentity",
            |o| o.eax(0x0092_02a0, &[x]),
            |_| model::bridge::rcbg_identity(x),
        );
        for (name, f) in [
            ("rcfxGravityInit", 0x0092_9570u32),
            ("rcfxParticleInit", 0x0092_88e0),
            ("rcfxNailInit", 0x0092_9100),
            ("rcfxWindInit", 0x0092_9850),
        ] {
            t.fill(fx, 0x60, &mut rng);
            t.same(
                name,
                |o| o.eax(f, &[fx]),
                |m| match f {
                    0x0092_9570 => model::bridge::rcfx_gravity_init(m, fx),
                    0x0092_88e0 => model::bridge::rcfx_particle_init(m, fx),
                    0x0092_9100 => model::bridge::rcfx_nail_init(m, fx),
                    _ => model::bridge::rcfx_wind_init(m, fx),
                },
            );
        }
        t.fill(keys, 0x100, &mut rng);
        t.put(keys + 6, i16::try_from(rng.range(-1, 10)).unwrap());
        for k in 0..16u32 {
            t.put(
                keys + 8 * k + 0x12,
                i16::try_from(rng.range(-5, 40)).unwrap(),
            );
        }
        t.put(anim + 4, keys);
        t.put(anim + 0x40, i16::try_from(rng.range(-10, 40)).unwrap());
        t.put(anim + 0x42, u16::try_from(rng.below(12)).unwrap());
        t.put(anim + 0x44, i16::try_from(rng.range(-3, 20)).unwrap());
        t.same(
            "rcefAnimeStep",
            |o| o.eax(0x0092_f400, &[anim]),
            |m| model::bridge::rcef_anime_step(m, anim),
        );
        let (tex, aa, bb) = (rng.next(), rng.next(), rng.next());
        t.same(
            "rcefSetupDraw",
            |o| o.eax(0x0092_f6b0, &[fx, tex, aa, bb]),
            |m| model::bridge::rcef_setup_draw(m, fx, tex, aa, bb),
        );
    }
    // rcui selection over the map system's layers
    let (sys, map, table, layers, work) = (
        t.alloc(0x10),
        t.alloc(0x10),
        t.alloc(0x10),
        t.alloc(0xc0 * 8),
        t.alloc(0x300),
    );
    t.set(rcui::WORK, work);
    for k in 0..60 {
        t.set(guide::YN_MAP_SYSTEM, if k % 7 == 0 { 0 } else { sys });
        t.put(sys, if k % 11 == 0 { 0 } else { map });
        t.put(map + 8, table);
        t.put(table + 4, rng.range(-1, 8));
        t.put(table + 8, layers);
        t.fill(layers, 0xc0 * 8, &mut rng);
        t.put(work + 0x28c, rng.below(4));
        t.put(work + 0x288, rng.below(8));
        let sys_ok = k % 7 != 0 && k % 11 != 0;
        let mode = Mem::read::<u32>(&t.b, work + 0x28c);
        if !sys_ok && (mode == 1 || mode == 2) {
            continue; // reads from address 0
        }
        t.same(
            "rcuiClearSelect",
            |o| o.eax(0x0093_4af0, &[]),
            rcui::bridge::rcui_clear_select,
        );
        t.same(
            "rcuiSetSelect",
            |o| o.eax(0x0093_4b50, &[]),
            rcui::bridge::rcui_set_select,
        );
    }
}
