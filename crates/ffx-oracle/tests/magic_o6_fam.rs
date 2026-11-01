//! Round 6's shape families (`ffx_magic::o6_fam_*`) against every DLL's
//! own copy, and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names, clippy::cast_possible_truncation)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use ffx_magic::{
    Ret, Xmm, o6_fam_bone, o6_fam_launch, o6_fam_node, o6_fam_object, o6_fam_pairs2, o6_fam_pairs3,
    o6_fam_pairs4, o6_fam_pool,
};
use magic::{Call, Eax, SCRATCH, Twin};
use o4::{copies, dlls, fill};

const P: u32 = SCRATCH + 0x6000;
const OBJ: u32 = SCRATCH + 0x2000;

/// Every slot records its first word in a ring at `SCRATCH + 0xa00` and
/// answers a value made of its first word and its slot.
fn stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |m, a| {
            let k = m.r32(SCRATCH + 0xa00);
            for (i, w) in (0u32..).zip(a.iter().take(1)) {
                m.w32(SCRATCH + 0xa10 + 0x18 * (k % 0x40) + 4 * i, *w ^ s);
            }
            m.w32(SCRATCH + 0xa00, k + 1);
            a.first().copied().unwrap_or(0) ^ (s << 20)
        });
    }
}

/// The regs of a case.
fn regs(rva: u32, c: u32) -> Call {
    Call {
        eax: 0x00ea_0000 + c,
        ecx: 0x00ec_0000 + rva,
        edx: 0x00ed_0000 + rva,
        ..Call::default()
    }
}

const OBJECT_SETUP: &[(u16, u32)] = &[
    (361, 0x16a0),
    (361, 0x17f0),
    (361, 0x18f0),
    (361, 0x19f0),
    (384, 0x14b0),
    (415, 0x12a0),
    (193, 0x1660),
    (382, 0x1ae0),
];

/// `o6_fam_setup::object_setup`: the object's words filled
/// first, `*G` a block whose `+0x198` word is passed on, and the first
/// slot replacing `*G` (the third call reads it again).
#[test]
fn object_setup_in_every_copy() {
    for (n, rva) in copies("o6 fam setup object_setup", OBJECT_SETUP) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        let gp = t.img.word(f + 4);
        t.set_u32(gp, P);
        fill(&mut t, P + 0x190, 4, rva);
        t.stub(441, move |m, a| {
            m.w32(gp, (P + 0x40) ^ a[0]);
            0x441
        });
        fill(&mut t, OBJ + 0xd0, 0x1c, rva ^ 0x55);
        t.check_entry(f, regs(rva, 0), &[OBJ], Eax::Checked);
    }
}

/// The copy at `rva` is the internal function `name` registers.
fn registered(t: &Twin, rva: u32, name: &str) {
    let a = t.at(rva);
    let found = ffx_magic::INTERNAL
        .iter()
        .find(|(_, sig)| t.img.matches(a, sig))
        .map(|(n, _)| *n);
    let ok = found.is_some_and(|n| n.starts_with(name));
    assert!(ok, "{}: {a:#x} is {found:?}", t.name);
}

const POOL_RESET_TAKE: &[(u16, u32)] = &[
    (180, 0x2140),
    (205, 0x2a60),
    (243, 0x2800),
    (350, 0x2cf0),
    (385, 0x1df0),
    (417, 0x4350),
    (250, 0x1e00),
    (284, 0x3430),
];

/// `o6_fam_pool::pool_reset_take`: a pool outside the scratch area (the
/// largest is 0x800 records of 0x74 bytes) filled with non-zero words;
/// a free ring of three nodes with the count 3 or 1 (the head moves on
/// or becomes 0); the busy list empty or holding a node. The 216-byte
/// copies have their operands past `+0x33` 5 bytes earlier.
#[test]
fn pool_reset_take_in_every_copy() {
    for (k, &(n, rva)) in (0u32..).zip(POOL_RESET_TAKE) {
        // a pool of its own per DLL (the original's arena keeps the last
        // DLL's pages)
        let pool = 0x5000_0000 + 0x0100_0000 * k;
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pool pool_reset_take");
        let f = t.at(rva);
        let short = t.img.word(f + 0x33) as u8 != 0xa3;
        let op = |o: u32| if short && o > 0x33 { f + o - 5 } else { f + o };
        let count = t.img.word(f + 0x0a);
        let stride = u32::from(t.img.word(f + 0x18) as u8);
        let saved = t.save();
        for c in 0..4u32 {
            t.restore(&saved);
            t.write(pool, &vec![0x5a; (count * stride + 0x100) as usize]);
            let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
            for i in 0..3 {
                t.write(node(i), &[0x77; 0x20]);
                t.set_u32(node(i), node((i + 2) % 3));
                t.set_u32(node(i) + 4, node((i + 1) % 3));
            }
            t.set_u32(t.img.word(op(0x3e)), node(1));
            t.set_u16(t.img.word(f + 0x20), if c & 1 == 0 { 3 } else { 1 });
            let busy = SCRATCH + 0x1400;
            t.write(busy, &[0x66; 0x20]);
            let (first, last) = (t.img.word(op(0xa6)), t.img.word(op(0xb7)));
            t.set_u32(first, if c & 2 == 0 { 0 } else { busy });
            t.set_u32(last, busy);
            let used: &[u32] = if short {
                &[0x25, 0x2b]
            } else {
                &[0x25, 0x2b, 0x34]
            };
            for (k, &o) in (0u32..).zip(used) {
                let at = t.img.word(op(o));
                fill(&mut t, at, 1, c + k);
            }
            t.check(f, Call::this(pool), &[], Eax::Checked, |g, img, fr| {
                Ret::eax(o6_fam_pool::pool_reset_take(g, img, fr.at, fr.ecx))
            });
        }
    }
}

const HOST_START_PARTS: &[(u16, u32)] = &[
    (162, 0x10a0),
    (165, 0x10a0),
    (304, 0x10a0),
    (430, 0x10a0),
    (568, 0x10a0),
    (586, 0x10a0),
    (588, 0x10a0),
    (606, 0x6370),
];

/// `o6_fam_start::host_start_parts`: the object block from slot 168
/// (zeroed), slot 790's float, every other slot answering from its first
/// word; two arguments.
#[test]
fn host_start_parts_in_every_copy() {
    for (n, rva) in copies("o6 fam start host_start_parts", HOST_START_PARTS) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let block = SCRATCH + 0x2000;
        t.stub(168, move |_, _| block);
        t.write(block, &[0; 0x200]);
        // no arguments: its stub reads none
        t.stub(790, |_, _| 0x790);
        t.stub_st0(790, |_, _| 0.625);
        let f = t.at(rva);
        // what the start-up overwrites, filled first: the three scales,
        // pool A's 0x3ff payloads, the free count and rover (u16 each,
        // their neighbours too) and G_DONE (the last `mov [G], 0`)
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        for o in [0x57, 0x61, 0x6b] {
            t.set_u32(w(&t, o), 0x4040_0000);
        }
        let pool_a = w(&t, 0x81);
        t.write(pool_a, &vec![0x5a; 12 * 0x3ff]);
        for o in [0xa3, 0xab] {
            t.set_u32(w(&t, o), 0x7777_7777);
        }
        let done = (0x150..0x1c0u32)
            .find(|&k| t.img.word(f + k) == 0xc35d_c033)
            .map(|k| t.img.word(f + k - 8))
            .expect("the G_DONE store");
        t.set_u32(done, 0x6666_6666);
        for (c, arg) in (0u32..).zip([0x0a0a_0001u32, 0x0a0a_0002]) {
            t.check_entry(f, regs(rva, c), &[arg], Eax::Checked);
        }
    }
}

