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
use ffx_field::{
    camera, freecam, geom, guide, ke, model, nkz, packet, rcdt, rcui, scene, task, vu0, yi,
};
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

    /// An integer (or no) result: the original must not leave a float in
    /// `ST0`, which a port without a float result would drop (a void
    /// `Ch_CheckRadian` made every model invisible).
    fn eax(&mut self, f: u32, args: &[u32]) -> u32 {
        let r = self.call(f, args);
        assert!(
            r.st0.is_none(),
            "{f:#x} leaves {:?} in ST0: the port must return it",
            r.st0
        );
        r.eax
    }

    fn void(&mut self, f: u32, args: &[u32]) {
        self.eax(f, args);
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

/// Runs a Phyre/CRT routine for the free camera ports: `arg` in `ecx`
/// (thiscall) or on the stack.
fn phyre_call(m: &mut Oracle, f: u32, arg: u32, thiscall: bool) {
    let conv = if thiscall {
        Conv::Thiscall(arg)
    } else {
        Conv::Cdecl
    };
    let args: &[u32] = if thiscall { &[] } else { &[arg] };
    m.call(f, conv, args).unwrap();
}

/// `_CItan`'s import slot (MSVCR110), which the oracle does not stub.
const CITAN_SLOT: u32 = 0x00b0_c3a0;
/// `_CItan` (ST0 in and out) as the CPU's `fptan`: `fptan; fstp st0; ret`.
const CITAN_STUB: [u8; 5] = [0xd9, 0xf2, 0xdd, 0xd8, 0xc3];

impl Twin {
    /// Points `_CItan` at [`CITAN_STUB`] in code space, in both.
    fn install_tan(&mut self) {
        for o in [&mut self.a, &mut self.b] {
            let at = o.rebase(CODE_AREA + 0x100);
            o.write(at, &CITAN_STUB).unwrap();
            let slot = o.rebase(CITAN_SLOT);
            o.write(slot, &at.to_le_bytes()).unwrap();
        }
    }
}

/// The free camera's constructor, destructors and instance teardown (the
/// originals link an exception frame at `fs:[0]`; the Phyre camera's
/// perspective calls `_CItan`, patched to `fptan`).
#[test]
fn round7_free_camera_matches_the_original() {
    use freecam::bridge as fb;
    let Some(mut t) = Twin::image() else { return };
    t.install_tan();
    // operator delete records its pointer instead of freeing (the CRT heap
    // takes critical sections the oracle does not stub)
    let deleted = t.alloc(4);
    let mut stub = vec![0x8b, 0x44, 0x24, 0x04, 0xa3];
    stub.extend_from_slice(&deleted.to_le_bytes());
    stub.push(0xc3);
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(freecam::OPERATOR_DELETE);
        o.write(at, &stub).unwrap();
    }
    let mut rng = Xorshift(0x0007_0001);
    for k in 0..6u32 {
        let cam = t.alloc(0x344);
        t.fill(cam, 0x344, &mut rng);
        t.same(
            "FreeCamera::FreeCamera",
            |o| o.conv(0x0067_fe70, Conv::Thiscall(cam), &[]),
            |m| fb::free_camera_ctor(m, cam, phyre_call),
        );
        match k % 3 {
            0 => t.same(
                "FreeCamera::~FreeCamera",
                |o| {
                    o.conv(0x0067_ff10, Conv::Thiscall(cam), &[]);
                    0
                },
                |m| {
                    fb::free_camera_dtor(m, cam, phyre_call);
                    0
                },
            ),
            1 => {
                let flags = k & 1;
                t.same(
                    "FreeCamera scalar deleting destructor",
                    |o| o.conv(0x0067_ffb0, Conv::Thiscall(cam), &[flags]),
                    |m| fb::free_camera_delete(m, cam, flags, phyre_call),
                );
            }
            _ => {
                t.set(freecam::INSTANCE, cam);
                t.same(
                    "FreeCamera_destroyInstance",
                    |o| o.void(0x0068_0000, &[]),
                    |m| fb::free_camera_destroy_instance(m, phyre_call),
                );
                t.same(
                    "FreeCamera_destroyInstance (none)",
                    |o| o.void(0x0068_0000, &[]),
                    |m| fb::free_camera_destroy_instance(m, phyre_call),
                );
            }
        }
    }
}

/// A logging stand-in for an original function: appends `id` and its
/// `args` words to `log` (`[count]`, then 0x20 bytes an entry) and
/// returns the word at `rv`.
fn log_stub(log: u32, rv: u32, id: u32, args: u32) -> Vec<u8> {
    log_stub_from(log, rv, id, 0, args)
}

/// [`log_stub`] logging arguments `first..first + args` only (a stack
/// buffer's address differs between the original and the port).
fn log_stub_from(log: u32, rv: u32, id: u32, first: u32, args: u32) -> Vec<u8> {
    let mut c = vec![0x8b, 0x0d];
    c.extend_from_slice(&log.to_le_bytes());
    c.extend_from_slice(&[0xc1, 0xe1, 0x05, 0xc7, 0x81]);
    c.extend_from_slice(&(log + 4).to_le_bytes());
    c.extend_from_slice(&id.to_le_bytes());
    for i in 0..args {
        c.extend_from_slice(&[
            0x8b,
            0x44,
            0x24,
            u8::try_from(4 + 4 * (first + i)).unwrap(),
            0x89,
            0x81,
        ]);
        c.extend_from_slice(&(log + 8 + 4 * i).to_le_bytes());
    }
    c.extend_from_slice(&[0xff, 0x05]);
    c.extend_from_slice(&log.to_le_bytes());
    c.push(0xa1);
    c.extend_from_slice(&rv.to_le_bytes());
    c.push(0xc3);
    c
}

impl Twin {
    /// Replaces `target` (when given) with a jump to a [`log_stub`] placed
    /// at `CODE_AREA + at`, in both; returns the stub's address.
    fn stub(&mut self, target: Option<u32>, at: u32, code: &[u8]) -> u32 {
        let mut stub = 0;
        for o in [&mut self.a, &mut self.b] {
            stub = o.rebase(CODE_AREA + at);
            o.write(stub, code).unwrap();
            if let Some(f) = target {
                let f = o.rebase(f);
                let rel = stub.wrapping_sub(f + 5);
                let mut jmp = vec![0xe9];
                jmp.extend_from_slice(&rel.to_le_bytes());
                o.write(f, &jmp).unwrap();
            }
        }
        stub
    }
}

/// The map teardown: `rcbgFini` and everything under it, over synthetic
/// map systems, with the frees, VFX, camera and VIF calls logged.
#[test]
#[allow(clippy::too_many_lines)] // the tree, then one case after another
fn round7_map_teardown_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0007_0002);
    let (log, rv, rv_vfx) = (t.alloc(4 + 0x20 * 512), t.alloc(4), t.alloc(4));
    let mut at = 0x120;
    for (id, target, args) in [
        (1u32, model::RC_FREE, 1u32),
        (2, guide::VFX_GET_RENDER_DATA, 2),
        (3, guide::VFX_DESTROY, 2),
        (4, guide::MS_SET_CAMERA_MATRIX, 6),
        (5, guide::VIF0_RESET, 0),
    ] {
        let r = if id == 2 { rv_vfx } else { rv };
        let code = log_stub(log, r, id, args);
        t.stub(Some(target), at, &code);
        at += 0x80;
    }
    let dtor = t.stub(None, at, &log_stub(log, rv, 6, 1));
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    // a leaf pointer: random, sometimes null
    let leaf = |rng: &mut Xorshift| if rng.below(4) == 0 { 0 } else { rng.next() | 1 };
    for k in 0..60u32 {
        t.put(log, 0u32);
        let r = rng.next();
        t.put(rv, r);
        t.put(rv_vfx, rng.below(2));
        // models, transfers, layers, scenes
        let model = |t: &mut Twin, rng: &mut Xorshift| {
            let m = t.alloc(0x20);
            t.fill(m, 0x20, rng);
            let d = leaf(rng);
            t.put(m + 0x14, d);
            m
        };
        let trx = |t: &mut Twin, rng: &mut Xorshift| {
            let x = t.alloc(0x40);
            t.fill(x, 0x40, rng);
            let d = leaf(rng);
            t.put(x + 0x38, d);
            x
        };
        let scenes = |t: &mut Twin, rng: &mut Xorshift, n: u32| {
            let s = t.alloc(0x110 * n.max(1) as usize);
            t.fill(s, 0x110 * n.max(1) as usize, rng);
            for i in 0..n {
                let sc = s + 0x110 * i;
                let layers = rng.below(3);
                let l = t.alloc(0xc0 * layers.max(1) as usize);
                t.fill(l, 0xc0 * layers.max(1) as usize, rng);
                for j in 0..layers {
                    let ly = l + 0xc0 * j;
                    for off in [0xa0, 0xbc] {
                        let x = if rng.below(3) == 0 { 0 } else { trx(t, rng) };
                        t.put(ly + off, x);
                    }
                    let d = leaf(rng);
                    t.put(ly + 0x9c, d);
                }
                t.put(sc + 4, layers);
                t.put(
                    sc + 8,
                    if layers == 0 && rng.below(2) == 0 {
                        0
                    } else {
                        l
                    },
                );
                let objs = rng.below(3);
                let table = t.alloc(4 * objs.max(1) as usize);
                for j in 0..objs {
                    let o = t.alloc(8);
                    t.put(o + 4, if rng.below(3) == 0 { 0 } else { dtor });
                    t.put(table + 4 * j, o);
                }
                t.put(sc + 0xc, objs);
                t.put(sc + 0x10, table);
                let d = leaf(rng);
                t.put(sc + 0x14, d);
                let models = rng.below(4);
                t.put(sc + 0x18, u16::try_from(models).unwrap());
                for j in 0..models {
                    let m = if rng.below(4) == 0 { 0 } else { model(t, rng) };
                    t.put(sc + 0x1c + 4 * j, m);
                }
            }
            s
        };
        let n = rng.below(3);
        let sc = scenes(&mut t, &mut rng, n);
        let (sys, map, gd) = (t.alloc(0x20), t.alloc(0x10), t.alloc(0x80));
        t.fill(gd, 0x80, &mut rng);
        let gm = if rng.below(3) == 0 {
            0
        } else {
            model(&mut t, &mut rng)
        };
        let wm = if rng.below(3) == 0 {
            0
        } else {
            model(&mut t, &mut rng)
        };
        t.put(gd, gm);
        t.put(gd + 8, wm);
        t.put(map, n);
        t.put(map + 4, sc);
        let d = leaf(&mut rng);
        t.put(map + 0xc, d);
        t.put(sys, if rng.below(5) == 0 { 0 } else { map });
        t.put(sys + 4, if rng.below(4) == 0 { 0 } else { gd });
        for off in [0x10, 0x14, 0x18] {
            let c = if rng.below(3) == 0 {
                0
            } else {
                let c = t.alloc(8);
                let d = leaf(&mut rng);
                t.put(c, d);
                c
            };
            t.put(sys + off, c);
        }
        t.set(
            guide::YN_MAP_SYSTEM,
            if rng.below(8) == 0 { 0 } else { sys },
        );
        t.put(t.b.rebase(0x0230_fd20), rng.next());
        match k % 6 {
            0 => t.same(
                "rcbgFini",
                |o| o.eax(0x0092_1aa0, &[]),
                |m| guide::bridge::rcbg_fini(m, call),
            ),
            1 => {
                // without a map system the guard returns the caller's eax
                let none = guide::YN_MAP_SYSTEM.get(&t.b) == 0;
                t.same(
                    "Yn_MapSystemFini",
                    |o| {
                        if none {
                            o.void(0x0091_9a60, &[]);
                            0
                        } else {
                            o.eax(0x0091_9a60, &[])
                        }
                    },
                    |m| guide::bridge::yn_map_system_fini(m, call),
                );
            }
            2 => t.same(
                "rcbgPreFini",
                |o| o.eax(0x0092_16b0, &[]),
                |m| guide::bridge::rcbg_pre_fini(m, call),
            ),
            3 => {
                t.same(
                    "rcbgGuideFini",
                    |o| o.eax(0x0091_d3d0, &[gd]),
                    |m| guide::bridge::rcbg_guide_fini(m, gd, call),
                );
                t.same(
                    "rcbgGuideDelete",
                    |o| o.eax(0x0091_cef0, &[gd]),
                    |m| guide::bridge::rcbg_guide_delete(m, gd, call),
                );
            }
            4 => {
                for i in 0..n {
                    let s = sc + 0x110 * i;
                    t.same(
                        "rcbgSceneFini",
                        |o| o.eax(0x0092_0870, &[s]),
                        |m| scene::bridge::rcbg_scene_fini(m, s, call),
                    );
                }
            }
            _ => {
                let (c, m0, x) = (t.alloc(8), model(&mut t, &mut rng), trx(&mut t, &mut rng));
                let d = leaf(&mut rng);
                t.put(c, d);
                t.same(
                    "rcdtChunkFini",
                    |o| o.eax(0x0092_b7f0, &[c]),
                    |m| rcdt::bridge::rcdt_chunk_fini(m, c, call),
                );
                t.same(
                    "rcdtChunkDelete",
                    |o| o.eax(0x0092_b7d0, &[c]),
                    |m| rcdt::bridge::rcdt_chunk_delete(m, c, call),
                );
                t.same(
                    "rcbgModelFini",
                    |o| o.eax(0x0092_8240, &[m0]),
                    |m| model::bridge::rcbg_model_fini(m, m0, call),
                );
                t.same(
                    "rcbgModelDelete",
                    |o| o.eax(0x0092_8000, &[m0]),
                    |m| model::bridge::rcbg_model_delete(m, m0, call),
                );
                t.same(
                    "rcbgTrxFini",
                    |o| {
                        o.void(0x0092_d490, &[x]);
                        0
                    },
                    |m| {
                        model::bridge::rcbg_trx_fini(m, x, call);
                        0
                    },
                );
                let d = leaf(&mut rng);
                t.put(x + 0x38, d);
                t.same(
                    "rcbgTrxDelete",
                    |o| o.eax(0x0092_d450, &[x]),
                    |m| model::bridge::rcbg_trx_delete(m, x, call),
                );
                t.same(
                    "rcbgLayerAnimeFini",
                    |o| o.eax(0x0093_0830, &[x]),
                    |m| model::bridge::rcbg_layer_anime_fini(m, x, call),
                );
                if n > 0 {
                    let l = Mem::read::<u32>(&t.b, sc + 8);
                    if Mem::read::<u32>(&t.b, sc + 4) > 0 {
                        t.same(
                            "rcbgLayerFini",
                            |o| o.eax(0x0092_ac90, &[l]),
                            |m| scene::bridge::rcbg_layer_fini(m, l, call),
                        );
                    }
                }
            }
        }
    }
}

/// The `yi` window handlers, the quadword copies, parser helpers and the
/// small table entries.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round7_windows_copies_and_parser_match_the_original() {
    use yi::bridge as yb;
    type Handler = fn(&mut Oracle, u32) -> u32;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0007_0003);
    let handlers: [(u32, &str, Handler); 31] = [
        (0x0090_7cc0, "yiWindowGrowInit", yb::yi_window_grow_init),
        (0x0090_7cf0, "yiWindowGrowOpen", yb::yi_window_grow_open),
        (0x0090_6fc0, "yiWindowGrowClose", yb::yi_window_grow_close),
        (0x0090_7040, "yiWindowFinishAll", yb::yi_window_finish_all),
        (
            0x0090_7bf0,
            "yiWindowFinishAll_00907bf0",
            yb::yi_window_finish_all,
        ),
        (0x0090_7160, "yiWindowPopInit", yb::yi_window_pop_init),
        (0x0090_7190, "yiWindowPopOpen", yb::yi_window_pop_open),
        (0x0090_71c0, "yiWindowPopClose", yb::yi_window_pop_close),
        (0x0090_71f0, "yiWindowFinish_009071f0", yb::yi_window_finish),
        (
            0x0090_7450,
            "yiWindowSlideDownInit",
            yb::yi_window_slide_down_init,
        ),
        (
            0x0090_72e0,
            "yiWindowSlideDownOpen",
            yb::yi_window_slide_down_open,
        ),
        (
            0x0090_7330,
            "yiWindowSlideDownClose",
            yb::yi_window_slide_down_close,
        ),
        (0x0090_7370, "yiWindowFinish_00907370", yb::yi_window_finish),
        (
            0x0090_74b0,
            "yiWindowSlideUpInitHere",
            yb::yi_window_slide_up_init_here,
        ),
        (
            0x0090_7690,
            "yiWindowSlideUpInit",
            yb::yi_window_slide_up_init,
        ),
        (
            0x0090_7520,
            "yiWindowSlideUpOpen",
            yb::yi_window_slide_up_open,
        ),
        (
            0x0090_7570,
            "yiWindowSlideUpClose",
            yb::yi_window_slide_up_close,
        ),
        (0x0090_75b0, "yiWindowFinish_009075b0", yb::yi_window_finish),
        (
            0x0090_76f0,
            "yiWindowSlideRightInitHere",
            yb::yi_window_slide_right_init_here,
        ),
        (
            0x0090_78c0,
            "yiWindowSlideRightInit",
            yb::yi_window_slide_right_init,
        ),
        (
            0x0090_7750,
            "yiWindowSlideRightOpen",
            yb::yi_window_slide_right_open,
        ),
        (
            0x0090_77a0,
            "yiWindowSlideRightClose",
            yb::yi_window_slide_right_close,
        ),
        (0x0090_77e0, "yiWindowFinish_009077e0", yb::yi_window_finish),
        (
            0x0090_7920,
            "yiWindowSlideLeftInitHere",
            yb::yi_window_slide_left_init_here,
        ),
        (
            0x0090_7b00,
            "yiWindowSlideLeftInit",
            yb::yi_window_slide_left_init,
        ),
        (
            0x0090_7990,
            "yiWindowSlideLeftOpen",
            yb::yi_window_slide_left_open,
        ),
        (
            0x0090_79e0,
            "yiWindowSlideLeftClose",
            yb::yi_window_slide_left_close,
        ),
        (0x0090_7a20, "yiWindowFinish_00907a20", yb::yi_window_finish),
        (0x0090_7b60, "yiWindowBarInit", yb::yi_window_bar_init),
        (0x0090_7ba0, "yiWindowBarOpen", yb::yi_window_bar_open),
        (0x0090_7bd0, "yiWindowBarClose", yb::yi_window_bar_close),
    ];
    let w = t.alloc(0x80);
    for k in 0..40 {
        for (f, name, h) in handlers {
            t.fill(w, 0x80, &mut rng);
            finite_floats(&mut t, w + 0x18, 0x44 / 4, &mut rng);
            // screen-sized coordinates now and then
            if k % 2 == 0 {
                for off in (0x18..0x58).step_by(4) {
                    #[allow(clippy::cast_precision_loss)]
                    let v = (rng.range(-64, 600) as f32 / 4.0).to_bits();
                    t.put(w + off, v);
                }
            }
            t.put(w + 6, i16::try_from(rng.range(-0x400, 0x1400)).unwrap());
            t.put(w + 0xc, i16::try_from(rng.range(-2, 8)).unwrap());
            t.put(w + 0xe, i16::try_from(rng.range(0, 8)).unwrap());
            t.put(w + 0x10, i16::try_from(rng.range(-0x200, 0x200)).unwrap());
            t.same(name, |o| o.eax(f, &[w]), |m| h(m, w));
        }
    }
    // quadword copies through vf31
    let (a, b) = (t.alloc(0x20), t.alloc(0x20));
    for f in [
        0x0091_cb40u32,
        0x0091_cbd0,
        0x0091_f710,
        0x0092_2510,
        0x0092_4830,
        0x0092_4f90,
        0x0092_50a0,
        0x0092_73b0,
        0x0092_7a90,
        0x0092_8eb0,
        0x0092_96a0,
        0x0092_d6f0,
        0x0092_f850,
    ] {
        for (d, s) in [(a, b), (a, a), (a, a + 8), (a + 8, a)] {
            t.fill(a, 0x20, &mut rng);
            t.fill(b, 0x20, &mut rng);
            t.same(
                "rcCopyQword",
                |o| o.eax(f, &[d, s]),
                |m| vu0::bridge::rc_copy_qword(m, d, s),
            );
        }
    }
    // parser helpers
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
        t.put(p + 4, start + rng.below(0x70));
        t.put(p + 8, rng.below(3));
        t.same(
            "rcdtParseTokenLen",
            |o| o.eax(0x0092_bf30, &[p]),
            |m| rcdt::bridge::rcdt_parse_token_len(m, p),
        );
        t.same(
            "rcdtParseTokenLenText",
            |o| o.eax(0x0092_be10, &[p]),
            |m| rcdt::bridge::rcdt_parse_token_len_text(m, p),
        );
        t.same(
            "rcdtParseNextToken",
            |o| o.eax(0x0092_ba30, &[p]),
            |m| rcdt::bridge::rcdt_parse_next_token(m, p),
        );
        let ch = [u32::from(b','), u32::from(b'x'), 0x1ff, 0x41][rng.below(4) as usize];
        t.same(
            "rcdtParseSeekChar",
            |o| o.eax(0x0092_b990, &[p, ch]),
            |m| {
                rcdt::bridge::rcdt_parse_seek_char(
                    m,
                    p,
                    u8::try_from(ch & 0xff).unwrap().cast_signed(),
                )
            },
        );
    }
    // table entries and small helpers
    let (obj, rec, ctx, voice, cmd) = (
        t.alloc(0x40),
        t.alloc(0x40),
        t.alloc(0x230),
        t.alloc(0xd0),
        t.alloc(8),
    );
    let hook = t.b.rebase(CODE_AREA); // SEQ_STUB: counts calls in rec+0x2c
    t.install_round4_code();
    for k in 0..40u32 {
        t.fill(obj, 0x40, &mut rng);
        t.same(
            "rcbgEmbeddedTrxGetCnt",
            |o| o.eax(0x0092_2140, &[obj]),
            |m| model::bridge::rcbg_embedded_trx_get_cnt(m, obj),
        );
        t.same(
            "Yn_EmbeddedTrxGetCnt",
            |o| o.eax(0x0091_9bc0, &[obj]),
            |m| model::bridge::rcbg_embedded_trx_get_cnt(m, obj),
        );
        let s = if k % 3 == 0 { 0 } else { rng.next() };
        t.same(
            "rcefSetStream",
            |o| o.eax(0x0092_5710, &[s]),
            |m| model::bridge::rcef_set_stream(m, s),
        );
        t.fill(rec, 0x40, &mut rng);
        t.put(obj + 0x14, rec);
        t.put(rec + 0xc, if k % 2 == 0 { 0 } else { hook });
        t.same(
            "rcbgRunRecordHook",
            |o| o.eax(0x0091_fd60, &[obj]),
            |m| {
                model::bridge::rcbg_run_record_hook(m, obj, |m, f, r| {
                    m.call(f, Conv::Cdecl, &[r]).unwrap().eax
                })
            },
        );
        t.fill(cmd, 8, &mut rng);
        t.put(ctx + 0x220, voice);
        t.same(
            "SqsCmdSetC2",
            |o| o.eax(0x0081_c140, &[ctx, cmd]),
            |m| nkz::bridge::sqs_cmd_set_c2(m, ctx, cmd),
        );
    }
}

/// The map render file reader: objects, cameras, textures (both upload
/// paths), CLUTs, fog, packet lists, animation tracks, matrices and
/// unknown kinds, over synthetic files; the GS, graphics-object and
/// rotation calls logged.
#[test]
#[allow(clippy::too_many_lines)] // the file, then its records
fn round8_map_file_reader_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0008_0001);
    let (log, rv) = (t.alloc(4 + 0x20 * 1024), t.alloc(4));
    let mut at = 0x120;
    for (id, target, first, args) in [
        (1u32, yi::SG_MFIFO_SYNC, 0u32, 0u32),
        (2, yi::GS_SYNC_PATH, 0, 2),
        (3, yi::GS_SET_DEF_LOAD_IMAGE, 1, 7),
        (4, yi::GS_EXEC_LOAD_IMAGE, 1, 1),
        (5, yi::FLUSH_CACHE, 0, 1),
        (6, yi::GRAPHIC_OBJECT_SET_TRANSLATION, 0, 2),
        (7, yi::GRAPHIC_OBJECT_SET_ROTATION, 0, 3),
        (8, yi::VU0_ROT_MATRIX_X, 0, 3),
        (9, yi::VU0_ROT_MATRIX_Y, 0, 3),
        (10, yi::YI_SWIZZLE_IMAGE, 0, 4),
    ] {
        let code = log_stub_from(log, rv, id, first, args);
        t.stub(Some(target), at, &code);
        at += u32::try_from(code.len()).unwrap().next_multiple_of(0x10);
        assert!(at <= 0x450, "the stubs outgrow pppFpLoop's body");
    }
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let scene = t.alloc(0xc_0000);
    let image = t.alloc(0x60);
    t.fill(scene, 0xc_0000, &mut rng);
    for k in 0..30u32 {
        t.put(log, 0u32);
        t.put(rv, rng.next());
        for (off, n) in [
            (0x6_8788u32, 3u32),
            (0x6_e18c, 3),
            (0x9_c198, 3),
            (0x9_d4c0, 8),
            (0x9_d9c4, 4),
        ] {
            t.put(scene + off, rng.below(n));
        }
        for set in 0..4u32 {
            t.put(scene + 0x9_d9e4 + set * 0x720, rng.below(6));
        }
        finite_floats(&mut t, scene + 0xb_a400, 2, &mut rng);
        // the file
        let file = t.alloc(0x6_0000);
        t.fill(file, 0x200, &mut rng);
        t.put(file, if k == 7 { 0x0021_4366u32 } else { 0x0021_4365 });
        t.put(file + 4, 0x110u32);
        let mut rec = file + 0x40;
        let mut count = 0u32;
        for r in 0..=rng.below(9) {
            let kind = if r == 0 {
                0
            } else {
                [1, 1, 2, 2, 3, 4, 5, 6, 7, 8, 9][rng.below(11) as usize]
            };
            t.fill(rec, 0x80, &mut rng);
            t.put(rec, kind);
            let data = rec + 0x40;
            let len = match kind {
                // float fields without signalling NaNs (the emulator keeps
                // them as fld/fstp would not)
                0 => {
                    t.floats(data, 0x10, &mut rng);
                    0x40 + 0x10 * rng.below(3)
                }
                1 => {
                    let keys = rng.below(4);
                    t.floats(data, 0x10 + 4 * keys, &mut rng);
                    t.put(data + 0xc, if rng.below(6) == 0 { keys + 1 } else { keys });
                    0x40 + 0x10 * keys
                }
                2 => {
                    let q = 1 + rng.below(4);
                    t.fill(data, 0x3c + 0x10 * q as usize, &mut rng);
                    let vram = rng.below(2) == 0;
                    t.put(
                        data + 4,
                        if vram {
                            0x3c00 + rng.below(0x10)
                        } else {
                            rng.below(0x3c00)
                        },
                    );
                    t.put(
                        data + 0xc,
                        [0u32, 0x13, 0x14, 0x14, 2][rng.below(5) as usize],
                    );
                    t.put(data + 0x10, rng.below(5));
                    t.put(data + 0x14, rng.below(5));
                    0x3c + 0x10 * q
                }
                3 => {
                    t.put(rec + 0x24, rng.below(6));
                    t.fill(data, 0x1_2000, &mut rng);
                    0x1_2000
                }
                4 => {
                    finite_floats(&mut t, data, 0x10, &mut rng);
                    0x40
                }
                5 | 8 => {
                    t.put(rec + 0x20, rng.below(2));
                    t.put(rec + 0x24, rng.below(8));
                    0x80 * (1 + rng.below(3))
                }
                6 => {
                    t.fill(data, 0x40, &mut rng);
                    t.put(data, rng.below(4));
                    t.put(data + 4, if rng.below(2) == 0 { 0 } else { rng.next() });
                    t.put(data + 0x18, rng.below(8));
                    t.put(data + 0x24, if rng.below(2) == 0 { 0 } else { rng.next() });
                    0x40 + 0x10 * rng.below(4)
                }
                7 => {
                    t.floats(data, 0x50, &mut rng);
                    0x140
                }
                _ => 0x40,
            };
            t.put(rec + 4, len);
            count += 1;
            rec = if kind > 8 { data } else { data + len };
        }
        t.put(file + 0xc, if k == 9 { 0 } else { count });
        t.same(
            "yiMapFileRead",
            |o| o.eax(0x0090_f900, &[scene, file]),
            |m| yi::bridge::yi_map_file_read(m, scene, file, image, call),
        );
        // the pieces on their own
        let obj = scene + 0x6_878c;
        t.same(
            "yiObjInit",
            |o| o.void(0x0090_ff40, &[obj]),
            |m| yi::bridge::yi_obj_init(m, obj),
        );
        let (entry, data) = (t.alloc(0x24), t.alloc(0x400));
        t.fill(entry, 0x24, &mut rng);
        t.fill(data, 0x400, &mut rng);
        t.put(entry + 4, 0x3c00 + rng.below(0x10));
        t.put(entry + 0xc, [0u32, 0x13, 0x14, 7][rng.below(4) as usize]);
        t.put(entry + 0x10, rng.range(-3, 8));
        t.put(entry + 0x14, rng.range(-3, 8));
        let swz = Mem::read::<u32>(&t.b, entry + 0xc) == 0x13;
        t.same(
            "yiLoadVramImage",
            |o| {
                let r = o.eax(0x0090_e5d0, &[scene, entry, data]);
                if swz { r } else { 0 }
            },
            |m| yi::bridge::yi_load_vram_image(m, scene, entry, data, call),
        );
    }
}

/// The window draw handlers, their `TO` drawing calls logged.
#[test]
fn round8_window_draw_matches_the_original() {
    use yi::bridge as yb;
    type Draw = fn(&mut Oracle, u32, &mut dyn FnMut(&mut Oracle, u32, &[u32]) -> u32) -> u32;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0008_0002);
    let (log, rv, rv_win, rv_shift) = (t.alloc(4 + 0x20 * 64), t.alloc(4), t.alloc(4), t.alloc(4));
    let mut at = 0x120;
    for (id, target, args, r) in [
        (1u32, yi::TO_DRAW_SCISSOR_XYWH, 4u32, rv),
        (2, yi::TO_DRAW_PLATE, 4, rv),
        (3, yi::TO_DRAW_SCISSOR, 4, rv),
        (4, yi::TO_MAKE_PKT_SCISSOR, 4, rv),
        (5, yi::TO_DRAW_MENU_PLATE, 5, rv),
        (6, yi::WINDOW_MENU_SHIFT, 0, rv_shift),
        (7, yi::TO_GET_WINDOW, 1, rv_win),
        (8, yi::TO_DRAW_SPRITE, 7, rv),
        (9, yi::TO_MKP_SHAPE_XY, 3, rv),
    ] {
        let code = log_stub(log, r, id, args);
        t.stub(Some(target), at, &code);
        at += u32::try_from(code.len()).unwrap().next_multiple_of(0x10);
        assert!(at <= 0x450, "the stubs outgrow pppFpLoop's body");
    }
    let draws: [(u32, &str, Draw); 7] = [
        (0x0090_7390, "yiWindowDrawPlate", |m, w, mut c| {
            yb::yi_window_draw_plate(m, w, &mut c)
        }),
        (0x0090_75d0, "yiWindowDrawPlate_009075d0", |m, w, mut c| {
            yb::yi_window_draw_plate(m, w, &mut c)
        }),
        (0x0090_7800, "yiWindowDrawPlate_00907800", |m, w, mut c| {
            yb::yi_window_draw_plate(m, w, &mut c)
        }),
        (0x0090_7a40, "yiWindowDrawPlate_00907a40", |m, w, mut c| {
            yb::yi_window_draw_plate(m, w, &mut c)
        }),
        (0x0090_7060, "yiWindowDrawMenu", |m, w, mut c| {
            yb::yi_window_draw_menu(m, w, &mut c)
        }),
        (0x0090_7210, "yiWindowDrawPop", |m, w, mut c| {
            yb::yi_window_draw_pop(m, w, &mut c)
        }),
        (0x0090_7c10, "yiWindowDrawBar", |m, w, mut c| {
            yb::yi_window_draw_bar(m, w, &mut c)
        }),
    ];
    let (w, p) = (t.alloc(0x80), t.alloc(0x80));
    let bar = t.b.rebase(yi::WINDOW_BAR_LAYOUT);
    for k in 0..60u32 {
        for (f, name, d) in draws {
            t.put(log, 0u32);
            t.put(rv_shift, rng.below(2));
            t.put(rv_win, p);
            for v in [w, p] {
                for off in (0..0x80).step_by(4) {
                    #[allow(clippy::cast_precision_loss)]
                    let x = (rng.range(-200, 2400) as f32 / 4.0).to_bits();
                    t.put(v + off, x);
                }
            }
            if k % 3 == 0 {
                t.put(w + 0x28, 65.0f32.to_bits());
                t.put(w + 0x2c, 290.0f32.to_bits());
                t.put(p + 0x28, 38.0f32.to_bits());
                t.put(
                    p + 0x2c,
                    if k % 2 == 0 { 115.0f32 } else { 114.0 }.to_bits(),
                );
            }
            t.set(yi::CURRENT_MAP, if k % 4 == 0 { 0 } else { 0x15b });
            t.fill(bar, 0x1a, &mut rng);
            let call =
                &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
            t.same(name, |o| o.eax(f, &[w]), |m| d(m, w, call));
        }
    }
}

/// A code cave for round 8's stand-ins: the body of `0x00764de0` (2750
/// bytes, a Ke effect builder these tests never run).
const CAVE: u32 = 0x0076_4de0;
/// Its length.
const CAVE_LEN: u32 = 2750;

impl Twin {
    /// Places each `(target, code)` stand-in in [`CAVE`] in turn (16-byte
    /// aligned) and points `target` (when given) at it with a `jmp`, in
    /// both. Returns the stand-ins' addresses.
    fn cave(&mut self, list: &[(Option<u32>, Vec<u8>)]) -> Vec<u32> {
        self.cave_from(0, list).0
    }

    /// [`Twin::cave`] from `start` bytes into [`CAVE`]; also returns where
    /// the next stand-in may go.
    fn cave_from(&mut self, start: u32, list: &[(Option<u32>, Vec<u8>)]) -> (Vec<u32>, u32) {
        let mut at = start;
        let mut out = vec![];
        for (target, code) in list {
            let mut stub = 0;
            for o in [&mut self.a, &mut self.b] {
                stub = o.rebase(CAVE + at);
                o.write(stub, code).unwrap();
                if let Some(f) = *target {
                    let f = o.rebase(f);
                    let rel = stub.wrapping_sub(f + 5);
                    let mut jmp = vec![0xe9];
                    jmp.extend_from_slice(&rel.to_le_bytes());
                    o.write(f, &jmp).unwrap();
                }
            }
            out.push(stub);
            at += u32::try_from(code.len()).unwrap().next_multiple_of(0x10);
            assert!(at <= CAVE_LEN, "the stand-ins outgrow the cave");
        }
        (out, at)
    }
}

/// `user_malloc(size)` as a bump allocator over `next` (the size kept
/// just below each 16-byte aligned block, for [`SIZE_STUB`]).
fn bump_stub(next: u32) -> Vec<u8> {
    let mut c = vec![0xa1];
    c.extend_from_slice(&next.to_le_bytes());
    c.extend_from_slice(&[
        0x8b, 0x4c, 0x24, 0x04, 0x89, 0x48, 0x0c, 0x83, 0xc0, 0x10, 0x83, 0xc1, 0x0f, 0x83, 0xe1,
        0xf0, 0x01, 0xc1, 0x89, 0x0d,
    ]);
    c.extend_from_slice(&next.to_le_bytes());
    c.push(0xc3);
    c
}

/// `Sg_MemGetSize(p)`: the size [`bump_stub`] keeps at `p - 4`.
const SIZE_STUB: [u8; 8] = [0x8b, 0x44, 0x24, 0x04, 0x8b, 0x40, 0xfc, 0xc3];

/// A test map's loading environment: the allocator, frees and the engine
/// calls the loaders make, logged.
struct LoadEnv {
    log: u32,
    rv: u32,
    /// Where more cave stand-ins may go (round 9).
    cave_end: u32,
}

impl LoadEnv {
    fn install(t: &mut Twin, rng: &mut Xorshift) -> Self {
        let (log, rv, next) = (t.alloc(4 + 0x20 * 4096), t.alloc(4), t.alloc(4));
        let arena = t.alloc(0x40_0000);
        t.put(next, arena);
        let _ = rng;
        let mut list: Vec<(Option<u32>, Vec<u8>)> = vec![
            (Some(model::USER_MALLOC), bump_stub(next)),
            (Some(model::SG_MEM_GET_SIZE), SIZE_STUB.to_vec()),
        ];
        for (id, (target, args)) in (1u32..).zip([
            (model::USER_FREE, 1u32),
            (model::SG_MEM_SET_ALIGN, 1),
            (scene::RCBG_TRX_DRAW, 1),
            (scene::SCENE_SET_MATRIX, 1),
            (guide::GUIDE_SET_MODEL_SUB, 1),
            (guide::GUIDE_SET_MODEL_FROM_ID_MAP, 1),
            (guide::GRAPHIC_SCENE_NO, 1),
            (guide::GRAPHIC_SET_2D_START_LAYER, 1),
            (guide::RCBG_DEBUG_INIT, 0),
            (guide::VFX_GET_RENDER_DATA, 2),
            (guide::VFX_DESTROY, 2),
            (guide::MS_SET_CAMERA_MATRIX, 6),
            (guide::VIF0_RESET, 0),
            (yi::SG_GET_KEEP_FPS, 0),
            (yi::ENABLE_KEYFRAME_ANIM, 1),
            (yi::RENDER_SETUP_VU1, 1),
            (yi::TK_RESTART_SYNC_H, 1),
            (yi::TK_GET_SYNC_H, 1),
            (yi::TK_SYNC_H_WAIT, 1),
            (yi::SG_ID_FIXUP_AND_SET, 1),
            (yi::PPP_FP_FREE, 0),
            (yi::GRAPHIC_CLUSTER_DELAY_RELEASE, 0),
            (yi::CACHE_CLEAR, 0),
            (yi::GRAPHIC_FIELD_MAP_SHUTDOWN, 0),
            (yi::GRAPHIC_FIELD_MAP_UNLOAD, 0),
            (yi::GRAPHIC_VFX_UNLOAD, 0),
            (yi::MAP_SECTION2_HANDLER, 1),
            (yi::GS_SYNC_PATH, 2),
            (yi::SG_MFIFO_SYNC, 0),
            (guide::EVENT_MAP_NO, 0),
            (guide::SAVE_WALK_MAP, 0),
            (yi::PPP_SET_FP_PDT, 1),
            (yi::MAP_CIRCLE_ON, 0),
        ]) {
            list.push((Some(target), log_stub(log, rv, id, args)));
        }
        let (_, cave_end) = t.cave_from(0, &list);
        t.install_round4_code();
        Self { log, rv, cave_end }
    }

    fn reset(&self, t: &mut Twin, rng: &mut Xorshift) {
        t.put(self.log, 0u32);
        t.put(self.rv, rng.below(2));
    }
}

