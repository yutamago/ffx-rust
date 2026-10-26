//! Continuation round 4: the list's larger remaining functions
//! (`ffx_prender::p6_tail`) against their originals.

use ffx_bridge::Mem;
use ffx_oracle::Conv;
use ffx_prender::p6_tail as tl;

use super::quads::deref_code;
use super::small::run_framed;
use super::{Out, Rng, Twins, object};

/// `sprintf_s(buf, n, format, x)` in the import slot: logs its four
/// words and ends the text at `buf + (x & 0x3f)`; returns 0x51.
fn sprintf_stub(t: &mut Twins) {
    let mut c = t.log_code(tl::SPRINTF_S, false, 4);
    c.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp+4]
    c.extend([0x0f, 0xb6, 0x54, 0x24, 0x10]); // movzx edx, byte [esp+0x10]
    c.extend([0x83, 0xe2, 0x3f]); // and edx, 0x3f
    c.extend([0xc6, 0x04, 0x10, 0x00]); // mov byte [eax+edx], 0
    c.extend([0xb8, 0x51, 0, 0, 0]); // mov eax, 0x51
    c.push(0xc3);
    let f = t.code(&c);
    let slot = t.rebase(tl::SPRINTF_S);
    t.w32(slot, f);
}

#[test]
fn iggy_warning_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_1201);
        sprintf_stub(&mut t);
        t.stub(tl::TTY_DISPATCH, false, 2, 0, 0x7e57_0001);
        for (i, code) in [0, 399, 400, 401, -1, i32::MIN, i32::MAX, 123, 1000, 7]
            .into_iter()
            .enumerate()
        {
            let a = rng.next();
            let b = rng.next();
            let text = rng.next();
            run_framed(
                &mut t,
                &format!("{name} iggy warning #{i}"),
                0x00a3_e600,
                Conv::Stdcall,
                &[a, b, code as u32, text],
                0x4004,
                // `_alloca_probe` leaves the caller's ecx at ebp - 8
                0x3ffc,
                |g, ebp| tl::bridge::iggy_warning(g, ebp - 0x4004, code, text),
            );
            assert_eq!(t.log_entries(), 3, "{name} #{i}: the calls");
        }
    }
}

/// `PhyreSceneCursor_Seek4(cursor)`: logs the cursor's head, first and
/// type words and its words 0, 4 and 1, then sets words 0, 3 and 4 from the next
/// three words of its table. Returns the table cursor.
fn seek_stub(t: &mut Twins) -> u32 {
    let cursor = t.zeros(4);
    let mut c = t.log_code(tl::CURSOR_SEEK, true, 0);
    for (k, off) in [0x14u8, 0x18, 0x1c].into_iter().enumerate() {
        c.extend([0x8b, 0x51, off]); // mov edx, [ecx+off]
        c.extend([0x89, 0x50, 8 + 4 * k as u8]); // mov [eax+8+4k], edx
    }
    c.extend([0x8b, 0x11, 0x89, 0x50, 0x14]); // mov edx, [ecx]; mov [eax+0x14], edx
    c.extend([0x8b, 0x51, 0x10, 0x89, 0x50, 0x18]); // log [ecx+0x10]
    c.extend([0x8b, 0x51, 0x04, 0x89, 0x50, 0x1c]); // log [ecx+4]
    c.extend([0x8b, 0x15]);
    c.extend(cursor.to_le_bytes()); // mov edx, [cursor]
    c.extend([0x8b, 0x02, 0x89, 0x01]); // mov eax, [edx]; mov [ecx], eax
    c.extend([0x8b, 0x42, 0x04, 0x89, 0x41, 0x0c]); // [ecx+0xc] = [edx+4]
    c.extend([0x8b, 0x42, 0x08, 0x89, 0x41, 0x10]); // [ecx+0x10] = [edx+8]
    c.extend([0x83, 0x05]);
    c.extend(cursor.to_le_bytes());
    c.push(0x0c); // add dword [cursor], 12
    c.push(0xc3);
    let f = t.code(&c);
    t.jump(tl::CURSOR_SEEK, f);
    cursor
}

/// A result that is 0 one time in `n`.
fn mostly_zero(rng: &mut Rng, n: u32) -> u32 {
    if rng.chance(n) { rng.next() | 1 } else { 0 }
}

