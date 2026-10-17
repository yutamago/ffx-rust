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
    accessor, btl, cd, config, container, daps, dtor, files, fixup, instance, math, menu, msg,
    names, pad, pkt, render, save, scene, sort, stl, stub, sync, tolib, vbf, window,
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
        // ST0 comes back as an f64 (exact for an f32), NaN payloads lost
        if r.is_nan() {
            assert!(st0.is_nan(), "length² case {n}: {v:?}");
        } else {
            #[allow(clippy::cast_possible_truncation)] // exact: an f32 value
            let st0 = st0 as f32;
            assert_eq!(st0.to_bits(), r.to_bits(), "length² case {n}: {v:?}");
        }
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
        // ST0 comes back as an f64 (exact for an f32), NaN payloads lost
        if r.is_nan() {
            assert!(st0.is_nan(), "{x} / {y}");
        } else {
            #[allow(clippy::cast_possible_truncation)] // exact: an f32 value
            let st0 = st0 as f32;
            assert_eq!(st0.to_bits(), r.to_bits(), "{x} / {y}");
        }
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
    let st0_f32 = |r: ffx_oracle::Ret| {
        #[allow(clippy::cast_possible_truncation)] // exact: an f32 value
        let v = r.st0.expect("a float result") as f32;
        v
    };
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
        assert!(
            v.to_bits() == got.to_bits() || (v.is_nan() && got.is_nan()),
            "{lang} {kind} {ws}"
        );
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
            assert!(
                st0.to_bits() == r.to_bits() || (st0.is_nan() && r.is_nan()),
                "advance {n}"
            );
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
