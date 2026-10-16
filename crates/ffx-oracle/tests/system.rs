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
    accessor, btl, cd, config, daps, fixup, instance, math, menu, msg, names, pad, pkt, render,
    save, scene, stl, stub, vbf, window,
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
}

impl TestOs {
    fn new(o: &Oracle, at: u32, fail_size: u32) -> Self {
        Self {
            at,
            fail_size,
            small: Mem::read(o, at),
            big: Mem::read(o, at + 4),
            locks: 0,
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
        if size == self.fail_size {
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
    fn malloc_pool_init(&mut self) {
        panic!("the tests start with a heap");
    }
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