/// `o6_fam_object::start_object_first`'s copies with the offset of their
/// `call pool_alloc`.
const START_OBJECT_FIRST: &[(u16, u32, u32)] = &[
    (74, 0x1c70, 0x9a),
    (247, 0x25e0, 0x9a),
    (200, 0x2be0, 0x98),
    (369, 0x2c10, 0x8e),
    (166, 0x1fd0, 0x8e),
    (606, 0x4d70, 0x8f),
    (132, 0x3cc0, 0x9a),
    (126, 0x22a0, 0x9e),
];

/// `o6_fam_object::start_object_first`: the program list from slot 941,
/// the part start's slot 714 logged; the pool with 3, 2, 0 and 1 free
/// nodes, the object list empty or not, several scales (a NaN too).
#[test]
fn start_object_first_in_every_copy() {
    for &(n, rva, call) in START_OBJECT_FIRST {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam object start_object_first");
        let at = t.at(rva);
        let framed = t.img.word(at) as u8 == 0x55;
        let (p_op, d_op) = if framed { (0x04, 0x5e) } else { (0x01, 0x5b) };
        t.set_u32(t.img.word(at + p_op), SCRATCH + 0x200);
        t.set_u32(t.img.word(at + d_op), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        for i in 0..8 {
            t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
        }
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x900, a[0]);
            SCRATCH + 0x400
        });
        t.stub(714, |m, a| {
            for (i, w) in (0u32..).zip(&a[..5]) {
                m.w32(SCRATCH + 0x910 + 4 * i, *w);
            }
            0x714
        });
        let pa = t.img.rel32_target(at + call + 1);
        let free = t.img.word(pa + 0x04);
        let nodes = t.img.word(pa + 0x27) - 8;
        for i in 0..8 {
            t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
            t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
            t.set_u32(nodes + 12 * i + 8, 0);
        }
        let tail = t.img.word(at + call + 0xc);
        // the object count (both branches' `inc [G_N]`)
        let framed_off = if framed { 0x32 } else { 0x31 };
        for op in [call + 5 + 0x29, call + 5 + framed_off + 0x15] {
            let count_at = t.img.word(at + op);
            t.set_u32(count_at, 0x40 + op);
        }
        let obj = SCRATCH + 0x1000;
        for (k, (count, scale, tl)) in (0u32..).zip([
            (3u16, 0.5f32, 0u32),
            (2, -2.0, SCRATCH + 0x600),
            (0, 1.0, 0),
            (1, f32::NAN, SCRATCH + 0x700),
        ]) {
            t.write(obj, &[0x5a; 0x130]);
            t.set_u16(free, count);
            t.set_u32(tail, tl);
            let c = Call {
                edx: obj,
                ecx: 0x0bad_0ecc,
                ..Call::default()
            }
            .with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(scale)]);
            let args = [2 * k + 1, 0x0a00 + k, 0xdead, 0x0b00 + k];
            t.check(at, c, &args, Eax::Checked, |g, img, fr| {
                Ret::eax(o6_fam_object::start_object_first(g, img, fr.at, fr))
            });
        }
    }
}

const BONE_NODE_SETUP: &[(u16, u32)] = &[(201, 0x6f20), (203, 0x7710), (273, 0x7770)];

/// `o6_fam_bone::bone_node_setup`: the root's ring empty or holding a
/// node, with the pass byte the node's own 0xff, so the bone update
/// returns at once (with another pass it calls the node's methods
/// through the DLL's vtable, whose own code has no port yet;
/// `sa_summon::bone_plain` has its own tests); slot 233 writes its first
/// word, slot 230 the matrix.
#[test]
fn bone_node_setup_in_every_copy() {
    for &(n, rva) in BONE_NODE_SETUP {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam bone bone_node_setup");
        stubs(&mut t);
        t.stub(233, |m, a| {
            m.w32(a[0], a[2] ^ 0x8000_0000);
            0x233
        });
        t.stub(230, |m, a| {
            m.w32(a[0], a[1] ^ 0x2300_0000);
            0x230
        });
        // slot 858 folds the two vectors it is handed into a log word
        t.stub(858, |m, a| {
            let mut x = 0u32;
            for i in 0..4 {
                x = x.rotate_left(5) ^ m.r32(a[2] + 4 * i) ^ m.r32(a[3] + 4 * i);
            }
            m.w32(SCRATCH + 0x9f0, x);
            0x858
        });
        let f = t.at(rva);
        // the bone's table entries: a function slot no overlay uses
        // (`Ch_SetPos`, slot 32)
        let bone = t.img.rel32_target(f + 0xf9);
        let table = t.img.word(bone + 0xed);
        for i in 0..4 {
            t.set_u32(table + 4 * i, magic::TRAPS + 16 * 32);
        }
        t.stub(32, |_, a| 0x0032_0000 ^ a[0]);
        let root = t.img.word(f + 0x4b);
        let pass_at = t.img.word(f + 0xeb);
        let (model, src, other) = (OBJ, SCRATCH + 0x3000, SCRATCH + 0x3800);
        for c in 0..2u32 {
            t.write(model, &[0x5a; 0x200]);
            t.write(src, &[0x3c; 0x40]);
            t.write(other, &[0x22; 0x20]);
            t.set_u32(root + 0x10, if c & 1 == 0 { root } else { other });
            t.set_u16(root + 0x18, 3);
            t.set_u8(pass_at, 0xff);
            let call = Call {
                ecx: model,
                edx: src,
                ..Call::default()
            };
            // the stack word becomes the node's `+0x144` node
            let arg = SCRATCH + 0x3c00;
            t.write(arg, &[0x44; 0x200]);
            t.check(f, call, &[arg + 4 * c], Eax::Checked, |g, img, fr| {
                o6_fam_bone::bone_node_setup(g, img, fr)
            });
        }
    }
}

const EFFECT_BOOT: &[(u16, u32)] = &[
    (368, 0x1140),
    (527, 0x11c0),
    (255, 0x12e0),
    (617, 0x11b0),
    (372, 0x1220),
    (669, 0x1ce0),
    (206, 0x1940),
];

