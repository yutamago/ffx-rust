//! M5 finish, WP5 part 7 (wp5h): the developers' save loader
//! (`ffx_system::wp5h_devsave`) against the original, on the image and on
//! the snapshots (`FFX_SNAPSHOT`). Results and every byte written must
//! agree. Needs `FFX_GAME_DIR`.
//!
//! The callees (`FFX.exe` functions and the `MSVCR110` imports) are
//! recording stubs; the ones that fill a buffer write a test string there.

#![allow(clippy::cast_possible_truncation, clippy::similar_names)]

mod m5_main3_common;

use m5_main3_common::{G, Out, Ret, Rng, Twins, fill_arg0, store_arg};

use ffx_oracle::Conv;
use ffx_system::wp5h_devsave::bridge as dv;

/// A cdecl call through the port's oracle.
fn cdecl(m: &mut G, f: u32, args: &[u32]) -> u32 {
    m.0.call(f, Conv::Cdecl, args).unwrap().eax
}

#[test]
fn wp5h_tk_debug_load_viewer_save_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x008b_55e0);
        let saves = [1u32, 5, 73, 110, 0x97, 0, u32::MAX, 0x96, 0x98];
        for k in 0..36u32 {
            t.reset();
            let n = saves[k as usize % saves.len()];
            // a repeat now and then; else another save last
            let repeat = k % 5 == 4;
            let last = if repeat { n } else { n ^ (1 + rng.below(4)) };
            t.g32(dv::VIEWER_SAVE_LAST, last);
            let read = rng.pick(&[0u32, 0, 0, u32::MAX, 1]);
            t.stub(dv::DEBUG_LOAD_FOR_VIEWER, false, 3, 0, read);
            let save = t.rebase(0x0112_ca90);
            t.stub(dv::MS_GET_SAVE_EVENT_ADDRESS, false, 0, 0, save);
            t.stub(dv::MEMCPY, false, 3, 0, save);
            t.stub(dv::ATEL_SET_EVENT_JUMP, false, 2, 0, 0x4a00 + k);
            t.g32(dv::SAVE_JUMP, rng.next());
            t.g32(dv::SAVE_SCENARIO, rng.next());
            let what = format!("TkDebugLoadViewerSave {k} ({n:#x}, read {read:#x})");
            t.check(
                &what,
                dv::TK_DEBUG_LOAD_VIEWER_SAVE,
                Conv::Cdecl,
                &[n],
                |m| {
                    let r = dv::tk_debug_load_viewer_save(m, n, &mut cdecl);
                    // a repeat leaves the caller's eax
                    if repeat { Out::None } else { Out::U32(r) }
                },
            );
            let calls = match (repeat, read) {
                (true, _) => 0,
                (false, 0) => 4,
                _ => 1,
            };
            assert_eq!(t.calls(), calls, "{what}");
        }
    });
}

/// `bytes` padded with NULs to whole words.
fn words_of(bytes: &[u8]) -> Vec<u32> {
    let mut b = bytes.to_vec();
    b.push(0);
    b.resize(b.len().div_ceil(4) * 4, 0);
    b.chunks(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[test]
fn wp5h_debug_load_for_viewer_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0064_9040);
        let numbers: [&[u8]; 5] = [b"7", b"42", b"110", b"-1", b""];
        for k in 0..30u32 {
            t.reset();
            // the folder: the stream prefix and the developers' folder, of
            // every length modulo 4
            let len = rng.below(70) as usize;
            let folder: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
            let mut set_folder = Vec::new();
            for (i, w) in words_of(&folder).into_iter().enumerate() {
                set_folder.extend(store_arg(1, (4 * i) as u8, w));
            }
            t.stub_ex(
                dv::GET_DATA_DIRECTORY,
                false,
                3,
                0,
                &Ret::Fixed(len as u32),
                &set_folder,
            );
            let number = numbers[k as usize % numbers.len()];
            let sprintf = t.stub_code(
                dv::SPRINTF_SLOT,
                false,
                3,
                0,
                &Ret::Fixed(number.len() as u32),
                &fill_arg0(&words_of(number)),
            );
            let slot = t.rebase(dv::SPRINTF_SLOT);
            t.patch(slot, &sprintf.to_le_bytes());
            t.stub(dv::MEMSET, false, 3, 0, 0x5151);
            t.stub(dv::CROSS_ASSERT_OUTPUT, false, 1, 0, 0x5252);
            let file = if k % 3 == 0 { 0 } else { 0x00ab_c000 + k };
            t.stub_import(dv::FOPEN_SLOT, 2, 0, file);
            t.stub_import(dv::FREAD_SLOT, 4, 0, rng.below(0x6900));
            t.stub_import(dv::FCLOSE_SLOT, 1, 0, 0);
            let n = rng.word();
            let buffer = rng.next();
            let size = rng.pick(&[0x6900, 0, 1, u32::MAX]);
            let what = format!("debugLoadForViewer {k} (file {file:#x}, folder {len})");
            t.check_framed(
                &what,
                dv::DEBUG_LOAD_FOR_VIEWER,
                Conv::Cdecl,
                &[n, buffer, size],
                dv::FRAME,
                |m, ebp| {
                    Out::U32(dv::debug_load_for_viewer(
                        m, ebp, n, buffer, size, &mut cdecl,
                    ))
                },
            );
            assert_eq!(t.calls(), 6, "{what}");
        }
    });
}
