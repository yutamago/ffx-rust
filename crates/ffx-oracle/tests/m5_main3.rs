//! M5 stage 1, list `main`, round 3 (`ffx_sg::m5main3_*`) against the
//! original: on the image alone with synthetic state, and on the
//! snapshots (`FFX_SNAPSHOT`) over the game's memory. Results and every
//! byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! None of these functions ran in a coverage run (the PS2 debug GUI and
//! its helpers), so every branch is reached with synthetic data; the
//! snapshots add the game's own globals underneath. Callees that stay
//! original are recording stubs in both oracles alike (`m5_main3_common`).

#![allow(
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod m5_main3_common;

use m5_main3_common::{Out, Rng, Twins};

use ffx_oracle::Conv;

// ---- the Gu packet builders and the debug printf ----

use ffx_sg::m5main3_gu::bridge as gu;

fn gu_globals(t: &mut Twins, rng: &mut Rng) -> u32 {
    let packet = t.zeros(0x800);
    t.g32(0x0130_36c4, packet);
    t.g32(0x0130_36c8, packet);
    t.g32(0x0130_36c0, rng.below(8));
    t.g32(0x00c4_df54, rng.pick(&[0x100, 0x80, 0x1c0, 0xffff_ff00]));
    t.g32(0x0130_36dc, rng.pick(&[0x100, 0x180, 0x40]));
    t.g32(0x023c_bbe0, rng.pick(&[448, 720, 1, 0xffff_fff0, 0x1001]));
    t.g32(0x023c_bbe4, rng.pick(&[640, 1280, 3, 0xffff_ffe1, 0x1003]));
    t.g32(0x0130_3730, rng.pick(&[0, 0x40, 0xffff_ffd0, 0x7fff_ffff]));
    packet
}

#[test]
fn gu_sprite_matches_the_original() {
    let mut rng = Rng(0x1234_5678);
    Twins::each(|t| {
        for _ in 0..40 {
            t.reset();
            gu_globals(t, &mut rng);
            let a: [u32; 9] = std::array::from_fn(|_| rng.word());
            t.check("sprite", 0x0084_6840, Conv::Cdecl, &a, |m| {
                Out::U32(gu::sprite(
                    m, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8],
                ))
            });
        }
    });
}

#[test]
fn gu_frame_matches_the_original() {
    let mut rng = Rng(0x0bad_cafe);
    Twins::each(|t| {
        for k in 0..60 {
            t.reset();
            t.stub(gu::GU_SET_TEST, false, 8, 0, 0x55);
            gu_globals(t, &mut rng);
            // Hokan off, on (both easing paths, the step 0 -> 2).
            t.g32(0x00c4_df60, u32::from(k % 3 != 0));
            let mut c = [0u8; 0x44];
            for b in &mut c {
                *b = rng.next() as u8;
            }
            for o in (0x8..0x18).step_by(2) {
                let v = rng.pick(&[0i16, 1, -1, 5, 100, -300, 0x7fff, -0x8000]);
                c[o..o + 2].copy_from_slice(&v.to_le_bytes());
            }
            let step = rng.pick(&[0i16, 1, 2, 3, -2, 7]);
            c[0x24..0x26].copy_from_slice(&step.to_le_bytes());
            if k % 5 == 0 {
                // The targets reached: no easing step.
                c.copy_within(0x10..0x18, 0x8);
            }
            let cp = t.alloc(&c);
            let (dx, dy) = (rng.word(), rng.word());
            let what = format!("frame {k}");
            t.check(&what, 0x0084_6b10, Conv::Cdecl, &[cp, dx, dy], |m| {
                Out::U32(gu::frame(m, cp, dx, dy))
            });
        }
    });
}

/// A printer stub: logs each character (tag 0x7000).
fn printer(t: &mut Twins) {
    let f = t.stub_code(0x7000, false, 1, 0, &m5_main3_common::Ret::Fixed(0x99), &[]);
    t.g32(0x00c4_dfb0, f);
}

#[test]
fn sg_vfprintf_matches_the_original() {
    // (format, words): the words are the variadic arguments; a word of
    // 0xffff_0000 + k is replaced by the address of the k-th string.
    let strings: [&[u8]; 3] = [b"hello\0", b"\0", b"abcdefghijklmnop\0"];
    let cases: &[(&[u8], &[u32])] = &[
        (b"\0", &[]),
        (b"plain text\0", &[]),
        (b"%d %i %u\0", &[5, 0xffff_fff6, 0xffff_ffff]),
        (
            b"%5d|%-5d|%05d|%+d|% d|%+05d\0",
            &[42, 42, 42, 42, 42, 0xffff_ffd6],
        ),
        (
            b"%x %X %p %08x %.3x %#x\0",
            &[0xbeef, 0xbeef, 0x1234, 0xab, 0, 7],
        ),
        (b"%b %.0d %.0u %.5b\0", &[5, 0, 0, 3]),
        (b"%c%c%-3c|%3c\0", &[0x41, 0x1ff, 0x42, 0x43]),
        (
            b"%s|%10s|%-8s|%.3s|%.30s|%*s\0",
            &[
                0xffff_0000,
                0xffff_0000,
                0xffff_0002,
                0xffff_0002,
                0xffff_0000,
                0xffff_fffa,
                0xffff_0001,
            ],
        ),
        (b"%*d|%.*d|%-*d\0", &[6, 9, 4, 9, 0xffff_fffd, 1]),
        (b"a%nb%%c%n\0", &[0xffff_0100, 0xffff_0101]),
        (b"%%%q tail\0", &[]),
        (b"%\0", &[]),
        (b"%12.4d %0.2x\0", &[0xffff_ff85, 0x7]),
        (b"%-08d|%0-8d|\0", &[3, 3]),
        (b"%d|%u|%x|%b\0", &[0, 0, 0, 0]),
        (b"%*d|%*x|\0", &[0xffff_ffff, 5, 0xffff_fffe, 6]),
    ];
    let defaults = [(0u32, 0u8, 0i32, -1i32), (4, b' ', 2, 3), (1, b'*', -1, 0)];
    Twins::each(|t| {
        for (dk, &(flags, sign, width, prec)) in defaults.iter().enumerate() {
            for (fmt, words) in cases {
                t.reset();
                printer(t);
                t.g32(0x0132_5150, flags);
                t.g8(0x0132_5154, sign);
                t.g32(0x0132_5158, width as u32);
                t.g32(0x0132_515c, prec as u32);
                let s: Vec<u32> = strings.iter().map(|s| t.alloc(s)).collect();
                let outs = t.zeros(8);
                let args: Vec<u32> = words
                    .iter()
                    .map(|&w| match w {
                        0xffff_0000..=0xffff_0002 => s[(w & 3) as usize],
                        0xffff_0100..=0xffff_0101 => outs + 4 * (w & 1),
                        _ => w,
                    })
                    .collect();
                let va = t.words(&args);
                let f = t.alloc(fmt);
                let what = format!("vfprintf {:?} defaults {dk}", String::from_utf8_lossy(fmt));
                t.check(&what, 0x0085_11d0, Conv::Cdecl, &[0x77, f, va], |m| {
                    Out::U32(gu::vfprintf(m, f, va))
                });
                assert!(fmt.len() < 3 || t.calls() > 0, "{what}: nothing printed");
            }
        }
    });
}

// ---- the CD library's read queue ----

use ffx_sg::m5main3_cd::bridge as cd;