/// `o6_fam_boot::effect_boot`: slot 803 gives the block, slot 719 a
/// handle whose three list words slot 941 turns into lists, slot 711
/// records the game lock it runs under; the cleared table filled first.
#[test]
fn effect_boot_in_every_copy() {
    for (n, rva) in copies("o6 fam boot effect_boot", EFFECT_BOOT) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let handle = SCRATCH + 0x3000;
        t.stub(803, |_, a| 0x0100_0000 ^ a[1]);
        t.stub(719, move |_, a| handle ^ (a[1] & 0xf0));
        t.write(handle, &[0x33; 0x100]);
        fill(&mut t, handle + 0x14, 3, n.into());
        t.stub(941, |_, a| a[0] ^ 0x0941_0000);
        // slot 711 sees the lock word (the global of the copy's only
        // `mov ecx, [G]`)
        let f = t.at(rva);
        let lock_at = (0..0x100u32)
            .find(|&k| t.img.word(f + k) & 0xffff == 0x0d8b)
            .expect("mov ecx, [G]");
        let lock_global = t.img.word(f + lock_at + 2);
        let lock = SCRATCH + 0x800;
        t.set_u32(lock_global, lock);
        t.set_u32(lock, 0x5555);
        t.stub(711, move |m, a| {
            let v = m.r32(lock);
            m.w32(SCRATCH + 0x810, v ^ a[0]);
            0x711
        });
        // the cleared table: the push before `mov [G], eax; call memset`
        for k in 0..0x200u32 {
            let p = f + k;
            if t.img.word(p) & 0xff == 0x68
                && t.img.word(p + 5) & 0xff == 0xa3
                && t.img.word(p + 10) & 0xff == 0xe8
            {
                let m = t.img.word(p + 1);
                t.write(m, &[0x7e; 0x110]);
            }
        }
        for c in 0..2u32 {
            t.check_entry(f, regs(rva, c), &[0x0a0a_0000 + c], Eax::Checked);
        }
    }
}

const NODE_RESET_SOUND: &[(u16, u32)] = &[(577, 0x36d0), (155, 0x2fc0), (70, 0x3ed0)];

/// `o6_fam_node::node_reset_sound`: no parent, a parent without children
/// and one with a child; the node's words filled first; slot 287 gives
/// the handle, slot 282 a float.
#[test]
fn node_reset_sound_in_every_copy() {
    for &(n, rva) in NODE_RESET_SOUND {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam node node_reset_sound");
        stubs(&mut t);
        t.stub(287, |_, a| 0x0287_0000 ^ a[0]);
        t.stub(282, |_, _| 0x282);
        t.stub_st0(282, |_, _| 0.3);
        let f = t.at(rva);
        let (rec, node, parent, child) = (OBJ, OBJ + 0x100, P, P + 0x100);
        for c in 0..3u32 {
            t.write(node, &[0x5a; 0x140]);
            t.write(parent, &[0x44; 0x40]);
            t.write(child, &[0x22; 0x20]);
            t.set_u32(parent + 0x10, if c == 2 { child } else { parent });
            t.set_u16(parent + 0x18, 2);
            t.set_u32(rec, node);
            t.set_u32(rec + 4, if c == 0 { 0 } else { parent });
            t.set_u32(rec + 8, 0x3fc0_0000 + c);
            t.check(f, Call::this(rec), &[], Eax::Checked, |g, img, fr| {
                Ret::eax(o6_fam_node::node_reset_sound(g, img, fr.at, fr.ecx))
            });
        }
    }
}

const RANDOM_LAUNCH: &[(u16, u32)] = &[(384, 0x15e0), (421, 0x15e0), (535, 0x15c0)];

/// `o6_fam_launch::random_launch`: with and without the angle step,
/// several angles; slot 474 writes the result vector from the start
/// vector and its word, the particle's words filled first, its slot
/// index `+0xac` 1.
#[test]
fn random_launch_in_every_copy() {
    for &(n, rva) in RANDOM_LAUNCH {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam launch random_launch");
        stubs(&mut t);
        t.stub(474, |m, a| {
            for (i, step) in (0u32..).zip([0.5f32, 1.0, 1.5, 2.0]) {
                let v = m.r32(a[2] + 4 * i);
                let x = f32::from_bits(v) + step;
                m.w32(a[3] + 4 * i, x.to_bits() ^ (a[0] & 1));
            }
            0x474
        });
        let f = t.at(rva);
        let p = OBJ;
        // the DLL's globals the arithmetic reads: the count `N` (each of
        // its three reads), the scale `S` and `G2`, non-zero and distinct
        for op in [0x97, 0x193, 0x20d] {
            t.set_u32(t.img.word(f + op), 5);
        }
        t.set_u32(t.img.word(f + 0x67), 1.5f32.to_bits());
        t.set_u32(t.img.word(f + 0xa2), 1.25f32.to_bits());
        t.set_u32(t.img.word(f + 0xc7), 0.75f32.to_bits());
        let g_d = t.u32(t.img.word(f + 0xd4));
        if g_d != 0 {
            t.set_u32(g_d + 0x198, 0x0198_0000 + u32::from(n));
        }
        for (c, (count, angle)) in (0u32..).zip([(0u32, 0.25f32), (3, -1.5), (1, 2.75)]) {
            // flag bit 14 clear (the `or` must show)
            t.write(p, &[0x11; 0x200]);
            t.set_u32(p + 0xac, 1);
            let call = Call {
                ecx: p,
                edx: count,
                eax: 0x00ea_0000 + c,
                ..Call::default()
            }
            .with_xmm(&[Xmm::f32(0.0), Xmm::f32(angle)]);
            t.check(f, call, &[], Eax::Checked, |g, img, fr| {
                Ret::eax(o6_fam_launch::random_launch(g, img, fr.at, fr))
            });
        }
    }
}

/// A free ring of `n` nodes at `at` for the pool take at `take` (its
/// count `+0x04`, its head `+0x17`).
fn free_ring(t: &mut Twin, take: u32, at: u32, n: u32) {
    let node = |i: u32| at + 0x40 * i;
    for i in 1..=n {
        t.write(node(i), &[0x5a; 0x1c]);
        t.set_u32(node(i), node(if i == 1 { n } else { i - 1 }));
        t.set_u32(node(i) + 4, node(if i == n { 1 } else { i + 1 }));
    }
    let head = t.img.word(take + 0x17);
    t.set_u32(head, node(1));
    let count = t.img.word(take + 0x04);
    t.set_u16(count, n as u16);
}

const SCENE_START: &[(u16, u32)] = &[(162, 0x3820), (586, 0x3430)];

/// `o6_fam_pairs::scene_start`: the scene block filled, the table entry
/// chosen by the second byte of `*G_B`; the object list empty or not.
#[test]
fn scene_start_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs scene_start", SCENE_START) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let scene = 0x6800_0000 + 0x0010_0000 * u32::from(n % 16);
        let (b, busy) = (SCRATCH + 0x3000, SCRATCH + 0x3400);
        t.set_u32(w(&t, 0x08), scene);
        t.set_u32(w(&t, 0x38), 0x0a0a_0001);
        t.set_u32(w(&t, 0x45), b);
        // the index byte with non-zero neighbours
        t.write(b, &[0x5a, 3, 0x5a, 0x5a]);
        let table = w(&t, 0x52);
        fill(&mut t, table, 6, 0x52);
        let take = t.img.rel32_target(f + 0x8a);
        for c in 0..2u32 {
            t.write(scene + 0x0a00, &[0x11; 0x40]);
            t.write(scene + 0x85c0, &[0x22; 0x40]);
            free_ring(&mut t, take, SCRATCH + 0x1000, 3);
            t.write(busy, &[0x33; 0x20]);
            let (first, last) = (w(&t, 0x90), w(&t, 0xa0));
            t.set_u32(first, if c == 0 { 0 } else { busy });
            t.set_u32(last, busy);
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const RING_PART_START: &[(u16, u32)] = &[(666, 0x1250), (666, 0x2080)];

/// `o6_fam_pairs::ring_part_start`: objects inside and past the first
/// 0x80 bytes of the base, the count below and at the limit; the part
/// start's slot 714 logged.
#[test]
fn ring_part_start_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs ring_part_start", RING_PART_START) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        t.set_u32(w(&t, 0x04), SCRATCH + 0x200);
        t.set_u32(w(&t, 0x10), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        fill(&mut t, SCRATCH + 0x400, 0x10, 0x41);
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x900, a[0]);
            SCRATCH + 0x400
        });
        let base = w(&t, 0x81);
        let limit = w(&t, 0xae);
        for (c, (off, count)) in (0u32..).zip([
            (0u32, limit),
            (0x40, limit - 1),
            (0x80, limit),
            (0x7c, limit + 5),
        ]) {
            let obj = base + off;
            t.write(obj, &[0x5a; 0x80]);
            t.set_u32(w(&t, 0xaa), count);
            t.check_entry(f, regs(rva, c), &[obj], Eax::Checked);
        }
    }
}

