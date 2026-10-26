//! Continuation round 5: `PApplication::frame` (`ffx_prender::p6_appframe`)
//! against its original, with the port's frame at the original's `ebp`
//! (the render scope locks are frame slots the stubs log).

#![allow(clippy::cast_precision_loss)] // small test values

use ffx_oracle::Conv;
use ffx_prender::p6_appframe as af;

use super::small::{fld, run_framed};
use super::{Rng, Twins};

/// A recording stub whose `eax` is the word at `cell` (logs `tag`, `ecx`
/// and `n` stack words; pops `pops`).
pub(super) fn cell_stub(t: &mut Twins, tag: u32, n: u32, pops: u32, cell: u32) -> u32 {
    cell_stub_with(t, tag, true, n, pops, cell)
}

/// [`cell_stub`], logging `ecx` only when `this` (a cdecl callee's
/// `ecx` is the caller's scratch).
pub(super) fn cell_stub_with(
    t: &mut Twins,
    tag: u32,
    this: bool,
    n: u32,
    pops: u32,
    cell: u32,
) -> u32 {
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(tag.to_le_bytes()); // mov [eax], tag
    if this {
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
    }
    for i in 0..n {
        c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.push(0xa1);
    c.extend(cell.to_le_bytes()); // mov eax, [cell]
    if pops == 0 {
        c.push(0xc3);
    } else {
        c.push(0xc2);
        c.extend(((pops * 4) as u16).to_le_bytes());
    }
    t.code(&c)
}

/// `PhyrePrintf`'s stand-in (cdecl): logs its first three words, and six
/// for level 2 (the frame rate's two doubles).
fn printf_stub(t: &mut Twins) -> u32 {
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(af::PRINTF.to_le_bytes()); // mov [eax], tag
    for i in 0..3u8 {
        c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x50, 8 + 4 * i]); // mov [eax+8+4i], edx
    }
    c.extend([0x83, 0x7c, 0x24, 0x04, 0x02]); // cmp dword [esp+4], 2
    c.extend([0x75, 21]); // jne skip
    for i in 3..6u8 {
        c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]);
        c.extend([0x89, 0x50, 8 + 4 * i]);
    }
    c.extend([0xff, 0x05]); // skip: inc dword [log]
    c.extend(log.to_le_bytes());
    c.extend([0x31, 0xc0, 0xc3]); // xor eax, eax; ret
    t.code(&c)
}

/// A vtable of `n` slots, slot `k` a [`cell_stub`] (no words) on cell
/// `cells + 4 k`, tagged `tag + k`.
fn vtable(t: &mut Twins, tag: u32, n: u32, cells: u32) -> u32 {
    let table = t.zeros(4 * n as usize);
    for k in 0..n {
        let s = cell_stub(t, tag + k, 0, 0, cells + 4 * k);
        t.w32(table + 4 * k, s);
    }
    table
}

/// Float bits for the clock fields: ordinary values, zeros, negatives
/// and quiet NaNs (no signalling NaNs: the emulator keeps them).
fn clock_bits(rng: &mut Rng) -> u32 {
    match rng.below(10) {
        0 => 0,
        1 => 0x8000_0000,
        2 => 0x7fc0_0000 | (rng.next() & 0xffff),
        3 => (-(rng.below(100) as f32) / 7.0).to_bits(),
        _ => (rng.below(4000) as f32 / 1000.0).to_bits(),
    }
}