#[test]
fn cd_queue_read_matches_the_original() {
    let mut rng = Rng(0x00c0_ffee);
    Twins::each(|t| {
        for k in 0..120 {
            t.reset();
            let mode = [0u32, 0, 0, 1, 2][k % 5];
            let hdd = u8::from(rng.chance(3));
            let calm_scene = rng.below(2);
            let calm_mgrp = rng.below(2);
            let movie = rng.below(3) == 0;
            // The name CDfileName_PC returns: the c101 path, a prefix of
            // it, another path, or none (then the Eternal Calm names).
            let path = b"host0:/ffx/master/jppc/chr/pc/c101/mdl/c101.chr\0";
            let name_kind = rng.below(4);
            let text: &[u8] = match name_kind {
                0 => path,
                1 => b"host0:/ffx\0",
                2 => b"host0:/ffx/other/name.bin\0",
                _ => b"\0",
            };
            let mut buf = text.to_vec();
            buf.resize(0x100, 0xcc);
            let name = t.alloc(&buf);
            let calm_name = t.alloc(b"eternal/calm.bin\0");
            // A name of 0 in the PC files without an Eternal Calm name
            // makes the original read address 0: not tested.
            let file = match k % 4 {
                0 => 18 * rng.below(40) + 3,
                1 => 18 * rng.below(40),
                2 => rng.next(),
                _ => (18 * rng.below(40)).wrapping_neg(),
            };
            let name_ret = if name_kind == 3 && mode == 0 && hdd == 0 {
                let r = (file as i32) % 18;
                let calm = (r == 3 && calm_mgrp != 0) || (r == 0 && calm_scene != 0);
                if calm { 0 } else { name }
            } else {
                name
            };
            t.stubs(&[
                (cd::MOVIE_ERR, 3, 0),
                (cd::CD_FILE_SIZE_CD, 1, 0x1111),
                (cd::CD_FILE_SIZE_PC, 1, 0x2222),
                (cd::CD_FILE_NAME_PC, 1, name_ret),
                (cd::MODULE_FILE_SECTOR_REL, 1, 0x3333),
                (cd::FILE_SIZE_BYTES, 1, 0x4444),
                (cd::IS_ETERNAL_CALM_SCENE, 1, calm_scene),
                (cd::IS_ETERNAL_CALM_MGRP, 1, calm_mgrp),
                (cd::ETERNAL_CALM_MGRP_NAME, 1, calm_name),
                (cd::ETERNAL_CALM_SCENE_NAME, 1, calm_name),
                (cd::TK_SYNC_V, 1, 0),
                (cd::MOVIE_RUN_CHECK, 0, u32::from(movie)),
                (cd::DBG_PRINTF, 2, 0),
                (cd::YI_ASSERT, 6, 0),
                (cd::CD_SIZE_FROM_TABLE, 1, 0x0101_0101 * rng.below(2)),
                (cd::REGIST_ASYNC_TASK, 2, 0x55),
            ]);
            t.g32(cd::MODE, mode);
            t.g8(cd::HDD, hdd);
            t.g32(cd::MODULE, rng.below(9));
            t.g32(cd::READ_ID, rng.pick(&[0, 7, 0xfffe, 0xffff, 0xffff_ffff]));
            // The save's swap key: equal or not.
            let v = 0x1234 + rng.below(2);
            t.g32(cd::SWAP_VALUE, 0x1234);
            t.write(t.rebase(cd::SWAP_KEY), &(v as u16).to_le_bytes());
            // Busy requests in front; all 64 now and then.
            let used = if rng.chance(6) { 64 } else { rng.below(64) };
            for i in 0..64 {
                let e = t.rebase(cd::CHANNELS) + i * 0x38;
                let busy = if i < used {
                    [1u8, rng.below(2) as u8]
                } else {
                    [0, 0]
                };
                let busy = if i < used && busy == [1, 0] && rng.chance(2) {
                    [0, 1]
                } else {
                    busy
                };
                t.write(e + 0x31, &busy);
                t.write(e + 0x34, &[0xcc, 0xcd]);
            }
            let opts = if rng.chance(2) {
                0
            } else {
                t.words(&[9, 9, 9, 0xa, 0xb, 0xc, 0xd, 9])
            };
            let (buffer, cb) = (rng.next(), rng.next());
            let what =
                format!("Cd_QueueRead {k} mode {mode} hdd {hdd} name {name_kind} file {file:#x}");
            t.check(
                &what,
                0x0076_b970,
                Conv::Cdecl,
                &[file, buffer, cb, opts],
                |m| Out::U32(cd::queue_read(m, file, buffer, cb, opts)),
            );
        }
    });
}

// ---- the movie viewer's camera ----

use ffx_sg::m5main3_misc::bridge as misc;

fn float_word(rng: &mut Rng) -> u32 {
    match rng.below(8) {
        0 => 0,
        1 => 0x8000_0000,
        2 => 0x7f80_0000,
        3 => 0x7fc0_1234,
        4 => 0xffc0_0000,
        _ => (rng.below(4000) as f32 / 7.0 - 250.0).to_bits(),
    }
}

#[test]
fn movie_camera_matches_the_original() {
    let mut rng = Rng(0x5eed_0001);
    Twins::each(|t| {
        let view = t.zeros(0x5_0800);
        for k in 0..60 {
            t.reset();
            t.stub(misc::VIEW_SCREEN_MATRIX, false, 10, 0, 0);
            let mut cam: Vec<u32> = (0..10).map(|_| float_word(&mut rng)).collect();
            if k % 3 != 0 {
                cam[9] = rng
                    .pick(&[60.0f32, 45.0, 90.0, 1.0, 179.0, -30.0])
                    .to_bits();
            }
            let camera = t.words(&cam);
            t.w8(view + 0x5_076c, u8::from(k % 2 == 0));
            for o in [0u32, 4, 8, 0xc, 0x20, 0x24] {
                let v = float_word(&mut rng);
                t.g32(0x0113_7000 + o, v);
            }
            let (w, h) = match k % 4 {
                0 => (640, 448),
                1 => (rng.below(4000), rng.below(4000)),
                2 => (0, 0),
                _ => (0xffff_fc00, 0x7fff_ffff),
            };
            let what = format!("movie camera {k}");
            t.check(
                &what,
                0x00a4_3e60,
                Conv::Cdecl,
                &[camera, w, h, view],
                |m| {
                    misc::movie_camera(m, camera, w, h, view);
                    Out::None
                },
            );
        }
    });
}

/// Writes `w` through the stack argument `k` (0-based): `mov eax, [esp +
/// 4 + 4k]` and `mov dword [eax + 4i], w[i]`.
fn fill_arg(k: u8, w: &[u32]) -> Vec<u8> {
    let mut c = vec![0x8b, 0x44, 0x24, 4 + 4 * k];
    for (i, v) in w.iter().enumerate() {
        c.extend([0xc7, 0x40, (4 * i) as u8]);
        c.extend(v.to_le_bytes());
    }
    c
}

#[test]
fn light_table_init_matches_the_original() {
    let mut rng = Rng(0x0001_1647);
    Twins::each(|t| {
        for k in 0..12 {
            t.reset();
            let colour: Vec<u32> = (0..4)
                .map(|i| match (k + i) % 6 {
                    0 => 0.5f32.to_bits(),
                    1 => 1.0f32.to_bits(),
                    2 => (-0.25f32).to_bits(),
                    3 => 1e12f32.to_bits(),
                    4 => 0x7fc0_0000,
                    _ => (rng.below(1000) as f32 / 999.0).to_bits(),
                })
                .collect();
            let extra = fill_arg(1, &colour);
            t.stub_ex(
                misc::LIGHT_GET_COLOR,
                false,
                1,
                0,
                &m5_main3_common::Ret::Fixed(0),
                &extra,
            );
            // __sse2_available picks _ftol2_sse's path.
            t.g32(0x0193_ed20, k % 2);
            let buf = t.b.stack_range().start + 0x400;
            let what = format!("light table {k}");
            t.check(&what, 0x0084_ea00, Conv::Cdecl, &[], |m| {
                Out::U32(misc::light_table_init(m, buf))
            });
        }
    });
}

