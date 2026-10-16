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
    btl, cd, config, fixup, instance, math, menu, msg, pad, pkt, render, save, scene, stub, vbf,
    window,
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