#[test]
fn bitmap_text_material_initialize_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_1202);
        let pass = t.seq_stub(tl::GET_RENDER_PASS, false, 1, 0);
        let seek = seek_stub(&mut t);
        let register = t.seq_stub(tl::STATE_REGISTER, true, 0, 0);
        let find = t.seq_stub(tl::FIND_PARAMETER, true, 1, 1);
        let set_texture =
            t.seq_stub_with(tl::SET_TEXTURE_CHECKED, true, 2, 2, &deref_code(1, 2, 4));
        let set_value = t.seq_stub(tl::SET_VALUE_DESC, true, 2, 2);
        let create = t.seq_stub(tl::CREATE_MATERIAL, true, 3, 3);
        let color = t.seq_stub_with(tl::SET_TEXT_COLOR, true, 1, 1, &deref_code(0, 1, 3));
        let set_word = t.seq_stub(tl::SET_WORD, true, 2, 2);
        t.stub(tl::MEMCPY, false, 3, 0, 0x3e3c_0001);
        t.stub(tl::FREE, false, 1, 0, 0x3e3c_0002);
        let mut results = std::collections::BTreeSet::new();
        for i in 0..160u32 {
            let this = object(&mut t, &mut rng, 0x10, false);
            let owner = object(&mut t, &mut rng, 9, false);
            t.w32(this + 0x30, owner);
            let flags = rng.next();
            t.w32(this + 0x28, flags);
            let count = match rng.below(5) {
                0 => 1,
                1 => 0,
                2 => rng.below(4) | 0x8000_0000,
                3 => 1 | 0x8000_0000,
                _ => 2 + rng.below(3),
            };
            t.w32(this + 4, count);
            if rng.chance(2) {
                t.w32(this + 8, this + 8);
            }
            let cluster = object(&mut t, &mut rng, 8, false);
            if rng.chance(3) {
                t.w32(cluster + 0x1c, cluster + 0x1c);
            }
            let buffer = object(&mut t, &mut rng, 4, false);
            let entry = object(&mut t, &mut rng, 2, false);
            t.w32(entry + 4, buffer);
            let base = rng.below(0x40);
            let (w0, w4) = if rng.chance(8) {
                (0, 0)
            } else {
                (base, entry.wrapping_sub(base))
            };
            t.seq_set(pass, &[if i % 16 == 0 { 0 } else { rng.next() }]);
            t.seq_set(seek, &[w0, mostly_zero(&mut rng, 8) ^ 1, w4]);
            let r = mostly_zero(&mut rng, 8);
            t.seq_set(register, &[r]);
            let finds = [
                rng.next() & !u32::from(rng.chance(4)),
                rng.next() & !u32::from(rng.chance(4)),
            ];
            let finds = finds.map(|f| if f & 1 == 0 { 0 } else { f });
            t.seq_set(find, &finds);
            let r = mostly_zero(&mut rng, 8);
            t.seq_set(set_texture, &[r]);
            let r = mostly_zero(&mut rng, 8);
            t.seq_set(set_value, &[r]);
            let null = rng.chance(8);
            let material = object(&mut t, &mut rng, 4, null);
            t.seq_set(create, &[material]);
            let r = mostly_zero(&mut rng, 8);
            t.seq_set(color, &[r]);
            let r = mostly_zero(&mut rng, 3);
            t.seq_set(set_word, &[r]);
            let technique = rng.below(8);
            let mut got = 0;
            run_framed(
                &mut t,
                &format!("{name} text material #{i}"),
                0x00a2_9ee0,
                Conv::Thiscall(this),
                &[cluster, technique],
                0x20,
                0x20,
                |g, ebp| {
                    let (r, slot) =
                        tl::material::initialize(g, this, ebp - 0x20, cluster, technique);
                    if let Some(v) = slot {
                        Mem::write(g, ebp + 0xc, v);
                    }
                    got = r;
                    r
                },
            );
            results.insert(got);
        }
        for want in [0, 7, 0xe] {
            assert!(results.contains(&want), "{name}: no result {want:#x}");
        }
        assert!(results.len() > 5, "{name}: {results:x?}");
    }
}

/// A recording stub at `sym` (thiscall, `n` stack words popped) that sets
/// the three words at `ecx` and returns a fourth, from the next 16 bytes
/// of its table. Returns the table cursor.
fn array_stub(t: &mut Twins, sym: u32, n: u32) -> u32 {
    let cursor = t.zeros(4);
    let mut c = t.log_code(sym, true, n);
    c.extend([0x8b, 0x15]);
    c.extend(cursor.to_le_bytes()); // mov edx, [cursor]
    for k in 0..3u8 {
        c.extend([0x8b, 0x42, 4 * k, 0x89, 0x41, 4 * k]); // [ecx+4k] = [edx+4k]
    }
    c.extend([0x8b, 0x42, 0x0c]); // mov eax, [edx+0xc]
    c.extend([0x83, 0x05]);
    c.extend(cursor.to_le_bytes());
    c.push(0x10); // add dword [cursor], 16
    c.push(0xc2);
    c.extend(((4 * n) as u16).to_le_bytes());
    let f = t.code(&c);
    t.jump(sym, f);
    cursor
}