/// Writes a chunk body for a parser: the bytes, then a chunk and a
/// parser over them (the caller's `chunk`/`parse` buffers).
fn parser_over(t: &mut Twin, chunk: u32, parse: u32, data: u32) {
    t.same(
        "rcdtChunkSetBin",
        |o| o.void(RCDT_CHUNK_SET_BIN, &[chunk, data]),
        |m| rcdt::bridge::rcdt_chunk_set_bin(m, chunk, data),
    );
    t.same(
        "rcdtParseInit",
        |o| o.void(RCDT_PARSE_INIT, &[parse]),
        |m| rcdt::bridge::rcdt_parse_init(m, parse),
    );
    t.same(
        "rcdtParseSet",
        |o| o.void(RCDT_PARSE_SET, &[parse, chunk]),
        |m| rcdt::bridge::rcdt_parse_set(m, parse, chunk),
    );
}

/// Random bytes.
fn bytes(rng: &mut Xorshift, n: usize) -> Vec<u8> {
    (0..n).map(|_| rng.next().to_le_bytes()[0]).collect()
}

/// A transfer record as `rcbgTrxLoad` reads it: three 8-byte words, a
/// count and an entry size (small), the entries.
fn trx_record(rng: &mut Xorshift) -> Vec<u8> {
    let mut v = bytes(rng, 0x18);
    let (n, size) = (rng.below(4), 4 * rng.below(0x10));
    v.extend_from_slice(&n.to_le_bytes());
    v.extend_from_slice(&size.to_le_bytes());
    v.extend(bytes(rng, (n * size) as usize));
    v
}

/// A model record (`rcbgModelLoad`).
fn model_record(rng: &mut Xorshift) -> Vec<u8> {
    let mut v = bytes(rng, 4);
    let size = rng.below(0x40);
    v.extend_from_slice(&size.to_le_bytes());
    v.extend(bytes(rng, size as usize + 0x70 + 0x20 + 0x10 + 0x80));
    v
}

/// A texture transfer record (`rcbgTrxTexLoad`).
fn trx_tex_record(rng: &mut Xorshift) -> Vec<u8> {
    let mut v = bytes(rng, 0x10);
    v.extend(trx_record(rng));
    v.extend(trx_record(rng));
    v
}

/// A layer record (`rcbgLayerLoad`): 0x90 bytes (size words +4/+8
/// small, flags random), frames, the transfers.
fn layer_record(rng: &mut Xorshift) -> Vec<u8> {
    let mut v = bytes(rng, 0x90);
    for off in [4usize, 8] {
        v[off..off + 4].copy_from_slice(&rng.below(0x300).to_le_bytes());
    }
    // image layers (flag 0x10) have their CLUT transfer (+0xa0)
    let frames = if v[0] & 0x10 != 0 {
        1 + rng.below(3)
    } else {
        rng.below(4)
    };
    v.extend_from_slice(&frames.to_le_bytes());
    v.extend(bytes(rng, 0xc));
    if frames > 0 {
        v.extend(bytes(rng, (frames * 4).next_multiple_of(16) as usize));
        v.extend(trx_record(rng));
    }
    v.extend(bytes(rng, 0x10));
    v.extend(trx_record(rng));
    v
}

/// A scene record (`rcbgSceneLoad`).
fn scene_record(rng: &mut Xorshift) -> Vec<u8> {
    let n = rng.below(4);
    let mut v = n.to_le_bytes().to_vec();
    v.extend(bytes(rng, 0xc));
    for _ in 0..n {
        v.extend(layer_record(rng));
    }
    v.extend(bytes(rng, 0x38));
    v
}

/// The background loaders: allocation, models, transfers, layers,
/// scenes, the map (`rcbgPre*`), `rcbgInit`, the guide, a whole `RCBG`
/// file (`rcbgLoadBin`, `Yn_PreBgSetData`), with the allocator a bump
/// allocator and the engine calls logged.
#[test]
#[allow(clippy::too_many_lines)] // one loader after another
fn round8_background_loaders_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0008_0003);
    let env = LoadEnv::install(&mut t, &mut rng);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (chunk, parse, data) = (t.alloc(8), t.alloc(0x90), t.alloc(0x4000));
    for k in 0..40u32 {
        env.reset(&mut t, &mut rng);
        // allocation
        let n = rng.below(0x100);
        t.same(
            "rcAlloc",
            |o| o.eax(0x0092_58a0, &[n, 0, 0]),
            |m| model::bridge::rc_alloc(m, call, n),
        );
        let (a, b) = (rng.below(8), rng.below(0x40));
        t.same(
            "rcCalloc",
            |o| o.eax(0x0092_58b0, &[a, b, 0, 0]),
            |m| model::bridge::rc_calloc(m, call, a, b),
        );
        let p = if k % 3 == 0 {
            0
        } else {
            // the same allocation on both sides
            model::bridge::rc_alloc(&mut t.a, call, 0x20);
            let p = model::bridge::rc_alloc(&mut t.b, call, 0x20);
            t.fill(p, 0x20, &mut rng);
            p
        };
        let size = [0x20u32, 0x10, 0x40, 0][rng.below(4) as usize];
        t.same(
            "rcRealloc",
            |o| o.eax(0x0092_58f0, &[p, size, 0, 0]),
            |m| model::bridge::rc_realloc(m, call, p, size),
        );
        t.same(
            "yiMalloc",
            |o| o.eax(0x0090_bbd0, &[n]),
            |m| model::bridge::yi_malloc(m, call, n),
        );
        t.same(
            "yiMalloc64",
            |o| o.eax(0x0090_bbe0, &[n]),
            |m| model::bridge::yi_malloc64(m, call, n),
        );
        // objects
        t.same(
            "rcbgTrxNew",
            |o| o.eax(0x0092_d580, &[]),
            |m| model::bridge::rcbg_trx_new(m, call),
        );
        t.same(
            "rcbgTrxTexNew",
            |o| o.eax(0x0092_21b0, &[]),
            |m| model::bridge::rcbg_trx_tex_new(m, call),
        );
        t.same(
            "rcbgFpTrxTexNew",
            |o| o.eax(0x0092_20c0, &[]),
            |m| model::bridge::rcbg_fp_trx_tex_new(m, call),
        );
        t.same(
            "rcbgModelNew",
            |o| o.eax(0x0092_83b0, &[]),
            |m| model::bridge::rcbg_model_new(m, call),
        );
        t.same(
            "rcbgGuideNew",
            |o| o.eax(0x0091_d4e0, &[]),
            |m| guide::bridge::rcbg_guide_new(m, call),
        );
        // a model, a texture transfer and a layer from parser data
        let rec = model_record(&mut rng);
        t.write(data, &rec);
        parser_over(&mut t, chunk, parse, data);
        let m0 = model::bridge::rc_alloc(&mut t.b, call, 0x140);
        model::bridge::rc_alloc(&mut t.a, call, 0x140);
        t.same(
            "rcbgModelInit",
            |o| o.void(0x0092_8320, &[m0]),
            |m| model::bridge::rcbg_model_init(m, m0),
        );
        t.same(
            "rcbgModelLoad",
            |o| o.void(0x0092_b2f0, &[m0, parse]),
            |m| model::bridge::rcbg_model_load(m, m0, parse, call),
        );
        let tx = model::bridge::rc_alloc(&mut t.b, call, 0xa0);
        model::bridge::rc_alloc(&mut t.a, call, 0xa0);
        t.same(
            "rcbgTrxTexInit",
            |o| o.void(0x0092_2160, &[tx]),
            |m| model::bridge::rcbg_trx_tex_init(m, tx),
        );
        let rec = trx_tex_record(&mut rng);
        t.write(data + 0x400, &rec);
        parser_over(&mut t, chunk, parse, data + 0x400);
        t.same(
            "rcbgTrxTexLoad",
            |o| o.void(0x0092_b780, &[tx, parse]),
            |m| model::bridge::rcbg_trx_tex_load(m, tx, parse),
        );
        for off in [0x20u32, 0x60] {
            t.put(
                tx + off + 0x3c,
                if rng.below(2) == 0 {
                    u32::MAX
                } else {
                    rng.below(4)
                },
            );
        }
        t.same(
            "rcbgTrxTexDraw",
            |o| o.void(0x0092_2100, &[tx]),
            |m| scene::bridge::rcbg_trx_tex_draw(m, tx, call),
        );
        t.same(
            "rcbgTrxTexFini",
            |o| o.void(0x0092_2120, &[tx]),
            |m| model::bridge::rcbg_trx_tex_fini(m, tx, call),
        );
        t.same(
            "rcbgTrxTexDelete",
            |o| o.eax(0x0092_20e0, &[tx]),
            |m| model::bridge::rcbg_trx_tex_delete(m, tx, call),
        );
        // a scene with layers
        let sc = model::bridge::rc_alloc(&mut t.b, call, 0x110);
        model::bridge::rc_alloc(&mut t.a, call, 0x110);
        t.same(
            "rcbgSceneInit",
            |o| o.void(0x0092_09e0, &[sc]),
            |m| scene::bridge::rcbg_scene_init(m, sc),
        );
        let rec = scene_record(&mut rng);
        assert!(rec.len() < 0x800);
        t.write(data + 0x800, &rec);
        parser_over(&mut t, chunk, parse, data + 0x800);
        t.same(
            "rcbgSceneLoad",
            |o| o.void(0x0092_b5e0, &[sc, parse]),
            |m| scene::bridge::rcbg_scene_load(m, sc, parse, call),
        );
        let rec = model_record(&mut rng);
        t.write(data + 0x1000, &rec);
        parser_over(&mut t, chunk, parse, data + 0x1000);
        t.same(
            "rcbgSceneModelLoad",
            |o| o.void(0x0092_b6b0, &[sc, parse]),
            |m| scene::bridge::rcbg_scene_model_load(m, sc, parse, call),
        );
        let rec = trx_record(&mut rng);
        t.write(data + 0x2000, &rec);
        parser_over(&mut t, chunk, parse, data + 0x2000);
        t.same(
            "rcbgSceneTrxLoad",
            |o| o.void(0x0092_b6e0, &[sc, parse]),
            |m| scene::bridge::rcbg_scene_trx_load(m, sc, parse, call),
        );
        t.same(
            "rcbgSceneChkLayers",
            |o| o.void(0x0092_b400, &[sc]),
            |m| scene::bridge::rcbg_scene_chk_layers(m, sc, call),
        );
        let an = t.alloc(0x10);
        t.put(an + 8, rng.below(4));
        let cnt = rng.range(0, 6);
        t.same(
            "rcbgAnimeSetDataCnt",
            |o| o.void(0x0093_08d0, &[an, cnt.cast_unsigned()]),
            |m| model::bridge::rcbg_anime_set_data_cnt(m, an, cnt, call),
        );
        // the map system
        if k % 4 == 0 {
            t.set(guide::YN_MAP_SYSTEM, 0);
        }
        t.same(
            "rcbgInit",
            |o| o.void(0x0092_1ce0, &[]),
            |m| guide::bridge::rcbg_init(m, call),
        );
        t.same(
            "rcbgPreInit",
            |o| o.void(0x0092_1770, &[]),
            |m| guide::bridge::rcbg_pre_init(m, call),
        );
        t.put(data + 0x3000, rng.below(3));
        parser_over(&mut t, chunk, parse, data + 0x3000);
        t.same(
            "rcbgPreLoad",
            |o| o.void(0x0092_b390, &[parse]),
            |m| guide::bridge::rcbg_pre_load(m, parse, call),
        );
        // an object without a destructor (the scene teardown calls +4)
        let obj = t.alloc(8);
        t.same(
            "rcbgPreAddObject",
            |o| o.void(0x0092_18c0, &[obj]),
            |m| guide::bridge::rcbg_pre_add_object(m, obj, call),
        );
        t.same(
            "rcbgPreDummyInit",
            |o| o.void(0x0092_1640, &[]),
            |m| guide::bridge::rcbg_pre_dummy_init(m, call),
        );
        let no = rng.range(-1, 3);
        t.same(
            "rcbgPreSetSceneNo",
            |o| o.void(0x0092_17e0, &[no.cast_unsigned()]),
            |m| guide::bridge::rcbg_pre_set_scene_no(m, no, call),
        );
        let g = Mem::read::<u32>(&t.b, guide::YN_MAP_SYSTEM.get(&t.b) + 4);
        let mdl = if k % 2 == 0 { 0 } else { m0 };
        t.same(
            "rcbgGuideSetModel",
            |o| o.eax(0x0091_d590, &[g, mdl]),
            |m| guide::bridge::rcbg_guide_set_model(m, g, mdl, call),
        );
    }
}

/// A chunk of an `RCBG` background file: its tag, the body padded to 16
/// bytes (the length in 16-byte units at `+4`), `+0xa` a scene index.
fn rcbg_chunk(tag: [u8; 4], scene: u16, body: &[u8]) -> Vec<u8> {
    let len = body.len().next_multiple_of(16);
    let mut v = tag.to_vec();
    v.extend_from_slice(&u32::try_from(len / 16).unwrap().to_le_bytes());
    v.extend_from_slice(&[0, 0]);
    v.extend_from_slice(&scene.to_le_bytes());
    v.extend_from_slice(&[0; 4]);
    v.extend_from_slice(body);
    v.resize(16 + len, 0);
    v
}

/// Whole background files (`rcbgLoadBin`, `Yn_PreBgSetData`) and the
/// field renderer's map (re)set-up: `yiArc_readmem`, animation slots
/// (`yiAnimInfo_init`, `yiAnimDetach`, `yiKeyFrame_attachDObj`,
/// `yiAnimAttachDefault`), `yiRender_resetobj`, `yiResetObj`,
/// `yiObjReadFinish`.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round8_background_files_and_renderer_reset_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0008_0004);
    let env = LoadEnv::install(&mut t, &mut rng);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (file, work, image) = (t.alloc(0x2_0000), t.alloc(0xa4), t.alloc(0x60));
    for k in 0..30u32 {
        env.reset(&mut t, &mut rng);
        // a background file
        let scenes = 1 + rng.below(3);
        let mut f = b"YNDT".to_vec();
        f.resize(16, 0);
        f.extend(rcbg_chunk(*b"YNPR", 0, &scenes.to_le_bytes()));
        for _ in 0..scenes {
            f.extend(rcbg_chunk(*b"YNSC", 0, &scene_record(&mut rng)));
        }
        for _ in 0..rng.below(6) {
            let sc = u16::try_from(rng.below(scenes)).unwrap();
            let c = match rng.below(6) {
                0 => rcbg_chunk(*b"YNSM", 0, &model_record(&mut rng)),
                1 => rcbg_chunk(*b"YNST", 0, &trx_record(&mut rng)),
                2 => rcbg_chunk(*b"YNTT", 0, &trx_tex_record(&mut rng)),
                3 => rcbg_chunk(*b"YNFT", 0, &trx_tex_record(&mut rng)),
                4 => rcbg_chunk(*b"YNGM", sc, &model_record(&mut rng)),
                _ => rcbg_chunk(*b"YNSP", 0, &bytes(&mut rng, 0x20)),
            };
            f.extend(c);
        }
        f.extend(rcbg_chunk(*b"YNED", 0, &[]));
        assert!(f.len() < 0x2_0000);
        t.write(file, &f);
        t.write(work, &[0; 0xa4]);
        // the loaders need a map system (Yn_PreBgSetData checks for one)
        if k % 10 == 1 {
            t.set(guide::YN_MAP_SYSTEM, 0);
        } else if guide::YN_MAP_SYSTEM.get(&t.b) == 0 {
            t.same(
                "rcbgInit",
                |o| o.void(0x0092_1ce0, &[]),
                |m| guide::bridge::rcbg_init(m, call),
            );
        }
        // the port's chunk and parser stand where the original's stack is
        let clean = |m: &mut Oracle| m.write(work, &[0; 0xa4]).unwrap();
        if k % 2 == 0 {
            t.same(
                "rcbgLoadBin",
                |o| o.void(0x0092_1d60, &[file]),
                |m| {
                    guide::bridge::rcbg_load_bin(m, file, work, call);
                    clean(m);
                },
            );
        } else {
            let d = if k % 7 == 0 { 0 } else { file };
            t.same(
                "Yn_PreBgSetData",
                |o| o.void(0x0091_aa60, &[d]),
                |m| {
                    guide::bridge::yn_pre_bg_set_data(m, d, work, call);
                    clean(m);
                },
            );
        }
        // the renderer
        let r = t.alloc(0xb_b000);
        t.fill(r + 0x6_8000, 0x4_0000, &mut rng);
        t.fill(r + 0x9_c000, 0x2000, &mut rng);
        t.fill(r + 0xb_a3c0, 0x40, &mut rng);
        t.put(r + 0x9_caa8, if rng.below(2) == 0 { 0 } else { rng.next() });
        t.same(
            "yiRender_resetobj",
            |o| o.void(0x0090_cd90, &[r]),
            |m| yi::bridge::yi_render_resetobj(m, r, call),
        );
        // animation slots
        for i in 0..0x40u32 {
            let e = r + 0x9_cac0 + 0x28 * i;
            t.put(e, u16::try_from(rng.below(2)).unwrap());
            t.put(e + 4, u16::try_from(rng.below(6)).unwrap());
            t.put(e + 6, u16::try_from(rng.below(4)).unwrap());
        }
        t.put(r + 0x9_cabc, rng.below(0x20));
        let (obj, kind) = (rng.below(4), rng.below(6));
        t.same(
            "yiAnimDetach",
            |o| o.eax(0x0091_4920, &[r, obj, kind]),
            |m| yi::bridge::yi_anim_detach(m, r, obj, kind, call),
        );
        t.put(r + 0x6_8788, rng.below(4));
        let rec = t.alloc(0x20);
        t.put(rec + 8, rng.below(4));
        for a in 0..4u32 {
            t.put(
                r + 0x9_d4c4 + 4 * a,
                if rng.below(2) == 0 { 0 } else { rec },
            );
            t.put(r + 0x9_d5c4 + 4 * a, rng.below(2));
        }
        t.set(yi::ANIM_USE_VCOUNT, rng.below(2));
        let (anim, arg) = (rng.below(4), rng.below(0x1_0000));
        t.same(
            "yiKeyFrame_attachDObj",
            |o| o.eax(0x0091_44a0, &[r, obj, anim, arg]),
            |m| {
                yi::bridge::yi_key_frame_attach_dobj(
                    m,
                    r,
                    obj,
                    anim,
                    u16::try_from(arg).unwrap(),
                    call,
                )
            },
        );
        for i in 0..4u32 {
            let e = r + 0x6_8818 + 0xb4 * i;
            t.put(e, rng.below(3));
            for j in 0..3u32 {
                t.put(e + 4 + 2 * j, u16::try_from(rng.below(4)).unwrap());
            }
        }
        t.put(r + 0x9_d9c4, 0u32);
        t.same(
            "yiAnimAttachDefault",
            |o| o.void(0x0091_4250, &[r]),
            |m| yi::bridge::yi_anim_attach_all(m, r, call),
        );
        let info = r + 0x9_cab8;
        t.same(
            "yiAnimInfo_init",
            |o| o.void(0x0091_4310, &[info]),
            |m| yi::bridge::yi_anim_info_init(m, info, call),
        );
        // a map file's render section
        let map = t.alloc(0x100);
        t.fill(map, 0x100, &mut rng);
        t.write(map, if k % 3 == 0 { b"MAPX" } else { b"MAP1" });
        t.put(map + 0x14, if k % 4 == 0 { 0 } else { 0x40 });
        t.put(map + 0x18, if k % 2 == 0 { 0 } else { 0x80 });
        t.put(map + 0x40, 0x0021_4365u32);
        t.put(map + 0x44, if k % 2 == 0 { 0x110u32 } else { 0x111 });
        t.put(map + 0x4c, 0u32);
        let read = t.alloc(0x20);
        t.fill(read, 0x20, &mut rng);
        t.put(read + 0x10, map);
        t.same(
            "yiArc_readmem",
            |o| o.eax(0x0090_f8e0, &[r, map + 0x40]),
            |m| yi::bridge::yi_arc_readmem(m, r, map + 0x40, image, call),
        );
        t.same(
            "yiObjReadFinish",
            |o| o.void(0x0090_a9e0, &[read]),
            |m| yi::bridge::yi_obj_read_finish(m, read, image, call),
        );
        // a whole map file taken in (yiObjReadCallback)
        if guide::YN_MAP_SYSTEM.get(&t.b) != 0 {
            let mf = t.alloc(0x2_1000);
            t.fill(mf, 0x60, &mut rng);
            t.write(mf, if k % 5 == 4 { b"MAPX" } else { b"MAP1" });
            t.put(mf + 0x10, if k % 2 == 0 { 0 } else { 0x1000 });
            t.put(mf + 0x14, if k % 3 == 0 { 0 } else { 0x80 });
            t.put(mf + 0x84, 0x111u32);
            for off in [0x18u32, 0x1c, 0x3c, 0x40] {
                t.put(mf + off, 0u32);
            }
            t.put(mf + 0x38, if k % 2 == 0 { 0 } else { 0x40 });
            t.write(mf + 0x1000, &f);
            let rd = t.alloc(0x20);
            t.fill(rd, 0x20, &mut rng);
            t.put(rd + 0x10, mf);
            t.put(rd + 0x14, rng.below(3));
            t.same(
                "yiObjReadCallback",
                |o| o.void(0x0090_97c0, &[rd]),
                |m| {
                    yi::bridge::yi_obj_read_callback(m, rd, work, image, call);
                    clean(m);
                },
            );
        }
        if k % 3 == 0 {
            t.put(t.b.rebase(0x0191_38a0), rng.below(2));
            t.put(t.b.rebase(0x0191_38a4), rng.below(2) * rng.next());
            t.put(t.b.rebase(0x0193_4ec4), rng.range(-1, 2));
            t.same(
                "yiResetObj",
                |o| o.void(0x0090_9ea0, &[]),
                |m| yi::bridge::yi_reset_obj(m, call),
            );
            // round 37: yiReset, the same teardown and the texture reset
            t.put(t.b.rebase(0x0191_38a0), rng.below(2));
            t.put(t.b.rebase(0x0191_38a4), rng.below(2) * rng.next());
            t.put(t.b.rebase(0x0193_4ec4), rng.range(-1, 2));
            t.same(
                "yiReset",
                |o| o.eax(0x0090_9e60, &[]),
                |m| yi::bridge::yi_reset(m, call),
            );
        }
    }
}

/// The x87 intrinsics' import slots the camera code reaches (MSVCR110's
/// `_CItan`, `_CIatan`, `_CIcos`, `_CIsin`), which the oracle does not stub.
const TRIG_SLOTS: [u32; 4] = [CITAN_SLOT, 0x00b0_c450, 0x00b0_c3b4, 0x00b0_c3b8];
/// Their stand-ins, the CPU's own instructions on `ST0`: `fptan; fstp
/// st0`, `fld1; fpatan`, `fcos`, `fsin`, each then `ret`.
const TRIG_STUBS: [&[u8]; 4] = [
    &CITAN_STUB,
    &[0xd9, 0xe8, 0xd9, 0xf3, 0xc3],
    &[0xd9, 0xff, 0xc3],
    &[0xd9, 0xfe, 0xc3],
];
/// Where [`ST0_CALL`] goes in code space.
const ST0_CALL_AT: u32 = CODE_AREA + 0x140;
/// Calls an `ST0` intrinsic for the bridge's side: `fld qword [esp + 4]`,
/// then a jump to the address in the third word.
const ST0_CALL: [u8; 10] = [0xdd, 0x44, 0x24, 0x04, 0x8b, 0x44, 0x24, 0x0c, 0xff, 0xe0];

/// Where [`ST01_CALL`] goes in code space.
const ST01_CALL_AT: u32 = CODE_AREA + 0x150;
/// Calls a two-operand intrinsic for the bridge's side: `fld qword [esp +
/// 4]` (ST1), `fld qword [esp + 0xc]` (ST0), a jump to the fifth word.
const ST01_CALL: [u8; 14] = [
    0xdd, 0x44, 0x24, 0x04, 0xdd, 0x44, 0x24, 0x0c, 0x8b, 0x44, 0x24, 0x14, 0xff, 0xe0,
];

impl Twin {
    /// Points the trigonometric intrinsics at [`TRIG_STUBS`] and places
    /// [`ST0_CALL`] and [`ST01_CALL`], in both.
    fn install_trig(&mut self) {
        for o in [&mut self.a, &mut self.b] {
            for (i, (slot, code)) in (0u32..).zip(TRIG_SLOTS.into_iter().zip(TRIG_STUBS)) {
                let at = o.rebase(CODE_AREA + 0x100 + 0x10 * i);
                o.write(at, code).unwrap();
                let s = o.rebase(slot);
                o.write(s, &at.to_le_bytes()).unwrap();
            }
            let at = o.rebase(ST0_CALL_AT);
            o.write(at, &ST0_CALL).unwrap();
            let at = o.rebase(ST01_CALL_AT);
            o.write(at, &ST01_CALL).unwrap();
        }
    }

    /// [`Twin::same`], then the bytes in `skip` (absolute) taken from the
    /// original's side before memory is compared: what the original copies
    /// from its own stack frame (the `VFX_ScaleVec3` callers' 16-byte
    /// argument copy) cannot be reproduced.
    fn same_but<R: PartialEq + Debug>(
        &mut self,
        what: &str,
        skip: &[(u32, u32)],
        original: impl FnOnce(&mut Orig) -> R,
        rust: impl FnOnce(&mut Oracle) -> R,
    ) {
        let ra = original(&mut Orig(&mut self.a));
        let rb = rust(&mut self.b);
        assert_eq!(ra, rb, "{what}: results");
        for &(at, n) in skip {
            let mut v = vec![0u8; n as usize];
            Mem::read_bytes(&self.a, at, &mut v);
            self.b.write(at, &v).unwrap();
        }
        assert_eq!(self.a.diff(&mut self.b).unwrap(), [], "{what}: memory");
    }

    /// The first bytes of `f` (before a stand-in replaces it).
    fn keep(&self, f: u32) -> (u32, [u8; 5]) {
        (f, Mem::read(&self.a, self.a.rebase(f)))
    }

    /// Puts back what [`Twin::keep`] saved, in both: the function runs as
    /// the original again.
    fn restore(&mut self, (f, code): (u32, [u8; 5])) {
        let at = self.a.rebase(f);
        self.write(at, &code);
    }

    /// A scratch buffer in the bridge's stack (the original's stack frames
    /// are left out of the comparison, so the port's are too).
    fn stack_scratch(&self, off: u32) -> u32 {
        self.b.stack_range().start + 0x1000 + off
    }
}

/// The bridge's `fcall`: an intrinsic with its argument in `ST0` through
/// [`ST0_CALL`], or a cdecl function returning in `ST0`; the result popped.
fn oracle_fcall(m: &mut Oracle, f: u32, args: camera::FArgs<'_>) -> f64 {
    let r = match args {
        camera::FArgs::St0(x) => {
            let at = m.rebase(ST0_CALL_AT);
            let b = x.to_bits();
            #[allow(clippy::cast_possible_truncation)] // the two halves
            let (lo, hi) = (b as u32, (b >> 32) as u32);
            m.call(at, Conv::Cdecl, &[lo, hi, f]).unwrap()
        }
        camera::FArgs::St01(a, b) => {
            let at = m.rebase(ST01_CALL_AT);
            let (a, b) = (a.to_bits(), b.to_bits());
            #[allow(clippy::cast_possible_truncation)] // the halves
            let w = [a as u32, (a >> 32) as u32, b as u32, (b >> 32) as u32, f];
            m.call(at, Conv::Cdecl, &w).unwrap()
        }
        camera::FArgs::Stack(a) => m.call(f, Conv::Cdecl, a).unwrap(),
    };
    let pop = m.rebase(POP_STUB);
    m.call(pop, Conv::Cdecl, &[]).unwrap();
    r.st0.expect("a float result")
}

/// A float in `lo..hi` (sixteenths), as bits.
fn float_in(rng: &mut Xorshift, lo: i32, hi: i32) -> u32 {
    #[allow(clippy::cast_precision_loss)]
    let v = rng.range(lo * 16, hi * 16) as f32 / 16.0;
    v.to_bits()
}

/// The scale-vector family (`VFX_ScaleVec3` through vf31, its `MulX` and
/// fixed-point twins), and the map camera: `rcbgSetProjectionMatrixSub`
/// (`_CItan`), `rcbgSetViewMatrix`, `rcbgSceneSetMatrix` (`_CIatan`; the
/// battle camera matrices and `FFXSamp0ViewScreenMatrix` run as the
/// original, `Ch_SetClipZ` and the renderer's camera setters logged).
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round9_camera_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    t.install_trig();
    let mut rng = Xorshift(0x0009_0001);
    let (log, rv) = (t.alloc(4 + 0x20 * 256), t.alloc(4));
    // Ch_SetClipZ(chr, z): [chr + 0x174] = z, 1.0 in ST0
    let clip = vec![
        0x8b, 0x44, 0x24, 0x04, 0x8b, 0x4c, 0x24, 0x08, 0x89, 0x88, 0x74, 0x01, 0x00, 0x00, 0xd9,
        0xe8, 0xc3,
    ];
    // graphicCameraSetFOVy/AspectRatio(&v): v to log + 4 / log + 8
    let deref = |slot: u32| {
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x00, 0xa3];
        c.extend_from_slice(&slot.to_le_bytes());
        c.push(0xc3);
        c
    };
    t.cave(&[
        (Some(camera::CH_SET_CLIP_Z), clip),
        (Some(camera::CAMERA_SET_FOVY), deref(log + 0x1000)),
        (Some(camera::CAMERA_SET_ASPECT), deref(log + 0x1004)),
        (Some(camera::CAMERA_SET_WORLD), log_stub(log, rv, 3, 1)),
    ]);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let fcall = &mut oracle_fcall;
    let vf_args = t.b.rebase(vu0::VU0_SCALE) + 4;
    let skip = [(vf_args, 12u32)];
    // MsCameraMatrix ends by storing four uninitialised locals of its own
    // frame (Ghidra's local_1c/18/2c/28) over vf registers; their values
    // depend on what ran on that stack before
    let ms = t.b.rebase(0x00c8_f7d8);
    let view_skip = [(vf_args, 12u32), (ms, 8), (ms + 0x10, 8)];
    let (sys, map, sc) = (t.alloc(8), t.alloc(0x10), t.alloc(0x110));
    let chr = t.alloc(0x200);
    let (v, out, dst) = (t.alloc(0xc0), t.alloc(0x40), t.alloc(0x40));
    let (eye, at, up, distance) = (t.alloc(0x10), t.alloc(0x10), t.alloc(0x10), t.alloc(4));
    for k in 0..40u32 {
        t.put(log, 0u32);
        // the vector helpers
        finite_floats(&mut t, v, 4, &mut rng);
        let s = rng.float();
        let (x, y, z) = (rng.next(), rng.next(), rng.next());
        for (f, name) in [
            (0x0092_00a0u32, "rcbgScaleVector"),
            (0x0092_7b00, "rcbgModelScaleVector"),
            (0x0091_cae0, "rcbgScaleVector_0091cae0"),
            (0x0091_f6b0, "rcbgScaleVector_0091f6b0"),
            (0x0092_24b0, "rcbgScaleVector_009224b0"),
            (0x0092_47d0, "rcbgScaleVector_009247d0"),
            (0x0092_8600, "rcbgScaleVector_00928600"),
            (0x0092_8e50, "rcbgScaleVector_00928e50"),
            (0x0092_92a0, "rcbgScaleVector_009292a0"),
            (0x0092_94a0, "rcbgScaleVector_009294a0"),
            (0x0092_9640, "rcbgScaleVector_00929640"),
            (0x0092_d690, "rcbgScaleVector_0092d690"),
            (0x0092_d780, "rcbgScaleVector_0092d780"),
            (0x0092_f7f0, "rcbgScaleVector_0092f7f0"),
        ] {
            t.same_but(
                name,
                &skip,
                |o| o.eax(f, &[dst, v, s, x, y, z]),
                |m| vu0::bridge::rcbg_scale_vector(m, dst, v, s),
            );
        }
        for f in [0x0091_cb70u32, 0x0092_5040] {
            t.same_but(
                "rcbgScaleVector4",
                &skip,
                |o| o.eax(f, &[dst, v, s, x, y, z]),
                |m| vu0::bridge::rcbg_scale_vector4(m, dst, v, s),
            );
        }
        for f in [0x0092_73e0u32, 0x0092_7ac0] {
            t.same(
                "rcVectorToFixed",
                |o| o.eax(f, &[dst, v]),
                |m| vu0::bridge::rc_vector_to_fixed(m, dst, v),
            );
        }
        t.same(
            "rcbgModelVectorToInt",
            |o| o.eax(0x0092_7b60, &[dst, v]),
            |m| vu0::bridge::rcbg_model_vector_to_int(m, dst, v),
        );
        // the map system and the screen
        let with_map = k % 7 != 3;
        t.set(guide::YN_MAP_SYSTEM, if with_map { sys } else { 0 });
        t.put(sys, map);
        t.put(map + guide::map::SCENE, sc);
        t.put(sc + 0xf4, float_in(&mut rng, 1, 20));
        t.put(sc + 0xfc, rng.below(2));
        t.set(camera::DEPTH, float_in(&mut rng, 200, 900));
        t.set(camera::NEAR, float_in(&mut rng, 1, 20));
        t.set(camera::FAR, float_in(&mut rng, 100, 60000));
        t.set(camera::CENTER_X, float_in(&mut rng, 1800, 2300));
        t.set(camera::CENTER_Y, float_in(&mut rng, 1800, 2300));
        t.set(camera::SCREEN_0, float_in(&mut rng, -10, 10));
        t.set(camera::SCREEN_1, float_in(&mut rng, -10, 10));
        // the projection
        let fov = float_in(&mut rng, 10, 120);
        let (dx, dy) = (float_in(&mut rng, -50, 50), float_in(&mut rng, -50, 50));
        let d = if rng.below(3) == 0 { 0 } else { distance };
        t.same(
            "rcbgSetProjectionMatrixSub",
            |o| o.float(0x0092_02e0, &[out, fov, dx, dy, rng.next(), d]),
            |m| {
                let r = camera::bridge::rcbg_set_projection_matrix_sub(
                    m, out, fov, dx, dy, d, call, fcall,
                );
                bits(r)
            },
        );
        if !with_map {
            continue;
        }
        // the view
        for p in [eye, at, up] {
            for i in 0..3 {
                t.put(p + 4 * i, float_in(&mut rng, -500, 500));
            }
            t.put(p + 12, 0x3f80_0000u32);
        }
        let roll = float_in(&mut rng, -3, 3);
        let work = t.stack_scratch(0);
        t.same_but(
            "rcbgSetViewMatrix",
            &view_skip,
            |o| o.eax(0x0092_04b0, &[out, eye, at, up, roll]),
            |m| camera::bridge::rcbg_set_view_matrix(m, out, eye, at, up, roll, work, call),
        );
        // the whole camera
        t.fill(v, 0xc0, &mut rng);
        for (o, w) in [
            (camera::view::EYE, eye),
            (camera::view::AT, at),
            (camera::view::UP, up),
        ] {
            for i in 0..4 {
                let x = Mem::read::<u32>(&t.b, w + 4 * i);
                t.put(v + o + 4 * i, x);
            }
        }
        let fov = match k % 5 {
            0 => 0,
            1 => 0x8000_0000,
            _ => float_in(&mut rng, 10, 120),
        };
        t.put(v + camera::view::FOV, fov);
        t.put(v + camera::view::ROLL, float_in(&mut rng, -3, 3));
        t.put(v + camera::view::Y_OFFSET, rng.range(-64, 64));
        t.put(v + camera::view::OFFSET_X, float_in(&mut rng, -30, 30));
        t.put(v + camera::view::OFFSET_Y, float_in(&mut rng, -30, 30));
        t.put(
            t.b.rebase(0x0130_0788),
            if rng.below(2) == 0 { 0 } else { chr },
        );
        let work = t.stack_scratch(0x100);
        t.same_but(
            "rcbgSceneSetMatrix",
            &view_skip,
            |o| o.void(0x0092_0c00, &[v]),
            |m| camera::bridge::rcbg_scene_set_matrix(m, v, work, call, fcall),
        );
    }
}

/// An `ID map` (walk mesh) for `rcbgGuideSetModelFromIdMap`: `+0x6`
/// groups, `+0xa` vertices (`i16` x, z at 8-byte steps), `+0xc` the scale,
/// `+0x18` the vertices, `+0x1c` the groups (0x10 each: `+0x8` triangles,
/// `+0xc` the triangles, 0x10 each with three `u16` indices). Returns the
/// triangle count.
fn id_map(t: &mut Twin, rng: &mut Xorshift, id: u32, many: bool) -> u32 {
    let groups = rng.below(4);
    let verts = 1 + rng.below(24);
    t.put(id + 6, u16::try_from(groups).unwrap());
    t.put(id + 0xa, u16::try_from(verts).unwrap());
    t.put(id + 0xc, float_in(rng, 1, 64));
    let v = t.alloc(8 * verts as usize);
    for i in 0..verts {
        let x = u16::try_from(rng.below(0x1_0000)).unwrap();
        let z = u16::try_from(rng.below(0x1_0000)).unwrap();
        t.put(v + 8 * i, x);
        t.put(v + 8 * i + 4, z);
    }
    t.put(id + 0x18, v);
    let list = t.alloc(0x10 * groups.max(1) as usize);
    t.put(id + 0x1c, list);
    let mut total = 0;
    for g in 0..groups {
        let n = if many {
            0x90 + rng.below(0x40)
        } else {
            rng.below(12)
        };
        total += n;
        let polys = t.alloc(0x10 * n.max(1) as usize);
        for p in 0..n {
            for c in 0..3 {
                t.put(
                    polys + 0x10 * p + 2 * c,
                    u16::try_from(rng.below(verts)).unwrap(),
                );
            }
        }
        t.put(list + 0x10 * g + 8, u16::try_from(n).unwrap());
        t.put(list + 0x10 * g + 0xc, polys);
    }
    total
}

