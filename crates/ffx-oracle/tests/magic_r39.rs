//! Round 39's ports (`ffx_magic::r39`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines
)]

mod magic;
#[path = "magic/ownb.rs"]
mod ownb;
#[path = "magic/summon_stubs.rs"]
mod summon_stubs;

use magic::{Call, Eax, SCRATCH, Twin};
use ownb::rec;
use summon_stubs::all_stubs;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// The control words of [`scene_task_0519`]'s stubs: slot 261's key (the
/// key that answers, 8 for none), the busy mode (0 never, 1 from the
/// start, 2 set by slot 222), and slot 225's and 227's answers.
const KEY: u32 = SCRATCH + 0x7f00;
const MODE: u32 = SCRATCH + 0x7f04;
const A225: u32 = SCRATCH + 0x7f08;
const A227: u32 = SCRATCH + 0x7f0c;
const BUSY: u32 = SCRATCH + 0x7f10;
const LOCK: u32 = SCRATCH + 0x7f14;
const HOLDER: u32 = SCRATCH + 0x7f18;
const TABLE: u32 = SCRATCH + 0x7e00;
const SINES: u32 = SCRATCH + 0x1_0000;

/// One case of [`scene_task_0519`]: the globals by operand (on top of the
/// defaults), the key slot 261 answers, the busy mode, slot 225's and
/// 227's answers.
struct Case0519 {
    set: Vec<(u32, u32)>,
    key: u32,
    mode: u32,
    a225: u32,
    a227: u32,
}

fn c519(set: &[(u32, u32)]) -> Case0519 {
    Case0519 {
        set: set.to_vec(),
        key: 8,
        mode: 0,
        a225: 0,
        a227: 1,
    }
}

fn keyed(key: u32, set: &[(u32, u32)]) -> Case0519 {
    Case0519 { key, ..c519(set) }
}

// the globals by the operand that names them
const G90: u32 = 0x08;
const G94: u32 = 0x35;
const G98: u32 = 0x60;
const G70: u32 = 0x8e;
const C1: u32 = 0x117;
const C2: u32 = 0x146;
const C5: u32 = 0x174;
const C4: u32 = 0x1c1;
const C6: u32 = 0x1c7;
const C7: u32 = 0x225;
const C3: u32 = 0x251;
const C8: u32 = 0x257;
const REC: u32 = 0x129c;

