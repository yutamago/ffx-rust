//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the chara (ffx-chara) tests. Every state
//! the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs` (`mod common` below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use ffx_oracle::{Oracle, harvest_test};

// chara
harvest_test!(
    ch_set_sequence,
    0x0083_7ac0,
    args = 2,
    |m: &mut Oracle, s| { ffx_chara::motion::bridge::ch_set_sequence(m, s.arg(0), s.arg(1)) }
);
// chara-main-map round 2
harvest_test!(
    ch_plight_calc,
    0x0083_05a0,
    args = 1,
    |m: &mut Oracle, s| {
        // fld [esp+4]; fld [esp+0xc]; call [esp+0x14]; fstp dword [esp+4];
        // mov eax, [esp+4]; ret: a CRT intrinsic on (ST1, ST0)
        const CI2: [u8; 21] = [
            0xdd, 0x44, 0x24, 0x04, 0xdd, 0x44, 0x24, 0x0c, 0xff, 0x54, 0x24, 0x14, 0xd9, 0x5c,
            0x24, 0x04, 0x8b, 0x44, 0x24, 0x04, 0xc3,
        ];
        let ci2 = m.alloc_code(&CI2).unwrap();
        let frame = m.stack_range().start + 0x100;
        let mut call = |m: &mut Oracle, f: u32, a: &[u32]| m.call_cdecl(f, a).unwrap();
        let mut fcall = |m: &mut Oracle, f: u32, a: &[u32]| m.call_f64(f, a).unwrap();
        let mut ci = |m: &mut Oracle, f: u32, a: &[f64]| {
            let mut words: Vec<u32> = a
                .iter()
                .flat_map(|x| {
                    let b = x.to_bits();
                    #[allow(clippy::cast_possible_truncation)]
                    [b as u32, (b >> 32) as u32]
                })
                .collect();
            words.push(f);
            f32::from_bits(m.call(ci2, ffx_oracle::Conv::Cdecl, &words).unwrap().eax)
        };
        let mut c = ffx_chara::draw::bridge::PLightCalls {
            call: &mut call,
            fcall: &mut fcall,
            ci: &mut ci,
        };
        ffx_chara::draw::bridge::ch_plight_calc(m, s.arg(0), frame, &mut c);
    }
);
// small round 2
harvest_test!(
    ch_move_motion,
    0x0083_5bb0,
    args = 1,
    |m: &mut Oracle, s| {
        ffx_chara::motion::bridge::ch_move_motion(
            m,
            s.arg(0),
            &mut |m: &mut Oracle, f, a: &[u32]| m.call_cdecl(f, a).unwrap(),
            &mut |m: &mut Oracle, f, a: &[u32]| m.call_f64(f, a).unwrap(),
        );
    }
);
harvest_test!(ch_mot_event, 0x0083_4350, args = 1, |m: &mut Oracle, s| {
    ffx_chara::motion::bridge::ch_mot_event(m, s.arg(0), &mut |m: &mut Oracle, f, a: &[u32]| {
        m.call_cdecl(f, a).unwrap()
    });
});