#[test]
fn mouse_main_matches_the_original() {
    let mut rng = Rng(0x0000_3a05);
    Twins::each(|t| {
        for k in 0..80 {
            t.reset();
            let report = t.rebase(misc::MOUSE_REPORT);
            let fail = [0u32, 0, 0, 0, 1, 0, 0, 0, 2][(k % 9) as usize];
            let r = [
                rng.word(),
                rng.word(),
                rng.word(),
                rng.pick(&[0, 1, 0xffff_fffd, 300, 0x7fff_ffff, 0x8000_0000]),
                rng.pick(&[0, 2, 0xffff_ff00, 900, 0x8000_0000]),
                rng.word(),
            ];
            let report_words: Vec<u8> = r.iter().flat_map(|v| v.to_le_bytes()).collect();
            // The read fills the report (all 0x20 bytes, as the RPC does).
            let mut fill = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp + 8]
            for (i, w) in report_words.chunks(4).enumerate() {
                fill.extend([0xc7, 0x40, (4 * i) as u8]);
                fill.extend(w);
            }
            t.stub(misc::USB_MOUSE_SYNC, false, 1, 0, 0);
            t.stub_ex(
                misc::USB_MOUSE_READ2,
                false,
                2,
                0,
                &m5_main3_common::Ret::Fixed(fail),
                &fill,
            );
            t.stub(misc::DBG_PRINTF, false, 1, 0, 0);
            let _ = report;
            let pos = |rng: &mut Rng| match rng.below(7) {
                0 => 0x7fc0_0000,
                5 => (-0.5f32).to_bits(),
                1 => (-5.0f32).to_bits(),
                2 => 1023.75f32.to_bits(),
                3 => 2000.0f32.to_bits(),
                _ => (rng.below(1300) as f32 + 0.5).to_bits(),
            };
            let (x, y) = (pos(&mut rng), pos(&mut rng));
            t.g32(misc::MOUSE_POS, x);
            t.g32(misc::MOUSE_POS + 4, y);
            t.g32(0x0193_ed20, k % 2);
            let what = format!("Sg_MouseMain {k}");
            t.check(&what, 0x0085_1920, Conv::Cdecl, &[], |m| {
                Out::U32(misc::mouse_main(m))
            });
        }
    });
}

/// Stub code that logs the `n` words the stack argument `k` points to,
/// at the log entry's word `at` (`eax` holds the entry).
fn log_pointee(k: u8, n: u8, at: u8) -> Vec<u8> {
    let mut c = vec![0x8b, 0x54, 0x24, 4 + 4 * k]; // mov edx, [esp + 4 + 4k]
    for j in 0..n {
        c.extend([0x8b, 0x4a, 4 * j]); // mov ecx, [edx + 4j]
        c.extend([0x89, 0x48, 4 * (at + j)]); // mov [eax + 4(at + j)], ecx
    }
    c
}

#[test]
fn light_apply_matches_the_original() {
    let mut rng = Rng(0x0044_ebd0);
    Twins::each(|t| {
        for k in 0..40 {
            t.reset();
            let log4 = log_pointee(1, 4, 4);
            t.stub_ex(
                misc::LIGHT_SET_COLOR,
                false,
                1,
                0,
                &m5_main3_common::Ret::Fixed(0x31),
                &log4,
            );
            t.stub_ex(
                misc::LIGHT_SET_DIR,
                false,
                1,
                0,
                &m5_main3_common::Ret::Seq(vec![0x32, 0x33]),
                &log4,
            );
            let deg = |rng: &mut Rng| rng.pick(&[0u32, 90, 0xffff_ffd3, 360, 720, 45, 3600, 13]);
            let light = t.words(&[
                rng.pick(&[0, 255, 128, 0xffff_ffff, 1000]),
                rng.word(),
                rng.below(256),
                rng.word(),
                deg(&mut rng),
                deg(&mut rng),
                rng.pick(&[0, 1, 3, 0xffff, 0x8002]) | (u32::from(k % 3 != 0) << 16),
            ]);
            let buf = t.b.stack_range().start + 0x400;
            t.b.write(buf, &[0xcc; 16]).unwrap();
            let what = format!("light apply {k}");
            t.check(&what, 0x0084_ebd0, Conv::Cdecl, &[light], |m| {
                Out::U32(misc::light_apply(m, light, buf))
            });
        }
    });
}

// ---- the debug menu's light editor ----

use ffx_sg::m5main3_menu::bridge as menu;

#[test]
fn light_editor_windows_match_the_original() {
    let mut rng = Rng(0x0084_e7c0);
    Twins::each(|t| {
        for k in 0..16 {
            t.reset();
            t.stub_ex(
                menu::WINDOW_CREATE,
                false,
                6,
                0,
                &m5_main3_common::Ret::Seq(vec![0x0a00, 0x0b00, 0x0c00, 0x0d00, 0x0e00]),
                &[],
            );
            t.stubs(&[
                (menu::WINDOW_SET_TITLE, 2, 0x41),
                (menu::WINDOW_SET_ARG, 2, 0x42),
            ]);
            let colour = fill_arg(0, &[rng.next(), rng.next(), rng.next(), rng.next()]);
            t.stub_ex(
                menu::GET_BACK_COLOR,
                false,
                0,
                0,
                &m5_main3_common::Ret::Fixed(0x43),
                &colour,
            );
            t.g8(0x0130_3c1c, u8::from(k % 4 == 3));
            let buf = t.b.stack_range().start + 0x400;
            let what = format!("SgGu_BgColor {k}");
            t.check(&what, 0x0084_e7c0, Conv::Cdecl, &[], |m| {
                menu::bg_color(m, buf);
                Out::None
            });
            t.reset();
            t.stub_ex(
                menu::WINDOW_CREATE,
                false,
                6,
                0,
                &m5_main3_common::Ret::Seq(vec![0x0a00, 0x0b00, 0x0c00, 0x0d00, 0x0e00]),
                &[],
            );
            t.stub(menu::WINDOW_SET_TITLE, false, 2, 0, 0x41);
            for i in 0..6u32 {
                t.g8(0x0130_3c18 + i, u8::from(rng.chance(2)));
                let idx = rng.pick(&[0i16, 1, 2, 3, 4, 5, -1, 7]);
                let e = t.rebase(0x0130_3c20 + 0x1c * i + 0x18);
                t.write(e, &idx.to_le_bytes());
            }
            let what = format!("SgGu_LightEdit {k}");
            t.check(&what, 0x0084_e8c0, Conv::Cdecl, &[], |m| {
                menu::light_edit(m);
                Out::None
            });
        }
    });
}