/// Parameter types: every class `0x005a08b0` tells apart.
const TYPES: [u8; 30] = [
    0x42, 0xd9, 0xda, 0xdb, 0xdc, 0x12, 0x43, 0x44, 0x47, 0x48, 0x49, 0x14, 0x15, 0x4b, 0x4c, 0x4a,
    0x13, 0x90, 0x91, 0xde, 0xe1, 0xe2, 0x4d, 0x4e, 0x00, 0x01, 0x05, 0x20, 0x45, 0x30,
];

#[test]
fn layout_build_records_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_1203);
        let malloc = t.seq_stub(tl::MALLOC, false, 2, 0);
        t.stub(tl::FREE, false, 1, 0, 0x1a70_0001);
        let format_size = t.seq_stub(tl::FORMAT_SIZE, false, 1, 0);
        let (size_fn, sizes) = t.seq_code(0x7700_0002, true, 1, 1);
        let (align_fn, aligns) = t.seq_code(0x7700_0003, true, 1, 1);
        let vt = t.zeros(16);
        t.w32(vt + 8, size_fn);
        t.w32(vt + 12, align_fn);
        for i in 0..48u32 {
            let n = match i % 8 {
                0 => 0,
                _ => 1 + rng.below(12),
            };
            let params = if i % 16 == 5 || i == 8 {
                0
            } else {
                let p = t.zeros(16 * n as usize + 16);
                for k in 0..n {
                    let ty = TYPES[rng.below(TYPES.len() as u32) as usize];
                    let count = if rng.chance(2) { 0 } else { rng.below(0x40) };
                    let format = rng.below(0x60) as u8;
                    t.write(p + 16 * k, &(count as u16).to_le_bytes());
                    t.write(p + 16 * k + 2, &[ty, format]);
                }
                p
            };
            let records = object(&mut t, &mut rng, 3, false);
            let head = match rng.below(4) {
                0 => n,
                1 => n | 0x8000_0000,
                _ => rng.below(8) | (rng.next() & 0x8000_0000),
            };
            t.w32(records, head);
            let null = rng.chance(5);
            let old = object(&mut t, &mut rng, 3 * 16, null);
            t.w32(records + 4, old);
            let data = if rng.chance(8) && n != 0 {
                0
            } else {
                t.zeros(12 * n as usize + 12)
            };
            if head == n {
                t.w32(records + 4, data.max(old));
            } else if i % 6 == 4 {
                // the allocator hands back the old (owned) block
                t.w32(records + 4, data);
                t.w32(records, n + 1);
            }
            t.seq_set(malloc, &[data]);
            let s: Vec<u32> = (0..n).map(|_| rng.next() & 0x1_ffff).collect();
            t.seq_set(sizes, &s);
            let a: Vec<u32> = (0..n).map(|_| 1 << rng.below(6)).collect();
            t.seq_set(aligns, &a);
            let f: Vec<u32> = (0..n)
                .map(|_| match rng.below(4) {
                    0 => 0,
                    1 => rng.next(),
                    _ => rng.below(0x400),
                })
                .collect();
            t.seq_set(format_size, &f);
            let sizer = t.zeros(8);
            t.w32(sizer, vt);
            t.reset_log();
            t.check(
                &format!("{name} build records #{i}"),
                tl::BUILD_RECORDS,
                Conv::Thiscall(records),
                &[params, n, sizer],
                |g| Out::U32(tl::layout::build_records(g, records, params, n, sizer)),
            );
        }
    }
}

