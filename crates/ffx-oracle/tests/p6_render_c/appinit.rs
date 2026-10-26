//! Continuation round 5: `PApplication::onInit` (`ffx_prender::p6_appinit`)
//! against its original, with the port's frame at the original's `ebp`.

#![allow(clippy::cast_precision_loss)] // small test values

use ffx_oracle::Conv;
use ffx_prender::p6_appinit as ai;

use super::appframe::{cell_stub, cell_stub_with};
use super::small::{fld, run_framed};
use super::{Rng, Twins};

/// `sprintf_s`'s stand-in (cdecl): logs five words and writes "ab" at
/// the first when it is not null.
fn sprintf_stub(t: &mut Twins) -> u32 {
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(ai::SPRINTF_S.to_le_bytes()); // mov [eax], tag
    for i in 0..5u8 {
        c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x50, 8 + 4 * i]); // mov [eax+8+4i], edx
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp+4]
    c.extend([0x85, 0xc9, 0x74, 0x06]); // test ecx, ecx; je +6
    c.extend([0xc7, 0x01, 0x61, 0x62, 0x00, 0x00]); // mov dword [ecx], "ab"
    c.extend([0x31, 0xc0, 0xc3]); // xor eax, eax; ret
    t.code(&c)
}

/// `PhyrePrintf`'s stand-in (cdecl): logs three words, four for the
/// format at `four` (the step message's double).
fn printf_stub(t: &mut Twins, four: u32) -> u32 {
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes());
    c.extend([0xc1, 0xe0, 0x05]);
    c.push(0x05);
    c.extend((log + 4).to_le_bytes());
    c.extend([0xc7, 0x00]);
    c.extend(ai::PRINTF.to_le_bytes());
    for i in 0..3u8 {
        c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]);
        c.extend([0x89, 0x50, 8 + 4 * i]);
    }
    c.extend([0x81, 0x7c, 0x24, 0x08]); // cmp dword [esp+8], four
    c.extend(four.to_le_bytes());
    c.extend([0x75, 0x07]); // jne skip
    c.extend([0x8b, 0x54, 0x24, 0x10, 0x89, 0x50, 0x14]); // the fourth word
    c.extend([0xff, 0x05]); // skip: inc dword [log]
    c.extend(log.to_le_bytes());
    c.extend([0x31, 0xc0, 0xc3]);
    t.code(&c)
}