#[test]
fn light_stock_proc_matches_the_original() {
    let mut rng = Rng(0x0084_e590);
    Twins::each(|t| {
        let cases: Vec<(u32, u32)> = [(0x8001, 0), (0x8003, 0), (0x8002, 3)]
            .into_iter()
            .chain((0..13).map(|id| (0x8007, id)))
            .chain([(0x8007, 0xffff_ffff)])
            .collect();
        for (msg, id) in cases {
            t.reset();
            let log_text = log_pointee(2, 1, 9);
            t.stub_ex(
                menu::WINDOW_ADD_BUTTON,
                false,
                7,
                0,
                &m5_main3_common::Ret::Fixed(0x51),
                &log_text,
            );
            t.stubs(&[
                (menu::WINDOW_SET_TITLE, 2, 0x52),
                (menu::BUTTON_SET_TOGGLE, 3, 0x53),
                (menu::BUTTON_SET_TOGGLE_VALUE, 3, 0x54),
                (menu::STOCK_LOAD, 2, 0x55),
                (menu::SAVE_LIGHT_FILE, 0, 0x56),
            ]);
            t.g8(menu::STOCK, rng.pick(&[0u8, 3, 6, 0xff, 2]));
            for i in 0..0xa8 / 4 {
                t.g32(0x0130_3c20 + 4 * i, rng.next());
            }
            let window = 0x00ab_0000;
            let slots = t.frame_ebp(3) + 8;
            let what = format!("LightStock msg {msg:#x} id {id}");
            // The port's own calls below the caller's argument words.
            t.b.set_call_top(Some(slots - 0x200));
            // The caller's argument words, as the port finds them.
            let words: Vec<u8> = [window, msg, id]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect();
            t.b.write(slots, &words).unwrap();
            t.check(&what, 0x0084_e590, Conv::Cdecl, &[window, msg, id], |m| {
                Out::U32(menu::save_proc(m, window, msg, id, slots))
            });
            t.b.set_call_top(None);
            let (x, y) = (t.a.read(slots, 12).unwrap(), t.b.read(slots, 12).unwrap());
            assert_eq!(x, y, "{what}: the caller's argument words");
        }
    });
}

#[test]
fn page_selector_matches_the_original() {
    let mut rng = Rng(0x0084_efd0);
    Twins::each(|t| {
        for k in 0..12 {
            t.reset();
            let left = rng.pick(&[0u32, 0x45, 0x46, 0x100, 0xffff_fff0]);
            t.stubs(&[
                (menu::WINDOW_GET_LEFT, 1, left),
                (menu::WINDOW_GET_X, 1, 0x33),
                (menu::WINDOW_GET_WIDTH, 1, 0x60),
                (menu::SELECTOR_ALLOCATE, 0, 0x5e10),
                (menu::SELECTOR_RESET_NB_ITEM, 2, 0x61),
                (menu::SELECTOR_SET_ITEM, 4, 0x62),
                (menu::SELECTOR_SET_SELECT, 2, 0x63),
                (menu::SELECTOR_SET_CALLBACK, 2, 0x64),
                (menu::SELECTOR_OPEN_XYWH, 7, 0x65),
            ]);
            for p in 0..8 {
                t.g32(menu::PAGES + 0x88 * p + 0x84, rng.next());
            }
            t.g32(menu::PAGE_SELECT, rng.below(9));
            let what = format!("page popup {k}");
            t.check(&what, 0x0084_efd0, Conv::Cdecl, &[0x00ab_0000], |m| {
                Out::U32(menu::page_popup(m, 0x00ab_0000))
            });
        }
    });
}

#[test]
fn select_main_menu_matches_the_original() {
    let mut rng = Rng(0x0084_f270);
    Twins::each(|t| {
        for page in [0u32, 3, 7, 8, 0xffff_ffff, 5] {
            t.reset();
            t.stubs(&[
                (menu::SET_RESOURCE, 2, 0x71),
                (menu::CONTROL_SET_HIDE, 3, 0x72),
                (menu::WINDOW_SET_CLIENT_H, 2, 0x73),
                (menu::CONTROL_SET_TEXT, 3, 0x74),
            ]);
            for p in 0..8 {
                let at = menu::PAGES + 0x88 * p;
                let n = rng.pick(&[0u32, 1, 3, 0xffff_ffff]);
                t.g32(at, n);
                for i in 0..4 {
                    t.g32(at + 4 + 4 * i, rng.below(0x40));
                }
                t.g32(at + 0x7c, rng.next());
                t.g32(at + 0x80, rng.next());
            }
            let cur = t.rebase(menu::PAGES + 0x88 * rng.below(8));
            t.g32(menu::PAGE, cur);
            let what = format!("Sg_SelectMainMenu {page:#x}");
            t.check(&what, 0x0084_f270, Conv::Cdecl, &[0x00ab_0000, page], |m| {
                Out::U32(menu::select_main_menu(m, 0x00ab_0000, page))
            });
        }
        for msg in [0x8011, 0x8001] {
            t.reset();
            t.stubs(&[
                (menu::WINDOW_GET_ARG, 1, 0x5e10),
                (menu::SELECTOR_GET_ITEM_VALUE, 1, 4),
                (menu::SELECT_MAIN_MENU, 2, 0x75),
                (menu::WINDOW_SET_WORD30, 2, 0x76),
            ]);
            t.g32(menu::MENU_WINDOW, 0x00ab_0000);
            let what = format!("page picked {msg:#x}");
            t.check(&what, 0x0084_f230, Conv::Cdecl, &[0x00cd_0000, msg], |m| {
                Out::U32(menu::page_picked(m, 0x00cd_0000, msg))
            });
        }
    });
}

#[test]
fn fuka_proc_matches_the_original() {
    Twins::each(|t| {
        for msg in [0x8001, 0x8011, 0x8002] {
            t.reset();
            t.stubs(&[
                (menu::WINDOW_SET_TITLE, 2, 0x41),
                (menu::PRINT_SET_ITALIC, 1, 0x42),
                (menu::PRINT_SET_BOLD, 1, 0x43),
                (menu::GU_PRINTF, 2, 0x44),
            ]);
            t.stub_ex(
                menu::GET_FUKA,
                false,
                1,
                0,
                &m5_main3_common::Ret::Seq(vec![10, 20, 30, 0xffff_fff0, 7, 8, 9, 11]),
                &[],
            );
            let what = format!("WFukaProc {msg:#x}");
            t.check(&what, 0x0084_f490, Conv::Cdecl, &[0x00ab_0000, msg], |m| {
                Out::U32(menu::fuka_proc(m, 0x00ab_0000, msg))
            });
        }
    });
}

#[test]
fn map_file_lists_match_the_original() {
    let mut rng = Rng(0x0085_06a0);
    Twins::each(|t| {
        for (k, (addr, of_map)) in [(0x0085_06a0u32, true), (0x0085_0770, false)]
            .into_iter()
            .cycle()
            .take(16)
            .enumerate()
        {
            t.reset();
            let dir = [0u32, 1, 2, 3][k / 2 % 4];
            let name = t.alloc(b"bika0100\0");
            t.stubs(&[
                (menu::MAP_GET_DIR_SELECT, 0, dir),
                (menu::MAP_GET_NAME, 1, name),
                (menu::MAKE_FILELIST, 3, 0x81),
                (menu::SELECTOR_RESET_NB_ITEM, 2, 0x82),
            ]);
            t.stub_ex(
                menu::SELECTOR_SET_ITEM,
                false,
                4,
                0,
                &m5_main3_common::Ret::Seq(vec![0x83, 0x84, 0x85]),
                &[],
            );
            let names: Vec<u32> = (0..4).map(|_| rng.next()).collect();
            let names = t.words(&names);
            let n = rng.pick(&[0u32, 1, 3, 0xffff_ffff]);
            let list = t.words(&[n, 0, names]);
            let what = format!("file list {addr:#x} dir {dir}");
            t.check_framed(&what, addr, Conv::Cdecl, &[list, 0x5e10], 0x84, |m, ebp| {
                Out::U32(menu::file_list(m, list, 0x5e10, ebp - 0x84, of_map))
            });
        }
    });
}