#[test]
fn layout_pack_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_1204);
        let qsort = t.stub_code(tl::QSORT, false, 4, 0, 0x5047);
        let slot = t.rebase(tl::QSORT);
        t.w32(slot, qsort);
        for i in 0..64u32 {
            let n = if i % 9 == 0 { 0 } else { 1 + rng.below(20) };
            let data = t.zeros(12 * n as usize + 12);
            for k in 0..n {
                let r = data + 12 * k;
                t.w32(r, rng.next());
                let size = match rng.below(5) {
                    0 => 4 << rng.below(3),
                    1 => rng.below(8),
                    2 => 0,
                    _ => rng.below(0x40),
                };
                t.write(r + 2, &(size as u16).to_le_bytes());
                let align = match rng.below(6) {
                    0 => 0,
                    1 => rng.next(),
                    _ => 1 << rng.below(5),
                };
                t.w32(r + 4, align);
                t.w32(r + 8, k);
            }
            let records = object(&mut t, &mut rng, 3, false);
            t.w32(records, n | (rng.next() & 0x8000_0000));
            t.w32(records + 4, data);
            let start = if rng.chance(3) { 0 } else { rng.below(0x40) };
            let below = (0x28 + 4 * n + 0x1f) & !0xf;
            let mut array = 0;
            run_framed(
                &mut t,
                &format!("{name} pack #{i}"),
                tl::PACK_RECORDS,
                Conv::Thiscall(records),
                &[start],
                below,
                0,
                |g, ebp| {
                    tl::layout::pack(g, records, start, |m| {
                        array = (ebp - 0x28 - 4 * m) & !0xf;
                        array
                    })
                },
            );
            let fa = ffx_oracle::Oracle::read(&mut t.a, array, 4 * n as usize).unwrap();
            let fb = ffx_oracle::Oracle::read(&mut t.b, array, 4 * n as usize).unwrap();
            assert_eq!(fa, fb, "{name} pack #{i}: the pointer array");
        }
    }
}

#[test]
fn layout_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_1205);
        // a descriptor past the records reads its offset and size at 0
        t.a.map_zeroed(0, 0x1000).unwrap();
        t.b.map_zeroed(0, 0x1000).unwrap();
        let build = array_stub(&mut t, tl::BUILD_RECORDS, 3);
        let pack = array_stub(&mut t, tl::PACK_RECORDS, 1);
        t.stub(tl::FREE, false, 1, 0, 0x1a70_0002);
        for i in 0..40u32 {
            let n = if i % 10 == 0 { 0 } else { 1 + rng.below(10) };
            let params = object(&mut t, &mut rng, 4 * n + 4, false);
            let count = if i % 5 == 3 && n > 0 {
                n - 1
            } else {
                n + rng.below(3)
            };
            // one record more than the count: a descriptor past it must
            // read the zero page, not this
            let data = object(&mut t, &mut rng, 3 * count + 3, false);
            let head = count | (rng.next() & 0x8000_0000);
            let data = if rng.chance(8) { 0 } else { data };
            let n = if data == 0 { 0 } else { n };
            let e1 = if i % 4 == 1 { rng.next() | 1 } else { 0 };
            let e2 = if i % 4 == 2 { rng.next() | 1 } else { 0 };
            t.seq_set(build, &[head, data, rng.next(), e1]);
            t.seq_set(pack, &[head, data, rng.next(), e2]);
            let end = t.zeros(4);
            let (base, sizer) = (rng.next(), rng.next());
            run_framed(
                &mut t,
                &format!("{name} layout #{i}"),
                0x005a_0600,
                Conv::Cdecl,
                &[base, params, n, end, sizer],
                0x1c,
                0xc,
                |g, ebp| tl::layout::layout(g, ebp - 0x1c, [base, params, n, end, sizer]),
            );
        }
    }
}

/// A recording stdcall stub of `n` words that copies `words` words from
/// its table into the pointer in argument `arg` and returns the word
/// before them (each call takes `1 + words` words of the table). Returns
/// its address and the table cursor.
fn fill_code(t: &mut Twins, tag: u32, n: u32, arg: u32, words: u32) -> (u32, u32) {
    let cursor = t.zeros(4);
    let mut c = t.log_code(tag, false, n);
    c.extend([0x8b, 0x15]);
    c.extend(cursor.to_le_bytes()); // mov edx, [cursor]
    c.extend([0x8b, 0x4c, 0x24, (4 + 4 * arg) as u8]); // mov ecx, [esp+4+4arg]
    for w in 0..words {
        c.extend([0x8b, 0x42, (4 + 4 * w) as u8]); // mov eax, [edx+4+4w]
        c.extend([0x89, 0x41, (4 * w) as u8]); // mov [ecx+4w], eax
    }
    c.extend([0x8b, 0x02]); // mov eax, [edx]
    c.extend([0x83, 0x05]);
    c.extend(cursor.to_le_bytes());
    c.push((4 + 4 * words) as u8); // add dword [cursor], 4 + 4 words
    c.push(0xc2);
    c.extend(((4 * n) as u16).to_le_bytes());
    (t.code(&c), cursor)
}

/// [`fill_code`] in the import slot `slot`.
fn fill_import(t: &mut Twins, slot: u32, n: u32, arg: u32, words: u32) -> u32 {
    let (f, cursor) = fill_code(t, slot, n, arg, words);
    let at = t.rebase(slot);
    t.w32(at, f);
    cursor
}

