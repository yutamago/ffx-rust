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
use ffx_system::{cd, config, instance, math, msg, save, stub};

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
/// handling of them is checked against the hardware in `ffx_system::x87`.
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
            eax(&mut a, sym, &[dst, x, y]);
            rust(&mut b, dst, x, y);
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
/// the load and the larger significand wins (`ffx_system::x87`'s
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
        use ffx_system::x87::load;
        (load(f32::from_bits(x)) + load(f32::from_bits(y)))
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