#[test]
fn mainmenu_mapname_matches_the_original() {
    Twins::each(|t| {
        for dir in [0u32, 1, 2, 3, 0xffff_ffff] {
            t.reset();
            t.stubs(&[
                (menu::MAP_GET_DIR_SELECT, 0, dir),
                (menu::CONTROL_SET_TEXT, 3, 0x91),
            ]);
            t.stub_ex(
                menu::MAP_GET_NAME,
                false,
                1,
                0,
                &m5_main3_common::Ret::Seq(vec![0x1111, 0x2222]),
                &[],
            );
            let what = format!("mainmenu mapname {dir:#x}");
            t.check(&what, 0x0085_0940, Conv::Cdecl, &[0x00ab_0000], |m| {
                Out::U32(menu::mainmenu_mapname(m, 0x00ab_0000))
            });
        }
    });
}

#[test]
fn deci2_callback_matches_the_original() {
    let mut rng = Rng(0x0084_ed10);
    Twins::each(|t| {
        for k in 0..40 {
            t.reset();
            let r = rng.pick(&[0u32, 5, 0xffff_ffff, 0x100]);
            t.stubs(&[(misc::DECI2_READ, 3, r), (misc::DECI2_WRITE, 3, r)]);
            let req = t.words(&[
                rng.next(),
                rng.pick(&[0, 4, 0x80]),
                rng.pick(&[0, 0x10, 0xfff0, 0x1_0000]),
                rng.next(),
                rng.next(),
                rng.next(),
            ]);
            let event = [0u32, 1, 2, 3, 4, 5, 6, 0xffff_ffff][k % 8];
            let n = rng.pick(&[0, 0x10, 0x20, 0xffff_0000]);
            let what = format!("deci2 callback {k} event {event}");
            t.check(&what, 0x0084_ed10, Conv::Cdecl, &[event, n, req], |m| {
                Out::U32(misc::deci2_callback(m, event, n, req))
            });
        }
    });
}

// ---- the spin controls and the character windows ----

fn spin_block(t: &mut Twins, rng: &mut Rng) -> u32 {
    let up = rng.pick(&[3u32, 0xd]);
    let w = [
        u32::from_le_bytes([
            rng.below(2) as u8,
            rng.below(2) as u8,
            rng.below(2) as u8,
            0x77,
        ]),
        rng.pick(&[1, 5, 0xffff_ffff, 0]),
        rng.pick(&[1, 0xffff_ffff, 2]),
        rng.pick(&[10, 0xffff_ffff, 3]),
        up,
        up + 5,
        up + 10,
        rng.pick(&[0, 0x00b0_f604]),
        rng.pick(&[0, 5, 299, 300, 301, 0xffff_fffe, 100, 0xffff_ffff]),
        rng.pick(&[300, 0xff, 0x168, 0, 7, 0xffff_fff0]),
        rng.pick(&[0, 1, 0xffff_ffff, 400, 0x168]),
        rng.pick(&[0, 1, 2, 10]),
    ];
    t.words(&w)
}

#[test]
fn spin_control_matches_the_original() {
    let mut rng = Rng(0x0085_1b20);
    Twins::each(|t| {
        for k in 0..300 {
            t.reset();
            t.stub(menu::CONTROL_SET_TEXTF, false, 4, 0, 0xa1);
            let sp = spin_block(t, &mut rng);
            let up: u32 = ffx_bridge::Mem::read(&t.a, sp + 0x10);
            let id = rng.pick(&[up, up + 5, up + 1, 0]);
            let msg = 0x8000 + rng.below(0x14);
            let what = format!("spin {k} msg {msg:#x} id {id}");
            t.check(
                &what,
                0x0085_1b20,
                Conv::Cdecl,
                &[0xab00, msg, id, sp],
                |m| Out::U32(menu::spin_msg(m, 0xab00, msg, id, sp)),
            );
        }
        // a press below 0 that wraps (minimum -1): -1 becomes max - 1
        for value in [0u32, 1] {
            t.reset();
            t.stub(menu::CONTROL_SET_TEXTF, false, 4, 0, 0xa1);
            let sp = t.words(&[1, 1, 1, 1, 3, 8, 13, 0, value, 7, 0xffff_ffff, 0]);
            let what = format!("spin wrap {value}");
            t.check(
                &what,
                0x0085_1b20,
                Conv::Cdecl,
                &[0xab00, 0x8005, 8, sp],
                |m| Out::U32(menu::spin_msg(m, 0xab00, 0x8005, 8, sp)),
            );
        }
        // a press past the maximum, clamped (minimum below maximum)
        for (value, min, max) in [
            (300u32, 0u32, 300u32),
            (5, 4, 5),
            (0xffff_fffe, 0xffff_fffd, 0xffff_ffff),
        ] {
            t.reset();
            t.stub(menu::CONTROL_SET_TEXTF, false, 4, 0, 0xa1);
            let sp = t.words(&[1, 1, 1, 1, 3, 8, 13, 0, value, max, min, 0]);
            let what = format!("spin clamp {value:#x}");
            t.check(
                &what,
                0x0085_1b20,
                Conv::Cdecl,
                &[0xab00, 0x8005, 3, sp],
                |m| Out::U32(menu::spin_msg(m, 0xab00, 0x8005, 3, sp)),
            );
        }
        for k in 0..8 {
            t.reset();
            t.stubs(&[
                (menu::WINDOW_ADD_STATIC, 7, 0xb1),
                (menu::WINDOW_ADD_BUTTON, 7, 0xb2),
                (menu::CONTROL_SET_TEXTF, 4, 0xb3),
            ]);
            let sp = spin_block(t, &mut rng);
            let a = [
                0xab00,
                sp,
                rng.below(0x50),
                rng.below(0x50),
                10,
                0x5500,
                rng.below(0x30),
                0x28,
            ];
            let what = format!("spin add {k}");
            t.check(&what, 0x0085_1c70, Conv::Cdecl, &a, |m| {
                Out::U32(menu::spin_add(
                    m, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7],
                ))
            });
        }
    });
}