#[test]
#[allow(clippy::too_many_lines)] // one original's set-up
fn application_on_init_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_a600);
        // the cells the stubs return
        let cells = t.zeros(4 * 24);
        let cell = |k: u32| cells + 4 * k;
        let clock_cell = t.zeros(8);
        let mut c = fld(clock_cell, true);
        c.push(0xc3);
        let get_time = t.code(&c);
        t.jump(ai::GET_TIME, get_time);
        let fmt = t.rebase(ai::STEP_MESSAGE);
        let pf = printf_stub(&mut t, fmt);
        t.jump(ai::PRINTF, pf);
        let sp = sprintf_stub(&mut t);
        let slot = t.rebase(ai::SPRINTF_S);
        t.w32(slot, sp);
        // (callee, logged words, popped words, its result cell)
        let stubs: [(u32, u32, u32, u32); 25] = [
            (ai::OPERATOR_NEW, 1, 0, 0),
            (ai::PROXY_FROM_BUFFER, 1, 1, 1),
            (ai::REGISTER_COMMANDS, 0, 0, 1),
            (ai::SERVER_CONNECT, 1, 0, 2),
            (ai::SERVER_SEND, 2, 2, 1),
            (ai::FILE_NUMBER, 1, 0, 3),
            (ai::MALLOC, 2, 0, 4),
            (ai::FREE, 1, 0, 1),
            (ai::LOCK_ENTER, 0, 0, 1),
            (ai::LOCK_LEAVE, 0, 0, 1),
            (ai::RENDERER_CTOR, 0, 0, 1),
            (ai::ATEXIT, 1, 0, 1),
            (ai::ALLOCATION_CTOR, 0, 0, 1),
            (ai::INIT_WINDOW, 1, 1, 5),
            (ai::SET_TEXTURE_QUALITY, 1, 1, 1),
            (ai::WORLD_INIT, 0, 0, 6),
            (ai::THREAD_INSTANCE, 0, 0, 7),
            (ai::THREAD_INIT, 0, 0, 8),
            (ai::INIT_DIRECT_INPUT, 0, 0, 9),
            (ai::RENDERER_INIT, 3, 3, 10),
            (ai::RING, 0, 0, 11),
            (ai::RING_INIT, 2, 2, 1),
            (ai::THREAD_COUNT, 0, 0, 12),
            (ai::POOL_INIT, 2, 2, 13),
            (ai::TIMELINE_INIT, 2, 2, 1),
        ];
        // cdecl callees don't log ecx
        let cdecl = [
            ai::OPERATOR_NEW,
            ai::SERVER_CONNECT,
            ai::FILE_NUMBER,
            ai::MALLOC,
            ai::FREE,
            ai::ATEXIT,
            ai::THREAD_INSTANCE,
            ai::INIT_DIRECT_INPUT,
            ai::RING,
            ai::THREAD_COUNT,
        ];
        for (f, n, pops, k) in stubs {
            let s = cell_stub_with(&mut t, f, !cdecl.contains(&f), n, pops, cell(k));
            t.jump(f, s);
        }
        // stepTimer moves the clock: [ecx + 0x20] = the word at cell 17
        let mut c = vec![0xa1];
        c.extend(cell(17).to_le_bytes()); // mov eax, [cell 17]
        c.extend([0x89, 0x41, 0x20, 0xc3]); // mov [ecx+0x20], eax; ret
        let step_timer = t.code(&c);
        t.jump(ai::STEP_TIMER, step_timer);
        for (f, n, pops, k) in [(ai::TIMELINE_ADD, 1, 1, 14), (ai::SETUP_INPUTS, 0, 0, 1)] {
            let s = cell_stub(&mut t, f, n, pops, cell(k));
            t.jump(f, s);
        }
        // the application's vtable: slot 2 (two words) and slot 3
        let vt = t.zeros(16);
        let s2 = cell_stub(&mut t, 0xa002, 2, 2, cell(15));
        let s3 = cell_stub(&mut t, 0xa003, 0, 0, cell(16));
        t.w32(vt + 8, s2);
        t.w32(vt + 12, s3);
        let settings = t.zeros(0x40);
        let settings_at = t.rebase(ai::SETTINGS);
        t.w32(settings_at, settings);
        let file_name = t.alloc(b"ffx_script_name\0");
        for case in 0..300u32 {
            let app = t.zeros(0x2e0);
            t.w32(app, vt);
            for k in (0x20..0x40).step_by(4) {
                let v = rng.next() & 0x3fff_ffff;
                t.w32(app + k, v);
            }
            for k in (0x2a0..0x2b8).step_by(4) {
                let v = if rng.chance(2) { 0 } else { rng.next() };
                t.w32(app + k, v);
            }
            let v = rng.next();
            t.w32(app + 0x264, v);
            t.w32(app + 0x2b8, 0x00c0_1000 | rng.below(0x100));
            // the script: buffer, server, file (each set or not)
            let pick = |rng: &mut Rng, v: u32| if rng.chance(3) { v } else { 0 };
            let v = pick(&mut rng, 0x00c0_2000);
            let buffer = t.rebase(ai::SCRIPT_BUFFER);
            t.w32(buffer, v);
            let v = pick(&mut rng, 0x00c0_3000);
            let server = t.rebase(ai::SCRIPT_SERVER);
            t.w32(server, v);
            let v = if rng.chance(2) { file_name } else { 0 };
            let file = t.rebase(ai::SCRIPT_FILE);
            t.w32(file, v);
            // the fixed step: 0 in some cases
            let step = if rng.chance(3) {
                0
            } else if rng.chance(4) {
                0x8000_0000
            } else {
                (rng.below(100) as f32 / 1000.0).to_bits()
            };
            let fixed = t.rebase(ai::FIXED_STEP);
            t.w32(fixed, step);
            // the proxy (operator new) and its command count
            let proxy = if rng.chance(5) { 0 } else { t.zeros(0x28) };
            if proxy != 0 && rng.chance(2) {
                t.w32(proxy + 8, 1);
            }
            t.w32(cell(0), proxy);
            let srv = if rng.chance(4) { 0 } else { 0x00c0_4000 };
            t.w32(cell(2), srv);
            let v = rng.below(20);
            t.w32(cell(3), v);
            // PhyreMalloc: a block or none; its size's top bit is never
            // set here (the string's length is small)
            let block = if rng.chance(8) { 0 } else { t.zeros(0x40) };
            t.w32(cell(4), block);
            // the renderer singleton built or not, windowed or not
            let guard = t.rebase(ai::RENDERER_GUARD);
            let g = rng.next() & !1 | u32::from(rng.chance(2));
            t.w32(guard, g);
            let renderer = t.rebase(ai::RENDERER);
            t.write(renderer, &[(rng.below(2)) as u8]);
            // failures: mostly none
            for k in [5u32, 6, 8, 9, 10, 13, 14, 15, 16] {
                let v = if rng.chance(14) { 1 + rng.below(9) } else { 0 };
                t.w32(cell(k), v);
            }
            t.w32(cell(7), 0x00c0_5000);
            t.w32(cell(11), 0x00c0_6000);
            let v = rng.below(4);
            t.w32(cell(12), v);
            let v = match rng.below(4) {
                0 => 0x8000_0000,
                1 => 0x7fc0_0000 | rng.below(0x100),
                _ => (rng.below(5000) as f32 / 300.0).to_bits(),
            };
            t.w32(cell(17), v);
            let now = 1000.0 + f64::from(rng.next()) / 3.7;
            t.write(clock_cell, &now.to_bits().to_le_bytes());
            for k in (0..0x40).step_by(4) {
                let v = rng.next();
                t.w32(settings + k, v);
            }
            let (a, b) = (rng.next(), rng.next());
            run_framed(
                &mut t,
                &format!("{name} onInit case {case}"),
                0x0062_a600,
                Conv::Thiscall(app),
                &[a, b],
                // the allocation and the file name string, ebp - 0x128 ..
                // ebp - 0x118 (the words above are the original's
                // temporaries)
                0x128,
                0x10,
                |g, ebp| ai::bridge::on_init(g, app, a, b, ebp),
            );
        }
    }
}