/// `magic_0519` 0x10002d60 (`r39::scene_task_0519`): the start-up checks
/// (slots 225 and 227 both ways, the one-time slots), every key with and
/// without the busy word, `G_C5` reaching 10, every part block, P8 running
/// or not, every `G_C8` value of the halfword, every block of the chain,
/// the busy word set late, the camera record and none.
#[test]
fn scene_task_0519() {
    let Some(mut t) = twin(519) else { return };
    all_stubs(&mut t);
    let f = t.at(0x2d60);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 225, 1, |m, _| m.r32(A225));
    rec(&mut t, 227, 1, |m, _| m.r32(A227));
    rec(&mut t, 261, 1, |m, a| {
        u32::from(a[0] == m.r32(KEY)) * (0x100 | a[0])
    });
    rec(&mut t, 941, 1, |_, a| TABLE + (a[0] & 0xf0));
    // the list slot: three entries
    rec(&mut t, 335, 1, |m, a| {
        for i in 0..3 {
            m.w32(a[0] + 4 * i, 0x0335_0000 + i);
        }
        3
    });
    rec(&mut t, 336, 1, |_, a| a[0] ^ 0x0336_0000);
    t.stub_st0(333, |_, a| 1.25 + f64::from(a[1] & 3));
    // the game's pointers: busy word, lock word, holder
    for (op, at) in [(0x108u32, BUSY), (0x97, LOCK), (0x2a8, HOLDER)] {
        let g = w(&t, op);
        if t.u32(g) == 0 {
            t.set_u32(g, at);
        }
    }
    let busy = t.u32(w(&t, 0x108));
    // slot 222 sets the busy word in mode 2
    rec(&mut t, 222, 4, move |m, a| {
        if m.r32(MODE) == 2 {
            m.w32(busy, 1);
        }
        a[1] ^ 0x222
    });
    let record = w(&t, 0x90c);
    let mut cases = vec![
        // the start-up checks
        Case0519 {
            a225: 5,
            ..c519(&[(G90, 0)])
        },
        c519(&[(G90, 0), (G94, 0), (G98, 0), (G70, 0)]),
        Case0519 {
            a227: 0,
            ..c519(&[(G98, 0)])
        },
        Case0519 {
            a227: 0xffff_ffff,
            ..c519(&[(G98, 0)])
        },
        c519(&[(G70, 0), (REC, record)]),
        c519(&[]),
        c519(&[(REC, record)]),
        // busy from the start, with and without a key
        Case0519 {
            mode: 1,
            ..c519(&[(REC, record)])
        },
        Case0519 {
            mode: 1,
            ..keyed(3, &[(REC, record)])
        },
        Case0519 {
            mode: 1,
            ..c519(&[])
        },
        // busy set late (slot 222)
        Case0519 {
            mode: 2,
            ..c519(&[(C7, 0), (REC, record)])
        },
        Case0519 {
            mode: 2,
            ..c519(&[(C7, 0)])
        },
        // G_C5 to 10
        keyed(5, &[(C5, 9)]),
        keyed(5, &[(C5, 9), (C3, 0x87)]),
        // the parts
        c519(&[(C3, 0x64)]),
        c519(&[(C3, 0x87)]),
        c519(&[(C3, 0x8a)]),
        c519(&[(C3, 0x122)]),
        c519(&[(C8, 0x290)]),
        c519(&[(C3, 0x64), (C8, 0x290)]),
        c519(&[(C8, 0x2e1)]),
        c519(&[(C8, 0x2e1), (C3, 0x8a)]),
        // P8 not running (its neighbour halfword set)
        c519(&[(C8, 0x2e1), (0x705, 0x5a5a_0000)]),
        // the halfword by G_C8
        c519(&[(C8, 0x106)]),
        c519(&[(C8, 0x14d)]),
        c519(&[(C8, 0x20f)]),
        c519(&[(C8, 0x122)]),
        c519(&[(C8, 0x1a4)]),
        c519(&[(C8, 0x23f)]),
        // the chain
        c519(&[(C7, 0)]),
        c519(&[(C7, 0x1e)]),
        c519(&[(C6, 0)]),
        c519(&[(C6, 0), (REC, record)]),
        c519(&[(C4, 0), (REC, record)]),
        c519(&[(C5, 0)]),
        c519(&[(C5, 0xc8)]),
        c519(&[(C2, 0)]),
        c519(&[(C2, 0x64)]),
        c519(&[(C1, 0)]),
        c519(&[(C1, 0x12c)]),
        c519(&[(C1, 0x12d)]),
    ];
    // every key (the counts land on the blocks: key 1 and 0 bump G_C3)
    for key in 0..8u32 {
        cases.push(keyed(key, &[(REC, record)]));
    }
    cases.push(keyed(0, &[(C3, 0x63)]));
    cases.push(keyed(1, &[(C3, 0x86), (C8, 0x2e0)]));
    cases.push(keyed(2, &[(C7, 0x1d)]));
    cases.push(keyed(3, &[(C6, 0xffff_ffff)]));
    cases.push(keyed(4, &[(C4, 0xffff_ffff), (C6, 0xffff_ffff)]));
    cases.push(keyed(6, &[(C2, 0x63)]));
    cases.push(keyed(7, &[(C1, 0x12b)]));
    for (k, cs) in (0u32..).zip(cases) {
        // the defaults: start-up done, every count off its values
        for (i, op) in (0u32..).zip([G90, G94, G98, G70]) {
            t.set_u32(w(&t, op), 0x10 + i + k);
        }
        for (i, op) in (0u32..).zip([C1, C2, C5, C4, C6, C7, C3, C8, 0xa63, 0x8f6]) {
            t.set_u32(w(&t, op), 0x55 + 3 * i + (k << 12));
        }
        t.set_u32(w(&t, REC), 0);
        t.set_u32(w(&t, 0x16), 0x7c00 + k);
        // P8 running, P0's halfword, the camera words junk first
        t.set_u16(w(&t, 0x705), 0x1234 + k as u16);
        t.set_u16(w(&t, 0x1282), 0x5a5a);
        for op in [0x912u32, 0x917, 0x91c] {
            t.set_u32(w(&t, op), 0x6b6b_0000 + op);
        }
        for (op, v) in &cs.set {
            t.set_u32(w(&t, *op), *v);
        }
        t.set_u32(KEY, cs.key);
        t.set_u32(MODE, cs.mode);
        t.set_u32(A225, cs.a225);
        t.set_u32(A227, cs.a227);
        t.set_u32(busy, u32::from(cs.mode == 1));
        t.set_u32(t.u32(w(&t, 0x97)), 0x0bad_0000 + k);
        for i in 0..0x40 {
            t.set_u32(TABLE + 4 * i, 0x0941_0000 + i + (k << 8));
        }
        ownb::dirty_stack(&mut t, 0x2d60 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0515`'s offset `o` in a copy (`(start, here)` sorted: from each
/// start on, `o - start + here`).
fn mp(map: &[(u32, u32)], o: u32) -> u32 {
    let mut r = o;
    for &(start, here) in map {
        if o >= start {
            r = o - start + here;
        }
    }
    r
}

/// `magic_0611`'s scene task against `magic_0515`'s (the states they
/// share; 0 and 0x15 are its own).
const M611: [(u32, u32); 24] = [
    (0x48d, 0x2e3),
    (0x4c3, 0x319),
    (0x564, 0x3af),
    (0x577, 0x3c2),
    (0x57a, 0x3c5),
    (0x62d, 0x480),
    (0x632, 0x478),
    (0x63a, 0x485),
    (0x760, 0x5a6),
    (0x9c1, 0x810),
    (0xb80, 0x9d0),
    (0xc27, 0xa77),
    (0xe8a, 0xcda),
    (0x130c, 0x115c),
    (0x1835, 0x1685),
    (0x1a52, 0x18a2),
    (0x3202, 0x3119),
    (0x33a0, 0x32b0),
    (0x3961, 0x3871),
    (0x3daf, 0x3cbf),
    (0x3e44, 0x3d49),
    (0x3e5f, 0x3d64),
    (0x3e72, 0x3d80),
    (0x4000, 0x4000),
];

/// `magic_0667`'s scene task against `magic_0515`'s (state 3's count of
/// actors is its own global: `0xb58` stands for it).
const M667: [(u32, u32); 63] = [
    (0x15c, 0x15c),
    (0x171, 0x16a),
    (0x1ef, 0x16f),
    (0x1f2, 0x178),
    (0x22c, 0x1b1),
    (0x48d, 0x412),
    (0x4c3, 0x448),
    (0x630, 0x5b5),
    (0x63a, 0x5c7),
    (0x63b, 0x5cd),
    (0x6c8, 0x65a),
    (0x6d7, 0x669),
    (0x6f5, 0x687),
    (0x70d, 0x69f),
    (0x712, 0x6a4),
    (0x72f, 0x6c1),
    (0x743, 0x6d5),
    (0x75d, 0x6ed),
    (0x887, 0x817),
    (0x8e8, 0x878),
    (0x946, 0x8d6),
    (0x977, 0x907),
    (0x97f, 0x90f),
    (0xaf2, 0xa82),
    (0xafa, 0xa8a),
    (0xb02, 0xa92),
    (0xb28, 0xab8),
    (0xb36, 0xacb),
    (0xb3e, 0xad5),
    (0xb49, 0xae0),
    (0xb51, 0xaef),
    (0xb58, 0xac7),
    (0xb59, 0xaf7),
    (0xb5c, 0xaf4),
    (0xb62, 0xb02),
    (0xb67, 0xb0f),
    (0xb6f, 0xb1d),
    (0xb80, 0xb30),
    (0xbd3, 0xb83),
    (0xbdd, 0xb8d),
    (0xbe4, 0xb94),
    (0xc08, 0xbb8),
    (0xc12, 0xbc2),
    (0xc27, 0xbd7),
    (0xe8a, 0xe3a),
    (0x130c, 0x12bc),
    (0x1835, 0x17e5),
    (0x1a52, 0x1a02),
    (0x2d69, 0x2d19),
    (0x2ebf, 0x2e76),
    (0x2ec2, 0x2e7e),
    (0x3202, 0x31be),
    (0x3399, 0x3357),
    (0x33a0, 0x3360),
    (0x3667, 0x34ba),
    (0x3961, 0x37b4),
    (0x3af8, 0x3949),
    (0x3b00, 0x3950),
    (0x3daf, 0x3bff),
    (0x3e5f, 0x3caf),
    (0x3e87, 0x3cd7),
    (0x3e9b, 0x3ceb),
    (0x3eab, 0x3cfb),
];
/// The 0515 scene task's control words: slot 253's and 313's answers (253
/// answering 0 makes the game busy).
const A253: u32 = SCRATCH + 0x7f20;
const A313: u32 = SCRATCH + 0x7f24;

/// One case of the 0515 scene task: the globals by operand (on top of the
/// defaults), the state, slots 253's, 313's, 225's and 227's answers.
struct Scene {
    set: Vec<(u32, u32)>,
    /// words by address
    mem: Vec<(u32, u32)>,
    state: u32,
    a253: u32,
    a313: u32,
    a225: u32,
    a227: u32,
}

fn scene(state: u32, set: &[(u32, u32)]) -> Scene {
    Scene {
        set: set.to_vec(),
        mem: Vec::new(),
        state,
        a253: 1,
        a313: 0,
        a225: 0,
        a227: 1,
    }
}

/// A scene task's twin (`magic_N`, the task at `at`) with its stubs and
/// the game's pointers set.
fn scene_twin(n: u16, at: u32, map: &[(u32, u32)]) -> Option<(Twin, u32)> {
    let mut t = twin(n)?;
    all_stubs(&mut t);
    let f = t.at(at);
    rec(&mut t, 225, 1, |m, _| m.r32(A225));
    rec(&mut t, 227, 1, |m, _| m.r32(A227));
    rec(&mut t, 253, 0, |m, _| m.r32(A253));
    // the handles' high words (slot 772's 64-bit answer)
    t.stub_edx(772, |_, a| a[1] ^ 0x0772_0000);
    rec(&mut t, 313, 1, |m, _| m.r32(A313));
    rec(&mut t, 941, 1, |_, a| TABLE + (a[0] & 0xf0));
    rec(&mut t, 263, 1, |_, a| 0x40 + (a[0] & 7));
    // (no arguments: the answer must not read the stack)
    rec(&mut t, 324, 0, |m, _| {
        let k = m.r32(SCRATCH + 0x7f34);
        m.w32(SCRATCH + 0x7f34, k.wrapping_add(0x1357));
        k
    });
    t.stub_st0(307, |m, _| {
        let k = m.r32(SCRATCH + 0x7f30);
        m.w32(SCRATCH + 0x7f30, k.wrapping_add(1));
        f64::from(k % 7) * 0.375 - 0.5
    });
    for (op, at) in [(0xb1u32, BUSY), (0xe3, LOCK)] {
        let g = t.img.word(f + op);
        if t.u32(g) == 0 {
            t.set_u32(g, at);
        }
    }
    // a sine table of 0x1000 floats (the game's pointer, set by
    // InitMagicPRX to a zeroed data page, points at it)
    let sin_at = t.img.word(f + mp(map, 0x10f8));
    t.set_u32(sin_at, SINES);
    let sines: Vec<u8> = (0..0x1000u32)
        .flat_map(|i| {
            ((i as f32 * 0.001_533_98).sin() + i as f32 * 1.0e-5)
                .to_bits()
                .to_le_bytes()
        })
        .collect();
    t.write(SINES, &sines);
    // the actors' and records' slot 296 (id, value) logged
    rec(&mut t, 296, 2, |_, a| a[1] ^ 0x0296_0000);
    // the cameras' jitter (slot 314's float)
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.375 - 0.0625);
    // slot 711 sees the lock word set
    rec(&mut t, 711, 3, |m, _| {
        let l = m.r32(LOCK);
        m.w32(SCRATCH + 0x7f38, l);
        0x711
    });
    Some((t, f))
}

/// Runs the cases through the original and `r39b::scene_0515`.
fn run_scene(t: &mut Twin, f: u32, map: &[(u32, u32)], cases: Vec<Scene>) {
    let w = |t: &Twin, o: u32| t.img.word(f + mp(map, o));
    let only = std::env::var("R39_CASE")
        .ok()
        .and_then(|v| v.parse::<u32>().ok());
    for (k, cs) in (0u32..).zip(cases) {
        if only.is_some_and(|o| o != k) {
            continue;
        }
        // the prologue done; the end's counters junk
        for (i, op) in (0u32..).zip([0x1cu32, 0x4b, 0x76, 0xbc]) {
            t.set_u32(w(t, op), 0x10 + i + k);
        }
        t.set_u32(w(t, 0x2c), 0x7c00 + k);
        for (i, op) in (0u32..).zip([0x3e89u32, 0x3e8f, 0x3e95, 0x3ec9, 0x3ed4]) {
            t.set_u32(w(t, op), 0x300 + 7 * i + k);
        }
        t.set_u32(w(t, 0x3ebf), 0);
        t.set_u32(w(t, 0x3e83), 0);
        t.set_u32(w(t, 0x141), cs.state);
        for (op, v) in &cs.set {
            t.set_u32(w(t, *op), *v);
        }
        for (at, v) in &cs.mem {
            t.set_u32(*at, *v);
        }
        t.set_u32(A253, cs.a253);
        t.set_u32(A313, cs.a313);
        t.set_u32(A225, cs.a225);
        t.set_u32(A227, cs.a227);
        let lock = t.u32(w(t, 0xe3));
        t.set_u32(lock, 0x0bad_0000 + k);
        for i in 0..0x40 {
            t.set_u32(TABLE + 4 * i, 0x0941_0000 + i + (k << 8));
        }
        ownb::dirty_stack(t, 0x4ce0 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0515` 0x10004ce0 (`r39b::scene_0515`, the ported states): the
/// prologue's checks (slots 225, 227 and 313 both ways, the one-time
/// slots), the busy word both ways, state 1 (its limit both ways), state
/// 0x64, states between and past the table, and the end (the record
/// counters, a camera record at and off its last frame).
fn scene_states(dll: u16, at: u32, map: &[(u32, u32)]) {
    let Some((mut t, f)) = scene_twin(dll, at, map) else {
        return;
    };
    // a camera record, its last frame 0x41
    let r = SCRATCH + 0x6000;
    for i in 0..0x40u32 {
        t.set_u32(r + 4 * i, 0x3f00_0000 + i * 0x0010_0000);
    }
    t.set_u32(r + 0x24, 0x41);
    let t_at = 0x3eaf;
    let mut cases = vec![
        Scene {
            a225: 3,
            ..scene(2, &[(0x1c, 0)])
        },
        Scene {
            a227: 0,
            ..scene(2, &[(0x76, 0)])
        },
        Scene {
            a313: 1,
            ..scene(2, &[(0xbc, 0)])
        },
        scene(2, &[(0x1c, 0), (0x4b, 0), (0x76, 0), (0xbc, 0)]),
        Scene {
            a253: 0,
            ..scene(2, &[])
        },
        scene(0x65, &[]),
        scene(0x40, &[(0x3e83, 2)]),
        scene(0x40, &[(0x3ebf, r), (t_at, 0x0005_0040)]),
        scene(0x40, &[(0x3ebf, r), (t_at, 0x0005_0041)]),
        scene(0x64, &[(0x3e21, 2)]),
        Scene {
            a253: 0,
            ..scene(1, &[])
        },
    ];
    // state 1: G_A against G_B - 0x2d000
    for (b, a) in [
        (0x2d010u32, 0x10u32),
        (0x2d010, 0xf),
        (0x2d000, 0xffff_ffff),
    ] {
        cases.push(scene(1, &[(0x49f, b), (0x4aa, a)]));
    }
    run_scene(&mut t, f, map, cases);
}

/// `magic_0515` states 6 and 8 (`r39b`): once and again, busy, every
/// count that does something and one past, the three sound records' kinds
/// at and off their frames and limits.
fn scene_states_6_8(dll: u16, at: u32, map: &[(u32, u32)]) {
    let Some((mut t, f)) = scene_twin(dll, at, map) else {
        return;
    };
    let w = |t: &Twin, o: u32| t.img.word(f + mp(map, o));
    let mut cases = vec![
        scene(6, &[(0xc2a, 0), (0xd34, 0)]),
        scene(6, &[(0xc2a, 1), (0xd34, 0)]),
        scene(6, &[(0xc2a, 1), (0xd34, 0x2d)]),
        scene(6, &[(0xc2a, 1), (0xd34, 0x67)]),
        scene(6, &[(0xc2a, 1), (0xd34, 0x68)]),
        Scene {
            a253: 0,
            ..scene(6, &[(0xc2a, 0)])
        },
        scene(8, &[(0xe8d, 0), (0x1181, 0x10)]),
        Scene {
            a253: 0,
            ..scene(8, &[(0xe8d, 0)])
        },
    ];
    for n in [0x55u32, 0x69, 0x7d, 0xa5, 0xe1, 0x108, 0x107] {
        cases.push(scene(8, &[(0xe8d, 1), (0x1181, n)]));
    }
    // the sound records: (kind, frame, limit, value) for records 0, 1, 2
    let p0 = w(&t, 0xf23);
    let recs: [[(u32, u32, u32, u32); 3]; 6] = [
        [(0xc4, 0x32, 0, 0), (0xc2, 0x14, 0, 0), (0xc2, 0x14, 0, 0)],
        [(0xc4, 0x31, 0, 0), (0xc2, 0x13, 0, 0), (0xc2, 0x15, 0, 0)],
        [(0xc5, 0x40, 5, 5), (0xc3, 0x30, 5, 6), (0xc3, 0x30, 5, 5)],
        [(0xc5, 0x45, 6, 5), (0xc3, 0x25, 6, 5), (0xc3, 0x10, 6, 5)],
        [(0xc5, 0x20, 6, 5), (0xc5, 0x25, 6, 7), (0xc5, 0x10, 6, 7)],
        [(0xc2, 0x20, 6, 5), (0xc4, 0x25, 6, 7), (0xc5, 0x10, 7, 6)],
    ];
    for rs in recs {
        let mut cs = scene(8, &[(0xe8d, 1), (0x1181, 0x20)]);
        for (i, (kind, fr, lim, v)) in (0u32..).zip(rs) {
            let p = p0 + 0xcc * i;
            cs.mem
                .extend([(p, kind), (p + 4, lim), (p + 8, v), (p + 0x10, fr << 16)]);
        }
        cases.push(cs);
    }
    // the models' records: an id each
    for i in 0..3u32 {
        t.set_u32(w(&t, 0xdf4) - 0x40 + 0xcc * i, 0x10 + i);
    }
    run_scene(&mut t, f, map, cases);
}

/// `magic_0515` states 9 and 0xd (`r39b`): once and again, busy, the
/// sound record's kinds at and off their frames, the curve once and not
/// again, part P3 before the curve, after 0x32 frames and before, the
/// counts that do something, the timer below 0.
fn scene_states_9_d(dll: u16, at: u32, map: &[(u32, u32)]) {
    let Some((mut t, f)) = scene_twin(dll, at, map) else {
        return;
    };
    let w = |t: &Twin, o: u32| t.img.word(f + mp(map, o));
    // the sound record's kind, limit and frame; P3's halfword; the flag;
    // P3's frames; the count
    let (kind, lim, frame) = (0x1403u32, 0x1414u32, 0x141du32);
    let (p3, flag, p3n, n9) = (0x15eau32, 0x15f7u32, 0x1621u32, 0x16a4u32);
    let mut cases = vec![
        scene(9, &[(0x130f, 0), (n9, 0)]),
        Scene {
            a253: 0,
            ..scene(9, &[(0x130f, 0)])
        },
        scene(9, &[(0x130f, 1), (n9, 0x83)]),
        scene(9, &[(0x130f, 1), (n9, 0x97)]),
        scene(9, &[(0x130f, 1), (n9, 0xbd), (0x17fd, 0xffff_fff0)]),
        scene(9, &[(0x130f, 1), (n9, 0xbc)]),
        scene(9, &[(0x130f, 1), (kind, 0xc5), (lim, 0x20), (frame, 0x16)]),
        scene(9, &[(0x130f, 1), (kind, 0xc5), (lim, 0x20), (frame, 0x20)]),
        scene(9, &[(0x130f, 1), (kind, 0xc5), (lim, 0x20), (frame, 0x1f)]),
        scene(9, &[(0x130f, 1), (kind, 0xc6), (frame, 0x1c)]),
        scene(9, &[(0x130f, 1), (kind, 0xc6), (frame, 0x1b)]),
        scene(9, &[(0x130f, 1), (kind, 0xc7), (frame, 0x17), (flag, 0)]),
        scene(9, &[(0x130f, 1), (kind, 0xc7), (frame, 0x17), (flag, 1)]),
        scene(9, &[(0x130f, 1), (kind, 0xc7), (frame, 0x16), (flag, 0)]),
        scene(9, &[(0x130f, 1), (kind, 0xc8)]),
        scene(9, &[(0x130f, 1), (p3, 1), (flag, 0)]),
        scene(9, &[(0x130f, 1), (p3, 1), (flag, 1), (p3n, 0x33)]),
        scene(9, &[(0x130f, 1), (p3, 1), (flag, 1), (p3n, 0x32)]),
        scene(9, &[(0x130f, 1), (p3, 1), (flag, 1), (p3n, 0xffff_fff0)]),
        scene(0xd, &[(0x1838, 0)]),
        Scene {
            a253: 0,
            ..scene(0xd, &[(0x1838, 0)])
        },
        scene(0xd, &[(0x1838, 1), (0x1a2d, 0x3b)]),
        scene(0xd, &[(0x1838, 1), (0x1a2d, 0x3a)]),
    ];
    // the curve's point (slot 237 leaves it as it is) and the models'
    // places
    let pt = w(&t, 0x14e8);
    for cs in &mut cases {
        cs.mem.extend([
            (pt, 0x3fc0_0000),
            (pt + 4, 0xc020_0000),
            (pt + 8, 0x4090_0000),
        ]);
    }
    let m = w(&t, 0x1922);
    for i in 0..12u32 {
        t.set_u32(
            m + 0xcc * (i / 4) + 4 * (i % 4),
            (0.75 * i as f32 - 2.0).to_bits(),
        );
    }
    run_scene(&mut t, f, map, cases);
}

/// `magic_0515` state 0x10 (`r39b`): the set-up, busy, slot 220 at 0,
/// the beams at 0x14, bursts at frames with and without pushed sparks and
/// past them, the ring's ageing and colours, the copies at 0x96, the
/// shrinking ring with and without the halfwords, the fade, the clear at
/// 0xfa, parts at 0xb8, 0xc2, 0x112, and the step into state 0x15.
fn scene_state_10(dll: u16, at: u32, map: &[(u32, u32)]) {
    let Some((mut t, f)) = scene_twin(dll, at, map) else {
        return;
    };
    let w = |t: &Twin, o: u32| t.img.word(f + mp(map, o));
    let n10 = 0x1cc0u32;
    let mut cases = vec![
        scene(0x10, &[(0x1a55, 0), (n10, 0)]),
        Scene {
            a253: 0,
            ..scene(0x10, &[(0x1a55, 0)])
        },
    ];
    for n in [
        0x14u32, 0x15, 0x16, 0x34, 0x40, 0x59, 0x77, 0x78, 0x96, 0x9a, 0xb3, 0xb7, 0xb8, 0xbe,
        0xc2, 0xdc, 0xdd, 0xf9, 0xfa, 0x112, 0x12f, 0x13,
    ] {
        cases.push(scene(0x10, &[(0x1a55, 1), (n10, n)]));
    }
    // parts P6 and P4 and the records the clear at 0xc2 zeroes: junk
    let parts = [0x1035_3310u32, 0x1035_1410];
    let recs0 = t.img.word(f + mp(map, 0x2bd7));
    for cs in &mut cases {
        for p in parts {
            for o in (0x40..0x80).step_by(4) {
                cs.mem.push((p + o, 0x5a5a_0000 + o));
            }
        }
        for i in 0..0x20u32 {
            cs.mem.push((recs0 + 0xa8 * i, 0x77 + i));
            cs.mem.push((recs0 + 0xa8 * i - 0x24, 0x88 + i));
        }
    }
    // the ring index at its wrap; more pushed bursts (other randoms)
    cases.push(scene(0x10, &[(0x1a55, 1), (n10, 0x14), (0x1f23, 0x1ff)]));
    cases.push(scene(0x10, &[(0x1a55, 1), (n10, 0x16), (0x1f23, 0x1ff)]));
    for n in [0x34u32, 0x35, 0x36, 0x37, 0x38, 0x54, 0x55, 0x56, 0x57] {
        cases.push(scene(0x10, &[(0x1a55, 1), (n10, n)]));
    }
    // ring entries alive (ageing past and before their life)
    let ring = w(&t, 0x26b0);
    for cs in &mut cases {
        for i in 0..6u32 {
            let p = ring + 0x3c * i;
            cs.mem.extend([(p - 8, i & 1), (p, 5 + i), (p + 4, 7)]);
        }
    }
    // the models' places and directions, again each case (the set-up's
    // ring moves the places)
    let (m, v) = (w(&t, 0x1cf8), w(&t, 0x1cfd) - 8);
    for (k, cs) in (0u32..).zip(&mut cases) {
        for i in 0..12u32 {
            let x = 0.75 * i as f32 - 2.0 + 0.125 * k as f32;
            cs.mem.push((m + 0xcc * (i / 4) + 4 * (i % 4), x.to_bits()));
            cs.mem.push((v + 4 * i, (1.5 - 0.375 * i as f32).to_bits()));
        }
    }
    // vector slots: (dst, a, b) with both read, 232 (dst, src), 233
    // (dst, src, scale)
    for (s, k) in [
        (232u32, 0.5f32),
        (229, 0.25),
        (248, -0.75),
        (234, 2.0),
        (266, 1.25),
    ] {
        rec(&mut t, s, if s == 232 { 2 } else { 3 }, move |m, a| {
            for i in 0..4 {
                let mut v = f32::from_bits(m.r32(a[1] + 4 * i)) * k + i as f32;
                if s != 232 {
                    v += f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5;
                }
                m.w32(a[0] + 4 * i, v.to_bits());
            }
            s
        });
    }
    rec(&mut t, 233, 3, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) * f32::from_bits(a[2]) + i as f32;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x233
    });
    // the axes' matrices (slot 247 into a local)
    rec(&mut t, 247, 2, |m, a| {
        for i in 0..16 {
            m.w32(
                a[0] + 4 * i,
                (0.125 * i as f32 - (a[1] & 7) as f32).to_bits(),
            );
        }
        0x247
    });
    run_scene(&mut t, f, map, cases);
}