const ANGLE_RESET: &[(u16, u32)] = &[(353, 0x1980), (672, 0x1500)];

/// `o6_fam_pairs::angle_reset`: slot 333's angle above the upper limit,
/// below the lower one, between them and NaN; slot 12 a float.
#[test]
fn angle_reset_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs angle_reset", ANGLE_RESET) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let kf = |t: &Twin, o: u32| f32::from_bits(t.u32(t.img.word(f + o)));
        let block = 0x6000_0000 + 0x0010_0000 * u32::from(n % 16);
        t.set_u32(w(&t, 0x0e), block);
        t.set_u32(w(&t, 0x26), 0x0a0a_0026);
        t.stub(12, |_, a| 0x12 ^ a[0]);
        t.stub_st0(12, |_, _| 0.75);
        let (k1, hi, lo) = (kf(&t, 0x36), kf(&t, 0x45), kf(&t, 0x59));
        for (c, x) in (0u32..).zip([hi + 1.0, lo - 1.0, hi.midpoint(lo), f32::NAN, hi, lo]) {
            t.write(block + 0xe40, &[0x5a; 0x40]);
            t.write(block + 0x13e0, &[0x5a; 0x20]);
            let r = f64::from(k1 - x);
            t.stub(333, |_, a| 0x333 ^ a[0]);
            t.stub_st0(333, move |_, _| r);
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const PLACED_PART_START: &[(u16, u32)] = &[(206, 0x17b0), (376, 0x1170)];

/// `o6_fam_pairs::placed_part_start`: objects inside and past the first
/// 0x80 bytes of the base; the part start's slot 714 logged.
#[test]
fn placed_part_start_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs placed_part_start", PLACED_PART_START) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        t.set_u32(w(&t, 0x14), SCRATCH + 0x200);
        t.set_u32(w(&t, 0x1f), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        fill(&mut t, SCRATCH + 0x400, 0x10, 0x41);
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x900, a[0]);
            SCRATCH + 0x400
        });
        let base = w(&t, 0x0a);
        for (c, off) in (0u32..).zip([0u32, 0x7c, 0x80, 0x400]) {
            let obj = base + off;
            t.write(obj, &[0x5a; 0x80]);
            t.check_entry(f, regs(rva, c), &[obj], Eax::Checked);
        }
    }
}

const SLOT_PART_START: &[(u16, u32)] = &[(371, 0x1520), (371, 0x1620)];

/// `o6_fam_pairs::slot_part_start`: objects in slots 0, 1, 2, 3 and
/// below the base (slot -1).
#[test]
fn slot_part_start_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs slot_part_start", SLOT_PART_START) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        t.set_u32(w(&t, 0x04), SCRATCH + 0x200);
        t.set_u32(w(&t, 0x13), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        fill(&mut t, SCRATCH + 0x400, 0x10, 0x41);
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x900, a[0]);
            SCRATCH + 0x400
        });
        let base = w(&t, 0x2a);
        // the objects are the DLL's own records (their neighbours are its
        // globals, so nothing is filled first, and each case starts from
        // the same memory: a stray slot's stores may reach the globals)
        let saved = t.save();
        for (c, obj) in (0u32..).zip([base, base + 0x80, base + 0x100, base + 0x180, base - 0x80]) {
            t.restore(&saved);
            t.check_entry(f, regs(rva, c), &[obj], Eax::Checked);
        }
    }
}

const MODELS_DETACH: &[(u16, u32)] = &[(164, 0x7cf0), (155, 0x7c70)];

/// `o6_fam_pairs2::models_detach`: no models, one and three; the models'
/// nodes with no parent, a parent where they are the only child, and a
/// parent with siblings on both sides.
#[test]
fn models_detach_in_every_copy() {
    for &(n, rva) in MODELS_DETACH {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pairs2 models_detach");
        stubs(&mut t);
        let f = t.at(rva);
        let summon = 0x6400_0000 + 0x0010_0000 * u32::from(n % 16);
        let rec = SCRATCH + 0x3000;
        let (parent, sib_a, sib_b) = (SCRATCH + 0x3100, SCRATCH + 0x3200, SCRATCH + 0x3300);
        for (c, count) in (0u32..).zip([0u32, 1, 3]) {
            t.write(summon, &[0x5a; 0x900]);
            t.set_u32(summon + 0x162c, count);
            t.write(parent, &[0x44; 0x40]);
            t.write(sib_a, &[0x33; 0x40]);
            t.write(sib_b, &[0x22; 0x40]);
            t.set_u16(parent + 0x18, 5);
            for i in 0..count {
                let model = summon + 0xbc + 0x274 * i;
                t.set_u32(model + 0x90, 0x0000_0001);
                let second = model + 0x98;
                match (i + c) % 3 {
                    0 => {
                        t.set_u32(model + 0x14, 0);
                        t.set_u32(second + 0x14, parent);
                        t.set_u32(second + 4, parent);
                        t.set_u32(second + 8, parent);
                    }
                    1 => {
                        t.set_u32(model + 0x14, parent);
                        t.set_u32(model + 4, parent);
                        t.set_u32(model + 8, parent);
                        t.set_u32(second + 0x14, 0);
                    }
                    _ => {
                        t.set_u32(model + 0x14, parent);
                        t.set_u32(model + 4, sib_a);
                        t.set_u32(model + 8, sib_b);
                        t.set_u32(second + 0x14, parent);
                        t.set_u32(second + 4, sib_b);
                        t.set_u32(second + 8, sib_a);
                    }
                }
            }
            t.set_u32(rec, summon);
            t.check(f, Call::this(rec), &[], Eax::Checked, |g, img, fr| {
                o6_fam_pairs2::models_detach(g, img, fr)
            });
        }
    }
}

const LIST_ANGLE_RESET: &[(u16, u32)] = &[(348, 0x6600), (350, 0x5a50)];