/// Models built from descriptions and the ID map, the guide's map
/// switch, scene object lookup and removal, the `rcfx` particle, spring,
/// nail and wind helpers, palette animations and transfer packets, over
/// round 8's loading environment.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round9_models_guide_and_rcfx_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0009_0002);
    let keep = t.keep(guide::GUIDE_SET_MODEL_FROM_ID_MAP);
    let env = LoadEnv::install(&mut t, &mut rng);
    t.restore(keep);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let vf_args = t.b.rebase(vu0::VU0_SCALE) + 4;
    let skip = [(vf_args, 12u32)];
    // a delete function that logs (user_free's stand-in)
    let del = t.b.rebase(model::USER_FREE);
    let (sys, map, scenes, g) = (t.alloc(8), t.alloc(0x10), t.alloc(0x110 * 3), t.alloc(0x80));
    let (id, desc, out) = (t.alloc(0x20), t.alloc(0x40), t.alloc(0x400));
    for k in 0..40u32 {
        env.reset(&mut t, &mut rng);
        // models from a description
        let mdl = model::bridge::rcbg_model_new(&mut t.b, call);
        model::bridge::rcbg_model_new(&mut t.a, call);
        let n = rng.below(4);
        t.put(desc, n);
        t.put(desc + 4, desc + 0x10);
        t.put(desc + 8, rng.below(40));
        t.put(desc + 0xc, rng.below(40));
        for i in 0..n {
            t.put(desc + 0x10 + 12 * i, rng.below(8));
            t.put(desc + 0x14 + 12 * i, rng.below(20));
            t.put(desc + 0x18 + 12 * i, rng.next());
        }
        t.same(
            "rcbgModelCreate",
            |o| o.eax(0x0092_7be0, &[mdl, desc]),
            |m| model::bridge::rcbg_model_create(m, mdl, desc, call),
        );
        let m2 = model::bridge::rcbg_model_new(&mut t.b, call);
        model::bridge::rcbg_model_new(&mut t.a, call);
        let (kind, count, verts, normals) =
            (rng.below(8), rng.below(30), rng.below(30), rng.below(30));
        let work = t.stack_scratch(0);
        t.same(
            "rcbgModelCreateOnePrimH",
            |o| o.eax(0x0092_7fa0, &[m2, kind, count, verts, normals]),
            |m| {
                model::bridge::rcbg_model_create_one_prim_h(
                    m, m2, kind, count, verts, normals, work, call,
                )
            },
        );
        let pts = t.alloc(0x10 * 8);
        for i in 0..32 {
            t.put(pts + 4 * i, float_in(&mut rng, -3000, 3000));
        }
        let (np, scale) = (rng.range(-1, 8), float_in(&mut rng, 1, 40));
        t.same_but(
            "rcbgModelFToS0",
            &skip,
            |o| {
                let r = o.eax(0x0092_81d0, &[out, pts, np.cast_unsigned(), scale]);
                if np > 0 { r } else { 0 }
            },
            |m| model::bridge::rcbg_model_f_to_s0(m, out, pts, np, scale, work),
        );
        // the guide's walk model from the ID map
        let old = if rng.below(2) == 0 { 0 } else { mdl };
        t.put(g + 8, old);
        let total = id_map(&mut t, &mut rng, id, k % 9 == 4);
        t.set(guide::ID_MAP, if k % 11 == 5 { 0 } else { id });
        t.same_but(
            "rcbgGuideSetModelFromIdMap",
            &skip,
            |o| o.void(0x0091_d5e0, &[g]),
            |m| guide::bridge::rcbg_guide_set_model_from_id_map(m, g, work, call),
        );
        let _ = total;
        // the map system: three scenes, objects of a few kinds
        t.set(guide::YN_MAP_SYSTEM, if k % 6 == 0 { 0 } else { sys });
        t.put(sys, if k % 6 == 1 { 0 } else { map });
        t.put(sys + 4, if k % 6 == 2 { 0 } else { g });
        t.put(map, rng.below(4));
        t.put(map + 4, scenes);
        let cur = scenes + 0x110 * rng.below(3);
        t.put(map + 8, if k % 6 == 3 { 0 } else { cur });
        for s in 0..3u32 {
            let sc = scenes + 0x110 * s;
            let n = rng.below(5);
            let list = model::bridge::rc_alloc(&mut t.b, call, 4 * n.max(1));
            model::bridge::rc_alloc(&mut t.a, call, 4 * n.max(1));
            for i in 0..n {
                let o = t.alloc(8);
                t.put(o, 0x2_0000 + rng.below(3));
                t.put(o + 4, if rng.below(3) == 0 { 0 } else { del });
                t.put(list + 4 * i, o);
            }
            t.put(sc + scene::layout::OBJECT_COUNT, n);
            t.put(sc + scene::layout::OBJECTS, list);
            let models = rng.below(4);
            t.put(
                sc + scene::layout::GUIDE_COUNT,
                u16::try_from(models).unwrap(),
            );
            for i in 0..4 {
                t.put(
                    sc + scene::layout::GUIDE_MODELS + 4 * i,
                    if i % 2 == 0 { 0 } else { m2 },
                );
            }
        }
        let (kind, nth) = (0x2_0000 + rng.below(3), rng.below(3));
        t.same(
            "Yn_SceneFindObject",
            |o| o.eax(0x0092_1c60, &[kind, nth]),
            |m| guide::bridge::yn_scene_find_object(m, kind, nth),
        );
        let no = rng.range(-1, 4);
        // the result is rcbgGuideSetModel's, or a leftover when it is not
        // reached
        let active = {
            let b = &t.b;
            let s = guide::YN_MAP_SYSTEM.get(b);
            let m = if s == 0 { 0 } else { Mem::read::<u32>(b, s) };
            let c = if m == 0 {
                0
            } else {
                Mem::read::<u32>(b, m + 8)
            };
            s != 0
                && Mem::read::<u32>(b, s + 4) != 0
                && c != 0
                && no >= 0
                && no < i32::from(Mem::read::<i16>(b, c + scene::layout::GUIDE_COUNT))
        };
        t.same(
            "Yn_GuideMapSetMapNo",
            |o| {
                let r = o.eax(0x0091_9da0, &[no.cast_unsigned()]);
                if active { r } else { 0 }
            },
            |m| guide::bridge::yn_guide_map_set_map_no(m, no, call),
        );
        let sc = scenes + 0x110 * rng.below(3);
        let cnt = Mem::read::<u32>(&t.b, sc + scene::layout::OBJECT_COUNT);
        let list = Mem::read::<u32>(&t.b, sc + scene::layout::OBJECTS);
        let victim = if cnt == 0 || rng.below(4) == 0 {
            rng.next()
        } else {
            Mem::read::<u32>(&t.b, list + 4 * rng.below(cnt))
        };
        t.same(
            "rcbgSceneDelObject",
            |o| o.void(0x0092_0670, &[sc, victim]),
            |m| scene::bridge::rcbg_scene_del_object(m, sc, victim, call),
        );
        // rcfx: a particle system's forces and buffers
        let ps = t.alloc(0x40);
        let nf = rng.below(4);
        let fl = if nf == 0 {
            0
        } else {
            let l = model::bridge::rc_alloc(&mut t.b, call, 4 * nf);
            model::bridge::rc_alloc(&mut t.a, call, 4 * nf);
            l
        };
        for i in 0..nf {
            let f = t.alloc(0x20);
            t.put(f + 0x18, rng.range(0, 8));
            t.put(f + 4, if rng.below(2) == 0 { 0 } else { del });
            t.put(fl + 4 * i, f);
        }
        t.put(ps + 0x34, nf);
        t.put(ps + 0x38, fl);
        let force = t.alloc(0x20);
        t.put(force + 0x18, rng.range(0, 8));
        t.same(
            "rcfxParticleAddForce",
            |o| o.void(0x0092_8660, &[ps, force]),
            |m| model::bridge::rcfx_particle_add_force(m, ps, force, call),
        );
        let cnt = rng.below(16);
        t.same(
            "rcfxParticleSetCnt",
            |o| o.void(0x0092_89f0, &[ps, cnt]),
            |m| model::bridge::rcfx_particle_set_cnt(m, ps, cnt, call),
        );
        t.same(
            "rcfxParticleFiniForces",
            |o| o.void(0x0092_8800, &[ps]),
            |m| model::bridge::rcfx_particle_fini_forces(m, ps, call),
        );
        let sp = t.alloc(0x30);
        t.fill(sp, 0x30, &mut rng);
        t.same(
            "rcfxSpringInit",
            |o| o.void(0x0092_8ce0, &[sp]),
            |m| model::bridge::rcfx_spring_init(m, sp),
        );
        let cnt = rng.below(16);
        t.same(
            "rcfxSpringSetCnt",
            |o| o.void(0x0092_8d80, &[sp, cnt]),
            |m| model::bridge::rcfx_spring_set_cnt(m, sp, cnt, call),
        );
        t.same(
            "rcfxSpringFini",
            |o| o.void(0x0092_8c80, &[sp]),
            |m| model::bridge::rcfx_spring_fini(m, sp, call),
        );
        let nail = t.alloc(0x30);
        let cnt = rng.below(16);
        t.same(
            "rcfxNailSetCnt",
            |o| o.void(0x0092_9230, &[nail, cnt]),
            |m| model::bridge::rcfx_nail_set_cnt(m, nail, cnt, call),
        );
        t.same(
            "rcfxNailFini",
            |o| o.void(0x0092_9090, &[nail]),
            |m| model::bridge::rcfx_nail_fini(m, nail, call),
        );
        let (wind, dir, pos) = (t.alloc(0x48), t.alloc(0x10), t.alloc(0x10));
        t.fill(dir, 0x10, &mut rng);
        t.fill(pos, 0x10, &mut rng);
        let power = rng.float();
        t.same(
            "rcfxWindSet",
            |o| o.void(0x0092_98e0, &[wind, dir, pos, power]),
            |m| model::bridge::rcfx_wind_set(m, wind, dir, pos, power),
        );
        // palettes over a transfer of 0x410-byte entries
        let (pal, trx) = (t.alloc(0x10), t.alloc(0x40));
        let entries = 2 + rng.below(3);
        let data = t.alloc(0x410 * entries as usize);
        t.fill(data, 0x410 * entries as usize, &mut rng);
        t.put(trx + 0x30, entries);
        t.put(trx + 0x34, 0x410u32);
        t.put(trx + 0x38, data);
        t.put(pal + 0xc, trx);
        t.put(pal + 4, u16::try_from(rng.below(5)).unwrap());
        t.put(pal + 6, u16::try_from(rng.below(0x100)).unwrap());
        t.put(pal + 8, u16::try_from(rng.below(entries - 1)).unwrap());
        t.put(pal + 0xb, u8::try_from(rng.below(4)).unwrap());
        match k % 3 {
            0 => t.same(
                "rcbgPalRotate",
                |o| o.void(0x0092_9a10, &[pal]),
                |m| model::bridge::rcbg_pal_rotate(m, pal),
            ),
            1 => t.same(
                "rcbgPalShift",
                |o| o.void(0x0092_9ac0, &[pal]),
                |m| model::bridge::rcbg_pal_shift(m, pal),
            ),
            _ => t.same(
                "rcbgPalWhite",
                |o| o.void(0x0092_9ba0, &[pal]),
                |m| model::bridge::rcbg_pal_white(m, pal),
            ),
        }
        // a texture's transfers and packet
        let tex = t.alloc(0xa0);
        t.fill(tex, 0xa0, &mut rng);
        let (image, clut) = (rng.next(), rng.next());
        t.same(
            "rcbgTrxTexSetAddr",
            |o| o.void(0x0092_2210, &[tex, image, clut]),
            |m| model::bridge::rcbg_trx_tex_set_addr(m, tex, image, clut),
        );
        let tr = t.alloc(0x40);
        t.fill(tr, 0x40, &mut rng);
        let format = [0u32, 0x13, 0x14][rng.below(3) as usize];
        let (w, h, n) = (rng.below(32), rng.below(32), rng.below(4));
        t.put(tr + 0x34, 0x40u32);
        t.same(
            "rcbgTrxAllocEntries",
            |o| o.void(0x0092_d3e0, &[tr, format, w, h, n]),
            |m| {
                #[allow(clippy::cast_possible_truncation)]
                let s = |v: u32| v as i16;
                model::bridge::rcbg_trx_alloc_entries(m, tr, s(format), s(w), s(h), n, call);
            },
        );
        t.put(tr + 0x3c, rng.below(4));
        t.same(
            "rcbgTrxPacketMake",
            |o| o.eax(0x0092_d5a0, &[tr, out]),
            |m| model::bridge::rcbg_trx_make_packet(m, tr, out),
        );
    }
}

/// `mov eax, imm32; ret`.
fn ret_stub(v: u32) -> Vec<u8> {
    let mut c = vec![0xb8];
    c.extend_from_slice(&v.to_le_bytes());
    c.push(0xc3);
    c
}

/// Parser helpers (new, skip line, integers in both modes), `rcdtTm2`
/// new/fini and the field lists' teardown, effect tasks and character
/// blocks, the pre-loaded map's field programs, the renderer's resets and
/// rotations, and the GS swizzle of format-0x13 images (its error path
/// through stand-ins for `__iob_func`/`fprintf`).
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round9_parse_tasks_yi_and_swizzle_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0009_0003);
    let env = LoadEnv::install(&mut t, &mut rng);
    t.install_trig();
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (log, rv, bn) = (env.log, env.rv, t.alloc(4));
    let del = t.b.rebase(model::USER_FREE);
    // more stand-ins after round 4's code and the intrinsics
    let mut at = 0x160;
    let mut stub = |t: &mut Twin, target: Option<u32>, code: Vec<u8>| {
        let s = t.stub(target, at, &code);
        at += u32::try_from(code.len()).unwrap().next_multiple_of(0x10);
        assert!(at <= 0x450, "the stubs outgrow pppFpLoop's body");
        s
    };
    stub(&mut t, Some(task::PPP_MEM_FREE), log_stub(log, rv, 40, 3));
    stub(
        &mut t,
        Some(guide::MS_BATTLE_GET_NUM),
        log_stub(log, bn, 41, 0),
    );
    let iob = stub(&mut t, None, ret_stub(0x0012_3400));
    let fprintf = stub(&mut t, None, log_stub(log, rv, 42, 2));
    for (slot, s) in [(yi::CRT_IOB_FUNC, iob), (yi::CRT_FPRINTF, fprintf)] {
        let at = t.b.rebase(slot);
        t.put(at, s);
    }
    let (chunk, parse, data) = (t.alloc(8), t.alloc(0x90), t.alloc(0x200));
    let r0 = yi::state::RENDER.ptr(&t.b).addr;
    for k in 0..30u32 {
        env.reset(&mut t, &mut rng);
        // parsers
        t.same(
            "rcdtParseNew",
            |o| o.eax(0x0092_bfd0, &[]),
            |m| rcdt::bridge::rcdt_parse_new(m, call),
        );
        let mut text = Vec::new();
        for _ in 0..=rng.below(6) {
            text.extend(format!("{} ", rng.range(-5000, 5000)).bytes());
            if rng.below(3) == 0 {
                text.push(b'\n');
            }
        }
        text.extend(b"\n0\n\0");
        t.write(data, &text);
        parser_over(&mut t, chunk, parse, data);
        if k % 2 == 1 {
            t.put(parse + 8, 1u32);
        }
        for _ in 0..3 {
            t.same(
                "rcdtParseGetInt",
                |o| o.eax(0x0092_bc50, &[parse]),
                |m| rcdt::bridge::rcdt_parse_get_int(m, parse, call),
            );
        }
        t.same(
            "rcdtParseSkipLine",
            |o| o.void(0x0092_b970, &[parse]),
            |m| rcdt::bridge::rcdt_parse_skip_line(m, parse),
        );
        // TIM2 holders and lists
        let tm = t.alloc(0x50);
        t.same(
            "rcdtTm2New",
            |o| o.eax(0x0092_3b40, &[]),
            |m| rcdt::bridge::rcdt_tm2_new(m, call),
        );
        for o in [0x40u32, 0x44] {
            let p = if rng.below(2) == 0 {
                0
            } else {
                model::bridge::rc_alloc(&mut t.a, call, 0x10);
                model::bridge::rc_alloc(&mut t.b, call, 0x10)
            };
            t.put(tm + o, p);
        }
        t.same(
            "rcdtTm2Fini",
            |o| o.void(0x0092_3470, &[tm]),
            |m| rcdt::bridge::rcdt_tm2_fini(m, tm, call),
        );
        let l = t.alloc(0x20);
        let n = rng.below(4);
        let table = t.alloc(4 * n.max(1) as usize);
        for i in 0..n {
            t.put(
                table + 4 * i,
                if rng.below(3) == 0 { 0 } else { rng.next() | 1 },
            );
        }
        for o in [4u32, 0xc, 0x14] {
            t.put(l + o, if rng.below(2) == 0 { 0 } else { rng.next() | 1 });
        }
        t.put(l + 0x18, n);
        t.put(
            l + 0x1c,
            if n == 0 && rng.below(2) == 0 {
                0
            } else {
                table
            },
        );
        t.same(
            "rcdtListsFini",
            |o| o.void(0x0092_42f0, &[l]),
            |m| model::bridge::rcdt_lists_fini(m, l, call),
        );
        // effect tasks
        let (prev, task0, next) = (t.alloc(0xbc), t.alloc(0xbc), t.alloc(0xbc));
        t.put(task0 + 4, prev);
        t.put(task0 + 8, next);
        t.put(task0 + 0xb4, if rng.below(2) == 0 { 0 } else { del });
        t.same(
            "rcefTaskKill",
            |o| o.eax(0x0092_ed50, &[task0]),
            |m| task::bridge::rcef_task_kill(m, task0, call),
        );
        let c = t.alloc(0x100);
        t.fill(c, 0x100, &mut rng);
        t.same(
            "rcefChrInit",
            |o| o.eax(0x0093_0770, &[c]),
            |m| task::bridge::rcef_chr_init(m, c, call),
        );
        let (w, a, b) = (t.alloc(0x200), t.alloc(0x10), t.alloc(0x10));
        t.fill(w + 0x1c0, 0x40, &mut rng);
        t.fill(a, 0x10, &mut rng);
        t.fill(b, 0x10, &mut rng);
        let tmp = t.stack_scratch(0);
        t.same(
            "rcefWorkSwap",
            |o| o.void(0x0092_5530, &[w]),
            |m| task::bridge::rcef_work_swap(m, w, tmp),
        );
        let v = rng.next();
        t.same(
            "rcefWorkSet",
            |o| o.void(0x0092_5630, &[w, a, b, v]),
            |m| task::bridge::rcef_work_set(m, w, a, b, v),
        );
        // the pre-loaded map's field programs (a map system first)
        if guide::YN_MAP_SYSTEM.get(&t.b) == 0 {
            t.same(
                "rcbgInit",
                |o| o.void(0x0092_1ce0, &[]),
                |m| guide::bridge::rcbg_init(m, call),
            );
        }
        if k % 3 == 0 {
            t.same(
                "rcbgPreInit",
                |o| o.void(0x0092_1770, &[]),
                |m| guide::bridge::rcbg_pre_init(m, call),
            );
        }
        // a map with a current scene for the objects
        t.same(
            "rcbgPreDummyInit",
            |o| o.void(0x0092_1640, &[]),
            |m| guide::bridge::rcbg_pre_dummy_init(m, call),
        );
        let no = [0x61, 0xad, 0x235, 0x236, 0x62, 0][rng.below(6) as usize];
        t.same(
            "rcbgPreAddFp",
            |o| o.void(0x0092_1a50, &[no]),
            |m| guide::bridge::rcbg_pre_add_fp(m, no.cast_signed(), call),
        );
        t.put(
            bn,
            [0, 0x0061_0000, 0x0235_0000, 0x0236_0000, 0x0100_0000][rng.below(5) as usize],
        );
        t.same(
            "rcbgPreGetMapNo",
            |o| o.eax(0x0092_1ba0, &[]),
            |m| guide::bridge::rcbg_pre_get_map_no(m, call),
        );
        let stage = rng.range(-1, 3);
        t.same(
            "Yn_PreBgChangeStage",
            |o| o.void(0x0091_a0d0, &[stage.cast_unsigned()]),
            |m| guide::bridge::yn_pre_bg_change_stage(m, stage, call),
        );
        // round 37: the ID map pages of a map file (none, another tag, or
        // MAP1 with some sections)
        let mapf = t.alloc(0x80);
        t.fill(mapf, 0x80, &mut rng);
        t.write(mapf, if rng.below(4) == 0 { b"MAPX" } else { b"MAP1" });
        for i in 0..6 {
            t.put(mapf + 0x10 + 4 * i, rng.below(2) * 0x40);
        }
        let md = t.b.rebase(0x0191_38a8);
        t.put(md, if rng.below(5) == 0 { 0 } else { mapf });
        let page = stage.cast_unsigned();
        t.same(
            "yiSetIDPage",
            |o| o.eax(0x0090_a4a0, &[page]),
            |m| yi::bridge::yi_set_id_page(m, page, call),
        );
        t.same(
            "yiSetPreBgPage",
            |o| o.eax(0x0090_a6f0, &[page]),
            |m| yi::bridge::yi_set_pre_bg_page(m, page, call),
        );
        // the renderer
        let r = t.alloc(0xb_b000);
        t.fill(r + 0x6_8000, 0x4_0000, &mut rng);
        t.fill(r + 0x9_c000, 0x2000, &mut rng);
        t.fill(r + 0xb_a3c0, 0x40, &mut rng);
        t.put(r + 0x9_caa8, if rng.below(2) == 0 { 0 } else { rng.next() });
        t.same(
            "yiRender_resetall",
            |o| o.void(0x0090_cd50, &[r]),
            |m| yi::bridge::yi_render_resetall(m, r, call),
        );
        t.same(
            "yiRender_initpacket",
            |o| o.eax(0x0090_cf00, &[r]),
            |m| yi::bridge::yi_render_initpacket(m, r),
        );
        t.same(
            "yiRender_initfog",
            |o| o.void(0x0091_3f50, &[r]),
            |m| yi::bridge::yi_render_initfog(m, r),
        );
        let n = rng.below(0x100);
        t.same(
            "yiMallocQuiet",
            |o| o.eax(0x0090_bc10, &[n]),
            |m| yi::bridge::yi_malloc_quiet(m, n, call),
        );
        // the renderer itself: an animation to attach
        t.put(r0 + 0x6_8788, rng.below(4));
        let rec = t.alloc(0x20);
        t.put(rec + 8, rng.below(4));
        for a in 0..4u32 {
            t.put(
                r0 + 0x9_d4c4 + 4 * a,
                if rng.below(2) == 0 { 0 } else { rec },
            );
            t.put(r0 + 0x9_d5c4 + 4 * a, rng.below(2));
        }
        let (obj, anim, arg) = (rng.below(4), rng.below(4), rng.next());
        t.same(
            "yiAttachAnimDObj",
            |o| o.eax(0x0090_8330, &[obj, anim, arg]),
            |m| yi::bridge::yi_attach_anim_dobj(m, obj, anim, arg, call),
        );
        let (m, angles) = (t.alloc(0x40), t.alloc(0xc));
        finite_floats(&mut t, m, 16, &mut rng);
        for i in 0..3 {
            t.put(angles + 4 * i, float_in(&mut rng, -7, 7));
        }
        let order = rng.below(7);
        t.same(
            "yiRotMatrixSeq",
            |o| o.eax(0x0091_51c0, &[m, angles, order]),
            |mm| yi::bridge::yi_rot_matrix_seq(mm, m, angles, order, call),
        );
        // the swizzle
        let (src, dst) = (t.alloc(0x100), t.alloc(0x100));
        t.fill(src, 0x100, &mut rng);
        t.same(
            "yiSwizzleColumns",
            |o| o.eax(0x0090_e000, &[src, dst]),
            |m| yi::bridge::yi_swizzle_columns(m, src, dst),
        );
        let work = t.stack_scratch(0x100);
        let (src, dst) = (t.alloc(0x2000), t.alloc(0x4000));
        t.fill(src, 0x2000, &mut rng);
        let (w, h) = (0x10 * (1 + rng.below(8)), 0x10 * (1 + rng.below(4)));
        t.same(
            "yiSwizzlePage",
            |o| o.void(0x0090_e2e0, &[w, h, src, dst]),
            |m| {
                yi::bridge::yi_swizzle_page(
                    m,
                    w.cast_signed(),
                    h.cast_signed(),
                    src,
                    dst,
                    work,
                    call,
                );
            },
        );
        let pow = |rng: &mut Xorshift| match rng.below(10) {
            0 => 3 + 2 * rng.below(20),
            _ => 1 << rng.below(9),
        };
        let (w, h) = (pow(&mut rng), pow(&mut rng));
        let size = (w * h).max(0x80 * 0x40) as usize;
        let (src, dst) = (t.alloc(size), t.alloc(2 * size));
        t.fill(src, size, &mut rng);
        t.same(
            "yiSwizzleImage",
            |o| o.eax(0x0090_e0b0, &[w, h, src, dst]),
            |m| {
                yi::bridge::yi_swizzle_image(
                    m,
                    w.cast_signed(),
                    h.cast_signed(),
                    src,
                    dst,
                    work,
                    call,
                )
            },
        );
    }
}

/// `memcpy(arg 2, src, len)` for a CD read stand-in, saving `esi`/`edi`.
fn copy_stub(src: u32, len: u32) -> Vec<u8> {
    let mut c = vec![0x56, 0x57, 0x8b, 0x7c, 0x24, 0x10, 0xbe];
    c.extend_from_slice(&src.to_le_bytes());
    c.push(0xb9);
    c.extend_from_slice(&len.to_le_bytes());
    c.extend_from_slice(&[0xf3, 0xa4, 0x5f, 0x5e, 0xc3]);
    c
}

/// A small `RCBG` background file (header, one to three scenes, a few
/// chunks, the end).
fn yn_file(rng: &mut Xorshift) -> Vec<u8> {
    let scenes = 1 + rng.below(2);
    let mut f = b"YNDT".to_vec();
    f.resize(16, 0);
    f.extend(rcbg_chunk(*b"YNPR", 0, &scenes.to_le_bytes()));
    for _ in 0..scenes {
        f.extend(rcbg_chunk(*b"YNSC", 0, &scene_record(rng)));
    }
    for _ in 0..rng.below(3) {
        f.extend(rcbg_chunk(*b"YNSP", 0, &bytes(rng, 0x20)));
    }
    f.extend(rcbg_chunk(*b"YNED", 0, &[]));
    f
}

/// `yiMapReadMaster` (a MAP1 file through `GetCD`'s table: the 2D and 3D
/// maps, the alarms, effect data and two more background files) and the
/// debug `yiMapSetup`, over round 8's loading environment with the CD,
/// file and renderer calls logged.
#[test]
#[allow(clippy::too_many_lines)] // the stand-ins, then the cases
fn round9_map_reads_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0009_0004);
    let env = LoadEnv::install(&mut t, &mut rng);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (log, rv) = (env.log, env.rv);
    let (cdp, cdt, sizev, fdv) = (t.alloc(4), t.alloc(0x18), t.alloc(4), t.alloc(4));
    let file = t.alloc(0x4000);
    // MAP_PATH_PARTS(map, dir, name, extra): the map logged, "id" and "na"
    let mut parts = vec![0x8b, 0x44, 0x24, 0x04, 0xa3];
    parts.extend_from_slice(&(log + 0x2_0010).to_le_bytes());
    parts.extend_from_slice(&[
        0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00, 0x69, 0x64, 0x00, 0x00, 0x8b, 0x44, 0x24, 0x0c, 0xc7,
        0x00, 0x6e, 0x61, 0x00, 0x00, 0xc3,
    ]);
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(yi::GRAPHIC_SET_HAS_2D_MAP), log_stub(log, rv, 50, 1)),
        (Some(yi::GRAPHIC_SET_HAS_3D_MAP), log_stub(log, rv, 51, 1)),
        (Some(yi::GRAPHIC_FIELD_MAP_LOAD), log_stub(log, rv, 52, 2)),
        (
            Some(yi::GRAPHIC_FIELD_MAP_INITIALIZE),
            log_stub(log, rv, 53, 0),
        ),
        (
            Some(yi::GRAPHIC_VFX_LOAD_LEVEL_DATA),
            log_stub(log, rv, 54, 0),
        ),
        (Some(yi::GET_CD), log_stub(log, cdp, 55, 0)),
        (Some(yi::GET_CD_FILE_NAME_PC), log_stub(log, rv, 56, 1)),
        (Some(yi::MAP_PATH_PARTS), parts),
        (Some(yi::SCE_OPEN), log_stub_from(log, fdv, 57, 1, 1)),
        (Some(yi::SCE_LSEEK), log_stub(log, sizev, 58, 3)),
        (Some(yi::SCE_READ), log_stub(log, rv, 59, 3)),
        (Some(yi::SCE_CLOSE), log_stub(log, rv, 60, 1)),
        // the CD table: read (a copy of `file`), finish, select, size
        (None, copy_stub(file, 0x4000)),
        (None, log_stub(log, rv, 61, 1)),
        (None, log_stub(log, rv, 62, 1)),
        (None, log_stub_from(log, sizev, 63, 0, 1)),
        (None, ret_stub(0x0012_3400)),
        (None, log_stub(log, rv, 64, 4)),
    ];
    let (s, end) = t.cave_from(env.cave_end, &list);
    let _ = end;
    t.put(cdt, s[12]);
    t.put(cdt + 8, s[13]);
    t.put(cdt + 0x10, s[14]);
    t.put(cdt + 0x14, s[15]);
    t.put(cdp, cdt);
    for (slot, f) in [(yi::CRT_IOB_FUNC, s[16]), (yi::CRT_FPRINTF, s[17])] {
        let at = t.b.rebase(slot);
        t.put(at, f);
    }
    let (image, work, local) = (
        t.stack_scratch(0),
        t.stack_scratch(0x100),
        t.stack_scratch(0x200),
    );
    let host = t.stack_scratch(0x400);
    for k in 0..24u32 {
        env.reset(&mut t, &mut rng);
        // the MAP1 file: sections 0, 11 and 12 background files, 1 a 3D
        // map of another version, 10 effect data, alarm + 2 the alarms
        t.write(file, &[0; 0x4000]);
        t.write(file, b"MAP1");
        let alarm = rng.below(6);
        let yn = yn_file(&mut rng);
        assert!(yn.len() < 0x1800);
        t.write(file + 0x100, &yn);
        t.fill(file + 0x2000, 0x40, &mut rng);
        t.put(file + 0x2004, 0u32);
        for (index, off) in [
            (0u32, 0x100u32),
            (1, 0x2000),
            (10, 0x2100),
            (11, 0x100),
            (12, 0x100),
            (alarm + 2, 0x2200),
        ] {
            if rng.below(4) != 0 {
                t.put(file + 0x10 + 4 * index, off);
            }
        }
        if k % 7 == 3 {
            t.write(file, b"MAP0");
        }
        t.put(sizev, if k % 9 == 4 { 0 } else { 0x4000 });
        t.put(fdv, if k % 5 == 2 { 0 } else { 3 });
        let ids = |rng: &mut Xorshift| {
            if rng.below(3) == 0 {
                (u32::MAX, u32::MAX)
            } else {
                (rng.below(0x400), rng.below(0x400))
            }
        };
        let (map, mfile) = ids(&mut rng);
        let (cd_map, cd_file) = ids(&mut rng);
        let clean = |m: &mut Oracle| {
            m.write(work, &[0; 0xa4]).unwrap();
            m.write(local, &[0; 0x20]).unwrap();
        };
        clean(&mut t.b);
        t.same(
            "yiMapReadMaster",
            |o| o.eax(0x0090_93b0, &[map, mfile, cd_map, cd_file, alarm]),
            |m| {
                yi::bridge::yi_map_read_master(
                    m, map, mfile, cd_map, cd_file, alarm, image, work, local, call,
                )
            },
        );
        t.b.write(host, &[0; 0x1000]).unwrap();
        let m0 = rng.below(0x400);
        t.same(
            "yiMapSetup",
            |o| o.eax(0x0090_9680, &[m0]),
            |m| yi::bridge::yi_map_read_host(m, m0, host, image, call),
        );
    }
}

/// Building and tearing down field objects with the game's own heap
/// (`user_malloc`/`user_free`, initialised in the snapshot; the CRT's
/// critical sections are the oracle's stubs): models and their primitive
/// blobs, textures and transfers, `rcfx` nails, springs and particle
/// forces, TIM2 holders, `yiMalloc`/`yiFree`.
#[test]
#[allow(clippy::too_many_lines)] // one object after another
fn round9_real_heap_matches_the_original_on_a_snapshot() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x0009_0005);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let desc = t.alloc(0x40);
    for _ in 0..12u32 {
        // a model and its primitive blob
        let mut mdl = 0;
        t.same(
            "rcbgModelNew",
            |o| o.eax(0x0092_83b0, &[]),
            |m| {
                mdl = model::bridge::rcbg_model_new(m, call);
                mdl
            },
        );
        let n = rng.below(4);
        t.put(desc, n);
        t.put(desc + 4, desc + 0x10);
        t.put(desc + 8, rng.below(40));
        t.put(desc + 0xc, rng.below(40));
        for i in 0..n {
            t.put(desc + 0x10 + 12 * i, rng.below(8));
            t.put(desc + 0x14 + 12 * i, rng.below(20));
            t.put(desc + 0x18 + 12 * i, rng.next());
        }
        t.same(
            "rcbgModelCreate",
            |o| o.eax(0x0092_7be0, &[mdl, desc]),
            |m| model::bridge::rcbg_model_create(m, mdl, desc, call),
        );
        t.same(
            "rcbgModelDelete",
            |o| o.eax(0x0092_8000, &[mdl]),
            |m| model::bridge::rcbg_model_delete(m, mdl, call),
        );
        // a texture whose image transfer owns its entries
        let mut tex = 0;
        t.same(
            "rcbgTrxTexNew",
            |o| o.eax(0x0092_21b0, &[]),
            |m| {
                tex = model::bridge::rcbg_trx_tex_new(m, call);
                tex
            },
        );
        let (w, h, e) = (rng.below(32), rng.below(32), 1 + rng.below(3));
        t.same(
            "rcbgTrxAllocEntries",
            |o| o.void(0x0092_d3e0, &[tex + 0x20, 0x13, w, h, e]),
            |m| {
                #[allow(clippy::cast_possible_truncation)]
                let s = |v: u32| v as i16;
                model::bridge::rcbg_trx_alloc_entries(m, tex + 0x20, 0x13, s(w), s(h), e, call);
            },
        );
        t.same(
            "rcbgTrxTexDelete",
            |o| o.eax(0x0092_20e0, &[tex]),
            |m| model::bridge::rcbg_trx_tex_delete(m, tex, call),
        );
        // rcfx: a nail, a spring, a particle system with forces
        let mut nail = 0;
        t.same(
            "rcfxNailNew",
            |o| o.eax(0x0092_9170, &[]),
            |m| {
                nail = model::bridge::rcfx_nail_new(m, call);
                nail
            },
        );
        for _ in 0..2 {
            let cnt = rng.below(24);
            t.same(
                "rcfxNailSetCnt",
                |o| o.void(0x0092_9230, &[nail, cnt]),
                |m| model::bridge::rcfx_nail_set_cnt(m, nail, cnt, call),
            );
        }
        t.same(
            "rcfxNailFini",
            |o| o.void(0x0092_9090, &[nail]),
            |m| model::bridge::rcfx_nail_fini(m, nail, call),
        );
        let mut sp = 0;
        t.same(
            "rcAlloc",
            |o| o.eax(0x0092_58a0, &[0x30, 0, 0]),
            |m| {
                sp = model::bridge::rc_alloc(m, call, 0x30);
                sp
            },
        );
        t.same(
            "rcfxSpringInit",
            |o| o.void(0x0092_8ce0, &[sp]),
            |m| model::bridge::rcfx_spring_init(m, sp),
        );
        for _ in 0..2 {
            let cnt = rng.below(24);
            t.same(
                "rcfxSpringSetCnt",
                |o| o.void(0x0092_8d80, &[sp, cnt]),
                |m| model::bridge::rcfx_spring_set_cnt(m, sp, cnt, call),
            );
        }
        t.same(
            "rcfxSpringFini",
            |o| o.void(0x0092_8c80, &[sp]),
            |m| model::bridge::rcfx_spring_fini(m, sp, call),
        );
        let mut ps = 0;
        t.same(
            "rcfxParticleNew",
            |o| o.eax(0x0092_8950, &[]),
            |m| {
                ps = model::bridge::rcfx_particle_new(m, call);
                ps
            },
        );
        let cnt = rng.below(24);
        t.same(
            "rcfxParticleSetCnt",
            |o| o.void(0x0092_89f0, &[ps, cnt]),
            |m| model::bridge::rcfx_particle_set_cnt(m, ps, cnt, call),
        );
        for _ in 0..rng.below(4) {
            let mut f = 0;
            t.same(
                "rcfxWindNew",
                |o| o.eax(0x0092_98c0, &[]),
                |m| {
                    f = model::bridge::rcfx_wind_new(m, call);
                    f
                },
            );
            t.put(f + 0x18, rng.range(0, 8));
            t.same(
                "rcfxParticleAddForce",
                |o| o.void(0x0092_8660, &[ps, f]),
                |m| model::bridge::rcfx_particle_add_force(m, ps, f, call),
            );
        }
        t.same(
            "rcfxParticleFiniForces",
            |o| o.void(0x0092_8800, &[ps]),
            |m| model::bridge::rcfx_particle_fini_forces(m, ps, call),
        );
        // a TIM2 holder with both buffers
        let mut tm = 0;
        t.same(
            "rcdtTm2New",
            |o| o.eax(0x0092_3b40, &[]),
            |m| {
                tm = rcdt::bridge::rcdt_tm2_new(m, call);
                tm
            },
        );
        for o in [0x40u32, 0x44] {
            let mut p = 0;
            let n = 1 + rng.below(0x80);
            t.same(
                "rcAlloc",
                |or| or.eax(0x0092_58a0, &[n, 0, 0]),
                |m| {
                    p = model::bridge::rc_alloc(m, call, n);
                    p
                },
            );
            t.put(tm + o, p);
        }
        t.same(
            "rcdtTm2Fini",
            |o| o.void(0x0092_3470, &[tm]),
            |m| rcdt::bridge::rcdt_tm2_fini(m, tm, call),
        );
        let mut q = 0;
        let n = rng.below(0x100);
        t.same(
            "yiMallocQuiet",
            |o| o.eax(0x0090_bc10, &[n]),
            |m| {
                q = yi::bridge::yi_malloc_quiet(m, n, call);
                q
            },
        );
        t.same(
            "yiFree",
            |o| o.eax(0x0090_bbc0, &[q]),
            |m| yi::bridge::yi_free(m, q, call),
        );
    }
}

/// `mov eax, [esp + 0xc]; mov ecx, [value]; mov [eax], ecx; ret`: a
/// `pppFpGetMngPtr` stand-in storing the manager through its third
/// argument.
fn store_arg3_stub(value: u32) -> Vec<u8> {
    let mut c = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d];
    c.extend_from_slice(&value.to_le_bytes());
    c.extend_from_slice(&[0x89, 0x08, 0xc3]);
    c
}

