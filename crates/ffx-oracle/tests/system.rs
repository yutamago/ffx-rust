//! The system layer (`ffx_system`) against the original, on two oracles
//! started alike: one runs `FFX.exe`'s code, the other the Rust bridge on
//! the same memory; results and every written byte must agree. Synthetic
//! cases need `FFX_GAME_DIR`; the snapshot cases also `FFX_SNAPSHOT`
//! (both kept snapshots: `field.snap`, `battle.snap`).
//!
//! The float cases run at the game's control word (`0x027f`, 53-bit) and
//! compare bits, NaNs included.

use std::path::PathBuf;

use ffx_bridge::{Global, Mem};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_system::{
    accessor, btl, cd, chr_plate, cmdwin, config, container, daps, dtor, files, fixup, instance,
    intl, math, menu, msg, names, pad, pkt, render, save, scene, seq_prim, sort, stl, stub, sync,
    tolib, vbf, window,
};

/// The game thread's x87 control word.
const GAME_FPU: u32 = 0x027f;

fn exe() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    Some(dir?.join("FFX.exe"))
}

/// Two oracles on the image alone, at the game's control word.
fn synthetic() -> Option<(Oracle, Oracle)> {
    let exe = exe()?;
    let mut a = Oracle::load(&exe).unwrap();
    let mut b = Oracle::load(&exe).unwrap();
    a.set_fpu_control(GAME_FPU).unwrap();
    b.set_fpu_control(GAME_FPU).unwrap();
    Some((a, b))
}

/// Two oracles on the snapshot in `FFX_SNAPSHOT`.
fn snapshot() -> Option<(Oracle, Oracle)> {
    let exe = exe()?;
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; skipping");
        return None;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    Some((
        Oracle::from_snapshot(&exe, &pages).unwrap(),
        Oracle::from_snapshot(&exe, &pages).unwrap(),
    ))
}

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// Mostly ordinary floats of mixed magnitudes, sometimes any bit
    /// pattern (NaNs, infinities, denormals) or a special value.
    fn float(&mut self) -> f32 {
        const SPECIAL: [u32; 10] = [
            0x0000_0000, // 0
            0x8000_0000, // -0
            0x7f80_0000, // inf
            0xff80_0000, // -inf
            0x7fc0_0000, // quiet NaN
            0x7f80_0001, // signalling NaN
            0xffc1_2345, // negative NaN with a payload
            0x0000_0001, // smallest denormal
            0x7f7f_ffff, // largest finite
            0x3f80_0000, // 1
        ];
        match self.next() % 16 {
            0 => f32::from_bits(self.next()),
            1 => f32::from_bits(SPECIAL[(self.next() % 10) as usize]),
            _ => {
                let e = self.next() % 40 + 107; // 2^-20 .. 2^20
                f32::from_bits(self.next() & 0x807f_ffff | e << 23)
            }
        }
    }
}

/// Calls `sym` in `o`.
fn call(o: &mut Oracle, sym: u32, conv: Conv, args: &[u32]) -> ffx_oracle::Ret {
    let at = o.rebase(sym);
    o.call(at, conv, args)
        .unwrap_or_else(|e| panic!("{sym:#010x}: {e}"))
}

fn eax(o: &mut Oracle, sym: u32, args: &[u32]) -> u32 {
    call(o, sym, Conv::Cdecl, args).eax
}

/// The same block in both oracles' heaps.
fn alloc(a: &mut Oracle, b: &mut Oracle, len: usize) -> u32 {
    let x = a.alloc(len).unwrap();
    assert_eq!(b.alloc(len).unwrap(), x);
    x
}

/// Stores `value` at `addr` in both.
fn put<T: ffx_bridge::Value>(a: &mut Oracle, b: &mut Oracle, addr: u32, value: T) {
    Mem::write(a, addr, value);
    Mem::write(b, addr, value);
}

/// Stores `value` in global `g` in both.
fn set<T: ffx_bridge::Value>(a: &mut Oracle, b: &mut Oracle, g: Global<T>, value: T) {
    g.set(a, value);
    g.set(b, value);
}

fn floats(m: &Oracle, addr: u32, n: usize) -> Vec<u32> {
    (0..n)
        .map(|i| Mem::read::<f32>(m, addr + 4 * u32::try_from(i).unwrap()).to_bits())
        .collect()
}

/// Both oracles' memory agrees.
fn same_memory(what: &str, a: &mut Oracle, b: &mut Oracle) {
    assert_eq!(a.diff(b).unwrap(), [], "{what}: memory");
}

/// Forgets the pages both sides wrote so far, after a diff found them
/// equal: a page neither side writes again stays equal, so later diffs need
/// only the pages written after this. Without it every diff of a long
/// random run re-read every page an earlier round wrote.
fn settle(a: &mut Oracle, b: &mut Oracle) {
    a.clear_dirty();
    b.clear_dirty();
}

/// A Rust getter.
type Getter = fn(&Oracle) -> u32;

const CDC_MUL4X4: u32 = 0x0070_58f0;
const CDC_MUL4X4_MATRIX: u32 = 0x0070_59b0;
const CDC_MUL3X4: u32 = 0x0070_5850;
const CDC_TRANSPOSE: u32 = 0x0070_63f0;
const CDC_ORTHONORMAL_INVERSE3X4: u32 = 0x0070_5db0;
const CDC_ROT_X: u32 = 0x0070_5ed0;
const CDC_ROT_Z: u32 = 0x0070_6070;
const SCE_VU0_VECTOR_LENGTH_SQUARED: u32 = 0x006e_d9b0;

/// A Rust matrix function `f(dst, x, y)`.
type Matrix3 = fn(&mut Oracle, u32, u32, u32);
/// A Rust matrix function `f(dst, x)`.
type Matrix2 = fn(&mut Oracle, u32, u32);

/// Every third case NaN-heavy: half the inputs NaNs with random payloads,
/// so products and sums of two NaNs come up. All NaNs quiet: the oracle's
/// x87 mishandles signalling ones (see
/// `the_oracle_keeps_loaded_signalling_nans_signalling`); the model's
/// handling of them is checked against the hardware in `ffx_bridge::x87`.
fn random_floats(rng: &mut Xorshift, n: usize) -> Vec<f32> {
    let nans = rng.next().is_multiple_of(3);
    (0..n)
        .map(|_| {
            let x = if nans && rng.next().is_multiple_of(2) {
                f32::from_bits(rng.next() | 0x7f80_0000)
            } else {
                rng.float()
            };
            if x.is_nan() {
                f32::from_bits(x.to_bits() | 0x0040_0000)
            } else {
                x
            }
        })
        .collect()
}

fn write_floats(a: &mut Oracle, b: &mut Oracle, at: u32, v: &[f32]) {
    for (i, &x) in v.iter().enumerate() {
        put(a, b, at + 4 * u32::try_from(i).unwrap(), x);
    }
}

#[test]
fn matrix_products_match_the_original_bit_for_bit() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_cdc0);
    // one block: [dst 64][x 64][y 64], dst moved around to overlap the inputs
    let block = alloc(&mut a, &mut b, 0x100);
    let cases: [(&str, u32, Matrix3); 3] = [
        ("cdc_Mul4x4", CDC_MUL4X4, |m, d, x, y| {
            math::bridge::mul4x4_vector(m, d, x, y);
        }),
        ("cdc_Mul3x4", CDC_MUL3X4, |m, d, x, y| {
            math::bridge::mul3x4(m, d, x, y);
        }),
        ("cdc_Mul4x4_007059b0", CDC_MUL4X4_MATRIX, |m, d, x, y| {
            math::bridge::mul4x4(m, d, x, y);
        }),
    ];
    for (name, sym, rust) in cases {
        for n in 0..3000 {
            let inputs = random_floats(&mut rng, 32);
            write_floats(&mut a, &mut b, block + 0x40, &inputs);
            // apart, then dst on x, on y, and half-overlapping each
            let dst = block
                + match n % 6 {
                    0..=1 => 0,
                    2 => 0x40,
                    3 => 0x80,
                    4 => 0x44,
                    _ => 0x3c,
                };
            let (x, y) = (block + 0x40, block + 0x80);
            let ret = eax(&mut a, sym, &[dst, x, y]);
            rust(&mut b, dst, x, y);
            if sym != CDC_MUL4X4_MATRIX {
                // the shims return dst: callers read the eax these leave
                assert_eq!(ret, dst, "{name} case {n}: eax");
            }
            assert_eq!(
                floats(&a, block, 48),
                floats(&b, block, 48),
                "{name} case {n} dst +{:#x}: {inputs:?}",
                dst - block
            );
        }
        same_memory(name, &mut a, &mut b);
    }
}

#[test]
fn matrix_copies_match_the_original_bit_for_bit() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_cdc1);
    let block = alloc(&mut a, &mut b, 0x100);
    let unary: [(&str, u32, Matrix2); 2] = [
        ("cdc_Transpose", CDC_TRANSPOSE, |m, d, x| {
            math::bridge::transpose(m, d, x);
        }),
        (
            "cdc_OrthonormalInverse3x4",
            CDC_ORTHONORMAL_INVERSE3X4,
            |m, d, x| math::bridge::orthonormal_inverse3x4(m, d, x),
        ),
    ];
    for (name, sym, rust) in unary {
        for n in 0..3000 {
            let inputs = random_floats(&mut rng, 16);
            write_floats(&mut a, &mut b, block + 0x40, &inputs);
            let dst = block + if n % 3 == 0 { 0x40 } else { 0 };
            eax(&mut a, sym, &[dst, block + 0x40]);
            rust(&mut b, dst, block + 0x40);
            assert_eq!(
                floats(&a, block, 32),
                floats(&b, block, 32),
                "{name} case {n}: {inputs:?}"
            );
        }
        same_memory(name, &mut a, &mut b);
    }
}

#[test]
fn rotations_and_lengths_match_the_original_bit_for_bit() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_cdc2);
    let block = alloc(&mut a, &mut b, 0x100);
    for (name, sym, first) in [("RotX", CDC_ROT_X, 1), ("RotZ", CDC_ROT_Z, 0)] {
        for n in 0..3000 {
            let inputs = random_floats(&mut rng, 18);
            write_floats(&mut a, &mut b, block, &inputs[..16]);
            let (s, c) = (inputs[16], inputs[17]);
            let ra = call(
                &mut a,
                sym,
                Conv::Thiscall(block),
                &[s.to_bits(), c.to_bits()],
            )
            .eax;
            let rb = math::bridge::rotate(&mut b, block, first, s, c);
            assert_eq!(ra, rb, "{name}: returns this");
            assert_eq!(
                floats(&a, block, 16),
                floats(&b, block, 16),
                "{name} case {n}: {inputs:?}"
            );
        }
        same_memory(name, &mut a, &mut b);
    }

    for n in 0..5000 {
        let v = random_floats(&mut rng, 3);
        write_floats(&mut a, &mut b, block, &v);
        let st0 = call(&mut a, SCE_VU0_VECTOR_LENGTH_SQUARED, Conv::Cdecl, &[block])
            .st0
            .expect("a float result");
        let r = math::bridge::length_squared(&b, block);
        // ST0 comes back as an f64 (exact for an f32, NaN payloads kept)
        #[allow(clippy::cast_possible_truncation)] // exact: an f32 value
        let st0 = st0 as f32;
        assert_eq!(st0.to_bits(), r.to_bits(), "length² case {n}: {v:?}");
    }
}

#[test]
fn the_precision_the_matrix_math_assumes_is_the_games() {
    // v (1, 1, 1, 0) times a column (1, 2^-24, 2^-60, 0): at 53 bits the sum
    // is 1 + 2^-24, a tie that rounds to 1.0; at 64 bits it rounds up.
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let block = alloc(&mut a, &mut b, 0x100);
    let (vec, mat) = (block + 0x40, block + 0x80);
    let mut col = [0.0f32; 16];
    col[0] = 1.0;
    col[4] = 2f32.powi(-24);
    col[8] = 2f32.powi(-60);
    write_floats(&mut a, &mut b, mat, &col);
    write_floats(&mut a, &mut b, vec, &[1.0, 1.0, 1.0, 0.0]);
    math::bridge::mul4x4_vector(&mut b, block, mat, vec);
    eax(&mut a, CDC_MUL4X4, &[block, mat, vec]);
    assert_eq!(Mem::read::<f32>(&a, block), 1.0);
    assert_eq!(Mem::read::<f32>(&b, block), 1.0);
    a.set_fpu_control(0x037f).unwrap();
    eax(&mut a, CDC_MUL4X4, &[block, mat, vec]);
    assert_eq!(
        Mem::read::<f32>(&a, block),
        1.0 + 2f32.powi(-23),
        "the oracle honours the precision control"
    );
    a.set_fpu_control(GAME_FPU).unwrap();

    // x² - y for x = 1 + 2^-23, y = 1 + 2^-22: f32 arithmetic gives 0
    let x = 1.0 + 2f32.powi(-23);
    let y = 1.0 + 2f32.powi(-22);
    let mut col = [0.0f32; 16];
    col[0] = x;
    col[4] = y;
    write_floats(&mut a, &mut b, mat, &col);
    write_floats(&mut a, &mut b, vec, &[x, -1.0, 0.0, 0.0]);
    eax(&mut a, CDC_MUL4X4, &[block, mat, vec]);
    math::bridge::mul4x4_vector(&mut b, block, mat, vec);
    assert_eq!(Mem::read::<f32>(&a, block), 2f32.powi(-46));
    assert_eq!(Mem::read::<f32>(&b, block), 2f32.powi(-46));
    assert_eq!(x * x - y, 0.0, "f32 steps lose it");
}

const SET_PTR32: u32 = 0x00a4_4700;
const GET_PTR32_FUNC: u32 = 0x00a4_4650;
const IS_PTR32_NULL: u32 = 0x00a4_46a0;
const TO_SKIP_CODE: u32 = 0x008d_fd80;
const TO_GET_GROUP: u32 = 0x008d_f610;
const SCE_MC_SYNC: u32 = 0x0062_f9f0;

/// The one-instruction accessors (`ffx_system::accessor`), on the image's
/// globals and random objects and arguments.
#[test]
#[allow(clippy::many_single_char_names)] // a, b: the oracles; x, y: arguments
fn accessors_match_the_original() {
    use accessor::bridge as acc;
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    let mut rng = Xorshift(0xacce_5502);
    for (f, v) in accessor::CONSTANTS {
        assert_eq!(eax(a, f, &[]), v, "{f:#x}");
    }
    for (f, _) in accessor::ADDRESSES {
        assert_eq!(eax(a, f, &[]), acc::address(b, f), "{f:#x}");
    }
    for (f, g) in accessor::GLOBALS {
        for _ in 0..4 {
            set(a, b, Global::<u32>::new(g), rng.next());
            assert_eq!(eax(a, f, &[]), acc::global(b, f), "{f:#x}");
        }
    }
    let world_map = random_block(a, b, &mut rng, 0x80);
    set(a, b, Global::<u32>::new(0x0187_1698), world_map);
    assert_eq!(eax(a, 0x008e_2d50, &[]), acc::world_map_window_word64(b));
    for _ in 0..50 {
        let (x, y) = (rng.next(), rng.next());
        for f in accessor::MINUS_ONE_3 {
            assert_eq!(
                call(a, f, Conv::Stdcall, &[x, y, x]).eax,
                u32::MAX,
                "{f:#x}"
            );
        }
        for f in accessor::FIRST {
            assert_eq!(eax(a, f, &[x, y]), stub::ptr32(x), "{f:#x}");
        }
        for f in accessor::FIRST_STDCALL {
            assert_eq!(
                call(a, f, Conv::Stdcall, &[x]).eax,
                stub::ptr32(x),
                "{f:#x}"
            );
        }
        assert_eq!(
            call(a, 0x0067_70b0, Conv::Stdcall, &[x]).eax,
            accessor::SIZE_0X18
        );
        assert_eq!(eax(a, 0x0040_1000, &[x, y]), accessor::second(x, y));
        let byte = u8::try_from(x & 0xff).unwrap();
        assert_eq!(
            eax(a, 0x0060_ab40, &[x]) & 0xff,
            u32::from(accessor::first_u8(byte))
        );
        let half = u16::try_from(x & 0xffff).unwrap();
        assert_eq!(
            eax(a, 0x0061_e710, &[x]) & 0xffff,
            u32::from(accessor::first_u16(half))
        );
        let r = call(a, 0x0060_ab70, Conv::Cdecl, &[x]);
        assert_eq!(
            u64::from(r.edx) << 32 | u64::from(r.eax),
            accessor::zero_extend(x)
        );
        assert_eq!(eax(a, 0x0060_ab80, &[x, y]), accessor::add(x, y));
        assert_eq!(eax(a, 0x0061_d110, &[x, y]), accessor::add(x, y));
        assert_eq!(eax(a, 0x0061_e6e0, &[x, y]), accessor::difference(x, y));
    }
    let obj = random_block(a, b, &mut rng, 0x400);
    let rec = random_block(a, b, &mut rng, 0x10);
    put(a, b, obj + 0xc, rec);
    let r = call(a, 0x0061_acb0, Conv::Thiscall(obj), &[]);
    assert_eq!(
        u64::from(r.edx) << 32 | u64::from(r.eax),
        acc::entry_size64(b, obj)
    );
    assert_eq!(
        call(a, 0x0063_1310, Conv::Thiscall(obj), &[]).eax,
        acc::stick_info(obj)
    );
    for state in 0..8u32 {
        put(a, b, obj + 8, state);
        for (f, s) in [(0x006f_20d0, 6), (0x006f_2150, 4), (0x006f_21a0, 2)] {
            let ra = call(a, f, Conv::Thiscall(obj), &[]).eax;
            assert_eq!(ra, acc::state_is(b, obj, s), "{f:#x} {state}");
        }
    }
    call(a, 0x009e_24f0, Conv::Thiscall(obj), &[]);
    acc::set_vftable(b, obj, acc::PINPUT_SOURCE_VFTABLE);
    call(a, 0x0061_a730, Conv::Thiscall(obj + 0x10), &[]);
    acc::set_vftable(b, obj + 0x10, acc::PWORLD_MATRIX_CONTROLLER_VFTABLE);
    call(a, 0x006f_1db0, Conv::Thiscall(obj + 0x20), &[]);
    acc::set_word0(b, obj + 0x20);
    call(a, 0x006f_1dc0, Conv::Thiscall(obj), &[]);
    acc::set_byte364(b, obj);
    same_memory("accessor stores", a, b);
}

/// The `dvd_cd_*` layout functions and their 63 per-language getters, for
/// every language byte (`TkGetLang` reads `0x01330830`).
#[test]
fn dvd_cd_layout_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    for g in (0x0077_a4f0..0x0077_a8e0).step_by(0x10) {
        let want = cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(g)];
        assert_eq!(eax(&mut a, g, &[]), want, "{g:#x}");
    }
    for lang in (0..=20u8).chain([0x7f, 0x80, 0xff]) {
        put(&mut a, &mut b, 0x0133_0830, lang);
        for (k, want) in cd::DVD_CD_LAYOUT.into_iter().enumerate() {
            let f = 0x0076_d6c0 + 0x80 * u32::try_from(k).unwrap();
            assert_eq!(eax(&mut a, f, &[]), want, "{f:#x} lang {lang}");
        }
    }
}

#[test]
fn pointer_helpers_stubs_and_text_codes_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x0bad_cafe);
    let mut values: Vec<u32> = vec![0, 1, 0x8000_0000, u32::MAX];
    values.extend((0..200).map(|_| rng.next()));
    for &p in &values {
        assert_eq!(eax(&mut a, SET_PTR32, &[p]), stub::ptr32(p));
        assert_eq!(eax(&mut a, GET_PTR32_FUNC, &[p]), stub::ptr32(p));
        assert_eq!(eax(&mut a, IS_PTR32_NULL, &[p]), stub::is_ptr32_null(p));
        assert_eq!(eax(&mut a, TO_GET_GROUP, &[p]), msg::group(p));
    }
    for code in 0..0x200 {
        assert_eq!(
            eax(&mut a, TO_GET_GROUP, &[code]),
            msg::group(code),
            "{code:#x}"
        );
    }
    for (sym, value) in [
        (0x0062_f550, stub::CD_DISK_READY),         // sceCdDiskReady
        (0x00a4_47c0, stub::CD_READ),               // sceCdRead
        (0x00a4_4790, stub::SIF_SET_DMA),           // sceSifSetDma
        (0x0062_fb30, stub::PAD_GET_STATE),         // scePadGetState
        (0x0072_ac20, stub::MODEL_BUFFER_ALLOCATE), // ModelBuffer_AllocateBuffer
    ] {
        assert_eq!(eax(&mut a, sym, &[1, 2, 3]), value, "{sym:#x}");
    }
    let out = alloc(&mut a, &mut b, 8);
    put(&mut a, &mut b, out, [7u32, 9]);
    for (cmd, result) in [(out, out + 4), (out + 4, out), (out, out)] {
        let ra = eax(&mut a, SCE_MC_SYNC, &[0, cmd, result]);
        let rb = stub::bridge::mc_sync(&mut b, cmd, result);
        assert_eq!(ra, rb.cast_unsigned(), "sceMcSync");
    }
    same_memory("sceMcSync", &mut a, &mut b);

    // every first byte, with each kind of second byte, and runs of G
    let text = alloc(&mut a, &mut b, 16);
    let mut cases: Vec<[u8; 6]> = Vec::new();
    for first in 0..=255u8 {
        for second in [0, 0x2f, 0x30, b'G', 0xff] {
            cases.push([first, second, b'G', b'G', 0, 0]);
        }
    }
    cases.push(*b"GGGGG\0");
    cases.push(*b"GGGGGx");
    for c in cases {
        put(&mut a, &mut b, text, c);
        let ra = eax(&mut a, TO_SKIP_CODE, &[text]);
        assert_eq!(ra, msg::bridge::skip_code(&b, text), "TOSkipCode {c:x?}");
    }
    same_memory("text codes", &mut a, &mut b);
}

// The CD layer.
const GET_CD: u32 = 0x0076_d0d0;
const CD_FILE_SECTOR: u32 = 0x0076_d510;
const CD_FILE_SIZE_CD_EX: u32 = 0x0076_c410;
const CHECK_EX_FILE_SIZE: u32 = 0x0076_d660;
const DISK_OPEN_CHECK_SET: u32 = 0x0076_d6b0;
const HDD_MOUNT_FLG_GET: u32 = 0x0077_9560;
const HDD_RMODE_GET: u32 = 0x0077_9570;
const HDD_HEAD_RESET: u32 = 0x0077_89e0;
const CLEAR_READ_CHGNAME: u32 = 0x00a4_2f30;
const SET_FULL_ARENA_LOCALIZATION_ENABLE: u32 = 0x00a4_31f0;

/// Whether `addr..addr + 4` is mapped (in the snapshot) in `a`.
fn mapped(a: &mut Oracle, addr: u32) -> bool {
    a.read(addr, 4).is_ok()
}

fn cd_layer(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use cd::bridge as c;
    assert_eq!(eax(a, GET_CD, &[]), c::get_cd(b));
    // the sector table as the game loaded it (snapshot) or the image's
    // (synthetic: a null table, so a fake one)
    if c::SECTORS.get(b) == 0 {
        let t = alloc(a, b, 0x400);
        for i in 0..0x100 {
            put(a, b, t + 4 * i, rng.next());
        }
        set(a, b, c::SECTORS, t);
        set(a, b, c::SECTOR_BASE, rng.next());
    }
    let table = c::SECTORS.get(b);
    for n in 0..0x4000 {
        if !mapped(a, table + 4 * n) {
            break;
        }
        let n = n.cast_signed();
        let ra = eax(a, CD_FILE_SECTOR, &[n.cast_unsigned()]);
        assert_eq!(ra, c::cd_file_sector(b, n), "cd_file_sector {n}");
    }
    // the extra size table: every entry and some past the end
    if c::EX_SIZES.get(b) == 0 {
        let t = alloc(a, b, 0x100);
        for i in 0..0x40 {
            put(a, b, t + 4 * i, rng.next());
        }
        set(a, b, c::EX_SIZES, t);
        set(a, b, c::EX_SIZE_COUNT, 0x40);
    }
    let base = c::FILE_BASE.get(b);
    let count = c::EX_SIZE_COUNT.get(b);
    for i in 0..count + 8 {
        let n = i.wrapping_sub(base);
        let ra = eax(a, CD_FILE_SIZE_CD_EX, &[n.cast_unsigned()]);
        assert_eq!(ra, c::cd_file_size_ex(b, n), "CDfileSize_CD_EX {n}");
    }
    // the HDD-install table: the game's, then a made-up one
    for round in 0..2 {
        if round == 1 {
            let t = alloc(a, b, 0x34 * 12);
            for i in 0..12 {
                let e = t + 0x34 * i;
                put(a, b, e, u8::try_from(i % 3).unwrap());
                put(a, b, e + 8, i / 2);
                put(a, b, e + 0x10, rng.next());
            }
            set(a, b, c::HDD_INSTALL, t);
            set(a, b, c::HDD_INSTALL_COUNT, 12);
        }
        for archive in [0, 1, 2, 3, 0x100] {
            for n in -1i32..8 {
                let ra = eax(a, CHECK_EX_FILE_SIZE, &[archive, n.cast_unsigned()]);
                assert_eq!(
                    ra,
                    c::check_ex_file_size(b, archive, n),
                    "check_ex_file_size({archive}, {n})"
                );
            }
        }
    }
    set(a, b, c::HDD_INSTALL_COUNT, -3);
    assert_eq!(
        eax(a, CHECK_EX_FILE_SIZE, &[0, 0]),
        c::check_ex_file_size(b, 0, 0)
    );
    same_memory("CD tables", a, b);
}

fn hdd_and_flags(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use cd::bridge as c;
    assert_eq!(
        eax(a, HDD_MOUNT_FLG_GET, &[]),
        c::hdd_mount_flg_get(b).cast_unsigned()
    );
    for v in [0x7f_i8, -1, 0] {
        set(a, b, c::HDD_MOUNTED, v);
        assert_eq!(
            eax(a, HDD_MOUNT_FLG_GET, &[]),
            c::hdd_mount_flg_get(b).cast_unsigned()
        );
    }
    for n in 0..cd::bridge::HDD_HEAD_COUNT {
        let n = n.cast_signed();
        assert_eq!(
            eax(a, HDD_RMODE_GET, &[n.cast_unsigned()]),
            c::hdd_rmode_get(b, n).cast_unsigned()
        );
    }
    let heads = c::HDD_HEADS.ptr(b).addr;
    put(a, b, heads + 8, -5i8);
    assert_eq!(
        eax(a, HDD_RMODE_GET, &[0]),
        c::hdd_rmode_get(b, 0).cast_unsigned()
    );
    // scramble the heads, then reset them
    for i in 0..cd::bridge::HDD_HEAD_COUNT * 3 {
        put(a, b, heads + 4 * i, rng.next());
    }
    eax(a, HDD_HEAD_RESET, &[]);
    c::hdd_head_reset(b);
    same_memory("hdd_head_reset", a, b);
    // and the bytes just past them are not touched
    let after = heads + 12 * cd::bridge::HDD_HEAD_COUNT;
    assert_eq!(Mem::read::<u32>(a, after), Mem::read::<u32>(b, after));

    for flag in [1u8, 0xff, 0] {
        eax(a, DISK_OPEN_CHECK_SET, &[u32::from(flag) | 0xabcd_ef00]);
        c::disk_open_check_set(b, flag);
    }
    for i in 0..16 {
        let from = c::CHGNAME_FROM.ptr(b).addr + 4 * i;
        put(a, b, from, rng.next());
        put(a, b, from + 0x40, rng.next());
    }
    for i in [0, 3, 15] {
        eax(a, CLEAR_READ_CHGNAME, &[i]);
        c::clear_read_chgname(b, i.cast_signed());
    }
    for v in [0, 1, 5, 0, u32::MAX] {
        set(a, b, c::FULL_ARENA_LOCALIZATION, v);
        eax(a, SET_FULL_ARENA_LOCALIZATION_ENABLE, &[]);
        c::set_full_arena_localization_enable(b);
    }
    same_memory("CD layer", a, b);
}

// Save data.
const SAVE_DATA_MANAGER_INSTANCE: u32 = 0x006f_2070;
const SAVE_DATA_MANAGER_GET_REF_BUFFER: u32 = 0x006f_1ed0;
const SAVE_DATA_MANAGER_SET_CB_RESULT: u32 = 0x006f_2500;
const SAVE_DATA_MANAGER_IS_BUSY: u32 = 0x006f_2090;
const SAVE_DATA_SET_EVENT_ID: u32 = 0x0064_8630;
const SAVE_DATA_GET_SCREEN_STATE: u32 = 0x0064_7f70;
const SAVE_DATA_IS_SCREEN_CLOSED: u32 = 0x0064_8100;
const SAVE_DATA_GET_DIALOG_STATE: u32 = 0x0064_7ee0;
const SAVE_DATA_SET_DIALOG_STATE: u32 = 0x0064_8620;
const SAVE_DATA_IS_SAVE: u32 = 0x0064_80d0;
const SAVE_DATA_SET_SAVE: u32 = 0x0064_86b0;
const SAVE_DATA_IS_LOADING_SUCCESSFUL: u32 = 0x0064_80a0;

fn save_data(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use save::bridge as s;
    let getters: [(u32, Getter); 6] = [
        (SAVE_DATA_MANAGER_INSTANCE, |m| s::manager_instance(m)),
        (SAVE_DATA_GET_SCREEN_STATE, |m| s::get_screen_state(m)),
        (SAVE_DATA_IS_SCREEN_CLOSED, |m| s::is_screen_closed(m)),
        (SAVE_DATA_GET_DIALOG_STATE, |m| s::get_dialog_state(m)),
        (SAVE_DATA_IS_SAVE, |m| s::is_save(m)),
        (SAVE_DATA_IS_LOADING_SUCCESSFUL, |m| {
            s::is_loading_successful(m)
        }),
    ];
    let check = |a: &mut Oracle, b: &mut Oracle| {
        for (sym, rust) in getters {
            assert_eq!(eax(a, sym, &[]), rust(b), "{sym:#x}");
        }
    };
    check(a, b);
    for v in [0, 1, 2, u32::MAX, rng.next()] {
        set(a, b, s::SCREEN_STATE, v);
        set(a, b, s::DIALOG_STATE, v);
        set(a, b, s::IS_SAVE, v);
        #[allow(clippy::cast_possible_truncation)] // the low byte
        set(a, b, s::LOAD_RESULT, v as u8);
        check(a, b);
    }
    for v in [0, 1, rng.next()] {
        eax(a, SAVE_DATA_SET_EVENT_ID, &[v]);
        s::set_event_id(b, v);
        eax(a, SAVE_DATA_SET_DIALOG_STATE, &[v]);
        s::set_dialog_state(b, v);
        eax(a, SAVE_DATA_SET_SAVE, &[v]);
        s::set_save(b, v);
        check(a, b);
    }
    // the manager: the game's (snapshot) and a made-up one
    let fake = alloc(a, b, 0x360);
    for this in [s::MANAGER.get(b), fake] {
        if this == 0 {
            continue;
        }
        let ra = call(
            a,
            SAVE_DATA_MANAGER_GET_REF_BUFFER,
            Conv::Thiscall(this),
            &[],
        )
        .eax;
        assert_eq!(ra, s::get_ref_buffer(b, this));
        let ra = call(a, SAVE_DATA_MANAGER_IS_BUSY, Conv::Thiscall(this), &[]).eax;
        assert_eq!(ra, s::is_busy(b, this), "isBusy, the game's state");
    }
    put(a, b, fake + 0x350, rng.next());
    for state in -3..20 {
        put(a, b, fake + 8, state);
        let ra = call(a, SAVE_DATA_MANAGER_IS_BUSY, Conv::Thiscall(fake), &[]).eax;
        assert_eq!(ra, s::is_busy(b, fake), "isBusy state {state}");
    }
    for v in [0, 7, rng.next()] {
        call(
            a,
            SAVE_DATA_MANAGER_SET_CB_RESULT,
            Conv::Thiscall(fake),
            &[v],
        );
        s::set_cb_result(b, fake, v);
    }
    same_memory("save data", a, b);
}

// Singletons and accessors.
const CLUSTER_MANAGER_GET_INSTANCE: u32 = 0x0069_a1a0;
const ASYNC_LOAD_MANAGER_S_GET_INSTANCE: u32 = 0x0069_1b40;
const ASYNC_LOAD_MANAGER_IS_ALL_REQUEST_DONE: u32 = 0x0069_1630;
const BIG_FILE_STREAM_GET: u32 = 0x0061_bdb0;
const MAGIC_FILE_GET_MAGIC_ID: u32 = 0x009d_a3b0;
const MAGIC_FILE_GET_HANDLE: u32 = 0x009d_a390;
const MAGIC_FILE_IS_NEX_DATA: u32 = 0x009d_a420;
const ASSET_LIST_READER_GET_ASSET_COUNT: u32 = 0x006f_4170;

fn instances(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use instance::bridge as i;
    let getters: [(u32, Global<u32>, Getter); 6] = [
        (CLUSTER_MANAGER_GET_INSTANCE, i::CLUSTER_MANAGER, |m| {
            i::cluster_manager(m)
        }),
        (
            ASYNC_LOAD_MANAGER_S_GET_INSTANCE,
            i::ASYNC_LOAD_MANAGER,
            |m| i::async_load_manager(m),
        ),
        (BIG_FILE_STREAM_GET, i::BIG_FILE_STREAM, |m| {
            i::big_file_stream(m)
        }),
        (MAGIC_FILE_GET_MAGIC_ID, i::MAGIC_ID, |m| i::magic_id(m)),
        (MAGIC_FILE_GET_HANDLE, i::MAGIC_HANDLE, |m| {
            i::magic_handle(m)
        }),
        (MAGIC_FILE_IS_NEX_DATA, i::MAGIC_IS_NEX, |m| {
            i::magic_is_nex(m)
        }),
    ];
    for (sym, global, rust) in getters {
        assert_eq!(eax(a, sym, &[]), rust(b), "{sym:#x}");
        let v = rng.next();
        set(a, b, global, v);
        assert_eq!(eax(a, sym, &[]), rust(b), "{sym:#x}");
    }
    let manager = i::ASYNC_LOAD_MANAGER.get(b);
    let fake = alloc(a, b, 0x40);
    for this in [manager, fake] {
        if this == 0 || !mapped(a, this + 0x2c) {
            continue;
        }
        for (x, y) in [
            (None, None),
            (Some(0), Some(0)),
            (Some(1), Some(0)),
            (Some(0), Some(5)),
        ] {
            if let (Some(x), Some(y)) = (x, y) {
                put(a, b, this + 0x20, x);
                put(a, b, this + 0x2c, y);
            }
            let ra = call(
                a,
                ASYNC_LOAD_MANAGER_IS_ALL_REQUEST_DONE,
                Conv::Thiscall(this),
                &[],
            )
            .eax;
            assert_eq!(ra & 0xff, u32::from(i::is_all_request_done(b, this)));
        }
    }
    for (begin, end) in [
        (0x1000, 0x1000),
        (0x1000, 0x1018),
        (0x1000, 0x1031),
        (0x1031, 0x1000),
    ] {
        put(a, b, fake + 0x14, begin);
        put(a, b, fake + 0x18, end);
        let ra = call(
            a,
            ASSET_LIST_READER_GET_ASSET_COUNT,
            Conv::Thiscall(fake),
            &[],
        )
        .eax;
        assert_eq!(
            ra,
            i::asset_count(b, fake).cast_unsigned(),
            "{begin:#x}..{end:#x}"
        );
    }
    same_memory("instances", a, b);
}

// Configuration.
const FFX_IS_DEBUG: u32 = 0x0088_7c50;
const FFX_GET_ASIA_MODE: u32 = 0x0088_7c20;
const FFX_SET_ASIA_MODE: u32 = 0x0088_7c70;
const TO_GET_FFX_LANG_SWAP_FLAG: u32 = 0x008a_c390;
const TO_GET_FFX_VIDEO_MODE: u32 = 0x008a_c3a0;
const TO_GET_SYSTEM_CONFIRM_BUTTON_ASSIGN: u32 = 0x008a_cf30;
const IS_ETERNAL_CALM_SCENE: u32 = 0x0067_ab50;
const IS_ETERNAL_CALM_MGRP: u32 = 0x0067_ab30;
const GET_ETERNAL_CALM_SCENE_NAME: u32 = 0x0067_aac0;
const SET_BATTLE_ENABLE: u32 = 0x0078_2f70;

fn configuration(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use config::bridge as c;
    let check = |a: &mut Oracle, b: &mut Oracle| {
        assert_eq!(eax(a, FFX_IS_DEBUG, &[]), c::is_debug(b).cast_unsigned());
        assert_eq!(eax(a, FFX_GET_ASIA_MODE, &[]), c::get_asia_mode(b));
        assert_eq!(eax(a, TO_GET_FFX_LANG_SWAP_FLAG, &[]), c::lang_swap_flag(b));
        assert_eq!(eax(a, TO_GET_FFX_VIDEO_MODE, &[]), c::video_mode(b));
        assert_eq!(
            eax(a, TO_GET_SYSTEM_CONFIRM_BUTTON_ASSIGN, &[]),
            c::confirm_button_assign(b).cast_unsigned()
        );
    };
    check(a, b);
    for v in [0, 1, 0x80, 0xff, rng.next()] {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        let byte = v as u8;
        set(a, b, c::IS_DEBUG, byte.cast_signed());
        set(a, b, c::ASIA_MODE, byte);
        set(a, b, c::LANG_SWAP, v);
        set(a, b, c::VIDEO_MODE, v);
        set(
            a,
            b,
            c::CONFIRM_BUTTON,
            if v == 0xff { -1 } else { v.cast_signed() },
        );
        check(a, b);
    }
    set(a, b, c::CONFIRM_BUTTON, -1);
    check(a, b);
    for v in [3u8, 0, 0xff] {
        eax(a, FFX_SET_ASIA_MODE, &[u32::from(v) | 0x1234_5600]);
        c::set_asia_mode(b, v);
    }
    for v in [1, 0, rng.next()] {
        eax(a, SET_BATTLE_ENABLE, &[v]);
        c::set_battle_enable(b, v);
    }
    // the Eternal Calm maps around the game's first one, and around the
    // ends of the integer range
    let first = c::ETERNAL_CALM_FIRST.get(b);
    for f in [first, i32::MIN, i32::MAX - 3] {
        set(a, b, c::ETERNAL_CALM_FIRST, f);
        for d in -3..10 {
            let n = f.wrapping_add(d).cast_unsigned();
            for sym in [IS_ETERNAL_CALM_SCENE, IS_ETERNAL_CALM_MGRP] {
                assert_eq!(eax(a, sym, &[n]), c::is_eternal_calm(b, n.cast_signed()));
            }
            assert_eq!(
                eax(a, GET_ETERNAL_CALM_SCENE_NAME, &[n]),
                c::eternal_calm_scene_name(b, n.cast_signed())
            );
        }
    }
    same_memory("configuration", a, b);
}

#[test]
fn system_state_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x5157_e300);
    cd_layer(&mut a, &mut b, &mut rng);
    hdd_and_flags(&mut a, &mut b, &mut rng);
    save_data(&mut a, &mut b, &mut rng);
    instances(&mut a, &mut b, &mut rng);
    configuration(&mut a, &mut b, &mut rng);
}

#[test]
fn system_state_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x5157_e301);
    cd_layer(&mut a, &mut b, &mut rng);
    hdd_and_flags(&mut a, &mut b, &mut rng);
    save_data(&mut a, &mut b, &mut rng);
    instances(&mut a, &mut b, &mut rng);
    configuration(&mut a, &mut b, &mut rng);
}

/// The oracle's x87 (Unicorn) keeps a signalling NaN signalling when `fld`
/// loads it, so it then loses to any quiet NaN; the hardware quiets it on
/// the load and the larger significand wins (`ffx_bridge::x87`'s
/// `the_model_is_the_hardware` checks the model against the machine's own
/// x87). The float cases above therefore leave out signalling NaNs that
/// meet another NaN; this pins the difference down.
#[test]
fn the_oracle_keeps_loaded_signalling_nans_signalling() {
    let Some((mut a, _)) = synthetic() else {
        return;
    };
    // over CdEnvInit, which nothing here calls:
    // mov eax, [esp+4]; fld dword [eax]; fld dword [eax+4]; faddp st(1), st;
    // fstp dword [eax+8]; ret
    let code = [
        0x8b, 0x44, 0x24, 0x04, 0xd9, 0x00, 0xd9, 0x40, 0x04, 0xde, 0xc1, 0xd9, 0x58, 0x08, 0xc3,
    ];
    let at = a.rebase(0x0076_c560);
    a.write(at, &code).unwrap();
    let buf = a.alloc(16).unwrap();
    let add = |a: &mut Oracle, x: u32, y: u32| {
        Mem::write(a, buf, [x, y]);
        a.call(at, Conv::Cdecl, &[buf]).unwrap();
        Mem::read::<u32>(a, buf + 8)
    };
    let model = |x: u32, y: u32| {
        use ffx_bridge::x87::Reg;
        (Reg::load(f32::from_bits(x)) + Reg::load(f32::from_bits(y)))
            .store()
            .to_bits()
    };
    // quiet NaNs: larger significand, then the positive one, as the hardware
    for (x, y) in [
        (0x7ff1_eb3f, 0xffec_cf72),
        (0xffec_cf72, 0x7ff1_eb3f),
        (0x7fc0_0005, 0xffc0_0005),
        (0xffc0_0005, 0x7fc0_0005),
    ] {
        assert_eq!(add(&mut a, x, y), model(x, y), "{x:08x} + {y:08x}");
    }
    // a signalling NaN with a larger significand: the hardware (and the
    // model) 0x7fffffff, the oracle the quiet one
    assert_eq!(model(0x7fbf_ffff, 0x7fc0_0001), 0x7fff_ffff);
    assert_eq!(add(&mut a, 0x7fbf_ffff, 0x7fc0_0001), 0x7fc0_0001);
}

// Round 2: the TO accessors, pads, battle windows, scene parameters, the
// fixup reader, the archive digest map, asset lists, render switches, save
// CRCs.

/// Points entry `i` of pointer table `table` at `p` in both.
fn set_entry(a: &mut Oracle, b: &mut Oracle, table: Global<u32>, i: u32, p: u32) {
    let at = table.ptr(b).addr + 4 * i;
    put(a, b, at, p);
}

/// A block of random bytes in both.
fn random_block(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, len: usize) -> u32 {
    let p = alloc(a, b, len);
    for k in (0..len).step_by(4) {
        put(a, b, p + u32::try_from(k).unwrap(), rng.next());
    }
    p
}

const TO_GET_WINDOW_STRUCT_ADRS: u32 = 0x0090_6fb0;
const TO_GET_WINDOW_TYPE: u32 = 0x0090_6c40;
const TO_GET_WINDOW_STATUS: u32 = 0x0090_6c20;
const TO_CLOSE_WINDOW: u32 = 0x0090_6ba0;
const TO_GET_MESSAGE_STATUS: u32 = 0x008b_7ca0;
const TO_CLOSE_MESSAGE: u32 = 0x008b_5810;
const TO_SET_MESSAGE_FUNC: u32 = 0x008b_8af0;
const TO_BTL_SET_MACRO_COMMAND_TYPE: u32 = 0x008b_57c0;
const TO_BTL_SET_MACRO_COMMAND_VALUE: u32 = 0x008b_57f0;
const TO_INIT_MACRO_CMD: u32 = 0x008b_81d0;
const MACRO_DIC_SECTION: u32 = 0x008b_70a0;
const TO_GET_SAVE_DIC_ADRS: u32 = 0x008b_7f60;

fn windows_and_messages(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use window::bridge as w;
    // windows 0..4 made up (in use or not, states 0..7 with handlers),
    // messages too; the game's own where the snapshot has them
    for i in 0..4u32 {
        let win = random_block(a, b, rng, 0x80);
        put(a, b, win, u16::from(i != 2));
        put(a, b, win + 8, u16::try_from(i).unwrap());
        set_entry(a, b, w::WINDOWS, i, win);
        let m = random_block(a, b, rng, 0x120);
        put(a, b, m, u16::from(i != 1));
        put(a, b, m + 0x2c, u16::try_from(i + 1).unwrap());
        put(a, b, m + 0x108, 0u32); // handler 2 unset
        set_entry(a, b, w::MSG_WINDOWS, i, m);
    }
    for i in 0..8 {
        let win = w::window(b, i);
        assert_eq!(eax(a, TO_GET_WINDOW_STRUCT_ADRS, &[i.cast_unsigned()]), win);
        if win != 0 && mapped(a, win + 0x80) {
            let iu = i.cast_unsigned();
            assert_eq!(
                eax(a, TO_GET_WINDOW_TYPE, &[iu]),
                w::window_type(b, i).cast_unsigned()
            );
            assert_eq!(
                eax(a, TO_GET_WINDOW_STATUS, &[iu]),
                w::window_status(b, i).cast_unsigned()
            );
            eax(a, TO_CLOSE_WINDOW, &[iu]);
            w::close_window(b, i);
        }
        let m = Mem::read::<u32>(b, w::MSG_WINDOWS.ptr(b).addr + 4 * i.cast_unsigned());
        if m != 0 && mapped(a, m + 0x120) {
            let iu = i.cast_unsigned();
            assert_eq!(
                eax(a, TO_GET_MESSAGE_STATUS, &[iu]),
                w::message_status(b, i).cast_unsigned()
            );
            for n in [0, 2, 3] {
                eax(a, TO_SET_MESSAGE_FUNC, &[iu, n]);
                w::set_message_func(b, i, n.cast_signed());
            }
            eax(a, TO_CLOSE_MESSAGE, &[iu]);
            w::close_message(b, i);
        }
    }
    same_memory("windows", a, b);

    // macro command records: 32 per message window, made up for windows 0..3
    for i in 0..3u32 {
        let recs = random_block(a, b, rng, 32 * 8);
        for n in 0..32 {
            set_entry(a, b, w::MSG_VARS, i * 32 + n, recs + 8 * n);
        }
    }
    for (i, n) in [(0, 0), (1, 31), (2, 7), (0, 5)] {
        let ty = rng.next() & 0xff;
        eax(a, TO_BTL_SET_MACRO_COMMAND_TYPE, &[i, n, ty | 0xabcd_ef00]);
        w::set_macro_command_type(
            b,
            i.cast_signed(),
            n.cast_signed(),
            u8::try_from(ty).unwrap(),
        );
        let v = rng.next();
        eax(a, TO_BTL_SET_MACRO_COMMAND_VALUE, &[i, n, v]);
        w::set_macro_command_value(b, i.cast_signed(), n.cast_signed(), v);
    }
    eax(a, TO_INIT_MACRO_CMD, &[1]);
    w::init_macro_cmd(b, 1);
    same_memory("macro commands", a, b);

    // the macro dictionaries: the game's where loaded, else made up
    for table in 0..4u32 {
        let mut d = Mem::read::<u32>(b, w::MACRO_DIC.ptr(b).addr + 4 * table);
        if d == 0 || !mapped(a, d.wrapping_sub(0x4c)) {
            d = random_block(a, b, rng, 0x100) + 0x4c;
            set_entry(a, b, w::MACRO_DIC, table, d);
        }
        for n in 0..32 {
            let ra = eax(a, MACRO_DIC_SECTION, &[n, table]);
            assert_eq!(
                ra,
                w::macro_dic_section(b, n.cast_signed(), table.cast_signed()),
                "MacroDic_Section({n}, {table})"
            );
        }
        let t = table.cast_signed();
        assert_eq!(eax(a, TO_GET_SAVE_DIC_ADRS, &[table]), w::save_dic(b, t));
    }
}

const TO_MENU_GET_CONTROL_PAD: u32 = 0x008b_e410;
const TO_MENU_GET_CONTROL_PAD_TRG: u32 = 0x008b_e4b0;
const TO_MENU_GET_CONTROL_PAD_REP: u32 = 0x008b_e470;
const TO_MENU_GET_CONTROL_PAD2: u32 = 0x008b_e440;
const TO_MENU_GET_DUMMY_ANALOG_CONTROLER: u32 = 0x008c_a320;
const SET_TO_PAD_GET_CTRL: u32 = 0x008a_b580;
const TO_DEBUG_ADD_PAD_PRINT_FLAG: u32 = 0x008a_bd00;

fn pads(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use pad::bridge as p;
    let flag_at = Global::<u8>::new(0x0186_a430); // g_TkMenuDummyControlerFlag
    for round in 0..40 {
        if round > 0 {
            #[allow(clippy::cast_possible_truncation)] // random halves
            let words = [0u32, 1, 2].map(|_| rng.next() as u16);
            let held = if round % 3 == 0 {
                words[0] & 0x0fff
            } else {
                words[0]
            };
            set(a, b, p::FIRST, [held, words[1], words[2]]);
            #[allow(clippy::cast_possible_truncation)]
            set(
                a,
                b,
                p::SECOND,
                [rng.next() as u16, rng.next() as u16, rng.next() as u16],
            );
            #[allow(clippy::cast_possible_truncation)]
            set(a, b, p::PAD2, rng.next() as u16);
            set(a, b, flag_at, [0u8, 1, 2][round % 3]);
            #[allow(clippy::cast_possible_truncation)]
            set(a, b, p::DUMMY_ANALOG, [0u32; 8].map(|_| rng.next() as u8));
        }
        let flag = ffx_menu::tk::bridge::tk_menu_get_dummy_controler_flag(b);
        for (which, sym) in [
            TO_MENU_GET_CONTROL_PAD,
            TO_MENU_GET_CONTROL_PAD_TRG,
            TO_MENU_GET_CONTROL_PAD_REP,
        ]
        .into_iter()
        .enumerate()
        {
            let ra = eax(a, sym, &[]);
            assert_eq!(
                ra,
                u32::from(p::control_pad(b, which, flag)),
                "pad {which} round {round}"
            );
        }
        assert_eq!(
            eax(a, TO_MENU_GET_CONTROL_PAD2, &[]) & 0xffff,
            u32::from(p::control_pad2(b))
        );
        for axis in 0..6 {
            let ra = eax(a, TO_MENU_GET_DUMMY_ANALOG_CONTROLER, &[axis]) & 0xff;
            assert_eq!(ra, u32::from(p::dummy_analog(b, axis)), "axis {axis}");
        }
    }
    for v in [0u8, 3, 0x7f, 0x80, 0xfe, 0xff, 2] {
        eax(a, SET_TO_PAD_GET_CTRL, &[u32::from(v) | 0x5500]);
        p::set_get_ctrl(b, v);
        set(a, b, p::PRINT_FLAG, v);
        eax(a, TO_DEBUG_ADD_PAD_PRINT_FLAG, &[]);
        p::add_print_flag(b);
    }
    same_memory("pads", a, b);
}

const TO_BTL_CLOSE_HELP_WIN: u32 = 0x0089_0eb0;
const TO_BTL_INIT_ANM: u32 = 0x008a_0cd0;
const TO_BTL_CLOSE_SIGN_WINDOW: u32 = 0x0089_0ee0;
const TO_BTL_CLOSE_SIMPLE_HELP_MES: u32 = 0x0089_0ef0;
const TO_GET_BTL_WINDOW_CONTROL_ENABLE: u32 = 0x0089_b590;
const TO_BTL_SET_CUR_POS: u32 = 0x0089_99f0;
const TO_BTL_SET_LULU_LIMIT_TBL: u32 = 0x0089_a2f0;
const TO_BTL_SET_TARGET_CALLBACK: u32 = 0x008a_1090;

fn battle_windows(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    set(a, b, t::HELP_WIN, 1);
    set(a, b, t::INIT_ANM, 0);
    set(a, b, t::SIGN_WINDOW, rng.next());
    set(a, b, t::SIMPLE_HELP, rng.next());
    for (sym, rust) in [
        (TO_BTL_CLOSE_HELP_WIN, t::close_help_win as fn(&mut Oracle)),
        (TO_BTL_INIT_ANM, t::init_anm),
        (TO_BTL_CLOSE_SIGN_WINDOW, t::close_sign_window),
        (TO_BTL_CLOSE_SIMPLE_HELP_MES, t::close_simple_help),
    ] {
        eax(a, sym, &[]);
        rust(b);
    }
    assert_eq!(
        eax(a, TO_GET_BTL_WINDOW_CONTROL_ENABLE, &[]),
        t::window_control_enable(b)
    );
    for start in [-2i16, 0, 2, 3, 4, 7] {
        set(a, b, t::CUR_POS_COUNT, start);
        for _ in 0..3 {
            #[allow(clippy::cast_possible_truncation)]
            let xyz = [rng.next() as u16, rng.next() as u16, rng.next() as u16];
            eax(
                a,
                TO_BTL_SET_CUR_POS,
                &xyz.map(|v| u32::from(v) | 0x7700_0000),
            );
            t::set_cur_pos(b, xyz[0], xyz[1], xyz[2]);
        }
    }
    let digits = alloc(a, b, 16);
    for _ in 0..20 {
        #[allow(clippy::cast_possible_truncation)]
        let d = [0u8; 16].map(|_| (rng.next() % 10) as u8 + b'0');
        put(a, b, digits, d);
        eax(a, TO_BTL_SET_LULU_LIMIT_TBL, &[digits]);
        t::set_lulu_limit_table(b, digits);
    }
    // the table as its own digits
    let table = t::LULU_LIMIT.ptr(b).addr;
    eax(a, TO_BTL_SET_LULU_LIMIT_TBL, &[table]);
    t::set_lulu_limit_table(b, table);
    let win = random_block(a, b, rng, 0xa4);
    eax(a, TO_BTL_SET_TARGET_CALLBACK, &[win]);
    t::set_target_callback(b, win);
    same_memory("battle windows", a, b);
}

const TO_SEQ_GET_NUM: u32 = 0x008a_e410;
const TO_SEQ_GET_COLOR: u32 = 0x008a_e360;
const TO_SEQ_GET_POINT: u32 = 0x008a_e450;
const TO_SEQ_GET_RECT: u32 = 0x008a_e4b0;

/// A menu scene block with one parameter per name (four values each).
fn scene_block(a: &mut Oracle, b: &mut Oracle, names: &[&[u8]], rng: &mut Xorshift) -> u32 {
    let mut v = vec![0u8; 0x80];
    let mut buckets: Vec<Vec<(usize, usize)>> = vec![Vec::new(); 16];
    let mut name_at = Vec::new();
    for name in names {
        name_at.push(v.len());
        v.extend_from_slice(name);
        v.push(0);
    }
    for (k, name) in names.iter().enumerate() {
        buckets[ffx_menu::scene::bucket(name) as usize].push((k, name_at[k]));
    }
    for (h, list) in buckets.iter().enumerate() {
        if list.is_empty() {
            continue;
        }
        while !v.len().is_multiple_of(4) {
            v.push(0);
        }
        let off = u32::try_from(v.len()).unwrap();
        v[4 * h..4 * h + 4].copy_from_slice(&off.to_le_bytes());
        v.extend_from_slice(&u32::try_from(list.len()).unwrap().to_le_bytes());
        for &(_, at) in list {
            let word = (4 << 24) | u32::try_from(at).unwrap();
            v.extend_from_slice(&word.to_le_bytes());
            for _ in 0..4 {
                v.extend_from_slice(&rng.next().to_le_bytes());
            }
        }
    }
    let p = alloc(a, b, v.len());
    a.write(p, &v).unwrap();
    b.write(p, &v).unwrap();
    p
}

fn scene_params(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_menu::scene::bridge as ms;
    use scene::bridge as s;
    let names: [&[u8]; 4] = [b"pos", b"color", b"rect", b"num"];
    let block = scene_block(a, b, &names, rng);
    let slot = ms::SCENE_DATA.ptr(b).addr + 4 * 3;
    put(a, b, slot, block);
    let outs = alloc(a, b, 16);
    let mut queries: Vec<u32> = Vec::new();
    for name in names.iter().copied().chain([&b"absent"[..]]) {
        let mut z = name.to_vec();
        z.push(0);
        let q = alloc(a, b, z.len());
        a.write(q, &z).unwrap();
        b.write(q, &z).unwrap();
        queries.push(q);
    }
    for &q in &queries {
        for i in [3i32, 0, 7] {
            let iu = i.cast_unsigned();
            put(a, b, outs, [0x1111_1111u32; 4]);
            eax(a, TO_SEQ_GET_NUM, &[iu, q, outs]);
            s::get_num(
                b,
                i,
                q,
                outs,
                ms::tk_menu_get_param_handle,
                ms::tk_menu_get_param_offset,
            );
            eax(a, TO_SEQ_GET_COLOR, &[iu, q, outs + 4]);
            s::get_color(
                b,
                i,
                q,
                outs + 4,
                ms::tk_menu_get_param_handle,
                ms::tk_menu_get_param_offset,
            );
            eax(a, TO_SEQ_GET_POINT, &[iu, q, outs + 8, outs + 10]);
            s::get_point(
                b,
                i,
                q,
                [outs + 8, outs + 10],
                ms::tk_menu_get_param_handle,
                ms::tk_menu_get_param_offset,
            );
            same_memory("TOSeqGet*", a, b);
            let rect = [outs, outs + 2, outs + 12, outs + 2];
            eax(
                a,
                TO_SEQ_GET_RECT,
                &[iu, q, rect[0], rect[1], rect[2], rect[3]],
            );
            s::get_rect(
                b,
                i,
                q,
                rect,
                ms::tk_menu_get_param_handle,
                ms::tk_menu_get_param_offset,
            );
            same_memory("TOSeqGetRect", a, b);
        }
    }
}

const TO_MENU_GET_FIRST_READ_END_FLAG: u32 = 0x008a_8e50;
const TO_MENU_HELP_SET_EXT_MODE: u32 = 0x008b_ef10;
const TO_MENU_GET_ABILITYMAP_WORK_ADRS: u32 = 0x008b_ea30;
const TO_AMAP_GET_ITEM_LIST_MAX: u32 = 0x008c_8c30;
const TO_AMAP_GET_ITEM_TYPE: u32 = 0x008c_6710;
const IS_IN_TK_MENU_PAUSE: u32 = 0x008a_8e10;
const SET_INVOKED_FROM_SELECT_CURSOR: u32 = 0x008e_89c0;
const TO_GET_MESSAGE_WINDOW_MODE: u32 = 0x008a_c7b0;
const TO_GET_FFX_TO_ALBHED_TBL_ADRS: u32 = 0x0090_6b20;
const TO_GET_FONT_WIDTH_TBL_ADRS: u32 = 0x008a_c3d0;
const TO_GET_LINE_FEED_HEIGHT: u32 = 0x008b_7c60;
const TO_FFX_TO_SJIS: u32 = 0x008b_6200;
const TO_MENU_GET_GET_IMPORTANT_NAME: u32 = 0x008d_dee0;
const TO_MENU_GET_GET_IMPORTANT_HELP: u32 = 0x008d_dea0;
const TO_INIT_ADD_PKT: u32 = 0x0090_4650;
const TO_SET_ADD_PKT: u32 = 0x008f_ae10;
const TO_SET_PKT_BUFF_ADRS: u32 = 0x0090_47c0;
const TO_GET_SCREEN_WH: u32 = 0x008f_a900;

fn menu_state(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use menu::bridge as mb;
    use pkt::bridge as pk;
    for round in 0..3 {
        if round > 0 {
            #[allow(clippy::cast_possible_truncation)]
            set(a, b, mb::FIRST_READ_END, (rng.next() as u8).cast_signed());
            set(a, b, mb::AMAP_ITEM_LIST_MAX, rng.next());
            set(a, b, mb::PAUSE_ENTERED, rng.next());
            set(a, b, mb::MESSAGE_WINDOW_MODE, rng.next());
        }
        assert_eq!(
            eax(a, TO_MENU_GET_FIRST_READ_END_FLAG, &[]),
            mb::first_read_end(b).cast_unsigned()
        );
        assert_eq!(
            eax(a, TO_MENU_GET_ABILITYMAP_WORK_ADRS, &[]),
            mb::abilitymap_work(b)
        );
        assert_eq!(
            eax(a, TO_AMAP_GET_ITEM_LIST_MAX, &[]),
            mb::amap_item_list_max(b)
        );
        assert_eq!(eax(a, IS_IN_TK_MENU_PAUSE, &[]), mb::is_in_pause(b));
        assert_eq!(
            eax(a, TO_GET_MESSAGE_WINDOW_MODE, &[]),
            mb::message_window_mode(b)
        );
        assert_eq!(
            eax(a, TO_GET_FFX_TO_ALBHED_TBL_ADRS, &[]),
            mb::albhed_table(b)
        );
        assert_eq!(eax(a, TO_GET_LINE_FEED_HEIGHT, &[]), stub::LINE_FEED_HEIGHT);
        for i in 0..64 {
            assert_eq!(
                eax(a, TO_AMAP_GET_ITEM_TYPE, &[i]),
                u32::from(mb::amap_item_type(b, i.cast_signed()))
            );
        }
        for f in 0..6 {
            assert_eq!(
                eax(a, TO_GET_FONT_WIDTH_TBL_ADRS, &[f]),
                mb::font_width_table(b, f.cast_signed())
            );
        }
    }
    let (mode, arg) = (rng.next(), rng.next());
    eax(a, TO_MENU_HELP_SET_EXT_MODE, &[mode, arg]);
    mb::help_set_ext_mode(b, mode, arg);
    eax(a, SET_INVOKED_FROM_SELECT_CURSOR, &[mode]);
    mb::set_invoked_from_select_cursor(b, mode);
    let out = random_block(a, b, rng, 8);
    eax(a, TO_FFX_TO_SJIS, &[0, out]);
    mb::ffx_to_sjis(b, out);
    eax(a, TO_SET_ADD_PKT, &[]);
    pk::set_add_pkt(b);
    eax(a, TO_INIT_ADD_PKT, &[]);
    pk::init_add_pkt(b);
    let (base, off1, off2) = (rng.next(), rng.next(), rng.next());
    eax(a, TO_SET_PKT_BUFF_ADRS, &[base, off1, off2]);
    pk::set_pkt_buff(b, base, off1, off2);
    for (wp, hp) in [(out, out + 4), (out + 2, out + 2)] {
        eax(a, TO_GET_SCREEN_WH, &[wp, hp]);
        pk::screen_wh(b, wp, hp);
    }
    same_memory("menu state", a, b);

    // key item texts: only where the kernel's key item table is loaded
    if ffx_battle::save::bridge::KERNEL_IMPORTANT.get(b) != 0 {
        let scratch = alloc(a, b, 4);
        for id in (0..0x60).chain([0x2000, 0xa005]) {
            for (sym, field) in [
                (TO_MENU_GET_GET_IMPORTANT_NAME, 0),
                (TO_MENU_GET_GET_IMPORTANT_HELP, 8),
            ] {
                let ra = eax(a, sym, &[id]);
                let hira = ffx_battle::save::bridge::ms_get_save_config_hiragana(b);
                let rec = ffx_battle::save::bridge::ms_important_ptr(b, id, scratch);
                let strings: u32 = Mem::read(b, scratch);
                put(a, b, scratch, strings);
                assert_eq!(
                    ra,
                    mb::important_text(b, field, hira, rec, strings),
                    "{sym:#x} {id:#x}"
                );
            }
        }
        same_memory("key item texts", a, b);
    }
}

const FIXUP_READ_LEB128: u32 = 0x0060_ac00;
const FIXUP_SET_OBJECT: u32 = 0x0060_ae40;
const FIXUP_SET_ARRAY_OBJECT: u32 = 0x0060_ae30;
const MD5_DIGEST_LESS: u32 = 0x0061_d6e0;
const VBF_MAP_LOOKUP: u32 = 0x0061_d140;

/// A balanced search tree of sorted digests under `head` (see
/// `vbf::bridge::map_lookup`): its root.
fn digest_tree(a: &mut Oracle, b: &mut Oracle, head: u32, keys: &[[u32; 4]]) -> u32 {
    if keys.is_empty() {
        return head;
    }
    let mid = keys.len() / 2;
    let node = alloc(a, b, 0x30);
    let l = digest_tree(a, b, head, &keys[..mid]);
    let r = digest_tree(a, b, head, &keys[mid + 1..]);
    put(a, b, node, l);
    put(a, b, node + 4, r);
    put(a, b, node + 0x10, keys[mid]);
    node
}

#[test]
fn fixup_reader_and_digest_map_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    let mut rng = Xorshift(0x1eb1_2800);
    let cursor = alloc(a, b, 4);
    let data = alloc(a, b, 64);
    // null cursor, then sequences of 1 to 40 bytes (shift counts past 255)
    put(a, b, cursor, 0u32);
    assert_eq!(
        eax(a, FIXUP_READ_LEB128, &[cursor]),
        fixup::bridge::read_leb128(b, cursor)
    );
    for n in 0..400u32 {
        let len = (n % 40) + 1;
        for k in 0..len {
            #[allow(clippy::cast_possible_truncation)] // a random byte
            let byte = rng.next() as u8 | 0x80;
            let byte = if k + 1 == len { byte & 0x7f } else { byte };
            put(a, b, data + k, byte);
        }
        put(a, b, cursor, data);
        let ra = eax(a, FIXUP_READ_LEB128, &[cursor]);
        assert_eq!(
            ra,
            fixup::bridge::read_leb128(b, cursor),
            "LEB128 of {len} bytes"
        );
    }
    for sym in [FIXUP_SET_OBJECT, FIXUP_SET_ARRAY_OBJECT] {
        let v = rng.next();
        eax(a, sym, &[data, v]);
        fixup::bridge::set_object(b, data, v);
    }
    same_memory("fixup", a, b);

    // digests sharing halves, words and signs, compared both ways
    let pick = |rng: &mut Xorshift| {
        [0, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX, rng.next()][(rng.next() % 6) as usize]
    };
    let (first, second) = (alloc(a, b, 16), alloc(a, b, 16));
    for _ in 0..3000 {
        let one: [u32; 4] = [0; 4].map(|_| pick(&mut rng));
        let mut other = one;
        other[(rng.next() % 4) as usize] = pick(&mut rng);
        put(a, b, first, one);
        put(a, b, second, other);
        for (lhs, rhs) in [(first, second), (second, first), (first, first)] {
            let ra = call(a, MD5_DIGEST_LESS, Conv::Thiscall(lhs), &[rhs]).eax & 0xff;
            assert_eq!(
                ra,
                u32::from(vbf::bridge::less(b, lhs, rhs)),
                "{one:x?} {other:x?}"
            );
        }
    }

    // a search tree of 200 digests in the game's order: head (end marker,
    // root at +0xc), nodes left +0, right +4, key +0x10
    let head = alloc(a, b, 0x30);
    let mut keys: Vec<[u32; 4]> = (0..200).map(|_| [0; 4].map(|_| pick(&mut rng))).collect();
    keys.sort_by(vbf::cmp);
    keys.dedup();
    let root = digest_tree(a, b, head, &keys);
    put(a, b, head + 0xc, root);
    let probe = alloc(a, b, 16);
    for k in 0..400 {
        let key = if k % 2 == 0 {
            keys[k / 2 % keys.len()]
        } else {
            [0; 4].map(|_| pick(&mut rng))
        };
        put(a, b, probe, key);
        let ra = call(a, VBF_MAP_LOOKUP, Conv::Thiscall(head), &[probe]).eax;
        assert_eq!(ra, vbf::bridge::map_lookup(b, head, probe), "{key:x?}");
    }
    let empty = alloc(a, b, 0x30);
    put(a, b, empty + 0xc, empty);
    let ra = call(a, VBF_MAP_LOOKUP, Conv::Thiscall(empty), &[probe]).eax;
    assert_eq!(ra, vbf::bridge::map_lookup(b, empty, probe));
    same_memory("digest map", a, b);
}

const ASSET_LIST_READER_GET_ASSET_PATH: u32 = 0x006f_41b0;
const ASSET_LIST_READER_IS_MODEL: u32 = 0x006f_46b0;
const ASSET_LIST_READER_GET_SYMBOL_BY_ID: u32 = 0x006f_42d0;
const ABMAP_MANAGER_TO_DESTROY: u32 = 0x0068_57b0;
const CHR_RESOURCE_CACHE_MANAGER_IS_CHR_DELAYED_RELEASE: u32 = 0x0069_b7b0;
const ASYNC_LOAD_NEW_CHARACTER: u32 = 0x0078_2f80;
const LOADING_SCREEN_SET_PARAMETER_MANUALLY: u32 = 0x006e_4ff0;
const ABM_CALC_CHR_MOVE_CUR_MOVE_CHECK: u32 = 0x00a4_5000;

/// An MSVC `std::map<id, std::string>` of these IDs (balanced; names
/// alternately inline and on the heap): its head.
fn symbol_map(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, ids: &[u32]) -> u32 {
    fn build(
        a: &mut Oracle,
        b: &mut Oracle,
        rng: &mut Xorshift,
        head: u32,
        parent: u32,
        ids: &[u32],
    ) -> u32 {
        if ids.is_empty() {
            return head;
        }
        let mid = ids.len() / 2;
        let node = random_block(a, b, rng, 0x30);
        put(a, b, node + 4, parent);
        put(a, b, node + 0xd, 0u8);
        put(a, b, node + 0x28, ids[mid]);
        put(
            a,
            b,
            node + 0x24,
            if ids[mid].is_multiple_of(2) {
                15u32
            } else {
                31
            },
        );
        let l = build(a, b, rng, head, node, &ids[..mid]);
        let r = build(a, b, rng, head, node, &ids[mid + 1..]);
        put(a, b, node, l);
        put(a, b, node + 8, r);
        node
    }
    let head = alloc(a, b, 0x30);
    put(a, b, head + 0xd, 1u8);
    let root = build(a, b, rng, head, head, ids);
    let mut leftmost = root;
    while leftmost != head && Mem::read::<u32>(b, leftmost) != head {
        leftmost = Mem::read(b, leftmost);
    }
    put(a, b, head, if root == head { head } else { leftmost });
    put(a, b, head + 4, root);
    head
}

fn asset_lists(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use instance::bridge as i;
    // an asset list: models at +8..+0xc, 5 asset records at +0x14..+0x18
    let reader = alloc(a, b, 0x20);
    let models = random_block(a, b, rng, 16);
    put(a, b, reader + 8, models);
    put(a, b, reader + 0xc, models + 16);
    let assets = random_block(a, b, rng, 5 * 24);
    for k in 0..5u32 {
        put(
            a,
            b,
            assets + 24 * k + 0x14,
            if k % 2 == 0 { 15u32 } else { 16 },
        );
    }
    put(a, b, reader + 0x14, assets);
    put(a, b, reader + 0x18, assets + 5 * 24);
    for k in [-1i32, 0, 1, 2, 3, 4, 5, 100] {
        let ra = call(
            a,
            ASSET_LIST_READER_GET_ASSET_PATH,
            Conv::Thiscall(reader),
            &[k.cast_unsigned()],
        )
        .eax;
        assert_eq!(ra, i::asset_path(b, reader, k), "getAssetPath({k})");
    }
    let present: u32 = Mem::read(b, models + 8);
    for id in [present, rng.next(), Mem::read(b, models)] {
        let ra = call(a, ASSET_LIST_READER_IS_MODEL, Conv::Thiscall(reader), &[id]).eax & 0xff;
        assert_eq!(ra, u32::from(i::is_model(b, reader, id)));
    }
    for n in [0usize, 1, 2, 7, 20] {
        let ids: Vec<u32> = (0..n).map(|k| 10 * u32::try_from(k).unwrap() + 3).collect();
        let head = symbol_map(a, b, rng, &ids);
        let this = alloc(a, b, 4);
        put(a, b, this, head);
        for id in ids.iter().copied().chain([0, 5, 1000]) {
            let ra = call(
                a,
                ASSET_LIST_READER_GET_SYMBOL_BY_ID,
                Conv::Thiscall(this),
                &[id],
            )
            .eax;
            assert_eq!(
                ra,
                i::symbol_by_id(b, this, id),
                "getSymbolById({id}) of {n}"
            );
        }
    }
    same_memory("asset lists", a, b);
}

fn managers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use instance::bridge as i;
    let obj = random_block(a, b, rng, 0x40);
    call(a, ABMAP_MANAGER_TO_DESTROY, Conv::Thiscall(obj), &[]);
    i::abmap_to_destroy(b, obj);
    for v in [2u32, 0, 3] {
        put(a, b, obj, v);
        let ra = call(
            a,
            CHR_RESOURCE_CACHE_MANAGER_IS_CHR_DELAYED_RELEASE,
            Conv::Thiscall(obj),
            &[],
        )
        .eax;
        assert_eq!(ra, i::is_chr_delayed_release(b, obj));
        put(a, b, obj + 0x22, v);
        assert_eq!(
            eax(a, ABM_CALC_CHR_MOVE_CUR_MOVE_CHECK, &[obj]),
            i::chr_move_cur_move_check(b, obj)
        );
    }
    for v in [-1i32, 0, 5, i32::MAX] {
        set(a, b, i::NEW_CHARACTERS, v);
        assert_eq!(
            eax(a, ASYNC_LOAD_NEW_CHARACTER, &[]),
            i::async_load_new_character(b)
        );
    }
    let params = alloc(a, b, 12);
    put(
        a,
        b,
        params,
        [1.5f32.to_bits(), rng.next(), f32::NAN.to_bits()],
    );
    for mask in 0..8u32 {
        let arg = |k: u32| {
            if mask >> k & 1 == 1 {
                params + 4 * k
            } else {
                0
            }
        };
        call(
            a,
            LOADING_SCREEN_SET_PARAMETER_MANUALLY,
            Conv::Thiscall(obj),
            &[arg(0), arg(1), arg(2)],
        );
        i::loading_screen_set_parameter(b, obj, arg(0), arg(1), arg(2));
    }
    same_memory("managers", a, b);
}

const USE_EXTENDED_ALPHA: u32 = 0x0071_e460;
const USE_SHADER_POINT_LIGHT: u32 = 0x0071_e6f0;
const DEPTH_BIAS_HOOKS_CLEAR_DEPTH_BIAS: u32 = 0x006e_40b0;
const JOB_SCHEDULE_CLEAR_FILL_BUFFER_REQUEST: u32 = 0x0071_ecd0;

fn render_switches(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use render::bridge as r;
    for on in [1, 0, 7] {
        eax(a, USE_EXTENDED_ALPHA, &[on]);
        r::use_extended_alpha(b, on);
    }
    for id in [0x207, 0x206, u32::MAX] {
        set(a, b, instance::bridge::MAGIC_ID, id);
        assert_eq!(
            eax(a, USE_SHADER_POINT_LIGHT, &[]),
            r::use_shader_point_light(b)
        );
    }
    for dirty in [1u8, 0, 1] {
        let t = r::DEPTH_BIAS.ptr(b).addr;
        for k in 0..r::DEPTH_BIAS_ENTRIES * 3 {
            put(a, b, t + 4 * k, rng.next());
        }
        set(a, b, r::DEPTH_BIAS_COUNT, rng.next());
        set(a, b, r::DEPTH_BIAS_DIRTY, dirty);
        eax(a, DEPTH_BIAS_HOOKS_CLEAR_DEPTH_BIAS, &[]);
        r::clear_depth_bias(b);
        same_memory("clearDepthBias", a, b);
    }
    set(a, b, r::FILL_REQUESTS, [0u32; 12].map(|_| rng.next()));
    eax(a, JOB_SCHEDULE_CLEAR_FILL_BUFFER_REQUEST, &[]);
    r::clear_fill_buffer_request(b);
    same_memory("render switches", a, b);
}

const SAVE_DATA_IS_BUSY: u32 = 0x0064_8040;
const SAVE_DATA_SET_SCREEN_STATE: u32 = 0x0064_86e0;
const SAVE_DATA_IS_OPERATION_CANCELED: u32 = 0x0064_80c0;
const SAVE_DATA_IS_LOADING_ALBHED: u32 = 0x0064_8070;
const SAVE_DATA_SET_ALBHED_FADE: u32 = 0x0064_85e0;
const SET_LOCAL_DATA_SLOT_AND_TIME: u32 = 0x0064_8fa0;
const SAVE_DATA_GET_CRC: u32 = 0x0064_7e80;
const SAVE_DATA_WRITE_CRC: u32 = 0x0064_8f20;
const SAVE_DATA_CHECK_CRC: u32 = 0x0064_7d70;

fn save_crcs(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_menu::tk::bridge::tk_crc;
    use save::bridge as s;
    let mut manager = s::MANAGER.get(b);
    if manager == 0 {
        manager = random_block(a, b, rng, 0x400);
        put(a, b, manager + 8, 0u32);
        set(a, b, s::MANAGER, manager);
    }
    // a save buffer of random bytes for the manager to refer to
    let buf = random_block(a, b, rng, 0x6500);
    put(a, b, manager + 0x350, buf);
    for v in [0u32, 1, 9] {
        put(a, b, manager + 8, v);
        put(a, b, manager + 0x358, v);
        put(a, b, manager + 0x360, v);
        assert_eq!(eax(a, SAVE_DATA_IS_BUSY, &[]), s::is_busy_now(b));
        assert_eq!(
            eax(a, SAVE_DATA_IS_OPERATION_CANCELED, &[]),
            s::is_operation_canceled(b)
        );
        assert_eq!(
            eax(a, SAVE_DATA_IS_LOADING_ALBHED, &[]),
            s::is_loading_albhed(b)
        );
    }
    for state in [3u32, 0, 1] {
        eax(a, SAVE_DATA_SET_SCREEN_STATE, &[state]);
        s::set_screen_state(b, state);
    }
    let v = rng.next();
    eax(a, SAVE_DATA_SET_ALBHED_FADE, &[v]);
    s::set_albhed_fade(b, v);
    let (slot, t1, t2) = (rng.next(), rng.next(), rng.next());
    eax(a, SET_LOCAL_DATA_SLOT_AND_TIME, &[0, slot, t1, t2]);
    #[allow(clippy::cast_possible_truncation)] // the low half
    s::set_local_data_slot_and_time(b, slot as u16, t1, t2);
    same_memory("save manager", a, b);

    // CRCs: random (no match), written (match), then a changed byte, with
    // the check on and off
    for case in 0..6 {
        if case == 0 || case == 3 {
            let fresh = random_block(a, b, rng, 0x6500);
            put(a, b, manager + 0x350, fresh);
        }
        let buf: u32 = Mem::read(b, manager + 0x350);
        match case {
            1 | 4 => {
                assert_eq!(
                    eax(a, SAVE_DATA_WRITE_CRC, &[buf]),
                    s::write_crc(b, buf, tk_crc)
                );
            }
            2 | 5 => put(a, b, buf + 0x1000, rng.next()),
            _ => {}
        }
        set(a, b, s::CRC_CHECK_OFF, u32::from(case >= 3 && case != 4));
        assert_eq!(
            eax(a, SAVE_DATA_CHECK_CRC, &[]),
            s::check_crc(b, tk_crc),
            "check case {case}"
        );
        let ra = eax(a, SAVE_DATA_GET_CRC, &[buf]) & 0xffff;
        assert_eq!(ra, u32::from(s::get_crc(b, buf, tk_crc)), "get case {case}");
        same_memory("save CRCs", a, b);
    }
}

fn round2(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    windows_and_messages(a, b, rng);
    pads(a, b, rng);
    battle_windows(a, b, rng);
    scene_params(a, b, rng);
    menu_state(a, b, rng);
    asset_lists(a, b, rng);
    managers(a, b, rng);
    render_switches(a, b, rng);
    save_crcs(a, b, rng);
}

#[test]
fn to_layer_managers_and_save_crcs_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round2(&mut a, &mut b, &mut Xorshift(0x2002_0001));
}

#[test]
fn to_layer_managers_and_save_crcs_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round2(&mut a, &mut b, &mut Xorshift(0x2002_0002));
}

const FIXUP_WRITE_LEB128: u32 = 0x0060_aba0;
const TROPHY_MANAGER_INSTALL: u32 = 0x0059_e520;
const TREE_ITERATOR_INC: [u32; 4] = [0x0063_52b0, 0x0063_5300, 0x0063_5350, 0x0069_9ee0];
const TO_DRAW_SCISSOR_XYWH: u32 = 0x008f_9230;
const KERNEL_GET_ITEM_NAME: u32 = 0x0077_67a0;

fn leb128_and_stubs(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let cursor = alloc(a, b, 4);
    let data = alloc(a, b, 0x40);
    put(a, b, cursor, 0u32);
    eax(a, FIXUP_WRITE_LEB128, &[rng.next(), cursor]);
    fixup::bridge::write_leb128(b, rng.next(), cursor);
    for n in 0..2000u32 {
        let v = match n % 4 {
            0 => rng.next() >> (rng.next() % 32),
            1 => [0, 1, 0x7f, 0x80, 0x3fff, 0x4000, u32::MAX][(n / 4 % 7) as usize],
            _ => rng.next(),
        };
        put(a, b, cursor, data + n % 16);
        eax(a, FIXUP_WRITE_LEB128, &[v, cursor]);
        fixup::bridge::write_leb128(b, v, cursor);
        same_memory("Fixup_WriteLeb128", a, b);
    }
    for _ in 0..8 {
        let ra = call(
            a,
            TROPHY_MANAGER_INSTALL,
            Conv::Thiscall(rng.next()),
            &[rng.next()],
        )
        .eax;
        assert_eq!(ra, stub::TROPHY_INSTALL);
    }
    for n in 0..2000u32 {
        let pick = |rng: &mut Xorshift| -> i32 {
            match rng.next() % 4 {
                0 => rng.next().cast_signed(),
                1 => (rng.next() % 2000).cast_signed() - 1000,
                2 => (rng.next() >> (rng.next() % 32)).cast_signed(),
                _ => [i32::MIN, i32::MAX, -1, 0, 0x0100_0001][(rng.next() % 5) as usize],
            }
        };
        let r = [pick(rng), pick(rng), pick(rng), pick(rng)];
        let args = r.map(i32::cast_unsigned);
        let ra = eax(a, TO_DRAW_SCISSOR_XYWH, &args);
        assert_eq!(
            ra,
            pkt::bridge::draw_scissor_xywh(b, r[0], r[1], r[2], r[3]),
            "{r:?}"
        );
        if n % 100 == 0 {
            same_memory("TODrawScissorXYWH", a, b);
        }
    }
    same_memory("TODrawScissorXYWH", a, b);
}

/// An MSVC `std::map` tree of `n` nodes (left +0, parent +4, right +8,
/// `_Isnil` +0xd), balanced, under a head (`_Isnil` 1, root as its
/// parent): the head and the nodes in order.
fn map_tree(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, n: usize) -> (u32, Vec<u32>) {
    fn build(a: &mut Oracle, b: &mut Oracle, head: u32, parent: u32, nodes: &[u32]) -> u32 {
        if nodes.is_empty() {
            return head;
        }
        let mid = nodes.len() / 2;
        let node = nodes[mid];
        let l = build(a, b, head, node, &nodes[..mid]);
        let r = build(a, b, head, node, &nodes[mid + 1..]);
        put(a, b, node, l);
        put(a, b, node + 4, parent);
        put(a, b, node + 8, r);
        node
    }
    let head = alloc(a, b, 0x10);
    let nodes: Vec<u32> = (0..n).map(|_| alloc(a, b, 0x10)).collect();
    for &x in &nodes {
        put(a, b, x + 0xc, rng.next() & 0xff);
        put(a, b, x + 0xd, 0u8);
    }
    let root = build(a, b, head, head, &nodes);
    put(a, b, head, nodes.first().copied().unwrap_or(head));
    put(a, b, head + 4, root);
    put(a, b, head + 8, nodes.last().copied().unwrap_or(head));
    put(a, b, head + 0xc, 0x0101u16);
    (head, nodes)
}

fn trees_and_names(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let it = alloc(a, b, 4);
    for n in [0usize, 1, 2, 3, 7, 30, 64] {
        let (head, nodes) = map_tree(a, b, rng, n);
        for (k, &sym) in TREE_ITERATOR_INC.iter().enumerate() {
            let start = if nodes.is_empty() {
                head
            } else {
                nodes[k % nodes.len()]
            };
            put(a, b, it, start);
            for _ in 0..=nodes.len() + 1 {
                let ra = call(a, sym, Conv::Fastcall, &[it]).eax;
                assert_eq!(ra, stl::bridge::tree_iterator_inc(b, it), "{sym:#x} {n}");
                assert_eq!(
                    Mem::read::<u32>(a, it),
                    Mem::read::<u32>(b, it),
                    "{sym:#x} {n}"
                );
            }
            assert_eq!(Mem::read::<u32>(a, it), head);
        }
        same_memory("tree iterators", a, b);
    }

    // an item table where there is none: two segments
    if ffx_kernel::bridge::ITEM.get(a) == 0 {
        let t = alloc(a, b, 0x400);
        put(a, b, t, 2u16);
        put(a, b, t + 8, [0x0001u16, 0x0010, 8, 0x200]);
        put(a, b, t + 16, 0x40u32);
        put(a, b, t + 20, [0x0020u16, 0x0030, 4, 0x280]);
        put(a, b, t + 28, 0xc0u32);
        for k in 0..0x100u32 {
            put(
                a,
                b,
                t + 0x40 + 2 * k,
                u16::try_from(rng.next() & 0x3ff).unwrap(),
            );
        }
        set(a, b, ffx_kernel::bridge::ITEM, t);
    }
    for n in 0..600u32 {
        let id = match n % 3 {
            0 => 0,
            1 => rng.next() & 0x3f,
            _ => rng.next(),
        };
        let ra = eax(a, KERNEL_GET_ITEM_NAME, &[id]);
        assert_eq!(ra, names::bridge::item_name(b, id), "item {id:#x}");
    }
    same_memory("item names", a, b);
}

/// Getters and setters of round 3: managers, configuration bytes, the
/// HDD state, message-window slots, scissors.
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)] // one pass over many small functions; random bytes
fn small_state(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use cd::bridge as c;
    use config::bridge as k;
    use instance::bridge as i;
    use window::bridge as w;
    for round in 0..30 {
        if round > 0 {
            set(a, b, i::MORPH_MANAGER, rng.next());
            set(a, b, i::TROPHY_MANAGER, rng.next());
            set(a, b, k::FFX_VIDEO_MODE, rng.next() as i8);
            set(a, b, k::DEF_DRAW_ENV2, rng.next() as i8);
            set(a, b, c::HDD_STATUS, rng.next() as i8);
            set(
                a,
                b,
                c::HDD_ERR,
                if round % 2 == 0 { 0 } else { rng.next() as i8 },
            );
            set(a, b, c::HDD_STATE, [rng.next(), rng.next()]);
        }
        assert_eq!(eax(a, 0x0063_1300, &[]), i::input_manager(b));
        assert_eq!(eax(a, 0x006d_c8a0, &[]), i::morph_manager(b));
        assert_eq!(eax(a, 0x0067_ac90, &[]), i::trophy_manager(b));
        assert_eq!(
            eax(a, 0x0088_7c40, &[]),
            k::ffx_video_mode(b).cast_unsigned()
        );
        assert_eq!(
            eax(a, 0x0088_7c60, &[]),
            k::gs_set_def_draw_env2(b).cast_unsigned()
        );
        assert_eq!(eax(a, 0x0077_7e80, &[]), c::hdd_err(b).cast_unsigned());
        assert_eq!(eax(a, 0x0077_7e90, &[]), c::hdd_status(b).cast_unsigned());
        eax(a, 0x0077_7420, &[]);
        c::hdd_state_clear(b);
        eax(a, 0x0077_7940, &[]);
        c::hdd_set_pass_names(b);
        let r = call(a, 0x008f_a7b0, Conv::Cdecl, &[]);
        assert_eq!(
            u64::from(r.edx) << 32 | u64::from(r.eax),
            pkt::bridge::GS_WORD_008FA7B0
        );
        same_memory("small state", a, b);
    }

    // the input manager's pads: 0x12 slots, some null, key states
    let manager = random_block(a, b, rng, 0x2c0);
    let table = random_block(a, b, rng, 0x100);
    put(a, b, manager + 0x2b8, table);
    for pad in 0..0x12u32 {
        let p = if pad % 5 == 3 {
            0
        } else {
            random_block(a, b, rng, 0x40)
        };
        put(a, b, table + 0x38 + 4 * pad, p);
        if p != 0 {
            for key in 0..0x10 {
                put(a, b, p + 0x18 + key, u8::from(rng.next().is_multiple_of(3)));
            }
        }
    }
    for n in 0..800u32 {
        let key = match n % 4 {
            0 => rng.next(),
            _ => rng.next() % 0x20,
        };
        let pad = if n % 7 == 0 {
            rng.next()
        } else {
            rng.next() % 0x14
        };
        let ra = call(a, 0x0062_9ca0, Conv::Thiscall(manager), &[key, pad]).eax;
        assert_eq!(
            ra,
            i::key_down(b, manager, key, pad),
            "key {key:#x} pad {pad:#x}"
        );
        let obj = random_block(a, b, rng, 0xb0);
        if n % 2 == 0 {
            put(a, b, obj + 0xa8, 0u32);
        }
        let ra = call(a, 0x0063_1320, Conv::Fastcall, &[obj]).eax & 0xff;
        assert_eq!(ra, u32::from(i::vibration_flag(b, obj)));
    }

    // the message-window slots: the game's objects where there are any
    for k in 0..8 {
        let at = w::MSG_WINDOW_SLOTS.ptr(b).addr + 4 * k;
        let cur = Mem::read::<u32>(b, at);
        if cur == 0 || !mapped(a, cur + 8) {
            let obj = random_block(a, b, rng, 0x10);
            put(a, b, at, obj);
        }
    }
    for n in 0..64u32 {
        let win = (n % 8).cast_signed();
        let ra = eax(a, 0x008a_f7d0, &[win.cast_unsigned()]);
        assert_eq!(ra, w::set_message_window_active(b, win));
        let v = rng.next();
        let ra = eax(a, 0x008a_fb90, &[win.cast_unsigned(), v]);
        assert_eq!(ra, w::set_message_window_value(b, win, v));
    }
    same_memory("message-window slots", a, b);
    for k in 0..4u32 {
        let win = random_block(a, b, rng, 0x80);
        put(a, b, win, u16::from(k != 2));
        set_entry(a, b, w::WINDOWS, k, win);
        let m = random_block(a, b, rng, 0x120);
        put(a, b, m, u16::from(k != 1));
        set_entry(a, b, w::MSG_WINDOWS, k, m);
        eax(a, 0x008a_b8f0, &[k]);
        w::close_message_window(b, k.cast_signed());
    }
    same_memory("TOCloseMessageWindow", a, b);

    // scissors and the save-player thunk
    for _ in 0..300 {
        let r: [i32; 4] = std::array::from_fn(|_| match rng.next() % 3 {
            0 => rng.next().cast_signed(),
            _ => (rng.next() % 4000).cast_signed() - 2000,
        });
        let args = r.map(i32::cast_unsigned);
        eax(a, 0x008f_dee0, &args);
        pkt::bridge::make_pkt_scissor(b, r[0], r[1], r[2], r[3]);
        eax(a, 0x008f_91f0, &args);
        pkt::bridge::make_pkt_scissor(b, r[0], r[1], r[2], r[3]);
        let chr = rng.next() % 0x20;
        assert_eq!(
            eax(a, 0x0077_6f50, &[chr]),
            names::bridge::save_player_ptr(b, chr)
        );
    }
    same_memory("scissors", a, b);
}

/// More one-global getters and setters of round 3.
fn small_state2(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use instance::bridge as i;
    use window::bridge as w;
    let getters = [
        (0x0068_98c0, i::TOUCH_MANAGER),
        (0x006f_15b0, i::ACHIEVEMENT_MANAGER),
        (0x009d_a3d0, i::MAGIC_TO_DELETE),
        (0x0077_87b0, i::HDD_WORD_518),
        (0x008a_c2d0, i::TO_GET_WORDS[0]),
        (0x008a_c3f0, i::TO_GET_WORDS[1]),
        (0x008a_c790, i::TO_GET_WORDS[2]),
        (0x008a_cf10, i::TO_GET_WORDS[3]),
    ];
    for round in 0..4 {
        for &(sym, g) in &getters {
            if round > 0 {
                set(a, b, g, rng.next());
            }
            assert_eq!(eax(a, sym, &[]), i::word(b, g), "{sym:#x}");
        }
    }
    let obj = random_block(a, b, rng, 0x100);
    for k in 0..0x28 {
        let ra = call(a, 0x0063_11b0, Conv::Thiscall(obj), &[k]).eax;
        assert_eq!(ra, i::table_60(b, obj, k));
    }
    for n in [
        0,
        1,
        7,
        8,
        9,
        15,
        16,
        17,
        100,
        -1,
        -9,
        -100,
        i32::MIN,
        i32::MAX,
    ] {
        let ra = call(a, 0x006d_b870, Conv::Stdcall, &[n.cast_unsigned(), 7]).eax;
        assert_eq!(ra, i::eighth_at_least_one(n).cast_unsigned(), "{n}");
    }
    for _ in 0..8 {
        let v = rng.next();
        assert_eq!(eax(a, 0x008a_f840, &[v]), w::set_message_window_mode(b, v));
    }
    for n in 0..64u32 {
        // small negatives (near the table), large positives (skipped)
        let k = match n % 3 {
            0 => (rng.next() % 40).cast_signed() - 20,
            1 => rng.next().cast_signed() | 0x7fff_0000,
            _ => (n / 3).cast_signed() % 16,
        };
        let v = rng.next();
        eax(a, 0x0090_0f60, &[k.cast_unsigned(), v]);
        w::menu_set_018762ee(b, k, v);
    }
    same_memory("small state 2", a, b);
    // TOIsMessageStatus3Pressed over made-up message windows 0..3
    for k in 0..4u32 {
        let m = random_block(a, b, rng, 0x120);
        set_entry(a, b, w::MSG_WINDOWS, k, m);
    }
    for n in 0..64u32 {
        let k = n % 4;
        let m = Mem::read::<u32>(b, w::MSG_WINDOWS.ptr(b).addr + 4 * k);
        put(a, b, m + 0x2e, u16::try_from(rng.next() % 5).unwrap());
        set(
            a,
            b,
            w::PAD_BYTE_25D09D4,
            u8::try_from(rng.next() & 0xff).unwrap(),
        );
        let ra = eax(a, 0x008b_8530, &[k]);
        assert_eq!(ra, w::is_message_status3_pressed(b, k.cast_signed()));
    }
}

/// The language manager and `TOGetFFXLang`, with and without a manager
/// (`Phyre_PhyrePrintf` stubbed in the original; the Rust side counts the
/// message).
fn languages(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use instance::bridge as i;
    let printf = a.rebase(0x0043_52f0);
    a.stub_return(printf, 0x5555).unwrap();
    let saved = i::LOCALIZATION_MANAGER.get(a);
    let fake = random_block(a, b, rng, 16);
    for m in [saved, 0, fake, saved] {
        set(a, b, i::LOCALIZATION_MANAGER, m);
        let mut printed = Vec::new();
        let ra = eax(a, 0x006d_b070, &[]);
        let rb = i::localization_manager(b, &mut |level, format| printed.push((level, format)));
        assert_eq!(ra, rb);
        let ra = eax(a, 0x0064_10e0, &[]);
        let rb = i::ffx_lang(b, &mut |level, format| printed.push((level, format)));
        assert_eq!(ra, rb, "TOGetFFXLang with {m:#x}");
        let message = i::NOT_CREATED_MESSAGE.ptr(b).addr;
        let want = if m == 0 {
            vec![(1, message); 2]
        } else {
            vec![]
        };
        assert_eq!(printed, want);
    }
    same_memory("languages", a, b);
}

fn round3(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    languages(a, b, rng);
    leb128_and_stubs(a, b, rng);
    trees_and_names(a, b, rng);
    small_state(a, b, rng);
    small_state2(a, b, rng);
}

#[test]
fn round3_system_functions_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round3(&mut a, &mut b, &mut Xorshift(0x3003_0001));
}

#[test]
fn round3_system_functions_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round3(&mut a, &mut b, &mut Xorshift(0x3003_0002));
}

// ---- the user heap and the page manager (ffx_system::heap) ----

const USER_MALLOC: u32 = 0x0068_7020;
const USER_FREE: u32 = 0x006f_b8b0;
/// The import slots the heap calls through.
const IAT_ENTER_CS: u32 = 0x00b0_c154;
const IAT_LEAVE_CS: u32 = 0x00b0_c158;
const IAT_VIRTUAL_ALLOC: u32 = 0x00b0_c09c;
const IAT_VIRTUAL_FREE: u32 = 0x00b0_c098;
const IAT_MEMCPY_S: u32 = 0x00b0_c434;
const USER_ALIGNED_MALLOC: u32 = 0x0068_6ff0;
const USER_MEM_SIZE: u32 = 0x0068_7050;
const USER_REALLOC: u32 = 0x0068_7060;
/// Free image bytes after `.text` for the stand-ins.
const HEAP_TEST_CODE: u32 = 0x00b0_bc00;

/// Win32 stand-ins in both oracles: the critical sections do nothing,
/// `VirtualFree` succeeds, `VirtualAlloc` commits in place or reserves the
/// next 64 KiB-aligned piece of a test region (1 MiB and up: of a range
/// nothing writes to), and fails for any size of exactly `fail_size` (to
/// reach the flush-and-retry paths; `u32::MAX`: never). Returns the two
/// cursors' addresses.
fn patch_heap_os(a: &mut Oracle, b: &mut Oracle, fail_size: u32) -> (u32, u32) {
    let region = alloc(a, b, 0x220_0000);
    let start = (region + 0xffff) & 0xffff_0000;
    let cursors = alloc(a, b, 8);
    put(a, b, cursors, [start, 0x8000_0000u32]);
    for o in [a, b] {
        let code = o.rebase(HEAP_TEST_CODE);
        #[rustfmt::skip]
        let mut va: Vec<u8> = vec![
            0x8b, 0x44, 0x24, 0x04, 0x8b, 0x4c, 0x24, 0x08,             // eax = addr, ecx = size
            0x81, 0xf9, 0, 0, 0, 0, 0x74, 0x34,                         // fail_size: fail
            0x85, 0xc0, 0x75, 0x32,                                     // addr: commit in place
            0x81, 0xc1, 0xff, 0xff, 0x00, 0x00, 0x81, 0xe1, 0x00, 0x00, 0xff, 0xff,
            0x81, 0xf9, 0x00, 0x00, 0x10, 0x00, 0x73, 0x0e,             // >= 1 MiB: big
            0xa1, 0, 0, 0, 0, 0x01, 0x0d, 0, 0, 0, 0, 0xc2, 0x10, 0x00, // small cursor
            0xa1, 0, 0, 0, 0, 0x01, 0x0d, 0, 0, 0, 0, 0xc2, 0x10, 0x00, // big cursor
            0x31, 0xc0, 0xc2, 0x10, 0x00,
        ];
        va[10..14].copy_from_slice(&fail_size.to_le_bytes());
        for (at, c) in [
            (41, cursors),
            (47, cursors),
            (55, cursors + 4),
            (61, cursors + 4),
        ] {
            va[at..at + 4].copy_from_slice(&c.to_le_bytes());
        }
        o.write(code, &va).unwrap();
        o.write(code + 0x60, &[0xb8, 1, 0, 0, 0, 0xc2, 0x0c, 0x00])
            .unwrap();
        // the critical sections: xor eax, eax; ret 4 (ntdll leaves STATUS_SUCCESS)
        o.write(code + 0x70, &[0x31, 0xc0, 0xc2, 0x04, 0x00])
            .unwrap();
        #[rustfmt::skip]
        let memcpy_s = [
            0x56, 0x57, 0x8b, 0x7c, 0x24, 0x0c, 0x8b, 0x74, 0x24, 0x14, // dst, src
            0x8b, 0x4c, 0x24, 0x18, 0xf3, 0xa4, 0x5f, 0x5e, 0x31, 0xc0, 0xc3, // rep movsb count
        ];
        o.write(code + 0x80, &memcpy_s).unwrap();
        for (slot, at) in [
            (IAT_MEMCPY_S, code + 0x80),
            (IAT_VIRTUAL_ALLOC, code),
            (IAT_VIRTUAL_FREE, code + 0x60),
            (IAT_ENTER_CS, code + 0x70),
            (IAT_LEAVE_CS, code + 0x70),
        ] {
            let s = o.rebase(slot);
            Mem::write(o, s, at);
        }
    }
    (cursors, cursors + 4)
}

/// The Rust side's Win32: the same stand-ins, the cursors kept here and
/// copied into memory after each call ([`TestOs::sync`]).
struct TestOs {
    at: u32,
    fail_size: u32,
    small: u32,
    big: u32,
    locks: i32,
    /// What the C runtime's `malloc` returns, in order (allocated in `b`
    /// beforehand, as `a`'s stub will).
    mallocs: Vec<u32>,
    /// What was handed to the C runtime's `free`.
    freed: Vec<u32>,
    /// Critical sections initialized.
    inits: Vec<u32>,
    /// `VirtualAlloc` fails above this size.
    max_size: u32,
    /// Mutexes waited for and not yet released.
    waits: Vec<u32>,
}

impl TestOs {
    fn new(o: &Oracle, at: u32, fail_size: u32) -> Self {
        Self {
            at,
            fail_size,
            small: Mem::read(o, at),
            big: Mem::read(o, at + 4),
            locks: 0,
            mallocs: Vec::new(),
            freed: Vec::new(),
            inits: Vec::new(),
            max_size: u32::MAX,
            waits: Vec::new(),
        }
    }

    /// Checks the cursors against `a`'s, then stores them in `b`.
    fn sync(&self, a: &Oracle, b: &mut Oracle, what: &str) {
        assert_eq!(
            Mem::read::<[u32; 2]>(a, self.at),
            [self.small, self.big],
            "{what}: reservations"
        );
        Mem::write(b, self.at, [self.small, self.big]);
    }
}

impl ffx_system::heap::Os for TestOs {
    fn enter_critical_section(&mut self, _cs: u32) {
        self.locks += 1;
    }
    fn leave_critical_section(&mut self, _cs: u32) {
        self.locks -= 1;
    }
    fn virtual_alloc(&mut self, addr: u32, size: u32, _alloc_type: u32, _protect: u32) -> u32 {
        if size == self.fail_size || size > self.max_size {
            return 0;
        }
        if addr != 0 {
            return addr;
        }
        let size = size.wrapping_add(0xffff) & 0xffff_0000;
        let cursor = if size >= 0x10_0000 {
            &mut self.big
        } else {
            &mut self.small
        };
        let r = *cursor;
        *cursor = cursor.wrapping_add(size);
        r
    }
    fn virtual_free(&mut self, _addr: u32, _size: u32, _free_type: u32) -> u32 {
        1
    }
    fn report_free_failure(&mut self) {
        panic!("VirtualFree failed");
    }
    fn initialize_critical_section(&mut self, cs: u32) {
        self.inits.push(cs);
    }
    fn crt_malloc(&mut self, _size: u32) -> u32 {
        self.mallocs.remove(0)
    }
    fn crt_free(&mut self, p: u32) {
        self.freed.push(p);
    }
    fn mem_system_init(&mut self) {
        panic!("the tests set the memory system up");
    }
    fn wait_for_single_object(&mut self, handle: u32, _ms: u32) -> u32 {
        self.waits.push(handle);
        0
    }
    fn release_mutex(&mut self, handle: u32) -> u32 {
        self.waits.retain(|&h| h != handle);
        1
    }
}

/// The allocation tracker's map: a balanced tree of (block, record)
/// nodes (left +0, right +4, key +0xc, record +0x10) under `head`.
fn tracker_tree(a: &mut Oracle, b: &mut Oracle, head: u32, items: &[(u32, u32)]) -> u32 {
    if items.is_empty() {
        return head;
    }
    let mid = items.len() / 2;
    let node = alloc(a, b, 0x14);
    let l = tracker_tree(a, b, head, &items[..mid]);
    let r = tracker_tree(a, b, head, &items[mid + 1..]);
    put(a, b, node, [l, r, 0, items[mid].0, items[mid].1]);
    node
}

/// The heap's entry points around the allocator: `malloc_pool_init` on a
/// fresh heap, calloc, aligned free, the Phyre allocations, operator new,
/// memclear, the jumps to them, the heap object's release.
#[test]
#[allow(clippy::too_many_lines)] // one script over the entry points
fn heap_entry_points_match_the_original() {
    use ffx_system::heap as h;
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    let mgr = fresh_page_manager(a, b);
    set(a, b, h::PAGE_MANAGER, mgr);
    let (cursors, _) = patch_heap_os(a, b, u32::MAX);
    // big reservations into a real region (arenas are written at both
    // ends), and VirtualAlloc failing above 16 MiB so malloc_pool_init
    // steps its 0x30000000-byte arena down to 16 MiB (48 heap objects)
    let region = alloc(a, b, 0x111_0000);
    put(a, b, cursors + 4, (region + 0xffff) & 0xffff_0000);
    for o in [&mut *a, &mut *b] {
        let code = o.rebase(HEAP_TEST_CODE);
        let slot = o.rebase(0x00b0_c150); // InitializeCriticalSection
        Mem::write(o, slot, code + 0x70);
        #[rustfmt::skip]
        let limit = [
            0x81, 0x7c, 0x24, 0x08, 0x00, 0x00, 0x00, 0x01, // cmp [esp+8], 16 MiB
            0x76, 0x05, 0x31, 0xc0, 0xc2, 0x10, 0x00,       // ja: xor eax, eax; ret 0x10
            0xe9, 0, 0, 0, 0,                               // jmp the VirtualAlloc stand-in
        ];
        let mut limit = limit.to_vec();
        let at = code + 0xc0;
        let rel = code.wrapping_sub(at + 20);
        limit[16..20].copy_from_slice(&rel.to_le_bytes());
        o.write(at, &limit).unwrap();
        let slot = o.rebase(IAT_VIRTUAL_ALLOC);
        Mem::write(o, slot, at);
    }
    set(a, b, h::MEM_SYSTEM, 1);
    set(a, b, h::USER_HEAP, 0);
    let mut os = TestOs::new(b, cursors, u32::MAX);
    os.max_size = 0x100_0000;
    for _ in 0..48 {
        os.mallocs.push(b.alloc(0x30).unwrap());
    }
    eax(a, 0x006f_b9b0, &[]);
    h::malloc_pool_init(b, &mut os);
    os.sync(a, b, "malloc_pool_init");
    assert!(
        os.mallocs.is_empty(),
        "{} heap objects left",
        os.mallocs.len()
    );
    assert_eq!(os.freed.len(), 47);
    assert_eq!(h::ARENA_SIZE.get(b), 0x100_0000);
    assert_eq!(os.inits.len(), 48);
    same_memory("malloc_pool_init", a, b);
    assert_ne!(h::FIRST_BLOCK.get(b), 0);
    // MemSystem_Ensure with the memory system set: nothing
    eax(a, 0x0094_28a0, &[]);
    h::mem_system_ensure(b, &mut os);
    let mut rng = Xorshift(0x4005_0001);
    let mut live: Vec<(u32, bool)> = Vec::new(); // (block, phyre)
    for n in 0..600u32 {
        let what = format!("heap entry op {n}");
        let mut os = TestOs::new(b, cursors, u32::MAX);
        os.max_size = 0x100_0000;
        match rng.next() % 9 {
            0 => {
                let (k, size) = (rng.next() % 8, rng.next() % 0x200);
                let sym = if n % 2 == 0 { 0x006f_b820 } else { 0x0068_7000 };
                let p = eax(a, sym, &[k, size]);
                assert_eq!(p, h::user_calloc(b, &mut os, k, size), "{what}");
                if p != 0 {
                    live.push((p, false));
                }
            }
            1 => {
                let size = rng.next() % 0x400;
                let p = eax(a, 0x0042_fb40, &[size]);
                assert_eq!(p, h::phyre_malloc(b, &mut os, size), "{what}");
                live.push((p, true));
            }
            2 => {
                let (size, align) = (rng.next() % 0x400, 1 << (2 + rng.next() % 6));
                let p = eax(a, 0x0042_fb80, &[size, align]);
                assert_eq!(
                    p,
                    h::phyre_malloc_aligned(b, &mut os, size, align),
                    "{what}"
                );
                live.push((p, true));
            }
            3 => {
                let size = rng.next() % 0x300;
                let sym = if n % 2 == 0 { 0x0063_0310 } else { 0x0063_04c0 };
                let p = eax(a, sym, &[size]);
                assert_eq!(p, h::user_malloc(b, &mut os, size), "{what}");
                live.push((p, false));
            }
            4 => {
                let (size, align) = (rng.next() % 0x300, 1 << (2 + rng.next() % 6));
                let p = eax(a, 0x0068_6ff0, &[size, align]);
                assert_eq!(p, h::user_aligned_malloc(b, &mut os, size, align), "{what}");
                eax(a, 0x006f_b6c0, &[p]);
                h::user_aligned_free(b, &mut os, p);
            }
            5 | 6 if !live.is_empty() => {
                let (p, phyre) = live.swap_remove((rng.next() as usize) % live.len());
                if phyre {
                    eax(a, 0x0042_fb20, &[p]);
                    h::phyre_free(b, &mut os, p);
                } else {
                    eax(a, 0x0068_7010, &[p]);
                    h::user_free(b, &mut os, p);
                }
            }
            7 => {
                eax(a, 0x0042_fb20, &[0]);
                h::phyre_free(b, &mut os, 0);
            }
            _ => {
                let (p, len) = (region + rng.next() % 0x1000, rng.next() % 0x100);
                assert_eq!(
                    eax(a, 0x0062_f4e0, &[p, len]),
                    h::memclear(b, p, len),
                    "{what}"
                );
            }
        }
        os.sync(a, b, &what);
        same_memory(&what, a, b);
    }
    // BigFileStream::setStreamPrefix: a copy in a new[] block
    let fs = random_block(a, b, &mut rng, 0x48);
    for path in [
        &b"\0"[..],
        b"c:/ffx/data/\0",
        b"a much longer prefix for the stream files/\0",
    ] {
        let p = alloc(a, b, path.len());
        a.write(p, path).unwrap();
        b.write(p, path).unwrap();
        let mut os = TestOs::new(b, cursors, u32::MAX);
        os.max_size = 0x100_0000;
        let ra = call(a, 0x0061_c3a0, Conv::Thiscall(fs), &[p]).eax;
        assert_eq!(
            ra,
            ffx_system::files::bridge::set_stream_prefix(b, &mut os, fs, p)
        );
        os.sync(a, b, "setStreamPrefix");
        same_memory("setStreamPrefix", a, b);
    }
    // operator delete / delete[] with the allocation tracker: its map holds
    // half the blocks (records freed with them), the tree removal recorded
    let cell = alloc(a, b, 4);
    put(a, b, cell, 0u32);
    #[rustfmt::skip]
    let mut remove = vec![0x8b, 0x44, 0x24, 0x04, 0xa3, 0, 0, 0, 0, 0xc2, 0x04, 0x00];
    remove[5..9].copy_from_slice(&cell.to_le_bytes());
    let remove = stand_in(a, b, 0x1c0, &remove);
    redirect(a, b, 0x0057_5000, remove);
    let tracker = alloc(a, b, 0xac);
    put(a, b, tracker + 0xa8, 0x1234u32);
    let mut blocks = Vec::new();
    for _ in 0..12 {
        let mut os = TestOs::new(b, cursors, u32::MAX);
        os.max_size = 0x100_0000;
        let size = 0x20 + rng.next() % 0x80;
        let p = eax(a, 0x0063_0310, &[size]);
        assert_eq!(p, h::user_malloc(b, &mut os, size));
        let r = eax(a, 0x0063_0310, &[0x10]);
        assert_eq!(r, h::user_malloc(b, &mut os, 0x10));
        os.sync(a, b, "tracked blocks");
        blocks.push((p, r));
    }
    let mut in_map: Vec<(u32, u32)> = blocks.iter().copied().step_by(2).collect();
    in_map.sort_unstable();
    let map = h::TRACKER_MAP.ptr(b).addr;
    set(a, b, h::TRACKER_NODES, 0);
    for (k, &(p, _)) in blocks.iter().enumerate() {
        // the stand-in removal leaves the tree as it was: rebuild it from
        // the blocks still tracked
        let root = tracker_tree(a, b, map, &in_map);
        put(a, b, map + 0xc, root);
        in_map.retain(|&(q, _)| q != p || k % 5 == 4 || k == 3);
        let mut os = TestOs::new(b, cursors, u32::MAX);
        os.max_size = 0x100_0000;
        let on = k % 5 != 4;
        set(a, b, h::ALLOC_TRACKER, if on { tracker } else { 0 });
        set(a, b, h::CRT_BLOCK, if k == 3 { p } else { 0 });
        let b_remove = |m: &mut Oracle, t: u32, n: u32| {
            m.call(remove, Conv::Thiscall(t), &[n]).unwrap();
        };
        if k % 2 == 0 {
            eax(a, 0x0063_0390, &[p]);
            h::operator_delete(b, &mut os, p, b_remove);
        } else {
            eax(a, 0x0063_0540, &[p]);
            h::operator_delete_array(b, &mut os, p, b_remove);
        }
        assert!(os.waits.is_empty(), "the tracker's mutex is released");
        os.sync(a, b, "operator delete");
        same_memory("operator delete", a, b);
    }
    // UserHeap_Init on a scratch object
    let obj = random_block(a, b, &mut rng, 0x18);
    for (arena, x, y) in [
        (0x30_0000, 0, 0x20_0000),
        (0x1234_5001, 7, 9),
        (0xffff_f001, 1, 2),
    ] {
        let ra = call(a, 0x0094_2a40, Conv::Thiscall(obj), &[arena, x, y]).eax;
        assert_eq!(ra, h::heap_init(b, obj, arena, x, y));
    }
    same_memory("UserHeap_Init", a, b);
    // the heap object back to the C runtime (a's free stub does nothing)
    let mut os = TestOs::new(b, cursors, u32::MAX);
    for sym in [0x006f_b9a0, 0x0068_7030] {
        eax(a, sym, &[]);
        h::user_heap_destroy(b, &mut os);
    }
    assert_eq!(os.freed, [h::USER_HEAP.get(b); 2]);
    same_memory("UserHeap_Destroy", a, b);
}

/// A page manager as the game's starts: empty size lists and queue, no
/// hash table, stamp 1.
fn fresh_page_manager(a: &mut Oracle, b: &mut Oracle) -> u32 {
    let mgr = alloc(a, b, 0x1c90);
    for k in 0..0x100 {
        let head = mgr + 0x1c * k;
        put(a, b, head + 0xc, head);
        put(a, b, head + 0x10, head);
    }
    let sentinel = mgr + 0x1c58;
    put(a, b, sentinel + 8, sentinel);
    put(a, b, sentinel + 0xc, sentinel);
    put(a, b, mgr + 0x1c6c, 1u32);
    mgr
}

/// Random mallocs (the original, in both; when `mallocs`) and frees (the
/// original in `a`, `ffx_system::heap::user_free` in `b`); `live` starts
/// with blocks the heap already holds. Memory compared after every free.
#[allow(clippy::many_single_char_names, clippy::too_many_lines)] // a, b: the oracles; one loop of operations
fn heap_rounds(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    mut live: Vec<u32>,
    rounds: u32,
    mallocs: bool,
) {
    let (cur_at, _) = patch_heap_os(a, b, u32::MAX);
    let mut os = TestOs::new(b, cur_at, u32::MAX);
    let start = os.small;
    let (mut frees, mut nulls) = (0, 0);
    for n in 0..rounds {
        if live.is_empty() && !mallocs {
            break;
        }
        let op = if !mallocs {
            if rng.next().is_multiple_of(4) { 4 } else { 5 }
        } else if live.is_empty() {
            0
        } else {
            rng.next() % 6
        };
        let size = match rng.next() % 4 {
            0 => rng.next() % 0x40,
            1 => rng.next() % 0x2000,
            2 => rng.next() % 0x2_0000,
            _ => 0x1000 * (rng.next() % 8),
        };
        let what = format!("op {op} at {n}");
        match op {
            0 => {
                let pa = eax(a, USER_MALLOC, &[size]);
                let pb = ffx_system::heap::user_malloc(b, &mut os, size);
                assert_eq!(pa, pb, "{what}: malloc {size:#x}");
                if pa == 0 {
                    nulls += 1;
                } else {
                    live.push(pa);
                }
            }
            1 => {
                // the distance back fits the byte below 256
                let align = [4, 8, 16, 32, 64, 100, 128][(rng.next() % 7) as usize];
                let ra = eax(a, USER_ALIGNED_MALLOC, &[size, align]);
                let rb = ffx_system::heap::user_aligned_malloc(b, &mut os, size, align);
                assert_eq!(ra, rb, "{what}: aligned malloc {size:#x} {align}");
                let back = u32::from(Mem::read::<u8>(a, ra - 1));
                let block = ra - back;
                live.push(block);
            }
            2 => {
                let p = if rng.next().is_multiple_of(5) {
                    0
                } else {
                    live.swap_remove((rng.next() as usize) % live.len())
                };
                let size = if rng.next().is_multiple_of(9) {
                    0
                } else {
                    size
                };
                let qa = eax(a, USER_REALLOC, &[p, size]);
                let qb = ffx_system::heap::user_realloc(b, &mut os, p, size);
                assert_eq!(qa, qb, "{what}: realloc {p:#x} {size:#x}");
                if qa != 0 {
                    live.push(qa);
                }
            }
            3 | 4 => {
                // a random pointer only image-only: the game's arena spans
                // pages the snapshot does not hold
                let p = if mallocs && rng.next().is_multiple_of(7) {
                    rng.next()
                } else {
                    live[(rng.next() as usize) % live.len()]
                };
                let na = eax(a, USER_MEM_SIZE, &[p]);
                assert_eq!(
                    na,
                    ffx_system::heap::user_mem_size(b, &mut os, p),
                    "{what}: size of {p:#x}"
                );
            }
            _ => {
                let k = (rng.next() as usize) % live.len();
                let p = live.swap_remove(k);
                eax(a, USER_FREE, &[p]);
                ffx_system::heap::user_free(b, &mut os, p);
                frees += 1;
            }
        }
        assert_eq!(os.locks, 0, "{what}");
        os.sync(a, b, &what);
        let d = a.diff(b).unwrap();
        assert!(
            d.is_empty(),
            "{what}: {} bytes differ, first {:x?}",
            d.len(),
            &d[..d.len().min(8)]
        );
        settle(a, b);
    }
    let mgr = ffx_system::heap::PAGE_MANAGER.get(b);
    eprintln!(
        "{frees} frees, {nulls} null mallocs, {} reserved, queue {:#x}, cached {:#x}, hash count {}, cache-node pages {}, queue-node pages {}",
        (os.small - start) >> 16,
        Mem::read::<u32>(b, mgr + 0x1c00),
        Mem::read::<u32>(b, mgr + 0x1c08),
        Mem::read::<u32>(b, mgr + 0x1c2c),
        Mem::read::<u32>(b, mgr + 0x1c48),
        Mem::read::<u32>(b, mgr + 0x1c54),
    );
}

#[test]
fn the_user_heap_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    // a small heap: arenas of 512 KiB, bins over 0x40..0x70000 (smaller
    // remainders stay free without a node: 3)
    let mgr = fresh_page_manager(a, b);
    set(a, b, ffx_system::heap::PAGE_MANAGER, mgr);
    let base = alloc(a, b, 0x30);
    let heap = base + 0x18;
    put(a, b, heap, [0x8_0000u32, 0x40, 0x7_0000, 0, 0x10, 0]);
    set(a, b, ffx_system::heap::USER_HEAP, base);
    heap_rounds(a, b, &mut Xorshift(0x4ea9_0001), Vec::new(), 6000, true);
}

/// The blocks in use in the game's heap, by walking its arenas.
fn heap_blocks(o: &Oracle) -> Vec<u32> {
    let heap = ffx_system::heap::USER_HEAP.get(o) + 0x18;
    let size: u32 = Mem::read(o, heap);
    let mut arena: u32 = Mem::read(o, heap + 0xc);
    let mut out = Vec::new();
    while arena != 0 {
        let mut blk = arena + 0x1010;
        while blk < arena + size {
            let s: u32 = Mem::read(o, blk + 4);
            if s == 0 {
                break;
            }
            if Mem::read::<u32>(o, blk + 0xc) <= 2 {
                out.push(blk + 0x10);
            }
            blk += s;
        }
        arena = Mem::read(o, arena + 0xff0);
    }
    out
}

/// Whether the page manager's decommit queue links both ways and its
/// queue-node free list stays on the pool's pages. The field snapshot
/// caught it torn (another thread inside its lock: the queue's one node
/// points back elsewhere, the free list runs into a block's data).
fn page_manager_consistent(o: &Oracle) -> bool {
    let mgr = ffx_system::heap::PAGE_MANAGER.get(o);
    let sentinel = mgr + 0x1c58;
    let mut n: u32 = Mem::read(o, sentinel + 0xc);
    let mut prev = sentinel;
    for _ in 0..100_000 {
        if Mem::read::<u32>(o, n + 8) != prev {
            return false;
        }
        if n == sentinel {
            break;
        }
        prev = n;
        n = Mem::read(o, n + 0xc);
    }
    let mut pages = Vec::new();
    let mut page: u32 = Mem::read(o, mgr + 0x1c50);
    while page != 0 && pages.len() < 1000 {
        pages.push(page);
        page = Mem::read(o, page);
    }
    let mut node: u32 = Mem::read(o, mgr + 0x1c4c);
    for _ in 0..100_000 {
        if node == 0 {
            return true;
        }
        if !pages.contains(&(node & 0xffff_f000)) {
            return false;
        }
        node = Mem::read(o, node);
    }
    false
}

#[test]
fn the_user_heap_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    if !page_manager_consistent(a) {
        // an empty queue and node pool, in both
        let mgr = ffx_system::heap::PAGE_MANAGER.get(a);
        let sentinel = mgr + 0x1c58;
        put(a, b, sentinel + 8, sentinel);
        put(a, b, sentinel + 0xc, sentinel);
        put(a, b, mgr + 0x1c00, 0u32);
        put(a, b, mgr + 0x1c4c, [0u32; 3]);
        eprintln!("the snapshot's page manager was torn: queue and queue nodes emptied");
    }
    let blocks = heap_blocks(a);
    assert!(blocks.len() > 100, "{} blocks in use", blocks.len());
    let mut rng = Xorshift(0x4ea9_0002);
    // runs of neighbours (merges both ways) and scattered blocks
    let mut live = Vec::new();
    while live.len() < 1500 {
        let k = (rng.next() as usize) % blocks.len();
        let run = 1 + rng.next() as usize % 6;
        live.extend(blocks.iter().skip(k).take(run));
    }
    live.sort_unstable();
    live.dedup();
    // frees only: a new block may need pages the snapshot does not hold
    heap_rounds(a, b, &mut rng, live, 4000, false);
}

const PAGE_ALLOC: u32 = 0x0094_8130;
const RELEASE_PAGES: u32 = 0x0094_7ec0;
const QUEUE_DECOMMIT: u32 = 0x0094_7690;
const COMMIT_PAGES: u32 = 0x0094_7590;
const FLUSH_PAGES: u32 = 0x0094_78e0;
const BIN_INDEX: u32 = 0x0094_32e0;

/// The page manager's entry points on a fresh manager: reservations
/// (cached and new, aligned or not, big ones, failing ones), give-backs,
/// queued decommits, commits over queued ranges, flushes as the stamp
/// moves; enough live reservations to grow the hash table.
#[test]
#[allow(clippy::too_many_lines)] // one script of page-manager operations
fn the_page_manager_matches_the_original() {
    use ffx_system::heap as h;
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    let mgr = fresh_page_manager(a, b);
    set(a, b, h::PAGE_MANAGER, mgr);
    let (cursors, _) = patch_heap_os(a, b, 0x3000);
    let mut os = TestOs::new(b, cursors, 0x3000);
    let mut rng = Xorshift(0x9a6e_0001);
    let mut live: Vec<(u32, u32)> = Vec::new();
    let mut queued: Vec<(u32, u32)> = Vec::new();
    let mut counts = [0u32; 6];
    for n in 0..5000u32 {
        let op = if n < 520 { 0 } else { rng.next() % 6 };
        counts[op as usize] += 1;
        let what = format!("op {op} at {n}");
        match op {
            0 => {
                let size = if n < 520 {
                    0x10_0000
                } else {
                    [
                        0x1000, 0x2000, 0x3000, 0x4000, 0x1_0000, 0x2_0000, 0x10_0000, 0x80_0000,
                    ][(rng.next() % 8) as usize]
                };
                let align = [0x1000, 0x1000, 0x1_0000, 0x4_0000][(rng.next() % 4) as usize];
                let ra = call(a, PAGE_ALLOC, Conv::Thiscall(mgr), &[size, align]).eax;
                let rb = h::page_alloc(b, &mut os, mgr, size, align);
                assert_eq!(ra, rb, "{what}");
                if ra != 0 {
                    assert!(live.iter().all(|&(q, _)| q != ra), "{what}: {ra:#x} twice");
                    live.push((ra, size));
                }
            }
            1 if !live.is_empty() && n >= 900 => {
                let (p, _) = live.swap_remove((rng.next() as usize) % live.len());
                call(a, RELEASE_PAGES, Conv::Thiscall(mgr), &[p]);
                h::release_pages(b, &mut os, mgr, p);
            }
            2 | 3 if !live.is_empty() => {
                let (p, size) = live[(rng.next() as usize) % live.len()];
                let pages = size / 0x1000;
                let first = rng.next() % pages;
                let len = 1 + rng.next() % (pages - first).min(8);
                let (mut at, mut bytes) = (p + first * 0x1000, len * 0x1000);
                if op == 3 && !queued.is_empty() && !rng.next().is_multiple_of(4) {
                    // over a queued range: the same, from inside, from
                    // before, or across the next ones
                    let (q, qlen) = queued[(rng.next() as usize) % queued.len()];
                    (at, bytes) = match rng.next() % 4 {
                        0 => (q, qlen),
                        1 => (
                            q + 0x1000 * (rng.next() % (qlen / 0x1000)),
                            0x1000 * (1 + rng.next() % 4),
                        ),
                        2 => (
                            q.saturating_sub(0x1000 * (1 + rng.next() % 3)),
                            qlen + 0x1000,
                        ),
                        _ => (q, qlen + 0x1000 * (rng.next() % 16)),
                    };
                }
                if op == 2 {
                    queued.push((at, bytes));
                    if queued.len() > 16 {
                        queued.remove(0);
                    }
                    call(a, QUEUE_DECOMMIT, Conv::Thiscall(mgr), &[at, bytes]);
                    h::queue_decommit(b, &mut os, mgr, at, bytes);
                } else {
                    let ra = call(a, COMMIT_PAGES, Conv::Thiscall(mgr), &[at, bytes]).eax & 0xff;
                    let rb = h::commit(b, &mut os, mgr, at, bytes);
                    assert_eq!(ra, u32::from(rb), "{what}");
                }
            }
            4 => {
                let force = u32::from(rng.next().is_multiple_of(4));
                call(a, FLUSH_PAGES, Conv::Thiscall(mgr), &[force]);
                h::flush(b, &mut os, mgr, u8::from(force != 0));
            }
            _ => {
                let stamp = Mem::read::<u32>(a, mgr + 0x1c6c) + 1;
                put(a, b, mgr + 0x1c6c, stamp);
            }
        }
        assert_eq!(os.locks, 0, "{what}");
        os.sync(a, b, &what);
        let d = a.diff(b).unwrap();
        assert!(
            d.is_empty(),
            "{what}: {} bytes differ, first {:x?}",
            d.len(),
            &d[..d.len().min(8)]
        );
        settle(a, b);
    }
    // one reservation of 256 MiB, released rather than kept
    let ra = call(a, PAGE_ALLOC, Conv::Thiscall(mgr), &[0x1000_0000, 0x1000]).eax;
    assert_eq!(ra, h::page_alloc(b, &mut os, mgr, 0x1000_0000, 0x1000));
    call(a, RELEASE_PAGES, Conv::Thiscall(mgr), &[ra]);
    h::release_pages(b, &mut os, mgr, ra);
    os.sync(a, b, "256 MiB");
    same_memory("256 MiB", a, b);
    eprintln!(
        "ops {counts:?}, hash capacity {:#x}, live {}",
        Mem::read::<u32>(b, mgr + 0x1c24),
        live.len()
    );

    // the bins over made-up bounds, both sides of the bend
    let heap = alloc(a, b, 0x18);
    for k in 0..4000u32 {
        let lo = [0, 0x10, rng.next() % 0x1000, rng.next()][(k % 4) as usize];
        let range = [
            0x1000,
            0x23dc,
            0x23dd,
            0x7_0000,
            0x2ff0_0000,
            rng.next() % 0x4000_0000 + 1,
        ][(k % 6) as usize];
        put(a, b, heap, [0, lo, lo.wrapping_add(range)]);
        let size = match k % 3 {
            0 => lo.wrapping_add(rng.next() % (range + 1)),
            1 => rng.next(),
            _ => lo.wrapping_add(range).wrapping_add(rng.next() % 0x100),
        };
        let ra = call(a, BIN_INDEX, Conv::Thiscall(heap), &[size]);
        let rb = h::bin_index(b, heap, size);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // edx:eax
        let (lo32, hi32) = (rb as u32, (rb >> 32) as u32);
        assert_eq!(
            (ra.eax, ra.edx),
            (lo32, hi32),
            "bin of {size:#x} over {lo:#x}+{range:#x}"
        );
    }
}

// ---- round 4 ----

/// The buffer-at-+8 destructors (`ffx_system::heap::free_member8`).
const FREE_MEMBER8: [u32; 13] = [
    0x0063_3b70,
    0x0063_3b80,
    0x0063_3b90,
    0x0063_3ba0,
    0x0063_3bb0,
    0x0063_3bc0,
    0x0063_3bd0,
    0x0063_3bf0,
    0x0063_3c00,
    0x0063_3c10,
    0x0063_3c20,
    0x0063_3c30,
    0x0069_0a20,
];

#[test]
fn buffer_destructors_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    let mgr = fresh_page_manager(a, b);
    set(a, b, ffx_system::heap::PAGE_MANAGER, mgr);
    let base = alloc(a, b, 0x30);
    put(a, b, base + 0x18, [0x8_0000u32, 0x40, 0x7_0000, 0, 0x10, 0]);
    set(a, b, ffx_system::heap::USER_HEAP, base);
    let (cursors, _) = patch_heap_os(a, b, u32::MAX);
    let obj = alloc(a, b, 16);
    let mut rng = Xorshift(0x4004_0001);
    for n in 0..400usize {
        let size = rng.next() % 0x3000;
        let p = eax(a, USER_MALLOC, &[size]);
        assert_eq!(p, eax(b, USER_MALLOC, &[size]));
        let mut os = TestOs::new(b, cursors, u32::MAX);
        put(a, b, obj + 8, if n % 5 == 0 { 0 } else { p });
        let sym = FREE_MEMBER8[n % FREE_MEMBER8.len()];
        let ra = call(a, sym, Conv::Thiscall(obj), &[]).eax;
        assert_eq!(
            ra,
            ffx_system::heap::free_member8(b, &mut os, obj),
            "{sym:#x}"
        );
        os.sync(a, b, "dtor");
        same_memory("buffer destructors", a, b);
    }
}

#[allow(clippy::many_single_char_names, clippy::too_many_lines)] // a, b: the oracles; one pass over many
fn round4(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use window::bridge as w;
    // identities
    for v in [0, 1, rng.next(), u32::MAX] {
        assert_eq!(eax(a, 0x008f_2f70, &[v]), stub::ptr32(v));
        assert_eq!(eax(a, 0x008e_d040, &[v]), stub::ptr32(v));
    }
    // GS packet words
    let pkt = alloc(a, b, 0x40);
    for _ in 0..200 {
        let v: [u32; 5] = std::array::from_fn(|_| rng.next());
        assert_eq!(
            eax(a, 0x008e_f8e0, &[pkt, v[0], v[1], v[2], v[3]]),
            pkt::bridge::adp_words4(b, pkt, [v[0], v[1], v[2], v[3]])
        );
        assert_eq!(
            eax(a, 0x008e_fb90, &[pkt + 4, v[0], v[1]]),
            pkt::bridge::adp_xy2(b, pkt + 4, v[0], v[1])
        );
        assert_eq!(
            eax(a, 0x008e_fbf0, &[pkt + 0x10, v[2], v[3]]),
            pkt::bridge::adp_xyz2_offset(b, pkt + 0x10, v[2], v[3])
        );
        assert_eq!(
            eax(a, 0x008e_f790, &[pkt + 0x20, v[4]]),
            pkt::bridge::adp_gs_direct_rgba(b, pkt + 0x20, v[4])
        );
        // quiet NaNs only: the oracle keeps a loaded signalling NaN
        let f: [f32; 4] = std::array::from_fn(|_| {
            let x = rng.float();
            if x.is_nan() {
                f32::from_bits(x.to_bits() | 0x0040_0000)
            } else {
                x
            }
        });
        eax(a, 0x008e_8940, &f.map(f32::to_bits));
        pkt::bridge::open_font_scissor(b, f);
        same_memory("packets", a, b);
    }
    // getters
    for round in 0..4 {
        for (sym, g) in [
            (0x008a_c2c0, instance::bridge::BTL_MENU_TEX),
            (0x008a_9b60, instance::bridge::YES_NO_WINDOW),
        ] {
            if round > 0 {
                set(a, b, g, rng.next());
            }
            assert_eq!(eax(a, sym, &[]), instance::bridge::word(b, g));
        }
    }
    // macro records and the 0x01868a90 table, made up
    for i in 0..3u32 {
        let recs = random_block(a, b, rng, 32 * 8);
        for n in 0..32 {
            set_entry(a, b, w::MSG_VARS, i * 32 + n, recs + 8 * n);
        }
        let e = random_block(a, b, rng, 0x10);
        set_entry(a, b, w::TABLE_01868A90, i, e);
    }
    for k in 0..60u32 {
        let (i, n) = (k % 3, rng.next() % 32);
        let t = rng.next();
        eax(a, 0x008b_8960, &[i, n, t]);
        w::macro_set_type(b, i.cast_signed(), n.cast_signed(), t.to_le_bytes()[0]);
        let v = rng.next();
        assert_eq!(
            eax(a, 0x008b_8980, &[i, n, v]),
            w::macro_set_value(b, i.cast_signed(), n.cast_signed(), v)
        );
        assert_eq!(
            eax(a, 0x008b_7c40, &[i]),
            w::table_01868a90(b, i.cast_signed())
        );
    }
    same_memory("macro records", a, b);
    // battle window close, hdd names
    set(
        a,
        b,
        btl::bridge::BTL_WINDOW_STATE,
        std::array::from_fn(|_| rng.next()),
    );
    assert_eq!(eax(a, 0x0089_ade0, &[]), btl::bridge::close_btl_window(b));
    eax(a, 0x0077_9a80, &[]);
    cd::bridge::hdd_set_pass_names_x(b);
    same_memory("battle window, hdd names", a, b);
    // module file sectors over a made-up module table and sector table
    if Mem::read::<u32>(a, cd::bridge::MODULE_FILES.ptr(a).addr) == 0 {
        let t = alloc(a, b, 0x40);
        for k in 0..0x20u32 {
            put(a, b, t + 2 * k, u16::try_from(rng.next() % 0x100).unwrap());
        }
        set(a, b, cd::bridge::MODULE_FILES, t);
        let sectors = random_block(a, b, rng, 0x800);
        set(a, b, cd::bridge::SECTORS, sectors);
    }
    for _ in 0..100 {
        let (m, n) = (rng.next() % 0x10, rng.next() % 0x80);
        assert_eq!(
            eax(a, 0x0076_c270, &[m, n]),
            cd::bridge::module_file_sector(b, m.cast_signed(), n.cast_signed())
        );
    }
    // a 12-byte element copy, overlapping or into null
    let blk = random_block(a, b, rng, 0x100);
    for (first, last, dest) in [
        (blk, blk + 0x30, blk + 0x60),
        (blk, blk, blk + 0x60),
        (blk, blk + 0xc, 0),
        (blk + 0x18, blk + 0x48, blk + 0xc),
    ] {
        assert_eq!(
            eax(a, 0x0059_e830, &[first, last, dest]),
            stl::bridge::uninitialized_copy12(b, first, last, dest)
        );
    }
    same_memory("copies", a, b);
}

/// A balanced tree of `keys` under `head` (left +0, right +4, key +0xc).
fn build_tree(a: &mut Oracle, b: &mut Oracle, head: u32, keys: &[u32]) -> u32 {
    if keys.is_empty() {
        return head;
    }
    let mid = keys.len() / 2;
    let node = alloc(a, b, 0x10);
    let l = build_tree(a, b, head, &keys[..mid]);
    let r = build_tree(a, b, head, &keys[mid + 1..]);
    put(a, b, node, [l, r, 0, keys[mid]]);
    node
}

/// Batch 2 of round 4: memcmp, word setter, a map lookup, window colours,
/// the HDD-install lookup, CD channel clears.
#[allow(clippy::many_single_char_names)] // a, b: the oracles
fn round4b(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use window::bridge as w;
    let x = random_block(a, b, rng, 0x40);
    let y = alloc(a, b, 0x40);
    for n in 0..400u32 {
        let bytes = Oracle::read(a, x, 0x40).unwrap();
        a.write(y, &bytes).unwrap();
        b.write(y, &bytes).unwrap();
        let len = rng.next() % 0x30;
        if n % 3 != 0 {
            let at = rng.next() % 0x30;
            put(a, b, y + at, u8::try_from(rng.next() & 0xff).unwrap());
        }
        let ra = eax(a, 0x0040_1020, &[x, y, len]);
        assert_eq!(
            ra,
            stl::bridge::memcmp(b, x, y, len).cast_unsigned(),
            "memcmp {n}"
        );
    }
    let obj = alloc(a, b, 0x40);
    for _ in 0..16 {
        let (i, v) = (rng.next() % 16, rng.next());
        let ra = call(a, 0x0040_2a00, Conv::Thiscall(obj), &[i, v]).eax;
        assert_eq!(ra, stl::bridge::set_word(b, obj, i, v));
    }
    // a tree of u32 keys under a head (root at +0xc)
    let head = alloc(a, b, 0x10);
    let mut keys: Vec<u32> = (0..40).map(|_| rng.next() % 1000).collect();
    keys.sort_unstable();
    keys.dedup();
    let root = build_tree(a, b, head, &keys);
    put(a, b, head + 0xc, root);
    let key = alloc(a, b, 4);
    for k in 0..1000u32 {
        put(a, b, key, k);
        let ra = call(a, 0x0062_fe50, Conv::Thiscall(head), &[key]).eax;
        assert_eq!(ra, stl::bridge::map_find_u32(b, head, key), "key {k}");
    }
    // window colours over made-up windows
    for i in 0..4u32 {
        let win = random_block(a, b, rng, 0x80);
        set_entry(a, b, w::WINDOWS, i, win);
    }
    for _ in 0..20 {
        let i = rng.next() % 4;
        let c: [u32; 4] = std::array::from_fn(|_| rng.next());
        eax(a, 0x0090_6d90, &[i, c[0], c[1], c[2], c[3]]);
        w::set_window_rgba(b, i.cast_signed(), c.map(|v| v.to_le_bytes()[0]));
    }
    same_memory("memcmp, words, windows", a, b);
    // the HDD-install table: the game's where loaded, else made up
    if cd::bridge::HDD_INSTALL_COUNT.get(a) <= 0 {
        let t = random_block(a, b, rng, 0x34 * 12);
        for k in 0..12 {
            put(a, b, t + 0x34 * k, u8::try_from(k % 3).unwrap());
            put(a, b, t + 0x34 * k + 8, k * 7);
        }
        set(a, b, cd::bridge::HDD_INSTALL, t);
        set(a, b, cd::bridge::HDD_INSTALL_COUNT, 12);
    }
    let t = cd::bridge::HDD_INSTALL.get(a);
    for k in 0..200u32 {
        let (arch, n) = if k % 2 == 0 && cd::bridge::HDD_INSTALL_COUNT.get(a) > 0 {
            let e = t + 0x34 * (rng.next() % cd::bridge::HDD_INSTALL_COUNT.get(a).cast_unsigned());
            (u32::from(Mem::read::<u8>(a, e)), Mem::read::<u32>(a, e + 8))
        } else {
            (
                [0, 1, 2, u32::MAX, rng.next()][(k % 5) as usize],
                [0, 7, 14, u32::MAX, rng.next()][(k / 5 % 5) as usize],
            )
        };
        assert_eq!(
            eax(a, 0x0076_d600, &[arch, n]),
            cd::bridge::hdd_install_find(b, arch, n),
            "{arch} {n}"
        );
    }
    for k in 0..20u32 {
        set(a, b, cd::bridge::CHANNEL_KEEP_NAMES, k % 2);
        let n = rng.next() % 8;
        eax(a, 0x0076_d530, &[n]);
        cd::bridge::cd_channel_clear(b, n);
    }
    same_memory("cd", a, b);
}

/// A balanced tree in the `0x0069a250` layout (left +0, right +8, nil
/// flag +0xd, key +0x10) under `head`.
fn nil_tree(a: &mut Oracle, b: &mut Oracle, head: u32, keys: &[u32]) -> u32 {
    if keys.is_empty() {
        return head;
    }
    let mid = keys.len() / 2;
    let node = alloc(a, b, 0x14);
    let l = nil_tree(a, b, head, &keys[..mid]);
    let r = nil_tree(a, b, head, &keys[mid + 1..]);
    put(a, b, node, [l, 0, r, 0, keys[mid]]);
    node
}

/// Batch 3 of round 4: the debug print buffer, the GS TEST register,
/// touch records, a map's equal range, glyph sprites, sequence end checks,
/// fixup records and the save table sort.
#[allow(clippy::many_single_char_names, clippy::too_many_lines)] // a, b: the oracles
fn round4c(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use daps::bridge as dp;
    use instance::bridge as ins;
    assert_eq!(eax(a, 0x0089_dc40, &[]), dp::init(b));
    let text = alloc(a, b, 0x100);
    for n in 0..300u32 {
        match n % 5 {
            0 | 1 => {
                let len = rng.next() % 0x60;
                for k in 0..len {
                    put(
                        a,
                        b,
                        text + k,
                        u8::try_from(0x20 + rng.next() % 0x5f).unwrap(),
                    );
                }
                put(a, b, text + len, 0u8);
                assert_eq!(
                    eax(a, 0x0089_dec0, &[text]),
                    dp::string(b, text),
                    "string {n}"
                );
            }
            2 | 3 => {
                let (x, y) = (rng.next() % 2000, rng.next());
                let y = [y % 700, y, 0x7fff_ffff, 0x8000_0000][(n as usize / 5) % 4];
                let ra = eax(a, 0x0089_de30, &[x, y]);
                assert_eq!(
                    ra,
                    dp::set_pos(b, x.cast_signed(), y.cast_signed()),
                    "set_pos {n}"
                );
            }
            _ => {
                if n % 50 == 4 {
                    assert_eq!(eax(a, 0x0089_dc10, &[]), dp::init(b));
                }
                let used = [0x7bf_i16, 0x7c0, -3, 0x100][(n as usize / 5) % 4];
                set(a, b, dp::USED, used);
            }
        }
    }
    same_memory("daps", a, b);
    let packet = alloc(a, b, 0x20);
    for _ in 0..100 {
        let f: [u32; 8] = std::array::from_fn(|k| {
            if rng.next().is_multiple_of(4) {
                rng.next()
            } else {
                rng.next() % [2, 8, 256, 4, 2, 2, 2, 4][k]
            }
        });
        let mut args = vec![packet];
        args.extend(f);
        let ra = eax(a, 0x008e_f970, &args);
        assert_eq!(
            ra,
            pkt::bridge::adp_test(b, packet, f.map(u32::cast_signed))
        );
    }
    same_memory("TOAdpTEST", a, b);
    let records = random_block(a, b, rng, 0x150 * 4);
    eax(a, 0x0068_9a10, &[records, 4]);
    pad::bridge::touch_records_init(b, records, 4);
    eax(a, 0x0068_9a10, &[records + 0x150, 0]);
    pad::bridge::touch_records_init(b, records + 0x150, 0);
    same_memory("touch records", a, b);

    // a tree in the 0x69a250 layout: key +0x10, nil flag +0xd, left +0, right +8
    let head = alloc(a, b, 0x14);
    put(a, b, head + 0xd, 1u8);
    let mut keys: Vec<u32> = (0..30).map(|_| rng.next() % 100).collect();
    keys.sort_unstable();
    let root = nil_tree(a, b, head, &keys);
    put(a, b, head + 4, root);
    let this = alloc(a, b, 4);
    put(a, b, this, head);
    let (key, out) = (alloc(a, b, 4), alloc(a, b, 8));
    for k in 0..110u32 {
        put(a, b, key, k);
        let ra = call(a, 0x0069_a250, Conv::Thiscall(this), &[out, key]).eax;
        assert_eq!(
            ra,
            stl::bridge::equal_range_u32(b, this, out, key),
            "key {k}"
        );
        same_memory("equal_range", a, b);
    }
    let sprite = alloc(a, b, 0x30);
    let chars = alloc(a, b, 4);
    for c in 0..0x200u32 {
        let (first, second) = if c < 0x100 {
            (c, rng.next() & 0xff)
        } else {
            (rng.next() % 0x30, c - 0x100)
        };
        put(
            a,
            b,
            chars,
            [u8::try_from(first).unwrap(), u8::try_from(second).unwrap()],
        );
        assert_eq!(
            eax(a, 0x0089_e360, &[chars, sprite]),
            msg::bridge::glyph_sprite(b, chars, sprite)
        );
        same_memory("glyph sprite", a, b);
    }
    // sequences: names in eight slots per scene, handles into the scene blocks
    let names: [&[u8]; 5] = [b"\0", b"a\0", b"ab\0", b"open\0", b"close_menu\0"];
    let name = alloc(a, b, 0x20);
    for i in 0..8u32 {
        let block = if i == 3 {
            0
        } else {
            random_block(a, b, rng, 0x400)
        };
        set_entry(a, b, ffx_menu::scene::bridge::SCENE_DATA, i, block);
        for j in 0..8u32 {
            let slot = scene::bridge::SLOTS.ptr(b).addr + 8 + i * 0x100 + j * 0x20;
            let mut bytes = [0u8; 0x14];
            let nm = names[(rng.next() % 5) as usize];
            bytes[..nm.len()].copy_from_slice(nm);
            put(a, b, slot, bytes);
            put(
                a,
                b,
                slot + 0x14,
                if rng.next().is_multiple_of(4) {
                    0
                } else {
                    rng.next() % 0x3f0
                },
            );
        }
    }
    for i in 0..8u32 {
        for nm in names.iter().chain([b"opened\0".as_slice(), b"b\0"].iter()) {
            let mut bytes = [0u8; 0x20];
            bytes[..nm.len()].copy_from_slice(nm);
            put(a, b, name, bytes);
            let ra = eax(a, 0x008a_e070, &[i, name]);
            let rb = scene::bridge::check_term_scene(
                b,
                i.cast_signed(),
                name,
                ffx_menu::scene::bridge::tk_menu_is_term_scene,
            );
            assert_eq!(ra, rb, "scene {i}");
        }
    }
    // fixup records from random LEB128 streams
    let stream = alloc(a, b, 0x40);
    let (cursor, record) = (alloc(a, b, 4), random_block(a, b, rng, 0x20));
    for n in 0..200u32 {
        let mut bytes = Vec::new();
        for _ in 0..6 {
            let v = [0, 1, rng.next() % 300, rng.next()][(rng.next() % 4) as usize];
            let mut v = v;
            loop {
                let low = u8::try_from(v & 0x7f).unwrap();
                v >>= 7;
                bytes.push(if v == 0 { low } else { low | 0x80 });
                if v == 0 {
                    break;
                }
            }
        }
        a.write(stream, &bytes).unwrap();
        b.write(stream, &bytes).unwrap();
        put(a, b, cursor, stream);
        let flags = rng.next() & 0x78;
        let ra = call(a, 0x0060_e730, Conv::Thiscall(record), &[cursor, flags]).eax;
        assert_eq!(
            ra,
            fixup::bridge::read_record(b, record, cursor, flags),
            "record {n}"
        );
        same_memory("Fixup_ReadRecord", a, b);
    }
    // the save table, with few distinct keys so ties happen
    for round in 0..4 {
        let base = save::bridge::SORTED_TABLE.ptr(b).addr;
        for i in 0..200u32 {
            let m = [3, 50, 0x7fff_ffff, u32::MAX][round];
            put(
                a,
                b,
                base + 12 * i,
                [rng.next(), rng.next() % m, (rng.next() % 5).wrapping_sub(2)],
            );
        }
        assert_eq!(eax(a, 0x006f_0fb0, &[]), save::bridge::sort_table(b));
        same_memory("sort table", a, b);
    }
    // the record resets of 0x00774740, counts 0..9 and a negative one
    let records = random_block(a, b, rng, 0x138 * 9);
    let table = alloc(a, b, 8 * 11);
    for e in 0..11u32 {
        let count: i16 = if e == 10 {
            -2
        } else {
            i16::try_from(e).unwrap()
        };
        put(a, b, table + 8 * e, records - 0x1000);
        put(a, b, table + 8 * e + 4, count);
    }
    set(a, b, ins::RECORD_ENTRIES, table);
    set(a, b, ins::RECORD_BASE, 0x1000);
    for e in 0..11u32 {
        set(a, b, ins::RECORD_ENTRY, e);
        assert_eq!(eax(a, 0x0077_4740, &[]), ins::reset_records(b), "entry {e}");
        same_memory("reset_records", a, b);
    }
}

/// Batch 4 of round 4: sequence slots, the battle command-window stack,
/// fill-buffer requests, touch tracks, a string-keyed map, window tables
/// and small setters.
#[allow(clippy::many_single_char_names, clippy::too_many_lines)] // a, b: the oracles
fn round4d(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as bt;
    use ffx_menu::scene::bridge as ms;
    use render::bridge as rd;
    use window::bridge as w;
    // sequences: scene blocks with named records for TkMenuGetSceneHandle
    let names: [&[u8]; 4] = [b"open\0", b"close\0", b"wait\0", b"none\0"];
    let name = alloc(a, b, 0x20);
    let put_name = |a: &mut Oracle, b: &mut Oracle, nm: &[u8]| {
        let mut bytes = [0u8; 0x20];
        bytes[..nm.len()].copy_from_slice(nm);
        put(a, b, name, bytes);
    };
    for i in 0..8u32 {
        let block = if i == 5 {
            0
        } else {
            seq_scene_block(a, b, rng, &names[..3])
        };
        set_entry(a, b, ms::SCENE_DATA, i, block);
        let base = scene::bridge::SLOTS.ptr(b).addr + i * 0x100;
        put(a, b, base, [0u8; 0x100]);
    }
    for n in 0..400u32 {
        let i = (rng.next() % 8).cast_signed();
        put_name(a, b, names[(rng.next() % 4) as usize]);
        let (ra, rb) = match n % 5 {
            0 => (
                eax(a, 0x008a_f0d0, &[i.cast_unsigned(), name]),
                scene::bridge::start_scene(b, i, name, false, ms::tk_menu_get_scene_handle),
            ),
            1 => (
                eax(a, 0x008a_f1b0, &[i.cast_unsigned(), name]),
                scene::bridge::start_scene(b, i, name, true, ms::tk_menu_get_scene_handle),
            ),
            2 => (
                eax(a, 0x008a_f4f0, &[i.cast_unsigned(), name]),
                scene::bridge::stop_scene(b, i, name),
            ),
            3 => (
                eax(a, 0x008a_e070, &[i.cast_unsigned(), name]),
                scene::bridge::check_term_scene(b, i, name, ms::tk_menu_is_term_scene),
            ),
            _ => {
                if rng.next().is_multiple_of(8) {
                    (
                        eax(a, 0x008a_e5a0, &[i.cast_unsigned()]),
                        scene::bridge::init_all_scene(b, i),
                    )
                } else {
                    let slot = scene::bridge::SLOTS.ptr(b).addr
                        + i.cast_unsigned() * 0x100
                        + (rng.next() % 8) * 0x20;
                    put(a, b, slot, u16::try_from(rng.next() % 5).unwrap());
                    (0, 0)
                }
            }
        };
        assert_eq!(ra, rb, "sequence call {n}");
        same_memory("sequences", a, b);
    }
    // the command-window stack
    let stack = bt::ACTION_STACK.ptr(b).addr;
    for n in 0..60u32 {
        if n % 6 == 0 {
            for k in 0..6u32 {
                let mut level = [0u8; 0xf0];
                for (j, v) in level.iter_mut().enumerate() {
                    *v = u8::try_from((rng.next() >> (j % 4)) & 0xff).unwrap();
                }
                level[0x52] = u8::try_from(rng.next() % 2).unwrap();
                level[0x5a] = u8::try_from(rng.next() % 5).unwrap();
                level[0x5b] = u8::try_from(rng.next() % 6).unwrap().wrapping_sub(1);
                let cmd: u16 = [0x311e, 0x3029, 0x1234][(rng.next() % 3) as usize];
                level[0x5c..0x5e].copy_from_slice(&cmd.to_le_bytes());
                put(a, b, stack + 0xf0 * k, level);
            }
            set(
                a,
                b,
                bt::ACTION_DEPTH,
                i8::try_from(rng.next() % 6).unwrap() - 1,
            );
            set(a, b, bt::ACTION_MASK, rng.next());
            set(
                a,
                b,
                bt::ACTION_PREV_VALUE,
                u16::try_from(rng.next() & 0xffff).unwrap(),
            );
            let saved: [u8; 0x140] =
                std::array::from_fn(|_| u8::try_from(rng.next() & 0xff).unwrap());
            put(a, b, bt::ACTION_SAVED.ptr(b).addr, saved);
        }
        assert_eq!(
            eax(a, 0x0089_a630, &[]),
            bt::set_prev_action(b),
            "TOBtlSetPrevAction {n}"
        );
        same_memory("TOBtlSetPrevAction", a, b);
    }
    // fill-buffer requests
    for n in 0..300u32 {
        if n % 50 == 0 {
            for k in 0..6u32 {
                put(a, b, rd::FILL_BUFFERS.ptr(b).addr + 4 * k, rng.next());
                put(
                    a,
                    b,
                    rd::FILL_COUNTS.ptr(b).addr + 4 * k,
                    0x150 + rng.next() % 8,
                );
                put(
                    a,
                    b,
                    rd::FILL_LOADS.ptr(b).addr + 4 * k,
                    [rng.next() % 1000, 100_000, 99_999, rng.next()][(rng.next() % 4) as usize],
                );
            }
        }
        let size = rng.next() % 300;
        assert_eq!(
            eax(a, 0x0072_8660, &[size]),
            rd::get_fill_buffer_request(b, size.cast_signed()),
            "fill {n}"
        );
        same_memory("fill buffers", a, b);
    }
    // touch tracks
    let tracks = random_block(a, b, rng, 0x1c8 * 3);
    for (base, n) in [(tracks, 3), (tracks, 0), (0, 1)] {
        let ra = eax(a, 0x0068_98d0, &[base, n]);
        let rb = pad::bridge::touch_tracks_init(b, base, n);
        if n != 0 {
            assert_eq!(ra, rb);
        }
    }
    same_memory("touch tracks", a, b);
    // a string-keyed tree with tagged keys (some null)
    let head = alloc(a, b, 0x10);
    let words = ["", "a", "ab", "abc", "b", "ba", "c", "zz"];
    let mut keys: Vec<u32> = vec![0];
    for w in words {
        let p = alloc(a, b, 8);
        let mut bytes = [0u8; 8];
        bytes[..w.len()].copy_from_slice(w.as_bytes());
        put(a, b, p, bytes);
        keys.push(p);
    }
    // sorted by the comparison the lookup uses: null first, then strings
    let tree_keys = keys.clone();
    let root = cstr_tree(a, b, head, &tree_keys, rng);
    put(a, b, head + 0xc, root);
    let key = alloc(a, b, 4);
    for &k in &keys {
        for tag in [0, 1] {
            put(a, b, key, k | tag);
            let ra = call(a, 0x006d_c010, Conv::Thiscall(head), &[key]).eax;
            assert_eq!(ra, stl::bridge::map_find_cstr(b, head, key));
        }
    }
    let other = alloc(a, b, 4);
    put(a, b, other, *b"abd\0");
    put(a, b, key, other);
    assert_eq!(
        call(a, 0x006d_c010, Conv::Thiscall(head), &[key]).eax,
        stl::bridge::map_find_cstr(b, head, key)
    );
    // window tables and small setters
    for i in 0..4u32 {
        let e = random_block(a, b, rng, 0x20);
        put(a, b, e + 1, u8::try_from(rng.next() % 4).unwrap());
        set_entry(a, b, w::TABLE_01868A90, i, e);
    }
    for i in 0..4u32 {
        let k = i.cast_signed();
        assert_eq!(
            eax(a, 0x008b_7fb0, &[i]),
            w::table_kind(b, k).cast_unsigned()
        );
        assert_eq!(eax(a, 0x008b_7fd0, &[i]), w::table_kind2_data(b, k));
        assert_eq!(
            eax(a, 0x008b_7ff0, &[i]),
            w::table_kind2_byte7(b, k).cast_unsigned()
        );
    }
    for win in 0..2u32 {
        for n in 0..4u32 {
            let r = random_block(a, b, rng, 8);
            put(a, b, w::MSG_VARS.ptr(b).addr + 4 * (win * 32 + n), r);
            let v = rng.next();
            eax(a, 0x008b_84f0, &[win, n, v]);
            w::macro_set_byte1(b, win.cast_signed(), n.cast_signed(), v.to_le_bytes()[0]);
            assert_eq!(
                eax(a, 0x008b_8510, &[win, n, v]),
                w::macro_set_value(b, win.cast_signed(), n.cast_signed(), v)
            );
        }
    }
    for i in 0..8u32 {
        let (win, msg) = (
            random_block(a, b, rng, 0x80),
            random_block(a, b, rng, 0x120),
        );
        set_entry(a, b, w::WINDOWS, i, win);
        set_entry(a, b, w::MSG_WINDOWS, i, msg);
    }
    eax(a, 0x008a_b7b0, &[]);
    w::all_close_message_window(b);
    same_memory("window tables", a, b);
    for i in 0..8u32 {
        assert_eq!(
            call(a, 0x0040_2be0, Conv::Stdcall, &[i]).eax,
            instance::bridge::table_00c8a538(b, i)
        );
        let v = rng.next();
        assert_eq!(
            eax(a, 0x008b_dfc0, &[i, v]),
            instance::bridge::set_table_02310f20(b, i, v)
        );
    }
    let manager = random_block(a, b, rng, 0x400);
    set(a, b, save::bridge::MANAGER, manager);
    let v = rng.next();
    assert_eq!(
        eax(a, 0x0064_8640, &[v]),
        save::bridge::set_manager_word360(b, v)
    );
    let printf = a.rebase(0x0043_52f0);
    a.stub_return(printf, 0x5555).unwrap();
    let saved = instance::bridge::LOCALIZATION_MANAGER.get(a);
    for m in [0, random_block(a, b, rng, 16), saved] {
        set(a, b, instance::bridge::LOCALIZATION_MANAGER, m);
        let ra = eax(a, 0x0064_1100, &[]);
        assert_eq!(
            ra,
            instance::bridge::localization_manager_word0(b, &mut |_, _| {})
        );
    }
    same_memory("small setters", a, b);
    if cd::bridge::SECTORS.get(a) != 0 {
        for n in 0..50 {
            assert_eq!(
                eax(a, 0x0076_c250, &[n]),
                cd::bridge::cd_module_file_sector_rel(b, n.cast_signed())
            );
        }
    }
    // Kernel_GetAbilityName over the a_ability table (made up when absent)
    if ffx_kernel::bridge::A_ABILITY.get(a) == 0 {
        let t = alloc(a, b, 0x400);
        put(a, b, t, 2u16);
        put(a, b, t + 8, [0x0001u16, 0x0010, 8, 0x200]);
        put(a, b, t + 16, 0x40u32);
        put(a, b, t + 20, [0x0020u16, 0x0030, 4, 0x280]);
        put(a, b, t + 28, 0xc0u32);
        let words: [u32; 0x80] = std::array::from_fn(|_| rng.next() & 0x03ff_03ff);
        put(a, b, t + 0x40, words);
        set(a, b, ffx_kernel::bridge::A_ABILITY, t);
    }
    for n in 0..300u32 {
        let id = [0, rng.next() & 0x3f, 0x8000 | (rng.next() & 0x7f)][(n % 3) as usize];
        let ra = eax(a, 0x0077_4720, &[id]);
        assert_eq!(
            ra,
            names::bridge::ability_name(b, id, ffx_menu::tk::bridge::tk_mn2_get_ability_mes),
            "ability {id:#x}"
        );
    }
    same_memory("ability names", a, b);
}

/// A menu scene block whose hash table (`TkMenuGetSceneHandle`) holds
/// records for `names` (each `"..\0"`), their status words random.
fn seq_scene_block(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, names: &[&[u8]]) -> u32 {
    let block = alloc(a, b, 0x400);
    put(a, b, block, [0u8; 0x400]);
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); 16];
    for (k, nm) in names.iter().enumerate() {
        groups[ffx_menu::scene::bucket(&nm[..nm.len() - 1]) as usize].push(k);
    }
    for (h, group) in (0u32..).zip(&groups) {
        if group.is_empty() {
            continue;
        }
        let list = 0x100 + 0x40 * h;
        put(a, b, block + 0x40 + 4 * h, list);
        put(a, b, block + list, u32::try_from(group.len()).unwrap());
        for (j, &k) in (0u32..).zip(group) {
            let at = 0x380 + 0x10 * u32::try_from(k).unwrap();
            let mut bytes = [0u8; 0x10];
            bytes[..names[k].len()].copy_from_slice(names[k]);
            put(a, b, block + at, bytes);
            let record = block + list + 4 + 12 * j;
            put(a, b, record, [at, rng.next(), rng.next()]);
        }
    }
    block
}

/// A balanced tree of `keys` (in lookup order) in the `0x006dc010` layout
/// (left +0, right +4, tagged key +0xc) under `head`, keys tagged at random.
fn cstr_tree(a: &mut Oracle, b: &mut Oracle, head: u32, keys: &[u32], rng: &mut Xorshift) -> u32 {
    if keys.is_empty() {
        return head;
    }
    let mid = keys.len() / 2;
    let node = alloc(a, b, 0x10);
    let l = cstr_tree(a, b, head, &keys[..mid], rng);
    let r = cstr_tree(a, b, head, &keys[mid + 1..], rng);
    put(a, b, node, [l, r, 0, keys[mid] | (rng.next() & 1)]);
    node
}

#[test]
fn round4_system_functions_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round4(&mut a, &mut b, &mut Xorshift(0x4004_0002));
    round4b(&mut a, &mut b, &mut Xorshift(0x4004_0012));
    round4c(&mut a, &mut b, &mut Xorshift(0x4004_0022));
    round4d(&mut a, &mut b, &mut Xorshift(0x4004_0032));
}

#[test]
fn round4_system_functions_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round4(&mut a, &mut b, &mut Xorshift(0x4004_0003));
    round4b(&mut a, &mut b, &mut Xorshift(0x4004_0013));
    round4c(&mut a, &mut b, &mut Xorshift(0x4004_0023));
    round4d(&mut a, &mut b, &mut Xorshift(0x4004_0033));
}

/// Free image bytes for the file-layer stand-ins (after the heap's).
const FILE_TEST_CODE: u32 = 0x00b0_be00;

/// Writes `code` at `FILE_TEST_CODE + off` in both oracles; its address.
fn stand_in(a: &mut Oracle, b: &mut Oracle, off: u32, code: &[u8]) -> u32 {
    let at = a.rebase(FILE_TEST_CODE) + off;
    a.write(at, code).unwrap();
    b.write(at, code).unwrap();
    at
}

/// `jmp to` at the original `sym` in both oracles.
fn redirect(a: &mut Oracle, b: &mut Oracle, sym: u32, to: u32) {
    let at = a.rebase(sym);
    let rel = to.wrapping_sub(at + 5);
    let mut jmp = vec![0xe9];
    jmp.extend_from_slice(&rel.to_le_bytes());
    a.write(at, &jmp).unwrap();
    b.write(at, &jmp).unwrap();
}

/// A cdecl stand-in: records its first argument at `cell` and returns it
/// xor `key`.
fn recorder(cell: u32, key: u32) -> Vec<u8> {
    let mut v = vec![0x8b, 0x44, 0x24, 0x04, 0xa3];
    v.extend_from_slice(&cell.to_le_bytes());
    v.push(0x35);
    v.extend_from_slice(&key.to_le_bytes());
    v.push(0xc3);
    v
}

/// The file layer over stand-ins for what it calls (the stream read,
/// `File_Probe`, `0x00a42c00`, `movie_err`, `GetFileSizeEx` and the CD
/// table's functions), so only the wrappers' own logic is compared.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one script over the wrappers; a, b: the oracles
fn file_layer(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use files::bridge as f;
    let cells = alloc(a, b, 0x40);
    put(a, b, cells, [0u8; 0x40]);
    // PStreamFile::read (thiscall, ret 8): this ^ buf + n, recorded
    #[rustfmt::skip]
    let mut read = vec![
        0x89, 0xc8, 0x33, 0x44, 0x24, 0x04, 0x03, 0x44, 0x24, 0x08, // eax = ecx ^ buf + n
        0xa3, 0, 0, 0, 0, 0xc2, 0x08, 0x00,
    ];
    read[11..15].copy_from_slice(&cells.to_le_bytes());
    let read = stand_in(a, b, 0, &read);
    redirect(a, b, 0x0060_8090, read);
    let probe = stand_in(a, b, 0x20, &recorder(cells + 4, 0x1234_5600));
    redirect(a, b, 0x0060_7e00, probe);
    let name = stand_in(a, b, 0x40, &recorder(cells + 8, 0x0bad_0000));
    redirect(a, b, 0x00a4_2c00, name);
    // movie_err: records all three arguments
    #[rustfmt::skip]
    let mut err = vec![
        0x8b, 0x44, 0x24, 0x04, 0xa3, 0, 0, 0, 0,
        0x8b, 0x44, 0x24, 0x08, 0xa3, 0, 0, 0, 0,
        0x8b, 0x44, 0x24, 0x0c, 0xa3, 0, 0, 0, 0, 0xc3,
    ];
    for (k, at) in [(5, cells + 0xc), (14, cells + 0x10), (23, cells + 0x14)] {
        err[k..k + 4].copy_from_slice(&u32::to_le_bytes(at));
    }
    let err = stand_in(a, b, 0x60, &err);
    redirect(a, b, 0x0076_efa0, err);
    // GetFileSizeEx(h, &size), stdcall: size = (h, h * 3), succeeds for odd h
    #[rustfmt::skip]
    let size_ex = [
        0x8b, 0x44, 0x24, 0x04, 0x8b, 0x4c, 0x24, 0x08,       // eax = h, ecx = &size
        0x89, 0x41, 0x04, 0x8d, 0x04, 0x40, 0x89, 0x01,       // hi = h, lo = h * 3
        0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x01, 0xc2, 0x08, 0x00, // eax = h & 1
    ];
    let size_ex = stand_in(a, b, 0x80, &size_ex);
    for o in [&mut *a, &mut *b] {
        let slot = o.rebase(0x00b0_c13c);
        Mem::write(o, slot, size_ex);
    }
    let scratch = alloc(a, b, 8);
    let get_size = |m: &mut Oracle, h: u32| -> Option<u64> {
        let ok = m.call(size_ex, Conv::Stdcall, &[h, scratch]).unwrap().eax;
        let v: u64 = Mem::read(m, scratch);
        Mem::write(m, scratch, 0u64);
        (ok != 0).then_some(v)
    };
    let b_read = |m: &mut Oracle, s: u32, buf: u32, n: u32| {
        m.call(read, Conv::Thiscall(s), &[buf, n]).unwrap().eax
    };
    let b_probe = |m: &mut Oracle, n: u32| m.call(probe, Conv::Cdecl, &[n]).unwrap().eax;
    // streams: closed, handle-backed, entry-backed
    let entry = random_block(a, b, rng, 0x10);
    let rec = random_block(a, b, rng, 0x10);
    put(a, b, entry + 0xc, rec);
    let mut streams = Vec::new();
    for (h, e) in [(u32::MAX, 0), (6, 0), (7, 0), (u32::MAX, entry), (9, entry)] {
        let s = alloc(a, b, 8);
        put(a, b, s, [h, e]);
        streams.push(s);
    }
    for &s in &streams {
        assert_eq!(
            call(a, 0x0060_7f20, Conv::Thiscall(s), &[]).eax & 0xff,
            u32::from(f::stream_is_open(b, s))
        );
        let r = call(a, 0x0060_7dc0, Conv::Thiscall(s), &[]);
        #[allow(clippy::cast_possible_truncation)] // eax
        let want = f::stream_file_size(b, s, get_size) as u32;
        assert_eq!(r.eax, want);
        let r = call(a, 0x006f_9d50, Conv::Thiscall(s), &[]);
        #[allow(clippy::cast_possible_truncation)]
        let want = f::fios_stream_file_size(b, s, get_size) as u32;
        assert_eq!(r.eax, want);
    }
    for &s in streams.iter().chain([0].iter()) {
        assert_eq!(eax(a, 0x0067_9570, &[s]), f::fios_file_size(b, s, get_size));
        assert_eq!(eax(a, 0x0062_f760, &[s]), f::fios_file_size(b, s, get_size));
        let (buf, n) = (rng.next(), rng.next() % 0x1000);
        assert_eq!(
            eax(a, 0x0067_9800, &[s, buf, n]),
            f::fios_read(b, s, buf, n, b_read)
        );
        assert_eq!(
            eax(a, 0x0062_fc00, &[s, buf, n]),
            f::sce_read(b, s, buf, n, b_read)
        );
        same_memory("stream reads", a, b);
    }
    for _ in 0..20 {
        let nm = rng.next();
        assert_eq!(
            eax(a, 0x0067_96f0, &[nm]),
            f::fios_is_file_exist(b, nm, b_probe)
        );
    }
    // the CD table's functions: recorders, one per slot
    let table = cd::bridge::CD_LIB.ptr(b).addr;
    for (k, slot) in [0u32, 8, 0x10, 0x58].into_iter().enumerate() {
        let k = u32::try_from(k).unwrap();
        let fun = stand_in(
            a,
            b,
            0xa0 + 0x10 * k,
            &recorder(cells + 0x18 + 4 * k, 0x100 * (k + 1)),
        );
        put(a, b, table + slot, fun);
    }
    let b_call =
        |m: &mut Oracle, fun: u32, args: &[u32]| m.call(fun, Conv::Cdecl, args).unwrap().eax;
    for _ in 0..10 {
        let (x, y, z) = (rng.next(), rng.next(), rng.next());
        assert_eq!(
            eax(a, 0x008a_ba30, &[x, y, z]),
            f::dvd_file_read(b, x, y, z, b_call)
        );
        assert_eq!(
            eax(a, 0x008a_b7d0, &[x, y]),
            f::cd_get_file_name(b, x, y, b_call)
        );
        same_memory("CD table calls", a, b);
    }
    let b_name = |m: &mut Oracle, n: u32| m.call(name, Conv::Cdecl, &[n]).unwrap().eax;
    for k in 0..12u32 {
        put(a, b, table + 0x20, rng.next() % 0x100);
        put(a, b, table + 0x30, k % 2);
        let n = rng.next() % 0x200;
        assert_eq!(eax(a, 0x00a4_2830, &[n]), f::cd_file_name_pc(b, n, b_name));
        assert_eq!(
            eax(a, 0x00a4_29d0, &[n]),
            f::get_cd_file_name_pc(b, n, b_name)
        );
        same_memory("CD names", a, b);
    }
    // VFile: assign, and fileRead over FileSys_Read (the read stand-in)
    redirect(a, b, 0x0061_aa90, read);
    for k in 0..6u32 {
        let state = random_block(a, b, rng, 0x18);
        let vf = random_block(a, b, rng, 0x20);
        put(a, b, vf + 0x18, state);
        let h = rng.next();
        assert_eq!(
            call(a, 0x0061_aa10, Conv::Thiscall(vf), &[h]).eax,
            f::vfile_assign(b, vf, h)
        );
        put(a, b, vf + 0x10, [u32::MAX - k, rng.next()]);
        let (buf, n) = (
            if k == 0 { vf } else { rng.next() },
            [0, 1, 0x800, rng.next()][(k % 4) as usize],
        );
        let ra = call(a, 0x0061_aa60, Conv::Thiscall(vf), &[buf, n]).eax;
        assert_eq!(
            ra,
            f::vfile_file_read(b, vf, buf, n, b_read),
            "fileRead {k}"
        );
        same_memory("VFile", a, b);
    }
    // the read-name table: fill all 16 slots, then one more (the assertion)
    let from = cd::bridge::CHGNAME_FROM.ptr(b).addr;
    put(a, b, from, [0u32; 33]);
    put(a, b, from + 8, 5u32);
    for k in 0..17u32 {
        let (x, y) = (rng.next() | 1, rng.next());
        let ra = eax(a, 0x00a4_3220, &[x, y]);
        let rb = cd::bridge::set_read_chgname(b, x, y, |m, e, fl, l| {
            m.call(err, Conv::Cdecl, &[e, fl, l]).unwrap();
        });
        assert_eq!(ra, rb, "set_read_chgname {k}");
        same_memory("set_read_chgname", a, b);
    }
}

#[test]
fn file_layer_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    file_layer(&mut a, &mut b, &mut Xorshift(0x4005_0101));
}

#[test]
fn file_layer_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    file_layer(&mut a, &mut b, &mut Xorshift(0x4005_0102));
}

/// Round 5's window dispatchers (over recording stand-ins), quads, key
/// state, lower-casing and the size-table flag.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one pass; a, b: the oracles
fn round5(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use window::bridge as w;
    let cells = alloc(a, b, 0x10);
    put(a, b, cells, [0u32; 4]);
    let fns: Vec<u32> = (0..4u32)
        .map(|k| stand_in(a, b, 0x100 + 0x10 * k, &recorder(cells + 4 * k, 0x5000 + k)))
        .collect();
    let b_call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    for i in 0..8u32 {
        let win = random_block(a, b, rng, 0x80);
        put(a, b, win, u16::try_from(rng.next() % 2).unwrap());
        put(a, b, win + 0x74, fns[0]);
        put(a, b, win + 0x78, fns[1]);
        set_entry(a, b, w::WINDOWS, i, win);
        let msg = random_block(a, b, rng, 0x120);
        put(a, b, msg, u16::try_from(rng.next() % 3).unwrap());
        put(a, b, msg + 0x1e, u16::try_from(rng.next() % 2).unwrap());
        put(a, b, msg + 0x2c, u16::try_from(1 + rng.next() % 2).unwrap());
        let flag = alloc(a, b, 4);
        put(a, b, flag, u32::from(rng.next().is_multiple_of(2)));
        put(a, b, msg + 0x98, flag);
        put(a, b, msg + 0x114, if i == 3 { 0 } else { fns[2] });
        put(a, b, msg + 0xf8, fns[3]);
        put(a, b, msg + 0xfc, fns[2]);
        set_entry(a, b, w::MSG_WINDOWS, i, msg);
        let e = random_block(a, b, rng, 0x20);
        put(
            a,
            b,
            e + 1,
            [
                i8::try_from(rng.next() % 3).unwrap() - 1,
                i8::try_from(rng.next() % 200).unwrap_or(-7),
            ],
        );
        set_entry(a, b, w::TABLE_01868A90, i, e);
    }
    for n in 0..40u32 {
        let i = n % 8;
        let k = i.cast_signed();
        assert_eq!(eax(a, 0x0090_6bd0, &[i]), w::control_window(b, k, b_call));
        assert_eq!(eax(a, 0x0090_6bf0, &[i]), w::draw_window(b, k, b_call));
        assert_eq!(eax(a, 0x008b_5850, &[i]), w::control_message(b, k, b_call));
        assert_eq!(eax(a, 0x008b_8ca0, &[i]), w::message_call_f8(b, k, b_call));
        assert_eq!(
            eax(a, 0x008b_6b70, &[i]),
            w::control_pad_status(b, k, b_call),
            "pad status {n}"
        );
        assert_eq!(
            eax(a, 0x008b_8010, &[i]),
            w::message_table_byte2(b, k).cast_unsigned()
        );
        same_memory("window dispatchers", a, b);
    }
    let quad = alloc(a, b, 0x10);
    for _ in 0..20 {
        let v: [u32; 4] = std::array::from_fn(|_| rng.next());
        eax(a, 0x0090_48a0, &[quad, v[0], v[1], v[2], v[3]]);
        w::set_poly_g4a_xywh(b, quad, v[0], v[1], v[2], v[3]);
    }
    same_memory("quads", a, b);
    // key state: devices with blocks for some of the 7
    let devices = alloc(a, b, 0xa0);
    put(a, b, devices, [0u8; 0xa0]);
    for d in 0..7u32 {
        if d != 2 {
            let block = random_block(a, b, rng, 0x120);
            put(a, b, devices + 0x80 + 4 * d, block);
        }
    }
    let this = alloc(a, b, 0x2bc);
    put(a, b, this + 0x2b8, devices);
    for binding in (0..300u32).step_by(7).chain([27, 28, 283, 284, u32::MAX]) {
        for d in [0, 2, 6, 7, 100] {
            let ra = call(a, 0x0062_9d50, Conv::Thiscall(this), &[binding, d]).eax & 0xff;
            assert_eq!(
                ra,
                u32::from(pad::bridge::key_held(b, this, binding, d)),
                "{binding} {d}"
            );
        }
    }
    let text = alloc(a, b, 0x40);
    for k in 0..30u32 {
        let len = (k % 20) as usize;
        let mut bytes: Vec<u8> = (0..len)
            .map(|_| u8::try_from(1 + rng.next() % 255).unwrap())
            .collect();
        bytes.push(0);
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        let p = if k == 29 { 0 } else { text };
        assert_eq!(eax(a, 0x0060_7d50, &[p]), stl::bridge::to_lower_ascii(b, p));
        same_memory("lowercase", a, b);
    }
    if cd::bridge::SECTORS.get(a) == 0 {
        let t = random_block(a, b, rng, 0x400);
        set(a, b, cd::bridge::SECTORS, t);
        set(a, b, cd::bridge::FILE_BASE, 0x10);
    }
    for n in 0..200 {
        assert_eq!(
            eax(a, 0x0076_dc20, &[n]),
            cd::bridge::cd_size_from_table(b, n.cast_signed())
        );
    }
    // the TOAdp register writers and packet words
    let packet = alloc(a, b, 0x10);
    for n in 0..200u32 {
        for (f, shifts, _) in pkt::bridge::ADP_REGS {
            let count = shifts.len() + 1;
            let fields: Vec<u32> = (0..count)
                .map(|_| {
                    if n % 5 == 0 {
                        rng.next()
                    } else {
                        rng.next() % 1024
                    }
                })
                .collect();
            let mut args = vec![packet];
            args.extend(&fields);
            let ra = eax(a, f, &args);
            let signed: Vec<i32> = fields.iter().map(|&v| v.cast_signed()).collect();
            assert_eq!(ra, pkt::bridge::adp_reg_at(b, f, packet, &signed), "{f:#x}");
            same_memory("TOAdp registers", a, b);
        }
        let v: [u32; 4] = std::array::from_fn(|_| rng.next());
        assert_eq!(
            eax(a, 0x008e_f800, &[packet, v[0]]),
            pkt::bridge::adp_fog(b, packet, v[0])
        );
        assert_eq!(
            eax(a, 0x008e_fb60, &[packet]),
            pkt::bridge::adp_texflush(b, packet)
        );
        assert_eq!(
            eax(a, 0x008e_fbc0, &[packet, v[0], v[1], v[2], v[3]]),
            pkt::bridge::adp_xy_frac(b, packet, v)
        );
        assert_eq!(
            eax(a, 0x008e_fc30, &[packet, v[0], v[1]]),
            pkt::bridge::adp_words2_reg10(b, packet, v[0], v[1])
        );
        assert_eq!(
            eax(a, 0x008e_fcd0, &[packet, v[0], v[1], v[2]]),
            pkt::bridge::adp_words3(b, packet, v[0], v[1], v[2])
        );
        assert_eq!(
            eax(a, 0x008e_fc90, &[packet, v[0], v[1], v[2]]),
            pkt::bridge::adp_xyz_offset(b, packet, v[0], v[1], v[2], None)
        );
        assert_eq!(
            eax(a, 0x008e_fd00, &[packet, v[0], v[1], v[2], v[3]]),
            pkt::bridge::adp_xyz_offset(b, packet, v[0], v[1], v[2], Some(v[3]))
        );
        same_memory("TOAdp words", a, b);
    }
    // input masks, the pair flag, entry clears, the screen size
    let input = alloc(a, b, 8);
    for _ in 0..100 {
        put(a, b, input, [rng.next() & 0xff, rng.next() & 0xff]);
        let mask = 1 << (rng.next() % 8)
            | if rng.next().is_multiple_of(4) {
                0x30
            } else {
                0
            };
        assert_eq!(
            call(a, 0x0063_1340, Conv::Thiscall(input), &[mask]).eax,
            pad::bridge::is_hold_button(b, input, mask)
        );
        let ra = call(a, 0x0063_1360, Conv::Thiscall(input), &[mask]).eax & 0xff;
        assert_eq!(ra, u32::from(pad::bridge::is_press_button(b, input, mask)));
        let ra = call(a, 0x0063_1380, Conv::Thiscall(input), &[mask]).eax & 0xff;
        assert_eq!(
            ra,
            u32::from(pad::bridge::is_release_button(b, input, mask))
        );
    }
    for _ in 0..30 {
        let (p, c) = (
            [0, 1, 2, rng.next()][(rng.next() % 4) as usize],
            rng.next() % 3,
        );
        assert_eq!(eax(a, 0x008b_dfe0, &[p, c]), w::set_pair(b, p, c));
        let i = [0, 5, 15, 16, u32::MAX, 0x8000_0000][(rng.next() % 6) as usize];
        assert_eq!(
            eax(a, 0x008f_f140, &[i]),
            w::clear_entry_18762e0(b, i.cast_signed())
        );
        let (sw, sh) = (rng.next(), rng.next());
        eax(a, 0x0090_48f0, &[sw, sh]);
        w::set_screen_wh(b, sw, sh);
        same_memory("pair, entries, screen", a, b);
    }
}

#[test]
fn round5_system_functions_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round5(&mut a, &mut b, &mut Xorshift(0x4005_0201));
}

#[test]
fn round5_system_functions_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round5(&mut a, &mut b, &mut Xorshift(0x4005_0202));
}

/// Round 6's small accessors: setters, flags, float copies, tree ends,
/// tables, an offset-list unlink and two block copies.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one pass; a, b: the oracles
fn round6(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use accessor::bridge as acc;
    use window::bridge as w;
    let g = |addr: u32| Global::<u32>::new(addr);
    for _ in 0..20 {
        let obj = random_block(a, b, rng, 0x440);
        set(a, b, g(0x00ce_81f4), obj);
        assert_eq!(eax(a, 0x006f_24d0, &[]), acc::clear_word8_of_00ce81f4(b));
        let v = rng.next();
        assert_eq!(
            call(a, 0x006f_24f0, Conv::Thiscall(obj), &[v]).eax,
            acc::set_member(b, obj, 0xc, v)
        );
        assert_eq!(
            call(a, 0x006f_2520, Conv::Thiscall(obj), &[v]).eax,
            acc::set_member(b, obj, 0x394, v)
        );
        for (f, _) in acc::WORD_SETTERS {
            assert_eq!(eax(a, f, &[v]), acc::set_word_global(b, f, v));
        }
        eax(a, 0x0076_dc60, &[v]);
        acc::set_byte_01128ac0(b, v.to_le_bytes()[0]);
        set(a, b, g(0x0132_6b28), obj);
        set(a, b, g(0x0132_5b88), rng.next() % 2);
        assert_eq!(eax(a, 0x0087_29e0, &[]), acc::check_01326b28(b));
        assert_eq!(eax(a, 0x0087_29c0, &[]), acc::flag_01326b28(b));
        set(a, b, g(0x0187_1698), obj);
        assert_eq!(
            eax(a, 0x008e_2dd0, &[v]),
            acc::set_world_map_window_word68(b, v)
        );
        put(a, b, obj + 0x364, u8::try_from(rng.next() % 3).unwrap());
        let ra = call(a, 0x006f_2120, Conv::Thiscall(obj), &[]).eax & 0xff;
        assert_eq!(ra, u32::from(acc::take_byte364(b, obj)));
        let i = rng.next() % 0x40;
        assert_eq!(
            call(a, 0x006f_2430, Conv::Thiscall(obj), &[i]).eax,
            acc::set_flag_3bc(b, obj, i)
        );
        call(a, 0x006e_4fd0, Conv::Thiscall(obj), &[]);
        acc::reset_e4fd0(b, obj);
        let src = alloc(a, b, 0x10);
        let floats: [f32; 4] = std::array::from_fn(|_| {
            let x = f32::from_bits(rng.next());
            if x.is_nan() { 1.5 } else { x }
        });
        put(a, b, src, floats);
        assert_eq!(
            call(a, 0x0070_37e0, Conv::Thiscall(obj), &[src]).eax,
            acc::copy_floats(b, obj, src, 0x14, 2)
        );
        assert_eq!(
            call(a, 0x0070_3750, Conv::Thiscall(obj), &[src]).eax,
            acc::copy_floats(b, obj, src, 4, 4)
        );
        let n = rng.next() % 3;
        eax(a, 0x0076_cfa0, &[n]);
        acc::set_c40e19(b, n);
        call(a, 0x0061_acf0, Conv::Thiscall(obj), &[]);
        acc::vfile_reset(b, obj);
        let k = rng.next() % 2;
        put(
            a,
            b,
            obj + k * 0x1e8 + 0x1f8,
            [8u32, 7][(rng.next() % 2) as usize],
        );
        assert_eq!(
            call(a, 0x006d_bf30, Conv::Thiscall(obj), &[k]).eax,
            acc::record_state_is8(b, obj, k)
        );
        let entries = random_block(a, b, rng, 0x40);
        put(a, b, obj + 4, rng.next() % 4);
        for e in 0..4u32 {
            put(a, b, obj + 0x10 + 8 * e, entries + 0x10 * e);
        }
        let e = rng.next() % 4;
        assert_eq!(
            call(a, 0x0068_4ca0, Conv::Thiscall(obj), &[e]).eax,
            acc::entry_word_c(b, obj, e)
        );
        assert_eq!(
            call(a, 0x0040_2b50, Conv::Thiscall(obj), &[]).eax,
            acc::copy_c8a538(b, obj)
        );
        same_memory("round 6 accessors", a, b);
    }
    // message windows and the 0x01868a90 table
    for i in 0..8u32 {
        let (m, e) = (rng.next(), rng.next());
        set_entry(a, b, window::bridge::MSG_WINDOWS, i, m);
        set_entry(a, b, window::bridge::TABLE_01868A90, i, e);
        assert_eq!(
            eax(a, 0x008b_9640, &[i]),
            window::bridge::message_window(b, i.cast_signed())
        );
        assert_eq!(
            eax(a, 0x008b_9a50, &[i]),
            window::bridge::table_01868a90_entry(b, i.cast_signed())
        );
    }
    // HDD heads
    let heads = a.rebase(0x0112_a52c);
    for i in 0..51u32 {
        put(a, b, heads + 12 * i, rng.next());
        assert_eq!(
            eax(a, 0x0077_8960, &[i]),
            acc::hdd_head_sectors(b, i).cast_unsigned()
        );
    }
    // tree ends: a chain each way ending at nil nodes
    let nil = alloc(a, b, 0x10);
    put(a, b, nil + 0xd, 1u8);
    let mut nodes = Vec::new();
    for _ in 0..6 {
        let n = alloc(a, b, 0x10);
        put(a, b, n, [nil, nil, nil, 0]);
        nodes.push(n);
    }
    for w in nodes.windows(2) {
        put(a, b, w[0], w[1]);
        put(a, b, w[1] + 8, w[0]);
    }
    for &n in &nodes {
        assert_eq!(eax(a, 0x00a4_19b0, &[n]), acc::tree_extreme(b, n, 0));
        assert_eq!(eax(a, 0x00a4_1990, &[n]), acc::tree_extreme(b, n, 8));
    }
    // the seven-word table
    set(a, b, g(0x00c3_3468), 100);
    for x in 90..115u32 {
        assert_eq!(eax(a, 0x0067_aa50, &[x]), acc::table_00c334d4(b, x));
    }
    // the offset list
    let block = random_block(a, b, rng, 0x400);
    set(a, b, g(0x0112_a03c), block);
    for _ in 0..10 {
        let rec = block + 0x10 * (rng.next() % 0x30);
        put(a, b, rec + 0x38, 0x10 * (rng.next() % 0x30));
        put(a, b, rec + 0x44, 0x10 * (rng.next() % 0x30));
        assert_eq!(eax(a, 0x0077_6910, &[rec]), acc::unlink_record(b, rec));
        same_memory("unlink", a, b);
    }
    let src = a.rebase(0x022f_b620);
    let bytes: Vec<u8> = (0..0x80)
        .map(|_| u8::try_from(rng.next() & 0xff).unwrap())
        .collect();
    a.write(src, &bytes).unwrap();
    b.write(src, &bytes).unwrap();
    eax(a, 0x009d_a750, &[]);
    acc::copy_9da750(b);
    same_memory("round 6 tables", a, b);
    // round 6, second batch
    let packet = alloc(a, b, 0x10);
    for _ in 0..50 {
        let v: [u32; 4] = std::array::from_fn(|_| rng.next());
        for f in [0x008e_f830, 0x008e_fd40] {
            let ra = eax(a, f, &[packet, v[0], v[1] % 256, v[2]]);
            let fields = [
                v[0].cast_signed(),
                (v[1] % 256).cast_signed(),
                v[2].cast_signed(),
            ];
            assert_eq!(ra, pkt::bridge::adp_reg_at(b, f, packet, &fields));
        }
        let floats: [f32; 4] = std::array::from_fn(|k| {
            let x = f32::from_bits(v[k]);
            if x.is_nan() { 2.0 } else { x }
        });
        eax(a, 0x008e_8980, &floats.map(f32::to_bits));
        pkt::bridge::open_scissor(b, floats);
        let ra = eax(a, 0x008b_8cd0, &[packet, v[0], v[1], v[2], v[3]]);
        assert_eq!(
            ra,
            pkt::bridge::set_rect_direct(b, packet, v.map(u32::cast_signed))
        );
        let i = v[0] % 6;
        eax(a, 0x008c_a370, &[i, v[1]]);
        w::set_byte_186a429(b, i, v[1].to_le_bytes()[0]);
        same_memory("round 6 packets", a, b);
    }
    for start in [-1i32, 0, 1, 2, -7] {
        set(a, b, w::SWITCH_18763E0, start);
        for _ in 0..4 {
            assert_eq!(eax(a, 0x0090_6740, &[]), w::step_switch(b).cast_unsigned());
        }
    }
    set(a, b, Global::<u32>::new(0x0134_0438), 0);
    for _ in 0..8 {
        let (x, y, z) = (rng.next(), rng.next(), rng.next());
        assert_eq!(
            eax(a, 0x0089_e4f0, &[x, y, z]),
            w::push_entry_13402b8(b, x, y, z)
        );
    }
    for (count, v) in [(0u8, 3u32), (5, 3), (5, 2), (12, 3)] {
        set(a, b, Global::<u8>::new(0x0186_9f18), count);
        eax(a, 0x008b_e050, &[v]);
        w::set_entries_1869f20(b, v);
    }
    let rec = random_block(a, b, rng, 0x18);
    put(a, b, rec + 0xf, 0u8);
    for _ in 0..10 {
        assert_eq!(eax(a, 0x008a_2ad0, &[rec]), w::anim_step_8a2ad0(b, rec));
    }
    for angle in [0.0f32, 3.0, 3.1, 3.125, -3.0, 100.0] {
        set(a, b, w::ANGLE_133FA7C, angle);
        for _ in 0..5 {
            eax(a, 0x0089_0f30, &[]);
            w::advance_angle(b);
        }
    }
    same_memory("round 6 window state", a, b);
    if cd::bridge::SECTORS.get(a) == 0 {
        let t = random_block(a, b, rng, 0x400);
        set(a, b, cd::bridge::SECTORS, t);
        set(a, b, cd::bridge::FILE_BASE, 0x10);
    }
    for n in 0..200 {
        assert_eq!(
            eax(a, 0x0076_c440, &[n]),
            cd::bridge::cd_file_size_bytes(b, n.cast_signed())
        );
    }
    let mgr = random_block(a, b, rng, 0x400);
    let entries = random_block(a, b, rng, 0x20 * 8);
    put(a, b, mgr + 0x368, entries);
    for k in 0..8u32 {
        put(a, b, entries + 0x20 * k + 6, u16::try_from(k * 3).unwrap());
    }
    for count in [0u32, 3, 8, u32::MAX] {
        put(a, b, mgr + 0x3a0, count);
        for id in [0u32, 3, 9, 21, 22] {
            let ra = call(a, 0x006f_1e40, Conv::Thiscall(mgr), &[id]).eax;
            assert_eq!(ra, save::bridge::find_entry_by_id(b, mgr, id));
        }
    }
    let slots = random_block(a, b, rng, 0x18 * 10);
    for k in 0..10u32 {
        put(
            a,
            b,
            slots + 0x18 * k,
            u8::try_from(rng.next() % 2).unwrap(),
        );
        put(a, b, slots + 0x18 * k + 4, rng.next() % 3);
    }
    put(a, b, mgr, [0x8000_000au32, slots]);
    for key in 0..3 {
        assert_eq!(
            call(a, 0x006f_c390, Conv::Thiscall(mgr), &[key]).eax,
            save::bridge::release_slots(b, mgr, key)
        );
    }
    same_memory("round 6 save tables", a, b);
    // MapFindU32_0060c6b0 over the u32 tree of round4b's layout
    let head = alloc(a, b, 0x10);
    let mut keys: Vec<u32> = (0..20).map(|_| rng.next() % 100).collect();
    keys.sort_unstable();
    keys.dedup();
    let root = build_tree(a, b, head, &keys);
    put(a, b, head + 0xc, root);
    let key = alloc(a, b, 4);
    for k in 0..100u32 {
        put(a, b, key, k);
        assert_eq!(
            call(a, 0x0060_c6b0, Conv::Thiscall(head), &[key]).eax,
            stl::bridge::map_find_u32(b, head, key)
        );
    }
}

#[test]
fn round6_system_functions_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round6(&mut a, &mut b, &mut Xorshift(0x4006_0001));
    round6c(&mut a, &mut b, &mut Xorshift(0x4006_0011));
}

#[test]
fn round6_system_functions_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round6(&mut a, &mut b, &mut Xorshift(0x4006_0002));
    round6c(&mut a, &mut b, &mut Xorshift(0x4006_0012));
}

/// Round 6, third batch: byte swaps, settings tables, input state, ids,
/// float pairs, the fixup runners, tree rotations, window opening, slots,
/// repeat buttons, the PC read table, packets, strings, preloads.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one pass; a, b: the oracles
fn round6c(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use accessor::bridge as acc;
    use window::bridge as w;
    let cells = alloc(a, b, 0x10);
    put(a, b, cells, [0u32; 4]);
    let fns: Vec<u32> = (0..4u32)
        .map(|k| stand_in(a, b, 0x180 + 0x10 * k, &recorder(cells + 4 * k, 0x7000 + k)))
        .collect();
    let b_call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    let obj = random_block(a, b, rng, 0x440);
    let out = alloc(a, b, 0x10);
    for _ in 0..10 {
        assert_eq!(eax(a, 0x0064_7440, &[obj]), acc::swap3(b, obj));
        assert_eq!(
            eax(a, 0x0064_7490, &[obj, out]),
            acc::swap3_out(b, obj, out)
        );
        assert_eq!(
            call(a, 0x0040_2b00, Conv::Thiscall(obj), &[]).eax,
            acc::copy_default_tables(b, obj)
        );
        let v = [rng.next(), rng.next(), rng.next()];
        let ra = call(
            a,
            0x0063_2930,
            Conv::Thiscall(obj),
            &[out, v[0], v[1], v[2]],
        )
        .eax;
        assert_eq!(ra, acc::swap_state_ac(b, obj, out, v));
        same_memory("round 6c state", a, b);
    }
    let list = random_block(a, b, rng, 0x20);
    for k in 0..8u32 {
        put(a, b, list + 4 * k, rng.next() % 600);
    }
    put(a, b, obj + 0x40, list);
    for _ in 0..30 {
        put(a, b, obj + 0x18, rng.next() % 9);
        let n = [0, 1, 500, 501, rng.next() % 600, Mem::read::<u32>(a, list)]
            [(rng.next() % 6) as usize];
        assert_eq!(
            call(a, 0x006f_b320, Conv::Thiscall(obj), &[n]).eax,
            acc::select_id(b, obj, n)
        );
        same_memory("select_id", a, b);
    }
    let pairs = alloc(a, b, 4 + 12 * 20);
    put(a, b, pairs, 0u32);
    for _ in 0..40 {
        let key = rng.next() % 12;
        let (x, y) = (rng.next() % 1000, rng.next() % 1000);
        #[allow(clippy::cast_precision_loss)]
        let (fx, fy) = (x as f32 / 7.0, y as f32 / 3.0);
        let ra = call(
            a,
            0x006e_4020,
            Conv::Thiscall(pairs),
            &[key, fx.to_bits(), fy.to_bits()],
        )
        .eax;
        assert_eq!(ra, acc::set_pair_floats(b, pairs, key, fx, fy));
        same_memory("pair floats", a, b);
    }
    // the fixup runners: a list of n items, the recorder as fn
    let src = random_block(a, b, rng, 0x20);
    let dst_buf = alloc(a, b, 0x1c * 8);
    let dst = alloc(a, b, 4);
    let flist = alloc(a, b, 8);
    for n in [0u32, 1, 5] {
        put(a, b, flist, [rng.next(), n]);
        for (sym, size) in [(0x0060_ca70, 0x1c), (0x0060_ca10, 0x10)] {
            let ctx = rng.next();
            put(a, b, dst, dst_buf);
            let ra = call(a, sym, Conv::Thiscall(flist), &[fns[0], src, ctx, dst]).eax;
            Mem::write(b, dst, dst_buf);
            let rb = acc::fixup_run_all(b, flist, [fns[0], src, ctx, dst], size, b_call);
            if n != 0 {
                assert_eq!(ra, rb, "{sym:#x} {n}");
            }
        }
    }
    same_memory("fixup runners", a, b);
    // tree rotations on a 7-node tree with parent links
    let head = alloc(a, b, 0x10);
    put(a, b, head + 0xd, 1u8);
    let nodes: Vec<u32> = (0..7).map(|_| alloc(a, b, 0x10)).collect();
    // in-order 0..7, root 3; children of 3: 1, 5; of 1: 0, 2; of 5: 4, 6
    let shape = [
        (3usize, Some(1usize), Some(5usize)),
        (1, Some(0), Some(2)),
        (5, Some(4), Some(6)),
    ];
    for &n in &nodes {
        put(a, b, n, [head, head, head, 0]);
    }
    for (p, l, r) in shape {
        for (c, off) in [(l, 0u32), (r, 8)] {
            if let Some(c) = c {
                put(a, b, nodes[p] + off, nodes[c]);
                put(a, b, nodes[c] + 4, nodes[p]);
            }
        }
    }
    put(a, b, head + 4, nodes[3]);
    let tree = alloc(a, b, 4);
    put(a, b, tree, head);
    for _ in 0..30 {
        let n = nodes[(rng.next() % 7) as usize];
        let right: u32 = Mem::read(a, n + 8);
        let left: u32 = Mem::read(a, n);
        if rng.next().is_multiple_of(2) && right != head {
            call(a, 0x00a4_1930, Conv::Thiscall(tree), &[n]);
            stl::bridge::tree_rotate_left(b, tree, n);
        } else if left != head {
            call(a, 0x00a4_1b40, Conv::Thiscall(tree), &[n]);
            stl::bridge::tree_rotate_right(b, tree, n);
        }
        same_memory("tree rotations", a, b);
    }
    // window and message opening, slot clearing
    for i in 0..8u32 {
        let win = random_block(a, b, rng, 0x80);
        put(a, b, win + 0x60, [fns[0], fns[1], fns[2]]);
        set_entry(a, b, w::WINDOWS, i, win);
        let msg = random_block(a, b, rng, 0x120);
        put(a, b, msg + 0x100, fns[2]);
        put(a, b, msg + 0xf8, fns[3]);
        set_entry(a, b, w::MSG_WINDOWS, i, msg);
        let slot = alloc(a, b, 0x1c);
        for off in [0x10, 0x14, 0x18] {
            let part = random_block(a, b, rng, 0x30);
            put(a, b, slot + off, part);
        }
        put(a, b, w::MSG_WINDOW_SLOTS.ptr(b).addr + 4 * i, slot);
    }
    for i in 0..8u32 {
        let k = i.cast_signed();
        assert_eq!(eax(a, 0x0090_6c80, &[i]), w::open_window(b, k, b_call));
        assert_eq!(eax(a, 0x008b_8560, &[i]), w::open_message(b, k, b_call));
        assert_eq!(eax(a, 0x008a_b800, &[i]), w::clear_message_slot(b, i));
        same_memory("window opening", a, b);
    }
    // repeat buttons
    let input = alloc(a, b, 8 + 4 * 29);
    for _ in 0..200 {
        let times: Vec<f32> = (0..29u32)
            .map(|_| f32::from(u16::try_from(rng.next() % 40).unwrap()) / 100.0)
            .collect();
        put(a, b, input, [rng.next() & 0xffff, rng.next() & 0xffff]);
        for (k, h) in (0u32..).zip(&times) {
            put(a, b, input + 8 + 4 * k, *h);
        }
        let mask = rng.next() & 0x1fff_ffff;
        let ra = call(a, 0x0063_13a0, Conv::Thiscall(input), &[mask]).eax & 0xff;
        assert_eq!(ra, u32::from(pad::bridge::is_rep_button(b, input, mask)));
        same_memory("isRepButton", a, b);
    }
    eax(a, 0x00a4_2ba0, &[]);
    cd::bridge::pc_read_func_set(b);
    let packets = alloc(a, b, 0x80);
    set(a, b, pkt::bridge::PACKET_CURSOR, packets);
    for _ in 0..4 {
        let (x, y) = (rng.next(), rng.next());
        assert_eq!(
            eax(a, 0x008f_f020, &[x, y]),
            pkt::bridge::xyoffset_packet(b, x, y)
        );
    }
    same_memory("PC read table, packets", a, b);
    let text = alloc(a, b, 0x40);
    let copy = alloc(a, b, 0x40);
    for k in 0..40u32 {
        let len = (k % 30) as usize;
        let mut bytes: Vec<u8> = (0..len)
            .map(|_| {
                [b'\n', b'1', b' ', b'-', b'/', b'.', b'a', b'9', 0x81][(rng.next() % 9) as usize]
            })
            .collect();
        bytes.push(0);
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        let s = if k == 39 { 0 } else { text };
        assert_eq!(
            eax(a, 0x00a7_b1a0, &[copy, s]),
            msg::bridge::copy_cutting_lines(b, copy, s)
        );
        // the number font only takes digits, space, - . / (others hang it)
        let digits: Vec<u8> = bytes
            .iter()
            .map(|&c| {
                if c == 0 || b"0123456789 -./".contains(&c) {
                    c
                } else {
                    b'5'
                }
            })
            .collect();
        a.write(text, &digits).unwrap();
        b.write(text, &digits).unwrap();
        eax(a, 0x0089_e600, &[text, copy]);
        msg::bridge::number_font_string(b, text, copy);
        same_memory("strings", a, b);
    }
    // preloaded magic PRX table
    let counts = a.rebase(0x0193_fd44);
    for on in [0u32, 1] {
        set(a, b, Global::<u32>::new(0x0193_fd40), on);
        for i in 0..12u32 {
            put(a, b, counts + 4 * i, (rng.next() % 3).cast_signed() - 1);
        }
        let ids = a.rebase(0x00c6_4c18);
        for k in 0..30 {
            let id = if k % 3 == 0 {
                -1
            } else {
                Mem::read::<i32>(a, ids + 12 * (rng.next() % 12))
            };
            let o = if k % 4 == 0 { 0 } else { out };
            assert_eq!(
                eax(a, 0x009d_ad70, &[id.cast_unsigned(), o]),
                instance::bridge::find_preloaded(b, id, o)
            );
            same_memory("preloads", a, b);
        }
    }
}

/// `fiosUnifyFilename` over prefixes and paths with and without the PS3
/// names, `size` from roomy to too small; `get_hdd_name`; the prefix getter.
#[test]
fn unify_filename_matches_the_original() {
    use files::bridge as f;
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    let printf = a.rebase(0x0043_52f0);
    a.stub_return(printf, 0x5555).unwrap();
    let prefix = alloc(a, b, 0x40);
    let src = alloc(a, b, 0x100);
    let out = alloc(a, b, 0x100);
    let put_str = |a: &mut Oracle, b: &mut Oracle, at: u32, s: &[u8]| {
        let mut v = s.to_vec();
        v.push(0);
        a.write(at, &v).unwrap();
        b.write(at, &v).unwrap();
    };
    let prefixes: [&[u8]; 3] = [b"", b"data/", b"c:/game/ffx/"];
    let paths: [&[u8]; 8] = [
        b"chr/c001.cgfx.phyre",
        b"data/chr/c001.cgfx.phyre",
        b"GCM/tex/a.dds.phyre",
        b"data/GCM/x.cgfx.phyre",
        b"plain.bin",
        b"",
        b"c:/game/ffx/abc/GCMGCM.cgfx.phyre.cgfx.phyre",
        b"a.cgfx.phyreGCM",
    ];
    for pre in prefixes {
        put_str(a, b, prefix, pre);
        set(a, b, f::STREAM_PREFIX, prefix);
        assert_eq!(eax(a, 0x0060_7d30, &[]), f::stream_prefix(b));
        for path in paths {
            put_str(a, b, src, path);
            for size in [0x100u32, 0x30, 0x14, 8, 1] {
                let ra = eax(a, 0x0067_9820, &[src, out, size]);
                let mut printed = Vec::new();
                let rb =
                    f::unify_filename(b, src, out, size, |_, l, fm, s| printed.push((l, fm, s)));
                assert_eq!(ra, rb, "{pre:?} {path:?} {size}");
                assert_eq!(printed.len(), usize::from(path.len() > size as usize));
                same_memory("fiosUnifyFilename", a, b);
            }
        }
    }
    for n in [0, 1, 9, 10, -1, i32::MAX, i32::MIN] {
        assert_eq!(
            eax(a, 0x0077_7eb0, &[n.cast_unsigned()]),
            cd::bridge::get_hdd_name(b, n)
        );
        same_memory("get_hdd_name", a, b);
    }
}

/// Round 7: record resets, half swaps, touch records, map finds, sequence
/// slots, quad colours, message areas.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one pass; a, b: the oracles
fn round7(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use accessor::bridge as acc;
    use window::bridge as w;
    let recs = random_block(a, b, rng, 0x18 * 4);
    for (base, n) in [(recs, 4), (0, 1)] {
        assert_eq!(
            eax(a, 0x006f_bdc0, &[base, n]),
            acc::reset_records_18(b, base, n)
        );
    }
    let data = random_block(a, b, rng, 0x340);
    assert_eq!(eax(a, 0x0064_73d0, &[data]), acc::swap_halves(b, data));
    let touch = random_block(a, b, rng, 0x150);
    assert_eq!(
        call(a, 0x0068_9510, Conv::Thiscall(touch), &[]).eax,
        pad::bridge::touch_record_reset(b, touch)
    );
    same_memory("round 7 resets", a, b);
    let head = alloc(a, b, 0x10);
    let mut keys: Vec<u32> = (0..20).map(|_| rng.next() % 100).collect();
    keys.sort_unstable();
    keys.dedup();
    let root = build_tree(a, b, head, &keys);
    put(a, b, head + 0xc, root);
    let key = alloc(a, b, 4);
    for k in 0..100u32 {
        put(a, b, key, k);
        assert_eq!(
            call(a, 0x0069_1dc0, Conv::Thiscall(head), &[key]).eax,
            stl::bridge::map_find_u32(b, head, key)
        );
    }
    // sequence slots: names, states, then the clear
    let names: [&[u8]; 3] = [b"open\0", b"close\0", b"x\0"];
    let name = alloc(a, b, 0x20);
    for i in 0..8u32 {
        for j in 0..8u32 {
            let s = scene::bridge::SLOTS.ptr(b).addr + i * 0x100 + j * 0x20;
            let mut bytes = [0u8; 0x20];
            let nm = names[(rng.next() % 3) as usize];
            bytes[8..8 + nm.len()].copy_from_slice(nm);
            bytes[..2].copy_from_slice(&u16::try_from(rng.next() % 6).unwrap().to_le_bytes());
            put(a, b, s, bytes);
        }
    }
    for i in 0..8u32 {
        for nm in names.iter().chain([b"none\0".as_slice()].iter()) {
            let mut bytes = [0u8; 0x20];
            bytes[..nm.len()].copy_from_slice(nm);
            put(a, b, name, bytes);
            let ra = eax(a, 0x008a_e530, &[i, name]);
            assert_eq!(
                ra,
                scene::bridge::scene_state(b, i.cast_signed(), name).cast_unsigned()
            );
        }
    }
    assert_eq!(eax(a, 0x008a_df20, &[]), scene::bridge::clear_slots(b));
    same_memory("sequence slots", a, b);
    let quad = random_block(a, b, rng, 0x20);
    for _ in 0..20 {
        let k = rng.next() % 6;
        let c: [u32; 4] = std::array::from_fn(|_| rng.next());
        eax(a, 0x0090_47f0, &[quad, k, c[0], c[1], c[2], c[3]]);
        w::set_quad_rgba(b, quad, k, c.map(|v| v.to_le_bytes()[0]));
    }
    for i in 0..8u32 {
        let m = random_block(a, b, rng, 0x120);
        set_entry(a, b, w::MSG_WINDOWS, i, m);
        let r: [u32; 4] = std::array::from_fn(|_| rng.next() % 0x1000);
        eax(a, 0x008b_8b20, &[i, r[0], r[1], r[2], r[3]]);
        w::set_message_area(b, i.cast_signed(), r.map(u32::cast_signed));
    }
    same_memory("round 7 windows", a, b);
    // VFile seeks, header swaps, init, pair compares, tree inserts, entries
    let size_rec = random_block(a, b, rng, 0x10);
    let vf = random_block(a, b, rng, 0x18);
    put(a, b, vf + 0xc, size_rec);
    for n in 0..60u32 {
        put(
            a,
            b,
            size_rec + 8,
            [rng.next() % 0x1000, u32::from(n.is_multiple_of(7))],
        );
        let off = [
            0,
            1,
            0x800,
            0xfff,
            0x1001,
            -1i32,
            i32::MIN,
            rng.next().cast_signed(),
        ][(n % 8) as usize];
        let whence = n % 4;
        let ra = call(
            a,
            0x0061_ac30,
            Conv::Thiscall(vf),
            &[off.cast_unsigned(), whence],
        )
        .eax;
        assert_eq!(
            ra,
            files::bridge::vfile_seek(b, vf, off, whence),
            "seek {n}"
        );
        same_memory("VFile seek", a, b);
    }
    let hdr = random_block(a, b, rng, 0xf20);
    assert_eq!(eax(a, 0x0064_7220, &[hdr]), acc::swap_header_f00(b, hdr));
    assert_eq!(eax(a, 0x0064_7560, &[hdr]), acc::swap_header_84(b, hdr));
    let o = random_block(a, b, rng, 0x48);
    let (x, y) = (rng.next(), rng.next());
    assert_eq!(
        call(a, 0x0068_7190, Conv::Thiscall(o), &[x, y]).eax,
        acc::init_68(b, o, x, y)
    );
    same_memory("swaps and init", a, b);
    let (p, q) = (alloc(a, b, 0x10), alloc(a, b, 0x10));
    for _ in 0..100 {
        let v: [u32; 4] =
            std::array::from_fn(|_| [0, 1, u32::MAX, 0x8000_0000][(rng.next() % 4) as usize]);
        let mut wv = v;
        if rng.next().is_multiple_of(2) {
            wv[(rng.next() % 4) as usize] =
                [0, 1, u32::MAX, 0x8000_0000][(rng.next() % 4) as usize];
        }
        put(a, b, p, v);
        put(a, b, q, wv);
        let ra = call(a, 0x0061_d730, Conv::Stdcall, &[p, q]).eax;
        assert_eq!(ra, acc::compare_i64_pairs(b, p, q).cast_unsigned());
    }
    for sym in [0x0060_c990, 0x0069_1e60] {
        let head = alloc(a, b, 0x10);
        put(a, b, head + 0xc, head);
        for _ in 0..30 {
            let node = random_block(a, b, rng, 0x10);
            put(a, b, node + 0xc, rng.next() % 40);
            let ra = call(a, sym, Conv::Thiscall(head), &[node]).eax;
            assert_eq!(ra, stl::bridge::tagged_tree_insert(b, head, node));
            same_memory("tagged tree insert", a, b);
        }
    }
    let table = w::ENTRIES_18762E0.ptr(b).addr;
    for i in 0..16u32 {
        put(
            a,
            b,
            table + 16 * i,
            [0u32, 0, 0, if i % 3 == 0 { 0x0001_0000 } else { 0 }],
        );
    }
    for _ in 0..60 {
        let i = [0, 3, 15, 16, rng.next() % 16][(rng.next() % 5) as usize];
        let (x, y) = (rng.next() % 4, rng.next() % 4);
        let d = (rng.next() % 2000).cast_signed() - 1000;
        eax(a, 0x0090_0f80, &[i, x, y, d.cast_unsigned()]);
        w::move_entry_18762e0(b, i.cast_signed(), x.cast_signed(), y.cast_signed(), d);
        same_memory("entry moves", a, b);
    }
}

#[test]
fn round7_system_functions_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round7(&mut a, &mut b, &mut Xorshift(0x4007_0001));
}

#[test]
fn round7_system_functions_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round7(&mut a, &mut b, &mut Xorshift(0x4007_0002));
}

/// Round 7's one-line accessors, every table entry with random arguments.
#[test]
#[allow(clippy::many_single_char_names)] // a, b: the oracles
fn round7_accessor_tables_match_the_original() {
    use accessor::bridge as acc;
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    let mut rng = Xorshift(0x4007_0101);
    let obj = random_block(a, b, &mut rng, 0x20);
    let out = alloc(a, b, 4);
    for _ in 0..30 {
        let (x, y) = (rng.next(), rng.next());
        for f in accessor::SUB2 {
            assert_eq!(eax(a, f, &[x, y]), accessor::difference(x, y), "{f:#x}");
        }
        for f in accessor::ADD2 {
            assert_eq!(eax(a, f, &[x, y]), accessor::add(x, y), "{f:#x}");
        }
        for f in accessor::ADD_U16 {
            assert_eq!(eax(a, f, &[x, y]), accessor::add(x, y & 0xffff), "{f:#x}");
        }
        for f in accessor::ADD_U8 {
            assert_eq!(eax(a, f, &[x, y]), accessor::add(x, y & 0xff), "{f:#x}");
        }
        for f in accessor::ZERO_RET4 {
            assert_eq!(call(a, f, Conv::Stdcall, &[x]).eax, 0, "{f:#x}");
        }
        for f in accessor::STORE0_RET_THIS {
            assert_eq!(call(a, f, Conv::Thiscall(obj), &[x, y]).eax, obj, "{f:#x}");
            acc::set_member(b, obj, 0, x);
        }
        for f in accessor::COPY_THIS_TO_P {
            assert_eq!(
                call(a, f, Conv::Thiscall(obj), &[out]).eax,
                acc::copy_member_to(b, obj, 0, out)
            );
        }
        for f in accessor::COPY_THIS4_TO_P {
            assert_eq!(
                call(a, f, Conv::Thiscall(obj), &[out]).eax,
                acc::copy_member_to(b, obj, 4, out)
            );
        }
        for f in accessor::STORE8_RET_V {
            assert_eq!(
                call(a, f, Conv::Thiscall(obj), &[y]).eax,
                acc::set_member(b, obj, 8, y)
            );
        }
        put(a, b, obj + 4, rng.next());
        for (table, at, scale) in [
            (&accessor::INDEX4_AT8[..], 8, 4),
            (&accessor::INDEX4_AT4[..], 4, 4),
            (&accessor::INDEX8_AT8[..], 8, 8),
        ] {
            for &f in table {
                let i = rng.next() % 64;
                assert_eq!(
                    call(a, f, Conv::Thiscall(obj), &[i]).eax,
                    acc::index_member(b, obj, at, i, scale)
                );
            }
        }
        for (f, _) in accessor::WORD_SETTERS2 {
            assert_eq!(eax(a, f, &[x]), acc::set_word_global2(b, f, x));
        }
        for (f, _) in accessor::BYTE_SETTERS {
            eax(a, f, &[y]);
            acc::set_byte_global(b, f, y.to_le_bytes()[0]);
        }
        same_memory("round 7 accessor tables", a, b);
    }
}

/// The oracle's KERNEL32 stubs on the Rust side: the same handle numbers
/// (`0x100` up, four apart) and semaphore counts, every call recorded.
#[derive(Default)]
struct TestKernel {
    handles: u32,
    semaphores: std::collections::HashMap<u32, i32>,
    calls: Vec<(&'static str, u32, u32)>,
}

impl sync::Kernel<Oracle> for TestKernel {
    fn sleep(&mut self, _mem: &mut Oracle, ms: u32) -> u32 {
        self.calls.push(("Sleep", ms, 0));
        0
    }
    fn wait(&mut self, _mem: &mut Oracle, handle: u32, ms: u32) -> u32 {
        self.calls.push(("Wait", handle, ms));
        if let Some(n) = self.semaphores.get_mut(&handle)
            && *n > 0
        {
            *n -= 1;
        }
        0
    }
    fn release_semaphore(
        &mut self,
        mem: &mut Oracle,
        handle: u32,
        count: u32,
        previous: u32,
    ) -> u32 {
        self.calls.push(("Release", handle, count));
        let n = self.semaphores.entry(handle).or_insert(0);
        let old = *n;
        *n = n.wrapping_add(count.cast_signed());
        if previous != 0 {
            Mem::write(mem, previous, old.cast_unsigned());
        }
        1
    }
    fn create_semaphore(
        &mut self,
        _mem: &mut Oracle,
        _attributes: u32,
        initial: u32,
        maximum: u32,
        _name: u32,
    ) -> u32 {
        self.calls.push(("Create", initial, maximum));
        let h = 0x100 + 4 * self.handles;
        self.handles += 1;
        self.semaphores.insert(h, initial.cast_signed());
        h
    }
    fn close_handle(&mut self, _mem: &mut Oracle, handle: u32) -> u32 {
        self.calls.push(("Close", handle, 0));
        self.semaphores.remove(&handle);
        1
    }
    fn set_event(&mut self, _mem: &mut Oracle, handle: u32) -> u32 {
        self.calls.push(("SetEvent", handle, 0));
        1
    }
    fn reset_event(&mut self, _mem: &mut Oracle, handle: u32) -> u32 {
        self.calls.push(("ResetEvent", handle, 0));
        1
    }
    fn initialize_critical_section(&mut self, _mem: &mut Oracle, cs: u32) -> u32 {
        self.calls.push(("InitCS", cs, 0));
        0
    }
    fn enter_critical_section(&mut self, _mem: &mut Oracle, cs: u32) -> u32 {
        self.calls.push(("EnterCS", cs, 0));
        0
    }
    fn leave_critical_section(&mut self, _mem: &mut Oracle, cs: u32) -> u32 {
        self.calls.push(("LeaveCS", cs, 0));
        0
    }
    fn delete_critical_section(&mut self, _mem: &mut Oracle, cs: u32) -> u32 {
        self.calls.push(("DeleteCS", cs, 0));
        0
    }
}

/// The thread and synchronisation wrappers, run in sequence so the
/// semaphores the original creates are the ones the later calls use.
#[allow(clippy::too_many_lines)] // one script
fn sync_wrappers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use sync::bridge as s;
    let k = &mut TestKernel::default();
    for ms in [0, 1, 17] {
        assert_eq!(eax(a, 0x006f_2d20, &[ms]), s::sleep_ms(b, k, ms));
        assert_eq!(eax(a, 0x006f_2a40, &[ms]), s::sleep(b, k, ms));
    }
    assert_eq!(eax(a, 0x006f_2d10, &[]), s::sleep_thread(b, k));
    // semaphores: three objects, each a handle cell pointed at by a cell
    let objs = alloc(a, b, 0x60);
    let cells = alloc(a, b, 0x20);
    for i in 0..3 {
        let obj = objs + 0x20 * i;
        let n = 1 + rng.next() % 4;
        assert_eq!(
            call(a, 0x0067_9ab0, Conv::Thiscall(obj), &[n]).eax,
            s::semaphore_ctor(b, k, obj, n)
        );
        put(a, b, cells + 4 * i, obj);
    }
    same_memory("semaphores created", a, b);
    for _ in 0..40 {
        let i = rng.next() % 3;
        let cell = if rng.next().is_multiple_of(6) {
            0
        } else {
            cells + 4 * i
        };
        let ms = rng.next() % 100;
        match rng.next() % 3 {
            0 => assert_eq!(eax(a, 0x006f_2ce0, &[cell]), s::signal_sema(b, k, cell)),
            1 => assert_eq!(
                eax(a, 0x006f_2d80, &[cell]),
                s::wait_sema(b, k, cell, sync::INFINITE)
            ),
            _ => assert_eq!(
                eax(a, 0x006f_2da0, &[cell, ms]),
                s::wait_sema(b, k, cell, ms)
            ),
        }
    }
    same_memory("semaphore waits", a, b);
    // queue posts and request waits: the queue's handle is object 0's
    let request = random_block(a, b, rng, 0x60);
    for r in [request, 0] {
        assert_eq!(
            call(a, 0x0061_b500, Conv::Thiscall(objs), &[r]).eax,
            s::queue_post(b, k, objs, r)
        );
    }
    put(a, b, request + 0x4c, Mem::read::<u32>(a, objs + 0x20));
    assert_eq!(
        call(a, 0x0061_ba60, Conv::Thiscall(rng.next()), &[request]).eax,
        s::request_wait(b, k, request)
    );
    let guard = alloc(a, b, 8);
    assert_eq!(
        call(a, 0x0063_3a80, Conv::Thiscall(guard), &[objs + 0x40]).eax,
        s::mutex_guard(b, k, guard, objs + 0x40)
    );
    same_memory("queue and guard", a, b);
    // worker: its semaphore at +0x2c
    let worker = random_block(a, b, rng, 0x40);
    put(a, b, worker + 0x2c, Mem::read::<u32>(a, objs + 0x20));
    assert_eq!(
        call(a, 0x006d_b7a0, Conv::Thiscall(worker), &[]).eax,
        s::reset_counters(b, k, worker)
    );
    same_memory("worker reset", a, b);
    assert_eq!(
        call(a, 0x006d_b6e0, Conv::Thiscall(worker), &[]).eax,
        s::close_member(b, k, worker, 0x2c)
    );
    assert_eq!(
        call(a, 0x0063_4830, Conv::Thiscall(objs + 0x40), &[]).eax,
        s::close_member(b, k, objs + 0x40, 0)
    );
    // thread join: with and without a handle at +4
    let thread = random_block(a, b, rng, 0x10);
    put(a, b, thread + 4, Mem::read::<u32>(a, objs));
    for _ in 0..2 {
        assert_eq!(
            call(a, 0x0070_3a90, Conv::Thiscall(thread), &[]).eax,
            s::join_close(b, k, thread)
        );
        same_memory("thread join", a, b);
    }
    // events
    let event = alloc(a, b, 4);
    put(a, b, event, 0x1234u32);
    assert_eq!(
        eax(a, 0x006f_2bf0, &[event]),
        s::event_set(b, k, event, true)
    );
    assert_eq!(
        eax(a, 0x006f_2be0, &[event]),
        s::event_set(b, k, event, false)
    );
    // critical sections and their guard
    let cs = alloc(a, b, 0x18);
    let lock = alloc(a, b, 4);
    assert_eq!(
        call(a, 0x006f_b500, Conv::Thiscall(cs), &[]).eax,
        s::cs_init(b, k, cs)
    );
    assert_eq!(
        call(a, 0x006f_b510, Conv::Thiscall(lock), &[cs]).eax,
        s::cs_guard(b, k, lock, cs)
    );
    assert_eq!(
        call(a, 0x006f_b540, Conv::Thiscall(lock), &[]).eax,
        s::cs_guard_drop(b, k, lock)
    );
    assert_eq!(
        call(a, 0x006f_b530, Conv::Thiscall(cs), &[]).eax,
        s::cs_delete(b, k, cs)
    );
    same_memory("events and critical sections", a, b);
    assert!(k.calls.len() > 50);
}

#[test]
fn sync_wrappers_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    sync_wrappers(&mut a, &mut b, &mut Xorshift(0x4008_0001));
}

#[test]
fn sync_wrappers_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    sync_wrappers(&mut a, &mut b, &mut Xorshift(0x4008_0002));
}

/// The `PClassDescriptorForType<T>` destructors over a stand-in parent
/// (records `this`, returns it xor a key): the vtable store and the tail
/// call, on every table entry.
#[test]
#[allow(clippy::many_single_char_names)] // a, b: the oracles
fn class_descriptor_destructors_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    let mut rng = Xorshift(0x4008_0101);
    let cell = alloc(a, b, 4);
    // mov [cell], ecx; mov eax, ecx; xor eax, key; ret
    let key = 0x5a5a_1234u32;
    let mut code = vec![0x89, 0x0d];
    code.extend_from_slice(&cell.to_le_bytes());
    code.extend_from_slice(&[0x89, 0xc8, 0x35]);
    code.extend_from_slice(&key.to_le_bytes());
    code.push(0xc3);
    let parent = stand_in(a, b, 0, &code);
    redirect(a, b, dtor::CLASS_DESCRIPTOR_DTOR, parent);
    for (f, vtable) in dtor::CLASS_DESCRIPTOR_DTORS {
        let this = random_block(a, b, &mut rng, 0x80);
        let got = dtor::bridge::chain(b, this, vtable, |m, this| {
            let at = m.rebase(dtor::CLASS_DESCRIPTOR_DTOR);
            m.call(at, Conv::Thiscall(this), &[]).unwrap().eax
        });
        assert_eq!(call(a, f, Conv::Thiscall(this), &[]).eax, got, "{f:#x}");
        assert_eq!(Mem::read::<u32>(a, this), a.rebase(vtable));
        same_memory("class descriptor destructors", a, b);
    }
}

/// Code space for round 8's stand-ins: `0x006090b0`, a 4.7 KB function
/// these tests never reach, overwritten in both oracles (the padding after
/// `.text` that [`stand_in`] uses ends at the import table).
const ROUND8_CODE: u32 = 0x0060_90b0;

/// Writes `code` at `ROUND8_CODE + off` in both oracles; its address.
fn stand_in8(a: &mut Oracle, b: &mut Oracle, off: u32, code: &[u8]) -> u32 {
    assert!(off as usize + code.len() <= 0x1200, "inside 0x006090b0");
    let at = a.rebase(ROUND8_CODE) + off;
    a.write(at, code).unwrap();
    b.write(at, code).unwrap();
    at
}

/// Round 8's system functions.
#[allow(clippy::many_single_char_names)] // a, b: the oracles
fn round8(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    // the TOAdp writers' cursor forms
    let buf = alloc(a, b, 0x400);
    for (f, writer, n) in pkt::bridge::ADP_AT_CURSOR {
        let at = buf + 0x10 * (rng.next() % 0x30);
        set(a, b, pkt::bridge::PACKET_CURSOR, at);
        let args: Vec<u32> = (0..n).map(|_| rng.next()).collect();
        assert_eq!(
            eax(a, f, &args),
            pkt::bridge::adp_at_cursor(b, writer, &args),
            "{f:#x}"
        );
        same_memory("TOAdp cursor forms", a, b);
    }
    round8_containers(a, b, rng);
    round8_windows_and_menus(a, b, rng);
}

/// A cdecl stand-in appending its first argument xor `key` to the log whose
/// cursor is at `cursor`.
fn logger(cursor: u32, key: u32) -> Vec<u8> {
    let mut v = vec![0x8b, 0x44, 0x24, 0x04, 0x35]; // mov eax, [esp+4]; xor eax, key
    v.extend_from_slice(&key.to_le_bytes());
    v.extend_from_slice(&[0x8b, 0x0d]); // mov ecx, [cursor]
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x01, 0x83, 0x05]); // mov [ecx], eax; add dword [cursor], 4
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x04, 0xc3]);
    v
}

/// The container helpers over logging stand-ins for `operator delete` and
/// `PhyreFree`.
#[allow(clippy::too_many_lines)] // one script
fn round8_containers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use container::bridge as c;
    let log = alloc(a, b, 0x800);
    let cursor = alloc(a, b, 4);
    put(a, b, cursor, log);
    let del = stand_in8(a, b, 0x200, &logger(cursor, 0xde1e_7e00));
    let free = stand_in8(a, b, 0x240, &logger(cursor, 0xf4ee_0000));
    redirect(a, b, 0x0063_0390, del);
    redirect(a, b, 0x0042_fb20, free);
    let b_delete = |m: &mut Oracle, p: u32| {
        let at = m.rebase(0x0063_0390);
        m.call(at, Conv::Cdecl, &[p]).unwrap();
    };
    let b_free = |m: &mut Oracle, p: u32| {
        let at = m.rebase(0x0042_fb20);
        m.call(at, Conv::Cdecl, &[p]).unwrap();
    };
    for _ in 0..8 {
        let s = random_block(a, b, rng, 0x18);
        put(
            a,
            b,
            s + 0x14,
            [0x0fu32, 0x10, 0x40][(rng.next() % 3) as usize],
        );
        for f in container::STRING_TIDY {
            put(a, b, s, rng.next());
            call(a, f, Conv::Thiscall(s), &[]);
            c::string_tidy(b, s, b_delete);
            same_memory("string tidy", a, b);
        }
        for f in container::VECTOR_TIDY {
            put(a, b, s, [0, rng.next()][(rng.next() % 2) as usize]);
            call(a, f, Conv::Thiscall(s), &[]);
            c::vector_tidy(b, s, b_delete);
            same_memory("vector tidy", a, b);
        }
        for (f, null_check) in container::OWNED_RESET {
            let count = rng.next() & 0x8000_00ff;
            let p = [0, rng.next()][(rng.next() % 2) as usize];
            put(a, b, s, [count, p]);
            call(a, f, Conv::Thiscall(s), &[]);
            c::owned_reset(b, s, null_check, b_free);
            same_memory("owned array reset", a, b);
        }
        let n = rng.next() % 6;
        let items = random_block(a, b, rng, 0x30);
        for it in [items, 0] {
            eax(a, 0x0069_a130, &[it, n]);
            c::free_owned_array(b, it, n, b_free);
        }
        same_memory("owned array free", a, b);
        // a parser state with three buffers and maybe an owned block
        let st = random_block(a, b, rng, 0x60);
        for off in [8, 0xc, 0x10] {
            let buf = alloc(a, b, 4);
            put(a, b, st + off, buf);
        }
        put(a, b, st + 0x1c, rng.next() % 2);
        call(a, 0x0061_c270, Conv::Thiscall(st), &[]);
        c::parser_reset(b, st, b_delete);
        same_memory("parser reset", a, b);
    }
    // a std::list<std::string> of up to five nodes
    for len in [0usize, 1, 5] {
        let list = alloc(a, b, 8);
        let head = alloc(a, b, 0x20);
        let nodes: Vec<u32> = (0..len).map(|_| random_block(a, b, rng, 0x20)).collect();
        let mut cycle = vec![head];
        cycle.extend(&nodes);
        for (k, &n) in cycle.iter().enumerate() {
            put(a, b, n, cycle[(k + 1) % cycle.len()]);
            put(a, b, n + 4, cycle[(k + cycle.len() - 1) % cycle.len()]);
        }
        for &n in &nodes {
            put(a, b, n + 0x1c, [0x0fu32, 0x20][(rng.next() % 2) as usize]);
        }
        put(a, b, list, [head, u32::try_from(len).unwrap()]);
        call(a, 0x0068_6da0, Conv::Thiscall(list), &[]);
        c::string_list_clear(b, list, b_delete);
        same_memory("string list clear", a, b);
    }
    // the map head: operator new redirected to a fixed block
    let block = alloc(a, b, 0x18);
    let mut code = vec![0xb8];
    code.extend_from_slice(&block.to_le_bytes());
    code.push(0xc3);
    let new = stand_in8(a, b, 0x280, &code);
    redirect(a, b, 0x0063_0310, new);
    let b_new = |m: &mut Oracle, size: u32| {
        let at = m.rebase(0x0063_0310);
        m.call(at, Conv::Cdecl, &[size]).unwrap().eax
    };
    assert_eq!(
        eax(a, 0x0069_a1b0, &[]),
        c::map_head_new(b, b_new, |_| unreachable!("allocated"))
    );
    same_memory("map head", a, b);
    // Phyre array appends into new buffers
    for f in [0x0062_9b90, 0x0069_b530, 0x0069_b5c0, 0x0069_b650] {
        for _ in 0..4 {
            let arr = alloc(a, b, 8);
            let n = rng.next() % 6;
            let old = random_block(a, b, rng, 0x20);
            let flag = [0, 0x8000_0000][(rng.next() % 2) as usize];
            put(a, b, arr, [n | flag, old]);
            let dst = [0, alloc(a, b, 0x20), old][(rng.next() % 3) as usize];
            let value = random_block(a, b, rng, 4);
            assert_eq!(
                call(a, f, Conv::Thiscall(arr), &[dst, value]).eax,
                c::parray_push_into(b, arr, dst, value, b_free)
            );
            same_memory("array append", a, b);
        }
    }
    let obj = random_block(a, b, rng, 0x160);
    for flags in [[0u32, 0], [0x8000_0001, 3], [2, 1]] {
        put(a, b, obj + 0x150, [flags[0], rng.next() % 2 * rng.next()]);
        put(a, b, obj + 0x158, [flags[1], rng.next()]);
        call(a, 0x0068_9620, Conv::Thiscall(obj), &[]);
        c::reset_arrays_689620(b, obj, b_free);
        same_memory("two array resets", a, b);
    }
    // an array of objects whose deleting destructors log `this ^ flag`
    #[rustfmt::skip]
    let mut dtor = vec![0x8b, 0x44, 0x24, 0x04, 0x31, 0xc8, 0x8b, 0x15]; // mov eax, [esp+4]; xor eax, ecx
    dtor.extend_from_slice(&cursor.to_le_bytes());
    dtor.extend_from_slice(&[0x89, 0x02, 0x83, 0x05]); // mov [edx], eax; add [cursor], 4
    dtor.extend_from_slice(&cursor.to_le_bytes());
    dtor.extend_from_slice(&[0x04, 0xc2, 0x04, 0x00]);
    let dtor = stand_in8(a, b, 0x2c0, &dtor);
    let vtable = alloc(a, b, 4);
    put(a, b, vtable, dtor);
    for n in [0u32, 1, 4] {
        let arr = alloc(a, b, 8);
        let data = alloc(a, b, 0x10);
        for k in 0..4 {
            let o = if rng.next().is_multiple_of(3) {
                0
            } else {
                alloc(a, b, 4)
            };
            if o != 0 {
                put(a, b, o, vtable);
            }
            put(a, b, data + 4 * k, o);
        }
        put(
            a,
            b,
            arr,
            [n | [0, 0x8000_0000][(rng.next() % 2) as usize], data],
        );
        call(a, 0x009e_2420, Conv::Thiscall(arr), &[]);
        c::parray_delete_all(
            b,
            arr,
            |m, o| {
                m.call(dtor, Conv::Thiscall(o), &[1]).unwrap();
            },
            b_free,
        );
        same_memory("array of objects deleted", a, b);
    }
    let logged = (Mem::read::<u32>(a, cursor) - log) / 4;
    assert!((50..0x200).contains(&logged), "{logged} frees logged");
}

/// Window, menu, message, CD-table, counter and float functions.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one script; a, b: the oracles
fn round8_windows_and_menus(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use window::bridge as w;
    let cells = alloc(a, b, 0x10);
    put(a, b, cells, [0u32; 4]);
    let fns: Vec<u32> = (0..4u32)
        .map(|k| stand_in8(a, b, 0x300 + 0x10 * k, &recorder(cells + 4 * k, 0x7100 + k)))
        .collect();
    let b_call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    for i in 0..8u32 {
        let win = random_block(a, b, rng, 0x80);
        put(a, b, win + 0x60, [fns[0], fns[1], fns[2]]);
        set_entry(a, b, w::WINDOWS, i, win);
        let msg = random_block(a, b, rng, 0x120);
        put(a, b, msg + 0x100, fns[2]);
        put(a, b, msg + 0xf8, fns[3]);
        set_entry(a, b, w::MSG_WINDOWS, i, msg);
        let slot = alloc(a, b, 0x1c);
        for off in [0x10, 0x14, 0x18] {
            let part = random_block(a, b, rng, 0x30);
            put(a, b, slot + off, part);
        }
        put(a, b, w::MSG_WINDOW_SLOTS.ptr(b).addr + 4 * i, slot);
    }
    for i in 0..8u32 {
        let k = i.cast_signed();
        assert_eq!(
            eax(a, 0x008a_de10, &[i]),
            w::open_window_and_message(b, k, b_call)
        );
        same_memory("window and message opening", a, b);
    }
    assert_eq!(eax(a, 0x008a_b590, &[]), w::all_clear_message_windows(b));
    same_memory("all message windows cleared", a, b);
    // the yes/no window: none, state 2 (its scene stopped), another state
    for state in [None, Some(2u32), Some(5)] {
        let win = match state {
            None => 0,
            Some(s) => {
                let win = random_block(a, b, rng, 0xa0);
                put(a, b, win + 0x94, s);
                win
            }
        };
        set(a, b, w::YES_NO_WINDOW, win);
        assert_eq!(eax(a, 0x008e_2de0, &[]), w::menu_close_yes_no_window(b));
        same_memory("yes/no window close", a, b);
    }
    // HP added with the cap, through the original menu functions on b
    let orig = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().eax
    };
    for _ in 0..20 {
        let chr = rng.next() % 8;
        let delta = (rng.next() % 4000).wrapping_sub(1000);
        assert_eq!(
            eax(a, 0x008c_8d10, &[chr, delta]),
            menu::bridge::add_hp_capped(
                b,
                chr,
                delta,
                |m, c| orig(m, 0x008a_98c0, &[c]),
                |m, c| orig(m, 0x008a_9990, &[c]),
                |m, c, v| orig(m, 0x008a_ae90, &[c, v]),
            )
        );
        same_memory("HP add", a, b);
    }
    // analog history and product codes
    for _ in 0..10 {
        let args = [
            rng.next() % 2,
            rng.next() % 2,
            rng.next() % 4,
            rng.next() % 4,
        ];
        assert_eq!(
            eax(a, 0x008b_e450, &args),
            menu::bridge::control_pad_analog_hist(
                b,
                args,
                |m| orig(m, 0x008c_a510, &[]),
                |m, x| orig(m, 0x0088_8c20, &x),
            )
        );
    }
    for lang in 0..20i8 {
        set(a, b, ffx_menu::tk::bridge::LANG, lang);
        assert_eq!(
            eax(a, 0x0077_9ca0, &[]),
            menu::bridge::save_product_code(b, i32::from(lang))
        );
        for on in [0, 1, 2] {
            eax(a, 0x008b_0920, &[on]);
            menu::bridge::swap_international_env_init(b, on, |m, e| orig(m, 0x0078_5ed0, &[e]));
            same_memory("international env", a, b);
        }
    }
    // scene positions by name, through the menu's original on b
    let names: [&[u8]; 3] = [b"alpha\0", b"beta\0", b"alp\0"];
    let name_at: Vec<u32> = names
        .iter()
        .map(|n| {
            let p = alloc(a, b, 8);
            a.write(p, n).unwrap();
            b.write(p, n).unwrap();
            p
        })
        .collect();
    let slots = scene::bridge::SLOTS.ptr(b).addr;
    for i in 0..2u32 {
        for j in 0..8 {
            let sl = slots + i * 0x100 + j * 0x20;
            let n = names[(rng.next() % 3) as usize];
            a.write(sl + 8, n).unwrap();
            b.write(sl + 8, n).unwrap();
            put(a, b, sl + 0x1c, rng.next() % 4);
        }
    }
    for _ in 0..6 {
        let i = rng.next() % 2;
        let name = name_at[(rng.next() % 3) as usize];
        let (x, y) = (rng.next() % 640, rng.next() % 448);
        eax(a, 0x008a_f060, &[i, name, x, y]);
        scene::bridge::set_scene_pos_by_name(b, i.cast_signed(), name, x, y, |m, i, h, x, y| {
            orig(m, 0x00a7_c810, &[i.cast_unsigned(), h, x, y]);
        });
        same_memory("scene positions by name", a, b);
    }
    // the first screen's projection, the identity through the original on b
    let mtx = alloc(a, b, 0x40);
    let screen = math::bridge::SCREEN0.ptr(b).addr;
    for _ in 0..200 {
        for k in [2u32, 3, 4, 5, 7] {
            let v = rng.float().to_bits();
            let v = if ffx_bridge::x87::is_snan(v) {
                v | 0x0040_0000
            } else {
                v
            };
            put(a, b, screen + 4 * k, v);
        }
        eax(a, 0x0076_6ae0, &[mtx]);
        math::bridge::screen_projection(b, mtx, |m, p| {
            orig(m, 0x006e_d9a0, &[p]);
        });
        same_memory("screen projection", a, b);
    }
    // language strings, TOGetFFXLang redirected to return a cell (the
    // language manager is not set up here); the second env init
    let lang_cell = alloc(a, b, 4);
    let mut code = vec![0xa1];
    code.extend_from_slice(&lang_cell.to_le_bytes());
    code.push(0xc3);
    let get_lang = stand_in8(a, b, 0x3e0, &code);
    redirect(a, b, 0x0064_10e0, get_lang);
    for lang in 0..20u32 {
        put(a, b, lang_cell, lang);
        set(
            a,
            b,
            ffx_menu::tk::bridge::LANG,
            i8::try_from(lang).unwrap(),
        );
        assert_eq!(
            eax(a, 0x008a_c300, &[]),
            menu::bridge::ffx_lang_str(b, lang)
        );
        let on = lang % 3;
        eax(a, 0x008b_0970, &[on]);
        menu::bridge::swap_international_env_init2(b, on, |m, e| orig(m, 0x0078_5ed0, &[e]));
        same_memory("language strings", a, b);
    }
    // the menu's script request over recording stand-ins for the script
    // module's three (its controllers are not set up here)
    let args5 = alloc(a, b, 0x20);
    let mut regist = Vec::new();
    for k in 0..5u32 {
        regist.extend_from_slice(&[0x8b, 0x44, 0x24, u8::try_from(4 + 4 * k).unwrap(), 0xa3]);
        regist.extend_from_slice(&(args5 + 4 * k).to_le_bytes());
    }
    regist.extend_from_slice(&[0xb8, 0x55, 0, 0, 0, 0xc3]);
    let regist = stand_in8(a, b, 0x400, &regist);
    redirect(a, b, 0x0086_ec40, regist);
    let change = stand_in8(a, b, 0x440, &recorder(args5 + 0x14, 0x6600));
    redirect(a, b, 0x0086_2d30, change);
    let restore = stand_in8(a, b, 0x460, &recorder(args5 + 0x18, 0x7700));
    redirect(a, b, 0x0086_f730, restore);
    for _ in 0..5 {
        let args = [rng.next() % 8, rng.next() % 16, rng.next() % 8];
        assert_eq!(
            eax(a, 0x008b_e370, &args),
            menu::bridge::menu_atel_req(
                b,
                args,
                |m, c| orig(m, 0x0086_2d30, &[c]),
                |m, x| {
                    let at = m.rebase(0x0086_ec40);
                    m.call(at, Conv::Cdecl, &x).unwrap().eax
                },
                |m, c| orig(m, 0x0086_f730, &[c]),
            )
        );
        same_memory("menu script request", a, b);
    }
    // scene colour parameters, through the menu's originals on b
    let rgba = alloc(a, b, 4);
    for _ in 0..6 {
        let i = rng.next() % 2;
        let name = name_at[(rng.next() % 3) as usize];
        let out = [rgba, rgba + 1, rgba + 2, rgba + 3];
        let mut args = vec![i, name];
        args.extend(out);
        assert_eq!(
            eax(a, 0x008a_e3a0, &args),
            scene::bridge::get_color_rgba(
                b,
                i.cast_signed(),
                name,
                out,
                |m, i, n| orig(m, 0x00a7_b7d0, &[i.cast_unsigned(), n]),
                |m, i, h, n| orig(m, 0x00a7_b8c0, &[i.cast_unsigned(), h, n]),
            )
        );
        same_memory("scene colours", a, b);
    }
    // listed ids
    for id in 0x400..0x480 {
        assert_eq!(
            eax(a, 0x0090_66f0, &[id]),
            msg::bridge::shift_listed_id(b, id.cast_signed()).cast_unsigned()
        );
    }
    // damage values and target bits
    for start in [0i16, 10, 62, 63, 70] {
        set(a, b, btl::bridge::DAMAGE_COUNT, start);
        let args: Vec<u32> = (0..8).map(|_| rng.next()).collect();
        assert_eq!(
            eax(a, 0x0089_9a40, &args),
            btl::bridge::set_damage_value(b, args.clone().try_into().unwrap()).cast_unsigned()
        );
        same_memory("damage values", a, b);
    }
    for _ in 0..10 {
        for list in btl::bridge::TARGET_LISTS {
            let at = b.rebase(list);
            let n = [0u32, 1, 3, 8][(rng.next() % 4) as usize];
            put(a, b, at, n);
            for i in 0..8 {
                put(
                    a,
                    b,
                    at + 4 + 2 * i,
                    u16::try_from(rng.next() % 40).unwrap(),
                );
            }
        }
        put(
            a,
            b,
            btl::bridge::TARGET_MASKS.ptr(b).addr,
            [rng.next() % 16, rng.next() % 16, 0],
        );
        assert_eq!(
            eax(a, 0x0089_85b0, &[]),
            btl::bridge::make_target_list_bit(b).cast_unsigned()
        );
        same_memory("target bits", a, b);
    }
    // the menu's message windows controlled (set up as for round 5's
    // control functions), both pad states
    for i in 0..8u32 {
        let win = random_block(a, b, rng, 0x80);
        put(a, b, win, u16::try_from(rng.next() % 2).unwrap());
        put(a, b, win + 0x74, fns[0]);
        put(a, b, win + 0x78, fns[1]);
        set_entry(a, b, w::WINDOWS, i, win);
        let msg = random_block(a, b, rng, 0x120);
        put(a, b, msg, u16::try_from(rng.next() % 3).unwrap());
        put(a, b, msg + 0x1e, u16::try_from(rng.next() % 2).unwrap());
        put(a, b, msg + 0x2c, u16::try_from(1 + rng.next() % 2).unwrap());
        let flag = alloc(a, b, 4);
        put(a, b, flag, u32::from(rng.next().is_multiple_of(2)));
        put(a, b, msg + 0x98, flag);
        put(a, b, msg + 0x100, [fns[2], 0, fns[3], fns[2]]);
        put(a, b, msg + 0x114, fns[2]);
        put(a, b, msg + 0xf8, fns[3]);
        put(a, b, msg + 0xfc, fns[2]);
        set_entry(a, b, w::MSG_WINDOWS, i, msg);
        let e = random_block(a, b, rng, 0x20);
        put(a, b, e + 1, [i8::try_from(rng.next() % 3).unwrap() - 1, 5]);
        set_entry(a, b, w::TABLE_01868A90, i, e);
        let slot = alloc(a, b, 0x1c);
        let part = alloc(a, b, 0x10);
        put(a, b, part + 8, u16::try_from(rng.next() % 4).unwrap());
        put(
            a,
            b,
            slot,
            [
                u16::try_from(rng.next() % 2).unwrap(),
                0,
                0,
                u16::try_from(rng.next() % 2).unwrap(),
            ],
        );
        put(a, b, slot + 0x10, part);
        put(a, b, w::MSG_WINDOW_SLOTS.ptr(b).addr + 4 * i, slot);
    }
    for pad in [[0u16, 0], [0x100, 0x100], [0x100, 0]] {
        put(a, b, ffx_system::pad::bridge::FIRST.ptr(b).addr, pad);
        assert_eq!(
            eax(a, 0x008a_b9a0, &[]),
            w::control_message_window_for_menu(b, b_call)
        );
        same_memory("message windows for the menu", a, b);
    }
    // control codes skipped
    let text = alloc(a, b, 0x40);
    for _ in 0..20 {
        let mut bytes: Vec<u8> = (0..0x3f).map(|_| (rng.next() % 6) as u8 + 1).collect();
        bytes.push(0);
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        assert_eq!(
            eax(a, 0x008b_7f40, &[text]),
            msg::bridge::skip_codes_above_2(b, text)
        );
    }
    // CD-table loads: recorders in the table, user_malloc a fixed block
    let table = cd::bridge::CD_LIB.ptr(b).addr;
    for (k, slot) in [0u32, 8, 0x10, 0x14].into_iter().enumerate() {
        let k = u32::try_from(k).unwrap();
        let fun = stand_in8(
            a,
            b,
            0x380 + 0x10 * k,
            &recorder(cells + 4 * k, 0x200 * (k + 1)),
        );
        put(a, b, table + slot, fun);
    }
    let block = alloc(a, b, 0x10);
    let mut code = vec![0xb8];
    code.extend_from_slice(&block.to_le_bytes());
    code.push(0xc3);
    let malloc = stand_in8(a, b, 0x3c0, &code);
    redirect(a, b, 0x0068_7020, malloc);
    for name in [0x600, rng.next(), rng.next()] {
        let (file, buf) = (rng.next(), rng.next());
        assert_eq!(
            eax(a, 0x008a_ba60, &[file, name]),
            files::bridge::dvd_file_load(b, file, name, b_call, |m, n| orig(m, 0x0068_7020, &[n]))
        );
        assert_eq!(
            eax(a, 0x008a_bab0, &[file, name, buf]),
            files::bridge::dvd_file_start_read(b, file, name, buf, b_call)
        );
        same_memory("CD-table loads", a, b);
    }
    // counters
    for _ in 0..10 {
        set(a, b, save::bridge::COUNTERS_PENDING, rng.next() % 1000);
        let k: i32 = [-1, 0, 5, 13][(rng.next() % 4) as usize];
        assert_eq!(
            eax(a, 0x0082_1a40, &[k.cast_unsigned()]),
            save::bridge::counters_flush(b, k)
        );
        set(a, b, save::bridge::COUNTERS_PENDING, rng.next() % 1000);
        assert_eq!(eax(a, 0x0082_1a70, &[]), save::bridge::counters_snapshot(b));
        same_memory("counters", a, b);
    }
    // (a / b)^2 through st(0)
    for _ in 0..200 {
        let x = rng.next().cast_signed() >> (rng.next() % 31);
        let y =
            [0, 1, -1, rng.next().cast_signed() >> (rng.next() % 31)][(rng.next() % 4) as usize];
        let st0 = call(
            a,
            0x008f_38c0,
            Conv::Cdecl,
            &[x.cast_unsigned(), y.cast_unsigned()],
        )
        .st0
        .expect("a float result");
        let r = math::bridge::ratio_squared(x, y);
        // ST0 comes back as an f64 (exact for an f32, NaN payloads kept)
        #[allow(clippy::cast_possible_truncation)] // exact: an f32 value
        let st0 = st0 as f32;
        assert_eq!(st0.to_bits(), r.to_bits(), "{x} / {y}");
    }
}

#[test]
fn round8_system_functions_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round8(&mut a, &mut b, &mut Xorshift(0x4008_0011));
}

#[test]
fn round8_system_functions_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round8(&mut a, &mut b, &mut Xorshift(0x4008_0012));
}

/// A cdecl comparator stand-in: `*a < *b` (signed first words), each call's
/// two keys appended to the log at `*cursor`, so the comparison sequence is
/// compared too.
fn logging_less(cursor: u32) -> Vec<u8> {
    let mut v = vec![
        0x8b, 0x44, 0x24, 0x04, 0x8b, 0x00, 0x8b, 0x4c, 0x24, 0x08, 0x8b, 0x09,
    ];
    v.extend_from_slice(&[0x8b, 0x15]); // mov edx, [cursor]
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x02, 0x89, 0x4a, 0x04, 0x83, 0x05]); // keys; add [cursor], 8
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x08, 0x39, 0xc8, 0x0f, 0x9c, 0xc0, 0xc3]); // cmp; setl al; ret
    v
}

/// One instantiation set (`size`-byte records) against the original.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one script; a, b: the oracles
fn sort_family(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    size: u32,
    table: &[(u32, sort::Algo)],
) {
    use sort::Algo;
    let log = alloc(a, b, 0x2_0000);
    let cursor = alloc(a, b, 4);
    let less = stand_in8(a, b, 0x400 + size * 4, &logging_less(cursor));
    let scratch = alloc(a, b, 28);
    let records = alloc(a, b, (size * 400) as usize);
    let val = alloc(a, b, 28);
    let result = alloc(a, b, 8);
    let fill = |a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, n: u32, keys: u32| {
        for i in 0..n * size / 4 {
            let w = if i % (size / 4) == 0 {
                (rng.next() % keys)
                    .cast_signed()
                    .wrapping_sub(10)
                    .cast_unsigned()
            } else {
                rng.next()
            };
            put(a, b, records + 4 * i, w);
        }
        for k in 0..7 {
            put(a, b, val + 4 * k, rng.next() % keys);
        }
    };
    let find = |algo: Algo| table.iter().filter(move |e| e.1 == algo).map(|e| e.0);
    let mut sorter = sort::bridge::Sorter {
        size,
        tmp: scratch,
        less: move |m: &mut Oracle, x: u32, y: u32| {
            m.call(less, Conv::Cdecl, &[x, y]).unwrap().eax & 0xff != 0
        },
    };
    let sync = |a: &mut Oracle, b: &mut Oracle, what: &str| {
        let t: [u32; 7] = Mem::read(b, scratch);
        Mem::write(a, scratch, t);
        same_memory(what, a, b);
        put(a, b, cursor, log);
    };
    put(a, b, cursor, log);
    for &(n, keys) in &[
        (0u32, 5u32),
        (1, 5),
        (2, 5),
        (5, 3),
        (33, 4),
        (41, 50),
        (100, 1000),
        (300, 7),
        (380, 100_000),
    ] {
        let last = records + n * size;
        // the sort itself (ideal = n, as std::sort passes)
        for f in find(Algo::Sort) {
            fill(a, b, rng, n, keys);
            eax(a, f, &[records, last, n, less]);
            sorter.sort(b, records, last, n.cast_signed());
            sync(a, b, "std::_Sort");
        }
        for f in find(Algo::StdSort) {
            fill(a, b, rng, n, keys);
            eax(a, f, &[records, last, less]);
            sorter.std_sort(b, records, last);
            sync(a, b, "std::sort");
        }
        // a small division budget forces the heap sort
        for f in find(Algo::Sort) {
            fill(a, b, rng, n, keys);
            eax(a, f, &[records, last, 1, less]);
            sorter.sort(b, records, last, 1);
            sync(a, b, "std::_Sort, heap fallback");
        }
        for f in find(Algo::InsertionSort) {
            fill(a, b, rng, n.min(60), keys);
            let l = records + n.min(60) * size;
            eax(a, f, &[records, l, less]);
            sorter.insertion_sort(b, records, l);
            sync(a, b, "std::_Insertion_sort");
        }
        for f in find(Algo::MakeHeap) {
            fill(a, b, rng, n, keys);
            eax(a, f, &[records, last, less]);
            sorter.make_heap(b, records, last);
            sync(a, b, "std::_Make_heap");
        }
        if n >= 1 {
            for f in find(Algo::SortHeap) {
                eax(a, f, &[records, last, less]);
                sorter.sort_heap(b, records, last);
                sync(a, b, "std::sort_heap");
                fill(a, b, rng, n, keys);
                eax(
                    a,
                    find(Algo::MakeHeap).next().unwrap(),
                    &[records, last, less],
                );
                sorter.make_heap(b, records, last);
                sync(a, b, "std::_Make_heap again");
            }
            for f in find(Algo::PopHeap) {
                eax(a, f, &[records, last, less]);
                sorter.pop_heap(b, records, last);
                sync(a, b, "std::_Pop_heap");
            }
            for f in find(Algo::PopHeapTo) {
                let dest = records + (n - 1) * size;
                eax(a, f, &[records, dest, dest, val, less]);
                sorter.pop_heap_to(b, records, dest, dest, val);
                sync(a, b, "std::_Pop_heap to");
            }
            for f in find(Algo::AdjustHeap) {
                let hole = (rng.next() % n).cast_signed();
                eax(a, f, &[records, hole.cast_unsigned(), n, val, less]);
                sorter.adjust_heap(b, records, hole, n.cast_signed(), val);
                sync(a, b, "std::_Adjust_heap");
            }
            for f in find(Algo::PushHeap) {
                let hole = (rng.next() % n).cast_signed();
                let top = (rng.next() % n).cast_signed().min(hole);
                eax(
                    a,
                    f,
                    &[
                        records,
                        hole.cast_unsigned(),
                        top.cast_unsigned(),
                        val,
                        less,
                    ],
                );
                sorter.push_heap(b, records, hole, top, val);
                sync(a, b, "std::_Push_heap");
            }
            for f in find(Algo::Med3) {
                let pick = |rng: &mut Xorshift| records + (rng.next() % n) * size;
                let (x, y, z) = (pick(rng), pick(rng), pick(rng));
                eax(a, f, &[x, y, z, less]);
                sorter.med3(b, x, y, z);
                sync(a, b, "std::_Med3");
            }
            for f in find(Algo::Median) {
                fill(a, b, rng, n, keys);
                let mid = records + (n / 2) * size;
                let l = records + (n - 1) * size;
                eax(a, f, &[records, mid, l, less]);
                sorter.median(b, records, mid, l);
                sync(a, b, "std::_Median");
            }
        }
        if n >= 2 {
            for f in find(Algo::Partition) {
                fill(a, b, rng, n, keys);
                let r = eax(a, f, &[result, records, last, less]);
                assert_eq!(r, result);
                let (lo, hi) = sorter.partition(b, records, last);
                Mem::write(b, result, [lo, hi]);
                sync(a, b, "std::_Unguarded_partition");
            }
        }
    }
}

#[test]
fn std_sort_instantiations_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x4008_0201);
    sort_family(&mut a, &mut b, &mut rng, 16, &sort::SORT16);
    sort_family(&mut a, &mut b, &mut rng, 28, &sort::SORT28);
}

/// Round 9: the TO library's menu, window and text pieces.
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::items_after_statements
)] // one script; a, b: the oracles; the callback adapters sit by their use
fn round9(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, tables: bool) {
    use tolib::bridge as t;
    let orig = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().eax
    };
    let cells = alloc(a, b, 0x10);
    put(a, b, cells, [0u32; 4]);
    let fns: Vec<u32> = (0..3u32)
        .map(|k| stand_in8(a, b, 0x700 + 0x20 * k, &recorder(cells + 4 * k, 0x7300 + k)))
        .collect();
    let b_call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    for x in 0..5 {
        assert_eq!(eax(a, 0x0090_47a0, &[x]), t::set_pkt_side(b, x));
        same_memory("packet side", a, b);
    }
    // the menu's script release over a recording AtelRemoveCtrlWork
    let remove = stand_in8(a, b, 0x500, &recorder(cells, 0x1234));
    redirect(a, b, 0x0086_ed40, remove);
    for on in [0u8, 1, 0] {
        set(a, b, t::MENU_ATEL_ACTIVE, on);
        eax(a, 0x008b_e350, &[]);
        t::menu_atel_release(b, |m, i| {
            orig(m, 0x0086_ed40, &[i]);
        });
        same_memory("menu script release", a, b);
    }
    // scrolls
    let height = orig(b, 0x008b_7c60, &[]);
    for _ in 0..10 {
        let w = random_block(a, b, rng, 0x90);
        set(a, b, t::SCROLL_LOCK, rng.next() % 2);
        assert_eq!(
            eax(a, 0x0089_8e20, &[w]),
            t::btl_pad_scroll(b, w, height, false)
        );
        assert_eq!(
            eax(a, 0x0089_8df0, &[w]),
            t::btl_pad_scroll(b, w, height, true)
        );
        same_memory("scrolls", a, b);
        let p = random_block(a, b, rng, 0x18);
        put(
            a,
            b,
            p + 0xf,
            [0u8, 2, 3, 0x7f, 0x80, 0xff][(rng.next() % 6) as usize],
        );
        assert_eq!(eax(a, 0x008a_2ba0, &[p]), t::tick4(b, p));
        assert_eq!(eax(a, 0x008d_fe70, &[w]), t::message_hold(b, w));
        same_memory("counters and holds", a, b);
    }
    for f in [0x008a_dbd0, 0x008b_e3d0] {
        assert_eq!(eax(a, f, &[]), t::init_pad_data(b));
        same_memory("pad data", a, b);
    }
    let out = alloc(a, b, 4);
    for o in [0, out] {
        assert_eq!(
            eax(a, 0x008a_8e30, &[o]),
            t::menu_get_dbuff_dc(b, o, |m, dc| orig(m, 0x0082_0530, &[dc]))
        );
    }
    assert_eq!(
        eax(a, 0x008b_e330, &[]),
        t::init_menu_graph(b, |m, dc| orig(m, 0x0082_0530, &[dc]))
    );
    same_memory("draw contexts", a, b);
    let open = stand_in8(a, b, 0x520, &recorder(cells + 4, 0x4321));
    redirect(a, b, 0x0090_4600, open);
    assert_eq!(
        eax(a, 0x008b_ef30, &[]),
        t::menu_open_pkt_buff_tmp(b, |m| orig(m, 0x0090_4600, &[]))
    );
    same_memory("packet buffer", a, b);
    for open in [false, true] {
        let w = if open {
            random_block(a, b, rng, 0x50)
        } else {
            0
        };
        set(a, b, t::HELP_WINDOW, w);
        assert_eq!(eax(a, 0x008b_3790, &[]), t::close_help_window(b));
        same_memory("help window", a, b);
    }
    // names through the kernel and battle tables (their originals on b;
    // the tables are loaded on the snapshots only)
    let tmp = alloc(a, b, 4);
    let menu_cfg = ffx_battle::save::bridge::MENU;
    for kana in [false, true].into_iter().filter(|_| tables) {
        let cfg = menu_cfg.get(b);
        set(a, b, menu_cfg, if kana { cfg | 8 } else { cfg & !8 });
        let hira = orig(b, 0x0078_51f0, &[]) != 0;
        assert_eq!(hira, kana);
        for id in 0..12u32 {
            for (f, getter, field, aware) in [
                (0x008b_8d70, 0x0079_a4b0, 0, false),
                (0x008b_8d90, 0x0079_0970, 0, false),
                (0x008b_ebb0, 0x0079_0a90, 0, true),
                (0x008b_eb70, 0x0079_0a90, 8, true),
                (0x008b_ec30, 0x0079_08a0, 0, true),
                (0x008b_ebf0, 0x0079_08a0, 8, true),
            ] {
                let field = if aware && hira { field + 4 } else { field };
                assert_eq!(
                    eax(a, f, &[id]),
                    t::table_name(b, id, field, tmp, |m, id, p| orig(m, getter, &[id, p])),
                    "{f:#x} {id}"
                );
            }
        }
    }
    let v: u32 = Mem::read(b, tmp);
    Mem::write(a, tmp, v);
    same_memory("names", a, b);
    // save slots shown
    let slots = t::SAVE_SLOTS.ptr(b).addr;
    for i in 0..16 {
        put(a, b, slots + i, u8::try_from(rng.next() % 6).unwrap());
    }
    for all in [0u8, 1] {
        set(a, b, t::SLOTS_ALL, all);
        for i in 0..16 {
            assert_eq!(
                eax(a, 0x008b_3730, &[rng.next(), i]),
                t::save_slot_shown(b, i)
            );
        }
    }
    // message text: line breaks, counted skips
    let text = alloc(a, b, 0x80);
    for _ in 0..30 {
        let mut bytes: Vec<u8> = (0..0x30)
            .map(|_| [0x10, 0xff, 0x03, 0x01, 0x30, 0x41, 0x26, 0x02][(rng.next() % 8) as usize])
            .collect();
        bytes.extend([0u8; 0x50]);
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        assert_eq!(eax(a, 0x008b_7f80, &[text]), t::count_line_breaks(b, text));
        let n = rng.next() % 6;
        assert_eq!(
            eax(a, 0x008d_f690, &[text, n]),
            t::skip_counted_codes(b, text, n)
        );
    }
    for i in 0..8u32 {
        let w = random_block(a, b, rng, 0x120);
        set_entry(a, b, window::bridge::MSG_WINDOWS, i, w);
        assert_eq!(
            eax(a, 0x008b_89a0, &[i, text]),
            t::set_message_text(b, i.cast_signed(), text)
        );
        same_memory("message text", a, b);
    }
    // scenes by name
    let slots = scene::bridge::SLOTS.ptr(b).addr;
    let names: [&[u8]; 3] = [b"ka\0", b"kb\0", b"k\0"];
    let name_at: Vec<u32> = names
        .iter()
        .map(|n| {
            let p = alloc(a, b, 4);
            a.write(p, n).unwrap();
            b.write(p, n).unwrap();
            p
        })
        .collect();
    for j in 0..8u32 {
        let n = names[(rng.next() % 2) as usize];
        a.write(slots + 0x200 + j * 0x20 + 8, n).unwrap();
        b.write(slots + 0x200 + j * 0x20 + 8, n).unwrap();
    }
    for &n in &name_at {
        assert_eq!(
            eax(a, 0x008a_e000, &[2, n]),
            scene::bridge::has_scene(b, 2, n)
        );
    }
    // message variables and message init
    let vars = t::MSG_VARS.ptr(b).addr;
    let params = t::MSG_PARAMS.ptr(b).addr;
    for i in 0..2u32 {
        for k in 0..32 {
            let r = random_block(a, b, rng, 4);
            put(a, b, vars + i * 0x80 + 4 * k, r);
        }
        let w = random_block(a, b, rng, 0x130);
        set_entry(a, b, window::bridge::MSG_WINDOWS, i, w);
        let pb = random_block(a, b, rng, 0x138);
        put(a, b, params + 4 * i, pb);
        let k = i.cast_signed();
        assert_eq!(eax(a, 0x008b_8210, &[i]), t::init_message(b, k));
        assert_eq!(eax(a, 0x008b_8680, &[i]), t::reset_message_vars(b, k));
        same_memory("message init", a, b);
    }
    // language-dependent text rules, TOGetFFXLang redirected to a cell
    let lang_cell = alloc(a, b, 4);
    let mut code = vec![0xa1];
    code.extend_from_slice(&lang_cell.to_le_bytes());
    code.push(0xc3);
    let get_lang = stand_in8(a, b, 0x540, &code);
    redirect(a, b, 0x0064_10e0, get_lang);
    let glyphs = alloc(a, b, 8);
    for lang in 0..12 {
        put(a, b, lang_cell, lang);
        assert_eq!(eax(a, 0x008b_56d0, &[]), t::select_macro_dic(b, lang));
        same_memory("macro dictionary", a, b);
        for _ in 0..30 {
            let bytes: Vec<u8> = (0..6)
                .map(|_| {
                    [0x2c, 0x10, 0x30, 0x5a, 0x63, 0x64, 0x8d, 0x8e][(rng.next() % 8) as usize]
                })
                .collect();
            a.write(glyphs, &bytes).unwrap();
            b.write(glyphs, &bytes).unwrap();
            assert_eq!(
                eax(a, 0x008b_5760, &[glyphs + 2]),
                t::joins_neighbours(b, glyphs + 2, lang)
            );
        }
    }
    // the limit timer over stand-ins for the two sound calls
    let playing = alloc(a, b, 4);
    let mut code = vec![0xa1];
    code.extend_from_slice(&playing.to_le_bytes());
    code.push(0xc3);
    let is_play = stand_in8(a, b, 0x560, &code);
    redirect(a, b, 0x0088_6680, is_play);
    let stop = stand_in8(a, b, 0x580, &recorder(cells + 8, 0x5150));
    redirect(a, b, 0x0088_6eb0, stop);
    for on in [0u32, 1] {
        put(a, b, playing, on);
        assert_eq!(
            eax(a, 0x0089_7f00, &[]),
            t::btl_init_limit_timer(
                b,
                |m, i, n, x, y| {
                    orig(m, 0x008a_e450, &[i, n, x, y]);
                },
                |m, stop, id| orig(m, if stop { 0x0088_6eb0 } else { 0x0088_6680 }, &[id]),
            )
        );
        same_memory("limit timer", a, b);
    }
    // the save/load menu set-up, the menu's originals on b
    struct Menu<'f>(&'f dyn Fn(&mut Oracle, u32, &[u32]) -> u32);
    impl t::SaveMenu<Oracle> for Menu<'_> {
        fn set_debug_lock(&mut self, m: &mut Oracle, v: u32) {
            (self.0)(m, 0x0088_8420, &[v]);
        }
        fn reset_toggles(&mut self, m: &mut Oracle) {
            (self.0)(m, 0x008b_4eb0, &[]);
        }
        fn set_screen_x2(&mut self, m: &mut Oracle, v: u32) {
            (self.0)(m, 0x008a_b020, &[v]);
        }
        fn set_frame_ratio_diff(&mut self, m: &mut Oracle, v: i32) -> u32 {
            (self.0)(m, 0x008a_b260, &[v.cast_unsigned()])
        }
    }
    let menu = random_block(a, b, rng, 0x20);
    for (f, all) in [(0x008b_3510, true), (0x008b_3580, false)] {
        assert_eq!(
            eax(a, f, &[menu]),
            t::save_menu_open(b, menu, all, &mut Menu(&orig))
        );
        same_memory("save menu set-up", a, b);
    }
    // the field music after the menu over stand-ins: getters read cells,
    // actions log their first argument
    let music = alloc(a, b, 0x10);
    let log = alloc(a, b, 0x100);
    let cursor = alloc(a, b, 4);
    put(a, b, cursor, log);
    for (k, f) in [0x0086_bc30u32, 0x0086_bc50, 0x0087_3b00, 0x0088_63f0]
        .into_iter()
        .enumerate()
    {
        let k = u32::try_from(k).unwrap();
        let mut code = vec![0xa1];
        code.extend_from_slice(&(music + 4 * k).to_le_bytes());
        code.push(0xc3);
        let at = stand_in8(a, b, 0x5a0 + 0x10 * k, &code);
        redirect(a, b, f, at);
    }
    for (k, f) in [0x0087_3d00u32, 0x0088_6830, 0x0088_6af0]
        .into_iter()
        .enumerate()
    {
        let k = u32::try_from(k).unwrap();
        let at = stand_in8(a, b, 0x600 + 0x30 * k, &logger(cursor, 0x100 * (k + 1)));
        redirect(a, b, f, at);
    }
    struct Music<'f>(&'f dyn Fn(&mut Oracle, u32, &[u32]) -> u32);
    impl t::MenuMusic<Oracle> for Music<'_> {
        fn keep_white_in_out(&mut self, m: &mut Oracle) -> u32 {
            (self.0)(m, 0x0086_bc30, &[])
        }
        fn keep_clear11(&mut self, m: &mut Oracle) -> u32 {
            (self.0)(m, 0x0086_bc50, &[])
        }
        fn music_number(&mut self, m: &mut Oracle) -> u32 {
            (self.0)(m, 0x0087_3b00, &[])
        }
        fn regist_music_number(&mut self, m: &mut Oracle, n: u32) {
            (self.0)(m, 0x0087_3d00, &[n]);
        }
        fn now_playing(&mut self, m: &mut Oracle) -> u32 {
            (self.0)(m, 0x0088_63f0, &[])
        }
        fn cross_start(&mut self, m: &mut Oracle, n: u32, f: u32) -> u32 {
            (self.0)(m, 0x0088_6830, &[n, f])
        }
        fn fade_out(&mut self, m: &mut Oracle, n: u32, f: u32) -> u32 {
            (self.0)(m, 0x0088_6af0, &[n, f])
        }
    }
    for state in [
        [1u32, 0, 5, 5],
        [0, 0, 5, 7],
        [0, 1, 5, 5],
        [0, 1, 5, 9],
        [0, 1, u32::MAX, 3],
    ] {
        put(a, b, music, state);
        assert_eq!(
            eax(a, 0x008b_df60, &[]),
            t::menu_music(b, &mut Music(&orig))
        );
        same_memory("menu music", a, b);
    }
    // the Libra window's states, a click stand-in
    let click = stand_in8(a, b, 0x6a0, &recorder(cells + 0xc, 0x77));
    redirect(a, b, 0x0088_6ba0, click);
    for state in 0..10i16 {
        for (auto, trg) in [(0u32, 0u16), (1, 0), (0, 0x20)] {
            set(a, b, t::LIBRA_STATE, state);
            set(a, b, t::LIBRA_AUTO, auto);
            set(a, b, t::MENU_PAD_TRG, trg);
            assert_eq!(
                eax(a, 0x0089_1a30, &[]),
                t::btl_ctrl_libra_window(b, |m, id| orig(m, 0x0088_6ba0, &[id]))
            );
            same_memory("Libra window", a, b);
        }
    }
    assert_eq!(eax(a, 0x0089_77f0, &[]), t::btl_init(b));
    same_memory("battle init", a, b);
    // the menu's main windows closed, handlers recording
    let base = t::MAIN_WINDOWS.ptr(b).addr;
    let free = stand_in8(a, b, 0x6c0, &recorder(cells + 8, 0x99));
    redirect(a, b, 0x008a_a3f0, free);
    let zero = stand_in8(a, b, 0x6e0, &[0x31, 0xc0, 0xc3]);
    for k in 0..16u32 {
        let w = base + k * 0x98;
        put(a, b, w + 0x3f, u8::try_from(rng.next() % 3).unwrap());
        put(a, b, w + 0x41, u8::try_from(rng.next() % 2).unwrap());
        let pick = |rng: &mut Xorshift| [0, fns[0], fns[1], zero][(rng.next() % 4) as usize];
        put(a, b, w + 0xc, pick(rng));
        put(a, b, w + 0x14, pick(rng));
        put(a, b, w + 0x18, [0, fns[2]][(rng.next() % 2) as usize]);
    }
    for owner in 0..3 {
        assert_eq!(
            eax(a, 0x008a_9230, &[owner]),
            t::close_main_windows(b, owner, b_call, |m, w| orig(m, 0x008a_a3f0, &[w]))
        );
        same_memory("main windows", a, b);
    }
    // window rectangles and moves, short damage records, icon widths
    let wtab = t::WINDOWS.ptr(b).addr;
    for i in 0..4u32 {
        let w = random_block(a, b, rng, 0x40);
        put(a, b, wtab + 4 * i, w);
        let r: Vec<u32> = (0..4)
            .map(|_| (rng.next() >> (rng.next() % 32)).wrapping_sub(rng.next() % 3))
            .collect();
        let mut args = vec![i];
        args.extend(&r);
        assert_eq!(
            eax(a, 0x0090_6dd0, &args),
            t::set_window_rect(
                b,
                i.cast_signed(),
                [r[0], r[1], r[2], r[3]].map(u32::cast_signed)
            )
        );
        same_memory("window rectangles", a, b);
    }
    let moves = t::WINDOW_MOVES.ptr(b).addr;
    for _ in 0..30 {
        let i = rng.next() % 18;
        if i < 16 {
            put(
                a,
                b,
                moves + i * 16 + 0xe,
                u16::try_from(rng.next() % 2).unwrap(),
            );
        }
        let (x, y, n) = (rng.next() % 3, rng.next() % 3, rng.next());
        eax(a, 0x0090_1090, &[i, x, y, n]);
        t::move_window_entry(b, i.cast_signed(), x.cast_signed(), y.cast_signed(), n);
        same_memory("window moves", a, b);
    }
    for start in [0i16, 62, 63] {
        set(a, b, btl::bridge::DAMAGE_COUNT, start);
        let args: Vec<u32> = (0..7).map(|_| rng.next()).collect();
        assert_eq!(
            eax(a, 0x0089_baa0, &args),
            btl::bridge::set_damage_value_short(b, args.clone().try_into().unwrap())
                .cast_unsigned()
        );
        same_memory("short damage values", a, b);
    }
    for _ in 0..50 {
        let v = rng.float().to_bits();
        let v = if ffx_bridge::x87::is_snan(v) {
            v | 0x0040_0000
        } else {
            v
        };
        set(a, b, t::FONT_SCALE, f32::from_bits(v));
        for code in [0, 0x31, 0x35, 0x38, 0x45, 0x60, 0x61] {
            assert_eq!(eax(a, 0x008b_65e0, &[code]), t::button_icon_width(b, code));
        }
    }
    // scene slots cleared, message slot modes
    let sl = scene::bridge::SLOTS.ptr(b).addr;
    let block = random_block(a, b, rng, 0x100);
    let bytes: Vec<u8> = (0..0x100).map(|k| Mem::read::<u8>(a, block + k)).collect();
    a.write(sl + 0x300, &bytes).unwrap();
    b.write(sl + 0x300, &bytes).unwrap();
    assert_eq!(eax(a, 0x008a_df80, &[3]), t::clear_scene_slots(b, 3));
    same_memory("scene slots cleared", a, b);
    for i in 0..8u32 {
        let slot = alloc(a, b, 0x1c);
        for off in [0x14, 0x18] {
            let part = random_block(a, b, rng, 0x100);
            put(a, b, slot + off, part);
        }
        put(
            a,
            b,
            window::bridge::MSG_WINDOW_SLOTS.ptr(b).addr + 4 * i,
            slot,
        );
        let (x, y, v) = (rng.next(), rng.next(), rng.next());
        assert_eq!(
            eax(a, 0x008b_0020, &[i, x, y, v]),
            t::message_slot_mode1(b, i, x, y, v)
        );
        let (x, y) = (
            (rng.next() % 20).cast_signed() - 6,
            (rng.next() % 20).cast_signed() - 6,
        );
        assert_eq!(
            eax(
                a,
                0x008a_f730,
                &[i, x.cast_unsigned(), y.cast_unsigned(), v]
            ),
            t::message_slot_cursor(b, i, x, y, v)
        );
        same_memory("message slot modes", a, b);
    }
    // FRAME packets at the cursor
    let buf = alloc(a, b, 0x100);
    for _ in 0..8 {
        set(
            a,
            b,
            pkt::bridge::PACKET_CURSOR,
            buf + 0x20 * (rng.next() % 6),
        );
        let f: Vec<u32> = (0..4).map(|_| rng.next() >> (rng.next() % 32)).collect();
        assert_eq!(
            eax(a, 0x008f_d1f0, &f),
            t::frame_packet(b, [f[0], f[1], f[2], f[3]].map(u32::cast_signed))
        );
        same_memory("FRAME packets", a, b);
    }
    // the menu's top window, through the menu's originals on b
    struct Main<'f>(&'f dyn Fn(&mut Oracle, u32, &[u32]) -> u32);
    impl t::MainWindows<Oracle> for Main<'_> {
        fn alloc(&mut self, m: &mut Oracle) -> u32 {
            (self.0)(m, 0x008a_a1a0, &[])
        }
        fn regist(&mut self, m: &mut Oracle, w: u32) -> u32 {
            (self.0)(m, 0x008a_ab00, &[w])
        }
    }
    assert_eq!(
        eax(a, 0x008a_8e70, &[]),
        t::menu_main_window_open(b, &mut Main(&orig))
    );
    same_memory("menu top window", a, b);
    // window entries started
    let moves2 = t::WINDOW_MOVES.ptr(b).addr;
    for _ in 0..30 {
        let i = rng.next() % 18;
        if i < 16 {
            put(
                a,
                b,
                moves2 + i * 16 + 0xe,
                u16::try_from(rng.next() % 2).unwrap(),
            );
        }
        let (x, y) = (rng.next() % 3, rng.next() % 3);
        let n = rng.next().cast_signed() >> (rng.next() % 32);
        eax(a, 0x0090_1000, &[i, x, y, n.cast_unsigned()]);
        t::start_window_entry(b, i.cast_signed(), x.cast_signed(), y.cast_signed(), n);
        same_memory("window entries started", a, b);
    }
    // window banks
    for bank in [0u32, 1, 7] {
        eax(a, 0x0090_6ce0, &[bank]);
        t::set_exe_window_area(b, bank);
        same_memory("window banks", a, b);
    }
    // target masks (the battle's originals on b; tables on the snapshots)
    if tables {
        let tb = t::TARGET_BITS.ptr(b).addr;
        struct Chr<'f>(&'f dyn Fn(&mut Oracle, u32, &[u32]) -> u32);
        impl t::ChrStatus<Oracle> for Chr<'_> {
            fn cursor_flag(&mut self, m: &mut Oracle, c: u32) -> u32 {
                (self.0)(m, 0x0079_4ca0, &[c])
            }
            fn event_flag(&mut self, m: &mut Oracle, c: u32, cmd: u32) -> u32 {
                (self.0)(m, 0x0079_4f20, &[c, cmd])
            }
            fn dist(&mut self, m: &mut Oracle, c: u32) -> u32 {
                (self.0)(m, 0x0079_4e00, &[c])
            }
            fn dead(&mut self, m: &mut Oracle, c: u32) -> u32 {
                (self.0)(m, 0x0079_5780, &[c])
            }
        }
        // the four per-character queries as stand-ins: (chr ^ key) & 3
        for (k, f) in [0x0079_4ca0u32, 0x0079_4f20, 0x0079_4e00, 0x0079_5780]
            .into_iter()
            .enumerate()
        {
            let k = u32::try_from(k).unwrap();
            let mut code = vec![0x8b, 0x44, 0x24, 0x04, 0x35];
            code.extend_from_slice(&(0x5 * k + 1).to_le_bytes());
            code.extend_from_slice(&[0x83, 0xe0, 0x03, 0xc3]);
            let at = stand_in8(a, b, 0x780 + 0x20 * k, &code);
            redirect(a, b, f, at);
        }
        for _ in 0..6 {
            let words: Vec<u32> = (0..8).map(|_| rng.next()).collect();
            for (k, w) in words.iter().enumerate() {
                put(a, b, tb + 4 * u32::try_from(k).unwrap(), *w);
            }
            set(
                a,
                b,
                t::TARGET_COMMAND,
                u16::try_from(rng.next() % 0x40).unwrap(),
            );
            put(a, b, b.rebase(0x0133_d0dc), rng.next() & 0x0fff_ffff);
            assert_eq!(
                eax(a, 0x0089_83f0, &[]),
                t::btl_make_target_command_bit(b, tmp, |m, id, p| orig(m, 0x0079_a4b0, &[id, p]))
            );
            let v: u32 = Mem::read(b, tmp);
            Mem::write(a, tmp, v);
            same_memory("target command bits", a, b);
            eax(a, 0x0089_8330, &[]);
            t::btl_make_target_chr_status_bit(b, &mut Chr(&orig));
            same_memory("target status bits", a, b);
        }
    }
    // font widths: the scales cached or computed (font screen size the
    // original's, on b)
    for _ in 0..40 {
        let cached = rng.next().is_multiple_of(2);
        let s0 = if cached {
            [rng.float(), rng.float()]
        } else {
            [rng.float(), 0.0]
        };
        let s0 = s0.map(|f| {
            let v = f.to_bits();
            f32::from_bits(if ffx_bridge::x87::is_snan(v) {
                v | 0x0040_0000
            } else {
                v
            })
        });
        set(a, b, t::FONT_SCALES, s0);
        let (x, y) = (rng.float(), rng.float());
        let (x, y) = (
            if ffx_bridge::x87::is_snan(x.to_bits()) {
                1.0
            } else {
                x
            },
            if ffx_bridge::x87::is_snan(y.to_bits()) {
                2.0
            } else {
                y
            },
        );
        let st0 = call(a, 0x008f_a6b0, Conv::Cdecl, &[x.to_bits(), y.to_bits()])
            .st0
            .expect("a float result");
        let wh = alloc(a, b, 8);
        let r = t::font_width_in_pixel(b, x, y, |m| {
            orig(m, 0x0064_0db0, &[wh, wh + 4]);
            (Mem::read(m, wh), Mem::read(m, wh + 4))
        });
        let ab: [u32; 2] = Mem::read(b, wh);
        Mem::write(a, wh, ab);
        if r.is_nan() {
            assert!(st0.is_nan());
        } else {
            #[allow(clippy::cast_possible_truncation)] // exact: an f32 value
            let st0 = st0 as f32;
            assert_eq!(st0.to_bits(), r.to_bits(), "{x} {y} {s0:?}");
        }
        same_memory("font width", a, b);
    }
    // packet tags, bit arrays
    let tag = alloc(a, b, 0x10);
    let bits = random_block(a, b, rng, 0x40);
    for _ in 0..20 {
        let end = tag + rng.next() % 0x400;
        let flags = rng.next();
        assert_eq!(
            eax(a, 0x0089_0e30, &[tag, end, flags]),
            t::gif_tag(b, tag, end, flags)
        );
        same_memory("packet tag", a, b);
        let i = (rng.next() % 0x200).cast_signed() - 0x40;
        let base = bits + 0x20;
        assert_eq!(
            eax(a, 0x008a_8f00, &[base, i.cast_unsigned()]),
            t::bit16(b, base, i)
        );
    }
}

#[test]
fn round9_system_functions_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round9(&mut a, &mut b, &mut Xorshift(0x4009_0001), false);
}

#[test]
fn round9_system_functions_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round9(&mut a, &mut b, &mut Xorshift(0x4009_0002), true);
}

/// Round 10: the next logic-class system functions.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one script; a, b: the oracles
fn round10(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let orig = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().eax
    };
    let wh = alloc(a, b, 8);
    let screen = |m: &mut Oracle| {
        orig(m, 0x0064_0db0, &[wh, wh + 4]);
        (Mem::read::<i32>(m, wh), Mem::read::<i32>(m, wh + 4))
    };
    let sync_wh = |a: &mut Oracle, b: &mut Oracle| {
        let v: [u32; 2] = Mem::read(b, wh);
        Mem::write(a, wh, v);
    };
    let st0_f32 = |r: ffx_oracle::Ret| r.st0_f32().expect("a float result");
    // wide-screen x positions
    let wtab = t::WINDOWS.ptr(b).addr;
    for i in 0..4u32 {
        let w = random_block(a, b, rng, 0x40);
        put(a, b, w + 4, u16::try_from(rng.next() % 3).unwrap());
        put(a, b, wtab + 4 * i, w);
    }
    for _ in 0..60 {
        for g in t::WIDE_BLOCKERS {
            put(a, b, b.rebase(g), u32::from(rng.next().is_multiple_of(4)));
        }
        let i = rng.next() % 4;
        let x = f32::from(u16::try_from(rng.next() % 2000).unwrap()) / 3.0 - 100.0;
        let r = call(a, 0x008b_8430, Conv::Cdecl, &[i, x.to_bits()]);
        let got = t::wide_window_x(b, i.cast_signed(), x, screen);
        sync_wh(a, b);
        assert_eq!(st0_f32(r).to_bits(), got.to_bits(), "{i} {x}");
        same_memory("wide window x", a, b);
    }
    // font spaces: TOGetFFXLang redirected to a cell
    let lang_cell = alloc(a, b, 4);
    let mut code = vec![0xa1];
    code.extend_from_slice(&lang_cell.to_le_bytes());
    code.push(0xc3);
    let get_lang = stand_in8(a, b, 0x800, &code);
    redirect(a, b, 0x0064_10e0, get_lang);
    for _ in 0..60 {
        let lang = rng.next() % 12;
        put(a, b, lang_cell, lang);
        let first = [0.0f32, 3.5][(rng.next() % 2) as usize];
        set(a, b, t::FONT_SPACES, [first, 7.25]);
        let ws = [0.0f32, 1.0, 3.75, 0.5][(rng.next() % 4) as usize];
        set(a, b, t::FONT_SCALES, [ws, 2.0]);
        let kind = rng.next() % 5;
        let r = call(a, 0x008f_a570, Conv::Cdecl, &[kind]);
        let got = t::font_space_in_pixel(b, kind, |m| orig(m, 0x0064_10e0, &[]), screen);
        sync_wh(a, b);
        let v = st0_f32(r);
        assert!(v.to_bits() == got.to_bits(), "{lang} {kind} {ws}");
        same_memory("font spaces", a, b);
    }
    // scenes initialised: running slots stopped unless kept
    let sl = scene::bridge::SLOTS.ptr(b).addr;
    for _ in 0..4 {
        for j in 0..8u32 {
            let s = sl + 0x100 + j * 0x20;
            put(
                a,
                b,
                s,
                [
                    u16::try_from(rng.next() % 5).unwrap(),
                    u16::try_from(rng.next() % 2).unwrap(),
                ],
            );
            put(a, b, s + 8, 0x41u8);
        }
        assert_eq!(eax(a, 0x008a_eec0, &[1]), scene::bridge::init_scene(b, 1));
        same_memory("scene init", a, b);
    }
    // image sizes: every id the original names and some unknown ones, the
    // language and battle state from stand-ins
    let cells = alloc(a, b, 8);
    for (k, f) in [0x0088_7c30u32, 0x0078_16f0].into_iter().enumerate() {
        let k = u32::try_from(k).unwrap();
        let mut code = vec![0xa1];
        code.extend_from_slice(&(cells + 4 * k).to_le_bytes());
        code.push(0xc3);
        let at = stand_in8(a, b, 0x820 + 0x10 * k, &code);
        redirect(a, b, f, at);
    }
    let sizes = t::IMAGE_SIZES.ptr(b).addr;
    for k in 0..6u32 {
        put(
            a,
            b,
            sizes + 16 * k,
            [rng.next() % 0x800 - 0x400, rng.next() % 0x800],
        );
    }
    let out = alloc(a, b, 8);
    let mut ids: Vec<u32> = (0x2e00..0x3000).chain(0x3b80..0x4000).collect();
    ids.extend([
        0x0013_2f00,
        0x0013_2f04,
        0x0013_2f08,
        0x0013_2f0c,
        0x0013_2f10,
        0x0013_2eff,
        0,
    ]);
    for id in ids {
        put(a, b, cells, [rng.next() % 2, rng.next() % 2]);
        assert_eq!(
            eax(a, 0x008a_c400, &[id, out, out + 4]),
            t::get_image_wh(
                b,
                id,
                [out, out + 4],
                |m| orig(m, 0x0088_7c30, &[]),
                |m| orig(m, 0x0078_16f0, &[])
            ),
            "{id:#x}"
        );
        same_memory("image sizes", a, b);
    }
    // the limit timer's sound over stand-ins: playing from a cell, stop
    // and play recorded
    let snd = alloc(a, b, 0x10);
    let mut code = vec![0xa1];
    code.extend_from_slice(&snd.to_le_bytes());
    code.push(0xc3);
    let is_play = stand_in8(a, b, 0x840, &code);
    redirect(a, b, 0x0088_6680, is_play);
    let stop = stand_in8(a, b, 0x860, &recorder(snd + 4, 0x51));
    redirect(a, b, 0x0088_6eb0, stop);
    let play = stand_in8(a, b, 0x880, &recorder(snd + 8, 0x52));
    redirect(a, b, 0x0088_6e80, play);
    let s3 = |m: &mut Oracle, flag: bool, id: u32, on: u32| {
        orig(m, if flag { on } else { 0x0088_6680 }, &[id])
    };
    for (playing, flag) in [(0u32, 0u32), (1, 0), (0, 2), (1, 2), (0, 1)] {
        put(a, b, snd, playing);
        set(a, b, t::LIMIT_TIMER, [flag, 0, 0, 0, 0, 0, 0]);
        for f in [0x0089_ab20, 0x0089_aba0] {
            assert_eq!(
                eax(a, f, &[]),
                t::limit_timer_stop(b, |m, st, id| s3(m, st, id, 0x0088_6eb0))
            );
        }
        set(a, b, t::LIMIT_TIMER, [flag, 0, 0, 0, 0, 0, 0]);
        eax(a, 0x0089_ab50, &[]);
        t::limit_timer_start(b, |m, pl, id| s3(m, pl, id, 0x0088_6e80));
        same_memory("limit timer sound", a, b);
    }
    // window layouts
    let table = alloc(a, b, 0x400);
    put(a, b, table, 0x45u32);
    for i in 0..0x45u32 {
        let off = 0x120 + 0x40 * (i % 8);
        put(a, b, table + 4 + 4 * i, off);
    }
    for k in 0..8u32 {
        let at = table + 0x120 + 0x40 * k;
        put(a, b, at, [1u32, 2, 4][(k % 3) as usize]);
        for w in 1..13 {
            put(a, b, at + 4 * w, rng.next());
        }
    }
    for i in [0u32, 1, 2, 5, 0x43, 0x44, 0x45, 0x100] {
        assert_eq!(
            eax(a, 0x0090_5e90, &[table, i]),
            t::copy_window_layout(b, table, i)
        );
        same_memory("window layouts", a, b);
    }
    // menu init, brushes, item lists (TkMenuItemRefresh recorded)
    assert_eq!(eax(a, 0x008b_e800, &[]), t::menu_init(b));
    assert_eq!(eax(a, 0x0090_0f40, &[]), t::menu_init_brush(b));
    same_memory("menu init", a, b);
    let refresh = stand_in8(a, b, 0x8a0, &recorder(snd + 0xc, 0x53));
    redirect(a, b, 0x008c_8e60, refresh);
    for k in 0..6 {
        assert_eq!(
            eax(a, 0x008c_6730, &[k]),
            t::amap_make_item_list(b, k, |m, mode| orig(m, 0x008c_8e60, &[mode]))
        );
        same_memory("item lists", a, b);
    }
    // message-window slots set up, with their tables as round 6 sets them
    for i in 0..8u32 {
        let slot = alloc(a, b, 0x1c);
        put(
            a,
            b,
            window::bridge::MSG_WINDOW_SLOTS.ptr(b).addr + 4 * i,
            slot,
        );
        let w = random_block(a, b, rng, 0x80);
        set_entry(a, b, window::bridge::WINDOWS, i, w);
        let m = random_block(a, b, rng, 0x130);
        set_entry(a, b, window::bridge::MSG_WINDOWS, i, m);
        let e = random_block(a, b, rng, 0x138);
        set_entry(a, b, window::bridge::TABLE_01868A90, i, e);
        put(a, b, t::MSG_PARAMS.ptr(b).addr + 4 * i, e);
        for k in 0..32 {
            let r = random_block(a, b, rng, 4);
            put(a, b, t::MSG_VARS.ptr(b).addr + i * 0x80 + 4 * k, r);
        }
        assert_eq!(
            eax(a, 0x008a_da30, &[i]),
            t::init_message_slot(b, i.cast_signed())
        );
        same_memory("message slot init", a, b);
    }
}

#[test]
fn round10_system_functions_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round10(&mut a, &mut b, &mut Xorshift(0x4010_0001));
}

#[test]
fn round10_system_functions_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round10(&mut a, &mut b, &mut Xorshift(0x4010_0002));
}

/// Round 11: `TOGetShapTextureName`, `TOSeqInitParam`, layouts, font advances.
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::items_after_statements
)] // one script; a, b: the oracles; the callback adapter sits by its use
fn round11(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let orig = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().eax
    };
    // TOGetFFXLang redirected to a cell; the name table filled with
    // distinct words
    let lang_cell = alloc(a, b, 4);
    let mut code = vec![0xa1];
    code.extend_from_slice(&lang_cell.to_le_bytes());
    code.push(0xc3);
    let get_lang = stand_in8(a, b, 0x900, &code);
    redirect(a, b, 0x0064_10e0, get_lang);
    put(a, b, b.rebase(0x00c5_997c), 0x9970_0000u32);
    for k in 0..0x5au32 {
        put(a, b, b.rebase(0x00c5_9d58 + 4 * k), 0x7000_0000 + k);
    }
    let mut ids: Vec<u32> = (0x2ea0..0x2f10)
        .chain(0x3d80..0x3f10)
        .chain(0x0013_2efe..0x0013_2f10)
        .collect();
    ids.extend([0, 0x3c00]);
    for lang in 0..12 {
        put(a, b, lang_cell, lang);
        for &id in &ids {
            let cache = [0u32, 0x1234][(rng.next() % 2) as usize];
            set(a, b, t::SHAP_CACHE, [cache, rng.next()]);
            assert_eq!(
                eax(a, 0x008a_c8c0, &[id]),
                t::shap_texture_name(b, id, |m| orig(m, 0x0064_10e0, &[])),
                "{lang} {id:#x}"
            );
            same_memory("shape texture names", a, b);
        }
    }
    // layouts from the three image tables
    for (f, table) in t::LAYOUT_TABLES {
        for i in [0u32, 1, 3, 7, 0x43, 0x200] {
            assert_eq!(
                eax(a, f, &[i]),
                t::copy_layout_from(b, table, i),
                "{f:#x} {i}"
            );
            same_memory("image layouts", a, b);
        }
    }
    // character advances and glyph snapping, the scales and spaces through
    // the ported functions (as the hook does)
    let wh = alloc(a, b, 8);
    let screen = |m: &mut Oracle| {
        orig(m, 0x0064_0db0, &[wh, wh + 4]);
        (Mem::read::<i32>(m, wh), Mem::read::<i32>(m, wh + 4))
    };
    let font = random_block(a, b, rng, 0x20);
    let glyph = random_block(a, b, rng, 0x20);
    let fl = |rng: &mut Xorshift| {
        let v = rng.float().to_bits();
        f32::from_bits(if ffx_bridge::x87::is_snan(v) {
            v | 0x0040_0000
        } else {
            v
        })
    };
    for n in 0..80 {
        put(a, b, lang_cell, rng.next() % 12);
        let ws = [0.0f32, 3.75, 1.0][(rng.next() % 3) as usize];
        set(
            a,
            b,
            t::FONT_SCALES,
            [ws, [0.0, 2.5][(rng.next() % 2) as usize]],
        );
        set(
            a,
            b,
            t::FONT_SPACES,
            [[0.0, 4.0][(rng.next() % 2) as usize], 6.0],
        );
        put(a, b, font + 0x1c, fl(rng));
        put(a, b, glyph + 4, fl(rng));
        put(a, b, glyph + 0x1a, u8::try_from(rng.next() % 5).unwrap());
        let size = fl(rng);
        let kind = rng.next() % 5;
        let width = |m: &mut Oracle, x: f32, y: f32| t::font_width_in_pixel(m, x, y, screen);
        let space = |m: &mut Oracle, k: u32| {
            t::font_space_in_pixel(m, k, |m| orig(m, 0x0064_10e0, &[]), screen)
        };
        if n % 2 == 0 {
            let st0 = call(a, 0x008f_a740, Conv::Cdecl, &[kind, font, size.to_bits()])
                .st0
                .expect("a float result");
            let r = t::font_advance(b, kind, font, size, width, space);
            #[allow(clippy::cast_possible_truncation)] // exact: an f32 value
            let st0 = st0 as f32;
            assert!(st0.to_bits() == r.to_bits(), "advance {n}");
        } else {
            call(a, 0x008e_8ed0, Conv::Cdecl, &[glyph, font, size.to_bits()]);
            t::add_font_width_space(b, glyph, font, size, screen, width, space);
        }
        let v: [u32; 2] = Mem::read(b, wh);
        Mem::write(a, wh, v);
        same_memory("font advances", a, b);
    }
    // wide-screen window rectangles
    let wtab = t::WINDOWS.ptr(b).addr;
    for i in 0..2u32 {
        let w = random_block(a, b, rng, 0x40);
        put(a, b, w + 4, u16::try_from(i).unwrap());
        put(a, b, wtab + 4 * i, w);
    }
    let rect = alloc(a, b, 0x10);
    for _ in 0..20 {
        let i = rng.next() % 2;
        let r: Vec<u32> = (0..4).map(|_| rng.next() % 2000).collect();
        let mut args = vec![i, rect];
        args.extend(&r);
        assert_eq!(
            eax(a, 0x008b_8d10, &args),
            t::wide_window_rect(
                b,
                i.cast_signed(),
                rect,
                [r[0], r[1], r[2], r[3]].map(u32::cast_signed),
                |m, i, x| { t::wide_window_x(m, i, x, screen) }
            )
            .cast_unsigned()
        );
        let v: [u32; 2] = Mem::read(b, wh);
        Mem::write(a, wh, v);
        same_memory("wide window rectangles", a, b);
    }
    // interned names
    let names: [&[u8]; 4] = [b"menu00\0", b"menu01\0", b"menu0\0", b"\0"];
    let at: Vec<u32> = names
        .iter()
        .map(|n| {
            let p = alloc(a, b, 8);
            a.write(p, n).unwrap();
            b.write(p, n).unwrap();
            p
        })
        .collect();
    set(a, b, t::NAME_COUNT, 0);
    for _ in 0..12 {
        let p = at[(rng.next() % 4) as usize];
        assert_eq!(eax(a, 0x008a_b440, &[p]), t::intern_name(b, p));
        same_memory("interned names", a, b);
    }
    // the summon list over stand-ins: a command list in memory, the joined
    // test a bit mask
    let list = alloc(a, b, 0x80);
    let joined = alloc(a, b, 4);
    let mut code = vec![0xb8];
    code.extend_from_slice(&list.to_le_bytes());
    code.push(0xc3);
    let status = stand_in8(a, b, 0x920, &code);
    redirect(a, b, 0x0079_0200, status);
    let count = alloc(a, b, 8);
    for (k, f) in [0x008d_2d80u32, 0x008d_2dc0].into_iter().enumerate() {
        let mut code = vec![0xa1];
        code.extend_from_slice(&(count + 4 * u32::try_from(k).unwrap()).to_le_bytes());
        code.push(0xc3);
        let s = stand_in8(a, b, 0x940 + 0x10 * u32::try_from(k).unwrap(), &code);
        redirect(a, b, f, s);
    }
    // mov ecx, [esp+4]; mov eax, [joined]; shr eax, cl; and eax, 1; ret
    let mut code = vec![0x8b, 0x4c, 0x24, 0x04, 0xa1];
    code.extend_from_slice(&joined.to_le_bytes());
    code.extend_from_slice(&[0xd3, 0xe8, 0x83, 0xe0, 0x01, 0xc3]);
    let join = stand_in8(a, b, 0x960, &code);
    redirect(a, b, 0x0078_5380, join);
    struct Summons<'f>(&'f dyn Fn(&mut Oracle, u32, &[u32]) -> u32);
    impl t::SummonList<Oracle> for Summons<'_> {
        fn status_num(&mut self, m: &mut Oracle, id: u32, s: u32) -> u32 {
            (self.0)(m, 0x0079_0200, &[id, s])
        }
        fn command_max(&mut self, m: &mut Oracle, l: u32, k: u32, x: u32, y: u32) -> i32 {
            (self.0)(m, 0x008d_2d80, &[l, k, x, y]).cast_signed()
        }
        fn command_offset(&mut self, m: &mut Oracle, l: u32, k: u32, x: u32, y: u32) -> i32 {
            (self.0)(m, 0x008d_2dc0, &[l, k, x, y]).cast_signed()
        }
        fn joined(&mut self, m: &mut Oracle, c: u32) -> u32 {
            (self.0)(m, 0x0078_5380, &[c])
        }
    }
    for _ in 0..8 {
        let n = rng.next() % 8;
        put(a, b, count, [n, rng.next() % 3]);
        for k in 0..0x20u32 {
            let id = [0x64u16, 9, 0xe, 0x12, 0x13, 0x0f][(rng.next() % 6) as usize];
            put(a, b, list + 4 * k, [0u16, id]);
        }
        put(a, b, joined, rng.next());
        eax(a, 0x008a_9090, &[]);
        t::summon_list_init(b, &mut Summons(&orig));
        same_memory("summon list", a, b);
    }
    // the menu's layout parameters from scenes 1 and 2
    let scratch = alloc(a, b, 4);
    let lo = b.rebase(0x025d_0a80);
    // the outputs filled first, so the zeros written without the menu's
    // scenes show (every output address is compared, none skipped)
    for k in 0..0xc38u32 {
        put(a, b, lo + k, 0xa5u8);
    }
    let before: Vec<u8> = (0..0xc38).map(|k| Mem::read::<u8>(b, lo + k)).collect();
    eax(a, 0x008a_e6f0, &[]);
    scene::bridge::init_param(
        b,
        scratch,
        ffx_menu::scene::bridge::tk_menu_get_param_handle,
        ffx_menu::scene::bridge::tk_menu_get_param_offset,
    );
    let v: u32 = Mem::read(b, scratch);
    Mem::write(a, scratch, v);
    same_memory("menu layout parameters", a, b);
    let changed = (0..0xc38u32)
        .filter(|&k| Mem::read::<u8>(b, lo + k) != before[k as usize])
        .count();
    assert!(changed > 0x100, "TOSeqInitParam changed {changed} bytes");
}

#[test]
fn round11_system_functions_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round11(&mut a, &mut b, &mut Xorshift(0x4011_0001));
}

#[test]
fn round11_system_functions_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round11(&mut a, &mut b, &mut Xorshift(0x4011_0002));
}

/// Round 12: the next logic-class system functions.
#[allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::items_after_statements
)] // one script; a, b: the oracles; the callback adapters sit by their use
fn round12(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let orig = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().eax
    };
    // texture files registered and copied
    let file = alloc(a, b, 0x400);
    for slot in 0..8u16 {
        for version in [0xc7u16, 0xc8] {
            let bytes: Vec<u8> = (0..0x400).map(|_| (rng.next() & 0xff) as u8).collect();
            a.write(file, &bytes).unwrap();
            b.write(file, &bytes).unwrap();
            put(a, b, file + 4, version);
            put(a, b, file + 8, slot);
            put(a, b, file + 0x20, 0x100u32);
            put(a, b, file + 0x24, rng.next() % 0x100);
            put(a, b, file + 0x30, 0x200u32);
            put(a, b, file + 0x10, rng.next() % 0x100);
            assert_eq!(
                eax(a, 0x008a_c130, &[file]),
                t::ftc_transfer(b, file),
                "{slot} {version:#x}"
            );
            same_memory("texture files", a, b);
        }
    }
    // the player list over stand-ins: party and reserve arrays with
    // counts, the joined test a bit mask
    let cells = alloc(a, b, 0x40);
    let party = cells + 0x10;
    let reserve = cells + 0x20;
    for (k, (f, list)) in [(0x0078_5270u32, party), (0x0078_52d0, reserve)]
        .into_iter()
        .enumerate()
    {
        let k = u32::try_from(k).unwrap();
        // mov eax, [esp+4]; mov ecx, [cells + 4k]; mov [eax], ecx; mov eax, list; ret
        let mut code = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x0d];
        code.extend_from_slice(&(cells + 4 * k).to_le_bytes());
        code.extend_from_slice(&[0x89, 0x08, 0xb8]);
        code.extend_from_slice(&list.to_le_bytes());
        code.push(0xc3);
        let s = stand_in8(a, b, 0xa00 + 0x20 * k, &code);
        redirect(a, b, f, s);
    }
    let mut code = vec![0x8b, 0x4c, 0x24, 0x04, 0xa1];
    code.extend_from_slice(&(cells + 8).to_le_bytes());
    code.extend_from_slice(&[0xd3, 0xe8, 0x83, 0xe0, 0x01, 0xc3]);
    let join = stand_in8(a, b, 0xa40, &code);
    redirect(a, b, 0x0078_53a0, join);
    struct Players<'f>(&'f dyn Fn(&mut Oracle, u32, &[u32]) -> u32);
    impl t::PlayerList<Oracle> for Players<'_> {
        fn in_party(&mut self, m: &mut Oracle, c: u32) -> u32 {
            (self.0)(m, 0x0078_5270, &[c])
        }
        fn out_party(&mut self, m: &mut Oracle, c: u32) -> u32 {
            (self.0)(m, 0x0078_52d0, &[c])
        }
        fn joined(&mut self, m: &mut Oracle, c: u32) -> u32 {
            (self.0)(m, 0x0078_53a0, &[c])
        }
    }
    let count = alloc(a, b, 4);
    for _ in 0..12 {
        put(a, b, cells, [rng.next() % 5, rng.next() % 5, rng.next()]);
        for k in 0..16u32 {
            put(
                a,
                b,
                party + k,
                [0xffu8, 7, 0, 1, 2, 3, 4, 5, 6][(rng.next() % 9) as usize],
            );
        }
        let flags = [0u32, 0x1_0000, 0x2_0003][(rng.next() % 3) as usize];
        eax(a, 0x008a_8f40, &[flags]);
        t::player_list_init(b, flags, count, &mut Players(&orig));
        let v: u32 = Mem::read(b, count);
        Mem::write(a, count, v);
        same_memory("player list", a, b);
    }
    // the battle info window, the remaps through the render module's
    // originals on b
    assert_eq!(
        eax(a, 0x0089_7bd0, &[]),
        t::btl_init_info_win_mes(b, |m, y, v| {
            let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
            let r = m
                .call(at, Conv::Cdecl, &[v.to_bits()])
                .unwrap()
                .st0
                .unwrap();
            #[allow(clippy::cast_possible_truncation)] // exact: an f32 value
            let r = r as f32;
            r
        })
    );
    same_memory("battle info window", a, b);
    // message-window priorities
    let slots = window::bridge::MSG_WINDOW_SLOTS.ptr(b).addr;
    let out = alloc(a, b, 0x10);
    for _ in 0..20 {
        for k in 0..8u32 {
            let s = alloc(a, b, 8);
            put(
                a,
                b,
                s,
                [
                    u16::try_from(rng.next() % 3).unwrap(),
                    0,
                    u16::try_from(rng.next() % 9).unwrap().wrapping_sub(4),
                    0,
                ],
            );
            put(a, b, slots + 4 * k, s);
        }
        assert_eq!(
            eax(a, 0x008b_00a0, &[out]),
            t::sort_message_window_priority(b, out)
        );
        same_memory("message priorities", a, b);
    }
    // character widths: TOGetFFXLang from a cell, width tables filled
    let lang_cell = alloc(a, b, 4);
    let mut code = vec![0xa1];
    code.extend_from_slice(&lang_cell.to_le_bytes());
    code.push(0xc3);
    let get_lang = stand_in8(a, b, 0xa60, &code);
    redirect(a, b, 0x0064_10e0, get_lang);
    let tex = t::TEXTURES.ptr(b).addr;
    for slot in 0..6u32 {
        let tbl = random_block(a, b, rng, 0x2400);
        put(
            a,
            b,
            tex + 16 * slot + 4,
            if slot == 5 && rng.next().is_multiple_of(2) {
                0
            } else {
                tbl + 0x400
            },
        );
    }
    let wh = alloc(a, b, 8);
    struct Fw<'f>(&'f dyn Fn(&mut Oracle, u32, &[u32]) -> u32, u32);
    impl t::FontWidth<Oracle> for Fw<'_> {
        fn screen_wh(&mut self, m: &mut Oracle) -> (i32, i32) {
            (self.0)(m, 0x0064_0db0, &[self.1, self.1 + 4]);
            (Mem::read(m, self.1), Mem::read(m, self.1 + 4))
        }
        fn lang(&mut self, m: &mut Oracle) -> u32 {
            (self.0)(m, 0x0064_10e0, &[])
        }
        fn width(&mut self, m: &mut Oracle, w: f32, s: f32) -> f32 {
            let wh = self.1;
            t::font_width_in_pixel(m, w, s, |m| {
                let at = m.rebase(0x0064_0db0);
                m.call(at, Conv::Cdecl, &[wh, wh + 4]).unwrap();
                (Mem::read(m, wh), Mem::read(m, wh + 4))
            })
        }
        fn space(&mut self, m: &mut Oracle, k: u32) -> f32 {
            let wh = self.1;
            t::font_space_in_pixel(
                m,
                k,
                |m| {
                    let at = m.rebase(0x0064_10e0);
                    m.call(at, Conv::Cdecl, &[]).unwrap().eax
                },
                |m| {
                    let at = m.rebase(0x0064_0db0);
                    m.call(at, Conv::Cdecl, &[wh, wh + 4]).unwrap();
                    (Mem::read(m, wh), Mem::read(m, wh + 4))
                },
            )
        }
    }
    let text = alloc(a, b, 0x10);
    let width = alloc(a, b, 4);
    for n in 0..300 {
        let mut bytes: Vec<u8> = (0..8)
            .map(|_| {
                [
                    4u8, 6, 0x26, 0x27, 0x28, 0x2a, 0x2c, 0x2f, 0x30, 0x47, 0x47, 0x50, 0x10,
                ][(rng.next() % 13) as usize]
            })
            .collect();
        bytes.push(0);
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        put(a, b, lang_cell, [0u32, 1, 9, 10][(rng.next() % 4) as usize]);
        set(a, b, t::FONT_ASPECT_ON, rng.next() % 2);
        set(
            a,
            b,
            t::FONT_SCALES,
            [[0.0f32, 3.75][(rng.next() % 2) as usize], 2.5],
        );
        set(
            a,
            b,
            t::FONT_SPACES,
            [[0.0f32, 4.0][(rng.next() % 2) as usize], 6.0],
        );
        let kind = rng.next() % 5;
        let table = rng.next() % 5; // table 5 may be null; the original only checks it for code 6
        put(a, b, width, rng.next() % 64);
        assert_eq!(
            eax(a, 0x008b_9330, &[text, kind, width, table]),
            t::font_width(b, text, kind, width, table, &mut Fw(&orig, wh)),
            "{n} {bytes:x?}"
        );
        let v: [u32; 2] = Mem::read(b, wh);
        Mem::write(a, wh, v);
        same_memory("character widths", a, b);
    }
}

#[test]
fn round12_system_functions_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round12(&mut a, &mut b, &mut Xorshift(0x4012_0001));
}

#[test]
fn round12_system_functions_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round12(&mut a, &mut b, &mut Xorshift(0x4012_0002));
}

/// Round 13's system functions.
fn round13(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::menu::bridge as mb;
    use tolib::bridge as t;
    // window layouts by id, across every range and past the last
    for id in (0..0x2a0u32).chain([0x2ff, 0x1000, u32::MAX]) {
        assert_eq!(
            eax(a, 0x008f_a4f0, &[id]),
            t::window_layout_of(b, id),
            "{id:#x}"
        );
        same_memory("window layouts", a, b);
    }
    assert_eq!(eax(a, 0x008a_8e20, &[]), t::save_gil_address(b));
    // the magic texture loaded once, through a stand-in recording the id
    let cell = alloc(a, b, 4);
    let load = stand_in8(a, b, 0xa80, &recorder(cell, 0x5a5a_0013));
    redirect(a, b, 0x0064_20b0, load);
    for _ in 0..16 {
        let handle = if rng.next() & 1 == 0 {
            u32::MAX
        } else {
            rng.next()
        };
        set(
            a,
            b,
            mb::MAGIC_FILE,
            [rng.next(), rng.next(), rng.next(), handle],
        );
        set(a, b, mb::MAGIC_TEXTURE_LOADED, rng.next() & 1);
        put(a, b, cell, 0u32);
        let got = mb::is_load_texture(b, |m, id| {
            let at = m.rebase(0x0064_20b0);
            m.call(at, Conv::Cdecl, &[id]).unwrap();
        });
        assert_eq!(eax(a, 0x009d_a280, &[]), got);
        same_memory("magic texture", a, b);
    }
}

#[test]
fn round13_system_functions_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round13(&mut a, &mut b, &mut Xorshift(0x4013_0001));
}

#[test]
fn round13_system_functions_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round13(&mut a, &mut b, &mut Xorshift(0x4013_0002));
}

/// Round 13's language-dependent help picture names.
fn round13_help_names(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let cell = alloc(a, b, 4);
    let mut getter = vec![0xa1];
    getter.extend_from_slice(&cell.to_le_bytes());
    getter.push(0xc3);
    let lang = stand_in8(a, b, 0xaa0, &getter);
    redirect(a, b, 0x0064_10e0, lang);
    let out = alloc(a, b, 0x200);
    let prefix = alloc(a, b, 0x110);
    let prefix_cell = ffx_system::files::bridge::STREAM_PREFIX;
    let mut percent = vec![b'x'; 0x90];
    percent.extend_from_slice(b"%%/");
    let prefixes: [Option<&[u8]>; 5] = [
        None,
        Some(b""),
        Some(b"data/"),
        Some(&percent),
        Some(&[b'y'; 0xd0]),
    ];
    for pre in prefixes {
        match pre {
            Some(text) => {
                let mut v = text.to_vec();
                v.push(0);
                a.write(prefix, &v).unwrap();
                b.write(prefix, &v).unwrap();
                set(a, b, prefix_cell, prefix);
            }
            None if prefix_cell.get(a) == 0 => continue,
            None => {}
        }
        for l in [0u32, 1, 2, 3, 4, 5, 9, 10] {
            put(a, b, cell, l);
            for n in [
                0i32,
                1,
                7,
                42,
                -3,
                i32::MAX,
                i32::MIN,
                rng.next().cast_signed(),
            ] {
                put(a, b, out, [0xa5u8; 0x200]);
                eax(a, 0x0077_4ae0, &[out, n.cast_unsigned()]);
                ffx_system::files::bridge::help_texture_name(b, out, n, l);
                same_memory("help names", a, b);
            }
        }
    }
}

#[test]
fn round13_help_names_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round13_help_names(&mut a, &mut b, &mut Xorshift(0x4013_0011));
}

#[test]
fn round13_help_names_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round13_help_names(&mut a, &mut b, &mut Xorshift(0x4013_0012));
}

/// Round 13's texture names over stand-ins for the languages, the battle
/// status and `dbgPrintf`.
#[allow(clippy::items_after_statements)]
fn round13_texture_names(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let cells = alloc(a, b, 0x10);
    let getter = |cell: u32| {
        let mut v = vec![0xa1];
        v.extend_from_slice(&cell.to_le_bytes());
        v.push(0xc3);
        v
    };
    let ffx = stand_in8(a, b, 0xac0, &getter(cells));
    redirect(a, b, 0x0064_10e0, ffx);
    let tk = stand_in8(a, b, 0xad0, &getter(cells + 4));
    redirect(a, b, 0x0088_7c30, tk);
    let battle = stand_in8(a, b, 0xae0, &getter(cells + 8));
    redirect(a, b, 0x0078_16f0, battle);
    let print = stand_in8(a, b, 0xaf0, &recorder(cells + 12, 0x5a5a_1300));
    redirect(a, b, 0x0062_f500, print);
    struct Calls;
    impl t::TextureName<Oracle> for Calls {
        fn battle_status(&mut self, m: &mut Oracle) -> u32 {
            eax(m, 0x0078_16f0, &[])
        }
        fn tk_lang(&mut self, m: &mut Oracle) -> u32 {
            eax(m, 0x0088_7c30, &[])
        }
        fn ffx_lang(&mut self, m: &mut Oracle) -> u32 {
            eax(m, 0x0064_10e0, &[])
        }
        fn print(&mut self, m: &mut Oracle, fmt: u32) {
            eax(m, 0x0062_f500, &[fmt]);
        }
    }
    let size = alloc(a, b, 8);
    let ids = [
        0x3c00u32,
        0x3d00,
        0x3d01,
        0x3d10,
        0x3d20,
        0x3d30,
        0x3d40,
        0x3d60,
        0x3d80,
        0x3bff,
        0x3d02,
        0x3d31,
        0x3d81,
        0x3e00,
        0x13d00,
        0xffff_3d10,
    ];
    let prefixes = [0x025d_08a0u32, 0x025d_06a0];
    for id in ids {
        for _ in 0..24 {
            put(a, b, cells, [0u32, 9, 10][(rng.next() % 3) as usize]);
            put(a, b, cells + 4, rng.next() & 1);
            put(a, b, cells + 8, rng.next() & 1);
            for p in prefixes {
                let n = rng.next() % 24;
                let mut text: Vec<u8> = (0..n).map(|_| b'a' + (rng.next() % 26) as u8).collect();
                text.push(0);
                let at = a.rebase(p);
                a.write(at, &text).unwrap();
                b.write(at, &text).unwrap();
            }
            // nibbles whose column k (0..=14) keeps index + k within the
            // 16-entry tables (or the byte below 16 for 0x3d80)
            let high = rng.next() & 1;
            let lang_table = matches!(id, 0x3d01 | 0x3d60);
            let nibbles = if id == 0x3d80 {
                rng.next() % 16
            } else {
                rng.next() & 0xff
            };
            let nib = if high == 0 {
                nibbles & 0xf
            } else {
                nibbles >> 4
            };
            let k = (nib & 7) * 2;
            let index = if lang_table {
                rng.next() % 16
            } else {
                rng.next() % (16 - k)
            };
            let high = high | (rng.next() & 0xffff_0000);
            put(a, b, size, [0xa5a5_a5a5u32; 2]);
            put(a, b, cells + 12, 0u32);
            let args = [id, nibbles | (rng.next() & 0xffff_ff00), index, high, size];
            let want = eax(a, 0x008a_cf50, &args);
            let got = t::texture_name(b, args[0], args[1], args[2], args[3], args[4], &mut Calls);
            assert_eq!(want, got, "{args:x?}");
            same_memory("texture names", a, b);
        }
    }
}

#[test]
fn round13_texture_names_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round13_texture_names(&mut a, &mut b, &mut Xorshift(0x4013_0021));
}

#[test]
fn round13_texture_names_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round13_texture_names(&mut a, &mut b, &mut Xorshift(0x4013_0022));
}

/// Round 13's magic file start-up, over logging stand-ins.
#[allow(clippy::items_after_statements)]
fn round13_magic_file(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::menu::bridge as mb;
    let log = alloc(a, b, 0x80);
    let cursor = alloc(a, b, 4);
    let stub = |a: &mut Oracle, b: &mut Oracle, k: u32| {
        stand_in8(
            a,
            b,
            0xb00 + 0x30 * k,
            &logger(cursor, 0x1300_0000 + (k << 8)),
        )
    };
    let printf = stub(a, b, 0);
    let init = stub(a, b, 1);
    for (k, sym) in [(2u32, 0x0064_3260u32), (3, 0x0064_2f10)] {
        let s = stub(a, b, k);
        redirect(a, b, sym, s);
    }
    // resetZanmatoSepList takes no argument: count its calls
    let mut count = vec![0x83, 0x05];
    count.extend_from_slice(&(log + 0x7c).to_le_bytes());
    count.extend_from_slice(&[0x01, 0xc3]);
    let s = stand_in8(a, b, 0xb00 + 0x30 * 4, &count);
    redirect(a, b, 0x007a_8f20, s);
    for (slot, to) in [(0x00b0_c380u32, printf), (0x022f_b6a0, init)] {
        let at = a.rebase(slot);
        a.write(at, &to.to_le_bytes()).unwrap();
        b.write(at, &to.to_le_bytes()).unwrap();
    }
    struct Calls(u32, u32);
    impl mb::MagicFileCalls<Oracle> for Calls {
        fn printf(&mut self, m: &mut Oracle, fmt: u32, arg: Option<u32>) {
            let mut args = vec![fmt];
            args.extend(arg);
            m.call(self.0, Conv::Cdecl, &args).unwrap();
        }
        fn game_over_notify(&mut self, m: &mut Oracle, on: u32) {
            eax(m, 0x0064_3260, &[on]);
        }
        fn init_magic(&mut self, m: &mut Oracle, imports: u32) {
            m.call(self.1, Conv::Cdecl, &[imports]).unwrap();
        }
        fn fog_fade(&mut self, m: &mut Oracle, on: u32) {
            eax(m, 0x0064_2f10, &[on]);
        }
        fn reset_zanmato(&mut self, m: &mut Oracle) {
            eax(m, 0x007a_8f20, &[]);
        }
    }
    let mut calls = Calls(printf, init);
    let ids = [
        0x2c7u32, 0x296, 0x29e, 0xa8, 0x1b2, 0x262, 0, 1, 0x2c6, 0x2c8,
    ];
    for _ in 0..40 {
        let id = if rng.next().is_multiple_of(3) {
            rng.next()
        } else {
            ids[(rng.next() % 10) as usize]
        };
        let handle = if rng.next() & 1 == 0 {
            u32::MAX
        } else {
            rng.next()
        };
        set(
            a,
            b,
            mb::MAGIC_FILE,
            [rng.next(), rng.next(), rng.next(), handle],
        );
        put(a, b, cursor, log);
        put(a, b, log, [0u32; 0x1f]);
        eax(a, 0x009d_a7a0, &[id]);
        mb::magic_file_set_magic_id(b, id, &mut calls);
        same_memory("magic id", a, b);
        if rng.next() & 1 == 0 {
            let start = ids[(rng.next() % 10) as usize];
            set(
                a,
                b,
                mb::MAGIC_FILE,
                [start, rng.next(), rng.next(), handle],
            );
        }
        put(a, b, cursor, log);
        assert_eq!(
            eax(a, 0x009d_a7f0, &[]),
            mb::magic_file_start(b, &mut calls)
        );
        same_memory("magic start", a, b);
    }
}

#[test]
fn round13_magic_file_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round13_magic_file(&mut a, &mut b, &mut Xorshift(0x4013_0031));
}

#[test]
fn round13_magic_file_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round13_magic_file(&mut a, &mut b, &mut Xorshift(0x4013_0032));
}

/// Round 13's message-window placement over a stand-in screen size.
#[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
fn round13_message_windows(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::{menu::bridge as mb, window::bridge as wb};
    use tolib::bridge as t;
    let cells = alloc(a, b, 8);
    // mov eax, [esp+4]; mov ecx, [w]; mov [eax], ecx; mov eax, [esp+8]; mov ecx, [h]; mov [eax], ecx; ret
    let mut code = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x0d];
    code.extend_from_slice(&cells.to_le_bytes());
    code.extend_from_slice(&[0x89, 0x08, 0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d]);
    code.extend_from_slice(&(cells + 4).to_le_bytes());
    code.extend_from_slice(&[0x89, 0x08, 0xc3]);
    let wh = stand_in8(a, b, 0xc00, &code);
    redirect(a, b, 0x0064_0db0, wh);
    let windows = alloc(a, b, 8 * 0x80);
    let messages = alloc(a, b, 8 * 0x80);
    let objects = alloc(a, b, 8 * 0x20);
    let styles = alloc(a, b, 8 * 0x10);
    let mut slots = [0u32; 8];
    for k in 0..8u32 {
        put(a, b, wb::WINDOWS.ptr(a).addr + 4 * k, windows + 0x80 * k);
        put(
            a,
            b,
            wb::MSG_WINDOWS.ptr(a).addr + 4 * k,
            messages + 0x80 * k,
        );
        put(a, b, objects + 0x20 * k + 0x10, styles + 0x10 * k);
        slots[k as usize] = objects + 0x20 * k;
    }
    set(a, b, t::MESSAGE_SLOTS, slots);
    let screens = [
        (640, 480),
        (1280, 720),
        (1920, 1080),
        (2560, 1080),
        (1024, 768),
        (3840, 2160),
        (720, 1280),
    ];
    let maps = [0x17u32, 0x15b, 0, 0x40];
    for _ in 0..3000 {
        let i = (rng.next() % 8) as i32;
        let (sw, sh) = screens[(rng.next() % 7) as usize];
        put(a, b, cells, [sw, sh]);
        set(a, b, mb::MESSAGE_WINDOW_MODE, rng.next() % 3);
        set(a, b, t::CURRENT_MAP, maps[(rng.next() % 4) as usize]);
        put(
            a,
            b,
            styles + 0x10 * i.cast_unsigned() + 4,
            (rng.next() % 8) as u16,
        );
        put(
            a,
            b,
            windows + 0x80 * i.cast_unsigned() + 4,
            (rng.next() % 3) as u16,
        );
        let rect = match rng.next() % 8 {
            0 => [
                400,
                0x110,
                (rng.next() % 500) as i32,
                (rng.next() % 300) as i32,
            ],
            1 => [
                (rng.next() % 600) as i32 - 50,
                (rng.next() % 450) as i32 - 50,
                0x1d8,
                0xf6,
            ],
            2 => [
                rng.next().cast_signed(),
                rng.next().cast_signed(),
                rng.next().cast_signed(),
                rng.next().cast_signed(),
            ],
            _ => [
                (rng.next() % 600) as i32 - 50,
                (rng.next() % 450) as i32 - 50,
                (rng.next() % 520) as i32,
                (rng.next() % 300) as i32,
            ],
        };
        let args = [
            i.cast_unsigned(),
            rect[0].cast_unsigned(),
            rect[1].cast_unsigned(),
            rect[2].cast_unsigned(),
            rect[3].cast_unsigned(),
        ];
        eax(a, 0x008a_fbb0, &args);
        t::place_message_window(b, i, rect, |m| {
            let v: [i32; 2] = Mem::read(m, cells);
            (v[0], v[1])
        });
        same_memory("message windows", a, b);
    }
}

#[test]
fn round13_message_windows_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round13_message_windows(&mut a, &mut b, &mut Xorshift(0x4013_0041));
}

#[test]
fn round13_message_windows_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round13_message_windows(&mut a, &mut b, &mut Xorshift(0x4013_0042));
}

/// Round 14's message-window positions over a stand-in screen size.
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]
fn round14_message_positions(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::{menu::bridge as mb, window::bridge as wb};
    use tolib::bridge as t;
    let cells = alloc(a, b, 8);
    // mov eax, [esp+4]; mov ecx, [w]; mov [eax], ecx; mov eax, [esp+8]; mov ecx, [h]; mov [eax], ecx; ret
    let mut code = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x0d];
    code.extend_from_slice(&cells.to_le_bytes());
    code.extend_from_slice(&[0x89, 0x08, 0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d]);
    code.extend_from_slice(&(cells + 4).to_le_bytes());
    code.extend_from_slice(&[0x89, 0x08, 0xc3]);
    let wh = stand_in8(a, b, 0xc00, &code);
    redirect(a, b, 0x0064_0db0, wh);
    let windows = alloc(a, b, 8 * 0x80);
    let messages = alloc(a, b, 8 * 0x80);
    let objects = alloc(a, b, 8 * 0x20);
    let styles = alloc(a, b, 8 * 0x10);
    let records = alloc(a, b, 8 * 0x60);
    let mut slots = [0u32; 8];
    for k in 0..8u32 {
        put(a, b, wb::WINDOWS.ptr(a).addr + 4 * k, windows + 0x80 * k);
        put(
            a,
            b,
            wb::MSG_WINDOWS.ptr(a).addr + 4 * k,
            messages + 0x80 * k,
        );
        put(a, b, objects + 0x20 * k + 0x10, styles + 0x10 * k);
        put(a, b, objects + 0x20 * k + 0x14, records + 0x60 * k);
        slots[k as usize] = objects + 0x20 * k;
    }
    set(a, b, t::MESSAGE_SLOTS, slots);
    let screens = [(640, 480), (1920, 1080), (2560, 1080), (1024, 768)];
    let maps = [0x5eu32, 0x12e, 0x17, 0x15b, 0];
    for _ in 0..3000 {
        let i = (rng.next() % 8) as i32;
        let (sw, sh) = screens[(rng.next() % 4) as usize];
        put(a, b, cells, [sw, sh]);
        set(a, b, mb::MESSAGE_WINDOW_MODE, rng.next() % 3);
        set(a, b, t::CURRENT_MAP, maps[(rng.next() % 5) as usize]);
        set(a, b, t::TUTO_CHOSEN, rng.next() & 1);
        set(a, b, t::MESSAGE_UNCLAMPED, rng.next() & 1);
        let magic = if rng.next() & 1 == 0 {
            0x281
        } else {
            rng.next() % 0x300
        };
        let file = mb::MAGIC_FILE.get(a);
        set(a, b, mb::MAGIC_FILE, [magic, file[1], file[2], file[3]]);
        let k = i.cast_unsigned();
        put(a, b, styles + 0x10 * k + 4, (rng.next() % 8) as u16);
        put(a, b, windows + 0x80 * k + 4, (rng.next() % 3) as u16);
        let (tw, th) = if rng.next().is_multiple_of(4) {
            (rng.next() as u16, rng.next() as u16)
        } else {
            ((rng.next() % 0x300) as u16, (rng.next() % 0x80) as u16)
        };
        put(a, b, records + 0x60 * k + 0x40, [tw, th]);
        let anchor = if rng.next().is_multiple_of(8) {
            rng.next()
        } else {
            rng.next() % 6
        };
        let (x, y) = if rng.next().is_multiple_of(6) {
            (rng.next().cast_signed(), rng.next().cast_signed())
        } else {
            (
                (rng.next() % 700) as i32 - 100,
                (rng.next() % 600) as i32 - 100,
            )
        };
        eax(
            a,
            0x008a_f850,
            &[k, anchor, x.cast_unsigned(), y.cast_unsigned()],
        );
        t::set_message_window_position(b, i, anchor, x, y, |m| {
            let v: [i32; 2] = Mem::read(m, cells);
            (v[0], v[1])
        });
        same_memory("message positions", a, b);
        let (w, h) = if rng.next().is_multiple_of(6) {
            (rng.next().cast_signed(), rng.next().cast_signed())
        } else {
            (
                (rng.next() % 0x260) as i32 - 0x20,
                (rng.next() % 0x1c0) as i32 - 0x10,
            )
        };
        let args = [
            k,
            anchor,
            x.cast_unsigned(),
            y.cast_unsigned(),
            w.cast_unsigned(),
            h.cast_unsigned(),
        ];
        eax(a, 0x008a_fa90, &args);
        t::set_message_window_rect(b, i, anchor, [x, y, w, h], |m| {
            let v: [i32; 2] = Mem::read(m, cells);
            (v[0], v[1])
        });
        same_memory("message rects", a, b);
    }
}

#[test]
fn round14_message_positions_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round14_message_positions(&mut a, &mut b, &mut Xorshift(0x4014_0001));
}

#[test]
fn round14_message_positions_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round14_message_positions(&mut a, &mut b, &mut Xorshift(0x4014_0002));
}

/// Round 14's battle limit-window set-ups over stand-in scene getters.
#[allow(clippy::items_after_statements, clippy::cast_possible_truncation)]
fn round14_limit_windows(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    // [out] = name ^ key
    let store = |key: u32| {
        let mut v = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x4c, 0x24, 0x0c, 0x35];
        v.extend_from_slice(&key.to_le_bytes());
        v.extend_from_slice(&[0x89, 0x01, 0xc3]);
        v
    };
    // *x = (name ^ key) & 0xffff, *y = (name ^ key) >> 16 (halves)
    let mut point = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x4c, 0x24, 0x0c, 0x35];
    point.extend_from_slice(&0x1400_0303u32.to_le_bytes());
    point.extend_from_slice(&[
        0x66, 0x89, 0x01, 0x8b, 0x4c, 0x24, 0x10, 0xc1, 0xe8, 0x10, 0x66, 0x89, 0x01, 0xc3,
    ]);
    let num = stand_in8(a, b, 0xc40, &store(0x1400_0101));
    let color = stand_in8(a, b, 0xc60, &store(0x1400_0202));
    let pt = stand_in8(a, b, 0xc80, &point);
    redirect(a, b, 0x008a_e410, num);
    redirect(a, b, 0x008a_e360, color);
    redirect(a, b, 0x008a_e450, pt);
    // the limit timer's sound calls are not set up: count its calls instead
    let timer_count = alloc(a, b, 4);
    let mut count = vec![0x83, 0x05];
    count.extend_from_slice(&timer_count.to_le_bytes());
    count.extend_from_slice(&[0x01, 0xc3]);
    let timer = stand_in8(a, b, 0xcb0, &count);
    redirect(a, b, 0x0089_7f00, timer);
    struct Calls;
    impl t::LimitWindowCalls<Oracle> for Calls {
        fn seq_num(&mut self, m: &mut Oracle, i: u32, name: u32, out: u32) {
            eax(m, 0x008a_e410, &[i, name, out]);
        }
        fn seq_point(&mut self, m: &mut Oracle, i: u32, name: u32, x: u32, y: u32) {
            eax(m, 0x008a_e450, &[i, name, x, y]);
        }
        fn seq_color(&mut self, m: &mut Oracle, i: u32, name: u32, out: u32) {
            eax(m, 0x008a_e360, &[i, name, out]);
        }
        fn init_pad(&mut self, m: &mut Oracle) {
            eax(m, 0x008a_dbd0, &[]);
        }
        fn init_limit_timer(&mut self, m: &mut Oracle) {
            eax(m, 0x0089_7f00, &[]);
        }
        fn remap(&mut self, m: &mut Oracle, y: bool, v: f32) -> f32 {
            let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
            let r = m
                .call(at, Conv::Cdecl, &[v.to_bits()])
                .unwrap()
                .st0
                .unwrap();
            #[allow(clippy::cast_possible_truncation)] // exact: an f32 value
            let r = r as f32;
            r
        }
    }
    let fill = |a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift| {
        for (from, len) in [
            (0x0133_d6f0u32, 0x60u32),
            (0x0133_f778, 0x320),
            (0x0133_f7d4, 4),
        ] {
            let bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
            let at = a.rebase(from);
            a.write(at, &bytes).unwrap();
            b.write(at, &bytes).unwrap();
        }
    };
    for _ in 0..4 {
        fill(a, b, rng);
        eax(a, 0x0089_7970, &[]);
        t::btl_init_auron_limit_window(b, &mut Calls);
        same_memory("Auron's limit window", a, b);
        fill(a, b, rng);
        eax(a, 0x0089_7f70, &[]);
        t::btl_init_lulu_limit_window(b, &mut Calls);
        same_memory("Lulu's limit window", a, b);
        fill(a, b, rng);
        eax(a, 0x0089_8180, &[]);
        t::btl_init_tidus_limit_window(b, &mut Calls);
        same_memory("Tidus' limit window", a, b);
        fill(a, b, rng);
        eax(a, 0x0089_7860, &[]);
        t::btl_init_active_chr_cursor(b, &mut Calls);
        same_memory("battle window parameters", a, b);
    }
}

#[test]
fn round14_limit_windows_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round14_limit_windows(&mut a, &mut b, &mut Xorshift(0x4014_0011));
}

#[test]
fn round14_limit_windows_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round14_limit_windows(&mut a, &mut b, &mut Xorshift(0x4014_0012));
}

/// Round 14's battle limit-window openers over stand-in overdrive tables.
#[allow(
    clippy::items_after_statements,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names
)]
fn round14_limit_openers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let cells = alloc(a, b, 0x10);
    let seq = alloc(a, b, 0x40);
    // fld dword [cells + 4k]; ret
    for (k, sym) in [(0u32, 0x00a5_ca70u32), (1, 0x00a5_cb10), (2, 0x00a5_cbd0)] {
        let mut v = vec![0xd9, 0x05];
        v.extend_from_slice(&(cells + 4 * k).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0xd00 + 0x10 * k, &v);
        redirect(a, b, sym, s);
    }
    // mov eax, seq; ret
    let mut v = vec![0xb8];
    v.extend_from_slice(&seq.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0xd40, &v);
    redirect(a, b, 0x00a5_ca60, s);
    // NkzSetLuluLimitTbl: [cells + 12] = group ^ key
    let s = stand_in8(a, b, 0xd50, &recorder(cells + 12, 0x1400_0404));
    redirect(a, b, 0x00a5_cc70, s);
    // tables A/B: eax = arg ^ key
    for (k, sym) in [(0u32, 0x00a5_cb30u32), (1, 0x00a5_cb80)] {
        let mut v = vec![0x8b, 0x44, 0x24, 0x04, 0x35];
        v.extend_from_slice(&(0x1400_0501u32 + 0x0137_0101 * k).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0xd70 + 0x10 * k, &v);
        redirect(a, b, sym, s);
    }
    struct Calls;
    impl t::LimitOpenCalls<Oracle> for Calls {
        fn limit_time(&mut self, m: &mut Oracle, who: t::Limit, arg: u32) -> f32 {
            let sym = match who {
                t::Limit::Auron => 0x00a5_ca70,
                t::Limit::Lulu => 0x00a5_cb10,
                t::Limit::Tidus => 0x00a5_cbd0,
            };
            let at = m.rebase(sym);
            let r = m.call(at, Conv::Cdecl, &[arg]).unwrap().st0.unwrap();
            #[allow(clippy::cast_possible_truncation)] // exact: an f32 value
            let r = r as f32;
            r
        }
        fn auron_buttons(&mut self, m: &mut Oracle, level: u32) -> u32 {
            eax(m, 0x00a5_ca60, &[level])
        }
        fn lulu_table(&mut self, m: &mut Oracle, group: u32) {
            eax(m, 0x00a5_cc70, &[group]);
        }
        fn tidus_table(&mut self, m: &mut Oracle, tb: bool, level: u32) -> u32 {
            eax(m, if tb { 0x00a5_cb80 } else { 0x00a5_cb30 }, &[level])
        }
    }
    let cmds = [
        0x22u32, 0x32, 0x42, 0x52, 0x20, 0x30, 0x40, 0x50, 0x21, 0x62, 0, 0x3078, 0x307a,
    ];
    for _ in 0..60 {
        for (from, len) in [(0x0133_d6f0u32, 0x60u32), (0x0133_f778, 0x30)] {
            let bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
            let at = a.rebase(from);
            a.write(at, &bytes).unwrap();
            b.write(at, &bytes).unwrap();
        }
        let times = [
            rng.next() % 0x4000_0000 + 0x3f00_0000,
            rng.next() % 0x4000_0000 + 0x3f00_0000,
            rng.next() % 0x4000_0000 + 0x3f00_0000,
        ];
        put(a, b, cells, times);
        let n = rng.next() % 30;
        let mut words: Vec<u16> = (0..n).map(|_| (rng.next() % 0xffff) as u16).collect();
        words.push(0xffff);
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        a.write(seq, &bytes).unwrap();
        b.write(seq, &bytes).unwrap();
        let cmd = if rng.next().is_multiple_of(4) {
            rng.next()
        } else {
            cmds[(rng.next() % 13) as usize]
        };
        assert_eq!(
            eax(a, 0x0089_8ad0, &[cmd]),
            t::btl_open_auron_limit_window(b, cmd, &mut Calls)
        );
        same_memory("Auron's limit window opened", a, b);
        assert_eq!(
            eax(a, 0x0089_8bf0, &[cmd]),
            t::btl_open_lulu_limit_window(b, cmd, &mut Calls)
        );
        same_memory("Lulu's limit window opened", a, b);
        assert_eq!(
            eax(a, 0x0089_8ca0, &[cmd]),
            t::btl_open_tidus_limit_window(b, cmd, &mut Calls)
        );
        same_memory("Tidus' limit window opened", a, b);
    }
}

#[test]
fn round14_limit_openers_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round14_limit_openers(&mut a, &mut b, &mut Xorshift(0x4014_0021));
}

#[test]
fn round14_limit_openers_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round14_limit_openers(&mut a, &mut b, &mut Xorshift(0x4014_0022));
}

/// Round 14's limit timer and command-cursor memory.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]
fn round14_timer_and_cursor(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let cell = alloc(a, b, 4);
    // rnd stand-in: eax = [cell]
    let mut v = vec![0xa1];
    v.extend_from_slice(&cell.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0xda0, &v);
    redirect(a, b, 0x0079_89a0, s);
    let w = |a: &mut Oracle, b: &mut Oracle, at: u32, v: u32| {
        let at = a.rebase(at);
        put(a, b, at, v);
    };
    for k in 0..400 {
        w(
            a,
            b,
            0x0133_f77c,
            if k % 5 == 0 { rng.next() % 4 } else { 2 },
        );
        let frames = (rng.next() % 2000) as f32 + if k % 3 == 0 { 0.5 } else { 0.0 };
        w(a, b, 0x0133_f788, frames.to_bits());
        let limit = match k % 7 {
            0 => f32::NAN.to_bits(),
            1 => rng.next(),
            _ => ((rng.next() % 6000) as f32 / 100.0).to_bits(),
        };
        let limit = if f32::from_bits(limit).is_nan() && limit & 0x0040_0000 == 0 {
            f32::NAN.to_bits()
        } else {
            limit
        };
        w(a, b, 0x0133_f78c, limit);
        w(a, b, 0x0133_f794, rng.next());
        w(a, b, 0x0186_5f00, rng.next() & 1);
        put(a, b, cell, rng.next());
        eax(a, 0x0089_1ac0, &[]);
        t::btl_ctrl_limit_timer(b, |m| eax(m, 0x0079_89a0, &[]));
        same_memory("limit timer", a, b);
    }
    for k in 0..200 {
        for (from, len) in [
            (0x0133_c940u32, 0x800u32),
            (0x0133_d770, 0x18),
            (0x0133_d788, 0x168 * 8),
        ] {
            let bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
            let at = a.rebase(from);
            a.write(at, &bytes).unwrap();
            b.write(at, &bytes).unwrap();
        }
        // depths up to 4 levels
        for i in 0..8u32 {
            let at = a.rebase(0x0133_c95a) + 0xf0 * i;
            put(a, b, at, (rng.next() % 6) as u8 - 1);
        }
        let cmds = [0xffffu16, 0xfffe, 0x3002, 0x3001, 0];
        w(
            a,
            b,
            0x0133_ca4c,
            u32::from(cmds[(rng.next() % 5) as usize]) | (rng.next() & 0xffff_0000),
        );
        let at = a.rebase(0x023c_c088);
        put(a, b, at, (rng.next() % 8) as u16);
        let at = a.rebase(0x023c_c092);
        put(
            a,
            b,
            at,
            if k % 9 == 0 {
                0xffu8
            } else {
                (rng.next() % 4) as u8
            },
        );
        eax(a, 0x0089_89d0, &[]);
        t::btl_memory_cursor_pos(b);
        same_memory("cursor memory", a, b);
    }
}

#[test]
fn round14_timer_and_cursor_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round14_timer_and_cursor(&mut a, &mut b, &mut Xorshift(0x4014_0031));
}

#[test]
fn round14_timer_and_cursor_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round14_timer_and_cursor(&mut a, &mut b, &mut Xorshift(0x4014_0032));
}

/// Round 14's target group masks over stand-in battle queries.
#[allow(clippy::cast_possible_truncation)]
fn round14_target_groups(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let rec = alloc(a, b, 0x40);
    let tmp = alloc(a, b, 4);
    // MsGetComData: eax = rec
    let mut v = vec![0xb8];
    v.extend_from_slice(&rec.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0xdc0, &v);
    redirect(a, b, 0x0079_a4b0, s);
    // MsGetChrGroup: (chr ^ key) & 7
    let mut v = vec![0x8b, 0x44, 0x24, 0x04, 0x35];
    v.extend_from_slice(&0x1400_0605u32.to_le_bytes());
    v.extend_from_slice(&[0x83, 0xe0, 0x07, 0xc3]);
    let s = stand_in8(a, b, 0xde0, &v);
    redirect(a, b, 0x0079_5040, s);
    for _ in 0..200 {
        let bytes: Vec<u8> = (0..0x90).map(|_| rng.next() as u8).collect();
        let at = a.rebase(0x0133_d0d0);
        a.write(at, &bytes).unwrap();
        b.write(at, &bytes).unwrap();
        put(a, b, at, (rng.next() % 28) as u16);
        put(a, b, rec + 0x1a, rng.next() as u16);
        eax(a, 0x0089_8490, &[]);
        t::btl_make_target_group_bit(
            b,
            tmp,
            |m, id, p| eax(m, 0x0079_a4b0, &[id, p]),
            |m, chr| eax(m, 0x0079_5040, &[chr]),
        );
        same_memory("target groups", a, b);
    }
}

#[test]
fn round14_target_groups_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round14_target_groups(&mut a, &mut b, &mut Xorshift(0x4014_0041));
}

#[test]
fn round14_target_groups_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round14_target_groups(&mut a, &mut b, &mut Xorshift(0x4014_0042));
}

/// Round 15's target lists over a stand-in command record.
#[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
fn round15_target_lists(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let rec = alloc(a, b, 0x40);
    let tmp = alloc(a, b, 0x10);
    let mut v = vec![0xb8];
    v.extend_from_slice(&rec.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0xdc0, &v);
    redirect(a, b, 0x0079_a4b0, s);
    let fill = |a: &mut Oracle, b: &mut Oracle, from: u32, bytes: &[u8]| {
        let at = a.rebase(from);
        a.write(at, bytes).unwrap();
        b.write(at, bytes).unwrap();
    };
    for k in 0..600 {
        let bytes: Vec<u8> = (0..0x140).map(|_| rng.next() as u8).collect();
        fill(a, b, 0x0133_d0d0, &bytes);
        // sparse group masks, so the selectable mask leaves some
        for g in 0..28u32 {
            let m = match rng.next() % 3 {
                0 => 0,
                1 => 1 << g,
                _ => rng.next() & rng.next(),
            };
            put(a, b, a.rebase(0x0133_d0e4) + 4 * g, m);
        }
        for off in [0x154u32, 0x168, 0x16c, 0x170] {
            let m = if k % 2 == 0 {
                u32::MAX
            } else {
                rng.next() | rng.next() | rng.next()
            };
            put(a, b, a.rebase(0x0133_d000) + off, m);
        }
        put(
            a,
            b,
            a.rebase(0x0133_d0e0),
            if k % 10 == 0 {
                rng.next() % 8
            } else {
                rng.next() % 4
            },
        );
        put(a, b, a.rebase(0x0133_d0d0), (rng.next() % 0x20) as u16);
        put(a, b, a.rebase(0x0133_d0d2), rng.next() as u16);
        let big = k % 25 == 0;
        for (count, ids) in [(0x0133_f0d8u32, 0x0133_f0dcu32), (0x0133_f0ec, 0x0133_f0f0)] {
            let n = if big { rng.next() % 30 } else { rng.next() % 8 };
            put(
                a,
                b,
                a.rebase(count),
                if k % 17 == 0 { n.wrapping_neg() } else { n },
            );
            for i in 0..8u32 {
                put(a, b, a.rebase(ids) + 2 * i, (rng.next() % 28) as u16);
            }
        }
        let windows: Vec<u8> = (0..0x400).map(|_| rng.next() as u8).collect();
        fill(a, b, 0x0133_c950, &windows);
        put(a, b, a.rebase(0x023c_c092), (rng.next() % 4) as u8);
        put(a, b, rec + 0x1a, rng.next() as u16);
        eax(a, 0x0089_8640, &[]);
        t::btl_make_target_target_bit(b, tmp, |m, id, p| eax(m, 0x0079_a4b0, &[id, p]));
        same_memory("target lists", a, b);
    }
}

#[test]
fn round15_target_lists_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round15_target_lists(&mut a, &mut b, &mut Xorshift(0x4015_0001));
}

#[test]
fn round15_target_lists_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round15_target_lists(&mut a, &mut b, &mut Xorshift(0x4015_0002));
}

/// Round 15's small limit-timer setters.
#[allow(clippy::many_single_char_names)]
fn round15_limit_timer_setters(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    for _ in 0..40 {
        let f = rng.float();
        let f = if ffx_bridge::x87::is_snan(f.to_bits()) {
            2.5
        } else {
            f
        };
        eax(a, 0x0089_a2a0, &[f.to_bits()]);
        t::btl_start_limit_timer(b, f);
        same_memory("limit timer start", a, b);
        let (x, y) = (rng.next(), rng.next());
        eax(a, 0x0089_a2d0, &[x, y]);
        t::btl_set_limit_timer_pos(b, x, y);
        same_memory("limit timer position", a, b);
    }
}

#[test]
fn round15_limit_timer_setters_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round15_limit_timer_setters(&mut a, &mut b, &mut Xorshift(0x4015_0011));
}

#[test]
fn round15_limit_timer_setters_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round15_limit_timer_setters(&mut a, &mut b, &mut Xorshift(0x4015_0012));
}

/// Round 15's default command-cursor restore.
#[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
fn round15_default_cursor(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let lists = alloc(a, b, 4 * 0x40);
    let fill = |a: &mut Oracle, b: &mut Oracle, from: u32, bytes: &[u8]| {
        a.write(from, bytes).unwrap();
        b.write(from, bytes).unwrap();
    };
    for k in 0..800u32 {
        let d = rng.next() % 4;
        let c = rng.next() % 8;
        let win = a.rebase(0x0133_c952) + d * 0xf0;
        let saved = a.rebase(0x0133_d788) + (d + c * 10) * 0x24;
        // all windows, all saved records and the target lists
        let bytes: Vec<u8> = (0..0x3c0).map(|_| rng.next() as u8).collect();
        fill(a, b, a.rebase(0x0133_c950), &bytes);
        let bytes: Vec<u8> = (0..0x168 * 8).map(|_| rng.next() as u8).collect();
        fill(a, b, a.rebase(0x0133_d788), &bytes);
        let bytes: Vec<u8> = (0..0x90).map(|_| rng.next() as u8).collect();
        fill(a, b, a.rebase(0x0133_d170), &bytes);
        put(a, b, a.rebase(0x023c_c092), d as u8);
        put(a, b, a.rebase(0x023c_c088), c as u16);
        // the window's command list, sometimes holding the saved command
        let lvl = rng.next() % 4;
        let n = rng.next() % 12;
        let list = lists + 0x40 * (k % 4);
        let entries: Vec<u16> = (0..16).map(|_| (rng.next() % 6) as u16).collect();
        for (i, v) in entries.iter().enumerate() {
            put(a, b, list + 2 * i as u32, *v);
        }
        put(a, b, win + 9, lvl as u8);
        put(a, b, win + 0x1e, list);
        put(
            a,
            b,
            win + 0x22,
            if k % 13 == 0 { 0u32.wrapping_sub(n) } else { n },
        );
        put(a, b, saved + 0xc + 2 * lvl, (rng.next() % 6) as u16);
        put(a, b, win + 0x36, (rng.next() % 4 + 1) as u16);
        put(a, b, win + 0x38, (rng.next() % 4 + 1) as u16);
        if k % 4 != 0 {
            let key: u16 = Mem::read(a, saved + 0x14);
            put(a, b, win + 0xa, key);
        }
        put(a, b, win, (rng.next() % 3) as u8);
        put(a, b, saved, (rng.next() % 3) as u8);
        put(a, b, saved + 0x1a, (rng.next() % 4) as u16);
        put(a, b, saved + 0x18, (rng.next() % 2) as u16);
        // the saved mask, sometimes an exact target entry
        let side = if rng.next() & 1 == 0 {
            0x0133_d180u32
        } else {
            0x0133_d1d4
        };
        let e: u32 = Mem::read(a, a.rebase(side) + 4 * (rng.next() % 4));
        let mask = match rng.next() % 3 {
            0 => e,
            1 => 1 << (rng.next() % 28),
            _ => 0,
        };
        put(a, b, saved + 0x1c, mask);
        put(a, b, a.rebase(0x0133_d17c), (rng.next() % 6) as u16);
        put(a, b, a.rebase(0x0133_d1d0), (rng.next() % 6) as u16);
        eax(a, 0x0089_9ac0, &[]);
        t::btl_set_default_cursor_pos(b);
        same_memory("default cursor", a, b);
    }
}

#[test]
fn round15_default_cursor_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round15_default_cursor(&mut a, &mut b, &mut Xorshift(0x4015_0021));
}

#[test]
fn round15_default_cursor_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round15_default_cursor(&mut a, &mut b, &mut Xorshift(0x4015_0022));
}

/// Round 15's small forwarding functions over logging stand-ins.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round15_forwarders(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::menu::bridge as mb;
    use tolib::bridge as t;
    let log = alloc(a, b, 0x200);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let rec = alloc(a, b, 4);
    let logged = [
        0x008a_df20u32,
        0x00a7_bb00,
        0x00a7_c7f0,
        0x008a_e6f0,
        0x008b_db60,
        0x0088_6e80,
        0x008a_ad70,
        0x008a_a100,
        0x008a_b020,
        0x0090_7d80,
        0x0064_2070,
        0x0064_2050,
        0x0063_95e0,
        0x007a_bdf0,
    ];
    for (k, sym) in logged.into_iter().enumerate() {
        let k = k as u32;
        let s = if matches!(sym, 0x008a_df20 | 0x00a7_bb00 | 0x008a_e6f0) {
            // no arguments: count the call, return a constant
            let mut v = vec![0x83, 0x05];
            v.extend_from_slice(&(cells + 0xc).to_le_bytes());
            v.extend_from_slice(&[0x01, 0xb8]);
            v.extend_from_slice(&(0x1500_0000 + k).to_le_bytes());
            v.push(0xc3);
            stand_in8(a, b, 0xe40 + 0x20 * k, &v)
        } else {
            stand_in8(
                a,
                b,
                0xe40 + 0x20 * k,
                &logger(cursor, 0x1500_0000 + (k << 8)),
            )
        };
        redirect(a, b, sym, s);
    }
    let code = |a: &mut Oracle, b: &mut Oracle, off: u32, sym: u32, v: Vec<u8>| {
        let s = stand_in8(a, b, off, &v);
        redirect(a, b, sym, s);
    };
    // MsGetRomItem(item, &base): base = 0x1234 + item, returns rec
    let mut v = vec![0x8b, 0x4c, 0x24, 0x08, 0x8b, 0x44, 0x24, 0x04, 0x05];
    v.extend_from_slice(&0x1234u32.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x01, 0xb8]);
    v.extend_from_slice(&rec.to_le_bytes());
    v.push(0xc3);
    code(a, b, 0x1100, 0x0079_09f0, v);
    // MagicFile_FindPreloaded(id, &e): e = [cells], returns [cells + 4]
    let mut v = vec![0x8b, 0x4c, 0x24, 0x08, 0xa1];
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x01, 0xa1]);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.push(0xc3);
    code(a, b, 0x1130, 0x009d_ad70, v);
    // MsGetBattleScene: [cells + 8]
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.push(0xc3);
    code(a, b, 0x1150, 0x0078_2970, v);
    // graphicMagicGetPath: a constant
    let mut v = vec![0xb8];
    v.extend_from_slice(&0x0abc_0123u32.to_le_bytes());
    v.push(0xc3);
    code(a, b, 0x1160, 0x0064_2030, v);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let tmp = alloc(a, b, 4);
    let windows = alloc(a, b, 0x100);
    for k in 0..8u32 {
        put(
            a,
            b,
            ffx_system::window::bridge::WINDOWS.ptr(a).addr + 4 * k,
            windows + 0x20 * k,
        );
    }
    for _ in 0..30 {
        put(a, b, cursor, log);
        put(a, b, rec, rng.next() as u16);
        put(
            a,
            b,
            cells,
            [
                rng.next() & 1,
                rng.next() & 1,
                if rng.next() & 1 == 0 {
                    0x018b_0046
                } else {
                    rng.next()
                },
                0,
            ],
        );
        put(a, b, a.rebase(0x0186_a230), rng.next() % 5);
        put(a, b, a.rebase(0x0186_a22c), (rng.next() & 1) as u16);
        put(
            a,
            b,
            a.rebase(0x0194_0abc),
            u32::from(rng.next().is_multiple_of(4)),
        );
        let prev = [0xa5u32, 0x133, 0x152, 0x151, rng.next()][(rng.next() % 5) as usize];
        set(
            a,
            b,
            mb::MAGIC_FILE,
            [rng.next(), rng.next(), prev, rng.next()],
        );
        assert_eq!(eax(a, 0x008a_dee0, &[]), t::reset2(b, call));
        let item = rng.next() % 0x200;
        assert_eq!(
            eax(a, 0x008b_8db0, &[item]),
            t::std_get_item_name(b, item, tmp, call)
        );
        assert_eq!(
            eax(a, 0x008b_8dd0, &[item]),
            t::std_get_weapon_name(b, item, call)
        );
        let (i, k) = (rng.next() % 8, rng.next() % 8);
        eax(a, 0x008b_e020, &[i, k]);
        t::btl_end_set_flag_f(b, i, k, call);
        eax(a, 0x008b_f0b0, &[]);
        t::menu_term_help(b, call);
        eax(a, 0x008e_27e0, &[]);
        t::menu_term_ability_map2(b, call);
        let kind = rng.next();
        eax(a, 0x0090_6e60, &[i, kind]);
        t::set_window_kind(b, i, kind, call);
        assert_eq!(
            eax(a, 0x009d_a700, &[]),
            mb::magic_file_need_restore_camera(b, call)
        );
        assert_eq!(
            eax(a, 0x009d_a780, &[]),
            mb::magic_file_restore_screen_setup(b, call)
        );
        assert_eq!(
            eax(a, 0x009d_ae60, &[item]),
            mb::magic_file_is_prx_preloaded(b, item, tmp, call)
        );
        eax(a, 0x009d_a2d0, &[]);
        mb::magic_file_cleanup(b, call);
        // the original's scratch word is on its stack
        let v: u32 = Mem::read(b, tmp);
        Mem::write(a, tmp, v);
        same_memory("forwarders", a, b);
    }
}

#[test]
fn round15_forwarders_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round15_forwarders(&mut a, &mut b, &mut Xorshift(0x4015_0041));
}

#[test]
fn round15_forwarders_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round15_forwarders(&mut a, &mut b, &mut Xorshift(0x4015_0042));
}

/// Round 15's map-message flag, visit counters, collected bits, block
/// clear and font width by language, over stand-ins.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round15_counters(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x100);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    let block = alloc(a, b, 0x400);
    let s = stand_in8(a, b, 0x1180, &logger(cursor, 0x1500_1100));
    redirect(a, b, 0x0082_2380, s);
    // MsGetSaveMenu: returns cells; 0x007851b0: [cells + 4]; 0x008b3630: block
    let mut v = vec![0xb8];
    v.extend_from_slice(&cells.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x11a0, &v);
    redirect(a, b, 0x0078_5290, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x11b0, &v);
    redirect(a, b, 0x0078_51b0, s);
    let mut v = vec![0xb8];
    v.extend_from_slice(&block.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x11c0, &v);
    redirect(a, b, 0x008b_3630, s);
    // TOGetFFXLang: [cells + 8]; TOGetFontWidth: logs its table, returns it ^ key
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x11d0, &v);
    redirect(a, b, 0x0064_10e0, s);
    let mut v = vec![0x8b, 0x44, 0x24, 0x10, 0x35];
    v.extend_from_slice(&0x1500_1200u32.to_le_bytes());
    v.extend_from_slice(&[0xa3]);
    v.extend_from_slice(&(cells + 0xc).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x11e0, &v);
    redirect(a, b, 0x008b_9330, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let maps = [0xa2u32, 0x11, 0x8a, 0x99, 0x76, 0xf5, 0x12, 0];
    for k in 0..400u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells,
            [
                rng.next() & 0x200,
                rng.next() & 1,
                [0u32, 9, 10, 1, 5][(rng.next() % 5) as usize],
            ],
        );
        put(
            a,
            b,
            a.rebase(0x00c5_9558),
            [0u32, 1, u32::MAX, 7][(rng.next() % 4) as usize],
        );
        set(a, b, t::CURRENT_MAP, maps[(rng.next() % 8) as usize]);
        let id = [0u32, 1, 2, 3, 4, 5, 7, 0x17, 0x24, 0x3b][(rng.next() % 10) as usize];
        assert_eq!(
            eax(a, 0x008a_f6a0, &[id]),
            t::check_map_message(b, id, call)
        );
        put(
            a,
            b,
            a.rebase(0x00c5_a03c),
            [1u8, 0, 0xff][(rng.next() % 3) as usize],
        );
        let map: u32 = Mem::read(a, a.rebase(0x00c3_347c));
        put(
            a,
            b,
            a.rebase(0x0133_0844),
            if rng.next() & 1 == 0 { map } else { rng.next() },
        );
        put(
            a,
            b,
            a.rebase(0x0186_63e4),
            [0x39u32, 0x3a, u32::MAX, 3][(rng.next() % 4) as usize],
        );
        eax(a, 0x008b_5680, &[]);
        t::count_map_visits(b, call);
        let n = [u32::MAX, 0, 5, 0x19, 0x1a, 0xff, 0x100][(rng.next() % 7) as usize];
        put(
            a,
            b,
            a.rebase(0x0113_07a0),
            if k % 3 == 0 {
                0x03ff_ffff ^ (1 << (rng.next() % 26))
            } else {
                rng.next()
            },
        );
        eax(a, 0x0090_6b30, &[n]);
        t::collect_bit(b, n, call);
        if k % 20 == 0 {
            let bytes: Vec<u8> = (0..0x400).map(|_| rng.next() as u8).collect();
            a.write(block, &bytes).unwrap();
            b.write(block, &bytes).unwrap();
            eax(a, 0x008b_3610, &[]);
            t::clear_block_159e468(b, call);
        }
        let (text, kind, out) = (rng.next(), rng.next(), rng.next());
        assert_eq!(
            eax(a, 0x008b_7c00, &[text, kind, out, rng.next()]),
            t::font_width_for_lang(
                b,
                |m| eax(m, 0x0064_10e0, &[]),
                |m, table| { eax(m, 0x008b_9330, &[text, kind, out, table]) }
            )
        );
        same_memory("counters", a, b);
    }
}

#[test]
fn round15_counters_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round15_counters(&mut a, &mut b, &mut Xorshift(0x4015_0051));
}

#[test]
fn round15_counters_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round15_counters(&mut a, &mut b, &mut Xorshift(0x4015_0052));
}

/// Round 16's reverse nearest-target search over stand-in positions and
/// distances, with the CRT's `_CIsqrt` and `_CIacos`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::items_after_statements
)]
fn round16_near_target(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let table = alloc(a, b, 32 * 16);
    let dists = alloc(a, b, 32);
    let vec = alloc(a, b, 16);
    let mut pos = vec![
        0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x1f, 0xc1, 0xe0, 0x04, 0x05,
    ];
    pos.extend_from_slice(&table.to_le_bytes());
    pos.extend_from_slice(&[
        0x8b, 0x4c, 0x24, 0x08, 0x8b, 0x10, 0x89, 0x11, 0x8b, 0x50, 0x04, 0x89, 0x51, 0x04, 0x8b,
        0x50, 0x08, 0x89, 0x51, 0x08, 0x8b, 0x50, 0x0c, 0x89, 0x51, 0x0c, 0x31, 0xc0, 0xc3,
    ]);
    let s = stand_in8(a, b, 0xe40, &pos);
    redirect(a, b, 0x0079_48e0, s);
    let mut dist = vec![0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x1f, 0x0f, 0xb6, 0x80];
    dist.extend_from_slice(&dists.to_le_bytes());
    dist.push(0xc3);
    let s = stand_in8(a, b, 0xe80, &dist);
    redirect(a, b, 0x0079_4e00, s);
    // fld qword [esp+4]; call [esp+0xc]; ret; ... ; fstp st(0); ret
    let tramp = stand_in8(
        a,
        b,
        0xea0,
        &[
            0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90,
            0x90, 0x90, 0xdd, 0xd8, 0xc3,
        ],
    );
    struct Calls(u32);
    impl Calls {
        fn intrinsic(&self, m: &mut Oracle, sym: u32, x: f64) -> f64 {
            let f = m.rebase(sym);
            let bits = x.to_bits();
            let r = m
                .call(self.0, Conv::Cdecl, &[bits as u32, (bits >> 32) as u32, f])
                .unwrap()
                .st0
                .unwrap();
            m.call(self.0 + 16, Conv::Cdecl, &[]).unwrap();
            r
        }
    }
    impl t::NearTarget<Oracle> for Calls {
        fn btl_pos(&mut self, m: &mut Oracle, chr: u32, vec: u32) {
            eax(m, 0x0079_48e0, &[chr, vec]);
        }
        fn dist(&mut self, m: &mut Oracle, chr: u32) -> u32 {
            eax(m, 0x0079_4e00, &[chr])
        }
        fn sqrt(&mut self, m: &mut Oracle, x: f64) -> f64 {
            self.intrinsic(m, 0x0094_97b8, x)
        }
        fn acos(&mut self, m: &mut Oracle, x: f64) -> f64 {
            self.intrinsic(m, 0x0094_98da, x)
        }
    }
    let mut calls = Calls(tramp);
    let mask = |rng: &mut Xorshift, lo: u32, n: u32| {
        let m = (rng.next() & ((1 << n) - 1)) << lo;
        if m == 0 {
            1 << (lo + rng.next() % n)
        } else {
            m
        }
    };
    for k in 0..400u32 {
        for chr in 0..32u32 {
            let v: [f32; 4] = if k % 9 == 0 && chr % 3 == 0 {
                [0.0, 0.0, 0.0, 1.0]
            } else {
                [
                    (rng.next() % 4000) as f32 / 10.0 - 200.0,
                    (rng.next() % 100) as f32 / 10.0,
                    (rng.next() % 4000) as f32 / 10.0 - 200.0,
                    1.0,
                ]
            };
            put(a, b, table + 16 * chr, v);
            put(a, b, dists + chr, u8::from(!rng.next().is_multiple_of(3)));
        }
        let n1 = rng.next() % 4 + u32::from(k % 7 != 0);
        let n2 = rng.next() % 4 + u32::from(k % 5 != 0);
        for i in 0..20 {
            put(
                a,
                b,
                a.rebase(0x0133_d180) + 4 * i,
                if i < n1 { mask(rng, 0, 20) } else { 0 },
            );
        }
        for i in 0..8 {
            put(
                a,
                b,
                a.rebase(0x0133_d1d4) + 4 * i,
                if i < n2 { mask(rng, 20, 8) } else { 0 },
            );
        }
        put(a, b, a.rebase(0x0133_d17c), n1 as u16);
        put(a, b, a.rebase(0x0133_d1d0), n2 as u16);
        put(a, b, a.rebase(0x0133_d1f4), (rng.next() % 28) as u16);
        let n_other = {
            let c: u16 = Mem::read(a, a.rebase(0x0133_d1f4));
            if c < 0x14 { n2 } else { n1 }
        };
        let want = eax(a, 0x0089_90f0, &[]);
        let got = t::btl_search_rev_near_target(b, vec, &mut calls);
        if n_other > 0 {
            assert_eq!(want, got, "case {k}");
        }
        let v: [u32; 4] = Mem::read(b, vec);
        Mem::write(a, vec, v);
        same_memory("reverse near target", a, b);
    }
}

#[test]
fn round16_near_target_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round16_near_target(&mut a, &mut b, &mut Xorshift(0x4016_0001));
}

#[test]
fn round16_near_target_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round16_near_target(&mut a, &mut b, &mut Xorshift(0x4016_0002));
}

/// Round 16's message-window set-ups.
#[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
fn round16_message_setup(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::menu::bridge as mb;
    use tolib::bridge as t;
    let objects = alloc(a, b, 8 * 0x20);
    let records = alloc(a, b, 8 * 3 * 0x40);
    let mut slots = [0u32; 8];
    for k in 0..8u32 {
        for (j, off) in [0x10u32, 0x14, 0x18].into_iter().enumerate() {
            put(
                a,
                b,
                objects + 0x20 * k + off,
                records + 0xc0 * k + 0x40 * j as u32,
            );
        }
        slots[k as usize] = objects + 0x20 * k;
    }
    let msg = alloc(a, b, 0x140);
    for _ in 0..8 {
        set(a, b, t::MESSAGE_SLOTS, slots);
        let bytes: Vec<u8> = (0..8 * 3 * 0x40).map(|_| rng.next() as u8).collect();
        a.write(records, &bytes).unwrap();
        b.write(records, &bytes).unwrap();
        eax(a, 0x008a_dbf0, &[]);
        t::init_scene_message_window(b);
        same_memory("scene message windows", a, b);
        let bank = [0u32, 1, 2, u32::MAX][(rng.next() % 4) as usize];
        eax(a, 0x008b_86e0, &[bank]);
        t::set_exe_message_area(b, bank);
        same_memory("message areas", a, b);
        let bytes: Vec<u8> = (0..0x140).map(|_| rng.next() as u8).collect();
        a.write(msg, &bytes).unwrap();
        b.write(msg, &bytes).unwrap();
        put(a, b, msg + 8, rng.next() % 6);
        set(a, b, mb::MESSAGE_WINDOW_MODE, rng.next() % 3);
        eax(a, 0x008d_f6d0, &[msg]);
        t::init_message_call_back(b, msg);
        same_memory("message callbacks", a, b);
    }
}

#[test]
fn round16_message_setup_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round16_message_setup(&mut a, &mut b, &mut Xorshift(0x4016_0011));
}

#[test]
fn round16_message_setup_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round16_message_setup(&mut a, &mut b, &mut Xorshift(0x4016_0012));
}

/// Round 16's save folder name and macro number text.
#[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
fn round16_save_name_and_numbers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let cell = alloc(a, b, 4);
    let mut getter = vec![0xa1];
    getter.extend_from_slice(&cell.to_le_bytes());
    getter.push(0xc3);
    let s = stand_in8(a, b, 0xec0, &getter);
    redirect(a, b, 0x0064_10e0, s);
    for lang in 0..0x16u32 {
        let mut pre: Vec<u8> = (0..rng.next() % 12)
            .map(|_| b'a' + (rng.next() % 26) as u8)
            .collect();
        pre.resize(0x40, 0);
        let at = a.rebase(0x023c_c0a0);
        a.write(at, &pre).unwrap();
        b.write(at, &pre).unwrap();
        put(a, b, cell, lang);
        put(a, b, a.rebase(0x0186_6240), rng.next());
        eax(a, 0x008b_3a40, &[]);
        t::menu_make_save_file_name(b, lang);
        same_memory("save folder name", a, b);
    }
    let cmd = alloc(a, b, 8);
    for k in 0..300u32 {
        let v = match k % 4 {
            0 => rng.next(),
            1 => rng.next() % 2_000_000,
            2 => (rng.next() % 2000).wrapping_neg(),
            _ => [0u32, 999, 1000, 0x8000_0000, 0x7fff_ffff, 100_000][(rng.next() % 6) as usize],
        };
        put(a, b, cmd, [u32::from(rng.next() as u8 % 4) << 8, v]);
        put(a, b, a.rebase(0x0186_9ee4), [0xa5u8; 0x20]);
        assert_eq!(eax(a, 0x008b_6210, &[cmd]), t::func_macro_cmd_value(b, cmd));
        same_memory("macro numbers", a, b);
    }
}

#[test]
fn round16_save_name_and_numbers_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round16_save_name_and_numbers(&mut a, &mut b, &mut Xorshift(0x4016_0021));
}

#[test]
fn round16_save_name_and_numbers_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round16_save_name_and_numbers(&mut a, &mut b, &mut Xorshift(0x4016_0022));
}

/// Round 16's pad repeat over stand-in controller history and time.
#[allow(
    clippy::cast_possible_truncation,
    clippy::items_after_statements,
    clippy::many_single_char_names
)]
fn round16_pad_status(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let cells = alloc(a, b, 0x10);
    // inputGetCurTime: fld dword [cells]; TkGetControlerHist: [cells + 4];
    // TkGetControlerAnalogHist: byte [cells + 8 + axis]
    let mut v = vec![0xd9, 0x05];
    v.extend_from_slice(&cells.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0xf00, &v);
    redirect(a, b, 0x0063_0c60, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0xf10, &v);
    redirect(a, b, 0x0088_8d70, s);
    let mut v = vec![0x8b, 0x44, 0x24, 0x0c, 0x83, 0xe0, 0x03, 0x0f, 0xb6, 0x80];
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0xf20, &v);
    redirect(a, b, 0x0088_8c20, s);
    struct Pad;
    impl t::PadInput<Oracle> for Pad {
        fn cur_time(&mut self, m: &mut Oracle) -> f64 {
            let at = m.rebase(0x0063_0c60);
            m.call(at, Conv::Cdecl, &[]).unwrap().st0.unwrap()
        }
        fn hist(&mut self, m: &mut Oracle, a: u32, b: u32, c: u32) -> u32 {
            eax(m, 0x0088_8d70, &[a, b, c])
        }
        fn analog(&mut self, m: &mut Oracle, a: u32, b: u32, axis: u32, d: u32) -> u32 {
            eax(m, 0x0088_8c20, &[a, b, axis, d])
        }
    }
    let base = a.rebase(0x025d_09c0);
    let bytes: Vec<u8> = (0..0xc0).map(|_| rng.next() as u8).collect();
    a.write(base, &bytes).unwrap();
    b.write(base, &bytes).unwrap();
    for k in 0..16u32 {
        put(a, b, a.rebase(0x025d_0a00) + 4 * k, 0f32);
        put(a, b, a.rebase(0x025d_0a40) + 4 * k, 0f32);
    }
    let mut time = 0f32;
    let mut held = 0u32;
    for k in 0..600u32 {
        time += [0.016f32, 0.033, 0.1, 0.25][(rng.next() % 4) as usize];
        if rng.next().is_multiple_of(4) {
            held ^= 1 << (rng.next() % 16);
        }
        put(a, b, cells, time);
        put(a, b, cells + 4, held | (rng.next() & 0xffff_0000));
        let sticks = [rng.next() as u8, (rng.next() % 3) as u8 * 0x7f, 0x10, 0xf0];
        put(
            a,
            b,
            cells + 8,
            [
                0u8,
                0,
                sticks[(rng.next() % 4) as usize],
                sticks[(rng.next() % 4) as usize],
            ],
        );
        put(a, b, base + 1, u8::from(k % 3 != 0));
        put(a, b, base, u8::from(rng.next().is_multiple_of(2)));
        eax(a, 0x008b_7cd0, &[]);
        t::get_pad_status(b, &mut Pad);
        same_memory("pad status", a, b);
    }
}

#[test]
fn round16_pad_status_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round16_pad_status(&mut a, &mut b, &mut Xorshift(0x4016_0031));
}

#[test]
fn round16_pad_status_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round16_pad_status(&mut a, &mut b, &mut Xorshift(0x4016_0032));
}

/// Round 16's menu pad repeat over stand-in controllers and time.
#[allow(
    clippy::cast_possible_truncation,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]
fn round16_menu_pad_status(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let cells = alloc(a, b, 0x40);
    let getter = |off: u32| {
        let mut v = vec![0xa1];
        v.extend_from_slice(&(cells + off).to_le_bytes());
        v.push(0xc3);
        v
    };
    let mut v = vec![0xd9, 0x05];
    v.extend_from_slice(&cells.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0xf00, &v);
    redirect(a, b, 0x0063_0c60, s);
    // no-argument getters: the dummy flag, flush, dummy pads
    for (k, sym) in [
        0x008c_a510u32,
        0x008c_a490,
        0x008c_a500,
        0x008c_a530,
        0x008c_a520,
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let s = stand_in8(a, b, 0xf40 + 0x10 * k, &getter(4 + 4 * k));
        redirect(a, b, sym, s);
    }
    // the history getters: [cells + 0x20 + 4k] ^ (third argument)
    for (k, sym) in [0x0088_8d70u32, 0x0088_8e80, 0x0088_8e50]
        .into_iter()
        .enumerate()
    {
        let k = k as u32;
        let mut v = vec![0x8b, 0x44, 0x24, 0x0c, 0x33, 0x05];
        v.extend_from_slice(&(cells + 0x20 + 4 * k).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0xfa0 + 0x10 * k, &v);
        redirect(a, b, sym, s);
    }
    // sticks: byte [cells + 0x30 + (axis & 3)] for both analog getters
    let mut v = vec![0x8b, 0x44, 0x24, 0x0c, 0x83, 0xe0, 0x03, 0x0f, 0xb6, 0x80];
    v.extend_from_slice(&(cells + 0x30).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0xfe0, &v);
    redirect(a, b, 0x0088_8c20, s);
    let mut v = vec![0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x03, 0x0f, 0xb6, 0x80];
    v.extend_from_slice(&(cells + 0x34).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x1000, &v);
    redirect(a, b, 0x008c_a320, s);
    let base = a.rebase(0x025d_09c0);
    let bytes: Vec<u8> = (0..0xc0).map(|_| rng.next() as u8).collect();
    a.write(base, &bytes).unwrap();
    b.write(base, &bytes).unwrap();
    for k in 0..16u32 {
        put(a, b, a.rebase(0x025d_0a00) + 4 * k, 0f32);
        put(a, b, a.rebase(0x025d_0a40) + 4 * k, 0f32);
    }
    let mut time = 0f32;
    let mut held = 0u32;
    for _ in 0..600u32 {
        time += [0.016f32, 0.033, 0.1, 0.25][(rng.next() % 4) as usize];
        if rng.next().is_multiple_of(4) {
            held ^= 1 << (rng.next() % 16);
        }
        let flag = u32::from(rng.next().is_multiple_of(8));
        put(
            a,
            b,
            cells,
            [time.to_bits(), flag, 0, rng.next(), rng.next(), rng.next()],
        );
        put(
            a,
            b,
            cells + 0x20,
            [held, rng.next() & 0xff, rng.next() & 0xff],
        );
        let sticks = [rng.next() as u8, 0x7f, 0x10, 0xf0];
        put(
            a,
            b,
            cells + 0x30,
            [
                0u8,
                0,
                sticks[(rng.next() % 4) as usize],
                sticks[(rng.next() % 4) as usize],
            ],
        );
        put(
            a,
            b,
            cells + 0x34,
            [0u8, 0, rng.next() as u8, rng.next() as u8],
        );
        eax(a, 0x008b_e500, &[]);
        t::menu_get_pad_status(
            b,
            |m| {
                let at = m.rebase(0x0063_0c60);
                m.call(at, Conv::Cdecl, &[]).unwrap().st0.unwrap()
            },
            eax,
        );
        same_memory("menu pad status", a, b);
    }
}

#[test]
fn round16_menu_pad_status_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round16_menu_pad_status(&mut a, &mut b, &mut Xorshift(0x4016_0041));
}

#[test]
fn round16_menu_pad_status_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round16_menu_pad_status(&mut a, &mut b, &mut Xorshift(0x4016_0042));
}

/// Round 16's kernel menu accessors over stand-in save getters.
#[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
fn round16_kernel_accessors(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let rec = alloc(a, b, 0x80);
    let table = alloc(a, b, 0x1d8 * 8);
    // MsGetSaveCommand(chr, cmd): chr * 0x100 ^ cmd ^ key
    let mut v = vec![
        0x8b, 0x44, 0x24, 0x04, 0xc1, 0xe0, 0x08, 0x33, 0x44, 0x24, 0x08, 0x35,
    ];
    v.extend_from_slice(&0x1600_0001u32.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x1040, &v);
    redirect(a, b, 0x0078_5020, s);
    // MsGetSavePlayerPtr: rec + 8 * (chr & 3)
    let mut v = vec![
        0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x03, 0xc1, 0xe0, 0x03, 0x05,
    ];
    v.extend_from_slice(&rec.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x1060, &v);
    redirect(a, b, 0x0077_6f50, s);
    // Kernel_GetAbilityName, MsGetSaveItemNum, MsImportantGet: arg ^ key
    for (k, sym) in [0x0077_4720u32, 0x0079_04b0, 0x0079_0800]
        .into_iter()
        .enumerate()
    {
        let mut v = vec![0x8b, 0x44, 0x24, 0x04, 0x35];
        v.extend_from_slice(&(0x1600_0100u32 + 0x100 * k as u32).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0x1080 + 0x10 * k as u32, &v);
        redirect(a, b, sym, s);
    }
    set(a, b, Global::<u32>::new(0x0112_a034), table);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let cases: [(char, u32, u32); 58] = [
        ('C', 0x0077_24c0, 0x0079_04b0),
        ('C', 0x0077_24f0, 0x0079_0800),
        ('B', 0x0077_2370, 0x50),
        ('B', 0x0077_23e0, 0x58),
        ('B', 0x0077_2450, 0x54),
        ('A', 0x0077_1c60, 0x30b8),
        ('A', 0x0077_1c80, 0x30b9),
        ('A', 0x0077_1cd0, 0x30ba),
        ('A', 0x0077_1d20, 0x30bb),
        ('A', 0x0077_1d70, 0x30bc),
        ('A', 0x0077_1dc0, 0x30bd),
        ('A', 0x0077_1e10, 0x30be),
        ('A', 0x0077_1e30, 0x30bf),
        ('A', 0x0077_1e80, 0x30c0),
        ('A', 0x0077_1ed0, 0x30c1),
        ('A', 0x0077_1f20, 0x30c2),
        ('A', 0x0077_1f70, 0x30c3),
        ('A', 0x0077_1fc0, 0x30c4),
        ('A', 0x0077_1fe0, 0x30c5),
        ('A', 0x0077_2030, 0x30c6),
        ('A', 0x0077_2080, 0x30c7),
        ('A', 0x0077_20d0, 0x30c8),
        ('A', 0x0077_2120, 0x30c9),
        ('A', 0x0077_2170, 0x30ca),
        ('A', 0x0077_2190, 0x30cb),
        ('A', 0x0077_21e0, 0x30cc),
        ('A', 0x0077_2230, 0x30cd),
        ('A', 0x0077_2280, 0x30ce),
        ('A', 0x0077_22f0, 0x30cf),
        ('A', 0x0077_2350, 0x30d0),
        ('A', 0x0077_2390, 0x30d1),
        ('A', 0x0077_2400, 0x30d2),
        ('A', 0x0077_2470, 0x30d3),
        ('A', 0x0077_24d0, 0x30d4),
        ('D', 0x0077_1c30, 0x11e),
        ('D', 0x0077_1ca0, 0x120),
        ('D', 0x0077_1cf0, 0x122),
        ('D', 0x0077_1d40, 0x124),
        ('D', 0x0077_1d90, 0x126),
        ('D', 0x0077_1de0, 0x128),
        ('D', 0x0077_1e50, 0x12a),
        ('D', 0x0077_1ea0, 0x12c),
        ('D', 0x0077_1ef0, 0x12e),
        ('D', 0x0077_1f40, 0x130),
        ('D', 0x0077_1f90, 0x132),
        ('D', 0x0077_2000, 0x134),
        ('D', 0x0077_2050, 0x136),
        ('D', 0x0077_20a0, 0x138),
        ('D', 0x0077_20f0, 0x13a),
        ('D', 0x0077_2140, 0x13c),
        ('D', 0x0077_21b0, 0x13e),
        ('D', 0x0077_2200, 0x140),
        ('D', 0x0077_2250, 0x142),
        ('D', 0x0077_22c0, 0x144),
        ('D', 0x0077_2320, 0x146),
        ('D', 0x0077_23b0, 0x148),
        ('D', 0x0077_2420, 0x14a),
        ('D', 0x0077_2490, 0x14c),
    ];
    for _ in 0..4 {
        let bytes: Vec<u8> = (0..0x80).map(|_| rng.next() as u8).collect();
        a.write(rec, &bytes).unwrap();
        b.write(rec, &bytes).unwrap();
        let bytes: Vec<u8> = (0..0x1d8 * 8).map(|_| rng.next() as u8).collect();
        a.write(table, &bytes).unwrap();
        b.write(table, &bytes).unwrap();
        for &(kind, at, p) in &cases {
            let arg = rng.next() % 8;
            let want = eax(a, at, &[arg]);
            let got = match kind {
                'A' => t::kernel_save_command(b, arg, p, call),
                'B' => t::kernel_player_word(b, arg, p, call),
                'C' => t::kernel_forward(b, p, arg, call),
                _ => t::kernel_slot_ability_name(b, arg, p, call),
            };
            assert_eq!(want, got, "{at:#x}");
        }
        same_memory("kernel accessors", a, b);
    }
}

#[test]
fn round16_kernel_accessors_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round16_kernel_accessors(&mut a, &mut b, &mut Xorshift(0x4016_0051));
}

#[test]
fn round16_kernel_accessors_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round16_kernel_accessors(&mut a, &mut b, &mut Xorshift(0x4016_0052));
}

/// Round 17's message and battle-target helpers over stand-ins.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round17_helpers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::{menu::bridge as mb, window::bridge as wb};
    use tolib::bridge as t;
    let log = alloc(a, b, 0x200);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x40);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    // no-argument stand-ins count their calls in cells + 4k
    for (k, sym) in [
        0x0089_85b0u32,
        0x0089_8490,
        0x0089_8330,
        0x0089_83f0,
        0x0089_8640,
        0x008b_7cd0,
        0x008a_bd00,
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        let mut v = vec![0x83, 0x05];
        v.extend_from_slice(&(cells + 4 * k).to_le_bytes());
        v.extend_from_slice(&[0x01, 0xb8]);
        v.extend_from_slice(&(0x1700_0000 + k).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0x100 + 0x10 * k, &v);
        redirect(a, b, sym, s);
    }
    // TOControlWindow: i & 1; the others log their first argument
    let s = stand_in8(
        a,
        b,
        0x180,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x01, 0xc3],
    );
    redirect(a, b, 0x0090_6bd0, s);
    for (k, sym) in [0x008b_5850u32, 0x008b_8af0, 0x008b_6b70]
        .into_iter()
        .enumerate()
    {
        let k = k as u32;
        let s = stand_in8(
            a,
            b,
            0x190 + 0x18 * k,
            &logger(cursor, 0x1700_0100 + (k << 8)),
        );
        redirect(a, b, sym, s);
    }
    // MsGetSaveMenu: cells + 0x20
    let mut v = vec![0xb8];
    v.extend_from_slice(&(cells + 0x20).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x1e0, &v);
    redirect(a, b, 0x0078_5290, s);
    // TOGetFontWidthForLang: *out = (float) *text, returns text + 1
    let mut v = vec![0x8b, 0x44, 0x24, 0x04, 0x0f, 0xb6, 0x08, 0x89, 0x0d];
    v.extend_from_slice(&(cells + 0x24).to_le_bytes());
    v.extend_from_slice(&[0xdb, 0x05]);
    v.extend_from_slice(&(cells + 0x24).to_le_bytes());
    v.extend_from_slice(&[0x8b, 0x54, 0x24, 0x0c, 0xd9, 0x1a, 0x40, 0xc3]);
    let s = stand_in8(a, b, 0x1f0, &v);
    redirect(a, b, 0x008b_7c00, s);
    let objects = alloc(a, b, 8 * 0x20);
    let recs = alloc(a, b, 8 * 0x10);
    let msgs = alloc(a, b, 8 * 0x140);
    let mut slots = [0u32; 8];
    for k in 0..8u32 {
        slots[k as usize] = objects + 0x20 * k;
        put(a, b, wb::MSG_WINDOWS.ptr(a).addr + 4 * k, msgs + 0x140 * k);
    }
    let text = alloc(a, b, 0x40);
    let scratch = alloc(a, b, 4);
    for k in 0..60u32 {
        put(a, b, cursor, log);
        let bytes: Vec<u8> = (0..0x140).map(|_| rng.next() as u8).collect();
        let at = a.rebase(0x0133_d0d0);
        a.write(at, &bytes).unwrap();
        b.write(at, &bytes).unwrap();
        put(a, b, a.rebase(0x023c_c092), (rng.next() % 4) as u8);
        put(
            a,
            b,
            a.rebase(0x0133_c94c),
            if k % 2 == 0 { 0x100 } else { rng.next() },
        );
        let (chr, cmd) = (rng.next(), rng.next());
        assert_eq!(
            eax(a, 0x0089_abd0, &[chr, cmd]),
            t::btl_check_command_target(b, chr, cmd, call)
        );
        // message windows
        set(a, b, t::MESSAGE_SLOTS, slots);
        for i in 0..8u32 {
            put(a, b, objects + 0x20 * i, (rng.next() % 2) as u16);
            put(a, b, objects + 0x20 * i + 6, (rng.next() % 3) as u16);
            put(a, b, objects + 0x20 * i + 0x10, recs + 0x10 * i);
            put(a, b, recs + 0x10 * i + 8, rng.next() as u16);
        }
        let pad = [0x100u16, 0x100, 0, 0x100][(rng.next() % 4) as usize];
        put(
            a,
            b,
            a.rebase(0x025d_09d2),
            [pad, [0x100u16, 7][(rng.next() % 2) as usize]],
        );
        eax(a, 0x008a_b910, &[]);
        t::control_message_window(b, call);
        // message types
        let bytes: Vec<u8> = (0..8 * 0x140).map(|_| rng.next() as u8).collect();
        a.write(msgs, &bytes).unwrap();
        b.write(msgs, &bytes).unwrap();
        put(a, b, cells + 0x20, rng.next() & 0x100);
        set(a, b, mb::MESSAGE_WINDOW_MODE, rng.next() % 3);
        let (i, kind) = (rng.next() % 8, rng.next() & 0xf7f3);
        let kind = if rng.next().is_multiple_of(3) {
            (kind & !0xf) | 3
        } else {
            kind & !0xc
        };
        assert_eq!(
            eax(a, 0x008b_8bb0, &[i, kind]),
            t::set_message_type(b, i, kind, call)
        );
        // text widths
        let n = rng.next() % 30;
        let mut s: Vec<u8> = (0..n).map(|_| (rng.next() % 255 + 1) as u8).collect();
        s.push(0);
        a.write(text, &s).unwrap();
        b.write(text, &s).unwrap();
        let arg = rng.next();
        let at = a.rebase(0x008b_70c0);
        let want = a.call(at, Conv::Cdecl, &[text, arg]).unwrap().st0.unwrap();
        let got = t::text_width(b, text, arg, scratch, |m, p, out, arg| {
            eax(m, 0x008b_7c00, &[p, 0, out, arg])
        });
        assert_eq!(want, f64::from(got));
        let v: u32 = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory("round 17 helpers", a, b);
    }
    for bank in [0u32, 1, 7] {
        eax(a, 0x008a_f5c0, &[bank]);
        t::set_exe_message_window_area(b, bank);
        same_memory("exe message window area", a, b);
    }
}

#[test]
fn round17_helpers_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round17_helpers(&mut a, &mut b, &mut Xorshift(0x4017_0001));
}

#[test]
fn round17_helpers_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round17_helpers(&mut a, &mut b, &mut Xorshift(0x4017_0002));
}

/// Round 17's second batch of kernel menu accessors over stand-ins.
#[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
fn round17_kernel_accessors(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let table = alloc(a, b, 0x1d8 * 8);
    // MsGetSaveCommand(chr, cmd): chr * 0x100 ^ cmd ^ key
    let mut v = vec![
        0x8b, 0x44, 0x24, 0x04, 0xc1, 0xe0, 0x08, 0x33, 0x44, 0x24, 0x08, 0x35,
    ];
    v.extend_from_slice(&0x1700_0001u32.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x0078_5020, s);
    // Kernel_GetAbilityName, Kernel_GetItemName, MsGetMonsterCapture: arg ^ key
    for (k, sym) in [0x0077_4720u32, 0x0077_67a0, 0x0079_0af0]
        .into_iter()
        .enumerate()
    {
        let mut v = vec![0x8b, 0x44, 0x24, 0x04, 0x35];
        v.extend_from_slice(&(0x1700_0100u32 + 0x100 * k as u32).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0x80 + 0x10 * k as u32, &v);
        redirect(a, b, sym, s);
    }
    set(a, b, Global::<u32>::new(0x0112_a034), table);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let cases: [(char, u32, u32, u32); 172] = [
        ('C', 0x0077_2790, 0x0079_0af0, 0x0),
        ('A', 0x0077_2530, 0x30d5, 0x0),
        ('A', 0x0077_2590, 0x30d6, 0x0),
        ('A', 0x0077_25d0, 0x30d7, 0x0),
        ('A', 0x0077_2660, 0x30d8, 0x0),
        ('A', 0x0077_26b0, 0x30d9, 0x0),
        ('A', 0x0077_2710, 0x30da, 0x0),
        ('A', 0x0077_2770, 0x30db, 0x0),
        ('A', 0x0077_27d0, 0x30dc, 0x0),
        ('A', 0x0077_2810, 0x30dd, 0x0),
        ('A', 0x0077_2890, 0x30de, 0x0),
        ('A', 0x0077_28e0, 0x30df, 0x0),
        ('A', 0x0077_2950, 0x30e0, 0x0),
        ('A', 0x0077_29b0, 0x30e1, 0x0),
        ('A', 0x0077_29e0, 0x3000, 0x0),
        ('A', 0x0077_2a30, 0x30e2, 0x0),
        ('A', 0x0077_2a50, 0x3001, 0x0),
        ('A', 0x0077_2a70, 0x30e3, 0x0),
        ('A', 0x0077_2ac0, 0x3002, 0x0),
        ('A', 0x0077_2ae0, 0x30e4, 0x0),
        ('A', 0x0077_2b30, 0x3003, 0x0),
        ('A', 0x0077_2b50, 0x30e5, 0x0),
        ('A', 0x0077_2ba0, 0x3004, 0x0),
        ('A', 0x0077_2bc0, 0x30e6, 0x0),
        ('A', 0x0077_2c10, 0x3005, 0x0),
        ('A', 0x0077_2c30, 0x30e7, 0x0),
        ('A', 0x0077_2c50, 0x3006, 0x0),
        ('A', 0x0077_2ca0, 0x30e8, 0x0),
        ('A', 0x0077_2cc0, 0x3007, 0x0),
        ('A', 0x0077_2ce0, 0x30e9, 0x0),
        ('A', 0x0077_2d30, 0x3008, 0x0),
        ('A', 0x0077_2d50, 0x30ea, 0x0),
        ('A', 0x0077_2da0, 0x3009, 0x0),
        ('A', 0x0077_2e20, 0x300a, 0x0),
        ('A', 0x0077_2ea0, 0x300b, 0x0),
        ('A', 0x0077_2ec0, 0x30ff, 0x0),
        ('A', 0x0077_2ee0, 0x300c, 0x0),
        ('A', 0x0077_2f30, 0x3100, 0x0),
        ('A', 0x0077_2f50, 0x300d, 0x0),
        ('A', 0x0077_2fa0, 0x3101, 0x0),
        ('A', 0x0077_2fc0, 0x300e, 0x0),
        ('A', 0x0077_3010, 0x3102, 0x0),
        ('A', 0x0077_3030, 0x300f, 0x0),
        ('A', 0x0077_3080, 0x3103, 0x0),
        ('A', 0x0077_30a0, 0x3010, 0x0),
        ('A', 0x0077_30c0, 0x3104, 0x0),
        ('A', 0x0077_3110, 0x3011, 0x0),
        ('A', 0x0077_3130, 0x3105, 0x0),
        ('A', 0x0077_3150, 0x3012, 0x0),
        ('A', 0x0077_31a0, 0x3106, 0x0),
        ('A', 0x0077_31c0, 0x3013, 0x0),
        ('A', 0x0077_3210, 0x3107, 0x0),
        ('A', 0x0077_3230, 0x3014, 0x0),
        ('A', 0x0077_3280, 0x3108, 0x0),
        ('A', 0x0077_32a0, 0x3015, 0x0),
        ('A', 0x0077_32f0, 0x3109, 0x0),
        ('A', 0x0077_3310, 0x3016, 0x0),
        ('A', 0x0077_3330, 0x311f, 0x0),
        ('A', 0x0077_3380, 0x3017, 0x0),
        ('A', 0x0077_33a0, 0x3120, 0x0),
        ('A', 0x0077_33c0, 0x3018, 0x0),
        ('A', 0x0077_3410, 0x3113, 0x0),
        ('A', 0x0077_3430, 0x3019, 0x0),
        ('A', 0x0077_3480, 0x3114, 0x0),
        ('A', 0x0077_34a0, 0x301a, 0x0),
        ('A', 0x0077_34f0, 0x3115, 0x0),
        ('A', 0x0077_3510, 0x301b, 0x0),
        ('A', 0x0077_3540, 0x3116, 0x0),
        ('A', 0x0077_3560, 0x301c, 0x0),
        ('A', 0x0077_3580, 0x3117, 0x0),
        ('A', 0x0077_35a0, 0x301d, 0x0),
        ('A', 0x0077_35c0, 0x3118, 0x0),
        ('A', 0x0077_35e0, 0x301e, 0x0),
        ('A', 0x0077_3600, 0x3119, 0x0),
        ('A', 0x0077_3620, 0x301f, 0x0),
        ('A', 0x0077_3640, 0x311a, 0x0),
        ('A', 0x0077_3660, 0x3020, 0x0),
        ('A', 0x0077_3680, 0x311b, 0x0),
        ('A', 0x0077_36a0, 0x3021, 0x0),
        ('A', 0x0077_36c0, 0x311c, 0x0),
        ('A', 0x0077_36e0, 0x3022, 0x0),
        ('A', 0x0077_3700, 0x311d, 0x0),
        ('A', 0x0077_3720, 0x3023, 0x0),
        ('A', 0x0077_3740, 0x311e, 0x0),
        ('A', 0x0077_3760, 0x3024, 0x0),
        ('A', 0x0077_37c0, 0x3025, 0x0),
        ('A', 0x0077_37e0, 0x3026, 0x0),
        ('A', 0x0077_3840, 0x3027, 0x0),
        ('A', 0x0077_38c0, 0x3028, 0x0),
        ('A', 0x0077_3900, 0x3029, 0x0),
        ('A', 0x0077_3950, 0x302b, 0x0),
        ('A', 0x0077_39a0, 0x302c, 0x0),
        ('A', 0x0077_3a10, 0x302d, 0x0),
        ('A', 0x0077_3a80, 0x302e, 0x0),
        ('A', 0x0077_3af0, 0x302f, 0x0),
        ('A', 0x0077_3b40, 0x3030, 0x0),
        ('A', 0x0077_3ba0, 0x3031, 0x0),
        ('A', 0x0077_3be0, 0x3032, 0x0),
        ('A', 0x0077_3c20, 0x3033, 0x0),
        ('A', 0x0077_3c90, 0x3034, 0x0),
        ('A', 0x0077_3cd0, 0x3035, 0x0),
        ('A', 0x0077_3d10, 0x3036, 0x0),
        ('A', 0x0077_3d50, 0x3037, 0x0),
        ('A', 0x0077_3d90, 0x3038, 0x0),
        ('A', 0x0077_3dd0, 0x3039, 0x0),
        ('A', 0x0077_3e10, 0x303a, 0x0),
        ('A', 0x0077_3e50, 0x303b, 0x0),
        ('A', 0x0077_3e90, 0x303c, 0x0),
        ('A', 0x0077_3ed0, 0x303d, 0x0),
        ('A', 0x0077_3f10, 0x303e, 0x0),
        ('A', 0x0077_3f50, 0x303f, 0x0),
        ('A', 0x0077_3f90, 0x3040, 0x0),
        ('A', 0x0077_3fd0, 0x3041, 0x0),
        ('A', 0x0077_4010, 0x3042, 0x0),
        ('A', 0x0077_4050, 0x3043, 0x0),
        ('A', 0x0077_4090, 0x3044, 0x0),
        ('A', 0x0077_40d0, 0x3045, 0x0),
        ('A', 0x0077_4110, 0x3046, 0x0),
        ('A', 0x0077_4150, 0x3047, 0x0),
        ('A', 0x0077_4190, 0x3048, 0x0),
        ('A', 0x0077_41d0, 0x3049, 0x0),
        ('A', 0x0077_4210, 0x304a, 0x0),
        ('A', 0x0077_4250, 0x304b, 0x0),
        ('A', 0x0077_4290, 0x304c, 0x0),
        ('A', 0x0077_42d0, 0x304d, 0x0),
        ('A', 0x0077_4310, 0x304e, 0x0),
        ('A', 0x0077_4350, 0x304f, 0x0),
        ('A', 0x0077_4370, 0x3050, 0x0),
        ('A', 0x0077_43c0, 0x3051, 0x0),
        ('A', 0x0077_4410, 0x3052, 0x0),
        ('A', 0x0077_4430, 0x3053, 0x0),
        ('A', 0x0077_4480, 0x3054, 0x0),
        ('A', 0x0077_44d0, 0x3055, 0x0),
        ('D', 0x0077_2500, 0x14e, 0x0077_4720),
        ('D', 0x0077_2560, 0x150, 0x0077_4720),
        ('D', 0x0077_25f0, 0x152, 0x0077_4720),
        ('D', 0x0077_2680, 0x154, 0x0077_4720),
        ('D', 0x0077_26e0, 0x156, 0x0077_4720),
        ('D', 0x0077_2740, 0x158, 0x0077_4720),
        ('D', 0x0077_27a0, 0x15a, 0x0077_4720),
        ('D', 0x0077_2830, 0x15c, 0x0077_4720),
        ('D', 0x0077_28b0, 0x15e, 0x0077_4720),
        ('D', 0x0077_2920, 0x160, 0x0077_4720),
        ('D', 0x0077_2980, 0x162, 0x0077_4720),
        ('D', 0x0077_2a00, 0x164, 0x0077_4720),
        ('D', 0x0077_2a90, 0x166, 0x0077_4720),
        ('D', 0x0077_2b00, 0x168, 0x0077_4720),
        ('D', 0x0077_2b70, 0x16a, 0x0077_4720),
        ('D', 0x0077_2be0, 0x16c, 0x0077_4720),
        ('D', 0x0077_2c70, 0x16e, 0x0077_4720),
        ('D', 0x0077_2d00, 0x170, 0x0077_4720),
        ('D', 0x0077_2d70, 0x172, 0x0077_4720),
        ('D', 0x0077_2df0, 0x174, 0x0077_4720),
        ('D', 0x0077_2e70, 0x176, 0x0077_4720),
        ('D', 0x0077_2f00, 0x178, 0x0077_4720),
        ('D', 0x0077_2f70, 0x17a, 0x0077_4720),
        ('D', 0x0077_2fe0, 0x17c, 0x0077_4720),
        ('D', 0x0077_3050, 0x17e, 0x0077_4720),
        ('D', 0x0077_30e0, 0x180, 0x0077_4720),
        ('D', 0x0077_3170, 0x182, 0x0077_4720),
        ('D', 0x0077_31e0, 0x184, 0x0077_4720),
        ('D', 0x0077_3250, 0x186, 0x0077_4720),
        ('D', 0x0077_32c0, 0x188, 0x0077_4720),
        ('D', 0x0077_3350, 0x18a, 0x0077_4720),
        ('D', 0x0077_33e0, 0x18c, 0x0077_4720),
        ('D', 0x0077_3450, 0x18e, 0x0077_4720),
        ('D', 0x0077_34c0, 0x190, 0x0077_4720),
        ('D', 0x0077_4390, 0x8c, 0x0077_67a0),
        ('D', 0x0077_43e0, 0x8e, 0x0077_67a0),
        ('D', 0x0077_4450, 0x90, 0x0077_67a0),
        ('D', 0x0077_44a0, 0x92, 0x0077_67a0),
        ('D', 0x0077_44f0, 0x98, 0x0077_67a0),
    ];
    for _ in 0..3 {
        let bytes: Vec<u8> = (0..0x1d8 * 8).map(|_| rng.next() as u8).collect();
        a.write(table, &bytes).unwrap();
        b.write(table, &bytes).unwrap();
        for &(kind, at, p, f) in &cases {
            let arg = rng.next() % 8;
            let want = eax(a, at, &[arg]);
            let got = match kind {
                'A' => t::kernel_save_command(b, arg, p, call),
                'C' => t::kernel_forward(b, p, arg, call),
                _ => t::kernel_slot_name(b, arg, p, f, call),
            };
            assert_eq!(want, got, "{at:#x}");
        }
        same_memory("kernel accessors", a, b);
    }
}

#[test]
fn round17_kernel_accessors_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round17_kernel_accessors(&mut a, &mut b, &mut Xorshift(0x4017_0011));
}

#[test]
fn round17_kernel_accessors_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round17_kernel_accessors(&mut a, &mut b, &mut Xorshift(0x4017_0012));
}

/// Round 17's event-flag, request and price kernel accessors.
#[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
fn round17_kernel_flags(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let table = alloc(a, b, 0x1d8 * 8);
    let flags = alloc(a, b, 0x100);
    let cells = alloc(a, b, 8);
    let ret = |v: u32| {
        let mut c = vec![0xb8];
        c.extend_from_slice(&v.to_le_bytes());
        c.push(0xc3);
        c
    };
    let s = stand_in8(a, b, 0x40, &ret(flags));
    redirect(a, b, 0x0078_5250, s);
    let mut c = vec![0xa1];
    c.extend_from_slice(&cells.to_le_bytes());
    c.push(0xc3);
    let s = stand_in8(a, b, 0x50, &c);
    redirect(a, b, 0x0078_4e80, s);
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x35];
    c.extend_from_slice(&0x1700_0300u32.to_le_bytes());
    c.push(0xc3);
    let s = stand_in8(a, b, 0x60, &c);
    redirect(a, b, 0x0088_6e80, s);
    let s = stand_in8(
        a,
        b,
        0x70,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x03, 0xc3],
    );
    redirect(a, b, 0x0078_5380, s);
    set(a, b, Global::<u32>::new(0x0112_a034), table);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for _ in 0..200 {
        let bytes: Vec<u8> = (0..0x1d8 * 8).map(|_| rng.next() as u8).collect();
        a.write(table, &bytes).unwrap();
        b.write(table, &bytes).unwrap();
        let bytes: Vec<u8> = (0..0x100).map(|_| rng.next() as u8).collect();
        a.write(flags, &bytes).unwrap();
        b.write(flags, &bytes).unwrap();
        put(a, b, cells, rng.next() % 100_000);
        let i = rng.next() % 8;
        let i2 = if rng.next().is_multiple_of(4) {
            0xf9
        } else {
            i
        };
        if i2 == 0xf9 {
            assert_eq!(eax(a, 0x0077_2dc0, &[i2]), 0x1f40);
            assert_eq!(t::kernel_slot_word_194(b, i2), 0x1f40);
        } else {
            assert_eq!(eax(a, 0x0077_2dc0, &[i2]), t::kernel_slot_word_194(b, i2));
        }
        assert_eq!(eax(a, 0x0077_2e40, &[i]), t::kernel_gil_after(b, i, call));
        let sfx = rng.next();
        assert_eq!(
            eax(a, 0x0077_3b10, &[sfx]),
            t::kernel_play_sfx(b, sfx, call)
        );
        let k = rng.next() % 0x100;
        assert_eq!(eax(a, 0x0077_25b0, &[k]), t::kernel_event_byte(b, k, call));
        let v = rng.next();
        assert_eq!(
            eax(a, 0x0077_3800, &[k, v]),
            t::kernel_set_event_byte(b, k, v, call)
        );
        let chr = rng.next();
        assert_eq!(eax(a, 0x0077_2900, &[chr]), t::kernel_joined(b, chr, call));
        let n = rng.next() % 0x800;
        assert_eq!(eax(a, 0x0077_2620, &[n]), t::kernel_event_bit(b, n, call));
        let on = rng.next() % 3;
        assert_eq!(
            eax(a, 0x0077_3880, &[n, on]),
            t::kernel_set_event_bit(b, n, on, call)
        );
        for (at, kind) in [(0x0077_39c0u32, 1u32), (0x0077_3a30, 0), (0x0077_3aa0, 3)] {
            let pending = u8::from(rng.next().is_multiple_of(2));
            put(a, b, a.rebase(0x0112_a038), pending);
            let v = rng.next();
            let want = eax(a, at, &[v]);
            let got = t::kernel_post_request(b, v, kind);
            if pending == 0 {
                assert_eq!(want, got);
            }
        }
        same_memory("kernel flags", a, b);
    }
}

#[test]
fn round17_kernel_flags_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round17_kernel_flags(&mut a, &mut b, &mut Xorshift(0x4017_0021));
}

#[test]
fn round17_kernel_flags_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round17_kernel_flags(&mut a, &mut b, &mut Xorshift(0x4017_0022));
}

/// Round 18's magic-file, language, message and record helpers over
/// stand-ins (their answers change only through data cells).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round18_helpers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::{menu::bridge as mb, window::bridge as wb};
    use tolib::bridge as t;
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x40);
    let mut off = 0u32;
    let mut stub = |a: &mut Oracle, b: &mut Oracle, code: &[u8]| {
        let s = stand_in8(a, b, off, code);
        off += (code.len() as u32 + 15) & !15;
        s
    };
    let getter = |cell: u32| {
        let mut v = vec![0xa1];
        v.extend_from_slice(&cell.to_le_bytes());
        v.push(0xc3);
        v
    };
    for (k, sym) in [
        0x0064_3670u32,
        0x0064_5280,
        0x0064_2f10,
        0x0064_35e0,
        0x008b_8ca0,
    ]
    .into_iter()
    .enumerate()
    {
        let s = stub(a, b, &logger(cursor, 0x1800_0000 + ((k as u32) << 8)));
        redirect(a, b, sym, s);
    }
    let printf = stub(a, b, &logger(cursor, 0x1800_1000));
    let at = a.rebase(0x00b0_c380);
    a.write(at, &printf.to_le_bytes()).unwrap();
    b.write(at, &printf.to_le_bytes()).unwrap();
    for (k, sym) in [
        0x0078_0c90u32,
        0x0064_27e0,
        0x0088_7c30,
        0x0088_7c20,
        0x0064_10e0,
    ]
    .into_iter()
    .enumerate()
    {
        let s = stub(a, b, &getter(cells + 4 * k as u32));
        redirect(a, b, sym, s);
    }
    // no-argument stand-ins counting their calls
    for (k, sym) in [0x009d_a2d0u32, 0x008b_5680].into_iter().enumerate() {
        let mut v = vec![0x83, 0x05];
        v.extend_from_slice(&(cells + 0x20 + 4 * k as u32).to_le_bytes());
        v.extend_from_slice(&[0x01, 0xc3]);
        let s = stub(a, b, &v);
        redirect(a, b, sym, s);
    }
    // TOSelectMacroDic: *0x01869eb8 = [cells + 0x28]
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 0x28).to_le_bytes());
    v.push(0xa3);
    v.extend_from_slice(&a.rebase(0x0186_9eb8).to_le_bytes());
    v.push(0xc3);
    let s = stub(a, b, &v);
    redirect(a, b, 0x008b_56d0, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let printf_b = |m: &mut Oracle, fmt: u32, arg: Option<u32>| {
        let mut args = vec![fmt];
        args.extend(arg);
        m.call(printf, Conv::Cdecl, &args).unwrap();
    };
    let ctx = alloc(a, b, 8);
    set(a, b, Global::<u32>::new(0x0132_6b28), ctx);
    let msgs = alloc(a, b, 8 * 0x40);
    let lines = alloc(a, b, 8 * 0x100);
    for k in 0..8u32 {
        put(a, b, wb::MSG_WINDOWS.ptr(a).addr + 4 * k, msgs + 0x40 * k);
        put(a, b, a.rebase(0x0186_8a90) + 4 * k, lines + 0x100 * k);
    }
    let win = alloc(a, b, 0x60);
    let rtable = alloc(a, b, 0x40 * 4);
    set(a, b, Global::<u32>::new(0x0112_a0a4), rtable);
    let rec = alloc(a, b, 0x30);
    let macros = alloc(a, b, 4 * 0x32 * 4);
    let ids = [
        0x114u32, 0xa8, 0x158, 0x1b5, 0x1b6, 0x26e, 0x274, 0x275, 0x15c, 0x15e, 0x277, 0x27b, 7,
    ];
    for k in 0..300u32 {
        put(a, b, cursor, log);
        let id = ids[(rng.next() % 13) as usize];
        let handle = if rng.next().is_multiple_of(5) {
            u32::MAX
        } else {
            rng.next()
        };
        let idv = if rng.next().is_multiple_of(7) {
            u32::MAX
        } else {
            id
        };
        set(a, b, mb::MAGIC_FILE, [idv, rng.next(), rng.next(), handle]);
        put(
            a,
            b,
            cells,
            [
                rng.next() & 1,
                if rng.next() & 1 == 0 {
                    0x1d58
                } else {
                    rng.next()
                },
                rng.next() % 20,
                rng.next() % 4,
                rng.next() % 14,
            ],
        );
        assert_eq!(
            eax(a, 0x009d_a880, &[]),
            mb::magic_file_stop(b, call, printf_b)
        );
        set(a, b, mb::MAGIC_FILE, [id, 0, 0, 0]);
        assert_eq!(eax(a, 0x009d_afe0, &[]), mb::magic_file_async_load(b, call));
        if k % 50 == 0 {
            assert_eq!(eax(a, 0x009d_adc0, &[]), mb::magic_file_reset_preloaded(b));
        }
        put(a, b, ctx, rng.next());
        assert_eq!(eax(a, 0x0087_2a00, &[]), t::previous_level_check(b, call));
        // message choices
        let bytes: Vec<u8> = (0..8 * 0x100).map(|_| rng.next() as u8).collect();
        a.write(lines, &bytes).unwrap();
        b.write(lines, &bytes).unwrap();
        let (i, n) = (rng.next() % 8, rng.next() % 12 + 1);
        let (x, y, total) = (rng.next() % 600, rng.next() % 400, rng.next() % 500);
        eax(a, 0x008b_8880, &[i, n, x, y, total]);
        t::set_message_choice(b, i, n, x, y, total);
        // window rectangle snap
        for j in 0..4u32 {
            let v = match rng.next() % 4 {
                0 => (rng.next() % 2000) as f32 / 3.0 - 300.0,
                1 => f32::from_bits(rng.next() & 0xbfff_ffff),
                2 => 70_000.5,
                _ => -1.5,
            };
            put(a, b, win + 0x28 + 4 * j, v);
        }
        eax(a, 0x0090_6e90, &[win]);
        t::snap_window_rect(b, win);
        // language
        eax(a, 0x008a_d900, &[]);
        t::init_language(b, call);
        // timed macro messages
        let bytes: Vec<u8> = (0..4 * 0x32 * 4).map(|_| (rng.next() % 6) as u8).collect();
        a.write(macros, &bytes).unwrap();
        b.write(macros, &bytes).unwrap();
        put(
            a,
            b,
            cells + 0x28,
            if rng.next().is_multiple_of(3) {
                0
            } else {
                macros
            },
        );
        put(a, b, a.rebase(0x0133_0844), 0x189 + rng.next() % 4);
        let base = a.rebase(0x0186_63d0);
        put(
            a,
            b,
            base,
            [u8::from(rng.next() & 1 == 0), (rng.next() % 3) as u8, 0, 0],
        );
        put(
            a,
            b,
            base + 4,
            [rng.next() % 6, rng.next() % 4, rng.next() % 0x34],
        );
        put(a, b, a.rebase(0x0186_9eb8), macros);
        let arg = rng.next();
        eax(a, 0x008b_9220, &[arg]);
        t::timed_macro_message(b, arg, call);
        // the language-special record
        put(
            a,
            b,
            a.rebase(0x00c4_16c0),
            if rng.next() & 1 == 0 { 8u32 } else { 7 },
        );
        let idx = (rng.next() % 16) as u16;
        put(a, b, rec + 0x26, idx);
        put(
            a,
            b,
            rtable + 4 * u32::from(idx),
            if rng.next() & 1 == 0 {
                0xffff_fe70u32
            } else {
                0
            },
        );
        let want = [0x25au16, 0x266, 0x276][(rng.next() % 3) as usize];
        put(
            a,
            b,
            rec + 4,
            [
                want,
                if rng.next().trailing_zeros() >= 2 {
                    0
                } else {
                    0xbb
                },
            ],
        );
        let r = if rng.next().is_multiple_of(9) { 0 } else { rec };
        assert_eq!(
            eax(a, 0x0077_4680, &[r]),
            t::is_lang_special_record(b, r, |m| eax(m, 0x0064_10e0, &[]))
        );
        same_memory("round 18 helpers", a, b);
    }
}

#[test]
fn round18_helpers_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round18_helpers(&mut a, &mut b, &mut Xorshift(0x4018_0001));
}

#[test]
fn round18_helpers_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round18_helpers(&mut a, &mut b, &mut Xorshift(0x4018_0002));
}

/// Round 18's resets and gil input over a stand-in gil.
#[allow(clippy::cast_possible_truncation)]
fn round18_resets(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let gil_cell = alloc(a, b, 4);
    let mut v = vec![0xa1];
    v.extend_from_slice(&gil_cell.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x0078_4e80, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..200u32 {
        if k % 40 == 0 {
            for (at, len) in [(0x012f_bbbcu32, 0x40u32), (0x025c_c5a0, 0x4000)] {
                let bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
                let at = a.rebase(at);
                a.write(at, &bytes).unwrap();
                b.write(at, &bytes).unwrap();
            }
            put(a, b, a.rebase(0x0230_56e8), rng.next());
            put(a, b, a.rebase(0x0184_1d90), rng.next());
            eax(a, 0x0082_1aa0, &[]);
            t::init_font_1208(b);
            eax(a, 0x008a_b550, &[]);
            t::event_font_texture_array_reset(b);
        }
        let gil = [0u32, 9, 10, 999_999_999, rng.next() % 1_000_000, rng.next()]
            [(rng.next() % 6) as usize];
        put(a, b, gil_cell, gil);
        let max = [u32::MAX, 0, 100, rng.next() % 2_000_000, rng.next()][(rng.next() % 5) as usize];
        assert_eq!(eax(a, 0x008a_2030, &[max]), t::init_gil_input(b, max, call));
        same_memory("round 18 resets", a, b);
    }
}

#[test]
fn round18_resets_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round18_resets(&mut a, &mut b, &mut Xorshift(0x4018_0011));
}

#[test]
fn round18_resets_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round18_resets(&mut a, &mut b, &mut Xorshift(0x4018_0012));
}

/// Round 19's menu record helpers over logging stand-ins.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round19_records(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let s = stand_in8(a, b, 0x40, &logger(cursor, 0x1900_0100));
    redirect(a, b, 0x0077_6910, s);
    let mut v = vec![0x83, 0x05];
    v.extend_from_slice(&counter.to_le_bytes());
    v.extend_from_slice(&[0x01, 0xc3]);
    let s = stand_in8(a, b, 0x80, &v);
    redirect(a, b, 0x0077_4740, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let pages = alloc(a, b, 16 * 8);
    let records = alloc(a, b, 16 * 13 * 0x138);
    let pairs = alloc(a, b, 0x40 * 8);
    let values = alloc(a, b, 0x40 * 4);
    let sizes = alloc(a, b, 4);
    let g = |a: &Oracle, x: u32| a.rebase(x);
    for k in 0..150u32 {
        put(a, b, cursor, log);
        let bytes: Vec<u8> = (0..16 * 13 * 0x138).map(|_| rng.next() as u8).collect();
        a.write(records, &bytes).unwrap();
        b.write(records, &bytes).unwrap();
        let base = records - 0x100;
        let npages = rng.next() % 16 + u32::from(k % 9 != 0);
        for p in 0..16u32 {
            let n = (rng.next() % 13) as u16;
            put(a, b, pages + 8 * p, 0x100 + p * 13 * 0x138);
            put(a, b, pages + 8 * p + 4, if p == 0 { 12 } else { n });
        }
        // flagged records rarely, and small value-pair counts
        for r in 0..16 * 13 {
            let rec = records + 0x138 * r;
            put(a, b, rec + 1, u8::from(rng.next().is_multiple_of(16)));
            put(
                a,
                b,
                rec + 0x1c,
                [(rng.next() % 4) as u16, (rng.next() % 0x30) as u16],
            );
        }
        for i in 0..0x40u32 {
            put(a, b, pairs + 8 * i, [rng.next() % 0x40, rng.next()]);
        }
        set(
            a,
            b,
            Global::<u32>::new(0x0112_a01c),
            if k % 13 == 0 { 0 } else { pages },
        );
        set(a, b, Global::<u32>::new(0x0112_a03c), base);
        set(a, b, Global::<u32>::new(0x0112_a080), npages);
        set(a, b, Global::<u32>::new(0x0112_a028), pairs);
        set(a, b, Global::<u32>::new(0x0112_a0a4), values);
        // the target of the positional search: some record's start
        let tp = rng.next() % 16;
        let tr = rng.next() % 13;
        let target = 0x100 + tp * 13 * 0x138 + tr * 0x138;
        put(
            a,
            b,
            g(a, 0x0112_a0a0),
            if rng.next().is_multiple_of(4) {
                rng.next()
            } else {
                target
            },
        );
        eax(a, 0x0077_5ce0, &[]);
        t::find_record_at(b, call);
        same_memory("record at", a, b);
        eax(a, 0x0077_5de0, &[]);
        t::find_record_flagged(b, call);
        same_memory("flagged record", a, b);
        let screen = [5u32, 0xb, 8][(rng.next() % 3) as usize];
        put(a, b, g(a, 0x00c4_16c0), screen);
        eax(a, 0x0077_67d0, &[]);
        t::patch_menu_records(b, call);
        same_memory("patched records", a, b);
    }
    // every image and kind on both screens (and one other)
    for screen in [5u32, 0xb, 8] {
        put(a, b, g(a, 0x00c4_16c0), screen);
        for image in 0..0x22u32 {
            for kind in 0..0x12u32 {
                put(a, b, sizes, 0xa5a5_a5a5u32);
                eax(a, 0x0077_5820, &[0, image, kind, sizes, sizes + 2]);
                t::menu_picture_size(b, image, kind, sizes, sizes + 2);
                same_memory("picture sizes", a, b);
            }
        }
    }
}

#[test]
fn round19_records_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round19_records(&mut a, &mut b, &mut Xorshift(0x4019_0001));
}

#[test]
fn round19_records_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round19_records(&mut a, &mut b, &mut Xorshift(0x4019_0002));
}

/// Round 19's language relayout of menu records.
#[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
fn round19_relayout(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let cell = alloc(a, b, 4);
    let mut v = vec![0xa1];
    v.extend_from_slice(&cell.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x0064_10e0, s);
    let recs = alloc(a, b, 24 * 0x138 + 8);
    let values = alloc(a, b, 16 * 4);
    set(a, b, Global::<u32>::new(0x0112_a0a4), values);
    let boxes: [[u16; 4]; 12] = [
        [0x236, 0xb3, 1, 2],
        [0x236, 0x8a, 1, 2],
        [0x236, 0x10, 1, 2],
        [0x20e, 0xb3, 0xff, 0x37],
        [0x23e, 0xc5, 3, 4],
        [0x20e, 0x71, 0xff, 9],
        [0x23e, 0x7e, 5, 6],
        [0x27a, 0xae, 5, 6],
        [0x27a, 0xc6, 5, 6],
        [0x28c, 0x107, 5, 6],
        [0x28c, 0x119, 5, 6],
        [0x111, 0x222, 5, 6],
    ];
    for _ in 0..200 {
        put(a, b, cell, rng.next() % 12);
        put(
            a,
            b,
            a.rebase(0x00c4_16c0),
            if rng.next().is_multiple_of(5) {
                5u32
            } else {
                8
            },
        );
        for k in 0..16u32 {
            put(
                a,
                b,
                values + 4 * k,
                if rng.next() & 1 == 0 {
                    0xffff_fe70u32
                } else {
                    rng.next()
                },
            );
        }
        for r in 0..24u32 {
            let p = recs + 4 + 0x138 * r;
            put(a, b, p, boxes[(rng.next() % 12) as usize]);
            put(a, b, p + 0x22, (rng.next() % 16) as u16);
        }
        let last = if rng.next().is_multiple_of(9) {
            u32::MAX
        } else {
            rng.next() % 24
        };
        eax(a, 0x0077_5a20, &[0, 0, recs, last]);
        t::relayout_menu_records(b, recs, last, |m| eax(m, 0x0064_10e0, &[]));
        same_memory("menu relayout", a, b);
    }
}

#[test]
fn round19_relayout_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round19_relayout(&mut a, &mut b, &mut Xorshift(0x4019_0011));
}

#[test]
fn round19_relayout_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round19_relayout(&mut a, &mut b, &mut Xorshift(0x4019_0012));
}

/// Round 19's menu cursor block check over a stand-in condition table.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round19_cursor(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    // conditions: arg & 1, arg & 2
    let c0 = stand_in8(
        a,
        b,
        0x40,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x01, 0xc3],
    );
    let c1 = stand_in8(
        a,
        b,
        0x50,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x02, 0xc3],
    );
    let table = alloc(a, b, 8);
    put(a, b, table, [c0, c1]);
    set(a, b, Global::<u32>::new(0x00c4_16d0), table);
    let pages = alloc(a, b, 8 * 8);
    let records = alloc(a, b, 8 * 8 * 0x138);
    let base = records - 0x40;
    let g = |a: &Oracle, x: u32| a.rebase(x);
    for k in 0..400u32 {
        let bytes: Vec<u8> = (0..8 * 8 * 0x138).map(|_| rng.next() as u8).collect();
        a.write(records, &bytes).unwrap();
        b.write(records, &bytes).unwrap();
        let npages = rng.next() % 8 + 1;
        let mut counts = [0u16; 8];
        for p in 0..8u32 {
            let n = (rng.next() % 8) as u16;
            counts[p as usize] = n;
            put(a, b, pages + 8 * p, 0x40 + p * 8 * 0x138);
            put(a, b, pages + 8 * p + 4, n);
        }
        set(
            a,
            b,
            Global::<u32>::new(0x0112_a01c),
            if k % 17 == 0 { 0 } else { pages },
        );
        set(a, b, Global::<u32>::new(0x0112_a03c), base);
        set(a, b, Global::<u32>::new(0x0112_a080), npages);
        let cur = records + 0x138 * (rng.next() % 64);
        set(
            a,
            b,
            Global::<u32>::new(0x0112_a084),
            if k % 19 == 0 { 0 } else { cur },
        );
        put(a, b, cur + 2, (rng.next() % 4) as u8);
        for d in 0..8u32 {
            let link = cur + 0x30 + 0xc * d;
            // the condition index, its argument and the expected flag
            let cond = if rng.next().is_multiple_of(5) {
                0xffff
            } else {
                (rng.next() % 2) as u16
            };
            put(a, b, link, [cond, rng.next() as u16]);
            put(a, b, link + 4, (rng.next() % 2) as u8);
            // the target: a first-page record, a later page's record, or
            // junk
            let p = rng.next() % 8;
            let r = rng.next() % 8;
            let target = match rng.next() % 3 {
                0 => 0x40 + r * 0x138,
                1 => 0x40 + p * 8 * 0x138 + r * 0x138,
                _ => rng.next(),
            };
            put(a, b, link + 8, target);
        }
        let bits = [
            0x1000u16, 0x4000, 0x2000, 0x8000, 0x80, 0x10, 0x40, 0x20, 0, 0x30,
        ];
        for at in [0x0112_a08cu32, 0x0112_a090, 0x0112_a094] {
            put(
                a,
                b,
                g(a, at),
                bits[(rng.next() % 10) as usize] | bits[(rng.next() % 10) as usize],
            );
        }
        eax(a, 0x0077_5530, &[]);
        t::menu_cursor_blocked(b, |m, f, arg| m.call(f, Conv::Cdecl, &[arg]).unwrap().eax);
        same_memory("menu cursor", a, b);
    }
}

/// A cdecl stand-in logging its first two arguments at `*cursor`.
fn logger2(cursor: u32) -> Vec<u8> {
    let mut v = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x54, 0x24, 0x08, 0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x01, 0x89, 0x51, 0x04, 0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x08, 0xc3]);
    v
}

/// A stand-in logging `value` at `*cursor` and returning it.
fn marker(cursor: u32, value: u32) -> Vec<u8> {
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0xc7, 0x01]);
    v.extend_from_slice(&value.to_le_bytes());
    v.extend_from_slice(&[0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.push(0x04);
    v.push(0xb8);
    v.extend_from_slice(&value.to_le_bytes());
    v.push(0xc3);
    v
}

/// A cdecl stand-in logging its first `n` arguments at `*cursor` and
/// returning the first.
fn logger_args(cursor: u32, n: u8) -> Vec<u8> {
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    for k in 0..n {
        // mov eax, [esp+4+4k]; mov [ecx+4k], eax
        v.extend_from_slice(&[0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 * k]);
    }
    v.extend_from_slice(&[0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[4 * n, 0x8b, 0x44, 0x24, 0x04, 0xc3]);
    v
}

/// A stand-in for `graphicDrawUIElement`/`graphicFontDraw(rec, …)` that
/// logs the record's first 16 words, its `+0x88` word and the four
/// arguments after the record.
fn record_logger(cursor: u32) -> Vec<u8> {
    let mut v = vec![0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c, 0x8b, 0x3d];
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[
        0xb9, 0x10, 0x00, 0x00, 0x00, 0xf3, 0xa5, 0x8b, 0x46, 0x48, 0xab, 0x8b, 0x44, 0x24, 0x10,
        0xab, 0x8b, 0x44, 0x24, 0x14, 0xab, 0x8b, 0x44, 0x24, 0x18, 0xab, 0x8b, 0x44, 0x24, 0x1c,
        0xab, 0x89, 0x3d,
    ]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x5f, 0x5e, 0xc3]);
    v
}

/// A stand-in for `graphicDrawUIElement(rec, …)` that logs the record
/// words at `offsets` and the four arguments after the record.
fn record_words_logger(cursor: u32, offsets: &[u8]) -> Vec<u8> {
    // push esi; mov esi, [esp+8]; mov ecx, [cursor]
    let mut v = vec![0x56, 0x8b, 0x74, 0x24, 0x08, 0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    let mut at = 0u8;
    for &o in offsets {
        // mov eax, [esi+o]; mov [ecx+at], eax
        v.extend_from_slice(&[0x8b, 0x86]);
        v.extend_from_slice(&u32::from(o).to_le_bytes());
        v.extend_from_slice(&[0x89, 0x41, at]);
        at += 4;
    }
    for k in 0..4u8 {
        // mov eax, [esp+0xc+4k]; mov [ecx+at], eax
        v.extend_from_slice(&[0x8b, 0x44, 0x24, 0x0c + 4 * k, 0x89, 0x41, at]);
        at += 4;
    }
    v.extend_from_slice(&[0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[at, 0x5e, 0xc3]);
    v
}

/// `0x00776110` over logging stand-ins for the sound, print, help and
/// reset calls and two condition functions.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round19_step(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x800);
    let cursor = alloc(a, b, 4);
    let two = logger2(cursor);
    let marker = |value: u32| marker(cursor, value);
    let s = stand_in8(a, b, 0x40, &logger(cursor, 0x1900_1000));
    redirect(a, b, 0x0088_6e80, s);
    let s = stand_in8(a, b, 0x80, &logger(cursor, 0x1900_2000));
    redirect(a, b, 0x0062_f500, s);
    let s = stand_in8(a, b, 0xc0, &two);
    redirect(a, b, 0x008b_ef10, s);
    let s = stand_in8(a, b, 0x100, &marker(0x1900_3000));
    redirect(a, b, 0x008b_f0b0, s);
    let s = stand_in8(a, b, 0x140, &marker(0x1900_4000));
    redirect(a, b, 0x0077_4740, s);
    let c0 = stand_in8(
        a,
        b,
        0x180,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x01, 0xc3],
    );
    let c1 = stand_in8(
        a,
        b,
        0x1c0,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x02, 0xc3],
    );
    let table = alloc(a, b, 8);
    put(a, b, table, [c0, c1]);
    set(a, b, Global::<u32>::new(0x00c4_16d0), table);
    let pages = alloc(a, b, 8 * 8);
    let records = alloc(a, b, 8 * 8 * 0x138);
    let pairs = alloc(a, b, 16 * 8);
    let values = alloc(a, b, 0x40 * 4);
    let out = alloc(a, b, 4);
    let base = records - 0x40;
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..500u32 {
        put(a, b, cursor, log);
        put(a, b, out, rng.next());
        let bytes: Vec<u8> = (0..8 * 8 * 0x138).map(|_| rng.next() as u8).collect();
        a.write(records, &bytes).unwrap();
        b.write(records, &bytes).unwrap();
        for r in 0..64u32 {
            let rec = records + 0x138 * r;
            put(
                a,
                b,
                rec + 0x1c,
                [(rng.next() % 4) as u16, (rng.next() % 8) as u16],
            );
            for off in [0xf0u32, 0xf2, 0xf4, 0xf6, 0xf8] {
                let v = if rng.next().is_multiple_of(2) {
                    0
                } else {
                    rng.next() as u16
                };
                put(a, b, rec + off, v);
            }
            put(a, b, rec + 0xfb, (rng.next() % 4) as u8);
            put(a, b, rec + 0xfc, (rng.next() % 2) as u8);
            let vi = if rng.next().is_multiple_of(4) {
                0xffff
            } else {
                (rng.next() % 0x40) as u16
            };
            let c = if rng.next().is_multiple_of(4) {
                0xffff
            } else {
                (rng.next() % 2) as u16
            };
            put(a, b, rec + 0xfe, [vi, c, rng.next() as u16]);
        }
        for i in 0..16u32 {
            put(a, b, pairs + 8 * i, [rng.next() % 0x40, rng.next()]);
        }
        set(a, b, Global::<u32>::new(0x0112_a028), pairs);
        set(a, b, Global::<u32>::new(0x0112_a0a4), values);
        let npages = rng.next() % 8 + u32::from(k % 11 != 0);
        for p in 0..8u32 {
            put(a, b, pages + 8 * p, 0x40 + p * 8 * 0x138);
            put(a, b, pages + 8 * p + 4, (rng.next() % 9) as u16);
        }
        set(
            a,
            b,
            Global::<u32>::new(0x0112_a01c),
            if k % 17 == 0 { 0 } else { pages },
        );
        set(a, b, Global::<u32>::new(0x0112_a03c), base);
        set(a, b, Global::<u32>::new(0x0112_a080), npages);
        let state = if rng.next().is_multiple_of(5) {
            (rng.next() % 3) as u8
        } else {
            0
        };
        put(a, b, g(a, 0x0112_a038), state);
        set(a, b, Global::<u32>::new(0x0112_a058), rng.next() % 3);
        set(a, b, Global::<u32>::new(0x0112_a054), rng.next());
        set(a, b, Global::<u32>::new(0x0112_a05c), rng.next());
        set(a, b, Global::<u32>::new(0x0112_a060), rng.next());
        set(
            a,
            b,
            Global::<u32>::new(0x0112_a088),
            base + 0x40 + (rng.next() % 3) * 8 * 0x138,
        );
        set(
            a,
            b,
            Global::<u32>::new(0x00c4_16c0),
            [5u32, 0xb, 8][(rng.next() % 3) as usize],
        );
        let cur = records + 0x138 * (rng.next() % 64);
        set(
            a,
            b,
            Global::<u32>::new(0x0112_a084),
            if k % 19 == 0 { 0 } else { cur },
        );
        put(a, b, cur + 2, (rng.next() % 4) as u8);
        for d in 0..8u32 {
            let link = cur + 0x30 + 0xc * d;
            let cond = if rng.next().is_multiple_of(5) {
                0xffff
            } else {
                (rng.next() % 2) as u16
            };
            put(a, b, link, [cond, rng.next() as u16]);
            put(a, b, link + 4, (rng.next() % 2) as u8);
            let p = rng.next() % 8;
            let r = rng.next() % 8;
            let target = match rng.next() % 4 {
                0 => 0x40 + r * 0x138,
                1 => 0x40 + p * 8 * 0x138 + r * 0x138,
                2 => base.wrapping_neg(),
                _ => rng.next(),
            };
            put(a, b, link + 8, target);
        }
        let bits = [
            0x1000u16, 0x4000, 0x2000, 0x8000, 0x80, 0x10, 0x40, 0x20, 0, 0x60,
        ];
        for at in [0x0112_a08cu32, 0x0112_a090, 0x0112_a094] {
            put(
                a,
                b,
                g(a, at),
                bits[(rng.next() % 10) as usize] | bits[(rng.next() % 10) as usize],
            );
        }
        eax(a, 0x0077_6110, &[out]);
        t::menu_cursor_step(b, out, call, |m, f, arg| {
            m.call(f, Conv::Cdecl, &[arg]).unwrap().eax
        });
        same_memory("menu cursor step", a, b);
    }
}

#[test]
fn round19_step_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round19_step(&mut a, &mut b, &mut Xorshift(0x4019_0031));
}

#[test]
fn round19_step_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round19_step(&mut a, &mut b, &mut Xorshift(0x4019_0032));
}

/// The record condition, text width and battle-window helpers of round 19
/// over logging stand-ins.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round19_to(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x800);
    let cursor = alloc(a, b, 4);
    let playing = alloc(a, b, 4);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let cond = |m: &mut Oracle, f: u32, arg: u32| m.call(f, Conv::Cdecl, &[arg]).unwrap().eax;
    let c0 = stand_in8(
        a,
        b,
        0x40,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x01, 0xc3],
    );
    let c1 = stand_in8(
        a,
        b,
        0x50,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x02, 0xc3],
    );
    let table = alloc(a, b, 8);
    put(a, b, table, [c0, c1]);
    set(a, b, Global::<u32>::new(0x00c4_16d0), table);
    // TOSkipCode: one byte, two when bit 0 is set
    let s = stand_in8(
        a,
        b,
        0x60,
        &[
            0x8b, 0x44, 0x24, 0x04, 0x0f, 0xb6, 0x08, 0x83, 0xe1, 0x01, 0x8d, 0x44, 0x08, 0x01,
            0xc3,
        ],
    );
    redirect(a, b, 0x008d_fd80, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&playing.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x80, &v);
    redirect(a, b, 0x0088_6680, s);
    for (i, sym) in [
        0x0089_abd0u32,
        0x008a_1090,
        0x0088_6eb0,
        0x008a_f5c0,
        0x008a_da30,
        0x008b_81d0,
        0x0090_6e60,
        0x008b_8bb0,
    ]
    .into_iter()
    .enumerate()
    {
        let off = 0xa0 + 0x40 * i as u32;
        let code = if i == 0 || i >= 6 {
            logger2(cursor)
        } else {
            logger(cursor, 0x1900_5000 + i as u32)
        };
        let s = stand_in8(a, b, off, &code);
        redirect(a, b, sym, s);
    }
    for (i, sym) in [0x008a_dbd0u32, 0x008b_e800, 0x0089_77f0, 0x0089_dc40]
        .into_iter()
        .enumerate()
    {
        let off = 0x300 + 0x40 * i as u32;
        let s = stand_in8(a, b, off, &marker(cursor, 0x1900_6000 + i as u32));
        redirect(a, b, sym, s);
    }
    let record = alloc(a, b, 0x30);
    let text = alloc(a, b, 0x40);
    let g = |a: &Oracle, x: u32| a.rebase(x);
    for k in 0..300u32 {
        put(a, b, cursor, log);
        // the record condition
        let c = if rng.next().is_multiple_of(4) {
            0x8000 | rng.next() as u16
        } else {
            (rng.next() % 2) as u16
        };
        put(a, b, record + 0x20, [c, rng.next() as u16]);
        put(a, b, record + 0x2b, (rng.next() % 4) as u8);
        assert_eq!(
            eax(a, 0x0077_4a50, &[record]),
            t::menu_record_condition(b, record, cond),
            "record condition {k}"
        );
        // the text width
        let n = rng.next() % 0x30;
        let bytes: Vec<u8> = (0..0x40)
            .map(|i| if i < n { (rng.next() as u8) | 0x10 } else { 0 })
            .collect();
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        assert_eq!(
            eax(a, 0x0089_dfa0, &[text]),
            t::text_width_12(b, text, call),
            "text width {k}"
        );
        // the battle window
        let slot = rng.next() % 4;
        let (x, y) = (rng.next(), rng.next());
        put(a, b, g(a, 0x0133_d1fc), rng.next() as u16);
        put(a, b, g(a, 0x0133_d178), rng.next());
        put(
            a,
            b,
            g(a, 0x0133_c950 + slot * 0xf0 + 0xa6),
            rng.next() as u16,
        );
        assert_eq!(
            eax(a, 0x0089_a860, &[slot, x, y]),
            t::to_btl_set_target(b, slot, x, y, call),
            "set target {k}"
        );
        put(a, b, playing, rng.next() % 2);
        put(a, b, g(a, 0x0133_c948), rng.next());
        put(a, b, g(a, 0x0133_f77c), rng.next());
        assert_eq!(
            eax(a, 0x0089_ad90, &[]),
            t::to_clear_btl_window(b, call),
            "clear {k}"
        );
        let (w, x, y) = (rng.next(), rng.next(), rng.next());
        assert_eq!(
            eax(a, 0x008b_0000, &[w, x, y]),
            t::to_set_window_kind_and_type(b, w, x, y, call),
            "window kind {k}"
        );
        if k % 10 == 0 {
            put(a, b, g(a, 0x0184_1e0c), rng.next());
            put(a, b, g(a, 0x0186_59bc), rng.next());
            assert_eq!(eax(a, 0x008a_de50, &[]), t::to_reset1(b, call), "reset {k}");
        }
        same_memory("round 19 TO helpers", a, b);
    }
}

#[test]
fn round19_to_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round19_to(&mut a, &mut b, &mut Xorshift(0x4019_0041));
}

#[test]
fn round19_to_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round19_to(&mut a, &mut b, &mut Xorshift(0x4019_0042));
}

/// Round 19's menu text code expansion and glyph text width.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round19_text(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let strings = alloc(a, b, 0x100);
    // the string function: strings + (arg & 0xf) * 0x10
    let mut v = vec![
        0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x0f, 0xc1, 0xe0, 0x04, 0x05,
    ];
    v.extend_from_slice(&strings.to_le_bytes());
    v.push(0xc3);
    let c0 = stand_in8(a, b, 0x40, &v);
    // the number function: arg * 0x1357, or arg alone for small values
    let c1 = stand_in8(
        a,
        b,
        0x60,
        &[
            0x8b, 0x44, 0x24, 0x04, 0x69, 0xc0, 0x57, 0x13, 0x00, 0x00, 0xc3,
        ],
    );
    let c2 = stand_in8(
        a,
        b,
        0x80,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x3f, 0xc3],
    );
    let table = alloc(a, b, 12);
    put(a, b, table, [c0, c1, c2]);
    set(a, b, Global::<u32>::new(0x00c4_16d0), table);
    let s = stand_in8(
        a,
        b,
        0xa0,
        &[
            0x8b, 0x44, 0x24, 0x04, 0x0f, 0xb6, 0x08, 0x83, 0xe1, 0x01, 0x8d, 0x44, 0x08, 0x01,
            0xc3,
        ],
    );
    redirect(a, b, 0x008d_fd80, s);
    let values = alloc(a, b, 0x40 * 4);
    set(a, b, Global::<u32>::new(0x0112_a0a4), values);
    let codes = alloc(a, b, 8 * 8);
    let out = alloc(a, b, 0x40);
    let cells = alloc(a, b, 8);
    let text = alloc(a, b, 0x40);
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let cond = |m: &mut Oracle, f: u32, arg: u32| m.call(f, Conv::Cdecl, &[arg]).unwrap().eax;
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..400u32 {
        let bytes: Vec<u8> = (0..0x100)
            .map(|_| {
                if rng.next().is_multiple_of(6) {
                    0
                } else {
                    rng.next() as u8
                }
            })
            .collect();
        a.write(strings, &bytes).unwrap();
        b.write(strings, &bytes).unwrap();
        for i in 0..0x40u32 {
            let v = match rng.next() % 4 {
                0 => rng.next() % 10,
                1 => rng.next() % 1_000_000,
                2 => (rng.next() % 1000) * 1000,
                _ => rng.next(),
            };
            put(a, b, values + 4 * i, v);
        }
        let at = codes + 8 * (rng.next() % 8);
        // the string function only for kind 1
        let kind = rng.next() % 3;
        let func = if kind == 1 { 0 } else { 1 + rng.next() % 2 };
        let w0 = func << 16 | kind << 8 | (rng.next() % 3);
        let flag = if rng.next().is_multiple_of(3) {
            rng.next() % 0x100
        } else {
            0
        };
        let w1 = flag << 16 | (rng.next() % 0x40) | (rng.next() & 0xff00_0000);
        put(a, b, at, [w0, w1]);
        put(a, b, cells, [out + rng.next() % 8, at]);
        put(a, b, g(a, 0x0112_a06c), rng.next());
        put(a, b, g(a, 0x0112_a070), rng.next());
        let arg = rng.next() % 0x20;
        eax(a, 0x0077_4860, &[cells, cells + 4, arg]);
        t::menu_text_code(b, cells, cells + 4, arg, cond);
        same_memory("menu text code", a, b);
        // the glyph width over a random glyph table
        if k % 50 == 0 {
            let glyphs: Vec<u8> = (0..0x680)
                .map(|i| {
                    if i % 4 == 0 {
                        u8::from(rng.next().is_multiple_of(3))
                    } else {
                        rng.next() as u8
                    }
                })
                .collect();
            let at = g(a, 0x00c5_8f02);
            a.write(at, &glyphs).unwrap();
            b.write(at, &glyphs).unwrap();
        }
        let n = rng.next() % 0x30;
        let bytes: Vec<u8> = (0..0x40)
            .map(|i| if i < n { (rng.next() as u8).max(1) } else { 0 })
            .collect();
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        assert_eq!(
            eax(a, 0x0089_e410, &[text]),
            t::text_width_glyphs(b, text, call),
            "glyph width {k}"
        );
        same_memory("glyph width", a, b);
    }
}

#[test]
fn round19_text_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round19_text(&mut a, &mut b, &mut Xorshift(0x4019_0051));
}

#[test]
fn round19_text_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round19_text(&mut a, &mut b, &mut Xorshift(0x4019_0052));
}

/// Round 20's button icon width, with the original `graphicUiRemapX2` on
/// both sides.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round20_icon(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let assign = alloc(a, b, 4);
    let mut v = vec![0xa1];
    v.extend_from_slice(&assign.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x008a_cf30, s);
    let chars = [
        0x20u32, 0x30, 0x35, 0x39, 0x3a, 0x3b, 0x3c, 0x2f, 0x40, 0x41, 0x43, 0x4f, 0x50, 0,
    ];
    for k in 0..2000u32 {
        let id = match rng.next() % 4 {
            0 => 0x258 + rng.next() % 0x14,
            1 => 0x268 + rng.next() % 2,
            2 => 0x250 + rng.next() % 0x30,
            _ => rng.next(),
        };
        let scale = match rng.next() % 4 {
            0 => (rng.next() % 400) as f32 / 100.0,
            1 => f32::from_bits(rng.next() & 0xbfff_ffff),
            2 => -((rng.next() % 400) as f32) / 37.0,
            _ => 1.0,
        };
        let c = if rng.next().is_multiple_of(5) {
            rng.next() % 0x80
        } else {
            chars[(rng.next() % 14) as usize]
        };
        put(a, b, g(a, 0x025d_16e4), c);
        let want = a
            .call(g(a, 0x0090_62b0), Conv::Cdecl, &[id, scale.to_bits()])
            .unwrap()
            .st0
            .unwrap();
        let got = t::scale_button_icon_width(b, id, scale, |m, v| {
            let at = m.rebase(0x0064_47e0);
            m.call(at, Conv::Cdecl, &[v.to_bits()])
                .unwrap()
                .st0
                .unwrap() as f32
        });
        assert_eq!(
            (want as f32).to_bits(),
            got.to_bits(),
            "icon width {k}: id {id:#x} scale {scale} char {c:#x}"
        );
        same_memory("icon width", a, b);
        // the character-to-icon step over the original icon width, with
        // the confirm button assignment from a cell
        put(a, b, assign, rng.next() % 3);
        let c = if rng.next().is_multiple_of(4) {
            rng.next() % 0x100
        } else {
            0x1f + rng.next() % 0x48
        };
        let want = a
            .call(g(a, 0x008f_a800), Conv::Cdecl, &[c, scale.to_bits()])
            .unwrap()
            .st0
            .unwrap();
        let got = t::to_get_scale_button_icon_width(b, c, scale, eax, |m, id, s| {
            let at = m.rebase(0x0090_62b0);
            m.call(at, Conv::Cdecl, &[id, s.to_bits()])
                .unwrap()
                .st0
                .unwrap() as f32
        });
        assert_eq!(
            (want as f32).to_bits(),
            got.to_bits(),
            "icon {k}: char {c:#x}"
        );
        same_memory("icon", a, b);
    }
    // the ids themselves: the icon width replaced by `fild [esp+4]`
    let s = stand_in8(a, b, 0x80, &[0xdb, 0x44, 0x24, 0x04, 0xc3]);
    redirect(a, b, 0x0090_62b0, s);
    for c in 0..0x100u32 {
        for assign_value in 0..2u32 {
            put(a, b, assign, assign_value);
            let want = a
                .call(g(a, 0x008f_a800), Conv::Cdecl, &[c, 0])
                .unwrap()
                .st0
                .unwrap();
            let got = t::to_get_scale_button_icon_width(b, c, 0.0, eax, |m, id, s| {
                m.call(m.rebase(0x0090_62b0), Conv::Cdecl, &[id, s.to_bits()])
                    .unwrap()
                    .st0
                    .unwrap() as f32
            });
            assert_eq!(
                (want as f32).to_bits(),
                got.to_bits(),
                "icon id: char {c:#x}"
            );
            same_memory("icon id", a, b);
        }
    }
}

#[test]
fn round20_icon_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round20_icon(&mut a, &mut b, &mut Xorshift(0x4020_0011));
}

#[test]
fn round20_icon_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round20_icon(&mut a, &mut b, &mut Xorshift(0x4020_0012));
}

/// Round 20's save-name, macro word-count and save-window helpers over
/// stand-ins.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round20_text(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let ret = |cell: u32| {
        let mut v = vec![0xa1];
        v.extend_from_slice(&cell.to_le_bytes());
        v.push(0xc3);
        v
    };
    let s = stand_in8(a, b, 0x40, &logger(cursor, 0x2000_0100));
    redirect(a, b, 0x0079_af10, s);
    let s = stand_in8(a, b, 0x80, &ret(cells));
    redirect(a, b, 0x008a_c2f0, s);
    let s = stand_in8(a, b, 0xc0, &marker(cursor, 0x2000_0200));
    redirect(a, b, 0x0078_51d0, s);
    let s = stand_in8(a, b, 0x100, &logger(cursor, 0x2000_0300));
    redirect(a, b, 0x0078_4fb0, s);
    // TOGetChangeMacroAddress: the text in cells+4 for small indices, else 0
    let mut v = vec![
        0x8b, 0x44, 0x24, 0x08, 0x3d, 0x00, 0x00, 0x01, 0x00, 0x73, 0x06, 0xa1,
    ];
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    // the macros' own macro: no group-3 codes, so no deeper recursion
    let inner = alloc(a, b, 0x20);
    let inner_text: Vec<u8> = (0..0x20u8)
        .map(|j| if j < 0x1f { 0x30 + (j % 3) * 0x11 } else { 0 })
        .collect();
    a.write(inner, &inner_text).unwrap();
    b.write(inner, &inner_text).unwrap();
    let inner_cell = alloc(a, b, 4);
    put(a, b, inner_cell, inner);
    v.push(0xc3);
    v.push(0xa1);
    v.extend_from_slice(&inner_cell.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x140, &v);
    redirect(a, b, 0x008b_6660, s);
    let s = stand_in8(
        a,
        b,
        0x180,
        &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x03, 0xc3],
    );
    redirect(a, b, 0x008d_f610, s);
    let s = stand_in8(
        a,
        b,
        0x1c0,
        &[
            0x8b, 0x44, 0x24, 0x04, 0x0f, 0xb6, 0x08, 0x83, 0xe1, 0x01, 0x8d, 0x44, 0x08, 0x01,
            0xc3,
        ],
    );
    redirect(a, b, 0x008d_fd80, s);
    // MsGetSaveWindow: *n = cells[2], returns cells[3]
    let mut v = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d];
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0xa1]);
    v.extend_from_slice(&(cells + 12).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x200, &v);
    redirect(a, b, 0x0078_5410, s);
    let text = alloc(a, b, 0x40);
    let window = alloc(a, b, 0x40);
    let count = alloc(a, b, 4);
    let local = alloc(a, b, 4);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..500u32 {
        put(a, b, cursor, log);
        // the save name
        put(a, b, cells, rng.next() % 3);
        let i = if rng.next().is_multiple_of(4) {
            rng.next()
        } else {
            rng.next() % 0x20
        };
        assert_eq!(
            eax(a, 0x008a_c850, &[i]),
            t::to_get_save_chr_name(b, i, call),
            "save name {k}"
        );
        assert_eq!(
            eax(a, 0x0089_76f0, &[i]),
            t::to_btl_get_chr_name(b, i, call),
            "battle name {k}"
        );
        // the macro word count
        let n = rng.next() % 0x30;
        let bytes: Vec<u8> = (0..0x40)
            .map(|j| if j < n { (rng.next() as u8).max(1) } else { 0 })
            .collect();
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        put(a, b, cells + 4, if k % 7 == 0 { 0 } else { text });
        let (x, y) = (rng.next(), rng.next() % 0x100);
        assert_eq!(
            eax(a, 0x008b_6b00, &[x, y]),
            t::to_get_change_macro_word_count(b, x, y, &mut { call }, &mut |m, tb, ix| {
                eax(m, 0x008b_6b00, &[tb, ix])
            }),
            "word count {k}"
        );
        // the save window
        let words: Vec<u8> = (0..0x20)
            .flat_map(|_| {
                // a high bit: the generator's low bits alternate
                let w: u16 = if (rng.next() >> 16).is_multiple_of(2) {
                    0xff
                } else {
                    rng.next() as u16
                };
                w.to_le_bytes()
            })
            .collect();
        a.write(window, &words).unwrap();
        b.write(window, &words).unwrap();
        let kind = rng.next() % 24;
        let n = if rng.next().is_multiple_of(2) {
            rng.next() % 4
        } else {
            rng.next() % 0x22
        };
        put(a, b, cells + 8, n.wrapping_sub(2));
        put(a, b, cells + 12, window);
        assert_eq!(
            eax(a, 0x0089_b5a0, &[x, kind, count]),
            t::to_get_save_window(b, x, kind, count, local, call),
            "save window {k}"
        );
        let v: u32 = Mem::read(b, local);
        Mem::write(a, local, v);
        same_memory("round 20 text helpers", a, b);
    }
}

#[test]
fn round20_text_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round20_text(&mut a, &mut b, &mut Xorshift(0x4020_0021));
}

#[test]
fn round20_text_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round20_text(&mut a, &mut b, &mut Xorshift(0x4020_0022));
}

/// Round 20's number text, command-use check and simple help line over
/// stand-ins.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round20_btl(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let cells = alloc(a, b, 0x20);
    let ret = |cell: u32| {
        let mut v = vec![0xa1];
        v.extend_from_slice(&cell.to_le_bytes());
        v.push(0xc3);
        v
    };
    // *[esp+off] = [cell]
    let store = |off: u8, cell: u32| {
        let mut v = vec![0x8b, 0x44, 0x24, off, 0x8b, 0x0d];
        v.extend_from_slice(&cell.to_le_bytes());
        v.extend_from_slice(&[0x89, 0x08, 0xc3]);
        v
    };
    for (i, sym) in [
        0x008a_c2f0u32,
        0x0078_51d0,
        0x0079_a4b0,
        0x0079_a5b0,
        0x0078_c690,
    ]
    .into_iter()
    .enumerate()
    {
        let s = stand_in8(a, b, 0x40 + 0x20 * i as u32, &ret(cells + 4 * i as u32));
        redirect(a, b, sym, s);
    }
    let s = stand_in8(a, b, 0x100, &store(0x0c, cells + 0x14));
    redirect(a, b, 0x0089_b5a0, s);
    let s = stand_in8(a, b, 0x140, &store(0x0c, cells + 0x18));
    redirect(a, b, 0x008b_9650, s);
    let out = alloc(a, b, 0x40);
    let data = alloc(a, b, 0x20);
    let scratch = alloc(a, b, 8);
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..1000u32 {
        put(a, b, cells, [0u32, 9, 10, 3][(rng.next() % 4) as usize]);
        put(a, b, cells + 4, rng.next() % 2);
        // the number text
        let value = match rng.next() % 3 {
            0 => rng.next() % 100,
            1 => rng.next() % 1_000_000,
            _ => rng.next(),
        };
        let width = match rng.next() % 4 {
            0 => 0,
            1 => rng.next().wrapping_neg() | 0x8000_0000,
            _ => rng.next() % 14,
        };
        eax(a, 0x008b_9140, &[value, out, width]);
        t::to_value_to_mes_font_l(b, value, out, width, call);
        same_memory("number text", a, b);
        // the command use
        let bytes: Vec<u8> = (0..0x20).map(|_| rng.next() as u8).collect();
        a.write(data, &bytes).unwrap();
        b.write(data, &bytes).unwrap();
        put(
            a,
            b,
            data + 0x16,
            [8u8, 0x10, 0x18, 0x0f, rng.next() as u8][(rng.next() % 5) as usize],
        );
        put(a, b, cells + 8, data);
        put(a, b, cells + 12, rng.next() % 3);
        put(
            a,
            b,
            cells + 16,
            [0u32, 0xffff_ffff, 1][(rng.next() % 3) as usize],
        );
        put(a, b, cells + 0x14, rng.next() % 3);
        put(
            a,
            b,
            g(a, 0x0133_c94c),
            if rng.next().is_multiple_of(2) {
                0x100
            } else {
                rng.next()
            },
        );
        let command = [0xfeu32, 0xff, rng.next() % 0x100, rng.next()][(rng.next() % 4) as usize];
        let chr = rng.next() % 0x20;
        assert_eq!(
            eax(a, 0x0089_aca0, &[chr, command]),
            t::to_check_btl_command_use(b, chr, command, scratch, call),
            "command use {k}"
        );
        // the simple help line
        let width = match rng.next() % 3 {
            0 => (rng.next() % 600) as f32 / 3.0,
            1 => -((rng.next() % 600) as f32) / 7.0,
            _ => f32::from_bits(rng.next() & 0xbfff_ffff),
        };
        put(a, b, cells + 0x18, width.to_bits());
        let i = rng.next() % 8;
        let message = rng.next();
        assert_eq!(
            eax(a, 0x0089_dba0, &[i, message]),
            t::to_btl_open_simple_help_mes(b, i, message, scratch, call),
            "help {k}"
        );
        let v: [u32; 2] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory("round 20 battle helpers", a, b);
        // the number text: after the first terminator the original copies
        // stale stack bytes, so those are taken from the port
        let value = match rng.next() % 4 {
            0 => rng.next() % 100,
            1 => rng.next() % 1_000_000,
            2 => rng.next().wrapping_neg(),
            _ => rng.next(),
        };
        let digits = rng.next() % 10;
        let mode = [0u32, 1, 0, 1, 2][(rng.next() % 5) as usize];
        let fill: Vec<u8> = (0..0x40).map(|_| rng.next() as u8).collect();
        a.write(out, &fill).unwrap();
        b.write(out, &fill).unwrap();
        assert_eq!(
            eax(a, 0x008b_8290, &[value, out, digits, mode]),
            t::to_int_to_mes(b, value, out, digits, mode, call),
            "int text {k}: {value} {digits} {mode}"
        );
        let got: [u8; 0x40] = Mem::read(a, out);
        let ours: [u8; 0x40] = Mem::read(b, out);
        let end = got.iter().position(|&c| c == 0).unwrap_or(0x3f);
        assert_eq!(
            got[..=end],
            ours[..=end],
            "int text {k}: {value} {digits} {mode}"
        );
        a.write(out + end as u32, &ours[end..]).unwrap();
        same_memory("int text", a, b);
    }
}

#[test]
fn round20_btl_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round20_btl(&mut a, &mut b, &mut Xorshift(0x4020_0031));
}

#[test]
fn round20_btl_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round20_btl(&mut a, &mut b, &mut Xorshift(0x4020_0032));
}

/// Round 20's Al Bhed translation over random tables, languages and
/// collected letters.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
fn round20_albhed(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let cells = alloc(a, b, 8);
    for (i, sym) in [0x008a_c2f0u32, 0x0078_51d0].into_iter().enumerate() {
        let mut v = vec![0xa1];
        v.extend_from_slice(&(cells + 4 * i as u32).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0x40 + 0x20 * i as u32, &v);
        redirect(a, b, sym, s);
    }
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let input = alloc(a, b, 4);
    let out = alloc(a, b, 8);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..3000u32 {
        if k % 100 == 0 {
            let tables: Vec<u8> = (0..0x300).map(|_| rng.next() as u8).collect();
            let at = g(a, 0x0187_626c);
            a.write(at, &tables).unwrap();
            b.write(at, &tables).unwrap();
        }
        put(
            a,
            b,
            cells,
            [0u32, 0, 9, 10, 10, 3, 1][(rng.next() % 7) as usize],
        );
        put(a, b, cells + 4, rng.next() % 2);
        let switch = [0xffff_ffffu32, 1, 0, 2][(rng.next() % 4) as usize];
        put(a, b, g(a, 0x0187_63e0), switch);
        let bits = if rng.next().is_multiple_of(5) {
            0x03ff_ffff
        } else {
            rng.next() & 0x03ff_ffff
        };
        put(a, b, g(a, 0x0113_07a0), bits);
        let lead = [
            4u8,
            0x2c,
            0x2d,
            0x2e,
            0x2f,
            rng.next() as u8,
            rng.next() as u8,
        ][(rng.next() % 7) as usize];
        let second = if lead == 4 && rng.next().is_multiple_of(2) {
            0x2c + (rng.next() % 4) as u8
        } else {
            rng.next() as u8
        };
        let mut bytes = [lead, second, rng.next() as u8, rng.next() as u8];
        match rng.next() % 4 {
            // the single-byte table boundaries
            0 => {
                let edges = [
                    0x5fu8, 0xae, 0xaf, 0x5e, 0x4f, 0x50, 0x69, 0x6a, 0x6f, 0x70, 0x89, 0x8a,
                ];
                bytes[0] = edges[(rng.next() % 12) as usize];
            }
            // the kana-id boundaries, encoded with or without the 4 lead
            1 => {
                let ids = [
                    0x5d, 0x5e, 0x5f, 0x2b4, 0x2b5, 0x3b8, 0x3b9, 0x40f, 0x410, 0x57f, 0x580,
                    0x711, 0x712, 0x713,
                ];
                let id: i32 = ids[(rng.next() % 14) as usize];
                let (lead, rest) = if id >= 0x410 {
                    (true, id - 0x410)
                } else {
                    (false, id)
                };
                let mut code = Vec::new();
                if lead {
                    code.push(4u8);
                }
                let pair = (0x2cu8..0x30)
                    .find(|&c| (0..0x100).contains(&(rest - (i32::from(c) * 0xd0 - 0x2320))));
                match pair {
                    Some(c) if !(0..0xd0).contains(&rest) || rng.next().is_multiple_of(2) => {
                        code.push(c);
                        code.push((rest - (i32::from(c) * 0xd0 - 0x2320)) as u8);
                    }
                    _ => code.push((rest + 0x30) as u8),
                }
                bytes[..code.len()].copy_from_slice(&code);
            }
            _ => {}
        }
        put(a, b, input, bytes);
        put(a, b, out, [rng.next(), rng.next()]);
        assert_eq!(
            eax(a, 0x0090_6780, &[input, out]),
            t::to_conv_albhed_word(b, input, out, call),
            "al bhed {k}"
        );
        same_memory("al bhed", a, b);
    }
}

#[test]
fn round20_albhed_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round20_albhed(&mut a, &mut b, &mut Xorshift(0x4020_0041));
}

#[test]
fn round20_albhed_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round20_albhed(&mut a, &mut b, &mut Xorshift(0x4020_0042));
}

/// Round 21's font dictionary and glyph info over stand-ins for the
/// language, English flag, width tables and the `:` join test.
#[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
fn round21_font(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let cells = alloc(a, b, 0x10);
    for (i, sym) in [0x008a_c2f0u32, 0x0078_51d0, 0x008a_c3d0, 0x008b_5760]
        .into_iter()
        .enumerate()
    {
        let mut v = vec![0xa1];
        v.extend_from_slice(&(cells + 4 * i as u32).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0x40 + 0x20 * i as u32, &v);
        redirect(a, b, sym, s);
    }
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let table = alloc(a, b, 0x2000);
    let widths: Vec<u8> = (0..0x2000).map(|_| rng.next() as u8).collect();
    a.write(table, &widths).unwrap();
    b.write(table, &widths).unwrap();
    let text = alloc(a, b, 0x20);
    let out = alloc(a, b, 0x30);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..3000u32 {
        put(
            a,
            b,
            cells,
            [0u32, 0, 9, 10, 3, 1, 8, 0xffff_ffff][(rng.next() % 8) as usize],
        );
        put(a, b, cells + 4, rng.next() % 2);
        put(a, b, g(a, 0x025d_16c0), rng.next() % 3);
        put(a, b, g(a, 0x0187_1534), rng.next() % 2);
        let mut bytes: Vec<u8> = (0..0x20).map(|_| rng.next() as u8).collect();
        let leads = [
            4u8, 0x2c, 0x2d, 0x2e, 0x2f, 0x2a, 0x2b, 0x28, 0x29, 0x26, 0x27, 6, 0x47, 0x3a, 0x5c,
            0x5e,
        ];
        if !rng.next().is_multiple_of(4) {
            bytes[0] = leads[(rng.next() % 16) as usize];
        }
        if rng.next().is_multiple_of(3) {
            bytes[1] = leads[(rng.next() % 16) as usize];
        }
        if bytes[0] == 0x47 && rng.next().is_multiple_of(2) {
            let n = 1 + (rng.next() % 6) as usize;
            for c in &mut bytes[..n] {
                *c = 0x47;
            }
        }
        // a table near the middle of its block, so lead rows stay inside;
        // none only for the table-5 icon, the one code that checks for it
        let none = bytes[0] == 6 && rng.next().is_multiple_of(3);
        put(a, b, cells + 8, if none { 0 } else { table + 0x1000 });
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        let fill: Vec<u8> = (0..0x30).map(|_| rng.next() as u8).collect();
        a.write(out, &fill).unwrap();
        b.write(out, &fill).unwrap();
        assert_eq!(
            eax(a, 0x008a_c760, &[]),
            t::to_get_international_base_dic_type(b, call),
            "dic {k}"
        );
        assert_eq!(
            eax(a, 0x008b_7240, &[text, out]),
            t::to_get_font_info(b, text, out, call),
            "font info {k}: {:x?}",
            &bytes[..4]
        );
        same_memory("font info", a, b);
    }
}

#[test]
fn round21_font_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round21_font(&mut a, &mut b, &mut Xorshift(0x4021_0011));
}

#[test]
fn round21_font_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round21_font(&mut a, &mut b, &mut Xorshift(0x4021_0012));
}

/// Round 21's battle text widths over the original glyph, advance and
/// letter-space functions, with `TOGetChangeMacroAddress` a stand-in that
/// gives one macro text for codes inside the text.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round21_widths(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let text = alloc(a, b, 0x100);
    let macro_text = alloc(a, b, 0x100);
    // [esp+8] in text..text+0x40: macro_text, else 0
    let mut v = vec![0x8b, 0x44, 0x24, 0x08, 0x2d];
    v.extend_from_slice(&text.to_le_bytes());
    v.extend_from_slice(&[0x83, 0xf8, 0x40, 0x73, 0x06, 0xb8]);
    v.extend_from_slice(&macro_text.to_le_bytes());
    v.push(0xc3);
    v.extend_from_slice(&[0x31, 0xc0, 0xc3]);
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x008b_6660, s);
    // TOGetFontInfo: the code byte into the record, the next byte
    let s = stand_in8(
        a,
        b,
        0x80,
        &[
            0x8b, 0x44, 0x24, 0x04, 0x8b, 0x4c, 0x24, 0x08, 0x0f, 0xb6, 0x10, 0x89, 0x11, 0x8d,
            0x40, 0x01, 0xc3,
        ],
    );
    redirect(a, b, 0x008b_7240, s);
    // TOGetFontAdvance: the recorded code plus the scale
    let s = stand_in8(
        a,
        b,
        0xc0,
        &[
            0x8b, 0x44, 0x24, 0x08, 0x0f, 0xb6, 0x00, 0x50, 0xdb, 0x04, 0x24, 0x58, 0xd8, 0x44,
            0x24, 0x0c, 0xc3,
        ],
    );
    redirect(a, b, 0x008f_a740, s);
    // TOGetFontSpaceInPiexl: 1.0; TOGetScaleButtonIconWidth: the icon code
    let s = stand_in8(a, b, 0x100, &[0xd9, 0xe8, 0xc3]);
    redirect(a, b, 0x008f_a570, s);
    let s = stand_in8(a, b, 0x110, &[0xdb, 0x44, 0x24, 0x04, 0xc3]);
    redirect(a, b, 0x008f_a800, s);
    // graphicFontGetScreenWH: 640 x 448
    let s = stand_in8(
        a,
        b,
        0x120,
        &[
            0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00, 0x80, 0x02, 0x00, 0x00, 0x8b, 0x44, 0x24, 0x08,
            0xc7, 0x00, 0xc0, 0x01, 0x00, 0x00, 0xc3,
        ],
    );
    redirect(a, b, 0x0064_0db0, s);
    // the language and English flag from cells
    let lang = alloc(a, b, 8);
    for (i, sym) in [0x008a_c2f0u32, 0x0078_51d0].into_iter().enumerate() {
        let mut v = vec![0xa1];
        v.extend_from_slice(&(lang + 4 * i as u32).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0x140 + 0x10 * i as u32, &v);
        redirect(a, b, sym, s);
    }
    let out = alloc(a, b, 4);
    let scratch = alloc(a, b, 0x20);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let codes = [
        0x30u8, 0x31, 0x45, 0x47, 0x3a, 0x7a, 7, 0xb, 0x12, 0x13, 0x22, 0x2c, 0x2d, 0x10, 0x5c,
    ];
    let random_text = |rng: &mut Xorshift, n: u32| -> Vec<u8> {
        let mut v = vec![0u8; 0x100];
        let mut k = 0usize;
        while (k as u32) < n {
            let c = if rng.next().is_multiple_of(3) {
                (0x30 + rng.next() % 0x80) as u8
            } else {
                codes[(rng.next() % 15) as usize]
            };
            v[k] = c;
            v[k + 1] = match c {
                7 => (0xd8 + rng.next() % 0x20) as u8,
                0xb => (0x20 + rng.next() % 0x50) as u8,
                _ => (0x30 + rng.next() % 0x80) as u8,
            };
            k += if c < 0x30 { 2 } else { 1 };
        }
        v[k] = 0;
        v
    };
    for k in 0..300u32 {
        put(
            a,
            b,
            lang,
            [[0u32, 9, 10, 3][(rng.next() % 4) as usize], rng.next() % 2],
        );
        let n = rng.next() % 0x18;
        let bytes = random_text(rng, n);
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        let n = rng.next() % 0x10;
        // nested macros inside the macro give no text (the stand-in), but
        // still take their own path
        let bytes = random_text(rng, n);
        a.write(macro_text, &bytes).unwrap();
        b.write(macro_text, &bytes).unwrap();
        let scale = [1.0f32, 0.5, 1.25, 0.8, (rng.next() % 300) as f32 / 100.0]
            [(rng.next() % 5) as usize]
            .to_bits();
        let extra = [1.0f32, 0.7][(rng.next() % 2) as usize].to_bits();
        put(a, b, out, rng.next());
        eax(a, 0x0090_5290, &[text, out, 0, scale]);
        t::to_get_btl_easy_font_width(b, text, out, scale, call, fcall);
        same_memory("easy font width", a, b);
        let value =
            [rng.next() % 10, rng.next() % 100_000, rng.next(), 0][(rng.next() % 4) as usize];
        let digits = rng.next() % 8;
        eax(a, 0x0090_5190, &[digits, value, out, scale]);
        t::to_get_btl_easy_digit_width(b, digits, value, out, scale, scratch, call, fcall);
        let tail: [u8; 0x20] = Mem::read(b, scratch);
        a.write(scratch, &tail).unwrap();
        same_memory("easy digit width", a, b);
        eax(a, 0x0090_5230, &[value, out, scale, extra]);
        t::to_get_btl_easy_digit_width2(b, value, out, scale, extra, call);
        same_memory("easy digit width 2", a, b);
        let table = if rng.next().is_multiple_of(5) {
            u32::MAX
        } else {
            rng.next() % 0x10
        };
        let code = text + rng.next() % 0x20;
        let want = fcall(a, 0x008f_a920, &[table, code, scale, extra]);
        let got = t::to_get_width_macro_help(b, table, code, scale, extra, call, fcall);
        assert_eq!((want as f32).to_bits(), got.to_bits(), "macro width {k}");
        same_memory("macro width", a, b);
        eax(a, 0x0090_5320, &[table, text, out, scale, extra]);
        t::to_get_cross_ext_mes_font_width(b, table, text, out, scale, extra, scratch, call, fcall);
        let wh: [u32; 2] = Mem::read(b, scratch);
        Mem::write(a, scratch, wh);
        same_memory("cross ext width", a, b);
    }
}

#[test]
fn round21_widths_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round21_widths(&mut a, &mut b, &mut Xorshift(0x4021_0021));
}

#[test]
fn round21_widths_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round21_widths(&mut a, &mut b, &mut Xorshift(0x4021_0022));
}

/// Round 21's macro text lookup over logging stand-ins for the name,
/// number and item functions, with random macro slots and tables.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round21_macro(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x800);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let g = |a: &Oracle, x: u32| a.rebase(x);
    for (i, sym) in [
        0x008b_6210u32,
        0x008b_8db0,
        0x008b_8dd0,
        0x008b_8d70,
        0x008b_8d90,
        0x0079_8ff0,
        0x0079_0860,
        0x008a_c850,
    ]
    .into_iter()
    .enumerate()
    {
        let s = stand_in8(
            a,
            b,
            0x40 + 0x30 * i as u32,
            &logger(cursor, 0x2100_0100 * (i as u32 + 1)),
        );
        redirect(a, b, sym, s);
    }
    // TOIntToMes: logs all four arguments, returns the first
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    for k in 0..4u8 {
        // mov eax, [esp+4+4k]; mov [ecx+4k], eax
        v.extend_from_slice(&[0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 * k]);
    }
    v.extend_from_slice(&[0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x10, 0x8b, 0x44, 0x24, 0x04, 0xc3]);
    let s = stand_in8(a, b, 0x200, &v);
    redirect(a, b, 0x008b_8290, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&cells.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x240, &v);
    redirect(a, b, 0x0078_51d0, s);
    let s = stand_in8(a, b, 0x260, &[0x8b, 0x44, 0x24, 0x04, 0xc3]);
    redirect(a, b, 0x00a4_4650, s);
    // MsGetRomItem: *out = cells[1], returns cells + 8
    let mut v = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d];
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0xb8]);
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x280, &v);
    redirect(a, b, 0x0079_09f0, s);
    let entries = alloc(a, b, 64 * 8);
    let windows = alloc(a, b, 2 * 0x24);
    let strings = alloc(a, b, 0x400);
    let tables = alloc(a, b, 0x40);
    let text = alloc(a, b, 0x440);
    let counters = alloc(a, b, 2 * 0x10);
    let code = alloc(a, b, 4);
    let scratch = alloc(a, b, 8);
    for w in 0..2u32 {
        put(a, b, g(a, 0x0186_76f0) + 4 * w, windows + 0x24 * w);
        put(a, b, g(a, 0x0186_8a90) + 4 * w, counters + 0x10 * w);
        put(a, b, g(a, 0x0186_9eb0) + 4 * w, tables);
    }
    for k in 0..64u32 {
        put(a, b, g(a, 0x0186_9ab0) + 4 * k, entries + 8 * k);
    }
    for k in 0..0x10u32 {
        put(a, b, tables + 4 * k, text);
    }
    put(a, b, g(a, 0x025d_16c4), strings);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let kinds = [
        0xffu8, 0, 1, 2, 0x33, 0x34, 0x35, 0x36, 0x37, 0x13, 0x18, 0x23, 0x2b, 0x32, 0x40,
    ];
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        if k % 50 == 0 {
            for blob in [strings, text] {
                let bytes: Vec<u8> = (0..0x400).map(|_| rng.next() as u8).collect();
                a.write(blob, &bytes).unwrap();
                b.write(blob, &bytes).unwrap();
            }
        }
        for e in 0..64u32 {
            let kind = kinds[(rng.next() % 15) as usize];
            put(a, b, entries + 8 * e, [rng.next() as u8, kind, 0, 0]);
            put(a, b, entries + 8 * e + 4, rng.next() % (0xd0 * 6));
        }
        for w in 0..2u32 {
            put(a, b, windows + 0x24 * w + 0x20, (rng.next() % 16) as u16);
            put(a, b, counters + 0x10 * w + 0xc, rng.next());
        }
        put(a, b, cells, rng.next() % 2);
        put(a, b, cells + 4, rng.next());
        put(a, b, cells + 8, rng.next());
        put(
            a,
            b,
            g(a, 0x0231_0dac),
            [0u32, 0, 5, 0xffff_fff0][(rng.next() % 4) as usize],
        );
        for (off, field) in [(0x0231_0db0u32, 0u32), (0x0231_0db1, 1), (0x0231_0db2, 2)] {
            let _ = field;
            put(
                a,
                b,
                g(a, off),
                [0u8, 0, 1, 2, 0xff][(rng.next() % 5) as usize],
            );
        }
        put(a, b, g(a, 0x0231_0db4), rng.next() as u16);
        put(
            a,
            b,
            g(a, 0x0231_0dcc),
            [rng.next() as u16, rng.next() as u16],
        );
        let c = (0x10 + rng.next() % 0x15) as u8;
        let next = if c == 0x12 && rng.next().is_multiple_of(4) {
            (0x50 + rng.next() % 0xb0) as u8
        } else if c == 0x10 {
            (0x4e + rng.next() % 0xc) as u8
        } else {
            (0x30 + rng.next() % 0x20) as u8
        };
        put(a, b, code, [c, next, 0, 0]);
        let window = rng.next() % 2;
        let want = eax(a, 0x008b_6660, &[window, code]);
        let got = t::to_get_change_macro_address(b, window, code, scratch, call);
        if got == scratch + 4 {
            // a folded macro expanded to its own code: the original
            // returns that code in its stack (its argument slot), the
            // port in `scratch`; the code is the same
            let theirs: [u8; 3] = Mem::read(a, want);
            let ours: [u8; 3] = Mem::read(b, got);
            assert_eq!(theirs, ours, "macro {k}: {c:#x} {next:#x}");
        } else {
            assert_eq!(want, got, "macro {k}: {c:#x} {next:#x}");
        }
        let v: [u32; 2] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory("macro address", a, b);
    }
}

#[test]
fn round21_macro_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round21_macro(&mut a, &mut b, &mut Xorshift(0x4021_0031));
}

#[test]
fn round21_macro_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round21_macro(&mut a, &mut b, &mut Xorshift(0x4021_0032));
}

/// Round 21's macro width and message size over stand-ins for the glyph
/// widths, Al Bhed, icons, window type and macro lookup.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round21_message(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let text = alloc(a, b, 0x100);
    let macro_text = alloc(a, b, 0x100);
    let cells = alloc(a, b, 0x10);
    // TOGetChangeMacroAddress: macro_text for codes inside the text
    let mut v = vec![0x8b, 0x44, 0x24, 0x08, 0x2d];
    v.extend_from_slice(&text.to_le_bytes());
    v.extend_from_slice(&[0x3d, 0x00, 0x01, 0x00, 0x00, 0x73, 0x06, 0xb8]);
    v.extend_from_slice(&macro_text.to_le_bytes());
    v.push(0xc3);
    v.extend_from_slice(&[0x31, 0xc0, 0xc3]);
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x008b_6660, s);
    // TOGetFontWidthInterMode: *w = the byte, the next byte
    let s = stand_in8(
        a,
        b,
        0x80,
        &[
            0x8b, 0x44, 0x24, 0x04, 0x0f, 0xb6, 0x08, 0x51, 0xdb, 0x04, 0x24, 0x59, 0x8b, 0x4c,
            0x24, 0x0c, 0xd9, 0x19, 0x40, 0xc3,
        ],
    );
    redirect(a, b, 0x008b_7be0, s);
    // TOConvAlbhedWord: the byte plus one plus the type-1 flag 0x018663ec,
    // so the flag's value while measuring shows
    let mut v = vec![
        0x8b, 0x44, 0x24, 0x04, 0x8b, 0x4c, 0x24, 0x08, 0x8a, 0x00, 0xfe, 0xc0, 0x02, 0x05,
    ];
    v.extend_from_slice(&g(a, 0x0186_63ec).to_le_bytes());
    v.extend_from_slice(&[0x88, 0x01, 0xc6, 0x41, 0x01, 0x00, 0xc3]);
    let s = stand_in8(a, b, 0xc0, &v);
    redirect(a, b, 0x0090_6780, s);
    let s = stand_in8(a, b, 0x100, &[0xdb, 0x44, 0x24, 0x04, 0xc3]);
    redirect(a, b, 0x008f_a800, s);
    for (i, sym) in [0x0090_6c40u32, 0x008a_c2f0, 0x0078_51d0]
        .into_iter()
        .enumerate()
    {
        let mut v = vec![0xa1];
        v.extend_from_slice(&(cells + 4 * i as u32).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0x120 + 0x10 * i as u32, &v);
        redirect(a, b, sym, s);
    }
    let windows = alloc(a, b, 8 * 0x30);
    let counters = alloc(a, b, 8 * 0x10);
    for w in 0..8u32 {
        put(a, b, g(a, 0x0186_76f0) + 4 * w, windows + 0x30 * w);
        put(a, b, g(a, 0x0186_8a90) + 4 * w, counters + 0x10 * w);
    }
    let out = alloc(a, b, 8);
    let scratch = alloc(a, b, 0x40);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let random_text = |rng: &mut Xorshift, n: u32, end: u8| -> Vec<u8> {
        let mut v = Vec::new();
        for _ in 0..n {
            match rng.next() % 12 {
                0 => v.push((1 + rng.next() % 3) as u8),
                1 => v.push(5),
                2 => v.extend_from_slice(&[
                    7,
                    [0x38u8, 0x40, 0x80, 0xbf, 0xe0, 0xe1, 0xef, 0xf0][(rng.next() % 8) as usize],
                ]),
                3 => v.extend_from_slice(&[0xe, [0x40u8, 0x41][(rng.next() % 2) as usize]]),
                4 => v.extend_from_slice(&[0xb, (0x30 + rng.next() % 0x40) as u8]),
                5 => v.extend_from_slice(&[
                    0x10,
                    [0x50u8, 0x5f, 0x60, 0xff, 0x30][(rng.next() % 5) as usize],
                ]),
                6 => v.extend_from_slice(&[
                    0x12 + (rng.next() % 0x10) as u8,
                    (0x30 + rng.next() % 0x20) as u8,
                ]),
                _ => v.push((0x30 + rng.next() % 0xc0) as u8),
            }
        }
        v.push(end);
        v.resize(0x100, 0);
        v
    };
    for k in 0..1500u32 {
        let n = rng.next() % 0x14;
        let bytes = random_text(rng, n, 0);
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        // the macro text: no line codes (they stop only messages), no
        // nested macros deeper than the stand-in gives
        let n = rng.next() % 8;
        let mut bytes = random_text(rng, n, 0);
        for c in bytes.iter_mut().take_while(|c| **c != 0) {
            if (1..=3).contains(c) {
                *c = 0x31;
            }
        }
        a.write(macro_text, &bytes).unwrap();
        b.write(macro_text, &bytes).unwrap();
        for w in 0..8u32 {
            let info = windows + 0x30 * w;
            put(a, b, info + 0xe, rng.next() as u16);
            put(
                a,
                b,
                info + 0x10,
                if rng.next().is_multiple_of(3) {
                    0x4000u16
                } else {
                    rng.next() as u16
                },
            );
            put(a, b, info + 0x1e, rng.next() as u16);
            put(a, b, info + 0x24, (rng.next() % 2) as u16);
            put(a, b, counters + 0x10 * w + 0xc, rng.next() % 100_000);
        }
        put(a, b, cells, rng.next() % 3);
        put(a, b, cells + 4, [0u32, 3, 9][(rng.next() % 3) as usize]);
        put(a, b, cells + 8, rng.next() % 2);
        for at in [0x0134_0824u32, 0x0133_c948, 0x0132_7100] {
            put(a, b, g(a, at), u32::from(rng.next().is_multiple_of(4)));
        }
        put(a, b, g(a, 0x0186_63ec), rng.next());
        let window = [0u32, 3, 7, 8, 0xffff_ffff][(rng.next() % 5) as usize];
        let window = if window == 8 || window == 0xffff_ffff {
            window
        } else {
            window % 8
        };
        let mode = if rng.next().is_multiple_of(8) {
            0x8000_0001
        } else {
            rng.next() % 5
        };
        let flags = (rng.next() % 4) | [0u32, 0x100, 0x200][(rng.next() % 3) as usize];
        // the macro width on its own
        let code = text + rng.next() % 0x10;
        let safe_window = window % 8;
        let want = fcall(a, 0x008b_6a30, &[safe_window, code, mode]);
        let got = t::to_get_change_macro_width(b, safe_window, code, mode, scratch, call, fcall);
        assert_eq!((want as f32).to_bits(), got.to_bits(), "macro width {k}");
        let v: u32 = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory("macro width", a, b);
        // -1 skips every window-dependent code; 8 would read past the tables
        let window = if window == 8 { safe_window } else { window };
        put(a, b, out, [rng.next(), rng.next()]);
        assert_eq!(
            eax(a, 0x008b_9650, &[window, text, out, out + 4, mode, flags]),
            t::to_get_message_wh(
                b,
                window,
                text,
                out,
                out + 4,
                mode,
                flags,
                scratch,
                call,
                fcall
            ),
            "message {k}"
        );
        let v: [u32; 16] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory("message size", a, b);
    }
}

#[test]
fn round21_message_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round21_message(&mut a, &mut b, &mut Xorshift(0x4021_0041));
}

#[test]
fn round21_message_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round21_message(&mut a, &mut b, &mut Xorshift(0x4021_0042));
}

/// Round 21's battle command windows over logging stand-ins for the
/// command data, checks, windows and targets.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round21_btl(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    let ret = |cell: u32| {
        let mut v = vec![0xa1];
        v.extend_from_slice(&cell.to_le_bytes());
        v.push(0xc3);
        v
    };
    let mut off = 0x40u32;
    let mut stand = |a: &mut Oracle, b: &mut Oracle, sym: u32, code: &[u8]| {
        let s = stand_in8(a, b, off, code);
        redirect(a, b, sym, s);
        off += 0x40;
    };
    stand(a, b, 0x0089_afc0, &marker(cursor, 0x2101_0001));
    stand(a, b, 0x0079_2d60, &logger_args(cursor, 3));
    stand(a, b, 0x0078_51c0, &ret(cells));
    stand(a, b, 0x0089_89d0, &marker(cursor, 0x2101_0002));
    stand(a, b, 0x008a_e5a0, &logger(cursor, 0x2101_0003));
    // MsGetComData: *out = 0x1234, returns the data in cells + 4
    let mut v = vec![
        0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00, 0x34, 0x12, 0x00, 0x00, 0xa1,
    ];
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.push(0xc3);
    stand(a, b, 0x0079_a4b0, &v);
    stand(a, b, 0x0089_b5a0, &logger_args(cursor, 3));
    stand(a, b, 0x008a_10d0, &logger(cursor, 0x2101_0004));
    stand(a, b, 0x008a_0de0, &logger(cursor, 0x2101_0005));
    stand(a, b, 0x0078_cf70, &logger_args(cursor, 2));
    stand(a, b, 0x0089_aca0, &ret(cells + 8));
    stand(a, b, 0x0089_9ac0, &logger(cursor, 0x2101_0006));
    // TOBtlSetTarget: marks the level targeting (+2 = 1) and logs
    let mut v = vec![
        0x8b, 0x44, 0x24, 0x04, 0x69, 0xc0, 0xf0, 0x00, 0x00, 0x00, 0xc6, 0x80,
    ];
    v.extend_from_slice(&(g(a, 0x0133_c952)).to_le_bytes());
    v.push(0x01);
    v.extend_from_slice(&logger_args(cursor, 3));
    stand(a, b, 0x0089_a860, &v);
    // MsGetChrStatDeath: the id less cells[3], so 1, 0 and other values
    let mut v = vec![0x8b, 0x44, 0x24, 0x04, 0x2b, 0x05];
    v.extend_from_slice(&(cells + 12).to_le_bytes());
    v.push(0xc3);
    stand(a, b, 0x0079_5780, &v);
    let data = alloc(a, b, 0x20);
    let scratch = alloc(a, b, 4);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let levels = g(a, 0x0133_c860);
    for k in 0..800u32 {
        put(a, b, cursor, log);
        let bytes: Vec<u8> = (0..5 * 0xf0).map(|_| rng.next() as u8).collect();
        a.write(levels, &bytes).unwrap();
        b.write(levels, &bytes).unwrap();
        for l in 0..5u32 {
            let level = levels + 0xf0 * l;
            put(a, b, level + 2, u8::from(rng.next().is_multiple_of(2)));
            put(a, b, level + 0xb, (rng.next() % 8) as u8);
            put(
                a,
                b,
                level + 0xc,
                [0x311eu16, 0x3029, rng.next() as u16][(rng.next() % 3) as usize],
            );
            put(
                a,
                b,
                level + 0xa4,
                [(rng.next() % 4) as u16, (rng.next() % 4) as u16],
            );
            put(a, b, level + 8, (rng.next() % 0x20) as u16);
        }
        let list: Vec<u8> = (0..0x140).map(|_| rng.next() as u8).collect();
        a.write(g(a, 0x0133_d0d0), &list).unwrap();
        b.write(g(a, 0x0133_d0d0), &list).unwrap();
        let bytes: Vec<u8> = (0..0x20).map(|_| rng.next() as u8).collect();
        a.write(data, &bytes).unwrap();
        b.write(data, &bytes).unwrap();
        put(a, b, cells, rng.next() % 2);
        put(a, b, cells + 4, data);
        put(
            a,
            b,
            cells + 8,
            [0u32, 0x10, 0x80, 0x81, 0xfe, 0xff, 0xffff_ffff, 0xffff_fffe]
                [(rng.next() % 8) as usize],
        );
        put(a, b, cells + 12, rng.next() % 4);
        put(
            a,
            b,
            g(a, 0x023c_c092),
            [0xffu8, 0, 0, 1, 2][(rng.next() % 5) as usize],
        );
        put(
            a,
            b,
            g(a, 0x0133_c94c),
            [0x0001_0000u32, 0x100, 0][(rng.next() % 3) as usize],
        );
        put(
            a,
            b,
            g(a, 0x0133_d168),
            [0xffff_ffffu32, 0][(rng.next() % 2) as usize],
        );
        put(
            a,
            b,
            g(a, 0x0133_d0e0),
            u32::from(rng.next().is_multiple_of(4)),
        );
        put(a, b, g(a, 0x0133_f0d8), rng.next() % 5);
        for i in 0..4u32 {
            put(a, b, g(a, 0x0133_f0dc) + 2 * i, (rng.next() % 6) as u16);
            put(a, b, g(a, 0x0133_d180) + 4 * i, rng.next());
        }
        put(a, b, g(a, 0x0133_d17c), rng.next() as u16);
        put(a, b, g(a, 0x023c_c088), (rng.next() % 4) as u16);
        put(
            a,
            b,
            g(a, 0x0133_d770),
            [
                (rng.next() % 2) as u8,
                (rng.next() % 2) as u8,
                (rng.next() % 2) as u8,
                (rng.next() % 2) as u8,
            ],
        );
        put(a, b, g(a, 0x0133_d6e0), rng.next() % 2);
        put(a, b, g(a, 0x023c_c038), rng.next());
        put(a, b, g(a, 0x023c_c040), (rng.next() % 0x20) as u8);
        let chr = rng.next() % 0x20;
        let command =
            [0xfeu32, 0xfffe, 0xffff, rng.next() % 0x4000, 0x3115][(rng.next() % 5) as usize];
        match k % 3 {
            0 => assert_eq!(
                eax(a, 0x0089_75a0, &[]),
                t::to_btl_finish_command(b, call),
                "finish {k}"
            ),
            1 => {
                let level = rng.next() % 4;
                assert_eq!(
                    eax(a, 0x0089_9870, &[level, chr, command]),
                    t::to_btl_set_command(b, level, chr, command, scratch, call),
                    "set command {k}"
                );
            }
            _ => {
                // the finish and sub-window steps as markers here
                assert_eq!(
                    eax(a, 0x0089_a330, &[chr, command]),
                    t::to_btl_set_next_action(b, chr, command, scratch, call),
                    "next action {k}"
                );
            }
        }
        let v: u32 = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory("battle command windows", a, b);
    }
    // the next action over stand-ins for the finish and sub-window steps
    stand(a, b, 0x0089_75a0, &marker(cursor, 0x2101_0007));
    stand(a, b, 0x0089_9870, &logger_args(cursor, 3));
    for k in 0..400u32 {
        put(a, b, cursor, log);
        let bytes: Vec<u8> = (0..5 * 0xf0).map(|_| rng.next() as u8).collect();
        a.write(levels, &bytes).unwrap();
        b.write(levels, &bytes).unwrap();
        for l in 0..5u32 {
            let level = levels + 0xf0 * l;
            put(a, b, level + 2, u8::from(rng.next().is_multiple_of(2)));
            put(a, b, level + 0xb, (rng.next() % 8) as u8);
            put(
                a,
                b,
                level + 0xa4,
                [(rng.next() % 4) as u16, (rng.next() % 4) as u16],
            );
        }
        put(a, b, cells, rng.next() % 2);
        put(
            a,
            b,
            cells + 8,
            [0u32, 0x10, 0x80, 0x81, 0xfe, 0xff, 0xffff_ffff][(rng.next() % 7) as usize],
        );
        put(a, b, cells + 12, rng.next() % 4);
        put(
            a,
            b,
            g(a, 0x023c_c092),
            [0xffu8, 0, 1, 2][(rng.next() % 4) as usize],
        );
        put(
            a,
            b,
            g(a, 0x0133_c94c),
            [0x0001_0000u32, 0][(rng.next() % 2) as usize],
        );
        put(
            a,
            b,
            g(a, 0x0133_d168),
            [0xffff_ffffu32, 0][(rng.next() % 2) as usize],
        );
        put(a, b, g(a, 0x0133_f0d8), rng.next() % 5);
        let chr = [0x11u32, 0x12, rng.next() % 0x20][(rng.next() % 3) as usize];
        let command = [0xfeu32, 0xfffe, 0xffff, rng.next() % 0x4000][(rng.next() % 4) as usize];
        assert_eq!(
            eax(a, 0x0089_a330, &[chr, command]),
            t::to_btl_set_next_action(b, chr, command, scratch, call),
            "next action {k}"
        );
        let v: u32 = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory("next action", a, b);
    }
}

#[test]
fn round21_btl_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round21_btl(&mut a, &mut b, &mut Xorshift(0x4021_0051));
}

#[test]
fn round21_btl_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round21_btl(&mut a, &mut b, &mut Xorshift(0x4021_0052));
}

/// Round 22's battle font layout over the original glyph records and pen
/// advance, with the packet writers stand-ins that log the pen position,
/// colour and scale they are given.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round22_layout(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let mut off = 0x40u32;
    let mut stand = |a: &mut Oracle, b: &mut Oracle, sym: u32, code: &[u8]| {
        let s = stand_in8(a, b, off, code);
        redirect(a, b, sym, s);
        // room for the eight-word logger
        off += 0x60;
    };
    // the packet writers: log the pen (+4, +8), the scale and the colour
    let mut pen = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x15];
    pen.extend_from_slice(&cursor.to_le_bytes());
    pen.extend_from_slice(&[
        0x8b, 0x48, 0x04, 0x89, 0x0a, 0x8b, 0x48, 0x08, 0x89, 0x4a, 0x04, 0x8b, 0x4c, 0x24, 0x0c,
        0x89, 0x4a, 0x08, 0x8b, 0x08, 0x89, 0x4a, 0x0c, 0x83, 0x05,
    ]);
    pen.extend_from_slice(&cursor.to_le_bytes());
    pen.extend_from_slice(&[0x10, 0xc3]);
    for sym in [0x008f_bae0u32, 0x008f_b940, 0x008f_ae40] {
        stand(a, b, sym, &pen);
    }
    stand(a, b, 0x0064_3af0, &logger(cursor, 0x2201_0001));
    stand(a, b, 0x0064_0dd0, &logger(cursor, 0x2201_0002));
    stand(a, b, 0x008b_1350, &logger_args(cursor, 2));
    stand(a, b, 0x008b_1330, &marker(cursor, 0x2201_0003));
    stand(a, b, 0x008f_8ee0, &logger_args(cursor, 8));
    stand(a, b, 0x008f_a800, &[0xdb, 0x44, 0x24, 0x04, 0xc3]);
    // the language, English flag, width table and join test from cells
    for (i, sym) in [0x008a_c2f0u32, 0x0078_51d0, 0x008a_c3d0, 0x008b_5760]
        .into_iter()
        .enumerate()
    {
        let mut v = vec![0xa1];
        v.extend_from_slice(&(cells + 4 * i as u32).to_le_bytes());
        v.push(0xc3);
        stand(a, b, sym, &v);
    }
    let table = alloc(a, b, 0x2000);
    let widths: Vec<u8> = (0..0x2000).map(|_| (rng.next() % 0x60) as u8).collect();
    a.write(table, &widths).unwrap();
    b.write(table, &widths).unwrap();
    let text = alloc(a, b, 0x100);
    let scratch = alloc(a, b, 0x24);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    for k in 0..600u32 {
        put(a, b, cursor, log);
        put(a, b, cells, [0u32, 3][(rng.next() % 2) as usize]);
        put(a, b, cells + 4, rng.next() % 2);
        put(a, b, cells + 8, table + 0x1000);
        put(a, b, cells + 12, rng.next() % 2);
        put(
            a,
            b,
            g(a, 0x0187_16bc),
            u32::from(rng.next().is_multiple_of(3)),
        );
        let mut bytes = vec![0u8; 0x100];
        let mut i = 0usize;
        for _ in 0..rng.next() % 0x14 {
            match rng.next() % 8 {
                0 => bytes[i] = (1 + rng.next() % 3) as u8,
                1 => {
                    bytes[i] = 0xb;
                    i += 1;
                    bytes[i] = [0x4au8, 0x30, 0x41, 0x60][(rng.next() % 4) as usize];
                }
                2 => {
                    bytes[i] = 7;
                    i += 1;
                    bytes[i] = 0xe2;
                }
                _ => bytes[i] = (0x30 + rng.next() % 0x60) as u8,
            }
            i += 1;
        }
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        let x = ((rng.next() % 6000) as f32 / 10.0).to_bits();
        let y = ((rng.next() % 4000) as f32 / 10.0).to_bits();
        let alpha = rng.next() % 0x100;
        let scale = [1.0f32, 0.75, 1.25][(rng.next() % 3) as usize].to_bits();
        let extra = [1.0f32, 0.5][(rng.next() % 2) as usize].to_bits();
        let edge = rng.next() % 0x100;
        let value = [
            rng.next() % 10,
            rng.next() % 100_000,
            rng.next() % 1_000_000_000,
            0,
        ][(rng.next() % 4) as usize];
        let digits = rng.next() % 9;
        // the icon mode is set by the callers; seed it
        put(a, b, g(a, 0x025d_09ba), (rng.next() % 4) as u8);
        if k % 9 == 8 {
            eax(a, 0x0090_5660, &[value, x, y, alpha, scale, extra]);
            t::to_make_btl_easy_digit_right_abmap(
                b,
                [value, x, y, alpha, scale, extra],
                scratch,
                call,
            );
            let v: [u32; 8] = Mem::read(b, scratch);
            Mem::write(a, scratch, v);
            let v: u32 = Mem::read(b, scratch + 0x20);
            Mem::write(a, scratch + 0x20, v);
            same_memory("battle digit right", a, b);
            continue;
        }
        match k % 8 {
            0 => {
                eax(a, 0x0090_5ab0, &[text, x, y, alpha, scale]);
                t::to_make_btl_easy_font(b, [text, x, y, alpha, scale], call, fcall);
            }
            1 => {
                eax(a, 0x0090_5970, &[text, x, y, alpha, scale, extra, edge]);
                t::to_make_btl_easy_edge_font_a(b, [text, x, y, alpha, scale, extra, edge], call);
            }
            2 => {
                eax(a, 0x0090_5930, &[text, x, y, alpha, scale, extra]);
                t::to_make_btl_easy_edge_font(b, [text, x, y, alpha, scale, extra], call);
            }
            3 => {
                eax(a, 0x0090_5440, &[digits, value, x, y, alpha, scale]);
                t::to_make_btl_easy_digit(b, digits, value, x, y, alpha, scale, scratch, call);
            }
            4 => {
                eax(a, 0x0090_5550, &[value, x, y, alpha, scale, extra]);
                t::to_make_btl_easy_digit2(b, [value, x, y, alpha, scale, extra], call);
            }
            5 => {
                eax(a, 0x0090_55c0, &[value, x, y, alpha, scale, extra]);
                t::to_make_btl_easy_digit_right(
                    b,
                    [value, x, y, alpha, scale, extra],
                    scratch,
                    call,
                );
            }
            6 => {
                eax(a, 0x0090_57c0, &[digits, value, x, y, alpha, scale, extra]);
                t::to_make_btl_easy_edge_digit(
                    b,
                    digits,
                    [value, x, y, alpha, scale, extra],
                    scratch,
                    call,
                );
            }
            _ => {
                eax(a, 0x0090_5820, &[value, x, y, alpha, scale, extra]);
                t::to_make_btl_easy_edge_digit2(b, [value, x, y, alpha, scale, extra], call);
            }
        }
        let v: [u32; 8] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory("battle font layout", a, b);
    }
}

#[test]
fn round22_layout_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round22_layout(&mut a, &mut b, &mut Xorshift(0x4022_0011));
}

#[test]
fn round22_layout_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round22_layout(&mut a, &mut b, &mut Xorshift(0x4022_0012));
}

/// Round 22's battle glyph quads with `TOGetImageWH` from cells, the
/// original texture names and scissor, and a `graphicFontDraw` stand-in
/// that logs the record words the writers set and its arguments.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::similar_names
)]
fn round22_quads(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    // TOGetTextureName logs its four values; BeginScissor_XYUV keeps the
    // quad when cells[2] says so
    let s = stand_in8(a, b, 0xc0, &logger_args(cursor, 4));
    redirect(a, b, 0x008a_cf50, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x100, &v);
    redirect(a, b, 0x008e_5a20, s);
    // TOGetImageWH(page, &w, &h): w, h from cells
    let mut v = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d];
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d]);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0xc3]);
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x008a_c400, s);
    let mut v = vec![0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c, 0x8b, 0x3d];
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[
        0xb9, 0x10, 0x00, 0x00, 0x00, 0xf3, 0xa5, 0x8b, 0x46, 0x40, 0xab, 0x8b, 0x46, 0x48, 0xab,
        0x8b, 0x44, 0x24, 0x10, 0xab, 0x8b, 0x44, 0x24, 0x14, 0xab, 0x8b, 0x44, 0x24, 0x18, 0xab,
        0x8b, 0x44, 0x24, 0x1c, 0xab, 0x89, 0x3d,
    ]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x5f, 0x5e, 0xc3]);
    let s = stand_in8(a, b, 0x80, &v);
    redirect(a, b, 0x0064_0660, s);
    let pen = alloc(a, b, 0x20);
    let glyph = alloc(a, b, 0x40);
    let scratch = alloc(a, b, 0xb0);
    // graphicFontGetEdgeRatio: cells[3]
    let mut v = vec![0xd9, 0x05];
    v.extend_from_slice(&(cells + 12).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x140, &v);
    redirect(a, b, 0x0064_0d70, s);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for _ in 0..2000u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells,
            [
                [256.0f32, 512.0, 1024.0][(rng.next() % 3) as usize].to_bits(),
                [256.0f32, 512.0][(rng.next() % 2) as usize].to_bits(),
            ],
        );
        put(
            a,
            b,
            cells + 12,
            [1.0f32, 1.5, 0.7, 3.0][(rng.next() % 4) as usize],
        );
        put(a, b, cells + 8, u32::from(!rng.next().is_multiple_of(5)));
        put(a, b, pen, rng.next());
        put(
            a,
            b,
            pen + 4,
            [
                ((rng.next() % 7000) as f32 / 10.0 - 50.0).to_bits(),
                ((rng.next() % 5000) as f32 / 10.0 - 30.0).to_bits(),
            ],
        );
        put(a, b, pen + 0x1b, rng.next() as u8);
        let page = [0x3c00u16, 0x3d00, 0x3d40, 0x3d60][(rng.next() % 4) as usize];
        put(a, b, glyph, [page, 0x14, (rng.next() % 2) as u16, 0]);
        let cell: [f32; 7] = [
            (rng.next() % 0x200) as f32,
            (rng.next() % 0x200) as f32,
            (rng.next() % 0x200) as f32,
            (rng.next() % 0x200) as f32,
            [-1.0f32, 0.0, 1.0][(rng.next() % 3) as usize],
            (rng.next() % 80) as f32 / 4.0,
            18.0,
        ];
        put(a, b, glyph + 8, cell);
        let kx = [1.0f32, 1.5, 0.75][(rng.next() % 3) as usize];
        let ky = [1.0f32, 1.125, 0.8][(rng.next() % 3) as usize];
        put(a, b, g(a, 0x00c5_d778), [kx, ky]);
        put(
            a,
            b,
            g(a, 0x0187_16e0),
            [(rng.next() % 5) as f32 - 2.0, (rng.next() % 5) as f32 - 2.0],
        );
        let scale = [1.0f32, 0.75, 1.3, 0.6][(rng.next() % 4) as usize].to_bits();
        let kind = rng.next() % 3;
        let (shadow, edge) = (kind == 1, kind == 2);
        eax(
            a,
            [0x008f_b940, 0x008f_bae0, 0x008f_ae40][kind as usize],
            &[pen, glyph, scale],
        );
        t::to_make_pkt_btl_easy_font(b, pen, glyph, scale, shadow, edge, scratch, call, fcall);
        let v: [u32; 0x2c] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory("glyph quad", a, b);
    }
}

#[test]
fn round22_quads_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round22_quads(&mut a, &mut b, &mut Xorshift(0x4022_0021));
}

#[test]
fn round22_quads_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round22_quads(&mut a, &mut b, &mut Xorshift(0x4022_0022));
}

/// Round 22's button icon choice over logging stand-ins for the icon and
/// shape drawers.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn round22_icons(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x800);
    let cursor = alloc(a, b, 4);
    let assign = alloc(a, b, 4);
    let s = stand_in8(a, b, 0x40, &logger_args(cursor, 6));
    redirect(a, b, 0x008e_7a90, s);
    let s = stand_in8(a, b, 0xc0, &logger_args(cursor, 11));
    redirect(a, b, 0x0090_3ee0, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&assign.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x140, &v);
    redirect(a, b, 0x008a_cf30, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for c in 0..0x100u32 {
        for k in 0..4u32 {
            put(a, b, cursor, log);
            put(a, b, assign, k % 2);
            let args = [
                c,
                ((rng.next() % 6000) as f32 / 10.0).to_bits(),
                ((rng.next() % 4000) as f32 / 10.0 + 0.05).to_bits(),
                [1.0f32, 0.65, 0.73][(rng.next() % 3) as usize].to_bits(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
            ];
            eax(a, 0x008f_8ee0, &args);
            t::to_draw_scale_button_icon_rgba(b, args, call);
            same_memory("button icon", a, b);
        }
    }
}

#[test]
fn round22_icons_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round22_icons(&mut a, &mut b, &mut Xorshift(0x4022_0031));
}

#[test]
fn round22_icons_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round22_icons(&mut a, &mut b, &mut Xorshift(0x4022_0032));
}

/// Round 22's message word count, size and centring over stand-ins for
/// the word count and message size.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]
fn round22_message(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x800);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 8);
    // TOGetChangeMacroWordCount: logs (window, code), returns window ^ key
    let s = stand_in8(a, b, 0x40, &logger_args(cursor, 2));
    redirect(a, b, 0x008b_6b00, s);
    // TOGetMessageWH: logs its scalar words, *w = cells[0], *h = cells[1]
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[
        0x8b, 0x44, 0x24, 0x04, 0x89, 0x01, 0x8b, 0x44, 0x24, 0x14, 0x89, 0x41, 0x04, 0x8b, 0x44,
        0x24, 0x18, 0x89, 0x41, 0x08, 0x83, 0x05,
    ]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x0c, 0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d]);
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0x8b, 0x44, 0x24, 0x10, 0x8b, 0x0d]);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.extend_from_slice(&[0x66, 0x89, 0x08, 0xc3]);
    let s = stand_in8(a, b, 0x80, &v);
    redirect(a, b, 0x008b_9650, s);
    let text = alloc(a, b, 0x100);
    let window = alloc(a, b, 0x60);
    let out = alloc(a, b, 0x10);
    let scratch = alloc(a, b, 8);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..1000u32 {
        put(a, b, cursor, log);
        let mut bytes = vec![0u8; 0x100];
        let mut i = 0usize;
        for _ in 0..rng.next() % 0x18 {
            let c =
                [3u8, 5, 7, 9, 0xb, 0x10, 0x12, 0x20, 0x31, 0x45, 0x80][(rng.next() % 11) as usize];
            bytes[i] = c;
            i += 1;
            if c < 0x30 {
                bytes[i] = [0x50u8, 0x5f, 0x60, 0xc0, 0xbf, 0x31][(rng.next() % 6) as usize];
                i += 1;
            }
        }
        bytes[i] = (rng.next() % 3) as u8;
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        let w = rng.next() % 4;
        assert_eq!(
            eax(a, 0x008b_80b0, &[w, text]),
            t::count_message_words(b, w, text, call),
            "words {k}"
        );
        // a negative limit never ends the walk, in the original too
        let limit =
            [0u32, 2, 3, 0x2f, 0x30, 0x45, 0xff, rng.next() % 0x100][(rng.next() % 8) as usize];
        assert_eq!(
            eax(a, 0x008b_8140, &[w, text, limit]),
            t::count_message_words_above(b, w, text, limit, call),
            "words above {limit:#x} {k}"
        );
        put(
            a,
            b,
            cells,
            [((rng.next() % 6000) as f32 / 7.0).to_bits(), rng.next()],
        );
        put(a, b, out, [rng.next(), rng.next()]);
        assert_eq!(
            eax(a, 0x008b_7130, &[text, out, out + 4]),
            t::message_size(b, text, out, out + 4, scratch, call),
            "size {k}"
        );
        let v: [u32; 2] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory("message size", a, b);
        let wide = match rng.next() % 6 {
            0 => f32::from_bits(rng.next() & 0xffbf_ffff | 0x0040_0000), // bit 22 set: NaNs quiet
            1 => 3.0e9,
            2 => -0.75,
            _ => (rng.next() % 600_000) as f32 / 7.0 - 1000.0,
        };
        put(a, b, cells, [wide.to_bits(), rng.next()]);
        assert_eq!(
            eax(a, 0x008b_7170, &[text]),
            t::message_size_packed(b, text, scratch, call),
            "packed size {k}"
        );
        let v: [u32; 2] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory("packed message size", a, b);
        let edges = [
            (rng.next() % 300) as f32 / 3.0,
            (rng.next() % 600) as f32 / 1.5,
        ];
        put(a, b, window + 0x48, edges[0]);
        put(a, b, window + 0x50, edges[1]);
        put(a, b, window + 4, (rng.next() % 8) as u16);
        put(a, b, window + 0x1a, (rng.next() % 4) as u16);
        put(a, b, out, [rng.next(), rng.next()]);
        assert_eq!(
            eax(a, 0x008d_f520, &[window, text, out]),
            t::to_first_init_message_normal(b, window, text, out, scratch, call),
            "centre {k}"
        );
        let v: [u32; 2] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory("message centre", a, b);
    }
}

#[test]
fn round22_message_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round22_message(&mut a, &mut b, &mut Xorshift(0x4022_0041));
}

#[test]
fn round22_message_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round22_message(&mut a, &mut b, &mut Xorshift(0x4022_0042));
}

/// Round 22's message window texts over cells for the language swap,
/// English and bit test, with a logging window function.
#[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
fn round22_window_texts(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0xc);
    for (i, sym) in [0x008a_c390u32, 0x0078_51d0, 0x0078_51b0]
        .into_iter()
        .enumerate()
    {
        let mut v = vec![0xa1];
        v.extend_from_slice(&(cells + 4 * i as u32).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0x40 + 0x10 * i as u32, &v);
        redirect(a, b, sym, s);
    }
    let func = stand_in8(a, b, 0x80, &logger(cursor, 0x2202_0001));
    let windows = alloc(a, b, 4 * 0x100);
    for w in 0..4u32 {
        put(a, b, g(a, 0x0186_76f0) + 4 * w, windows + 0x100 * w);
    }
    let scratch = alloc(a, b, 0x420);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..400u32 {
        put(a, b, cursor, log);
        for c in 0..3u32 {
            put(a, b, cells + 4 * c, rng.next() % 3);
        }
        put(a, b, g(a, 0x025d_16bc), rng.next() % 2);
        put(a, b, g(a, 0x00c5_9558), rng.next() % 2);
        let window = rng.next() % 4;
        let bytes: Vec<u8> = (0..0x100).map(|_| rng.next() as u8).collect();
        a.write(windows + 0x100 * window, &bytes).unwrap();
        b.write(windows + 0x100 * window, &bytes).unwrap();
        put(a, b, windows + 0x100 * window + 0xf4, func);
        let (text, other) = (rng.next(), rng.next());
        eax(a, 0x008b_89d0, &[window, text, other]);
        t::set_message_window_texts(b, window, text, other, scratch, call, |m, f, w| {
            m.call(f, Conv::Cdecl, &[w]).unwrap().eax
        });
        same_memory(&format!("window texts {k}"), a, b);
    }
}

#[test]
fn round22_window_texts_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round22_window_texts(&mut a, &mut b, &mut Xorshift(0x4022_0051));
}

#[test]
fn round22_window_texts_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round22_window_texts(&mut a, &mut b, &mut Xorshift(0x4022_0052));
}

/// Round 23's UI shape quads with the image size and scissor from cells,
/// a logging texture-name stand-in and a record-logging draw.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn round23_shapes(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let mut v = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d];
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d]);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0xc3]);
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x008a_c400, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x80, &v);
    redirect(a, b, 0x008e_5a20, s);
    let s = stand_in8(a, b, 0xa0, &logger(cursor, 0x2301_0001));
    redirect(a, b, 0x008a_c8c0, s);
    let s = stand_in8(a, b, 0xe0, &record_logger(cursor));
    redirect(a, b, 0x0063_eee0, s);
    let scratch = alloc(a, b, 0xd0);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let shapes = [
        0xffff_ffffu32,
        0xffff_fffe,
        0xffff_fffd,
        0xffff_fffc,
        0xffff_fffa,
        0xffff_fff9,
        0,
        0xc6,
        0xc8,
        0x18e,
        0x190,
        0x256,
        0x258,
        0x289,
    ];
    for k in 0..3000u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells,
            [
                [256.0f32, 512.0, 1024.0][(rng.next() % 3) as usize],
                [128.0f32, 512.0][(rng.next() % 2) as usize],
            ],
        );
        put(a, b, cells + 8, u32::from(!rng.next().is_multiple_of(6)));
        let shape = shapes[(rng.next() % shapes.len() as u32) as usize];
        let fl = |rng: &mut Xorshift, m: u32, d: f32| ((rng.next() % m) as f32 / d).to_bits();
        let args = [
            shape,
            fl(rng, 6000, 10.0),
            fl(rng, 4000, 10.0),
            fl(rng, 3000, 10.0),
            fl(rng, 2000, 10.0),
            fl(rng, 1024, 1.0),
            fl(rng, 1024, 1.0),
            fl(rng, 1024, 1.0),
            fl(rng, 1024, 1.0),
        ];
        let c2 = k % 2 == 1;
        let colours = [rng.next(), rng.next()];
        let mut words = args.to_vec();
        if c2 {
            words.extend_from_slice(&colours);
        }
        eax(a, if c2 { 0x0090_3ee0 } else { 0x0090_3bb0 }, &words);
        t::to_mkp_shape_xywhuv(b, args, c2.then_some(colours), scratch, call);
        let v: [u32; 0x34] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        // the texture of unknown shapes is a stale stack word in the
        // original: compare the logs only where both are known
        let unknown = matches!(shape, 0xffff_fffa | 0xffff_fff9) || (shape == 0xffff_fffc && !c2);
        if unknown {
            let ours: [u32; 0x200] = Mem::read(b, log);
            Mem::write(a, log, ours);
        }
        same_memory(&format!("shape quad {k}: {shape:#x}"), a, b);
    }
}

#[test]
fn round23_shapes_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round23_shapes(&mut a, &mut b, &mut Xorshift(0x4023_0011));
}

#[test]
fn round23_shapes_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round23_shapes(&mut a, &mut b, &mut Xorshift(0x4023_0012));
}

/// Round 23's menu plates over the original remaps, with logging
/// stand-ins for the shape, frame and panel calls.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn round23_plates(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::plate::bridge as p;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let abmap = alloc(a, b, 4);
    for (i, (sym, n)) in [
        (0x0090_3ee0u32, 11u8),
        (0x0090_3bb0, 9),
        (0x008f_4b20, 6),
        (0x008f_9c20, 5),
        (0x008f_4df0, 6),
    ]
    .into_iter()
    .enumerate()
    {
        let s = stand_in8(a, b, 0x40 + 0x80 * i as u32, &logger_args(cursor, n));
        redirect(a, b, sym, s);
    }
    let mut v = vec![0xa1];
    v.extend_from_slice(&abmap.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x300, &v);
    redirect(a, b, 0x0064_1060, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    for k in 0..600u32 {
        put(a, b, cursor, log);
        put(a, b, abmap, rng.next() % 2);
        let kind = k % 16;
        let fl = |rng: &mut Xorshift, m: u32, d: f32| ((rng.next() % m) as f32 / d).to_bits();
        let args = [
            fl(rng, 6000, 10.0),
            fl(rng, 4000, 10.0),
            fl(rng, 6000, 7.0),
            fl(rng, 3000, 7.0),
            kind,
        ];
        eax(a, 0x008f_5f70, &args);
        p::to_draw_menu_plate_xywh_type(b, args, remap, call);
        same_memory(&format!("menu plate {k} kind {kind}"), a, b);
    }
}

#[test]
fn round23_plates_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round23_plates(&mut a, &mut b, &mut Xorshift(0x4023_0021));
}

#[test]
fn round23_plates_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round23_plates(&mut a, &mut b, &mut Xorshift(0x4023_0022));
}

/// Round 24's gradient box and sheet cell over cells for the scissor and
/// image size and a logging draw.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
fn round24_boxes(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let mut v = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d];
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d]);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0xc3]);
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x008a_c400, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x80, &v);
    redirect(a, b, 0x008e_58e0, s);
    let s = stand_in8(a, b, 0xa0, &logger(cursor, 0x2401_0001));
    redirect(a, b, 0x008a_c8c0, s);
    let box_words: Vec<u8> = vec![
        0, 4, 0x10, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x30, 0x34, 0x38, 0x3c, 0x88,
    ];
    let cell_words: Vec<u8> = (0..16u8).map(|k| 4 * k).chain([0x88]).collect();
    let box_logger = stand_in8(a, b, 0xe0, &record_words_logger(cursor, &box_words));
    let cell_logger = stand_in8(a, b, 0x1a0, &record_words_logger(cursor, &cell_words));
    let int_words: Vec<u8> = vec![
        0, 4, 0x10, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x30, 0x34, 0x38, 0x3c, 0x88, 0x90, 0x94,
    ];
    let int_logger = stand_in8(a, b, 0x280, &record_words_logger(cursor, &int_words));
    let abmap = alloc(a, b, 4);
    let mut v = vec![0xa1];
    v.extend_from_slice(&abmap.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x360, &v);
    redirect(a, b, 0x0064_1060, s);
    let scratch = alloc(a, b, 0xa0);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fl = |rng: &mut Xorshift, m: u32, d: f32| ((rng.next() % m) as f32 / d).to_bits();
    for k in 0..2000u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells,
            [
                [256.0f32, 512.0, 1024.0][(rng.next() % 3) as usize],
                [128.0f32, 256.0][(rng.next() % 2) as usize],
            ],
        );
        put(a, b, cells + 8, u32::from(!rng.next().is_multiple_of(5)));
        let rect = [
            fl(rng, 6000, 7.0),
            fl(rng, 4000, 7.0),
            fl(rng, 6000, 3.0),
            fl(rng, 4000, 3.0),
        ];
        if k % 4 == 3 {
            // the integer boxes
            redirect(a, b, 0x0063_eee0, int_logger);
            let rect = [
                rng.next() % 640,
                rng.next() % 448,
                rng.next() % 640,
                (rng.next() % 448).wrapping_sub(20),
            ];
            put(a, b, abmap, rng.next() % 3);
            if k % 8 == 3 {
                let args = [rect[0], rect[1], rect[2], rect[3], rng.next(), rng.next()];
                eax(a, 0x008f_3db0, &args);
                t::to_draw_box_xywh_c2(b, args, scratch, call);
            } else {
                let args = [
                    rect[0],
                    rect[1],
                    rect[2],
                    rect[3],
                    rng.next(),
                    rng.next(),
                    rng.next(),
                    rng.next(),
                ];
                eax(a, 0x008f_b150, &args);
                t::to_make_pkt_box_xywh_rgba(b, args, scratch, call);
            }
        } else if k % 2 == 0 {
            redirect(a, b, 0x0063_eee0, box_logger);
            let args = [rect[0], rect[1], rect[2], rect[3], rng.next(), rng.next()];
            eax(a, 0x008f_4b20, &args);
            t::to_draw_cross_box_xywh_c2(b, args, scratch, call);
        } else {
            redirect(a, b, 0x0063_eee0, cell_logger);
            let index = if rng.next().is_multiple_of(8) {
                rng.next()
            } else {
                rng.next() % 60
            };
            let args = [index, rect[0], rect[1], rect[2], rect[3]];
            eax(a, 0x008f_8bb0, &args);
            t::draw_sheet_cell(b, args, scratch, call);
        }
        let v: [u32; 0x28] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("box {k}"), a, b);
    }
}

#[test]
fn round24_boxes_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round24_boxes(&mut a, &mut b, &mut Xorshift(0x4024_0011));
}

#[test]
fn round24_boxes_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round24_boxes(&mut a, &mut b, &mut Xorshift(0x4024_0012));
}

/// Round 25's menu background and message-font wrapper over logging
/// stand-ins.
fn round25_wrappers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let s = stand_in8(a, b, 0x40, &logger_args(cursor, 9));
    redirect(a, b, 0x0090_3bb0, s);
    let s = stand_in8(a, b, 0xc0, &logger_args(cursor, 12));
    redirect(a, b, 0x0090_1700, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..50u32 {
        put(a, b, cursor, log);
        eax(a, 0x008f_5c10, &[]);
        t::to_draw_menu_bg(b, call);
        let args: [u32; 7] = std::array::from_fn(|i| {
            if (2..4).contains(&i) || i >= 5 {
                (rng.next() % 4000) * 0x0001_0000 + 0x4000_0000
            } else {
                rng.next()
            }
        });
        eax(a, 0x0090_16b0, &args);
        t::to_mkp_cross_ext_mes_font_l_clut(b, args, call);
        same_memory(&format!("wrappers {k}"), a, b);
    }
}

/// Round 25's 16 × 12 glyph passes with `TOGetImageWH` from cells, a
/// texture-name logger, a scissor that keeps the quad when cells[2] says
/// so and a `graphicFontDraw` stand-in that logs every record word the
/// writers set and its arguments.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names
)]
fn round25_font_1612_passes(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge::{self as t, Font1612Pass};
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let log = alloc(a, b, 0x2000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let s = stand_in8(a, b, 0xc0, &logger_args(cursor, 4));
    redirect(a, b, 0x008a_cf50, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x100, &v);
    redirect(a, b, 0x008e_5a20, s);
    // TOGetImageWH(page, &w, &h) logs the page and takes w, h from cells
    let mut v = logger(cursor, 0x5a5a);
    v.pop();
    v.extend_from_slice(&[0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d]);
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d]);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0xc3]);
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x008a_c400, s);
    let words: Vec<u8> = (0..12u8)
        .map(|k| 4 * k)
        .chain([0x20 + 0x10, 0x3c, 0x80, 0x88])
        .collect();
    let s = stand_in8(a, b, 0x200, &record_words_logger(cursor, &words));
    redirect(a, b, 0x0064_0660, s);
    let mut v = vec![0xd9, 0x05];
    v.extend_from_slice(&(cells + 12).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x140, &v);
    redirect(a, b, 0x0064_0d70, s);
    let pen = alloc(a, b, 0x20);
    let glyph = alloc(a, b, 0x40);
    let scratch = alloc(a, b, 0xb0);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for _ in 0..3000u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells,
            [
                [256.0f32, 512.0, 1024.0][(rng.next() % 3) as usize].to_bits(),
                [256.0f32, 512.0][(rng.next() % 2) as usize].to_bits(),
            ],
        );
        put(
            a,
            b,
            cells + 12,
            [1.0f32, 1.5, 0.7, 3.0][(rng.next() % 4) as usize],
        );
        put(a, b, cells + 8, u32::from(!rng.next().is_multiple_of(5)));
        put(a, b, pen, rng.next());
        put(
            a,
            b,
            pen + 4,
            [
                ((rng.next() % 7000) as f32 / 10.0 - 50.0).to_bits(),
                ((rng.next() % 5000) as f32 / 10.0 - 30.0).to_bits(),
            ],
        );
        put(a, b, pen + 0x1a, [(rng.next() % 5) as u8, rng.next() as u8]);
        let page = [0x3c00u16, 0x3d00, 0x8d40, 0xfd60][(rng.next() % 4) as usize];
        put(
            a,
            b,
            glyph,
            [page, 0x14, [0u16, 1, 0xfffe][(rng.next() % 3) as usize], 0],
        );
        let cell: [f32; 7] = [
            (rng.next() % 0x200) as f32,
            (rng.next() % 0x200) as f32,
            (rng.next() % 0x200) as f32,
            (rng.next() % 0x200) as f32,
            [-1.0f32, 0.0, 1.0, 0.3][(rng.next() % 4) as usize],
            (rng.next() % 80) as f32 / 4.0 + 0.1,
            [
                12.0f32,
                16.0,
                13.7,
                f32::from_bits(0x4140_0000 | (rng.next() & 0x7f_ffff)),
            ][(rng.next() % 4) as usize],
        ];
        put(a, b, glyph + 8, cell);
        let kx = [1.0f32, 1.5, 0.75, 1.1][(rng.next() % 4) as usize];
        let ky = [1.0f32, 1.125, 0.8, 0.9][(rng.next() % 4) as usize];
        put(a, b, g(a, 0x00c5_d778), [kx, ky]);
        put(
            a,
            b,
            g(a, 0x00c5_d784),
            [1.0f32, 1.25, 0.9][(rng.next() % 3) as usize],
        );
        put(
            a,
            b,
            g(a, 0x00b5_ee10),
            [
                0.78f32,
                0.7,
                f32::from_bits(0x3f00_0000 | (rng.next() & 0x7f_ffff)),
            ][(rng.next() % 3) as usize],
        );
        put(
            a,
            b,
            g(a, 0x00b9_2268),
            [2.0f32, 1.5, -1.0][(rng.next() % 3) as usize],
        );
        put(
            a,
            b,
            g(a, 0x00b4_1da4),
            [-2.0f32, -1.75][(rng.next() % 2) as usize],
        );
        put(
            a,
            b,
            g(a, 0x00b9_2340),
            [-1.0f32, -0.5][(rng.next() % 2) as usize],
        );
        put(
            a,
            b,
            g(a, 0x0187_16e0),
            [(rng.next() % 5) as f32 - 2.3, (rng.next() % 5) as f32 - 1.7],
        );
        let arg = [0u32, 1, 4, 5, 6][(rng.next() % 5) as usize];
        let (sym, pass, args) = match rng.next() % 5 {
            0 => (0x008f_c870, Font1612Pass::Edge, vec![pen, glyph]),
            3 => (0x008f_ce90, Font1612Pass::ItalicEdge, vec![pen, glyph]),
            4 => (0x008f_d0a0, Font1612Pass::ItalicFace, vec![pen, glyph]),
            1 => (
                0x008f_cca0,
                Font1612Pass::Shadow(arg),
                vec![pen, glyph, arg],
            ),
            _ => (0x008f_cad0, Font1612Pass::Face(arg), vec![pen, glyph, arg]),
        };
        eax(a, sym, &args);
        t::to_make_pkt_font_1612_pass(b, pen, glyph, pass, scratch, call, fcall);
        let v: [u32; 0x2c] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("glyph pass {pass:?}"), a, b);
    }
}

#[test]
fn round25_font_1612_passes_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round25_font_1612_passes(&mut a, &mut b, &mut Xorshift(0x4025_0021));
}

#[test]
fn round25_font_1612_passes_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round25_font_1612_passes(&mut a, &mut b, &mut Xorshift(0x4025_0022));
}

/// Round 25's `TOMakePktFont1612` dispatch and default passes over
/// stand-ins that log a key and their arguments.
fn round25_font_1612_dispatch(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let keyed = |cursor: u32, key: u32, n: u8| {
        let mut v = marker(cursor, key);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        v
    };
    let stand_ins: [(u32, u8); 8] = [
        (0x008f_c870, 2),
        (0x008f_cad0, 3),
        (0x008f_cca0, 3),
        (0x008f_ce90, 2),
        (0x008f_d0a0, 2),
        (0x0064_3af0, 1),
        (0x0064_0dd0, 1),
        (0x008e_8ed0, 3),
    ];
    for (k, (sym, n)) in (0u32..).zip(stand_ins) {
        let s = stand_in8(a, b, 0x40 + 0x60 * k, &keyed(cursor, 0x5100 + k, n));
        redirect(a, b, sym, s);
    }
    let pen = alloc(a, b, 0x20);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..200u32 {
        put(a, b, cursor, log);
        put(a, b, pen + 0x1a, (rng.next() % 5) as u8);
        put(a, b, g(a, 0x00c5_d784), rng.next() | 1);
        put(a, b, g(a, 0x00b5_ee10), rng.next() | 1);
        let glyph = rng.next();
        eax(a, 0x008f_c7c0, &[pen, glyph]);
        t::to_make_pkt_font_1612(b, pen, glyph, call);
        let shadow = rng.next().is_multiple_of(2);
        eax(
            a,
            [0x0090_6270, 0x0090_6290][usize::from(shadow)],
            &[pen, glyph],
        );
        t::to_make_pkt_font_1612_default(b, pen, glyph, shadow, call);
        same_memory(&format!("1612 dispatch {k}"), a, b);
    }
}

#[test]
fn round25_font_1612_dispatch_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round25_font_1612_dispatch(&mut a, &mut b, &mut Xorshift(0x4025_0031));
}

#[test]
fn round25_font_1612_dispatch_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round25_font_1612_dispatch(&mut a, &mut b, &mut Xorshift(0x4025_0032));
}

/// Round 25's cross-menu icon name rows over an entry lookup that hands
/// out cells, logging stand-ins for the icon and text drawers, and the
/// original remaps.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]
fn round25_icon_names(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge::{self as t, CrossMenuIconName};
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let entry = alloc(a, b, 0x80);
    let base = alloc(a, b, 4);
    let out = alloc(a, b, 4);
    // lookup(id, &base): logs id, stores *base, returns the entry
    let mut v = logger(cursor, 0x77);
    v.pop();
    v.extend_from_slice(&[0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d]);
    v.extend_from_slice(&base.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0xb8]);
    v.extend_from_slice(&entry.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x0079_0970, s);
    redirect(a, b, 0x0079_a4b0, s);
    let s = stand_in8(a, b, 0xc0, &logger_args(cursor, 9));
    redirect(a, b, 0x008e_6af0, s);
    let s = stand_in8(a, b, 0x140, &logger_args(cursor, 6));
    redirect(a, b, 0x0090_5ab0, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    for k in 0..400u32 {
        put(a, b, cursor, log);
        put(a, b, base, rng.next());
        let bytes: [u32; 0x20] = std::array::from_fn(|_| rng.next());
        put(a, b, entry, bytes);
        let command = k % 2 == 1;
        let kind = if command {
            CrossMenuIconName::Command
        } else {
            CrossMenuIconName::Ability
        };
        let id = rng.next() % 0x200;
        let x = ((rng.next() % 20000) as f32 / 10.0 - 100.0).to_bits();
        let y = ((rng.next() % 12000) as f32 / 10.0 - 100.0).to_bits();
        let alpha = rng.next() % 0x100;
        eax(
            a,
            [0x008f_4f10, 0x008f_5000][usize::from(command)],
            &[id, x, y, alpha],
        );
        t::to_draw_cross_menu_icon_name(b, kind, id, (x, y), alpha, out, call, remap);
        let stored: u32 = Mem::read(b, out);
        Mem::write(a, out, stored);
        same_memory(&format!("icon name {k}"), a, b);
    }
}

#[test]
fn round25_icon_names_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round25_icon_names(&mut a, &mut b, &mut Xorshift(0x4025_0041));
}

#[test]
fn round25_icon_names_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round25_icon_names(&mut a, &mut b, &mut Xorshift(0x4025_0042));
}

/// Round 25's turned shape quad with `fcos`/`fsin` in the C runtime's
/// slots, `TOGetImageWH` from cells, a logging texture-name stand-in, the
/// original remaps and a `graphicDrawUIElement` stand-in logging all four
/// vertices.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]
fn round25_rotate_shape(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 8);
    let scratch = alloc(a, b, 0xa0);
    for (slot, code, at) in [
        (0x00b0_c3b4u32, [0xd9u8, 0xff, 0xc3], 0x00b0_bfc0u32),
        (0x00b0_c3b8, [0xd9, 0xfe, 0xc3], 0x00b0_bfc8),
    ] {
        let at = a.rebase(at);
        a.write(at, &code).unwrap();
        b.write(at, &code).unwrap();
        let slot = a.rebase(slot);
        put(a, b, slot, at);
    }
    let s = stand_in8(a, b, 0x40, &logger_args(cursor, 1));
    redirect(a, b, 0x008a_c8c0, s);
    let mut v = logger(cursor, 0x3e3e);
    v.pop();
    v.extend_from_slice(&[0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d]);
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d]);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0xc3]);
    let s = stand_in8(a, b, 0x80, &v);
    redirect(a, b, 0x008a_c400, s);
    let words: Vec<u8> = (0..0x26u8).map(|k| 4 * k).collect();
    let s = stand_in8(a, b, 0x200, &record_words_logger(cursor, &words));
    redirect(a, b, 0x0063_eee0, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    let shapes = [
        u32::MAX,
        0,
        5,
        0xc6,
        0xc7,
        200,
        0x18e,
        0x18f,
        400,
        0x256,
        600,
        0x289,
        0x28a,
        0xffff_fffe,
    ];
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells,
            [
                [256.0f32, 512.0, 1024.0][(rng.next() % 3) as usize],
                [128.0f32, 512.0][(rng.next() % 2) as usize],
            ],
        );
        let fl = |rng: &mut Xorshift, m: u32, d: f32, o: f32| {
            ((rng.next() % m) as f32 / d - o).to_bits()
        };
        let shape = shapes[(rng.next() as usize) % shapes.len()];
        let angle = if k % 7 == 0 {
            0.0f32.to_bits()
        } else {
            fl(rng, 20000, 1000.0, 10.0)
        };
        let args = [
            shape,
            fl(rng, 6000, 3.0, 50.0),
            fl(rng, 4000, 3.0, 50.0),
            fl(rng, 3000, 7.0, 0.0),
            fl(rng, 3000, 7.0, 0.0),
            fl(rng, 0x400, 1.0, 0.0),
            fl(rng, 0x400, 1.0, 0.0),
            fl(rng, 0x400, 1.0, 0.0),
            fl(rng, 0x400, 1.0, 0.0),
            rng.next(),
            rng.next(),
            angle,
        ];
        let texture_known = shape == u32::MAX
            || shape <= 0xc6
            || (200..=0x289).contains(&shape)
                && !(0x18f..400).contains(&shape)
                && !(0x257..600).contains(&shape);
        if !texture_known {
            // the original passes a stale stack word as the texture
            continue;
        }
        eax(a, 0x0090_2ef0, &args);
        t::to_mkp_rotate_shape_xywhuv_c2(b, args, scratch, call, fcall, remap);
        let v: [u32; 0x28] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("rotate shape {k}"), a, b);
    }
}

#[test]
fn round25_rotate_shape_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round25_rotate_shape(&mut a, &mut b, &mut Xorshift(0x4025_0051));
}

#[test]
fn round25_rotate_shape_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round25_rotate_shape(&mut a, &mut b, &mut Xorshift(0x4025_0052));
}

/// Round 25's battle network window over logging shape stand-ins, a
/// clock from a cell, `fcos` in the C runtime's slot and the original
/// remaps, with every UV constant randomised.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]
fn round25_net_window(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let clock = alloc(a, b, 8);
    let slot = a.rebase(0x00b0_c3b4);
    let fcos = a.rebase(0x00b0_bfc0);
    a.write(fcos, &[0xd9u8, 0xff, 0xc3]).unwrap();
    b.write(fcos, &[0xd9u8, 0xff, 0xc3]).unwrap();
    put(a, b, slot, fcos);
    let s = stand_in8(a, b, 0x40, &logger_args(cursor, 9));
    redirect(a, b, 0x0090_3bb0, s);
    let s = stand_in8(a, b, 0xc0, &logger_args(cursor, 11));
    redirect(a, b, 0x0090_3ee0, s);
    let mut v = vec![0xdd, 0x05];
    v.extend_from_slice(&clock.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x180, &v);
    redirect(a, b, 0x0064_1410, s);
    // fld qword [esp+4]; call [slot]; ret
    let mut v = vec![0xdd, 0x44, 0x24, 0x04, 0xff, 0x15];
    v.extend_from_slice(&slot.to_le_bytes());
    v.push(0xc3);
    let cos_thunk = stand_in8(a, b, 0x1c0, &v);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let cos = |m: &mut Oracle, v: f64| {
        let bits = v.to_bits();
        m.call(cos_thunk, Conv::Cdecl, &[bits as u32, (bits >> 32) as u32])
            .unwrap()
            .st0
            .unwrap()
    };
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    let uv_constants = [
        0x00b6_6bf0u32,
        0x00b6_6bf4,
        0x00b6_6bfc,
        0x00b6_6c00,
        0x00b4_1cb8,
        0x00b6_6c1c,
        0x00b6_6c20,
        0x00b6_6c24,
        0x00b6_6c30,
        0x00b6_6914,
        0x00b5_ee18,
        0x00b6_6c38,
    ];
    for k in 0..600u32 {
        put(a, b, cursor, log);
        // k = 0: cos(3t) just under 0.5, which the f32 rounding of the
        // cosine lifts to 0.5 (alpha 112, not 111)
        let time = if k == 0 {
            (0.5f64.acos() + 1e-9 / 0.75f64.sqrt()) / 3.0
        } else {
            f64::from(rng.next() % 100_000) / 7.0
        };
        put(a, b, clock, time);
        for at in uv_constants {
            let at = a.rebase(at);
            put(
                a,
                b,
                at,
                ((rng.next() % 0x10000) as f32 / 65536.0).to_bits(),
            );
        }
        let fl = |rng: &mut Xorshift, m: u32, d: f32, o: f32| (rng.next() % m) as f32 / d - o;
        let x = fl(rng, 60000, 30.0, 50.0);
        let y = fl(rng, 40000, 30.0, 50.0);
        let w = fl(rng, 30000, 29.0, 0.0);
        let h = fl(rng, 30000, 31.0, 0.0);
        let kind = if k == 0 { 1 } else { rng.next() % 6 };
        let args = [x, y, w, h].map(f32::to_bits);
        eax(a, 0x008f_41b0, &[args[0], args[1], args[2], args[3], kind]);
        t::to_draw_btl_net_window_xywh_type(b, (x, y, w, h), kind, call, fcall, cos, remap);
        same_memory(&format!("net window {k}"), a, b);
    }
}

#[test]
fn round25_net_window_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round25_net_window(&mut a, &mut b, &mut Xorshift(0x4025_0061));
}

#[test]
fn round25_net_window_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round25_net_window(&mut a, &mut b, &mut Xorshift(0x4025_0062));
}

/// A stand-in that returns the f64 at `cell` in ST0.
fn st0_cell(cell: u32) -> Vec<u8> {
    let mut v = vec![0xdd, 0x05];
    v.extend_from_slice(&cell.to_le_bytes());
    v.push(0xc3);
    v
}

/// Round 26's cross-menu message writer and its macro helper over logging
/// stand-ins for the glyph drawers, a code skipper that steps 2 bytes over
/// codes below 0x30 and 1 otherwise, and the original `TOGetGroup` and
/// button-data functions.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]
fn round26_cross_text(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    let text = alloc(a, b, 0x80);
    let macro_text = alloc(a, b, 0x40);
    let keyed = |cursor: u32, key: u32, n: u8| {
        let mut v = marker(cursor, key);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        v
    };
    for (k, (sym, n)) in (0u32..).zip([
        (0x008f_ae40u32, 3u8),
        (0x008f_bae0, 3),
        (0x008f_b940, 3),
        (0x008e_8ed0, 3),
        (0x008f_8ee0, 8),
    ]) {
        let s = stand_in8(a, b, 0x40 + 0x80 * k, &keyed(cursor, 0x6100 + k, n));
        redirect(a, b, sym, s);
    }
    // TOGetFontInfo(p, rec): logs both, returns p + 1
    let mut v = keyed(cursor, 0x6200, 2);
    v.truncate(v.len() - 1);
    v.extend_from_slice(&[0x40, 0xc3]); // inc eax; ret
    let s = stand_in8(a, b, 0x300, &v);
    redirect(a, b, 0x008b_7240, s);
    // TOSkipCode(p): p + 2 below 0x30, else p + 1
    let skip = [
        0x8b, 0x44, 0x24, 0x04, 0x80, 0x38, 0x30, 0x72, 0x02, 0x40, 0xc3, 0x83, 0xc0, 0x02, 0xc3,
    ];
    let s = stand_in8(a, b, 0x380, &skip);
    redirect(a, b, 0x008d_fd80, s);
    // TOGetChangeMacroAddress(key, code): logs both, returns cells[0]
    let mut v = keyed(cursor, 0x6300, 2);
    v.truncate(v.len() - 5); // drop "mov eax, [esp+4]; ret"
    v.push(0xa1);
    v.extend_from_slice(&cells.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x3c0, &v);
    redirect(a, b, 0x008b_6660, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x440, &v);
    redirect(a, b, 0x008a_c2f0, s);
    let s = stand_in8(a, b, 0x460, &st0_cell(cells + 8));
    redirect(a, b, 0x008f_a800, s);
    let s = stand_in8(a, b, 0x480, &st0_cell(cells + 0x10));
    redirect(a, b, 0x008f_a570, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let pen = alloc(a, b, 0x20);
    let codes: [u8; 14] = [
        0x30, 0x41, 0x5a, 3, 7, 7, 0xa, 0xa, 0xb, 0xb, 0x12, 0x22, 0x23, 1,
    ];
    for k in 0..800u32 {
        put(a, b, cursor, log);
        let mut bytes = [0u8; 0x40];
        let n = (rng.next() % 30) as usize;
        let mut i = 0;
        while i + 2 < n {
            let c = codes[(rng.next() as usize) % codes.len()];
            bytes[i] = c;
            if c < 0x30 {
                bytes[i + 1] = [0xe0u8, 0xe1, 0xe7, 0xef, 0xf0, 0x45, 0x07, 0x00, 0x3f]
                    [(rng.next() % 9) as usize];
                if bytes[i + 1] == 0 {
                    bytes[i + 1] = 1;
                }
                i += 2;
            } else {
                i += 1;
            }
        }
        put(a, b, text, bytes);
        let mut mbytes = [0u8; 0x20];
        for (j, m) in mbytes
            .iter_mut()
            .take((rng.next() % 12) as usize)
            .enumerate()
        {
            *m = match j % 3 {
                1 => [7u8, 0x31][(rng.next() % 2) as usize],
                2 => [0xe0u8, 0xe5, 0xf0, 0x32][(rng.next() % 4) as usize],
                _ => 0x30 + (rng.next() % 40) as u8,
            };
        }
        put(a, b, macro_text, mbytes);
        put(
            a,
            b,
            cells,
            if rng.next().is_multiple_of(6) {
                0
            } else {
                macro_text
            },
        );
        put(a, b, cells + 4, rng.next() % 2);
        put(
            a,
            b,
            cells + 8,
            f64::from_bits(
                0x4030_0000_0000_0000
                    | (u64::from(rng.next()) << 20)
                    | u64::from(rng.next() & 0xf_ffff),
            ),
        );
        put(a, b, cells + 0x10, f64::from(rng.next() % 40) / 3.0);
        put(a, b, g(a, 0x0187_16bc), rng.next() % 3);
        let mut scale = [0.64f32, 0.6, 1.0, 0.73][(rng.next() % 4) as usize].to_bits();
        if k == 1 || k == 2 {
            // the battle AP screen's "[C]: Next" hint as the battle snapshot
            // holds it: button icon 0x31, then ": Next"; at a scale other
            // than 0.64 the icon is lifted by AdjustButtonIconY (five words)
            put(
                a,
                b,
                text,
                [0x0bu8, 0x31, 0x4a, 0x3a, 0x5d, 0x74, 0x87, 0x83, 0, 0],
            );
            scale = if k == 1 { 1.0f32 } else { 0.64 }.to_bits();
        }
        let x = if rng.next().is_multiple_of(5) {
            (-0.0f32).to_bits()
        } else {
            ((rng.next() % 6000) as f32 / 7.0).to_bits()
        };
        let y = ((rng.next() % 4000) as f32 / 7.0).to_bits();
        if k % 3 == 0 {
            put(a, b, pen, rng.next());
            put(a, b, pen + 4, [x, y]);
            let args = [
                rng.next() % 4,
                rng.next(),
                pen,
                scale,
                rng.next(),
                rng.next() % 2,
            ];
            let ra = eax(a, 0x008e_9ca0, &args);
            let rb = t::to_add_pkt_change_macro_help(b, args, call);
            assert_eq!(ra, rb, "macro help {k}");
        } else {
            let key = if rng.next().is_multiple_of(4) {
                u32::MAX
            } else {
                rng.next() % 4
            };
            let args = [
                key,
                text,
                x,
                y,
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                scale,
                rng.next(),
            ];
            eax(a, 0x0090_1700, &args);
            t::to_mkp_cross_ext_mes_font_l_clut_type_rgba(b, args, call, fcall);
        }
        let lang = rng.next() % 2;
        put(a, b, cells + 4, lang);
        let at = a.rebase(0x008e_5280);
        let ra = a
            .call(at, Conv::Cdecl, &[0, 0, 0, 0, 0])
            .unwrap()
            .st0
            .unwrap();
        let rb = t::adjust_button_icon_y(b, call);
        assert_eq!(ra.to_bits(), f64::from(rb).to_bits(), "icon y {k}");
        same_memory(&format!("cross text {k}"), a, b);
    }
}

#[test]
fn round26_cross_text_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round26_cross_text(&mut a, &mut b, &mut Xorshift(0x4026_0011));
}

#[test]
fn round26_cross_text_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round26_cross_text(&mut a, &mut b, &mut Xorshift(0x4026_0012));
}

/// Round 27's 16×12-font text writers: `TOAddPktBaseMesFontLAbmap`, its
/// macro helper `TOAddPktChangeMacro` and the macro width
/// `TOGetChangeMacroAdvance_008b6bb0`, over logging stand-ins for the glyph
/// drawers and the width helpers (as round 26) and the original
/// `TOGetGroup` and button-data functions.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]
fn round27_text(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x28);
    let inner_text = alloc(a, b, 0x20);
    let text = alloc(a, b, 0x80);
    let macro_text = alloc(a, b, 0x40);
    let keyed = |cursor: u32, key: u32, n: u8| {
        let mut v = marker(cursor, key);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        v
    };
    for (k, (sym, n)) in (0u32..).zip([
        (0x008f_cca0u32, 3u8),
        (0x008f_cad0, 3),
        (0x008e_8ed0, 3),
        (0x008f_c870, 2),
        (0x0090_6270, 2),
        (0x008f_8ee0, 8),
    ]) {
        let s = stand_in8(a, b, 0x40 + 0x80 * k, &keyed(cursor, 0x7100 + k, n));
        redirect(a, b, sym, s);
    }
    // TOGetFontInfo(p, rec): logs both, returns p + 1
    let mut v = keyed(cursor, 0x7200, 2);
    v.truncate(v.len() - 1);
    v.extend_from_slice(&[0x40, 0xc3]); // inc eax; ret
    let s = stand_in8(a, b, 0x340, &v);
    redirect(a, b, 0x008b_7240, s);
    // TOSkipCode(p): p + 2 below 0x30, else p + 1
    let skip = [
        0x8b, 0x44, 0x24, 0x04, 0x80, 0x38, 0x30, 0x72, 0x02, 0x40, 0xc3, 0x83, 0xc0, 0x02, 0xc3,
    ];
    let s = stand_in8(a, b, 0x3c0, &skip);
    redirect(a, b, 0x008d_fd80, s);
    // TOGetChangeMacroAddress(key, code): logs both, returns cells[0]
    // and moves cells[8] (the nested macro's text) there
    let mut v = keyed(cursor, 0x7300, 2);
    v.truncate(v.len() - 5); // drop "mov eax, [esp+4]; ret"
    v.push(0xa1);
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x8b, 0x0d]); // mov ecx, [cells + 0x20]
    v.extend_from_slice(&(cells + 0x20).to_le_bytes());
    v.extend_from_slice(&[0x89, 0x0d]); // mov [cells], ecx
    v.extend_from_slice(&cells.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x400, &v);
    redirect(a, b, 0x008b_6660, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x480, &v);
    redirect(a, b, 0x008a_c2f0, s);
    let s = stand_in8(a, b, 0x4a0, &st0_cell(cells + 8));
    redirect(a, b, 0x008f_a800, s);
    // the advance and the space: log their arguments, return a cell in ST0
    for (off, sym, n, cell, key) in [
        (0x4c0u32, 0x008f_a740u32, 3u8, cells + 0x10, 0x7400u32),
        (0x540, 0x008f_a570, 1, cells + 0x18, 0x7500),
    ] {
        let mut v = keyed(cursor, key, n);
        v.truncate(v.len() - 1);
        v.extend_from_slice(&st0_cell(cell));
        let s = stand_in8(a, b, off, &v);
        redirect(a, b, sym, s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let pen = alloc(a, b, 0x20);
    let codes: [u8; 14] = [
        0x30, 0x41, 0x5a, 3, 7, 7, 0xa, 0xa, 0xb, 0xb, 0x12, 0x22, 0x23, 1,
    ];
    for k in 0..900u32 {
        put(a, b, cursor, log);
        let mut bytes = [0u8; 0x40];
        let n = (rng.next() % 30) as usize;
        let mut i = 0;
        while i + 2 < n {
            let c = codes[(rng.next() as usize) % codes.len()];
            bytes[i] = c;
            if c < 0x30 {
                bytes[i + 1] = [0xe0u8, 0xe1, 0xe7, 0xef, 0xf0, 0x45, 0x07, 0x01, 0x3f]
                    [(rng.next() % 9) as usize];
                i += 2;
            } else {
                i += 1;
            }
        }
        put(a, b, text, bytes);
        let mut mbytes = [0u8; 0x20];
        for (j, m) in mbytes
            .iter_mut()
            .take((rng.next() % 12) as usize)
            .enumerate()
        {
            *m = match j % 3 {
                1 => [7u8, 0x31, 0x12][(rng.next() % 3) as usize],
                2 => [0xe0u8, 0xe1, 0xef, 0xf0, 0x32][(rng.next() % 5) as usize],
                _ => 0x30 + (rng.next() % 40) as u8,
            };
        }
        put(a, b, macro_text, mbytes);
        let inner: [u8; 8] = std::array::from_fn(|j| match j {
            7 => 0,
            _ => [0x30u8, 0x45, 7, 0xe3][(rng.next() % 4) as usize],
        });
        put(a, b, inner_text, inner);
        put(a, b, cells + 0x20, inner_text);
        let macro_at = if rng.next().is_multiple_of(6) {
            0
        } else {
            macro_text
        };
        put(a, b, cells, macro_at);
        put(a, b, cells + 4, rng.next() % 2);
        put(a, b, cells + 8, f64::from(rng.next() % 4000) / 7.0);
        put(a, b, cells + 0x10, f64::from(rng.next() % 4000) / 9.0);
        put(a, b, cells + 0x18, f64::from(rng.next() % 400) / 11.0);
        put(a, b, g(a, 0x00c5_d784), (rng.next() % 300) as f32 / 256.0);
        let small = |rng: &mut Xorshift| ((rng.next() % 6000) as f32 / 7.0 - 100.0).to_bits();
        match k % 3 {
            0 => {
                put(a, b, pen, rng.next());
                put(a, b, pen + 4, [small(rng), small(rng)]);
                put(a, b, pen + 0x1a, (rng.next() % 3) as u8);
                let args = [rng.next() % 4, rng.next(), pen];
                let ra = eax(a, 0x008e_9b80, &args);
                let rb = t::to_add_pkt_change_macro(b, args, call);
                assert_eq!(ra, rb, "change macro {k}");
            }
            1 => {
                let key = if rng.next().is_multiple_of(4) {
                    u32::MAX
                } else {
                    rng.next() % 4
                };
                let int = |rng: &mut Xorshift| (rng.next() % 4000).wrapping_sub(500);
                let args = [
                    rng.next(),
                    key,
                    text,
                    int(rng),
                    int(rng),
                    rng.next(),
                    [0, 1, 2, rng.next()][(rng.next() % 4) as usize],
                    rng.next(),
                    rng.next(),
                    rng.next(),
                    rng.next(),
                    [0, 1, 5, u32::MAX, rng.next()][(rng.next() % 5) as usize],
                ];
                let ra = eax(a, 0x008e_8fb0, &args);
                let rb = t::to_add_pkt_base_mes_font_l_abmap(b, args, call, fcall);
                assert_eq!(ra, rb, "abmap {k}");
            }
            _ => {
                let args = [
                    rng.next() % 4,
                    rng.next(),
                    rng.next() % 4,
                    small(rng),
                    small(rng),
                ];
                let ra = fcall(a, 0x008b_6bb0, &args);
                let rb = t::to_get_change_macro_advance(b, args, call, fcall);
                assert_eq!(ra.to_bits(), f64::from(rb).to_bits(), "advance {k}");
            }
        }
        same_memory(&format!("text {k}"), a, b);
    }
}

#[test]
fn round27_text_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round27_text(&mut a, &mut b, &mut Xorshift(0x4027_0021));
}

#[test]
fn round27_text_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round27_text(&mut a, &mut b, &mut Xorshift(0x4027_0022));
}

/// Round 27's message measure `TOGetMessageWH1612_008b6cb0` over the
/// round-27 text stand-ins, a logging `TOGetButtonIconWidth` stand-in,
/// stand-ins for `TOIntToMes` and `TOConvAlbhedWord`, two message windows
/// with counters, and the original `TOGetGroup` and macro width.
#[allow(
    clippy::cast_possible_truncation,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]
fn round27_measure(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let g = |a: &Oracle, x: u32| a.rebase(x);
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x30);
    let text = alloc(a, b, 0x80);
    let macro_text = alloc(a, b, 0x20);
    let windows = alloc(a, b, 2 * 0x40);
    let counters = alloc(a, b, 2 * 0x10);
    let out = alloc(a, b, 8);
    let scratch = alloc(a, b, 0x30);
    for w in 0..2u32 {
        put(a, b, g(a, 0x0186_76f0) + 4 * w, windows + 0x40 * w);
        put(a, b, g(a, 0x0186_8a90) + 4 * w, counters + 0x10 * w);
    }
    let keyed = |cursor: u32, key: u32, n: u8| {
        let mut v = marker(cursor, key);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        v
    };
    // logs its two arguments, the text pointer `text` (0 or 1, which
    // differs between the original's stack and the port's scratch) as the
    // byte it points at; leaves the text pointer in eax
    let byte_logger = |cursor: u32, text: u8| {
        let (t, o) = (4 + 4 * text, 8 - 4 * text);
        let mut v = vec![0x8b, 0x44, 0x24, t, 0x0f, 0xb6, 0x10, 0x8b, 0x0d];
        v.extend_from_slice(&cursor.to_le_bytes());
        v.extend_from_slice(&[
            0x89,
            0x51,
            4 * text,
            0x8b,
            0x54,
            0x24,
            o,
            0x89,
            0x51,
            4 - 4 * text,
        ]);
        v.extend_from_slice(&[0x83, 0x05]);
        v.extend_from_slice(&cursor.to_le_bytes());
        v.push(8);
        v
    };
    // TOGetFontInfo(p, rec): returns p + 1
    let mut v = byte_logger(cursor, 0);
    v.extend_from_slice(&[0x40, 0xc3]); // inc eax; ret
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x008b_7240, s);
    // TOSkipCode(p): p + 2 below 0x30, else p + 1
    let skip = [
        0x8b, 0x44, 0x24, 0x04, 0x80, 0x38, 0x30, 0x72, 0x02, 0x40, 0xc3, 0x83, 0xc0, 0x02, 0xc3,
    ];
    let s = stand_in8(a, b, 0xc0, &skip);
    redirect(a, b, 0x008d_fd80, s);
    // logs its arguments, returns a cell's word
    // TOGetChangeMacroAddress(key, code): returns cells[0]
    let mut v = byte_logger(cursor, 1);
    v.push(0xa1);
    v.extend_from_slice(&cells.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x100, &v);
    redirect(a, b, 0x008b_6660, s);
    // TOGetButtonIconWidth(n): logs n, returns cells[1]
    let mut v = keyed(cursor, 0x7800, 1);
    v.truncate(v.len() - 5); // drop "mov eax, [esp+4]; ret"
    v.push(0xa1);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x180, &v);
    redirect(a, b, 0x008b_65e0, s);
    // the advance and the space: log their arguments, return a cell in ST0
    for (off, sym, n, cell, key) in [
        (0x200u32, 0x008f_a740u32, 3u8, cells + 0x10, 0x7900u32),
        (0x280, 0x008f_a570, 1, cells + 0x18, 0x7a00),
    ] {
        let mut v = keyed(cursor, key, n);
        v.truncate(v.len() - 1);
        v.extend_from_slice(&st0_cell(cell));
        let s = stand_in8(a, b, off, &v);
        redirect(a, b, sym, s);
    }
    // TOIntToMes(value, out, digits, mode): logs value and digits, writes
    // `digits` 0x35 bytes and a terminator
    let mut v = vec![0x8b, 0x15];
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[
        0x8b, 0x44, 0x24, 0x04, 0x89, 0x02, 0x8b, 0x44, 0x24, 0x0c, 0x89, 0x42, 0x04,
    ]);
    v.extend_from_slice(&[0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.push(8);
    v.extend_from_slice(&[
        0x8b, 0x44, 0x24, 0x08, 0x8b, 0x4c, 0x24, 0x0c, 0xc6, 0x00, 0x35, 0x40, 0xe2, 0xfa, 0xc6,
        0x00, 0x00, 0xc3,
    ]);
    let s = stand_in8(a, b, 0x300, &v);
    redirect(a, b, 0x008b_8290, s);
    // TOConvAlbhedWord(in, out): logs the byte, writes 'X', 0
    let mut v = vec![0x8b, 0x44, 0x24, 0x04, 0x0f, 0xb6, 0x10, 0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x11, 0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.push(4);
    v.extend_from_slice(&[0x8b, 0x44, 0x24, 0x08, 0x66, 0xc7, 0x00, 0x58, 0x00, 0xc3]);
    let s = stand_in8(a, b, 0x380, &v);
    redirect(a, b, 0x0090_6780, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let quiet = |rng: &mut Xorshift| {
        let x = rng.float();
        if x.is_nan() {
            x.to_bits() | 0x0040_0000
        } else {
            x.to_bits()
        }
    };
    let pick = |rng: &mut Xorshift, v: &[u8]| v[(rng.next() as usize) % v.len()];
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        let mut bytes = Vec::new();
        for _ in 0..rng.next() % 24 {
            match rng.next() % 12 {
                0 => bytes.extend([
                    7,
                    pick(
                        rng,
                        &[0x38, 0x38, 0x40, 0x41, 0xbf, 0xc0, 0xe0, 0xe1, 0xef, 0xf0],
                    ),
                ]),
                1 => bytes.extend([0xb, (rng.next() % 30) as u8]),
                2 => bytes.extend([0xe, pick(rng, &[0x40, 0x41, 0x42])]),
                3 => bytes.extend([0x10, pick(rng, &[0x50, 0x55, 0x5f, 0x60, 0xff, 0x20])]),
                4 => bytes.extend([pick(rng, &[0x12, 0x22]), 0x31]),
                5 => bytes.extend([pick(rng, &[1, 2, 3, 3]), 0x31]),
                6 => bytes.extend([pick(rng, &[4, 0xc, 0xd, 8]), 0x33]),
                _ => bytes.push(0x30 + (rng.next() % 0x40) as u8),
            }
        }
        bytes.push(0);
        bytes.resize(0x80, 0);
        let bytes: [u8; 0x80] = bytes.try_into().unwrap();
        put(a, b, text, bytes);
        let mut mbytes = [0u8; 0x20];
        for (j, m) in mbytes
            .iter_mut()
            .take((rng.next() % 10) as usize)
            .enumerate()
        {
            *m = match j % 3 {
                1 => pick(rng, &[7, 0x31]),
                2 => pick(rng, &[0xe0, 0xe1, 0xef, 0x32]),
                _ => 0x30 + (rng.next() % 40) as u8,
            };
        }
        put(a, b, macro_text, mbytes);
        put(
            a,
            b,
            cells,
            if rng.next().is_multiple_of(5) {
                0
            } else {
                macro_text
            },
        );
        put(a, b, cells + 4, (rng.next() % 40).wrapping_sub(5));
        put(a, b, cells + 0x10, f64::from(rng.next() % 4000) / 9.0);
        put(a, b, cells + 0x18, f64::from(rng.next() % 400) / 11.0);
        for w in 0..2u32 {
            let at = windows + 0x40 * w;
            put(a, b, at + 0xe, 0u16);
            put(
                a,
                b,
                at + 0x10,
                [0x4000u16, 0, 0x4001][(rng.next() % 3) as usize],
            );
            put(a, b, at + 0x1e, 0u16);
            put(a, b, at + 0x24, [0u16, 0, 1, 2][(rng.next() % 4) as usize]);
            put(a, b, counters + 0x10 * w + 0xc, rng.next() % 100_000);
        }
        let window = [u32::MAX, 0, 1, 1][(rng.next() % 4) as usize];
        let kind = [0u32, 1, 3, 3, rng.next()][(rng.next() % 5) as usize];
        let flags = [0u32, 0, 1, 3, 0x100, 0x103, 0x200, rng.next()][(rng.next() % 8) as usize];
        let args = [
            window,
            text,
            out,
            out + 4,
            quiet(rng),
            quiet(rng),
            kind,
            flags,
        ];
        let ra = eax(a, 0x008b_6cb0, &args);
        let rb = t::to_get_message_wh_1612(b, args, scratch, call, fcall);
        assert_eq!(ra, rb, "measure {k}");
        let v: [u32; 0xc] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("measure {k}"), a, b);
    }
}

#[test]
fn round27_measure_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round27_measure(&mut a, &mut b, &mut Xorshift(0x4027_0031));
}

#[test]
fn round27_measure_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round27_measure(&mut a, &mut b, &mut Xorshift(0x4027_0032));
}

/// Round 27's battle message openers `TODrawAMES*` over logging stand-ins
/// for the message-window calls and the language.
#[allow(clippy::many_single_char_names)] // a, b: the oracles
fn round27_ames(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 8);
    for (k, (sym, n)) in (0u32..).zip([
        (0x008a_da30u32, 1u8),
        (0x008b_0000, 3),
        (0x008b_89d0, 3),
        (0x008a_f850, 4),
        (0x008a_fb90, 2),
        (0x008a_f7d0, 1),
        (0x008a_de10, 1),
    ]) {
        let mut v = marker(cursor, 0x7b00 + k);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        let s = stand_in8(a, b, 0x40 + 0x80 * k, &v);
        redirect(a, b, sym, s);
    }
    // TOGetFFXLang: returns cells[0], then cells[1] (rotating them)
    let mut v = vec![0xa1];
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x8b, 0x0d]);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.extend_from_slice(&[0x89, 0x0d]);
    v.extend_from_slice(&cells.to_le_bytes());
    v.push(0xa3);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x400, &v);
    redirect(a, b, 0x0064_10e0, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..600u32 {
        put(a, b, cursor, log);
        put(a, b, cells, [rng.next() % 8, rng.next() % 8]);
        let map = [0x185u32, 0x185, 0x184, rng.next()][(rng.next() % 4) as usize];
        put(a, b, a.rebase(0x012f_bc40), map);
        let window = rng.next() % 4;
        let text = rng.next();
        let (ra, rb) = match k % 4 {
            0 => {
                let x = [0xfau32, 0xfa, 0xf9, rng.next()][(rng.next() % 4) as usize];
                let y = [0x96u32, 0x96, 0x97, rng.next()][(rng.next() % 4) as usize];
                let args = [window, text, x, y, rng.next() % 9];
                (eax(a, 0x0089_b140, &args), t::to_draw_ames(b, args, call))
            }
            1 => (
                eax(a, 0x0089_b1d0, &[window, text]),
                t::to_draw_ames_centermes(b, window, text, call),
            ),
            2 => (
                eax(a, 0x0089_b230, &[window, text]),
                t::to_draw_ames_evjimaku(b, window, text, call),
            ),
            _ => (
                eax(a, 0x0089_b290, &[window, text]),
                t::to_draw_ames_evjimaku_albhed(b, window, text, call),
            ),
        };
        assert_eq!(ra, rb, "ames {k}");
        same_memory(&format!("ames {k}"), a, b);
    }
}

#[test]
fn round27_ames_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round27_ames(&mut a, &mut b, &mut Xorshift(0x4027_0041));
}

#[test]
fn round27_ames_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round27_ames(&mut a, &mut b, &mut Xorshift(0x4027_0042));
}

/// Round 27's `ToMakeBtlEasyEdgeDigitRight` over stand-ins for the width
/// (logs its words, stores a cell's float) and the drawer (logs its words).
#[allow(clippy::many_single_char_names)] // a, b: the oracles
fn round27_digit_right(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 4);
    let scratch = alloc(a, b, 4);
    // ToGetBtlEasyDigitWidth(digits, value, &w, scale, extra): logs all but
    // &w, *w = cells[0]
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    for (k, at) in [(0u8, 4u8), (1, 8), (2, 0x10), (3, 0x14)] {
        // mov eax, [esp+at]; mov [ecx+4k], eax
        v.extend_from_slice(&[0x8b, 0x44, 0x24, at, 0x89, 0x41, 4 * k]);
    }
    v.extend_from_slice(&[0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x10, 0x8b, 0x4c, 0x24, 0x0c, 0xa1]);
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x01, 0xc3]);
    let s = stand_in8(a, b, 0x40, &v);
    redirect(a, b, 0x0090_5190, s);
    let mut v = marker(cursor, 0x7c00);
    v.pop();
    v.extend_from_slice(&logger_args(cursor, 7));
    let s = stand_in8(a, b, 0x100, &v);
    redirect(a, b, 0x0090_57c0, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let quiet = |rng: &mut Xorshift| {
        let x = rng.float();
        if x.is_nan() {
            x.to_bits() | 0x0040_0000
        } else {
            x.to_bits()
        }
    };
    for k in 0..2000u32 {
        put(a, b, cursor, log);
        put(a, b, cells, quiet(rng));
        let value = match rng.next() % 5 {
            0 => 0,
            1 => rng.next() % 10,
            2 => rng.next() % 100_000,
            3 => [
                i32::MIN.cast_unsigned(),
                u32::MAX,
                1_000_000_000,
                999_999_999,
            ][(rng.next() % 4) as usize],
            _ => rng.next(),
        };
        let args = [
            value,
            quiet(rng),
            quiet(rng),
            rng.next(),
            quiet(rng),
            quiet(rng),
        ];
        assert_eq!(
            eax(a, 0x0090_5890, &args),
            t::to_make_btl_easy_edge_digit_right(b, args, scratch, call),
            "digit right {k}"
        );
        let v: u32 = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("digit right {k}"), a, b);
    }
}

#[test]
fn round27_digit_right_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round27_digit_right(&mut a, &mut b, &mut Xorshift(0x4027_0051));
}

#[test]
fn round27_digit_right_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round27_digit_right(&mut a, &mut b, &mut Xorshift(0x4027_0052));
}

/// Round 27's battle help-text setters and digit labels over stand-ins for
/// `TOGetMessageWH` and `ToMakeBtlEasyDigitRight` (logging their words),
/// `MsGetComData` (logs the command, `*strings = cells[0]`, returns
/// `cells + 4`) and the language (`cells[4]`).
#[allow(clippy::many_single_char_names, clippy::cast_possible_truncation)]
fn round27_help(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let strings = alloc(a, b, 0x200);
    let scratch = alloc(a, b, 4);
    for (k, (sym, n)) in (0u32..).zip([(0x008b_9650u32, 6u8), (0x0090_55c0, 6)]) {
        let mut v = marker(cursor, 0x7d00 + k);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        let s = stand_in8(a, b, 0x40 + 0x80 * k, &v);
        redirect(a, b, sym, s);
    }
    // MsGetComData(command, &strings)
    let mut v = marker(cursor, 0x7e00);
    v.pop();
    v.extend_from_slice(&logger_args(cursor, 1));
    v.truncate(v.len() - 5); // drop "mov eax, [esp+4]; ret"
    v.extend_from_slice(&[0x8b, 0x4c, 0x24, 0x08, 0xa1]); // mov ecx, [esp+8]; mov eax, [cells]
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x01, 0xb8]); // mov [ecx], eax; mov eax, cells + 4
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x200, &v);
    redirect(a, b, 0x0079_a4b0, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 0xc).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x280, &v);
    redirect(a, b, 0x0064_10e0, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..800u32 {
        put(a, b, cursor, log);
        let text: Vec<u8> = (0..0x200)
            .map(|_| {
                if rng.next().is_multiple_of(9) {
                    0
                } else {
                    rng.next() as u8 | 1
                }
            })
            .collect();
        let text: [u8; 0x200] = text.try_into().unwrap();
        put(a, b, strings, text);
        let off = rng.next() % 0x100;
        put(a, b, cells, strings);
        put(a, b, cells + 4, [0u32, off]);
        put(
            a,
            b,
            cells + 0xc,
            [0u32, 1, 8, 9, 0xb, 0xc, u32::MAX][(rng.next() % 7) as usize],
        );
        // the help buffer holds an older text: the copy clears it first
        let old: [u32; 0x20] = std::array::from_fn(|_| rng.next());
        put(a, b, a.rebase(0x0133_f538), old);
        match k % 4 {
            0 => {
                let p = if rng.next().is_multiple_of(5) {
                    0
                } else {
                    strings + rng.next() % 0x100
                };
                let ra = eax(a, 0x0089_9ec0, &[p]);
                if let Some(rb) = t::to_btl_set_direct_help_mes(b, p, call) {
                    assert_eq!(ra, rb, "direct help {k}");
                }
            }
            1 => {
                let command = rng.next();
                assert_eq!(
                    eax(a, 0x0089_9970, &[command]),
                    t::to_btl_set_command_help_mes(b, command, scratch, call),
                    "command help {k}"
                );
                let v: u32 = Mem::read(b, scratch);
                Mem::write(a, scratch, v);
            }
            n => {
                let which = (n - 2) as u8;
                let int = |rng: &mut Xorshift| match rng.next() % 4 {
                    0 => rng.next(),
                    _ => (rng.next() % 2000).wrapping_sub(300),
                };
                let args = [rng.next(), rng.next(), int(rng), int(rng), rng.next()];
                let f = [0x008f_3300, 0x008f_3420][usize::from(which)];
                assert_eq!(
                    eax(a, f, &args),
                    t::to_mkp_btl_digit_right(b, which, args, call),
                    "digit label {which} {k}"
                );
            }
        }
        same_memory(&format!("help {k}"), a, b);
    }
}

#[test]
fn round27_help_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round27_help(&mut a, &mut b, &mut Xorshift(0x4027_0061));
}

#[test]
fn round27_help_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round27_help(&mut a, &mut b, &mut Xorshift(0x4027_0062));
}

/// `TODrawKick` and `TOCtrlPkt` over logging stand-ins for the PS2 layer's
/// `FlushCache`, `sceDmaSend` and `sceGsSyncPath`.
fn round27_kick(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use pkt::bridge as p;
    let log = alloc(a, b, 0x100);
    let cursor = alloc(a, b, 4);
    let s = stand_in8(a, b, 0x40, &logger_args(cursor, 1));
    redirect(a, b, 0x0062_f4c0, s);
    let s = stand_in8(a, b, 0x80, &logger_args(cursor, 2));
    redirect(a, b, 0x0062_f710, s);
    let s = stand_in8(a, b, 0xc0, &logger(cursor, 0x5a5a_0000));
    redirect(a, b, 0x0062_f880, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..2000u32 {
        put(a, b, cursor, log);
        Mem::write(a, log, [0u32; 8]);
        Mem::write(b, log, [0u32; 8]);
        let start = rng.next();
        let near = |rng: &mut Xorshift| match rng.next() % 4 {
            0 => rng.next(),
            _ => start.wrapping_add(rng.next() % 0x60).wrapping_sub(0x10),
        };
        let (pos, limit) = (near(rng), near(rng));
        set(a, b, p::PKT_BUFF, [start, rng.next(), limit]);
        set(a, b, p::PACKET_CURSOR, pos);
        set(a, b, p::DMA_CHANNEL, rng.next());
        set(
            a,
            b,
            p::ADD_PKT,
            [0, 1, rng.next()][(rng.next() % 3) as usize],
        );
        if k % 2 == 0 {
            let want = eax(a, 0x008f_5bc0, &[]);
            assert_eq!(p::draw_kick(b, call), want, "kick {k}");
        } else {
            let want = eax(a, 0x008f_3a30, &[]);
            if let Some(got) = p::ctrl_pkt(b, call) {
                assert_eq!(got, want, "ctrl {k}");
            }
        }
        same_memory(&format!("kick {k}"), a, b);
    }
}

#[test]
fn round27_kick_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round27_kick(&mut a, &mut b, &mut Xorshift(0x4027_0011));
}

#[test]
fn round27_kick_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round27_kick(&mut a, &mut b, &mut Xorshift(0x4027_0012));
}

/// Round 26's cross-menu icon over a logging texture-name stand-in,
/// `TOGetImageWH` and the scissor from cells, and a `graphicDrawUIElement`
/// stand-in logging the record.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn round26_cross_icon(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let scratch = alloc(a, b, 0xa0);
    let s = stand_in8(a, b, 0x40, &logger_args(cursor, 1));
    redirect(a, b, 0x008a_c8c0, s);
    let mut v = logger(cursor, 0x3dc0);
    v.pop();
    v.extend_from_slice(&[0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d]);
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d]);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0xc3]);
    let s = stand_in8(a, b, 0x80, &v);
    redirect(a, b, 0x008a_c400, s);
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x100, &v);
    redirect(a, b, 0x008e_5a20, s);
    let words: Vec<u8> = (0..16u8).map(|k| 4 * k).chain([0x80, 0x88]).collect();
    let s = stand_in8(a, b, 0x200, &record_words_logger(cursor, &words));
    redirect(a, b, 0x0063_eee0, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..1000u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells,
            [
                [256.0f32, 255.0, 512.0][(rng.next() % 3) as usize],
                [256.0f32, 300.0][(rng.next() % 2) as usize],
            ],
        );
        put(a, b, cells + 8, u32::from(!rng.next().is_multiple_of(6)));
        let fl =
            |rng: &mut Xorshift, m: u32, d: f32| ((rng.next() % m) as f32 / d - 20.0).to_bits();
        let args = [
            fl(rng, 60000, 33.0),
            fl(rng, 40000, 33.0),
            fl(rng, 3000, 9.0),
            fl(rng, 3000, 9.0),
            rng.next(),
            rng.next(),
            rng.next(),
            rng.next(),
            rng.next(),
        ];
        eax(a, 0x008e_6af0, &args);
        t::draw_cross_menu_icon_xywh_rgba(b, args, scratch, call);
        let v: [u32; 0x28] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("cross icon {k}"), a, b);
    }
}

#[test]
fn round26_cross_icon_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round26_cross_icon(&mut a, &mut b, &mut Xorshift(0x4026_0021));
}

#[test]
fn round26_cross_icon_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round26_cross_icon(&mut a, &mut b, &mut Xorshift(0x4026_0022));
}

#[test]
fn round25_wrappers_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round25_wrappers(&mut a, &mut b, &mut Xorshift(0x4025_0011));
}

#[test]
fn round25_wrappers_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round25_wrappers(&mut a, &mut b, &mut Xorshift(0x4025_0012));
}

#[test]
fn round19_cursor_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round19_cursor(&mut a, &mut b, &mut Xorshift(0x4019_0021));
}

#[test]
fn round19_cursor_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round19_cursor(&mut a, &mut b, &mut Xorshift(0x4019_0022));
}

/// Appends `mov eax, [esp+off]; mov [ecx+at], eax`.
fn log_arg(v: &mut Vec<u8>, off: u8, at: u8) {
    v.extend_from_slice(&[0x8b, 0x44, 0x24, off, 0x89, 0x41, at]);
}

/// A stand-in that logs the low half of the word its pointer argument `+4`
/// points at
/// and the words at `[esp+offs]`, then (with `out` set) stores `[cell]`
/// through the pointer at `[esp+out]` and steps `[cell]` by 0x00400000.
fn deref_logger(cursor: u32, offs: &[u8], out: Option<(u8, u32)>) -> Vec<u8> {
    // the rest of the originals' text buffers is stack garbage
    deref_logger_mask(cursor, offs, out, 0xffff)
}

/// [`deref_logger`] keeping the pointed-at word's bits in `mask`.
fn deref_logger_mask(cursor: u32, offs: &[u8], out: Option<(u8, u32)>, mask: u32) -> Vec<u8> {
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    // mov eax, [esp+4]; mov eax, [eax]; and eax, mask; mov [ecx], eax
    v.extend_from_slice(&[0x8b, 0x44, 0x24, 0x04, 0x8b, 0x00, 0x25]);
    v.extend_from_slice(&mask.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x01]);
    for (k, &off) in (1u8..).zip(offs) {
        log_arg(&mut v, off, 4 * k);
    }
    v.extend_from_slice(&[0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.push(4 * (1 + u8::try_from(offs.len()).unwrap()));
    if let Some((at, cell)) = out {
        v.push(0xa1);
        v.extend_from_slice(&cell.to_le_bytes());
        v.extend_from_slice(&[0x8b, 0x4c, 0x24, at, 0x89, 0x01, 0x81, 0x05]);
        v.extend_from_slice(&cell.to_le_bytes());
        v.extend_from_slice(&0x0040_0000u32.to_le_bytes());
    }
    v.push(0xc3);
    v
}

/// `TOMenuDrawTimer` over logging stand-ins for the battle font's width
/// and drawing calls, the language and the letter space.
#[allow(clippy::cast_precision_loss)] // small test floats
fn round28_timer(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x200);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let scratch = alloc(a, b, 0x10);
    // ToGetBtlEasyFontWidth(text, &w, alpha, scale, wide)
    let s = stand_in8(
        a,
        b,
        0x40,
        &deref_logger(cursor, &[0xc, 0x10, 0x14], Some((8, cells))),
    );
    redirect(a, b, 0x0090_5290, s);
    // ToGetBtlEasyDigitWidth2(value, &w, scale, wide): logs value as a word
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    for (k, off) in [4u8, 0xc, 0x10].into_iter().enumerate() {
        log_arg(&mut v, off, 4 * u8::try_from(k).unwrap());
    }
    v.extend_from_slice(&[0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x0c, 0xa1]);
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x8b, 0x4c, 0x24, 0x08, 0x89, 0x01, 0xc3]);
    let s = stand_in8(a, b, 0xc0, &v);
    redirect(a, b, 0x0090_5230, s);
    // ToMakeBtlEasyEdgeFont(text, x, y, alpha, scale, wide)
    let s = stand_in8(
        a,
        b,
        0x140,
        &deref_logger(cursor, &[8, 0xc, 0x10, 0x14, 0x18], None),
    );
    redirect(a, b, 0x0090_5930, s);
    let s = stand_in8(a, b, 0x1c0, &logger_args(cursor, 6));
    redirect(a, b, 0x0090_5820, s);
    // TOGetFFXLang counts its calls in cells + 0xc
    let mut v = vec![0xff, 0x05];
    v.extend_from_slice(&(cells + 0xc).to_le_bytes());
    v.push(0xa1);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x240, &v);
    redirect(a, b, 0x008a_c2f0, s);
    // TOGetFontSpaceInPiexl: fld dword [cells + 8]
    let mut v = vec![0xd9, 0x05];
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x2c0, &v);
    redirect(a, b, 0x008f_a570, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let float = |rng: &mut Xorshift, range: u32| ((rng.next() % range) as f32 / 8.0).to_bits();
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        put(a, b, cells, float(rng, 800));
        put(
            a,
            b,
            cells + 4,
            [0u32, 1, 8, 9, 0xb, u32::MAX][(rng.next() % 6) as usize],
        );
        put(a, b, cells + 8, float(rng, 200));
        put(a, b, cells + 0xc, 0u32);
        let frames = match rng.next() % 4 {
            0 => rng.next(),
            1 => rng.next() % 3600,
            _ => rng.next() % 4_000_000,
        };
        let kind = [0x25, 0x24, rng.next()][(rng.next() % 3) as usize];
        let args = [
            frames,
            float(rng, 8000),
            float(rng, 4000),
            rng.next() % 0x100,
            kind,
        ];
        eax(a, 0x008e_19f0, &args);
        t::to_menu_draw_timer(b, args, scratch, call, fcall);
        let v: [u32; 4] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("timer {k}"), a, b);
    }
}

#[test]
fn round28_timer_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round28_timer(&mut a, &mut b, &mut Xorshift(0x4028_0011));
}

#[test]
fn round28_timer_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round28_timer(&mut a, &mut b, &mut Xorshift(0x4028_0012));
}

/// `DrawCrossCtbIcon` and `TOBtlDrawInfoWinMes` over logging stand-ins for
/// the shape, font, scissor and button-data calls.
#[allow(clippy::cast_precision_loss)] // small test floats
fn round28_icons(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    for (k, (sym, n)) in (0u32..).zip([
        (0x008b_1350u32, 2u8),
        (0x0090_3bb0, 9),
        (0x008e_8940, 4),
        (0x0090_16b0, 7),
    ]) {
        let mut v = marker(cursor, 0x2800 + k);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        let s = stand_in8(a, b, 0x80 * k, &v);
        redirect(a, b, sym, s);
    }
    let s = stand_in8(a, b, 0x200, &marker(cursor, 0x2810));
    redirect(a, b, 0x008b_1330, s);
    let s = stand_in8(a, b, 0x280, &marker(cursor, 0x2811));
    redirect(a, b, 0x008e_5cd0, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let float = |rng: &mut Xorshift| match rng.next() % 8 {
        0 => -((rng.next() % 4000) as f32 / 16.0),
        _ => (rng.next() % 16000) as f32 / 16.0,
    };
    let [libra, shown, settings, boxes, lines] = [
        0x0133_d6e0,
        0x0133_c93d,
        0x0113_079c,
        0x0133_f694,
        0x0133_f618,
    ]
    .map(|x| a.rebase(x));
    for k in 0..2000u32 {
        put(a, b, cursor, log);
        if k % 4 == 3 {
            put(a, b, libra, rng.next() % 3);
            put(a, b, shown, (rng.next() % 2) as u8);
            put(a, b, settings, rng.next());
            for off in (0..0x10).step_by(4) {
                put(a, b, boxes + off, (rng.next() % 2000) as i16 - 300);
            }
            for line in 0..4 {
                let at = lines + 0x1c * line;
                put(a, b, at, [0i16, 1, -1][(rng.next() % 3) as usize]);
                put(a, b, at + 0xc, float(rng).to_bits());
                put(a, b, at + 0x10, float(rng).to_bits());
                put(a, b, at + 0x18, rng.next());
            }
            eax(a, 0x0089_5140, &[]);
            t::to_btl_draw_info_win_mes(b, call);
        } else {
            let id = match rng.next() % 4 {
                0 => rng.next(),
                _ => 0x1b0 + rng.next() % 0x2c,
            };
            let args = [
                id,
                float(rng).to_bits(),
                float(rng).to_bits(),
                float(rng).to_bits(),
                float(rng).to_bits(),
            ];
            assert_eq!(
                eax(a, 0x0089_e660, &args),
                t::draw_cross_ctb_icon(b, args, call, fcall),
                "icon {k} {id:#x}"
            );
        }
        same_memory(&format!("icons {k}"), a, b);
    }
}

#[test]
fn round28_icons_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round28_icons(&mut a, &mut b, &mut Xorshift(0x4028_0021));
}

#[test]
fn round28_icons_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round28_icons(&mut a, &mut b, &mut Xorshift(0x4028_0022));
}

/// A logging stand-in (`logger_args`) that returns `[cell]` instead.
fn logger_returning(cursor: u32, n: u8, cell: u32) -> Vec<u8> {
    let mut v = logger_args(cursor, n);
    v.truncate(v.len() - 5); // drop "mov eax, [esp+4]; ret"
    v.push(0xa1);
    v.extend_from_slice(&cell.to_le_bytes());
    v.push(0xc3);
    v
}

/// `TOBtlDrawHelpWin` over logging stand-ins for the scissors, shapes,
/// font width and font drawing.
#[allow(clippy::cast_precision_loss, clippy::similar_names)] // small test floats; cell/call
fn round28_help_win(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x800);
    let cursor = alloc(a, b, 4);
    let cell = alloc(a, b, 4);
    let scratch = alloc(a, b, 4);
    for (k, (sym, n)) in (0u32..).zip([
        (0x008f_9230u32, 4u8),
        (0x0090_3ee0, 11),
        (0x0090_3bb0, 9),
        (0x008e_8940, 4),
        (0x0090_16b0, 7),
    ]) {
        let mut v = marker(cursor, 0x2820 + k);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        let s = stand_in8(a, b, 0x80 * k, &v);
        redirect(a, b, sym, s);
    }
    let s = stand_in8(a, b, 0x280, &marker(cursor, 0x2830));
    redirect(a, b, 0x008e_5cd0, s);
    // ToGetCrossExtMesFontWidth(kind, text, &w, scale, extra): w = [cell]
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    for (k, off) in [4u8, 8, 0x10, 0x14].into_iter().enumerate() {
        log_arg(&mut v, off, 4 * u8::try_from(k).unwrap());
    }
    v.extend_from_slice(&[0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x10, 0xa1]);
    v.extend_from_slice(&cell.to_le_bytes());
    v.extend_from_slice(&[0x8b, 0x4c, 0x24, 0x0c, 0x89, 0x01, 0xc3]);
    let s = stand_in8(a, b, 0x300, &v);
    redirect(a, b, 0x0090_5320, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let [libra, open, shown, settings] =
        [0x0133_d6e0, 0x0133_c93c, 0x0133_c93d, 0x0113_079c].map(|x| a.rebase(x));
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        put(a, b, libra, [0u32, 0, 1, 2][(rng.next() % 4) as usize]);
        put(a, b, open, [1u8, 1, 1, 0, 2][(rng.next() % 5) as usize]);
        put(a, b, shown, (rng.next() % 2) as u8);
        put(
            a,
            b,
            settings,
            rng.next() | (0x400 * u32::from(!rng.next().is_multiple_of(4))),
        );
        put(a, b, cell, (rng.next() % 9000) as f32 / 8.0);
        let scroll = -((rng.next() % 9000) as f32 / 8.0) + 40.0;
        t::HELP_SCROLL.set(a, scroll);
        t::HELP_SCROLL.set(b, scroll);
        eax(a, 0x0089_36d0, &[]);
        t::to_btl_draw_help_win(b, scratch, call, fcall);
        let v: u32 = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("help window {k}"), a, b);
    }
}

#[test]
fn round28_help_win_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round28_help_win(&mut a, &mut b, &mut Xorshift(0x4028_0031));
}

#[test]
fn round28_help_win_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round28_help_win(&mut a, &mut b, &mut Xorshift(0x4028_0032));
}

/// `TOGetPadStatusPrepare`, the side pads and `TOBtlGetStdPad` over
/// logging stand-ins for the click, the item in use and the battle action
/// calls.
#[allow(
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
fn round28_pad_prepare(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x100);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let w = alloc(a, b, 0x100);
    let list = alloc(a, b, 0x200) + 0x100;
    for (k, (sym, n)) in (0u32..).zip([(0x0079_0790u32, 1u8), (0x0088_6ba0, 1)]) {
        let mut v = marker(cursor, 0x2840 + k);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        let s = stand_in8(a, b, 0x80 * k, &v);
        redirect(a, b, sym, s);
    }
    let s = stand_in8(a, b, 0x100, &marker(cursor, 0x2850));
    redirect(a, b, 0x0089_a630, s);
    let s = stand_in8(a, b, 0x180, &logger_returning(cursor, 2, cells));
    redirect(a, b, 0x0089_aca0, s);
    let s = stand_in8(a, b, 0x200, &logger_returning(cursor, 2, cells + 4));
    redirect(a, b, 0x0089_a330, s);
    // the side pads' skip command, battle command, window and scene calls
    let s = stand_in8(a, b, 0x280, &logger_returning(cursor, 1, cells + 8));
    redirect(a, b, 0x0078_fb80, s);
    let s = stand_in8(a, b, 0x300, &logger_returning(cursor, 1, cells + 0xc));
    redirect(a, b, 0x0078_4ff0, s);
    for (k, (sym, n)) in (0u32..).zip([(0x0079_2d60u32, 3u8), (0x0089_ade0, 1), (0x008a_e5a0, 1)]) {
        let mut v = marker(cursor, 0x2860 + k);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        let s = stand_in8(a, b, 0x380 + 0x80 * k, &v);
        redirect(a, b, sym, s);
    }
    let s = stand_in8(a, b, 0x500, &marker(cursor, 0x2870));
    redirect(a, b, 0x0089_0eb0, s);
    // the standard pad's MsCheckLeftWindow and MsGetSaveWindow (count
    // from a cell into its third argument, the list from another)
    let more = alloc(a, b, 0x10);
    let windows = alloc(a, b, 0x40);
    let scratch = alloc(a, b, 4);
    let s = stand_in8(a, b, 0x580, &logger_returning(cursor, 1, more));
    redirect(a, b, 0x0078_f700, s);
    let mut v = logger_args(cursor, 2);
    v.truncate(v.len() - 5);
    v.extend_from_slice(&[0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d]);
    v.extend_from_slice(&(more + 4).to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0xa1]);
    v.extend_from_slice(&(more + 8).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x600, &v);
    redirect(a, b, 0x0078_5410, s);
    put(a, b, more + 8, windows + 0x20);
    let [flag, missed] = [0x0133_d6e4, 0x0133_d6ec].map(|x| a.rebase(x));
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let [open, libra, depth, pad] =
        [0x023c_c08c, 0x0133_d6e0, 0x023c_c092, 0x025d_09d2].map(|x| a.rebase(x));
    let idle = a.rebase(t::SCROLL_DONE);
    for k in 0..8000u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            more,
            [0u32, 1, 5, u32::MAX][(rng.next() % 4) as usize],
        );
        put(
            a,
            b,
            more + 4,
            [0u32, 1, 2, 4, 8, u32::MAX, (-5i32).cast_unsigned()][(rng.next() % 7) as usize],
        );
        let entries: [u16; 0x20] = std::array::from_fn(|_| {
            if (rng.next() >> 16).is_multiple_of(2) {
                0xff
            } else {
                rng.next() as u16
            }
        });
        put(a, b, windows, entries);
        put(a, b, flag, rng.next() % 3);
        put(a, b, missed, rng.next() % 3);
        put(
            a,
            b,
            cells,
            [0u32, 1, 0xfe, u32::MAX][(rng.next() % 4) as usize],
        );
        put(a, b, cells + 4, [0u32, 1, 2][(rng.next() % 3) as usize]);
        put(
            a,
            b,
            cells + 8,
            [0u32, 0x0001_0000, 0x3002, rng.next()][(rng.next() % 4) as usize],
        );
        put(a, b, cells + 0xc, u32::from(rng.next().is_multiple_of(3)));
        put(a, b, open, u32::from(!rng.next().is_multiple_of(8)));
        put(a, b, libra, [0u32, 0, 0, 1, 2][(rng.next() % 5) as usize]);
        put(a, b, depth, [0i8, 1, 2, -1][(rng.next() % 4) as usize]);
        let held = [
            0u16,
            0x9000,
            0x3000,
            0xc000,
            0x6000,
            0x1000,
            0xa000,
            rng.next() as u16,
        ][(rng.next() % 8) as usize];
        let trigger = [
            0u16,
            0,
            0,
            0x40,
            0x20,
            0x04,
            0x10,
            0x2000,
            0x8000,
            rng.next() as u16,
        ][(rng.next() % 10) as usize];
        let rep =
            [0x1000u16, 0x4000, 0x8000, 0x2000, 0, rng.next() as u16][(rng.next() % 6) as usize];
        put(a, b, pad, [held, trigger, rep]);
        let bytes: [u32; 0x40] = std::array::from_fn(|_| rng.next());
        put(a, b, w, bytes);
        let items: [u32; 0x40] = std::array::from_fn(|_| rng.next());
        put(a, b, list - 0x40, items);
        put(a, b, w + 0x20, list);
        put(
            a,
            b,
            w + 0x88,
            if rng.next().is_multiple_of(8) {
                rng.next()
            } else {
                idle
            },
        );
        let count = (1 + rng.next() % 20) as i16;
        let step = (1 + rng.next() % 4) as i16;
        let rows = (1 + rng.next() % 5) as i16;
        put(a, b, w + 0x38, [step, rows]);
        put(a, b, w + 0x44, count);
        put(a, b, w + 0x40, (rng.next() % 14) as i16 - 2);
        put(a, b, w + 0x42, (rng.next() % (count as u32 + 4)) as i16 - 2);
        put(
            a,
            b,
            w + 0xa,
            [(1 + rng.next() % 3) as i8, (rng.next() % 4) as i8 - 1],
        );
        put(a, b, a.rebase(0x023c_c088), rng.next() as u16);
        if k % 4 == 3 && rng.next().is_multiple_of(3) {
            // the right window's list check, more often
            put(a, b, depth, 0i8);
            put(a, b, pad + 2, 0x2000u16);
        }
        match k % 4 {
            0 => {
                eax(a, 0x0089_d610, &[w]);
                t::to_get_pad_status_prepare(b, w, call);
            }
            3 => {
                eax(a, 0x0089_cde0, &[w]);
                t::to_btl_get_std_pad(b, w, scratch, call);
                let v: u32 = Mem::read(b, scratch);
                Mem::write(a, scratch, v);
            }
            n => {
                let (f, cancel) = [(0x008a_22f0, 0x2000), (0x008a_2690, 0x8000)][n as usize - 1];
                eax(a, f, &[w]);
                t::to_btl_get_side_pad(b, w, cancel, call);
            }
        }
        same_memory(&format!("pad prepare {k}"), a, b);
    }
}

#[test]
fn round28_pad_prepare_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round28_pad_prepare(&mut a, &mut b, &mut Xorshift(0x4028_0041));
}

#[test]
fn round28_pad_prepare_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round28_pad_prepare(&mut a, &mut b, &mut Xorshift(0x4028_0042));
}

/// `MenuItem_ApplyHeal_008c8c40` over logging stand-ins for the heal
/// amount, the HP change, the item record and the party and aeon lists.
#[allow(clippy::cast_possible_truncation)]
fn round28_item_heal(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let record = alloc(a, b, 0x40);
    let scratch = alloc(a, b, 4);
    // TkMenuItemHealAmount_008c69a0(item, who, mp): logs, returns who
    let mut v = logger_args(cursor, 3);
    v.truncate(v.len() - 5);
    v.extend_from_slice(&[0x8b, 0x44, 0x24, 0x08, 0xc3]);
    let s = stand_in8(a, b, 0, &v);
    redirect(a, b, 0x008c_69a0, s);
    for (k, (sym, n)) in (1u32..).zip([(0x008c_8d10u32, 2u8), (0x008a_9bc0, 1), (0x008a_9ad0, 1)]) {
        let mut v = marker(cursor, 0x2880 + k);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        let s = stand_in8(a, b, 0x80 * k, &v);
        redirect(a, b, sym, s);
    }
    let s = stand_in8(a, b, 0x200, &logger_returning(cursor, 1, cells));
    redirect(a, b, 0x008a_b280, s);
    let s = stand_in8(a, b, 0x280, &logger_returning(cursor, 0, cells + 4));
    redirect(a, b, 0x008a_9bf0, s);
    let s = stand_in8(a, b, 0x300, &logger_returning(cursor, 0, cells + 8));
    redirect(a, b, 0x008a_9b30, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let [item, single] = [0x0186_a3a8, 0x0186_a391].map(|x| a.rebase(x));
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        put(a, b, cells, record);
        let count = |rng: &mut Xorshift| [0u32, 1, 3, 8, u32::MAX][(rng.next() % 5) as usize];
        put(a, b, cells + 4, count(rng));
        put(a, b, cells + 8, count(rng));
        put(a, b, record + 0x20, rng.next());
        put(a, b, item, rng.next() as u16);
        put(a, b, single, [0u8, 1, 1, 2][(rng.next() % 4) as usize]);
        let target = rng.next() % 0x20;
        assert_eq!(
            eax(a, 0x008c_8c40, &[target]),
            t::menu_item_apply_heal(b, target, scratch, call),
            "item heal {k}"
        );
        let v: u32 = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("item heal {k}"), a, b);
    }
}

#[test]
fn round28_item_heal_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round28_item_heal(&mut a, &mut b, &mut Xorshift(0x4028_0051));
}

#[test]
fn round28_item_heal_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round28_item_heal(&mut a, &mut b, &mut Xorshift(0x4028_0052));
}

/// `DrawWaterWaveShapeC2` over stand-ins for the clock, the sheet's
/// texture and size, and a draw call that logs the record's set words.
#[allow(clippy::cast_precision_loss)] // small test floats
fn round28_wave(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x200);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let scratch = alloc(a, b, 0xa0);
    // graphicGetTime: fld qword [cells]; add dword [cells], 0x1000
    let mut v = vec![0xdd, 0x05];
    v.extend_from_slice(&cells.to_le_bytes());
    v.extend_from_slice(&[0x81, 0x05]);
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.extend_from_slice(&0x0000_1000u32.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0, &v);
    redirect(a, b, 0x0064_1410, s);
    // TOGetShapTextureName(id): logs, returns id ^ key
    let s = stand_in8(a, b, 0x80, &logger(cursor, 0x2890_0000));
    redirect(a, b, 0x008a_c8c0, s);
    // TOGetImageWH(id, &w, &h): w, h from cells + 8, + 0xc
    let mut v = vec![0xa1];
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.extend_from_slice(&[0x8b, 0x4c, 0x24, 0x08, 0x89, 0x01, 0xa1]);
    v.extend_from_slice(&(cells + 0xc).to_le_bytes());
    v.extend_from_slice(&[0x8b, 0x4c, 0x24, 0x0c, 0x89, 0x01, 0xc3]);
    let s = stand_in8(a, b, 0x100, &v);
    redirect(a, b, 0x008a_c400, s);
    // graphicDrawUIElement(rec, texture, 1, 0, 0): the record's 0x40 bytes
    // and +0x90/+0x94, then the other arguments
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
    for (k, off) in (0..0x10u8).map(|k| 4 * k).chain([0x90, 0x94]).enumerate() {
        // mov eax, [edx+off]; mov [ecx+4k], eax
        v.extend_from_slice(&[0x8b, 0x82]);
        v.extend_from_slice(&u32::from(off).to_le_bytes());
        v.extend_from_slice(&[0x89, 0x81]);
        v.extend_from_slice(&(4 * u32::try_from(k).unwrap()).to_le_bytes());
    }
    for (k, off) in [8u8, 0xc, 0x10, 0x14].into_iter().enumerate() {
        v.extend_from_slice(&[0x8b, 0x44, 0x24, off, 0x89, 0x81]);
        v.extend_from_slice(&(0x48 + 4 * u32::try_from(k).unwrap()).to_le_bytes());
    }
    v.extend_from_slice(&[0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x58, 0xb8]);
    v.extend_from_slice(&0x2891_0000u32.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x180, &v);
    redirect(a, b, 0x0063_eee0, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let float = |rng: &mut Xorshift, range: u32| (rng.next() % range) as f32 / 16.0 - 100.0;
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        let time = f64::from(rng.next() % 100_000) / 60.0;
        put(a, b, cells, time);
        let size =
            |rng: &mut Xorshift| [256.0f32, 300.0, 7.5, 3.0, 1.0, 0.0][(rng.next() % 6) as usize];
        put(a, b, cells + 8, [size(rng), size(rng)]);
        let mut args = [0u32; 12];
        for v in &mut args[..10] {
            *v = match rng.next() % 4 {
                0 => (float(rng, 8000) * 3e6).to_bits(),
                _ => (float(rng, 8000) / 3.0).to_bits(),
            };
        }
        // the record's words the original leaves are its stack's
        let junk: [u32; 0x28] = std::array::from_fn(|_| rng.next());
        put(a, b, scratch, junk);
        args[10] = rng.next();
        args[11] = rng.next();
        assert_eq!(
            eax(a, 0x008e_7d30, &args),
            t::draw_water_wave_shape_c2(b, args, scratch, call, fcall),
            "wave {k}"
        );
        let v: [u32; 0x28] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("wave {k}"), a, b);
    }
}

#[test]
fn round28_wave_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round28_wave(&mut a, &mut b, &mut Xorshift(0x4028_0061));
}

#[test]
fn round28_wave_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round28_wave(&mut a, &mut b, &mut Xorshift(0x4028_0062));
}

/// `QuickRecoverMenuDraw` over stand-ins for the battle font's width,
/// glyph record, advance and drawing calls and the scissor.
#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names
)]
fn round29_quick_recover(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let scratch = alloc(a, b, 0x50);
    let text = alloc(a, b, 0x40);
    let stand = |a: &mut Oracle, b: &mut Oracle, off: u32, sym: u32, code: &[u8]| {
        let s = stand_in8(a, b, off, code);
        redirect(a, b, sym, s);
    };
    // TOGetFFXLang: [cells]
    let mut v = vec![0xa1];
    v.extend_from_slice(&cells.to_le_bytes());
    v.push(0xc3);
    stand(a, b, 0, 0x008a_c2f0, &v);
    // ToGetBtlEasyFontWidth(text, &w, alpha, scale, extra): w = [cells + 4]
    stand(
        a,
        b,
        0x80,
        0x0090_5290,
        &deref_logger_mask(cursor, &[0xc, 0x10, 0x14], Some((8, cells + 4)), u32::MAX),
    );
    // ToMakeBtlEasyEdgeFont(text, x, y, alpha, scale, extra)
    stand(
        a,
        b,
        0x100,
        0x0090_5930,
        &deref_logger_mask(cursor, &[8, 0xc, 0x10, 0x14, 0x18], None, u32::MAX),
    );
    stand(a, b, 0x180, 0x008e_8940, &logger_args(cursor, 4));
    stand(a, b, 0x200, 0x008e_5cd0, &marker(cursor, 0x2910));
    // TOGetFontInfo(text, out): logs the code, out+0x1c = [cells + 8]
    let mut v = deref_logger_mask(cursor, &[], None, u32::MAX);
    v.pop();
    v.push(0xa1);
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.extend_from_slice(&[0x8b, 0x4c, 0x24, 0x08, 0x89, 0x41, 0x1c, 0xc3]);
    stand(a, b, 0x280, 0x008b_7240, &v);
    // TOGetFontAdvance_008fa740(kind, glyph, scale): the glyph's +0x1c
    stand(
        a,
        b,
        0x300,
        0x008f_a740,
        &[0x8b, 0x44, 0x24, 0x08, 0xd9, 0x40, 0x1c, 0xc3],
    );
    stand(a, b, 0x380, 0x0064_3af0, &logger_args(cursor, 1));
    stand(a, b, 0x400, 0x0064_0dd0, &logger_args(cursor, 1));
    stand(a, b, 0x480, 0x008f_ae40, &logger_args(cursor, 3));
    stand(a, b, 0x500, 0x008f_b940, &logger_args(cursor, 3));
    // TOAddFontWidthSpace(pen, glyph, scale): logs, pen x += glyph +0x1c
    let mut v = logger_args(cursor, 3);
    v.truncate(v.len() - 5);
    v.extend_from_slice(&[
        0x8b, 0x44, 0x24, 0x08, 0xd9, 0x40, 0x1c, 0x8b, 0x44, 0x24, 0x04, 0xd8, 0x40, 0x04, 0xd9,
        0x58, 0x04, 0xc3,
    ]);
    stand(a, b, 0x580, 0x008e_8ed0, &v);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let [offsets, hashes, phases, counts] =
        [0x0187_6110, 0x0187_6120, 0x0187_1708, 0x0187_6130].map(|x| a.rebase(x));
    let float = |rng: &mut Xorshift, lo: u32, hi: u32| (lo + rng.next() % (hi - lo)) as f32 / 4.0;
    for k in 0..2500u32 {
        put(a, b, cursor, log);
        put(a, b, cells, [0u32, 1, 8, 9, 0xb][(rng.next() % 5) as usize]);
        let lw = float(rng, 40, 1600);
        put(a, b, cells + 4, lw);
        put(a, b, cells + 8, float(rng, 8, 120));
        let codes = [0x30u8, 0x41, 0x47, 0x47, 0x61, 0x7a, 0x0a, 0x03, 0x01, 0x20];
        let n = (rng.next() % 0x18) as usize;
        let mut bytes = [0u8; 0x40];
        for c in bytes.iter_mut().take(n) {
            *c = codes[(rng.next() % codes.len() as u32) as usize];
        }
        put(a, b, text, bytes);
        for i in 0..4 {
            put(a, b, offsets + 4 * i, -float(rng, 0, 800));
            put(a, b, hashes + 4 * i, rng.next() % 16);
            put(a, b, phases + i, (rng.next() % 4) as u8);
            put(a, b, counts + 4 * i, 45 + rng.next() % 10);
        }
        // edge cases: the text exactly as wide as the box; the marquee
        // reaching the box's end exactly; the first glyph ending exactly at
        // the ellipsis
        let w = match rng.next() % 6 {
            0 => lw,
            _ => float(rng, 40, 1200),
        };
        let slot = rng.next() % 6;
        if rng.next().is_multiple_of(4) {
            let s = slot.min(3);
            put(a, b, offsets + 4 * s, w - lw + 0.5);
            put(a, b, phases + s, 1u8);
        }
        let ew = f32::from_bits(lw.to_bits() + 0x0040_0000);
        let exact = rng.next().is_multiple_of(4) && lw > w;
        if exact {
            put(a, b, cells + 8, w - ew);
            put(a, b, text, [0x41u8, 0x41]);
        }
        let mut args = [
            text,
            float(rng, 0, 2000).to_bits(),
            float(rng, 0, 2000).to_bits(),
            w.to_bits(),
            rng.next(),
            [1.0f32, 0.63][(rng.next() % 2) as usize].to_bits(),
            1.5f32.to_bits(),
            rng.next() % 2,
            [0u32, 1, 1, 2][(rng.next() % 4) as usize],
            slot,
        ];
        if exact {
            // cut to fit, the first glyph's advance (negative here) ending
            // exactly at the ellipsis
            args[7] = 0;
            args[8] = 0;
        }
        eax(a, 0x0090_4ba0, &args);
        t::quick_recover_menu_draw(b, args, scratch, call, fcall);
        let v: [u32; 0x14] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("quick recover {k}"), a, b);
    }
}

#[test]
fn round29_quick_recover_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round29_quick_recover(&mut a, &mut b, &mut Xorshift(0x4029_0011));
}

#[test]
fn round29_quick_recover_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round29_quick_recover(&mut a, &mut b, &mut Xorshift(0x4029_0012));
}

/// A cdecl stand-in copying a tag and its first `n` arguments to `*cursor`
/// (advanced past them), then running `tail` (which may set `eax`) and
/// returning.
fn copy_logger(cursor: u32, tag: u32, n: u8, tail: &[u8]) -> Vec<u8> {
    // push esi; push edi; lea esi, [esp+0xc]; mov edi, [cursor]
    let mut v = vec![0x56, 0x57, 0x8d, 0x74, 0x24, 0x0c, 0x8b, 0x3d];
    v.extend_from_slice(&cursor.to_le_bytes());
    // mov dword [edi], tag; add edi, 4; mov ecx, n; rep movsd
    v.extend_from_slice(&[0xc7, 0x07]);
    v.extend_from_slice(&tag.to_le_bytes());
    v.extend_from_slice(&[0x83, 0xc7, 0x04, 0xb9, n, 0, 0, 0, 0xf3, 0xa5, 0x89, 0x3d]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x5f, 0x5e]);
    v.extend_from_slice(tail);
    v.push(0xc3);
    assert!(v.len() <= 0x80, "a stand-in slot");
    v
}

/// `eax = table[([esp+off] >> shift) & 7]`.
fn table_tail(off: u8, shift: u8, table: u32) -> Vec<u8> {
    let mut v = vec![
        0x8b, 0x44, 0x24, off, 0xc1, 0xe8, shift, 0x83, 0xe0, 0x07, 0x8b, 0x04, 0x85,
    ];
    v.extend_from_slice(&table.to_le_bytes());
    v
}

/// `TOBtlDrawCommandWindow` over stand-ins for the clock, the cosine, the
/// draw calls, the command checks and the getters; the remaps and the
/// truncations are the original's.
#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]
fn round30_command_window(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use cmdwin::bridge as c;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let clock = alloc(a, b, 8);
    let scratch = alloc(a, b, 8);
    let win = alloc(a, b, 0x100);
    let list = alloc(a, b, 0x80);
    let records = alloc(a, b, 8 * 0x20);
    let weapons = alloc(a, b, 8 * 8);
    let tables = alloc(a, b, 6 * 0x20);
    let strings = 0x0123_4000u32;
    let slot = a.rebase(0x00b0_c3b4);
    let fcos = a.rebase(0x00b0_bfc0);
    a.write(fcos, &[0xd9u8, 0xff, 0xc3]).unwrap();
    b.write(fcos, &[0xd9u8, 0xff, 0xc3]).unwrap();
    put(a, b, slot, fcos);
    let stand = |a: &mut Oracle, b: &mut Oracle, k: u32, sym: u32, code: &[u8]| {
        let s = stand_in8(a, b, 0x80 * k, code);
        if sym != 0 {
            redirect(a, b, sym, s);
        }
        s
    };
    // graphicGetTime: fld qword [clock]
    let mut v = vec![0xdd, 0x05];
    v.extend_from_slice(&clock.to_le_bytes());
    v.push(0xc3);
    stand(a, b, 0, 0x0064_1410, &v);
    // fld qword [esp+4]; call [slot]; ret
    let mut v = vec![0xdd, 0x44, 0x24, 0x04, 0xff, 0x15];
    v.extend_from_slice(&slot.to_le_bytes());
    v.push(0xc3);
    let cos_thunk = stand(a, b, 1, 0, &v);
    for (k, sym, n) in [
        (2, 0x0090_3ee0u32, 11u8),
        (3, 0x008e_7d30, 12),
        (4, 0x008e_6cc0, 7),
        (5, 0x008f_dee0, 4),
        (6, 0x008e_6af0, 9),
        (7, 0x0090_55c0, 6),
        (8, 0x0090_4ba0, 10),
    ] {
        stand(a, b, k, sym, &copy_logger(cursor, sym, n, &[]));
    }
    let draw_step = stand(a, b, 9, 0, &copy_logger(cursor, 0x88, 1, &[]));
    // TOCheckBtlCommandUse(chr, command): table 0 by the command
    stand(
        a,
        b,
        10,
        0x0089_aca0,
        &copy_logger(cursor, 0x0089_aca0, 2, &table_tail(8, 0, tables)),
    );
    // MsGetComData(id, &strings): *strings = strings; records + (id & 7) * 0x20
    let mut tail = vec![0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00];
    tail.extend_from_slice(&strings.to_le_bytes());
    tail.extend_from_slice(&[
        0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x07, 0xc1, 0xe0, 0x05, 0x05,
    ]);
    tail.extend_from_slice(&records.to_le_bytes());
    stand(
        a,
        b,
        11,
        0x0079_a4b0,
        &copy_logger(cursor, 0x0079_a4b0, 1, &tail),
    );
    // the counts: tables 1..=4
    for (k, sym, n, off, shift) in [
        (12, 0x0079_04b0u32, 1u8, 4u8, 0u8),
        (13, 0x0078_cf70, 2, 8, 5),
        (14, 0x0079_add0, 1, 4, 0),
        (15, 0x0079_ae50, 1, 4, 0),
    ] {
        let table = tables + 0x20 * (k - 11);
        stand(
            a,
            b,
            k,
            sym,
            &copy_logger(cursor, sym, n, &table_tail(off, shift, table)),
        );
    }
    // MsGetSaveWeapon(id, &x): weapons + (id & 7) * 8
    let mut tail = vec![
        0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x07, 0xc1, 0xe0, 0x03, 0x05,
    ];
    tail.extend_from_slice(&weapons.to_le_bytes());
    stand(
        a,
        b,
        16,
        0x007a_bbd0,
        &copy_logger(cursor, 0x007a_bbd0, 1, &tail),
    );
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let call_at =
        |m: &mut Oracle, at: u32, args: &[u32]| m.call(at, Conv::Cdecl, args).unwrap().eax;
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let cos = |m: &mut Oracle, v: f64| {
        let bits = v.to_bits();
        m.call(cos_thunk, Conv::Cdecl, &[bits as u32, (bits >> 32) as u32])
            .unwrap()
            .st0
            .unwrap()
    };
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    let frame_word = a.stack_range().end - 0x114;
    let pick = |rng: &mut Xorshift, v: &[i32]| v[(rng.next() % v.len() as u32) as usize];
    for k in 0..400u32 {
        put(a, b, cursor, log);
        for t in 0..8 {
            put(
                a,
                b,
                tables + 4 * t,
                pick(rng, &[0, 1, -1, -2, -4, -4, 5, 0x10]),
            );
            for g in 1..5 {
                put(
                    a,
                    b,
                    tables + 0x20 * g + 4 * t,
                    pick(rng, &[-3, 0, 1, 7, 99, 1234, 9999, -1, i32::MAX]),
                );
            }
            put(a, b, records + 0x20 * t, (rng.next() % 0x400) as u16);
            put(a, b, records + 0x20 * t + 0x14, rng.next() as u8);
            put(
                a,
                b,
                weapons + 8 * t + 4,
                [
                    rng.next() as u8 % 4,
                    rng.next() as u8 % 4,
                    rng.next() as u8 % 4,
                ],
            );
        }
        for e in 0..0x40 {
            let id = if rng.next().is_multiple_of(6) {
                0xff
            } else {
                rng.next() % 0x40
            };
            put(a, b, list + 2 * e, id as u16);
        }
        // k = 0: the cursor plate with cos(5t) just under 0.5, which the f32
        // rounding of the cosine lifts to 0.5 (alpha 112, not 111)
        let columns = if k == 0 {
            2
        } else {
            pick(rng, &[1, 2, 2, 2, 3, 4, -1, 1]) as i16
        };
        let count = pick(rng, &[0, 1, 3, 5, 8, 12, 17, 24, 31, -2]);
        let first = (rng.next() % 30) as i16 - 3;
        let at = if k == 0 {
            1
        } else {
            (rng.next() % 16) as i16 - 2
        };
        let pitch = if rng.next().is_multiple_of(8) {
            -((rng.next() % 300) as i16)
        } else {
            100 + (rng.next() % 800) as i16
        };
        let fl = |rng: &mut Xorshift, m: u32, d: f32| (rng.next() % m) as f32 / d;
        // the window's clock: elapsed from -0.2 to 0.7 s, sometimes exactly
        // 0.3 (the ripples' start), not opened yet, or opened in the future
        let now = f64::from(1 + rng.next() % 1000) / 8.0;
        let opened = match rng.next() % 8 {
            0 => 0f32,
            1 => -1.5,
            2 => now as f32 - 0.3,
            _ => (now - f64::from(fl(rng, 9000, 10000.0)) + 0.2) as f32,
        };
        let now = if k == 0 {
            let now = (0.5f64.acos() + 1e-9 / 0.75f64.sqrt()) / 5.0;
            put(a, b, win + 0xdc, opened);
            now
        } else if rng.next().is_multiple_of(10) {
            put(a, b, clock, 1.0f64 + f64::from(0.3f32));
            put(a, b, win + 0xdc, 1.0f32);
            1.0 + f64::from(0.3f32)
        } else {
            put(a, b, win + 0xdc, opened);
            now
        };
        put(a, b, clock, now);
        put(a, b, win + 8, rng.next() as u16);
        put(a, b, win + 0x20, list);
        put(a, b, win + 0x24, count);
        let parts = pick(rng, &[0, 1, 2, 3, 4, 6, 6]) as i16;
        put(a, b, win + 0x28, parts);
        put(a, b, win + 0x2a, (rng.next() % 3) as i16);
        for p in 0..6 {
            let kind = pick(rng, &[1, 2, 3, 4, 5, 6, 7, 0x10, 0x10, 0, 9, 0x11]) as u8;
            put(a, b, win + 0x2c + 2 * p, [rng.next() as u8, kind]);
        }
        put(a, b, win + 0x38, columns);
        put(a, b, win + 0x3a, (1 + rng.next() % 5) as i16);
        put(a, b, win + 0x40, first);
        put(a, b, win + 0x42, first + at);
        put(a, b, win + 0x62, pitch);
        put(a, b, win + 0x66, (50 + rng.next() % 600) as i16);
        for o in [0x68, 0x6c, 0x70, 0x74, 0x7c] {
            put(a, b, win + o, fl(rng, 400_000, 200.0) - 50.0);
        }
        put(a, b, win + 0x88, draw_step);
        put(a, b, win + 0xe0, 0u32);
        put(a, b, win + 0xe4, rng.next() % 2);
        // the original's frame word under the counts' alpha (stack garbage
        // until the panel or a label sets it): planted in its frame
        let garbage = rng.next();
        Mem::write(a, frame_word, garbage);
        put(a, b, scratch + 4, garbage);
        let r = eax(a, 0x0089_f370, &[win]);
        let ours = c::to_btl_draw_command_window(b, win, scratch, call, call_at, fcall, cos, remap);
        assert_eq!(r, ours, "command window {k}");
        let v: [u32; 2] = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("command window {k}"), a, b);
    }
}

#[test]
fn round30_command_window_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round30_command_window(&mut a, &mut b, &mut Xorshift(0x4030_0011));
}

#[test]
fn round30_command_window_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round30_command_window(&mut a, &mut b, &mut Xorshift(0x4030_0012));
}

/// `TOSeqDraw` over a stand-in for `TOMenuExecLayerHandle`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names
)]
fn round30_seq_draw(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x2000);
    let cursor = alloc(a, b, 4);
    // TOMenuExecLayerHandle(i, a, handle, b): logs, returns the handle ^ key
    let mut tail = vec![0x8b, 0x44, 0x24, 0x0c, 0x35];
    tail.extend_from_slice(&0x5a5a_0000u32.to_le_bytes());
    let s = stand_in8(a, b, 0x880, &copy_logger(cursor, 0x00a7_b220, 4, &tail));
    redirect(a, b, 0x00a7_b220, s);
    let slots = a.rebase(0x0186_5f20);
    for k in 0..300u32 {
        put(a, b, cursor, log);
        let i = rng.next() % 3;
        for j in 0..8 {
            let at = slots + 0x100 * i + 0x20 * j;
            put(
                a,
                b,
                at,
                [0u16, 1, 2, 3, 4, 2, 3, 0x102][(rng.next() % 8) as usize],
            );
            put(a, b, at + 0x1c, rng.next());
        }
        let (x, y) = (rng.next(), rng.next());
        let r = eax(a, 0x008a_e2f0, &[i, x, y]);
        assert_eq!(
            r,
            scene::bridge::draw(b, i as i32, x, y, eax),
            "seq draw {k}"
        );
        same_memory(&format!("seq draw {k}"), a, b);
    }
}

#[test]
fn round30_seq_draw_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round30_seq_draw(&mut a, &mut b, &mut Xorshift(0x4030_0021));
}

#[test]
fn round30_seq_draw_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round30_seq_draw(&mut a, &mut b, &mut Xorshift(0x4030_0022));
}

/// `mov eax, [cell]`: a stand-in tail returning a word the test sets.
fn ret_cell(cell: u32) -> Vec<u8> {
    let mut v = vec![0xa1];
    v.extend_from_slice(&cell.to_le_bytes());
    v
}

/// Logging stand-ins at `syms` (tag = the address, `n` arguments), each
/// returning its own word of `rets` (in order); `slot0` is the first
/// stand-in slot used.
fn loggers(a: &mut Oracle, b: &mut Oracle, cursor: u32, rets: u32, slot0: u32, syms: &[(u32, u8)]) {
    for (k, &(sym, n)) in (0u32..).zip(syms) {
        let code = copy_logger(cursor, sym, n, &ret_cell(rets + 4 * k));
        let s = stand_in8(a, b, 0x80 * (slot0 + k), &code);
        redirect(a, b, sym, s);
    }
}

/// The callees of the international-environment reload.
const ROUND31_ENV_CALLEES: [(u32, u8); 12] = [
    (0x0062_f500, 1), // dbgPrintf
    (0x0078_2050, 0), // MsBattleReadData
    (0x0078_2090, 1), // MsBattleReadDataNB
    (0x0089_de30, 2), // TODapsSetPos
    (0x0089_dde0, 2), // TODapsPrintf
    (0x008a_ba30, 3), // TODVDFileRead
    (0x008a_bab0, 3), // TODVDFileStartRead
    (0x008a_bb00, 0), // the CD-busy check
    (0x008a_e5a0, 1), // TOSeqInitAllScene
    (0x008a_e6f0, 0), // TOSeqInitParam
    (0x00a7_c7f0, 2), // TkMenuSetSceneDataAdrs
    (0x0090_6b20, 0), // TOGetFFXToAlbhedTblAdrs
];

/// `TOSwapInternationalEnvExec`, `...ExecMenu` and `...ExecNonBlock` over
/// logging stand-ins (random results), from every step.
#[allow(clippy::cast_possible_truncation)]
fn round31_env(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use intl::bridge as i;
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let n = ROUND31_ENV_CALLEES.len();
    let rets = alloc(a, b, 4 * (n + 1));
    loggers(a, b, cursor, rets, 0, &ROUND31_ENV_CALLEES);
    // GetCD()'s +8: the CD wait of step 0x18
    let cd_wait = stand_in8(
        a,
        b,
        0x80 * n as u32,
        &copy_logger(cursor, 0x0008, 1, &ret_cell(rets + 4 * n as u32)),
    );
    put(a, b, cd::bridge::CD_LIB.ptr(a).addr + 8, cd_wait);
    let b_call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    for k in 0..600u32 {
        put(a, b, cursor, log);
        for j in 0..=n as u32 {
            // mostly 0 (a read finished, no more battle files)
            let r = if rng.next().is_multiple_of(2) {
                0
            } else {
                rng.next()
            };
            put(a, b, rets + 4 * j, r);
        }
        set(a, b, i::ARCHIVE, rng.next() % 0x20);
        set(a, b, i::MENU_ARCHIVE, rng.next() % 0x20);
        set(a, b, i::DONE, rng.next() % 3);
        set(a, b, i::BUSY, rng.next() % 3);
        set(a, b, i::MENU_DONE, rng.next() % 3);
        set(a, b, i::BATTLE_FILE, (rng.next() % 40).cast_signed() - 2);
        set(
            a,
            b,
            i::DEBUG_PRINT,
            [0u8, 0, 1, 3][(rng.next() % 4) as usize],
        );
        for w in [
            0x0184_1d98u32,
            0x0184_1d9c,
            0x0184_1e00,
            0x0184_1e04,
            0x0184_1e08,
        ] {
            put(a, b, a.rebase(w), rng.next());
        }
        let what = match k % 3 {
            0 => {
                set(a, b, i::STEP, rng.next() % 0x20);
                let r = eax(a, 0x008b_0500, &[]);
                assert_eq!(r, i::exec_non_block(b, b_call), "non-block {k}");
                "non-block"
            }
            1 => {
                set(a, b, i::MENU_STEP, rng.next() % 9);
                let r = eax(a, 0x008b_0440, &[]);
                assert_eq!(r, i::exec_menu(b, b_call), "menu {k}");
                "menu"
            }
            _ => {
                let r = eax(a, 0x008b_0340, &[]);
                assert_eq!(r, i::exec(b, b_call), "exec {k}");
                "exec"
            }
        };
        same_memory(&format!("{what} {k}"), a, b);
    }
}

#[test]
fn round31_env_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round31_env(&mut a, &mut b, &mut Xorshift(0x4031_0011));
}

#[test]
fn round31_env_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round31_env(&mut a, &mut b, &mut Xorshift(0x4031_0012));
}

/// The callees of the texture uploads.
const ROUND31_TEX_CALLEES: [(u32, u8); 8] = [
    (0x008b_0b10, 7), // TOTransImageHL
    (0x0078_16f0, 0), // MsBattleGetStatus
    (0x0078_1680, 0), // MsBattleGetFontAddress
    (0x008a_c2f0, 0), // TOGetFFXLang
    (0x0087_3ac0, 0), // AtelGetExpandFTCAdrs
    (0x0077_4670, 0), // GetU32_0112a010
    (0x008a_cf20, 0), // GetU32_01841d30
    (0x008a_c760, 0), // TOGetInternationalBaseDicType
];

/// `TOTransTexture` and its menu form over logging stand-ins.
fn round31_textures(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use intl::bridge as i;
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let rets = alloc(a, b, 4 * ROUND31_TEX_CALLEES.len());
    loggers(a, b, cursor, rets, 0, &ROUND31_TEX_CALLEES);
    let b_call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    let textures = a.rebase(i::TEXTURES);
    for k in 0..400u32 {
        put(a, b, cursor, log);
        put(a, b, rets, rng.next());
        for j in 1..7 {
            put(a, b, rets + 4 * j, rng.next() % 2 * rng.next());
        }
        put(a, b, rets + 28, rng.next() % 6);
        for w in 0..0x18 {
            put(a, b, textures + 4 * w, rng.next());
        }
        for w in [
            0x0184_1d94u32,
            0x0184_1d98,
            0x0184_1d9c,
            0x0184_1e00,
            0x0184_1e04,
            0x0184_1e08,
        ] {
            put(a, b, a.rebase(w), rng.next());
        }
        if k % 2 == 0 {
            let r = eax(a, 0x008b_1020, &[]);
            assert_eq!(r, i::trans_texture(b, b_call), "trans texture {k}");
        } else {
            let r = eax(a, 0x008b_1190, &[]);
            assert_eq!(r, i::trans_texture_menu(b, b_call), "menu textures {k}");
        }
        same_memory(&format!("textures {k}"), a, b);
    }
}

#[test]
fn round31_textures_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round31_textures(&mut a, &mut b, &mut Xorshift(0x4031_0021));
}

#[test]
fn round31_textures_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round31_textures(&mut a, &mut b, &mut Xorshift(0x4031_0022));
}

/// `cd_decode_data` over logging stand-ins for the decoders: small and
/// large files, every method byte, heads moved up and down.
#[allow(clippy::cast_possible_truncation)]
fn round31_decode(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let rets = alloc(a, b, 12);
    loggers(
        a,
        b,
        cursor,
        rets,
        0,
        &[(0x00a4_3de0, 2), (0x00a4_3cb0, 4), (0x00a4_3ca0, 4)],
    );
    let buffer = alloc(a, b, 0x1800);
    let req = alloc(a, b, 0x40);
    let data = alloc(a, b, 0x3800);
    set(a, b, cd::bridge::READ_BUFFER, buffer);
    let b_call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    for k in 0..300u32 {
        put(a, b, cursor, log);
        for j in 0..3 {
            put(a, b, rets + 4 * j, rng.next());
        }
        let size = if k % 20 == 0 {
            0x1800 + k / 20 % 2
        } else if k % 3 == 0 {
            rng.next() % 0x1801
        } else {
            0x1801 + rng.next() % 0x1000
        };
        let packed = size.saturating_sub(0x1800);
        // unpacked: within the block, from below the head to beyond it
        let unpacked = (packed + rng.next() % 0x1000)
            .saturating_sub(0x400)
            .max(packed / 2);
        let bytes: Vec<u8> = (0..0x3800).map(|_| rng.next() as u8).collect();
        a.write(data, &bytes).unwrap();
        b.write(data, &bytes).unwrap();
        put(a, b, data, [0u8, 1, 2, 3, 0xff][(rng.next() % 5) as usize]);
        put(a, b, data + 1, unpacked);
        put(a, b, req + 4, data);
        put(a, b, req + 0x2c, size);
        let r = eax(a, 0x0076_d440, &[req]);
        assert_eq!(r, cd::bridge::decode_data(b, req, b_call), "decode {k}");
        same_memory(&format!("decode {k}"), a, b);
    }
}

#[test]
fn round31_decode_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round31_decode(&mut a, &mut b, &mut Xorshift(0x4031_0031));
}

#[test]
fn round31_decode_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round31_decode(&mut a, &mut b, &mut Xorshift(0x4031_0032));
}

/// A cdecl stand-in logging a tag, the arguments at `args` (stack word
/// indices from 0), and the first word of the text each of `texts` points
/// at; then running `tail` (which may set `eax`) and returning.
fn pick_logger(cursor: u32, tag: u32, args: &[u8], texts: &[u8], tail: &[u8]) -> Vec<u8> {
    // mov ecx, [cursor]; mov dword [ecx], tag
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0xc7, 0x01]);
    v.extend_from_slice(&tag.to_le_bytes());
    let mut at = 4u8;
    for &k in args {
        // mov eax, [esp+4+4k]; mov [ecx+at], eax
        v.extend_from_slice(&[0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, at]);
        at += 4;
    }
    for &k in texts {
        // mov eax, [esp+4+4k]; mov eax, [eax]; mov [ecx+at], eax
        v.extend_from_slice(&[0x8b, 0x44, 0x24, 4 + 4 * k, 0x8b, 0x00, 0x89, 0x41, at]);
        at += 4;
    }
    // add dword [cursor], at
    v.extend_from_slice(&[0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.push(at);
    v.extend_from_slice(tail);
    v.push(0xc3);
    assert!(v.len() <= 0x80, "a stand-in slot");
    v
}

/// `mov edx, [esp+4+4k]; mov eax, [cell]; mov [edx], eax`: a stand-in
/// tail storing a test word through argument `k`.
fn store_through(k: u8, cell: u32) -> Vec<u8> {
    let mut v = vec![0x8b, 0x54, 0x24, 4 + 4 * k, 0xa1];
    v.extend_from_slice(&cell.to_le_bytes());
    v.extend_from_slice(&[0x89, 0x02]);
    v
}

/// `TOLayoutMessageChoices_008b9a60` over random message texts, with
/// stand-ins for the measuring and choice calls (the code walk itself,
/// `TOGetGroup` and `TOSkipCode`, is the game's).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round31_choices(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    let scratch = alloc(a, b, t::CHOICE_SCRATCH_BYTES as usize);
    let w = alloc(a, b, 0x100);
    let window = alloc(a, b, 0x100);
    let lines = alloc(a, b, 0x400) + 0x200;
    let text = alloc(a, b, 0x200);
    // TOGetMessageWH(i, text, &w, &h, mode, flags): w and h from cells
    let mut tail = store_through(2, cells);
    tail.extend_from_slice(&store_through(3, cells + 4));
    let stand = |a: &mut Oracle, b: &mut Oracle, k: u32, sym: u32, code: &[u8]| {
        let s = stand_in8(a, b, 0x80 * k, code);
        redirect(a, b, sym, s);
    };
    stand(
        a,
        b,
        0,
        0x008b_9650,
        &pick_logger(cursor, 0x9650, &[0, 4, 5], &[1], &tail),
    );
    // TOGetFontWidthInterMode(text, mode, &w)
    stand(
        a,
        b,
        1,
        0x008b_7be0,
        &pick_logger(cursor, 0x7be0, &[1], &[0], &store_through(2, cells + 8)),
    );
    // TOConvAlbhedWord(in, out)
    stand(
        a,
        b,
        2,
        0x0090_6780,
        &pick_logger(cursor, 0x6780, &[], &[0], &store_through(1, cells + 0xc)),
    );
    // TOGetButtonIconWidth(b)
    stand(
        a,
        b,
        3,
        0x008b_65e0,
        &pick_logger(cursor, 0x65e0, &[0], &[], &ret_cell(cells + 0x10)),
    );
    // TOIntToMes(value, out, digits, mode)
    stand(
        a,
        b,
        4,
        0x008b_8290,
        &pick_logger(
            cursor,
            0x8290,
            &[0, 2, 3],
            &[],
            &store_through(1, cells + 0x14),
        ),
    );
    // TOSetMessageChoice(i, n, x, y, total)
    stand(
        a,
        b,
        5,
        0x008b_8880,
        &pick_logger(cursor, 0x8880, &[0, 1, 2, 3, 4], &[], &[]),
    );
    // TOGetChangeMacroWidth(i, text, mode): fld dword [cell]
    let mut fld = vec![0xd9, 0x05];
    fld.extend_from_slice(&(cells + 0x18).to_le_bytes());
    stand(
        a,
        b,
        6,
        0x008b_6a30,
        &pick_logger(cursor, 0x6a30, &[0, 1, 2], &[], &fld),
    );
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let map = a.rebase(0x012f_bc40);
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        let i = rng.next() % 8;
        put(a, b, a.rebase(0x0186_8a90) + 4 * i, lines);
        put(a, b, a.rebase(0x0186_76f0) + 4 * i, window);
        let junk: [u32; 0x100] = std::array::from_fn(|_| rng.next());
        put(a, b, lines - 0x200, junk);
        put(a, b, lines + 5, (rng.next() % 4) as u8);
        put(
            a,
            b,
            window + 0xe,
            [1u16, 1, 0, 2][(rng.next() % 4) as usize],
        );
        put(
            a,
            b,
            window + 0x10,
            [0x4000u16, 0, 0x4001][(rng.next() % 3) as usize],
        );
        put(a, b, window + 0x48, (rng.next() % 200) as f32 + 0.25);
        put(a, b, window + 0x50, (rng.next() % 800) as f32 + 0.5);
        put(a, b, cells, (rng.next() % 4000) as f32 / 8.0);
        put(a, b, cells + 4, (rng.next() % 64) as f32);
        put(a, b, cells + 8, (rng.next() % 400) as f32 / 4.0);
        put(a, b, cells + 0xc, rng.next());
        put(a, b, cells + 0x10, rng.next() % 40);
        put(a, b, cells + 0x14, rng.next());
        put(a, b, cells + 0x18, (rng.next() % 900) as f32 / 3.0);
        // the text: line codes, alignment, icons, items, numbers, macros,
        // characters
        let mut bytes = Vec::new();
        for _ in 0..(rng.next() % 24) {
            match rng.next() % 12 {
                0 => bytes.push([1u8, 2, 3][(rng.next() % 3) as usize]),
                1 => bytes.extend_from_slice(&[
                    7,
                    [0x38u8, 0x31, 0x40, 0x7f, 0xbf, 0xc0, 0xe1, 0xef, 0xf0, 0x20]
                        [(rng.next() % 10) as usize],
                ]),
                2 => bytes.extend_from_slice(&[0xb, rng.next() as u8]),
                3 | 4 => bytes.extend_from_slice(&[0x10, (0x30 + rng.next() % 0x20) as u8]),
                5 => bytes.extend_from_slice(&[
                    0x10,
                    [0xffu8, 0x4f, 0x50, 0x55, 0x5f, 0x60, 0x18][(rng.next() % 7) as usize],
                ]),
                6 => bytes.extend_from_slice(&[(0x12 + rng.next() % 0x12) as u8, rng.next() as u8]),
                7 => bytes.extend_from_slice(&[4, rng.next() as u8, rng.next() as u8]),
                _ => bytes.push((0x30 + rng.next() % 0xc0) as u8),
            }
        }
        bytes.push(0);
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        put(a, b, w + 4, i as i16);
        put(a, b, w + 0x1a, (rng.next() % 4) as i16);
        put(a, b, w + 0x94, text);
        let special = rng.next().is_multiple_of(4);
        put(
            a,
            b,
            w + 0x14,
            if special { 0x1c7u16 } else { rng.next() as u16 },
        );
        put(
            a,
            b,
            w + 0x16,
            if special { 199u16 } else { rng.next() as u16 },
        );
        put(
            a,
            b,
            map,
            if rng.next().is_multiple_of(2) {
                0x17u32
            } else {
                rng.next() % 0x40
            },
        );
        let sel = [0u32, 1, 2, 3, 0x13, 0x20][(rng.next() % 6) as usize];
        let r = eax(a, 0x008b_9a60, &[w, sel]);
        assert_eq!(
            r,
            t::layout_message_choices(b, w, sel, scratch, call, fcall),
            "choices {k}"
        );
        let mut s = vec![0u8; t::CHOICE_SCRATCH_BYTES as usize];
        b.read_bytes(scratch, &mut s);
        a.write(scratch, &s).unwrap();
        same_memory(&format!("choices {k}"), a, b);
    }
}

#[test]
fn round31_choices_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round31_choices(&mut a, &mut b, &mut Xorshift(0x4031_0041));
}

#[test]
fn round31_choices_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round31_choices(&mut a, &mut b, &mut Xorshift(0x4031_0042));
}

/// A UI vertex setter's bridge.
type SetPair = fn(&mut Oracle, f32, f32, u32) -> u32;

/// The CTB window's animation steps, the CD channel lookup, the UI vertex
/// setters and `TOMenuTransFacePlyTex`.
#[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
fn round31_small(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let rec = alloc(a, b, 0x20);
    for k in 0..2000u32 {
        let bytes: [u32; 8] = std::array::from_fn(|_| rng.next());
        put(a, b, rec, bytes);
        put(
            a,
            b,
            rec + 0xf,
            ((rng.next() % 5) as u8).wrapping_sub(u8::from(k % 7 == 0)),
        );
        put(
            a,
            b,
            rec + 0xc,
            [0x0fu8, 0x10, 0x11, 0x80, 0x7f, rng.next() as u8][(rng.next() % 6) as usize],
        );
        let (f, which) = [
            (0x008a_2b10u32, 0),
            (0x008a_2bc0, 1),
            (0x008a_2b60, 2),
            (0x008a_2c10, 3),
        ][(k % 4) as usize];
        if which == 2 {
            put(a, b, rec + 0xf, (rng.next() % 4) as u8);
        }
        let r = eax(a, f, &[rec]);
        let s = match which {
            0 => window::bridge::slide_step(b, rec, 0x008a_2b60),
            1 => window::bridge::slide_step(b, rec, 0x008a_2c10),
            2 => window::bridge::anim_step_8a2b60(b, rec),
            _ => window::bridge::anim_wait_8a2c10(b, rec),
        };
        assert_eq!(r, s, "anim step {f:#x} {k}");
        same_memory(&format!("anim step {f:#x} {k}"), a, b);
    }
    // the CD channels (sometimes only the last one matches)
    let channels = cd::bridge::CHANNELS.ptr(a).addr;
    for k in 0..400u32 {
        let (file, archive) = (rng.next() % 2, rng.next() % 2);
        let last_only = k % 8 == 0;
        for c in 0..64u32 {
            let at = channels + c * 0x38;
            put(a, b, at + 8, rng.next() % 2);
            put(a, b, at + 0x28, rng.next() % 2);
            let used = if last_only {
                c == 63
            } else {
                rng.next().is_multiple_of(3)
            };
            put(a, b, at + 0x31, u8::from(used));
        }
        if last_only {
            put(a, b, channels + 63 * 0x38 + 8, archive);
            put(a, b, channels + 63 * 0x38 + 0x28, file);
        }
        set(a, b, cd::bridge::READING, rng.next() % 2);
        let reading = if last_only { 63 } else { rng.next() % 16 };
        set(a, b, cd::bridge::READING_CHANNEL, reading);
        assert_eq!(
            eax(a, 0x0076_d0e0, &[file, archive]),
            cd::bridge::channel_state(b, file, archive),
            "channel state {k}"
        );
    }
    // the UI vertices (no signalling NaNs: the emulator keeps them)
    let quiet = |rng: &mut Xorshift| loop {
        let f = rng.float();
        if !(f.is_nan() && f.to_bits() & 0x0040_0000 == 0) {
            return f;
        }
    };
    for k in 0..400u32 {
        let (x, y, n) = (quiet(rng), quiet(rng), rng.next() % 4);
        let (f, set_pair): (u32, SetPair) = if k % 2 == 0 {
            (0x0090_66d0, render::bridge::set_ui_vertex_xy)
        } else {
            (0x0090_66b0, render::bridge::set_ui_vertex_uv)
        };
        let r = eax(a, f, &[x.to_bits(), y.to_bits(), n]);
        assert_eq!(r, set_pair(b, x, y, n), "ui vertex {k}");
        same_memory(&format!("ui vertex {k}"), a, b);
    }
    // TOMenuTransFacePlyTex over a logging TOTransImageHL
    let log = alloc(a, b, 0x40);
    let cursor = alloc(a, b, 4);
    let ret = alloc(a, b, 4);
    let s = stand_in8(a, b, 0, &copy_logger(cursor, 0x0b10, 7, &ret_cell(ret)));
    redirect(a, b, 0x008b_0b10, s);
    let b_call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    for k in 0..4 {
        put(a, b, cursor, log);
        put(a, b, ret, rng.next());
        let r = eax(a, 0x0090_1100, &[]);
        assert_eq!(
            r,
            intl::bridge::trans_face_ply_tex(b, b_call),
            "face texture {k}"
        );
        same_memory(&format!("face texture {k}"), a, b);
    }
}

#[test]
fn round31_small_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round31_small(&mut a, &mut b, &mut Xorshift(0x4031_0051));
}

#[test]
fn round31_small_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round31_small(&mut a, &mut b, &mut Xorshift(0x4031_0052));
}

/// A stand-in: the symbol, the arguments it logs, the arguments it logs
/// by their first word, and its tail.
type Stand = (u32, Vec<u8>, Vec<u8>, Vec<u8>);

/// `TOMenuDrawChrStatusPlate_008ff490` over the original remaps, with
/// logging stand-ins for the shapes, the water, the getters, the texts
/// and numbers (pointer arguments logged by what they point at).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round32_chr_plate(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use chr_plate::bridge as p;
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x80);
    let scratch = alloc(a, b, p::SCRATCH_BYTES as usize);
    // (sym, logged arguments, arguments logged by their first word, tail)
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x0090_3bb0, all(9), vec![], vec![]),
        (0x0090_3ee0, all(11), vec![], vec![]),
        (0x008e_7d30, all(12), vec![], vec![]),
        (0x008a_9b60, vec![], vec![], ret_cell(cells)),
        (0x008a_c2f0, vec![], vec![], ret_cell(cells + 4)),
        (0x008a_98c0, all(1), vec![], ret_cell(cells + 8)),
        (0x008a_9990, all(1), vec![], ret_cell(cells + 0xc)),
        (0x008a_9970, all(1), vec![], ret_cell(cells + 0x10)),
        (0x008a_99b0, all(1), vec![], ret_cell(cells + 0x14)),
        (0x008a_9b80, all(1), vec![], ret_cell(cells + 0x18)),
        (0x008a_9930, all(1), vec![], ret_cell(cells + 0x1c)),
        (0x008a_9950, all(1), vec![], ret_cell(cells + 0x20)),
        (0x008a_9a70, all(1), vec![], ret_cell(cells + 0x24)),
        (0x008a_c850, all(1), vec![], ret_cell(cells + 0x28)),
        (0x0090_5930, all(6), vec![], ret_cell(cells + 0x2c)),
        (
            0x0090_1660,
            vec![1, 2, 3, 4, 5],
            vec![0],
            ret_cell(cells + 0x30),
        ),
        (0x008c_eb00, all(3), vec![], ret_cell(cells + 0x34)),
        (
            0x0090_5230,
            vec![0, 2, 3],
            vec![],
            store_through(1, cells + 0x38),
        ),
        (0x0090_5820, all(6), vec![], ret_cell(cells + 0x3c)),
        (0x0090_55c0, all(6), vec![], ret_cell(cells + 0x40)),
        (0x0078_4e90, all(1), vec![], ret_cell(cells + 0x44)),
        (0x0078_52b0, all(1), vec![], ret_cell(cells + 0x48)),
        (0x008d_3690, all(6), vec![], ret_cell(cells + 0x4c)),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    // sprintf through its import slot: logs the format and the two
    // numbers, writes a word of text
    let k = stands.len() as u32;
    let s = stand_in8(
        a,
        b,
        0x80 * k,
        &pick_logger(
            cursor,
            0x5f,
            &[1, 2, 3],
            &[],
            &store_through(0, cells + 0x50),
        ),
    );
    put(a, b, a.rebase(0x00b0_c3dc), s);
    let call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    for k in 0..800u32 {
        put(a, b, cursor, log);
        for j in 0..0x15 {
            put(a, b, cells + 4 * j, rng.next());
        }
        put(
            a,
            b,
            cells,
            [0u32, u32::MAX, rng.next()][(rng.next() % 3) as usize],
        );
        put(
            a,
            b,
            cells + 4,
            [0xbu32, 0xb, 0, 1, 2][(rng.next() % 5) as usize],
        );
        put(
            a,
            b,
            cells + 8,
            [0u32, 1, 9999, u32::MAX, 0x8000_0000][(rng.next() % 5) as usize],
        );
        put(a, b, cells + 0x38, (rng.next() % 2000) as f32 / 8.0);
        put(a, b, cells + 0x50, rng.next() & 0x00ff_ffff);
        let chr = [
            0u32,
            3,
            7,
            8,
            12,
            0x11,
            0x12,
            0x20,
            u32::MAX,
            (rng.next() % 0x14),
        ][(rng.next() % 10) as usize];
        let x = ((rng.next() % 8000) as f32 / 7.0 - 100.0).to_bits();
        let y = ((rng.next() % 6000) as f32 / 3.0 - 50.0).to_bits();
        let r = eax(a, 0x008f_f490, &[chr, x, y]);
        assert_eq!(
            r,
            p::draw_chr_status_plate(b, [chr, x, y], scratch, remap, call),
            "chr plate {k}"
        );
        let s: [u32; 9] = Mem::read(b, scratch);
        Mem::write(a, scratch, s);
        same_memory(&format!("chr plate {k} chr {chr:#x}"), a, b);
    }
}

#[test]
fn round32_chr_plate_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round32_chr_plate(&mut a, &mut b, &mut Xorshift(0x4032_0011));
}

#[test]
fn round32_chr_plate_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round32_chr_plate(&mut a, &mut b, &mut Xorshift(0x4032_0012));
}

/// Round 32's small functions: `TOSeqStopExecScene`, the sheet cell UVs
/// (`0x00684b00`), `LocalizationManager_Initialize`, the 64-bit program
/// keys, `MagicFile_GetEffectOverlayTable` and the command icon/name
/// packet (`0x008f0590`).
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round32_small(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    // TOSeqStopExecScene
    let slots = scene::bridge::SLOTS.ptr(a).addr;
    for k in 0..300u32 {
        let i = rng.next() % 3;
        for j in 0..8 {
            let at = slots + 0x100 * i + 0x20 * j;
            put(
                a,
                b,
                at,
                [0u16, 1, 2, 3, 4, 2, 3][(rng.next() % 7) as usize],
            );
            put(a, b, at + 2, [0u16, 1, 1, 2][(rng.next() % 4) as usize]);
            put(a, b, at + 8, rng.next());
        }
        assert_eq!(
            eax(a, 0x008a_f380, &[i]),
            scene::bridge::stop_exec_scene(b, i.cast_signed()),
            "stop scene {k}"
        );
        same_memory(&format!("stop scene {k}"), a, b);
    }
    // the sheet cell UVs (stdcall; no signalling NaNs)
    let out = alloc(a, b, 8);
    for k in 0..400u32 {
        let kind = [
            0x3700u32,
            0x3720,
            0x3740,
            0x3760,
            0x37a0,
            0x3780,
            0x36ff,
            0x37a1,
            rng.next(),
        ][(rng.next() % 9) as usize];
        let quiet = |rng: &mut Xorshift| loop {
            let f = rng.float();
            if !(f.is_nan() && f.to_bits() & 0x0040_0000 == 0) {
                return f;
            }
        };
        let (mut u, mut v) = (quiet(rng), quiet(rng));
        if k % 7 == 0 {
            // -0 shows where an offset is added
            (u, v) = (-0.0, -0.0);
        }
        let r = call(
            a,
            0x0068_4b00,
            Conv::Stdcall,
            &[kind, u.to_bits(), v.to_bits(), out, out + 4],
        )
        .eax;
        assert_eq!(
            r,
            render::bridge::sheet_cell_uv(b, kind, u, v, [out, out + 4]),
            "cell uv {k}"
        );
        same_memory(&format!("cell uv {k}"), a, b);
    }
    // LocalizationManager_Initialize
    let this = alloc(a, b, 0x10);
    let settings = alloc(a, b, 4);
    set(a, b, config::bridge::SETTINGS, settings);
    for k in 0..30u32 {
        put(a, b, settings, [k % 10, u32::MAX][usize::from(k == 29)]);
        let words: [u32; 4] = std::array::from_fn(|_| rng.next());
        put(a, b, this, words);
        let at = a.rebase(0x006d_b090);
        let r = a.call(at, Conv::Thiscall(this), &[]).unwrap().eax;
        assert_eq!(
            r,
            config::bridge::localization_initialize(b, this),
            "localization {k}"
        );
        same_memory(&format!("localization {k}"), a, b);
    }
    // the 64-bit keys: edx:eax
    for k in 0..200u32 {
        let (hi, lo) = (
            rng.next(),
            [0u32, 1, u32::MAX, 0x8000_0000, rng.next()][(rng.next() % 5) as usize],
        );
        set(a, b, math::bridge::PPP_CURRENT_PROGRAM, lo);
        let r = call(a, 0x009d_b250, Conv::Cdecl, &[hi, lo]);
        assert_eq!(
            u64::from(r.edx) << 32 | u64::from(r.eax),
            math::bridge::wide_key(hi, lo),
            "key {k}"
        );
        let r = call(a, 0x009d_b230, Conv::Cdecl, &[hi]);
        assert_eq!(
            u64::from(r.edx) << 32 | u64::from(r.eax),
            math::bridge::current_program_key(b, hi),
            "program key {k}"
        );
    }
    // MagicFile_GetEffectOverlayTable over a stand-in table function
    let cell = alloc(a, b, 4);
    let f = stand_in8(a, b, 0, &{
        let mut v = ret_cell(cell);
        v.push(0xc3);
        v
    });
    put(a, b, a.rebase(0x022f_b6a4), f);
    let b_call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    for k in 0..40u32 {
        put(a, b, cell, rng.next());
        put(
            a,
            b,
            a.rebase(0x00c6_4ca0),
            [u32::MAX, 0, 7][(rng.next() % 3) as usize],
        );
        put(
            a,
            b,
            a.rebase(0x00c6_4cac),
            [u32::MAX, 0, 3][(rng.next() % 3) as usize],
        );
        assert_eq!(
            eax(a, 0x009d_a370, &[]),
            stub::bridge::magic_file_effect_overlay_table(b, b_call),
            "overlay table {k}"
        );
    }
    // the command icon and name over logging stand-ins
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let record = alloc(a, b, 0x20);
    let scratch = alloc(a, b, 4);
    // MsGetComData(id, &strings): *strings = [cells + 8]; record
    let mut tail = store_through(1, cells + 8);
    tail.extend_from_slice(&ret_cell(cells + 0xc));
    let s = stand_in8(a, b, 0x80, &pick_logger(cursor, 0xa4b0, &[0], &[], &tail));
    redirect(a, b, 0x0079_a4b0, s);
    let s = stand_in8(
        a,
        b,
        0x100,
        &pick_logger(cursor, 0xb240, &[0, 1, 2, 3], &[], &ret_cell(cells)),
    );
    redirect(a, b, 0x008e_b240, s);
    let s = stand_in8(
        a,
        b,
        0x180,
        &pick_logger(
            cursor,
            0x8fb0,
            &(0..12).collect::<Vec<u8>>(),
            &[],
            &ret_cell(cells + 4),
        ),
    );
    redirect(a, b, 0x008e_8fb0, s);
    put(a, b, cells + 0xc, record);
    let call_sym = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..200u32 {
        put(a, b, cursor, log);
        put(a, b, cells, rng.next());
        put(a, b, cells + 4, rng.next());
        put(a, b, cells + 8, rng.next());
        let bytes: [u32; 8] = std::array::from_fn(|_| rng.next());
        put(a, b, record, bytes);
        let args = [
            rng.next(),
            rng.next() % 0x400,
            rng.next() % 2000,
            rng.next() % 2000,
            rng.next(),
        ];
        let r = eax(a, 0x008f_0590, &args);
        assert_eq!(
            r,
            pkt::bridge::add_command_icon_name(b, args, scratch, call_sym),
            "icon name {k}"
        );
        let v: u32 = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("icon name {k}"), a, b);
    }
}

#[test]
fn round32_small_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round32_small(&mut a, &mut b, &mut Xorshift(0x4032_0021));
}

#[test]
fn round32_small_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round32_small(&mut a, &mut b, &mut Xorshift(0x4032_0022));
}

/// `sce::Vectormath::Scalar::Aos::inverse` on random matrices (mostly
/// ordinary floats, some special values; no signalling NaNs).
fn round32_inverse(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let input = alloc(a, b, 0x40);
    let out = alloc(a, b, 0x40);
    for k in 0..20000u32 {
        // every third matrix spreads its exponents over 2^-60..2^60, so
        // the sums round and their order shows
        let wide = k % 3 == 0;
        let m: [f32; 16] = std::array::from_fn(|_| {
            loop {
                let f = if wide {
                    let e = rng.next() % 120 + 67;
                    f32::from_bits(rng.next() & 0x807f_ffff | e << 23)
                } else {
                    rng.float()
                };
                if !(f.is_nan() && f.to_bits() & 0x0040_0000 == 0) {
                    break f;
                }
            }
        });
        let mut m = m;
        if k % 3 == 1 {
            // nearly singular: column 3 is nearly column 0, so the cofactor
            // sums cancel and their rounding order shows
            for r in 0..4 {
                let nudge = f32::from_bits(rng.next() % 0x40 + 0x3380_0000);
                m[12 + r] = m[r] * (1.0 + nudge) + m[4 + r] * nudge;
            }
        }
        put(a, b, input, m);
        let to = if k % 5 == 0 { input } else { out };
        assert_eq!(
            eax(a, 0x0057_3d00, &[to, input]),
            math::bridge::inverse4(b, to, input),
            "inverse {k}"
        );
        same_memory(&format!("inverse {k}"), a, b);
    }
}

#[test]
fn round32_inverse_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round32_inverse(&mut a, &mut b, &mut Xorshift(0x4032_0031));
}

#[test]
fn round32_inverse_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round32_inverse(&mut a, &mut b, &mut Xorshift(0x4032_0032));
}

/// `0x006851d0`, the area-map manager's layer reset (thiscall).
fn round32_abmap_layers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let this = alloc(a, b, 0x100);
    let lists = alloc(a, b, 6 * 0x100);
    let entries = alloc(a, b, 6 * 8 * 0x6c);
    for k in 0..400u32 {
        for (j, off) in [0x94u32, 0xc8, 0xd0, 0xd4, 0xd8, 0xdc]
            .into_iter()
            .enumerate()
        {
            let j = u32::try_from(j).unwrap();
            let l = lists + 0x100 * j;
            put(a, b, this + off, l);
            put(
                a,
                b,
                l + 0x90,
                [0i32, 1, 3, 8, -1][(rng.next() % 5) as usize],
            );
            put(a, b, l + 0x94, entries + 8 * 0x6c * j);
        }
        let junk: [u32; 0x510] = std::array::from_fn(|_| rng.next());
        put(a, b, entries, junk);
        put(
            a,
            b,
            this + 8,
            [0i32, 1, -1, 7, i32::MIN][(rng.next() % 5) as usize],
        );
        let mode = [0u32, 0, 1, 2, 3, 4, u32::MAX][(rng.next() % 7) as usize];
        let at = a.rebase(0x0068_51d0);
        let r = a.call(at, Conv::Thiscall(this), &[mode]).unwrap().eax;
        assert_eq!(
            r,
            render::bridge::abmap_reset_layers(b, this, mode),
            "layers {k}"
        );
        same_memory(&format!("layers {k}"), a, b);
    }
}

#[test]
fn round32_abmap_layers_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round32_abmap_layers(&mut a, &mut b, &mut Xorshift(0x4032_0041));
}

#[test]
fn round32_abmap_layers_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round32_abmap_layers(&mut a, &mut b, &mut Xorshift(0x4032_0042));
}

/// Round 32 group C: `TOSeqDrawLayer`, `TOMenuTransFaceSmnTex`, the
/// identity at `+0x34`, the VU0 scale and the text scroll clock.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round32_more(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x400);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    // TOMenuExecLayerHandle (under TOSeqDraw) and TOTransImageHL: loggers
    let s = stand_in8(a, b, 0, &copy_logger(cursor, 0xb220, 4, &ret_cell(cells)));
    redirect(a, b, 0x00a7_b220, s);
    let s = stand_in8(
        a,
        b,
        0x80,
        &copy_logger(cursor, 0x0b10, 7, &ret_cell(cells + 4)),
    );
    redirect(a, b, 0x008b_0b10, s);
    // Tk_GetScale(which): fld dword [cells + 8]; TOGetFFXVideoMode: [cells + 0xc]
    let mut fld = vec![0xd9, 0x05];
    fld.extend_from_slice(&(cells + 8).to_le_bytes());
    let s = stand_in8(a, b, 0x100, &pick_logger(cursor, 0x2360, &[0], &[], &fld));
    redirect(a, b, 0x0087_2360, s);
    let s = stand_in8(
        a,
        b,
        0x180,
        &pick_logger(cursor, 0xc3a0, &[], &[], &ret_cell(cells + 0xc)),
    );
    redirect(a, b, 0x008a_c3a0, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let b_call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let slots = scene::bridge::SLOTS.ptr(a).addr;
    for k in 0..200u32 {
        put(a, b, cursor, log);
        put(a, b, cells, rng.next());
        let i = rng.next() % 3;
        for j in 0..8 {
            let at = slots + 0x100 * i + 0x20 * j;
            put(a, b, at, [0u16, 2, 3, 4][(rng.next() % 4) as usize]);
            put(a, b, at + 0x1c, rng.next());
        }
        let x = rng.next();
        assert_eq!(
            eax(a, 0x008a_e340, &[i, x]),
            scene::bridge::draw_layer(b, i.cast_signed(), x, call),
            "draw layer {k}"
        );
        same_memory(&format!("draw layer {k}"), a, b);
    }
    for k in 0..4 {
        put(a, b, cursor, log);
        put(a, b, cells + 4, rng.next());
        assert_eq!(
            eax(a, 0x0090_1130, &[]),
            intl::bridge::trans_face_smn_tex(b, b_call),
            "smn tex {k}"
        );
        same_memory(&format!("smn tex {k}"), a, b);
    }
    // the identity at +0x34 (thiscall)
    let this = alloc(a, b, 0x80);
    for k in 0..10 {
        let junk: [u32; 0x20] = std::array::from_fn(|_| rng.next());
        put(a, b, this, junk);
        let at = a.rebase(0x0061_a810);
        a.call(at, Conv::Thiscall(this), &[]).unwrap();
        math::bridge::identity_at_34(b, this);
        same_memory(&format!("identity {k}"), a, b);
    }
    // the VU0 scale (no signalling NaNs)
    let quiet = |rng: &mut Xorshift| loop {
        let f = rng.float();
        if !(f.is_nan() && f.to_bits() & 0x0040_0000 == 0) {
            return f;
        }
    };
    let v = alloc(a, b, 0x10);
    let out = alloc(a, b, 0x10);
    for k in 0..500u32 {
        let words: [u32; 4] = std::array::from_fn(|_| quiet(rng).to_bits());
        put(a, b, v, words);
        let s = quiet(rng);
        let to = if k % 4 == 0 { v } else { out };
        assert_eq!(
            eax(a, 0x009d_bd90, &[to, s.to_bits(), v]),
            math::bridge::vu0_scale_xyz(b, to, s, v),
            "vu0 scale {k}"
        );
        same_memory(&format!("vu0 scale {k}"), a, b);
    }
    // the text scroll clock
    let w = alloc(a, b, 0x100);
    for k in 0..600u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells + 8,
            [1.0f32, 0.5, 1.25, 2.0, (rng.next() % 1000) as f32 / 7.0][(rng.next() % 5) as usize],
        );
        put(a, b, cells + 0xc, rng.next() % 2);
        put(
            a,
            b,
            w + 0xb4,
            [0u32, 0, 0, 1, 5, u32::MAX][(rng.next() % 6) as usize],
        );
        put(a, b, w + 0xac, (rng.next() % 200).cast_signed() - 20);
        put(a, b, w + 0xb0, (rng.next() % 100).cast_signed() - 10);
        put(a, b, w + 0xa0, rng.next() % 50);
        if k % 10 == 0 {
            // exactly one line's worth
            put(a, b, cells + 8, 1.0f32);
            put(a, b, w + 0xb4, 0u32);
            put(a, b, w + 0xac, 10i32);
            put(a, b, w + 0xb0, [20i32, 15][(rng.next() % 2) as usize]);
        }
        assert_eq!(
            eax(a, 0x008d_f570, &[w]),
            tolib::bridge::text_scroll_tick(b, w, call, fcall),
            "scroll tick {k}"
        );
        same_memory(&format!("scroll tick {k}"), a, b);
    }
}

#[test]
fn round32_more_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round32_more(&mut a, &mut b, &mut Xorshift(0x4032_0051));
}

#[test]
fn round32_more_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round32_more(&mut a, &mut b, &mut Xorshift(0x4032_0052));
}

/// Round 33 group A: `updateVFXKeThSetPoint` and the two cluster fixup
/// unpackers.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names
)]
fn round33_keth_and_clusters(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    // no signalling NaNs (the oracle keeps them signalling through fld)
    let quiet = |rng: &mut Xorshift| loop {
        let f = rng.float();
        if !(f.is_nan() && f.to_bits() & 0x0040_0000 == 0) {
            return f.to_bits();
        }
    };
    if synthetic {
        for (pos, col) in render::bridge::KETH_POINT_TABLES {
            for k in 0..0x30 * 4 {
                let w = quiet(rng);
                put(a, b, a.rebase(pos) - 0x40 + 4 * k, w);
                let w = quiet(rng);
                put(a, b, a.rebase(col) - 0x40 + 4 * k, w);
            }
        }
    }
    let mesh = alloc(a, b, 0x40);
    let pos = alloc(a, b, 0x400);
    let col = alloc(a, b, 0x400);
    let col2 = alloc(a, b, 0x400);
    for k in 0..400u32 {
        let i = (rng.next() % 44) as i32 - 3;
        put(a, b, mesh + 0xc, pos + 0x40);
        put(a, b, mesh + 0x14, col + 0x40);
        let mut m = mesh;
        if k % 8 == 7 && i >= 0 {
            // a position store lands on [mesh + 0x14]: the colour base is
            // read after it
            let word = rng.next() % 3;
            m = pos + 0x40 + 12 * i.cast_unsigned() + 4 * word - 0x14;
            put(a, b, m + 0xc, pos + 0x40);
            put(a, b, m + 0x14, col + 0x40);
            let (table, row) = match i {
                ..6 => (0, i),
                6..12 => (1, i - 6),
                _ => (2, i - 12),
            };
            let src =
                a.rebase(render::bridge::KETH_POINT_TABLES[table].0) + 16 * row.cast_unsigned();
            put(a, b, src + 4 * word, col2 + 0x40);
        }
        assert_eq!(
            eax(a, 0x0076_69c0, &[m, i.cast_unsigned()]),
            render::bridge::keth_set_point(b, m, i),
            "keth point {k} ({i})"
        );
        same_memory(&format!("keth point {k} ({i})"), a, b);
    }
    // the unpackers over logging stand-ins
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let s = stand_in8(a, b, 0, &copy_logger(cursor, 0xa340, 4, &[]));
    redirect(a, b, fixup::bridge::UNPACK_ARRAY_LIST, s);
    let s = stand_in8(a, b, 0x80, &copy_logger(cursor, 0xa750, 4, &[]));
    redirect(a, b, fixup::bridge::UNPACK_LIST, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let out = alloc(a, b, 8);
    let lists = alloc(a, b, 8);
    let headers = alloc(a, b, 0x24 * 8);
    for k in 0..300u32 {
        put(a, b, cursor, log);
        let n = (rng.next() % 7) | [0, 0x8000_0000][(rng.next() % 2) as usize];
        put(a, b, lists, n);
        put(a, b, lists + 4, headers);
        put(a, b, out + 4, rng.next());
        for w in 0..0x24 * 2 {
            put(a, b, headers + 4 * w, rng.next());
        }
        let c = rng.next();
        if k % 2 == 0 {
            let flag = [0u32, 1, 0x100, 0xffff_ff00, rng.next()][(rng.next() % 5) as usize];
            assert_eq!(
                eax(a, 0x0060_dac0, &[out, c, lists, flag]),
                fixup::bridge::unpack_array_fixups(b, out, c, lists, flag, call),
                "array fixups {k}"
            );
        } else {
            assert_eq!(
                eax(a, 0x0060_db20, &[out, c, lists]),
                fixup::bridge::unpack_pointer_fixups(b, out, c, lists, call),
                "pointer fixups {k}"
            );
        }
        same_memory(&format!("cluster fixups {k}"), a, b);
    }
}

#[test]
fn round33_keth_and_clusters_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round33_keth_and_clusters(&mut a, &mut b, &mut Xorshift(0x4033_0011), true);
}

#[test]
fn round33_keth_and_clusters_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round33_keth_and_clusters(&mut a, &mut b, &mut Xorshift(0x4033_0012), false);
}

/// Round 33 group B: `PcFileName_00a42c00` over stand-ins for
/// `Sg_GetUserName2`, `ReturnGlobalSx8_006bc960` and `dbgPrintf`. No
/// empty substitution pattern: one makes the original loop forever (it
/// matches without advancing), and it is the only way to reach the
/// scan's "at the NUL" branch. Heads stay under the original's 256-byte
/// local.
#[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
fn round33_pc_file_name(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, real_table: bool) {
    let log = alloc(a, b, 0x2000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    // the format only: the "file name NULL" print passes no second word
    let s = stand_in8(a, b, 0x200, &copy_logger(cursor, 0xf500, 1, &[]));
    redirect(a, b, cd::bridge::DBG_PRINTF, s);
    let s = stand_in8(
        a,
        b,
        0x280,
        &copy_logger(cursor, 0x0620, 0, &ret_cell(cells)),
    );
    redirect(a, b, cd::bridge::SG_GET_USER_NAME2, s);
    let s = stand_in8(
        a,
        b,
        0x300,
        &copy_logger(cursor, 0xc960, 0, &ret_cell(cells + 4)),
    );
    redirect(a, b, cd::bridge::RETURN_GLOBAL_SX8, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let cd_table = cd::bridge::get_cd(a) + 0x30;
    let strings = alloc(a, b, 0x400);
    let text = |a: &mut Oracle, b: &mut Oracle, at: &mut u32, s: &[u8]| {
        let p = *at;
        a.write(p, s).unwrap();
        b.write(p, s).unwrap();
        put(a, b, p + s.len() as u32, 0u8);
        *at += s.len() as u32 + 1;
        p
    };
    let mut at = strings;
    let users = [
        text(a, b, &mut at, b"tidus"),
        text(a, b, &mut at, b""),
        text(a, b, &mut at, b"x"),
    ];
    let pattern_texts: [&[u8]; 6] = [b"ab", b"c", b"nagi", b"$USE", b"bc", b"0000"];
    let patterns: Vec<u32> = pattern_texts
        .iter()
        .map(|s| text(a, b, &mut at, s))
        .collect();
    let replacement_texts: [&[u8]; 4] = [b"Q", b"", b"long_", b"ab"];
    let replacements: Vec<u32> = replacement_texts
        .iter()
        .map(|s| text(a, b, &mut at, s))
        .collect();
    let pieces: [&[u8]; 12] = [
        b"ab",
        b"c",
        b"$USER$",
        b"$USE",
        b"$USER",
        b"/x/",
        b"nagi0000.ebp",
        b"kami0400.ebp",
        b"nagi0700.ebp",
        b"bc",
        b"0000",
        b"nagi",
    ];
    let set_flags = |a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift| {
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells,
            [0, users[0], users[1], users[2]][(rng.next() % 4) as usize],
        );
        put(
            a,
            b,
            cells + 4,
            [0u32, 1, 0xffff_ff80][(rng.next() % 3) as usize],
        );
        set(a, b, cd::bridge::THUNDER_200, rng.next() % 2);
        set(a, b, cd::bridge::FULL_ARENA_LOCALIZATION, rng.next() % 2);
        for t in 0..16 {
            let p = if rng.next().is_multiple_of(3) {
                (
                    patterns[(rng.next() % 6) as usize],
                    replacements[(rng.next() % 4) as usize],
                )
            } else {
                (0, rng.next())
            };
            put(a, b, cd::bridge::CHGNAME_FROM.ptr(a).addr + 4 * t, p.0);
            put(a, b, cd::bridge::CHGNAME_TO.ptr(a).addr + 4 * t, p.1);
        }
    };
    if real_table {
        // the snapshot's own table, with and without substitutions
        let table: u32 = Mem::read(a, cd_table);
        let count: i32 = if table == 0 { 0 } else { Mem::read(a, table) };
        for k in 0..count.clamp(0, 400) {
            set_flags(a, b, rng);
            if k % 2 == 0 {
                for t in 0..16 {
                    put(a, b, cd::bridge::CHGNAME_FROM.ptr(a).addr + 4 * t, 0u32);
                }
            }
            assert_eq!(
                eax(a, 0x00a4_2c00, &[k.cast_unsigned()]),
                cd::bridge::pc_file_name(b, k, call),
                "real name {k}"
            );
            same_memory(&format!("real name {k}"), a, b);
        }
    }
    let table = alloc(a, b, 0x800) + 0x20;
    for k in 0..600u32 {
        set_flags(a, b, rng);
        let n = rng.next() % 6 + 1;
        put(a, b, table, n);
        let mut off = 4 * (n + 3);
        let mut at = table + off;
        for e in 0..=n {
            put(a, b, table + 4 + 4 * e, off);
            if e == n {
                break;
            }
            // the byte before an entry: 0 or not
            put(a, b, at - 1, (rng.next() % 2) as u8 * b'z');
            let mut name = Vec::new();
            for _ in 0..rng.next() % 6 {
                name.extend_from_slice(pieces[(rng.next() % 12) as usize]);
            }
            name.push(0);
            a.write(at, &name).unwrap();
            b.write(at, &name).unwrap();
            at += name.len() as u32 + 1;
            off += name.len() as u32 + 1;
        }
        let mut loc: i32 = (rng.next() % (n + 3)).cast_signed() - 1;
        match k % 20 {
            // an empty entry and a reversed one
            3 => put(a, b, table + 8, Mem::read::<u32>(a, table + 4)),
            4 => put(a, b, table + 8, Mem::read::<u32>(a, table + 4) - 1),
            // an entry at address 0
            5 => put(a, b, table + 4, table.wrapping_neg()),
            // no table
            6 => put(a, b, cd_table, 0u32),
            _ => put(a, b, cd_table, table),
        }
        if (3..=5).contains(&(k % 20)) {
            loc = 0;
            put(a, b, cd_table, table);
        }
        assert_eq!(
            eax(a, 0x00a4_2c00, &[loc.cast_unsigned()]),
            cd::bridge::pc_file_name(b, loc, call),
            "file name {k} ({loc})"
        );
        same_memory(&format!("file name {k} ({loc})"), a, b);
    }
}

#[test]
fn round33_pc_file_name_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round33_pc_file_name(&mut a, &mut b, &mut Xorshift(0x4033_0021), false);
}

#[test]
fn round33_pc_file_name_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round33_pc_file_name(&mut a, &mut b, &mut Xorshift(0x4033_0022), true);
}

/// Round 33 group C: `TOSeqCtrl` (the scene stand-ins log their arguments;
/// `TkMenuStartScene`'s also bumps slot 0's handle, so a handle read once
/// shows in the next call's arguments) and the button icons over a
/// logging `TOMkpShapeXYC2`.
#[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
fn round33_seq_ctrl_and_icons(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x2000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let slots = scene::bridge::SLOTS.ptr(a).addr;
    for k in 0..400u32 {
        let i = rng.next() % 3;
        let base = slots + 0x100 * i;
        let mut bump = vec![0xff, 0x05];
        bump.extend_from_slice(&(base + 0x1c).to_le_bytes());
        bump.extend(ret_cell(cells));
        let s = stand_in8(a, b, 0x400, &copy_logger(cursor, 0xc880, 2, &bump));
        redirect(a, b, scene::bridge::TK_MENU_START_SCENE, s);
        let s = stand_in8(
            a,
            b,
            0x480,
            &copy_logger(cursor, 0xbb90, 2, &ret_cell(cells + 4)),
        );
        redirect(a, b, scene::bridge::TK_MENU_IS_TERM_SCENE, s);
        let s = stand_in8(
            a,
            b,
            0x500,
            &copy_logger(cursor, 0xc8a0, 2, &ret_cell(cells + 8)),
        );
        redirect(a, b, scene::bridge::TK_MENU_STOP_SCENE, s);
        put(a, b, cursor, log);
        put(a, b, cells, rng.next());
        put(
            a,
            b,
            cells + 4,
            [0, 0, 1, rng.next()][(rng.next() % 4) as usize],
        );
        put(a, b, cells + 8, rng.next());
        for j in 0..8 {
            let at = base + 0x20 * j;
            let words: [u32; 8] = std::array::from_fn(|_| rng.next());
            put(a, b, at, words);
            let state = [0u16, 0, 1, 2, 3, 4, 5, 0xffff, 0x0101][(rng.next() % 9) as usize];
            put(a, b, at, state);
            put(
                a,
                b,
                at + 4,
                [0u16, 1, 1, 2, 0x0101][(rng.next() % 5) as usize],
            );
            put(a, b, at + 6, (rng.next() % 2) as u16);
        }
        eax(a, 0x008a_e0f0, &[i]);
        scene::bridge::seq_ctrl(b, i.cast_signed(), call);
        same_memory(&format!("seq ctrl {k}"), a, b);
    }
    // the button icons
    let s = stand_in8(a, b, 0x580, &copy_logger(cursor, 0x3a50, 5, &[]));
    redirect(a, b, tolib::bridge::SHAPE_XY_C2, s);
    let s = stand_in8(
        a,
        b,
        0x600,
        &copy_logger(cursor, 0xcf30, 0, &ret_cell(cells + 0xc)),
    );
    redirect(a, b, 0x008a_cf30, s);
    let cursor_global = ffx_system::pkt::bridge::PACKET_CURSOR;
    for k in 0..600u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells + 0xc,
            [0u32, 1, 1, 2, u32::MAX][(rng.next() % 5) as usize],
        );
        set(a, b, cursor_global, rng.next());
        let c = match rng.next() % 4 {
            0 => 0x40 + rng.next() % 16,
            1 => 0x30 + rng.next() % 10,
            2 => [
                0x60,
                0x2f,
                0x3a,
                0x3f,
                0x50,
                0x5f,
                0x61,
                0x20,
                0x140,
                0xffff_ff40,
            ][(rng.next() % 10) as usize],
            _ => [0x31, 0x32][(rng.next() % 2) as usize],
        };
        if k % 2 == 0 {
            let args = [
                rng.next(),
                c,
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
            ];
            assert_eq!(
                eax(a, 0x008e_99a0, &args),
                tolib::bridge::button_icon_rgba(b, args, call),
                "icon rgba {k} ({c:#x})"
            );
        } else {
            let args = [rng.next(), c, rng.next(), rng.next()];
            assert_eq!(
                eax(a, 0x008e_9970, &args),
                tolib::bridge::button_icon(b, args, call),
                "icon {k} ({c:#x})"
            );
        }
        same_memory(&format!("icon {k}"), a, b);
    }
}

#[test]
fn round33_seq_ctrl_and_icons_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round33_seq_ctrl_and_icons(&mut a, &mut b, &mut Xorshift(0x4033_0031));
}

#[test]
fn round33_seq_ctrl_and_icons_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round33_seq_ctrl_and_icons(&mut a, &mut b, &mut Xorshift(0x4033_0032));
}

/// Round 33 group D: `TOBtlDrawCrossParamPlate` over the original remaps,
/// with logging stand-ins for the shapes, the gauge, the battle getters,
/// the weak level and the numbers.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn round33_cross_plate(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x2000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x40);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x0090_3bb0, all(9), vec![], ret_cell(cells)),
        (0x0089_2910, all(5), vec![], ret_cell(cells + 4)),
        (0x008a_c2f0, vec![], vec![], ret_cell(cells + 8)),
        (0x0079_add0, all(1), vec![], ret_cell(cells + 0xc)),
        (0x0079_adf0, all(1), vec![], ret_cell(cells + 0x10)),
        (0x0079_ae50, all(1), vec![], ret_cell(cells + 0x14)),
        (0x0078_bf00, all(2), vec![], ret_cell(cells + 0x18)),
        (0x0090_5890, all(6), vec![], ret_cell(cells + 0x1c)),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    for k in 0..600u32 {
        put(a, b, cursor, log);
        for j in 0..8 {
            put(a, b, cells + 4 * j, rng.next());
        }
        put(
            a,
            b,
            cells + 8,
            [0xbu32, 0xb, 0, 1, 9][(rng.next() % 5) as usize],
        );
        put(
            a,
            b,
            cells + 0x18,
            [0u32, 1, 2, 3, 4, u32::MAX][(rng.next() % 6) as usize],
        );
        let chr = rng.next() % 8;
        // the wide layout's `+ 0.0` changes nothing here: `ry(2) + y`
        // is never -0 (`a + -a` rounds to +0)
        let (x, y) = match k % 10 {
            0 | 1 => (f32::from_bits(rng.next()), f32::from_bits(rng.next())),
            2 => (1.0e9, -1.0e-9),
            _ => (
                (rng.next() % 8000) as f32 / 7.0 - 100.0,
                (rng.next() % 6000) as f32 / 3.0 - 50.0,
            ),
        };
        // no signalling NaNs
        let quiet = |f: f32| if f.is_nan() { f32::NAN } else { f };
        let (x, y) = (quiet(x).to_bits(), quiet(y).to_bits());
        assert_eq!(
            eax(a, 0x0089_2cb0, &[chr, x, y]),
            chr_plate::bridge::draw_cross_param_plate(b, [chr, x, y], remap, call),
            "cross plate {k}"
        );
        same_memory(&format!("cross plate {k}"), a, b);
    }
}

#[test]
fn round33_cross_plate_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round33_cross_plate(&mut a, &mut b, &mut Xorshift(0x4033_0041));
}

#[test]
fn round33_cross_plate_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round33_cross_plate(&mut a, &mut b, &mut Xorshift(0x4033_0042));
}

/// Round 33 group E: `TOSetDrawInfo` over logging stand-ins (the message
/// size writes a test width; `AdjustButtonIconY` and the icon width
/// return test floats in ST0; `TOGetFFXLang` returns its cell and then
/// bumps it, so 10 then 11 reaches the language-11 move). The message
/// size writes its width only while `cells + 0x18` is set, and the icon
/// drawer changes the icon mode (`cells + 0x14`), which the original reads
/// again for the pen step.
#[allow(
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round33_set_draw_info(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x2000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x40);
    let scratch = alloc(a, b, t::SET_DRAW_INFO_SCRATCH as usize);
    let pen = alloc(a, b, 0x20);
    // cmp dword [cells + 0x18], 0; je over the store
    let mut message_wh = vec![0x83, 0x3d];
    message_wh.extend_from_slice(&(cells + 0x18).to_le_bytes());
    message_wh.extend_from_slice(&[0x00, 0x74, 0x0b]);
    message_wh.extend(store_through(2, cells));
    // mov al, [cells + 0x14]; mov [pen + 0x1a], al
    let mut draw_icon = vec![0xa0];
    draw_icon.extend_from_slice(&(cells + 0x14).to_le_bytes());
    draw_icon.push(0xa2);
    draw_icon.extend_from_slice(&(pen + 0x1a).to_le_bytes());

    let fld = |cell: u32| {
        let mut v = vec![0xd9, 0x05];
        v.extend_from_slice(&cell.to_le_bytes());
        v
    };
    let mut lang = ret_cell(cells + 0xc);
    lang.extend_from_slice(&[0xff, 0x05]);
    lang.extend_from_slice(&(cells + 0xc).to_le_bytes());
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x008b_9650, vec![0, 1, 4, 5], vec![], message_wh),
        (0x008d_f520, all(3), vec![], vec![]),
        (0x008b_1350, all(2), vec![], vec![]),
        (0x008b_1330, vec![], vec![], vec![]),
        (0x008e_5280, all(5), vec![], fld(cells + 4)),
        (0x008f_8ee0, all(8), vec![], draw_icon),
        (0x008f_a800, all(2), vec![], fld(cells + 8)),
        (0x008a_c2f0, vec![], vec![], lang),
        (0x0064_1060, vec![], vec![], ret_cell(cells + 0x10)),
        (0x0064_2ec0, all(2), vec![], vec![]),
        (0x0064_2c60, vec![], vec![], vec![]),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let small = |rng: &mut Xorshift| (rng.next() % 4000) as f32 / 8.0 - 100.0;
    let code = alloc(a, b, 0x10);
    let window = alloc(a, b, 0x100);
    let map = a.rebase(0x012f_bc40);
    let args = [
        0u8, 1, b'0', b'1', b'2', b'3', b'4', b'5', b'7', b'8', b'9', b'@', b'A', b'B', 0x3f, 0x40,
        0x41, 0x7f, 0xbf, 0xc0, 0xc5, 0xdf, 0xe0, 0xe7, 0xef, 0xf0, 0xff, 0x0f, 0x10, 0x21, 0x5a,
    ];
    for k in 0..3000u32 {
        put(a, b, cursor, log);
        let kind = [
            7u8, 8, 9, 0xa, 0xb, 0xb, 0xb, 0xc, 0xd, 0xe, 0, 6, 0xf, 0xff,
        ][(rng.next() % 14) as usize];
        let n = args[(rng.next() as usize) % args.len()];
        put(a, b, code, [kind, n, b'x', 0]);
        for w in 0..0x40 {
            put(a, b, window + 4 * w, rng.next());
        }
        for (off, v) in [(0x48, small(rng)), (0x50, small(rng))] {
            put(a, b, window + off, v);
        }
        put(a, b, window + 0x24, [0u16, 0, 1][(rng.next() % 3) as usize]);
        put(
            a,
            b,
            window + 0xac,
            [0u32, 0, 1, rng.next()][(rng.next() % 4) as usize],
        );
        let shown = (rng.next() % 5).cast_signed() - 1;
        put(a, b, window + 0xa0, shown);
        put(
            a,
            b,
            window + 0xa8,
            shown + (rng.next() % 3).cast_signed() - 1,
        );
        for w in 0..8 {
            put(a, b, pen + 4 * w, rng.next());
        }
        put(a, b, pen + 4, small(rng));
        put(a, b, pen + 8, small(rng));
        put(
            a,
            b,
            pen + 0x1a,
            [0u8, 1, 2, 3, 4, 0xff][(rng.next() % 6) as usize],
        );
        put(a, b, cells, small(rng));
        put(a, b, cells + 4, small(rng) / 16.0);
        put(a, b, cells + 8, small(rng) / 2.0);
        put(
            a,
            b,
            cells + 0xc,
            [8u32, 9, 10, 0, 0xb][(rng.next() % 5) as usize],
        );
        put(
            a,
            b,
            cells + 0x10,
            [0u32, 1, 1, 2][(rng.next() % 4) as usize],
        );
        put(
            a,
            b,
            cells + 0x14,
            [0u8, 1, 3, 2][(rng.next() % 4) as usize],
        );
        put(a, b, cells + 0x18, rng.next() % 3);
        put(
            a,
            b,
            map,
            [0xb8u32, 0xb8, 0, 0xb7][(rng.next() % 4) as usize],
        );
        put(a, b, scratch, [0u32, 0]);
        let r = [code, window, pen];
        eax(a, 0x008d_f8f0, &r);
        t::set_draw_info(b, r, scratch, call, fcall);
        let s: [u32; 2] = Mem::read(b, scratch);
        Mem::write(a, scratch, s);
        same_memory(&format!("set draw info {k} ({kind:#x} {n:#x})"), a, b);
    }
}

#[test]
fn round33_set_draw_info_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round33_set_draw_info(&mut a, &mut b, &mut Xorshift(0x4033_0051));
}

#[test]
fn round33_set_draw_info_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round33_set_draw_info(&mut a, &mut b, &mut Xorshift(0x4033_0052));
}

/// Round 33 group F: the CD read queue's sector request
/// (`0x0076ce40`/`0x0076ce20`) and the direct read (`0x0076d140`) over
/// logging stand-ins for `movie_err`, `dbgPrintf`, `FlushCache` and
/// `sceCdRead`.
fn round33_cd_queue(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use cd::bridge as c;
    let log = alloc(a, b, 0x2000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let stands: Vec<Stand> = vec![
        (c::MOVIE_ERR, vec![0, 1, 2], vec![], vec![]),
        (c::DBG_PRINTF, vec![0], vec![], vec![]),
        (c::FLUSH_CACHE, vec![0], vec![], vec![]),
        (c::SCE_CD_READ, vec![0, 1, 2, 3], vec![], ret_cell(cells)),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let channels = a.rebase(c::CD_CHANNELS);
    // everything from the requests to the direct read's words
    let span = (a.rebase(c::DIRECT_READ) + 0x10 - channels) / 4;
    let saved: Vec<u32> = (0..span).map(|w| Mem::read(a, channels + 4 * w)).collect();
    for k in 0..400u32 {
        put(a, b, cursor, log);
        let full = k % 10 == 0;
        for w in 0..span {
            put(a, b, channels + 4 * w, rng.next());
        }

        for n in 0..64 {
            let busy = full || !rng.next().is_multiple_of(8);
            put(
                a,
                b,
                channels + 0x38 * n + 0x31,
                u8::from(busy) * [1u8, 2, 0xff][(rng.next() % 3) as usize],
            );
        }
        set(
            a,
            b,
            c::READ_ID,
            [0i32, 5, 0xfffe, 0xffff, 0x10000, -1, -5][(rng.next() % 7) as usize],
        );
        let args = [rng.next(), rng.next(), rng.next()];
        if k % 2 == 0 {
            assert_eq!(
                eax(a, 0x0076_ce40, &args),
                c::queue_sector_read(b, args, call),
                "queue {k}"
            );
        } else {
            assert_eq!(
                eax(a, 0x0076_ce20, &args[..1]),
                c::queue_sector(b, args[0], call),
                "queue1 {k}"
            );
        }
        same_memory(&format!("queue {k}"), a, b);
        for (w, v) in (0u32..).zip(&saved) {
            put(a, b, channels + 4 * w, *v);
        }
    }
    let mode = alloc(a, b, 4);
    for k in 0..300u32 {
        put(a, b, cursor, log);
        put(a, b, cells, rng.next());
        put(a, b, mode, rng.next());
        put(
            a,
            b,
            a.rebase(c::FFX_HDD_MODE),
            [0u8, 0, 1, 0x80][(rng.next() % 4) as usize],
        );
        put(a, b, a.rebase(0x0112_8a84), [rng.next(), rng.next()]);
        put(a, b, a.rebase(0x0112_8ab0), rng.next());
        let size = [
            0u32,
            0x7ff,
            0x800,
            0x801,
            0x1_0000,
            0xffff_f800,
            0xffff_f7ff,
            u32::MAX,
            rng.next(),
        ][(rng.next() % 9) as usize];
        let buffer = [0x1000u32, 0x1040, 0x1001, 0x103f, rng.next()][(rng.next() % 5) as usize];
        let args = [rng.next(), size, buffer, mode];
        assert_eq!(
            eax(a, 0x0076_d140, &args),
            c::read_direct(b, args, call),
            "direct {k}"
        );
        same_memory(&format!("direct {k}"), a, b);
    }
}

#[test]
fn round33_cd_queue_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round33_cd_queue(&mut a, &mut b, &mut Xorshift(0x4033_0061));
}

#[test]
fn round33_cd_queue_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round33_cd_queue(&mut a, &mut b, &mut Xorshift(0x4033_0062));
}

/// Round 33 group G: `CdSetModule_0076c510`, `PcFileTblSet`,
/// `TOBtlOpenSignWindow` and the two width helpers over logging
/// stand-ins (out-pointer arguments are not logged: the original's point
/// into its stack).
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::too_many_lines
)]
fn round33_small(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use cd::bridge as c;
    use tolib::bridge as t;
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    let scratch = alloc(a, b, 8);
    // TOGetFontWidthInterMode: copies its text's first byte to
    // [cells + 0x1c], writes [cells + 0xc] through its third argument and
    // returns its first plus [cells + 0x10]
    // (mov eax, [esp+4]; movzx eax, byte [eax]; mov [cells + 0x1c], eax)
    let mut width = vec![0x8b, 0x44, 0x24, 0x04, 0x0f, 0xb6, 0x00, 0xa3];
    width.extend_from_slice(&(cells + 0x1c).to_le_bytes());
    width.extend(store_through(2, cells + 0xc));
    width.extend_from_slice(&[0x8b, 0x44, 0x24, 0x04, 0x03, 0x05]);
    width.extend_from_slice(&(cells + 0x10).to_le_bytes());
    let stands: Vec<Stand> = vec![
        (c::MOVIE_ERR, vec![0, 1, 2], vec![], vec![]),
        (c::USER_FREE, vec![0], vec![], vec![]),
        (c::READ_FILE_TABLE, vec![0, 1], vec![], ret_cell(cells)),
        (
            t::MS_GET_COM_DATA,
            vec![0],
            vec![],
            [store_through(1, cells + 4), ret_cell(cells + 8)].concat(),
        ),
        (t::OPEN_SIMPLE_HELP, vec![0, 1], vec![], vec![]),
        (t::FONT_WIDTH_INTER_MODE, vec![1], vec![], width),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let modules = alloc(a, b, 0x40);
    let init = a.rebase(c::READ_INIT_DONE);
    let saved_table = c::MODULE_FILES.get(a);
    for k in 0..200u32 {
        put(a, b, cursor, log);
        set(a, b, c::MODULE_FILES, modules);
        let firsts: [i16; 16] =
            std::array::from_fn(|_| [0i16, 5, 0x7fff, -1, -0x8000, 300][(rng.next() % 6) as usize]);
        put(a, b, modules, firsts);
        put(a, b, init, (rng.next() % 2) as u8);
        let m = (rng.next() % 16).cast_signed();
        assert_eq!(
            eax(a, 0x0076_c510, &[m.cast_unsigned()]).cast_signed(),
            c::cd_set_module(b, m, call),
            "module {k}"
        );
        same_memory(&format!("module {k}"), a, b);
    }
    set(a, b, c::MODULE_FILES, saved_table);
    let saved = c::PC_FILE_TABLE.get(a);
    for k in 0..50u32 {
        put(a, b, cursor, log);
        put(a, b, cells, rng.next());
        set(
            a,
            b,
            c::PC_FILE_TABLE,
            [0, rng.next()][(rng.next() % 2) as usize],
        );
        let path = rng.next();
        assert_eq!(
            eax(a, 0x00a4_2b70, &[path]),
            c::pc_file_tbl_set(b, path, call),
            "tbl set {k}"
        );
        same_memory(&format!("tbl set {k}"), a, b);
    }
    set(a, b, c::PC_FILE_TABLE, saved);
    let record = alloc(a, b, 0x20);
    for k in 0..200u32 {
        put(a, b, cursor, log);
        put(a, b, record, rng.next() as u16);
        put(a, b, record + 0x1c, rng.next() as u8);
        put(a, b, cells + 4, rng.next());
        put(a, b, cells + 8, record);
        let id = rng.next();
        assert_eq!(
            eax(a, 0x0089_8c60, &[id]),
            t::btl_open_sign_window(b, id, scratch, call),
            "sign {k}"
        );
        let s: [u32; 2] = Mem::read(b, scratch);
        Mem::write(a, scratch, s);
        same_memory(&format!("sign {k}"), a, b);
    }
    let text = alloc(a, b, 0x40);
    for k in 0..200u32 {
        put(a, b, cursor, log);
        let bytes: [u8; 0x20] = std::array::from_fn(|i| {
            if i == 0x1f || rng.next().is_multiple_of(6) {
                0
            } else {
                0x41
            }
        });
        put(a, b, text, bytes);
        put(a, b, cells + 0xc, rng.next() % 40);
        put(a, b, cells + 0x10, 1 + rng.next() % 3);
        assert_eq!(
            eax(a, 0x008b_8040, &[text]),
            t::text_width_inter_mode(b, text, scratch, call),
            "width {k}"
        );
        let s: [u32; 2] = Mem::read(b, scratch);
        Mem::write(a, scratch, s);
        same_memory(&format!("width {k}"), a, b);
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells + 0xc,
            [
                (rng.next() % 4000) as f32 / 7.0 - 100.0,
                f32::NAN,
                3e9,
                -0.5,
            ][(rng.next() % 4) as usize],
        );
        let font = rng.next();
        assert_eq!(
            eax(a, 0x008d_fe20, &[font]),
            t::colon_width(b, font, scratch, call),
            "colon {k}"
        );
        let s: [u32; 2] = Mem::read(b, scratch);
        Mem::write(a, scratch, s);
        same_memory(&format!("colon {k}"), a, b);
    }
}

#[test]
fn round33_small_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round33_small(&mut a, &mut b, &mut Xorshift(0x4033_0071));
}

#[test]
fn round33_small_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round33_small(&mut a, &mut b, &mut Xorshift(0x4033_0072));
}

/// `Vec3At24_0061a7f0` (thiscall), no signalling NaNs.
#[test]
fn round33_vec3_at_24_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let (a, b) = (&mut a, &mut b);
    let mut rng = Xorshift(0x4033_0081);
    let this = alloc(a, b, 0x40);
    let out = alloc(a, b, 0x10);
    for k in 0..200 {
        let words: [u32; 0x10] = std::array::from_fn(|_| {
            loop {
                let f = rng.float();
                if !(f.is_nan() && f.to_bits() & 0x0040_0000 == 0) {
                    break f.to_bits();
                }
            }
        });
        put(a, b, this, words);
        let to = if k % 5 == 0 { this + 0x28 } else { out };
        let at = a.rebase(0x0061_a7f0);
        let r = a.call(at, Conv::Thiscall(this), &[to]).unwrap().eax;
        assert_eq!(r, accessor::bridge::vec3_at_24(b, this, to), "vec3 {k}");
        same_memory(&format!("vec3 {k}"), a, b);
    }
}

/// Round 34 group A: `TOGetLangColonWidth_008df4f0` and `MagicFile_Init`
/// (its `printf` through the import slot, a logging stand-in).
fn round34_small(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let s = stand_in8(a, b, 0, &copy_logger(cursor, 0xc2f0, 0, &ret_cell(cells)));
    redirect(a, b, 0x008a_c2f0, s);
    let s = stand_in8(
        a,
        b,
        0x80,
        &copy_logger(cursor, 0xfe20, 1, &ret_cell(cells + 4)),
    );
    redirect(a, b, 0x008d_fe20, s);
    let printf = stand_in8(a, b, 0x100, &copy_logger(cursor, 0xc380, 1, &[]));
    let slot = a.rebase(0x00b0_c380);
    put(a, b, slot, printf);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..300u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells,
            [0u32, 1, 2, 3, 5, 6, 9, 0xb, u32::MAX][(rng.next() % 9) as usize],
        );
        put(a, b, cells + 4, rng.next());
        let kind = [0u32, 1, 2, u32::MAX][(rng.next() % 4) as usize];
        let font = rng.next();
        assert_eq!(
            eax(a, 0x008d_f4f0, &[kind, font]),
            tolib::bridge::lang_colon_width(b, kind, font, call),
            "colon {k}"
        );
        same_memory(&format!("colon {k}"), a, b);
    }
    for k in 0..3 {
        put(a, b, cursor, log);
        for at in [0x00c6_4ca0, 0x00c6_4ca8, 0x00c6_4cac, 0x00c6_4cb0] {
            put(a, b, a.rebase(at), rng.next());
        }
        let r = menu::bridge::magic_file_init(b, |m, fmt| {
            let f: u32 = Mem::read(m, slot);
            m.call(f, Conv::Cdecl, &[fmt]).unwrap();
        });
        assert_eq!(eax(a, 0x009d_a3e0, &[]), r, "magic init {k}");
        same_memory(&format!("magic init {k}"), a, b);
    }
}

#[test]
fn round34_small_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round34_small(&mut a, &mut b, &mut Xorshift(0x4034_0011));
}

#[test]
fn round34_small_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round34_small(&mut a, &mut b, &mut Xorshift(0x4034_0012));
}

/// Round 34 group B: the 16x12 message writers `0x008e92a0` and
/// `0x008ea060` over logging stand-ins. `TOGetGroup` logs its whole
/// argument (the original's register keeps upper bytes) and answers by the
/// byte's low three bits ([0, 1, 2, 2, 4, 3, 4, 2]: 7, 0xa and 0xb are
/// control codes); `TOSkipCode` steps two bytes; `TOGetFontInfo` puts a
/// test width in the record.
#[allow(
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round34_mes_writers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    let groups = alloc(a, b, 0x20);
    put(a, b, groups, [0u32, 1, 2, 2, 4, 3, 4, 2]);
    let glyph = a.rebase(t::MES_GLYPH);
    // mov eax, [cells + 0x10]; mov [glyph + 0x1c], eax
    let mut font_info = vec![0xa1];
    font_info.extend_from_slice(&(cells + 0x10).to_le_bytes());
    font_info.push(0xa3);
    font_info.extend_from_slice(&(glyph + 0x1c).to_le_bytes());
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x008d_f610, all(1), vec![], table_tail(4, 0, groups)),
        (
            0x008d_fd80,
            all(1),
            vec![],
            vec![0x8b, 0x44, 0x24, 0x04, 0x83, 0xc0, 0x02],
        ),
        (0x008b_1350, all(2), vec![], vec![]),
        (0x008b_1330, vec![], vec![], vec![]),
        (0x008e_99a0, all(8), vec![], ret_cell(cells)),
        (0x008e_9970, all(4), vec![], ret_cell(cells + 4)),
        (0x008b_65e0, all(1), vec![], ret_cell(cells + 8)),
        (0x008e_9b80, all(3), vec![], vec![]),
        (0x008b_7240, all(2), vec![], font_info),
        (0x0090_6260, all(2), vec![], vec![]),
        (0x0090_6270, all(2), vec![], vec![]),
        (0x0090_6290, all(2), vec![], vec![]),
        (0x008e_8ed0, all(3), vec![], vec![]),
        (0x0064_3af0, all(1), vec![], vec![]),
        (0x0064_0dd0, all(1), vec![], vec![]),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let text = alloc(a, b, 0x40);
    let codes = [1u8, 9, 7, 7, 0xa, 0xa, 0xb, 0xb, 5, 4, 0xc, 8, 0x10];
    let args = [
        0u8, 0xe0, 0xe1, 0xe7, 0xef, 0xf0, 0x31, 0x45, 0x0f, 0x10, 0x40, 0x7f, 0xa3, 0xff,
    ];
    let pen = a.rebase(t::MES_PEN);
    for k in 0..800u32 {
        put(a, b, cursor, log);
        let mut bytes = [0u8; 0x40];
        let n = (rng.next() % 12) as usize;
        for i in 0..n {
            bytes[2 * i] = codes[(rng.next() as usize) % codes.len()];
            bytes[2 * i + 1] = args[(rng.next() as usize) % args.len()];
        }
        put(a, b, text, bytes);
        for w in 0..8 {
            put(a, b, pen + 4 * w, rng.next());
        }
        put(a, b, cells, rng.next());
        put(a, b, cells + 4, rng.next());
        put(
            a,
            b,
            cells + 8,
            [0u32, 12, 0x7fff_ffff, u32::MAX][(rng.next() % 4) as usize],
        );
        put(a, b, cells + 0x10, (rng.next() % 4000) as f32 / 7.0 - 50.0);
        put(
            a,
            b,
            a.rebase(0x00c5_d778),
            [1.0f32, 0.75, 1.5, 1e30][(rng.next() % 4) as usize],
        );
        put(
            a,
            b,
            a.rebase(0x00c5_d784),
            [1.0f32, 0.5, -2.0][(rng.next() % 3) as usize],
        );
        let xy = |rng: &mut Xorshift| {
            [
                rng.next() % 2000,
                rng.next(),
                (rng.next() % 100).wrapping_neg(),
                0x7fff_ffff,
                16_777_217,
            ][(rng.next() % 5) as usize]
        };
        let (x, y) = (xy(rng), xy(rng));
        let key = [u32::MAX, rng.next()][(rng.next() % 2) as usize];
        let kind = [0u32, 1, 1, 2, 0x101][(rng.next() % 5) as usize];
        let colour = rng.next();
        let rgba: [u32; 4] = std::array::from_fn(|_| rng.next());
        let c = rng.next();
        if k % 2 == 0 {
            let args = [
                c, key, text, x, y, colour, kind, rgba[0], rgba[1], rgba[2], rgba[3],
            ];
            assert_eq!(
                eax(a, 0x008e_92a0, &args),
                t::base_mes_font_l_clut_type_rgba(b, args, call),
                "base mes {k}"
            );
        } else {
            let args = [c, text, x, y, colour, rgba[0], rgba[1], rgba[2], rgba[3]];
            assert_eq!(
                eax(a, 0x008e_a060, &args),
                t::mes_font_1612(b, args, call),
                "mes 1612 {k}"
            );
        }
        same_memory(&format!("mes writer {k}"), a, b);
    }
}

#[test]
fn round34_mes_writers_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round34_mes_writers(&mut a, &mut b, &mut Xorshift(0x4034_0021));
}

#[test]
fn round34_mes_writers_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round34_mes_writers(&mut a, &mut b, &mut Xorshift(0x4034_0022));
}

/// Round 34 group C: `TOAdpCrossNumber_009019f0` with the CRT `sprintf`
/// (through its import slot) and logging stand-ins for the glyph writer
/// (its text logged by its first word) and `TOSkipCode` (one byte).
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names
)]
fn round34_cross_number(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    // the glyph's first byte only: what follows the NUL is the original's
    // stack (mov eax, [esp+4]; movzx eax, byte [eax]; mov ecx, [cursor];
    // mov [ecx], eax; add dword [cursor], 4)
    let mut first_byte = vec![0x8b, 0x44, 0x24, 0x04, 0x0f, 0xb6, 0x00, 0x8b, 0x0d];
    first_byte.extend_from_slice(&cursor.to_le_bytes());
    first_byte.extend_from_slice(&[0x89, 0x01, 0x83, 0x05]);
    first_byte.extend_from_slice(&cursor.to_le_bytes());
    first_byte.push(4);
    let s = stand_in8(
        a,
        b,
        0,
        &pick_logger(cursor, 0xc720, &[1, 2, 3, 4, 5, 6, 7, 8], &[], &first_byte),
    );
    redirect(a, b, 0x008e_c720, s);
    let s = stand_in8(
        a,
        b,
        0x80,
        &pick_logger(cursor, 0xfd80, &[], &[], &[0x8b, 0x44, 0x24, 0x04, 0x40]),
    );
    redirect(a, b, 0x008d_fd80, s);
    let slot = a.rebase(0x00b0_c3dc);
    let sprintf = |m: &mut Oracle, args: [u32; 3]| {
        let f: u32 = Mem::read(m, slot);
        m.call(f, Conv::Cdecl, &args).unwrap();
    };
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let text = alloc(a, b, tolib::bridge::CROSS_NUMBER_TEXT as usize);
    let quiet = |f: f32| if f.is_nan() { f32::NAN } else { f };
    for k in 0..600u32 {
        put(a, b, cursor, log);
        let value =
            [0u32, 7, 42, 99_999, u32::MAX, 0x8000_0000, rng.next()][(rng.next() % 7) as usize];
        let x = quiet(
            [
                (rng.next() % 4000) as f32 / 3.0,
                f32::from_bits(rng.next()),
                1e8,
            ][(rng.next() % 3) as usize],
        );
        let y = quiet((rng.next() % 600) as f32 - 100.0);
        let scale = quiet(
            [
                0.65f32,
                1.0,
                0.5,
                (rng.next() % 100) as f32 / 37.0,
                f32::from_bits(rng.next()),
            ][(rng.next() % 5) as usize],
        );
        let gap = quiet(
            [0.0f32, -1.5, 2.0, (rng.next() % 100) as f32 / 9.0 - 5.0][(rng.next() % 4) as usize],
        );
        let args = [
            value,
            x.to_bits(),
            y.to_bits(),
            rng.next(),
            scale.to_bits(),
            gap.to_bits(),
        ];
        eax(a, 0x0090_19f0, &args);
        tolib::bridge::cross_number(b, args, text, sprintf, call);
        let t: [u32; 8] = Mem::read(b, text);
        Mem::write(a, text, t);
        same_memory(&format!("cross number {k}"), a, b);
    }
}

#[test]
fn round34_cross_number_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round34_cross_number(&mut a, &mut b, &mut Xorshift(0x4034_0031));
}

#[test]
fn round34_cross_number_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round34_cross_number(&mut a, &mut b, &mut Xorshift(0x4034_0032));
}

/// Round 34 group D: `ShapeDataModify` with the clock from a cell, `fcos`
/// in the C runtime's slot and the original 720p remaps.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]
fn round34_shape_data_modify(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let clock = alloc(a, b, 8);
    let slot = a.rebase(0x00b0_c3b4);
    let fcos = a.rebase(0x00b0_bfc0);
    a.write(fcos, &[0xd9u8, 0xff, 0xc3]).unwrap();
    b.write(fcos, &[0xd9u8, 0xff, 0xc3]).unwrap();
    put(a, b, slot, fcos);
    let mut v = vec![0xdd, 0x05];
    v.extend_from_slice(&clock.to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x180, &v);
    redirect(a, b, 0x0064_1410, s);
    // fld qword [esp+4]; call [slot]; ret
    let mut v = vec![0xdd, 0x44, 0x24, 0x04, 0xff, 0x15];
    v.extend_from_slice(&slot.to_le_bytes());
    v.push(0xc3);
    let cos_thunk = stand_in8(a, b, 0x1c0, &v);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let cos = |m: &mut Oracle, v: f64| {
        let bits = v.to_bits();
        m.call(cos_thunk, Conv::Cdecl, &[bits as u32, (bits >> 32) as u32])
            .unwrap()
            .st0
            .unwrap()
    };
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4800 } else { 0x0064_47c0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    let shape = alloc(a, b, 0x40);
    let ids: [i32; 15] = [
        0x3e, 0x19b, 0x19c, 0x19f, 0x1f7, 0x1fb, 0x1fc, 0x1023, 0x1024, 0x102d, 0, 0x19d, 0x1fd,
        -1, 0x2000,
    ];
    for k in 0..1500u32 {
        let id = ids[(rng.next() as usize) % ids.len()];
        for w in 0..0x10 {
            let v = match w {
                4..=7 | 0xc..=0xf => {
                    [0u32, 0, 0x45, 0x46, 0x80, u32::MAX, rng.next()][(rng.next() % 7) as usize]
                }
                _ => {
                    let f = [
                        (rng.next() % 4000) as f32 / 3.0 - 600.0,
                        -0.0,
                        0.0,
                        f32::from_bits(rng.next()),
                    ][(rng.next() % 4) as usize];
                    if f.is_nan() {
                        f32::NAN.to_bits()
                    } else {
                        f.to_bits()
                    }
                }
            };
            put(a, b, shape + 4 * w, v);
        }
        if k % 3 == 0 {
            put(a, b, shape + 0x10, [0u32; 3]);
        }
        put(
            a,
            b,
            clock,
            [
                f64::from(rng.next() % 100_000) / 777.0,
                0.0,
                1e7,
                // cos(3t) just under 0.5: its f32 rounding lifts the alpha
                // to 123
                (0.5f64.acos() + 1e-9 / 0.75f64.sqrt()) / 3.0,
            ][(rng.next() % 4) as usize],
        );
        eax(a, 0x008e_8a10, &[shape, id.cast_unsigned()]);
        tolib::bridge::shape_data_modify(b, shape, id, fcall, cos, remap);
        same_memory(&format!("shape {k} ({id:#x})"), a, b);
    }
}

#[test]
fn round34_shape_data_modify_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round34_shape_data_modify(&mut a, &mut b, &mut Xorshift(0x4034_0041));
}

#[test]
fn round34_shape_data_modify_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round34_shape_data_modify(&mut a, &mut b, &mut Xorshift(0x4034_0042));
}

/// One random stretch of a script message for `round34_expand_codes`:
/// text, copied and unknown codes, the record codes and nested
/// if/then/else/end-if blocks (`depth` deep at most).
fn expand_stretch(rng: &mut Xorshift, out: &mut Vec<u8>, depth: u32) {
    let rec = |out: &mut Vec<u8>, words: &[u32]| {
        for w in words {
            out.extend_from_slice(&w.to_le_bytes());
        }
    };
    for _ in 0..rng.next() % 6 {
        match rng.next() % 14 {
            0 | 1 => out.extend([0x41 + (rng.next() % 26) as u8, 3, 0xff]),
            2 => out.extend([0x13, [0x30u8, 0x37, 0x38, 0x2f][(rng.next() % 4) as usize]]),
            3 => out.extend([
                0xb,
                [0x30u8, 0x49, 0x4a, 0x60, 0x2f, 0xfb, 0xf5][(rng.next() % 7) as usize],
            ]),
            4 => out.extend([[1u8, 0xa, 0x2f, 0x12][(rng.next() % 4) as usize], 0x41]),
            5 => out.extend([0xb, 0xf9]),
            6 => {
                out.extend([0xb, 0xfa]);
                rec(out, &[rng.next() | 1]);
            }
            7 => {
                // MenuTextCode's stand-in steps 8 bytes
                out.extend([0xb, 0xf4]);
                // bytes that end an if when misread four bytes in
                rec(out, &[0x0101_0101, 0x41f2_0b41]);
            }
            8 => {
                out.extend([0xb, 0xf6]);
                let t = rng.next() % 3;
                rec(
                    out,
                    &[t | (rng.next() % 0x40) << 16, rng.next() % 4, rng.next()],
                );
            }
            9 => {
                if rng.next().is_multiple_of(8) {
                    out.extend([0xb, 0xf7]);
                    rec(out, &[rng.next()]);
                }
            }
            10 => {
                out.extend([0xb, 0xf8]);
                let kind = [0u32, 1, 2, 3, 4, 5, 0x80][(rng.next() % 7) as usize];
                rec(
                    out,
                    &[
                        (rng.next() % 4) | kind << 16,
                        (rng.next() % 4) | (rng.next() % 0x40) << 16,
                        rng.next(),
                    ],
                );
            }
            _ => {
                if depth >= 3 {
                    continue;
                }
                out.extend([0xb, 0xf0]);
                let d0 = (rng.next() % 3) | (rng.next() % 8) << 8 | (rng.next() % 3) << 16;
                let side = |rng: &mut Xorshift| (rng.next() % 4) | (rng.next() % 0x40) << 16;
                let words = [
                    d0,
                    side(rng),
                    side(rng),
                    [0u32, 1, 5, u32::MAX, rng.next()][(rng.next() % 5) as usize],
                ];
                rec(out, &words);
                if !rng.next().is_multiple_of(4) {
                    out.extend([0xb, 0xf1]);
                }
                expand_stretch(rng, out, depth + 1);
                if rng.next().is_multiple_of(2) {
                    out.extend([0xb, 0xf3]);
                    expand_stretch(rng, out, depth + 1);
                }
                if !rng.next().is_multiple_of(10) {
                    out.extend([0xb, 0xf2]);
                }
            }
        }
    }
}

/// Round 34 group E: `MsgExpandCodes_00776940` over random code streams,
/// with fake getter, setter and call tables of logging stand-ins,
/// random variables and a `MenuTextCode_00774860` stand-in that moves
/// both cursors (its pointer arguments are not logged: the original's
/// point into its stack).
#[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
fn round34_expand_codes(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use msg::bridge as m;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    let scratch = alloc(a, b, 8);
    let mut slot = 0u32;
    let mut stand = |a: &mut Oracle, b: &mut Oracle, tag: u32, args: &[u8], tail: &[u8]| {
        let s = stand_in8(
            a,
            b,
            0x80 * slot,
            &pick_logger(cursor, tag, args, &[], tail),
        );
        slot += 1;
        s
    };
    let getters: Vec<u32> = (0..4)
        .map(|k| stand(a, b, 0x100 + k, &[0], &ret_cell(cells + 4 * k)))
        .collect();
    let setters: Vec<u32> = (0..4)
        .map(|k| stand(a, b, 0x200 + k, &[0, 1], &[]))
        .collect();
    let call_table: Vec<u32> = (0..4).map(|k| stand(a, b, 0x300 + k, &[0], &[])).collect();
    // out += 3; code += 8
    let s = stand(
        a,
        b,
        0x4860,
        &[2],
        &[
            0x8b, 0x44, 0x24, 0x04, 0x83, 0x00, 0x03, 0x8b, 0x44, 0x24, 0x08, 0x83, 0x00, 0x08,
        ],
    );
    redirect(a, b, m::MENU_TEXT_CODE, s);
    let s = stand(a, b, 0xae50, &[0, 1, 2], &[]);
    redirect(a, b, m::YI_ASSERT, s);
    let tables = alloc(a, b, 0x30);
    for (k, list) in [&getters, &setters, &call_table].into_iter().enumerate() {
        let t = tables + 0x10 * k as u32;
        for (i, f) in list.iter().enumerate() {
            put(a, b, t + 4 * i as u32, *f);
        }
    }
    put(a, b, a.rebase(m::SCRIPT_GETTERS), tables);
    put(a, b, a.rebase(m::SCRIPT_SETTERS), tables + 0x10);
    put(a, b, a.rebase(m::SCRIPT_CALLS), tables + 0x20);
    let vars = alloc(a, b, 0x100);
    put(a, b, a.rebase(m::SCRIPT_VARS), vars);
    let call = |o: &mut Oracle, f: u32, args: &[u32]| o.call(f, Conv::Cdecl, args).unwrap().eax;
    let text = alloc(a, b, 0x1000);
    let out = alloc(a, b, 0x1000);
    let words = alloc(a, b, 8);
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        let mut bytes = Vec::new();
        expand_stretch(rng, &mut bytes, 0);
        bytes.push(0);
        bytes.resize(0x400, 0);
        a.write(text, &bytes).unwrap();
        b.write(text, &bytes).unwrap();
        for w in 0..0x40 {
            put(
                a,
                b,
                vars + 4 * w,
                [0u32, 1, 5, u32::MAX, 0x8000_0000, rng.next()][(rng.next() % 6) as usize],
            );
        }
        for w in 0..4 {
            put(
                a,
                b,
                cells + 4 * w,
                [0u32, 1, 5, u32::MAX, rng.next()][(rng.next() % 5) as usize],
            );
        }
        put(a, b, words, [rng.next(), rng.next()]);
        let base = if k % 16 == 0 {
            out.wrapping_sub(0x3fff)
        } else {
            out
        };
        put(a, b, a.rebase(m::MES_BUFFER), base);
        put(a, b, a.rebase(m::EXPAND_JUMP_BASE), rng.next());
        let jump = if k % 3 == 0 { 0 } else { words };
        let flag = if k % 4 == 0 { 0 } else { words + 4 };
        let arg = rng.next();
        let args = [out, text, arg, jump, flag];
        assert_eq!(
            eax(a, 0x0077_6940, &args),
            m::expand_codes(b, args, scratch, call),
            "expand {k}"
        );
        let s: [u32; 2] = Mem::read(b, scratch);
        Mem::write(a, scratch, s);
        same_memory(&format!("expand {k}"), a, b);
    }
}

#[test]
fn round34_expand_codes_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round34_expand_codes(&mut a, &mut b, &mut Xorshift(0x4034_0051));
}

#[test]
fn round34_expand_codes_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round34_expand_codes(&mut a, &mut b, &mut Xorshift(0x4034_0052));
}

/// Round 34 group F: the message boxes `0x008ff370`/`0x008ff3e0` and
/// `BtlDrawRatio_008955e0` over logging stand-ins and the original remap.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn round34_boxes(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let scratch = alloc(a, b, t::MES_BOX_FIT_SCRATCH as usize);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x008f_b150, all(8), vec![], ret_cell(cells)),
        (0x008f_d390, all(6), vec![], ret_cell(cells)),
        (0x0090_1cb0, all(4), vec![], ret_cell(cells + 4)),
        (
            0x008b_9650,
            vec![0, 1, 4, 5],
            vec![],
            store_through(2, cells + 8),
        ),
        (0x0090_1490, all(4), vec![], vec![]),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    let mes_box = a.rebase(t::MES_BOX);
    for k in 0..600u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            mes_box,
            [rng.next(), rng.next(), rng.next(), rng.next()],
        );
        put(a, b, cells, rng.next());
        put(a, b, cells + 4, rng.next());
        let w = [
            (rng.next() % 6000) as f32 / 9.0,
            478.0,
            -3.5,
            f32::NAN,
            1e12,
        ][(rng.next() % 5) as usize];
        put(a, b, cells + 8, w);
        let args = [rng.next(), rng.next() % 1000, rng.next(), rng.next()];
        match k % 3 {
            0 => assert_eq!(
                eax(a, 0x008f_f370, &args),
                t::mes_box(b, args, call),
                "box {k}"
            ),
            1 => {
                assert_eq!(
                    eax(a, 0x008f_f3e0, &args),
                    t::mes_box_fit(b, args, scratch, call),
                    "fit {k}"
                );
                let s: [u32; 2] = Mem::read(b, scratch);
                Mem::write(a, scratch, s);
            }
            _ => {
                put(a, b, a.rebase(0x0133_f77c), (rng.next() % 3) & 1);
                put(a, b, a.rebase(0x0133_c93f), (rng.next() % 2) as u8);
                put(a, b, a.rebase(0x0133_f780), [rng.next(), rng.next()]);
                let f = [
                    0.5f32,
                    1.0,
                    1.5,
                    f32::NAN,
                    -0.0,
                    (rng.next() % 300) as f32 / 100.0,
                ][(rng.next() % 6) as usize];
                put(a, b, a.rebase(0x0133_f794), f);
                eax(a, 0x0089_55e0, &[]);
                t::btl_draw_ratio(b, remap, call);
            }
        }
        same_memory(&format!("boxes {k}"), a, b);
    }
}

#[test]
fn round34_boxes_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round34_boxes(&mut a, &mut b, &mut Xorshift(0x4034_0061));
}

#[test]
fn round34_boxes_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round34_boxes(&mut a, &mut b, &mut Xorshift(0x4034_0062));
}

/// Round 34 group G: the 16x12 writers' wrappers over logging stand-ins
/// for the two writers and `TOValueToMesFontL` (which puts a test word in
/// its text).
#[allow(clippy::many_single_char_names)]
fn round34_mes_wrappers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let text = alloc(a, b, t::MES_VALUE_TEXT as usize);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    // the text pointer is the original's stack: log the value and width
    let stands: Vec<Stand> = vec![
        (t::BASE_MES_FONT, all(11), vec![], ret_cell(cells)),
        (t::MES_FONT_1612, all(9), vec![], ret_cell(cells + 4)),
        (
            t::VALUE_TO_MES_FONT_L,
            vec![0, 2],
            vec![],
            store_through(1, cells + 8),
        ),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..300u32 {
        put(a, b, cursor, log);
        for w in 0..3 {
            put(a, b, cells + 4 * w, rng.next());
        }
        let w: [u32; 10] = std::array::from_fn(|_| rng.next());
        let tag = format!("wrapper {k}");
        match k % 6 {
            0 => assert_eq!(
                eax(a, 0x008e_d0a0, &w[..5]),
                t::base_mes_font(b, [w[0], w[1], w[2], w[3], w[4]], call),
                "{tag}"
            ),
            1 => assert_eq!(
                eax(a, 0x008e_d060, &w[..4]),
                t::base_mes_font_plain(b, [w[0], w[1], w[2], w[3]], call),
                "{tag}"
            ),
            2 => {
                let args: [u32; 9] = std::array::from_fn(|i| w[i]);
                assert_eq!(
                    eax(a, 0x008e_d160, &args),
                    t::base_mes_font_rgba(b, args, call),
                    "{tag}"
                );
            }
            3 => assert_eq!(
                eax(a, 0x008e_e210, &w),
                t::base_mes_font_l_clut_rgba(b, w, call),
                "{tag}"
            ),
            4 => assert_eq!(
                eax(a, 0x008e_a000, &w[..5]),
                t::mes_font_1612_grey(b, [w[0], w[1], w[2], w[3], w[4]], call),
                "{tag}"
            ),
            _ => {
                let args = [w[0], w[1], w[2], w[3], w[4]];
                // the writer's text argument differs (stack against scratch): log it apart
                assert_eq!(
                    eax(a, 0x008f_2ff0, &args),
                    t::mes_value(b, args, text, call),
                    "{tag}"
                );
                let s: [u32; 8] = Mem::read(b, text);
                Mem::write(a, text, s);
                // the logged text pointers differ: compare the rest
                let n: u32 = Mem::read(a, cursor);
                let m: u32 = Mem::read(b, cursor);
                assert_eq!(n, m, "{tag}");
                let at = n - 4 * 12 + 4 * 3;
                let p: u32 = Mem::read(b, at);
                Mem::write(a, at, p);
            }
        }
        same_memory(&tag, a, b);
    }
}

#[test]
fn round34_mes_wrappers_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round34_mes_wrappers(&mut a, &mut b, &mut Xorshift(0x4034_0071));
}

#[test]
fn round34_mes_wrappers_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round34_mes_wrappers(&mut a, &mut b, &mut Xorshift(0x4034_0072));
}

/// Round 35 group A: `prcPacketDataClose` (its `dbgPrintf` a logging
/// stand-in; the packet cursor also aliasing the descriptor) and
/// `BtlEnd_BuildList_008bde00` (its callees the originals).
#[allow(clippy::cast_possible_truncation)]
fn round35_close_and_list(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let ret = alloc(a, b, 4);
    let printf = stand_in8(a, b, 0, &copy_logger(cursor, 0xf500, 4, &ret_cell(ret)));
    redirect(a, b, 0x0062_f500, printf);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let buf = alloc(a, b, 0x400);
    let desc = alloc(a, b, 0x40) + 0x20;
    for k in 0..400u32 {
        put(a, b, cursor, log);
        put(a, b, ret, rng.next());
        let tag = match rng.next() % 6 {
            0 => desc - 4,
            1 => desc - 8,
            2 => desc - 0xc,
            _ => buf + (rng.next() % 0x3c) * 0x10,
        };
        let start = match rng.next() % 3 {
            0 => buf,
            1 => tag,
            _ => buf + rng.next() % 0x400,
        };
        let size = tag.wrapping_add(0x10).wrapping_sub(start);
        let limit = match rng.next() % 4 {
            0 => size,
            1 => size.wrapping_sub(1),
            2 => size.wrapping_add(1),
            _ => rng.next(),
        };
        put(a, b, desc, start);
        put(a, b, desc + 4, tag);
        put(a, b, desc + 8, limit);
        assert_eq!(
            eax(a, 0x0076_6b90, &[desc]),
            pkt::bridge::prc_packet_data_close(b, desc, call),
            "close {k}"
        );
        same_memory(&format!("close {k}"), a, b);
    }
    let counts = a.rebase(0x0231_0ea0);
    let results = a.rebase(0x0231_0f20);
    let entries = a.rebase(0x0186_9f20);
    for k in 0..200u32 {
        for i in 0..8 {
            let counted = [0u8, 0, 1, 0xff, 7][(rng.next() % 5) as usize];
            put(a, b, counts + i, counted);
            let result = match rng.next() % 6 {
                0 => rng.next(),
                1 => rng.next() & 0xffff_ff00,
                _ => 0,
            };
            put(a, b, results + 4 * i, result);
        }
        // stale entries: every byte the list writes must be written
        for j in 0..0x70 {
            put(a, b, entries + j, rng.next() as u8);
        }
        assert_eq!(
            eax(a, 0x008b_de00, &[]),
            btl::bridge::btl_end_build_list(b, call),
            "list {k}"
        );
        same_memory(&format!("list {k}"), a, b);
    }
}

#[test]
fn round35_close_and_list_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round35_close_and_list(&mut a, &mut b, &mut Xorshift(0x4035_0011));
}

#[test]
fn round35_close_and_list_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round35_close_and_list(&mut a, &mut b, &mut Xorshift(0x4035_0012));
}

/// Round 35 group B: `useMultipleInstances_model`, `MacroDic_Load` (its
/// file loads and the swap flag logging stand-ins returning test buffers;
/// `set_ptr32`/`get_ptr32_func` the originals) and `TOBtlDrawDamageValue2`
/// (logging stand-ins for the font setup, the achievements and the two
/// packet writers).
#[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
fn round35_group_b(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x0088_cac0, all(1), vec![], ret_cell(cells)),
        (0x008a_c390, vec![], vec![], ret_cell(cells + 4)),
        (0x008a_ba60, all(2), vec![], ret_cell(cells + 8)),
        (0x0090_3300, vec![], vec![], vec![]),
        (0x0082_2380, all(1), vec![], vec![]),
        (0x008f_b840, all(7), vec![], vec![]),
        (0x008f_bcc0, all(4), vec![], vec![]),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let magic = a.rebase(0x00c6_4ca0);
    let ids = [
        0x133u32, 0x25d, 0x6a, 0x119, 0x233, 0x264, 0x20a, 0x1bd, 0xa2, 0x24a, 0xe, 0x11b, 0,
        0x134, 0xf,
    ];
    let model = alloc(a, b, 4);
    let instance = alloc(a, b, 0x20);
    let vfx_pass = a.rebase(0x0113_335c);
    let map = a.rebase(0x012f_bc40);
    for k in 0..600u32 {
        put(a, b, model, rng.next());
        put(
            a,
            b,
            instance + 0x10,
            [0x833u16, 0x834, 0x8000, 0x7fff, 0, rng.next() as u16][(rng.next() % 6) as usize],
        );
        put(a, b, magic, ids[(rng.next() % 15) as usize]);
        put(
            a,
            b,
            vfx_pass,
            if rng.next().is_multiple_of(3) {
                0
            } else {
                rng.next()
            },
        );
        put(
            a,
            b,
            map,
            [0x183u32, 0x184, 0x185, 0x186, 0x187, 0][(rng.next() % 6) as usize],
        );
        assert_eq!(
            eax(a, 0x0071_e610, &[model, instance]),
            render::bridge::use_multiple_instances_model(b, model, instance),
            "instances {k}"
        );
        same_memory(&format!("instances {k}"), a, b);
    }
    let files = [alloc(a, b, 0x40), alloc(a, b, 0x40)];
    for k in 0..60u32 {
        put(a, b, cursor, log);
        for f in files {
            for j in 0..16 {
                let off = match rng.next() % 4 {
                    0 => 0,
                    1 => rng.next() | 0x8000_0000,
                    2 => 1,
                    _ => rng.next() % 0x10000,
                };
                put(a, b, f + 4 * j, off);
            }
        }
        put(a, b, cells, files[0]);
        put(
            a,
            b,
            cells + 4,
            [1u32, 0, 2, u32::MAX][(rng.next() % 4) as usize],
        );
        put(a, b, cells + 8, files[1]);
        assert_eq!(
            eax(a, 0x008b_85c0, &[]),
            window::bridge::macro_dic_load(b, call),
            "macro dic {k}"
        );
        same_memory(&format!("macro dic {k}"), a, b);
    }
    let count = a.rebase(0x0133_f108);
    let entries = a.rebase(0x0133_f110);
    let values = [0u32, 9, 10, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x39, u32::MAX, 5];
    for k in 0..300u32 {
        put(a, b, cursor, log);
        for j in 0..24 {
            let e = entries + 0x10 * j;
            put(a, b, e, rng.next());
            put(
                a,
                b,
                e + 4,
                if rng.next().is_multiple_of(2) {
                    3
                } else {
                    rng.next() % 5
                },
            );
            put(a, b, e + 8, values[(rng.next() % 11) as usize]);
            put(a, b, e + 0xc, rng.next());
        }
        put(
            a,
            b,
            count,
            [0i16, 1, 3, 20, -1, i16::MIN][(rng.next() % 6) as usize],
        );
        put(
            a,
            b,
            a.rebase(0x0113_3390),
            if rng.next().is_multiple_of(3) {
                rng.next()
            } else {
                0
            },
        );
        put(
            a,
            b,
            a.rebase(0x0113_3394),
            if rng.next().is_multiple_of(3) {
                rng.next()
            } else {
                0
            },
        );
        put(
            a,
            b,
            magic,
            [0x11bu32, 0x11b, 0x11c, 0][(rng.next() % 4) as usize],
        );
        assert_eq!(
            eax(a, 0x0089_3290, &[]),
            btl::bridge::draw_damage_value2(b, call),
            "damage {k}"
        );
        same_memory(&format!("damage {k}"), a, b);
    }
}

#[test]
fn round35_group_b_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round35_group_b(&mut a, &mut b, &mut Xorshift(0x4035_0021));
}

#[test]
fn round35_group_b_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round35_group_b(&mut a, &mut b, &mut Xorshift(0x4035_0022));
}

/// Round 35 group C: `TOBtlDrawSummonPrim` and `TOBtlDrawChangePrim` over
/// the original remaps, with logging stand-ins for the window and frame,
/// the name, its width (stored through the pointer), the edge font, the
/// plate and the scenario flag.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn round35_prims(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x008f_41b0, all(5), vec![], vec![]),
        (0x0089_76f0, all(1), vec![], ret_cell(cells)),
        (
            0x0090_5290,
            vec![0, 2, 3, 4],
            vec![],
            store_through(1, cells + 4),
        ),
        (0x0090_5930, all(6), vec![], vec![]),
        (0x0089_2cb0, all(3), vec![], ret_cell(cells + 8)),
        (0x0090_34b0, all(8), vec![], vec![]),
        (0x0086_c470, vec![], vec![], ret_cell(cells + 0xc)),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    let scratch = alloc(a, b, 4);
    for k in 0..800u32 {
        put(a, b, cursor, log);
        put(a, b, cells, rng.next());
        let width = match rng.next() % 4 {
            0 => rng.float(),
            _ => (rng.next() % 4000) as f32 / 9.0,
        };
        let width = if width.is_nan() { f32::NAN } else { width };
        put(a, b, cells + 4, width);
        put(a, b, cells + 8, rng.next());
        put(
            a,
            b,
            cells + 0xc,
            [0x433u32, 0x434, 0, u32::MAX, 0x8000_0000][(rng.next() % 5) as usize],
        );
        put(a, b, scratch, rng.next());
        let chr =
            [0u32, 5, 6, 6, 7, 8, 0x11, 0x12, u32::MAX, 0x8000_0000][(rng.next() % 10) as usize];
        let (x, y) = match k % 8 {
            0 => (rng.next(), rng.next()),
            1 => (0x7fff_ffff, 0x8000_0000),
            _ => (rng.next() % 1200, rng.next() % 900),
        };
        let summon = k % 2 == 0;
        let sym = if summon { 0x008a_0ba0 } else { 0x0089_f0e0 };
        let want = eax(a, sym, &[chr, x, y]);
        let mut prim = btl::bridge::BtlPrim {
            mem: b,
            remap,
            call,
            scratch,
        };
        let got = if summon {
            prim.summon(chr, x.cast_signed(), y.cast_signed())
        } else {
            prim.change(chr, x.cast_signed(), y.cast_signed())
        };
        assert_eq!(want, got, "prim {k}");
        // the port's width goes to `scratch`, the original's to its stack
        let width_bits: u32 = Mem::read(b, scratch);
        Mem::write(a, scratch, width_bits);
        same_memory(&format!("prim {k}"), a, b);
    }
}

#[test]
fn round35_prims_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round35_prims(&mut a, &mut b, &mut Xorshift(0x4035_0031));
}

#[test]
fn round35_prims_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round35_prims(&mut a, &mut b, &mut Xorshift(0x4035_0032));
}

/// Round 35 group D: `TOBtlInitStatusWin`, its getters logging stand-ins
/// returning test words (`MsGetSaveWindow` a test party list, its count
/// stored through the pointer).
#[allow(clippy::cast_possible_truncation)]
fn round35_status_win(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    let mut window_tail = store_through(2, cells + 0x18);
    window_tail.extend_from_slice(&ret_cell(cells));
    let stands: Vec<Stand> = vec![
        (0x0078_5410, vec![0, 1], vec![], window_tail),
        (0x0079_add0, vec![0], vec![], ret_cell(cells + 4)),
        (0x0079_ae50, vec![0], vec![], ret_cell(cells + 8)),
        (0x0079_5550, vec![0], vec![], ret_cell(cells + 0xc)),
        (0x0079_5590, vec![0], vec![], ret_cell(cells + 0x10)),
        (0x0089_8260, vec![0, 1], vec![], ret_cell(cells + 0x14)),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let stand = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), stand);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let list = alloc(a, b, 0x40);
    let count = alloc(a, b, 4);
    let width = a.rebase(0x025d_0aa4);
    for k in 0..400u32 {
        put(a, b, cursor, log);
        for j in 0..0x10 {
            let chr = [0u16, 1, 6, 7, 0xff, 0xff, 0x12, 0xffff][(rng.next() % 8) as usize];
            put(a, b, list + 2 * j, chr);
        }
        put(a, b, cells, list);
        // stale rows: every byte the rows get must be written
        let rows = a.rebase(0x0133_f7d0);
        for j in 0..0x70 {
            put(a, b, rows + 4 * j, rng.next());
        }
        for j in 1..6 {
            put(a, b, cells + 4 * j, rng.next());
        }
        // a nonzero maximum, never i32::MIN / -1 (both fault)
        let max = [1u32, 3, 0x3e8, 0xff, 2][(rng.next() % 5) as usize];
        put(
            a,
            b,
            cells + 0x10,
            if rng.next().is_multiple_of(2) {
                max
            } else {
                max.wrapping_neg() | 1
            },
        );
        put(a, b, cells + 0xc, rng.next() % 0x10000);
        put(
            a,
            b,
            cells + 0x18,
            [0u32, 1, 2, 3, 8, 0x10, u32::MAX][(rng.next() % 7) as usize],
        );
        put(a, b, width, rng.next() as u16);
        put(a, b, count, rng.next());
        let want = eax(a, 0x0089_8010, &[]);
        let got = btl::bridge::init_status_win(b, count, call);
        assert_eq!(want, got, "status win {k}");
        // the port's count lives in `count`, the original's on its stack
        let trimmed: u32 = Mem::read(b, count);
        Mem::write(a, count, trimmed);
        same_memory(&format!("status win {k}"), a, b);
    }
}

#[test]
fn round35_status_win_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round35_status_win(&mut a, &mut b, &mut Xorshift(0x4035_0041));
}

#[test]
fn round35_status_win_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round35_status_win(&mut a, &mut b, &mut Xorshift(0x4035_0042));
}

/// Round 36 group A: `updateVFXZBufClearRenderData`, its lookup, buffer
/// and visibility calls logging stand-ins (the lookup and the buffer
/// returning test words); the geometry's index and position lists may
/// alias the geometry itself.
fn round36_zbuf_clear(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 8);
    let stands: Vec<Stand> = vec![
        (0x0064_53d0, vec![0, 1], vec![], ret_cell(cells)),
        (0x0063_fdb0, vec![0, 1, 2, 3], vec![], ret_cell(cells + 4)),
        (0x0064_5710, vec![0, 1, 2], vec![], vec![]),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let stand = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), stand);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let data = alloc(a, b, 0x100);
    let geometry = alloc(a, b, 0x40);
    let lists = alloc(a, b, 0x100);
    for k in 0..300u32 {
        put(a, b, cursor, log);
        for j in 0..0x40 {
            put(a, b, lists + 4 * j, rng.next());
        }
        for j in 0..0x10 {
            put(a, b, geometry + 4 * j, rng.next());
        }
        put(a, b, data + 0x94, geometry);
        put(
            a,
            b,
            cells,
            if rng.next().is_multiple_of(5) {
                0
            } else {
                data
            },
        );
        put(
            a,
            b,
            cells + 4,
            if rng.next().is_multiple_of(5) {
                0
            } else {
                rng.next()
            },
        );
        // the index list sometimes overlaps the positions or the geometry
        let index_list = match rng.next() % 4 {
            0 => geometry + 0x10,
            1 => lists + 0x20,
            _ => lists + 0x80,
        };
        // (positions over the geometry stop short of its own pointer +0xc)
        let position_list = match rng.next() % 4 {
            0 => geometry + 0x10,
            1 => index_list,
            _ => lists,
        };
        put(a, b, geometry + 0x1c, index_list);
        put(a, b, geometry + 0xc, position_list);
        let z = match rng.next() % 3 {
            0 => f32::from_bits(rng.next()),
            _ => rng.float(),
        };
        let z = if z.is_nan() { f32::NAN } else { z };
        let (lo, hi) = (rng.next(), rng.next());
        assert_eq!(
            eax(a, 0x0071_e330, &[lo, hi, z.to_bits()]),
            render::bridge::update_vfx_zbuf_clear_render_data(b, lo, hi, z, call),
            "zbuf clear {k}"
        );
        same_memory(&format!("zbuf clear {k}"), a, b);
    }
}

#[test]
fn round36_zbuf_clear_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round36_zbuf_clear(&mut a, &mut b, &mut Xorshift(0x4036_0011));
}

#[test]
fn round36_zbuf_clear_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round36_zbuf_clear(&mut a, &mut b, &mut Xorshift(0x4036_0012));
}

/// Round 36 group B: `TOAddPktChrNameIcon_008eba80`, its name lookup,
/// icon and grey writer logging stand-ins returning test words.
fn round36_chr_name_icon(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0xc);
    let pkt = a.rebase(0x0187_16cc);
    // the writers also log the packet cursor they find
    let with_pkt = |ret: u32| {
        let mut v = vec![0xa1];
        v.extend_from_slice(&pkt.to_le_bytes());
        v.extend_from_slice(&[0x8b, 0x0d]);
        v.extend_from_slice(&cursor.to_le_bytes());
        v.extend_from_slice(&[0x89, 0x01, 0x83, 0x05]);
        v.extend_from_slice(&cursor.to_le_bytes());
        v.push(4);
        v.extend_from_slice(&ret_cell(ret));
        v
    };
    let stands: Vec<Stand> = vec![
        (0x008a_c850, vec![0], vec![], ret_cell(cells)),
        (0x008e_b240, vec![0, 1, 2, 3], vec![], with_pkt(cells + 4)),
        (
            0x008e_a000,
            vec![0, 1, 2, 3, 4],
            vec![],
            with_pkt(cells + 8),
        ),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let stand = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), stand);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..200u32 {
        put(a, b, cursor, log);
        for j in 0..3 {
            put(a, b, cells + 4 * j, rng.next());
        }
        put(a, b, pkt, rng.next());
        let args = [rng.next(), rng.next(), rng.next(), rng.next(), rng.next()];
        assert_eq!(
            eax(a, 0x008e_ba80, &args),
            tolib::bridge::add_pkt_chr_name_icon(b, args, call),
            "name icon {k}"
        );
        same_memory(&format!("name icon {k}"), a, b);
    }
}

#[test]
fn round36_chr_name_icon_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round36_chr_name_icon(&mut a, &mut b, &mut Xorshift(0x4036_0021));
}

#[test]
fn round36_chr_name_icon_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round36_chr_name_icon(&mut a, &mut b, &mut Xorshift(0x4036_0022));
}

/// A cdecl `graphicFontDraw` stand-in for round 36: logs the sprite
/// record's two corners (0x40 bytes), its `+0x88`, `+0x90` and `+0x94`,
/// then arguments 1..=4.
fn font_draw_logger(cursor: u32) -> Vec<u8> {
    // push esi; push edi; mov esi, [esp+0xc]; mov edi, [cursor]
    let mut v = vec![0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c, 0x8b, 0x3d];
    v.extend_from_slice(&cursor.to_le_bytes());
    // mov ecx, 0x10; cld; rep movsd
    v.extend_from_slice(&[0xb9, 0x10, 0, 0, 0, 0xfc, 0xf3, 0xa5]);
    // mov eax, [esi+0x48 / 0x50 / 0x54]; stosd (esi is record + 0x40)
    for off in [0x48u8, 0x50, 0x54] {
        v.extend_from_slice(&[0x8b, 0x46, off, 0xab]);
    }
    // mov eax, [esp+0x10 + 4k]; stosd
    for k in 0..4u8 {
        v.extend_from_slice(&[0x8b, 0x44, 0x24, 0x10 + 4 * k, 0xab]);
    }
    // mov [cursor], edi; pop edi; pop esi; ret
    v.extend_from_slice(&[0x89, 0x3d]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x5f, 0x5e, 0xc3]);
    v
}

/// Round 36 group C: `TODrawFont3d80Glyph_008eeee0`. `TOGetTextureName`
/// logs and stores a test size pair, `TOGetImageWH` stores the next test
/// width and height (two pairs, rotated so the two calls differ),
/// `graphicFontDraw` logs the record.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
fn round36_glyph(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    // the size pair through argument 4, the texture name from cells + 8
    // (skipped while the word at cells + 0x1c is 0; it flips each call)
    let mut name_tail = vec![0x83, 0x3d];
    name_tail.extend_from_slice(&(cells + 0x1c).to_le_bytes());
    name_tail.extend_from_slice(&[0x00, 0x74, 0x13]);
    name_tail.extend_from_slice(&[0x8b, 0x54, 0x24, 0x14, 0xa1]);
    name_tail.extend_from_slice(&cells.to_le_bytes());
    name_tail.extend_from_slice(&[0x89, 0x02, 0xa1]);
    name_tail.extend_from_slice(&(cells + 4).to_le_bytes());
    name_tail.extend_from_slice(&[0x89, 0x42, 0x04]);
    // xor dword [cells + 0x1c], 1: the next call does the other
    name_tail.extend_from_slice(&[0x83, 0x35]);
    name_tail.extend_from_slice(&(cells + 0x1c).to_le_bytes());
    name_tail.push(1);
    name_tail.extend_from_slice(&ret_cell(cells + 8));
    // width/height from cells + 0xc/0x10 through arguments 1/2, then
    // swapped with cells + 0x14/0x18 for the next call
    let mut wh_tail = Vec::new();
    for (k, (now, next)) in [(0xcu32, 0x14u32), (0x10, 0x18)].into_iter().enumerate() {
        wh_tail.extend_from_slice(&[0x8b, 0x54, 0x24, 8 + 4 * k as u8, 0xa1]);
        wh_tail.extend_from_slice(&(cells + now).to_le_bytes());
        wh_tail.extend_from_slice(&[0x89, 0x02, 0x87, 0x05]);
        wh_tail.extend_from_slice(&(cells + next).to_le_bytes());
        wh_tail.push(0xa3);
        wh_tail.extend_from_slice(&(cells + now).to_le_bytes());
    }
    let stands: Vec<(u32, Vec<u8>)> = vec![
        (
            0x008a_cf50,
            pick_logger(cursor, 0xcf50, &[0, 1, 2, 3], &[], &name_tail),
        ),
        (
            0x008a_c400,
            pick_logger(cursor, 0xc400, &[0], &[], &wh_tail),
        ),
        (0x0064_0660, font_draw_logger(cursor)),
    ];
    for (k, (sym, code)) in (0u32..).zip(&stands) {
        let stand = stand_in8(a, b, 0x80 * k, code);
        redirect(a, b, *sym, stand);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let text = alloc(a, b, 4);
    let scratch = alloc(a, b, tolib::bridge::GLYPH_3D80_SCRATCH as usize);
    let sizes = [
        1.0f32,
        256.0,
        512.0,
        3.0,
        0.0,
        -0.0,
        1.0e-30,
        f32::INFINITY,
        f32::NAN,
    ];
    for k in 0..600u32 {
        put(a, b, cursor, log);
        put(a, b, text, rng.next() as u8);
        put(a, b, cells, rng.next());
        put(a, b, cells + 4, rng.next());
        put(a, b, cells + 8, rng.next());
        put(a, b, cells + 0x1c, rng.next() % 2);
        for j in 0..42 {
            put(a, b, scratch + 4 * j, rng.next());
        }
        for j in 0..4 {
            let s = if rng.next().is_multiple_of(4) {
                sizes[(rng.next() % 9) as usize]
            } else {
                (rng.next() % 1024 + 1) as f32
            };
            put(a, b, cells + 0xc + 4 * j, s);
        }
        let args = [
            rng.next(),
            text,
            [rng.next(), 0x7fff_fff5, 0x8000_0000, rng.next() % 640][(rng.next() % 4) as usize],
            [rng.next(), 0x7fff_fff4, 0x8000_0000, rng.next() % 480][(rng.next() % 4) as usize],
            rng.next(),
            rng.next(),
            rng.next(),
            rng.next(),
            rng.next(),
        ];
        let want = eax(a, 0x008e_eee0, &args);
        let got = tolib::bridge::draw_font_3d80_glyph(b, args, scratch, call);
        assert_eq!(want, got, "glyph {k}");
        // the port's record and sizes live in `scratch`, the original's on
        // its stack
        let kept: [u32; 42] = Mem::read(b, scratch);
        Mem::write(a, scratch, kept);
        same_memory(&format!("glyph {k}"), a, b);
    }
}

#[test]
fn round36_glyph_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round36_glyph(&mut a, &mut b, &mut Xorshift(0x4036_0031));
}

#[test]
fn round36_glyph_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round36_glyph(&mut a, &mut b, &mut Xorshift(0x4036_0032));
}

/// A stand-in tail storing the word at `now` through argument `k` (`u32`,
/// or `u16` when `word`) and swapping `now` with `next` for the next call.
fn store_rotating(k: u8, now: u32, next: u32, word: bool) -> Vec<u8> {
    // mov edx, [esp+4+4k]; mov eax, [now]
    let mut v = vec![0x8b, 0x54, 0x24, 4 + 4 * k, 0xa1];
    v.extend_from_slice(&now.to_le_bytes());
    // mov [edx], eax (or ax)
    if word {
        v.push(0x66);
    }
    v.extend_from_slice(&[0x89, 0x02]);
    // xchg eax, [next]; mov [now], eax
    v.extend_from_slice(&[0x87, 0x05]);
    v.extend_from_slice(&next.to_le_bytes());
    v.push(0xa3);
    v.extend_from_slice(&now.to_le_bytes());
    v
}

/// Round 36 group D: `TOSetupMessageWindow_008b8de0`, every callee a
/// logging stand-in returning test words (`TOGetMessageWH` storing a
/// rotating width and height, `TOGetChangeMacroWidth` loading a test
/// float into `ST0`); the callback is a logging stand-in too.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
fn round36_setup_window(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x60);
    let slot = |k: u32| cells + 4 * k;
    // the width store is skipped while cell 16 is 0 (it flips each call)
    let mut wh = vec![0x83, 0x3d];
    wh.extend_from_slice(&slot(16).to_le_bytes());
    let width_store = store_rotating(2, slot(0), slot(1), false);
    wh.extend_from_slice(&[0x00, 0x74, u8::try_from(width_store.len()).unwrap()]);
    wh.extend_from_slice(&width_store);
    wh.extend_from_slice(&[0x83, 0x35]);
    wh.extend_from_slice(&slot(16).to_le_bytes());
    wh.push(1);
    wh.extend_from_slice(&store_rotating(3, slot(2), slot(3), true));
    wh.extend_from_slice(&ret_cell(slot(4)));
    let mut count = ret_cell(slot(5));
    count.extend_from_slice(&[0x87, 0x05]);
    count.extend_from_slice(&slot(6).to_le_bytes());
    count.push(0xa3);
    count.extend_from_slice(&slot(5).to_le_bytes());
    // fld qword [cells 14..15]: a width at full double precision
    let mut macro_width = vec![0xdd, 0x05];
    macro_width.extend_from_slice(&slot(14).to_le_bytes());
    let stands: Vec<Stand> = vec![
        (0x008b_80b0, vec![0, 1], vec![], count.clone()),
        (0x008b_9650, vec![0, 1, 4, 5], vec![], wh),
        (0x008a_c7b0, vec![], vec![], ret_cell(slot(8))),
        (
            0x008b_8d10,
            vec![0, 1, 2, 3, 4, 5],
            vec![],
            ret_cell(slot(9)),
        ),
        (0x008b_8140, vec![0, 1, 2], vec![], {
            let mut v = ret_cell(slot(10));
            v.extend_from_slice(&[0x87, 0x05]);
            v.extend_from_slice(&slot(11).to_le_bytes());
            v.push(0xa3);
            v.extend_from_slice(&slot(10).to_le_bytes());
            v
        }),
        (0x008b_6a30, vec![0, 1, 2], vec![], macro_width),
        (0x008b_6660, vec![0, 1], vec![], ret_cell(slot(12))),
        (0x008b_9a60, vec![0, 1], vec![], ret_cell(slot(13))),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let stand = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), stand);
    }
    let callback = stand_in8(a, b, 0x80 * 8, &pick_logger(cursor, 0xf4, &[0], &[], &[]));
    let call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let w = alloc(a, b, 0x100);
    let scratch = alloc(a, b, tolib::bridge::SETUP_MESSAGE_WINDOW_SCRATCH as usize);
    let floats = [
        0.0f32,
        -0.0,
        1.5,
        -3.25,
        100.75,
        65_535.9,
        -40_000.0,
        f32::NAN,
        1.0e10,
    ];
    let float = |rng: &mut Xorshift| {
        if rng.next().is_multiple_of(3) {
            floats[(rng.next() % 9) as usize]
        } else {
            (rng.next() % 4000) as f32 / 7.0
        }
    };
    for k in 0..800u32 {
        put(a, b, cursor, log);
        for j in 0..0x40 {
            put(a, b, w + 4 * j, rng.next());
        }
        put(a, b, w + 0xf4, callback);
        put(
            a,
            b,
            w + 8,
            if rng.next().is_multiple_of(2) {
                0
            } else {
                rng.next()
            },
        );
        put(
            a,
            b,
            w + 0x22,
            [1u16, 1, 0, 2, 0xffff, 0x0101][(rng.next() % 6) as usize],
        );
        put(a, b, slot(16), rng.next() % 2);
        let wide = f64::from(float(rng)) * (1.0 + f64::from(rng.next()) / 4_294_967_296.0);
        put(a, b, slot(14), wide);
        put(a, b, w + 0x1e, [1u16, 0, 2][(rng.next() % 3) as usize]);
        put(a, b, w + 0x30, float(rng));
        put(a, b, w + 0x34, float(rng));
        for j in [0, 1, 7] {
            put(a, b, slot(j), float(rng));
        }
        for j in [2, 3] {
            put(a, b, slot(j), rng.next() % 0x200);
        }
        let small = |rng: &mut Xorshift| {
            [0u32, 1, 2, 3, 0xffff, 0x8000, rng.next()][(rng.next() % 7) as usize]
        };
        for j in [4, 5, 6, 8, 9, 10, 11, 12, 13] {
            put(a, b, slot(j), small(rng));
        }
        if rng.next().is_multiple_of(3) {
            // the word count matching the choices
            let c = Mem::read::<u32>(a, slot(10)) as i16;
            put(a, b, slot(6), i32::from(c).cast_unsigned());
            put(a, b, slot(5), i32::from(c).cast_unsigned());
        }
        put(a, b, slot(8), [1u32, 1, 0, 2][(rng.next() % 4) as usize]);
        if rng.next().is_multiple_of(6) {
            // 2^23 + (0.5 + 2^-30): rounding the sum once gives 8388609,
            // rounding the macro width to f32 first 8388608
            put(a, b, slot(0), 8_388_608.0f32);
            put(a, b, slot(14), 0.5 + f64::powi(2.0, -30));
        }
        put(a, b, scratch, [rng.next(), rng.next(), rng.next()]);
        let want = eax(a, 0x008b_8de0, &[w]);
        let got = tolib::bridge::setup_message_window(b, w, scratch, call, fcall);
        assert_eq!(want, got, "setup window {k}");
        // the port's sizes live in `scratch`, the original's on its stack
        let kept: [u32; 3] = Mem::read(b, scratch);
        Mem::write(a, scratch, kept);
        same_memory(&format!("setup window {k}"), a, b);
    }
}

#[test]
fn round36_setup_window_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round36_setup_window(&mut a, &mut b, &mut Xorshift(0x4036_0041));
}

#[test]
fn round36_setup_window_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round36_setup_window(&mut a, &mut b, &mut Xorshift(0x4036_0042));
}

/// Round 36 group E: `TOBtlDrawCtbWindow` over the original remaps, every
/// drawing callee a logging stand-in, the CTB entries test records.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round36_ctb_window(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x10000);
    let cursor = alloc(a, b, 4);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let ret = alloc(a, b, 4);
    let stands: Vec<Stand> = vec![
        (0x008f_9230, all(4), vec![], ret_cell(ret)),
        (0x008f_41b0, all(5), vec![], vec![]),
        (0x0089_e660, all(5), vec![], vec![]),
        (0x008e_6cc0, all(7), vec![], vec![]),
        (0x008e_8980, all(4), vec![], vec![]),
        (0x0090_3bb0, all(9), vec![], vec![]),
        (0x008a_16d0, all(4), vec![], vec![]),
        (0x008f_dee0, all(4), vec![], vec![]),
        (0x0090_3a10, all(3), vec![], vec![]),
        (0x0090_3ee0, all(11), vec![], vec![]),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let stand = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), stand);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    let records = alloc(a, b, 32 * 0x20);
    let sliding = a.rebase(0x008a_2b10);
    let g = |o: &Oracle, at: u32| o.rebase(at);
    let (hidden, scroll, row) = (g(a, 0x0133_d6e0), g(a, 0x0134_0446), g(a, 0x0134_0448));
    let (targets, entries, depth) = (g(a, 0x0134_0470), g(a, 0x0134_0478), g(a, 0x023c_c092));
    let open = g(a, 0x0133_c952);
    for k in 0..400u32 {
        put(a, b, cursor, log);
        put(a, b, ret, rng.next());
        put(a, b, hidden, [0u32, 0, 0, 1, 2][(rng.next() % 5) as usize]);
        put(
            a,
            b,
            scroll,
            [0u8, 1, 9, 10, 0x80, 5][(rng.next() % 6) as usize],
        );
        put(
            a,
            b,
            row,
            [0u16, 0, 1, 3, 0xffff, rng.next() as u16][(rng.next() % 6) as usize],
        );
        put(a, b, targets, rng.next());
        let d = (rng.next() % 3) as u8;
        put(a, b, depth, d);
        let window_open = rng.next().is_multiple_of(2);
        for j in 0..3u32 {
            let v = if j != u32::from(d) {
                [0u8, 2, 1][(rng.next() % 3) as usize]
            } else if window_open {
                1
            } else {
                [0u8, 2][(rng.next() % 2) as usize]
            };
            put(a, b, open + 0xf0 * j, v);
        }
        for j in 0..32u32 {
            let rec = records + 0x20 * j;
            for w in 0..8 {
                put(a, b, rec + 4 * w, rng.next());
            }
            put(
                a,
                b,
                rec + 1,
                [0u8, 5, 0x13, 0x14, 0x20, 0x80, 0xff][(rng.next() % 7) as usize],
            );
            put(
                a,
                b,
                rec + 0x10,
                ((rng.next() % 900) as u16).wrapping_sub(300),
            );
            put(
                a,
                b,
                rec + 0x12,
                ((rng.next() % 1000) as u16).wrapping_sub(300),
            );
            if rng.next().is_multiple_of(4) {
                put(a, b, rec + 0x14, sliding);
            }
            // null entries only where the second pass never reads them
            let null = (j >= 16 || !window_open) && rng.next().is_multiple_of(3);
            put(a, b, entries + 4 * j, if null { 0 } else { rec });
        }
        let x = [rng.next(), rng.next() % 640, 0x8000_0000][(rng.next() % 3) as usize];
        let y = rng.next();
        let want = eax(a, 0x008a_0590, &[x, y]);
        let mut window = btl::bridge::CtbWindow {
            mem: b,
            remap,
            call,
        };
        let got = window.draw(x.cast_signed(), y);
        if Mem::read::<u32>(a, hidden) != 1 {
            assert_eq!(want, got, "ctb {k}");
        }
        same_memory(&format!("ctb {k}"), a, b);
    }
}

#[test]
fn round36_ctb_window_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round36_ctb_window(&mut a, &mut b, &mut Xorshift(0x4036_0051));
}

#[test]
fn round36_ctb_window_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round36_ctb_window(&mut a, &mut b, &mut Xorshift(0x4036_0052));
}

/// Round 37 group A: `TOMenuDrawChrLayoutPlate_008f0a70` over the
/// original `graphicUiRemapX`/`Y`, every callee a logging stand-in, the
/// layout words and colours random.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round37_layout_plate(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use chr_plate::bridge as p;
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x80);
    let text = alloc(a, b, p::LAYOUT_TEXT_BYTES as usize);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let pkt = a.rebase(0x0187_16cc);
    // the stand-ins move the packet cursor so the result shows it
    let mut bump = vec![0x83, 0x05];
    bump.extend_from_slice(&pkt.to_le_bytes());
    bump.push(0x10);
    let stands: Vec<Stand> = vec![
        (0x0090_3bb0, all(9), vec![], bump.clone()),
        (0x0090_3ee0, all(11), vec![], bump.clone()),
        (0x0090_3a10, all(3), vec![], bump.clone()),
        (0x0090_3b70, all(7), vec![], bump.clone()),
        (0x008f_5e90, all(7), vec![], bump.clone()),
        (0x008a_99f0, all(3), vec![], ret_cell(cells)),
        (0x008a_c850, all(1), vec![], ret_cell(cells + 4)),
        (0x0090_5ab0, all(6), vec![], bump.clone()),
        (0x008a_98c0, all(1), vec![], ret_cell(cells + 8)),
        (0x008a_9990, all(1), vec![], ret_cell(cells + 0xc)),
        (0x008a_9970, all(1), vec![], ret_cell(cells + 0x10)),
        (0x008a_99b0, all(1), vec![], ret_cell(cells + 0x14)),
        (0x008a_9b80, all(1), vec![], ret_cell(cells + 0x18)),
        (0x0078_4e90, all(1), vec![], ret_cell(cells + 0x1c)),
        (0x0078_52b0, all(1), vec![], ret_cell(cells + 0x20)),
        (0x0090_1660, vec![1, 2, 3, 4, 5], vec![0], bump.clone()),
        (0x0090_19f0, all(6), vec![], bump),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let k = stands.len() as u32;
    let s = stand_in8(
        a,
        b,
        0x80 * k,
        &pick_logger(
            cursor,
            0x5f,
            &[1, 2, 3],
            &[],
            &store_through(0, cells + 0x24),
        ),
    );
    put(a, b, a.rebase(0x00b0_c3dc), s);
    let call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4800 } else { 0x0064_47c0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    let words = a.rebase(0x025d_164a);
    for k in 0..600u32 {
        put(a, b, cursor, log);
        for j in 0..0xa {
            put(a, b, cells + 4 * j, rng.next());
        }
        for j in 0..0x15 {
            let w = match rng.next() % 4 {
                0 => rng.next() as u16,
                _ => (rng.next() % 400) as u16,
            };
            put(a, b, words + 2 * j, w);
        }
        let chr = [0u32, 3, 7, 8, 12, 0x20, u32::MAX, rng.next() % 0x10][(rng.next() % 8) as usize];
        let coord = |r: &mut Xorshift| match r.next() % 5 {
            0 => r.next(),
            1 => 0x7fff_fff0 + r.next() % 0x20,
            _ => (r.next() % 1200).wrapping_sub(100),
        };
        let (x, y) = (coord(rng), coord(rng));
        let cur = rng.next();
        let want = eax(a, 0x008f_0a70, &[cur, chr, x, y]);
        let got = p::draw_layout_plate(b, [cur, chr, x, y], text, remap, call);
        assert_eq!(want, got, "layout plate {k}");
        let t: [u32; 8] = Mem::read(b, text);
        Mem::write(a, text, t);
        same_memory(&format!("layout plate {k} chr {chr:#x}"), a, b);
    }
}

#[test]
fn round37_layout_plate_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round37_layout_plate(&mut a, &mut b, &mut Xorshift(0x4037_0011));
}

#[test]
fn round37_layout_plate_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round37_layout_plate(&mut a, &mut b, &mut Xorshift(0x4037_0012));
}

/// Round 37 group B: `TOMenuDrawChrSmallPlate_008f51e0` over the original
/// remaps, with logging stand-ins for the shapes, the water, the getters
/// and the texts (as round 32's status plate).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round37_small_plate(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use chr_plate::bridge as p;
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x80);
    let scratch = alloc(a, b, p::SCRATCH_BYTES as usize);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x0090_3bb0, all(9), vec![], ret_cell(cells + 0x40)),
        (0x008e_7d30, all(12), vec![], ret_cell(cells + 0x44)),
        (0x008a_9b60, vec![], vec![], ret_cell(cells)),
        (0x008a_c2f0, vec![], vec![], ret_cell(cells + 4)),
        (0x008a_98c0, all(1), vec![], ret_cell(cells + 8)),
        (0x008a_9990, all(1), vec![], ret_cell(cells + 0xc)),
        (0x008a_9970, all(1), vec![], ret_cell(cells + 0x10)),
        (0x008a_99b0, all(1), vec![], ret_cell(cells + 0x14)),
        (0x008a_9a70, all(1), vec![], ret_cell(cells + 0x24)),
        (0x008a_c850, all(1), vec![], ret_cell(cells + 0x28)),
        (0x0090_5930, all(6), vec![], ret_cell(cells + 0x2c)),
        (
            0x0090_1660,
            vec![1, 2, 3, 4, 5],
            vec![0],
            ret_cell(cells + 0x30),
        ),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let k = stands.len() as u32;
    let s = stand_in8(
        a,
        b,
        0x80 * k,
        &pick_logger(
            cursor,
            0x5f,
            &[1, 2, 3],
            &[],
            &store_through(0, cells + 0x50),
        ),
    );
    put(a, b, a.rebase(0x00b0_c3dc), s);
    let call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    for k in 0..600u32 {
        put(a, b, cursor, log);
        for j in 0..0x15 {
            put(a, b, cells + 4 * j, rng.next());
        }
        put(
            a,
            b,
            cells,
            [0u32, u32::MAX, rng.next()][(rng.next() % 3) as usize],
        );
        put(
            a,
            b,
            cells + 4,
            [0xbu32, 0xb, 0, 1, 2][(rng.next() % 5) as usize],
        );
        put(a, b, cells + 0x50, rng.next() & 0x00ff_ffff);
        let chr = [0u32, 3, 7, 8, 12, 0x20, u32::MAX, rng.next() % 0x14][(rng.next() % 8) as usize];
        let x = ((rng.next() % 8000) as f32 / 7.0 - 100.0).to_bits();
        let y = ((rng.next() % 6000) as f32 / 3.0 - 50.0).to_bits();
        let r = eax(a, 0x008f_51e0, &[chr, x, y]);
        assert_eq!(
            r,
            p::draw_chr_small_plate(b, [chr, x, y], scratch, remap, call),
            "small plate {k}"
        );
        let s: [u32; 9] = Mem::read(b, scratch);
        Mem::write(a, scratch, s);
        same_memory(&format!("small plate {k} chr {chr:#x}"), a, b);
    }
}

#[test]
fn round37_small_plate_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round37_small_plate(&mut a, &mut b, &mut Xorshift(0x4037_0021));
}

#[test]
fn round37_small_plate_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round37_small_plate(&mut a, &mut b, &mut Xorshift(0x4037_0022));
}

/// A cdecl stand-in logging a 0x98-byte UI record (argument 0): its two
/// corners (0x40 bytes), `+0x80`, `+0x88`, `+0x90`, `+0x94`, then
/// arguments `1..args` and the tag.
fn ui_record_logger(cursor: u32, tag: u32, args: u8) -> Vec<u8> {
    // push esi; push edi; mov esi, [esp+0xc]; mov edi, [cursor]
    let mut v = vec![0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c, 0x8b, 0x3d];
    v.extend_from_slice(&cursor.to_le_bytes());
    // mov ecx, 0x10; cld; rep movsd
    v.extend_from_slice(&[0xb9, 0x10, 0, 0, 0, 0xfc, 0xf3, 0xa5]);
    // mov eax, [esi+off]; stosd (esi is record + 0x40)
    for off in [0x40u8, 0x48, 0x50, 0x54] {
        v.extend_from_slice(&[0x8b, 0x46, off, 0xab]);
    }
    // mov eax, [esp+0x10 + 4k]; stosd
    for k in 0..args - 1 {
        v.extend_from_slice(&[0x8b, 0x44, 0x24, 0x10 + 4 * k, 0xab]);
    }
    // mov eax, tag; stosd
    v.push(0xb8);
    v.extend_from_slice(&tag.to_le_bytes());
    v.push(0xab);
    // mov [cursor], edi; pop edi; pop esi; ret
    v.extend_from_slice(&[0x89, 0x3d]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0x5f, 0x5e, 0xc3]);
    v
}

/// Round 37 group C: `TOMakePktSeqPrimLayout_008fe0b0` over the original
/// `graphicUiRemapX2`/`Y2`: the layout copies return a test layout, the
/// image size comes from test cells, the shapes and the cursor log, the
/// UI record is logged by `ShapeDataModify` and `graphicDrawUIElement`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
fn round37_seq_prim_layout(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use seq_prim::bridge as t;
    let log = alloc(a, b, 0x10000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x40);
    let layout = alloc(a, b, 4 + 0xc * 8);
    let prim = alloc(a, b, 0x18);
    let scratch = alloc(a, b, t::SEQ_PRIM_LAYOUT_SCRATCH as usize);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    // TOGetImageWH: width and height from cells + 8 / + 0xc through
    // arguments 1 and 2
    let mut wh_tail = Vec::new();
    for (k, cell) in [(0u8, cells + 8), (1, cells + 0xc)] {
        // mov edx, [esp+8+4k]; mov eax, [cell]; mov [edx], eax
        wh_tail.extend_from_slice(&[0x8b, 0x54, 0x24, 8 + 4 * k, 0xa1]);
        wh_tail.extend_from_slice(&cell.to_le_bytes());
        wh_tail.extend_from_slice(&[0x89, 0x02]);
    }
    let stands: Vec<(u32, Vec<u8>)> = vec![
        (
            0x0090_3bb0,
            pick_logger(cursor, 0x3bb0, &all(9), &[], &ret_cell(cells + 0x10)),
        ),
        (
            0x0090_3ee0,
            pick_logger(cursor, 0x3ee0, &all(11), &[], &ret_cell(cells + 0x14)),
        ),
        (
            0x008f_a7d0,
            pick_logger(cursor, 0xa7d0, &all(1), &[], &ret_cell(cells)),
        ),
        (
            0x008f_a450,
            pick_logger(cursor, 0xa450, &all(1), &[], &ret_cell(cells)),
        ),
        (
            0x008a_c8c0,
            pick_logger(cursor, 0xc8c0, &all(1), &[], &ret_cell(cells + 4)),
        ),
        (
            0x008a_c400,
            pick_logger(cursor, 0xc400, &all(1), &[], &wh_tail),
        ),
        (
            0x008f_59c0,
            pick_logger(cursor, 0x59c0, &all(8), &[], &ret_cell(cells + 0x18)),
        ),
        (0x008e_8a10, ui_record_logger(cursor, 0x8a10, 2)),
        (0x0063_eee0, ui_record_logger(cursor, 0xeee0, 5)),
    ];
    for (k, (sym, code)) in (0u32..).zip(&stands) {
        let stand = stand_in8(a, b, 0x80 * k, code);
        redirect(a, b, *sym, stand);
    }
    let call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    let kinds = [
        0x19fu16, 0x1a3, 0x19b, 0x19c, 0x1fb, 0x1a1, 0x1a1, 0xf0, 0x19d, 0x19d, 0x19e, 0x1f7,
        0x1fc, 0, 199, 200, 399, 400, 599, 0x1a0,
    ];
    let sizes = [1.0f32, 256.0, 512.0, 3.0, 1.0e-30, 1024.0, 77.5];
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        put(a, b, cells, layout);
        for j in 1..0x10 {
            put(a, b, cells + 4 * j, rng.next());
        }
        for j in 0..2 {
            let s = if rng.next().is_multiple_of(3) {
                sizes[(rng.next() % sizes.len() as u32) as usize]
            } else {
                (rng.next() % 1024 + 1) as f32
            };
            put(a, b, cells + 8 + 4 * j, s);
        }
        let count = [0u32, 1, 1, 2, 3, 5, 8][(rng.next() % 7) as usize];
        put(a, b, layout, count);
        for j in 0..8 {
            let e = layout + 4 + 0xc * j;
            for w in 0..3 {
                put(a, b, e + 4 * w, rng.next());
            }
            put(a, b, e + 4, (rng.next() % 300) as u16);
            put(a, b, e + 6, (rng.next() % 300) as u16);
            put(a, b, e + 8, [0u16, 1, 1, 2][(rng.next() % 4) as usize]);
            put(
                a,
                b,
                e + 0xa,
                [0u16, 1, 1, 0xffff][(rng.next() % 4) as usize],
            );
        }
        let kind = if rng.next().is_multiple_of(3) {
            (rng.next() % 600) as u16
        } else {
            kinds[(rng.next() % kinds.len() as u32) as usize]
        };
        for w in 0..5 {
            put(a, b, prim + 4 * w, rng.next());
        }
        put(a, b, prim + 0x14, kind);
        let word = |r: &mut Xorshift, near: i16| match r.next() % 4 {
            0 => r.next() as u16,
            _ => near
                .cast_unsigned()
                .wrapping_add((r.next() % 600) as u16)
                .wrapping_sub(300),
        };
        let x = if rng.next().is_multiple_of(4) {
            0x74
        } else {
            word(rng, 200)
        };
        put(a, b, prim + 0xc, x);
        put(a, b, prim + 0xe, word(rng, 300));
        for at in [0x10, 0x12] {
            let s = match rng.next() % 4 {
                0 | 1 => 100,
                2 => (rng.next() % 300) as u16,
                _ => rng.next() as u16,
            };
            put(a, b, prim + at, s);
        }
        if kind == 0x19d && rng.next().is_multiple_of(3) {
            // the cursor's corner exactly on its line (y 300)
            put(a, b, layout + 4 + 3, 0u8);
            put(a, b, layout + 4 + 0xa, 0u16);
            put(a, b, prim + 0x12, 100u16);
            put(a, b, prim + 0xe, 300u16);
        }
        for j in 0..(t::SEQ_PRIM_LAYOUT_SCRATCH / 4) {
            put(a, b, scratch + 4 * j, rng.next());
        }
        let want = eax(a, 0x008f_e0b0, &[prim]);
        let got = t::make_pkt_seq_prim_layout(b, prim, scratch, remap, call);
        assert_eq!(want, got, "seq prim layout {k} kind {kind:#x}");
        // the port's record and image size live in `scratch`, the
        // original's on its stack
        let kept: [u32; 40] = Mem::read(b, scratch);
        Mem::write(a, scratch, kept);
        same_memory(&format!("seq prim layout {k} kind {kind:#x}"), a, b);
    }
}

#[test]
fn round37_seq_prim_layout_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round37_seq_prim_layout(&mut a, &mut b, &mut Xorshift(0x4037_0031));
}

#[test]
fn round37_seq_prim_layout_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round37_seq_prim_layout(&mut a, &mut b, &mut Xorshift(0x4037_0032));
}

/// Round 37 group D: `fix_mappic` over every map number around the valid
/// range, the player's position random, near and between the two
/// pictures' points.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn round37_fix_mappic(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let pos = a.rebase(save::bridge::PLAYER_POSITION);
    let points = [
        [-443.13, 159.49, 658.23],
        [-10.62, 13.39, 241.57],
        [1301.0, -85.0, -1365.0],
        [1620.0, -85.0, -1707.5],
        [63.75, -117.39, -130.37],
        [1.93, -155.28, -34.8],
        [-13.93, -0.83, 156.05],
        [13.86, 0.0, 1058.92],
        [-63.35, -459.72, 105.54],
        [153.26, -75.89, -972.12],
        [-41.59, -1.0, -132.01],
        [3.71, -20.88, -743.03],
        [32.05, 95.34, 1249.66],
        [34.7, 8.87, 261.22],
        [0.0, 0.0, 0.0],
    ];
    for k in 0..12000u32 {
        let map = match k % 3 {
            0 => k / 3 % 0x170,
            1 => [
                0u32,
                0x15c,
                0x15d,
                u32::MAX,
                0xd8,
                0x1f,
                0x101,
                0x11c,
                0x81,
                0x15c,
                0xf4,
                0xf5,
                0xf6,
                0xf7,
            ][(rng.next() % 14) as usize],
            _ => rng.next(),
        };
        for j in 0..3u32 {
            let base = points[(rng.next() % points.len() as u32) as usize][j as usize];
            let v = match rng.next() % 4 {
                0 => f32::from_bits(rng.next() & 0x7f7f_ffff | (rng.next() & 0x8000_0000)),
                1 => base,
                _ => base + ((rng.next() % 4000) as f32 - 2000.0) / 3.0,
            };
            put(a, b, pos + 4 * j, v);
        }
        let want = eax(a, 0x006e_f6f0, &[map]);
        assert_eq!(
            want,
            save::bridge::fix_mappic(b, map),
            "fix_mappic {k} map {map:#x}"
        );
    }
}

#[test]
fn round37_fix_mappic_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round37_fix_mappic(&mut a, &mut b, &mut Xorshift(0x4037_0041));
}

#[test]
fn round37_fix_mappic_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round37_fix_mappic(&mut a, &mut b, &mut Xorshift(0x4037_0042));
}

/// Round 37 group E: `TODrawFont3d80String_008ee060` over a cursor-moving
/// glyph stand-in, the three `TOMenuGet*` text getters over logging
/// `MsGetSaveConfigHiragana`/`MsMenuGetText`/`MsMenuGetHelp` stand-ins, and
/// `BtlEnd_InitState_008bdeb0` over logging callees.
#[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
fn round37_tiny(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    // the glyph returns its cursor argument + 0x10 (lea eax, [ecx+0x10]
    // after mov ecx, [esp+4])
    let glyph_tail = vec![0x8b, 0x4c, 0x24, 0x04, 0x8d, 0x41, 0x10];
    let stands: Vec<Stand> = vec![
        (0x008e_eee0, all(9), vec![1], glyph_tail),
        (0x008f_2f70, all(1), vec![], ret_cell(cells)),
        (0x0078_51f0, vec![], vec![], ret_cell(cells + 4)),
        (0x0078_fcf0, all(3), vec![], ret_cell(cells + 8)),
        (0x0078_fbb0, all(3), vec![], ret_cell(cells + 0xc)),
        (0x0086_f3a0, all(1), vec![], ret_cell(cells + 0x10)),
        (0x008b_de00, vec![], vec![], ret_cell(cells + 0x14)),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let text = alloc(a, b, 0x20);
    for k in 0..800u32 {
        put(a, b, cursor, log);
        for j in 0..8 {
            put(a, b, cells + 4 * j, rng.next());
        }
        let len = (rng.next() % 12) as usize;
        for j in 0..0x20u32 {
            let c = if (j as usize) < len {
                (rng.next() % 255 + 1) as u8
            } else {
                0
            };
            put(a, b, text + j, c);
        }
        let args = [
            rng.next(),
            text,
            [rng.next(), 0x7fff_fffa, rng.next() % 640][(rng.next() % 3) as usize],
            rng.next(),
            rng.next(),
            rng.next(),
            rng.next(),
            rng.next(),
            rng.next(),
        ];
        let want = eax(a, 0x008e_e060, &args);
        let got = tolib::bridge::draw_font_3d80_string(b, args, call);
        assert_eq!(want, got, "string {k}");
        let id = rng.next();
        for (f, port) in [
            (
                0x008b_ee30u32,
                menu::bridge::get_save_mes as fn(&mut Oracle, u32, _) -> u32,
            ),
            (0x008b_ee50, menu::bridge::get_status_help),
            (0x008b_ee90, menu::bridge::get_summon_help),
        ] {
            let want = eax(a, f, &[id]);
            assert_eq!(want, port(b, id, call), "menu text {f:#x} {k}");
        }
        let kind = [1u32, 2, 0, 3, 0x101, 0x102, rng.next()][(rng.next() % 7) as usize];
        for at in [0x0186_9f08u32, 0x0186_9f0c, 0x0186_9fbc, 0x00c5_a068] {
            put(a, b, a.rebase(at), rng.next());
        }
        let want = eax(a, 0x008b_deb0, &[kind]);
        assert_eq!(
            want,
            btl::bridge::btl_end_init_state(b, kind, call),
            "btl end {k}"
        );
        same_memory(&format!("round37 small {k}"), a, b);
    }
}

#[test]
fn round37_tiny_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round37_tiny(&mut a, &mut b, &mut Xorshift(0x4037_0051));
}

#[test]
fn round37_tiny_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round37_tiny(&mut a, &mut b, &mut Xorshift(0x4037_0052));
}

/// Round 37 group F: the battle windows' wait and open functions,
/// `TOFuncPadMessageNormal` and `TOMenuEndScreenX2_008e2790` over logging
/// stand-ins.
#[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
fn round37_win(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge::{self as t, WaitBtlWin};
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x008a_22f0, all(1), vec![], ret_cell(cells)),
        (0x008a_2690, all(1), vec![], ret_cell(cells)),
        (0x0089_cde0, all(1), vec![], ret_cell(cells)),
        (0x0089_d610, all(1), vec![], ret_cell(cells)),
        (0x008a_f0d0, all(2), vec![1], ret_cell(cells + 4)),
        (0x008b_8ca0, all(1), vec![], ret_cell(cells + 8)),
        (0x0067_ab50, all(1), vec![], ret_cell(cells + 0xc)),
        (0x008b_9220, all(1), vec![], ret_cell(cells + 0x10)),
        (0x008a_b020, all(1), vec![], ret_cell(cells + 0x14)),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let w = alloc(a, b, 0x100);
    let opens = [
        0.0f32,
        0.3,
        0.299_999_98,
        0.300_000_04,
        0.45,
        0.449_999_96,
        0.450_000_02,
        1.0,
        -1.0,
        f32::NAN,
        f32::INFINITY,
    ];
    let trigger = a.rebase(0x025d_09d4);
    let map = a.rebase(0x0133_0844);
    let flag = a.rebase(0x0134_0858);
    for k in 0..800u32 {
        put(a, b, cursor, log);
        for j in 0..8 {
            put(a, b, cells + 4 * j, rng.next());
        }
        put(
            a,
            b,
            cells + 0xc,
            [0u32, 0, 1, rng.next()][(rng.next() % 4) as usize],
        );
        for j in 0..0x40 {
            put(a, b, w + 4 * j, rng.next());
        }
        let open = if rng.next().is_multiple_of(4) {
            f32::from_bits(rng.next() & 0x7f7f_ffff)
        } else {
            opens[(rng.next() % opens.len() as u32) as usize]
        };
        put(a, b, w + 0xe0, open);
        for (f, kind) in [
            (0x008a_8a40u32, WaitBtlWin::Left),
            (0x008a_8af0, WaitBtlWin::Right),
            (0x008a_8b20, WaitBtlWin::Std),
            (0x008a_8ac0, WaitBtlWin::Prepare),
        ] {
            let want = eax(a, f, &[w]);
            assert_eq!(want, t::wait_btl_win(b, kind, w, call), "{kind:?} {k}");
        }
        for (f, right) in [(0x008a_8620u32, false), (0x008a_86d0, true)] {
            let want = eax(a, f, &[]);
            assert_eq!(want, t::open_btl_win_side(b, right, call), "open {k}");
        }
        put(a, b, trigger, rng.next() as u8);
        put(a, b, map, rng.next());
        let want = eax(a, 0x008b_64a0, &[w]);
        assert_eq!(
            want,
            t::func_pad_message_normal(b, w, call),
            "pad message {k}"
        );
        // the flag byte and the three after it
        put(a, b, flag, rng.next());
        let want = eax(a, 0x008e_2790, &[w]);
        assert_eq!(want, t::menu_end_screen_x2(b, w, call), "screen x2 {k}");
        same_memory(&format!("round37 win {k}"), a, b);
    }
}

#[test]
fn round37_win_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round37_win(&mut a, &mut b, &mut Xorshift(0x4037_0061));
}

#[test]
fn round37_win_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round37_win(&mut a, &mut b, &mut Xorshift(0x4037_0062));
}

/// Round 37 group G: the battle menu's character lists, status ids, help
/// text, info window line and Libra name over logging stand-ins (the
/// window lists, text sources and status data are test memory).
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round37_btl(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x40);
    let lists = alloc(a, b, 0x40);
    let texts = alloc(a, b, 0x200);
    let out = texts + 0x180;
    let scratch = alloc(a, b, 0x10);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    // MsGetSaveWindow(chr, kind, &count): unless cells + 0x30 is 0, count
    // from cells + 0x20 or + 0x24 by the call; returns the list pointer
    let mut window_tail = vec![0x83, 0x3d];
    window_tail.extend_from_slice(&(cells + 0x30).to_le_bytes());
    window_tail.extend_from_slice(&[0x00, 0x74, 0x0b]);
    window_tail.extend_from_slice(&[0x8b, 0x54, 0x24, 0x0c, 0xa1]);
    window_tail.extend_from_slice(&(cells + 0x20).to_le_bytes());
    window_tail.extend_from_slice(&[0x89, 0x02]);
    window_tail.extend_from_slice(&[0x87, 0x05]);
    window_tail.extend_from_slice(&(cells + 0x24).to_le_bytes());
    window_tail.push(0xa3);
    window_tail.extend_from_slice(&(cells + 0x20).to_le_bytes());
    window_tail.extend_from_slice(&ret_cell(cells));
    // MsGetComData(id, &base): base from cells + 0x2c, returns texts +
    // 0x1f0 (its u16 +8 the text offset)
    let mut com_tail = vec![0x8b, 0x54, 0x24, 0x08, 0xa1];
    com_tail.extend_from_slice(&(cells + 0x2c).to_le_bytes());
    com_tail.extend_from_slice(&[0x89, 0x02, 0xb8]);
    com_tail.extend_from_slice(&(texts + 0x1f0).to_le_bytes());
    // TOGetMessageWH(k, text, &w, &h, ...): w from cells + 0x38
    let mut wh_tail = vec![0x8b, 0x54, 0x24, 0x0c, 0xa1];
    wh_tail.extend_from_slice(&(cells + 0x38).to_le_bytes());
    wh_tail.extend_from_slice(&[0x89, 0x02]);
    wh_tail.extend_from_slice(&ret_cell(cells + 0x3c));
    let stands: Vec<Stand> = vec![
        (0x0078_5410, all(2), vec![], window_tail),
        (0x0078_f930, all(1), vec![], ret_cell(cells + 4)),
        (0x0079_afc0, all(1), vec![], ret_cell(cells + 8)),
        (0x0078_fa80, all(1), vec![], ret_cell(cells + 0xc)),
        (0x0078_f950, all(1), vec![], ret_cell(cells + 0x10)),
        (0x0078_f7c0, all(2), vec![], ret_cell(cells + 0x14)),
        (0x0079_a4b0, all(1), vec![], com_tail),
        (0x008b_9650, vec![0, 1, 4, 5], vec![1], wh_tail),
        (0x0089_7bd0, vec![], vec![], ret_cell(cells + 0x18)),
        (0x0090_16b0, all(7), vec![1], ret_cell(cells + 0x1c)),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    let g = |o: &Oracle, at: u32| o.rebase(at);
    let globals = [
        (g(a, 0x0133_f0d8), 0x40u32),
        (g(a, 0x0133_f530), 0x100),
        (g(a, 0x0133_f618), 0x80),
        (g(a, 0x0133_f688), 0x30),
    ];
    for k in 0..600u32 {
        put(a, b, cursor, log);
        for j in 0..0x10 {
            put(a, b, cells + 4 * j, rng.next());
        }
        for &(at, len) in &globals {
            for j in 0..len / 4 {
                put(a, b, at + 4 * j, rng.next());
            }
        }
        // the two windows' lists: entries with 0xff tails
        put(a, b, cells, lists);
        put(a, b, cells + 0x30, rng.next() % 3);
        for j in 0..0x20u32 {
            let v = if rng.next().is_multiple_of(3) {
                0xff
            } else {
                rng.next() as u16
            };
            put(a, b, lists + 2 * j, v);
        }
        put(
            a,
            b,
            cells + 0x20,
            [0u32, 1, 3, 8, 0xffff_ffff][(rng.next() % 5) as usize],
        );
        put(
            a,
            b,
            cells + 0x24,
            [0u32, 2, 5, 8][(rng.next() % 4) as usize],
        );
        let want = eax(a, 0x0089_79d0, &[]);
        assert_eq!(want, t::btl_chr_num(b, scratch, call), "chr num {k}");
        let s: [u32; 4] = Mem::read(b, scratch);
        Mem::write(a, scratch, s);
        // the status ids
        put(a, b, cells + 0xc, texts);
        for j in 0..0x10u32 {
            put(a, b, texts + j, [0u8, 0, 1, 7][(rng.next() % 4) as usize]);
        }
        put(a, b, cells + 8, [0u32, 1, 2][(rng.next() % 3) as usize]);
        let chr = rng.next() % 0x20;
        let want = eax(a, 0x0089_8260, &[chr, out]);
        assert_eq!(
            want,
            t::btl_make_status_buff(b, chr, out, call),
            "status {k}"
        );
        // the help text: a source string of random length
        let len = rng.next() % 0x60;
        for j in 0..0x100u32 {
            let c = if j < len {
                (rng.next() % 255 + 1) as u8
            } else {
                0
            };
            put(a, b, texts + 0x80 + j, c);
        }
        put(a, b, cells + 0x14, texts + 0x80);
        put(a, b, cells + 0x2c, texts + 0x80);
        put(a, b, texts + 0x1f8, (rng.next() % 0x40) as u16);
        put(
            a,
            b,
            cells + 0x38,
            [1.0f32, 100.0, 255.5, -3.0, 1.0e9][(rng.next() % 5) as usize],
        );
        let n = [0u32, 7, 8, 0x11, 0x12, 0x30ff, u32::MAX, rng.next() % 0x20]
            [(rng.next() % 8) as usize];
        let want = eax(a, 0x0089_97c0, &[n]);
        let got = t::btl_set_change_help_mes(b, n, scratch, call);
        if (0..=0x11).contains(&n.cast_signed()) || n == 0x30ff {
            assert_eq!(want, got, "help {k}");
        }
        let s: [u32; 4] = Mem::read(b, scratch);
        Mem::write(a, scratch, s);
        // the info window line (its width sometimes exactly the limit) and
        // the Libra name
        if rng.next().is_multiple_of(3) {
            let limit: i16 = Mem::read(a, a.rebase(0x0133_f61e));
            put(a, b, cells + 0x38, f32::from(limit));
        }
        let want = eax(a, 0x0089_9f20, &[chr]);
        assert_eq!(
            want,
            t::btl_set_info_win_mes(b, chr, scratch, call),
            "info {k}"
        );
        let s: [u32; 4] = Mem::read(b, scratch);
        Mem::write(a, scratch, s);
        let x = [rng.next(), rng.next() % 1280, 0x8000_0000][(rng.next() % 3) as usize];
        let want = eax(a, 0x0089_c7d0, &[x]);
        let got = t::btl_draw_libra_right(b, x.cast_signed(), scratch, remap, call);
        assert_eq!(want, got, "libra {k}");
        let s: [u32; 4] = Mem::read(b, scratch);
        Mem::write(a, scratch, s);
        same_memory(&format!("round37 btl {k}"), a, b);
    }
}

#[test]
fn round37_btl_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round37_btl(&mut a, &mut b, &mut Xorshift(0x4037_0071));
}

#[test]
fn round37_btl_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round37_btl(&mut a, &mut b, &mut Xorshift(0x4037_0072));
}

/// Round 37 group H: the battle info window's status cycle
/// (`TOBtlSetInfoWinParam`, `TOBtlDrawInfoWinStatus`) over logging
/// stand-ins and the original remaps.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn round37_info(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x40);
    let counts = alloc(a, b, 0x10);
    let scratch = alloc(a, b, 0x10);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    // a float from `cell` through argument `k`, then eax from `ret`
    let store_float = |k: u8, cell: u32, ret: u32| {
        let mut v = vec![0x8b, 0x54, 0x24, 4 + 4 * k, 0xa1];
        v.extend_from_slice(&cell.to_le_bytes());
        v.extend_from_slice(&[0x89, 0x02]);
        v.extend_from_slice(&ret_cell(ret));
        v
    };
    let stands: Vec<Stand> = vec![
        (0x0079_add0, all(1), vec![], ret_cell(cells)),
        (0x0079_adf0, all(1), vec![], ret_cell(cells + 4)),
        (0x0078_f930, all(1), vec![], ret_cell(cells + 8)),
        (0x0078_fa80, all(1), vec![], ret_cell(cells + 0xc)),
        (0x0078_f950, all(1), vec![], ret_cell(cells + 0x10)),
        (0x0078_f7c0, all(2), vec![], ret_cell(cells + 0x14)),
        (
            0x008b_9650,
            vec![0, 1, 4, 5],
            vec![],
            store_float(2, cells + 0x18, cells + 0x1c),
        ),
        (0x0078_fcf0, all(3), vec![], ret_cell(cells + 0x20)),
        (
            0x0090_5290,
            vec![0, 2, 3, 4],
            vec![],
            store_float(1, cells + 0x24, cells + 0x28),
        ),
        (0x0090_5930, all(6), vec![], ret_cell(cells + 0x2c)),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let s = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), s);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        m.call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32
    };
    let window = a.rebase(0x0133_f5c0);
    let floats = [0.0f32, 1.5, 99.9, 380.0, 1.0e6, -12.25];
    for k in 0..800u32 {
        put(a, b, cursor, log);
        for j in 0..0x10 {
            put(a, b, cells + 4 * j, rng.next());
        }
        for j in 0..0x40 {
            put(a, b, window + 4 * j, rng.next());
        }
        put(a, b, cells + 0xc, counts);
        for j in 0..0x10u32 {
            put(a, b, counts + j, [0u8, 0, 3][(rng.next() % 3) as usize]);
        }
        put(a, b, cells + 0x18, floats[(rng.next() % 6) as usize]);
        put(a, b, cells + 0x24, floats[(rng.next() % 6) as usize]);
        let chr = [0u32, 5, 0x13, 0x20, rng.next()][(rng.next() % 5) as usize];
        if rng.next().is_multiple_of(2) {
            let want = eax(a, 0x0089_9fd0, &[chr]);
            assert_eq!(
                want,
                t::btl_set_info_win_param(b, chr, scratch, call),
                "param {k}"
            );
            let s: [u32; 4] = Mem::read(b, scratch);
            Mem::write(a, scratch, s);
        } else {
            // a cycle mid-way: timer, shown entry and count small
            put(
                a,
                b,
                window + 2,
                [0u16, 1, 0x14, 0xffff][(rng.next() % 4) as usize],
            );
            let count = (rng.next() % 6) as u16;
            put(a, b, window + 6, count);
            put(a, b, window + 4, (rng.next() % 7) as u16);
        }
        for _ in 0..3 {
            let x = [rng.next(), rng.next() % 1280][(rng.next() % 2) as usize];
            let y = [rng.next(), rng.next() % 720][(rng.next() % 2) as usize];
            let want = eax(a, 0x0089_5240, &[rng.next(), x, y]);
            let got = t::btl_draw_info_win_status(
                b,
                [x.cast_signed(), y.cast_signed()],
                scratch,
                remap,
                call,
            );
            assert_eq!(want, got, "status {k}");
            let s: [u32; 4] = Mem::read(b, scratch);
            Mem::write(a, scratch, s);
        }
        same_memory(&format!("round37 info {k}"), a, b);
    }
}

#[test]
fn round37_info_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round37_info(&mut a, &mut b, &mut Xorshift(0x4037_0081));
}

#[test]
fn round37_info_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round37_info(&mut a, &mut b, &mut Xorshift(0x4037_0082));
}

/// A float for the Ke thing cases: no signalling NaNs (the oracle keeps
/// them signalling through `fld`).
fn round38_quiet(rng: &mut Xorshift) -> u32 {
    loop {
        let f = rng.float();
        if !(f.is_nan() && f.to_bits() & 0x0040_0000 == 0) {
            return f.to_bits();
        }
    }
}

/// Round 38 group A: `setKeThPointColor` over the original
/// `FFXVu0ApplyMatrix`, every table (and one it ignores), rows below and
/// past the tables' first rows.
fn round38_point_color(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let work = b.stack_range().start + 0x1000;
    let pos = alloc(a, b, 0x10);
    let colour = alloc(a, b, 0x10);
    let unproject = a.rebase(render::bridge::KETH_UNPROJECT);
    for k in 0..400u32 {
        if k % 4 == 0 {
            for j in 0..16 {
                let v = round38_quiet(rng);
                put(a, b, unproject + 4 * j, v);
            }
        }
        for j in 0..4 {
            let v = round38_quiet(rng);
            put(a, b, pos + 4 * j, v);
            let v = round38_quiet(rng);
            put(a, b, colour + 4 * j, v);
        }
        let alpha = round38_quiet(rng);
        let table = match rng.next() % 9 {
            0 => 3,
            1 => u32::MAX,
            n => n % 3,
        };
        let i = (rng.next() % 14).wrapping_sub(2);
        let row = rng.next() % 8;
        let args = [pos, colour, alpha, table, i, row];
        assert_eq!(
            eax(a, 0x0076_65c0, &args),
            render::bridge::set_keth_point_color(b, args, work, call),
            "point colour {k}"
        );
        same_memory(&format!("point colour {k}"), a, b);
    }
}

#[test]
fn round38_point_color_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round38_point_color(&mut a, &mut b, &mut Xorshift(0x4038_0011));
}

#[test]
fn round38_point_color_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round38_point_color(&mut a, &mut b, &mut Xorshift(0x4038_0012));
}

/// Round 38 group A: `updateVFXKeThRenderData`, its lookup and buffer
/// calls logging stand-ins returning test words, `updateVFXKeThSetPoint`
/// the original; the last part's index list sometimes lies in the part
/// itself (past its pointer).
#[allow(clippy::cast_possible_wrap)]
fn round38_keth_render_data(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 8);
    let stands: Vec<Stand> = vec![
        (0x0064_53d0, vec![0, 1], vec![], ret_cell(cells)),
        (0x0063_fdb0, vec![0, 1, 2, 3], vec![], ret_cell(cells + 4)),
    ];
    for (k, (sym, args, texts, tail)) in (0u32..).zip(&stands) {
        let stand = stand_in8(
            a,
            b,
            0x80 * k,
            &pick_logger(cursor, *sym, args, texts, tail),
        );
        redirect(a, b, sym.to_owned(), stand);
    }
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let data = alloc(a, b, 0x100);
    // part -1 (a negative texture count) lies before the first
    let parts = alloc(a, b, 0x6c * 7 + 0x200) + 0x6c;
    let lists = alloc(a, b, 0x400);
    let positions = alloc(a, b, 0x1000);
    let colours = alloc(a, b, 0x1000);
    let count = a.rebase(render::bridge::KETH_TEXTURE_COUNT);
    for k in 0..300u32 {
        put(a, b, cursor, log);
        let part = (rng.next() % 6) as i32 - 1;
        put(a, b, count, part);
        put(a, b, data + 0x90, rng.next() % 6);
        put(a, b, data + 0x94, parts);
        put(
            a,
            b,
            cells,
            if rng.next().is_multiple_of(6) {
                0
            } else {
                data
            },
        );
        put(
            a,
            b,
            cells + 4,
            if rng.next().is_multiple_of(6) {
                0
            } else {
                rng.next()
            },
        );
        for j in 0..0x6c * 7 / 4 {
            put(a, b, parts - 0x6c + 4 * j, rng.next());
        }
        for j in 0..0x100 {
            put(a, b, lists + 4 * j, rng.next());
        }
        // every part (the one picked may be any of them) gets lists
        for p in 0..7 {
            let mesh = parts + 0x6c * p - 0x6c;
            let list = match rng.next() % 4 {
                0 if p == 6 => mesh + 0x20,
                1 => lists + 2 * (rng.next() % 0x40),
                _ => lists,
            };
            put(a, b, mesh + 0x1c, list);
            put(a, b, mesh + 0xc, positions);
            put(a, b, mesh + 0x14, colours);
        }
        // the point tables and the lists they are copied to hold test words
        if k % 8 == 0 {
            for (pos, col) in render::bridge::KETH_POINT_TABLES {
                for j in 0..0x60 {
                    let w = round38_quiet(rng);
                    put(a, b, a.rebase(pos) + 4 * j, w);
                    let w = round38_quiet(rng);
                    put(a, b, a.rebase(col) + 4 * j, w);
                }
            }
        }
        for j in 0..0x400 {
            put(a, b, positions + 4 * j, rng.next());
            put(a, b, colours + 4 * j, rng.next());
        }
        let n = (rng.next() % 8) as i32 - 1;
        let (lo, hi) = (rng.next(), rng.next());
        assert_eq!(
            eax(a, 0x0076_6770, &[lo, hi, n.cast_unsigned()]),
            render::bridge::update_vfx_keth_render_data(b, lo, hi, n, call),
            "keth render data {k}"
        );
        same_memory(&format!("keth render data {k}"), a, b);
    }
}

#[test]
fn round38_keth_render_data_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round38_keth_render_data(&mut a, &mut b, &mut Xorshift(0x4038_0021));
}

#[test]
fn round38_keth_render_data_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round38_keth_render_data(&mut a, &mut b, &mut Xorshift(0x4038_0022));
}

/// A stand-in tail: `eax` = the next word of `table` (the shared
/// `counter` counts the calls).
fn round38_seq(counter: u32, table: u32) -> Vec<u8> {
    // mov ecx, [counter]; mov eax, [table + ecx*4]; inc dword [counter]
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&counter.to_le_bytes());
    v.extend_from_slice(&[0x8b, 0x04, 0x8d]);
    v.extend_from_slice(&table.to_le_bytes());
    v.extend_from_slice(&[0xff, 0x05]);
    v.extend_from_slice(&counter.to_le_bytes());
    v
}

/// Logging stand-ins (`syms`: address, logged arguments, tail) in slots
/// from `slot0`.
fn round38_stands(a: &mut Oracle, b: &mut Oracle, cursor: u32, slot0: u32, stands: &[Stand]) {
    for (k, (sym, args, texts, tail)) in (slot0..).zip(stands) {
        let code = pick_logger(cursor, *sym, args, texts, tail);
        let s = stand_in8(a, b, 0x80 * k, &code);
        redirect(a, b, *sym, s);
    }
}

/// Round 38 group B: `TOBtlCtrlStatusWin` over logging stand-ins (the
/// value getters return words of a table of a few values, so the rows'
/// targets are met now and then); the party, the rows and the mode random.
#[allow(clippy::too_many_lines)]
fn round38_ctrl_status(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let cells = alloc(a, b, 0x10);
    let seq = round38_seq(counter, table);
    // MsGetSaveWindow(0, 0x12, &n): n = cells[0], returns cells[1]
    let mut save = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d];
    save.extend_from_slice(&cells.to_le_bytes());
    save.extend_from_slice(&[0x89, 0x08, 0xa1]);
    save.extend_from_slice(&(cells + 4).to_le_bytes());
    let stands: Vec<Stand> = vec![
        (0x0079_a4b0, vec![0], vec![], ret_cell(cells + 8)),
        (0x0089_8010, vec![], vec![], vec![]),
        (0x0078_5410, vec![0, 1], vec![], save),
        (0x0079_add0, vec![0], vec![], seq.clone()),
        (0x0079_ae50, vec![0], vec![], seq.clone()),
        (0x0079_adf0, vec![0], vec![], seq.clone()),
        (0x0079_ae70, vec![0], vec![], seq.clone()),
        (0x0078_bf00, vec![0, 1], vec![], seq.clone()),
        // the gauge maximum: one word a case (a zero second maximum faults)
        (0x0079_5590, vec![0], vec![], ret_cell(cells + 12)),
        (0x0079_5550, vec![0], vec![], seq),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let work = b.stack_range().start + 0x1000;
    let command = alloc(a, b, 0x40);
    let window = alloc(a, b, 0x10);
    let few = [0u32, 1, 5, 7, 100, 9999, 0xffff_ffce, 3, 2];
    let party = a.rebase(t::STATUS_PARTY);
    let rows = a.rebase(t::STATUS_ROWS);
    let pick = |rng: &mut Xorshift| few[(rng.next() % 9) as usize];
    for k in 0..400u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        for j in 0..0x100 {
            let v = if j % 7 == 6 { rng.next() } else { pick(rng) };
            put(a, b, table + 4 * j, v);
        }
        let chr = |rng: &mut Xorshift| [0u16, 1, 2, 7, 0xff][(rng.next() % 5) as usize];
        for j in 0..3 {
            let c = chr(rng);
            put(a, b, party + 2 * j, c);
        }
        for j in 0..3 * 0x90 / 4 {
            let v = if j % 3 == 0 {
                pick(rng)
            } else {
                rng.next() % 40
            };
            put(a, b, rows + 4 * j, v);
        }
        // the frame counts: some running, some done
        for j in 0..3 {
            for o in [0x18u32, 0x30] {
                put(
                    a,
                    b,
                    rows + 0x90 * j + o,
                    (rng.next() % 4) as u8
                        * if rng.next().is_multiple_of(3) {
                            0x40
                        } else {
                            1
                        },
                );
            }
        }
        // the party in the save window: often the same
        for j in 0..3 {
            let c = if rng.next().is_multiple_of(3) {
                chr(rng)
            } else {
                Mem::read::<u16>(a, party + 2 * j)
            };
            put(a, b, window + 2 * j, c);
        }
        for j in 3..6 {
            put(
                a,
                b,
                window + 2 * j,
                if rng.next().is_multiple_of(2) {
                    0xffu16
                } else {
                    1
                },
            );
        }
        put(a, b, cells, rng.next() % 7);
        put(a, b, cells + 4, window);
        put(a, b, cells + 8, command);
        put(
            a,
            b,
            cells + 12,
            [0xffff_ffffu32, 0, 7, 100, 1][(rng.next() % 5) as usize],
        );
        put(a, b, command + 0x22, (rng.next() & 0xff) as u8);
        put(a, b, a.rebase(t::STATUS_MODE), (rng.next() % 5) as u8);
        set(
            a,
            b,
            t::ACTION_PREV_VALUE,
            if rng.next().is_multiple_of(4) {
                0xff
            } else {
                (rng.next() % 0x200) as u16
            },
        );
        set(a, b, t::ACTION_DEPTH, (rng.next() % 3) as i8);
        for j in 0..3 {
            put(
                a,
                b,
                a.rebase(0x0133_c95c) + 0xf0 * j,
                (rng.next() & 0xffff) as u16,
            );
        }
        put(
            a,
            b,
            a.rebase(0x025d_0aa4),
            (rng.next() & 0xffff) as u16 >> (rng.next() % 8),
        );
        eax(a, 0x0089_20d0, &[]);
        t::ctrl_status_win(b, work, call);
        same_memory(&format!("ctrl status {k}"), a, b);
    }
}

#[test]
fn round38_ctrl_status_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round38_ctrl_status(&mut a, &mut b, &mut Xorshift(0x4038_0031));
}

#[test]
fn round38_ctrl_status_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round38_ctrl_status(&mut a, &mut b, &mut Xorshift(0x4038_0032));
}

/// The limit windows' callees as logging stand-ins: the scene calls
/// (return words of the table), the timer, the click, the limit set,
/// `TkGetControlerTrigHist`, `TkGetControlerAnalogHist`,
/// `setLuluODKeyboardInput` (storing two table words), the control mode,
/// `inputClearAllInputInfo` and `graphicGetTime` (an `f32` of `cells`).
fn round38_limit_stands(
    a: &mut Oracle,
    b: &mut Oracle,
    cursor: u32,
    counter: u32,
    table: u32,
    cells: u32,
) {
    let seq = round38_seq(counter, table);
    // mov eax,[esp+4]; mov ecx,[cells+4]; mov [eax],ecx; mov eax,[esp+8]; mov ecx,[cells+8]; mov [eax],ecx
    let mut keys = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x0d];
    keys.extend_from_slice(&(cells + 4).to_le_bytes());
    keys.extend_from_slice(&[0x89, 0x08, 0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d]);
    keys.extend_from_slice(&(cells + 8).to_le_bytes());
    keys.extend_from_slice(&[0x89, 0x08]);
    // fld dword [cells + 12 + 4 * (count & 1)]: two times in turn
    let mut time = vec![0xa1];
    time.extend_from_slice(&counter.to_le_bytes());
    time.extend_from_slice(&[0x83, 0xe0, 0x01, 0xd9, 0x04, 0x85]);
    time.extend_from_slice(&(cells + 12).to_le_bytes());
    time.extend_from_slice(&[0xff, 0x05]);
    time.extend_from_slice(&counter.to_le_bytes());
    let stands: Vec<Stand> = vec![
        (0x008a_f1b0, vec![0, 1], vec![], seq.clone()),
        (0x008a_e070, vec![0, 1], vec![], seq.clone()),
        (0x008a_f0d0, vec![0, 1], vec![], seq.clone()),
        (0x008a_f4f0, vec![0, 1], vec![], seq.clone()),
        (0x0089_ab20, vec![], vec![], vec![]),
        (0x0089_aba0, vec![], vec![], vec![]),
        (0x0089_ab50, vec![], vec![], vec![]),
        (0x0088_6ba0, vec![0], vec![], vec![]),
        (0x007b_0470, vec![0, 1, 2, 3], vec![], vec![]),
        (0x0089_7f00, vec![], vec![], vec![]),
        (0x0088_8e80, vec![0, 1, 2], vec![], seq.clone()),
        (0x0088_8c20, vec![0, 1, 2, 3], vec![], seq.clone()),
        (0x0064_5eb0, vec![0, 1], vec![], keys),
        (0x0063_8e30, vec![], vec![], seq),
        (0x0063_0c40, vec![], vec![], vec![]),
        (0x0064_1410, vec![], vec![], time),
    ];
    round38_stands(a, b, cursor, 0, &stands);
}

/// Round 38 group C: the Auron, Tidus and Lulu limit windows' state
/// machines, every state and the out-of-range ones, over
/// [`round38_limit_stands`].
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn round38_limits(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let cells = alloc(a, b, 0x20);
    round38_limit_stands(a, b, cursor, counter, table, cells);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let buttons = alloc(a, b, 0x20);
    let r = |a: &Oracle, x: u32| a.rebase(x);
    let time = |rng: &mut Xorshift| match rng.next() % 4 {
        0 => 0.0f32,
        1 => -1.5,
        _ => f32::from(u16::try_from(rng.next() % 4000).unwrap()) / 16.0,
    };
    for k in 0..2400u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        // scene results 0..2, stick bytes around the thresholds, keys
        for j in 0..0x100 {
            let v = match rng.next() % 6 {
                0 => rng.next(),
                1 | 2 => rng.next() % 3,
                _ => {
                    [0x00u32, 0x5f, 0x60, 0x80, 0x9f, 0xa0, 0xff, 0x120][(rng.next() % 8) as usize]
                }
            };
            put(a, b, table + 4 * j, v);
        }
        put(a, b, cells + 4, rng.next() % 2);
        put(a, b, cells + 8, rng.next() % 2);
        put(a, b, cells + 12, time(rng).to_bits());
        put(a, b, cells + 16, time(rng).to_bits());
        put(a, b, r(a, 0x0133_f790), time(rng).to_bits());
        put(a, b, r(a, 0x0133_f78c), time(rng).to_bits());
        let state = (rng.next() % 12) as i16;
        // Auron: the button and the combination
        put(a, b, r(a, t::AURON_STATE), state);
        put(
            a,
            b,
            r(a, 0x0133_d6f2),
            [0u8, 1, 2, 0x80, 0xff][(rng.next() % 5) as usize],
        );
        put(
            a,
            b,
            r(a, 0x025d_09d4),
            [0u16, 1, 2, 4][(rng.next() % 4) as usize],
        );
        put(a, b, r(a, 0x0133_d6fc), (rng.next() % 4) as u16);
        put(a, b, r(a, 0x0133_d700), rng.next() % 5);
        put(a, b, r(a, 0x0133_d704), buttons);
        for j in 0..8 {
            put(
                a,
                b,
                buttons + 2 * j,
                [1u16, 2, 4][(rng.next() % 3) as usize],
            );
        }
        // Tidus: the bar
        put(a, b, r(a, t::TIDUS_STATE), (rng.next() % 12) as i16);
        put(
            a,
            b,
            r(a, 0x0133_d6f3),
            [0u8, 1, 2, 0x80, 0xff][(rng.next() % 5) as usize],
        );
        let start = (rng.next() % 50) as i16 - 10;
        let length = (rng.next() % 200) as i16;
        let good = (rng.next() % 200) as i16 + start;
        let step = (rng.next() % 40) as i16 - 20;
        for (o, v) in [
            (0u32, start),
            (4, length),
            (6, good),
            (8, good + (rng.next() % 30) as i16),
            (
                0xc,
                match rng.next() % 6 {
                    0 => good,
                    // one step from either end
                    1 => start + length - step,
                    2 => start - step,
                    _ => start + (rng.next() % 220) as i16 - 10,
                },
            ),
            (0xe, step),
        ] {
            put(a, b, r(a, 0x0133_d736) + o, v);
        }
        // Lulu: the directions, the level and the gauge
        // Lulu: half the time in state 4, the stick or keyboard turns
        let lulu_state = if rng.next().is_multiple_of(2) {
            4
        } else {
            (rng.next() % 11) as i16
        };
        put(a, b, r(a, t::LULU_STATE), lulu_state);
        put(
            a,
            b,
            r(a, 0x0133_c924),
            if rng.next().is_multiple_of(3) {
                0
            } else {
                time(rng).to_bits()
            },
        );
        put(a, b, r(a, 0x0133_c928), rng.next() % 2);
        put(a, b, r(a, 0x0133_c92c), rng.next() % 2);
        put(a, b, r(a, 0x0133_c930), rng.next() % 2);
        put(a, b, r(a, 0x00c5_8ef0), rng.next() % 3);
        let presses = (rng.next() % 40) as u8;
        put(a, b, r(a, 0x0133_d71e), presses);
        // the level: now and then full (16, or past it)
        let level = if rng.next().is_multiple_of(3) {
            16 + rng.next() % 2
        } else {
            rng.next() % 16
        };
        put(a, b, r(a, 0x0133_d71f), level as u8);
        put(a, b, r(a, 0x0133_d71c), (rng.next() % 300) as i16);
        put(a, b, r(a, 0x0133_d70e), (rng.next() % 600) as i16 - 100);
        // a circle (either way) now and then, else random directions 4..=7
        let count = [0u8, 1, 3, 4, 5, 6, 5, 6][(rng.next() % 8) as usize];
        let first = (rng.next() % 4) as u8 + 4;
        let way = rng.next() % 3;
        for j in 0..16u8 {
            let d = match way {
                0 => 4 + (first - 4 + j) % 4,
                1 => 4 + (first - 4 + 4 - j % 4) % 4,
                _ => (rng.next() % 4) as u8 + 4,
            };
            put(
                a,
                b,
                r(a, 0x0133_d721) + u32::from(j),
                if j == 4 && way < 2 { first } else { d },
            );
        }
        put(a, b, r(a, 0x0133_d731), count);
        // the presses each level needs: often just reached
        for j in 0..16 {
            let need = if rng.next().is_multiple_of(2) {
                presses / 2 + (rng.next() % 3) as u8
            } else {
                presses + (rng.next() % 3) as u8
            };
            put(a, b, a.rebase(0x0133_f798) + j, need);
        }
        // now and then exactly the keyboard's reset time since the last reset
        if rng.next().is_multiple_of(8) {
            let tiny = 2f32.powi(-20);
            let now = f32::from_bits(0x3f00_068e) + tiny;
            put(a, b, cells + 12, now.to_bits());
            put(a, b, r(a, 0x0133_c924), tiny.to_bits());
        }
        match k % 3 {
            0 => {
                eax(a, 0x0089_0f70, &[]);
                t::ctrl_auron_limit_window(b, call);
            }
            1 => {
                eax(a, 0x0089_2320, &[]);
                t::ctrl_tidus_limit_window(b, call);
            }
            _ => {
                eax(a, 0x0089_1b80, &[]);
                t::ctrl_lulu_limit_window(b, call, fcall);
            }
        }
        same_memory(&format!("limit window {k}"), a, b);
    }
}

#[test]
fn round38_limits_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round38_limits(&mut a, &mut b, &mut Xorshift(0x4038_0041));
}

#[test]
fn round38_limits_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round38_limits(&mut a, &mut b, &mut Xorshift(0x4038_0042));
}

/// Round 38 group D: `TOBtlDrawStatusWin` over logging stand-ins (the
/// remaps the originals), every mode (and two it ignores), the party
/// slots, marks, current command, language and status cycles random.
#[allow(clippy::too_many_lines)]
fn round38_status_window(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x10000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x20);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x008f_dee0, all(4), vec![], vec![]),
        (0x0090_3bb0, all(9), vec![], vec![]),
        (0x0090_5930, all(6), vec![], vec![]),
        (0x0090_5890, all(6), vec![], vec![]),
        (0x0090_1660, all(6), vec![], vec![]),
        (0x0089_60e0, all(5), vec![], vec![]),
        (0x0090_5ab0, all(6), vec![], vec![]),
        (0x008a_c2f0, vec![], vec![], ret_cell(cells)),
        (0x0079_af10, all(1), vec![], ret_cell(cells + 4)),
        (0x0079_adf0, all(1), vec![], ret_cell(cells + 8)),
        (0x0079_ae70, all(1), vec![], ret_cell(cells + 12)),
        (0x0078_fcf0, all(3), vec![], ret_cell(cells + 16)),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        #[allow(clippy::cast_possible_truncation)] // an f32 in ST0
        let r = m
            .call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32;
        r
    };
    let party = a.rebase(t::STATUS_PARTY);
    let rows = a.rebase(t::STATUS_ROWS);
    let marks = a.rebase(0x0133_d178);
    for k in 0..600u32 {
        put(a, b, cursor, log);
        put(
            a,
            b,
            cells,
            if rng.next().is_multiple_of(2) {
                0xb
            } else {
                rng.next() % 12
            },
        );
        for j in 1..8 {
            put(a, b, cells + 4 * j, rng.next());
        }
        for j in 0..3 {
            let c = [0u16, 1, 2, 7, 0x1f, 0x20, 0xff][(rng.next() % 7) as usize];
            put(a, b, party + 2 * j, c);
        }
        for j in 0..3 * 0x90 / 4 {
            put(a, b, rows + 4 * j, rng.next());
        }
        // the status cycles: counts 0..=4, timers around 0, shown entries
        for j in 0..3 {
            let row = rows + 0x90 * j;
            put(a, b, row + 0x40, (rng.next() % 5) as u16);
            put(
                a,
                b,
                row + 0x3c,
                (rng.next() % 4) as u16
                    * if rng.next().is_multiple_of(3) {
                        0xffff
                    } else {
                        1
                    },
            );
            put(a, b, row + 0x3e, (rng.next() % 5) as u16);
        }
        put(a, b, marks, rng.next());
        put(
            a,
            b,
            a.rebase(t::STATUS_MODE),
            (rng.next() % 6) as u8
                | if rng.next().is_multiple_of(8) {
                    0x80
                } else {
                    0
                },
        );
        let current = if rng.next().is_multiple_of(2) {
            Mem::read::<u16>(a, party + 2 * (rng.next() % 3))
        } else {
            (rng.next() & 0xffff) as u16
        };
        set(a, b, t::ACTION_PREV_VALUE, current);
        eax(a, 0x0089_6450, &[]);
        t::StatusWindow {
            mem: b,
            remap,
            call,
        }
        .draw();
        same_memory(&format!("status window {k}"), a, b);
    }
}

#[test]
fn round38_status_window_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round38_status_window(&mut a, &mut b, &mut Xorshift(0x4038_0051));
}

#[test]
fn round38_status_window_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round38_status_window(&mut a, &mut b, &mut Xorshift(0x4038_0052));
}

/// Round 38 group E: `TOBtlDrawWeaponAbilityWindow` over logging
/// stand-ins (the remaps the originals, `ToGetBtlEasyFontWidth` storing
/// widths around the window's minimum); 0..=6 slots, empty ones among them.
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]
fn round38_weapon_abilities(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x10000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x200);
    let cells = alloc(a, b, 0x20);
    let weapon = alloc(a, b, 0x40);
    let records = alloc(a, b, 0x400);
    // MsGetSaveWeapon(i, &_) / MsGetRomAbility(id, &base): base = cells[1],
    // the weapon / a record by id
    let mut out_base = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d];
    out_base.extend_from_slice(&(cells + 4).to_le_bytes());
    out_base.extend_from_slice(&[0x89, 0x08]);
    let mut save = out_base.clone();
    save.push(0xa1);
    save.extend_from_slice(&cells.to_le_bytes());
    // eax = records + (id & 7) * 0x80
    let mut rom = out_base;
    rom.extend_from_slice(&[
        0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x07, 0xc1, 0xe0, 0x07, 0x05,
    ]);
    rom.extend_from_slice(&records.to_le_bytes());
    // the width: the next table word into [esp+8]
    let mut width = vec![0x8b, 0x0d];
    width.extend_from_slice(&counter.to_le_bytes());
    width.extend_from_slice(&[0x8b, 0x04, 0x8d]);
    width.extend_from_slice(&table.to_le_bytes());
    width.extend_from_slice(&[0xff, 0x05]);
    width.extend_from_slice(&counter.to_le_bytes());
    width.extend_from_slice(&[0x8b, 0x4c, 0x24, 0x08, 0x89, 0x01]);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x007a_bbd0, all(1), vec![], save),
        (0x0079_8eb0, all(1), vec![], vec![]),
        (0x008f_9230, all(4), vec![], vec![]),
        (0x0079_0970, all(1), vec![], rom),
        (0x0090_5290, vec![0, 2, 3, 4], vec![], width),
        (0x008f_41b0, all(5), vec![], vec![]),
        (0x0078_fcf0, all(3), vec![], ret_cell(cells + 8)),
        (0x0090_5ab0, all(6), vec![], vec![]),
        (0x008e_6af0, all(9), vec![], vec![]),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let remap = |m: &mut Oracle, y: bool, v: f32| {
        let at = m.rebase(if y { 0x0064_4820 } else { 0x0064_47e0 });
        #[allow(clippy::cast_possible_truncation)] // an f32 in ST0
        let r = m
            .call(at, Conv::Cdecl, &[v.to_bits()])
            .unwrap()
            .st0
            .unwrap() as f32;
        r
    };
    let work = b.stack_range().start + 0x1000;
    let least = remap(a, false, 300.0);
    for k in 0..400u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        // widths around the minimum, a few huge or negative
        for j in 0..0x80 {
            let v = match rng.next() % 6 {
                0 => least,
                1 => least + 1.0,
                2 => (rng.next() % 0x2_0000) as f32,
                3 => -((rng.next() % 100) as f32),
                _ => (rng.next() % 0x200) as f32 + 0.5,
            };
            put(a, b, table + 4 * j, v.to_bits());
        }
        put(a, b, cells, weapon);
        put(a, b, cells + 4, rng.next() % 0x1000);
        put(a, b, cells + 8, rng.next());
        for j in 0..0x100 {
            put(a, b, records + 4 * j, rng.next());
        }
        for j in 0..0x10 {
            put(a, b, weapon + 4 * j, rng.next());
        }
        let n = [0u8, 1, 2, 3, 4, 6, 9][(rng.next() % 7) as usize];
        put(a, b, weapon + 0xb, n);
        for j in 0..9 {
            let id = if rng.next().is_multiple_of(4) {
                0xff
            } else {
                (rng.next() % 0x80) as u16
            };
            put(a, b, weapon + 0xe + 2 * j, id);
        }
        let i = rng.next() % 4;
        let x = (rng.next() % 1000) as f32 / 4.0;
        let y = (rng.next() % 1000) as f32 / 4.0;
        eax(a, 0x008f_34c0, &[i, x.to_bits(), y.to_bits()]);
        t::WeaponAbilityWindow {
            mem: b,
            remap,
            call,
            work,
        }
        .draw(i, x, y);
        same_memory(&format!("weapon abilities {k}"), a, b);
    }
}

#[test]
fn round38_weapon_abilities_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round38_weapon_abilities(&mut a, &mut b, &mut Xorshift(0x4038_0061));
}

#[test]
fn round38_weapon_abilities_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round38_weapon_abilities(&mut a, &mut b, &mut Xorshift(0x4038_0062));
}

/// Round 38 group F: `Kernel_LoadTables` over logging stand-ins (the two
/// allocations from a table of addresses, the read size from a cell, the
/// CD read behind `GetCD`), every kind and two it ignores, tables present
/// or not; and `MagicSqrt_009dbe10`.
#[allow(clippy::too_many_lines)]
fn round38_kernel_tables(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let blocks = alloc(a, b, 0x10);
    let cells = alloc(a, b, 0x10);
    let buffer = alloc(a, b, 0x26000);
    let table = alloc(a, b, 0x26000);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x0068_7020, all(1), vec![], round38_seq(counter, blocks)),
        (0x0068_7010, all(1), vec![], vec![]),
        (0x0083_c4a0, all(1), vec![], vec![]),
        (0x0079_9f00, all(2), vec![], vec![]),
        (0x0078_2df0, all(2), vec![], ret_cell(cells)),
        (0x0076_d0d0, vec![], vec![], ret_cell(cells + 4)),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    // the CD read: a logging stand-in behind a pointer
    let read = stand_in8(
        a,
        b,
        0x80 * 6,
        &pick_logger(cursor, 0xcd, &all(4), &[], &[]),
    );
    put(a, b, cells + 8, read);
    let call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    for k in 0..300u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        put(a, b, blocks, buffer);
        put(a, b, blocks + 4, table);
        let size = [0u32, 1, 0x100, 0x2_5800, 0x1_0010, 0x7ff, 0x3][(rng.next() % 7) as usize];
        put(a, b, cells, size);
        put(a, b, cells + 4, cells + 8);
        for j in 0..0x40 {
            put(a, b, buffer + 4 * j, rng.next());
            put(a, b, table + 4 * j, rng.next());
        }
        for j in (0x7f0..0x810)
            .chain(0x1_0000..0x1_0020)
            .chain(0x2_57f0..0x2_5800)
        {
            put(a, b, buffer + j, (rng.next() & 0xff) as u8);
        }
        for t in &files::bridge::KERNEL_TABLES {
            put(
                a,
                b,
                a.rebase(t.pointer),
                if rng.next().is_multiple_of(2) {
                    0
                } else {
                    table
                },
            );
        }
        let kind = [0u32, 1, 2, 3, 4, 5, 6, u32::MAX][(rng.next() % 8) as usize];
        assert_eq!(
            eax(a, 0x0078_1d40, &[kind]),
            files::bridge::kernel_load_tables(b, kind, call),
            "kernel tables {k}"
        );
        same_memory(&format!("kernel tables {k}"), a, b);
    }
    // the square root
    let src = alloc(a, b, 4);
    let dst = alloc(a, b, 4);
    for k in 0..300u32 {
        let v = loop {
            let f = rng.float();
            if !(f.is_nan() && f.to_bits() & 0x0040_0000 == 0) {
                break f.to_bits();
            }
        };
        put(a, b, src, v);
        put(a, b, dst, rng.next());
        assert_eq!(
            eax(a, 0x009d_be10, &[src, dst]),
            math::bridge::magic_sqrt(b, src, dst),
            "sqrt {k}"
        );
        same_memory(&format!("sqrt {k}"), a, b);
    }
}

#[test]
fn round38_kernel_tables_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round38_kernel_tables(&mut a, &mut b, &mut Xorshift(0x4038_0071));
}

#[test]
fn round38_kernel_tables_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round38_kernel_tables(&mut a, &mut b, &mut Xorshift(0x4038_0072));
}

/// Round 39 group A: `TOBtlCtrlInfoWin` and `TOBtlCtrlInfoWinMes` over
/// logging stand-ins (the checks return words of a table; the two set-ups
/// move the target cursor to the next table word, so the re-reads after
/// them count), the target near the party slots 0x14..=0x1b, and the
/// message lines in every state around the box's edges.
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
fn round39_info_win(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let cells = alloc(a, b, 0x10);
    let seq = round38_seq(counter, table);
    let target = a.rebase(t::TARGET_CHR);
    // the set-ups: the next table word to the target cursor (mov [target], eax)
    let mut moves = seq.clone();
    moves.push(0xa3);
    moves.extend_from_slice(&target.to_le_bytes());
    let mut time = vec![0xd9, 0x05];
    time.extend_from_slice(&cells.to_le_bytes());
    let stands: Vec<Stand> = vec![
        (0x0078_f9e0, vec![], vec![], seq.clone()),
        (0x0078_f990, vec![0], vec![], seq.clone()),
        (0x0078_f970, vec![0], vec![], seq),
        (0x0089_9fd0, vec![0], vec![], moves.clone()),
        (0x0089_9f20, vec![0], vec![], moves),
        (0x0064_1410, vec![], vec![], time),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let chr = |rng: &mut Xorshift| -> u32 {
        let c = 0x12 + rng.next() % 12;
        if rng.next().is_multiple_of(5) {
            c | (rng.next() << 16)
        } else {
            c
        }
    };
    // no signalling NaNs (the emulator keeps them where hardware quiets)
    let float = |rng: &mut Xorshift| {
        let f = rng.float();
        if ffx_bridge::x87::is_snan(f.to_bits()) {
            1.5
        } else {
            f
        }
    };
    let lines = a.rebase(t::INFO_LINES);
    let bx = a.rebase(0x0133_f694);
    for k in 0..3000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        for j in 0..0x100 {
            let v = match rng.next() % 4 {
                0 => u32::MAX,
                1 => 1,
                2 => chr(rng),
                _ => rng.next() % 3,
            };
            put(a, b, table + 4 * j, v);
        }
        // a time (never a NaN)
        put(a, b, cells, ((rng.next() % 100_000) as f32 / 7.0).to_bits());
        put(a, b, target, chr(rng));
        for at in [t::INFO_PARAM_CHR, t::INFO_MES_CHR] {
            let c = if rng.next().is_multiple_of(2) {
                Mem::read::<u16>(a, target)
            } else {
                chr(rng) as u16
            };
            put(a, b, a.rebase(at), c);
        }
        for at in [t::INFO_SHOWN, t::INFO_MES_SHOWN] {
            put(a, b, a.rebase(at), (rng.next() % 3) as u8);
        }
        let depth = (rng.next() % 3) as i8;
        set(a, b, t::ACTION_DEPTH, depth);
        for j in 0..3 {
            let open = [0u8, 1, 1, 2][(rng.next() % 4) as usize];
            put(a, b, a.rebase(0x0133_c952) + 0xf0 * j, open);
        }
        // the box and the lines around its edges
        let left = (rng.next() % 200) as i16 - 50;
        let right = left + 100 + (rng.next() % 600) as i16;
        put(a, b, bx, left);
        put(a, b, bx + 8, right);
        for i in 0..4 {
            let line = lines + 0x1c * i;
            for j in 0..7 {
                put(a, b, line + 4 * j, rng.next());
            }
            // now and then no line in use at all
            let state = if k % 13 == 1 {
                0
            } else {
                [0u16, 2, 2, 3, 3, 8, 1, 0x102][(rng.next() % 8) as usize]
            };
            put(a, b, line, state);
            let width = (rng.next() % 400) as i16 - 20;
            put(a, b, line + 0x14, width);
            let edge = if rng.next().is_multiple_of(2) {
                f32::from(left) - f32::from(width)
            } else {
                f32::from(right) - 80.0 - f32::from(width)
            };
            let x = match rng.next() % 6 {
                0 => f32::NAN,
                1 => edge,
                2 => edge + 1.2,
                _ => edge + (rng.next() % 64) as f32 * 0.37 - 6.0,
            };
            put(a, b, line + 0xc, x);
            let y = if rng.next().is_multiple_of(9) {
                f32::NAN
            } else {
                float(rng)
            };
            put(a, b, line + 0x10, y);
        }
        if k % 2 == 0 {
            eax(a, 0x0089_1480, &[]);
            t::ctrl_info_win(b, call, fcall);
        } else {
            eax(a, 0x0089_1580, &[]);
            t::ctrl_info_win_mes(b, call);
        }
        same_memory(&format!("info win {k}"), a, b);
    }
}

#[test]
fn round39_info_win_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round39_info_win(&mut a, &mut b, &mut Xorshift(0x4039_0011));
}

#[test]
fn round39_info_win_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round39_info_win(&mut a, &mut b, &mut Xorshift(0x4039_0012));
}

/// Round 39 group B: `TOMenuCtrlYesNoWindow` and `TOMenuDrawYesNoWindow`
/// over logging stand-ins: the pad words from a table, the measures
/// storing test floats, the remaps the argument times a test factor (so
/// every remap's argument and order count), every state and icon.
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
fn round39_yes_no(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use window::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let cells = alloc(a, b, 0x20);
    let seq = round38_seq(counter, table);
    let mut measure = store_through(2, cells);
    measure.extend(store_through(3, cells + 4));
    // fld dword [esp+4]; fmul dword [factor]
    let remap = |factor: u32| {
        let mut v = vec![0xd9, 0x44, 0x24, 0x04, 0xd8, 0x0d];
        v.extend_from_slice(&factor.to_le_bytes());
        v
    };
    let stands: Vec<Stand> = vec![
        (0x008b_e410, vec![], vec![], seq.clone()),
        (0x008b_e470, vec![], vec![], seq.clone()),
        (0x008b_e4b0, vec![], vec![], seq),
        (0x0088_6e80, vec![0], vec![], vec![]),
        (0x008b_6cb0, vec![0, 1, 4, 5, 6, 7], vec![], measure),
        (
            0x0090_5290,
            vec![0, 2, 3, 4],
            vec![],
            store_through(1, cells + 8),
        ),
        (0x008f_5f70, vec![0, 1, 2, 3, 4], vec![], vec![]),
        (0x0090_3bb0, vec![0, 1, 2, 3, 4, 5, 6, 7, 8], vec![], vec![]),
        (0x008a_f0d0, vec![0, 1], vec![], vec![]),
        (0x008a_f060, vec![0, 1, 2, 3], vec![], vec![]),
        (0x0090_16b0, vec![0, 1, 2, 3, 4, 5, 6], vec![], vec![]),
        (0x008c_0670, vec![0, 1, 2], vec![], vec![]),
        (0x0064_47e0, vec![0], vec![], remap(cells + 12)),
        (0x0064_4820, vec![0], vec![], remap(cells + 16)),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let work = b.stack_range().start + 0x1000;
    let win = alloc(a, b, 0x100);
    let size = |rng: &mut Xorshift| match rng.next() % 8 {
        0 => f32::NAN,
        1 => 0.0,
        2 => -3.5,
        _ => (rng.next() % 600) as f32 * 0.73,
    };
    for k in 0..3000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        for j in 0..0x100 {
            let v = [
                0u32, 0x20, 0x40, 0x60, 0x1000, 0x4000, 0x1_0020, 0x8, 0xffff, 0x1_0000,
            ][(rng.next() % 10) as usize];
            let v = if rng.next().is_multiple_of(8) {
                rng.next()
            } else {
                v
            };
            put(a, b, table + 4 * j, v);
        }
        put(a, b, cells, size(rng));
        put(a, b, cells + 4, size(rng));
        put(a, b, cells + 8, size(rng));
        for at in [cells + 12, cells + 16] {
            let f = [1.0f32, 0.5, 1.25, 2.0, 3.0, 0.1][(rng.next() % 6) as usize];
            put(a, b, at, f);
        }
        for j in 0..0x40 {
            put(a, b, win + 4 * j, rng.next());
        }
        put(a, b, win + 0x28, rng.next() % 10);
        put(a, b, win + 0x37, (rng.next() % 2) as u8);
        put(a, b, win + 0x48, (rng.next() % 3) as u16);
        put(
            a,
            b,
            win + 0x58,
            if rng.next().is_multiple_of(3) {
                0
            } else {
                rng.next()
            },
        );
        let offset = match rng.next() % 4 {
            0 => rng.next(),
            _ => (rng.next() % 200).wrapping_sub(100),
        };
        put(a, b, win + 0x60, offset);
        put(a, b, win + 0x90, rng.next() % 3);
        put(a, b, win + 0x94, rng.next() % 4);
        if k % 7 == 1 {
            // an icon window exactly Y2(60) plus the margins high
            let fy: f32 = Mem::read(a, cells + 16);
            put(a, b, cells + 4, 60.0 * fy);
            put(a, b, win + 0x58, 0u32);
            put(a, b, win + 0x94, 1 + rng.next() % 2);
        }
        if k % 2 == 0 {
            eax(a, 0x008e_2e20, &[win]);
            t::menu_ctrl_yes_no_window(b, win, call);
        } else {
            eax(a, 0x008e_2f80, &[win]);
            t::menu_draw_yes_no_window(b, win, work, call, fcall);
        }
        same_memory(&format!("yes/no {k}"), a, b);
    }
}

#[test]
fn round39_yes_no_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round39_yes_no(&mut a, &mut b, &mut Xorshift(0x4039_0021));
}

#[test]
fn round39_yes_no_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round39_yes_no(&mut a, &mut b, &mut Xorshift(0x4039_0022));
}

/// Round 39 group C: `TOGetFontInfoInterMode` and `TOBtlSetTargetHelpMes`
/// over logging stand-ins (the language, English and icon getters return
/// table words; the text and name getters pointers to test strings of
/// many lengths, past the original's 16-byte buffers too), glyph codes
/// around every boundary.
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn round39_font_help(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let counter2 = alloc(a, b, 4);
    let table2 = alloc(a, b, 0x400);
    let cells = alloc(a, b, 0x10);
    let widths = alloc(a, b, 0x1000) + 0x40;
    let strings = alloc(a, b, 0x400);
    let text = alloc(a, b, 0x10);
    let out = alloc(a, b, 0x40);
    let seq = round38_seq(counter, table);
    let seq2 = round38_seq(counter2, table2);
    let stands: Vec<Stand> = vec![
        (0x008a_c2f0, vec![], vec![], seq.clone()),
        (0x0078_51d0, vec![], vec![], seq.clone()),
        (0x008a_c3d0, vec![0], vec![], ret_cell(cells)),
        (0x0078_fcf0, vec![0, 1, 2], vec![], seq2.clone()),
        (0x0079_af10, vec![0], vec![], seq2),
        (0x0079_ae30, vec![0], vec![], seq.clone()),
        (0x0079_ae10, vec![0], vec![], seq),
        (0x008b_9650, vec![0, 1, 2, 3, 4, 5], vec![], vec![]),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    put(a, b, cells, widths);
    for j in 0..0xfc0 {
        put(a, b, widths - 0x40 + j, (rng.next() & 0xff) as u8);
    }
    // strings of 0, 1, 3, 15, 16, 20, 40 and (the last, for names only:
    // the help text then runs past its 0x80 bytes) 100 bytes
    let lens = [0u32, 1, 3, 15, 16, 20, 40, 100];
    for (k, &n) in (0u32..).zip(&lens) {
        for j in 0..n {
            put(
                a,
                b,
                strings + 0x40 * k + j,
                (0x20 + rng.next() % 0x5f) as u8,
            );
        }
        put(a, b, strings + 0x40 * k + n, 0u8);
    }
    let codes = [
        0u8, 4, 5, 6, 0x2b, 0x2c, 0x2d, 0x2f, 0x30, 0x31, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x7f, 0xff,
        // after a 0x2c lead: glyphs 0xea, 0x103 and 0x104 (the slant's ends)
        0x4a, 0x63, 0x64,
    ];
    for k in 0..4000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        put(a, b, counter2, 0u32);
        for j in 0..0x100 {
            let v = match rng.next() % 5 {
                0 => [0u32, 1, 9, 10, 2][(rng.next() % 5) as usize],
                1 => [0x14u32, 0x15, 0x16, 0x13, 0][(rng.next() % 5) as usize],
                2 => rng.next() % 4,
                3 => rng.next(),
                _ => 0,
            };
            put(a, b, table + 4 * j, v);
            // the texts fill the original's stack buffers 0x1047 (0x30
            // bytes to the frame check), 0x1048 (0x10; at least two bytes:
            // its second is raised, past a NUL the original reads stale
            // stack) and 0x1049 (0x20)
            let pick = match j {
                1 => 2 + rng.next() % 2,
                2 => rng.next() % 6,
                0 => rng.next() % 7,
                _ => rng.next() % 8,
            };
            put(a, b, table2 + 4 * j, strings + 0x40 * pick);
        }
        for j in 0..0x40 {
            put(a, b, out + j, (rng.next() & 0xff) as u8);
        }
        if k % 2 == 0 {
            // the language and English words
            for j in 0..0x100 {
                put(
                    a,
                    b,
                    table + 4 * j,
                    [0u32, 9, 10, 1, 0, 9, 10, 2][(rng.next() % 8) as usize],
                );
            }
            for j in 0..4 {
                let c = if rng.next().is_multiple_of(4) {
                    (rng.next() & 0xff) as u8
                } else {
                    codes[(rng.next() % codes.len() as u32) as usize]
                };
                put(a, b, text + j, c);
            }
            let inter = [0u32, 1, 0x100][(rng.next() % 3) as usize];
            let want = eax(a, 0x008b_78b0, &[text, out, inter]);
            let got = tolib::bridge::to_get_font_info_inter_mode(b, text, out, inter, call);
            assert_eq!(got, want, "font info inter mode {k}");
        } else {
            for j in 0..0x80 {
                put(a, b, a.rebase(0x0133_f538) + j, (rng.next() & 0xff) as u8);
            }
            let chr = match rng.next() % 4 {
                0 => 0xff,
                1 => rng.next() % 0x14,
                2 => 0x14 + rng.next() % 0x20,
                _ => rng.next(),
            };
            eax(a, 0x0089_a8b0, &[chr]);
            btl::bridge::btl_set_target_help_mes(b, chr, call);
        }
        same_memory(&format!("font/help {k}"), a, b);
    }
}

#[test]
fn round39_font_help_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round39_font_help(&mut a, &mut b, &mut Xorshift(0x4039_0031));
}

#[test]
fn round39_font_help_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round39_font_help(&mut a, &mut b, &mut Xorshift(0x4039_0032));
}

/// Round 39 group D: `TOWaitBtlWinInputGil` and `TOInitBtlWinChange` over
/// logging stand-ins (`TOBtlSetNextAction` and `TOGetSaveWindow` return
/// table words), every pad bit, digits around both ends (0..=8: a higher
/// top digit makes the original read past its table) and amounts around
/// 0 and the most.
#[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
fn round39_gil(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge::{self as t, gil};
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let seq = round38_seq(counter, table);
    let stands: Vec<Stand> = vec![
        (0x0088_6ba0, vec![0], vec![], vec![]),
        (0x0089_a330, vec![0, 1], vec![], seq.clone()),
        (0x0089_a630, vec![], vec![], vec![]),
        (0x0089_b5a0, vec![0, 1, 2], vec![], seq),
        (0x008a_eec0, vec![0], vec![], vec![]),
        (0x008a_f0d0, vec![0, 1], vec![], vec![]),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let w = alloc(a, b, 0x60);
    let list = alloc(a, b, 0x10);
    let r = |a: &Oracle, x: u32| a.rebase(x);
    for k in 0..4000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        for j in 0..0x100 {
            let v = if rng.next().is_multiple_of(3) {
                rng.next()
            } else {
                rng.next() % 3
            };
            put(a, b, table + 4 * j, v);
        }
        for j in 0..0x18 {
            put(a, b, w + 4 * j, rng.next());
        }
        put(a, b, w + 0x20, list);
        put(a, b, list, rng.next());
        if k % 4 == 3 {
            // the change window's state, all of it cleared by the original
            for j in 0..8 {
                put(a, b, a.rebase(0x0134_0800) + 4 * j, rng.next());
            }
            let want = eax(a, 0x008a_84d0, &[w]);
            let got = t::init_btl_win_change(b, w, call);
            assert_eq!(got, want, "win change {k}");
            same_memory(&format!("win change {k}"), a, b);
            continue;
        }
        let pad = [
            0x8000u16, 0x2000, 0x1000, 0x4000, 0x20, 0x40, 0x9020, 0x5040, 0xa000, 0,
        ][(rng.next() % 10) as usize]
            | if rng.next().is_multiple_of(4) {
                (rng.next() & 0xffff) as u16
            } else {
                0
            };
        put(a, b, r(a, 0x025d_09d6), pad);
        put(
            a,
            b,
            r(a, 0x0133_d6e0),
            [0u32, 0, 0, 1, 2][(rng.next() % 5) as usize],
        );
        put(
            a,
            b,
            r(a, 0x023c_c088),
            [0xeu16, 0xe, 3, 0x10e][(rng.next() % 4) as usize],
        );
        let top = (rng.next() % 9) as i16;
        put(a, b, r(a, gil::TOP_DIGIT), top);
        let digit = [0i16, top, top + 1, -1, (rng.next() % 9) as i16][(rng.next() % 5) as usize];
        put(a, b, r(a, gil::DIGIT), digit);
        let most = [0i32, 1, 9, 10, 999, 100_000, -5][(rng.next() % 7) as usize];
        put(a, b, r(a, gil::MOST), most);
        let amount = [0i32, 1, -1, most, most - 1, most + 1, 50][(rng.next() % 7) as usize];
        put(a, b, r(a, gil::AMOUNT), amount);
        put(a, b, r(a, gil::OWNED), rng.next());
        set(
            a,
            b,
            btl::bridge::ACTION_DEPTH,
            [0i8, 1, 2, -1][(rng.next() % 4) as usize],
        );
        let want = eax(a, 0x008a_87d0, &[w]);
        let got = t::wait_btl_win_input_gil(b, w, call);
        assert_eq!(got, want, "gil {k}");
        same_memory(&format!("gil {k}"), a, b);
    }
}

#[test]
fn round39_gil_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round39_gil(&mut a, &mut b, &mut Xorshift(0x4039_0041));
}

#[test]
fn round39_gil_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round39_gil(&mut a, &mut b, &mut Xorshift(0x4039_0042));
}

/// Round 39 group E: the VU0 point transform `0x009db270` on random
/// matrices and points (no signalling NaNs) and
/// `TORelayoutMessagesForLanguage_008b09b0` over logging stand-ins for its
/// callees (English from a table), the eight message slots in every
/// state.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn round39_vu0_relayout(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let seq = round38_seq(counter, table);
    let stands: Vec<Stand> = vec![
        (0x0078_51d0, vec![], vec![], seq),
        (0x008b_8de0, vec![0], vec![], vec![]),
        (0x008a_f850, vec![0, 1, 2, 3], vec![], vec![]),
        (0x0090_6e90, vec![0], vec![], vec![]),
        (0x008b_8d10, vec![0, 1, 2, 3, 4, 5], vec![], vec![]),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let float = |rng: &mut Xorshift| {
        let f = rng.float();
        if ffx_bridge::x87::is_snan(f.to_bits()) {
            -0.25
        } else {
            f
        }
    };
    let vec_in = alloc(a, b, 0x10);
    let matrix = alloc(a, b, 0x40);
    let out = alloc(a, b, 0x10);
    let objs = alloc(a, b, 8 * 0x200);
    let slots = a.rebase(0x0186_5b3c);
    for k in 0..3000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        for j in 0..0x100 {
            put(a, b, table + 4 * j, rng.next() % 3);
        }
        if k % 2 == 0 {
            for j in 0..4 {
                put(a, b, vec_in + 4 * j, float(rng));
            }
            for j in 0..16 {
                put(a, b, matrix + 4 * j, float(rng));
            }
            if k % 6 == 2 {
                // signed zeros: only the last sums stay in memory, so x, y
                // and z all zero; a first product of -0.0 is kept, a sum
                // with +0.0 would make the lanes whose products are all
                // -0.0 come out +0.0
                for j in 0..3 {
                    let zero = if rng.next().is_multiple_of(2) {
                        -0.0f32
                    } else {
                        0.0
                    };
                    put(a, b, vec_in + 4 * j, zero);
                }
            }
            if k % 8 == 0 {
                put(a, b, a.rebase(0x00c0_a010), float(rng));
            } else {
                put(a, b, a.rebase(0x00c0_a010), 1.0f32);
            }
            let want = eax(a, 0x009d_b270, &[out, matrix, vec_in]);
            let got = math::bridge::vu0_apply_point(b, out, matrix, vec_in);
            assert_eq!(got, want, "vu0 apply {k}");
        } else {
            for i in 0..8u32 {
                let slot = objs + 0x200 * i;
                for j in 0..0x80 {
                    put(a, b, slot + 4 * j, rng.next());
                }
                put(a, b, slots + 4 * i, slot);
                put(a, b, slot + 0x10, slot + 0x40);
                put(a, b, slot + 0x14, slot + 0x80);
                let msg = slot + 0x80;
                put(
                    a,
                    b,
                    msg + 0x2c,
                    [0i16, 1, 2, 3, 4, -1][(rng.next() % 6) as usize],
                );
                put(
                    a,
                    b,
                    msg + 0x94,
                    if rng.next().is_multiple_of(4) {
                        0
                    } else {
                        rng.next()
                    },
                );
                for o in [0x40u32, 0x42, 0x44, 0x46] {
                    put(a, b, msg + o, (rng.next() % 600) as i16 - 100);
                }
                for o in [0x30u32, 0x34] {
                    put(a, b, msg + o, (rng.next() % 4000) as f32 / 3.0 - 200.0);
                }
            }
            eax(a, 0x008b_09b0, &[]);
            window::bridge::relayout_messages_for_language(b, call);
        }
        same_memory(&format!("vu0/relayout {k}"), a, b);
    }
}

#[test]
fn round39_vu0_relayout_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round39_vu0_relayout(&mut a, &mut b, &mut Xorshift(0x4039_0051));
}

#[test]
fn round39_vu0_relayout_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round39_vu0_relayout(&mut a, &mut b, &mut Xorshift(0x4039_0052));
}

/// Round 40 group A: the choice and number-entry pad handlers
/// (`TOFuncPadMessageSelect`, `TOFuncPadMessageInputValue`) over a logging
/// `SndPlayClick`, on random line records and pad words. The number entry
/// is called through a stub that sets `esi` to the window index, as
/// `TOControlMessageWindow` has it when the cursor's column 0 reads the
/// byte below the digit buffer (the top byte of the saved `esi`).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::similar_names
)]
fn round40_pad_message(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let cell = alloc(a, b, 4);
    let stands: Vec<Stand> = vec![(0x0088_6ba0, vec![0], vec![], ret_cell(cell))];
    round38_stands(a, b, cursor, 0, &stands);
    // push esi; mov esi, [esp+8]; push esi; mov eax, f; call eax;
    // add esp, 4; pop esi; ret
    let mut code = vec![0x56, 0x8b, 0x74, 0x24, 0x08, 0x56, 0xb8];
    code.extend_from_slice(&a.rebase(0x008b_62b0).to_le_bytes());
    code.extend_from_slice(&[0xff, 0xd0, 0x83, 0xc4, 0x04, 0x5e, 0xc3]);
    let input = stand_in8(a, b, 0x80, &code);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let records = alloc(a, b, 8 * 0x40);
    let slots = a.rebase(0x0186_8a90);
    let (trigger, repeat) = (a.rebase(0x025d_09d4), a.rebase(0x025d_09d6));
    let bits = |rng: &mut Xorshift, set: &[u16]| {
        set.iter()
            .filter(|_| rng.next().is_multiple_of(3))
            .fold(0u16, |v, &b| v | b)
            | if rng.next().is_multiple_of(8) {
                rng.next() as u16
            } else {
                0
            }
    };
    for k in 0..6000u32 {
        put(a, b, cursor, log);
        put(a, b, cell, rng.next());
        let w = rng.next() % 8;
        for i in 0..8u32 {
            let r = records + 0x40 * i;
            put(a, b, slots + 4 * i, r);
            for j in 0..0x10 {
                put(a, b, r + 4 * j, rng.next());
            }
            let start = (rng.next() % 4) as i8;
            let end = start + 1 + (rng.next() % (9 - start as u32)) as i8;
            // outside the columns, but with column `end - cursor` inside
            // the digit buffer (0..=16) after a move
            let column = if rng.next().is_multiple_of(8) {
                end - 1 - (rng.next() % 15) as i8
            } else {
                start - 1 + (rng.next() % (end - start + 2) as u32) as i8
            };
            put(a, b, r + 2, column);
            put(a, b, r + 4, start);
            put(a, b, r + 5, end);
            let (picks, made) = (rng.next() % 6, rng.next() % 6);
            put(a, b, r + 6, picks as u8);
            put(
                a,
                b,
                r + 7,
                if rng.next().is_multiple_of(8) {
                    (rng.next() % 0x40) as u8 - 0x18
                } else {
                    made as u8
                },
            );
            if rng.next().is_multiple_of(2) {
                put(a, b, r + 3, column);
            }
            let value = match rng.next() % 4 {
                0 => rng.next(),
                1 => rng.next() % 1000,
                _ => rng.next() % 1_000_000_000,
            };
            put(a, b, r + 0xc, value);
            if rng.next().is_multiple_of(4) {
                // nines and zeros in a row
                let digits = (0..9).fold(0u32, |v, _| {
                    v * 10 + [0, 9, 0, 9, 5][(rng.next() % 5) as usize]
                });
                put(a, b, r + 0xc, digits);
            }
        }
        put(a, b, trigger, bits(rng, &[0x20, 0x40]));
        put(a, b, repeat, bits(rng, &[0x1000, 0x2000, 0x4000, 0x8000]));
        if k % 2 == 0 {
            let want = eax(a, 0x008b_64e0, &[w]);
            let got = t::func_pad_message_select(b, w, call);
            if Mem::read::<u32>(a, cursor) != log {
                assert_eq!(got, want, "select {k}");
            }
        } else {
            a.call(input, Conv::Cdecl, &[w]).unwrap();
            t::func_pad_message_input_value(b, w, call);
        }
        same_memory(&format!("pad message {k}"), a, b);
    }
}

#[test]
fn round40_pad_message_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round40_pad_message(&mut a, &mut b, &mut Xorshift(0x4040_0011));
}

#[test]
fn round40_pad_message_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round40_pad_message(&mut a, &mut b, &mut Xorshift(0x4040_0012));
}

/// Round 40 group B: `TOBtlGetChangePad` and the battle windows' wait
/// functions `TOWaitBtlWinChange`, `TOWaitBtlWinStdMode2_008a8b50` and
/// `TOWaitBtlWinStdMode3_008a8b90` over logging stand-ins (their returns
/// from a table of a few values; the `TOBtlGetStdPad` stand-in copies the
/// list mode `0x0133d6e8` it sees into a cell).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]
fn round40_change_pad(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge::{self as t, WaitBtlWin};
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let seen = alloc(a, b, 4);
    let seq = round38_seq(counter, table);
    let mode = a.rebase(0x0133_d6e8);
    // mov eax, [mode]; mov [seen], eax
    let mut std_pad = vec![0xa1];
    std_pad.extend_from_slice(&mode.to_le_bytes());
    std_pad.push(0xa3);
    std_pad.extend_from_slice(&seen.to_le_bytes());
    // inputIsRepButton also flips held 0x1000 (xor word [held], 0x1000),
    // so the held word read again after it counts
    let mut rep_button = seq.clone();
    rep_button.extend_from_slice(&[0x66, 0x81, 0x35]);
    rep_button.extend_from_slice(&a.rebase(0x025d_09d2).to_le_bytes());
    rep_button.extend_from_slice(&[0x00, 0x10]);
    let stands: Vec<Stand> = vec![
        (0x0063_0d30, vec![0], vec![], rep_button),
        (0x0089_97c0, vec![0], vec![], vec![]),
        (0x0089_a630, vec![], vec![], vec![]),
        (0x0089_aca0, vec![0, 1], vec![], seq.clone()),
        (0x0089_a330, vec![0, 1], vec![], vec![]),
        (0x0088_6ba0, vec![0], vec![], seq),
        (0x0089_cde0, vec![0], vec![], std_pad),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let w = alloc(a, b, 0x100);
    let list = alloc(a, b, 0x40);
    let (held, trigger, repeat) = (
        a.rebase(0x025d_09d2),
        a.rebase(0x025d_09d4),
        a.rebase(0x025d_09d6),
    );
    let count = a.rebase(0x0134_080c);
    let opens = [
        0.3f32,
        0.299_999_98,
        0.45,
        0.449_999_96,
        0.450_000_02,
        1.0,
        -1.0,
        f32::NAN,
        0.0,
    ];
    for k in 0..6000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        put(a, b, seen, rng.next());
        put(a, b, mode, rng.next());
        for j in 0..0x100 {
            let v = if rng.next().is_multiple_of(3) {
                rng.next()
            } else {
                rng.next() % 3
            };
            put(a, b, table + 4 * j, v);
        }
        for j in 0..0x38 {
            put(a, b, w + 4 * j, rng.next());
        }
        for j in 0..0x10 {
            put(a, b, list + 4 * j, rng.next());
        }
        let n = [0i32, 1, 2, 3, 5, 8][(rng.next() % 6) as usize];
        put(a, b, count, n);
        put(a, b, w + 0x20, list);
        put(a, b, w + 0x42, (rng.next() % 12) as i16 - 2);
        put(a, b, w + 0xb, (rng.next() % 6) as i8 - 2);
        let held_bits = [
            0u16, 0x4000, 4, 0x4004, 0x1000, 1, 0x1001, 0x5005, 0x4001, 0x1004,
        ][(rng.next() % 10) as usize];
        put(a, b, held, held_bits | (rng.next() as u16 & 0x0aaa));
        put(
            a,
            b,
            repeat,
            [0u16, 0x4000, 0x1000, 0x5000][(rng.next() % 4) as usize]
                | (rng.next() as u16 & 0x2fff),
        );
        put(
            a,
            b,
            trigger,
            [0u16, 0x20, 0x40, 0x60][(rng.next() % 4) as usize] | (rng.next() as u16 & 0xff9f),
        );
        put(
            a,
            b,
            a.rebase(0x0133_d6e0),
            [0u32, 1, 2][(rng.next() % 3) as usize],
        );
        put(a, b, a.rebase(0x023c_c092), (rng.next() % 4) as i8 - 1);
        if k % 3 == 0 {
            let open = opens[(rng.next() % opens.len() as u32) as usize];
            put(a, b, w + 0xe0, open);
            for (f, kind) in [
                (0x008a_87a0u32, WaitBtlWin::Change),
                (0x008a_8b50, WaitBtlWin::StdMode2),
                (0x008a_8b90, WaitBtlWin::StdMode3),
            ] {
                let want = eax(a, f, &[w]);
                assert_eq!(want, t::wait_btl_win(b, kind, w, call), "{kind:?} {k}");
                same_memory(&format!("{kind:?} {k}"), a, b);
            }
        } else {
            let want = eax(a, 0x008a_2160, &[w]);
            let got = t::btl_get_change_pad(b, w, call);
            // a click is logged with SndPlayClick's address as its tag
            let end = Mem::read::<u32>(a, cursor);
            let clicked = (log..end)
                .step_by(4)
                .any(|at| Mem::read::<u32>(a, at) == 0x0088_6ba0);
            if clicked {
                assert_eq!(got, want, "change pad {k}");
            }
        }
        same_memory(&format!("change pad {k}"), a, b);
    }
}

#[test]
fn round40_change_pad_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round40_change_pad(&mut a, &mut b, &mut Xorshift(0x4040_0021));
}

#[test]
fn round40_change_pad_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round40_change_pad(&mut a, &mut b, &mut Xorshift(0x4040_0022));
}

/// Round 40 group C: the save menu's choice window,
/// `TkMenuInitSaveChoice_008b3680` and `TkMenuDrawSaveChoice_008b3860`,
/// over logging stand-ins; the `TOGetMessageWH` stand-in stores the next
/// two table words as the width (`f32`) and height.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn round40_save_choice(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::menu::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let seq = round38_seq(counter, table);
    // the next table word to [esp + 0xc] (&w) and the one after to
    // [esp + 0x10] (&h)
    let mut wh = Vec::new();
    for off in [0x0cu8, 0x10] {
        wh.extend_from_slice(&[0x8b, 0x0d]);
        wh.extend_from_slice(&counter.to_le_bytes());
        wh.extend_from_slice(&[0x8b, 0x14, 0x8d]);
        wh.extend_from_slice(&table.to_le_bytes());
        wh.extend_from_slice(&[0x8b, 0x44, 0x24, off, 0x89, 0x10, 0xff, 0x05]);
        wh.extend_from_slice(&counter.to_le_bytes());
    }
    let stands: Vec<Stand> = vec![
        (0x0088_ec60, vec![], vec![], seq.clone()),
        (0x008b_ef30, vec![], vec![], vec![]),
        (0x008b_ee30, vec![0], vec![], seq.clone()),
        (0x008b_9650, vec![0, 1, 4, 5], vec![], wh),
        (0x008d_c9e0, vec![0], vec![], seq.clone()),
        (0x008a_f0d0, vec![0, 1], vec![], seq.clone()),
        (0x008a_f4f0, vec![0, 1], vec![], seq),
        (0x008f_9a40, vec![0, 1, 2, 3], vec![], vec![]),
        (0x0090_3a10, vec![0, 1, 2], vec![], vec![]),
        (0x0090_1cb0, vec![0, 1, 2, 3], vec![], vec![]),
        (0x008f_eb00, vec![0, 1, 2], vec![], vec![]),
        (0x008b_ea20, vec![], vec![], vec![]),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let scratch = b.stack_range().start + 0x1000;
    let w = alloc(a, b, 0x40);
    for k in 0..3000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        for j in 0..0x100 {
            let v = match rng.next() % 5 {
                0 => rng.next(),
                1 => rng.float().to_bits(),
                2 => rng.next() % 0x300,
                _ => ((rng.next() % 6000) as f32 / 10.0).to_bits(),
            };
            put(a, b, table + 4 * j, v);
        }
        for j in 0..0x10 {
            put(a, b, w + 4 * j, rng.next());
        }
        put(a, b, a.rebase(0x0186_6268), rng.next() as u16 & 0x0101);
        put(
            a,
            b,
            a.rebase(0x0186_6264),
            [0u32, 1, 2, rng.next()][(rng.next() % 4) as usize],
        );
        put(a, b, a.rebase(0x0186_6288), rng.next() as u16);
        if k % 2 == 0 {
            let want = eax(a, 0x008b_3860, &[]);
            assert_eq!(want, t::draw_save_choice(b, scratch, call), "draw {k}");
        } else {
            let want = eax(a, 0x008b_3680, &[w]);
            assert_eq!(want, t::init_save_choice(b, w, scratch, call), "init {k}");
        }
        same_memory(&format!("save choice {k}"), a, b);
    }
}

#[test]
fn round40_save_choice_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round40_save_choice(&mut a, &mut b, &mut Xorshift(0x4040_0031));
}

#[test]
fn round40_save_choice_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round40_save_choice(&mut a, &mut b, &mut Xorshift(0x4040_0032));
}

/// Round 40 group D: `MenuAbility_OpenWindow_008c6520` over logging
/// stand-ins: `MenuAbility_RefreshList` stores the next two words of a
/// table as the row count and the first usable slot,
/// `TkMenuGetCurrentPlayer` returns words of its own table (mostly 0..8,
/// sometimes 8..16, which reads the frame's line counts).
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn round40_ability_window(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::menu::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let players = alloc(a, b, 4);
    let player_table = alloc(a, b, 0x40);
    let window = alloc(a, b, 0x100);
    let w = alloc(a, b, 0x40);
    let mut refresh = Vec::new();
    for at in [a.rebase(0x0186_a38c), a.rebase(0x0186_a3b0)] {
        // mov ecx, [counter]; mov edx, [table + ecx*4]; mov [at], edx;
        // inc dword [counter]
        refresh.extend_from_slice(&[0x8b, 0x0d]);
        refresh.extend_from_slice(&counter.to_le_bytes());
        refresh.extend_from_slice(&[0x8b, 0x14, 0x8d]);
        refresh.extend_from_slice(&table.to_le_bytes());
        refresh.extend_from_slice(&[0x89, 0x15]);
        refresh.extend_from_slice(&at.to_le_bytes());
        refresh.extend_from_slice(&[0xff, 0x05]);
        refresh.extend_from_slice(&counter.to_le_bytes());
    }
    let mut open = vec![0xb8];
    open.extend_from_slice(&window.to_le_bytes());
    let seq = round38_seq(counter, table);
    let stands: Vec<Stand> = vec![
        (0x008c_9140, vec![0], vec![], open),
        (0x008c_93a0, vec![], vec![], vec![]),
        (0x008c_6800, vec![0], vec![], refresh),
        (
            0x008a_9860,
            vec![],
            vec![],
            round38_seq(players, player_table),
        ),
        (0x008a_eec0, vec![0], vec![], seq),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    for k in 0..3000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        put(a, b, players, 0u32);
        for j in 0..0x40 {
            let v = match j % 2 {
                // row counts
                0 => match rng.next() % 6 {
                    0 => rng.next(),
                    1 => (rng.next() % 8).wrapping_sub(4),
                    _ => rng.next() % 120,
                },
                // first usable slots (high half random)
                _ => match rng.next() % 4 {
                    0 => rng.next(),
                    1 => rng.next() | 0xffff,
                    _ => (rng.next() & 0xffff_0000) | (rng.next() % 120),
                },
            };
            put(a, b, table + 4 * j, v);
        }
        for j in 0..0x10 {
            // the fourth asks for the line count, whose frame array ends at
            // the stack cookie: 0..8 there; the fifth for the entry count
            // (0..16 reads the frame); the others index globals (past the
            // top rows into g_TkMenuItemGearCounts from 16 on)
            let p = match j {
                3 => rng.next() % 8,
                4 => rng.next() % 16,
                _ if rng.next().is_multiple_of(4) => rng.next() % 24,
                _ => rng.next() % 8,
            };
            put(a, b, player_table + 4 * j, p);
        }
        for j in 0..0x40 {
            put(a, b, window + 4 * j, rng.next());
        }
        put(
            a,
            b,
            window + 0x3a,
            [0u16, 1, 4, 6, 7, 12, rng.next() as u16][(rng.next() % 7) as usize],
        );
        for j in 0..0x10 {
            put(a, b, w + 4 * j, rng.next());
        }
        for at in [
            0x0186_a3e4u32,
            0x0186_a3e8,
            0x0186_a3ec,
            0x0186_a3f0,
            0x0186_a408,
            0x0186_a40c,
        ] {
            put(a, b, a.rebase(at), rng.next());
        }
        let want = eax(a, 0x008c_6520, &[w]);
        assert_eq!(
            want,
            t::ability_open_window(b, w, call),
            "ability window {k}"
        );
        same_memory(&format!("ability window {k}"), a, b);
    }
}

#[test]
fn round40_ability_window_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round40_ability_window(&mut a, &mut b, &mut Xorshift(0x4040_0041));
}

#[test]
fn round40_ability_window_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round40_ability_window(&mut a, &mut b, &mut Xorshift(0x4040_0042));
}

/// Round 41 group A: the scrolling text window's clock reset and step,
/// `TOTextScrollInit_008ded00` and `TOTextScrollStep_008ded60`; the
/// scroll tick is a logging stand-in that stores the next table word as
/// the new line count, so the second clamp sees every range.
fn round41_text_scroll(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let next_line = alloc(a, b, 4);
    // mov edx, [esp+4]; mov eax, [next_line]; mov [edx+0xa0], eax
    let mut tick = vec![0x8b, 0x54, 0x24, 0x04, 0xa1];
    tick.extend_from_slice(&next_line.to_le_bytes());
    tick.extend_from_slice(&[0x89, 0x82, 0xa0, 0, 0, 0]);
    round38_stands(a, b, cursor, 0, &[(0x008d_f570, vec![0], vec![], tick)]);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let w = alloc(a, b, 0x140);
    let text = alloc(a, b, 4);
    let mode = a.rebase(0x0186_5f00);
    let near = |rng: &mut Xorshift| -> i32 {
        [
            0i32,
            1,
            -1,
            2,
            5,
            9,
            10,
            11,
            i32::MIN,
            i32::MAX,
            rng.next().cast_signed(),
        ][(rng.next() % 11) as usize]
    };
    for k in 0..4000u32 {
        put(a, b, cursor, log);
        for j in 0..0x50 {
            put(a, b, w + 4 * j, rng.next());
        }
        put(
            a,
            b,
            mode,
            [0u32, 1, 2, 0x8000_0000][(rng.next() % 4) as usize],
        );
        let first = [0u32, 1, 0x80][(rng.next() % 3) as usize];
        put(a, b, text, first | (rng.next() & 0xffff_ff00));
        put(a, b, w + 0x98, text);
        put(a, b, w + 0xa0, near(rng));
        let last = if rng.next().is_multiple_of(4) {
            near(rng)
        } else {
            10
        };
        put(a, b, w + 0xa8, last);
        put(a, b, next_line, near(rng));
        put(
            a,
            b,
            w + 0xb4,
            [0u32, 0, 1, 0x100, u32::MAX][(rng.next() % 5) as usize],
        );
        if k % 2 == 0 {
            let want = eax(a, 0x008d_ed00, &[w]);
            assert_eq!(want, t::text_scroll_init(b, w, call), "scroll init {k}");
            same_memory(&format!("scroll init {k}"), a, b);
        } else {
            let want = eax(a, 0x008d_ed60, &[w]);
            assert_eq!(want, t::text_scroll_step(b, w, call), "scroll step {k}");
            same_memory(&format!("scroll step {k}"), a, b);
        }
    }
}

#[test]
fn round41_text_scroll_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round41_text_scroll(&mut a, &mut b, &mut Xorshift(0x4041_0011));
}

#[test]
fn round41_text_scroll_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round41_text_scroll(&mut a, &mut b, &mut Xorshift(0x4041_0012));
}

/// Round 41 group B: `LocalizationManager_getText` over random kits
/// (lines of letters, `\r`, high bytes and now and then a NUL; lengths
/// short of, at and past the text, negative and 0). The kit reader is a
/// stand-in that counts its calls and installs the next text and length.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names
)]
fn round41_get_text(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use instance::bridge as i;
    let cells = alloc(a, b, 0x10);
    // mov eax, [cells]; mov [ecx+0x18], eax; mov eax, [cells+4];
    // mov [ecx+0x14], eax; inc dword [cells+8]; ret
    let mut kit = vec![0xa1];
    kit.extend_from_slice(&cells.to_le_bytes());
    kit.extend_from_slice(&[0x89, 0x41, 0x18, 0xa1]);
    kit.extend_from_slice(&(cells + 4).to_le_bytes());
    kit.extend_from_slice(&[0x89, 0x41, 0x14, 0xff, 0x05]);
    kit.extend_from_slice(&(cells + 8).to_le_bytes());
    kit.push(0xc3);
    let s = stand_in8(a, b, 0, &kit);
    redirect(a, b, 0x006d_b2f0, s);
    let this = alloc(a, b, 0x20);
    let texts = [alloc(a, b, 0x1000), alloc(a, b, 0x1000)];
    let out = alloc(a, b, 0x800);
    let read_kit = |m: &mut Oracle, this: u32| {
        call(m, 0x006d_b2f0, Conv::Thiscall(this), &[]);
    };
    for k in 0..3000u32 {
        let mut lens = [0i32; 2];
        for (t, len) in texts.iter().zip(&mut lens) {
            let mut v = Vec::new();
            let lines = rng.next() % 8;
            for _ in 0..lines {
                let n = if rng.next().is_multiple_of(16) {
                    rng.next() % 1000
                } else {
                    rng.next() % 40
                };
                for _ in 0..n {
                    let r = rng.next();
                    v.push(match r % 23 {
                        0 => b'\r',
                        1 => 0,
                        2 => 0x80 | (r >> 8) as u8,
                        _ => b'a' + (r >> 8) as u8 % 26,
                    });
                }
                v.push(b'\n');
            }
            for _ in 0..rng.next() % 5 {
                v.push(b'a' + (rng.next() % 26) as u8);
            }
            *len = v.len() as i32;
            v.extend((0..16).map(|_| rng.next() as u8));
            v.resize(0x1000, 0);
            a.write(*t, &v).unwrap();
            b.write(*t, &v).unwrap();
        }
        // whole multiples of 0x100 (a zero low byte) count as lengths too
        let pick = |rng: &mut Xorshift, len: i32| match rng.next() % 7 {
            0 => len - 1 - (rng.next() % 3) as i32,
            1 => len + (rng.next() % 4) as i32,
            2 => -1 - (rng.next() % 2) as i32,
            3 => 0,
            4 => 0x100 * (1 + (rng.next() % 3) as i32),
            _ => len,
        };
        let first_len = pick(rng, lens[0]);
        put(a, b, this + 0x18, first_len);
        put(a, b, this + 0x14, texts[0]);
        let kit_len = pick(rng, lens[1]).max(1);
        put(a, b, cells, kit_len);
        put(a, b, cells + 4, texts[1]);
        let index = match rng.next() % 5 {
            0 => rng.next(),
            1 => u32::MAX,
            _ => rng.next() % 9,
        };
        let junk = rng.next();
        for j in 0..0x200 {
            put(a, b, out + 4 * j, rng.next());
        }
        let want = call(a, 0x006d_b1f0, Conv::Thiscall(this), &[index, out, junk]).eax;
        let got = i::localization_get_text(b, this, index, out, read_kit);
        assert_eq!(want, got, "get text {k}");
        same_memory(&format!("get text {k}"), a, b);
    }
}

#[test]
fn round41_get_text_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round41_get_text(&mut a, &mut b, &mut Xorshift(0x4041_0021));
}

#[test]
fn round41_get_text_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round41_get_text(&mut a, &mut b, &mut Xorshift(0x4041_0022));
}

/// Round 41 group C: `MagicFile_Unload` over logging stand-ins for every
/// callee (printf through its import slot, logging the id only for the
/// completion message); the VFX destroy and the shader-table unload
/// stand-ins may switch the previous magic (a cell each), so every re-read
/// of it counts. Magic ids, buffers, the pair table and the instance runs
/// random.
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn round41_magic_unload(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use menu::bridge as mb;
    let log = alloc(a, b, 0x10000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let cells = alloc(a, b, 0x10);
    let seq = round38_seq(counter, table);
    let previous = a.rebase(0x00c6_4ca8);
    // mov eax, [cell]; mov [previous], eax
    let switch = |cell: u32| {
        let mut v = vec![0xa1];
        v.extend_from_slice(&cell.to_le_bytes());
        v.push(0xa3);
        v.extend_from_slice(&previous.to_le_bytes());
        v
    };
    let done_fmt = a.rebase(0x00b7_05f0);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x0064_5320, vec![], vec![], switch(cells)),
        (0x0082_b2a0, all(2), vec![], vec![]),
        (0x0068_6fe0, all(1), vec![], vec![]),
        (0x0064_5280, all(2), vec![], switch(cells + 4)),
        (0x0064_20f0, all(1), vec![], vec![]),
        (0x0064_20e0, vec![], vec![], switch(cells + 8)),
        (0x0064_1da0, all(1), vec![], seq),
        (0x009d_acc0, all(2), vec![], vec![]),
        (0x0067_94f0, all(2), vec![], vec![]),
        (0x0064_2fe0, all(5), vec![], vec![]),
        (0x0064_2070, all(1), vec![], vec![]),
        (0x0064_35e0, all(1), vec![], vec![]),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    // printf: logs the format, and the id after the completion message
    let mut tail = vec![0x8b, 0x44, 0x24, 0x04, 0x3d];
    tail.extend_from_slice(&done_fmt.to_le_bytes());
    tail.extend_from_slice(&[0x75, 0x13, 0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d]);
    tail.extend_from_slice(&cursor.to_le_bytes());
    tail.extend_from_slice(&[0x89, 0x01, 0x83, 0x05]);
    tail.extend_from_slice(&cursor.to_le_bytes());
    tail.push(4);
    let printf = stand_in8(
        a,
        b,
        0x80 * stands.len() as u32,
        &pick_logger(cursor, 0x5f, &[0], &[], &tail),
    );
    put(a, b, a.rebase(0x00b0_c380), printf);
    // every callee gets the original's argument count (the stand-ins log
    // words the caller never pushed as whatever the frame holds)
    let arity: Vec<(u32, usize)> = stands.iter().map(|s| (s.0, s.1.len())).collect();
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| {
        if let Some(&(_, n)) = arity.iter().find(|e| e.0 == sym) {
            assert_eq!(args.len(), n, "{sym:#x} arguments");
        }
        eax(m, sym, args)
    };
    let printf_b = |m: &mut Oracle, fmt: u32, arg: Option<u32>| {
        let mut args = vec![fmt];
        args.extend(arg);
        m.call(printf, Conv::Cdecl, &args).unwrap();
    };
    let prx = a.rebase(0x00c6_4cb0);
    let pairs = a.rebase(mb::OP_PAIR_TABLE);
    let runs = a.rebase(mb::VFX_RUNS);
    let magics = [
        u32::MAX,
        0x1b5,
        0x1b6,
        0x246,
        0x1b4,
        0x247,
        0x1b5,
        0x1b6,
        0x246,
    ];
    let pick_magic = |rng: &mut Xorshift| {
        if rng.next().is_multiple_of(9) {
            rng.next()
        } else {
            magics[(rng.next() % magics.len() as u32) as usize]
        }
    };
    for k in 0..4000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        for j in 0..0x40 {
            put(a, b, table + 4 * j, rng.next());
        }
        let magic = pick_magic(rng);
        put(a, b, previous, magic);
        put(
            a,
            b,
            prx,
            if rng.next().is_multiple_of(8) {
                u32::MAX
            } else {
                rng.next()
            },
        );
        for j in 0..3 {
            let v = if rng.next().is_multiple_of(4) {
                pick_magic(rng)
            } else {
                magic
            };
            put(a, b, cells + 4 * j, v);
        }
        put(a, b, a.rebase(0x0193_fdac), rng.next());
        put(a, b, a.rebase(0x022f_b3f8), rng.next());
        put(
            a,
            b,
            a.rebase(0x022f_b3dc),
            if rng.next().is_multiple_of(3) {
                0
            } else {
                rng.next()
            },
        );
        for t in mb::MAGIC_BUFFERS {
            for j in 0..6 {
                let v = if rng.next().is_multiple_of(3) {
                    0
                } else {
                    rng.next()
                };
                put(a, b, a.rebase(t) + 4 * j, v);
            }
        }
        let n = [-1i32, 0, 1, 2, 5, 8][(rng.next() % 6) as usize];
        put(a, b, pairs, [n.cast_unsigned(), rng.next()]);
        for j in 1..=9 {
            put(a, b, pairs + 8 * j, [rng.next(), rng.next()]);
        }
        let n = [-1i32, 0, 1, 2, 4][(rng.next() % 5) as usize];
        put(a, b, a.rebase(0x012f_411c), n);
        for j in 0..4 {
            let lo = if rng.next().is_multiple_of(3) {
                u32::MAX - rng.next() % 3
            } else {
                rng.next()
            };
            let count = [-1i32, 0, 1, 3][(rng.next() % 4) as usize];
            put(
                a,
                b,
                runs + 16 * j,
                [lo, rng.next(), count.cast_unsigned(), rng.next()],
            );
        }
        let want = eax(a, 0x009d_a960, &[]);
        assert_eq!(
            want,
            mb::magic_file_unload(b, call, printf_b),
            "magic unload {k}"
        );
        same_memory(&format!("magic unload {k}"), a, b);
    }
}

#[test]
fn round41_magic_unload_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round41_magic_unload(&mut a, &mut b, &mut Xorshift(0x4041_0031));
}

#[test]
fn round41_magic_unload_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round41_magic_unload(&mut a, &mut b, &mut Xorshift(0x4041_0032));
}

/// Round 41 group D: `MagicFile_Update` over logging stand-ins for
/// `MagicFile_Unload` and `MsSetChrVisible` (its result from a table).
fn round41_magic_update(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use menu::bridge as mb;
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x40);
    let seq = round38_seq(counter, table);
    let stands: Vec<Stand> = vec![
        (0x009d_a960, vec![], vec![], vec![]),
        (0x0079_6660, vec![0, 1], vec![], seq),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let word = |rng: &mut Xorshift| match rng.next() % 4 {
        0 => u32::MAX,
        1 => 0xffff_fffe,
        2 => 0x7fff_ffff,
        _ => rng.next() % 0x300,
    };
    for k in 0..2000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        for j in 0..0x10 {
            put(a, b, table + 4 * j, rng.next());
        }
        put(a, b, a.rebase(0x00c6_4cb0), word(rng));
        put(a, b, a.rebase(0x00c6_4ca8), word(rng));
        put(a, b, a.rebase(0x00c6_4cdc), word(rng));
        let want = eax(a, 0x009d_ac60, &[]);
        assert_eq!(want, mb::magic_file_update(b, call), "magic update {k}");
        same_memory(&format!("magic update {k}"), a, b);
    }
}

#[test]
fn round41_magic_update_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round41_magic_update(&mut a, &mut b, &mut Xorshift(0x4041_0041));
}

#[test]
fn round41_magic_update_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round41_magic_update(&mut a, &mut b, &mut Xorshift(0x4041_0042));
}

/// Round 42 group A: the battle message windows, the battle window scene
/// openers, `TOBtlElementLampFlags_00897650` and the kernel monster-name
/// slot over logging stand-ins for every callee (results the next words
/// of a table of small values, so the switches and sign tests come up).
/// `MsGetComData` stores a test word through its name pointer (a stack
/// word on both sides).
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names
)]
fn round42_messages(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x100);
    let cells = alloc(a, b, 0x10);
    let seq = round38_seq(counter, table);
    // MsGetComData(id, &table): [table] = cells[0]; the record is cells + 4
    let mut com = store_through(1, cells);
    com.push(0xb8); // mov eax, cells + 4
    com.extend_from_slice(&(cells + 4).to_le_bytes());
    let stands: Vec<Stand> = vec![
        (0x0078_f8f0, vec![0, 1], vec![], seq.clone()), // MsGetRomBtlText
        (0x0089_dba0, vec![0, 1], vec![], seq.clone()), // TOBtlOpenSimpleHelpMes
        (0x008b_57c0, vec![0, 1, 2], vec![], seq.clone()), // TOBtlSetMacroCommandType
        (0x008b_57f0, vec![0, 1, 2], vec![], seq.clone()), // TOBtlSetMacroCommandValue
        (0x0079_af10, vec![0], vec![], seq.clone()),    // MsGetRamChrName
        (0x008a_c850, vec![0], vec![], seq.clone()),    // TOGetSaveChrName
        (0x0078_fcf0, vec![0, 1, 2], vec![], seq.clone()), // MsMenuGetText
        (0x0079_a4b0, vec![0], vec![], com),            // MsGetComData
        (0x0079_0240, vec![0], vec![], seq.clone()),    // MsMenuWeakElement_00790240
        (0x0078_f910, vec![0], vec![], seq.clone()),    // MsMenuAbsElement_0078f910
        (0x0078_fe80, vec![0], vec![], seq.clone()),    // MsMenuInvElement
        (0x0078_fe40, vec![0], vec![], seq.clone()),    // MsMenuHalfElement
        (0x0078_4260, vec![0, 1], vec![], seq.clone()), // Mon_GetName
        (0x008a_eec0, vec![0], vec![], seq.clone()),    // TOSeqInitScene
        (0x0078_f700, vec![0], vec![], seq.clone()),    // MsCheckLeftWindow
        (0x008a_f0d0, vec![0, 1], vec![], seq),         // TOSeqStartScene
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let arity: Vec<(u32, usize)> = stands.iter().map(|s| (s.0, s.1.len())).collect();
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| {
        if let Some(&(_, n)) = arity.iter().find(|e| e.0 == sym) {
            let n = if sym == 0x0079_a4b0 { 2 } else { n };
            assert_eq!(args.len(), n, "{sym:#x} arguments");
        }
        eax(m, sym, args)
    };
    let local = b.stack_range().start + 0x100;
    let w = alloc(a, b, 0xa0);
    let kernel = a.rebase(0x0112_a034);
    let small = [0u32, 1, 2, 3, 4, 5, u32::MAX, 0x8000_0000];
    for k in 0..3000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        for j in 0..0x40 {
            let v = if rng.next().is_multiple_of(4) {
                rng.next()
            } else {
                small[(rng.next() % small.len() as u32) as usize]
            };
            put(a, b, table + 4 * j, v);
        }
        for j in 0..4 {
            put(a, b, cells + 4 * j, rng.next());
        }
        for j in 0..0x28 {
            put(a, b, w + 4 * j, rng.next());
        }
        let pick = |rng: &mut Xorshift| {
            if rng.next().is_multiple_of(3) {
                rng.next()
            } else {
                small[(rng.next() % small.len() as u32) as usize]
            }
        };
        let (x, y) = (pick(rng), pick(rng));
        let what = format!("round42 {k} {x:#x} {y:#x}");
        let r = match k % 11 {
            0 => (
                eax(a, 0x0089_34d0, &[]),
                t::btl_draw_first_strike_message_window(b, true, call),
            ),
            1 => (
                eax(a, 0x0089_34f0, &[]),
                t::btl_draw_first_strike_message_window(b, false, call),
            ),
            2 => (
                eax(a, 0x0089_2870, &[x, y]),
                t::btl_draw_name_message_window(b, true, x, y, call),
            ),
            3 => (
                eax(a, 0x0089_7200, &[x, y]),
                t::btl_draw_name_message_window(b, false, x, y, call),
            ),
            4 => (
                eax(a, 0x0089_3660, &[x]),
                t::btl_draw_get_money_message_window(b, x.cast_signed(), call),
            ),
            5 => (
                eax(a, 0x0089_35f0, &[x, y]),
                t::btl_draw_get_limit_type_message_window(b, x, y, call),
            ),
            6 => (
                eax(a, 0x0089_5320, &[x, y]),
                t::btl_draw_learning_message_window(b, x, y, local, call),
            ),
            7 => {
                let element = if rng.next().is_multiple_of(2) {
                    y
                } else {
                    rng.next() % 5
                };
                (
                    eax(a, 0x0089_7650, &[x, element]),
                    t::btl_element_lamp_flags(b, x, element, call),
                )
            }
            8 => {
                put(a, b, kernel, rng.next());
                (
                    eax(a, 0x0077_3780, &[x]),
                    t::kernel_slot_monster_name(b, x, call),
                )
            }
            9 => {
                put(a, b, w + 8, (x % 6) as u16);
                (eax(a, 0x008a_8640, &[w]), t::open_btl_win_main(b, w, call))
            }
            _ => {
                put(a, b, w + 8, (x % 6) as u16);
                (
                    eax(a, 0x008a_85a0, &[w]),
                    t::open_btl_limit_break(b, w, call),
                )
            }
        };
        assert_eq!(r.0, r.1, "{what}");
        same_memory(&what, a, b);
        settle(a, b);
    }
}

#[test]
fn round42_messages_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round42_messages(&mut a, &mut b, &mut Xorshift(0x4042_0011));
}

#[test]
fn round42_messages_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round42_messages(&mut a, &mut b, &mut Xorshift(0x4042_0012));
}

/// Round 42 group B: the enemy limit bars, the CTB window's set-up, Wakka's
/// and Auron's limit windows, the get-item message and the menu scene key
/// pick, over logging stand-ins (results the next words of a table without
/// 0: the originals divide by them unguarded) and the original remaps and
/// `MenuScene_KeyEase`. Frame words (widths, the CTB count) are written by
/// the stand-ins and not logged.
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names
)]
fn round42_windows(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let cells = alloc(a, b, 0x10);
    let list = alloc(a, b, 0x40);
    let seq = round38_seq(counter, table);
    let mut ctb = vec![0xb8]; // mov eax, list
    ctb.extend_from_slice(&list.to_le_bytes());
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x0079_5250, all(1), vec![], seq.clone()), // MsGetChrLimitBarFlag
        (0x0079_5550, all(1), vec![], seq.clone()), // MsGetChrLimitGauge
        (0x0079_5590, all(1), vec![], seq.clone()), // MsGetChrLimitGaugeMax
        (0x0079_a200, vec![], vec![], ctb),         // MsGetCTBlist
        (0x008b_1020, vec![], vec![], seq.clone()), // TOTransTexture
        (0x008a_c2c0, vec![], vec![], seq.clone()), // TOGetBtlMenuTexAdrs
        (0x008b_0b10, all(7), vec![], seq.clone()), // TOTransImageHL
        (0x0090_47c0, all(3), vec![], seq.clone()), // TOSetPktBuffAdrs
        (0x0090_4600, vec![], vec![], seq.clone()), // TOOpenPktbuff
        (0x008a_1e50, vec![], vec![], seq.clone()), // 0x008a1e50
        (0x008a_e2f0, all(3), vec![], seq.clone()), // TOSeqDraw
        (0x008a_e530, all(2), vec![], seq.clone()), // TOSeqGetSceneState_008ae530
        (0x0078_f8f0, all(2), vec![], seq.clone()), // MsGetRomBtlText
        // ToGetBtlEasyFontWidth(text, &width, ..): width = cells[0]
        (
            0x0090_5290,
            vec![0, 2, 3, 4],
            vec![],
            store_through(1, cells),
        ),
        (0x0090_16b0, all(7), vec![], seq.clone()), // TOMkpCrossExtMesFontLClut
        (0x008f_3930, vec![], vec![], seq.clone()), // TOClosePktbuff
        (0x008f_5bc0, vec![], vec![], seq.clone()), // TODrawKick
        (0x008b_1350, all(2), vec![], seq.clone()), // setButtonData
        (0x0064_2ec0, all(2), vec![], seq.clone()), // graphicSetButtonIconScale
        (0x0090_29b0, all(3), vec![], seq.clone()), // TOMkpPadButtonData
        (0x0064_2c60, vec![], vec![], seq.clone()), // graphicResetButtonIconScale
        (0x008b_1330, vec![], vec![], seq.clone()), // resetButtonData
        (0x008b_57c0, all(3), vec![], seq.clone()), // TOBtlSetMacroCommandType
        (0x008b_57f0, all(3), vec![], seq.clone()), // TOBtlSetMacroCommandValue
        (0x0089_dba0, all(2), vec![], seq.clone()), // TOBtlOpenSimpleHelpMes
        (0x00a7_bbc0, all(5), vec![], seq),         // 0x00a7bbc0
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let arity: Vec<(u32, usize)> = stands.iter().map(|s| (s.0, s.1.len())).collect();
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| {
        if let Some(&(_, n)) = arity.iter().find(|e| e.0 == sym) {
            let n = match sym {
                0x0079_a200 => 1,
                0x0090_5290 => 5,
                _ => n,
            };
            assert_eq!(args.len(), n, "{sym:#x} arguments");
        }
        eax(m, sym, args)
    };
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call_f64(at, args).unwrap()
    };
    let local = b.stack_range().start + 0x100;
    let g = |a: &Oracle, s: u32| a.rebase(s);
    let words = [1u32, 2, 3, 5, 0x30, 0x7f, 0x1000, u32::MAX, 0xffff_fff0];
    let keys = alloc(a, b, 0x40);
    let track = alloc(a, b, 0x10);
    let buttons = alloc(a, b, 0x40);
    for k in 0..3000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        for j in 0..0x40 {
            let v = if rng.next().is_multiple_of(5) {
                rng.next() | 1
            } else {
                words[(rng.next() % words.len() as u32) as usize]
            };
            put(a, b, table + 4 * j, v);
        }
        for j in 0..4 {
            put(a, b, cells + 4 * j, rng.next());
        }
        let what = format!("round42 b {k}");
        let r = match k % 7 {
            0 | 1 => {
                let n = [-1i32, 0, 1, 3, 8, 8][(rng.next() % 6) as usize];
                put(a, b, g(a, 0x0133_f0ec), n);
                for j in 0..8 {
                    let id = [0xffu16, 0xff, 0, 3, 0x14, 0x1ff][(rng.next() % 6) as usize];
                    put(a, b, g(a, 0x0133_f0f0) + 2 * j, id);
                }
                for j in 0..0x20 {
                    let v = if rng.next().is_multiple_of(2) {
                        rng.next() % 0x100
                    } else {
                        rng.next()
                    };
                    put(a, b, g(a, 0x0133_f988) + 4 * j, v);
                }
                put(a, b, g(a, 0x025d_0aa4), rng.next() as u16);
                if k % 7 == 0 {
                    eax(a, 0x0089_7af0, &[]);
                    t::init_enemy_limit(b, call);
                } else {
                    eax(a, 0x0089_1220, &[]);
                    t::ctrl_enemy_limit(b, call);
                }
                (0, 0)
            }
            2 => {
                for j in 0..0x10 {
                    put(a, b, list + 4 * j, rng.next());
                }
                for j in 0..0x100 {
                    put(a, b, g(a, 0x0134_0440) + 4 * j, rng.next());
                }
                eax(a, 0x008a_0ce0, &[]);
                t::init_ctb_window(b, local, call);
                (0, 0)
            }
            3 => {
                put(
                    a,
                    b,
                    g(a, 0x0133_c93f),
                    [0u8, 1, 0x80][(rng.next() % 3) as usize],
                );
                let w = if rng.next().is_multiple_of(4) {
                    round38_quiet(rng)
                } else {
                    [0f32, 100.0, 37.5, 600.0][(rng.next() % 4) as usize].to_bits()
                };
                put(a, b, cells, w);
                // the scene state: 3 now and then
                put(
                    a,
                    b,
                    table + 4 * 7,
                    [3u32, 3, 2, 0x103][(rng.next() % 4) as usize],
                );
                eax(a, 0x0089_74a0, &[]);
                t::draw_wakka_limit_window(b, local, call);
                (0, 0)
            }
            4 => {
                let state = [2i16, 3, 4, 4, 5, -1][(rng.next() % 6) as usize];
                put(a, b, g(a, 0x0133_d6f4), state);
                put(a, b, g(a, 0x0133_d6f6), rng.next() as u16);
                let from = [0i16, 0, 1, 3, -1][(rng.next() % 5) as usize];
                put(a, b, g(a, 0x0133_d6fc), from);
                let to = [0i32, 1, 2, 4, 6, -1][(rng.next() % 6) as usize];
                put(a, b, g(a, 0x0133_d700), to);
                for j in 0..0x10 {
                    put(a, b, buttons + 4 * j, rng.next());
                }
                put(a, b, g(a, 0x0133_d704), buttons + 2);
                put(a, b, cells, round38_quiet(rng));
                eax(a, 0x0089_2740, &[]);
                t::draw_auron_limit_window(b, local, call, fcall);
                (0, 0)
            }
            5 => {
                let item = rng.next();
                let n = [-2i32, -1, 0, 1, 2, 7, i32::MIN][(rng.next() % 7) as usize];
                (
                    eax(a, 0x0089_3510, &[item, n.cast_unsigned()]),
                    tolib::bridge::btl_draw_get_item_message_window(b, item, n, call),
                )
            }
            _ => {
                for j in 0..0x10 {
                    let v = [0x1000u32, 0x2000, 0x3000, 0x4000, 0x2001, 0x0801, 0x0401]
                        [(rng.next() % 7) as usize]
                        | (rng.next() % 3) << 16;
                    put(a, b, keys + 4 * j, v);
                }
                put(
                    a,
                    b,
                    track,
                    [rng.next(), rng.next(), rng.next(), rng.next()],
                );
                put(
                    a,
                    b,
                    track + 6,
                    [0u16, 1, 0x8000][(rng.next() % 3) as usize],
                );
                put(a, b, track + 8, (rng.next() % 0x20) as u16);
                put(a, b, track + 10, (rng.next() % 0x20) as u16);
                let frames = [0u32, 1, 4, 10, 30][(rng.next() % 5) as usize];
                let frame = match rng.next() % 4 {
                    0 => 0,
                    1 => frames,
                    _ => rng.next() % 31,
                };
                let out = rng.next();
                let args = [keys, track, frame, frames, out];
                (
                    eax(a, 0x00a7_b290, &args),
                    scene::bridge::menu_scene_key_pick(b, args, call, fcall),
                )
            }
        };
        assert_eq!(r.0, r.1, "{what}");
        same_memory(&what, a, b);
        settle(a, b);
    }
}

#[test]
fn round42_windows_match_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round42_windows(&mut a, &mut b, &mut Xorshift(0x4042_0021));
}

#[test]
fn round42_windows_match_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round42_windows(&mut a, &mut b, &mut Xorshift(0x4042_0022));
}

/// A stand-in tail: `ST0` = the next f32 of `table` (`counter` counts).
fn round42_fseq(counter: u32, table: u32) -> Vec<u8> {
    // mov ecx, [counter]; fld dword [table + ecx*4]; inc dword [counter]
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&counter.to_le_bytes());
    v.extend_from_slice(&[0xd9, 0x04, 0x8d]);
    v.extend_from_slice(&table.to_le_bytes());
    v.extend_from_slice(&[0xff, 0x05]);
    v.extend_from_slice(&counter.to_le_bytes());
    v
}

/// Round 42 group C: the Libra window's lines, the battle help line, the
/// active character's cursor and the battle window callbacks, over
/// logging stand-ins. The projection logs the four words of its point,
/// writes `cells[2..4]` as the screen point and returns the next float of
/// the table in `ST0`, as `sinf`/`cosf` do.
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names
)]
fn round42_btl(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let cells = alloc(a, b, 0x20);
    let bytes = [alloc(a, b, 0x20), alloc(a, b, 0x20)];
    let menu = alloc(a, b, 4);
    let seq = round38_seq(counter, table);
    let fseq = round42_fseq(counter, table);
    let ret = |p: u32| {
        let mut v = vec![0xb8];
        v.extend_from_slice(&p.to_le_bytes());
        v
    };
    let mut com = store_through(1, cells);
    com.extend(ret(cells + 4));
    #[rustfmt::skip]
    let mut project = vec![
        0x8b, 0x54, 0x24, 0x08, 0x8b, 0x0d, // mov edx, [esp+8]; mov ecx, [cursor]
    ];
    project.extend_from_slice(&cursor.to_le_bytes());
    for k in 0..4u8 {
        project.extend_from_slice(&[0x8b, 0x42, 4 * k, 0x89, 0x41, 4 * k]); // mov eax, [edx+4k]; mov [ecx+4k], eax
    }
    project.extend_from_slice(&[0x83, 0x05]);
    project.extend_from_slice(&cursor.to_le_bytes());
    project.push(0x10);
    project.extend_from_slice(&[0x8b, 0x44, 0x24, 0x04, 0x8b, 0x0d]); // mov eax, [esp+4]; mov ecx, [cells+8]
    project.extend_from_slice(&(cells + 8).to_le_bytes());
    project.extend_from_slice(&[0x89, 0x08, 0x8b, 0x0d]); // mov [eax], ecx; mov ecx, [cells+12]
    project.extend_from_slice(&(cells + 12).to_le_bytes());
    project.extend_from_slice(&[0x89, 0x48, 0x04]); // mov [eax+4], ecx
    project.extend(fseq.clone());
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x0079_add0, all(1), vec![], seq.clone()), // MsGetRamChrHP
        (0x0079_adf0, all(1), vec![], seq.clone()), // MsGetRamChrHPmax
        (0x0078_faa0, all(1), vec![], ret(bytes[0])), // MsMenuDefCountStatus
        (0x0078_fae0, all(1), vec![], ret(bytes[1])), // MsGetChrLibraName_0078fae0
        (0x0078_fac0, all(1), vec![], seq.clone()), // MsGetChrLibraWord_0078fac0
        (0x0078_5290, vec![], vec![], ret(menu)),   // MsGetSaveMenu
        (0x0078_f8f0, all(2), vec![], seq.clone()), // MsGetRomBtlText
        (0x0089_9ec0, all(1), vec![], seq.clone()), // TOBtlSetDirectHelpMes
        (0x0089_97c0, all(1), vec![], seq.clone()), // TOBtlSetChangeHelpMes
        (0x0089_9970, all(1), vec![], seq.clone()), // TOBtlSetCommandHelpMes
        (0x0079_a4b0, vec![0], vec![], com),        // MsGetComData
        (0x0089_a8b0, all(1), vec![], seq.clone()), // TOBtlSetTargetHelpMes
        (0x0079_50b0, all(2), vec![], seq.clone()), // MsGetChrHeadPos
        (0x004c_eae0, all(1), vec![], fseq.clone()), // sinf
        (0x004c_eac0, all(1), vec![], fseq),        // cosf
        (0x0064_28f0, vec![], vec![], project), // graphicProject3DPointOntoScreenConsider2DScene
        (0x0090_1190, all(8), vec![], seq.clone()), // TOMkpActiveChrCursor
        (0x0078_f700, all(1), vec![], seq.clone()), // MsCheckLeftWindow
        (0x008a_2030, all(1), vec![], seq),     // TOInitGilInput
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let arity: Vec<(u32, usize)> = stands.iter().map(|s| (s.0, s.1.len())).collect();
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| {
        if let Some(&(_, n)) = arity.iter().find(|e| e.0 == sym) {
            let n = match sym {
                0x0079_a4b0 | 0x0064_28f0 => 2,
                _ => n,
            };
            assert_eq!(args.len(), n, "{sym:#x} arguments");
        }
        eax(m, sym, args)
    };
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call_f64(at, args).unwrap()
    };
    let local = b.stack_range().start + 0x100;
    let g = |a: &Oracle, s: u32| a.rebase(s);
    let w = alloc(a, b, 0x100);
    let list = alloc(a, b, 0x40);
    let words = [0u32, 1, 2, 3, 0xff, 0x3024, 0x8000, u32::MAX];
    let floats = [0f32, 1.0, -1.0, 0.5, 300.0, -0.0, 1e-30];
    for k in 0..3000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        let float_table = k % 4 == 2;
        for j in 0..0x40 {
            let v = if float_table {
                if rng.next().is_multiple_of(5) {
                    round38_quiet(rng)
                } else {
                    floats[(rng.next() % floats.len() as u32) as usize].to_bits()
                }
            } else if rng.next().is_multiple_of(4) {
                rng.next()
            } else {
                words[(rng.next() % words.len() as u32) as usize]
            };
            put(a, b, table + 4 * j, v);
        }
        for j in 0..8 {
            put(a, b, cells + 4 * j, rng.next());
        }
        let what = format!("round42 c {k}");
        match k % 4 {
            0 => {
                for buf in bytes {
                    for j in 0..0x20 {
                        let v = [0u8, 0xff, 0xff, 1, 0x80][(rng.next() % 5) as usize];
                        put(a, b, buf + j, v);
                    }
                }
                put(a, b, table + 8, rng.next());
                let n = [0u16, 0, 3, 0xffff, 0x7fff][(rng.next() % 5) as usize];
                put(a, b, g(a, 0x0133_f6b0), n);
                for j in 0..0x40 {
                    put(a, b, g(a, 0x0133_f6aa) + 4 * j, rng.next());
                }
                let chr = rng.next();
                eax(a, 0x0089_a160, &[chr]);
                t::set_libra_window_param(b, chr, call);
            }
            1 => {
                put(
                    a,
                    b,
                    g(a, 0x023c_c088),
                    [0xffu16, 0, 1, 0x1ff][(rng.next() % 4) as usize],
                );
                put(a, b, g(a, 0x025d_09d4), rng.next());
                put(a, b, menu, rng.next());
                let depth = [0i8, 1, 2, 3][(rng.next() % 4) as usize];
                put(a, b, g(a, 0x023c_c092), depth);
                let win = g(a, 0x0133_c952) + (0xf0 * i32::from(depth)).cast_unsigned();
                put(a, b, win, [0u8, 0, 1, 1, 2][(rng.next() % 5) as usize]);
                let commands = [
                    0x3002u16, 0x3024, 0x302a, 0x302b, 0x3029, 0x3056, 0x3116, 0x3117, 0x3118,
                    0x3119, 0x3123, 0x3124, 0x1000, 0xffff,
                ];
                put(
                    a,
                    b,
                    win + 0xa,
                    commands[(rng.next() % commands.len() as u32) as usize],
                );
                for j in 0..0x10 {
                    let v = [0xffu16, 0xff, 1, 0x8001, 0x7fff][(rng.next() % 5) as usize];
                    put(a, b, list + 2 * j, v);
                }
                put(a, b, win + 0x1e, list + 8);
                put(
                    a,
                    b,
                    win + 0x40,
                    [0i16, 1, -1, 3][(rng.next() % 4) as usize],
                );
                put(
                    a,
                    b,
                    g(a, 0x0133_d178),
                    [0u32, 0x8000, 0x3_8000, 0x4_0000][(rng.next() % 4) as usize],
                );
                put(
                    a,
                    b,
                    g(a, 0x0133_d0e0),
                    [0u32, 1, 3, 4][(rng.next() % 4) as usize],
                );
                put(a, b, g(a, 0x0133_d1f4), rng.next() as u16);
                put(a, b, g(a, 0x0133_c93c), rng.next() as u8);
                eax(a, 0x0089_12e0, &[]);
                t::ctrl_help_win(b, local, call);
            }
            2 => {
                for j in 0..0x30 {
                    let v = if rng.next().is_multiple_of(4) {
                        round38_quiet(rng)
                    } else {
                        floats[(rng.next() % floats.len() as u32) as usize].to_bits()
                    };
                    put(a, b, g(a, 0x0133_fa08) + 4 * j, v);
                }
                put(a, b, g(a, 0x0133_f7d4), rng.next());
                let chr = rng.next();
                eax(a, 0x0089_2590, &[chr]);
                t::draw_active_chr_cursor(b, chr, local, call, fcall);
            }
            _ => {
                for j in 0..0x40 {
                    put(a, b, w + 4 * j, rng.next());
                }
                let kind = match rng.next() % 4 {
                    0 => rng.next() as u16,
                    1 => 0,
                    _ => (rng.next() % 0x18) as u16,
                };
                put(a, b, w + 6, kind);
                put(
                    a,
                    b,
                    w + 0xc,
                    [0x3024u16, 0x3025, 0][(rng.next() % 3) as usize],
                );
                put(
                    a,
                    b,
                    g(a, 0x0134_0452),
                    [0u8, 1, 2][(rng.next() % 3) as usize],
                );
                eax(a, 0x008a_0de0, &[w]);
                t::set_callback(b, w, call);
            }
        }
        same_memory(&what, a, b);
        settle(a, b);
    }
}

#[test]
fn round42_btl_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round42_btl(&mut a, &mut b, &mut Xorshift(0x4042_0031));
}

#[test]
fn round42_btl_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round42_btl(&mut a, &mut b, &mut Xorshift(0x4042_0032));
}

/// Round 42 group D: `TOBtlGetTargetPad` over logging stand-ins. The
/// cursor type stays in 0..=3 (higher ones read the original's frame);
/// counts and the near-target search's index stay inside the target lists.
#[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
fn round42_target_pad(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x1000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x100);
    let kind = alloc(a, b, 4);
    let near = alloc(a, b, 4);
    let seq = round38_seq(counter, table);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x0078_0ce0, vec![], vec![], ret_cell(kind)), // MsBattleCursorType
        (0x0089_90f0, vec![], vec![], ret_cell(near)), // TOBtlSearchRevNearTarget
        (0x0088_6ba0, all(1), vec![], seq.clone()),    // SndPlayClick
        (0x0079_2de0, all(2), vec![], seq.clone()),    // MsSetBtlTarget
        (0x0079_2a40, all(1), vec![], seq.clone()),    // MsResetBtlTarget
        (0x0089_a630, vec![], vec![], seq.clone()),    // TOBtlSetPrevAction
        (0x0089_a330, all(2), vec![], seq),            // TOBtlSetNextAction
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let arity: Vec<(u32, usize)> = stands.iter().map(|s| (s.0, s.1.len())).collect();
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| {
        if let Some(&(_, n)) = arity.iter().find(|e| e.0 == sym) {
            assert_eq!(args.len(), n, "{sym:#x} arguments");
        }
        eax(m, sym, args)
    };
    let g = |a: &Oracle, s: u32| a.rebase(s);
    let w = alloc(a, b, 0x20);
    let own = alloc(a, b, 0x40);
    let pads = [
        0u16, 0x1000, 0x2000, 0x4000, 0x8000, 0x20, 0x40, 0x60, 0x3000, 0xffff,
    ];
    let pick = |rng: &mut Xorshift, v: &[u16]| v[(rng.next() % v.len() as u32) as usize];
    for k in 0..4000u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        for j in 0..0x40 {
            let v = [0u32, 1, 0xffff_ffff, 0x8000_0000, 5][(rng.next() % 5) as usize];
            put(a, b, table + 4 * j, v);
        }
        put(a, b, kind, (rng.next() % 4) | (rng.next() & 0xff00));
        put(a, b, near, (rng.next() % 8) | (rng.next() & 0xffff_0000));
        for j in 0..0x10 {
            let v = [rng.next(), 1, 0, 0x0800_0001][(rng.next() % 4) as usize];
            put(a, b, own + 4 * j, v);
        }
        for j in 0..0x30 {
            let v = [rng.next(), 1, 0][(rng.next() % 3) as usize];
            put(a, b, g(a, 0x0133_d178) + 4 * j, v);
        }
        let ptr = [own, g(a, 0x0133_d180), g(a, 0x0133_d1d4)][(rng.next() % 3) as usize];
        put(a, b, g(a, 0x0133_d1f8), ptr);
        put(a, b, g(a, 0x0133_d1f6), (rng.next() % 8) as u16 + 1);
        let index = [0u16, 1, 3, 7, 8, 0xffff][(rng.next() % 6) as usize];
        put(a, b, g(a, 0x0133_d1fc), index);
        put(a, b, g(a, 0x0133_d1fe), pick(rng, &[0, 1, 1, 2]));
        put(a, b, g(a, 0x0133_d200), pick(rng, &[0, 1, 2]));
        for at in [0x0133_d1d0u32, 0x0133_d17c] {
            put(a, b, g(a, at), pick(rng, &[0, 1, 3, 8, 0xffff]));
        }
        for at in [0x025d_09d6u32, 0x025d_09d4] {
            let v = if rng.next().is_multiple_of(3) {
                rng.next() as u16
            } else {
                pick(rng, &pads)
            };
            put(a, b, g(a, at), v);
        }
        // a count past 0x7fff (a signed compare), the index kept in range
        if k % 16 == 0 {
            put(a, b, g(a, 0x0133_d1f6), 0x8001u16);
            put(a, b, g(a, 0x0133_d1fc), pick(rng, &[1, 3]));
            put(a, b, g(a, 0x025d_09d6), 0u16);
        }
        put(
            a,
            b,
            g(a, 0x0133_d0e0),
            [0u32, 3, 1, 2][(rng.next() % 4) as usize],
        );
        put(
            a,
            b,
            g(a, 0x0133_d6e0),
            [0u32, 1, 2][(rng.next() % 3) as usize],
        );
        put(a, b, g(a, 0x0133_d0d0), rng.next());
        let depth = [0i8, 1, 2, 3][(rng.next() % 4) as usize];
        put(a, b, g(a, 0x023c_c092), depth);
        put(a, b, w + 8, rng.next());
        let what = format!("round42 d {k}");
        eax(a, 0x0089_d390, &[w]);
        t::get_target_pad(b, w, call);
        same_memory(&what, a, b);
        settle(a, b);
    }
}

#[test]
fn round42_target_pad_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    round42_target_pad(&mut a, &mut b, &mut Xorshift(0x4042_0041));
}

#[test]
fn round42_target_pad_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    round42_target_pad(&mut a, &mut b, &mut Xorshift(0x4042_0042));
}

/// Small round 1 b: `TOBtlGetMainPad` over the logging stand-ins of
/// [`round28_pad_prepare`] plus `inputIsPressButton` (from a cell) and
/// `forceAutoBattleOff`.
#[allow(
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
fn small1_main_pad(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x100);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let w = alloc(a, b, 0x100);
    let list = alloc(a, b, 0x200) + 0x100;
    for (k, (sym, n)) in (0u32..).zip([(0x0079_0790u32, 1u8), (0x0088_6ba0, 1)]) {
        let mut v = marker(cursor, 0x2840 + k);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        let s = stand_in8(a, b, 0x80 * k, &v);
        redirect(a, b, sym, s);
    }
    let s = stand_in8(a, b, 0x100, &marker(cursor, 0x2850));
    redirect(a, b, 0x0089_a630, s);
    let s = stand_in8(a, b, 0x180, &logger_returning(cursor, 2, cells));
    redirect(a, b, 0x0089_aca0, s);
    let s = stand_in8(a, b, 0x200, &logger_returning(cursor, 2, cells + 4));
    redirect(a, b, 0x0089_a330, s);
    // the side pads' skip command, battle command, window and scene calls
    let s = stand_in8(a, b, 0x280, &logger_returning(cursor, 1, cells + 8));
    redirect(a, b, 0x0078_fb80, s);
    let s = stand_in8(a, b, 0x300, &logger_returning(cursor, 1, cells + 0xc));
    redirect(a, b, 0x0078_4ff0, s);
    for (k, (sym, n)) in (0u32..).zip([(0x0079_2d60u32, 3u8), (0x0089_ade0, 1), (0x008a_e5a0, 1)]) {
        let mut v = marker(cursor, 0x2860 + k);
        v.pop();
        v.extend_from_slice(&logger_args(cursor, n));
        let s = stand_in8(a, b, 0x380 + 0x80 * k, &v);
        redirect(a, b, sym, s);
    }
    let s = stand_in8(a, b, 0x500, &marker(cursor, 0x2870));
    redirect(a, b, 0x0089_0eb0, s);
    // the standard pad's MsCheckLeftWindow and MsGetSaveWindow (count
    // from a cell into its third argument, the list from another)
    let more = alloc(a, b, 0x10);
    let windows = alloc(a, b, 0x40);
    let scratch = alloc(a, b, 4);
    let s = stand_in8(a, b, 0x580, &logger_returning(cursor, 1, more));
    redirect(a, b, 0x0078_f700, s);
    let mut v = logger_args(cursor, 2);
    v.truncate(v.len() - 5);
    v.extend_from_slice(&[0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x0d]);
    v.extend_from_slice(&(more + 4).to_le_bytes());
    v.extend_from_slice(&[0x89, 0x08, 0xa1]);
    v.extend_from_slice(&(more + 8).to_le_bytes());
    v.push(0xc3);
    let s = stand_in8(a, b, 0x600, &v);
    redirect(a, b, 0x0078_5410, s);
    put(a, b, more + 8, windows + 0x20);
    // inputIsPressButton (the summon button) and forceAutoBattleOff
    let press = alloc(a, b, 4);
    let s = stand_in8(a, b, 0x680, &logger_returning(cursor, 1, press));
    redirect(a, b, 0x0063_0cf0, s);
    let s = stand_in8(a, b, 0x700, &marker(cursor, 0x2880));
    redirect(a, b, 0x0078_71f0, s);
    let [flag, missed] = [0x0133_d6e4, 0x0133_d6ec].map(|x| a.rebase(x));
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let [open, libra, depth, pad] =
        [0x023c_c08c, 0x0133_d6e0, 0x023c_c092, 0x025d_09d2].map(|x| a.rebase(x));
    let idle = a.rebase(t::SCROLL_DONE);
    for k in 0..6000u32 {
        put(a, b, cursor, log);
        put(a, b, press, u32::from(rng.next().is_multiple_of(6)));
        put(
            a,
            b,
            more,
            [0u32, 1, 5, u32::MAX][(rng.next() % 4) as usize],
        );
        put(
            a,
            b,
            more + 4,
            [0u32, 1, 2, 4, 8, u32::MAX, (-5i32).cast_unsigned()][(rng.next() % 7) as usize],
        );
        let entries: [u16; 0x20] = std::array::from_fn(|_| {
            if (rng.next() >> 16).is_multiple_of(2) {
                0xff
            } else {
                rng.next() as u16
            }
        });
        put(a, b, windows, entries);
        put(a, b, flag, rng.next() % 3);
        put(a, b, missed, rng.next() % 3);
        put(
            a,
            b,
            cells,
            [0u32, 1, 0xfe, u32::MAX][(rng.next() % 4) as usize],
        );
        put(a, b, cells + 4, [0u32, 1, 2][(rng.next() % 3) as usize]);
        put(
            a,
            b,
            cells + 8,
            [0u32, 0x0001_0000, 0x3002, rng.next()][(rng.next() % 4) as usize],
        );
        put(a, b, cells + 0xc, u32::from(rng.next().is_multiple_of(3)));
        put(a, b, open, u32::from(!rng.next().is_multiple_of(8)));
        put(a, b, libra, [0u32, 0, 0, 1, 2][(rng.next() % 5) as usize]);
        put(a, b, depth, [0i8, 1, 2, -1][(rng.next() % 4) as usize]);
        let held = [
            0u16,
            0x9000,
            0x3000,
            0xc000,
            0x6000,
            0x1000,
            0xa000,
            rng.next() as u16,
        ][(rng.next() % 8) as usize];
        let trigger = [
            0u16,
            0,
            0,
            0x40,
            0x20,
            0x04,
            0x10,
            0x2000,
            0x8000,
            rng.next() as u16,
        ][(rng.next() % 10) as usize];
        let rep =
            [0x1000u16, 0x4000, 0x5000, 0x8000, 0, rng.next() as u16][(rng.next() % 6) as usize];
        put(a, b, pad, [held, trigger, rep]);
        let bytes: [u32; 0x40] = std::array::from_fn(|_| rng.next());
        put(a, b, w, bytes);
        let items: [u32; 0x40] = std::array::from_fn(|_| rng.next());
        put(a, b, list - 0x40, items);
        put(a, b, w + 0x20, list);
        put(
            a,
            b,
            w + 0x88,
            if rng.next().is_multiple_of(8) {
                rng.next()
            } else {
                idle
            },
        );
        let count = (1 + rng.next() % 20) as i16;
        let step = (1 + rng.next() % 4) as i16;
        let rows = (1 + rng.next() % 5) as i16;
        put(a, b, w + 0x38, [step, rows]);
        put(a, b, w + 0x44, count);
        put(a, b, w + 0x40, (rng.next() % 14) as i16 - 2);
        put(a, b, w + 0x42, (rng.next() % (count as u32 + 4)) as i16 - 2);
        put(
            a,
            b,
            w + 0xa,
            [(1 + rng.next() % 3) as i8, (rng.next() % 4) as i8 - 1],
        );
        put(a, b, a.rebase(0x023c_c088), rng.next() as u16);
        if k % 3 == 2 {
            // the right window's list check, more often
            put(a, b, pad + 2, 0x2000u16);
        }
        eax(a, 0x0089_c900, &[w]);
        t::to_btl_get_main_pad(b, w, scratch, call);
        let v: u32 = Mem::read(b, scratch);
        Mem::write(a, scratch, v);
        same_memory(&format!("main pad {k}"), a, b);
    }
}

#[test]
fn small1_main_pad_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    small1_main_pad(&mut a, &mut b, &mut Xorshift(0x5a11_0b01));
}

#[test]
fn small1_main_pad_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    small1_main_pad(&mut a, &mut b, &mut Xorshift(0x5a11_0b02));
}

/// Small round 1 c: `TOBtlCtrlCtbWindow` frame after frame from a
/// `TOBtlInitCtbWindow` state, `MsGetCTBlist` a logging stand-in returning
/// a list the test shuffles (actors from a small set, repeats and gaps),
/// `inputIsHoldButton` a stand-in answering up and down from two cells,
/// the step functions real (`Tick4_008a2ba0`, `TOBtlCtbAnimStep_008a2ad0`)
/// or a logging stand-in; rows, leaving rows and the scroll globals
/// disturbed now and then (free rows used up, targets off the shown
/// ticks).
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names
)]
fn small1_ctb_window(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 0x10);
    let list = alloc(a, b, 0x80);
    let local = alloc(a, b, 4);
    put(a, b, cells, list);
    // the count word differs (frame vs local): not logged
    let s = stand_in8(a, b, 0x800, &logger_returning(cursor, 0, cells));
    redirect(a, b, 0x0079_a200, s);
    // inputIsHoldButton(b): [cells + 4] for 0x02000000, [cells + 8] for
    // 0x04000000, else 1
    let mut v = vec![0x81, 0x7c, 0x24, 0x04, 0, 0, 0, 2, 0x75, 0x06, 0xa1];
    v.extend_from_slice(&(cells + 4).to_le_bytes());
    v.push(0xc3);
    v.extend_from_slice(&[0x81, 0x7c, 0x24, 0x04, 0, 0, 0, 4, 0x75, 0x06, 0xa1]);
    v.extend_from_slice(&(cells + 8).to_le_bytes());
    v.push(0xc3);
    v.extend_from_slice(&[0xb8, 1, 0, 0, 0, 0xc3]);
    let s = stand_in8(a, b, 0x880, &v);
    redirect(a, b, 0x0063_0cd0, s);
    let stub = stand_in8(a, b, 0x900, &logger_args(cursor, 1));
    let steps = [0, a.rebase(0x008a_2ba0), a.rebase(0x008a_2ad0), stub];
    let g = |o: &Oracle, x: u32| o.rebase(x);
    let rows = g(a, t::CTB_ROWS);
    let slots = g(a, t::CTB_SLOTS);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let call_ptr = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    let shuffle = |a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift| {
        for k in 0..16u32 {
            let id = [0u8, 1, 2, 3, 4, 5, 6, 7, 0x14, 0x15, 0x16, 0xff][(rng.next() % 12) as usize];
            put(a, b, list + 4 * k, id);
            // ticks past 0x7fff now and then (they widen as u16)
            let ticks = if rng.next().is_multiple_of(8) {
                rng.next() as u16
            } else {
                (rng.next() % 0x400) as u16
            };
            put(a, b, list + 4 * k + 2, ticks);
        }
    };
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        if k % 60 == 0 {
            shuffle(a, b, rng);
            for o in [&mut *a, &mut *b] {
                eax(o, 0x008a_0ce0, &[]);
            }
        }
        match rng.next() % 6 {
            0 | 1 => shuffle(a, b, rng),
            2 => {
                // two entries swap
                let (i, j) = (rng.next() % 16, rng.next() % 16);
                let x: u32 = Mem::read(a, list + 4 * i);
                let y: u32 = Mem::read(a, list + 4 * j);
                put(a, b, list + 4 * i, y);
                put(a, b, list + 4 * j, x);
            }
            _ => {}
        }
        if rng.next().is_multiple_of(4) {
            for _ in 0..4 {
                let r = rows + 0x18 * (rng.next() % 32);
                put(a, b, r + 4, (rng.next() % 0x400) as i32 - 0x10);
                put(a, b, r + 8, (rng.next() % 0x400) as i32 - 0x10);
                put(a, b, r + 0x14, steps[(rng.next() % 4) as usize]);
                // 0..=3: a step count of 4 makes TOBtlCtbAnimStep read its
                // caller's frame
                put(a, b, r + 0xf, (rng.next() % 4) as u8);
            }
        }
        if rng.next().is_multiple_of(8) {
            // rows in use (all of them now and then) and leaving rows
            let full = rng.next().is_multiple_of(3);
            for i in 0..32 {
                if full || rng.next().is_multiple_of(2) {
                    put(
                        a,
                        b,
                        rows + 0x18 * i,
                        [1u8, 1, 2, 0x80][(rng.next() % 4) as usize],
                    );
                }
            }
            for j in 16..32 {
                let p = if rng.next().is_multiple_of(2) {
                    0
                } else {
                    rows + 0x18 * (rng.next() % 32)
                };
                put(a, b, slots + 4 * j, p);
            }
        }
        if rng.next().is_multiple_of(3) {
            let off = [0i16, 0x16, -0x16, 0x2c, 5, -7, 0x84][(rng.next() % 7) as usize];
            put(a, b, g(a, 0x0134_0450), off);
            let off = [0i16, 0x16, -0x16, 0x2c, 5, -7, 0x84][(rng.next() % 7) as usize];
            put(a, b, g(a, 0x0134_0448), off);
            put(
                a,
                b,
                g(a, 0x0134_0446),
                [0i8, 1, 9, 10, 11, -1][(rng.next() % 6) as usize],
            );
        }
        put(a, b, cells + 4, u32::from(rng.next().is_multiple_of(3)));
        put(a, b, cells + 8, u32::from(rng.next().is_multiple_of(3)));
        let mask = [0u32, u32::MAX, rng.next(), 1 << (rng.next() % 32)][(rng.next() % 4) as usize];
        let junk = rng.next();
        eax(a, 0x0089_ece0, &[junk, mask]);
        t::ctrl_ctb_window(b, mask, local, call, call_ptr);
        let v: u32 = Mem::read(b, local);
        Mem::write(a, local, v);
        same_memory(&format!("ctb window {k}"), a, b);
        settle(a, b);
    }
}

#[test]
fn small1_ctb_window_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    small1_ctb_window(&mut a, &mut b, &mut Xorshift(0x5a11_0c01));
}

#[test]
fn small1_ctb_window_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    small1_ctb_window(&mut a, &mut b, &mut Xorshift(0x5a11_0c02));
}

/// Small round 1 d: `TOBtlSetWindowLayout` for every kind (and some out of
/// range), the remaps real, the box constants in `.rdata` replaced by
/// random values in both oracles now and then (results past `i32` for
/// `ftol2_sse`), `MsGetRomBtlText` a logging stand-in.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]
fn small1_window_layout(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use tolib::bridge as t;
    let log = alloc(a, b, 0x100);
    let cursor = alloc(a, b, 4);
    let cells = alloc(a, b, 4);
    let w = alloc(a, b, 0x100);
    let s = stand_in8(a, b, 0xa00, &logger_returning(cursor, 2, cells));
    redirect(a, b, 0x0078_f8f0, s);
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| eax(m, sym, args);
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let at = m.rebase(sym);
        m.call(at, Conv::Cdecl, args).unwrap().st0.unwrap()
    };
    let consts: Vec<u32> = [
        0x00b4_1d90u32,
        0x00b4_ae1c,
        0x00b4_bf08,
        0x00b5_ef64,
        0x00b5_f344,
        0x00b5_f384,
        0x00b5_f388,
        0x00b5_f3d8,
        0x00b5_f3fc,
        0x00b5_f400,
        0x00b5_f404,
        0x00b5_f41c,
        0x00b5_f424,
        0x00b5_f428,
    ]
    .iter()
    .map(|&c| a.rebase(c))
    .collect();
    let doubles = [a.rebase(0x00b5_f230), a.rebase(0x00b5_f2b8)];
    let originals: Vec<u32> = consts.iter().map(|&c| Mem::read(a, c)).collect();
    let original_doubles: Vec<f64> = doubles.iter().map(|&c| Mem::read(a, c)).collect();
    for k in 0..3000u32 {
        put(a, b, cursor, log);
        put(a, b, cells, rng.next());
        let patch = rng.next().is_multiple_of(2);
        for (&c, &v) in consts.iter().zip(&originals) {
            let x = if patch {
                match rng.next() % 4 {
                    0 => (rng.next() % 4000) as f32 / 7.0 - 100.0,
                    1 => ((rng.next() % 2000) as f32 - 1000.0) * 3e7,
                    2 => f32::from_bits(v),
                    _ => (rng.next() % 2000) as f32,
                }
            } else {
                f32::from_bits(v)
            };
            put(a, b, c, x);
        }
        for (&c, &v) in doubles.iter().zip(&original_doubles) {
            let x = if patch && rng.next().is_multiple_of(2) {
                // -0.0 + 0.0 is +0.0
                [-0.0, f64::from(rng.next() % 100_000) / 9.0 - 500.0][(rng.next() % 2) as usize]
            } else {
                v
            };
            put(a, b, c, x);
        }
        let words: [u32; 0x40] = std::array::from_fn(|_| rng.next());
        put(a, b, w, words);
        let kind = if rng.next().is_multiple_of(8) {
            rng.next() as i16
        } else {
            (rng.next() % 26) as i16 - 2
        };
        put(a, b, w + 6, kind);
        put(a, b, a.rebase(0x025d_0af8), rng.next());
        let ra = eax(a, 0x008a_10d0, &[w]);
        let rb = t::to_btl_set_window_layout(b, w, call, fcall);
        assert_eq!(ra, rb, "window layout {k}: eax");
        same_memory(&format!("window layout {k}"), a, b);
        settle(a, b);
    }
    for (&c, &v) in consts.iter().zip(&originals) {
        put(a, b, c, v);
    }
    for (&c, &v) in doubles.iter().zip(&original_doubles) {
        put(a, b, c, v);
    }
}

#[test]
fn small1_window_layout_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    small1_window_layout(&mut a, &mut b, &mut Xorshift(0x5a11_0d01));
}

#[test]
fn small1_window_layout_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    small1_window_layout(&mut a, &mut b, &mut Xorshift(0x5a11_0d02));
}

/// Small round 2 d: `TOWaitBtlWinMain`, `TOBtlOpenLibraWindow` and
/// `TOMenuAtelStart`, their callees logging stand-ins.
#[allow(clippy::cast_possible_truncation, clippy::similar_names)]
fn small2_d(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let cell = alloc(a, b, 4);
    for (k, (sym, n)) in [
        (btl::bridge::PLAY_CLICK, 1u8),
        (btl::bridge::GET_MAIN_PAD, 1),
        (tolib::bridge::SET_LIBRA_WINDOW_PARAM, 1),
        (tolib::bridge::ATEL_COMMON_SET_UP, 6),
    ]
    .into_iter()
    .enumerate()
    {
        // each stand-in logs its own tag first, so a swapped callee shows
        let mut v = vec![0x8b, 0x0d];
        v.extend_from_slice(&cursor.to_le_bytes());
        v.extend_from_slice(&[0xc7, 0x01]);
        v.extend_from_slice(&(0x5100 + k as u32).to_le_bytes());
        v.extend_from_slice(&[0x83, 0x05]);
        v.extend_from_slice(&cursor.to_le_bytes());
        v.push(4);
        v.extend(logger_returning(cursor, n, cell));
        let s = stand_in8(a, b, 0x100 * k as u32, &v);
        redirect(a, b, sym, s);
    }
    let call = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    let w = alloc(a, b, 0x100);
    let opens = [
        0.45f32,
        f32::from_bits(0x3ee6_6665),
        f32::from_bits(0x3ee6_6667),
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::NAN,
        f32::INFINITY,
    ];
    for k in 0..200u32 {
        put(a, b, cursor, log);
        put(a, b, cell, rng.next());
        match k % 3 {
            0 => {
                set(
                    a,
                    b,
                    btl::bridge::INIT_ANM,
                    [0u8, 1, 2, 0x81][(rng.next() % 4) as usize],
                );
                let open = opens[(rng.next() % 9) as usize];
                put(a, b, w + 0xe0, open);
                let ra = eax(a, 0x008a_8a70, &[w]);
                let rb = btl::bridge::to_wait_btl_win_main(b, w, &mut |m, f, x| call(m, f, x));
                assert_eq!(ra, rb, "TOWaitBtlWinMain {k}");
            }
            1 => {
                let chr = rng.next();
                set(a, b, tolib::bridge::LIBRA_STATE, rng.next() as i16);
                let ra = eax(a, 0x0089_8bd0, &[chr]);
                let rb =
                    tolib::bridge::to_btl_open_libra_window(b, chr, &mut |m, f, x| call(m, f, x));
                assert_eq!(ra, rb, "TOBtlOpenLibraWindow {k}");
            }
            _ => {
                set(a, b, tolib::bridge::MENU_ATEL_ACTIVE, rng.next() as u8);
                let ra = eax(a, 0x008b_e3a0, &[]);
                let rb = tolib::bridge::to_menu_atel_start(b, &mut |m, f, x| call(m, f, x));
                assert_eq!(ra, rb, "TOMenuAtelStart {k}");
            }
        }
        same_memory(&format!("small2 d {k}"), a, b);
    }
}

#[test]
fn small2_d_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    small2_d(&mut a, &mut b, &mut Xorshift(0x5a11_2d01));
}

#[test]
fn small2_d_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    small2_d(&mut a, &mut b, &mut Xorshift(0x5a11_2d02));
}

/// Logging stand-ins for `(sym, args)`: each logs its tag (0x5500 + its
/// index) and arguments at `*cursor` and returns the word in its own
/// cell. Returns the log, the cursor and the cells.
#[allow(clippy::many_single_char_names)] // a, b: the oracles
fn small5_stubs(a: &mut Oracle, b: &mut Oracle, list: &[(u32, u8)]) -> (u32, u32, Vec<u32>) {
    let log = alloc(a, b, 0x4000);
    let cursor = alloc(a, b, 4);
    let mut cells = vec![];
    for (k, &(sym, n)) in list.iter().enumerate() {
        let k = u32::try_from(k).unwrap();
        let cell = alloc(a, b, 8);
        let mut v = vec![0x8b, 0x0d];
        v.extend_from_slice(&cursor.to_le_bytes());
        v.extend_from_slice(&[0xc7, 0x01]);
        v.extend_from_slice(&(0x5500 + k).to_le_bytes());
        v.extend_from_slice(&[0x83, 0x05]);
        v.extend_from_slice(&cursor.to_le_bytes());
        v.push(4);
        v.extend(logger_returning(cursor, n, cell));
        // mov edx, [cell + 4] before the ret: a 64-bit result's high half
        v.pop();
        v.extend_from_slice(&[0x8b, 0x15]);
        v.extend_from_slice(&(cell + 4).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0x80 * k, &v);
        redirect(a, b, sym, s);
        cells.push(cell);
    }
    (log, cursor, cells)
}

/// Small round 5 c: `CdReadSync_0076c490`, `TOWaitBtlWinTarget_008a8be0`,
/// `TOMenuFirstReadProc_008a8c10` (every step, polls done or not) and
/// `Pause_Enter`, every callee a logging stand-in.
#[allow(clippy::too_many_lines)] // one block per function
fn small5_c(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::{cd, tolib};
    // callees by their ffx.sym address, as the hook's call_sym
    let call = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = m.rebase(f);
        m.call(f, Conv::Cdecl, args).unwrap().eax
    };
    let call64 = |m: &mut Oracle, f: u32| {
        let f = m.rebase(f);
        let r = m.call(f, Conv::Cdecl, &[]).unwrap();
        (u64::from(r.edx) << 32) | u64::from(r.eax)
    };
    let mut list: Vec<(u32, u8)> = vec![];
    list.extend(
        cd::bridge::READ_SYNC_CALLS
            .iter()
            .zip([0u8, 0, 1, 0, 1, 3])
            .map(|(&f, n)| (f, n)),
    );
    list.push((0x0089_d390, 1));
    list.extend(
        tolib::bridge::MENU_FIRST_READ_CALLS
            .iter()
            .zip([0u8, 0, 1, 3, 0, 2, 0, 3])
            .map(|(&f, n)| (f, n)),
    );
    list.extend(
        tolib::bridge::PAUSE_ENTER_CALLS
            .iter()
            .zip([0u8, 1, 1, 0, 0, 0, 0])
            .map(|(&f, n)| (f, n)),
    );
    let (log, cursor, cells) = small5_stubs(a, b, &list);
    let channels = cd::bridge::CHANNELS.ptr(&*b).addr;
    let task = alloc(a, b, 0x40);
    let pick = |rng: &mut Xorshift| [0u32, 0, 1, 2, rng.next()][(rng.next() % 5) as usize];
    for k in 0..600u32 {
        put(a, b, cursor, log);
        for &c in &cells {
            let (lo, hi) = (pick(rng), rng.next());
            put(a, b, c, lo);
            put(a, b, c + 4, hi);
        }
        match k % 4 {
            0 => {
                for ch in 0..64 {
                    put(a, b, channels + ch * 0x38 + 0x31, 0u8);
                }
                if !rng.next().is_multiple_of(3) {
                    let ch = rng.next() % 64;
                    put(
                        a,
                        b,
                        channels + ch * 0x38 + 0x31,
                        u8::try_from(rng.next() % 255 + 1).unwrap(),
                    );
                }
                set(a, b, cd::bridge::READ_SEMA, rng.next());
                let mode = [0u32, 1, 1, 2, u32::MAX][(rng.next() % 5) as usize];
                let ra = eax(a, 0x0076_c490, &[mode]);
                let rb = cd::bridge::read_sync(b, mode, call);
                assert_eq!(ra, rb, "CdReadSync {k} mode {mode}");
            }
            1 => {
                let w = rng.next();
                let ra = eax(a, 0x008a_8be0, &[w]);
                let rb = tolib::bridge::wait_btl_win_target(b, w, call);
                assert_eq!(ra, rb, "TOWaitBtlWinTarget {k}");
            }
            2 => {
                let step = if rng.next().is_multiple_of(8) {
                    rng.next()
                } else if rng.next().is_multiple_of(4) {
                    1
                } else {
                    rng.next() % 0x1a
                };
                put(a, b, task + 0x1c, step);
                let mode = [
                    0x0080_0000u32,
                    0x0080_ffff,
                    0x0081_0000,
                    0x0180_0000,
                    rng.next(),
                ][(rng.next() % 5) as usize];
                set(a, b, tolib::bridge::MN2_MENU_MODE, mode);
                set(a, b, tolib::bridge::MENU_MAIN_ACTIVE_MASK, rng.next());
                set(
                    a,
                    b,
                    tolib::bridge::MENU_FIRST_READ_END,
                    rng.next().to_le_bytes()[0],
                );
                let ra = eax(a, 0x008a_8c10, &[task]);
                let rb = tolib::bridge::menu_first_read_proc(b, task, call);
                assert_eq!(ra, rb, "TOMenuFirstReadProc {k} step {step:#x}");
            }
            _ => {
                set(a, b, tolib::bridge::MOVIE_PICTURE_ON, pick(rng));
                for g in [
                    tolib::bridge::PAUSE_ON,
                    tolib::bridge::PAUSE_FADE,
                    tolib::bridge::PAUSE_FADE_STEP,
                    tolib::bridge::PAUSE_ENTERED,
                ] {
                    set(a, b, g, rng.next());
                }
                let ra = eax(a, 0x008a_b3d0, &[]);
                let rb = tolib::bridge::pause_enter(b, call, call64);
                assert_eq!(ra, rb, "Pause_Enter {k}");
            }
        }
        same_memory(&format!("small5 c {k}"), a, b);
        settle(a, b);
    }
}

#[test]
fn small5_c_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    small5_c(&mut a, &mut b, &mut Xorshift(0x5a11_5c01));
}

#[test]
fn small5_c_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    small5_c(&mut a, &mut b, &mut Xorshift(0x5a11_5c02));
}

/// Small round 5 e: `TOBtlDrawInfoWinElementLamp` and
/// `TOBtlDrawScanWinElementLamp` (the same code) for elements 0..=3 and
/// others, every flag combination; the lamp flags and the shape maker are
/// logging stand-ins (the shape returns the log position), the two remaps
/// return their argument times a per-case factor (0, negative, NaN and
/// infinity included).
#[allow(clippy::many_single_char_names)] // a, b: the oracles; f, k, s, v
fn small5_e(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::tolib;
    let call = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = m.rebase(f);
        m.call(f, Conv::Cdecl, args).unwrap().eax
    };
    let fcall = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = m.rebase(f);
        m.call_f64(f, args).unwrap()
    };
    let (log, cursor, cells) = small5_stubs(a, b, &[(0x0089_7650, 2)]);
    // the shape maker: its tag, nine words, and the log position in EAX
    let mut v = vec![0x8b, 0x0d];
    v.extend_from_slice(&cursor.to_le_bytes());
    v.extend_from_slice(&[0xc7, 0x01, 0x5f, 0x55, 0, 0, 0x83, 0x05]);
    v.extend_from_slice(&cursor.to_le_bytes());
    v.push(4);
    v.extend(logger_returning(cursor, 9, cursor));
    let s = stand_in8(a, b, 0x200, &v);
    redirect(a, b, 0x0090_3bb0, s);
    // the remaps: fld dword [esp+4]; fmul dword [factor]; ret
    let factor = alloc(a, b, 8);
    for (k, f) in [0x0064_47e0u32, 0x0064_4820].into_iter().enumerate() {
        let mut v = vec![0xd9, 0x44, 0x24, 0x04, 0xd8, 0x0d];
        v.extend_from_slice(&(factor + 4 * u32::try_from(k).unwrap()).to_le_bytes());
        v.push(0xc3);
        let s = stand_in8(a, b, 0x300 + 0x20 * u32::try_from(k).unwrap(), &v);
        redirect(a, b, f, s);
    }
    let factors = [
        1.0f32,
        2.25,
        0.5,
        -1.5,
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NAN,
        1.0e-40,
        3.0e38,
    ];
    for k in 0..400u32 {
        put(a, b, cursor, log);
        put(a, b, cells[0], rng.next());
        for j in 0..2 {
            let f = if k % 4 == 0 {
                f32::from_bits(rng.next() | 0x0040_0000)
            } else {
                factors[(rng.next() % 10) as usize]
            };
            put(a, b, factor + 4 * j, f);
        }
        let element = if k % 8 == 7 {
            rng.next()
        } else {
            rng.next() % 5
        };
        let pos = |rng: &mut Xorshift| match rng.next() % 4 {
            0 => rng.next(),
            1 => 0x0100_0001,
            _ => rng.next() % 0x400,
        };
        let args = [rng.next(), element, pos(rng), pos(rng)];
        let sym = if k % 2 == 0 { 0x0089_4b40 } else { 0x0089_5ae0 };
        let ra = eax(a, sym, &args);
        let rb = tolib::bridge::btl_draw_win_element_lamp(b, args, call, fcall);
        assert_eq!(ra, rb, "element lamp {sym:#x} {k} {args:x?}");
        same_memory(&format!("small5 e {k}"), a, b);
        settle(a, b);
    }
}

#[test]
fn small5_e_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    small5_e(&mut a, &mut b, &mut Xorshift(0x5a11_5e01));
}

#[test]
fn small5_e_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    small5_e(&mut a, &mut b, &mut Xorshift(0x5a11_5e02));
}

/// Small round 5 f: `TOControlMsSetMenuCommand` over random levels
/// (commands 0xffff/0xfffe/0xff mixed in, flags set or not, depths -1 to
/// 20, battle info status 0x100 or not) and `TOSetBtlActiveChr` (status 1
/// or not, a window open or not); every callee a logging stand-in.
#[allow(clippy::many_single_char_names)] // a, b: the oracles
fn small5_f(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::tolib::bridge as t;
    let call = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = m.rebase(f);
        m.call(f, Conv::Cdecl, args).unwrap().eax
    };
    let (log, cursor, cells) = small5_stubs(
        a,
        b,
        &[
            (0x0079_2e50, 1),
            (0x0089_ece0, 2),
            (0x0078_16f0, 0),
            (0x008a_0cd0, 0),
            (0x008a_0ce0, 0),
            (0x0089_79d0, 0),
            (0x0089_8010, 0),
            (0x0089_7bd0, 0),
            (0x0089_7f00, 0),
            (0x0089_a330, 2),
        ],
    );
    let levels = a.rebase(t::BTL_LEVELS);
    let rec = a.rebase(t::BTL_COMMAND_REC);
    for k in 0..600u32 {
        put(a, b, cursor, log);
        for &c in &cells {
            put(a, b, c, rng.next());
        }
        // every byte the two touch random first, so a narrower or missing
        // store shows
        let mut fill = |a: &mut Oracle, b: &mut Oracle, at: u32, n: u32| {
            for j in 0..n {
                put(a, b, at + j, rng.next().to_le_bytes()[0]);
            }
        };
        fill(a, b, levels - 0xf0, 0x790 + 0xf0);
        fill(a, b, levels + 0xff * 0xf0, 0x10);
        for at in [0x0133_d740u32, 0x0133_c938, 0x023c_c030, 0x0133_f0f8] {
            fill(a, b, a.rebase(at), 0x38);
        }
        fill(a, b, a.rebase(0x0133_f508), 0xc0);
        for i in 0..21 {
            let l = levels + 0xf0 * i;
            let cmd = match rng.next() % 6 {
                0 => 0xffff,
                1 => 0xfffe,
                2 => 0xff,
                _ => rng.next() & 0xffff,
            };
            put(a, b, l + 0x16, u16::try_from(cmd).unwrap());
            put(a, b, l + 2, u8::from(rng.next().is_multiple_of(3)));
            put(a, b, l + 8, rng.next().to_le_bytes()[0]);
            put(a, b, l + 0xd8, rng.next());
        }
        for j in 0..0x48 {
            put(a, b, rec + j, rng.next().to_le_bytes()[0]);
        }
        if k % 2 == 0 {
            let depth = [-1i8, 0, 1, 2, 3, 5, 7, 20][(rng.next() % 8) as usize];
            set(a, b, t::BTL_ACTION_DEPTH, depth);
            let status = match rng.next() % 6 {
                0 | 1 => 0x100u32,
                2 => 0x0001_0100,
                _ => rng.next() % 0x102,
            };
            put(a, b, a.rebase(0x0133_c94c), status);
            put(a, b, a.rebase(0x0133_cb46), rng.next());
            put(a, b, a.rebase(0x0133_d178), rng.next());
            let ra = eax(a, 0x0089_afc0, &[]);
            let rb = t::to_control_ms_set_menu_command(b, call);
            assert_eq!(ra, rb, "TOControlMsSetMenuCommand {k} depth {depth}");
        } else {
            put(a, b, cells[2], [1u32, 1, 0, 2][(rng.next() % 4) as usize]);
            let open = match rng.next() % 6 {
                0 => rng.next(),
                1 => 0x0001_0000 << (rng.next() % 16),
                _ => 0,
            };
            put(a, b, a.rebase(0x023c_c08c), open);
            let chr = if rng.next().is_multiple_of(4) {
                rng.next()
            } else {
                rng.next() % 0x12
            };
            let ra = eax(a, 0x0089_bb10, &[chr]);
            let rb = t::to_set_btl_active_chr(b, chr, call);
            assert_eq!(ra, rb, "TOSetBtlActiveChr {k} {chr:#x}");
        }
        same_memory(&format!("small5 f {k}"), a, b);
        settle(a, b);
    }
}

#[test]
fn small5_f_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    small5_f(&mut a, &mut b, &mut Xorshift(0x5a11_5f01));
}

#[test]
fn small5_f_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    small5_f(&mut a, &mut b, &mut Xorshift(0x5a11_5f02));
}

/// Small round 5 h: `Pause_Update` (pausing or not, entered or not, Start
/// pressed at 0x40 or not, fades and steps around 0x40 and 0x80, the end
/// with or without a movie); every callee a logging stand-in, the time
/// 64 bits.
#[allow(clippy::many_single_char_names)] // a, b: the oracles
fn small5_h(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::tolib::bridge as t;
    let call = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = m.rebase(f);
        m.call(f, Conv::Cdecl, args).unwrap().eax
    };
    let call64 = |m: &mut Oracle, f: u32| {
        let f = m.rebase(f);
        let r = m.call(f, Conv::Cdecl, &[]).unwrap();
        (u64::from(r.edx) << 32) | u64::from(r.eax)
    };
    let mut list: Vec<(u32, u8)> = t::PAUSE_UPDATE_CALLS
        .iter()
        .zip([0u8, 0, 1, 0, 0])
        .map(|(&f, n)| (f, n))
        .collect();
    list.extend(
        t::PAUSE_ENTER_CALLS
            .iter()
            .zip([0u8, 1, 1, 0, 0, 0, 0])
            .map(|(&f, n)| (f, n)),
    );
    let (log, cursor, cells) = small5_stubs(a, b, &list);
    let ctrl = alloc(a, b, 0x220);
    let fades = [0x40u32, 0x40, 0x3c, 0x44, 0x7c, 0x80, 0x84, 0x7f, 0];
    let steps = [4u32, 0xffff_fffc, 0, 1, 0x40, 0x8000_0000];
    for k in 0..600u32 {
        put(a, b, cursor, log);
        for &c in &cells {
            put(a, b, c, rng.next());
            put(a, b, c + 4, rng.next());
        }
        put(a, b, cells[0], ctrl);
        put(a, b, cells[2], [0u32, 1, 0x800][(rng.next() % 3) as usize]);
        put(a, b, cells[10], [0u32, 0, 1][(rng.next() % 3) as usize]);
        put(
            a,
            b,
            ctrl + 0x210,
            rng.next().to_le_bytes()[0] | (0x10 * u8::from(k % 5 != 0)),
        );
        set(
            a,
            b,
            t::PAUSE_ENTERED,
            [0u32, 1, 2][(rng.next() % 3) as usize],
        );
        let fade = if k % 7 == 0 {
            rng.next()
        } else {
            fades[(rng.next() % 9) as usize]
        };
        set(a, b, t::PAUSE_FADE, fade);
        set(a, b, t::PAUSE_FADE_STEP, steps[(rng.next() % 6) as usize]);
        set(
            a,
            b,
            t::MOVIE_PICTURE_ON,
            [0u32, 0, 1][(rng.next() % 3) as usize],
        );
        for g in [t::PAUSE_ON, t::PAUSE_REQUEST] {
            set(a, b, g, rng.next());
        }
        for at in [t::PAUSE_TIME, t::PAUSE_TOTAL] {
            let at = a.rebase(at);
            put(a, b, at, rng.next());
            put(a, b, at + 4, rng.next());
        }
        let ra = eax(a, 0x008a_b340, &[]);
        let rb = t::pause_update(b, call, call64);
        assert_eq!(ra, rb, "Pause_Update {k} fade {fade:#x}");
        same_memory(&format!("small5 h {k}"), a, b);
        settle(a, b);
    }
}

#[test]
fn small5_h_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    small5_h(&mut a, &mut b, &mut Xorshift(0x5a11_5801));
}

#[test]
fn small5_h_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    small5_h(&mut a, &mut b, &mut Xorshift(0x5a11_5802));
}

/// A key script for `small6_a`: statement 0 holds (a stop), then random
/// statements, then the end (opcode 0). Every loop (opcode 8) has a stop
/// between its mark and itself, so no call spins. Returns the words and,
/// per statement start, the loop mark in force there.
fn small6_script(rng: &mut Xorshift, lens: &[u8; 11]) -> (Vec<u16>, Vec<(u16, u16)>) {
    let mut words: Vec<u16> = vec![];
    let mut starts = vec![];
    let mut mark = 0u16;
    let mut stop_since_mark = false;
    let n = 3 + rng.next() % 14;
    for k in 0..=n {
        let at = u16::try_from(words.len()).unwrap();
        starts.push((at, mark));
        let pick = [1u16, 2, 3, 4, 5, 6, 7, 8, 9, 10, 8, 3][(rng.next() % 12) as usize];
        let mut op = match k {
            0 => 4,
            _ if k == n => 0,
            _ => pick,
        };
        if op == 8 && !stop_since_mark {
            op = 1;
        }
        let hi = u16::try_from(rng.next() & 0xf000).unwrap();
        let mut head = op | hi;
        let operand: u16 = match op {
            3..=7 | 10 => {
                let v = if k == 0 {
                    1 + rng.next() % 3
                } else {
                    [0, 0, 1, 2, 3][(rng.next() % 5) as usize]
                };
                if v != 0 {
                    stop_since_mark = true;
                }
                u16::try_from(v).unwrap()
            }
            8 => {
                if rng.next().is_multiple_of(4) {
                    head |= 0x800;
                }
                [0xfffe, 0xffff, 0, 1, 2, 3, 4][(rng.next() % 7) as usize]
            }
            2 => [0u16, 1, 2, 3, 0xffff, 0x8000][(rng.next() % 6) as usize],
            _ => u16::try_from(rng.next() & 0xffff).unwrap(),
        };
        let len = usize::from(lens[usize::from(op)]);
        words.push(head);
        words.push(operand);
        for _ in 2..len {
            words.push(u16::try_from(rng.next() & 0xffff).unwrap());
        }
        if op == 9 {
            mark = u16::try_from(words.len()).unwrap();
            stop_since_mark = false;
        }
    }
    (words, starts)
}

/// Small round 6 a: `MenuScene_TrackStep_00a7b510` over random key scripts
/// (random operand lengths in `0x00c89c40`, random flags, waits, list
/// entries and loop state), stepping each track up to 24 frames;
/// `MenuScene_KeyPick` is a logging stand-in.
fn small6_a(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let call = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = m.rebase(f);
        m.call(f, Conv::Cdecl, args).unwrap().eax
    };
    let (log, cursor, cells) = small5_stubs(a, b, &[(0x00a7_b290, 5)]);
    let base = alloc(a, b, 0x2000);
    let table = alloc(a, b, 0x20);
    let list = alloc(a, b, 0x200);
    let track = alloc(a, b, 0x20);
    let ops = a.rebase(scene::bridge::KEY_OP_WORDS);
    let mut lens = [2u8, 2, 2, 2, 10, 10, 12, 12, 2, 2, 16];
    for k in 0..300u32 {
        if k % 3 == 1 {
            for l in &mut lens {
                *l = u8::try_from(2 + rng.next() % 15).unwrap();
            }
        } else if k % 3 == 2 {
            lens = [2, 2, 2, 2, 10, 10, 12, 12, 2, 2, 16];
        }
        put(a, b, ops, lens);
        let key = rng.next() % 4;
        let (words, starts) = small6_script(rng, &lens);
        let off = 0x10 + 0x800 * key + 2 * (rng.next() % 8);
        for (i, w) in words.iter().enumerate() {
            put(a, b, base + off + 2 * u32::try_from(i).unwrap(), *w);
        }
        for s in 0..4 {
            put(a, b, table + 4 * s, if s == key { off } else { rng.next() });
        }
        let count = [0i32, 1, 2, 5, 8, -1][(rng.next() % 6) as usize];
        put(a, b, list, count);
        for e in 0..12 {
            let id = [0u32, 1, 2, 3, 0xffff, 0x8000, rng.next()][(rng.next() % 7) as usize];
            let flag = if rng.next().is_multiple_of(3) {
                0x8000_0000
            } else {
                0
            };
            put(a, b, list + 4 + 20 * e, id & 0x7fff_ffff | flag);
        }
        let (pc, mark) = starts[(rng.next() as usize) % starts.len()];
        put(a, b, track, key | rng.next() & 0xc000_0000);
        let wait = if rng.next().is_multiple_of(4) {
            (rng.next() % 3) as u16
        } else {
            0
        };
        put(a, b, track + 4, wait);
        put(
            a,
            b,
            track + 6,
            [0u16, 1, 5, 0xffff, 0x8000][(rng.next() % 5) as usize],
        );
        put(a, b, track + 8, pc);
        put(a, b, track + 0xa, (rng.next() & 0xffff) as u16);
        put(
            a,
            b,
            track + 0xc,
            [0xffffu16, 0, 1, 3, 7, 0x8000][(rng.next() % 6) as usize],
        );
        put(a, b, track + 0xe, mark);
        put(
            a,
            b,
            track + 0x10,
            [0u16, 1, 2, 0xffff][(rng.next() % 4) as usize],
        );
        let out = rng.next();
        for frame in 0..24 {
            put(a, b, cursor, log);
            put(a, b, cells[0], rng.next());
            if rng.next().is_multiple_of(5) {
                let e = rng.next() % 12;
                put(a, b, list + 4 + 20 * e, rng.next());
            }
            let args = [base, list, track, table, out];
            let ra = eax(a, 0x00a7_b510, &args);
            let rb = scene::bridge::menu_scene_track_step(b, args, call);
            assert_eq!(ra, rb, "TrackStep {k} frame {frame}");
            same_memory(&format!("small6 a {k} frame {frame}"), a, b);
            settle(a, b);
            if ra == 0 && rng.next().is_multiple_of(2) {
                break;
            }
        }
    }
}

#[test]
fn small6_a_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    small6_a(&mut a, &mut b, &mut Xorshift(0x5a11_6a01));
}

#[test]
fn small6_a_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    small6_a(&mut a, &mut b, &mut Xorshift(0x5a11_6a02));
}

/// Logging stand-ins for `fs` (`(sym, words)`), each at new code behind a
/// jump: it appends its index and its first `words` arguments at
/// `*cursor` and returns the word at its cell, which then grows by the
/// cell's `+4` (0 for a constant). Returns `(log, cursor, cells)`.
fn small6_stubs(a: &mut Oracle, b: &mut Oracle, fs: &[(u32, u8)]) -> (u32, u32, Vec<u32>) {
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    put(a, b, cursor, log);
    let mut cells = vec![];
    for (k, &(sym, n)) in fs.iter().enumerate() {
        let cell = alloc(a, b, 8);
        let c = cursor.to_le_bytes();
        let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &c].concat(); // mov ecx, [cursor]
        code.extend([0xc7, 0x01]); // mov dword [ecx], k
        code.extend((0x6600 + u32::try_from(k).unwrap()).to_le_bytes());
        for j in 0..n {
            code.extend([0x8b, 0x44, 0x24, 4 + 4 * j, 0x89, 0x41, 4 + 4 * j]);
        }
        code.extend([&[0x83, 0x05][..], &c, &[4 + 4 * n]].concat()); // add dword [cursor], 4 + 4n
        code.push(0xa1); // mov eax, [cell]
        code.extend(cell.to_le_bytes());
        code.extend([0x8b, 0x15]); // mov edx, [cell + 4]
        code.extend((cell + 4).to_le_bytes());
        code.extend([0x01, 0x15]); // add [cell], edx
        code.extend(cell.to_le_bytes());
        code.push(0xc3);
        let at = a.alloc_code(&code).unwrap();
        assert_eq!(b.alloc_code(&code).unwrap(), at);
        redirect(a, b, sym, at);
        cells.push(cell);
    }
    (log, cursor, cells)
}

/// Small round 6 f: `TOInitBtlWindow` (the SE playing or not) and `TOInit`
/// (every language path: 0, 1, 9, 10, 0x11, others; the localisation
/// word 0, 3 and others; "newkit" found or not); every callee a logging
/// stand-in, the file loads counting up.
#[allow(clippy::too_many_lines)] // two functions
fn small6_f(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let call = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = m.rebase(f);
        m.call(f, Conv::Cdecl, args).unwrap().eax
    };
    let win: &[(u32, u8)] = &[
        (0x008a_dbd0, 0),
        (0x0089_7860, 0),
        (0x008a_e410, 3),
        (0x008a_f5c0, 1),
        (0x0089_79d0, 0),
        (0x0089_8010, 0),
        (0x0089_7af0, 0),
        (0x0089_7bd0, 0),
        (0x008a_e450, 4),
        (0x0088_6680, 1),
        (0x0088_6eb0, 1),
        (0x008a_e5a0, 1),
    ];
    let init: &[(u32, u8)] = &[
        (0x0064_1100, 0),
        (0x008a_d900, 0),
        (0x008b_3a40, 0),
        (0x008f_ae20, 0),
        (0x008a_c2f0, 0),
        (0x0062_f500, 1),
        (0x008a_ba60, 2),
        (0x0088_c860, 1),
        (0x008a_c130, 1),
        (0x0068_7010, 1),
        (0x0064_22e0, 1),
        (0x008a_ba30, 3),
        (0x0090_6b20, 0),
        (0x0088_ccf0, 2),
        (0x008a_df20, 0),
        (0x00a7_bb00, 0),
        (0x00a7_c7f0, 2),
        (0x008a_e6f0, 0),
        (0x008b_85c0, 0),
        (0x008a_da30, 1),
        (0x008b_81d0, 1),
        (0x0089_dc40, 0),
        (0x008b_e800, 0),
        (0x0089_77f0, 0),
    ];
    let mut fs = win.to_vec();
    fs.extend(init);
    let (log, cursor, cells) = small6_stubs(a, b, &fs);
    let cell_of = |sym: u32| cells[fs.iter().position(|f| f.0 == sym).unwrap()];
    for k in 0..48u32 {
        put(a, b, cursor, log);
        for &c in &cells {
            put(a, b, c, rng.next());
            put(a, b, c + 4, 0u32);
        }
        // every global the window reset writes, random first
        let block = a.rebase(0x0133_c93c);
        let bytes: Vec<u8> = (0..0x2ec4).map(|_| rng.next().to_le_bytes()[0]).collect();
        a.write(block, &bytes).unwrap();
        b.write(block, &bytes).unwrap();
        for g in [
            0x023c_c088u32,
            0x023c_c08c,
            0x00c5_8ef4,
            0x0186_59bc,
            0x0184_1e0c,
        ] {
            let at = a.rebase(g);
            put(a, b, at, rng.next());
        }
        let sep = cell_of(0x0088_6680);
        put(a, b, sep, u32::from(rng.next().is_multiple_of(2)));
        let ra = eax(a, 0x0089_b660, &[]);
        let rb = tolib::bridge::init_btl_window(b, call);
        assert_eq!(ra, rb, "TOInitBtlWindow {k}");
        same_memory(&format!("small6 f window {k}"), a, b);
        settle(a, b);

        put(a, b, cursor, log);
        let lang = [0u32, 1, 9, 10, 0x11, 2, 5][(k % 7) as usize];
        put(a, b, cell_of(0x008a_c2f0), lang);
        put(
            a,
            b,
            cell_of(0x0064_1100),
            [0u32, 3, 1, 7][((k / 7) % 4) as usize],
        );
        put(a, b, cell_of(0x008a_ba60) + 4, 0x100u32);
        let kit = if rng.next().is_multiple_of(3) {
            0
        } else {
            rng.next() | 1
        };
        put(a, b, cell_of(0x0088_c860), kit);
        put(a, b, cell_of(0x0088_c860) + 4, 0u32);
        for g in [0x00c5_9558u32, 0x0184_1d20, 0x0184_1d43, 0x025d_09bc] {
            let at = a.rebase(g);
            put(a, b, at, rng.next());
        }
        let ra = eax(a, 0x008a_d4d0, &[]);
        let rb = tolib::bridge::to_init(b, call);
        assert_eq!(ra, rb, "TOInit {k} lang {lang}");
        same_memory(&format!("small6 f init {k}"), a, b);
        settle(a, b);
    }
}

#[test]
fn small6_f_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    small6_f(&mut a, &mut b, &mut Xorshift(0x5a11_6f01));
}

#[test]
fn small6_f_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    small6_f(&mut a, &mut b, &mut Xorshift(0x5a11_6f02));
}

/// pc round 6 e: `MagicFile_PreloadPrx_009dae90` (preloading on or off,
/// table entries matching by magic and file or as wildcards, slots empty,
/// negative or holding another file) and `TOControlBtlWindow` (keyboard
/// flag, battle status, window open and control flags, every level state
/// and callback result). Every callee is a logging stand-in; the C
/// runtime imports point at stand-ins too, `PhyreFIOS_LoadMagicfilePrx`
/// writes its path into the operation word and 7 into the handle, and
/// `graphicGetTime` returns a double from a cell.
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)] // two functions; random bytes
fn pc6_e(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use ffx_system::magic::bridge as mg;
    // the hook's call_sym, call_ptr and fcall
    let call = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = m.rebase(f);
        m.call(f, Conv::Cdecl, args).unwrap().eax
    };
    let call_ptr = |m: &mut Oracle, f: u32, args: &[u32]| m.call(f, Conv::Cdecl, args).unwrap().eax;
    let fcall = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = m.rebase(f);
        m.call_f64(f, args).unwrap()
    };
    // dummies for the imports and the levels' callbacks
    let iob = 0x009d_afd0u32;
    let fprintf = 0x009d_b270u32;
    let cbs = [0x009d_afe0u32, 0x009d_b230, 0x009d_b250, 0x009d_b490];
    let mut list: Vec<(u32, u8)> = vec![
        (mg::ASYNC_LOAD_ENABLE, 1),
        (mg::UNLOAD_PRX, 2),
        (iob, 0),
        (0x008a_e0f0, 1),
        (0x0078_16f0, 0),
    ];
    for f in [
        0x0089_1140u32,
        0x0089_20d0,
        0x0089_1220,
        0x0089_1a30,
        0x0089_1ac0,
        0x0089_0f70,
        0x0089_2320,
        0x0089_1b80,
        0x0063_0de0,
        0x0088_9a80,
        0x0088_93e0,
        0x0089_0f30,
        0x0089_12e0,
        0x0089_1480,
        0x0089_1580,
        0x0089_afc0,
    ] {
        list.push((f, 0));
    }
    for f in cbs {
        list.push((f, 1));
    }
    let (log, cursor, cells) = small5_stubs(a, b, &list);
    let cell_of = |sym: u32| cells[list.iter().position(|&(s, _)| s == sym).unwrap()];
    // fprintf(stream, fmt, ..): 4 words for the "non-null" message, else 5
    let non_null = a.rebase(mg::MSG_NON_NULL);
    let mut code = vec![0x8b, 0x0d]; // mov ecx, [cursor]
    code.extend(cursor.to_le_bytes());
    for k in 0..4u8 {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 * k]);
    }
    code.extend([0x81, 0x7c, 0x24, 0x08]); // cmp dword [esp+8], non_null
    code.extend(non_null.to_le_bytes());
    code.extend([0x74, 0x07]); // je +7
    code.extend([0x8b, 0x44, 0x24, 0x14, 0x89, 0x41, 0x10]);
    code.extend([0x83, 0xc1, 0x14, 0x89, 0x0d]); // add ecx, 0x14; mov [cursor], ecx
    code.extend(cursor.to_le_bytes());
    code.push(0xc3);
    let printer = stand_in8(a, b, 0x80 * 33, &code);
    redirect(a, b, fprintf, printer);
    for (slot, f) in [(mg::IOB_FUNC, iob), (mg::FPRINTF, fprintf)] {
        let at = a.rebase(slot);
        let to = a.rebase(f);
        put(a, b, at, to);
    }
    // PhyreFIOS_LoadMagicfilePrx(path, &handle, &op): *op = path when it
    // is 0, *handle = 7
    let load = stand_in8(
        a,
        b,
        0x80 * 34,
        &[
            0x8b, 0x44, 0x24, 0x04, // mov eax, [esp+4]
            0x8b, 0x4c, 0x24, 0x0c, // mov ecx, [esp+0xc]
            0x83, 0x39, 0x00, // cmp dword [ecx], 0
            0x75, 0x02, // jne +2 (an operation left set stays)
            0x89, 0x01, // mov [ecx], eax
            0x8b, 0x4c, 0x24, 0x08, // mov ecx, [esp+8]
            0xc7, 0x01, 0x07, 0, 0, 0, // mov dword [ecx], 7
            0xc3,
        ],
    );
    redirect(a, b, mg::LOAD_PRX, load);
    // the open callback moves its level's state on by 3 (the original
    // re-reads the state after it)
    let mut code = vec![0x8b, 0x44, 0x24, 0x04, 0x80, 0x40, 0x01, 0x03, 0xa1]; // mov eax, [esp+4]; add byte [eax+1], 3; mov eax, [cell]
    code.extend(cell_of(cbs[0]).to_le_bytes());
    code.push(0xc3);
    let open = stand_in8(a, b, 0x80 * 32, &code);
    redirect(a, b, cbs[0], open);
    // graphicGetTime: fld qword [time]
    let time = alloc(a, b, 8);
    let mut code = vec![0xdd, 0x05];
    code.extend(time.to_le_bytes());
    code.push(0xc3);
    let t = stand_in8(a, b, 0x80 * 35, &code);
    redirect(a, b, 0x0064_1410, t);
    let on = a.rebase(mg::PRELOAD_ON);
    let table = a.rebase(mg::PRELOAD_TABLE);
    let handles = a.rebase(mg::PRELOAD_HANDLES);
    let ops = a.rebase(mg::PRELOAD_OPS);
    let levels = a.rebase(ffx_system::btl::bridge::CMD_LEVELS);
    let word = |rng: &mut Xorshift| {
        [0u32, 1, 2, 3, u32::MAX, 0x8000_0000, rng.next()][(rng.next() % 7) as usize]
    };
    for k in 0..300u32 {
        put(a, b, cursor, log);
        for &c in &cells {
            put(a, b, c, word(rng));
        }
        // MagicFile_PreloadPrx
        put(
            a,
            b,
            on,
            [0u32, 1, 1, 1, rng.next()][(rng.next() % 5) as usize],
        );
        for i in 0..mg::PRELOAD_SLOTS {
            let e = table + i * 0xc;
            put(
                a,
                b,
                e,
                [0u32, 1, 2, u32::MAX, rng.next()][(rng.next() % 5) as usize],
            );
            put(
                a,
                b,
                e + 4,
                [0u32, 1, 2, u32::MAX, rng.next()][(rng.next() % 5) as usize],
            );
            put(
                a,
                b,
                e + 8,
                [0u32, 1, 2, u32::MAX, 0x8000_0000][(rng.next() % 5) as usize],
            );
            put(
                a,
                b,
                handles + i * 4,
                [0u32, 0, 5, u32::MAX, rng.next()][(rng.next() % 5) as usize],
            );
            put(a, b, ops + i * 4, rng.next());
        }
        let magic = [0u32, 1, 2, u32::MAX, rng.next()][(rng.next() % 5) as usize];
        let file = [0u32, 1, 2, u32::MAX, rng.next()][(rng.next() % 5) as usize];
        call(a, 0x009d_ae90, &[magic, file]);
        mg::magic_file_preload_prx(b, magic, file, &mut |m, f, args| call_ptr(m, f, args));
        same_memory(&format!("pc6 e magic {k}"), a, b);
        settle(a, b);
        // TOControlBtlWindow
        put(a, b, cursor, log);
        put(
            a,
            b,
            cell_of(0x0078_16f0),
            [1u32, 1, 1, 0, 2][(rng.next() % 5) as usize],
        );
        for &f in &cbs {
            put(a, b, cell_of(f), [0u32, 1, 1, 2][(rng.next() % 4) as usize]);
        }
        for g in [0x0133_c930u32, 0x023c_c08c, 0x00c5_8ef8] {
            let at = a.rebase(g);
            put(
                a,
                b,
                at,
                [0u32, 1, 1, 1, rng.next()][(rng.next() % 5) as usize],
            );
        }
        let now = f64::from(rng.next() % 100_000) / 7.0;
        put(a, b, time, now.to_bits());
        for i in 0..8 {
            let lv = levels + i * 0xf0;
            put(
                a,
                b,
                lv + 1,
                [0u8, 1, 2, 3, 4, 5, 6, 7, 0xff][(rng.next() % 9) as usize],
            );
            for (c, &f) in cbs.iter().enumerate() {
                let p = if c == 0 && rng.next().is_multiple_of(4) {
                    0
                } else {
                    a.rebase(f)
                };
                put(a, b, lv + 0x94 + 4 * c as u32, p);
            }
            put(
                a,
                b,
                lv + 0xdc,
                (f64::from(rng.next() % 50_000) / 3.0) as f32,
            );
            put(a, b, lv + 0xe0, rng.next());
        }
        put(a, b, a.rebase(0x0133_d6f0), rng.next() as u16);
        for w in 0..8 {
            put(a, b, a.rebase(0x0133_f7b4) + 4 * w, rng.next());
        }
        call(a, 0x0089_ae20, &[]);
        ffx_system::btl::bridge::to_control_btl_window(b, call, call_ptr, fcall);
        same_memory(&format!("pc6 e btl window {k}"), a, b);
        settle(a, b);
    }
}

#[test]
fn pc6_e_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    pc6_e(&mut a, &mut b, &mut Xorshift(0x9c06_e001));
}

#[test]
fn pc6_e_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    pc6_e(&mut a, &mut b, &mut Xorshift(0x9c06_e002));
}

/// pc round 7 a: `TOBtlDrawLibraLeft` over logging stand-ins and the
/// original remaps. Each stand-in logs every stack word its original
/// reads, including the words the original stored for the call before
/// its last pushes (`TOGetMessageWH` 6, `TOMkpCrossExtMesFontLClut` 7,
/// `TOMkpShapeXYWHUV` 9, `TOMkpEasyStrFontS` 3), except the two frame
/// addresses (`[ebp-8]`, `[ebp-4]`), through which the measuring
/// stand-ins store the test's widths; the port's lists must have exactly
/// those counts. Language 0xb, line counts, timers and lines vary.
#[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
fn pc7_a(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let cells = alloc(a, b, 0x10);
    let seq = round38_seq(counter, table);
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let then = |mut v: Vec<u8>, w: &[u8]| {
        v.extend_from_slice(w);
        v
    };
    let stands: Vec<Stand> = vec![
        (0x0079_af10, all(1), vec![], seq.clone()), // MsGetRamChrName
        // TOGetMessageWH(0, name, &w, &h, 0, 0): w = cells[0], h = cells[2]
        (
            0x008b_9650,
            vec![0, 1, 4, 5],
            vec![],
            then(
                then(store_through(2, cells), &store_through(3, cells + 8)),
                &seq,
            ),
        ),
        (0x0090_16b0, all(7), vec![], seq.clone()), // TOMkpCrossExtMesFontLClut
        (0x0090_3bb0, all(9), vec![], seq.clone()), // TOMkpShapeXYWHUV
        (0x008a_c2f0, vec![], vec![], seq.clone()), // TOGetFFXLangJmp
        (0x0079_add0, all(1), vec![], seq.clone()), // MsGetRamChrHP
        (0x0079_adf0, all(1), vec![], seq.clone()), // MsGetRamChrHPmax
        (0x0079_af40, all(1), vec![], seq.clone()), // MsGetRamChrStr
        (0x0079_ae90, all(1), vec![], seq.clone()), // MsGetRamChrMag
        (0x0090_55c0, all(6), vec![], seq.clone()), // ToMakeBtlEasyDigitRight
        (0x0090_5550, all(6), vec![], seq.clone()), // ToMakeBtlEasyDigit2
        (0x0090_2040, all(3), vec![], seq.clone()), // TOMkpEasyStrFontS
        (0x0089_5ae0, all(4), vec![], seq.clone()), // TOBtlDrawScanWinElementLamp
        (0x0078_fcf0, all(3), vec![], seq.clone()), // MsMenuGetText
        // ToGetBtlEasyFontWidth(text, &w, 0, 0.64, 0.5): w = cells[1]
        (
            0x0090_5290,
            vec![0, 2, 3, 4],
            vec![],
            then(store_through(1, cells + 4), &seq),
        ),
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let arity: Vec<(u32, usize)> = stands
        .iter()
        .map(|s| match s.0 {
            0x008b_9650 => (s.0, 6),
            0x0090_5290 => (s.0, 5),
            _ => (s.0, s.1.len()),
        })
        .collect();
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let &(_, n) = arity
            .iter()
            .find(|e| e.0 == sym)
            .unwrap_or_else(|| panic!("{sym:#x}: no stand-in"));
        assert_eq!(args.len(), n, "{sym:#x} words");
        eax(m, sym, args)
    };
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        assert!(sym == 0x0064_47e0 || sym == 0x0064_4820, "{sym:#x}");
        let at = m.rebase(sym);
        m.call_f64(at, args).unwrap()
    };
    let local = b.stack_range().start + 0x100;
    let g = |a: &Oracle, s: u32| a.rebase(s);
    let words = [0xbu32, 0xb, 0, 1, 2, 0x7f, 0xffff, u32::MAX];
    let widths = [0f32, 37.5, 120.0, 900.0, -4.0, 1.0e7];
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        for j in 0..0x40 {
            let v = if rng.next().is_multiple_of(5) {
                rng.next()
            } else {
                words[(rng.next() % words.len() as u32) as usize]
            };
            put(a, b, table + 4 * j, v);
        }
        for j in 0..3 {
            let w = if rng.next().is_multiple_of(4) {
                round38_quiet(rng)
            } else {
                widths[(rng.next() % 6) as usize].to_bits()
            };
            put(a, b, cells + 4 * j, w);
        }
        put(a, b, g(a, 0x0133_f6aa), rng.next() as u16);
        let count = [-1i16, 0, 1, 1, 3, 6, i16::MIN][(rng.next() % 7) as usize];
        put(a, b, g(a, 0x0133_f6b0), count);
        let timer = [0u16, 0, 1, 0x14, 0xffff][(rng.next() % 5) as usize];
        put(a, b, g(a, 0x0133_f6ac), timer);
        let line = [0i16, 1, 2, 5, -1, -3][(rng.next() % 6) as usize];
        put(a, b, g(a, 0x0133_f6ae), line);
        for j in 0..0x10 {
            put(a, b, g(a, 0x0133_f6b2) + 2 * j, rng.next() as u16);
        }
        let x =
            [0u32, 5, 0x100, rng.next() % 1280, rng.next(), 0x8000_0000][(rng.next() % 6) as usize];
        let want = call_raw(a, 0x0089_bf70, &[x]);
        assert!(want.st0.is_none(), "libra left leaves ST0 {k}");
        // the name, 2 frames + 4 rows, 4 captions, 4 values, 4 lamps
        let logged: u32 = Mem::read(a, cursor);
        assert!(logged - log >= 0x180, "libra left logged {k}");
        let got = t::btl_draw_libra_left(b, x.cast_signed(), local, call, fcall);
        assert_eq!(want.eax, got, "libra left {k}");
        same_memory(&format!("pc7 a libra left {k}"), a, b);
        settle(a, b);
    }
}

/// [`call`] as cdecl, the whole result.
fn call_raw(o: &mut Oracle, sym: u32, args: &[u32]) -> ffx_oracle::Ret {
    call(o, sym, Conv::Cdecl, args)
}

#[test]
fn pc7_a_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    pc7_a(&mut a, &mut b, &mut Xorshift(0x9c07_a001));
}

#[test]
fn pc7_a_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    pc7_a(&mut a, &mut b, &mut Xorshift(0x9c07_a002));
}

/// pc round 7 b: `TOBtlDrawInfoWin` over logging stand-ins and the
/// original remaps. Each stand-in logs every stack word its original
/// reads (`TOMkpCrossEasyStrFontSClut` 6 and the second
/// `TOMkpRotateShapeXYWHUVC2` 12 include words the original stored for
/// the call before its last pushes); the port's lists must have exactly
/// those counts. `graphicGetTime` returns floats of a table in turn, so the
/// fade runs below 0, inside, above 1 and NaN; the element flags, the
/// language and the status count vary.
#[allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]
fn pc7_b(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use btl::bridge as t;
    let log = alloc(a, b, 0x8000);
    let cursor = alloc(a, b, 4);
    let counter = alloc(a, b, 4);
    let table = alloc(a, b, 0x400);
    let tcounter = alloc(a, b, 4);
    let times = alloc(a, b, 0x10);
    let seq = round38_seq(counter, table);
    // mov ecx, [tcounter]; fld dword [times + ecx*4]; inc dword [tcounter]
    let mut time = vec![0x8b, 0x0d];
    time.extend_from_slice(&tcounter.to_le_bytes());
    time.extend_from_slice(&[0xd9, 0x04, 0x8d]);
    time.extend_from_slice(&times.to_le_bytes());
    time.extend_from_slice(&[0xff, 0x05]);
    time.extend_from_slice(&tcounter.to_le_bytes());
    let all = |n: u8| (0..n).collect::<Vec<u8>>();
    let stands: Vec<Stand> = vec![
        (0x0064_1410, vec![], vec![], time),         // graphicGetTime
        (0x0079_0240, all(1), vec![], seq.clone()),  // MsMenuWeakElement_00790240
        (0x0078_f910, all(1), vec![], seq.clone()),  // MsMenuAbsElement_0078f910
        (0x0078_fe80, all(1), vec![], seq.clone()),  // MsMenuInvElement
        (0x0078_fe40, all(1), vec![], seq.clone()),  // MsMenuHalfElement
        (0x008f_9230, all(4), vec![], seq.clone()),  // TODrawScissorXYWH
        (0x008f_41b0, all(5), vec![], seq.clone()),  // TODrawBtlNetWindowXYWHType
        (0x008a_c2f0, vec![], vec![], seq.clone()),  // TOGetFFXLangJmp
        (0x0090_3bb0, all(9), vec![], seq.clone()),  // TOMkpShapeXYWHUV
        (0x0079_add0, all(1), vec![], seq.clone()),  // MsGetRamChrHP
        (0x0079_adf0, all(1), vec![], seq.clone()),  // MsGetRamChrHPmax
        (0x0090_55c0, all(6), vec![], seq.clone()),  // ToMakeBtlEasyDigitRight
        (0x0090_1660, all(6), vec![], seq.clone()),  // TOMkpCrossEasyStrFontSClut
        (0x0089_4b40, all(4), vec![], seq.clone()),  // TOBtlDrawInfoWinElementLamp
        (0x0089_5240, all(3), vec![], seq.clone()),  // TOBtlDrawInfoWinStatus
        (0x0090_2ef0, all(12), vec![], seq.clone()), // TOMkpRotateShapeXYWHUVC2
        (0x0090_3ee0, all(11), vec![], seq),         // TOMkpShapeXYWHUVC2
    ];
    round38_stands(a, b, cursor, 0, &stands);
    let arity: Vec<(u32, usize)> = stands.iter().map(|s| (s.0, s.1.len())).collect();
    let call = |m: &mut Oracle, sym: u32, args: &[u32]| {
        let &(_, n) = arity
            .iter()
            .find(|e| e.0 == sym)
            .unwrap_or_else(|| panic!("{sym:#x}: no stand-in"));
        assert_eq!(args.len(), n, "{sym:#x} words");
        eax(m, sym, args)
    };
    let fcall = |m: &mut Oracle, sym: u32, args: &[u32]| {
        assert!(
            [0x0064_47e0, 0x0064_4820, 0x0064_1410].contains(&sym),
            "{sym:#x}"
        );
        let at = m.rebase(sym);
        m.call_f64(at, args).unwrap()
    };
    let g = |a: &Oracle, s: u32| a.rebase(s);
    let words = [0xbu32, 0, 0, 1, 2, 4, 8, 0xf, 0x100, u32::MAX];
    // time - start: before, at, inside, past the 0.2 s fade, NaN
    let deltas = [-0.1f32, 0.0, -0.0, 0.03, 0.1, 0.2, 0.5, f32::NAN];
    for k in 0..1500u32 {
        put(a, b, cursor, log);
        put(a, b, counter, 0u32);
        put(a, b, tcounter, 0u32);
        for j in 0..0x40 {
            let v = if rng.next().is_multiple_of(5) {
                rng.next()
            } else {
                words[(rng.next() % words.len() as u32) as usize]
            };
            put(a, b, table + 4 * j, v);
        }
        let now = [100.0f32, 3.25, 5000.5][(rng.next() % 3) as usize];
        let delta = deltas[(rng.next() % deltas.len() as u32) as usize];
        let (now, start) = match rng.next() % 8 {
            // f32(since / 0.2f32) and f32(since / 0.2) truncate to
            // different 128ths here
            0 => {
                let since = [0x3ca6_6666u32, 0x3cd9_9999, 0x3d06_6666, 0x3d26_6666]
                    [(rng.next() % 4) as usize];
                (0.0, -f32::from_bits(since))
            }
            // two NaNs: the x87 keeps the larger payload
            1 => (
                f32::from_bits(0x7fc0_1234),
                f32::from_bits([0xffc5_6789u32, 0x7fc0_0001][(rng.next() % 2) as usize]),
            ),
            _ => (now, now - delta),
        };
        put(a, b, g(a, 0x0133_c944), start);
        put(a, b, times, now);
        put(a, b, times + 4, now + 1.5);
        put(a, b, times + 8, f32::from_bits(round38_quiet(rng)));
        put(
            a,
            b,
            g(a, 0x0133_c93e),
            [0u8, 1, 1, 1, 0x80][(rng.next() % 5) as usize],
        );
        put(
            a,
            b,
            g(a, 0x0133_d6e0),
            [0i32, 0, 0, 1, 2, 0x101][(rng.next() % 6) as usize],
        );
        put(
            a,
            b,
            g(a, 0x0133_f5c6),
            [-1i16, 0, 1, 3][(rng.next() % 4) as usize],
        );
        let chr = [0u32, 3, 0x13, rng.next()][(rng.next() % 4) as usize];
        let x = [0xb3u32, 0, rng.next() % 1280, rng.next(), 0x8000_0000][(rng.next() % 5) as usize];
        let y = [0xf0u32, 0, rng.next() % 720, rng.next()][(rng.next() % 4) as usize];
        let want = call_raw(a, 0x0089_3a30, &[chr, x, y]);
        assert!(want.st0.is_none(), "info win leaves ST0 {k}");
        // drawn: the window, the caption, 3 values, 2 rings, 2 plates
        let shown: u8 = Mem::read(a, g(a, 0x0133_c93e));
        let off: i32 = Mem::read(a, g(a, 0x0133_d6e0));
        let logged: u32 = Mem::read(a, cursor);
        assert_eq!(
            shown != 0 && off != 1,
            logged - log >= 0x180,
            "info win logged {k}"
        );
        t::btl_draw_info_win(b, chr, [x.cast_signed(), y.cast_signed()], call, fcall);
        same_memory(&format!("pc7 b info win {k}"), a, b);
        settle(a, b);
    }
}

#[test]
fn pc7_b_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    pc7_b(&mut a, &mut b, &mut Xorshift(0x9c07_b001));
}

#[test]
fn pc7_b_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    pc7_b(&mut a, &mut b, &mut Xorshift(0x9c07_b002));
}

/// pc round 8 a: `TOMenuExecLayerHandle` (scenes with and without data,
/// layers below 0, inside and above 15, lists missing, empty or holding
/// the handle 0..3 times among other words; `TkMenuExecScene` a logging
/// stand-in returning counting values) and `TOMenuTryOpen_008be840`
/// (`g_MovieFading`/`g_PauseOn` on and `TkMenuStart` clearing them or
/// not, each open condition, the window-area flag, the English setting
/// changing or not). Every callee is a logging stand-in.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // two functions; a, b, g, i, k
fn pc8_a(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let call = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = m.rebase(f);
        m.call(f, Conv::Cdecl, args).unwrap().eax
    };
    let g = |m: &Oracle, at: u32| m.rebase(at);
    let fs: &[(u32, u8)] = &[
        (0x00a7_b460, 3),
        (0x008b_e320, 0),
        (0x0086_bae0, 0),
        (0x0078_16f0, 0),
        (0x0078_51d0, 0),
        (0x008b_e3d0, 0),
        (0x008a_f5c0, 1),
        (0x008b_1190, 0),
        (0x008a_df80, 1),
        (0x008b_09b0, 0),
        (0x008a_b030, 1),
    ];
    let (log, cursor, cells) = small6_stubs(a, b, fs);
    // TkMenuStart: logged, then g_MovieFading and g_PauseOn &= [mask]
    let mask = alloc(a, b, 4);
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &cursor.to_le_bytes()].concat(); // mov ecx, [cursor]
    code.extend([0xc7, 0x01, 0x77, 0x77, 0, 0]); // mov dword [ecx], 0x7777
    code.extend([0x8b, 0x44, 0x24, 0x04, 0x89, 0x41, 0x04]); // mov eax, [esp+4]; mov [ecx+4], eax
    code.extend([&[0x83, 0x05][..], &cursor.to_le_bytes(), &[8]].concat()); // add dword [cursor], 8
    code.push(0xa1); // mov eax, [mask]
    code.extend(mask.to_le_bytes());
    for at in [0x0134_0824u32, 0x0134_0828] {
        code.extend([0x21, 0x05]); // and [at], eax
        code.extend(g(a, at).to_le_bytes());
    }
    // and the kept English setting ^= [mask] (the menu changed it)
    code.extend([0x31, 0x05]); // xor [0x0186a1cc], eax
    code.extend(g(a, 0x0186_a1cc).to_le_bytes());
    code.push(0xc3);
    let at = a.alloc_code(&code).unwrap();
    assert_eq!(b.alloc_code(&code).unwrap(), at);
    redirect(a, b, 0x008a_b030, at);

    // TOMenuExecLayerHandle
    let table = g(a, 0x022f_b4dc);
    let data = alloc(a, b, 0x200);
    for k in 0..300u32 {
        put(a, b, cursor, log);
        put(a, b, cells[0], rng.next());
        put(a, b, cells[0] + 4, rng.next() % 3);
        let i = rng.next() % 8;
        let scene = if rng.next().is_multiple_of(6) {
            0
        } else {
            data
        };
        put(a, b, table + i * 4, scene);
        let layer =
            [0u32, 3, 15, 16, 0xffff_ffff, 0x8000_0000, 7, 0x7fff_ffff][(rng.next() % 8) as usize];
        let handle = rng.next() % 4;
        for l in 0..16 {
            let off = match rng.next() % 5 {
                0 => 0,
                _ => 0xa0 + 0x10 * (rng.next() % 8),
            };
            put(a, b, data + 0x90 + l * 4, off);
        }
        for w in 0..0x18 {
            let v = if w % 6 == 0 {
                rng.next() % 5
            } else {
                rng.next() % 6
            };
            put(a, b, data + 0xa0 + w * 4, v);
        }
        let out = rng.next();
        let args = [i, layer, handle, out];
        let ra = eax(a, 0x00a7_b220, &args);
        let rb = ffx_system::scene::bridge::exec_layer_handle(b, args, call);
        assert_eq!(ra, rb, "TOMenuExecLayerHandle {k} {args:x?}");
        same_memory(&format!("pc8 a layer {k}"), a, b);
        settle(a, b);
    }

    // TOMenuTryOpen_008be840
    for k in 0..400u32 {
        put(a, b, cursor, log);
        for &c in &cells {
            put(a, b, c, rng.next() % 3);
            put(a, b, c + 4, rng.next() % 2);
        }
        put(a, b, cells[2], [0u32, 1, 2][(rng.next() % 3) as usize]);
        put(a, b, mask, [0u32, u32::MAX][(rng.next() % 2) as usize]);
        put(
            a,
            b,
            g(a, 0x0134_0824),
            u32::from(rng.next().is_multiple_of(3)),
        );
        put(
            a,
            b,
            g(a, 0x0134_0828),
            u32::from(rng.next().is_multiple_of(3)),
        );
        put(a, b, g(a, 0x0132_70f4), rng.next() % 2);
        put(a, b, g(a, 0x0186_a1cc), rng.next());
        let mode = [0u32, 1, 2, rng.next()][(rng.next() % 4) as usize];
        let ra = eax(a, 0x008b_e840, &[mode]);
        let rb = ffx_system::tolib::bridge::menu_try_open(b, mode, call);
        assert_eq!(ra, rb, "TOMenuTryOpen {k}");
        same_memory(&format!("pc8 a open {k}"), a, b);
        settle(a, b);
    }
}

#[test]
fn pc8_a_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    pc8_a(&mut a, &mut b, &mut Xorshift(0x9c08_a001));
}

#[test]
fn pc8_a_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    pc8_a(&mut a, &mut b, &mut Xorshift(0x9c08_a002));
}

/// pc round 8 b: `TOBtlDrawLibraWindow` over window states -1..=8, the
/// i16 extremes and high bits the `movsx` drops; `ToDrawLibraWin` 0 or
/// not, `TOGetFFXLangJmp` 0 or not, both halves logged with the two words
/// the original leaves (`x` and the unused `ftol(Y2(535))`). The remaps
/// and `ftol2_sse` are the originals, the remaps' `.rdata` scales the
/// game's or random (the game's truncate 60 -> 61 and 535 -> 536 alike).
fn pc8_b(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let call = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = m.rebase(f);
        m.call(f, Conv::Cdecl, args).unwrap().eax
    };
    let fcall = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = m.rebase(f);
        m.call_f64(f, args).unwrap()
    };
    let fs: &[(u32, u8)] = &[
        (0x0089_bc80, 0),
        (0x008a_c2f0, 0),
        (0x0089_bf70, 2),
        (0x0089_c7d0, 2),
    ];
    let (log, cursor, cells) = small6_stubs(a, b, fs);
    let state_at = a.rebase(0x0133_f6a8);
    for k in 0..300u32 {
        put(a, b, cursor, log);
        for &c in &cells {
            put(a, b, c, rng.next() % 3);
            put(a, b, c + 4, rng.next());
        }
        let state = match rng.next() % 4 {
            0 => rng.next(),
            _ => [
                0xffff_u32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 0x7fff, 0x8000, 0x1_0003,
            ][(rng.next() % 13) as usize],
        };
        put(a, b, state_at, state);
        // the remaps' scales (`.rdata` f64s: X2 * [0x00b41d38] / 1920,
        // Y2 * [0x00b41d30] / 1080): the game's 512 and 416, or others
        for (g, v) in [(0x00b4_1d38_u32, 512.0), (0x00b4_1d30, 416.0)] {
            let at = a.rebase(g);
            let x = if k % 3 == 0 {
                v
            } else {
                f64::from(rng.next() % 4000) + 0.5
            };
            put(a, b, at, x);
        }
        let ra = call(a, 0x0089_5390, &[]);
        let rb = ffx_system::btl::bridge::btl_draw_libra_window(b, call, fcall);
        assert_eq!(ra, rb, "TOBtlDrawLibraWindow {k} state {state:#x}");
        same_memory(&format!("pc8 b libra window {k}"), a, b);
        settle(a, b);
    }
}

#[test]
fn pc8_b_matches_the_original_synthetic() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    pc8_b(&mut a, &mut b, &mut Xorshift(0x9c08_b001));
}

#[test]
fn pc8_b_matches_the_original_on_the_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    pc8_b(&mut a, &mut b, &mut Xorshift(0x9c08_b002));
}