/// `o6_fam_pairs2::list_angle_reset`: lists of 0 and 3 bytes (slot 288
/// writes the count it is handed a pointer to); slot 333's angle above,
/// below and between the limits.
#[test]
fn list_angle_reset_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs2 list_angle_reset", LIST_ANGLE_RESET) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let kf = |t: &Twin, o: u32| f32::from_bits(t.u32(t.img.word(f + o)));
        let block = 0x6c00_0000 + 0x0010_0000 * u32::from(n % 16);
        t.set_u32(w(&t, 0x09), block);
        t.set_u32(w(&t, 0x5f), 0x0a0a_005f);
        let list = SCRATCH + 0x3000;
        t.write(list, &[7, 9, 11, 13]);
        t.stub(12, |_, a| 0x12 ^ a[0]);
        t.stub_st0(12, |_, _| 0.75);
        let (k1, hi, lo) = (kf(&t, 0x72), kf(&t, 0x81), kf(&t, 0x95));
        for (c, x) in (0u32..).zip([hi + 1.0, lo - 1.0, hi.midpoint(lo), hi, lo]) {
            t.write(block + 0xd10, &[0x5a; 0x40]);
            t.set_u32(block + 0xd180, 0x5a5a);
            let count = 3 * (c & 1);
            t.stub(288, move |m, a| {
                m.w32(a[1], count);
                list
            });
            let r = f64::from(k1 - x);
            t.stub(333, |_, a| 0x333 ^ a[0]);
            t.stub_st0(333, move |_, _| r);
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const ACTOR_OBJECT_START: &[(u16, u32)] = &[(644, 0x8df0), (565, 0x4dd0)];

/// `o6_fam_pairs2::actor_object_start`: the object start's environment
/// as in `magic_s4_misc.rs` (`start_object_at`): slot 941's list, slot
/// 714 logged, the pool with free nodes; slot 641 gives index 0 or 1; the
/// actor's byte index -1 or 2; the mode flag clear or set.
#[test]
fn actor_object_start_in_every_copy() {
    for &(n, rva) in ACTOR_OBJECT_START {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pairs2 actor_object_start");
        stubs(&mut t);
        let f = t.at(rva);
        let at = t.img.rel32_target(f + 0x3e);
        t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(at + 0x5e), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        for i in 0..8 {
            t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
        }
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x900, a[0]);
            SCRATCH + 0x400
        });
        t.stub(714, |m, a| {
            for (i, w) in (0u32..).zip(&a[..5]) {
                m.w32(SCRATCH + 0x910 + 4 * i, *w);
            }
            0x714
        });
        let pa = t.img.rel32_target(at + 0x9d);
        let free = t.img.word(pa + 0x04);
        let nodes = t.img.word(pa + 0x27) - 8;
        let tail = t.img.word(at + 0xa8);
        let (bytes_k, bytes_t) = (SCRATCH + 0x3000, SCRATCH + 0x3100);
        t.write(bytes_k - 4, &[1, 2, 3, 4, 5, 6, 7, 8]);
        t.write(bytes_t, &[9, 10, 11, 12, 13, 14, 15, 16]);
        t.set_u32(t.img.word(f + 0x10), bytes_k);
        t.set_u32(t.img.word(f + 0x99), bytes_t);
        let mode = SCRATCH + 0x3200;
        t.set_u32(t.img.word(f + 0xa8), mode - 0xd618);
        let (actor, pos) = (SCRATCH + 0x8000, SCRATCH + 0x3300);
        fill(&mut t, pos, 2, 0x99);
        // the two tables slot 641's index `k` picks from (their operands
        // are biased bases): `k` chosen so that both land in scratch
        // memory, with distinct scales and program indices (0..7, the
        // list's entries)
        let (scales, indices) = (t.img.word(f + 0x1f), t.img.word(f + 0x37));
        let k0 = (SCRATCH + 0x5000).wrapping_sub(indices) / 4;
        for i in 0..4u32 {
            let k = k0 + i;
            let scale = 0.5f32 + 0.25 * f32::from(i as u8);
            t.set_u32(scales.wrapping_add(4 * k), scale.to_bits());
            t.set_u32(indices.wrapping_add(4 * k), 5 - i);
        }
        for c in 0..4u32 {
            for i in 0..8 {
                t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
                t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
                t.set_u32(nodes + 12 * i + 8, 0);
            }
            t.set_u16(free, 3);
            t.set_u32(tail, 0);
            t.write(actor, &[0x5a; 0x520]);
            t.set_u32(actor + 0x514, if c & 1 == 0 { u32::MAX } else { 2 });
            t.set_u32(mode, c & 2);
            let k = k0 + (c & 1);
            t.stub(641, move |_, _| k);
            let call = Call {
                ecx: actor,
                edx: pos,
                ..Call::default()
            };
            t.check(f, call, &[], Eax::Checked, |g, img, fr| {
                o6_fam_pairs2::actor_object_start(g, img, fr)
            });
        }
    }
}

/// `o6_fam_pairs2::shake_visit` (internal) and its thunk (entry) per
/// DLL: `(DLL, visit, thunk)`.
const SHAKE_VISIT: &[(u16, u32, u32)] = &[(201, 0x4780, 0x4910), (387, 0x5eb0, 0x6040)];

/// The shaking node through its thunk: the walk stopping at once (the
/// owner already at the group's pass), the flags' bit 0 clear, the timer
/// running (slot 307's float, slot 324's signs), the timer at 0 with and
/// without bit 1 (the node leaving its parent, which has a sibling).
#[test]
fn shake_visit_in_every_copy() {
    for &(n, visit, thunk) in SHAKE_VISIT {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, visit, "o6 fam pairs2 shake_visit");
        let thunks: Vec<(u16, u32)> = vec![(n, thunk)];
        copies("o6 fam pairs2 shake_visit_thunk", &thunks);
        stubs(&mut t);
        // no arguments: their stubs read none
        t.stub(307, |_, _| 0x307);
        t.stub_st0(307, |_, _| 0.375);
        for (k, s) in [(0u32, 324u32), (1, 250)] {
            t.stub(s, move |m, a| {
                let i = m.r32(SCRATCH + 0x9e0);
                m.w32(SCRATCH + 0x9e0, i + 1);
                if k == 1 {
                    m.w32(SCRATCH + 0x9e4 + 4 * (i % 4), a[0] ^ a[1].rotate_left(7));
                }
                0x0f + 3 * (i % 2)
            });
        }
        let f = t.at(thunk);
        let (group, owner, parent, sib) = (
            SCRATCH + 0x3000,
            SCRATCH + 0x3100,
            SCRATCH + 0x3200,
            SCRATCH + 0x3300,
        );
        for (c, (pass, flags, timer)) in (0u32..).zip([
            (7u8, 1u8, 1.0f32),
            (2, 0, 1.0),
            (2, 1, 2.0),
            (2, 3, 0.0),
            (2, 1, 0.0),
        ]) {
            t.write(owner, &[0x5a; 0x40]);
            t.write(parent, &[0x44; 0x40]);
            t.write(sib, &[0x33; 0x40]);
            t.write(group, &[0x11; 8]);
            t.set_u32(group, owner);
            t.set_u8(group + 5, 7);
            t.set_u8(owner + 0x1a, pass);
            t.set_u32(owner + 0x10, owner);
            t.set_u8(owner + 0x30, flags);
            for (i, v) in (0u32..).zip([0.5f32, 1.5, -2.0, 3.0]) {
                t.set_u32(owner + 0x1c + 4 * i, v.to_bits());
            }
            t.set_u32(owner + 0x2c, timer.to_bits());
            t.set_u32(owner + 0x14, parent);
            t.set_u32(owner + 4, parent);
            t.set_u32(owner + 8, sib);
            t.set_u16(parent + 0x18, 2);
            t.check_entry(f, regs(thunk, c), &[group], Eax::Checked);
        }
    }
}