/// Round 10: parser keywords, floats and text, the TIM2/list deleters,
/// matrix row scales, field-program matrices and restarts, map reads by
/// kind, map and base paths, rcui reset, transfer set-up, `rcfx` springs,
/// particles and the simulation's forces, palette dispatch, effect task
/// lists and character blocks, scene screen settings, the pre-loaded map's
/// field objects and object removal by kind.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round10_field_helpers_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x000a_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    t.install_trig();
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let fcall = &mut oracle_fcall;
    let (log, rv) = (env.log, env.rv);
    let del = t.b.rebase(model::USER_FREE);
    let (mngv, bn, block) = (t.alloc(4), t.alloc(4), t.alloc(4));
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(model::CH_ELEM_GET_MATRIX), log_stub(log, rv, 70, 3)),
        (Some(model::PPP_FP_GET_MNG_PTR), store_arg3_stub(mngv)),
        (Some(model::FP_RESTART), log_stub(log, rv, 71, 1)),
        (Some(yi::YI_MAP_READ_MASTER), log_stub(log, rv, 72, 5)),
        (Some(rcui::GU_WINDOW_CLOSE_TREE), log_stub(log, rv, 73, 1)),
        (Some(task::PPP_RUN_PART), log_stub(log, rv, 74, 2)),
        (Some(task::PPP_MEM_ALLOC), log_stub(log, block, 75, 3)),
        (Some(task::PPP_MEM_FREE), log_stub(log, rv, 76, 3)),
        (
            Some(task::CH_TEXTURE_SET_CLUT_BASE),
            log_stub(log, rv, 77, 3),
        ),
        (
            Some(task::CH_TEXTURE_SET_IMAGE_BASE),
            log_stub(log, rv, 78, 3),
        ),
        (Some(task::CH_DRAW_EN_EFFECT), log_stub(log, rv, 79, 2)),
        (Some(guide::MS_DEBUG_DRAW_BOARD), log_stub(log, rv, 82, 0)),
        (Some(guide::MS_BATTLE_GET_NUM), log_stub(log, bn, 83, 0)),
    ];
    t.cave_from(env.cave_end, &list);
    let vf_args = t.b.rebase(vu0::VU0_SCALE) + 4;
    let skip = [(vf_args, 12u32)];
    let slot = t.alloc(4);
    t.set(task::main::ENV_SLOT, slot);
    let (chunk, parse, data, out) = (t.alloc(8), t.alloc(0x90), t.alloc(0x200), t.alloc(0x100));
    let words = ["alpha", "Beta", "gamma", "be", "x"];
    let table = t.alloc(4 * (words.len() + 1));
    for (i, w) in (0u32..).zip(words) {
        let s = t.alloc(8);
        let mut b = w.as_bytes().to_vec();
        b.push(0);
        t.write(s, &b);
        t.put(table + 4 * i, s);
    }
    let names = t.alloc(0x40);
    for i in 0..4u32 {
        t.write(names + 16 * i, format!("map{i}\0").as_bytes());
        t.put(names + 16 * i + 0xc, 0x100 + i);
    }
    let rw = t.alloc(0x400);
    t.set(rcui::WORK, rw);
    for k in 0..30u32 {
        env.reset(&mut t, &mut rng);
        // the parser
        let mut text = Vec::new();
        for _ in 0..=rng.below(6) {
            let w = match rng.below(4) {
                0 => words[rng.below(5) as usize].to_uppercase(),
                1 => format!("{}.{}", rng.range(-300, 300), rng.below(100)),
                2 => "zeta".to_string(),
                _ => words[rng.below(5) as usize].to_string(),
            };
            text.extend(w.bytes());
            text.push(if rng.below(3) == 0 { b'\n' } else { b' ' });
        }
        text.extend(b"\n1.5\n\0");
        t.write(data, &text);
        parser_over(&mut t, chunk, parse, data);
        if k % 3 != 0 {
            t.put(parse + 8, 1u32);
        }
        t.same(
            "rcdtParseGetKeyword",
            |o| o.eax(0x0092_be40, &[parse, table]),
            |m| rcdt::bridge::rcdt_parse_get_keyword(m, parse, table, call),
        );
        t.same(
            "rcdtParseGetFloat",
            |o| o.float(0x0092_bbb0, &[parse]),
            |m| bits(rcdt::bridge::rcdt_parse_get_float(m, parse, fcall)),
        );
        t.same(
            "rcdtParseGetText",
            |o| o.eax(0x0092_bdd0, &[parse, out]),
            |m| rcdt::bridge::rcdt_parse_get_text(m, parse, out, call),
        );
        t.same(
            "rcdtParseAdvanceLine",
            |o| o.void(0x0092_b9b0, &[parse]),
            |m| rcdt::bridge::rcdt_parse_advance_line(m, parse),
        );
        // deleters
        let tm = rcdt::bridge::rcdt_tm2_new(&mut t.a, call);
        rcdt::bridge::rcdt_tm2_new(&mut t.b, call);
        t.put(
            tm + 0x40,
            if rng.below(2) == 0 { 0 } else { rng.next() | 1 },
        );
        t.same(
            "rcdtTm2Delete",
            |o| o.eax(0x0092_3450, &[tm]),
            |m| rcdt::bridge::rcdt_tm2_delete(m, tm, call),
        );
        let l = model::bridge::rc_alloc(&mut t.a, call, 0x20);
        model::bridge::rc_alloc(&mut t.b, call, 0x20);
        t.write(l, &[0; 0x20]);
        t.put(l + 4, rng.next() | 1);
        t.same(
            "rcdtListsDelete",
            |o| o.eax(0x0092_42d0, &[l]),
            |m| model::bridge::rcdt_lists_delete(m, l, call),
        );
        // vectors and matrices
        let m0 = t.alloc(0x40);
        for i in 0..16 {
            t.put(m0 + 4 * i, float_in(&mut rng, -500, 500));
        }
        t.same_but(
            "rcefVecToField",
            &skip,
            |o| o.void(0x0092_d7e0, &[m0]),
            |m| vu0::bridge::rcef_vec_to_field(m, m0),
        );
        t.same_but(
            "rcbgScaleMatrix100",
            &skip,
            |o| o.void(0x0091_ffd0, &[m0]),
            |m| vu0::bridge::rcbg_scale_rows(m, m0, vu0::SCALE_100),
        );
        t.same_but(
            "rcefScaleMatrix100",
            &skip,
            |o| o.void(0x0092_e630, &[m0]),
            |m| vu0::bridge::rcbg_scale_rows(m, m0, vu0::SCALE_100),
        );
        let (s, a, b, c) = (rng.float(), rng.next(), rng.next(), rng.next());
        t.same(
            "rcuiScaleVector",
            |o| o.eax(0x0093_0f30, &[out, m0, s, a, b, c]),
            |m| vu0::bridge::rcui_scale_vector(m, out, m0, s, [a, b, c]),
        );
        t.same(
            "rcefGetVf00",
            |o| o.eax(0x0092_5020, &[out]),
            |m| vu0::bridge::rcef_get_vf00(m, out),
        );
        let (chr, elem) = (rng.next(), rng.below(8));
        t.same_but(
            "rcbgFPmGetElemMatrix",
            &skip,
            |o| o.void(0x0092_2350, &[m0, chr, elem]),
            |m| model::bridge::rcbg_fpm_get_elem_matrix(m, m0, chr, elem, call),
        );
        let fp = t.alloc(0x40);
        for i in 0..16 {
            t.put(fp + 4 * i, float_in(&mut rng, -10, 10));
        }
        let work = t.stack_scratch(0);
        t.same_but(
            "rcbgFPmGetMatrix",
            &skip,
            |o| o.eax(0x0092_2290, &[fp, m0]),
            |m| model::bridge::rcbg_fpm_get_matrix(m, fp, m0, work, call),
        );
        let mng = t.alloc(0x50);
        t.put(mngv, if rng.below(3) == 0 { 0 } else { mng });
        t.same(
            "rcbgFPmRestart",
            |o| {
                let r = o.eax(0x0092_2300, &[7, chr, elem]);
                if Mem::read::<u32>(&*o.0, mngv) == 0 {
                    0
                } else {
                    r
                }
            },
            |m| model::bridge::rcbg_fpm_restart(m, 7, chr, elem, work, call),
        );
        // map reads by kind, names and paths
        let (map, alarm) = (rng.below(0x400), rng.below(6));
        t.same(
            "yiFieldMapRead",
            |o| o.void(0x0090_85d0, &[map, alarm]),
            |m| yi::bridge::yi_field_map_read(m, map, alarm, call),
        );
        t.same(
            "yiBattleMapRead",
            |o| o.void(0x0090_8370, &[map, alarm]),
            |m| yi::bridge::yi_battle_map_read(m, map, alarm, call),
        );
        t.set(yi::MAP_NAMES, names);
        t.set(yi::MAP_NAME_COUNT, rng.below(5));
        t.set(
            yi::CURRENT_MAP,
            [0, 0x100, 0x102, 0x103, 0x200][rng.below(5) as usize],
        );
        t.same(
            "yiGetMapName",
            |o| o.eax(0x0090_7e70, &[out]),
            |m| yi::bridge::yi_get_map_name(m, out, call),
        );
        t.put(rw + 0x64, rng.below(4));
        t.same(
            "rcuiMakeBasePath",
            |o| o.eax(0x0093_7af0, &[]),
            |m| rcui::bridge::rcui_make_base_path(m, call),
        );
        let hw = t.alloc(4);
        t.put(hw, if rng.below(2) == 0 { 0 } else { rng.next() });
        t.same(
            "rcuiCloseWindow",
            |o| o.void(0x0093_7f50, &[hw]),
            |m| rcui::bridge::rcui_close_window(m, hw, call),
        );
        t.fill(rw + 0x40, 0x300, &mut rng);
        t.set(yi::RENDER_STATUS, rng.next());
        t.same(
            "rcuiReset",
            |o| o.void(0x0093_6570, &[]),
            rcui::bridge::rcui_reset,
        );
        // transfers, rcfx
        let tr = t.alloc(0x40);
        t.fill(tr, 0x40, &mut rng);
        t.put(tr + 0x34, 0x40u32);
        let regs = [
            rng.next(),
            rng.next(),
            [0u32, 0x13, 0x14][rng.below(3) as usize],
            rng.next(),
            rng.next(),
            rng.below(32),
            rng.below(32),
        ];
        let n = rng.below(4);
        let mut args = vec![tr];
        args.extend(regs);
        args.push(n);
        t.same(
            "rcbgTrxSetRegsAlloc",
            |o| o.void(0x0092_d320, &args),
            |m| model::bridge::rcbg_trx_set_regs_alloc(m, tr, regs, n, call),
        );
        t.same(
            "rcfxSpringNew",
            |o| o.eax(0x0092_8d50, &[]),
            |m| model::bridge::rcfx_spring_new(m, call),
        );
        let ps = t.alloc(0x40);
        for o in [0x24u32, 0x28, 0x2c, 0x30] {
            t.put(ps + o, if rng.below(2) == 0 { 0 } else { rng.next() | 1 });
        }
        t.put(ps + 0x34, 0u32);
        t.put(ps + 0x38, 0u32);
        t.same(
            "rcfxParticleFini",
            |o| o.void(0x0092_8870, &[ps]),
            |m| model::bridge::rcfx_particle_fini(m, ps, call),
        );
        let (psim, ps) = (t.alloc(0x20), t.alloc(0x40));
        t.put(psim + 0x18, ps);
        let k1 = rng.float();
        t.same(
            "rcbgPSimAddDamp",
            |o| o.void(0x0091_f7a0, &[psim, k1]),
            |m| model::bridge::rcbg_psim_add_damp(m, psim, k1, call),
        );
        let (dir, pos) = (t.alloc(0x10), t.alloc(0x10));
        t.fill(dir, 0x10, &mut rng);
        t.fill(pos, 0x10, &mut rng);
        let power = rng.float();
        t.same(
            "rcbgPSimAddWind",
            |o| o.void(0x0091_f980, &[psim, dir, pos, power]),
            |m| model::bridge::rcbg_psim_add_wind(m, psim, dir, pos, power, call),
        );
        let g = t.alloc(0x10);
        t.fill(g, 0x10, &mut rng);
        t.same(
            "rcbgPSimAddGravity",
            |o| o.void(0x0091_f7c0, &[psim, g]),
            |m| model::bridge::rcbg_psim_add_gravity(m, psim, g, call),
        );
        // a palette through the dispatcher
        let (pal, trx) = (t.alloc(0x10), t.alloc(0x40));
        let entries = 2 + rng.below(3);
        let pd = t.alloc(0x410 * entries as usize);
        t.fill(pd, 0x410 * entries as usize, &mut rng);
        t.put(trx + 0x30, entries);
        t.put(trx + 0x34, 0x410u32);
        t.put(trx + 0x38, pd);
        t.put(pal, u8::try_from(rng.below(4)).unwrap());
        t.put(pal + 0xc, trx);
        t.put(pal + 4, u16::try_from(rng.below(5)).unwrap());
        t.put(pal + 6, u16::try_from(rng.below(0x100)).unwrap());
        t.put(pal + 8, u16::try_from(rng.below(entries - 1)).unwrap());
        t.put(pal + 0xa, u8::try_from(rng.below(4)).unwrap());
        t.put(pal + 0xb, u8::try_from(rng.below(4)).unwrap());
        t.same(
            "rcbgPalProc",
            |o| o.void(0x0092_9b60, &[pal]),
            |m| model::bridge::rcbg_pal_proc(m, pal),
        );
        let page = rng.next();
        // rcbgSetReg is a bare `ret`: the result is the record's stack
        // address
        t.same(
            "rcbgModelSetTexReg",
            |o| o.void(0x0092_84d0, &[page]),
            |m| {
                model::bridge::rcbg_model_set_tex_reg(m, page, work, call);
            },
        );
        // effect tasks
        let (e, part, arg) = (rng.next(), rng.next(), rng.next());
        t.same(
            "rcefRunPart",
            |o| o.eax(0x0092_eec0, &[e, part, arg]),
            |m| task::bridge::rcef_run_part(m, e, part, arg, call),
        );
        let obj = t.alloc(0x490);
        t.put(block, obj);
        t.same(
            "rcefObjNew",
            |o| o.eax(0x0093_0030, &[]),
            |m| task::bridge::rcef_obj_new(m, call),
        );
        // a task list: the head at ctrl, the tail at ctrl + 0xbc
        let ctrl = t.alloc(0x200);
        let n = rng.below(4);
        let tasks: Vec<u32> = (0..n).map(|_| t.alloc(0xbc)).collect();
        let mut nodes = vec![ctrl];
        nodes.extend(&tasks);
        nodes.push(ctrl + 0xbc);
        for w in nodes.windows(2) {
            t.put(w[0] + 8, w[1]);
            t.put(w[1] + 4, w[0]);
        }
        for &x in &tasks {
            t.put(x + 0xb4, if rng.below(2) == 0 { 0 } else { del });
        }
        t.same(
            "rcefTaskCtrlFini",
            |o| o.void(0x0093_0260, &[ctrl]),
            |m| task::bridge::rcef_task_ctrl_fini(m, ctrl, call),
        );
        // a chain to run: dead tasks in the middle are killed
        let n = 1 + rng.below(4);
        let tasks: Vec<u32> = (0..n).map(|_| t.alloc(0xbc)).collect();
        let before = t.alloc(0xbc);
        for (i, &x) in tasks.iter().enumerate() {
            let at_end = i + 1 == tasks.len();
            t.put(
                x,
                if !at_end && rng.below(2) == 0 {
                    4u32
                } else {
                    1
                },
            );
            t.put(x + 4, if i == 0 { before } else { tasks[i - 1] });
            t.put(x + 8, if at_end { 0 } else { tasks[i + 1] });
            t.put(x + 0xac, 0u32);
            t.put(x + 0xc, if rng.below(2) == 0 { 0 } else { del });
            t.put(x + 0xb0, 0u32);
            t.put(x + 0xb4, 0u32);
        }
        t.same(
            "rcefTaskCtrlProc",
            |o| o.void(0x0093_0300, &[tasks[0]]),
            |m| task::bridge::rcef_task_ctrl_proc(m, tasks[0], call),
        );
        let ch = t.alloc(0x100);
        t.fill(ch, 0x10, &mut rng);
        if rng.below(2) == 0 {
            t.put(ch + 8, 0u32);
        }
        t.same(
            "rcefChrFini",
            |o| o.void(0x0093_0710, &[ch]),
            |m| task::bridge::rcef_chr_fini(m, ch, call),
        );
        // the scene's screen, the pre-loaded map's field objects
        if guide::YN_MAP_SYSTEM.get(&t.b) == 0 {
            t.same(
                "rcbgInit",
                |o| o.void(0x0092_1ce0, &[]),
                |m| guide::bridge::rcbg_init(m, call),
            );
        }
        t.same(
            "rcbgPreDummyInit",
            |o| o.void(0x0092_1640, &[]),
            |m| guide::bridge::rcbg_pre_dummy_init(m, call),
        );
        let roll = rng.float();
        t.same(
            "Yn_PreBgSetScrBank",
            |o| o.void(0x0091_ad10, &[roll]),
            |m| guide::bridge::yn_pre_bg_set_scr_bank(m, roll, call),
        );
        let (x, y) = (rng.float(), rng.float());
        t.same(
            "Yn_PreBgSetScrOffset",
            |o| o.void(0x0091_ad40, &[x, y]),
            |m| guide::bridge::yn_pre_bg_set_scr_offset(m, x, y, call),
        );
        let sys = guide::YN_MAP_SYSTEM.get(&t.b);
        t.put(sys + 8, rng.below(2));
        t.put(
            bn,
            [0, 0x0061_0000, 0x0235_0000, 0x0100_0000][rng.below(4) as usize],
        );
        t.same(
            "rcbgPreAddFpObjects",
            |o| o.void(0x0092_2010, &[]),
            |m| guide::bridge::rcbg_pre_add_fp_objects(m, call),
        );
        let fs = Mem::read::<u32>(&t.b, Mem::read::<u32>(&t.b, sys) + 8);
        let nth = rng.below(2);
        t.same(
            "Yn_FpTrxTexGet",
            |o| o.eax(0x0091_9ba0, &[nth]),
            |m| guide::bridge::yn_fp_trx_tex_get(m, nth),
        );
        let tex = t.alloc(0xa0);
        t.fill(tex, 0xa0, &mut rng);
        let (image, clut) = (rng.next(), rng.next());
        t.same(
            "Yn_FpTrxTexSetAddr",
            |o| o.void(0x0091_9be0, &[tex, image, clut]),
            |m| guide::bridge::yn_fp_trx_tex_set_addr(m, tex, image, clut),
        );
        let kind = Mem::read::<u32>(&t.b, fs + scene::layout::OBJECT_COUNT);
        let kind = if kind == 0 {
            0x2_0006
        } else {
            let first = Mem::read::<u32>(&t.b, Mem::read::<u32>(&t.b, fs + scene::layout::OBJECTS));
            Mem::read::<u32>(&t.b, first)
        };
        t.same(
            "rcbgSceneDelObjects",
            |o| o.void(0x0092_06f0, &[fs, kind]),
            |m| scene::bridge::rcbg_scene_del_objects(m, fs, kind, call),
        );
    }
}

/// `_CIacos`'s import slot, which the oracle does not stub.
const CIACOS_SLOT: u32 = 0x00b0_c3c4;
/// `_CIacos` as `atan2(sqrt(1 - x²), x)` on the x87: `fld st0; fmul
/// st0, st0; fld1; fsubrp st1; fsqrt; fxch; fpatan; ret`.
const CIACOS_STUB: [u8; 15] = [
    0xd9, 0xc0, 0xd8, 0xc8, 0xd9, 0xe8, 0xde, 0xe1, 0xd9, 0xfa, 0xd9, 0xc9, 0xd9, 0xf3, 0xc3,
];

/// A `pppFpGetMngPtr(id, &data, &mng)` stand-in storing the word at
/// `value` through whichever of its pointers is not null.
fn store_args_stub(value: u32) -> Vec<u8> {
    let mut c = vec![0x8b, 0x0d];
    c.extend_from_slice(&value.to_le_bytes());
    c.extend_from_slice(&[
        0x8b, 0x44, 0x24, 0x08, 0x85, 0xc0, 0x74, 0x02, 0x89, 0x08, 0x8b, 0x44, 0x24, 0x0c, 0x85,
        0xc0, 0x74, 0x02, 0x89, 0x08, 0xc3,
    ]);
    c
}

/// Round 11: the two-operand intrinsic (`_CIfmod`, the oracle's `fprem`
/// stub) in the angle wrap, `_CIacos`/`_CIsin`/`_CIcos` angle helpers,
/// effect mains and work, palette allocation and configuration, the
/// field program's part data, text sub-config keywords, the renderer's
/// battle view, view matrices and lights, image GS allocation, rcui motion
/// steps, numbers and the guide save size.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round11_field_helpers_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x000b_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    t.install_trig();
    for o in [&mut t.a, &mut t.b] {
        let at = o.rebase(CODE_AREA + 0x160);
        o.write(at, &CIACOS_STUB).unwrap();
        let s = o.rebase(CIACOS_SLOT);
        o.write(s, &at.to_le_bytes()).unwrap();
    }
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let fcall = &mut oracle_fcall;
    let (log, rv) = (env.log, env.rv);
    let (mngv, block, depth) = (t.alloc(4), t.alloc(4), t.alloc(4));
    // MsCameraGetScrDpt: `fld dword [depth]; ret` (the image has no camera)
    let mut dpt = vec![0xd9, 0x05];
    dpt.extend_from_slice(&depth.to_le_bytes());
    dpt.push(0xc3);
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(yi::MS_CAMERA_GET_SCR_DPT), dpt),
        (Some(model::CH_OPTPOS_GET_ELEMENT), log_stub(log, rv, 90, 3)),
        (Some(model::PPP_FP_GET_MNG_PTR), store_args_stub(mngv)),
        (Some(model::FP_RESTART), log_stub(log, rv, 91, 1)),
        (Some(model::CH_ELEM_GET_MATRIX), log_stub(log, rv, 92, 3)),
        (Some(task::PPP_MEM_ALLOC), log_stub(log, block, 93, 3)),
        (Some(task::PPP_MEM_FREE), log_stub(log, rv, 94, 3)),
    ];
    t.cave_from(env.cave_end, &list);
    let slot = t.alloc(4);
    t.set(task::main::ENV_SLOT, slot);
    let (chunk, parse, data, out) = (t.alloc(8), t.alloc(0x90), t.alloc(0x200), t.alloc(0x100));
    let rw = t.alloc(0x1000);
    t.set(rcui::WORK, rw);
    for k in 0..30u32 {
        env.reset(&mut t, &mut rng);
        // angles
        let a = match k % 4 {
            0 => float_in(&mut rng, -40, 0),
            1 => float_in(&mut rng, 7, 40),
            2 => float_in(&mut rng, 0, 6),
            _ => rng.float(),
        };
        t.same(
            "rcefAngleWrap",
            |o| o.float(0x0092_d810, &[a]),
            |m| bits(vu0::bridge::rcef_angle_wrap(m, a, fcall)),
        );
        let v = t.alloc(0x10);
        for i in 0..4 {
            t.put(v + 4 * i, float_in(&mut rng, -1, 1));
        }
        t.same(
            "rcefVecToAngle",
            |o| o.float(0x0092_ec20, &[v]),
            |m| bits(vu0::bridge::rcef_vec_to_angle(m, v, fcall)),
        );
        let a = float_in(&mut rng, -7, 7);
        t.same(
            "rcefAngleToDir",
            |o| o.void(0x0092_ded0, &[out, a]),
            |m| vu0::bridge::rcef_angle_to_dir(m, out, a, fcall),
        );
        let (s, x, y, z) = (rng.float(), rng.next(), rng.next(), rng.next());
        t.same(
            "rcuiScaleVector_00937610",
            |o| o.eax(0x0093_7610, &[out, v, s, x, y, z]),
            |m| vu0::bridge::rcui_scale_vector(m, out, v, s, [x, y, z]),
        );
        // effect mains
        let main = t.alloc(0x200);
        let ctrl = main + 0x1c;
        t.put(ctrl + 8, ctrl + 0xbc);
        t.put(ctrl + 0xbc + 4, ctrl);
        t.put(ctrl + 0xb8, 0u32);
        t.put(ctrl + 0xbc + 0xb8, 0xffffu32);
        let obj = t.alloc(0x490);
        t.put(block, obj);
        t.same(
            "rcefMainCreateObj",
            |o| o.eax(0x0092_5140, &[main]),
            |m| task::bridge::rcef_main_create_obj(m, main, call),
        );
        t.put(main + 0x10, rng.next());
        t.put(main + 0x18, if rng.below(2) == 0 { 0 } else { rng.next() });
        t.same(
            "rcefMainFini",
            |o| o.void(0x0092_51c0, &[main]),
            |m| task::bridge::rcef_main_fini(m, main, call),
        );
        let (w, b) = (t.alloc(0x200), t.alloc(0x10));
        t.fill(b, 0x10, &mut rng);
        let val = rng.next();
        t.same(
            "rcefWorkSetFromVf00",
            |o| o.void(0x0092_55d0, &[w, b, val]),
            |m| task::bridge::rcef_work_set_from_vf00(m, w, b, val),
        );
        // palettes and particles
        let pal = t.alloc(0x10);
        let entries = rng.below(4);
        t.same(
            "rcbgPalAlloc",
            |o| o.void(0x0092_9950, &[pal, entries]),
            |m| model::bridge::rcbg_pal_alloc(m, pal, entries, call),
        );
        let ps = model::bridge::rc_alloc(&mut t.a, call, 0x40);
        model::bridge::rc_alloc(&mut t.b, call, 0x40);
        t.write(ps, &[0; 0x40]);
        t.same(
            "rcfxParticleDelete",
            |o| o.eax(0x0092_8850, &[ps]),
            |m| model::bridge::rcfx_particle_delete(m, ps, call),
        );
        // field programs
        let fp = t.alloc(0x30);
        t.put(fp + 0x1c, if rng.below(2) == 0 { 0x7c } else { 3 });
        t.put(fp + 0x10, rng.next());
        t.put(fp + 0x20, 9u32);
        let mng = t.alloc(0x50);
        t.put(mngv, if rng.below(4) == 0 { 0 } else { mng });
        let work = t.stack_scratch(0);
        t.same(
            "rcbgFPm173ProcSphere",
            |o| o.void(0x0092_2eb0, &[fp]),
            |m| model::bridge::rcbg_fpm173_proc_sphere(m, fp, work, call),
        );
        // part data: a chain of nodes, a table, parts and offsets
        let nodes: Vec<u32> = (0..3).map(|_| t.alloc(0x40)).collect();
        for (i, &nd) in nodes.iter().enumerate() {
            t.put(nd, nodes.get(i + 1).copied().unwrap_or(0));
            let table = t.alloc(4);
            let e = t.alloc(0x80);
            let offs = t.alloc(0x10);
            for j in 0..4 {
                t.put(offs + 4 * j, rng.below(0x100));
            }
            for p in 0..4 {
                t.put(e + 0x28 + 0x10 * p + 0xc, offs);
            }
            t.put(table, if rng.below(5) == 0 { 0 } else { e });
            t.put(nd + 8, if rng.below(6) == 0 { 0 } else { table });
        }
        t.put(mng + 0x30, nodes[0]);
        let (n, part, kk) = (rng.range(-1, 4), rng.below(4), rng.below(4));
        t.same(
            "rcbgFPmGetPartData",
            |o| o.eax(0x0092_3190, &[1, n.cast_unsigned(), part, kk]),
            |m| model::bridge::rcbg_fpm_get_part_data(m, 1, n, part, kk, work, call),
        );
        // keyword configs
        let kw = |rng: &mut Xorshift, t: &Twin, table: u32| -> String {
            let n = (0..8)
                .take_while(|&i| Mem::read::<u32>(&t.b, t.b.rebase(table) + 4 * i) != 0)
                .count();
            let i = rng.below(u32::try_from(n).unwrap() + 1);
            if i as usize == n {
                return "junk".to_string();
            }
            let s = Mem::read::<u32>(&t.b, t.b.rebase(table) + 4 * i);
            let mut out = String::new();
            for j in 0.. {
                let c = Mem::read::<u8>(&t.b, s + j);
                if c == 0 {
                    break;
                }
                out.push(char::from(c));
            }
            out
        };
        for (table, f) in [
            (model::PAL_CONFIG_KEYS, 0x0093_adf0u32),
            (model::TEXT_SUB_KEYS, 0x0093_aee0),
        ] {
            let mut text = b"head ".to_vec();
            for _ in 0..rng.below(6) {
                let w = kw(&mut rng, &t, table);
                text.extend(w.bytes());
                text.extend(format!(" {} ", rng.range(-300, 300)).bytes());
            }
            text.push(0);
            t.write(data, &text);
            parser_over(&mut t, chunk, parse, data);
            // a bounded chunk: past trailing separators an unbounded scan
            // runs on through memory (NUL is a separator)
            t.put(chunk + 4, u32::try_from(text.len() - 1).unwrap());
            t.put(parse + 8, 1u32);
            let o = t.alloc(0x80);
            if f == 0x0093_adf0 {
                t.same(
                    "rcbgPalLoadConfig",
                    |or| or.void(f, &[o, parse]),
                    |m| model::bridge::rcbg_pal_load_config(m, o, parse, call),
                );
            } else {
                t.same(
                    "rcbgSceneLoadTextSubConfigKeys",
                    |or| or.void(f, &[o, parse]),
                    |m| model::bridge::rcbg_scene_load_text_sub_keys(m, o, parse, call),
                );
            }
        }
        // the renderer's views and lights
        let (mm, view) = (t.alloc(0x40), t.alloc(0x40));
        for i in 0..16 {
            t.put(mm + 4 * i, float_in(&mut rng, -10, 10));
            t.put(view + 4 * i, float_in(&mut rng, -10, 10));
        }
        t.set(yi::RENDER_STATUS, rng.next());
        t.put(depth, float_in(&mut rng, 100, 1000));
        let vw = if rng.below(4) == 0 { 0 } else { view };
        let mwork = t.stack_scratch(0x100);
        t.same(
            "yiSetBattleView",
            |o| o.void(0x0090_a140, &[vw]),
            |m| yi::bridge::yi_set_battle_view(m, vw, mwork, call, fcall),
        );
        let mx = if rng.below(3) == 0 { 0 } else { mm };
        t.same(
            "yiSetViewMatrices",
            |o| o.void(0x0090_a640, &[mx, vw]),
            |m| yi::bridge::yi_set_view_matrices(m, mx, vw, mwork, call, fcall),
        );
        let lwork = t.stack_scratch(0x200);
        t.same(
            "yiSetLightMatrices",
            |o| o.eax(0x0090_8d00, &[]),
            |m| yi::bridge::yi_set_light_matrices(m, lwork, call),
        );
        let img = t.alloc(0x20);
        t.fill(img, 0x20, &mut rng);
        t.put(
            img + 0xc,
            [0u32, 1, 2, 0x13, 0x14, 7][rng.below(6) as usize],
        );
        if rng.below(2) == 0 {
            t.set(yi::IMAGE_PAGE_CACHE, u32::MAX);
        }
        t.same(
            "yiImageVramAlloc",
            |o| o.eax(0x0091_7d80, &[img]),
            |m| yi::bridge::yi_image_vram_alloc(m, img),
        );
        // rcui
        t.put(rw + 0x8d8, rng.below(0x69));
        let d = rng.range(-30, 30);
        t.same(
            "rcuiMotionStep",
            |o| o.void(0x0093_5690, &[d.cast_unsigned()]),
            |m| rcui::bridge::rcui_motion_step(m, d),
        );
        let s1 = format!("name{}\0", rng.below(1000));
        t.write(rw + 0x174, s1.as_bytes());
        t.write(rw + 0x27c, b"slot99\0");
        let n = rng.range(0, 100);
        t.same(
            "rcuiSetNumber",
            |o| o.void(0x0093_7f70, &[n.cast_unsigned()]),
            |m| rcui::bridge::rcui_set_number(m, n),
        );
        let g = t.alloc(0x10);
        let maps = rng.below(4);
        let ms = t.alloc(0xc0 * maps.max(1) as usize);
        for i in 0..maps {
            let e = ms + 0xc0 * i + 0x98;
            let (t1, t2) = (t.alloc(0x40), t.alloc(0x40));
            for tr in [t1, t2] {
                t.put(tr + 0x30, rng.below(4));
                t.put(tr + 0x34, rng.below(0x400));
            }
            t.put(e, rng.range(-2, 20));
            t.put(e + 8, t1);
            t.put(e + 0x24, t2);
        }
        t.put(g + 4, maps);
        t.put(g + 8, ms);
        t.same(
            "rcuiGuideSaveSize",
            |o| o.eax(0x0093_96a0, &[g]),
            |m| rcui::bridge::rcui_guide_save_size(m, g),
        );
        // removing objects from the current scene
        if guide::YN_MAP_SYSTEM.get(&t.b) == 0 {
            t.same(
                "rcbgInit",
                |o| o.void(0x0092_1ce0, &[]),
                |m| guide::bridge::rcbg_init(m, call),
            );
        }
        t.same(
            "rcbgPreDummyInit",
            |o| o.void(0x0092_1640, &[]),
            |m| guide::bridge::rcbg_pre_dummy_init(m, call),
        );
        let kind = 0x2_0000 + rng.below(8);
        t.same(
            "rcbgDelObjects",
            |o| o.void(0x0092_19e0, &[kind]),
            |m| guide::bridge::rcbg_del_objects(m, kind, call),
        );
    }
}

/// Round 12: random vectors (the real `op_random`/`op_random_p` on VU0's
/// `R`), the pmcom environment, the RGBA resampler, `KeThBP_Init`, two
/// `KeVu0` helpers, the sphere grid's cursor, save block, player tag and
/// level values, the rcui motion setup, the text camera's reference
/// point, and the renderer's direct block, triangle test and bounds.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round12_field_helpers_match_the_original() {
    use ffx_field::abmap;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x000c_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    t.install_trig();
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let fcall = &mut oracle_fcall;
    let (log, rv) = (env.log, env.rv);
    let nbv = t.alloc(4);
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(ke::PPP_ALL_FREE_POBJECT), log_stub(log, rv, 100, 1)),
        (Some(ke::PPP_INIT_ENV), log_stub(log, rv, 101, 4)),
        (Some(rcui::CH_GET_NB_ELEM), log_stub(log, nbv, 102, 1)),
        (Some(rcui::CH_SET_MOTION_SPEED), log_stub(log, rv, 103, 2)),
    ];
    t.cave_from(env.cave_end, &list);
    let slot = t.alloc(4);
    t.set(task::main::ENV_SLOT, slot);
    let grid = t.alloc(0x1_2000);
    t.set(abmap::ABMAP, grid);
    let rw = t.alloc(0x1000);
    t.set(rcui::WORK, rw);
    let render = t.alloc(0xba400);
    let (blocks, tags, src) = (t.alloc(0x800), t.alloc(0x400), t.alloc(0x100));
    let (dst, v, out) = (t.alloc(0x10), t.alloc(0x10), t.alloc(0x10));
    let (img, pix) = (t.alloc(0x400), t.alloc(0x1000));
    let (bp, args) = (t.alloc(0x40 + 0x30 * 256 + 0x10), t.alloc(8));
    let (cam, tag, tri, p) = (t.alloc(0x100), t.alloc(0x40), t.alloc(0x30), t.alloc(0x10));
    let (obj, min, max) = (t.alloc(0x200), t.alloc(0x10), t.alloc(0x10));
    let count = t.b.rebase(ke::PPP_OBJECT_COUNT);
    for k in 0..30u32 {
        env.reset(&mut t, &mut rng);
        // random vectors
        for i in 0..4 {
            t.put(dst + 4 * i, float_in(&mut rng, -8, 8));
            t.put(v + 4 * i, float_in(&mut rng, -8, 8));
        }
        let scaled = rng.below(2);
        t.same(
            "KeVecAddRandom",
            |o| o.void(0x0076_6f10, &[dst, v, scaled]),
            |m| ke::bridge::ke_vec_add_random(m, dst, v, scaled, fcall),
        );
        t.same(
            "KeVecScaleRandom",
            |o| o.eax(0x0076_6f70, &[dst, v, scaled]),
            |m| ke::bridge::ke_vec_scale_random(m, dst, v, scaled, fcall),
        );
        // the pmcom environment
        t.put(count, rng.range(-1, 3));
        t.same(
            "pppFreeAllEnvObjects",
            |o| o.void(0x0076_05d0, &[]),
            |m| ke::bridge::ppp_free_all_env_objects(m, call),
        );
        t.same(
            "pppInitPmcomEnv",
            |o| o.eax(0x0076_0610, &[]),
            |m| ke::bridge::ppp_init_pmcom_env(m, call),
        );
        let (sw, sh) = (rng.range(-1, 12), rng.range(-1, 12));
        let (dw, dh) = (rng.range(1, 12), rng.range(1, 12));
        t.fill(img, 0x400, &mut rng);
        let d = pix + 0x400;
        let a = [
            img,
            sw.cast_unsigned(),
            sh.cast_unsigned(),
            d,
            dw.cast_unsigned(),
            dh.cast_unsigned(),
        ];
        t.same(
            "KeResampleRgbaToRgb",
            |o| o.eax(0x0076_0640, &a),
            |m| ke::bridge::ke_resample_rgba_to_rgb(m, img, sw, sh, d, dw, dh),
        );
        t.fill(bp, 0x80, &mut rng);
        t.put(args, bp);
        t.put(args + 4, rng.next());
        t.same(
            "KeThBP_Init",
            |o| o.eax(0x0076_3030, &[args]),
            |m| ke::bridge::ke_th_bp_init(m, args),
        );
        // KeVu0
        let s = float_in(&mut rng, -4, 4);
        t.same(
            "KeVu0ScaleVecXY",
            |o| o.eax(0x0075_fab0, &[out, v, s]),
            |m| vu0::bridge::ke_vu0_scale_vec_xy(m, out, v, s),
        );
        t.same(
            "KeVu0SubVecXYZ",
            |o| o.eax(0x0075_fcd0, &[out, dst, v]),
            |m| vu0::bridge::ke_vu0_sub_vec_xyz(m, out, dst, v),
        );
        // the sphere grid
        t.fill(grid + 0x800, 0x400, &mut rng);
        t.fill(grid + 0xa800, 0x400, &mut rng);
        t.fill(grid + 0x11000, 0x700, &mut rng);
        for i in 0..7 * 0x14 {
            t.put(grid + 0x11088 + 4 * i, float_in(&mut rng, -100, 100));
        }
        t.put(grid + 0x115bc, u8::try_from(rng.below(7)).unwrap());
        t.put(grid + 2, i16::try_from(rng.range(-1, 40)).unwrap());
        t.put(grid + 4, i16::try_from(rng.range(-1, 40)).unwrap());
        t.put(grid + 0x11350, float_in(&mut rng, 1, 4));
        t.same(
            "AbmapResetCursorCamera",
            |o| o.eax(0x00a5_b030, &[]),
            abmap::bridge::abmap_reset_cursor_camera,
        );
        t.same(
            "AbmapStoreSave",
            |o| o.eax(0x00a5_bb70, &[]),
            |m| abmap::bridge::abmap_store_save(m, call),
        );
        t.put(tag + 0x30, i16::try_from(rng.range(-300, 300)).unwrap());
        t.put(tag + 0x32, i16::try_from(rng.range(-300, 300)).unwrap());
        t.same(
            "AbmapSetupPlyTag",
            |o| o.eax(0x00a4_7c60, &[tag]),
            |m| abmap::bridge::abmap_setup_ply_tag(m, tag),
        );
        let (level, i, j) = (rng.range(-60, 120), rng.below(5), rng.below(10));
        t.same(
            "AbmapScaledLevelValue",
            |o| o.eax(0x00a5_cbe0, &[level.cast_unsigned(), i, j]),
            |m| abmap::bridge::abmap_scaled_level_value(m, level, i, j),
        );
        // rcui
        t.put(rw + 0x8b0, rng.below(4));
        for i in 0..4 {
            let h = if rng.below(3) == 0 {
                u32::MAX
            } else {
                rng.below(0x40)
            };
            t.put(rw + 0x870 + 4 * i, h);
        }
        let old = if rng.below(2) == 0 {
            0
        } else {
            let p = model::bridge::rc_alloc(&mut t.a, call, 0x20);
            model::bridge::rc_alloc(&mut t.b, call, 0x20);
            p
        };
        t.put(rw + 0x8c8, old);
        t.put(nbv, rng.below(8));
        t.same(
            "rcuiChrMotionSetup",
            |o| o.eax(0x0093_2470, &[]),
            |m| rcui::bridge::rcui_chr_motion_setup(m, call),
        );
        // the text camera
        for o in (0x68..0x74).step_by(4).chain([0xf4]) {
            t.put(cam + o, float_in(&mut rng, -50, 50));
        }
        for o in [0x98u32, 0x9c, 0xa0] {
            t.put(cam + o, float_in(&mut rng, -360, 360));
        }
        let work = t.stack_scratch(0);
        // the result is `FFXVu0RotMatrixZ`'s, the matrix's stack address
        t.same(
            "rcbgSceneLoadTextCameraCalcRef",
            |o| o.void(0x0093_b110, &[cam]),
            |m| {
                scene::bridge::rcbg_scene_load_text_camera_calc_ref(m, cam, work, call);
            },
        );
        // the renderer
        t.same(
            "yiImage_dumpall",
            |o| o.eax(0x0091_8680, &[render]),
            |_| yi::bridge::yi_image_dumpall(render),
        );
        t.put(render + 0xba3e0, blocks + rng.below(0x40));
        t.put(render + 0xba3f0, tags + 0x10 * rng.below(8));
        t.put(render + 0xba3f8, rng.next());
        t.fill(src, 0x100, &mut rng);
        let n = rng.below(6);
        t.same(
            "yiRender_addDirectBlock",
            |o| o.eax(0x0090_c120, &[render, src, n]),
            |m| yi::bridge::yi_render_add_direct_block(m, render, src, n),
        );
        for i in 0..12 {
            t.put(tri + 4 * i, float_in(&mut rng, -8, 8));
        }
        for i in 0..4 {
            t.put(p + 4 * i, float_in(&mut rng, -8, 8));
        }
        if k % 5 == 0 {
            // a degenerate triangle
            let x = Mem::read::<u32>(&t.b, tri);
            t.put(tri + 0x10, x);
        }
        t.same(
            "yiTriContainsXZ",
            |o| o.eax(0x0091_0270, &[tri, p]),
            |m| yi::bridge::yi_tri_contains_xz(m, tri, p),
        );
        t.put(obj + 8, 0x20u32);
        t.put(obj + 0x12, i16::try_from(rng.range(-1, 8)).unwrap());
        for i in 0..0x30 {
            t.put(obj + 0x20 + 4 * i, float_in(&mut rng, -100, 100));
        }
        t.same(
            "yiBoundsMinMax",
            |o| o.eax(0x0091_30f0, &[obj, min, max]),
            |m| yi::bridge::yi_bounds_min_max(m, obj, min, max),
        );
    }
}