#[test]
fn mouth_and_physics_windows_match_the_original() {
    let mut rng = Rng(0x0085_1e70);
    Twins::each(|t| {
        let msgs: Vec<(u32, u32)> = [(0x8001, 0), (0x8003, 0), (0x8011, 0), (0x8002, 1)]
            .into_iter()
            .chain((0..9).map(|id| (0x8007, id)))
            .collect();
        for &(msg, id) in &msgs {
            t.reset();
            t.stubs(&[
                (menu::WINDOW_SET_TITLE, 2, 0xc1),
                (menu::WINDOW_ADD_BUTTON, 7, 0xc2),
                (menu::WINDOW_ADD_STATIC, 7, 0xc3),
                (menu::CONTROL_SET_TEXT, 3, 0xc4),
                (menu::BUTTON_SET_TOGGLE, 3, 0xc5),
                (menu::BUTTON_SET_TOGGLE_VALUE, 3, 0xc6),
                (menu::GUI_SET_TALK, 2, 0xc7),
                (menu::GUI_SET_MOUTH_TH, 1, 0xc8),
            ]);
            t.stub_ex(
                menu::GUI_GET_MOUTH_TH,
                false,
                0,
                0,
                &m5_main3_common::Ret::Seq(vec![25, 0xffff_ffe2, 7]),
                &[],
            );
            t.g8(
                menu::MOUTH_COUNT,
                if id == 1 {
                    3
                } else {
                    rng.pick(&[0u8, 3, 0xff, 2])
                },
            );
            t.g8(menu::MOUTH_OPEN, 1);
            let what = format!("mouth msg {msg:#x} id {id}");
            t.check(&what, 0x0085_1e70, Conv::Cdecl, &[0xab00, msg, id], |m| {
                Out::U32(menu::mouth_proc(m, 0xab00, msg, id))
            });
        }
        for &(msg, id) in &msgs {
            for odd in [0u32, 1] {
                t.reset();
                t.stubs(&[
                    (menu::WINDOW_SET_TITLE, 2, 0xc1),
                    (menu::WINDOW_ADD_BUTTON, 7, 0xc2),
                    (menu::WINDOW_ADD_STATIC, 7, 0xc3),
                    (menu::BUTTON_SET_TOGGLE, 3, 0xc5),
                    (menu::BUTTON_SET_TOGGLE_VALUE, 3, 0xc6),
                    (menu::CONTROL_SET_TEXTF, 5, 0xc7),
                    (menu::CLOTH_GET_PARA_EDIT, 0, 1),
                    (menu::CLOTH_SET_PARA_EDIT, 1, 0xc8),
                    (menu::CLOTH_RESET_ALL, 0, 0xc9),
                    (menu::GLOBAL_SET_WIND, 3, 0xca),
                    (menu::CLOTH_SET_Y, 1, 0xcb),
                    (menu::CLOTH_SET_XZ, 1, 0xcc),
                ]);
                t.g32(menu::WIND_COUNT, odd + 2 * rng.below(4));
                for sp in menu::CLOTH_SPINS {
                    let w: Vec<u32> = (0..12).map(|_| rng.word()).collect();
                    let at = t.rebase(sp);
                    let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
                    t.write(at, &bytes);
                    t.g32(sp + 0x20, rng.pick(&[100, 0xffff_ff9c, 12345, 0]));
                }
                let what = format!("physics msg {msg:#x} id {id} odd {odd}");
                let slots = t.frame_ebp(3) + 8;
                let words: Vec<u8> = [0xab00, msg, id]
                    .iter()
                    .flat_map(|w: &u32| w.to_le_bytes())
                    .collect();
                t.b.write(slots, &words).unwrap();
                t.b.set_call_top(Some(slots - 0x200));
                t.check(&what, 0x0085_2080, Conv::Cdecl, &[0xab00, msg, id], |m| {
                    Out::U32(menu::physics_proc(m, 0xab00, msg, id, slots))
                });
                t.b.set_call_top(None);
                let (x, y) = (t.a.read(slots, 12).unwrap(), t.b.read(slots, 12).unwrap());
                assert_eq!(x, y, "{what}: the caller's argument words");
            }
        }
    });
}

#[test]
fn shadow_window_matches_the_original() {
    let mut rng = Rng(0x0085_23e0);
    Twins::each(|t| {
        let msgs: Vec<(u32, u32)> = [(0x8001, 0), (0x8003, 0), (0x8002, 1)]
            .into_iter()
            .chain((0..9).map(|id| (0x8007, id)))
            .chain((0..6).map(|_| (0x8011, 0)))
            .collect();
        // no security cookie: the whole frame is compared
        t.skip = 0;
        for (k, &(msg, id)) in msgs.iter().enumerate() {
            t.reset();
            t.stubs(&[
                (menu::WINDOW_SET_TITLE, 2, 0xd1),
                (menu::WINDOW_ADD_BUTTON, 7, 0xd2),
                (menu::WINDOW_ADD_STATIC, 7, 0xd3),
                (menu::BUTTON_SET_TOGGLE, 3, 0xd5),
                (menu::BUTTON_SET_TOGGLE_VALUE, 3, 0xd6),
                (menu::CONTROL_SET_TEXTF, 4, 0xd7),
                (menu::WINDOW_SET_CLIENT_H, 2, 0xd8),
                (menu::SET_SHADOW_MODE, 1, 0xd9),
                (menu::GET_SHADOW_SEMI, 0, 1),
                (menu::SET_MOVIE_GOMI_AVOID, 1, 0xda),
                (menu::GET_SHADOW_MODE, 0, (k % 6) as u32),
                (menu::SET_SHADOW_COLOR, 3, 0xdc),
                (menu::SET_SHADOW_DIR, 2, 0xdd),
                (menu::CONTROL_SET_DISABLE, 3, 0xde),
            ]);
            // dbgPrintf logs three words; the last format's call passes one
            // (its other two words are the caller's stack: cleared)
            let last = t.rebase(menu::SHADOW_FORMATS[2]);
            let mut p = vec![0x8b, 0x54, 0x24, 0x04, 0x81, 0xfa];
            p.extend(last.to_le_bytes());
            p.extend([
                0x75, 0x0e, 0xc7, 0x40, 0x0c, 0, 0, 0, 0, 0xc7, 0x40, 0x10, 0, 0, 0, 0,
            ]);
            t.stub_ex(
                menu::DBG_PRINTF,
                false,
                3,
                0,
                &m5_main3_common::Ret::Fixed(0xdb),
                &p,
            );
            // the getters write one word through each pointer (logged)
            let mut c = fill_arg(0, &[0x11]);
            c.extend(fill_arg(1, &[0x22]));
            c.extend(fill_arg(2, &[0x33]));
            t.stub_ex(
                menu::GET_SHADOW_COLOR,
                false,
                3,
                0,
                &m5_main3_common::Ret::Fixed(0xe1),
                &c,
            );
            let mut d = fill_arg(0, &[0x44]);
            d.extend(fill_arg(1, &[0x55]));
            t.stub_ex(
                menu::GET_SHADOW_DIR,
                false,
                2,
                0,
                &m5_main3_common::Ret::Fixed(0xe2),
                &d,
            );
            for (sp, _, _) in menu::SHADOW_SPINS {
                let w: Vec<u32> = (0..12).map(|_| rng.word()).collect();
                let at = t.rebase(sp);
                let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
                t.write(at, &bytes);
            }
            let window = 0xab00;
            let slots = t.frame_ebp(3) + 8;
            let words: Vec<u8> = [window, msg, id]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect();
            t.b.write(slots, &words).unwrap();
            let what = format!("shadow {k} msg {msg:#x} id {id}");
            t.check_framed(
                &what,
                0x0085_23e0,
                Conv::Cdecl,
                &[window, msg, id],
                menu::SHADOW_FRAME,
                |m, ebp| Out::U32(menu::shadow_proc(m, window, msg, id, slots, ebp)),
            );
            let (x, y) = (t.a.read(slots, 12).unwrap(), t.b.read(slots, 12).unwrap());
            assert_eq!(x, y, "{what}: the caller's argument words");
        }
    });
}

// ---- the camera debug screen ----

use ffx_sg::m5main3_cam::bridge as cam;

/// A camera list as the screen keeps it: `n` points (12-byte flag
/// entries with bit 1 on the `sel`-th, 16-byte points).
fn cam_list(t: &mut Twins, rng: &mut Rng, list: u32, n: u32, sel: Option<u32>) {
    let mut flags = vec![0u8; 12 * n.max(1) as usize];
    for (i, f) in flags.chunks_mut(12).enumerate() {
        f[0] = (rng.below(256) as u8) & !2;
        if Some(i as u32) == sel {
            f[0] |= 2;
        }
    }
    let flags = t.alloc(&flags);
    let points: Vec<u32> = (0..4 * n.max(1)).map(|_| float_word(rng)).collect();
    let points = t.words(&points);
    t.g32(cam::LIST, list);
    t.g32(cam::LIST_FLAGS + 4 * list, flags);
    t.g32(cam::LIST_COUNTS + 4 * list, n);
    t.g32(cam::LIST_POINTS + 4 * list, points);
}