const TWO_NODE_START: &[(u16, u32)] = &[(638, 0x3270), (689, 0x32c0)];

/// `o6_fam_pairs3::two_node_start`: the part set-up's lock word at a
/// scratch word; the pool with free nodes; the list empty or holding a
/// node (`G_C` is one of the stored buffer globals, inside the DLL).
#[test]
fn two_node_start_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs3 two_node_start", TWO_NODE_START) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let part_init = t.img.rel32_target(f + 0x3f);
        let lock = SCRATCH + 0x800;
        t.set_u32(t.img.word(part_init + 0x01), lock);
        t.set_u32(lock, 0x5555);
        let take = t.img.rel32_target(f + 0x7f);
        let busy = SCRATCH + 0x3400;
        let saved = t.save();
        for c in 0..2u32 {
            t.restore(&saved);
            free_ring(&mut t, take, SCRATCH + 0x1000, 4);
            t.write(busy, &[0x33; 0x20]);
            // what the stores overwrite with zeros, non-zero first: the
            // block word `(*G_C)[0x130]` (`G_C` gets its immediate first),
            // a word and a u16 global
            let block = t.img.word(f + 0x30).wrapping_add(t.img.word(f + 0x4d));
            t.set_u32(block, 0x7777_7777);
            t.set_u32(w(&t, 0x66), 0x6666_6666);
            t.set_u32(w(&t, 0x71), 0x5555_5555);
            t.set_u32(w(&t, 0x85), if c == 0 { 0 } else { busy });
            t.set_u32(w(&t, 0x9c), busy);
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

/// Both copies bind to the one port (the same code, `magic_0434`'s name).
const MATRIX_SCALE_SETUP: &[(u16, u32)] = &[(434, 0x7c90), (671, 0x1390)];

/// `o6_fam_pairs3::matrix_scale_setup`: slots 723/724 fill the matrices
/// they are handed, slot 229 reads the vector, slot 282 a float (and a
/// zero, which gives an infinite scale).
#[test]
fn matrix_scale_setup_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs3 matrix_scale_setup", MATRIX_SCALE_SETUP) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        for s in [723u32, 724] {
            t.stub(s, move |m, a| {
                for i in 0..16 {
                    m.w32(a[0] + 4 * i, (s << 8) ^ i);
                }
                s
            });
        }
        t.stub(229, |m, a| {
            let mut x = 0u32;
            for i in 0..4 {
                x = x.rotate_left(3) ^ m.r32(a[2] + 4 * i);
            }
            m.w32(SCRATCH + 0x9f0, x ^ a[0] ^ a[1]);
            0x229
        });
        let f = t.at(rva);
        let obj = SCRATCH + 0x3000;
        for (c, r) in (0u32..).zip([2.5f64, 0.0]) {
            t.write(obj, &[0x5a; 0x100]);
            t.stub(282, |_, _| 0x282);
            t.stub_st0(282, move |_, _| r);
            t.check_entry(f, regs(rva, c), &[obj], Eax::Checked);
        }
    }
}

/// Both copies bind to the one port (the same code, `magic_0644`'s name).
const PARTY_COLOURS: &[(u16, u32)] = &[(644, 0x6150), (565, 0x1fc0)];

/// `o6_fam_pairs3::party_colours`: no members and three, their tracks'
/// indices before the end and at it (wrapping to the first byte); slot
/// 808 reads the colour table it is handed.
#[test]
fn party_colours_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs3 party_colours", PARTY_COLOURS) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        t.stub(651, |_, _| 0x651);
        t.stub_st0(651, |_, _| 0.625);
        t.stub(808, |m, a| {
            let x = m.r32(a[1]) ^ m.r32(a[1] + 4).rotate_left(8) ^ m.r32(a[1] + 8).rotate_left(16);
            m.w32(SCRATCH + 0x9f0, x ^ a[0] ^ a[2] ^ a[3]);
            0x808
        });
        let f = t.at(rva);
        let (recs, members) = (t.img.word(f + 0x33), t.img.word(f + 0x38));
        // three tracks, one per colour byte, and the slot's first word
        let tracks = [SCRATCH + 0x3000, SCRATCH + 0x3020, SCRATCH + 0x3040];
        for (k, &tr) in (0u8..).zip(&tracks) {
            t.set_u32(tr, 4);
            t.set_u32(tr + 4, tr + 0x10);
            t.write(tr + 0x10, &[0x21 + k, 0x32 + k, 0x43 + k, 0x54 + k]);
        }
        let p_at = t.img.word(f + 0xdf);
        t.set_u32(p_at, 0x0a0a_00df);
        let saved = t.save();
        for (c, count) in (0u32..).zip([0u32, 3]) {
            t.restore(&saved);
            t.set_u32(t.img.word(f + 0x13), count);
            for i in 0..count {
                let rec = recs + 0x78 * i;
                for (k, (tr, ix)) in (0u32..).zip([
                    (rec - 0x1c, rec - 0x16),
                    (rec, rec + 6),
                    (rec + 0x1c, rec + 0x22),
                ]) {
                    t.set_u32(tr, tracks[k as usize]);
                    t.set_u16(ix, ((i + k) % 4) as u16);
                }
                t.set_u32(members + 4 * i, 2 - i);
            }
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

/// Both copies bind to the one port (the same code, `magic_0249`'s name).
const BONE_TRANSFORM: &[(u16, u32)] = &[(249, 0x1d70), (562, 0x1d80)];

/// `o6_fam_pairs3::bone_transform`: slot 237 reads the vector it is
/// handed, slot 277 a float (and a NaN-free zero: an infinite scale).
#[test]
fn bone_transform_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs3 bone_transform", BONE_TRANSFORM) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        t.stub(237, |m, a| {
            let mut x = 0u32;
            for i in 0..4 {
                x = x.rotate_left(3) ^ m.r32(a[2] + 4 * i);
            }
            m.w32(SCRATCH + 0x9f0, x ^ a[0] ^ a[1] ^ a[3]);
            0x237
        });
        let f = t.at(rva);
        let obj = SCRATCH + 0x3000;
        for (c, r) in (0u32..).zip([4.0f64, 0.5, 0.0]) {
            t.write(obj, &[0x5a; 0x140]);
            for i in 0..12 {
                let v = (f32::from(i as u8) - 5.5) * 0.375;
                t.set_u32(obj + 0x80 + 4 * i, v.to_bits());
            }
            t.stub(277, |_, a| 0x277 ^ a[0]);
            t.stub_st0(277, move |_, _| r);
            t.check_entry(f, regs(rva, c), &[obj], Eax::Checked);
        }
    }
}