/// `magic_0515` state 0x18 (`r39b`): once (the actor there and not),
/// busy, slot 220 at 0, the actors and sounds at 0x28, the third model's
/// part at frame 0x54, the first's at 0x11, each at its limit and not,
/// the flash below and at 0xf, the step into state 0x64.
fn scene_state_18(dll: u16, at: u32, map: &[(u32, u32)]) {
    let Some((mut t, f)) = scene_twin(dll, at, map) else {
        return;
    };
    let w = |t: &Twin, o: u32| t.img.word(f + mp(map, o));
    let (once, n18, flash, id) = (0x3964u32, 0x3acbu32, 0x3c70u32, 0x3a56u32);
    let mut cases = vec![
        scene(0x18, &[(once, 0), (n18, 0), (id, 0x21)]),
        scene(0x18, &[(once, 0), (n18, 5), (id, 0xff)]),
        Scene {
            a253: 0,
            ..scene(0x18, &[(once, 0)])
        },
        scene(0x18, &[(once, 1), (n18, 0x28), (flash, 0xe)]),
        scene(0x18, &[(once, 1), (n18, 0x9f), (flash, 0xf)]),
        scene(0x18, &[(once, 1), (n18, 0x9e), (flash, 0xffff)]),
    ];
    let p0 = w(&t, 0x3af2);
    // (kind, limit, frame) per model
    let recs: [[(u32, u32, u32); 3]; 4] = [
        [(0xc8, 0x40, 0x11), (0xc8, 0x30, 0x30), (0xc8, 0x60, 0x54)],
        [(0xc8, 0x11, 0x12), (0xc8, 0x30, 0x2f), (0xc8, 0x55, 0x55)],
        [(0xc7, 0x11, 0x11), (0xc9, 0x30, 0x30), (0xc7, 0x54, 0x54)],
        [(0xc8, 0x12, 0x12), (0xc8, 0x54, 0x54), (0xc8, 0x11, 0x11)],
    ];
    for rs in recs {
        let mut cs = scene(0x18, &[(once, 1), (n18, 0x30), (flash, 3)]);
        for (i, (kind, lim, fr)) in (0u32..).zip(rs) {
            let p = p0 + 0xcc * i;
            cs.mem.extend([
                (p, kind),
                (p + 4, lim),
                (p + 8, fr),
                (p - 8, 0x30 + i),
                (p - 4, 0x40 + i),
            ]);
            for j in 0..3u32 {
                cs.mem
                    .push((p + 0x34 + 4 * j, (0.5 * (i + j) as f32 - 1.0).to_bits()));
            }
        }
        cases.push(cs);
    }
    let pt = SCRATCH + 0x7c00;
    rec(&mut t, 237, 4, move |m, a| {
        for i in 0..4 {
            let v = 0.25 * (a[1] + i) as f32 - 2.0;
            m.w32(a[3] + 4 * i, v.to_bits());
        }
        m.w32(pt, a[1]);
        0x237
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + f32::from_bits(m.r32(a[1] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    rec(&mut t, 247, 2, |m, a| {
        for i in 0..16 {
            m.w32(
                a[0] + 4 * i,
                (0.125 * i as f32 - (a[1] & 7) as f32).to_bits(),
            );
        }
        0x247
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            m.w32(
                a[0] + 4 * i,
                (1.25 * i as f32 + (a[1] & 3) as f32).to_bits(),
            );
        }
        0x325
    });
    run_scene(&mut t, f, map, cases);
}

/// `magic_0515` state 0 (`r39b`): once with either flag or none, busy,
/// the camera's second record or not, the counts 0x3e and 0x19, the
/// spread's range and past it, the count reaching 0.
fn scene_state_0(dll: u16, at: u32, map: &[(u32, u32)]) {
    let Some((mut t, f)) = scene_twin(dll, at, map) else {
        return;
    };
    let (once, fa, fb, n0) = (0x15fu32, 0x16cu32, 0x180u32, 0x2c6u32);
    let (cam, left) = (0x2aeu32, 0x2b7u32);
    let mut cases = vec![
        scene(0, &[(once, 0), (fa, 1), (fb, 0), (n0, 0x30), (0x245, 2)]),
        scene(0, &[(once, 0), (fa, 0), (fb, 1), (n0, 0x30)]),
        scene(0, &[(once, 0), (fa, 0), (fb, 0), (n0, 0x30)]),
        Scene {
            a253: 0,
            ..scene(0, &[(once, 0)])
        },
        scene(0, &[(once, 1), (cam, 2), (left, 0), (n0, 0x3e)]),
        scene(0, &[(once, 1), (cam, 2), (left, 1), (n0, 0x19)]),
        scene(0, &[(once, 1), (cam, 3), (left, 0), (n0, 0x16)]),
        scene(0, &[(once, 1), (n0, 0x15)]),
        scene(0, &[(once, 1), (n0, 0x6)]),
        scene(0, &[(once, 1), (n0, 0x5)]),
        scene(0, &[(once, 1), (n0, 0x1)]),
        scene(0, &[(once, 1), (n0, 0x8001)]),
    ];
    // the records slot 296 goes over (with 0, or `_off` in 0667): two
    let with = t.img.rel32_target(f + mp(map, 0x22d));
    let n_with = t.img.word(with + if dll == 667 { 8 } else { 7 });
    for cs in &mut cases {
        cs.mem.push((n_with, 2));
        if dll == 667 {
            // (no flags there)
            cs.set.retain(|(o, _)| *o != fa && *o != fb);
        }
    }
    for (k, cs) in (0u32..).zip(&mut cases) {
        // the first model's sound record and centre
        let m = t.img.word(f + mp(map, 0x259));
        cs.mem.extend([(m, 0x20 + k), (m + 0x16, 0x1234_0000 + k)]);
    }
    rec(&mut t, 289, 1, |_, a| a[0] ^ 0x289);
    t.stub_st0(289, |_, a| 1.5 + f64::from(a[0] & 3));
    rec(&mut t, 237, 4, move |m, a| {
        for i in 0..4 {
            let v = 0.25 * (a[1] + i) as f32 - 2.0;
            m.w32(a[3] + 4 * i, v.to_bits());
        }
        0x237
    });
    run_scene(&mut t, f, map, cases);
}

/// `magic_0515` state 3 (`r39b`): once, busy, the fade's three ranges,
/// slot 220 at 0, the counts that do something, the sounds at 0x104 and
/// after with every kind at and off its frames, the colour from 0xcd, the
/// placed parts' sizes, the step into state 6.
fn scene_state_3(dll: u16, at: u32, map: &[(u32, u32)]) {
    let Some((mut t, f)) = scene_twin(dll, at, map) else {
        return;
    };
    let w = |t: &Twin, o: u32| t.img.word(f + mp(map, o));
    let (once, n3) = (0x4c6u32, 0x6afu32);
    let mut cases = vec![
        scene(3, &[(once, 0), (n3, 0)]),
        Scene {
            a253: 0,
            ..scene(3, &[(once, 0)])
        },
    ];
    for n in [
        0u32, 0x59, 0x6d, 0xcc, 0xcd, 0xd6, 0x103, 0x108, 0x109, 0x115, 0x120, 0x13f, 0x13e,
    ] {
        cases.push(scene(3, &[(once, 1), (n3, n)]));
    }
    // the sound records' (kind, frame, limit) per model, from 0x104 on
    let p0 = w(&t, 0x9bd);
    let recs: [[(u32, u32, u32); 3]; 5] = [
        [(0xc3, 0x17, 0x17), (0xc1, 0x14, 0x14), (0xc1, 0x1c, 0x1c)],
        [(0xc3, 0x16, 0x17), (0xc1, 0x13, 0x14), (0xc1, 0x1b, 0x1c)],
        [(0xc3, 0x18, 0x20), (0xc1, 0x15, 0x20), (0xc1, 0x1d, 0x20)],
        [(0xc4, 0x18, 0x20), (0xc2, 0x15, 0x20), (0xc2, 0x1d, 0x20)],
        [(0xc2, 0x18, 0x20), (0xc3, 0x15, 0x20), (0xc4, 0x1d, 0x20)],
    ];
    for rs in recs {
        let mut cs = scene(3, &[(once, 1), (n3, 0x110)]);
        for (i, (kind, fr, lim)) in (0u32..).zip(rs) {
            let p = p0 + 0xcc * i;
            cs.mem.extend([
                (p - 8, kind),
                (p, fr),
                (p - 4, lim),
                (p + 0x30, (1.5 * i as f32).to_bits()),
            ]);
        }
        cases.push(cs);
    }
    // the models' radii, heights and angles (the parts at 0x5a)
    let m = w(&t, 0x74a);
    for cs in &mut cases {
        for i in 0..3u32 {
            let p = m + 0xcc * i;
            cs.mem.extend([
                (p - 0x20, (2.5 + i as f32).to_bits()),
                (p + 0x20, (7.5 + i as f32).to_bits()),
                (p - 0x1c, (0.75 * i as f32 + 0.5).to_bits()),
                (p - 0x28, (0x1234 * (i + 1)) << 16),
            ]);
        }
    }
    // the placed parts' count (two) and sizes
    let parts = w(&t, 0xb68);
    for cs in &mut cases {
        cs.set.push((0xb58, 2));
        for i in 0..2u32 {
            cs.mem
                .push((parts + 0xcc * i + 0x24, (0.5 + i as f32).to_bits()));
        }
    }
    // slot 300 answers the fade it is given, halved, in its second word
    rec(&mut t, 300, 2, |m, a| {
        let x = f32::from_bits(m.r32(a[0])) * 0.5 + 0.125;
        m.w32(a[1], x.to_bits());
        0x300
    });
    rec(&mut t, 247, 2, |m, a| {
        for i in 0..16 {
            m.w32(
                a[0] + 4 * i,
                (0.125 * i as f32 - (a[1] & 7) as f32).to_bits(),
            );
        }
        0x247
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) * 0.5 + f32::from_bits(m.r32(a[1] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    run_scene(&mut t, f, map, cases);
}

/// `magic_0515` state 0x15 (`r39b`): once, busy, the count's 0 (and
/// still 0), the spread's fall, the flash off, the spread from 0x37 with
/// slot 279 busy and not, the wait running, at 0 with actors and without,
/// the parts at 0xf, 0x37 and 0x3d, the step into state 0x17.
fn scene_state_15(dll: u16, at: u32, map: &[(u32, u32)]) {
    let Some((mut t, f)) = scene_twin(dll, at, map) else {
        return;
    };
    let (once, n15, wait, left, count) = (0x2d6cu32, 0x2edau32, 0x304fu32, 0x305bu32, 0x3070u32);
    let mut cases = vec![
        scene(0x15, &[(once, 0), (n15, 0)]),
        Scene {
            a253: 0,
            ..scene(0x15, &[(once, 0)])
        },
    ];
    for n in [0u32, 0xe, 0xf, 0x10, 0x1d, 0x1e, 0x36, 0x3d, 0xcc, 0xcb] {
        cases.push(scene(0x15, &[(once, 1), (n15, n), (count, 3)]));
    }
    for (n, w, l, ans) in [
        (0x37u32, 0u32, 2u32, 0u32),
        (0x40, 0, 2, 1),
        (0x40, 5, 2, 0),
        (0x40, 0, 0, 0),
        (0x40, 0, 0xffff, 0),
        (0x60, 0, 7, 0),
    ] {
        let mut cs = scene(
            0x15,
            &[(once, 1), (n15, n), (wait, w), (left, l), (count, 3)],
        );
        cs.mem.push((SCRATCH + 0x7f3c, ans));
        cases.push(cs);
    }
    // the next actor's index
    for cs in &mut cases {
        cs.set.push((0x309d, 2));
    }
    rec(&mut t, 279, 0, |m, _| m.r32(SCRATCH + 0x7f3c));
    rec(&mut t, 239, 3, |_, a| a[1] ^ 0x239);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            m.w32(
                a[0] + 4 * i,
                (1.25 * i as f32 + (a[1] & 3) as f32).to_bits(),
            );
        }
        0x325
    });
    run_scene(&mut t, f, map, cases);
}

/// `magic_0515` state 0x17 (`r39b`): once with each camera flag and none
/// (actors widening the box both ways, one without an actor id), busy,
/// the sparks below 0x3c (records free and none), the actor's byte at
/// 0x1e (slot 926 both ways), the step into state 0x18.
fn scene_state_17(dll: u16, at: u32, map: &[(u32, u32)]) {
    let Some((mut t, f)) = scene_twin(dll, at, map) else {
        return;
    };
    let w = |t: &Twin, o: u32| t.img.word(f + mp(map, o));
    let (once, n17, count) = (0x3205u32, 0x3681u32, 0x3486u32);
    let flags = [0x34fcu32, 0x3593, 0x3627, 0x3641, 0x365b];
    let mut cases = Vec::new();
    for k in 0..6usize {
        let mut set = vec![(once, 0), (n17, 0x40), (count, 3)];
        for (j, fl) in flags.iter().enumerate() {
            set.push((*fl, u32::from(j == k)));
        }
        cases.push(scene(0x17, &set));
    }
    cases.push(Scene {
        a253: 0,
        ..scene(0x17, &[(once, 0), (count, 0)])
    });
    for n in [0u32, 0x3b, 0x1e, 0x59, 0x58] {
        cases.push(scene(0x17, &[(once, 1), (n17, n)]));
    }
    let mut cs = scene(0x17, &[(once, 1), (n17, 0x1e)]);
    cs.mem.push((SCRATCH + 0x7f40, 1));
    cases.push(cs);
    // no free record
    let pool = t.img.word(t.img.rel32_target(f + mp(map, 0x3696)) + 6);
    let mut cs = scene(0x17, &[(once, 1), (n17, 0x10)]);
    for i in 0..0x100u32 {
        cs.mem.push((pool + 0x4c * i + 0x28, 1));
    }
    cases.push(cs);
    // the actors' places, ids, models, angles
    let p0 = w(&t, 0x3395);
    for cs in &mut cases {
        for i in 0..3u32 {
            let p = p0 + 0xcc * i;
            let x = [3.0f32, -2.5, 0.75][i as usize];
            let z = [-1.0f32, 4.5, 0.25][i as usize];
            cs.mem.extend([
                (p, x.to_bits()),
                (p + 4, (0.5 * i as f32).to_bits()),
                (p + 8, z.to_bits()),
                (p - 0x2c, if i == 1 { 0xff } else { 0x30 + i }),
                (p - 0x28, 0x40 + i),
                (p - 0x18, 0x1100 * (i + 1)),
            ]);
        }
    }
    // the records slot 296 goes over: two; the actor byte's neighbours
    // (0667 has no camera flags)
    let n_with = if dll == 667 {
        None
    } else {
        let with = t.img.rel32_target(f + mp(map, 0x3631));
        Some(t.img.word(with + 7))
    };
    for cs in &mut cases {
        if let Some(n) = n_with {
            cs.mem.push((n, 2));
        } else {
            cs.set.retain(|(o, _)| !flags.contains(o));
        }
        cs.mem.push((SCRATCH + 0x7614, 0x5a5a_5a5a));
    }
    rec(&mut t, 290, 2, |m, a| {
        for i in 0..4 {
            m.w32(
                a[1] + 4 * i,
                (0.5 + 0.25 * i as f32 + (a[0] & 3) as f32).to_bits(),
            );
        }
        0x290
    });
    t.stub_st0(277, |_, a| 1.25 + f64::from(a[0] & 3));
    t.stub_st0(289, |_, a| 0.5 + f64::from(a[0] & 3));
    rec(&mut t, 926, 1, |m, _| m.r32(SCRATCH + 0x7f40));
    rec(&mut t, 925, 1, |_, _| SCRATCH + 0x7000);
    rec(&mut t, 239, 3, |_, a| a[1] ^ 0x239);
    run_scene(&mut t, f, map, cases);
}

#[test]
fn scene_0515_states() {
    scene_states(515, 0x4ce0, &[]);
}

/// `magic_0611` 0x10004d10 (`r39b::scene_task_0611`): [`scene_0515_states`]'s cases on
/// the copy.
#[test]
fn scene_0611_states() {
    scene_states(611, 0x4d10, &M611);
}

#[test]
fn scene_0515_states_6_8() {
    scene_states_6_8(515, 0x4ce0, &[]);
}

/// `magic_0611` 0x10004d10 (`r39b::scene_task_0611`): [`scene_0515_states_6_8`]'s cases on
/// the copy.
#[test]
fn scene_0611_states_6_8() {
    scene_states_6_8(611, 0x4d10, &M611);
}

#[test]
fn scene_0515_states_9_d() {
    scene_states_9_d(515, 0x4ce0, &[]);
}

/// `magic_0611` 0x10004d10 (`r39b::scene_task_0611`): [`scene_0515_states_9_d`]'s cases on
/// the copy.
#[test]
fn scene_0611_states_9_d() {
    scene_states_9_d(611, 0x4d10, &M611);
}

#[test]
fn scene_0515_state_10() {
    scene_state_10(515, 0x4ce0, &[]);
}

/// `magic_0611` 0x10004d10 (`r39b::scene_task_0611`): [`scene_0515_state_10`]'s cases on
/// the copy.
#[test]
fn scene_0611_state_10() {
    scene_state_10(611, 0x4d10, &M611);
}

#[test]
fn scene_0515_state_18() {
    scene_state_18(515, 0x4ce0, &[]);
}

/// `magic_0611` 0x10004d10 (`r39b::scene_task_0611`): [`scene_0515_state_18`]'s cases on
/// the copy.
#[test]
fn scene_0611_state_18() {
    scene_state_18(611, 0x4d10, &M611);
}

#[test]
fn scene_0515_state_0() {
    scene_state_0(515, 0x4ce0, &[]);
}

#[test]
fn scene_0515_state_3() {
    scene_state_3(515, 0x4ce0, &[]);
}

/// `magic_0611` 0x10004d10 (`r39b::scene_task_0611`): [`scene_0515_state_3`]'s cases on
/// the copy.
#[test]
fn scene_0611_state_3() {
    scene_state_3(611, 0x4d10, &M611);
}

#[test]
fn scene_0515_state_15() {
    scene_state_15(515, 0x4ce0, &[]);
}

#[test]
fn scene_0515_state_17() {
    scene_state_17(515, 0x4ce0, &[]);
}

/// `magic_0611` 0x10004d10 (`r39b::scene_task_0611`): [`scene_0515_state_17`]'s cases on
/// the copy.
#[test]
fn scene_0611_state_17() {
    scene_state_17(611, 0x4d10, &M611);
}

/// `magic_0611` state 0 (its own; `r39b::state_0_0611`): once with
/// either flag or none, busy, the count below and at 0x14.
#[test]
fn scene_0611_state_0() {
    let Some((mut t, f)) = scene_twin(611, 0x4d10, &M611) else {
        return;
    };
    let (once, fa, fb, n0) = (0x15fu32, 0x1a7u32, 0x1b0u32, 0x22fu32);
    let nat = |t: &Twin, state: u32, set: &[(u32, u32)]| native(t, f, state, set);
    let cases = vec![
        nat(&t, 0, &[(once, 0), (fa, 1), (fb, 0), (n0, 2)]),
        nat(&t, 0, &[(once, 0), (fa, 0), (fb, 1), (n0, 2)]),
        nat(&t, 0, &[(once, 0), (fa, 0), (fb, 0), (n0, 2)]),
        Scene {
            a253: 0,
            ..nat(&t, 0, &[(once, 0)])
        },
        nat(&t, 0, &[(once, 1), (n0, 0x12)]),
        nat(&t, 0, &[(once, 1), (n0, 0x13)]),
        nat(&t, 0, &[(once, 1), (n0, 0x40)]),
    ];
    // the records slot 296 goes over (with 0): two
    let with = t.img.rel32_target(f + 0x2a9);
    let n_with = t.img.word(with + 7);
    t.set_u32(n_with, 2);
    run_scene(&mut t, f, &M611, cases);
}

/// `magic_0611` state 0x15 (its own head, `r39b::state_15_0611`, and the
/// shared tail with this copy's thresholds): once, busy, the count's 0,
/// 5 and 0x1e, the spread's fall from 0x2d, the flash off, the spread from
/// 0x55 with slot 279 busy and not and the waits, the parts at 0x2d,
/// 0x55 and 0x5b, the step into state 0x17.
#[test]
fn scene_0611_state_15() {
    let Some((mut t, f)) = scene_twin(611, 0x4d10, &M611) else {
        return;
    };
    let (once, n15, wait, left, count) = (0x2bbcu32, 0x2de4u32, 0x2f66u32, 0x2f72u32, 0x2f87u32);
    let nat = |t: &Twin, state: u32, set: &[(u32, u32)]| native(t, f, state, set);
    let mut cases = vec![
        nat(&t, 0x15, &[(once, 0), (n15, 0)]),
        Scene {
            a253: 0,
            ..nat(&t, 0x15, &[(once, 0)])
        },
    ];
    for n in [
        0u32, 4, 5, 0x1e, 0x2c, 0x2d, 0x2e, 0x3b, 0x3c, 0x54, 0x5b, 0xcc, 0xcb,
    ] {
        cases.push(nat(&t, 0x15, &[(once, 1), (n15, n), (count, 3)]));
    }
    for (n, w, l, ans) in [
        (0x55u32, 0u32, 2u32, 0u32),
        (0x60, 0, 2, 1),
        (0x60, 5, 2, 0),
        (0x60, 0, 0, 0),
        (0x80, 0, 7, 0),
    ] {
        let mut cs = nat(
            &t,
            0x15,
            &[(once, 1), (n15, n), (wait, w), (left, l), (count, 3)],
        );
        cs.mem.push((SCRATCH + 0x7f3c, ans));
        cases.push(cs);
    }
    // the placed parts' sizes: two records with sizes; the next actor
    let sizes = t.img.word(f + 0x2c2d);
    let recs = t.img.word(f + 0x2c3c);
    let next = t.img.word(f + 0x2fb4);
    for cs in &mut cases {
        cs.mem.push((sizes, 2));
        cs.mem.push((recs + 0x24, 0x3fc0_0000));
        cs.mem.push((recs + 0xcc + 0x24, 0x4020_0000));
        cs.mem.push((next, 2));
    }
    rec(&mut t, 279, 0, |m, _| m.r32(SCRATCH + 0x7f3c));
    rec(&mut t, 239, 3, |_, a| a[1] ^ 0x239);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            m.w32(
                a[0] + 4 * i,
                (1.25 * i as f32 + (a[1] & 3) as f32).to_bits(),
            );
        }
        0x325
    });
    run_scene(&mut t, f, &M611, cases);
}