#[test]
fn camera_debug_lists_match_the_original() {
    let mut rng = Rng(0x0094_1a30);
    Twins::each(|t| {
        for k in 0..24 {
            let n = [0u32, 1, 3, 5][k % 4];
            let sel = if k % 3 == 0 {
                None
            } else {
                Some(rng.below(n.max(1)))
            };
            let list = rng.below(2);
            for addr in [0x0094_1a30u32, 0x0094_2060] {
                t.reset();
                t.stub_ex(
                    cam::SET_RECT,
                    false,
                    2,
                    0,
                    &m5_main3_common::Ret::Seq(vec![0xe1, 0xe2]),
                    &log_pointee(2, 3, 4),
                );
                let pos = fill_arg(
                    1,
                    &[
                        float_word(&mut rng),
                        float_word(&mut rng),
                        float_word(&mut rng),
                    ],
                );
                t.stub_ex(
                    cam::GET_POS_NOW,
                    false,
                    1,
                    0,
                    &m5_main3_common::Ret::Fixed(0xe3),
                    &pos,
                );
                t.g32(cam::CAMERA, 0x0102);
                cam_list(t, &mut rng, list, n, sel);
                let what = format!("camera rect {addr:#x} {k}");
                t.check_framed(&what, addr, Conv::Cdecl, &[], 0x24, |m, ebp| {
                    Out::U32(if addr == 0x0094_1a30 {
                        cam::rect_from_point(m, ebp)
                    } else {
                        cam::rect_keep_y(m, ebp)
                    })
                });
            }
            t.reset();
            // dbgPrintf logs nine words; past each format's own words the
            // caller's stack is cleared
            let float_fmt = t.rebase(cam::DUMP_FORMATS[2]);
            let count_fmt = t.rebase(cam::DUMP_FORMATS[1]);
            let mut p = vec![0x8b, 0x54, 0x24, 0x04, 0x81, 0xfa];
            p.extend(float_fmt.to_le_bytes());
            p.extend([0x74, 7 * 7 + 6 + 2 + 7]); // je past everything
            for w in 0..7u8 {
                p.extend([0xc7, 0x40, 0x10 + 4 * w, 0, 0, 0, 0]);
            }
            p.extend([0x81, 0xfa]);
            p.extend(count_fmt.to_le_bytes());
            p.extend([0x74, 7, 0xc7, 0x40, 0x0c, 0, 0, 0, 0]);
            t.stub_ex(
                cam::DBG_PRINTF,
                false,
                9,
                0,
                &m5_main3_common::Ret::Fixed(0xe4),
                &p,
            );
            cam_list(t, &mut rng, list, n, sel);
            let what = format!("camera dump {k}");
            t.check("camera dump", 0x0094_1ae0, Conv::Cdecl, &[], |m| {
                Out::U32(cam::dump_points(m))
            });
            let _ = what;
            t.reset();
            t.stubs(&[
                (cam::SCE_OPEN, 2, if k % 5 == 4 { 0xffff_ffff } else { 7 }),
                (cam::SCE_CLOSE, 1, 0xe6),
                (cam::DBG_PRINTF, 2, 0xe7),
            ]);
            t.stub_ex(
                cam::SCE_WRITE,
                false,
                3,
                0,
                &m5_main3_common::Ret::Fixed(4),
                &log_pointee(1, 1, 5),
            );
            cam_list(t, &mut rng, list, n, sel);
            if k % 7 == 6 {
                t.g32(cam::LIST_POINTS + 4 * list, 0);
            }
            t.g8(cam::PATH_FILE, if k % 6 == 5 { 0 } else { b'p' });
            let skip = t.skip;
            t.skip = 0;
            let what = format!("camera write {k}");
            t.check_framed(&what, 0x0094_2140, Conv::Cdecl, &[], 0xc, |m, ebp| {
                cam::write_points(m, ebp);
                Out::None
            });
            t.skip = skip;
        }
    });
}

#[test]
fn mat_vec_and_path_names_match_the_original() {
    let mut rng = Rng(0x0094_2260);
    Twins::each(|t| {
        for k in 0..20 {
            t.reset();
            t.stub_ex(
                cam::COPY_VECTOR,
                false,
                1,
                0,
                &m5_main3_common::Ret::Fixed(0xf1),
                &log_pointee(1, 4, 4),
            );
            let m: Vec<u32> = (0..16).map(|_| float_word(&mut rng)).collect();
            let v: Vec<u32> = (0..4).map(|_| float_word(&mut rng)).collect();
            let (m, v) = (t.words(&m), t.words(&v));
            let what = format!("mat vec {k}");
            t.check_framed(
                &what,
                0x0094_2260,
                Conv::Cdecl,
                &[0x1234, m, v],
                0x14,
                |g, ebp| Out::U32(cam::mat_vec(g, 0x1234, m, v, ebp - 0x14)),
            );
        }
        for k in 0..12 {
            t.reset();
            let n = [0u32, 1, 3][k % 3];
            let names: Vec<u32> = (0..n.max(1))
                .map(|i| t.alloc(format!("file{i}.bin\0").as_bytes()))
                .collect();
            let names = t.words(&names);
            // Gu_MakeFileList fills the list: count, 0, names
            let fill = fill_arg(0, &[n, 0, names]);
            t.stub_ex(
                cam::MAKE_FILE_LIST,
                false,
                2,
                0,
                &m5_main3_common::Ret::Fixed(0xf2),
                &fill,
            );
            let buf = t.zeros(0x200 * 12);
            t.stubs(&[
                (cam::USER_FREE, 1, 0xf3),
                (cam::USER_MALLOC, 1, if k == 7 { 0 } else { buf }),
                (cam::MOVIE_ERR, 3, 0xf4),
                (cam::SET_TEXT, 3, 0xf5),
            ]);
            let ms = t.zeros(0x220);
            t.w32(ms + 0x214, rng.pick(&[0, 0x5555]));
            t.g32(cam::NONE_BELOW, 2);
            let page = rng.below(4);
            let controls = [0x102, 0x103, 0x106, 0x100][k % 4];
            let what = format!("set_path_name {k}");
            if k == 7 {
                // a failed allocation: the original copies to address 0
                continue;
            }
            t.check(
                &what,
                0x0094_2360,
                Conv::Cdecl,
                &[0xab00, ms, controls, page],
                |g| {
                    cam::set_path_name(g, 0xab00, ms, controls, page);
                    Out::None
                },
            );
        }
    });
}

// ---- small: texture animation tags, character memory, the movie viewer ----

use ffx_sg::m5main3_small::bridge as small;