/// The node-handler thunks: `(DLL, thunk, label, cases)`; the cases
/// past the first run the handler past its walk.
const THUNKS: &[(u16, u32, &str, u32)] = &[
    (164, 0x80c0, "o6 fam pairs3 models_detach_thunk", 2),
    (155, 0x8040, "o6 fam pairs3 models_detach_thunk", 2),
    (262, 0x4650, "o6 fam pairs3 walk_scale_steps_thunk", 2),
    (42, 0x10b0, "o6 fam pairs3 visit_pool_thunk", 2),
    (71, 0x4080, "o6 fam pairs3 spin_visit_thunk", 1),
];

/// Each thunk with its cursor on the stack: the summon's models (none,
/// one) for `models_detach`; for the node visits a group whose owner is
/// already at its pass (the walk stops), then one at another pass with
/// no children (its flags and fields filled; the spinning node's sine
/// table is the game's, so it runs only the first case).
#[test]
fn thunks_in_every_copy() {
    for &(n, rva, label, cases) in THUNKS {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        copies(label, &[(n, rva)]);
        stubs(&mut t);
        let f = t.at(rva);
        let (cursor, owner, summon) = (SCRATCH + 0x3000, SCRATCH + 0x3100, 0x6600_0000);
        for c in 0..cases {
            if label.contains("models_detach") {
                t.write(summon, &[0x5a; 0x400]);
                t.set_u32(summon + 0x162c, c);
                t.set_u32(summon + 0xbc + 0x14, 0);
                t.set_u32(summon + 0xbc + 0x98 + 0x14, 0);
                t.set_u32(summon + 0xbc + 0x90, 1);
                t.set_u32(cursor, summon);
            } else {
                t.write(owner, &[0x5a; 0x80]);
                for i in 0..8 {
                    t.set_u32(owner + 0x2c + 4 * i, (i + 2) * 0x0100_0000);
                }
                t.set_u32(owner + 0x10, owner);
                t.set_u8(owner + 0x1a, if c == 0 { 7 } else { 3 });
                t.set_u8(owner + 0x1c, 0);
                t.set_u32(cursor, owner);
                t.set_u8(cursor + 4, 0x11);
                t.set_u8(cursor + 5, 7);
            }
            t.check_entry(f, regs(rva, c), &[cursor], Eax::Checked);
        }
    }
}

const RANDOM_PARTICLE_SETUP: &[(u16, u32)] = &[(359, 0x1580), (359, 0x16d0)];

/// `o6_fam_pairs3::random_particle_setup`: the index and count globals
/// set (two counts), `*G_A` a block whose `+0x198` word is passed on, the
/// particle's words filled (flag bit 14 clear).
#[test]
fn random_particle_setup_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs3 random_particle_setup", RANDOM_PARTICLE_SETUP) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let block = SCRATCH + 0x3000;
        fill(&mut t, block + 0x190, 4, 0x19);
        t.set_u32(w(&t, 0x36), block);
        let obj = SCRATCH + 0x3400;
        for (c, (i, count)) in (0u32..).zip([(2u32, 7u32), (0, 3)]) {
            t.set_u32(w(&t, 0x07), i);
            t.set_u32(w(&t, 0x0f), count);
            t.write(obj, &[0x11; 0x180]);
            t.check_entry(f, regs(rva, c), &[obj], Eax::Checked);
        }
    }
}

const BUTTON_TIMELINES: &[(u16, u32)] = &[(375, 0x2370), (376, 0x2700)];

/// `o6_fam_pairs3::button_timelines`: off; on with no button (slot 261
/// answers 0); on with buttons 0, 2 and 4 (the timelines are the DLL's,
/// with no live events).
#[test]
fn button_timelines_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs3 button_timelines", BUTTON_TIMELINES) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        t.set_u32(w(&t, 0x3f), SCRATCH + 0x900);
        t.set_u32(w(&t, 0x44), 0x0a0a_0044);
        for (c, (on, mask)) in (0u32..).zip([(0u32, 0u32), (1, 0), (1, 0b10101)]) {
            t.set_u32(w(&t, 0x02), on);
            t.stub(261, move |_, a| (mask >> (a[0] & 31)) & 1);
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const PART_MATRIX_HANDOVER: &[(u16, u32)] = &[(307, 0x2550), (605, 0x7ed0)];

/// `o6_fam_pairs3::part_matrix_handover`: slot 772's 64-bit handle,
/// slot 773 answering 0 and 1 (each path's calls), the lock word
/// logged under every call; the scene record a scratch block.
#[test]
fn part_matrix_handover_in_every_copy() {
    for &(n, rva) in PART_MATRIX_HANDOVER {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pairs3 part_matrix_handover");
        stubs(&mut t);
        let f = t.at(rva);
        let lock = SCRATCH + 0x800;
        t.set_u32(t.img.word(f + 0x97), lock);
        t.set_u32(lock, 0x5555);
        let scene = SCRATCH + 0x3000;
        t.set_u32(t.img.word(f + 0x7b), scene - t.img.word(f + 0x81));
        t.stub(772, |_, a| 0x0772_0000 ^ a[0]);
        t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
        for s in [819u32, 870, 871] {
            t.stub(s, move |m, a| {
                let v = m.r32(lock);
                m.w32(SCRATCH + 0x9c0 + 4 * (s % 4), v ^ a[0] ^ a[1]);
                s
            });
        }
        let (part, src) = (SCRATCH + 0x3400, SCRATCH + 0x3500);
        for c in 0..2u32 {
            t.write(scene, &[0x5a; 0x60]);
            t.stub(773, move |m, a| {
                let v = m.r32(lock);
                m.w32(SCRATCH + 0x9d0, v ^ a[0] ^ a[1]);
                c
            });
            let call = Call {
                ecx: part,
                edx: src,
                ..Call::default()
            };
            t.check(f, call, &[], Eax::Checked, |g, img, fr| {
                o6_fam_pairs3::part_matrix_handover(g, img, fr)
            });
        }
    }
}

const NEAR_BONE_MATRIX: &[(u16, u32)] = &[(172, 0x1920), (174, 0x1ad0)];

/// `o6_fam_pairs3::near_bone_matrix`: far (only the flag word cleared;
/// `eax` the cookie word) and at the limit and nearer (the matrices; slot
/// 235 fills the matrix it is handed, slot 277 a float).
#[test]
fn near_bone_matrix_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs3 near_bone_matrix", NEAR_BONE_MATRIX) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        t.stub(235, |m, a| {
            for i in 0..16 {
                m.w32(a[0] + 4 * i, 0x0235_0000 ^ i);
            }
            0x235
        });
        t.stub(277, |_, a| 0x277 ^ a[0]);
        t.stub_st0(277, |_, _| 2.5);
        let f = t.at(rva);
        let obj = SCRATCH + 0x3000;
        for (c, dist) in (0u32..).zip([0x51u32, 0x50, 0x10]) {
            t.write(obj, &[0x5a; 0x140]);
            for i in 0..12 {
                let v = (f32::from(i as u8) - 5.5) * 0.375;
                t.set_u32(obj + 0x80 + 4 * i, v.to_bits());
            }
            t.set_u32(obj + 0x118, dist);
            t.check_entry(f, regs(rva, c), &[obj], Eax::Checked);
        }
    }
}

/// Both copies bind to the one port (the same code, `magic_0249`'s name).
const PULSE_LIGHTS: &[(u16, u32)] = &[(249, 0x1f30), (562, 0x1f40)];