#[test]
fn mouse_update_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_1206);
        let cursor_pos = fill_import(&mut t, tl::GET_CURSOR_POS, 1, 0, 2);
        let window = fill_import(&mut t, tl::GET_WINDOW_RECT, 2, 1, 4);
        let client = fill_import(&mut t, tl::GET_CLIENT_RECT, 2, 1, 4);
        let info = fill_import(&mut t, tl::GET_WINDOW_INFO, 2, 1, 10);
        let metrics = fill_import(&mut t, tl::GET_SYSTEM_METRICS, 1, 0, 0);
        t.import_stub(tl::CLIP_CURSOR, 1, 1);
        t.stub(tl::DEVICE_UPDATE, true, 0, 0, 0x0d5e_0001);
        // the state's last word is left as it was (the frame shows the
        // original's zeroing)
        let (get_state, states) = fill_code(&mut t, 0x7700_0009, 3, 2, 4);
        let (acquire, acquires) = fill_code(&mut t, 0x7700_0007, 1, 0, 0);
        let vt = t.zeros(0x40);
        t.w32(vt + 0x24, get_state);
        t.w32(vt + 0x1c, acquire);
        let hwnd = t.rebase(tl::HWND);
        t.w32(hwnd, 0x0001_0203);
        let settings = object(&mut t, &mut rng, 4, false);
        let at = t.rebase(tl::SETTINGS);
        t.w32(at, settings);
        let lost = tl::INPUT_LOST;
        for i in 0..96u32 {
            let mouse = object(&mut t, &mut rng, 0x48, false);
            let dev = if i % 24 == 0 {
                0
            } else {
                let d = object(&mut t, &mut rng, 2, false);
                let com = t.zeros(4);
                t.w32(com, vt);
                t.w32(d + 4, com);
                t.write(d, &[[0u8, 1, 2][rng.below(3) as usize]]);
                d
            };
            t.w32(mouse + 0x14, dev);
            t.write(mouse + 0x108, &[[1u8, 1, 1, 0, 2][rng.below(5) as usize]]);
            for k in 0..3 {
                t.write(mouse + 0x118 + k, &[[0u8, 1, 0x80][rng.below(3) as usize]]);
            }
            t.w32(settings + 8, rng.below(3));
            let z = [0, 1, 120, (-1i32) as u32, rng.next()][rng.below(5) as usize];
            let buttons = rng.next() & 0x8080_8080;
            let state = [
                match i % 8 {
                    3 => 0x8000_4005,
                    5 => 1,
                    _ => 0,
                },
                rng.next() % 64,
                rng.next() % 64,
                z,
                buttons,
                rng.next(),
            ];
            t.seq_set(states, &state);
            let again = rng.below(2) * lost;
            t.seq_set(acquires, &[lost, lost, again, 0]);
            let (w, h) = (100 + rng.below(50), 80 + rng.below(40));
            let (left, top) = (rng.below(40), rng.below(40));
            let ok = |r: &mut Rng| u32::from(!r.chance(8));
            let r = ok(&mut rng);
            let style = rng.next() & 0x00c0_0000;
            let (mut px, mut py) = (left + rng.below(w + 40), top + rng.below(h + 60));
            if style == 0 {
                // the client area's edges
                px = left + [0, w - 1, w, rng.below(w)][rng.below(4) as usize];
                py = top + [0, h - 1, h, rng.below(h)][rng.below(4) as usize];
            }
            t.seq_set(cursor_pos, &[r, px, py]);
            let r = ok(&mut rng);
            t.seq_set(window, &[r, left, top, left + w + 20, top + h + 40]);
            let r = ok(&mut rng);
            t.seq_set(client, &[r, 0, 0, w, h]);
            let mut words = vec![1, rng.next()];
            words.extend([0; 8]);
            words.push(style);
            t.seq_set(info, &words);
            let m = [rng.below(8), 0, rng.below(8), 0, 10 + rng.below(20), 0];
            t.seq_set(metrics, &m);
            run_framed(
                &mut t,
                &format!("{name} mouse #{i}"),
                0x0062_d6f0,
                Conv::Thiscall(mouse),
                &[],
                0x84,
                0x80,
                |g, ebp| {
                    let r = tl::mouse::update(g, mouse, ebp - 0x84);
                    if dev == 0 {
                        // the original's eax is its cookie word: garbage
                        let cookie = Mem::rebase(g, 0x00c6_13d8);
                        Mem::read::<u32>(g, cookie) ^ ebp
                    } else {
                        r
                    }
                },
            );
        }
    }
}