/// Round 13: particle simulation teardown, the load window's title, the
/// scene's wave, effect particle kicks, key frame angle wrapping and the
/// image VRAM areas (placement, overlap, the last area).
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round13_field_helpers_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x000d_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let log = env.log;
    let (is_end_rv, new_obj_rv, par_end_rv) = (t.alloc(4), t.alloc(4), t.alloc(4));
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (
            Some(rcui::GU_WINDOW_SET_TITLE),
            log_stub_from(log, env.rv, 110, 0, 1),
        ),
        (
            Some(task::TASK_CTRL_IS_END),
            log_stub(log, is_end_rv, 111, 1),
        ),
        (
            Some(task::MAIN_CREATE_OBJ),
            log_stub(log, new_obj_rv, 112, 1),
        ),
        (
            Some(task::OBJ_IS_PAR_END),
            log_stub(log, par_end_rv, 113, 1),
        ),
        (Some(task::MAIN_SET_PAR), log_stub(log, env.rv, 114, 4)),
        (Some(task::OBJ_ATTACH_CHR), log_stub(log, env.rv, 115, 3)),
    ];
    t.cave_from(env.cave_end, &list);
    let rw = t.alloc(0x1000);
    t.set(rcui::WORK, rw);
    let (psim, s, m) = (t.alloc(0x40), t.alloc(0x100), t.alloc(0x40));
    let objs = [t.alloc(0x160), t.alloc(0x160)];
    let vecs = [t.alloc(0x10), t.alloc(0x10), t.alloc(0x10)];
    let (v, page, img) = (t.alloc(0x10), t.alloc(0x20), t.alloc(0x160));
    let imgs: Vec<u32> = (0..4).map(|_| t.alloc(0x160)).collect();
    for k in 0..40u32 {
        env.reset(&mut t, &mut rng);
        // particle simulations
        for (o, n) in [(0x14u32, 0x20u32), (0x18, 0x40)] {
            let p = if rng.below(3) == 0 {
                0
            } else {
                let p = model::bridge::rc_alloc(&mut t.a, call, n);
                model::bridge::rc_alloc(&mut t.b, call, n);
                t.write(p, &vec![0; n as usize]);
                p
            };
            t.put(psim + o, p);
        }
        if k % 2 == 0 {
            t.same(
                "rcbgPSimFini",
                |o| o.eax(0x0091_fd80, &[psim]),
                |m| model::bridge::rcbg_psim_fini(m, psim, call),
            );
        } else {
            let p = model::bridge::rc_alloc(&mut t.a, call, 0x40);
            model::bridge::rc_alloc(&mut t.b, call, 0x40);
            let (a, b) = (
                Mem::read::<u32>(&t.b, psim + 0x14),
                Mem::read::<u32>(&t.b, psim + 0x18),
            );
            t.put(p + 0x14, a);
            t.put(p + 0x18, b);
            t.same(
                "rcbgPSimDelete",
                |o| o.eax(0x0091_fd40, &[p]),
                |m| model::bridge::rcbg_psim_delete(m, p, call),
            );
        }
        // the load window
        t.put(rw + 0x64, rng.below(8));
        t.put(rw + 4, rng.next());
        let buf = t.stack_scratch(0);
        t.same(
            "rcuiLoadSetTitle",
            |o| o.eax(0x0093_4fb0, &[]),
            |m| rcui::bridge::rcui_load_set_title(m, buf, call),
        );
        // the scene's wave
        t.put(s + 0xdc, rng.below(3));
        t.put(s + 0xe4, rng.range(-2, 5));
        t.put(s + 0xf0, rng.range(0, 5));
        for o in [0xe0u32, 0xe8, 0xec] {
            t.put(s + o, float_in(&mut rng, -40, 40));
        }
        t.same(
            "rcbgSceneWaveDCProc",
            |o| o.void(0x0092_0580, &[s]),
            |m| scene::bridge::rcbg_scene_wave_dc_proc(m, s, call),
        );
        // effect particle kicks
        t.put(is_end_rv, rng.below(2));
        t.put(par_end_rv, rng.below(2));
        t.put(new_obj_rv, objs[0]);
        t.put(m + 0x24, objs[1]);
        for &o in &objs {
            t.fill(o + 0xb0, 0x20, &mut rng);
        }
        for &p in &vecs {
            t.fill(p, 0x10, &mut rng);
        }
        let mut a = [
            m,
            rng.next(),
            rng.next(),
            rng.next(),
            rng.next(),
            vecs[0],
            vecs[1],
            vecs[2],
            0,
            0,
        ];
        a[8] = rng.next() & 0x1ff0;
        a[9] = rng.below(2);
        t.same(
            "rcefMainKickPar",
            |o| o.eax(0x0092_53a0, &a),
            |m| task::bridge::rcef_main_kick_par(m, a, call),
        );
        // key frame angles
        for i in 0..4 {
            t.put(v + 4 * i, float_in(&mut rng, -30, 30));
        }
        t.same(
            "yiKeyFrameWrapAngles",
            |o| o.void(0x0091_5120, &[v]),
            |m| yi::bridge::yi_key_frame_wrap_angles(m, v),
        );
        // image rectangles in a page
        t.put(page + 0x10, rng.range(16, 128));
        t.put(page + 0x14, rng.range(16, 128));
        let rect = |t: &mut Twin, rng: &mut Xorshift, o: u32| {
            let size = |rng: &mut Xorshift| {
                if rng.below(2) == 0 {
                    1 << rng.below(6)
                } else {
                    rng.range(1, 40)
                }
            };
            let (w, h) = (size(rng), size(rng));
            t.put(o + 0x11c, rng.range(-4, 100));
            t.put(o + 0x120, rng.range(-4, 100));
            t.put(o + 0x13c, w);
            t.put(o + 0x140, h);
        };
        rect(&mut t, &mut rng, img);
        for (i, &o) in imgs.iter().enumerate() {
            rect(&mut t, &mut rng, o);
            let next = imgs.get(i + 1).copied().unwrap_or(0);
            t.put(o + 0x15c, if i == 1 && k % 3 == 0 { img } else { next });
        }
        if k % 4 == 0 {
            // the image itself in the list
            t.put(imgs[2] + 0x15c, img);
            t.put(img + 0x15c, imgs[3]);
        }
        let head = if k % 5 == 0 { 0 } else { imgs[0] };
        t.same(
            "yiImageRectFree",
            |o| o.eax(0x0091_7490, &[page, head, img]),
            |m| yi::bridge::yi_image_rect_free(m, page, head, img),
        );
        // the last area
        let start = rng.below(0x10) << 12;
        t.set(yi::LAST_AREA_START, start);
        t.set(yi::LAST_AREA_END, start + (rng.below(8) << 8));
        t.set(
            yi::LAST_AREA_USED,
            if rng.below(2) == 0 {
                start
            } else {
                start + 0x100
            },
        );
        t.set(yi::LAST_AREA_PSMT4, u8::try_from(rng.below(2)).unwrap());
        t.put(
            img + 0xc,
            [0x13u32, 0x14, 0x14, 0x24][rng.below(4) as usize],
        );
        t.put(img + 0x140, rng.below(8));
        t.same(
            "yiImageTakeLastPage",
            |o| o.eax(0x0091_7e70, &[img, img]),
            |m| yi::bridge::yi_image_take_last_page(m, img, img),
        );
        let (pages, mode) = (rng.below(0x38), rng.below(4));
        t.same(
            "yiVramSetup",
            |o| o.eax(0x0091_7f20, &[pages, mode]),
            |m| yi::bridge::yi_vram_setup(m, pages, mode),
        );
    }
}

/// Round 14: a character's direction (`MsGetChrDir` stood in, the
/// angle wrap through `_CIfmod`), the rcui character lists and vector
/// add, layer animation steps, key frame tracks and the image placement
/// search.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round14_field_helpers_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x000e_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    t.install_trig();
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let fcall = &mut oracle_fcall;
    let (dirv, pausev, handles) = (t.alloc(4), t.alloc(4), t.alloc(0x80));
    // MsGetChrDir(chr, &out): `mov ecx, [dirv]; mov eax, [esp + 8]; mov [eax], ecx; ret`
    let mut dir = vec![0x8b, 0x0d];
    dir.extend_from_slice(&dirv.to_le_bytes());
    dir.extend_from_slice(&[0x8b, 0x44, 0x24, 0x08, 0x89, 0x08, 0xc3]);
    // MsGetChrHandle(id): `mov eax, [esp + 4]; mov eax, [handles + eax * 4]; ret`
    let mut handle = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x04, 0x85];
    handle.extend_from_slice(&handles.to_le_bytes());
    handle.push(0xc3);
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(vu0::MS_GET_CHR_DIR), dir),
        (Some(rcui::MS_GET_CHR_HANDLE), handle),
        (Some(yi::SG_GET_PAUSE), log_stub(env.log, pausev, 120, 0)),
    ];
    t.cave_from(env.cave_end, &list);
    let rw = t.alloc(0x1000);
    t.set(rcui::WORK, rw);
    let (dst, a, b) = (t.alloc(0x10), t.alloc(0x10), t.alloc(0x10));
    let (anim, frames) = (t.alloc(0x20), t.alloc(0x40));
    let r = t.alloc(0x9e000);
    let (track, recs) = (t.alloc(0x30), t.alloc(0x200));
    let page = t.alloc(0x20);
    let imgs: Vec<u32> = (0..5).map(|_| t.alloc(0x160)).collect();
    for k in 0..40u32 {
        env.reset(&mut t, &mut rng);
        // a character's direction
        let d = match k % 3 {
            0 => float_in(&mut rng, -20, 0),
            1 => float_in(&mut rng, 7, 20),
            _ => float_in(&mut rng, 0, 6),
        };
        t.put(dirv, d);
        let out = t.stack_scratch(0);
        t.same(
            "rcefGetChrDir",
            |o| o.float(0x0092_dfd0, &[rng_free(k)]),
            |m| {
                bits(vu0::bridge::rcef_get_chr_dir(
                    m,
                    rng_free(k),
                    out,
                    call,
                    fcall,
                ))
            },
        );
        // the rcui lists
        for i in 0..0x20 {
            let h = if rng.below(3) == 0 { 0 } else { rng.next() };
            t.put(handles + 4 * i, h);
        }
        t.put(rw + 0x824, rng.below(4));
        t.put(rw + 0x828, rng.next());
        t.put(rw + 0x82c, rng.next());
        t.same(
            "rcuiCollectChrHandlesByGroup",
            |o| o.void(0x0093_23b0, &[]),
            |m| rcui::bridge::rcui_collect_chr_handles_by_group(m, call),
        );
        for i in 0..4 {
            t.put(a + 4 * i, float_in(&mut rng, -50, 50));
            t.put(b + 4 * i, float_in(&mut rng, -50, 50));
        }
        t.same(
            "rcuiVu0AddVector",
            |o| o.void(0x0093_7540, &[dst, a, b]),
            |m| rcui::bridge::rcui_vu0_add_vector(m, dst, a, b),
        );
        // a layer animation
        let n = rng.range(0, 6);
        t.put(
            anim,
            rng.below(4) | ([0u32, 0x100, 0x200][rng.below(3) as usize]),
        );
        t.put(anim + 4, u16::try_from(rng.below(40)).unwrap());
        t.put(anim + 6, i16::try_from(rng.range(0, n.max(1) - 1)).unwrap());
        t.put(anim + 8, n);
        t.put(anim + 0xc, frames);
        t.put(anim + 0x14, rng.range(-0x100, 0x600));
        t.put(anim + 0x18, rng.range(0, 0x200));
        t.fill(frames, 0x40, &mut rng);
        for i in 0..0x10 {
            t.put(frames + 4 * i + 1, u8::try_from(rng.below(12)).unwrap());
        }
        t.same(
            "rcbgLayerAnimStep",
            |o| o.eax(0x0093_0920, &[anim]),
            |m| scene::bridge::rcbg_layer_anim_step(m, anim),
        );
        // a key frame track
        t.put(pausev, u32::from(rng.below(5) == 0));
        let idx = rng.below(4);
        t.put(track + 8, u16::try_from(idx).unwrap());
        t.put(track + 2, u16::try_from(rng.below(2)).unwrap());
        t.put(track + 0xa, u16::try_from(rng.below(4)).unwrap());
        t.put(track + 0x14, rng.below(100));
        t.put(track + 0x18, rng.below(100));
        t.put(track + 0x24, rng.next());
        t.put(r + 0x9d5c4 + 4 * idx, recs);
        t.put(r + 0x9d6c4 + 4 * idx, rng.below(6));
        t.put(r + 0x9cabc, rng.next());
        for i in 0..0x10 {
            t.put(recs + 0x20 * i + 4, rng.below(20));
        }
        let now = rng.below(200);
        t.same(
            "yiKeyFrameAdvance",
            |o| o.eax(0x0091_4180, &[r, track, now]),
            |m| yi::bridge::yi_key_frame_advance(m, r, track, now, call),
        );
        // image placement
        t.put(page + 0x10, rng.range(16, 128));
        t.put(page + 0x14, rng.range(16, 128));
        let used = rng.below(5);
        for (i, &o) in imgs.iter().enumerate() {
            let size = |rng: &mut Xorshift| {
                if rng.below(2) == 0 {
                    1u32 << rng.below(6)
                } else {
                    1 + rng.below(40)
                }
            };
            let (w, h) = (size(&mut rng), size(&mut rng));
            t.put(o + 0x11c, rng.below(100));
            t.put(o + 0x120, rng.below(100));
            t.put(o + 0x13c, w);
            t.put(o + 0x140, h);
            let next = if i + 1 < used as usize {
                imgs[i + 1]
            } else {
                0
            };
            t.put(o + 0x15c, next);
        }
        t.put(page + 0x1c, if used == 0 { 0 } else { imgs[0] });
        let img = imgs[4];
        t.same(
            "yiImageRman_serch",
            |o| o.eax(0x0091_7b70, &[0, page, img]),
            |m| yi::bridge::yi_image_rman_serch(m, page, img),
        );
    }
}

/// A character number for the stand-ins (unused by them).
fn rng_free(k: u32) -> u32 {
    k % 7
}

/// Round 15: Hermite curves (the weights stored as floats, products in
/// the original's order), linked key frame tracks, the rectangle and
/// triangle overlap on x/z (through `FFXVu0ApplyMatrix` or unsigned
/// integers).
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round15_field_helpers_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x000f_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let pausev = t.alloc(4);
    let list: Vec<(Option<u32>, Vec<u8>)> =
        vec![(Some(yi::SG_GET_PAUSE), log_stub(env.log, pausev, 130, 0))];
    t.cave_from(env.cave_end, &list);
    let ps: Vec<u32> = (0..4).map(|_| t.alloc(0x10)).collect();
    let out = t.alloc(0x10);
    let r = t.alloc(0x9e000);
    let (track, recs, recs2) = (t.alloc(0x30), t.alloc(0x800), t.alloc(0x800));
    let (tri, m, a, b) = (t.alloc(0x30), t.alloc(0x40), t.alloc(0x10), t.alloc(0x10));
    for k in 0..40u32 {
        env.reset(&mut t, &mut rng);
        // Hermite curves
        let tt = float_in(&mut rng, -1, 2);
        for &p in &ps {
            for i in 0..4 {
                t.put(p + 4 * i, float_in(&mut rng, -100, 100));
            }
        }
        let p = [ps[0], ps[1], ps[2], ps[3]];
        let args = [tt, p[0], p[1], p[2], p[3], out];
        if k % 2 == 0 {
            t.same(
                "yiHermiteVec2",
                |o| o.void(0x0091_4700, &args),
                |m| yi::bridge::yi_hermite_vec2(m, tt, p, out),
            );
        } else {
            t.same(
                "yiHermiteVec4",
                |o| o.void(0x0091_47b0, &args),
                |m| yi::bridge::yi_hermite_vec4(m, tt, p, out),
            );
        }
        // a linked key frame track
        t.put(pausev, u32::from(rng.below(6) == 0));
        let idx = rng.below(4);
        t.put(track + 8, u16::try_from(idx).unwrap());
        t.put(track + 2, u16::try_from(rng.below(2)).unwrap());
        t.put(track + 0xa, u16::try_from(rng.below(3)).unwrap());
        t.put(track + 0x14, rng.below(100));
        t.put(track + 0x18, rng.below(100));
        t.put(track + 0x24, recs + 0x80 * rng.below(3));
        t.put(r + 0x9d4c4 + 4 * idx, recs2);
        t.put(r + 0x9d6c4 + 4 * idx, rng.below(6));
        t.put(r + 0x9cabc, rng.next());
        for base in [recs, recs2] {
            for i in 0..0x10 {
                t.put(
                    base + 0x80 * i,
                    u8::try_from(u32::from(rng.below(8) == 0)).unwrap(),
                );
                t.put(
                    base + 0x80 * i + 4,
                    if rng.below(8) == 0 { 0 } else { rng.below(20) },
                );
            }
        }
        let now = rng.below(200);
        t.same(
            "yiKeyFrameAdvanceLinked",
            |o| o.eax(0x0091_4070, &[r, track, now]),
            |m| yi::bridge::yi_key_frame_advance_linked(m, r, track, now, call),
        );
        // rectangle and triangle
        for i in 0..4 {
            t.put(a + 4 * i, float_in(&mut rng, -8, 8));
            t.put(b + 4 * i, float_in(&mut rng, -8, 8));
        }
        let mat = if k % 2 == 0 { 0 } else { m };
        for i in 0..12 {
            let v = if mat == 0 {
                rng.below(10)
            } else {
                float_in(&mut rng, -8, 8)
            };
            t.put(tri + 4 * i, v);
        }
        for i in 0..16 {
            t.put(m + 4 * i, float_in(&mut rng, -2, 2));
        }
        let work = t.stack_scratch(0);
        t.same(
            "yiRectTriOverlapXZ",
            |o| o.eax(0x0091_0070, &[tri, mat, a, b]),
            |o| yi::bridge::yi_rect_tri_overlap_xz(o, tri, mat, [a, b], work, call),
        );
    }
}

/// A renderer stand-in for `updateVFXRenderDataShp` and
/// `InitVFXRenderDataShp`: records `id`, the arguments in `logged`, the
/// render description (bytes 2..0x10 and 0x1c..0x20 of argument 3; the
/// others are the original's uninitialised stack and the stack address of
/// its matrix) and the 64 bytes that matrix pointer (`+0x10`) points at,
/// from `dst`.
fn capture_desc_stub(dst: u32, id: u32, logged: &[u32]) -> Vec<u8> {
    let mut c = vec![0x56, 0x57, 0xc7, 0x05];
    c.extend_from_slice(&dst.to_le_bytes());
    c.extend_from_slice(&id.to_le_bytes());
    for (j, &i) in (0u32..).zip(logged) {
        c.extend_from_slice(&[0x8b, 0x44, 0x24, u8::try_from(0xc + 4 * i).unwrap(), 0xa3]);
        c.extend_from_slice(&(dst + 4 + 4 * j).to_le_bytes());
    }
    let copy = |c: &mut Vec<u8>, deref: bool, add: u8, to: u32, n: u32| {
        c.extend_from_slice(&[0x8b, 0x74, 0x24, 0x18]);
        if deref {
            c.extend_from_slice(&[0x8b, 0x76, 0x10]);
        }
        c.extend_from_slice(&[0x83, 0xc6, add, 0xbf]);
        c.extend_from_slice(&to.to_le_bytes());
        c.push(0xb9);
        c.extend_from_slice(&n.to_le_bytes());
        c.extend_from_slice(&[0xf3, 0xa4]);
    };
    copy(&mut c, false, 2, dst + 0x20, 0xe);
    copy(&mut c, false, 0x1c, dst + 0x40, 4);
    copy(&mut c, true, 0, dst + 0x50, 0x40);
    c.extend_from_slice(&[0x5f, 0x5e, 0xc3]);
    c
}

/// Round 15 (the draw family): the lens shapes' render data
/// (`KeLnsArnd_Draw`, `KeLnsCrn_Draw`, `KeLnsFls_Draw`) with the
/// renderer's functions stood in; their descriptions and matrices are
/// recorded and compared.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round15_lens_render_data_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x000f_0002);
    let env = LoadEnv::install(&mut t, &mut rng);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let log = env.log;
    let (rdv, afv, cap) = (t.alloc(4), t.alloc(4), t.alloc(0x200));
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (
            Some(ke::GRAPHIC_VFX_GET_RENDER_DATA_BY_INSTANCE),
            log_stub(log, rdv, 140, 2),
        ),
        (Some(ke::GRAPHIC_VFX_UPDATE), log_stub(log, env.rv, 141, 2)),
        (
            Some(ke::GRAPHIC_VFX_SET_VISIBLE),
            log_stub(log, env.rv, 142, 3),
        ),
        (
            Some(ke::CALCULATE_VFX_SP_ALPHA_FUNCTION),
            log_stub(log, afv, 143, 1),
        ),
        (
            Some(ke::UPDATE_VFX_RENDER_DATA_SHP),
            capture_desc_stub(cap, 144, &[0, 1, 2, 4]),
        ),
        (
            Some(ke::INIT_VFX_RENDER_DATA_SHP),
            capture_desc_stub(cap + 0x100, 145, &[0, 1, 2, 4, 5]),
        ),
    ];
    t.cave_from(env.cave_end, &list);
    let (e, shape, s, data, table) = (
        t.alloc(0x20),
        t.alloc(0x20),
        t.alloc(0xb0),
        t.alloc(4),
        t.alloc(0x100),
    );
    let scales = t.alloc(0x10);
    let off = t.b.rebase(ke::VFX_OFFSET);
    let ot = t.b.rebase(ke::PPP_OT_DRAWING);
    let prog = t.b.rebase(ke::PPP_CURRENT_PROGRAM);
    let default = t.b.rebase(ke::LNS_DEFAULT);
    for k in 0..90u32 {
        env.reset(&mut t, &mut rng);
        t.write(cap, &[0; 0x200]);
        // the source
        for i in 0..0x2c {
            t.put(s + 4 * i, float_in(&mut rng, -8, 8));
        }
        t.put(
            s + 0x7c,
            if rng.below(6) == 0 {
                0
            } else {
                float_in(&mut rng, 0, 2)
            },
        );
        t.put(s + 0xa4, float_in(&mut rng, 0, 40));
        t.put(s + 0x8f, u8::try_from(rng.below(2)).unwrap());
        for i in 0..3 {
            t.put(off + 4 * i, float_in(&mut rng, -4, 4));
            t.put(scales + 4 * i, float_in(&mut rng, -3, 3));
        }
        // the shape and its frames
        t.put(shape, if rng.below(8) == 0 { default } else { s });
        t.put(shape + 4, if rng.below(8) == 0 { 0 } else { data });
        t.put(shape + 8, u16::try_from(rng.below(0x800)).unwrap());
        t.put(shape + 0xa, u8::try_from(rng.below(3)).unwrap());
        t.put(shape + 0xb, u8::try_from(rng.below(0x100)).unwrap());
        t.put(data, table);
        t.fill(table, 0x100, &mut rng);
        for i in 0..8 {
            t.put(table + 8 * i + 0x10, 0x80u16);
            t.put(
                table + 8 * i + 0x12,
                u16::try_from(0x100 + rng.below(0x600)).unwrap(),
            );
        }
        t.put(table + 0x84, 0x10u32);
        // the lens object
        t.put(e, if rng.below(5) == 0 { 0 } else { rng.next() | 1 });
        t.put(e + 4, shape);
        t.put(e + 8, rng.next());
        t.put(e + 0x10, float_in(&mut rng, -4, 4));
        t.put(e + 0x14, u16::try_from(1 + rng.below(30)).unwrap());
        t.put(e + 0x16, u16::try_from(1 + rng.below(30)).unwrap());
        t.put(e + 0x18, scales);
        t.put(ot, rng.below(2));
        t.put(prog, rng.next());
        t.put(rdv, rng.below(2));
        t.put(afv, rng.next());
        let flags = u16::try_from(rng.below(0x10000)).unwrap();
        let work = t.stack_scratch(0);
        match k % 3 {
            0 => {
                // Arnd reads its scale vector at +0x14
                t.put(e + 0x14, scales);
                t.same(
                    "KeLnsArnd_Draw",
                    |o| o.void(0x0076_9340, &[e, u32::from(flags)]),
                    |m| ke::bridge::ke_lns_arnd_draw(m, e, flags, work, call),
                );
            }
            1 => t.same(
                "KeLnsCrn_Draw",
                |o| o.void(0x0076_9d70, &[e, u32::from(flags)]),
                |m| ke::bridge::ke_lns_crn_draw(m, e, flags, work, call),
            ),
            _ => {
                t.put(e + 0x16, u8::try_from(rng.below(3)).unwrap());
                t.put(e + 0x17, u8::try_from(rng.below(3)).unwrap());
                t.same(
                    "KeLnsFls_Draw",
                    |o| o.void(0x0076_a1f0, &[e, u32::from(flags)]),
                    |m| ke::bridge::ke_lns_fls_draw(m, e, flags, work, call),
                );
            }
        }
    }
}

/// A renderer stand-in recording `id`, the arguments in `logged`, the
/// byte ranges `segs` (offset, length) of the description at argument
/// `desc`, and, when its flags word has bit 13, the 64 bytes its matrix
/// pointer (`+0x30`) points at (`KeMdl_Draw`'s descriptions).
fn capture_segs_stub(dst: u32, id: u32, logged: &[u32], desc: u32, segs: &[(u32, u32)]) -> Vec<u8> {
    let mut c = vec![0x56, 0x57, 0xc7, 0x05];
    c.extend_from_slice(&dst.to_le_bytes());
    c.extend_from_slice(&id.to_le_bytes());
    for (j, &i) in (0u32..).zip(logged) {
        c.extend_from_slice(&[0x8b, 0x44, 0x24, u8::try_from(0xc + 4 * i).unwrap(), 0xa3]);
        c.extend_from_slice(&(dst + 4 + 4 * j).to_le_bytes());
    }
    let d = u8::try_from(0xc + 4 * desc).unwrap();
    let mut to = dst + 0x20;
    for &(off, n) in segs {
        c.extend_from_slice(&[
            0x8b,
            0x74,
            0x24,
            d,
            0x83,
            0xc6,
            u8::try_from(off).unwrap(),
            0xbf,
        ]);
        c.extend_from_slice(&to.to_le_bytes());
        c.push(0xb9);
        c.extend_from_slice(&n.to_le_bytes());
        c.extend_from_slice(&[0xf3, 0xa4]);
        to += n;
    }
    c.extend_from_slice(&[0x8b, 0x74, 0x24, d, 0xf7, 0x06]);
    c.extend_from_slice(&0x2000u32.to_le_bytes());
    c.extend_from_slice(&[0x74, 15, 0x8b, 0x76, 0x30, 0xbf]);
    c.extend_from_slice(&(dst + 0xa0).to_le_bytes());
    c.push(0xb9);
    c.extend_from_slice(&0x40u32.to_le_bytes());
    c.extend_from_slice(&[0xf3, 0xa4, 0x5f, 0x5e, 0xc3]);
    c
}

/// Round 16: `KeMdl_Draw`, the VFX model's render data: the colour
/// through the emulated EE registers (`pppEePmulth`, `pppEePpacb`, the
/// register words compared), the flags, view matrix and description
/// handed to the stood-in renderer.
#[test]
#[allow(clippy::too_many_lines)] // one case after another
fn round16_model_render_data_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0010_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let log = env.log;
    let (rdv, afv, zv, cap) = (t.alloc(4), t.alloc(4), t.alloc(4), t.alloc(0x400));
    let segs = [(0, 0xb), (0xc, 0x24), (0x34, 0x14)];
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (
            Some(ke::GRAPHIC_VFX_GET_RENDER_DATA_BY_INSTANCE),
            log_stub(log, rdv, 150, 2),
        ),
        (Some(ke::GRAPHIC_VFX_UPDATE), log_stub(log, env.rv, 151, 2)),
        (
            Some(ke::GRAPHIC_VFX_PATH3_UPDATE),
            log_stub(log, env.rv, 152, 2),
        ),
        (
            Some(ke::CALCULATE_VFX_SP_ALPHA_FUNCTION),
            log_stub(log, afv, 153, 1),
        ),
        (Some(ke::CALCULATE_VFX_SP_ZMAT), log_stub(log, zv, 154, 1)),
        (
            Some(ke::GRAPHIC_VFX_SET_CLUT_PARAMETERS),
            log_stub(log, env.rv, 155, 5),
        ),
        (
            Some(ke::UPDATE_VFX_RENDER_DATA),
            capture_segs_stub(cap, 156, &[0, 1, 2], 3, &segs),
        ),
        (
            Some(ke::INIT_VFX_RENDER_DATA),
            capture_segs_stub(cap + 0x100, 157, &[0, 1, 2, 4], 3, &segs),
        ),
        (
            Some(ke::INIT_VFX_PATH3_RENDER_DATA),
            capture_segs_stub(cap + 0x200, 158, &[0, 1, 2, 4], 3, &segs),
        ),
    ];
    t.cave_from(env.cave_end, &list);
    let (slot, envb, recs, pb) = (t.alloc(4), t.alloc(0x40), t.alloc(0x100), t.alloc(0x20));
    let (e, colw, part) = (t.alloc(0xa0), t.alloc(0x40), t.alloc(0x80));
    t.set(task::main::ENV_SLOT, slot);
    t.put(slot, envb);
    t.put(envb + 0x20, recs);
    let ee = t.b.rebase(ke::EE_REGS);
    let shift = t.b.rebase(ke::PPP_COLOR_CON_SHIFT);
    let part_at = t.b.rebase(ke::PPP_CURRENT_PART);
    let prog = t.b.rebase(ke::PPP_CURRENT_PROGRAM);
    let view = t.b.rebase(ke::PPP_VIEW_MATRIX);
    let clut = t.b.rebase(0x010d_5600);
    for k in 0..80u32 {
        env.reset(&mut t, &mut rng);
        t.write(cap, &[0; 0x400]);
        t.fill(ee, 0xb0, &mut rng);
        t.fill(colw, 0x40, &mut rng);
        t.put(shift, 4 * rng.below(4));
        t.fill(part, 0x80, &mut rng);
        t.put(part_at, part);
        if k % 5 == 0 {
            // a colour of 0: nothing drawn, only the registers written
            t.write(ee, &[0; 0xb0]);
            t.write(colw, &[0; 0x40]);
            t.write(part, &[0; 0x80]);
        }
        t.put(prog, rng.next());
        for i in 0..16 {
            t.put(view + 4 * i, float_in(&mut rng, -4, 4));
            t.put(e + 0x10 + 4 * i, float_in(&mut rng, -4, 4));
        }
        t.put(e + 0x90, rng.next());
        t.put(e + 0x94, rng.next());
        for i in 0..3 {
            t.put(clut + 4 * i, float_in(&mut rng, -4, 4));
        }
        let idx = rng.below(8);
        t.put(recs + 0x20 * idx + 0x1c, pb);
        t.fill(pb, 0x20, &mut rng);
        t.put(rdv, rng.below(2));
        t.put(afv, rng.next());
        t.put(zv, rng.next());
        let a = [
            e,
            idx,
            colw,
            rng.next(),
            rng.next(),
            rng.next(),
            rng.below(0x40),
            rng.next(),
            rng.next(),
            rng.next(),
        ];
        let work = t.stack_scratch(0);
        t.same(
            "KeMdl_Draw",
            |o| o.void(0x0076_6ff0, &a),
            |m| ke::bridge::ke_mdl_draw(m, a, work, call),
        );
    }
}

/// Keyword `i` of a parser keyword table (the strings it points at),
/// past the end "junk".
fn table_keyword(t: &Twin, table: u32, i: u32) -> String {
    let n = (0..8)
        .take_while(|&k| Mem::read::<u32>(&t.b, t.b.rebase(table) + 4 * k) != 0)
        .count();
    if i as usize >= n {
        return "junk".to_string();
    }
    let s = Mem::read::<u32>(&t.b, t.b.rebase(table) + 4 * i);
    (0..)
        .map(|j| Mem::read::<u8>(&t.b, s + j))
        .take_while(|&c| c != 0)
        .map(char::from)
        .collect()
}

/// Round 16 (rcui and anime helpers): the VU0 vector subtraction, the
/// vector clamp, the rigid matrix inverse, the projection to integers
/// (its `1/w` left in `ST0`) and the anime frame configuration keywords.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round16_rcui_and_anime_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0010_0002);
    let env = LoadEnv::install(&mut t, &mut rng);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (dst, a, b, m) = (t.alloc(0x10), t.alloc(0x10), t.alloc(0x10), t.alloc(0x40));
    let (out, chunk, parse, data, anime) = (
        t.alloc(0x40),
        t.alloc(8),
        t.alloc(0x90),
        t.alloc(0x200),
        t.alloc(0x20),
    );
    for _ in 0..40u32 {
        env.reset(&mut t, &mut rng);
        for i in 0..4 {
            t.put(a + 4 * i, float_in(&mut rng, -50, 50));
            t.put(b + 4 * i, float_in(&mut rng, -50, 50));
        }
        t.same(
            "rcuiVu0SubVector",
            |o| o.void(0x0093_76b0, &[dst, a, b]),
            |o| rcui::bridge::rcui_vu0_sub_vector(o, dst, a, b),
        );
        let (lo, hi) = (float_in(&mut rng, -30, 0), float_in(&mut rng, 0, 30));
        let work = t.stack_scratch(0);
        t.same(
            "rcuiClampVector",
            |o| o.eax(0x0093_cd20, &[dst, a, lo, hi]),
            |o| rcui::bridge::rcui_clamp_vector(o, dst, a, (lo, hi), work, call),
        );
        for i in 0..16 {
            t.put(m + 4 * i, float_in(&mut rng, -4, 4));
        }
        t.same(
            "rcuiInvertRigidMatrix",
            |o| o.eax(0x0093_d090, &[out, m]),
            |o| rcui::bridge::rcui_invert_rigid_matrix(o, out, m, work, call),
        );
        t.put(m + 0x3c, float_in(&mut rng, 1, 8));
        let raw = rng.below(2);
        t.same(
            "rcuiProjectToInt",
            |o| o.float(0x0093_d710, &[out, m, a, raw]),
            |o| {
                bits(rcui::bridge::rcui_project_to_int(
                    o,
                    out,
                    (m, a),
                    raw,
                    work,
                    call,
                ))
            },
        );
        // anime frames
        let mut text = b"head ".to_vec();
        for _ in 0..rng.below(5) {
            let w = table_keyword(&t, model::ANIME_CONFIG_KEYS, rng.below(4));
            text.extend(w.bytes());
            for _ in 0..=rng.below(9) {
                text.extend(format!(" {}", rng.range(0, 5)).bytes());
            }
            text.push(b' ');
        }
        text.push(0);
        t.write(data, &text);
        parser_over(&mut t, chunk, parse, data);
        t.put(chunk + 4, u32::try_from(text.len() - 1).unwrap());
        t.put(parse + 8, 1u32);
        t.write(anime, &[0; 0x20]);
        t.same(
            "rcbgAnimeLoadConfig",
            |o| o.eax(0x0093_a650, &[anime, parse]),
            |o| model::bridge::rcbg_anime_load_config(o, anime, parse, call),
        );
    }
}

/// Round 17: `FFXSamp0TransMatrix`, the vector scale, the sphere grid's
/// panel sounds (`SndSepPlaySimple` stood in) and the 2D segment
/// intersection.
#[test]
fn round17_field_helpers_match_the_original() {
    use ffx_field::abmap;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0011_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![(
        Some(abmap::SND_SEP_PLAY_SIMPLE),
        log_stub(env.log, env.rv, 170, 1),
    )];
    t.cave_from(env.cave_end, &list);
    let (dst, m, v, out) = (t.alloc(0x40), t.alloc(0x40), t.alloc(0x10), t.alloc(0x10));
    let pts: Vec<u32> = (0..4).map(|_| t.alloc(8)).collect();
    for k in 0..60u32 {
        env.reset(&mut t, &mut rng);
        for i in 0..16 {
            t.put(m + 4 * i, float_in(&mut rng, -50, 50));
        }
        for i in 0..4 {
            t.put(v + 4 * i, float_in(&mut rng, -50, 50));
        }
        t.same(
            "FFXSamp0TransMatrix",
            |o| o.eax(0x0093_d830, &[dst, m, v]),
            |o| rcui::bridge::ffx_samp0_trans_matrix(o, dst, m, v),
        );
        let s = float_in(&mut rng, -8, 8);
        t.same(
            "rcuiScaleVec4",
            |o| o.eax(0x0093_d7d0, &[out, v, s]),
            |o| rcui::bridge::rcui_scale_vec4(o, out, v, s),
        );
        let which = rng.below(2);
        t.same(
            "abmap_snd_chg_panel",
            |o| o.eax(0x00a4_5910 + 0x10 * which, &[]),
            |o| abmap::bridge::abmap_snd_chg_panel(o, which, call),
        );
        for &p in &pts {
            t.put(p, float_in(&mut rng, -10, 10));
            t.put(p + 4, float_in(&mut rng, -10, 10));
        }
        if k % 6 == 0 {
            // parallel segments
            let (ax, ay) = (
                Mem::read::<u32>(&t.b, pts[0]),
                Mem::read::<u32>(&t.b, pts[0] + 4),
            );
            let (bx, by) = (
                Mem::read::<u32>(&t.b, pts[1]),
                Mem::read::<u32>(&t.b, pts[1] + 4),
            );
            t.put(pts[2], ax);
            t.put(pts[2] + 4, ay);
            t.put(pts[3], bx);
            t.put(pts[3] + 4, by);
        }
        let p = [pts[0], pts[1], pts[2], pts[3]];
        t.same(
            "yiSegIntersect2",
            |o| o.eax(0x0091_cd00, &[p[0], p[1], p[2], p[3], out]),
            |o| yi::bridge::yi_seg_intersect2(o, p, out),
        );
    }
}