/// `o6_fam_pairs4::pulse_lights`: off; then the six lights fading (above
/// and below the cap), counting down and starting a new pulse (the
/// fade flag `*G_F` 0 and 1); slot 857 reads the colour it is handed.
#[test]
fn pulse_lights_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs4 pulse_lights", PULSE_LIGHTS) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        t.stub(404, |_, a| 0x404 ^ a[0]);
        t.stub_st0(404, |_, _| 3.25);
        t.stub(857, |m, a| {
            let mut x = a[0] ^ a[1];
            for &p in &a[2..6] {
                x = x.rotate_left(5) ^ m.r32(p);
            }
            let k = m.r32(SCRATCH + 0x9a0);
            m.w32(SCRATCH + 0x9a0, k + 1);
            m.w32(SCRATCH + 0x9a4 + 4 * (k % 8), x);
            0x857
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let lights = w(&t, 0x3a);
        let cap = f32::from_bits(t.u32(w(&t, 0x187)));
        let step = f32::from_bits(t.u32(w(&t, 0x180)));
        let saved = t.save();
        for (c, (on, flag)) in (0u32..).zip([(0u32, 0u32), (1, 0), (1, 1)]) {
            t.restore(&saved);
            t.set_u32(w(&t, 0x12), on);
            t.set_u32(w(&t, 0x94), flag);
            for i in 0..6u32 {
                let l = lights + 0x34 * i;
                for (j, v) in (0u32..).zip([0.5f32, -1.5, 2.25, 0.0, 1.0, 0.0]) {
                    t.set_u32(l - 0x18 + 4 * j, v.to_bits());
                }
                let (count, fading, level) = match i {
                    0 => (5u32, 1u32, cap - step + 1.0),
                    1 => (5, 1, cap - step - 1.0),
                    2 => (5, 0, 0.0),
                    4 => (2, 0, 0.0),
                    _ => (1, 0, 0.0),
                };
                t.set_u32(l, count);
                t.set_u32(l + 4, 9);
                t.set_u32(l + 8, fading);
                t.set_u32(l - 8, level.to_bits());
                t.set_u32(l - 4, 4.0f32.to_bits());
            }
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const THREE_PASS_MODEL: &[(u16, u32)] = &[(162, 0x1a80), (586, 0x1a20)];

/// `o6_fam_pairs4::three_pass_model`: slots 235 and 230 fill the
/// matrices they are handed, slot 828 logs the lock word and its pass;
/// the scene a scratch block.
#[test]
fn three_pass_model_in_every_copy() {
    for &(n, rva) in THREE_PASS_MODEL {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pairs4 three_pass_model");
        stubs(&mut t);
        for s in [235u32, 230] {
            t.stub(s, move |m, a| {
                for i in 0..16 {
                    m.w32(a[0] + 4 * i, (s << 12) ^ i);
                }
                s
            });
        }
        let f = t.at(rva);
        let lock = SCRATCH + 0x800;
        t.set_u32(t.img.word(f + 0xff), lock);
        t.set_u32(lock, 0x5555);
        t.stub(828, move |m, a| {
            let v = m.r32(lock);
            m.w32(SCRATCH + 0x9b0 + 4 * (a[8] % 4), v ^ a[0] ^ a[6] ^ a[7]);
            0x828
        });
        let scene = SCRATCH + 0x3000;
        t.set_u32(t.img.word(f + 0x17), scene);
        t.write(scene + 0xa00, &[0x33; 0x20]);
        let call = Call {
            ecx: 0x0a0a_00ec,
            edx: 0x0a0a_00ed,
            ..Call::default()
        };
        t.check(f, call, &[0x0a0a_00a0], Eax::Checked, |g, img, fr| {
            o6_fam_pairs4::three_pass_model(g, img, fr)
        });
    }
}

const BATTLE_EFFECT_END: &[(u16, u32)] = &[(434, 0x95e0), (610, 0xd4b0)];

/// A node at rest for the pool's give-back: its own links, no chain.
fn rest_node(t: &mut Twin, n: u32) {
    t.write(n, &[0x5a; 0x1c]);
    t.set_u32(n, n);
    t.set_u32(n + 4, n);
    t.set_u32(n + 8, n);
    t.set_u32(n + 0xc, 0);
}

/// `o6_fam_pairs4::battle_effect_end`: no nodes and both nodes at rest
/// (given back); busy or not, the actor index below 0x20 or not, the
/// u16 wait word 0 or not; slot 288 writes a list count of 0 (the show
/// and party lists run empty), slot 293 answers by its word.
#[test]
fn battle_effect_end_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs4 battle_effect_end", BATTLE_EFFECT_END) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        t.stub(288, |m, a| {
            m.w32(a[1], 0);
            SCRATCH + 0x4000
        });
        t.stub(293, |_, a| a[0] & 1);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let scene = SCRATCH + 0x3000;
        t.set_u32(w(&t, 0x13), scene);
        let (node_a, node_b) = (SCRATCH + 0x3400, SCRATCH + 0x3500);
        let saved = t.save();
        for (c, (nodes, busy, actor, wait)) in (0u32..).zip([
            (false, 0u32, 3u32, 0u16),
            (true, 0, 0x20, 1),
            (true, 1, 5, 0),
        ]) {
            t.restore(&saved);
            t.write(scene + 0x1a60, &[0x5a; 0x30]);
            if nodes {
                rest_node(&mut t, node_a);
                rest_node(&mut t, node_b);
            }
            t.set_u32(scene + 0x1a64, if nodes { node_a } else { 0 });
            t.set_u32(w(&t, 0x114), if nodes { node_b } else { 0 });
            t.set_u32(w(&t, 0x32), busy);
            t.set_u32(w(&t, 0x43), actor);
            t.set_u16(w(&t, 0x8a), wait);
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let firsts: Vec<(u16, u32)> = START_OBJECT_FIRST.iter().map(|c| (c.0, c.1)).collect();
    let shakes: Vec<(u16, u32)> = SHAKE_VISIT.iter().map(|c| (c.0, c.1)).collect();
    let thunk_dlls: Vec<(u16, u32)> = THUNKS.iter().map(|c| (c.0, c.1)).collect();
    let lists: Vec<&[(u16, u32)]> = vec![
        OBJECT_SETUP,
        POOL_RESET_TAKE,
        HOST_START_PARTS,
        &firsts,
        BONE_NODE_SETUP,
        EFFECT_BOOT,
        NODE_RESET_SOUND,
        RANDOM_LAUNCH,
        SCENE_START,
        RING_PART_START,
        ANGLE_RESET,
        PLACED_PART_START,
        SLOT_PART_START,
        MODELS_DETACH,
        LIST_ANGLE_RESET,
        ACTOR_OBJECT_START,
        &shakes,
        TWO_NODE_START,
        MATRIX_SCALE_SETUP,
        PARTY_COLOURS,
        BONE_TRANSFORM,
        &thunk_dlls,
        RANDOM_PARTICLE_SETUP,
        BUTTON_TIMELINES,
        PART_MATRIX_HANDOVER,
        NEAR_BONE_MATRIX,
        PULSE_LIGHTS,
        THREE_PASS_MODEL,
        BATTLE_EFFECT_END,
    ];
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}