#[test]
fn small_helpers_match_the_original() {
    let mut rng = Rng(0x0077_f0e0);
    Twins::each(|t| {
        for _ in 0..8 {
            t.reset();
            t.stub(small::SET_QWORD, false, 4, 0, 0x53);
            let (q, b, c, d) = (rng.next(), rng.next(), rng.next(), rng.next());
            t.check("trxdir", 0x0077_f0e0, Conv::Cdecl, &[q, b], |m| {
                Out::U32(small::trxdir(m, q, b))
            });
            t.check("trxpos", 0x0077_f100, Conv::Cdecl, &[q, b, c, d], |m| {
                Out::U32(small::trxpos(m, q, b, c, d))
            });
            t.check("trxreg", 0x0077_f140, Conv::Cdecl, &[q, b, c], |m| {
                Out::U32(small::trxreg(m, q, b, c))
            });
        }
        for v in [0u32, 0x4000, 0x8000, 0xc000, 0xffff_ffff, 0x7fff] {
            t.reset();
            t.g32(small::GS_SINK, v);
            t.check("gs sink", 0x0077_cf80, Conv::Cdecl, &[], |m| {
                Out::U32(small::gs_sink_ready(m))
            });
        }
        t.reset();
        t.stubs(&[
            (small::USER_MALLOC, 1, 0x00ab_0000),
            (small::SG_REALLOC, 2, 0xa1),
            (small::MEM_SET_STR, 3, 0xa2),
            (small::MEM_SET_MOVE_HEAP, 1, 0xa3),
        ]);
        t.check("Ch_ChrMemHeapSet", 0x0082_6230, Conv::Cdecl, &[], |m| {
            Out::U32(small::chr_mem_heap_set(m))
        });
        for k in [0u32, 5, 39, 40] {
            t.reset();
            t.stubs(&[
                (small::BLK_DISPOSE, 1, 0xb1),
                (small::CACHE_DISPOSE_CHR, 1, 0xb2),
            ]);
            for i in 0..40 {
                t.g32(small::BLOCKS + 0x12c * i, 0x1000 + i);
            }
            let handle = 0x1000 + k;
            let what = format!("Ch_BlkDisposeHandle {k}");
            t.check(&what, 0x0082_59f0, Conv::Cdecl, &[handle], |m| {
                Out::U32(small::blk_dispose_handle(m, handle))
            });
        }
        t.reset();
        t.stubs(&[
            (small::DBG_PRINTF, 2, 0xc1),
            (small::WINDOW_CREATE, 6, 0xc2),
        ]);
        t.check("chr_anim_win", 0x0093_e510, Conv::Cdecl, &[0x77], |m| {
            Out::U32(small::chr_anim_win(m, 0x77))
        });
        t.reset();
        t.stubs(&[(small::MGRP_READ, 2, 0xd1), (small::SET_KEEP_FPS, 1, 0xd2)]);
        t.check(
            "Ch_MgrpRead thunk",
            0x0083_6a40,
            Conv::Cdecl,
            &[3, 4],
            |m| Out::U32(small::thunk(m, small::MGRP_READ, &[3, 4])),
        );
        t.check("Sg_SetKeepFps thunk", 0x0083_9ca0, Conv::Cdecl, &[2], |m| {
            Out::U32(small::thunk(m, small::SET_KEEP_FPS, &[2]))
        });
        for (ok, name) in [
            (1u32, &b"x\0"[..]),
            (0, b"\0"),
            (0, b"file.bin\0"),
            (2, b"\0"),
        ] {
            t.reset();
            m5_main3_common::printf_stub(
                t,
                small::DBG_PRINTF,
                &[(small::READ_ERR_NONAME, 1)],
                2,
                0xe1,
            );
            t.stub(small::MOVIE_ERR, false, 3, 0, 0xe2);
            let s = t.alloc(name);
            t.check("read check", 0x00a4_2f50, Conv::Cdecl, &[ok, s], |m| {
                small::read_check(m, ok, s);
                Out::None
            });
        }
    });
}

#[test]
fn cluster_map_memalign_and_status_match_the_original() {
    let mut rng = Rng(0x0069_a970);
    Twins::each(|t| {
        for n in [0u32, 1, 4] {
            t.reset();
            t.stub(small::ERASE_SUBTREE, true, 1, 1, 0xf1);
            t.stub(small::OPERATOR_DELETE, false, 1, 0, 0xf2);
            // the head (sentinel: byte +0xd set), then a chain of n nodes
            let head = t.zeros(0x10);
            t.w8(head + 0xd, 1);
            let mut next = head;
            for i in 0..n {
                let node = t.words(&[next, 0, 0x7700 + i, 0]);
                next = node;
            }
            t.w32(head + 4, next);
            let map = t.words(&[head, 9]);
            let what = format!("ClusterMap_clear {n}");
            t.check(&what, 0x0069_a970, Conv::Thiscall(map), &[], |m| {
                Out::U32(small::cluster_map_clear(m, map))
            });
        }
        for head in [0x41u32, 0x40, 0x13, 0x10, 0x8, 0x5, 0x4, 0x42, 0x4d, 0xc] {
            t.reset();
            let block = t.zeros(0x50);
            t.w32(block, head);
            t.stub(small::INTERNAL_MEMALIGN, false, 3, 0, block + 4);
            t.stub(small::MEMSET, false, 3, 0, 0x5e);
            let what = format!("mspace_memalign {head:#x}");
            t.check(
                &what,
                0x006e_c8d0,
                Conv::Cdecl,
                &[0x1111, 0x10, 0x20],
                |m| Out::U32(small::mspace_memalign(m, 0x1111, 0x10, 0x20)),
            );
        }
        t.reset();
        t.stub_import(small::GLOBAL_MEMORY_STATUS_EX, 1, 1, 0x1);
        let _ = rng.next();
        t.check_framed(
            "memory status",
            0x0094_37c0,
            Conv::Cdecl,
            &[],
            0x44,
            |m, ebp| Out::U32(small::memory_status(m, ebp - 0x44)),
        );
    });
}

#[test]
fn history_and_viewer_steps_match_the_original() {
    let mut rng = Rng(0x0082_69e0);
    Twins::each(|t| {
        for (next, filled) in [(0u32, 0u32), (5, 3), (0x80, 0x80), (1, 200)] {
            t.reset();
            m5_main3_common::printf_stub(
                t,
                small::DBG_PRINTF,
                &[(small::HISTORY_HEADER, 1)],
                4,
                0x91,
            );
            t.stub(small::GET_FUNC_NAME, false, 1, 0, 0x92);
            for i in 0..0x80u32 {
                let e = small::HISTORY + 16 * i;
                // the entries before `next` (newest first) are filled
                let age = next.wrapping_sub(1).wrapping_sub(i) & 0x7f;
                t.g8(e, u8::from(age < filled));
                t.g32(e + 8, rng.next());
                t.g32(e + 0xc, rng.next());
            }
            t.g32(small::HISTORY_NEXT, next);
            t.check("history dump", 0x0082_69e0, Conv::Cdecl, &[], |m| {
                Out::U32(small::function_history_dump(m))
            });
        }
        for (mode, len) in [(0u32, 0u32), (1, 0x20), (2, 0), (0xffff_ffff, 0x40)] {
            t.reset();
            t.stub(small::DECODE_BLOCKS, false, 4, 0, 0xa1);
            t.stub(small::MOVIE_CAMERA, false, 4, 0, 0xa2);
            let viewer = t.zeros(0x5_0800);
            let stream = t.words(&[
                len, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
            ]);
            t.w32(viewer + 0x5_06c8, mode);
            t.w32(viewer + 0x5_059c, stream);
            t.w32(viewer + 0x5_051c, 640);
            t.g32(small::VIEWER, viewer);
            t.check("viewer audio", 0x0077_c360, Conv::Cdecl, &[], |m| {
                Out::U32(small::viewer_audio(m))
            });
            t.check("viewer camera", 0x0077_c470, Conv::Cdecl, &[], |m| {
                Out::U32(small::viewer_camera(m))
            });
        }
    });
}