#[test]
fn application_frame_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_7940);
        // return cells: app slots 0..10, renderer slots 0..2, view slot 3,
        // the context, the flip, the clock
        let app_cells = t.zeros(4 * 11);
        let ren_cells = t.zeros(4 * 3);
        let view_cells = t.zeros(4 * 4);
        let ctx_cell = t.zeros(4);
        let flip_cell = t.zeros(4);
        let clock_cell = t.zeros(8);
        let app_vt = vtable(&mut t, 0xa000, 11, app_cells);
        let ren_vt = vtable(&mut t, 0xb000, 3, ren_cells);
        let view_vt = vtable(&mut t, 0xc000, 4, view_cells);
        // GetTime: the double at clock_cell in ST0
        let mut c = fld(clock_cell, true);
        c.push(0xc3);
        let get_time = t.code(&c);
        t.jump(af::GET_TIME, get_time);
        let pf = printf_stub(&mut t);
        t.jump(af::PRINTF, pf);
        t.stub(af::PROFILER_RESET, true, 2, 2, 0x11);
        t.stub(af::LOG_KEYBOARD, true, 0, 0, 0x12);
        t.stub(af::LATCH_BUTTONS, true, 0, 0, 0x13);
        t.stub(af::TTY_WRITE, false, 4, 0, 0x14);
        t.stub(af::SYNC_RENDER, true, 0, 0, 0x15);
        t.stub(af::LOCK_ENTER, true, 0, 0, 0x16);
        let ctx = cell_stub(&mut t, af::CONTEXT, 0, 0, ctx_cell);
        t.jump(af::CONTEXT, ctx);
        t.stub(af::LOCK_LEAVE, true, 0, 0, 0x17);
        t.stub(af::SET_TEXTURE_QUALITY, true, 1, 1, 0x18);
        t.stub(af::SET_ANISOTROPY, false, 1, 0, 0x19);
        t.stub(af::SET_DISPLAY_MODE, true, 2, 2, 0x1a);
        t.stub(af::SET_VSYNC, true, 1, 1, 0x1b);
        t.stub(af::SET_VIEWPORT, true, 4, 4, 0x1c);
        let flip = cell_stub(&mut t, af::FLIP, 0, 0, flip_cell);
        t.jump(af::FLIP, flip);
        let settings = t.zeros(0x40);
        let settings_at = t.rebase(af::SETTINGS);
        t.w32(settings_at, settings);
        for case in 0..400u32 {
            let app = t.zeros(0x2d0);
            t.w32(app, app_vt);
            t.w32(app + af::RENDERER, ren_vt);
            for k in [0x20, 0x24, 0x28, 0x3c] {
                let v = clock_bits(&mut rng);
                t.w32(app + k, v);
            }
            // the fixed step: above 0 in a third of the cases
            if case % 3 == 0 {
                let v = (1 + rng.below(50)) as f32 / 1000.0;
                t.w32(app + 0x3c, v.to_bits());
            }
            let quit = u8::from(rng.chance(16));
            t.write(app + 0x39, &[quit]);
            let first = if rng.chance(4) { 0 } else { rng.next() };
            t.w32(app + 0x34, first);
            t.write(app + 0x288, &[(rng.below(2) * 0x41) as u8]);
            let kb = if rng.chance(2) { 0 } else { rng.next() };
            t.w32(app + 0x29c, kb);
            t.w32(app + 0x2b8, 0x00c0_0000 | rng.below(0x1000));
            // pending settings
            for k in 0x14..=0x1d {
                let b = if rng.chance(3) { 1 + rng.below(255) } else { 0 };
                t.write(app + k, &[b as u8]);
            }
            for k in 0..4 {
                let v = rng.next();
                t.w32(app + af::RENDERER + 8 + 4 * k, v);
            }
            // the viewports
            let views = rng.below(4);
            let list = t.zeros(16);
            for k in 0..4 {
                let v = t.zeros(4);
                t.w32(v, view_vt);
                t.w32(list + 4 * k, v);
            }
            // a count past 0xffff (bit 16) in a few cases: the first view
            // fails then (below), so the loop stops there
            let wide = case % 25 == 7;
            let count = if wide { 0x1_0000 } else { views };
            t.w32(app + 0x2bc, count | (rng.next() & 0x8000_0000));
            t.w32(app + 0x2c0, list);
            // return values: mostly 0
            let err = |rng: &mut Rng| if rng.chance(12) { 1 + rng.below(9) } else { 0 };
            for k in 0..11 {
                let v = err(&mut rng);
                t.w32(app_cells + 4 * k, v);
            }
            for k in 0..3 {
                let v = err(&mut rng);
                t.w32(ren_cells + 4 * k, v);
            }
            let v = if wide { 5 } else { err(&mut rng) };
            t.w32(view_cells + 12, v);
            let v = err(&mut rng);
            t.w32(flip_cell, v);
            let v = if rng.chance(4) {
                0
            } else {
                0x00c1_0000 | rng.below(0x100)
            };
            t.w32(ctx_cell, v);
            // the clock and the counters
            // the clock: exact sums, or full doubles (the subtractions'
            // order rounds differently)
            let (now, s) = if rng.chance(2) {
                (
                    f64::from(rng.below(100_000)) / 64.0,
                    f64::from(rng.below(1000)) / 8.0,
                )
            } else {
                (
                    1000.0 + f64::from(rng.next()) / 3.7,
                    f64::from(rng.next()) / 17.3,
                )
            };
            t.write(clock_cell, &now.to_bits().to_le_bytes());
            let start = t.rebase(af::APP_START_TIME);
            t.write(start, &s.to_bits().to_le_bytes());
            let rate_time = t.rebase(af::RATE_TIME);
            let acc = match rng.below(4) {
                0 => clock_bits(&mut rng),
                // with an elapsed time of 0.5, exactly the interval
                1 => 1.5f32.to_bits(),
                _ => (rng.below(3500) as f32 / 1000.0).to_bits(),
            };
            t.w32(rate_time, acc);
            if acc == 1.5f32.to_bits() && rng.chance(2) {
                t.w32(app + 0x24, 0.5f32.to_bits());
                t.w32(app + 0x3c, 0.5f32.to_bits());
            }
            let rate_frames = t.rebase(af::RATE_FRAMES);
            let n = match rng.below(5) {
                0 => u32::MAX - rng.below(3),
                1 => 0x7fff_ffff,
                // 2^24 + 1 frames: the count rounds to f32 before the
                // division
                2 => 0x100_0000 + rng.below(4),
                _ => rng.below(200),
            };
            t.w32(rate_frames, n);
            for k in (0..0x40).step_by(4) {
                let v = match rng.below(3) {
                    0 => 0,
                    1 => rng.next() & 0xffff,
                    _ => rng.next(),
                };
                t.w32(settings + k, v);
            }
            run_framed(
                &mut t,
                &format!("{name} frame case {case}"),
                0x0062_7940,
                Conv::Thiscall(app),
                &[],
                0x40,
                0,
                |g, ebp| af::bridge::frame(g, app, ebp),
            );
        }
    }
}