/// A case whose globals are given by this copy's own operand offsets.
fn native(t: &Twin, f: u32, state: u32, set: &[(u32, u32)]) -> Scene {
    let mut cs = scene(state, &[]);
    cs.mem = set.iter().map(|&(op, v)| (t.img.word(f + op), v)).collect();
    cs
}

/// `magic_0667` 0x10004af0 (`r39b::scene_task_0667`): `scene_0515_states`'s
/// cases on the copy.
#[test]
fn scene_0667_states() {
    scene_states(667, 0x4af0, &M667);
}

/// `magic_0667` 0x10004af0 (`r39b::scene_task_0667`): `scene_0515_states_6_8`'s
/// cases on the copy.
#[test]
fn scene_0667_states_6_8() {
    scene_states_6_8(667, 0x4af0, &M667);
}

/// `magic_0667` 0x10004af0 (`r39b::scene_task_0667`): `scene_0515_states_9_d`'s
/// cases on the copy.
#[test]
fn scene_0667_states_9_d() {
    scene_states_9_d(667, 0x4af0, &M667);
}

/// `magic_0667` 0x10004af0 (`r39b::scene_task_0667`): `scene_0515_state_10`'s
/// cases on the copy.
#[test]
fn scene_0667_state_10() {
    scene_state_10(667, 0x4af0, &M667);
}

/// `magic_0667` 0x10004af0 (`r39b::scene_task_0667`): `scene_0515_state_18`'s
/// cases on the copy.
#[test]
fn scene_0667_state_18() {
    scene_state_18(667, 0x4af0, &M667);
}

/// `magic_0667` 0x10004af0 (`r39b::scene_task_0667`): `scene_0515_state_0`'s
/// cases on the copy.
#[test]
fn scene_0667_state_0() {
    scene_state_0(667, 0x4af0, &M667);
}

/// `magic_0667` 0x10004af0 (`r39b::scene_task_0667`): `scene_0515_state_3`'s
/// cases on the copy.
#[test]
fn scene_0667_state_3() {
    scene_state_3(667, 0x4af0, &M667);
}

/// `magic_0667` 0x10004af0 (`r39b::scene_task_0667`): `scene_0515_state_15`'s
/// cases on the copy.
#[test]
fn scene_0667_state_15() {
    scene_state_15(667, 0x4af0, &M667);
}

/// `magic_0667` 0x10004af0 (`r39b::scene_task_0667`): `scene_0515_state_17`'s
/// cases on the copy.
#[test]
fn scene_0667_state_17() {
    scene_state_17(667, 0x4af0, &M667);
}