/// Round 18: the Tidus limit tables, and sphere grid set-up: kind sizes
/// (`op_oef2_a_get` stood in), locked menu entries
/// (`MsGetSavePlayerPtr` stood in), link cells and node link lists.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round18_abmap_and_nkz_match_the_original() {
    use ffx_field::abmap;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0012_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (recv, savev) = (t.alloc(4), t.alloc(4));
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(abmap::OP_OEF2_A_GET), log_stub(env.log, recv, 180, 3)),
        (
            Some(abmap::MS_GET_SAVE_PLAYER_PTR),
            log_stub(env.log, savev, 181, 1),
        ),
    ];
    t.cave_from(env.cave_end, &list);
    let grid = t.alloc(0x1_2000);
    t.set(abmap::ABMAP, grid);
    let (rec, save, pads, link) = (
        t.alloc(0x20),
        t.alloc(0x100),
        t.alloc(0x340 * 2),
        t.alloc(0x10),
    );
    t.put(t.b.rebase(abmap::ABMAP_PAD), pads);
    t.put(savev, save);
    for _ in 0..40u32 {
        env.reset(&mut t, &mut rng);
        // indices past 3 read the original's stack
        let i = rng.below(4);
        for (k, f) in [(0u32, 0x00a5_cb30u32), (1, 0x00a5_cb80)] {
            t.same(
                "NkzTidusLimitTable",
                |o| o.eax(f, &[i]),
                |_| nkz::bridge::nkz_tidus_limit_table(k, i),
            );
        }
        // kind sizes
        t.fill(rec, 0x20, &mut rng);
        t.put(recv, rec);
        let src = rng.next();
        t.same(
            "AbmapSetKindSizes",
            |o| o.eax(0x00a5_adf0, &[src]),
            |o| abmap::bridge::abmap_set_kind_sizes(o, src, call),
        );
        // locked entries
        t.fill(save, 0x100, &mut rng);
        t.put(grid + 0x115bc, u8::try_from(rng.below(7)).unwrap());
        let pad = rng.below(2);
        let p = pads + 0x340 * pad;
        t.put(p + 0x1e, i16::try_from(rng.range(-1, 20)).unwrap());
        for k in 0..20 {
            t.put(p + 0x48 + 0xc * k, rng.below(0x100));
        }
        t.same(
            "AbmapMarkLockedEntries",
            |o| o.eax(0x00a5_a2e0, &[pad]),
            |o| abmap::bridge::abmap_mark_locked_entries(o, pad, call),
        );
        // the grid's nodes and links
        let (nodes, links) = (rng.range(0, 40), rng.range(0, 60));
        t.put(grid + 2, i16::try_from(nodes).unwrap());
        t.put(grid + 4, i16::try_from(links).unwrap());
        for n in 0..41u32 {
            let nd = grid + 0x808 + 0x28 * n;
            t.fill(nd, 0x28, &mut rng);
            t.put(nd + 6, if rng.below(4) == 0 { 0xffffu16 } else { 1 });
            t.put(nd, i16::try_from(rng.range(-0xa00, 0xa00)).unwrap());
            t.put(nd + 2, i16::try_from(rng.range(-0x920, 0x920)).unwrap());
        }
        for l in 0..61u32 {
            let at = grid + 0xa808 + 0x14 * l;
            t.fill(at, 0x14, &mut rng);
            t.put(at, u16::try_from(rng.below(42)).unwrap());
            t.put(at + 2, u16::try_from(rng.below(42)).unwrap());
        }
        t.put(link, u16::try_from(rng.below(41)).unwrap());
        t.put(link + 2, u16::try_from(rng.below(41)).unwrap());
        t.same(
            "AbmapLinkCells",
            |o| o.eax(0x00a5_a760, &[link]),
            |o| abmap::bridge::abmap_link_cells(o, link),
        );
        t.same(
            "AbmapCollectNodeLinks",
            |o| o.eax(0x00a5_b140, &[]),
            abmap::bridge::abmap_collect_node_links,
        );
        // a beam added to a lens's list
        let (cp, pool, va, vb) = (t.alloc(0x10), t.alloc(0x10), t.alloc(0x10), t.alloc(0x10));
        let blocks: Vec<u32> = (0..2).map(|_| t.alloc(0x40 + 0x30 * 8 + 0x10)).collect();
        for &b in &blocks {
            t.fill(b, 0x80, &mut rng);
        }
        t.put(blocks[0], if rng.below(2) == 0 { 0 } else { blocks[1] });
        t.put(pool + 1, u8::try_from(rng.below(8)).unwrap());
        t.put(pool + 4, if rng.below(5) == 0 { 0 } else { blocks[0] });
        let old = t.alloc(0x10);
        t.put(cp, if rng.below(2) == 0 { 0 } else { old });
        t.put(cp + 4, u8::try_from(rng.below(0x40)).unwrap());
        for i in 0..4 {
            t.put(va + 4 * i, float_in(&mut rng, -9, 9));
            t.put(vb + 4 * i, float_in(&mut rng, -9, 9));
        }
        let work = t.stack_scratch(0);
        t.same(
            "KeThCp_AddBp",
            |o| o.eax(0x0076_44d0, &[cp, pool, va, vb]),
            |o| ke::bridge::ke_th_cp_add_bp(o, cp, pool, (va, vb), work),
        );
    }
}

/// `mov eax, [esp + 4]; and eax, 7; shl eax, 6; add eax, base; ret`: an
/// excel lookup stand-in, row `i & 7` of 64-byte rows at `base`.
fn excel_rows_stub(base: u32) -> Vec<u8> {
    let mut c = vec![
        0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x07, 0xc1, 0xe0, 0x06, 0x05,
    ];
    c.extend_from_slice(&base.to_le_bytes());
    c.push(0xc3);
    c
}

/// Round 19: items over the sphere grid (the iterators' wrappers with
/// their original callbacks, and the callbacks themselves) and the node
/// kinds' shapes, the excel lookups and item records stood in.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round19_abmap_items_and_kinds_match_the_original() {
    use ffx_field::abmap;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0013_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    t.install_trig();
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (heldv, recv, specv, rows) = (t.alloc(4), t.alloc(4), t.alloc(4), t.alloc(0x200));
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (
            Some(abmap::MS_GET_SAVE_ITEM_NUM),
            log_stub(env.log, heldv, 190, 1),
        ),
        (
            Some(abmap::MS_GET_ROM_ITEM),
            log_stub_from(env.log, recv, 191, 0, 1),
        ),
        (Some(abmap::MS_GET_EXCEL_DATA), excel_rows_stub(rows)),
        (
            Some(abmap::ABMAP_SPECIAL_EFFECT),
            log_stub(env.log, specv, 192, 4),
        ),
    ];
    t.cave_from(env.cave_end, &list);
    let grid = t.alloc(0x1_2000);
    t.set(abmap::ABMAP, grid);
    let rec = t.alloc(0x60);
    t.put(recv, rec);
    let shapes: Vec<u32> = (0..8).map(|_| t.alloc(0x40)).collect();
    let tables = t.b.rebase(abmap::KIND_SHAPE_TABLES);
    let classes = t.b.rebase(abmap::SIZE_CLASS_SHAPES);
    for k in 0..30u32 {
        env.reset(&mut t, &mut rng);
        t.fill(rows, 0x200, &mut rng);
        for r in 0..8 {
            t.put(rows + 0x40 * r + 0x16, u8::try_from(rng.below(20)).unwrap());
            let fl = match rng.below(3) {
                0 => 0u16,
                1 => 0x8000 | u16::try_from(rng.below(0x100)).unwrap(),
                _ => u16::try_from(1 + rng.below(0x7fff)).unwrap(),
            };
            t.put(rows + 0x40 * r + 0x10, fl);
            t.put(
                rows + 0x40 * r + 0xc,
                if rng.below(3) == 0 { 0x20u8 } else { 1 },
            );
            t.put(rows + 0x40 * r + 0xd, u8::try_from(rng.below(3)).unwrap());
        }
        t.fill(rec, 0x60, &mut rng);
        t.put(
            rec + 0x5d,
            if rng.below(4) == 0 {
                0xffu8
            } else {
                u8::try_from(rng.below(8)).unwrap()
            },
        );
        // the grid
        let (nodes, links) = (rng.range(1, 30), rng.range(0, 40));
        t.put(grid + 2, i16::try_from(nodes).unwrap());
        t.put(grid + 4, i16::try_from(links).unwrap());
        for n in 0..31u32 {
            let nd = grid + 0x808 + 0x28 * n;
            t.fill(nd, 0x28, &mut rng);
            for s in 0..5u32 {
                let l = if rng.below(2) == 0 {
                    0
                } else {
                    grid + 0xa808 + 0x14 * rng.below(40)
                };
                t.put(nd + 0xc + 4 * s, l);
            }
        }
        for l in 0..41u32 {
            let at = grid + 0xa808 + 0x14 * l;
            t.fill(at, 0x14, &mut rng);
            t.put(at, u16::try_from(rng.below(31)).unwrap());
            t.put(at + 2, u16::try_from(rng.below(31)).unwrap());
        }
        t.put(heldv, rng.below(3));
        t.put(specv, rng.next());
        let chr = rng.below(7);
        let node = grid + 0x808 + 0x28 * rng.below(u32::try_from(nodes).unwrap());
        let item = rng.next();
        let work = t.stack_scratch(0);
        t.same(
            "AbmapItemUsableAt",
            |o| o.eax(0x00a4_9310, &[chr, node, item]),
            |o| abmap::bridge::abmap_item_usable_at(o, [chr, node, item], work, call),
        );
        let mark = rng.next();
        t.same(
            "AbmapItemApplyAt",
            |o| o.eax(0x00a5_b400, &[chr, node, item, mark]),
            |o| abmap::bridge::abmap_item_apply_at(o, [chr, node, item, mark], work, call),
        );
        // the callbacks on their own
        let ctx = t.alloc(0x14);
        t.put(ctx, rows + 0x40 * rng.below(8));
        t.put(ctx + 4, chr);
        t.put(ctx + 8, 1u32 << chr);
        t.put(ctx + 0xc, rng.next() & 0xff);
        t.put(ctx + 0x10, u8::try_from(rng.below(0x100)).unwrap());
        t.same(
            "AbmapItemNodeEffect",
            |o| o.eax(0x00a4_9440, &[node, ctx]),
            |o| abmap::bridge::abmap_item_node_effect(o, node, ctx, work, call),
        );
        t.same(
            "AbmapItemApplyNode",
            |o| o.eax(0x00a5_b490, &[node, ctx]),
            |o| abmap::bridge::abmap_item_apply_node(o, node, ctx, work, call),
        );
        // the kinds' shapes
        if k % 3 == 0 {
            for &s in &shapes {
                t.fill(s, 0x40, &mut rng);
                t.put(s + 0x10, i16::try_from(rng.range(0, 0x20)).unwrap());
                t.put(s + 0x20, i16::try_from(rng.range(-0x200, 0x200)).unwrap());
                t.put(s + 0x2c, i16::try_from(rng.range(-0x200, 0x400)).unwrap());
            }
            // every entry is read (`+0x10`), so none is null
            for i in 0..40 {
                t.put(tables + 4 * i, shapes[rng.below(8) as usize]);
            }
            for i in 0..3 {
                t.put(classes + 4 * i, shapes[rng.below(8) as usize]);
            }
            t.same(
                "AbmapSetupKindShapes",
                |o| o.eax(0x00a5_b230, &[]),
                |o| abmap::bridge::abmap_setup_kind_shapes(o, work, call),
            );
        }
    }
}

/// Copies the three words at `src` through its three pointer arguments
/// (`graphicGetFogColor`'s shape).
fn fill3_stub(src: u32) -> Vec<u8> {
    let mut c = vec![];
    for i in 0..3u8 {
        c.extend_from_slice(&[0x8b, 0x44, 0x24, 4 + 4 * i, 0x8b, 0x0d]);
        c.extend_from_slice(&(src + 4 * u32::from(i)).to_le_bytes());
        c.extend_from_slice(&[0x89, 0x08]);
    }
    c.push(0xc3);
    c
}

/// `fld dword [at]; ret`: a stand-in returning a float in ST0.
fn fld_stub(at: u32) -> Vec<u8> {
    let mut c = vec![0xd9, 0x05];
    c.extend_from_slice(&at.to_le_bytes());
    c.push(0xc3);
    c
}

/// Rounds 19 and 20: fog and background colour, the PC renderer's getters and
/// setters stood in.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round19_fog_and_back_colour_match_the_original() {
    use yi::fog;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0013_0002);
    let env = LoadEnv::install(&mut t, &mut rng);
    let (log, rv) = (env.log, env.rv);
    let (gets, magicv, rgb) = (t.alloc(0xc), t.alloc(4), t.alloc(0xc));
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(fog::GET_NEAR), fld_stub(gets)),
        (Some(fog::GET_FAR), fld_stub(gets + 4)),
        (Some(fog::GET_LIMIT), fld_stub(gets + 8)),
        (Some(fog::SET_COLOUR), log_stub(log, rv, 195, 3)),
        (Some(fog::SET_NEAR), log_stub(log, rv, 196, 1)),
        (Some(fog::SET_FAR), log_stub(log, rv, 197, 1)),
        (Some(fog::SET_LIMIT), log_stub(log, rv, 198, 1)),
        (Some(fog::SET_FOG_643180), log_stub(log, rv, 199, 1)),
        (Some(fog::SET_BACKGROUND), log_stub(log, rv, 200, 4)),
        (Some(fog::SET_DISTORTION), log_stub(log, rv, 201, 1)),
        (Some(fog::MAGIC_GET_ID), log_stub(log, magicv, 202, 0)),
        (Some(fog::GET_COLOUR), fill3_stub(rgb)),
        (Some(fog::GET_AMBIENT), fill3_stub(rgb)),
    ];
    t.cave_from(env.cave_end, &list);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let fcall = &mut oracle_fcall;
    let (out, c) = (t.alloc(0x10), t.alloc(0x10));
    let at = |t: &Twin, a: u32| t.b.rebase(a);
    for k in 0..40u32 {
        env.reset(&mut t, &mut rng);
        t.set(
            yi::CURRENT_MAP,
            if k % 2 == 0 {
                fog::PC_FOG_MAP
            } else {
                rng.below(0x200)
            },
        );
        t.set(yi::RENDER_STATUS, rng.next());
        t.put(at(&t, fog::SCALE), float_in(&mut rng, 1, 64));
        for a in [fog::LIMIT, fog::NEAR, fog::FAR] {
            let v = if k % 7 == 3 {
                rng.next()
            } else {
                float_in(&mut rng, -100, 4000)
            };
            t.put(at(&t, a), v);
        }
        for i in 0..3 {
            t.put(gets + 4 * i, float_in(&mut rng, -100, 4000));
        }
        t.fill(out, 0xc, &mut rng);
        t.same(
            "yiGetFogParam",
            |o| o.void(0x0090_8bb0, &[out, out + 4, out + 8]),
            |m| yi::bridge::yi_get_fog_param(m, [out, out + 4, out + 8], fcall),
        );
        for i in 0..4 {
            let v = match rng.below(3) {
                0 => rng.below(0x100),
                1 => rng.next(),
                _ => rng.range(-300, 300).cast_unsigned(),
            };
            t.put(c + 4 * i, v);
        }
        t.put(
            magicv,
            if rng.below(3) == 0 {
                fog::MAGIC_KEEP_BACK
            } else {
                rng.below(0x400)
            },
        );
        t.put(at(&t, fog::COLOUR), rng.next());
        for i in 0..3 {
            let v = if k % 9 == 5 {
                rng.next()
            } else {
                float_in(&mut rng, -8, 24)
            };
            t.put(rgb + 4 * i, v);
        }
        for i in 0..4 {
            let sc = at(&t, 0x018d_f00c) + 4 * i;
            t.put(sc, float_in(&mut rng, -16, 48));
            let of = at(&t, 0x018d_f01c) + 4 * i;
            t.put(of, float_in(&mut rng, -2000, 2000));
        }
        t.fill(out, 0x10, &mut rng);
        let work = t.stack_scratch(0);
        t.same(
            "yiGetFogColor",
            |o| o.eax(0x0090_8980, &[out]),
            |m| yi::bridge::yi_get_fog_color(m, out, work, call),
        );
        t.same(
            "yiSetBackColor",
            |o| o.void(0x0090_a040, &[c]),
            |m| yi::bridge::yi_set_back_color(m, c, call),
        );
        t.same(
            "yiSetFogColor",
            |o| o.void(0x0090_a2c0, &[c]),
            |m| yi::bridge::yi_set_fog_color(m, c, call),
        );
        let args = if k % 5 == 4 {
            [rng.next(), rng.next(), rng.next()]
        } else {
            [
                float_in(&mut rng, -100, 4000),
                float_in(&mut rng, -100, 4000),
                float_in(&mut rng, -10, 300),
            ]
        };
        t.same(
            "yiSetFogParam",
            |o| o.void(0x0090_a350, &args),
            |m| yi::bridge::yi_set_fog_param(m, args, call),
        );
        t.same(
            "yiDisableHamonEff",
            |o| o.eax(0x0090_8450, &[]),
            |m| yi::bridge::yi_disable_hamon_eff(m, call),
        );
    }
}

/// Round 20: the sphere grid's pad slides, rectangle tweens, node flag
/// masks and the empty-menu marks.
#[test]
fn round20_abmap_pads_and_tweens_match_the_original() {
    use ffx_field::abmap;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0014_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let (log, rv) = (env.log, env.rv);
    let (spv, maxv) = (t.alloc(4), t.alloc(4));
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(abmap::MS_GET_SAVE_SP), log_stub(log, spv, 210, 1)),
        (
            Some(abmap::TO_AMAP_MAKE_ITEM_LIST),
            log_stub(log, rv, 211, 1),
        ),
        (
            Some(abmap::TO_AMAP_GET_ITEM_LIST_MAX),
            log_stub(log, maxv, 212, 0),
        ),
    ];
    t.cave_from(env.cave_end, &list);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let grid = t.alloc(0x1_2000);
    t.set(abmap::ABMAP, grid);
    let pads = t.alloc(0x340 * 4);
    t.put(t.b.rebase(abmap::ABMAP_PAD), pads);
    for k in 0..300u32 {
        env.reset(&mut t, &mut rng);
        t.fill(pads, 0x340 * 4, &mut rng);
        let pad = rng.below(4);
        let p = pads + 0x340 * pad;
        t.put(p + 0x23, u8::try_from(rng.below(2)).unwrap());
        if rng.below(2) == 0 {
            t.put(p + 0x34, 0u32);
        }
        t.put(
            p + 0x29,
            u8::try_from(rng.below(4)).unwrap() + if k % 7 == 0 { 0xfc } else { 0 },
        );
        t.same(
            "AbmapPadStartSlideOut",
            |o| o.void(0x00a4_8c20, &[pad]),
            |m| abmap::bridge::abmap_pad_start_slide_out(m, pad),
        );
        t.same(
            "AbmapPadStartSlideIn",
            |o| o.void(0x00a4_8f20, &[pad]),
            |m| abmap::bridge::abmap_pad_start_slide_in(m, pad),
        );
        t.put(p + 0x29, u8::try_from(rng.below(4)).unwrap());
        t.same(
            "AbmapPadSlideOutStep",
            |o| o.void(0x00a5_9800, &[p]),
            |m| abmap::bridge::abmap_pad_slide_out_step(m, p),
        );
        t.put(p + 0x29, u8::try_from(rng.below(4)).unwrap());
        t.same(
            "AbmapPadSlideInStep",
            |o| o.void(0x00a5_a270, &[p]),
            |m| abmap::bridge::abmap_pad_slide_in_step(m, p),
        );
        t.put(p + 0x32, u8::try_from(rng.below(5)).unwrap());
        t.same(
            "AbmapRectTweenIn",
            |o| o.void(0x00a5_a080, &[p]),
            |m| abmap::bridge::abmap_rect_tween_in(m, p),
        );
        t.put(p + 0x32, u8::try_from(rng.below(5)).unwrap());
        t.same(
            "AbmapRectTweenOut",
            |o| o.void(0x00a5_a170, &[p]),
            |m| abmap::bridge::abmap_rect_tween_out(m, p),
        );
        // the nodes
        let nodes = rng.below(40);
        t.put(grid + 2, u16::try_from(nodes).unwrap());
        t.fill(grid + 0x808, 0x28 * 41, &mut rng);
        for n in 0..41 {
            if rng.below(3) == 0 {
                t.put(grid + 0x808 + 0x28 * n + 6, -1i16);
            }
        }
        let mask = rng.next();
        t.same(
            "AbmapMaskNodeFlags",
            |o| o.void(0x00a4_7400, &[mask]),
            #[allow(clippy::cast_possible_truncation)]
            |m| abmap::bridge::abmap_mask_node_flags(m, mask as u8),
        );
        // the menus
        t.put(pads + 0x16de, i16::try_from(rng.range(-2, 6)).unwrap());
        t.put(
            grid + abmap::CURRENT_CHR,
            u8::try_from(rng.below(7)).unwrap(),
        );
        t.put(spv, if rng.below(2) == 0 { 0 } else { rng.next() });
        t.put(maxv, if rng.below(2) == 0 { 0 } else { rng.next() });
        t.same(
            "AbmapMarkEmptyMenus",
            |o| o.eax(0x00a5_aca0, &[]),
            |m| abmap::bridge::abmap_mark_empty_menus(m, call),
        );
    }
}

/// `AbmapItemNodeEffect`'s stand-in: copies the context's first four
/// words and the node to `buf` (the context is a stack buffer) and returns
/// the word at `rv`.
fn ctx_copy_stub(buf: u32, rv: u32) -> Vec<u8> {
    let mut c = vec![0x8b, 0x44, 0x24, 0x08];
    for i in 0..4u8 {
        if i == 0 {
            c.extend_from_slice(&[0x8b, 0x08]);
        } else {
            c.extend_from_slice(&[0x8b, 0x48, 4 * i]);
        }
        c.extend_from_slice(&[0x89, 0x0d]);
        c.extend_from_slice(&(buf + 4 * u32::from(i)).to_le_bytes());
    }
    c.extend_from_slice(&[0x8b, 0x4c, 0x24, 0x04, 0x89, 0x0d]);
    c.extend_from_slice(&(buf + 0x10).to_le_bytes());
    c.push(0xa1);
    c.extend_from_slice(&rv.to_le_bytes());
    c.push(0xc3);
    c
}

/// Round 21: the effect request, the effect finalisers, detaching all of
/// an object's animations, Yn's scene draw and the sphere grid's menus.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round21_requests_finis_and_menus_match_the_original() {
    use ffx_field::abmap;
    use task::{bridge as tb, header};
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0015_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let (log, rv) = (env.log, env.rv);
    let (heldv, recv, rows, ctxv, ctxbuf) = (
        t.alloc(4),
        t.alloc(4),
        t.alloc(0x200),
        t.alloc(4),
        t.alloc(0x14),
    );
    let (usev, spv, maxv) = (t.alloc(4), t.alloc(4), t.alloc(4));
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (
            Some(abmap::MS_GET_SAVE_ITEM_NUM),
            log_stub(log, heldv, 230, 1),
        ),
        (
            Some(abmap::MS_GET_ROM_ITEM),
            log_stub_from(log, recv, 231, 0, 1),
        ),
        (Some(abmap::MS_GET_EXCEL_DATA), excel_rows_stub(rows)),
        (Some(abmap::ITEM_NODE_EFFECT), ctx_copy_stub(ctxbuf, ctxv)),
        (Some(abmap::ITEM_USABLE_AT), log_stub(log, usev, 232, 3)),
        (Some(model::SCENE_DRAW), log_stub(log, rv, 233, 1)),
        (Some(abmap::MS_GET_SAVE_SP), log_stub(log, spv, 234, 1)),
        (
            Some(abmap::TO_AMAP_MAKE_ITEM_LIST),
            log_stub(log, rv, 235, 1),
        ),
        (
            Some(abmap::TO_AMAP_GET_ITEM_LIST_MAX),
            log_stub(log, maxv, 236, 0),
        ),
    ];
    t.cave_from(env.cave_end, &list);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let task = t.alloc(0xbc);
    let scene = t.alloc(0x9cac0 + 0x28 * 0x40);
    let (sys, map) = (t.alloc(0x20), t.alloc(0x20));
    let grid = t.alloc(0x1_2000);
    t.set(abmap::ABMAP, grid);
    let pads = t.alloc(0x2710);
    t.put(t.b.rebase(abmap::ABMAP_PAD), pads);
    let rec = t.alloc(0x60);
    t.put(recv, rec);
    for k in 0..200u32 {
        env.reset(&mut t, &mut rng);
        // the request
        let mut args = [rng.next(), rng.next(), rng.next(), rng.next(), rng.next()];
        if k % 3 != 0 {
            args[1] = float_in(&mut rng, -1000, 1000);
        } else if args[1] & 0x7f80_0000 == 0x7f80_0000 {
            args[1] |= 0x0040_0000; // no signalling NaNs (the emulator keeps them)
        }
        t.same(
            "rcefPostRequest",
            |o| o.eax(0x0092_eba0, &args),
            |m| tb::rcef_post_request(m, args),
        );
        t.fill(task, 0xbc, &mut rng);
        t.put(task + header::LEVEL, rng.range(-1, 8));
        t.set(tb::REQUEST_PENDING, rng.below(2));
        t.same(
            "rcEffectTaskRequestWait",
            |o| o.void(0x0093_04e0, &[task]),
            |m| tb::rc_effect_task_request_wait(m, task),
        );
        // the finalisers
        let id = rng.next();
        t.set(tb::CURRENT_PROGRAM, rng.next());
        t.same(
            "rcefAnimeFini",
            |o| o.eax(0x0092_f4a0, &[id]),
            |m| tb::rcef_anime_fini(m, id, call),
        );
        t.put(rv, rng.below(2));
        t.same(
            "rcefModelFini",
            |o| o.eax(0x0092_f710, &[id]),
            |m| tb::rcef_model_fini(m, id, call),
        );
        // the animation slots
        t.fill(scene + 0x9cac0, 0x28 * 0x40, &mut rng);
        for i in 0..0x40 {
            let e = scene + 0x9cac0 + 0x28 * i;
            t.put(e + 4, u16::try_from(rng.below(4)).unwrap());
            t.put(e + 6, u16::try_from(rng.below(5)).unwrap());
        }
        t.put(scene + 0x9cabc, rng.below(70));
        let obj = rng.below(6);
        t.same(
            "yiAnimDetachAll",
            |o| o.eax(0x0091_49a0, &[scene, obj]),
            |m| yi::bridge::yi_anim_detach_all(m, scene, obj, call),
        );
        // Yn's scene
        let (s, m0) = match rng.below(3) {
            0 => (0, 0),
            1 => (sys, 0),
            _ => (sys, map),
        };
        t.set(guide::YN_MAP_SYSTEM, s);
        t.put(sys, m0);
        t.put(map + 8, if rng.below(4) == 0 { 0 } else { rng.next() });
        t.same(
            "Yn_DrawScene",
            |o| {
                let e = o.eax(0x0091_a240, &[]);
                (s != 0 && m0 != 0).then_some(e)
            },
            |m| model::bridge::yn_draw_scene(m, call),
        );
        // the sphere grid
        t.fill(rows, 0x200, &mut rng);
        t.fill(rec, 0x60, &mut rng);
        t.put(
            rec + 0x5d,
            if rng.below(4) == 0 {
                0xffu8
            } else {
                u8::try_from(rng.below(8)).unwrap()
            },
        );
        t.put(heldv, rng.below(3));
        t.put(ctxv, rng.next());
        t.fill(ctxbuf, 0x14, &mut rng);
        let (chr, node, item) = (rng.below(40), rng.below(0x400), rng.next());
        let work = t.stack_scratch(0);
        t.same(
            "AbmapItemEffectAtNode",
            |o| o.eax(0x00a4_93b0, &[chr, node, item]),
            |m| abmap::bridge::abmap_item_effect_at_node(m, [chr, node, item], work, call),
        );
        t.fill(pads, 0x2710, &mut rng);
        t.put(pads + 0x16de, i16::try_from(rng.range(-2, 6)).unwrap());
        t.put(pads + 0x1a1e, i16::try_from(rng.range(-2, 12)).unwrap());
        t.put(
            grid + abmap::CURRENT_CHR,
            u8::try_from(rng.below(7)).unwrap(),
        );
        t.put(spv, if rng.below(2) == 0 { 0 } else { rng.next() });
        t.put(maxv, if rng.below(2) == 0 { 0 } else { rng.next() });
        t.same(
            "AbmapEnterCommandMenu",
            |o| o.eax(0x00a5_9950, &[]),
            |m| abmap::bridge::abmap_enter_command_menu(m, call),
        );
        t.put(usev, rng.below(2));
        t.same(
            "AbmapEnterItemMenu",
            |o| o.eax(0x00a5_98a0, &[]),
            |m| abmap::bridge::abmap_enter_item_menu(m, call),
        );
    }
}

/// `MsMenuStatusNum(i, _)`'s stand-in: `base + 4 i`.
fn index4_stub(base: u32) -> Vec<u8> {
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xc1, 0xe0, 0x02, 0x05];
    c.extend_from_slice(&base.to_le_bytes());
    c.push(0xc3);
    c
}

/// `Ch_GetPos(chr, &x, &y, &z)`'s stand-in: the three words at `src`
/// through its pointer arguments 1..=3.
fn pos3_stub(src: u32) -> Vec<u8> {
    let mut c = vec![];
    for i in 0..3u8 {
        c.extend_from_slice(&[0x8b, 0x44, 0x24, 8 + 4 * i, 0x8b, 0x0d]);
        c.extend_from_slice(&(src + 4 * u32::from(i)).to_le_bytes());
        c.extend_from_slice(&[0x89, 0x08]);
    }
    c.push(0xc3);
    c
}

/// Round 22: disposing of animations, the effect object finaliser, the
/// command menus, the guide's edge fade and the character distance.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round22_dispose_fini_menus_fade_and_chadist_match_the_original() {
    use ffx_field::abmap;
    use task::bridge as tb;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0016_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let (log, rv) = (env.log, env.rv);
    let (nv, entries, exitv, plyv, pos) = (
        t.alloc(4),
        t.alloc(4 * 64),
        t.alloc(4),
        t.alloc(4),
        t.alloc(0xc),
    );
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (
            Some(abmap::MS_MENU_STATUS_NUM_SIZE),
            log_stub(log, nv, 240, 0),
        ),
        (Some(abmap::MS_MENU_STATUS_NUM), index4_stub(entries)),
        (
            Some(abmap::MS_GET_ROM_PLY_COMMAND),
            log_stub_from(log, rv, 241, 0, 1),
        ),
        (Some(abmap::DBG_PRINTF), log_stub(log, rv, 242, 2)),
        (Some(abmap::DEBUG_EXIT_TRACE), log_stub(log, exitv, 243, 1)),
        (Some(camera::SG_GET_PLAYER), log_stub(log, plyv, 244, 0)),
        (Some(model::CH_GET_POS), pos3_stub(pos)),
        (Some(model::GRAPHIC_SET_CHADIST), log_stub(log, rv, 245, 1)),
    ];
    t.cave_from(env.cave_end, &list);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let render = t.b.rebase(0x0187_6e00);
    let o = t.alloc(0x490);
    let pads = t.alloc(0x340 * 6);
    t.put(t.b.rebase(abmap::ABMAP_PAD), pads);
    let (a, b, p, rect, out) = (
        t.alloc(8),
        t.alloc(8),
        t.alloc(8),
        t.alloc(0x10),
        t.alloc(4),
    );
    let (scene, sys, map) = (t.alloc(0x110), t.alloc(4), t.alloc(0x20));
    for k in 0..200u32 {
        env.reset(&mut t, &mut rng);
        // the animation slots of the renderer
        t.fill(render + 0x9cac0, 0x28 * 0x40, &mut rng);
        for i in 0..0x40 {
            let e = render + 0x9cac0 + 0x28 * i;
            t.put(e + 4, u16::try_from(rng.below(4)).unwrap());
            t.put(e + 6, u16::try_from(rng.below(5)).unwrap());
        }
        t.put(render + 0x9cabc, rng.below(70));
        let obj = rng.below(6);
        let kind = if rng.below(2) == 0 {
            rng.below(4)
        } else {
            0x8000_0000 | rng.next()
        };
        t.same(
            "yiDisposeAnimDObj",
            |x| x.eax(0x0090_84b0, &[obj, kind]),
            |m| yi::bridge::yi_dispose_anim_dobj(m, obj, kind, call),
        );
        // the effect object
        t.set(tb::CURRENT_PROGRAM, rng.next());
        t.same(
            "rcefObjFini",
            |x| x.eax(0x0092_fee0, &[o]),
            |m| tb::rcef_obj_fini(m, o, call),
        );
        // the command menus
        t.fill(entries, 4 * 64, &mut rng);
        for i in 0..64 {
            t.put(
                entries + 4 * i,
                0x40u8 + u8::try_from(rng.below(6)).unwrap(),
            );
        }
        t.put(nv, rng.range(-1, 64).cast_unsigned());
        t.put(exitv, rng.next());
        let pad = 2 + rng.below(4);
        t.fill(pads + 0x340 * pad, 0x340, &mut rng);
        t.put(
            pads + 0x340 * pad + 0x1e,
            i16::try_from(rng.range(-4, 0x42)).unwrap(),
        );
        let mk = 0x40 + rng.below(6);
        let work = t.stack_scratch(0);
        t.same(
            "AbmapFillCommandMenu",
            |x| x.eax(0x00a4_59e0, &[pad, mk]),
            |m| abmap::bridge::abmap_fill_command_menu(m, pad, mk, work, call),
        );
        // the guide's edge fade
        for at in [a, b, p] {
            for i in 0..2 {
                t.put(at + 4 * i, float_in(&mut rng, -200, 200));
            }
        }
        if k % 5 == 0 {
            t.put(b, t.b.read::<u32>(a));
            t.put(b + 4, t.b.read::<u32>(a + 4));
        }
        for i in 0..4 {
            t.put(rect + 4 * i, float_in(&mut rng, -2000, 2000));
        }
        if k % 11 == 0 {
            t.put(rect, 0x7fc0_0000u32);
        }
        t.fill(out, 4, &mut rng);
        let work = t.stack_scratch(0);
        t.same(
            "Yn_GuideEdgeFade",
            |x| x.eax(0x0091_f530, &[p, a, b, rect, out]),
            |m| guide::bridge::yn_guide_edge_fade(m, [p, a, b, rect, out], work),
        );
        // the character distance
        t.put(plyv, if rng.below(3) == 0 { 0 } else { rng.next() });
        for i in 0..3 {
            t.put(pos + 4 * i, float_in(&mut rng, -5000, 5000));
        }
        for i in 0..16 {
            let m = t.b.rebase(camera::CAMERA_MATRIX) + 4 * i;
            t.put(m, float_in(&mut rng, -2, 2));
        }
        t.set(guide::YN_MAP_SYSTEM, sys);
        t.put(sys, map);
        t.fill(scene, 0x110, &mut rng);
        let work = t.stack_scratch(0);
        t.same(
            "rcbgSceneSetChadist",
            |x| x.eax(0x0092_0790, &[scene]),
            |m| model::bridge::rcbg_scene_set_chadist(m, scene, work, call),
        );
    }
}

/// Round 23: the effect sequences' waits and forwarders.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round23_waits_and_forwarders_match_the_original() {
    use task::{bridge as tb, header};
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0017_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let log = env.log;
    let v: Vec<u32> = (0..7).map(|_| t.alloc(4)).collect();
    let [cue, mot, hv, ev, ov, rv2, rv3] = v[..] else {
        unreachable!()
    };
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(tb::SOUND_CUE_CHECK), log_stub(log, cue, 250, 1)),
        (Some(tb::MS_EFFECT_CHECK_MOTION), log_stub(log, mot, 251, 1)),
        (Some(tb::MS_SET_CHR_DIR_TARGET), log_stub(log, rv2, 252, 6)),
        (Some(tb::MS_GET_WEAPON_HANDLE), log_stub(log, hv, 253, 2)),
        (Some(tb::CH_SET_HIDE_EFFECT), log_stub(log, rv2, 254, 2)),
        (Some(tb::OP_OEF2_A_GET), log_stub(log, ev, 255, 3)),
        (Some(tb::OP_OEF2_SEP_NUMBER_GET), log_stub(log, rv3, 256, 1)),
        (Some(tb::OP_OEF2_SE_ON), log_stub(log, rv2, 257, 3)),
        (Some(tb::RCEF_MAIN_CREATE_OBJ), log_stub(log, ov, 258, 1)),
        (Some(tb::RCEF_TASK_SET_SEQ), log_stub(log, rv3, 259, 2)),
        (Some(tb::RCEF_CHR_FINI), log_stub(log, rv3, 260, 1)),
        (Some(task::PPP_MEM_FREE), log_stub(log, rv2, 261, 3)),
    ];
    t.cave_from(env.cave_end, &list);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let task = t.alloc(0xbc);
    let envw = t.alloc(4);
    for k in 0..200u32 {
        env.reset(&mut t, &mut rng);
        for &w in &v {
            t.put(w, if rng.below(3) == 0 { 0 } else { rng.next() });
        }
        if k % 4 == 0 {
            t.put(mot, u32::MAX);
        }
        t.fill(task, 0xbc, &mut rng);
        t.put(task + header::LEVEL, rng.range(0, 8));
        t.same(
            "rcEffectTaskSoundCueWait",
            |o| o.void(0x0093_0500, &[task]),
            |m| tb::rc_effect_task_sound_cue_wait(m, task, call),
        );
        t.same(
            "rcEffectTaskMotionWait",
            |o| o.void(0x0093_0430, &[task]),
            |m| tb::rc_effect_task_motion_wait(m, task, call),
        );
        let a = [rng.next(), rng.next(), rng.next(), rng.next()];
        t.same(
            "rcefSetChrDirTarget",
            |o| o.eax(0x0092_e750, &a[..3]),
            |m| tb::rcef_set_chr_dir_target(m, [a[0], a[1], a[2]], call),
        );
        t.same(
            "rcefSetChrDirTarget2",
            |o| o.eax(0x0092_e960, &a),
            |m| tb::rcef_set_chr_dir_target2(m, a, call),
        );
        let show = rng.below(3);
        t.same(
            "rcefWeaponSetHide",
            |o| o.eax(0x0092_eb70, &[a[0], show]),
            |m| tb::rcef_weapon_set_hide(m, a[0], show, call),
        );
        t.set(tb::RCEF_OEF2, rng.next());
        t.same(
            "rcefPlaySE",
            |o| o.eax(0x0092_e680, &[a[1]]),
            |m| tb::rcef_play_se(m, a[1], call),
        );
        t.same(
            "rcefMainCreateTask",
            |o| o.eax(0x0092_5170, &[a[0], a[1]]),
            |m| tb::rcef_main_create_task(m, a[0], a[1], call),
        );
        t.set(tb::PPP_ENV_SLOT, envw);
        t.put(envw, rng.next());
        t.same(
            "rcefChrDelete",
            |o| o.eax(0x0093_0600, &[a[2]]),
            |m| tb::rcef_chr_delete(m, a[2], call),
        );
    }
}

/// Round 23, second part: Yn's pre-BG layer switches and character
/// distance, the rcfx deletes, the `FPm173` end and the transfer pair.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round23_yn_layers_and_rcfx_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0017_0002);
    let env = LoadEnv::install(&mut t, &mut rng);
    let log = env.log;
    let v: Vec<u32> = (0..6).map(|_| t.alloc(4)).collect();
    let [cv, sv, gv, rv2, rv3, _] = v[..] else {
        unreachable!()
    };
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (
            Some(model::SCENE_SET_CHADIST_THUNK),
            log_stub(log, rv2, 262, 1),
        ),
        (Some(model::YN_MAP_PART_COUNT), log_stub(log, cv, 263, 0)),
        (
            Some(model::GRAPHIC_2D_HIDE_LAYER),
            log_stub(log, rv3, 264, 1),
        ),
        (
            Some(model::GRAPHIC_2D_SHOW_LAYER),
            log_stub(log, rv3, 265, 1),
        ),
        (
            Some(model::TEX_ANIM_SET_VISIBILITY),
            log_stub(log, rv2, 266, 2),
        ),
        (Some(model::RCFX_SPRING_FINI), log_stub(log, rv3, 267, 1)),
        (Some(model::RCFX_NAIL_FINI), log_stub(log, rv3, 268, 1)),
        (Some(model::YN_SCENE_GET_OBJECT), log_stub(log, gv, 269, 2)),
        (Some(model::FPM173_END), log_stub(log, rv2, 270, 1)),
        (Some(model::TRX_SET_NO), log_stub(log, rv2, 271, 2)),
        (None, log_stub(log, sv, 272, 2)),
    ];
    let (stubs, _) = t.cave_from(env.cave_end, &list);
    let strstr = *stubs.last().unwrap();
    t.put(t.b.rebase(model::STRSTR_IMPORT), strstr);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (sys, map, scene, parts) = (
        t.alloc(4),
        t.alloc(0x20),
        t.alloc(0x20),
        t.alloc(0xc0 * 0x30),
    );
    for _ in 0..200u32 {
        env.reset(&mut t, &mut rng);
        for &w in &v {
            t.put(w, if rng.below(3) == 0 { 0 } else { rng.next() });
        }
        let a = [rng.next(), rng.next(), rng.next(), rng.next()];
        // Yn
        let (s, m0) = match rng.below(3) {
            0 => (0, 0),
            1 => (sys, 0),
            _ => (sys, map),
        };
        t.set(guide::YN_MAP_SYSTEM, s);
        t.put(sys, m0);
        t.put(map + 8, scene);
        t.same(
            "Yn_SetChadist",
            |o| {
                let e = o.eax(0x0091_a250, &[]);
                (s != 0 && m0 != 0).then_some(e)
            },
            |m| model::bridge::yn_set_chadist(m, call),
        );
        // the layer switches need the scene (the original reads address 8
        // without one)
        t.set(guide::YN_MAP_SYSTEM, sys);
        t.put(sys, map);
        t.put(scene + 8, parts);
        t.fill(parts, 0xc0 * 0x30, &mut rng);
        t.put(cv, rng.range(-2, 0x30).cast_unsigned());
        let mapno = match rng.below(4) {
            0 => 0x15b,
            1 => 0xd4,
            _ => rng.below(0x400),
        };
        t.put(t.b.rebase(model::CURRENT_MAP_NO), mapno);
        let n = rng.below(0x30);
        t.same(
            "Yn_PreBgLayerDispOff",
            |o| o.eax(0x0091_a7c0, &[n]),
            |m| model::bridge::yn_pre_bg_layer_disp_off(m, n, call),
        );
        t.same(
            "Yn_PreBgLayerDispOn",
            |o| o.eax(0x0091_a830, &[n]),
            |m| model::bridge::yn_pre_bg_layer_disp_on(m, n, call),
        );
        // rcfx, FPm173, transfers
        t.same(
            "rcfxSpringDelete",
            |o| o.eax(0x0092_8c60, &[a[3]]),
            |m| model::bridge::rcfx_spring_delete(m, a[3], call),
        );
        t.same(
            "rcfxNailDelete",
            |o| o.eax(0x0092_9070, &[a[3]]),
            |m| model::bridge::rcfx_nail_delete(m, a[3], call),
        );
        t.same(
            "Yn_FPm173End",
            |o| o.eax(0x0091_9ab0, &[]),
            |m| model::bridge::yn_fpm173_end(m, call),
        );
        t.same(
            "rcbgTrxPairSetNo",
            |o| o.eax(0x0092_21e0, &[a[0], a[1]]),
            |m| model::bridge::rcbg_trx_pair_set_no(m, a[0], a[1], call),
        );
    }
}

/// Round 24: the aeons' stat gains and the effect's stream, vibration
/// and object-start forwarders.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round24_aeon_gains_and_streams_match_the_original() {
    use ffx_field::nkz;
    use task::bridge as tb;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0018_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let log = env.log;
    let v: Vec<u32> = (0..4).map(|_| t.alloc(4)).collect();
    let [cue, ev, rv2, rv3] = v[..] else {
        unreachable!()
    };
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(nkz::DBG_PRINTF), log_stub(log, rv3, 280, 2)),
        (Some(tb::SOUND_CUE_CHECK), log_stub(log, cue, 281, 1)),
        (Some(tb::OP_OEF2_A_GET), log_stub(log, ev, 282, 3)),
        (Some(tb::OP_OEF2_SE_ON), log_stub(log, rv2, 283, 3)),
        (Some(tb::OP_OEF2_STREAM_CUE), log_stub(log, rv2, 284, 3)),
        (Some(tb::TK_SET_VIBRATE_STREAM), log_stub(log, rv2, 285, 3)),
        (Some(tb::RCEF_ANIME_START), log_stub(log, rv2, 286, 4)),
        (Some(tb::RCEF_SETUP_DRAW), log_stub(log, rv2, 287, 4)),
    ];
    t.cave_from(env.cave_end, &list);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let halves = [
        0x00a5_c660u32,
        0x00a5_c740,
        0x00a5_c910,
        0x00a5_c980,
        0x00a5_c9f0,
        0x00a5_c860,
        0x00a5_c6d0,
        0x00a5_c7f0,
    ];
    let envp = t.alloc(0x10);
    for k in 0..400u32 {
        env.reset(&mut t, &mut rng);
        for &w in &v {
            t.put(w, if rng.below(3) == 0 { 0 } else { rng.next() });
        }
        let pc = if rng.below(4) == 0 {
            rng.next()
        } else {
            rng.below(20)
        };
        let val = match k % 4 {
            0 => rng.range(-3, 4),
            1 => rng.range(0, 100_000),
            2 => rng.next().cast_signed(),
            _ => rng.range(0, 300),
        };
        let f = halves[(k % 8) as usize];
        t.same(
            "NkzAeonGain (half)",
            |o| o.eax(f, &[pc, val.cast_unsigned()]),
            |m| nkz::bridge::nkz_aeon_gain_half(m, pc, val, call),
        );
        t.same(
            "NkzAeonGainHp",
            |o| o.eax(0x00a5_c7b0, &[pc, val.cast_unsigned()]),
            |m| nkz::bridge::nkz_aeon_gain_hp(m, val),
        );
        t.same(
            "NkzAeonGainMp",
            |o| o.eax(0x00a5_c8d0, &[pc, val.cast_unsigned()]),
            |m| nkz::bridge::nkz_aeon_gain_mp(m, val),
        );
        // the effect forwarders
        t.set(tb::RCEF_OEF2, rng.next());
        let (id, arg, o) = (rng.next(), rng.next(), rng.next());
        t.same(
            "rcefSetVibrate",
            |x| x.eax(0x0092_ec60, &[id, arg]),
            |m| tb::rcef_set_vibrate(m, id, arg, call),
        );
        t.same(
            "rcefStreamCue",
            |x| x.eax(0x0092_ebf0, &[id, arg]),
            |m| tb::rcef_stream_cue(m, id, arg, call),
        );
        t.same(
            "rcefPlayStream",
            |x| x.eax(0x0092_e6b0, &[id]),
            |m| tb::rcef_play_stream(m, id, call),
        );
        t.put(envp + 8, rng.next());
        t.same(
            "rcefObjAnimeStart",
            |x| x.eax(0x0092_5590, &[envp, o, id]),
            |m| tb::rcef_obj_anime_start(m, [envp, o, id], call),
        );
        t.same(
            "rcefObjSetupDraw",
            |x| x.eax(0x0092_5690, &[envp, o, id]),
            |m| tb::rcef_obj_setup_draw(m, [envp, o, id], call),
        );
    }
}

/// Round 25: the compiler's copies of `rcuiScaleVector` and
/// `rcVec4fToFixed`, the display objects' morph value, the colour offset
/// as ints, the effect's interpolation and colour work.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round25_copies_morph_lerp_and_colours_match_the_original() {
    use task::bridge as tb;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0019_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let (log, rv) = (env.log, env.rv);
    let list: Vec<(Option<u32>, Vec<u8>)> =
        vec![(Some(yi::SET_MORPH_VALUE), log_stub(log, rv, 290, 2))];
    t.cave_from(env.cave_end, &list);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (dst, v, p) = (t.alloc(0x10), t.alloc(0x10), t.alloc(4));
    let work_at = t.b.rebase(tb::COLOR_WORK);
    for k in 0..300u32 {
        env.reset(&mut t, &mut rng);
        for i in 0..4 {
            let x = if k % 13 == 7 {
                rng.next() | 0x0040_0000
            } else {
                float_in(&mut rng, -3000, 3000)
            };
            t.put(v + 4 * i, x);
        }
        t.fill(dst, 0x10, &mut rng);
        let (s, a, b, c) = (
            float_in(&mut rng, -8, 8),
            rng.next(),
            rng.next(),
            rng.next(),
        );
        t.same(
            "rcuiScaleVector_copy_00756600",
            |o| o.eax(0x0075_6600, &[dst, v, s, a, b, c]),
            |m| vu0::bridge::rcui_scale_vector(m, dst, v, s, [a, b, c]),
        );
        t.same(
            "rcVec4fToFixed_copy_007e6cc0",
            |o| o.eax(0x007e_6cc0, &[dst, v]),
            |m| model::bridge::rc_vec4f_to_fixed(m, dst, v),
        );
        // the morph value
        t.set(yi::DOBJ_COUNT, rng.below(0x40));
        let obj = rng.below(0x50);
        t.put(p, float_in(&mut rng, -4, 4));
        t.same(
            "yiSetMIMeParamDObj",
            |o| o.eax(0x0090_a4d0, &[obj, p]),
            |m| yi::bridge::yi_set_mime_param_dobj(m, obj, p, call),
        );
        // the colour offset
        for i in 0..4 {
            let at = t.b.rebase(0x018d_f01c) + 4 * i;
            t.put(at, float_in(&mut rng, -400, 400));
        }
        t.same(
            "yiGetColorOffsetInt",
            |o| o.eax(0x0090_8660, &[dst]),
            |m| yi::bridge::yi_get_color_offset_int(m, dst),
        );
        // the interpolation
        let mode = rng.below(4);
        let (a0, b0) = (float_in(&mut rng, -500, 500), float_in(&mut rng, -500, 500));
        let tt = if k % 5 == 0 {
            float_in(&mut rng, -4, 4)
        } else {
            float_in(&mut rng, 0, 1)
        };
        t.same(
            "rcefLerp",
            |o| o.float(0x0092_e580, &[a0, b0, tt, mode]),
            |m| bits(tb::rcef_lerp(m, [a0, b0, tt, mode])),
        );
        // the colour work
        t.fill(work_at + 0x1c0, 0x40, &mut rng);
        let cols: Vec<u32> = (0..9).map(|_| rng.next()).collect();
        let work = t.stack_scratch(0);
        t.same(
            "rcefColorSet",
            |o| o.void(0x0092_e9c0, &cols[..5]),
            |m| {
                let a = [cols[0], cols[1], cols[2], cols[3], cols[4]];
                tb::rcef_color_set(m, a, work);
            },
        );
        t.fill(work_at + 0x1c0, 0x40, &mut rng);
        let mut a9 = [0u32; 9];
        a9.copy_from_slice(&cols);
        t.same(
            "rcefColorFade",
            |o| o.void(0x0092_ea40, &a9),
            |m| tb::rcef_color_fade(m, a9, work),
        );
    }
}

/// Round 26: the effect's screen shake, the scene's wave and the sphere
/// grid's part start.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round26_shake_wave_and_part_start_match_the_original() {
    use ffx_field::abmap;
    use task::bridge as tb;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x001a_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let (log, rv) = (env.log, env.rv);
    let (randv, tabv) = (t.alloc(4), t.alloc(4));
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(tb::MS_SCREEN_OFFSET), log_stub(log, rv, 300, 2)),
        (Some(model::SCENE_WAVE_C0_STOP), log_stub(log, rv, 301, 1)),
        (Some(abmap::GET_PTR32_FUNC), log_stub(log, tabv, 302, 1)),
        (Some(abmap::PPP_START_PART), log_stub(log, rv, 303, 5)),
        (None, log_stub(log, randv, 304, 0)),
    ];
    let (stubs, _) = t.cave_from(env.cave_end, &list);
    let rand = *stubs.last().unwrap();
    t.put(t.b.rebase(model::RAND_IMPORT), rand);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (e, s) = (t.alloc(0x220), t.alloc(0xe0));
    let (part, tab, data) = (t.alloc(0x80), t.alloc(0x40), t.alloc(0x20));
    let slot = t.alloc(4);
    t.put(t.b.rebase(abmap::PPP_ENV_SLOT), slot);
    t.put(t.b.rebase(abmap::ABMAP_PPP_DATA), data);
    t.put(tabv, tab);
    for k in 0..400u32 {
        env.reset(&mut t, &mut rng);
        t.put(randv, rng.below(0x8000));
        // the shake
        t.fill(e + 0x1f0, 0x30, &mut rng);
        t.put(e + 0x214, rng.below(3));
        t.put(e + 0x1f8, rng.below(6));
        t.put(e + 0x210, float_in(&mut rng, -10, 10));
        t.put(e + 0x1fc, float_in(&mut rng, -1, 1));
        t.put(e + 0x200, rng.range(-2, 20));
        t.put(e + 0x204, rng.range(0, 22));
        if k % 11 == 3 {
            // frames beyond f32's integers
            t.put(e + 0x200, rng.range(0x0100_0001, 0x7fff_ffff));
        }
        let range = if rng.below(2) == 0 {
            rng.range(1, 60)
        } else {
            rng.range(-60, -1)
        };
        t.put(e + 0x20c, range);
        t.put(e + 0x208, rng.range(-100, 100));
        t.same(
            "rcefScreenShake",
            |o| o.void(0x0092_4df0, &[e]),
            |m| tb::rcef_screen_shake(m, e, call),
        );
        // the wave
        t.fill(s + 0xb0, 0x30, &mut rng);
        t.put(s + 0xc0, rng.below(6));
        t.put(s + 0xd4, float_in(&mut rng, -10, 10));
        t.put(s + 0xc4, float_in(&mut rng, -1, 1));
        t.put(s + 0xc8, rng.range(-2, 20));
        t.put(s + 0xd8, rng.range(0, 22));
        if k % 11 == 4 {
            t.put(s + 0xc8, rng.range(0x0100_0001, 0x7fff_ffff));
        }
        t.put(s + 0xd0, rng.range(-3, 40));
        t.put(s + 0xcc, rng.range(-100, 100));
        if k % 9 == 0 {
            t.put(s + 0xcc, rng.next());
        }
        t.same(
            "rcbgSceneWaveC0Update",
            |o| o.void(0x0092_10e0, &[s]),
            |m| model::bridge::rcbg_scene_wave_c0_update(m, s, call),
        );
        // the part start
        t.fill(part, 0x80, &mut rng);
        t.fill(tab, 0x40, &mut rng);
        t.fill(data, 0x20, &mut rng);
        let mut a = [0u32; 11];
        a[0] = part;
        a[1] = rng.below(0x10);
        for v in &mut a[2..] {
            *v = float_in(&mut rng, -3000, 3000);
        }
        if k % 7 == 0 {
            a[7] = rng.next() | 0x0040_0000;
        }
        t.same(
            "AbmapStartPart",
            |o| o.eax(0x00a5_bad0, &a),
            |m| abmap::bridge::abmap_start_part(m, a, call),
        );
    }
}

/// Round 27: the Phyre field map load, the sphere grid's warp start and
/// the 2D direction angle.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round27_map_load_warp_and_angle_match_the_original() {
    use ffx_field::abmap;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x001b_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    t.install_trig();
    let (log, rv) = (env.log, env.rv);
    let (iobv, tabv) = (t.alloc(4), t.alloc(4));
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(yi::GRAPHIC_SET_HAS_2D_MAP), log_stub(log, rv, 310, 1)),
        (Some(yi::GRAPHIC_SET_HAS_3D_MAP), log_stub(log, rv, 311, 1)),
        (Some(yi::GRAPHIC_FIELD_MAP_LOAD), log_stub(log, rv, 312, 2)),
        (
            Some(yi::GRAPHIC_FIELD_MAP_INITIALIZE),
            log_stub(log, rv, 313, 0),
        ),
        (
            Some(yi::GRAPHIC_VFX_LOAD_LEVEL_DATA),
            log_stub(log, rv, 314, 0),
        ),
        (Some(abmap::SND_SEP_PLAY_SIMPLE), log_stub(log, rv, 315, 1)),
        (Some(abmap::PPP_CREATE_HEAP), log_stub(log, rv, 316, 3)),
        (Some(abmap::GET_PTR32_FUNC), log_stub(log, tabv, 317, 1)),
        (Some(abmap::PPP_START_PART), log_stub(log, rv, 318, 5)),
        (None, log_stub(log, iobv, 319, 0)),
        (None, log_stub(log, rv, 320, 4)),
    ];
    let (stubs, _) = t.cave_from(env.cave_end, &list);
    let n = stubs.len();
    t.put(t.b.rebase(yi::CRT_IOB_FUNC), stubs[n - 2]);
    t.put(t.b.rebase(yi::CRT_FPRINTF), stubs[n - 1]);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let fcall = &mut oracle_fcall;
    let map = t.alloc(0x40);
    let grid = t.alloc(0x1_2000);
    t.set(abmap::ABMAP, grid);
    let (tab, data, slot) = (t.alloc(0x40), t.alloc(0x20), t.alloc(4));
    t.put(t.b.rebase(abmap::PPP_ENV_SLOT), slot);
    t.put(t.b.rebase(abmap::ABMAP_PPP_DATA), data);
    t.put(tabv, tab);
    let name = t.b.rebase(yi::PENDING_MAP_NAME);
    for k in 0..300u32 {
        env.reset(&mut t, &mut rng);
        t.put(iobv, rng.next());
        // the map load
        t.fill(map, 0x40, &mut rng);
        if rng.below(4) != 0 {
            t.write(map, b"MAP1");
        }
        for i in 0..2 {
            t.put(
                map + 0x10 + 4 * i,
                if rng.below(2) == 0 { 0 } else { rng.next() },
            );
        }
        t.set(yi::PENDING_MAP, if rng.below(5) == 0 { 0 } else { map });
        t.put(name, if rng.below(5) == 0 { 0u8 } else { b'm' });
        t.set(yi::RENDER_STATUS, rng.next());
        t.set(yi::BTL_MAP_READ, rng.next());
        let pending = t.b.read::<u32>(t.b.rebase(0x0193_4ed0)) != 0 && t.b.read::<u8>(name) != 0;
        t.same(
            "loadPhyreFieldMap",
            |o| {
                let e = o.eax(0x0090_8000, &[]);
                pending.then_some(e)
            },
            |m| yi::bridge::load_phyre_field_map(m, call),
        );
        // the warp
        t.fill(grid + 0x11000, 0x700, &mut rng);
        let chr = rng.below(7);
        let node = rng.below(40);
        t.put(grid + 0x110cc + 0x50 * chr, u16::try_from(node).unwrap());
        t.fill(grid + 0x808 + 0x28 * node, 4, &mut rng);
        for at in [0x115b0, 0x115b4] {
            if rng.below(2) == 0 {
                t.put(grid + at, 0u32);
            }
        }
        t.fill(tab, 0x40, &mut rng);
        let target = rng.next();
        t.same(
            "AbmapStartWarp",
            |o| o.eax(0x00a4_8a80, &[chr, target]),
            |m| abmap::bridge::abmap_start_warp(m, chr, target, call),
        );
        // the angle
        let mut a = [0u32; 4];
        for v in &mut a {
            *v = float_in(&mut rng, -500, 500);
        }
        if k % 6 == 0 {
            a[2] = a[0];
        }
        if k % 10 == 1 {
            a[3] = 0x7fc0_0000;
        }
        t.same(
            "rcAngle2D",
            |o| o.float(0x007e_6d80, &a),
            |m| bits(geom::bridge::rc_angle_2d(m, a, &mut *fcall)),
        );
    }
}

/// Round 28: the sphere grid's `abmap_set_chr_pos`, the effect's blur
/// update and the nail effect's point update.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round28_set_chr_pos_blur_and_nail_match_the_original() {
    use ffx_field::abmap;
    use task::bridge as tb;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x001c_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let (log, rv) = (env.log, env.rv);
    let tabv = t.alloc(4);
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(abmap::SND_SEP_PLAY_SIMPLE), log_stub(log, rv, 330, 1)),
        (Some(abmap::PPP_CREATE_HEAP), log_stub(log, rv, 331, 3)),
        (Some(abmap::GET_PTR32_FUNC), log_stub(log, tabv, 332, 1)),
        (Some(abmap::PPP_START_PART), log_stub(log, rv, 333, 5)),
        (Some(tb::SG_BLUR), log_stub(log, rv, 334, 1)),
    ];
    t.cave_from(env.cave_end, &list);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let grid = t.alloc(0x1_2000);
    t.set(abmap::ABMAP, grid);
    let (tab, data, slot) = (t.alloc(0x40), t.alloc(0x20), t.alloc(4));
    t.put(t.b.rebase(abmap::PPP_ENV_SLOT), slot);
    t.put(t.b.rebase(abmap::ABMAP_PPP_DATA), data);
    t.put(tabv, tab);
    let e = t.alloc(0x230);
    let (fx, ctx, idx, src, m) = (
        t.alloc(0x40),
        t.alloc(0x30),
        t.alloc(0x40),
        t.alloc(0x200),
        t.alloc(0x40),
    );
    let arrays: Vec<u32> = (0..3).map(|_| t.alloc(0x100)).collect();
    for k in 0..300u32 {
        env.reset(&mut t, &mut rng);
        // abmap_set_chr_pos
        t.fill(grid + 0x11000, 0x700, &mut rng);
        let chr = rng.below(7);
        let node = rng.below(40);
        t.put(grid + 0x110cc + 0x50 * chr, u16::try_from(node).unwrap());
        t.fill(grid + 0x808 + 0x28 * node, 4, &mut rng);
        t.fill(tab, 0x40, &mut rng);
        let target = rng.next();
        t.same(
            "abmap_set_chr_pos",
            |o| o.eax(0x00a4_58f0, &[chr, target]),
            |m| abmap::bridge::abmap_set_chr_pos(m, chr, target, call),
        );
        // the blur
        t.fill(e + 0x210, 0x20, &mut rng);
        t.put(e + 0x228, rng.below(3));
        t.put(e + 0x218, rng.below(4));
        t.put(e + 0x21c, rng.range(-300, 300));
        t.put(e + 0x220, rng.range(-2, 20));
        t.put(e + 0x224, rng.range(0, 22));
        if k % 9 == 0 {
            t.put(e + 0x21c, rng.next());
        }
        t.same(
            "rcefBlurUpdate",
            |o| o.void(0x0092_4bf0, &[e]),
            |m| tb::rcef_blur_update(m, e, call),
        );
        // the nail
        t.put(fx + 0x1c, rng.range(-1, 15));
        t.put(fx + 0x20, idx);
        for (i, &a) in (0u32..).zip(&arrays) {
            t.put(fx + 0x24 + 4 * i, a);
            t.fill(a, 0x100, &mut rng);
        }
        t.put(fx + 0x38, if rng.below(2) == 0 { 0 } else { m });
        for i in 0..16 {
            t.put(idx + 4 * i, rng.below(0x20));
            t.put(m + 4 * i, float_in(&mut rng, -4, 4));
        }
        for i in 0..0x80 {
            t.put(src + 4 * i, float_in(&mut rng, -1000, 1000));
        }
        t.put(ctx + 0x24, src);
        t.put(
            ctx,
            (u64::from(rng.next()) << 32 | u64::from(rng.next())) | 1 << 51,
        );
        if k % 7 == 0 {
            // a quiet NaN (the emulator keeps signalling ones, which the
            // hardware's fld quiets)
            t.put(ctx, 0x7ff8_0000_0000_0001u64);
        }
        t.same(
            "rcfxNailUpdate",
            |o| o.eax(0x0092_9190, &[fx, ctx]),
            |m| model::bridge::rcfx_nail_update(m, fx, ctx),
        );
    }
}

/// A file reader's stand-in: its first argument to `idlog`, then 0x480
/// bytes from `src` to its second argument.
fn read_file_stub(src: u32, idlog: u32) -> Vec<u8> {
    let mut c = vec![0x56, 0x57, 0x8b, 0x44, 0x24, 0x0c, 0xa3];
    c.extend_from_slice(&idlog.to_le_bytes());
    c.extend_from_slice(&[0x8b, 0x7c, 0x24, 0x10, 0xbe]);
    c.extend_from_slice(&src.to_le_bytes());
    c.extend_from_slice(&[0xb9, 0x80, 0x04, 0x00, 0x00, 0xf3, 0xa4, 0x5f, 0x5e, 0xc3]);
    c
}

/// Round 29: the sphere grid's achievement check and its special item
/// kinds.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round29_achievements_and_special_kinds_match_the_original() {
    use ffx_field::abmap;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x001d_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let (log, rv) = (env.log, env.rv);
    let v: Vec<u32> = (0..7).map(|_| t.alloc(4)).collect();
    let [menuv, savev, idlog, panelv, otherv, nodev, joinedv] = v[..] else {
        unreachable!()
    };
    let (file, cdtab, cdptr) = (t.alloc(0x480), t.alloc(0x20), t.alloc(4));
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(abmap::MARK_LOCKED_ENTRIES), log_stub(log, rv, 340, 1)),
        (Some(abmap::MS_GET_SAVE_MENU), log_stub(log, menuv, 341, 0)),
        (Some(abmap::GET_CD), log_stub(log, cdptr, 342, 0)),
        (
            Some(abmap::MS_GET_SAVE_ABILITY_MAP),
            log_stub(log, savev, 343, 0),
        ),
        (Some(abmap::ACHIEVEMENT_UNLOCK), log_stub(log, rv, 344, 1)),
        (Some(abmap::ABMAP_TEST_PANEL), log_stub(log, panelv, 345, 2)),
        (
            Some(abmap::ABMAP_OTHER_HAS_PANEL),
            log_stub(log, otherv, 346, 2),
        ),
        (Some(abmap::ABMAP_CHR_NODE), log_stub(log, nodev, 347, 1)),
        (
            Some(abmap::MS_GET_SAVE_PLY_JOINED),
            log_stub(log, joinedv, 348, 1),
        ),
        (None, log_stub(log, rv, 349, 1)),
        (None, read_file_stub(file, idlog)),
        (None, log_stub(log, rv, 350, 1)),
    ];
    let (stubs, _) = t.cave_from(env.cave_end, &list);
    let n = stubs.len();
    // the CD table: +0 the reader, +8 the closer, +0x10 the opener
    t.put(cdtab + 0x10, stubs[n - 3]);
    t.put(cdtab, stubs[n - 2]);
    t.put(cdtab + 8, stubs[n - 1]);
    // GetCD returns the table (the stub returns the word at cdptr)
    t.put(cdptr, cdtab);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (menu, save, row) = (t.alloc(4), t.alloc(0x900), t.alloc(0x20));
    t.put(menuv, menu);
    t.put(savev, save);
    for k in 0..300u32 {
        env.reset(&mut t, &mut rng);
        // the achievements
        t.put(menu, rng.next());
        t.fill(file, 0x480, &mut rng);
        // (without the 0x31 header the original counts to the ability
        // map's address, reading far past both buffers: left out)
        let n = rng.range(-2, 0x200);
        t.put(file, 0x0031u16);
        t.put(file + 2, i16::try_from(n).unwrap());
        for j in 0..0x470 {
            if rng.below(3) == 0 {
                t.put(file + 8 + j, 0xffu8);
            }
        }
        let byte = match k % 3 {
            0 => 0x7fu8,
            1 => 0xff,
            _ => 0,
        };
        for j in 0..0x480u32 {
            let b = if rng.below(40) == 0 {
                u8::try_from(rng.below(0x100)).unwrap()
            } else {
                byte
            };
            t.put(save + j, b);
        }
        let work = t.stack_scratch(0x100);
        t.same(
            "AbmapCheckAchievements",
            |o| o.void(0x00a4_7210, &[]),
            |m| abmap::bridge::abmap_check_achievements(m, work, call),
        );
        // the special kinds
        for _ in 0..20 {
            env.reset(&mut t, &mut rng);
            let kind = rng.below(0x1a);
            let (node, chr) = (rng.below(8), rng.below(7));
            t.fill(row, 0x20, &mut rng);
            let ty = if rng.below(2) == 0 {
                [0u8, 1, 0xc, 0xd, 0xe, 0xf, 0x10, 0x11, 0x12][rng.below(9) as usize]
            } else {
                u8::try_from(rng.below(0x14)).unwrap()
            };
            t.put(row + 0x16, ty);
            t.put(panelv, rng.below(2));
            t.put(otherv, rng.below(2));
            t.put(
                nodev,
                if rng.below(2) == 0 {
                    node
                } else {
                    rng.below(8)
                },
            );
            t.put(joinedv, rng.below(2));
            t.same(
                "AbmapSpecialEffect",
                |o| o.eax(0x00a5_d120, &[kind, node, row, chr]),
                |m| abmap::bridge::abmap_special_effect(m, [kind, node, row, chr], call),
            );
        }
    }
}

/// Copies 0x40 bytes from `src` to the stand-in's second argument
/// (`rcbgFPmGetMatrix(fp, out)`'s shape).
fn copy64_stub(src: u32) -> Vec<u8> {
    let mut c = vec![0x56, 0x57, 0x8b, 0x7c, 0x24, 0x10, 0xbe];
    c.extend_from_slice(&src.to_le_bytes());
    c.extend_from_slice(&[0xb9, 0x40, 0x00, 0x00, 0x00, 0xf3, 0xa4, 0x5f, 0x5e, 0xc3]);
    c
}

/// Round 30: the sphere grid's bounds and the sea's field-effect fade.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round30_grid_bounds_and_sea_fade_match_the_original() {
    use ffx_field::abmap;
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x001e_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let (log, rv) = (env.log, env.rv);
    let (angv, mngv, mat) = (t.alloc(4), t.alloc(4), t.alloc(0x40));
    // pppFpGetMngPtr(id, 0, &out): out = *mngv
    let mut mng_stub = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d];
    mng_stub.extend_from_slice(&mngv.to_le_bytes());
    mng_stub.extend_from_slice(&[0x89, 0x08, 0xc3]);
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![
        (Some(model::YN_SEA_GET_ANG_PTR), log_stub(log, angv, 360, 0)),
        (Some(model::PPP_FP_ID_DISP_ON), log_stub(log, rv, 361, 1)),
        (Some(model::PPP_FP_ID_DISP_OFF), log_stub(log, rv, 362, 1)),
        (Some(model::FP_RESTART), log_stub(log, rv, 363, 1)),
        (Some(model::FP_END), log_stub(log, rv, 364, 1)),
        (Some(model::FPM_GET_MATRIX), copy64_stub(mat)),
        (Some(model::PPP_FP_GET_MNG_PTR), mng_stub),
    ];
    t.cave_from(env.cave_end, &list);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let grid = t.alloc(0x1_2000);
    t.set(abmap::ABMAP, grid);
    let (fade, ang, mng) = (t.alloc(0x14), t.alloc(0x10), t.alloc(0x50));
    t.put(angv, ang);
    let chunks = t.b.rebase(model::CHUNK_IDS);
    for k in 0..400u32 {
        env.reset(&mut t, &mut rng);
        // the bounds
        let n = rng.range(0, 60);
        t.put(grid, i16::try_from(n).unwrap());
        t.fill(grid + 0x115f0, 0x30, &mut rng);
        for i in 0..61u32 {
            let r = grid + 8 + 0x10 * i;
            t.fill(r, 0x10, &mut rng);
            t.put(r + 6, u16::try_from(rng.below(8)).unwrap());
        }
        t.fill(grid + 0xf808, 0x20, &mut rng);
        t.same(
            "AbmapCalcBounds",
            |o| o.eax(0x00a5_ae40, &[]),
            abmap::bridge::abmap_calc_bounds,
        );
        // the fade
        t.put(fade, rng.below(5));
        t.put(fade + 4, rng.range(-1, 13));
        let a = match k % 4 {
            0 => float_in(&mut rng, -40, 40),
            1 => 0u32,
            _ => float_in(&mut rng, -4, 4),
        };
        t.put(fade + 8, a);
        t.put(fade + 0xc, rng.range(-1, 30));
        t.put(fade + 0x10, rng.range(-2, 32));
        t.put(
            ang + 8,
            if rng.below(3) == 0 {
                a
            } else {
                float_in(&mut rng, -40, 40)
            },
        );
        if k % 5 == 2 {
            // counts beyond f32's integers
            let big = rng.range(0x0100_0001, 0x7fff_0000);
            t.put(fade + 0xc, big);
            t.put(fade + 0x10, rng.range(1, big));
        }
        for i in 0..16 {
            t.put(mat + 4 * i, float_in(&mut rng, -4, 4));
        }
        t.put(mngv, if rng.below(3) == 0 { 0 } else { mng });
        t.fill(chunks, 0x38 * 14, &mut rng);
        let work = t.stack_scratch(0x100);
        t.same(
            "Yn_SeaFpFade",
            |o| o.void(0x0092_2970, &[fade]),
            |m| model::bridge::yn_sea_fp_fade(m, fade, work, call),
        );
    }
}

/// Round 31: the Ke library's ray-sphere test and its particle gap
/// filler.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round31_ke_ball_line_and_born_points_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x001f_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    let objv = t.alloc(4);
    // pppCreatePObject(part, base): the word at objv, which then moves on
    // by 0x100 (0 stays 0: the creation fails)
    let mut create = vec![0xa1];
    create.extend_from_slice(&objv.to_le_bytes());
    create.extend_from_slice(&[0x85, 0xc0, 0x74, 0x0a, 0x81, 0x05]);
    create.extend_from_slice(&objv.to_le_bytes());
    create.extend_from_slice(&0x100u32.to_le_bytes());
    create.push(0xc3);
    let list: Vec<(Option<u32>, Vec<u8>)> = vec![(Some(ke::PPP_CREATE_POBJECT), create)];
    t.cave_from(env.cave_end, &list);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let q = t.alloc(0x30);
    let (pc, pd, prog, tab, out, part, prevv) = (
        t.alloc(0x200),
        t.alloc(0x10),
        t.alloc(0x10),
        t.alloc(8),
        t.alloc(4),
        t.alloc(0x40),
        t.alloc(0x10),
    );
    let pool = t.alloc(0x100 * 14);
    t.put(t.b.rebase(ke::PPP_CURRENT_PART), part);
    for k in 0..400u32 {
        env.reset(&mut t, &mut rng);
        // the ray and the sphere
        for i in 0..10 {
            let v = if k % 2 == 0 {
                float_in(&mut rng, -50, 50)
            } else {
                // not dyadic: every f32 store rounds
                #[allow(clippy::cast_precision_loss)]
                let x = rng.next() as f32 / 4.294_967e7 - 50.0;
                x.to_bits()
            };
            t.put(q + 4 * i, v);
        }
        if k % 3 == 0 {
            t.put(q + 0x20, float_in(&mut rng, 0, 400));
        }
        t.same(
            "KeVu0ItrsBallLine",
            |o| o.eax(0x0075_eed0, &[q]),
            |m| ke::bridge::ke_vu0_itrs_ball_line(m, q),
        );
        // the gap filler
        t.fill(pool, 0x100 * 14, &mut rng);
        t.put(objv, if rng.below(8) == 0 { 0 } else { pool });
        t.put(part + 0x3c, if rng.below(10) == 0 { 0u32 } else { 0x40 });
        t.put(pd + 4, u8::try_from(rng.below(12)).unwrap());
        t.put(pd + 8, if k % 10 == 0 { 0u32 } else { rng.below(4) });
        t.put(pd + 0xc, rng.below(4) * 0x10);
        t.put(prog + 0xc, tab);
        t.put(tab, 0x40u32);
        t.put(tab + 4, 0x10u32);
        for i in 0..0x80 {
            t.put(pc + 4 * i, float_in(&mut rng, -100, 100));
        }
        for i in 0..4 {
            t.put(prevv + 4 * i, float_in(&mut rng, -100, 100));
        }
        t.put(pc + 0x40 + 0xa0, if rng.below(3) == 0 { 0 } else { prevv });
        t.put(pc + 0x40 + 0xa8, float_in(&mut rng, -2, 2));
        let work = t.stack_scratch(0x100);
        // the original crashes on a missing object address (it writes
        // through 0); leave that pairing out
        if t.b.read::<u32>(part + 0x3c) == 0 && t.b.read::<u32>(pd + 8) == 0 {
            continue;
        }
        t.same(
            "KeBornPtCmlp",
            |o| o.void(0x0075_9400, &[pc, pd, prog, out]),
            |m| ke::bridge::ke_born_pt_cmlp(m, [pc, pd, prog, out], work, call),
        );
    }
}

/// Round 32: the Ke lens flare's screen frame and fade (`KeLnsLp_Update`),
/// object and world space, with the vector library run as the original.
#[test]
#[allow(clippy::too_many_lines)] // one function after another
fn round32_ke_lens_flare_update_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0020_0001);
    let env = LoadEnv::install(&mut t, &mut rng);
    t.install_trig();
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let fcall = &mut oracle_fcall;
    let (lp, obj, vec, mat) = (t.alloc(0x10), t.alloc(0xb0), t.alloc(0x10), t.alloc(0x40));
    let off = t.b.rebase(ke::VFX_OFFSET);
    t.put(lp, obj);
    t.put(lp + 4, vec);
    t.put(lp + 8, mat);
    let fl = |rng: &mut Xorshift, lo: i32, hi: i32, k: u32| -> u32 {
        if k.is_multiple_of(2) {
            float_in(rng, lo, hi)
        } else {
            #[allow(clippy::cast_precision_loss)]
            let x = (rng.next() as f32 / 4_294_967_296.0) * (hi - lo) as f32 + lo as f32;
            x.to_bits()
        }
    };
    for k in 0..400u32 {
        env.reset(&mut t, &mut rng);
        t.fill(obj, 0xb0, &mut rng);
        for i in 0..16 {
            let v = if i % 5 == 0 {
                fl(&mut rng, -2, 2, k)
            } else {
                fl(&mut rng, -1, 1, k)
            };
            t.put(mat + 4 * i, v);
        }
        if k % 6 == 0 {
            // a translation-free matrix: the object-space d is zero
            for i in 12..15 {
                t.put(mat + 4 * i, 0u32);
            }
        }
        for i in 0..4 {
            t.put(vec + 4 * i, fl(&mut rng, -50, 50, k));
            t.put(off + 4 * i, fl(&mut rng, -500, 500, k));
            t.put(obj + 0x80 + 4 * i, fl(&mut rng, -500, 500, k));
            t.put(obj + 0x90 + 4 * i, fl(&mut rng, -1, 1, k));
        }
        if k % 7 == 3 {
            // the flare at the offset: world-space d is zero
            for i in 0..3 {
                t.put(obj + 0x80 + 4 * i, 0u32);
            }
        }
        t.put(obj + 0x8f, u8::from(rng.below(2) == 0));
        t.put(obj + 0x8e, u8::try_from(rng.below(40)).unwrap());
        let w9c = match rng.below(4) {
            0 => 0u32,
            1 => fl(&mut rng, -2, 0, k),
            _ => fl(&mut rng, 0, 3, k),
        };
        t.put(obj + 0x9c, w9c);
        let a8 = match rng.below(5) {
            0 => fl(&mut rng, -10, 0, k),
            1 => 0u32,
            _ => fl(&mut rng, 0, 900, k),
        };
        t.put(obj + 0xa8, a8);
        let work = t.stack_scratch(0x100);
        t.same(
            "KeLnsLp_Update",
            |o| o.void(0x0076_a690, &[lp]),
            |m| ke::bridge::ke_lns_lp_update(m, lp, work, call, &mut *fcall),
        );
    }
}

// ---- round 33 ----

/// A [`log_stub_from`] of all `args` in `1 << shift`-byte entries, its
/// return replaced by `tail`.
fn log_tail(log: u32, id: u32, args: u32, shift: u8, tail: &[u8]) -> Vec<u8> {
    let mut c = log_stub_from(log, 0, id, 0, args);
    c[8] = shift;
    c.truncate(c.len() - 6);
    c.extend_from_slice(tail);
    c
}

/// Returns `-n` (non-zero) while the counter `n` at `cnt` is above 0,
/// counting it down; then 0.
fn retry_tail(cnt: u32) -> Vec<u8> {
    let mut c = vec![0xa1];
    c.extend_from_slice(&cnt.to_le_bytes());
    c.extend_from_slice(&[0x85, 0xc0, 0x74, 0x08, 0xff, 0x0d]);
    c.extend_from_slice(&cnt.to_le_bytes());
    c.extend_from_slice(&[0xf7, 0xd8, 0xc3]);
    c
}

/// Returns the counter at `cnt`, counting it down (n, n - 1, .., 0, -1).
fn down_tail(cnt: u32) -> Vec<u8> {
    let mut c = vec![0xa1];
    c.extend_from_slice(&cnt.to_le_bytes());
    c.extend_from_slice(&[0xff, 0x0d]);
    c.extend_from_slice(&cnt.to_le_bytes());
    c.push(0xc3);
    c
}

/// Round 33's stand-ins: the SIF calls (logged, with retry counters),
/// the FMOD glue (logged, returning `rv`) and the CRT's `__iob_func` /
/// `fprintf` (through their import slots).
struct SqsEnv {
    log: u32,
    log9: u32,
    rv: u32,
    check_cnt: u32,
    call_cnt: u32,
    dma_cnt: u32,
    /// Where the next stand-in may go in [`CAVE`].
    cave_end: u32,
}

impl SqsEnv {
    const GLUE: [(u32, u32); 13] = [
        (nkz::I_SFX_FORCE_START, 3),
        (nkz::I_SFX_IS_PLAYING, 1),
        (nkz::I_SFX_LOAD_DATA, 2),
        (nkz::I_SFX_PLAY_BATTLE_STREAMING, 3),
        (nkz::I_SFX_SET_PAN, 2),
        (nkz::I_SFX_SET_PAN_TRANSLATE, 3),
        (nkz::I_SFX_SET_VOLUME, 2),
        (nkz::I_SFX_SET_VOLUME_TRANSLATE, 3),
        (nkz::I_SFX_START, 3),
        (nkz::I_SFX_STOP, 1),
        (nkz::I_SFX_STOP_ALL_NON_BLOCKING_SOUND, 1),
        (nkz::I_SFX_STOP_NON_BLOCKING_SOUND, 1),
        (nkz::I_SFX_UNLOAD_ALL_DATA, 0),
    ];

    fn install(t: &mut Twin) -> Self {
        let (log, log9) = (t.alloc(4 + 0x20 * 64), t.alloc(4 + 0x40 * 32));
        let (rv, file) = (t.alloc(4), t.alloc(0x80));
        let (check_cnt, call_cnt, dma_cnt) = (t.alloc(4), t.alloc(4), t.alloc(4));
        let mut list: Vec<(Option<u32>, Vec<u8>)> = vec![
            (
                Some(nkz::SIF_CHECK_STAT_RPC),
                log_tail(log, 1, 1, 5, &retry_tail(check_cnt)),
            ),
            (
                Some(nkz::SIF_CALL_RPC),
                log_tail(log9, 2, 9, 6, &retry_tail(call_cnt)),
            ),
            (
                Some(nkz::SIF_DMA_STAT),
                log_tail(log, 3, 1, 5, &down_tail(dma_cnt)),
            ),
        ];
        for (id, (f, n)) in (0x10u32..).zip(Self::GLUE) {
            list.push((Some(f), log_stub(log, rv, id, n)));
        }
        // __iob_func: the fake FILE array; fprintf(stream, format) logged
        let mut iob = vec![0xb8];
        iob.extend_from_slice(&file.to_le_bytes());
        iob.push(0xc3);
        list.push((None, iob));
        list.push((None, log_stub(log, rv, 0x30, 2)));
        let (at, cave_end) = t.cave_from(0, &list);
        let n = at.len();
        let iob_slot = t.b.rebase(yi::CRT_IOB_FUNC);
        let fprintf_slot = t.b.rebase(yi::CRT_FPRINTF);
        t.put(iob_slot, at[n - 2]);
        t.put(fprintf_slot, at[n - 1]);
        Self {
            log,
            log9,
            rv,
            check_cnt,
            call_cnt,
            dma_cnt,
            cave_end,
        }
    }

    fn reset(&self, t: &mut Twin, rng: &mut Xorshift) {
        t.put(self.log, 0u32);
        t.put(self.log9, 0u32);
        t.put(self.rv, rng.next());
        t.put(self.check_cnt, rng.below(3));
        t.put(self.call_cnt, rng.below(3));
        t.put(self.dma_cnt, rng.below(3));
    }
}

/// Round 33 on `t`: the Sqs IOP calls and SE wrappers (stand-ins from
/// [`SqsEnv`]), `SqsCmdSetC4C6`, the ADPCM decoder and `Sg_MemSetFree`.
#[allow(clippy::too_many_lines)] // one function after another
fn round33_cases(t: &mut Twin, rng: &mut Xorshift, rounds: u32) {
    let env = SqsEnv::install(t);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let g = |t: &Twin, a: u32| t.b.rebase(a);
    let (send, recv, send2, recv2) = (t.alloc(0x40), t.alloc(0x400), t.alloc(0x40), t.alloc(0x40));
    for k in 0..rounds {
        env.reset(t, rng);
        let cmd = rng.next();
        t.set(nkz::SQS_RPC_SEND, send);
        t.set(nkz::SQS_RPC_RECV, recv);
        t.set(nkz::SQS_RPC_SEND2, send2);
        t.set(nkz::SQS_RPC_RECV2, recv2);
        t.same(
            "Sqs_CallRpc",
            |o| o.eax(0x0081_cbd0, &[cmd]),
            |m| nkz::bridge::sqs_call_rpc(m, cmd, call),
        );
        env.reset(t, rng);
        let (end, arg) = (rng.next(), rng.next());
        t.same(
            "Sqs_CallRpc2",
            |o| o.eax(0x0081_cc20, &[cmd, 7, end, arg]),
            |m| nkz::bridge::sqs_call_rpc2(m, cmd, end, arg, call),
        );
        // the command queue and the reply histories
        env.reset(t, rng);
        let top = rng.next();
        t.set(nkz::SQS_CMD_TOP, top);
        t.set(nkz::SQS_CMD_NEXT, top.wrapping_add(rng.below(0x400)));
        let h = g(t, nkz::SQS_REPLY_HISTORY);
        t.fill(h, 0x40, rng);
        let r = g(t, nkz::SQS_REPLY);
        t.fill(r, 0x40, rng);
        t.set(nkz::SQS_IOP_RESULT, rng.next());
        t.same(
            "SqsIopCommandSend",
            |o| o.eax(0x0081_d5f0, &[]),
            |m| nkz::bridge::sqs_iop_command_send(m, call),
        );
        // the DMA wait: with `wait`, the counter runs out (to -1)
        env.reset(t, rng);
        let wait = u32::from(rng.below(2) == 0) * rng.next();
        t.set(nkz::SIF_DMA_ID, rng.next());
        t.same(
            "SqsSifDmaState",
            |o| o.eax(0x0081_eef0, &[rng.next(), wait]),
            |m| nkz::bridge::sqs_sif_dma_state(m, wait, call),
        );
        // SE starts: the handle (wrapping past 0) and the started bit
        let (id, pan, vol) = (rng.next(), rng.next(), rng.next());
        for (name, f, start) in [
            ("SqsSePlay", 0x0081_e4e0, nkz::I_SFX_START),
            ("SqsSeForcePlay", 0x0081_e410, nkz::I_SFX_FORCE_START),
        ] {
            env.reset(t, rng);
            let h = match rng.below(3) {
                0 => u32::MAX,
                1 => 0,
                _ => rng.next(),
            };
            t.set(nkz::SQS_SE_HANDLE, h);
            t.set(nkz::SQS_SE_STARTED, rng.next());
            t.same(
                name,
                |o| o.eax(f, &[9, id, pan, vol]),
                |m| nkz::bridge::sqs_se_play(m, start, [id, pan, vol], call),
            );
        }
        env.reset(t, rng);
        t.same(
            "SqsSeStop",
            |o| o.eax(0x0081_e7c0, &[9, id]),
            |m| nkz::bridge::sqs_se_stop(m, id, call),
        );
        for (name, f, glue) in [
            ("SqsSePanDirect", 0x0081_e460, nkz::I_SFX_SET_PAN),
            ("SqsSeVolumeDirect", 0x0081_e820, nkz::I_SFX_SET_VOLUME),
        ] {
            env.reset(t, rng);
            t.same(
                name,
                |o| o.eax(f, &[9, id, pan]),
                |m| nkz::bridge::sqs_se_direct(m, glue, id, pan, call),
            );
        }
        for (name, f, glue) in [
            (
                "SqsSePanTranslate",
                0x0081_e480,
                nkz::I_SFX_SET_PAN_TRANSLATE,
            ),
            (
                "SqsSeVolumeTranslate",
                0x0081_e860,
                nkz::I_SFX_SET_VOLUME_TRANSLATE,
            ),
        ] {
            env.reset(t, rng);
            t.same(
                name,
                |o| o.eax(f, &[9, id, pan, vol]),
                |m| nkz::bridge::sqs_se_translate(m, glue, [id, pan, vol], call),
            );
        }
        env.reset(t, rng);
        t.same(
            "SqsSeVolumeFadeOut",
            |o| o.eax(0x0081_e840, &[9, id, pan]),
            |m| nkz::bridge::sqs_se_volume_fade_out(m, id, pan, call),
        );
        for (name, f) in [
            ("SqsSeBlockPlaying", 0x0081_e3e0),
            ("SqsSeSeparatePlaying", 0x0081_e7a0),
        ] {
            env.reset(t, rng);
            t.same(
                name,
                |o| o.eax(f, &[id]),
                |m| nkz::bridge::sqs_se_playing(m, id, call),
            );
        }
        env.reset(t, rng);
        let sid = if k % 2 == 0 { u32::MAX } else { id };
        t.fill(send, 0x40, rng);
        t.same(
            "SqsSeSeparateClear",
            |o| o.eax(0x0081_e5d0, &[sid]),
            |m| nkz::bridge::sqs_se_separate_clear(m, sid, call),
        );
        env.reset(t, rng);
        let mask: [u32; 8] = std::array::from_fn(|_| rng.next());
        t.same(
            "SqsSeSeparateMaskClear",
            |o| o.eax(0x0081_e750, &mask),
            |m| nkz::bridge::sqs_se_separate_mask_clear(m, mask, call),
        );
        // a wave file ("WD") or not
        env.reset(t, rng);
        let wave = t.alloc(8);
        let tag = if k % 3 == 0 { rng.next() } else { 0x4457 };
        t.put(wave, tag);
        t.put(wave + 2, u16::try_from(rng.below(0x1_0000)).unwrap());
        t.same(
            "SqsBattleWaveDataSend",
            |o| o.eax(0x0081_cf90, &[wave, rng.next()]),
            |m| nkz::bridge::sqs_battle_wave_data_send(m, wave, call),
        );
        // the battle stream: no magic file, no stream id, or both
        env.reset(t, rng);
        let magic = g(t, 0x00c6_4ca0);
        t.put(magic, if k % 3 == 0 { u32::MAX } else { rng.below(100) });
        t.set(
            nkz::BATTLE_STREAM,
            if k % 3 == 1 { u32::MAX } else { rng.next() },
        );
        let (a, b) = (rng.next(), rng.next());
        t.same(
            "SqsBattleStreamingStart",
            |o| o.eax(0x0081_cf40, &[9, a, b]),
            |m| nkz::bridge::sqs_battle_streaming_start(m, a, b, call),
        );
        // the IOP command handler
        let (ctx, rec, c) = (t.alloc(0x220 + 4), t.alloc(0xc8), t.alloc(4));
        t.fill(ctx, 0x220, rng);
        t.put(ctx + 0x220, rec);
        t.fill(c, 4, rng);
        t.same(
            "SqsCmdSetC4C6",
            |o| o.eax(0x0081_c260, &[ctx, c]),
            |m| nkz::bridge::sqs_cmd_set_c4_c6(m, ctx, c),
        );
        // Sg_MemSetFree: a header, its heap's lowest freed block
        let block = t.alloc(0x10);
        t.fill(block, 0x10, rng);
        let heaps = g(t, yi::MEM_HEAPS);
        t.fill(heaps, 0x60, rng);
        if k % 2 == 0 {
            let w = t.b.read::<u32>(block + 0xc);
            let heap = (w >> 27) & 3;
            let low = block.wrapping_add(rng.below(0x40)).wrapping_sub(0x20);
            t.put(heaps + 0x18 * heap + 0x14, low);
        }
        t.same(
            "Sg_MemSetFree",
            |o| o.eax(0x0083_bdb0, &[block]),
            |m| yi::bridge::sg_mem_set_free(m, block),
        );
    }
    // the ADPCM decoder: random data and history, the game's filters or
    // random ones (the products wrap)
    let (src, dst) = (t.alloc(7200), t.alloc(0x6400));
    let coefs = g(t, nkz::SQS_ADPCM_COEFS);
    let state = g(t, nkz::SQS_ADPCM_STATE);
    let game: Vec<u8> = (0..0x80).map(|i| t.b.read::<u8>(coefs + i)).collect();
    for k in 0..rounds.div_ceil(4) {
        t.fill(src, 7200, rng);
        t.fill(state, 0x10, rng);
        if k % 2 == 0 {
            t.write(coefs, &game);
        } else {
            t.fill(coefs, 0x80, rng);
        }
        t.same(
            "Sqs_DecodeAdpcm",
            |o| o.eax(0x0081_cc70, &[src, dst]),
            |m| nkz::bridge::sqs_decode_adpcm(m, src, dst),
        );
    }
}

/// Round 33: the Sqs IOP calls and SE wrappers, the IOP command handler
/// `SqsCmdSetC4C6`, `Sqs_DecodeAdpcm` and `Sg_MemSetFree`, synthetic.
#[test]
fn round33_sqs_adpcm_and_sg_mem_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0021_0001);
    round33_cases(&mut t, &mut rng, 200);
}

/// Round 33 on the snapshot (`FFX_SNAPSHOT`): the game's own Sqs state,
/// filters and heaps under the same cases.
#[test]
fn round33_on_a_snapshot_matches_the_original() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x0021_0002);
    round33_cases(&mut t, &mut rng, 40);
}

// ---- round 34 ----

/// Round 34 on `t`: `yiPolyBufUnpack`, every size and count, `TOPS` on or
/// off, random rows.
fn round34_cases(t: &mut Twin, rng: &mut Xorshift, rounds: u32) {
    let (buf, src, code) = (t.alloc(0x1_0000), t.alloc(0x1000), t.alloc(4));
    for k in 0..rounds {
        t.fill(src, 0x1000, rng);
        t.put(buf + 0x4070, rng.below(0x400));
        let op = (k % 16) | (rng.below(8) << 4);
        let num = match rng.below(4) {
            0 => 0,
            1 => 1,
            _ => rng.below(256),
        };
        let w = (op << 24)
            | (num << 16)
            | (rng.below(2) << 15)
            | (rng.next() & 0x7c00)
            | rng.below(0x400);
        t.put(code, w);
        t.same(
            "yiPolyBufUnpack",
            |o| o.eax(0x0091_1620, &[buf, src, code]),
            |m| yi::bridge::yi_poly_buf_unpack(m, buf, src, code),
        );
    }
}

/// A random VIF code stream of about `n` words: every command the poly
/// buffer knows (and some it ignores), small skips; data words keep a zero
/// command byte, so a stream read past an `UNPACK`'s data decodes as `NOP`s.
fn vif_stream(rng: &mut Xorshift, n: usize, end: bool) -> Vec<u32> {
    const CMDS: [u32; 17] = [
        0, 1, 2, 3, 4, 5, 6, 7, 0x10, 0x20, 0x30, 0x31, 0x4a, 0x50, 0x51, 0x60, 0x6f,
    ];
    let data = |rng: &mut Xorshift| rng.next() & 0x00ff_ffff;
    let mut v = vec![];
    while v.len() < n {
        let cmd = CMDS[rng.below(17) as usize];
        let imm = match cmd {
            0x4a => rng.below(4) << 16,
            0x50 | 0x51 => rng.below(4),
            0x60..=0x7f => (rng.below(16) << 16) | (rng.next() & 0x83ff),
            _ => rng.next() & 0x00ff_ffff,
        };
        let cmd = if cmd >= 0x60 {
            cmd | rng.below(0x20)
        } else {
            cmd
        };
        v.push((rng.below(2) << 31) | (cmd << 24) | imm);
        let extra = match cmd {
            0x20 => 1,
            0x30 | 0x31 => 4,
            0x4a => ((imm >> 16) & 0xff) * 2,
            0x50 | 0x51 => imm * 4,
            0x60..=0x7f => ((imm >> 16) & 0xff) * 4,
            _ => 0,
        };
        for _ in 0..extra {
            v.push(data(rng));
        }
    }
    if end {
        v.push([0x14u32, 0x15, 0x17][rng.below(3) as usize] << 24);
    }
    v
}

/// Round 35 on `t`: `yiPolyBufExecVif` over random streams, stopping by
/// count or at a microprogram call, from either double-buffer state.
fn round35_cases(t: &mut Twin, rng: &mut Xorshift, rounds: u32) {
    let (buf, codes) = (t.alloc(0x1_0000), t.alloc(0x4000));
    for k in 0..rounds {
        let n = 1 + rng.below(0x200) as usize;
        let v = vif_stream(rng, n, k % 3 == 0);
        let mut bytes = vec![0u8; 0x4000];
        for (i, w) in v.iter().enumerate() {
            bytes[4 * i..4 * i + 4].copy_from_slice(&w.to_le_bytes());
        }
        t.write(codes, &bytes);
        t.fill(buf + 0x4000, 0x88, rng);
        t.put(buf + 0x4070, rng.below(0x400));
        t.put(buf + 0x4074, rng.below(0x400));
        t.put(buf + 0x4078, rng.below(0x400));
        let len = u32::try_from(v.len()).unwrap();
        let words = match rng.below(4) {
            0 => 0,
            1 => 1 + rng.below(len),
            _ => len,
        };
        t.same(
            "yiPolyBufExecVif",
            |o| o.eax(0x0091_13a0, &[buf, codes, words]),
            |m| yi::bridge::yi_poly_buf_exec_vif(m, buf, codes, words),
        );
    }
}

/// Round 35: the poly buffer's VIF interpreter (`yiPolyBufExecVif`).
#[test]
fn round35_poly_buf_exec_vif_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0023_0001);
    round35_cases(&mut t, &mut rng, 300);
}

/// Round 35 on the snapshot (`FFX_SNAPSHOT`).
#[test]
fn round35_on_a_snapshot_matches_the_original() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x0023_0002);
    round35_cases(&mut t, &mut rng, 60);
}

/// Round 34: the poly buffer's VIF `UNPACK` (`yiPolyBufUnpack`), synthetic.
#[test]
fn round34_poly_buf_unpack_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0022_0001);
    round34_cases(&mut t, &mut rng, 400);
}

/// Round 34 on the snapshot (`FFX_SNAPSHOT`).
#[test]
fn round34_on_a_snapshot_matches_the_original() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x0022_0002);
    round34_cases(&mut t, &mut rng, 100);
}

// ---- round 36 ----

/// Round 36 on `t`: the standby requests, `SqsDataSendingCompleted`, the
/// info file paths and `SqsSetProjectBase` (its pointer result through a
/// second `FFXSifCallRpc` stand-in).
#[allow(clippy::too_many_lines)] // one function after another
fn round36_cases(t: &mut Twin, rng: &mut Xorshift, rounds: u32) {
    let env = SqsEnv::install(t);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (send, recv, send2, recv2) = (t.alloc(0x40), t.alloc(0x400), t.alloc(0x40), t.alloc(0x40));
    let path = t.alloc(0x40);
    let state = |t: &mut Twin, rng: &mut Xorshift| {
        t.set(nkz::SQS_RPC_SEND, send);
        t.set(nkz::SQS_RPC_RECV, recv);
        t.set(nkz::SQS_RPC_SEND2, send2);
        t.set(nkz::SQS_RPC_RECV2, recv2);
        let count = rng.next();
        t.set(nkz::SQS_STANDBY_SENT, count);
        t.set(
            nkz::SQS_IOP_RESULT,
            if rng.below(2) == 0 { count } else { rng.next() },
        );
        let top = rng.next();
        t.set(nkz::SQS_CMD_TOP, top);
        let next = match rng.below(3) {
            0 => top, // an empty queue is not sent
            _ => top.wrapping_add(rng.below(0x100)).wrapping_sub(0x40),
        };
        t.set(nkz::SQS_CMD_NEXT, next);
        t.set(nkz::SQS_REPLY_PENDING, rng.below(2));
        t.set(nkz::SQS_RPC_FLAGS, rng.next());
        t.fill(recv2, 0x40, rng);
        let h = t.b.rebase(nkz::SQS_REPLY_HISTORY);
        t.fill(h, 0x40, rng);
    };
    for _ in 0..rounds {
        env.reset(t, rng);
        state(t, rng);
        t.same(
            "SqsReportStandby",
            |o| o.eax(0x0081_e260, &[]),
            |m| nkz::bridge::sqs_report_standby(m, call),
        );
        env.reset(t, rng);
        state(t, rng);
        let flush = rng.below(2) * rng.next();
        t.same(
            "SqsReportStandbyStatus",
            |o| o.eax(0x0081_e2d0, &[flush]),
            |m| nkz::bridge::sqs_report_standby_status(m, flush, call),
        );
        env.reset(t, rng);
        let wait = rng.below(2) * rng.next();
        t.set(nkz::SIF_DMA_ID, rng.next());
        t.same(
            "SqsDataSendingCompleted",
            |o| o.eax(0x0081_cfd0, &[wait]),
            |m| nkz::bridge::sqs_data_sending_completed(m, wait, call),
        );
        // a path of 0..62 non-NUL bytes, then the NUL
        let n = rng.below(63);
        let mut p: Vec<u8> = (0..n)
            .map(|_| 1 + u8::try_from(rng.below(255)).unwrap())
            .collect();
        p.push(0);
        t.write(path, &p);
        t.fill(send, 0x40, rng);
        for (name, f, cmd) in [
            ("SqsInitMusicInfoHdd", 0x0081_d3c0, 7),
            ("SqsInitWaveInfoHdd", 0x0081_d470, 6),
        ] {
            env.reset(t, rng);
            t.same(
                name,
                |o| o.eax(f, &[path]),
                |m| nkz::bridge::sqs_init_info_hdd(m, path, cmd, call),
            );
        }
        env.reset(t, rng);
        t.same(
            "SqsSetProjectBase",
            |o| o.eax(0x0081_eab0, &[path]),
            |m| nkz::bridge::sqs_set_project_base(m, path, call),
        );
    }
    // FFXSifCallRpc giving a pointer (0 now and then): the word it reads
    let (rv, word) = (t.alloc(4), t.alloc(4));
    let stub = log_stub(env.log9, rv, 0x40, 2);
    t.cave_from(env.cave_end, &[(Some(nkz::SIF_CALL_RPC), stub)]);
    for k in 0..rounds {
        env.reset(t, rng);
        t.put(rv, if k % 4 == 0 { 0 } else { word });
        t.put(word, rng.next());
        t.same(
            "SqsSetProjectBase (pointer)",
            |o| o.eax(0x0081_eab0, &[path]),
            |m| nkz::bridge::sqs_set_project_base(m, path, call),
        );
    }
}

/// Round 36: the Sqs standby requests, data sends and info paths.
#[test]
fn round36_sqs_standby_and_paths_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0024_0001);
    round36_cases(&mut t, &mut rng, 200);
}

/// Round 36 on the snapshot (`FFX_SNAPSHOT`).
#[test]
fn round36_on_a_snapshot_matches_the_original() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x0024_0002);
    round36_cases(&mut t, &mut rng, 40);
}

// ---- round 38 ----

/// Round 38 on `t`: the two VU0 subtractions (`rcVu0Sub` and its twin),
/// `a` apart from or the same as `b` or `c`.
fn round38_cases(t: &mut Twin, rng: &mut Xorshift, rounds: u32) {
    let (a, b, c) = (t.alloc(0x10), t.alloc(0x10), t.alloc(0x10));
    let vf00 = t.b.rebase(vu0::VU0_VF00);
    for k in 0..rounds {
        for v in [a, b, c] {
            for i in 0..4 {
                let x = if k % 2 == 0 {
                    rng.float()
                } else {
                    rng.any_float()
                };
                t.put(v + 4 * i, x);
            }
        }
        t.fill(vf00, 0x10, rng);
        let dst = [a, a, b, c][rng.below(4) as usize];
        for f in [0x0091_f740, 0x0092_d720] {
            t.same(
                "rcVu0Sub",
                |o| o.eax(f, &[dst, b, c]),
                |m| vu0::bridge::rc_vu0_sub(m, dst, b, c),
            );
        }
    }
}

/// Round 38: `rcVu0Sub` (`0x0091f740`) and `rcVu0Sub_0092d720`.
#[test]
fn round38_vu0_sub_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0026_0001);
    round38_cases(&mut t, &mut rng, 400);
}

/// Round 38 on the snapshot (`FFX_SNAPSHOT`).
#[test]
fn round38_on_a_snapshot_matches_the_original() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x0026_0002);
    round38_cases(&mut t, &mut rng, 100);
}

// ---- round 39 ----

/// Round 39 on `t`: the IOP command builders over the queue at
/// `0x012f4a00..0x012f4e00`, empty to full (a full queue is sent first,
/// with the `dbgPrintf` on or off), and `SqsInitWaveInfo`.
#[allow(clippy::too_many_lines)] // one function after another
fn round39_cases(t: &mut Twin, rng: &mut Xorshift, rounds: u32) {
    let env = SqsEnv::install(t);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (send, recv) = (t.alloc(0x40), t.alloc(0x400));
    let top = t.b.rebase(nkz::SQS_CMD_BUF);
    let queue = |t: &mut Twin, rng: &mut Xorshift| {
        env.reset(t, rng);
        t.set(nkz::SQS_RPC_SEND, send);
        t.set(nkz::SQS_RPC_RECV, recv);
        t.set(nkz::SQS_CMD_TOP, top);
        let n = match rng.below(3) {
            0 => 0x1f, // the last slot: sent first
            1 => 0x20, // past it
            _ => rng.below(0x20),
        };
        t.set(nkz::SQS_CMD_NEXT, top + 0x20 * n);
        t.fill(top, 0x400, rng);
        t.set(nkz::SQS_DEBUG_FLAGS, rng.next());
        let h = t.b.rebase(nkz::SQS_REPLY_HISTORY);
        t.fill(h, 0x40, rng);
    };
    for _ in 0..rounds {
        let a = rng.next();
        for (name, f, op) in [
            ("SqsSetCycleMode", 0x0081_e900, 4),
            ("SqsSetReverbPlaySwitch", 0x0081_ec20, 0xc),
            ("SqsSetStraightStreamAreaSize", 0x0081_ef50, 5),
        ] {
            queue(t, rng);
            t.same(
                name,
                |o| o.eax(f, &[a]),
                |m| nkz::bridge::sqs_cmd_arg(m, op, a, call),
            );
        }
        queue(t, rng);
        t.same(
            "SqsAllVoiceTerminate",
            |o| o.eax(0x0081_ce70, &[]),
            |m| nkz::bridge::sqs_all_voice_terminate(m, call),
        );
        queue(t, rng);
        t.same(
            "SqsMusicDataRead",
            |o| o.eax(0x0081_d820, &[a]),
            |m| nkz::bridge::sqs_music_data_read(m, a, call),
        );
        queue(t, rng);
        let mode = [0, 1, 2, 3, 4, u32::MAX, a][rng.below(7) as usize];
        t.same(
            "SqsSetPlayMode",
            |o| o.eax(0x0081_ea00, &[mode]),
            |m| nkz::bridge::sqs_set_play_mode(m, mode, call),
        );
        queue(t, rng);
        t.same(
            "SqsShoutStreamingStart",
            |o| o.eax(0x0081_ee20, &[a]),
            |m| nkz::bridge::sqs_shout_streaming_start(m, a, call),
        );
        queue(t, rng);
        t.set(nkz::SQS_STRAIGHT_STREAM, rng.next());
        t.same(
            "SqsStraightStreamingStart",
            |o| o.eax(0x0081_f6c0, &[]),
            |m| nkz::bridge::sqs_straight_streaming_start(m, call),
        );
        queue(t, rng);
        t.same(
            "SqsStraightStreamingStop",
            |o| o.eax(0x0081_f770, &[]),
            |m| nkz::bridge::sqs_straight_streaming_stop(m, call),
        );
        let v = rng.next();
        queue(t, rng);
        t.same(
            "SqsStraightStreamingVolumeDirect",
            |o| o.eax(0x0081_f800, &[v]),
            |m| nkz::bridge::sqs_straight_streaming_volume_direct(m, v, call),
        );
        queue(t, rng);
        t.same(
            "SqsStraightStreamingVolumeTranslate",
            |o| o.eax(0x0081_f880, &[a, v]),
            |m| nkz::bridge::sqs_straight_streaming_volume_translate(m, a, v, call),
        );
        // the wave info: done already, without or with a source
        queue(t, rng);
        t.set(nkz::SQS_WAVE_INFO_DONE, u32::from(rng.below(4) == 0));
        t.set(nkz::SQS_WAVE_INFO_DST, rng.next());
        t.set(nkz::SIF_DMA_ID, rng.next());
        let dma = t.b.rebase(nkz::SIF_DMA);
        t.fill(dma, 0x20, rng);
        let src = rng.below(2) * rng.next();
        t.same(
            "SqsInitWaveInfo",
            |o| o.void(0x0081_d3f0, &[src, 7, a]),
            |m| nkz::bridge::sqs_init_wave_info(m, src, a, call),
        );
    }
}

/// Round 39: the Sqs IOP command builders.
#[test]
fn round39_sqs_command_builders_match_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0027_0001);
    round39_cases(&mut t, &mut rng, 150);
}

/// Round 39 on the snapshot (`FFX_SNAPSHOT`).
#[test]
fn round39_on_a_snapshot_matches_the_original() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x0027_0002);
    round39_cases(&mut t, &mut rng, 30);
}

// ---- round 40 ----

/// Round 40 on `t`: `SqsSeSeparateDataSend` over chunk streams: repeated
/// ids, a bad chunk, a size cut short, sizes out of range, and more than
/// 256 distinct ids (the overflow message, `fflush` logged).
fn round40_cases(t: &mut Twin, rng: &mut Xorshift, rounds: u32) {
    let env = SqsEnv::install(t);
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (send, recv) = (t.alloc(0x40), t.alloc(0x400));
    let (stubs, _) = t.cave_from(env.cave_end, &[(None, log_stub(env.log, env.rv, 0x31, 1))]);
    let slot = t.b.rebase(nkz::CRT_FFLUSH);
    t.put(slot, stubs[0]);
    let data = t.alloc(0x1_0000);
    let table = t.b.rebase(nkz::SQS_SEP_WAVES);
    for k in 0..rounds {
        env.reset(t, rng);
        t.set(nkz::SQS_RPC_SEND, send);
        t.set(nkz::SQS_RPC_RECV, recv);
        t.set(nkz::SQS_CMD_TOP, rng.next());
        t.set(nkz::SQS_CMD_NEXT, rng.next());
        t.set(nkz::SQS_DEBUG_FLAGS, rng.next());
        t.fill(table, 0x400, rng);
        let many = k % 10 == 9;
        let chunks = if many { 300 } else { rng.below(40) };
        let pool = if many { 0x1_0000 } else { 1 + rng.below(12) };
        let mut bytes = vec![];
        for c in 0..chunks {
            let len = if many { 0x14 } else { 0x14 + 4 * rng.below(8) };
            let mut ch: Vec<u8> = (0..len)
                .map(|_| u8::try_from(rng.below(256)).unwrap())
                .collect();
            // a bad chunk ends the walk: its magic, tag or length
            let bad = if many || rng.below(20) != 0 {
                0
            } else {
                1 + rng.below(5)
            };
            let magic: u32 = if bad == 1 { 0x6553_6554 } else { 0x6553_6553 };
            let tag: u32 = if bad == 2 { 0x2020_2071 } else { 0x2020_2070 };
            let word = match bad {
                3 => 0,
                4 => len + 2,
                5 => 0x8000_0000 | len,
                _ => len,
            };
            ch[..4].copy_from_slice(&magic.to_le_bytes());
            ch[4..8].copy_from_slice(&tag.to_le_bytes());
            ch[0xc..0x10].copy_from_slice(&word.to_le_bytes());
            let id = if many { c } else { rng.below(pool) };
            ch[0x11..0x13].copy_from_slice(&u16::try_from(id & 0xffff).unwrap().to_le_bytes());
            bytes.extend_from_slice(&ch);
        }
        bytes.resize(bytes.len() + 0x20, 0);
        t.write(data, &bytes);
        let total = u32::try_from(bytes.len() - 0x20).unwrap();
        let size = match rng.below(7) {
            0 => 0,
            1 => 0x1_0001 + rng.below(4),
            6 => 0x1_0000, // the largest size taken
            2 => rng.below(total + 1).max(1),
            _ => total.max(1),
        };
        t.same(
            "SqsSeSeparateDataSend",
            |o| o.eax(0x0081_e600, &[data, size]),
            |m| nkz::bridge::sqs_se_separate_data_send(m, data, size, call),
        );
    }
}

/// Round 40: `SqsSeSeparateDataSend`.
#[test]
fn round40_sqs_se_separate_data_send_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0028_0001);
    round40_cases(&mut t, &mut rng, 200);
}

/// Round 40 on the snapshot (`FFX_SNAPSHOT`).
#[test]
fn round40_on_a_snapshot_matches_the_original() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x0028_0002);
    round40_cases(&mut t, &mut rng, 40);
}

// ---- round 41 ----

/// A `rcPrimDraw2D` stand-in: the eight vertex words its second argument
/// points at to `log`, its other arguments after them, a call count at
/// `log + 0x2c`; returns 0x1234.
fn prim_draw_stub(log: u32) -> Vec<u8> {
    let mut c = vec![0x8b, 0x4c, 0x24, 0x08]; // mov ecx, [esp + 8]
    for i in 0..8u8 {
        c.extend_from_slice(&[0x8b, 0x41, 4 * i, 0xa3]); // mov eax, [ecx + 4i]; mov [log + 4i], eax
        c.extend_from_slice(&(log + 4 * u32::from(i)).to_le_bytes());
    }
    for (arg, at) in [(4u8, 0x20u32), (0xc, 0x24), (0x10, 0x28)] {
        c.extend_from_slice(&[0x8b, 0x44, 0x24, arg, 0xa3]); // mov eax, [esp + arg]; mov [log + at], eax
        c.extend_from_slice(&(log + at).to_le_bytes());
    }
    c.extend_from_slice(&[0xff, 0x05]); // inc dword [log + 0x2c]
    c.extend_from_slice(&(log + 0x2c).to_le_bytes());
    c.push(0xb8); // mov eax, 0x1234; ret
    c.extend_from_slice(&0x1234u32.to_le_bytes());
    c.push(0xc3);
    c
}

/// Round 41 on `t`: `rcPrimLine` with the UI type on or off, its vertices
/// and arguments logged by the `rcPrimDraw2D` stand-in, the UI type
/// calls by another.
fn round41_cases(t: &mut Twin, rng: &mut Xorshift, rounds: u32) {
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    let (log, log2, rv) = (t.alloc(0x30), t.alloc(4 + 0x20 * 4), t.alloc(4));
    // cgraphicSetNowUIType logged (the image has no UI state behind it)
    t.cave(&[
        (Some(rcui::bridge::RC_PRIM_DRAW_2D), prim_draw_stub(log)),
        (
            Some(rcui::bridge::SET_NOW_UI_TYPE),
            log_stub(log2, rv, 1, 1),
        ),
    ]);
    for _ in 0..rounds {
        let a: [u32; 6] = [
            rng.any_float(),
            rng.any_float(),
            rng.any_float(),
            rng.any_float(),
            rng.next(),
            rng.below(2) * rng.next(),
        ];
        t.fill(log, 0x2c, rng);
        t.put(log2, 0u32);
        t.put(rv, rng.next());
        let work = t.stack_scratch(0x100);
        t.same(
            "rcPrimLine",
            |o| o.eax(0x0092_6c00, &a),
            |m| rcui::bridge::rc_prim_line(m, a, work, call),
        );
    }
}

/// Round 41: `rcPrimLine`.
#[test]
fn round41_prim_line_matches_the_original() {
    let Some(mut t) = Twin::image() else { return };
    let mut rng = Xorshift(0x0029_0001);
    round41_cases(&mut t, &mut rng, 300);
}

/// Round 41 on the snapshot (`FFX_SNAPSHOT`).
#[test]
fn round41_on_a_snapshot_matches_the_original() {
    let Some(mut t) = Twin::snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x0029_0002);
    round41_cases(&mut t, &mut rng, 60);
}
