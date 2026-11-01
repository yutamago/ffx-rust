//! Round 6's shape families (`ffx_magic::o6_fam_*`) against every DLL's
//! own copy, and `DllMain` of each of those DLLs. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::many_single_char_names,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

mod magic;
mod magic_l3;
#[path = "magic/o4.rs"]
mod o4;

use ffx_magic::{
    Ret, Xmm, o6_fam_bone, o6_fam_launch, o6_fam_node, o6_fam_object, o6_fam_pairs2, o6_fam_pairs3,
    o6_fam_pairs4, o6_fam_pairs5, o6_fam_pairs6, o6_fam_pool,
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
    (671, 0x2c60, 0x8e),
    (475, 0x27e0, 0x94),
    (492, 0x1e90, 0x9e),
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
        // the actor word `*G_A` (all three reads), non-zero
        t.set_u32(t.img.word(f + 0x29), 0x0a0a_0029);
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
        t.set_u32(w(&t, 0x4b), 0x0a0a_004b);
        t.set_u32(w(&t, 0x119), 0x0a0a_0119);
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
        t.set_u32(t.img.word(f + 0x8f), 0x0a0a_008f);
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
        // slot 231 reads the frame matrix's word `+0x38` it is handed
        t.stub(231, |m, a| {
            let v = m.r32(a[2] + 0x38);
            m.w32(SCRATCH + 0x9f0, v ^ a[0]);
            0x231
        });
        let f = t.at(rva);
        let obj = SCRATCH + 0x3000;
        t.set_u32(t.img.word(f + 0x71), 0x0a0a_0071);
        for (c, dist) in (0u32..).zip([0x51u32, 0x50, 0x10]) {
            t.write(obj, &[0x5a; 0x140]);
            fill(&mut t, obj + 0xb0, 4, 0xb0);
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
        t.set_u32(w(&t, 0x18), 0x0a0a_0018);
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
                    2 => (5, 0, 0.75),
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
            m.w32(a[1], 2);
            SCRATCH + 0x4000
        });
        t.write(SCRATCH + 0x4000, &[3, 5]);
        t.stub(293, |_, a| a[0] & 1);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        for (o, v) in [(0x3a, 0x0a0a_003a), (0x9b, 0x0a0a_009b)] {
            t.set_u32(w(&t, o), v);
        }
        t.set_u32(w(&t, 0x8a) + 2, 0x7777);
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

const CAMERA_RESET_NODES: &[(u16, u32)] = &[(307, 0x6bc0), (605, 0xa100)];

/// `o6_fam_pairs4::camera_reset_nodes`: the angle above, below and
/// between its limits; a free ring of three and of two nodes; the list
/// empty or holding a node.
#[test]
fn camera_reset_nodes_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs4 camera_reset_nodes", CAMERA_RESET_NODES) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let kf = |t: &Twin, o: u32| f32::from_bits(t.u32(t.img.word(f + o)));
        let block = 0x6a00_0000 + 0x0010_0000 * u32::from(n % 16);
        t.set_u32(w(&t, 0x05), block);
        let (k1, hi, lo) = (kf(&t, 0x1f), kf(&t, 0x26), kf(&t, 0x3a));
        let busy = SCRATCH + 0x3400;
        let saved = t.save();
        for (c, (x, ring, list)) in (0u32..).zip([
            (hi + 1.0, 3u32, false),
            (lo - 1.0, 2, true),
            (hi.midpoint(lo), 3, true),
            (hi, 3, true),
            (lo, 2, false),
        ]) {
            t.restore(&saved);
            t.write(block + 0x16d0, &[0x5a; 0x800]);
            t.write(block + 0x1e30, &[0x5a; 0x40]);
            // the node word `+0x2864` (its page is compared only when the
            // set-up touches it)
            t.write(block + 0x2860, &[0x5a; 0x10]);
            t.set_u32(block + 0xc24, (x + k1).to_bits());
            let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
            for i in 1..=ring {
                t.write(node(i), &[0x5a; 0x1c]);
                t.set_u32(node(i), node(if i == 1 { ring } else { i - 1 }));
                t.set_u32(node(i) + 4, node(if i == ring { 1 } else { i + 1 }));
            }
            t.set_u32(w(&t, 0x163), node(1));
            t.set_u16(w(&t, 0x148), ring as u16);
            t.write(busy, &[0x33; 0x20]);
            t.set_u32(w(&t, 0x1c3), if list { busy } else { 0 });
            t.set_u32(w(&t, 0x1d2), busy);
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

/// Both copies bind to the one port (the same code, `magic_0249`'s name).
const SPARK_TASK: &[(u16, u32)] = &[(249, 0x1b20), (562, 0x1b30)];

/// `o6_fam_pairs4::spark_task`: delayed, at the end of its life (the
/// task, at rest, goes back to the pool) and alive (four sparks from the
/// DLL's ring as it is after the start-up; slot 314 answers floats, slot
/// 237 reads its vector).
#[test]
fn spark_task_in_every_copy() {
    for (n, rva) in copies("o6 fam pairs4 spark_task", SPARK_TASK) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        t.stub(314, |_, a| 0x314 ^ a[0]);
        t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.5);
        t.stub(237, |m, a| {
            let mut x = 0u32;
            for i in 0..4 {
                x = x.rotate_left(3) ^ m.r32(a[2] + 4 * i);
            }
            m.w32(a[3], x ^ a[0]);
            0x237
        });
        let f = t.at(rva);
        let task = SCRATCH + 0x3000;
        // the spark ring: 8 free records of 0x50 bytes in scratch memory,
        // the rover at 6 (it wraps)
        let take = t.img.rel32_target(f + 0x4a);
        let ring = SCRATCH + 0x5000;
        t.set_u32(t.img.word(f + 0x9b), 0x0a0a_009b);
        t.set_u32(t.img.word(take + 0x09), 8);
        t.set_u32(t.img.word(take + 0x16), ring);
        let saved = t.save();
        for (c, (delay, life)) in (0u32..).zip([(3u16, 5u16), (0, 1), (0, 4), (0, 4)]) {
            t.restore(&saved);
            t.write(ring, &[0x5a; 0x280]);
            // the last case: a full ring (no spark at all)
            for i in 0..8 {
                t.set_u32(ring + 0x50 * i + 0x48, u32::from(i == 7 || c == 3));
            }
            t.set_u32(t.img.word(take + 0x01), 6);
            rest_node(&mut t, task);
            t.write(task + 0x1c, &[0x44; 0x10]);
            t.set_u16(task + 0x14, delay);
            t.set_u16(task + 0x16, life);
            let r = t.check_entry(f, regs(rva, c), &[task], Eax::Checked);
            // the alive case reaches the sparks (the ring has room)
            assert!(c != 2 || r.eax != 0, "{n}: no spark taken");
        }
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let firsts: Vec<(u16, u32)> = START_OBJECT_FIRST.iter().map(|c| (c.0, c.1)).collect();
    let shakes: Vec<(u16, u32)> = SHAKE_VISIT.iter().map(|c| (c.0, c.1)).collect();
    let thunk_dlls: Vec<(u16, u32)> = THUNKS.iter().map(|c| (c.0, c.1)).collect();
    let ribbons: Vec<(u16, u32)> = RIBBON_VISIT.iter().map(|c| (c.0, c.1)).collect();
    let timed: Vec<(u16, u32)> = TIMED_ITEMS_VISIT.iter().map(|c| (c.0, c.1)).collect();
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
        CAMERA_RESET_NODES,
        SPARK_TASK,
        LOOK_MATRIX_CALLBACK,
        RECORD_NODE_START,
        GLOBAL_NODE_START,
        RECORD_UNLIST,
        TWO_PARTICLE_BURST,
        LEVEL_PARTICLES,
        TWO_PART_VISIT,
        SCROLLED_TEXTURE,
        &ribbons,
        &timed,
        SPRITE_VISIT,
        LIGHT_SETUP,
        TURN_BOX,
        PARTICLE_ROWS,
        NODE_RINGS,
        TWO_PASS_PARTICLES,
        MESH_CENTRES,
        TARGET_OBJECTS,
        SPRITE_FRAME,
        MOVING_SPRITES,
        ACTOR_LIGHT,
        AIMED_OBJECTS,
        CURVE_FLYERS,
        ACTOR_SPAWNER,
        STAGED_NODES,
        TRAIL_DRAW,
        TRAIL_TASK,
        PACKED_SPRITE,
        BASIS_START,
        OVERLAY_SETUP,
        SCENE_END,
    ];
    for n in dlls(&lists) {
        magic_l3::dll_main(n);
    }
}

// ---- families2 (o6_fam_pairs5..) ----

/// The argument words of the game's functions behind the slots these
/// tests reach (1 for the others).
fn slot_words(s: u32) -> usize {
    match s {
        714 => 5,
        231 | 233 | 234 | 236 | 266 => 3,
        217 | 229 | 230 | 232 | 325 | 712 | 941 => 2,
        307 | 545 | 562 | 790 => 0,
        _ => 1,
    }
}

/// Every slot records its argument words ([`slot_words`], words under
/// them are the caller's leftovers) in a ring at `SCRATCH + 0xb00` (so a
/// dropped or misplaced argument shows) and answers a value made of them
/// and its slot.
fn stubs4(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |m, a| {
            let k = m.r32(SCRATCH + 0xb00);
            let mut x = s << 20;
            for (i, w) in (0u32..).zip(a.iter().take(slot_words(s))) {
                m.w32(SCRATCH + 0xb10 + 0x18 * (k % 0x20) + 4 * i, *w ^ s);
                x = x.rotate_left(7) ^ *w;
            }
            m.w32(SCRATCH + 0xb00, k + 1);
            x
        });
    }
}

const LOOK_MATRIX_CALLBACK: &[(u16, u32)] = &[(664, 0x16e0), (45, 0x1310)];

/// `o6_fam_pairs5::look_matrix_callback`: the object outside and inside
/// the 0x80 bytes from `BASE` (the two sources), slot 712 answering 0
/// (the part is not restarted) and non-zero; slot 941's list a scratch
/// block of distinct words.
#[test]
fn fam2_look_matrix_callback() {
    for (n, rva) in copies("o6 fam pairs5 look_matrix_callback", LOOK_MATRIX_CALLBACK) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let base = w(&t, 0x2a);
        t.set_u32(w(&t, 0x1a), 0x0051_0000);
        t.set_u32(w(&t, 0x3a), 0x0052_0000);
        for (k, o) in [0x4fu32, 0x68, 0x74, 0x87].into_iter().enumerate() {
            t.set_u32(w(&t, o), 0x00a0_0000 + 0x100 * k as u32);
        }
        t.set_u32(w(&t, 0x94), SCRATCH + 0x3000);
        t.set_u32(w(&t, 0xa1), SCRATCH + 0x3010);
        t.set_u32(SCRATCH + 0x3010, 0x55);
        t.set_u32(w(&t, 0xf0), SCRATCH + 0x3020);
        fill(&mut t, SCRATCH + 0x3020, 0x10, 3);
        fill(&mut t, P, 0x10, 9);
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x3080, a[0] ^ a[1].rotate_left(3));
            P
        });
        let saved = t.save();
        for c in 0..4u32 {
            t.restore(&saved);
            let r = if c == 0 { 0 } else { 0x712 };
            // 712 sees the lock word and the item; 712 and 713 change the
            // item (the game's own code does)
            t.stub(712, move |m, a| {
                let seen = m.r32(SCRATCH + 0x3010) ^ m.r32(SCRATCH + 0x3000).rotate_left(9);
                m.w32(SCRATCH + 0x3084, a[0] ^ a[1].rotate_left(5) ^ a[2] ^ seen);
                m.w32(SCRATCH + 0x3000, 0x0712_0000);
                r
            });
            t.stub(713, |m, a| {
                m.w32(SCRATCH + 0x3088, a[0] ^ m.r32(SCRATCH + 0x3000));
                m.w32(SCRATCH + 0x3000, 0x0713_0000);
                0x713
            });
            // the look matrix's last call: the rotation and the scale
            t.stub(231, |m, a| {
                let mut x = 0u32;
                for p in [a[1], a[2]] {
                    for i in 0..16 {
                        x = x.rotate_left(3) ^ m.r32(p + 4 * i);
                    }
                }
                m.w32(SCRATCH + 0x308c, x);
                0x231
            });
            let obj = if c & 1 == 1 { base + 0x10 } else { OBJ };
            fill(&mut t, obj, 0x20, c + 5);
            t.set_u32(obj + 0x60, (1.5f32 + c as f32).to_bits());
            t.check_entry(f, regs(rva, c), &[obj], Eax::Checked);
        }
    }
}

const RECORD_NODE_START: &[(u16, u32)] = &[(199, 0x2160), (251, 0x2550)];
const GLOBAL_NODE_START: &[(u16, u32)] = &[(253, 0x4370), (311, 0x1f60)];

/// A node start kind: its copies, its port name, the offsets of `G_A`,
/// `G_HEAD`, `G_FREE`, `G_LAST` (compared) and `G_LAST` (read).
type NodeKind = (&'static [(u16, u32)], &'static str, [u32; 5]);

/// `o6_fam_pairs5::record_node_start` (the record in `ecx`) and
/// `global_node_start` (the record at a fixed address): slots 217 and 234
/// fill the vectors they are handed; a free ring of three nodes with the
/// count 3 or 1 (the head moves on or becomes 0); the busy list empty or
/// holding a node.
#[test]
fn fam2_record_node_start() {
    // (copies, name, G_A, G_HEAD, G_FREE, G_LAST compared, G_LAST read)
    let kinds: [NodeKind; 2] = [
        (
            RECORD_NODE_START,
            "o6 fam pairs5 record_node_start",
            [0x18, 0x1bf, 0x14d, 0x226, 0x23f],
        ),
        (
            GLOBAL_NODE_START,
            "o6 fam pairs5 global_node_start",
            [0x17, 0x1fb, 0x1b2, 0x262, 0x27e],
        ),
    ];
    for (list, name, ops) in kinds {
        for &(n, rva) in list {
            let Some(mut t) = magic_l3::twin(n) else {
                return;
            };
            registered(&t, rva, name);
            stubs4(&mut t);
            for s in [217u32, 234] {
                t.stub(s, move |m, a| {
                    let (dst, src) = if s == 217 {
                        (a[1], a[0])
                    } else {
                        (a[0], a[1] ^ a[2])
                    };
                    for i in 0..4 {
                        m.w32(dst + 4 * i, src.rotate_left(i + s) | 1);
                    }
                    s
                });
            }
            let f = t.at(rva);
            let w = |t: &Twin, o: u32| t.img.word(f + o);
            let global = name.ends_with("global_node_start");
            let rec = if global { w(&t, 0x26f) } else { OBJ };
            t.set_u32(w(&t, ops[0]), 0x00a1_0000);
            let saved = t.save();
            for c in 0..4u32 {
                t.restore(&saved);
                t.write(rec, &[0x5a; 0xc0]);
                fill(&mut t, P, 4, c + 1);
                let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
                for i in 0..3 {
                    t.write(node(i), &[0x77; 0x20]);
                    t.set_u32(node(i), node((i + 2) % 3));
                    t.set_u32(node(i) + 4, node((i + 1) % 3));
                }
                t.set_u32(w(&t, ops[1]), node(1));
                t.set_u16(w(&t, ops[2]), if c & 1 == 0 { 3 } else { 1 });
                let busy = SCRATCH + 0x1400;
                t.write(busy, &[0x66; 0x20]);
                t.set_u32(w(&t, ops[3]), if c & 2 == 0 { 0 } else { busy });
                t.set_u32(w(&t, ops[4]), busy);
                let args = [P, 2.5f32.to_bits(), (-0.75f32).to_bits()];
                let call = Call::this(if global { 0x00ec_0000 } else { OBJ }).with_xmm(&[
                    Xmm::f32(9.0),
                    Xmm::f32(8.0),
                    Xmm::f32(7.0),
                    Xmm::f32(0.125 + c as f32),
                ]);
                t.check(f, call, &args, Eax::Checked, |g, img, fr| {
                    if global {
                        o6_fam_pairs5::global_node_start(g, img, fr)
                    } else {
                        o6_fam_pairs5::record_node_start(g, img, fr)
                    }
                });
            }
        }
    }
}

const RECORD_UNLIST: &[(u16, u32)] = &[(479, 0x3c00), (673, 0x15b0)];

/// `o6_fam_pairs5::record_unlist`: a list of three nodes holding the
/// record at the head, in the middle, at the tail, or not at all.
#[test]
fn fam2_record_unlist() {
    for (n, rva) in copies("o6 fam pairs5 record_unlist", RECORD_UNLIST) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let imm = t.img.word(f + 0x10) as u8 == 0x81;
        let base = if imm {
            w(&t, 0x12)
        } else {
            t.set_u32(w(&t, 0x12), SCRATCH + 0x4000);
            SCRATCH + 0x4000
        };
        let list = if imm { w(&t, 0x19) } else { w(&t, 0x17) };
        t.set_u32(w(&t, 0x27), SCRATCH + 0x3000);
        let idx_at = OBJ + u32::from(t.img.word(f + 9) as u8);
        let saved = t.save();
        for c in 0..4u32 {
            t.restore(&saved);
            fill(&mut t, OBJ, 0x10, c);
            t.set_u32(idx_at, c + 1);
            let rec = base + 0x130 * (c + 1);
            t.set_u32(rec + 0x124, 0xfff1_8123 ^ c);
            let node = |i: u32| SCRATCH + 0x1800 + 0x20 * i;
            for i in 0..3u32 {
                t.set_u32(node(i), if i == 0 { 0 } else { node(i - 1) });
                t.set_u32(node(i) + 4, if i == 2 { 0 } else { node(i + 1) });
                let item = if i == c { rec } else { 0x7700 + i };
                t.set_u32(node(i) + 8, item);
            }
            t.set_u32(list, node(0));
            t.set_u32(list + 4, node(2));
            t.set_u32(list + 8, node(c % 3));
            t.set_u32(list + 0xc, 3);
            t.check_entry(f, regs(rva, c), &[OBJ], Eax::Checked);
        }
    }
}

const TWO_PARTICLE_BURST: &[(u16, u32)] = &[(162, 0x48a0), (598, 0x7ef0)];

/// `o6_fam_pairs5::two_particle_burst`: the counter 0 (no particles), 1
/// with both records free, with the second taken (the pool gives 0) and
/// with none free. The pool starts at a negative index so the search
/// never wraps (no program-cache refill); slot 237 fills the colour it
/// is handed, slots 314 and 404 return floats made of their argument.
#[test]
fn fam2_two_particle_burst() {
    for (n, rva) in copies("o6 fam pairs5 two_particle_burst", TWO_PARTICLE_BURST) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        t.stub(237, |m, a| {
            for i in 0..4 {
                let x = 1.25f32 + ((a[0] ^ a[1] ^ a[2]) & 0xff) as f32 + i as f32;
                m.w32(a[3] + 4 * i, x.to_bits());
            }
            0x237
        });
        t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.375 + 0.1);
        t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) - 0.2);
        // slot 229 reads the vector it is handed
        t.stub(229, |m, a| {
            let mut x = a[0] ^ a[1];
            for i in 0..4 {
                x = x.rotate_left(7) ^ m.r32(a[2] + 4 * i);
            }
            m.w32(SCRATCH + 0x3090, x);
            0x229
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let obj = 0x6100_0000;
        t.set_u32(w(&t, 0x14), obj);
        t.write(obj, &vec![0x11; 0x8700]);
        t.set_u32(obj + w(&t, 0x1f), 0.5f32.to_bits());
        let count_at = obj + w(&t, 0xcc);
        t.set_u32(w(&t, 0xe3), SCRATCH + 0x3000);
        t.set_u32(SCRATCH + 0x300c, 0x0077_1234);
        t.set_u32(w(&t, 0x12c), SCRATCH + 0x3100);
        t.write(SCRATCH + 0x3100, &[3; 4]);
        let table = w(&t, 0x13d);
        fill(&mut t, table, 8, 77);
        let take = t.img.rel32_target(f + 0xf1);
        let tw = |t: &Twin, o: u32| t.img.word(take + o);
        let pool = 0x6200_0000;
        t.set_u32(tw(&t, 0x37), pool + 0x74 * 5);
        let saved = t.save();
        for c in 0..4u32 {
            t.restore(&saved);
            t.set_u32(count_at, u32::from(c != 0));
            t.set_u32(tw(&t, 0x08), (-5i32).cast_unsigned());
            t.set_u32(tw(&t, 0x0f), 1);
            t.write(pool - 0x100, &[0x33; 0x100 + 0x74 * 2]);
            t.set_u32(pool + 0x6c, u32::from(c == 3));
            t.set_u32(pool + 0x74 + 0x6c, u32::from(c >= 2));
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const LEVEL_PARTICLES: &[(u16, u32)] = &[(307, 0x5310), (563, 0x35b0)];

/// `o6_fam_pairs5::level_particles`: the delay counting down (also from
/// 0x8000, below 0); the life running out (the node at rest goes back to
/// the pool); then the particles: slot 237 fills the vector it is handed
/// with a `y` far above or below the level (per vector, from a mask at
/// `SCRATCH + 0x3200`), slot 12 and 314 return floats; the pool (as in
/// `two_particle_burst_in_every_copy`) with both records free, the second
/// taken, or none.
#[test]
fn fam2_level_particles() {
    for (n, rva) in copies("o6 fam pairs5 level_particles", LEVEL_PARTICLES) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        t.stub(237, |m, a| {
            let k = m.r32(SCRATCH + 0x3204);
            m.w32(SCRATCH + 0x3204, k + 1);
            let mode = m.r32(SCRATCH + 0x3200);
            let above = mode >> k & 1 == 1;
            // mode bit 2: the first vector's `y` is the level itself
            let y = if mode & 4 != 0 && k == 0 {
                f32::from_bits(m.r32(SCRATCH + 0x3208))
            } else if above {
                1.0e6f32
            } else {
                -1.0e6
            };
            let base = ((a[0] ^ a[1] ^ a[2]) & 0xff) as f32;
            for (i, x) in [base + 0.5, y, base - 0.25, 7.0].into_iter().enumerate() {
                m.w32(a[3] + 4 * i as u32, x.to_bits());
            }
            0x237
        });
        t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.5 + 0.125);
        t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.375 + 0.1);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        t.set_u32(w(&t, 0x64), 0x0000_1234);
        // the level: slot 12's 26.125 (0x34 * 0.5 + 0.125) minus K1
        let k1 = f32::from_bits(t.u32(w(&t, 0xfc)));
        t.set_u32(SCRATCH + 0x3208, (26.125f32 - k1).to_bits());
        t.set_u32(w(&t, 0x12d), SCRATCH + 0x3000);
        fill(&mut t, SCRATCH + 0x3000, 0x10, 21);
        let take = t.img.rel32_target(f + 0x11d);
        let tw = |t: &Twin, o: u32| t.img.word(take + o);
        let pool = 0x6200_0000;
        t.set_u32(tw(&t, 0x37), pool + 0x74 * 5);
        let node = SCRATCH + 0x3400;
        let saved = t.save();
        // (delay, life, the vectors above the level, records taken)
        let cases = [
            (3u16, 0u16, 0u32, 0u32),
            (0x8000, 4, 0, 0),
            (0, 1, 0, 0),
            (0, 5, 0b00, 0),
            (0, 5, 0b01, 0),
            (0, 5, 0b10, 0b10),
            (0, 5, 0b11, 0b10),
            (0, 5, 0b11, 0b01),
            (0, 5, 0b111, 0),
        ];
        for (c, (delay, life, above, taken)) in (0u32..).zip(cases) {
            t.restore(&saved);
            rest_node(&mut t, node);
            t.set_u16(node + 0x14, delay);
            t.set_u16(node + 0x16, life);
            t.set_u32(SCRATCH + 0x3200, above);
            t.set_u32(SCRATCH + 0x3204, 0);
            t.set_u32(tw(&t, 0x08), (-5i32).cast_unsigned());
            t.set_u32(tw(&t, 0x0f), 1);
            t.write(pool - 0x100, &[0x33; 0x100 + 0x74 * 2]);
            t.set_u32(pool + 0x6c, taken & 1);
            t.set_u32(pool + 0x74 + 0x6c, taken >> 1);
            t.check_entry(f, regs(rva, c), &[node], Eax::Checked);
        }
    }
}

const TWO_PART_VISIT: &[(u16, u32)] = &[(70, 0x1160), (577, 0x1000)];

/// A two-part node case: marked already, its children, the flags `+0x68`,
/// `+0x28`, the speed `+0x5c`, the step `+0x50` and fade `+0x58` (1.0 is
/// the end of both).
#[derive(Clone, Copy)]
struct TwoPart {
    marked: bool,
    children: usize,
    flags: u8,
    x28: f32,
    speed: f32,
    step: f32,
    fade: f32,
}

/// Lays out case `k` (`c` the cursor, `node`, two children, the parts
/// `pa`/`pb` whose class's visit is the trap `class`).
fn two_part_setup(t: &mut Twin, k: u32, p: TwoPart, class: u32) {
    let (c, node) = (SCRATCH + 0x3000, SCRATCH + 0x3100);
    let kids = [SCRATCH + 0x3200, SCRATCH + 0x3240];
    let (pa, pb) = (SCRATCH + 0x3300, SCRATCH + 0x3400);
    t.write(node, &[0x44; 0x80]);
    for i in 0..19u32 {
        let x = 0.25 * (k + i) as f32 - 1.5;
        t.set_u32(node + 0x1c + 4 * i, x.to_bits());
    }
    t.set_u32(node + 0x4c, 0.375f32.to_bits());
    t.set_u32(node + 0x54, 0.125f32.to_bits());
    t.set_u32(node + 0x50, p.step.to_bits());
    t.set_u32(node + 0x58, p.fade.to_bits());
    t.set_u32(node + 0x28, p.x28.to_bits());
    t.set_u32(node + 0x5c, p.speed.to_bits());
    t.set_u8(node + 0x68, p.flags);
    t.set_u8(node + 0x1a, 0xf0);
    t.set_u32(c, node);
    t.set_u8(c + 4, 0x55);
    t.set_u8(c + 5, if p.marked { 0xf0 } else { 3 + k as u8 });
    let mut prev = node;
    for &kid in kids.iter().take(p.children).rev() {
        t.write(kid, &[0x22; 0x10]);
        t.set_u32(kid, class);
        t.set_u32(kid + 8, prev);
        prev = kid;
    }
    t.set_u32(node + 0x10, prev);
    for part in [pa, pb] {
        t.write(part, &[0x11; 0x40]);
        t.set_u32(part, class);
        for i in 0..3u32 {
            t.set_u32(part + 0x1c + 4 * i, (0.5 * (i + k) as f32).to_bits());
        }
    }
    t.set_u32(node + 0x6c, pa);
    t.set_u32(node + 0x70, pb);
}

/// `o6_fam_pairs5::two_part_visit` (and its thunk in `magic_0070`): the
/// node already marked (`eax` junk there: the /GS cookie's bits); no
/// children, one or two; the flags 0..3 with `+0x28` above or below 0 and
/// the speed `+0x5c` above or below 0 (the slot block); the step and fade
/// before, across and at their end. The visits are a slot trap that logs
/// the cursor's words.
#[test]
fn fam2_two_part_visit() {
    for &(n, rva) in TWO_PART_VISIT {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pairs5 two_part_visit");
        stubs4(&mut t);
        let visit = 1000u32;
        t.stub(visit, move |m, a| {
            let k = m.r32(SCRATCH + 0x3800);
            m.w32(SCRATCH + 0x3800, k + 1);
            let cur = m.r32(a[0]);
            m.w32(SCRATCH + 0x3810 + 8 * (k % 8), cur);
            m.w32(SCRATCH + 0x3814 + 8 * (k % 8), m.r32(a[0] + 4));
            0x1000 + k
        });
        let class = SCRATCH + 0x3700;
        t.set_u32(class + 0x1c, magic::TRAPS + 16 * visit);
        let f = t.at(rva);
        let c = SCRATCH + 0x3000;
        let saved = t.save();
        let case = |marked, children, flags, x28, speed, step, fade| TwoPart {
            marked,
            children,
            flags,
            x28,
            speed,
            step,
            fade,
        };
        let cases = [
            case(true, 0, 0, 1.0, 1.0, 0.0, 0.0),
            case(false, 0, 0, 1.0, 1.0, 0.25, 0.5),
            case(false, 2, 1, 1.0, 2.0, 0.25, 0.5),
            case(false, 2, 3, 1.0, 2.0, 0.25, 0.5),
            case(false, 1, 1, 1.0, -2.0, 1.0, 1.0),
            case(false, 0, 1, -1.0, 2.0, 0.75, 0.5),
            case(false, 0, 2, 1.0, 2.0, 0.75, 0.5),
            case(false, 0, 0, 1.0, 2.0, 0.75, 0.95),
            case(false, 0, 0, 1.0, 2.0, 1.5, 0.25),
            case(false, 0, 1, 0.0, 2.0, 0.25, 0.5),
            case(false, 0, 1, 1.0, 0.0, 0.25, 0.5),
        ];
        for (k, p) in (0u32..).zip(cases) {
            let eax = if p.marked { Eax::Junk } else { Eax::Checked };
            t.restore(&saved);
            two_part_setup(&mut t, k, p, class);
            t.check(f, Call::this(c), &[], eax, |g, img, fr| {
                o6_fam_pairs5::two_part_visit(g, img, fr)
            });
            if n == 70 {
                t.restore(&saved);
                two_part_setup(&mut t, k, p, class);
                t.check_entry(t.at(0x14a0), regs(0x14a0, k), &[c], eax);
            }
        }
    }
}

const SCROLLED_TEXTURE: &[(u16, u32)] = &[(188, 0x1220), (351, 0x1080)];

/// `o6_fam_pairs5::scrolled_texture`: slot 719 hands out a `TEX0` word
/// (sizes 1..2^15, every field set), slot 885 answers 0 (nothing drawn)
/// or not; the counters positive and negative (the remainders' signs),
/// the lock word, the effect word `*G_F + 0x830`. Slot 879 logs the
/// floats it is handed, 235 and 885 read their matrices.
#[test]
fn fam2_scrolled_texture() {
    for (n, rva) in copies("o6 fam pairs5 scrolled_texture", SCROLLED_TEXTURE) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        let tex = SCRATCH + 0x3000;
        t.stub(719, move |_, a| tex ^ (a[1] & a[2] & 0x10));
        t.stub(879, |m, a| {
            let mut x = 0u32;
            for &v in &a[..10] {
                x = x.rotate_left(5) ^ v;
            }
            for i in 0..4 {
                x = x.rotate_left(3) ^ m.r32(a[4] + 4 * i);
            }
            // the matrix M1 and the lock word while it draws
            for i in 0..12 {
                x = x.rotate_left(3) ^ m.r32(a[6] + 4 * i);
            }
            x ^= m.r32(SCRATCH + 0x3040).rotate_left(11);
            m.w32(SCRATCH + 0x3080, x);
            0x879
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        t.set_u32(w(&t, 0x3b), 0x0071_9000);
        t.set_u32(w(&t, 0x10f), SCRATCH + 0x3040);
        t.set_u32(w(&t, 0xb9), SCRATCH + 0x3100);
        let saved = t.save();
        // (TEX0 low and high words, 885's answer, count, C1, D, E)
        let cases = [
            (0x4c0c_1234u32, 0x1u32, 0u32, 100u32, 50u32, 3u32, 4u32),
            (0x4c0c_1234, 0x1, 1, 0x0012_3456, 0x0009_8765, 3, 4),
            (
                0xffff_ffff,
                0x3,
                2,
                0xfff0_0001,
                0x8000_0001,
                0xffff_fff0,
                1,
            ),
            (0x0000_0000, 0x2, 3, 0x7fff_ffff, 0xfedc_ba98, 7, 0),
            (0x8423_c00f, 0x0, 4, 0x8000_0000, 17, 0, 0),
        ];
        for (c, (lo, hi, made, count, c1, d, e)) in (0u32..).zip(cases) {
            t.restore(&saved);
            t.set_u32(tex, lo);
            t.set_u32(tex + 4, hi);
            t.stub(885, move |m, a| {
                let mut x = 0u32;
                for &v in &a[..7] {
                    x = x.rotate_left(7) ^ v;
                }
                m.w32(
                    SCRATCH + 0x3084,
                    x ^ m.r32(a[6]) ^ m.r32(SCRATCH + 0x3040).rotate_left(9),
                );
                made
            });
            t.set_u32(SCRATCH + 0x3040, 0x55 + c);
            // the scale K (a constant of the DLL) per case: the tint differs
            t.set_u32(w(&t, 0x33), (1.0f32 + 0.5 * c as f32).to_bits());
            t.set_u32(SCRATCH + 0x3100 + w(&t, 0xca), 0x0830_0000 + c);
            t.set_u32(w(&t, 0x82), count);
            t.set_u32(w(&t, 0x9b), c1);
            t.set_u32(w(&t, 0xa4), d);
            t.set_u32(w(&t, 0xaa), e);
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const RIBBON_VISIT: &[(u16, u32, u32)] = &[(70, 0x86d0, 0x8840), (155, 0x8170, 0x8340)];

/// `o6_fam_pairs5::ribbon_visit` (and its thunks): the node already
/// marked (`eax` junk there: the /GS cookie's bits); no children or two;
/// bit 0 of `+0x54` set or clear; `magic_0155`'s skip word 0 or not.
/// Slots 677, 679 and 680 log what their pointers hold.
#[test]
fn fam2_ribbon_visit() {
    for &(n, rva, thunk) in RIBBON_VISIT {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pairs5 ribbon_visit");
        copies("o6 fam pairs5 ribbon_visit_thunk", &[(n, thunk)]);
        stubs4(&mut t);
        let visit = 1000u32;
        t.stub(visit, move |m, a| {
            let k = m.r32(SCRATCH + 0x3800);
            m.w32(SCRATCH + 0x3800, k + 1);
            m.w32(SCRATCH + 0x3810 + 4 * (k % 8), m.r32(a[0]));
            0x1000 + k
        });
        for s in [677u32, 679, 680] {
            t.stub(s, move |m, a| {
                let mut x = s;
                // (the pointer word, the words it points at: V, P, X)
                let (i, n) = match s {
                    677 => (3, 4),
                    679 => (0, 6),
                    _ => (2, 2),
                };
                for j in 0..n {
                    x = x.rotate_left(5) ^ m.r32(a[i] + 4 * j);
                }
                let words = match s {
                    680 => 11,
                    679 => 1,
                    _ => 4,
                };
                for &v in a.iter().take(words) {
                    x = x.rotate_left(3) ^ v;
                }
                m.w32(SCRATCH + 0x3880 + 4 * (s - 677), x);
                x
            });
        }
        let class = SCRATCH + 0x3700;
        t.set_u32(class + 0x1c, magic::TRAPS + 16 * visit);
        let f = t.at(rva);
        let (c, node) = (SCRATCH + 0x3000, SCRATCH + 0x3100);
        let kids = [SCRATCH + 0x3200, SCRATCH + 0x3240];
        let saved = t.save();
        // (marked, children, flags, skip)
        let cases = [
            (true, 0usize, 0u8, 0u32),
            (false, 0, 0, 0),
            (false, 2, 1, 0),
            (false, 1, 0xff, 1),
            (false, 0, 0xfe, 0),
        ];
        for (k, (marked, children, flags, skip)) in (0u32..).zip(cases) {
            let setup = |t: &mut Twin| {
                t.restore(&saved);
                t.write(node, &[0x44; 0x60]);
                for i in 0..0x10u32 {
                    t.set_u32(node + 0x1c + 4 * i, 0x0100_0000 * (i + 1) + k);
                }
                t.set_u8(node + 0x54, flags);
                t.set_u8(node + 0x1a, 0xf0);
                t.set_u32(c, node);
                t.set_u8(c + 4, 0x55);
                t.set_u8(c + 5, if marked { 0xf0 } else { 3 + k as u8 });
                let mut prev = node;
                for &kid in kids.iter().take(children).rev() {
                    t.write(kid, &[0x22; 0x10]);
                    t.set_u32(kid, class);
                    t.set_u32(kid + 8, prev);
                    prev = kid;
                }
                t.set_u32(node + 0x10, prev);
                if n == 155 {
                    let at = t.img.word(f + 0x113);
                    t.set_u32(at, skip);
                }
            };
            let eax = if marked { Eax::Junk } else { Eax::Checked };
            setup(&mut t);
            t.check(f, Call::this(c), &[], eax, |g, img, fr| {
                o6_fam_pairs5::ribbon_visit(g, img, fr)
            });
            setup(&mut t);
            t.check_entry(t.at(thunk), regs(thunk, k), &[c], eax);
        }
    }
}

const TIMED_ITEMS_VISIT: &[(u16, u32, u32)] = &[(155, 0x7d70, 0x8050), (164, 0x7df0, 0x80d0)];

/// `o6_fam_pairs5::timed_items_visit` (and its thunks): the node marked
/// already (`eax` junk); the flags 0, 2, 4, 6 with no children or two;
/// six items: one skipped (bit 0 of `+0x26e`), the rest reaching each way
/// of the time step (below 0; below the target; at the end; stepping
/// with and without reaching 1, the time left with bit 0 of `+0x90` set
/// or clear); slot 534 0 or not, slot 390 0x1079 or not (`0155`). The
/// float slots answer in ST0, slot 262 fills what it is handed.
#[test]
fn fam2_timed_items_visit() {
    for &(n, rva, thunk) in TIMED_ITEMS_VISIT {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pairs5 timed_items_visit");
        copies("o6 fam pairs5 timed_items_visit_thunk", &[(n, thunk)]);
        stubs4(&mut t);
        let visit = 1000u32;
        t.stub(visit, move |m, a| {
            let k = m.r32(SCRATCH + 0x3800);
            m.w32(SCRATCH + 0x3800, k + 1);
            m.w32(SCRATCH + 0x3810 + 4 * (k % 8), m.r32(a[0]));
            0x1000 + k
        });
        t.stub(262, |m, a| {
            for (i, &p) in a[1..4].iter().enumerate() {
                m.w32(p, (0.125 * (i as f32 + 1.0)).to_bits());
            }
            a[0] ^ 0x262
        });
        t.stub_st0(12, |_, _| 2.0);
        t.stub_st0(216, |_, a| f64::from(a[0] & 0xf) * 0.25);
        t.stub_st0(664, |_, _| 0.625);
        t.stub_st0(333, |_, _| 0.375);
        t.stub(534, |m, a| m.r32(SCRATCH + 0x3900) ^ (a[0] & 0x100));
        t.stub(390, |m, _| m.r32(SCRATCH + 0x3904));
        let class = SCRATCH + 0x3700;
        t.set_u32(class + 0x1c, magic::TRAPS + 16 * visit);
        let f = t.at(rva);
        let second = n == 164;
        let (flags_at, count_at, item0, stride, time_at) = if second {
            (0x1640u32, 0x162cu32, 0xbcu32, 0x274u32, 0x1630u32)
        } else {
            (0x1650, 0x163c, 0xac, 0x278, 0x1640)
        };
        let (c, node) = (SCRATCH + 0x3000, 0x6300_0000);
        let kids = [SCRATCH + 0x3200, SCRATCH + 0x3240];
        let saved = t.save();
        // (marked, children, flags, slot 534, slot 390, bit 0 of +0x90)
        let cases = [
            (true, 0usize, 6u8, 0u32, 0u32, 0u8),
            (false, 0, 6, 0, 0, 0),
            (false, 2, 6, 1, 0, 1),
            (false, 2, 4, 0, 0x1079, 0),
            (false, 2, 2, 0, 0, 0),
            (false, 2, 0, 0, 0, 0),
            (false, 0, 6, 0, 0x1079, 1),
            (false, 2, 6, 0, 0, 0),
        ];
        // (+0x26e, +0x26c, +0x268, +0x230)
        let items = [
            (1u16, 0u16, 1.0f32, 1.0f32),
            (0, 0, 5.0, 1.0),
            (0, 1, 0.5, 1.0),
            (0, 2, 3.0, 1.0),
            (0, 0, -3.0, 1.0),
            (0, 0xffff, 5.0, 1.0),
        ];
        for (k, (marked, children, flags, r534, r390, bit90)) in (0u32..).zip(cases) {
            let setup = |t: &mut Twin| {
                t.restore(&saved);
                t.write(node, &vec![0x11; 0x1700]);
                t.set_u8(node + 0x1a, 0xf0);
                t.set_u32(c, node);
                t.set_u8(c + 4, 0x55);
                t.set_u8(c + 5, if marked { 0xf0 } else { 3 + k as u8 });
                let mut prev = node;
                for &kid in kids.iter().take(children).rev() {
                    t.write(kid, &[0x22; 0x10]);
                    t.set_u32(kid, class);
                    t.set_u32(kid + 8, prev);
                    prev = kid;
                }
                t.set_u32(node + 0x10, prev);
                t.set_u8(node + flags_at, flags);
                t.set_u32(node + count_at, if k == 7 { 0 } else { items.len() as u32 });
                t.set_u32(node + time_at, 0.25f32.to_bits());
                for (i, &(bits, steps, time, x230)) in (0u32..).zip(&items) {
                    let r = node + item0 + stride * i;
                    t.set_u16(r + 0x26e, bits);
                    t.set_u16(r + 0x26c, steps);
                    t.set_u32(r + 0x268, time.to_bits());
                    t.set_u32(r + 0x230, x230.to_bits());
                    t.set_u32(r + 0x274, (0.5f32 + i as f32).to_bits());
                    t.set_u32(r + 0x270, 0x0100 + i);
                    t.set_u32(r + 0x204, 0x0020_0000 + i);
                    t.set_u32(r + 0x50, 0x0050_0000 + i);
                    t.set_u32(r + 0x54, 0x0054_0000 + i);
                    t.set_u8(r + 0x90, bit90);
                }
                t.set_u32(SCRATCH + 0x3900, r534);
                t.set_u32(SCRATCH + 0x3904, r390);
            };
            let eax = if marked { Eax::Junk } else { Eax::Checked };
            setup(&mut t);
            t.check(f, Call::this(c), &[], eax, |g, img, fr| {
                o6_fam_pairs5::timed_items_visit(g, img, fr)
            });
            setup(&mut t);
            t.check_entry(t.at(thunk), regs(thunk, k), &[c], eax);
        }
    }
}

const SPRITE_VISIT: &[(u16, u32)] = &[(578, 0x30d0), (585, 0x3110)];

/// `o6_fam_pairs5::sprite_visit` (and its thunk in `magic_0578`): the
/// node marked already (`eax` junk); the delay above 0, at 0, below 0;
/// the draw byte `+0x7b` 0 or not; slot 885 0 or not; bit 4 of `+0xa4`
/// with the node in a parent (it leaves) or without one; colours and
/// counters of both signs. Slot 879 logs the floats and `Q`.
#[test]
#[allow(clippy::too_many_lines)] // the cases' set-up, field by field
fn fam2_sprite_visit() {
    for &(n, rva) in SPRITE_VISIT {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pairs5 sprite_visit");
        if n == 578 {
            copies("o6 fam pairs5 sprite_visit_thunk", &[(578, 0x33c0)]);
        }
        stubs4(&mut t);
        let visit = 1000u32;
        t.stub(visit, move |m, a| {
            let k = m.r32(SCRATCH + 0x3800);
            m.w32(SCRATCH + 0x3800, k + 1);
            m.w32(SCRATCH + 0x3810 + 4 * (k % 8), m.r32(a[0]));
            0x1000 + k
        });
        t.stub(879, |m, a| {
            let mut x = 0u32;
            for &v in &a[..10] {
                x = x.rotate_left(5) ^ v;
            }
            for i in 0..4 {
                x = x.rotate_left(3) ^ m.r32(a[4] + 4 * i);
            }
            m.w32(SCRATCH + 0x3880, x);
            0x879
        });
        let class = SCRATCH + 0x3700;
        t.set_u32(class + 0x1c, magic::TRAPS + 16 * visit);
        let f = t.at(rva);
        let (c, node, parent) = (SCRATCH + 0x3000, SCRATCH + 0x3100, SCRATCH + 0x3300);
        let (owner, fxb, tex) = (SCRATCH + 0x3400, SCRATCH + 0x3500, SCRATCH + 0x3600);
        let kid = SCRATCH + 0x3200;
        let saved = t.save();
        // (marked, delay, draw, 885, +0xa4, parent, colour, +0x98, +0x9c, TEX0 high)
        let cases = [
            (true, 0u16, 1u8, 1u32, 0x10u8, true, 0u32, 0u32, 0u32, 0u32),
            (false, 3, 1, 1, 0x10, true, 0x8040_2010, 5, 7, 1),
            (
                false,
                0,
                1,
                1,
                0x10,
                true,
                0xff7f_0180,
                0x0012_3456,
                0xfff0_1234,
                3,
            ),
            (false, 0xffff, 1, 0, 0x10, true, 0x0102_0304, 9, 9, 2),
            (false, 0, 0, 1, 0x10, true, 0x0102_0304, 9, 9, 2),
            (
                false,
                0,
                1,
                1,
                0xef,
                false,
                0xdead_beef,
                0x8000_0000,
                0x7fff_ffff,
                0,
            ),
            (false, 0, 1, 1, 0x10, false, 0x1122_3344, 100, 200, 1),
        ];
        for (k, case) in (0u32..).zip(cases) {
            let (marked, delay, draw, made, flags, in_parent, colour, c98, c9c, hi) = case;
            let setup = |t: &mut Twin| {
                t.restore(&saved);
                t.write(node, &[0x44; 0xb0]);
                t.set_u8(node + 0x1a, 0xf0);
                t.set_u32(c, node);
                t.set_u8(c + 4, 0x55);
                t.set_u8(c + 5, if marked { 0xf0 } else { 3 + k as u8 });
                t.write(kid, &[0x22; 0x10]);
                t.set_u32(kid, class);
                t.set_u32(kid + 8, node);
                t.set_u32(node + 0x10, kid);
                t.set_u32(node + 4, node);
                t.set_u32(node + 8, node);
                t.write(parent, &[0x33; 0x20]);
                t.set_u32(node + 0x14, if in_parent { parent } else { 0 });
                t.set_u32(node + 0x68, owner);
                t.set_u32(owner + 0x54, fxb);
                t.set_u32(fxb + t.img.word(f + 0xeb), 0x0830_0000 + k);
                t.set_u32(node + 0x6c, 0x006c_0000 + k);
                t.set_u32(node + 0x70, tex);
                t.set_u32(tex, 0x4c0c_1234 ^ k.rotate_left(13));
                t.set_u32(tex + 4, hi);
                t.set_u32(node + 0x78, colour);
                t.set_u8(node + 0x7b, draw);
                t.set_u32(node + 0x98, c98);
                t.set_u32(node + 0x9c, c9c);
                t.set_u8(node + 0xa4, flags);
                t.set_u16(node + 0xaa, delay);
                let lock = SCRATCH + 0x3040;
                t.set_u32(t.img.word(f + 0x126), lock);
                t.set_u32(lock, 0x55 + k);
                t.stub(885, move |m, a| {
                    let mut x = 0u32;
                    for &v in &a[..7] {
                        x = x.rotate_left(7) ^ v;
                    }
                    m.w32(SCRATCH + 0x3884, x);
                    made
                });
            };
            let eax = if marked { Eax::Junk } else { Eax::Checked };
            setup(&mut t);
            t.check(f, Call::this(c), &[], eax, |g, img, fr| {
                o6_fam_pairs5::sprite_visit(g, img, fr)
            });
            if n == 578 {
                setup(&mut t);
                t.check_entry(t.at(0x33c0), regs(0x33c0, k), &[c], eax);
            }
        }
    }
}

const LIGHT_SETUP: &[(u16, u32)] = &[(644, 0xb390), (565, 0x74f0)];

/// `o6_fam_pairs5::light_setup`: the count `*G_N` 0 (slot 847 too), 5,
/// 0x14 (the last below 0x15) and 0x15 (no table row, no slot 931); the
/// float slots answer in ST0 from their word, slot 219 a block whose
/// `+0x830` is the effect word, the matrix slots fill what they are
/// handed.
#[test]
fn fam2_light_setup() {
    for &(n, rva) in LIGHT_SETUP {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pairs5 light_setup");
        stubs4(&mut t);
        let h = SCRATCH + 0x3000;
        t.stub(219, move |_, a| h ^ (a[1] & 0x10));
        for s in [277u32, 15, 16, 17] {
            t.stub_st0(s, move |_, a| {
                f64::from(a[0] & 0xff) * 0.125 + f64::from(s) * 0.5
            });
        }
        for s in [235u32, 723, 724] {
            t.stub(s, move |m, a| {
                for i in 0..16 {
                    m.w32(a[0] + 4 * i, (0.25 * (i + s) as f32).to_bits());
                }
                s
            });
        }
        t.stub(545, |_, _| 0x0054_5000);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        t.set_u32(w(&t, 0x36), 0x0021_9000);
        t.set_u32(h + 0x830, 0x0830_0001);
        let (angles, pos) = (SCRATCH + 0x3100, SCRATCH + 0x3200);
        let saved = t.save();
        for (c, count) in (0u32..).zip([0u32, 5, 0x14, 0x15]) {
            t.restore(&saved);
            for i in 0..4 {
                t.set_u32(angles + 4 * i, (0.5 + (i + c) as f32).to_bits());
                t.set_u32(pos + 4 * i, (-1.5 * (i + 1) as f32).to_bits());
            }
            let gn = w(&t, 0x18b);
            t.set_u32(gn, count);
            let call = Call {
                ecx: angles,
                edx: pos,
                ..Call::default()
            };
            t.check(f, call, &[7 + c], Eax::Checked, |g, img, fr| {
                o6_fam_pairs5::light_setup(g, img, fr)
            });
        }
    }
}

const TURN_BOX: &[(u16, u32)] = &[(307, 0x1d20), (605, 0x76a0)];

/// `o6_fam_pairs5::turn_box`: the object's turn above, inside and below
/// the wrap range for both wraps (`KA` and `KD` differ), and NaN (no
/// wrap); the matrix slots fill what they are handed, slot 858 logs the
/// corners.
#[test]
fn fam2_turn_box() {
    for (n, rva) in copies("o6 fam pairs5 turn_box", TURN_BOX) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        for s in [235u32, 229, 554, 327] {
            t.stub(s, move |m, a| {
                for i in 0..16 {
                    m.w32(a[0] + 4 * i, (0.125 * (i + s) as f32).to_bits());
                }
                s
            });
        }
        t.stub(858, |m, a| {
            let mut x = 0u32;
            for &v in &a[..5] {
                x = x.rotate_left(5) ^ v;
            }
            for i in 0..4 {
                x = x.rotate_left(3) ^ m.r32(a[2] + 4 * i) ^ m.r32(a[3] + 4 * i);
            }
            m.w32(SCRATCH + 0x3880, x);
            0x858
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let obj = 0x6400_0000;
        t.set_u32(w(&t, 0x1e), obj);
        let saved = t.save();
        for (c, r) in (0u32..).zip([0.0f32, 3.5, 7.0, 12.0, -7.0, -12.0, f32::NAN]) {
            t.restore(&saved);
            t.write(obj, &vec![0x11; 0x2000]);
            t.set_u32(obj + w(&t, 0x2d), r.to_bits());
            for (i, o) in [0x7eu32, 0x8b, 0x98, 0xb1].into_iter().enumerate() {
                t.set_u32(
                    obj + w(&t, o),
                    (1.5 * (i as f32 + 1.0) - c as f32).to_bits(),
                );
            }
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const PARTICLE_ROWS: &[(u16, u32)] = &[(304, 0x2ae0), (568, 0x2510)];

/// `o6_fam_pairs5::particle_rows`: a pool of 220 free records searched
/// from a negative index (no wrap), with the first busy record at 0 (no
/// particles), 13 (the pool runs out in the second row) or none (all 200
/// particles); slots 314/404 answer floats from their word, 229 fills
/// `W` from what it is handed.
#[test]
fn fam2_particle_rows() {
    for (d, (n, rva)) in (0u32..).zip(copies("o6 fam pairs5 particle_rows", PARTICLE_ROWS)) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        t.stub_st0(314, |m, a| {
            let k = m.r32(SCRATCH + 0x3800);
            m.w32(SCRATCH + 0x3800, k + 1);
            f64::from(f32::from_bits(a[0])) * 0.375 + f64::from(k % 7) * 0.25
        });
        t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) - 0.2);
        t.stub(229, |m, a| {
            let mut x = a[1] ^ a[2];
            for i in 0..4 {
                x = x.rotate_left(7) ^ m.r32(a[2] + 4 * i);
            }
            for i in 0..4 {
                m.w32(a[0] + 4 * i, (0.5 * (x % 13 + i) as f32).to_bits());
            }
            0x229
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        // an object and a pool of its own per DLL (the original's arena
        // keeps the last DLL's pages)
        let own = 0x0100_0000 * d;
        let obj = 0x6500_0000 + own;
        t.set_u32(w(&t, 0x14), obj);
        t.write(obj + w(&t, 0x9a), &[0x40; 0x10]);
        t.set_u32(w(&t, 0x71), SCRATCH + 0x3000);
        t.set_u32(SCRATCH + 0x3008, 0x0077_1234);
        let take = t.img.rel32_target(f + 0x61);
        let tw = |t: &Twin, o: u32| t.img.word(take + o);
        let pool = 0x5800_0000 + own;
        let start = 230u32;
        t.set_u32(tw(&t, 0x37), pool + 0x74 * start);
        let saved = t.save();
        for (c, busy) in (0u32..).zip([Some(0u32), Some(13), None]) {
            t.restore(&saved);
            t.set_u32(tw(&t, 0x08), start.wrapping_neg());
            t.set_u32(tw(&t, 0x0f), 1);
            t.write(pool, &vec![0x33; 0x74 * 220]);
            for i in 0..220 {
                t.set_u32(pool + 0x74 * i + 0x6c, u32::from(Some(i) == busy));
            }
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const NODE_RINGS: &[(u16, u32)] = &[(150, 0xb210), (603, 0xb140)];

/// `o6_fam_pairs5::node_rings`: skipped; still waiting; below the limit;
/// the ring's float below `K2` (no nodes); the first ring (the effect
/// nodes too) with `*G_MODE` 0 and 1; a ring from 5 on (slot 239); the
/// tenth (the task leaves its list); `E + 0xe64` going below 0 or not.
/// Slot 307 answers a float, the node set-ups run as ported.
#[test]
fn fam2_node_rings() {
    for (d, (n, rva)) in (0u32..).zip(copies("o6 fam pairs5 node_rings", NODE_RINGS)) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        fill(&mut t, P, 0x40, 17);
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x3804, a[0]);
            P
        });
        t.stub_st0(307, |m, _| {
            let k = m.r32(SCRATCH + 0x3800);
            m.w32(SCRATCH + 0x3800, k + 1);
            0.75 + f64::from(k)
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        // an effect of its own per DLL (the original's arena keeps the
        // last DLL's pages)
        let e = 0x6600_0000 + 0x0100_0000 * d;
        let (task, link) = (SCRATCH + 0x3000, SCRATCH + 0x3040);
        let (count, wait, time, limit) = (w(&t, 0x20), w(&t, 0x54), w(&t, 0x63), w(&t, 0x72));
        let saved = t.save();
        // (skip, ring, wait, limit, the ring's float, mode, +0xe64)
        let cases = [
            (1u32, 3u32, 1u32, 0.0f32, 1.0e6f32, 0u32, 1.0f32),
            (0, 3, 2, 0.0, 1.0e6, 0, 1.0),
            (0, 3, 1, 1.0e6, 1.0e6, 0, 1.0),
            (0, 3, 1, 0.0, -1.0e6, 0, -1.0e6),
            (0, 0, 1, 0.0, 1.0e6, 0, 1.0),
            (0, 0, 1, 0.0, 1.0e6, 1, 1.0),
            (0, 6, 1, 0.0, 1.0e6, 1, 1.0e6),
            (0, 9, 1, 0.0, 1.0e6, 0, 1.0),
        ];
        for (c, (skip, ring, wt, lim, x, mode, e64)) in (0u32..).zip(cases) {
            t.restore(&saved);
            t.write(e, &vec![0x11; 0x2d00]);
            t.set_u32(e + count, ring);
            t.set_u32(e + wait, wt);
            t.set_u32(e + time, 100.0f32.to_bits());
            t.set_u32(e + limit, lim.to_bits());
            t.set_u32(e + 0xe64, e64.to_bits());
            t.set_u32(e + 0x468 + 0x10 * ring, x.to_bits());
            t.set_u32(w(&t, 0x0b), skip);
            t.set_u32(w(&t, 0x171), mode);
            // the node list: empty (its last is itself) or holding a node
            let list = w(&t, 0xce);
            let held = SCRATCH + 0x3100;
            t.write(held, &[0x69; 0x20]);
            t.set_u32(list + 0x10, if c & 1 == 0 { list } else { held });
            t.set_u16(list + 0x18, 1);
            // the particle and effect nodes' own lists: empty
            let pnw = t.img.rel32_target(f + 0x2d7);
            let enw = t.img.rel32_target(f + 0x2fa);
            for l in [t.img.word(pnw + 0x26), t.img.word(enw + 0x1e)] {
                t.set_u32(l + 0x10, l);
            }
            // the particle node's owner and its program table (slot 941)
            let owner = t.img.word(pnw + 0x0d);
            t.set_u32(owner, SCRATCH + 0x3500);
            fill(&mut t, SCRATCH + 0x3500, 0x20, 41);
            t.write(task, &[0x5a; 0x10]);
            t.set_u32(task, link);
            t.set_u32(task + 4, e);
            let (prev, next) = (SCRATCH + 0x3080, SCRATCH + 0x30c0);
            t.write(link, &[0x66; 0x10]);
            t.set_u32(link + 4, prev);
            t.set_u32(link + 8, next);
            t.write(prev, &[0x67; 0x10]);
            t.write(next, &[0x68; 0x10]);
            t.check_entry(f, regs(rva, c), &[task], Eax::Checked);
        }
    }
}

const TWO_PASS_PARTICLES: &[(u16, u32)] = &[(304, 0x2740), (568, 0x2170)];

/// `o6_fam_pairs5::two_pass_particles`: a pool of 130 free records
/// searched from a negative index (no wrap), with the first busy record
/// at 0 (no particles), 70 (the second pass runs out) or none (all 120);
/// slots 314/404 answer floats from their word and a running count, 229
/// fills `W` from what it is handed.
#[test]
fn fam2_two_pass_particles() {
    for (d, (n, rva)) in (0u32..).zip(copies(
        "o6 fam pairs5 two_pass_particles",
        TWO_PASS_PARTICLES,
    )) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        for s in [314u32, 404] {
            t.stub_st0(s, move |m, a| {
                let k = m.r32(SCRATCH + 0x3800);
                m.w32(SCRATCH + 0x3800, k + 1);
                f64::from(f32::from_bits(a[0])) * 0.375 + f64::from((k + s) % 7) * 0.25
            });
        }
        t.stub(229, |m, a| {
            let mut x = a[1] ^ a[2];
            for i in 0..4 {
                x = x.rotate_left(7) ^ m.r32(a[2] + 4 * i);
            }
            for i in 0..4 {
                m.w32(a[0] + 4 * i, (0.5 * (x % 13 + i) as f32).to_bits());
            }
            0x229
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        // an object and a pool of their own per DLL
        let own = 0x0100_0000 * d;
        let obj = 0x6500_0000 + own;
        t.set_u32(w(&t, 0x18), obj);
        for (i, o) in [0x26u32, 0x2e, 0x43, 0x56].into_iter().enumerate() {
            t.set_u32(obj + w(&t, o), (1.25 * (i as f32 + 1.0)).to_bits());
        }
        t.set_u32(w(&t, 0xb6), SCRATCH + 0x3000);
        t.set_u32(SCRATCH + 0x300c, 0x0077_1234);
        let take = t.img.rel32_target(f + 0xa6);
        let tw = |t: &Twin, o: u32| t.img.word(take + o);
        let pool = 0x5800_0000 + own;
        let start = 140u32;
        t.set_u32(tw(&t, 0x37), pool + 0x74 * start);
        let saved = t.save();
        for (c, busy) in (0u32..).zip([Some(0u32), Some(70), None]) {
            t.restore(&saved);
            t.set_u32(tw(&t, 0x08), start.wrapping_neg());
            t.set_u32(tw(&t, 0x0f), 1);
            t.write(pool, &vec![0x33; 0x74 * 130]);
            for i in 0..130 {
                t.set_u32(pool + 0x74 * i + 0x6c, u32::from(Some(i) == busy));
            }
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const MESH_CENTRES: &[(u16, u32)] = &[(307, 0x2010), (605, 0x7990)];

/// Lays out mesh `k` at `m`: quads (type 6), triangles (type 2), a packet
/// of another type and an empty one, by `k`; the vertex count total
/// from the packets (0 or below for some meshes).
fn mesh(t: &mut Twin, m: u32, k: u32) {
    t.write(m, &[0x5a; 0x400]);
    t.set_u32(m + 4, 0x40);
    t.set_u32(m + 8, 0x300);
    for i in 0..32u32 {
        for c in 0..3u32 {
            let x = ((i * 37 + c * 11 + k * 5) % 200) as i16 - 100;
            t.set_u16(m + 0x300 + 6 * i + 2 * c, x.cast_unsigned());
        }
    }
    // (type, count, face size, index offset, corners)
    let packets: &[(u8, u16, u32, u32, u32)] = match k % 4 {
        0 => &[(6, 2, 0x28, 0x10, 4), (2, 3, 0x20, 0xc, 3)],
        1 => &[
            (2, 1, 0x20, 0xc, 3),
            (5, 1, 0, 0, 0),
            (6, 0, 0x28, 0x10, 4),
            (6, 1, 0x28, 0x10, 4),
        ],
        2 => &[(6, 3, 0x28, 0x10, 4)],
        _ => &[],
    };
    let mut p = m + 0x40;
    let mut total = 0u16;
    for (j, &(kind, n, size, first, corners)) in (0u32..).zip(packets) {
        t.set_u8(p + 1, kind);
        t.set_u16(p + 2, n);
        total += n;
        let mut face = p + 0x10;
        for q in 0..u32::from(n) {
            for corner in 0..corners {
                let index = (q * 7 + corner * 3 + j + k) % 32;
                t.set_u16(face + first + 2 * corner, index as u16);
            }
            face += size;
        }
        p = face;
    }
    let count = if k % 4 == 3 {
        if k % 8 == 3 { 0 } else { 0xfffe }
    } else {
        total
    };
    t.set_u16(m + 0x10, count);
}

/// `o6_fam_pairs5::mesh_centres`: 18 meshes of quads, triangles, other
/// packets and none (the count 0 and below); slot 564 scales the vector
/// it is handed, slot 248 adds, as the game's do (in f32).
#[test]
fn fam2_mesh_centres() {
    for (d, (n, rva)) in (0u32..).zip(copies("o6 fam pairs5 mesh_centres", MESH_CENTRES)) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        t.stub(564, |m, a| {
            let k = f32::from_bits(a[2]);
            for i in 0..4 {
                let x = f32::from_bits(m.r32(a[1] + 4 * i)) / k;
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x564
        });
        t.stub(248, |m, a| {
            for i in 0..4 {
                let x = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x248
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let own = 0x0100_0000 * d;
        let (obj, table, meshes) = (0x6700_0000 + own, 0x6780_0000 + own, 0x6800_0000 + own);
        t.write(obj + w(&t, 0x1a), &[0x77; 0x60 * 18]);
        t.set_u32(w(&t, 0x14), obj);
        t.set_u32(w(&t, 0x31), table);
        for k in 0..18u32 {
            let m = meshes + 0x1000 * k;
            t.set_u32(table + 0x20 * k + w(&t, 0x38), m);
            mesh(&mut t, m, k);
        }
        t.check_entry(f, regs(rva, 0), &[], Eax::Checked);
        // the last mesh empty: `eax` is its count
        t.set_u16(meshes + 0x1000 * 17 + 0x10, 0x8000);
        t.check_entry(f, regs(rva, 1), &[], Eax::Checked);
    }
}

const TARGET_OBJECTS: &[(u16, u32)] = &[(644, 0x8900), (565, 0x48e0)];

/// `o6_fam_pairs5::target_objects`: no targets, and three (the node take
/// with the free count 3, then 1: the busy list empty or holding a node);
/// the object to remove in the list or not. Slot 262 writes the target's
/// position (the distances' signs vary), slot 237 the vector `L`; the
/// arcs are the game's `_CIatan2` (the harness's), the objects start as
/// ported.
#[test]
fn fam2_target_objects() {
    for (d, (n, rva)) in (0u32..).zip(copies("o6 fam pairs5 target_objects", TARGET_OBJECTS)) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        t.stub(237, |m, a| {
            for i in 0..4 {
                m.w32(a[3] + 4 * i, (1.5 - i as f32).to_bits());
            }
            0x237
        });
        t.stub(262, |m, a| {
            for (i, &p) in a[1..4].iter().enumerate() {
                let x = ((a[0] ^ p) % 23) as f32 * 9.75 - 108.0 + i as f32;
                m.w32(p, x.to_bits());
            }
            0x262
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let own = 0x0100_0000 * d;
        let obj = 0x6900_0000 + own;
        let list = SCRATCH + 0x3000;
        t.set_u32(w(&t, 0x19), obj);
        t.set_u32(w(&t, 0xbe), SCRATCH + 0x3100);
        t.write(SCRATCH + 0x3100, &[1, 2, 3, 0, 2, 1, 3, 0]);
        // the key table: distinct keys
        let keys = w(&t, 0xc9);
        fill(&mut t, keys, 4, 61);
        t.set_u32(w(&t, 0x3b0), SCRATCH + 0x3200);
        // the object start's owner block and program table (slot 941)
        let start = t.img.rel32_target(f + 0x281);
        let owner = t.img.word(start + 0x5e);
        t.set_u32(owner, SCRATCH + 0x3a00);
        fill(&mut t, SCRATCH + 0x3a00, 0x10, 5);
        fill(&mut t, P, 0x40, 17);
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x3b00, a[0]);
            P
        });
        let saved = t.save();
        for (c, (count, free, busy, listed)) in (0u32..).zip([
            (0u32, 3u16, false, false),
            (3, 3, false, true),
            (1, 1, true, false),
        ]) {
            t.restore(&saved);
            t.write(obj, &vec![0x11; 0x10000]);
            t.set_u32(obj + w(&t, 0x21), list);
            t.write(list, &[0x22; 0x20]);
            t.set_u32(list + 4, count);
            for i in 0..8 {
                t.set_u8(list + 8 + i, (i * 3 + c) as u8 % 6);
            }
            t.set_u32(obj + w(&t, 0x267), c % 2);
            // the node ring and the busy list
            let node = |i: u32| SCRATCH + 0x3400 + 0x40 * i;
            for i in 0..3 {
                t.write(node(i), &[0x77; 0x20]);
                t.set_u32(node(i), node((i + 2) % 3));
                t.set_u32(node(i) + 4, node((i + 1) % 3));
            }
            t.set_u32(w(&t, 0x2ca), node(1));
            t.set_u16(w(&t, 0x2b2), free);
            let held = SCRATCH + 0x3600;
            t.write(held, &[0x66; 0x20]);
            t.set_u32(w(&t, 0x331), if busy { held } else { 0 });
            t.set_u32(w(&t, 0x349), held);
            // the object to remove and the list `LS`
            let gone = SCRATCH + 0x3800;
            t.write(gone, &[0x55; 0x130]);
            t.set_u32(obj + w(&t, 0x391), gone);
            let ls = w(&t, 0x398);
            let lnode = SCRATCH + 0x3700;
            t.write(lnode, &[0; 0x10]);
            t.set_u32(lnode + 8, if listed { gone } else { 0x1234 });
            t.set_u32(ls, lnode);
            t.set_u32(ls + 4, lnode);
            t.set_u32(ls + 8, 0);
            t.set_u32(ls + 0xc, 1);
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const SPRITE_FRAME: &[(u16, u32)] = &[(206, 0x2730), (371, 0x2480)];

/// `o6_fam_pairs5::sprite_frame`: no record, slot 941 answering 0 (the
/// error string); the timer running on, or running out with the frame
/// moving on (and wrapping at the table's count); slot 773 0 (slot 778
/// with slots 776/777) or not (slots 774/775). Slot 772 gives a 64-bit
/// handle; the lock word is logged by the drawing slots.
#[test]
fn fam2_sprite_frame() {
    for &(n, rva) in SPRITE_FRAME {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pairs5 sprite_frame");
        stubs4(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let (r, tbl) = (SCRATCH + 0x3000, SCRATCH + 0x3200);
        let lock = SCRATCH + 0x3100;
        t.set_u32(w(&t, 0x290), lock);
        t.stub(941, move |m, a| {
            m.w32(SCRATCH + 0x3880, a[0]);
            if m.r32(SCRATCH + 0x3900) == 0 { tbl } else { 0 }
        });
        t.stub(772, |m, a| {
            m.w32(SCRATCH + 0x3884, a[0] ^ a[1]);
            0x0772_0001
        });
        t.stub_edx(772, |_, _| 0x0772_0002);
        t.stub(773, move |m, a| {
            m.w32(SCRATCH + 0x3888, a[0] ^ a[1] ^ a[2] ^ a[3] ^ m.r32(lock));
            m.r32(SCRATCH + 0x3904)
        });
        for s in [774u32, 775, 778] {
            t.stub(s, move |m, a| {
                let words = match s {
                    774 => 5,
                    775 => 2,
                    _ => 6,
                };
                let mut x = m.r32(lock);
                for &v in &a[..words] {
                    x = x.rotate_left(5) ^ v;
                }
                m.w32(SCRATCH + 0x3890 + 4 * (s - 774), x);
                s
            });
        }
        t.stub(776, |_, a| a[0] << 4);
        t.stub(777, |_, a| a[0] ^ 0x7700);
        let saved = t.save();
        // (record, 941 gives 0, the timer, the frame, 773's answer)
        let cases = [
            (false, false, 0x300u16, 1u16, 1u32),
            (true, true, 0x300, 1, 1),
            (true, false, 0x300, 1, 1),
            (true, false, 0x200, 1, 0),
            (true, false, 0x200, 2, 1),
        ];
        for (c, (with_r, none, timer, frame, drawn)) in (0u32..).zip(cases) {
            t.restore(&saved);
            t.write(r, &[0x44; 0x40]);
            t.set_u32(r, 0x0941_0000 + c);
            t.set_u16(r + 8, frame);
            t.set_u16(r + 0x10, timer);
            t.set_u32(r + 0xc, tbl + 0x80);
            t.set_u32(r + 0x24, 1.5f32.to_bits());
            t.set_u32(r + 0x28, (-0.5f32).to_bits());
            t.set_u32(r + 0x38, (40.25f32 + c as f32).to_bits());
            fill(&mut t, tbl, 0x40, 23 + c);
            t.set_u16(tbl + 4, 3);
            t.write(r - 0x40, &[0x5b; 0x40]);
            t.set_u32(w(&t, 0x179), (2.5f32 + c as f32).to_bits());
            for i in 0..3u32 {
                t.set_u16(tbl + 8 * i + 0x10, (0x80 + 8 * i) as u16);
                t.set_u16(tbl + 8 * i + 0x12, (0x100 * (i + 1)) as u16);
            }
            for i in 0..4u32 {
                t.set_u32(tbl + 0x80 + 8 * i + 4, 0x10 + i);
            }
            t.set_u32(SCRATCH + 0x3900, u32::from(none));
            t.set_u32(SCRATCH + 0x3904, drawn);
            t.set_u32(lock, 0x55 + c);
            let rec = if with_r { r } else { 0 };
            let call = Call::this(rec).with_xmm(&[
                Xmm::f32(9.0),
                Xmm::f32(8.0),
                Xmm::f32(7.0),
                Xmm::f32(0.25 + c as f32),
            ]);
            t.check(
                f,
                call,
                &[0x1d00 + c, 0x2a00 + c],
                Eax::Checked,
                o6_fam_pairs5::sprite_frame,
            );
        }
    }
}

const MOVING_SPRITES: &[(u16, u32)] = &[(250, 0x1ee0), (251, 0x2090)];

/// One moving sprite record (0x68 bytes at `rec`, `p = rec + 0x14`),
/// variant `v`: active or not, the callback, the stop flag, the level
/// past its limit or not, the speeds 0 or not, the colour blend, the
/// frame and timer, and the countdown `p.38`.
fn sprite_record(t: &mut Twin, rec: u32, v: u32, cb: u32) {
    let p = rec + 0x14;
    t.write(rec, &[0x21; 0x68]);
    t.set_u32(rec, 0x0941_0000 + v);
    t.set_u32(rec + 4, [1u32, 0, 2, 1][(v % 4) as usize]);
    t.set_u16(rec + 8, [0u16, 2, 1, 2][(v % 4) as usize]);
    t.set_u16(
        rec + 0xa,
        [0x200u16, 0x300, 0x180, 0x200][(v / 2 % 4) as usize],
    );
    let fl = |x: f32| x.to_bits();
    for (o, x) in [
        (0u32, 1.0f32),
        (0xc, 0.5),
        (0x10, if v % 5 == 1 { 0.0 } else { -0.25 }),
        (0x14, 0.75),
        (0x18, if v % 7 == 2 { -1.5 } else { 0.125 }),
        (0x1c, 0.0625),
        (0x20, if v.is_multiple_of(3) { 8.0 } else { 0.5 }),
        (0x24, 0.875),
        (0x28, if v.is_multiple_of(2) { -100.0 } else { 100.0 }),
        (0x2c, 3.0 + v as f32),
        (0x30, 1.5),
        (0x34, 2.0),
    ] {
        t.set_u32(p + o, fl(x));
    }
    t.set_u32(rec + 0x10, fl(-3.0));
    t.set_u32(rec + 0xc, fl(2.0));
    t.set_u32(p + 8, cb);
    t.set_u32(p + 0x38, [0u32, 3, 1, 0xffff_ffff][(v % 4) as usize]);
    t.set_u32(p + 0x3c, [4u32, 0, 4, 2][(v / 3 % 4) as usize]);
    t.set_u32(p + 0x40, [1u32, 0xffff_fffe, 4, 0][(v % 4) as usize]);
    t.set_u32(p + 0x44, 0x4080_c0ff ^ v);
    t.set_u32(p + 0x48, 0x1020_3040 + v);
    t.set_u32(p + 0x4c, u32::from(v % 6 != 5));
    t.set_u8(p + 0x50, u8::from(v % 8 == 6));
}

/// `o6_fam_pairs5::moving_sprites`: no records, then twelve records
/// through the motion, the limit's back-off (speeds 0 or not), the
/// shrink test, the colour blend (count 0, signs of the step), the
/// frames (timer on/out, wrapping or stopping at the table's end) and the
/// countdown; the callback a slot trap that moves the record; slot 773
/// answering 0 or not for the part links.
#[test]
fn fam2_moving_sprites() {
    for (d, (n, rva)) in (0u32..).zip(copies("o6 fam pairs5 moving_sprites", MOVING_SPRITES)) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        let tbl = SCRATCH + 0x3000;
        t.stub(941, move |m, a| {
            m.w32(SCRATCH + 0x3880, a[0]);
            tbl
        });
        let cb = 1001u32;
        t.stub(cb, |m, a| {
            let y = f32::from_bits(m.r32(a[0] + 0x14 + 0x18));
            m.w32(a[0] + 0x14 + 0x18, (y * 0.5).to_bits());
            0xcb
        });
        t.stub(773, |m, a| m.r32(SCRATCH + 0x3904) ^ (a[3] & 1));
        t.stub_edx(772, |_, a| a[1] ^ 0x0772_0000);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let link = t.img.rel32_target(f + 0x3c3);
        t.set_u32(t.img.word(link + 0x4f), SCRATCH + 0x3100);
        let recs = 0x6a00_0000 + 0x0100_0000 * d;
        t.set_u32(w(&t, 0x1e), recs);
        // the frame table: 3 entries; the last's byte +4 by case
        t.write(tbl, &[0; 0x80]);
        t.set_u16(tbl + 6, 3);
        for i in 0..3u32 {
            let e = tbl + 8 * (i + 2);
            t.set_u16(e, 0x40);
            t.set_u16(e + 2, (0x100 * (i + 1)) as u16);
        }
        t.set_u32(tbl + 0x44, 8);
        let saved = t.save();
        for (c, count) in (0u32..).zip([0u32, 12, 12]) {
            t.restore(&saved);
            t.set_u32(w(&t, 0x6d), count);
            t.set_u8(tbl + 8 * 4 + 4, if c == 1 { 0x80 } else { 0 });
            t.set_u32(SCRATCH + 0x3904, c & 1);
            for v in 0..count {
                let trap = if v % 3 == 1 {
                    magic::TRAPS + 16 * cb
                } else {
                    0
                };
                sprite_record(&mut t, recs + 0x68 * v, v + c, trap);
            }
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const ACTOR_LIGHT: &[(u16, u32)] = &[(565, 0x5470), (644, 0x92b0)];

/// `o6_fam_pairs5::actor_light`: the countdown above 1 (the light only),
/// reaching 0, at 0 and below; the effect's flag set (the glow record
/// and its node, the busy list empty or not) or clear (the new aim). The
/// actor's object start has the environment of
/// `actor_object_start_in_every_copy`; slots 314/404 answer floats from
/// their word, 229 transforms and 248 adds the vectors they are handed.
#[test]
#[allow(clippy::too_many_lines)] // the set-up of four callees
fn fam2_actor_light() {
    for (d, (n, rva)) in (0u32..).zip(copies("o6 fam pairs5 actor_light", ACTOR_LIGHT)) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        // the actor's object start (as in actor_object_start_in_every_copy)
        let aos = t.img.rel32_target(f + 0xfb);
        let at = t.img.rel32_target(aos + 0x3e);
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
        let pa = t.img.rel32_target(at + 0x9d);
        let free = t.img.word(pa + 0x04);
        let nodes = t.img.word(pa + 0x27) - 8;
        let tail = t.img.word(at + 0xa8);
        let (bytes_k, bytes_t) = (SCRATCH + 0x3000, SCRATCH + 0x3100);
        t.write(bytes_k - 4, &[1, 2, 3, 4, 5, 6, 7, 8]);
        t.write(bytes_t, &[9, 10, 11, 12, 13, 14, 15, 16]);
        t.set_u32(t.img.word(aos + 0x10), bytes_k);
        t.set_u32(t.img.word(aos + 0x99), bytes_t);
        let own = 0x0100_0000 * d;
        let gobj = 0x6b00_0000 + own;
        t.set_u32(t.img.word(aos + 0xa8), gobj);
        t.set_u32(w(&t, 0x1e), gobj);
        let (scales, indices) = (t.img.word(aos + 0x1f), t.img.word(aos + 0x37));
        let k0 = (SCRATCH + 0x5000).wrapping_sub(indices) / 4;
        for i in 0..4u32 {
            let k = k0 + i;
            t.set_u32(
                scales.wrapping_add(4 * k),
                (0.5f32 + 0.25 * i as f32).to_bits(),
            );
            t.set_u32(indices.wrapping_add(4 * k), 5 - i);
        }
        t.stub(641, move |_, _| k0);
        // the floats and the vectors
        for s in [314u32, 404] {
            t.stub_st0(s, move |_, a| {
                f64::from(f32::from_bits(a[0])) * 1.5 + f64::from(s % 7) * 0.125
            });
        }
        t.stub(229, |m, a| {
            for i in 0..4 {
                let x = f32::from_bits(m.r32(a[2] + 4 * i)) * 1.25
                    + f32::from_bits(m.r32(a[1] + 4 * i));
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x229
        });
        t.stub(248, |m, a| {
            for i in 0..4 {
                let x = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x248
        });
        // the glow's node: the free ring and the busy list
        let take = t.img.rel32_target(f + 0x161);
        let (count, head) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
        let task = SCRATCH + 0x3400;
        let rec = 0x6c00_0000 + own;
        let saved = t.save();
        // (countdown, the flag, the busy list held)
        let cases = [
            (5u32, 0u32, false),
            (1, 1, false),
            (0, 0, false),
            (0xffff_fffd, 0, false),
            (1, 1, true),
        ];
        for (c, (countdown, flag, busy)) in (0u32..).zip(cases) {
            t.restore(&saved);
            for i in 0..8 {
                t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
                t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
                t.set_u32(nodes + 12 * i + 8, 0);
            }
            t.set_u16(free, 3);
            t.set_u32(tail, 0);
            t.write(gobj, &vec![0x31; 0x10000]);
            t.set_u32(gobj + w(&t, 0x105), flag);
            t.set_u32(gobj + 0xd618, c & 2);
            t.write(rec, &vec![0x5a; 0x600]);
            t.set_u32(rec + 0x514, if c & 1 == 0 { u32::MAX } else { 2 });
            t.set_u32(rec + 0x50c, countdown);
            t.set_u32(rec + 0x510, 6);
            for (i, x) in [1.5f32, -2.0, 0.75, 1.0, -0.5, 3.0, 2.25, 1.0]
                .into_iter()
                .enumerate()
            {
                t.set_u32(rec + 0x260 + 4 * i as u32, x.to_bits());
            }
            t.set_u32(rec + 0x280, 0.375f32.to_bits());
            t.set_u32(rec + 0x284, (-1.25f32).to_bits());
            t.write(task, &[0x44; 0x20]);
            t.set_u32(task + 0x14, rec);
            t.set_u32(task + 8, SCRATCH + 0x3500);
            let ring = |i: u32| SCRATCH + 0x3600 + 0x40 * i;
            for i in 0..3 {
                t.write(ring(i), &[0x77; 0x20]);
                t.set_u32(ring(i), ring((i + 2) % 3));
                t.set_u32(ring(i) + 4, ring((i + 1) % 3));
            }
            t.set_u16(count, 3);
            t.set_u32(head, ring(1));
            let busy_node = SCRATCH + 0x3800;
            t.write(busy_node, &[0x66; 0x20]);
            t.set_u32(w(&t, 0x167), if busy { busy_node } else { 0 });
            t.set_u32(w(&t, 0x17a), busy_node);
            t.check_entry(f, regs(rva, c), &[task], Eax::Checked);
        }
    }
}

const AIMED_OBJECTS: &[(u16, u32)] = &[(565, 0x40d0), (644, 0x80f0)];

/// `o6_fam_pairs6::aimed_objects`: no targets, three (the free count 3,
/// the busy list empty) and one (the free count 1, the busy list holding
/// a node); the pass word 0 (the sprite set up, `rand` drawn) or not.
/// Slot 262 writes the target's position, slot 237 the vector `L`; the
/// objects start as ported.
#[test]
fn fam2_aimed_objects() {
    for (d, (n, rva)) in (0u32..).zip(copies("o6 fam pairs6 aimed_objects", AIMED_OBJECTS)) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        t.stub(237, |m, a| {
            for i in 0..4 {
                m.w32(a[3] + 4 * i, (1.5 - i as f32).to_bits());
            }
            0x237
        });
        t.stub(262, |m, a| {
            for (i, &p) in a[1..4].iter().enumerate() {
                let x = ((a[0] ^ p) % 23) as f32 * 9.75 - 108.0 + i as f32;
                m.w32(p, x.to_bits());
            }
            0x262
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let obj = 0x6d00_0000 + 0x0100_0000 * d;
        let list = SCRATCH + 0x3000;
        t.set_u32(w(&t, 0x18), obj);
        t.set_u32(w(&t, 0xd6), SCRATCH + 0x3100);
        t.write(SCRATCH + 0x3100, &[1, 2, 3, 0, 2, 1, 3, 0]);
        // the key table: distinct keys
        let keys = w(&t, 0xe1);
        fill(&mut t, keys, 4, 61);
        t.set_u32(w(&t, 0x280), SCRATCH + 0x3300);
        fill(&mut t, SCRATCH + 0x3300, 0x10, 7);
        let start = t.img.rel32_target(f + 0x2d6);
        t.set_u32(t.img.word(start + 0x04), SCRATCH + 0x3200);
        t.set_u32(t.img.word(start + 0x5e), SCRATCH + 0x3a00);
        fill(&mut t, SCRATCH + 0x3a00, 0x10, 5);
        fill(&mut t, P, 0x40, 17);
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x3b00, a[0]);
            P
        });
        let saved = t.save();
        for (c, (count, free, busy, pass)) in
            (0u32..).zip([(0u32, 3u16, false, 0u32), (3, 3, false, 0), (1, 1, true, 1)])
        {
            t.restore(&saved);
            t.write(obj, &vec![0x11; 0x10000]);
            t.set_u32(obj + w(&t, 0x1f), list);
            t.write(list, &[0x22; 0x20]);
            t.set_u32(list + 4, count);
            for i in 0..8 {
                t.set_u8(list + 8 + i, (i * 3 + c) as u8 % 6);
            }
            t.set_u32(obj + w(&t, 0x277), pass);
            let node = |i: u32| SCRATCH + 0x3400 + 0x40 * i;
            for i in 0..3 {
                t.write(node(i), &[0x77; 0x20]);
                t.set_u32(node(i), node((i + 2) % 3));
                t.set_u32(node(i) + 4, node((i + 1) % 3));
            }
            t.set_u32(w(&t, 0x323), node(1));
            t.set_u16(w(&t, 0x2e0), free);
            let held = SCRATCH + 0x3600;
            t.write(held, &[0x66; 0x20]);
            t.set_u32(w(&t, 0x38a), if busy { held } else { 0 });
            t.set_u32(w(&t, 0x3a0), held);
            t.check_entry(f, regs(rva, c), &[], Eax::Checked);
        }
    }
}

const CURVE_FLYERS: &[(u16, u32)] = &[(307, 0x2be0), (563, 0x26d0)];

/// `o6_fam_pairs6::curve_flyers`: the two records' times before and past
/// the end, their countdowns running or out (the record in the list or
/// not, the task given back); the pool with eight free records, or with
/// the third one taken (the first record's particles stop there and the
/// second's at once). Slot 314 answers floats from its word and a count.
#[test]
fn fam2_curve_flyers() {
    for (d, (n, rva)) in (0u32..).zip(copies("o6 fam pairs6 curve_flyers", CURVE_FLYERS)) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        t.stub_st0(314, |m, a| {
            let k = m.r32(SCRATCH + 0x3800);
            m.w32(SCRATCH + 0x3800, k + 1);
            f64::from(f32::from_bits(a[0])) * 0.375 + f64::from(k % 5) * 0.25
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let own = 0x0100_0000 * d;
        let obj = 0x6e00_0000 + own;
        t.set_u32(w(&t, 0x27), obj);
        let recs = obj + w(&t, 0x2d);
        t.set_u32(w(&t, 0x1f4), SCRATCH + 0x3000);
        t.set_u32(w(&t, 0x241), SCRATCH + 0x3100);
        fill(&mut t, SCRATCH + 0x3100, 8, 3);
        let take = t.img.rel32_target(f + 0x231);
        let tw = |t: &Twin, o: u32| t.img.word(take + o);
        let pool = 0x5900_0000 + own;
        let start = 20u32;
        t.set_u32(tw(&t, 0x37), pool + 0x74 * start);
        let list = w(&t, 0x1e6);
        let task = SCRATCH + 0x3200;
        let saved = t.save();
        // (times, countdowns, the first record listed, the busy record)
        let cases = [
            ([0.25f32, 0.5], [3u32, 2], false, None),
            ([0.875, 0.75], [0, 5], true, Some(2u32)),
            ([0.5, 0.95], [1, 0], false, Some(0)),
        ];
        for (c, (times, counts, listed, busy)) in (0u32..).zip(cases) {
            t.restore(&saved);
            t.write(obj, &vec![0x21; (w(&t, 0x2d) + 0x400) as usize]);
            for i in 0..2u32 {
                let r = recs + 0x190 * i;
                for j in 0..16u32 {
                    let x = ((j * 7 + i * 3 + c) % 11) as f32 * 0.5 - 2.0;
                    t.set_u32(r + 0x130 + 4 * j, x.to_bits());
                }
                t.set_u32(r + 0x180, times[i as usize].to_bits());
                t.set_u32(r + 0x184, 0.125f32.to_bits());
                t.set_u32(r + 0x188, counts[i as usize]);
                t.set_u32(r + 0x124, 0xffff_8123);
            }
            let lnode = SCRATCH + 0x3300;
            t.write(lnode, &[0; 0x10]);
            t.set_u32(lnode + 8, if listed { recs } else { 0x1234 });
            t.set_u32(list, lnode);
            t.set_u32(list + 4, lnode);
            t.set_u32(list + 8, 0);
            t.set_u32(list + 0xc, 1);
            t.set_u32(tw(&t, 0x08), start.wrapping_neg());
            t.set_u32(tw(&t, 0x0f), 1);
            t.write(pool, &vec![0x33; 0x74 * 10]);
            for i in 0..10 {
                t.set_u32(pool + 0x74 * i + 0x6c, u32::from(Some(i) == busy));
            }
            // the task at rest for the pool's give-back
            rest_node(&mut t, task);
            t.check_entry(f, regs(rva, c), &[task], Eax::Checked);
        }
    }
}

const ACTOR_SPAWNER: &[(u16, u32)] = &[(565, 0x5bd0), (644, 0x9a00)];

/// `o6_fam_pairs6::actor_spawner`: still waiting; the mode 0 (the start
/// turned and moved, the target pushed by slot 314) and 1 (slot 237's
/// start); not the last actor and the last (the task marked); the busy
/// list empty or holding a node; slot 324 answering a positive and a
/// negative roll. Slot 262 writes the target, 229 transforms and 248 adds
/// the vectors they are handed, 314 answers floats from its word.
#[test]
fn fam2_actor_spawner() {
    for (d, (n, rva)) in (0u32..).zip(copies("o6 fam pairs6 actor_spawner", ACTOR_SPAWNER)) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 1.5 - 0.25);
        t.stub(237, |m, a| {
            for i in 0..4 {
                m.w32(a[3] + 4 * i, (2.5 - i as f32).to_bits());
            }
            0x237
        });
        t.stub(262, |m, a| {
            for (i, &p) in a[1..4].iter().enumerate() {
                let x = ((a[0] ^ p) % 23) as f32 * 9.75 - 108.0 + i as f32;
                m.w32(p, x.to_bits());
            }
            0x262
        });
        t.stub(229, |m, a| {
            for i in 0..4 {
                let x = f32::from_bits(m.r32(a[2] + 4 * i)) * 1.25 + 0.5;
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x229
        });
        t.stub(248, |m, a| {
            for i in 0..4 {
                let x = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x248
        });
        t.stub(324, |m, _| m.r32(SCRATCH + 0x3900));
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let obj = 0x7000_0000 + 0x0100_0000 * d;
        t.set_u32(w(&t, 0x1a), obj);
        let a = obj + w(&t, 0x22);
        let list = SCRATCH + 0x3000;
        t.set_u32(w(&t, 0x1de), SCRATCH + 0x3100);
        t.write(SCRATCH + 0x3100, &[1, 2, 3, 0, 2, 1, 3, 0]);
        let take = t.img.rel32_target(f + 0x46d);
        let (count, head) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
        let task = SCRATCH + 0x3200;
        let saved = t.save();
        // (wait, mode, spawned, of, busy list held, roll)
        let cases = [
            (2u32, 0u32, 0u32, 3u32, false, 5u32),
            (0, 0, 0, 3, false, 13),
            (0, 1, 2, 3, true, 0xffff_fff3),
            (0, 1, 0, 3, true, 7),
        ];
        for (c, (wait, mode, spawned, of, busy, roll)) in (0u32..).zip(cases) {
            t.restore(&saved);
            t.write(obj, &vec![0x11; 0x10000]);
            t.set_u32(obj + w(&t, 0x1b8), list);
            t.write(list, &[0x22; 0x20]);
            for i in 0..8 {
                t.set_u8(list + 8 + i, (i * 3 + c) as u8 % 6);
            }
            for (i, o) in [0x5au32, 0x68, 0x76, 0x84].into_iter().enumerate() {
                t.set_u32(obj + w(&t, o), (1.25 * i as f32 - 0.5 + c as f32).to_bits());
            }
            t.set_u32(obj + w(&t, 0xd1), 0.375f32.to_bits());
            t.set_u32(a + 0x5200, wait);
            t.set_u32(a + 0x5204, spawned);
            t.set_u32(a + 0x5208, mode);
            t.set_u32(a + 0x520c, of);
            t.set_u32(SCRATCH + 0x3900, roll);
            let ring = |i: u32| SCRATCH + 0x3600 + 0x40 * i;
            for i in 0..3 {
                t.write(ring(i), &[0x77; 0x20]);
                t.set_u32(ring(i), ring((i + 2) % 3));
                t.set_u32(ring(i) + 4, ring((i + 1) % 3));
            }
            t.set_u16(count, 3);
            t.set_u32(head, ring(1));
            let busy_node = SCRATCH + 0x3800;
            t.write(busy_node, &[0x66; 0x20]);
            t.set_u32(w(&t, 0x473), if busy { busy_node } else { 0 });
            t.set_u32(w(&t, 0x486), busy_node);
            t.write(task, &[0x44; 0x60]);
            t.check_entry(f, regs(rva, c), &[task], Eax::Checked);
        }
    }
}

const STAGED_NODES: &[(u16, u32)] = &[(398, 0x6960), (402, 0x6960)];

/// `o6_fam_pairs6::staged_nodes`: every state (0, an idle one, 10, 11
/// before and past its counter, 12 before 20, with more indices and the
/// last, 13 before and past 80, 10000, a large one); the parent's list
/// empty or holding a child. Slot 218 gives one model, 941 a program
/// table; the node set-ups run as ported.
#[test]
#[allow(clippy::too_many_lines)] // the set-up of seven callees
fn fam2_staged_nodes() {
    for (n, rva) in copies("o6 fam pairs6 staged_nodes", STAGED_NODES) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        t.stub(218, |m, a| {
            m.w32(a[1], 1);
            m.w32(SCRATCH + 0x3880, a[0]);
            SCRATCH + 0x3890
        });
        t.write(SCRATCH + 0x3890, &[7, 0, 0, 0]);
        // the model's time (model_node_init stores slot 333's ST0)
        t.stub_st0(333, |_, a| f64::from(a[0] & 0xff) * 0.5);
        // the colour block's float (slot 790) and the node's (289)
        t.stub_st0(790, |_, _| 0.625);
        t.stub_st0(289, |_, a| f64::from(a[0] & 0xf) * 0.25);
        fill(&mut t, P, 0x40, 17);
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x3884, a[0]);
            P
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let parent = w(&t, 0x162);
        t.set_u32(w(&t, 0x2f0), SCRATCH + 0x3a00);
        fill(&mut t, SCRATCH + 0x3a00, 0x10, 5);
        let (task, s, link) = (SCRATCH + 0x3000, SCRATCH + 0x3100, SCRATCH + 0x3200);
        let saved = t.save();
        // (state, counter, index, the index count, the parent holding a child)
        let cases = [
            (0u32, 0u32, 0u32, 2u32, false),
            (5, 3, 0, 2, false),
            (10, 7, 1, 2, true),
            (11, 0x10, 1, 2, false),
            (11, 0x60, 1, 2, true),
            (12, 0x10, 0, 2, false),
            (12, 0x30, 0, 2, false),
            (12, 0x30, 1, 2, false),
            (13, 0x20, 0, 2, false),
            (13, 0x60, 0, 2, false),
            (0x2710, 0, 0, 2, false),
            (0x3000, 0, 0, 2, false),
        ];
        for (c, (state, counter, index, count, held)) in (0u32..).zip(cases) {
            t.restore(&saved);
            t.write(s, &[0x31; 0x40]);
            t.set_u32(s + 0x18, state);
            t.set_u32(s + 0x28, counter);
            t.set_u32(s + 0x24, index);
            t.set_u32(s + 0x1c, 0.5f32.to_bits());
            t.set_u32(s + 0x20, 1.5f32.to_bits());
            t.set_u32(w(&t, 0x1ec), count);
            t.set_u32(task, link);
            t.set_u32(task + 4, s);
            let (prev, next) = (SCRATCH + 0x3280, SCRATCH + 0x32c0);
            t.write(link, &[0x66; 0x10]);
            t.set_u32(link + 4, prev);
            t.set_u32(link + 8, next);
            t.write(parent, &[0x55; 0x20]);
            let child = SCRATCH + 0x3300;
            t.write(child, &[0x44; 0x20]);
            t.set_u32(parent + 0x10, if held { child } else { parent });
            t.set_u16(parent + 0x18, u16::from(held));
            // the other nodes' parents: empty lists
            for o in [0x6bu32, 0x9f, 0x30d] {
                let l = w(&t, o);
                t.set_u32(l + 0x10, l);
            }
            t.check_entry(f, regs(rva, c), &[task], Eax::Checked);
        }
    }
}

const TRAIL_DRAW: &[(u16, u32)] = &[(446, 0x4e20), (544, 0x4ce0), (565, 0x6ea0), (697, 0x6ea0)];

/// `o6_fam_pairs6::trail_draw`: no sprites; a short trail; an unmarked
/// point on the way back (it stops); a zero-length segment (a sprite
/// skipped); the colours' alpha 0 (nothing drawn); a long step that runs
/// past 32 segments around the ring's end; a head at the ring's last
/// point. The new point lies half a segment past the head. Slot 234
/// writes the difference of its points, 248 the sum; slot 941 answers a
/// frame table, 772 a 64-bit handle; the matrix slots as in
/// [`matrix_stubs`]; the animation steps and the sprites draw or link as
/// ported.
#[test]
#[allow(clippy::too_many_lines)] // the stubs and seven cases
fn fam2_trail_draw() {
    for &(n, rva) in TRAIL_DRAW {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        // `magic_0565`'s copy (shared-9's `trail draw (0565)`) has no `M4`
        let flat = rva == 0x6ea0;
        registered(
            &t,
            rva,
            if flat {
                "trail draw (0565)"
            } else {
                "o6 fam pairs6 trail_draw"
            },
        );
        stubs4(&mut t);
        matrix_stubs(&mut t);
        t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
        let table = P;
        t.stub(941, move |m, a| {
            let k = m.r32(SCRATCH + 0x3f80);
            m.w32(SCRATCH + 0x3f84 + 4 * (k % 8), a[0]);
            m.w32(SCRATCH + 0x3f80, k + 1);
            table
        });
        t.stub(234, |m, a| {
            for i in 0..3 {
                let x = f32::from_bits(m.r32(a[1] + 4 * i)) - f32::from_bits(m.r32(a[2] + 4 * i));
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x234
        });
        t.stub(248, |m, a| {
            for i in 0..3 {
                let x = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x248
        });
        let f = t.at(rva);
        // the sprite's slots 774 and 778 hash the matrices `*G_M3` and
        // `*G_M4` and the colour (what the game draws with)
        let (gm3, gm4, gc) = if flat {
            (
                t.img.word(f + 0x61),
                t.img.word(f + 0x54),
                t.img.word(f + 0x236),
            )
        } else {
            (
                t.img.word(f + 0x57),
                t.img.word(f + 0x63),
                t.img.word(f + 0x236),
            )
        };
        for s in [774u32, 778] {
            t.stub(s, move |m, a| {
                let mut x = s ^ a[2];
                for g in [gm3, gm4] {
                    // (`G_M4` is 0 in the flat copy)
                    let at = m.r32(g);
                    if at == 0 {
                        continue;
                    }
                    for i in 0..16 {
                        x = x.rotate_left(5) ^ m.r32(at + 4 * i);
                    }
                }
                x ^= m.r32(gc).rotate_left(3);
                let k = m.r32(SCRATCH + 0x3fc0);
                m.w32(SCRATCH + 0x3fc4 + 4 * (k % 15), x);
                m.w32(SCRATCH + 0x3fc0, k + 1);
                x
            });
        }
        // the sprite call: `draw_frame` (0446) or `link_part` (0544), whose
        // lock global points at a scratch word
        let (call, lock_offs): (u32, &[u32]) = if n == 446 {
            (0x546, &[0x4e, 0x79, 0xa8, 0xc5, 0x114, 0x13a])
        } else if flat {
            (0x531, &[0x4e, 0x79, 0xa8, 0xc5, 0x114, 0x13a])
        } else {
            (0x55d, &[0x4f, 0x7a, 0xa9, 0xc6, 0x115, 0x13b])
        };
        let callee = t.img.rel32_target(f + call);
        for &off in lock_offs {
            let g = t.img.word(callee + off);
            t.set_u32(g, SCRATCH + 0x3f00);
        }
        let (o, basis, point) = (SCRATCH + 0x4000, SCRATCH + 0x4400, SCRATCH + 0x4480);
        let saved = t.save();
        // (count, head, unmarked, zero segment at, alpha, step, playing)
        let cases = [
            (0u32, 5u32, 99u32, 99u32, 0x80u8, 0.6f32, 1u32),
            (6, 5, 99, 99, 0x80, 0.6, 1),
            (12, 7, 3, 99, 0x40, 0.6, 0),
            (9, 10, 99, 8, 0x80, 0.6, 1),
            (5, 3, 99, 99, 0, 0.6, 1),
            (80, 2, 99, 99, 0x7f, 5.0, 1),
            (7, 31, 28, 99, 0x80, 0.9, 1),
        ];
        for (c, (count, head, gap, zero, alpha, step, playing)) in (0u32..).zip(cases) {
            t.restore(&saved);
            fill(&mut t, table, 0x20, 17 + c);
            t.set_u16(table + 6, 5);
            t.write(SCRATCH + 0x3f00, &[0x31; 0x40]);
            t.write(o, &[0x13; 0x300]);
            t.set_u32(o, 0x0041_0000 + c);
            t.set_u32(o + 4, playing);
            t.set_u16(o + 8, 2);
            t.set_u16(o + 0xa, 0x100 + 0x80 * c as u16);
            for i in 0..32u32 {
                // the point before `zero` repeats it: a zero-length segment
                let k = if i + 1 == zero { zero } else { i } as f32;
                let xyz = [
                    k * 1.5 + c as f32,
                    (k * 0.7).sin() * 2.0,
                    k * 0.25 - 3.0,
                    1.0,
                ];
                for (j, v) in (0u32..).zip(xyz) {
                    t.set_u32(o + 0xc + 16 * i + 4 * j, v.to_bits());
                }
                // the head's mark is set by the call
                t.set_u8(o + 0x22c + i, u8::from(i != gap && i != head));
            }
            t.write(o + 0x20c, &[0x20 + c as u8, 0x90, 0x10, alpha]);
            t.write(o + 0x210, &[0xe0, 0x30 + c as u8, 0x70, alpha]);
            t.set_u32(o + 0x214, 1.5f32.to_bits());
            t.set_u32(o + 0x218, 0.5f32.to_bits());
            t.set_u32(o + 0x21c, step.to_bits());
            t.set_u32(o + 0x220, 0x1234 + 0x111 * c);
            t.set_u32(o + 0x224, head);
            t.set_u32(o + 0x228, count);
            for i in 0..12u32 {
                let v = 0.5 + i as f32 * 0.25 - c as f32 * 0.125;
                t.set_u32(basis + 4 * i, v.to_bits());
            }
            // the new point half a step past the previous one
            let k = head as f32 - 0.5;
            let xyz = [
                k * 1.5 + c as f32,
                (k * 0.7).sin() * 2.0,
                k * 0.25 - 3.0,
                1.0,
            ];
            for (j, v) in (0u32..).zip(xyz) {
                t.set_u32(point + 4 * j, v.to_bits());
            }
            let call = Call {
                ecx: o,
                edx: basis,
                ..Call::default()
            };
            t.check(f, call, &[point], Eax::Checked, |g, img, fr| {
                o6_fam_pairs6::trail_draw(g, img, fr)
            });
        }
    }
}

const TRAIL_TASK: &[(u16, u32)] = &[(544, 0x2250), (555, 0x2250), (556, 0x2250)];

/// `o6_fam_pairs6::trail_task`: the timer running with each sparks mode
/// (0, 2, 3 and none); reaching 0 with the mode 0 (a child, then the
/// trail) and non-zero (the object leaves, in the host list or not);
/// below 0 with the trail drawn, not drawn, and run out. The sparks take
/// from a free pool, slots 15/16/17/277/314/404 answer floats; the
/// trail's slots as in [`fam2_trail_draw`].
#[test]
#[allow(clippy::too_many_lines)] // the stubs, the pools and eleven cases
fn fam2_trail_task() {
    for (d, (n, rva)) in (0u32..).zip(copies("o6 fam pairs6 trail_task", TRAIL_TASK)) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
        let table = P;
        t.stub(941, move |m, a| {
            let k = m.r32(SCRATCH + 0x3f80);
            m.w32(SCRATCH + 0x3f84 + 4 * (k % 8), a[0]);
            m.w32(SCRATCH + 0x3f80, k + 1);
            table
        });
        t.stub(234, |m, a| {
            for i in 0..3 {
                let x = f32::from_bits(m.r32(a[1] + 4 * i)) - f32::from_bits(m.r32(a[2] + 4 * i));
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x234
        });
        t.stub(248, |m, a| {
            for i in 0..3 {
                let x = f32::from_bits(m.r32(a[1] + 4 * i)) + f32::from_bits(m.r32(a[2] + 4 * i));
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x248
        });
        for s in [15u32, 16, 17, 277, 314, 404] {
            t.stub_st0(s, move |_, a| {
                f64::from(s) * 0.125 + f64::from(a.first().copied().unwrap_or(0) % 7)
            });
        }
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let trail_at = t.img.rel32_target(f + 0x25d);
        // the trail's draw slots hash the matrices and the colour
        let (gm3, gm4, gc) = (
            t.img.word(trail_at + 0x57),
            t.img.word(trail_at + 0x63),
            t.img.word(trail_at + 0x236),
        );
        for s in [774u32, 778] {
            t.stub(s, move |m, a| {
                let mut x = s ^ a[2];
                for g in [gm3, gm4] {
                    let at = m.r32(g);
                    for i in 0..16 {
                        x = x.rotate_left(5) ^ m.r32(at + 4 * i);
                    }
                }
                x ^= m.r32(gc).rotate_left(3);
                let k = m.r32(SCRATCH + 0x3fc0);
                m.w32(SCRATCH + 0x3fc4 + 4 * (k % 15), x);
                m.w32(SCRATCH + 0x3fc0, k + 1);
                x
            });
        }
        // the trail's sprite call: `draw_frame` (the 0446 form, `inc eax`
        // at +0x344) or `link_part`, whose lock global points at a word
        let (call, lock_offs): (u32, &[u32]) = if t.img.word(trail_at + 0x344) as u8 == 0x40 {
            (0x546, &[0x4e, 0x79, 0xa8, 0xc5, 0x114, 0x13a])
        } else {
            (0x55d, &[0x4f, 0x7a, 0xa9, 0xc6, 0x115, 0x13b])
        };
        let link = t.img.rel32_target(trail_at + call);
        for &off in lock_offs {
            let g = t.img.word(link + off);
            t.set_u32(g, SCRATCH + 0x3f00);
        }
        // the sparks' pools and globals
        let recs = SCRATCH + 0x5000;
        let (gf, r2) = (SCRATCH + 0x800, SCRATCH + 0x880);
        let mut takes = Vec::new();
        for (call, take_at, gf_at, r2_at) in [
            (0x10du32, 0x9u32, 0x150u32, &[0x190u32][..]),
            (0x103, 0x13, 0x14c, &[0x5, 0x18f][..]),
            (0xf9, 0x17, 0x5, &[0x27, 0x18c][..]),
        ] {
            let sp = t.img.rel32_target(f + call);
            let take = t.img.rel32_target(sp + take_at);
            t.set_u32(t.img.word(take + 0x37), recs);
            t.set_u32(t.img.word(take + 0x61), recs);
            t.set_u32(t.img.word(sp + gf_at), gf);
            for &o in r2_at {
                t.set_u32(t.img.word(sp + o), r2);
            }
            takes.push(take);
        }
        for i in 0..8 {
            t.set_u32(r2 + 4 * i, 0x0e00_0000 + i);
        }
        t.set_u8(gf, 0);
        let ww = 0x6800_0000 + 0x0100_0000 * d;
        t.set_u32(w(&t, 0x21), ww);
        t.set_u32(w(&t, 0x191), SCRATCH + 0x3480);
        let (task, o, node) = (SCRATCH + 0x3000, SCRATCH + 0x4000, SCRATCH + 0x3400);
        let trail = o + 0x2c0;
        let list = w(&t, 0x183);
        // the child's object start (as in `actor_object_start_in_every_copy`)
        let soa = t.img.rel32_target(t.img.rel32_target(f + 0x170) + 0x2d);
        t.set_u32(t.img.word(soa + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(soa + 0x5e), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        let pa = t.img.rel32_target(soa + 0x9d);
        let free = t.img.word(pa + 0x04);
        let nodes = t.img.word(pa + 0x27) - 8;
        let pool_tail = t.img.word(soa + 0xa8);
        let saved = t.save();
        // (timer, mode, trail count, in the host list)
        let cases = [
            (5u32, 1u32, 6u32, false),
            (5, 0, 6, false),
            (3, 2, 6, false),
            (7, 3, 6, false),
            (1, 0, 6, false),
            (1, 1, 6, true),
            (1, 4, 6, false),
            (0xffff_ffff, 0, 5, false),
            (0xffff_fffe, 1, 3, false),
            (0xffff_ffff, 0, 0, true),
            (0, 0, 4, false),
        ];
        for (c, (timer, mode, count, listed)) in (0u32..).zip(cases) {
            t.restore(&saved);
            for &take in &takes {
                t.set_u32(t.img.word(take + 0x08), 0);
                t.set_u32(t.img.word(take + 0x0f), 8);
                t.set_u32(t.img.word(take + 0x97), 8);
                t.set_u32(t.img.word(take + 0x2b), 0);
            }
            for i in 0..8 {
                t.write(recs + 0x74 * i, &[0x22; 0x74]);
                t.set_u32(recs + 0x74 * i + 0x6c, 0);
            }
            fill(&mut t, table, 0x20, 17 + c);
            t.set_u16(table + 6, 5);
            for i in 0..8 {
                t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
                t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
                t.set_u32(nodes + 12 * i + 8, 0);
            }
            t.set_u16(free, 3);
            t.set_u32(pool_tail, 0);
            t.write(SCRATCH + 0x3f00, &[0x31; 0x40]);
            t.write(SCRATCH + 0x3480, &[0x48; 0x10]);
            t.write(ww, &vec![0x11; 0x5c20]);
            t.set_u32(ww + 0x5c04, mode);
            t.set_u32(ww + 0x5c0c, 0x30 + c);
            // the task at rest for the pool's give-back
            rest_node(&mut t, task);
            t.write(task + 0x10, &[0x5a; 0x10]);
            t.set_u32(task + 0x14, o);
            // the object and its place, the timer
            t.write(o, &[0x13; 0x600]);
            for i in 0..12u32 {
                let x = 0.75 * i as f32 - 2.0 + c as f32 * 0.5;
                t.set_u32(o + 0x260 + 4 * i, x.to_bits());
            }
            for i in 0..4u32 {
                t.set_u32(o + 0x280 + 4 * i, (0.25 * i as f32 + 0.1).to_bits());
            }
            t.set_u32(o + 0x50c, timer);
            t.set_u32(o + 0x510, 10 + c);
            t.set_u32(o + 0x514, u32::MAX);
            // the trail
            t.set_u32(trail, 0x0041_0000 + c);
            t.set_u32(trail + 4, 1);
            t.set_u16(trail + 8, 2);
            t.set_u16(trail + 0xa, 0x100);
            for i in 0..32u32 {
                let k = i as f32;
                let xyz = [
                    k * 1.5 + c as f32,
                    (k * 0.7).sin() * 2.0,
                    k * 0.25 - 3.0,
                    1.0,
                ];
                for (j, v) in (0u32..).zip(xyz) {
                    t.set_u32(trail + 0xc + 16 * i + 4 * j, v.to_bits());
                }
                t.set_u8(trail + 0x22c + i, 1);
            }
            t.write(trail + 0x20c, &[0x20 + c as u8, 0x90, 0x10, 0x80]);
            t.write(trail + 0x210, &[0xe0, 0x30, 0x70, 0x60]);
            t.set_u32(trail + 0x214, 1.5f32.to_bits());
            t.set_u32(trail + 0x218, 0.5f32.to_bits());
            t.set_u32(trail + 0x21c, 0.6f32.to_bits());
            t.set_u32(trail + 0x220, 0x1234 + 0x111 * c);
            t.set_u32(trail + 0x224, 5);
            t.set_u32(trail + 0x228, count);
            // the host list: empty or holding the object
            t.write(node, &[0x3c; 0x10]);
            t.set_u32(node, 0);
            t.set_u32(node + 4, 0);
            t.set_u32(node + 8, o);
            t.set_u32(list, if listed { node } else { 0 });
            t.set_u32(list + 4, if listed { node } else { 0 });
            t.set_u32(list + 8, 0);
            t.check_entry(f, regs(rva, c), &[task], Eax::Checked);
        }
    }
}

/// The trail's matrix slots write what they are handed: 235 a unit
/// matrix, 330 the matrix turned by its float (a shear), 229 a scaled
/// point plus a row of `M2`, 230 a copy; slot 773 answers 0 every other
/// call (the sprite registered the other way).
fn matrix_stubs(t: &mut Twin) {
    t.stub(235, |m, a| {
        for i in 0..16u32 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0x235
    });
    t.stub(330, |m, a| {
        let s = f32::from_bits(a[2]);
        for i in 0..16u32 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) + s * (i as f32 + 1.0) * 0.01;
            m.w32(a[0] + 4 * i, x.to_bits());
        }
        0x330
    });
    t.stub(229, |m, a| {
        for i in 0..4u32 {
            let x = f32::from_bits(m.r32(a[2] + 4 * i)) * 1.5 + f32::from_bits(m.r32(a[1] + 4 * i));
            m.w32(a[0] + 4 * i, x.to_bits());
        }
        0x229
    });
    t.stub(230, |m, a| {
        for i in 0..16u32 {
            let x = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, x);
        }
        0x230
    });
    t.stub(773, |m, a| {
        let k = m.r32(SCRATCH + 0x3fb0);
        m.w32(SCRATCH + 0x3fb0, k + 1);
        if k % 2 == 0 { 0x0773_0000 ^ a[0] } else { 0 }
    });
}

const PACKED_SPRITE: &[(u16, u32)] = &[(353, 0x13b0), (672, 0x1310), (266, 0x1310)];

/// `o6_fam_pairs6::packed_sprite`: descriptors with their fields spread
/// (the shifted bits from 0 to 15), sizes positive and negative (the
/// division toward 0); slot 885 answering 0 (nothing more) or not. Slot
/// 885 fills its buffer from its words and the lock; 905 hashes the
/// buffer and the colour words it is handed.
#[test]
fn fam2_packed_sprite() {
    for &(n, rva) in PACKED_SPRITE {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pairs6 packed_sprite");
        stubs4(&mut t);
        let f = t.at(rva);
        let lock_at = t.img.word(f + if n == 266 { 0x51 } else { 0x3c });
        let lock = SCRATCH + 0x3f00;
        t.set_u32(lock_at, lock);
        t.stub(885, move |m, a| {
            let mut x = 0x885 ^ m.r32(lock).rotate_left(13);
            for (i, w) in (0u32..).zip(&a[..6]) {
                x = x.rotate_left(7) ^ *w;
                m.w32(a[6] + 4 * i, x);
            }
            m.w32(SCRATCH + 0x3f10, x);
            if m.r32(SCRATCH + 0x3f20) == 0 {
                0
            } else {
                x | 1
            }
        });
        t.stub(905, |m, a| {
            let mut x = 0x905u32;
            for (i, w) in (0u32..).zip(&a[..9]) {
                x = x.rotate_left(5) ^ *w ^ i;
            }
            for i in 0..6 {
                x = x.rotate_left(3) ^ m.r32(a[1] + 4 * i);
            }
            for i in 0..4 {
                x = x.rotate_left(3) ^ m.r32(a[7] + 4 * i);
            }
            m.w32(SCRATCH + 0x3f14, x);
            x
        });
        let (owner, buffer) = (SCRATCH + 0x4000, SCRATCH + 0x5000);
        let saved = t.save();
        // (lo, hi, n, slot 885 answers)
        let cases = [
            (0x1234_5678u32, 0x0000_0002u32, 7u32, 1u32),
            (0xfedc_ba98, 0xffff_fff1, 0xffff_fffd, 1),
            (0x4c00_3fff, 0x0000_0003, 0x0100_0001, 1),
            (0x0000_0000, 0x0000_0000, 0, 1),
            (0x8765_4321, 0x0000_000c, 5, 0),
        ];
        for (c, (lo, hi, size, answers)) in (0u32..).zip(cases) {
            t.restore(&saved);
            t.write(owner, &[0x2d; 0x840]);
            t.set_u32(owner + 0x830, 0x0083_0000 + c);
            t.write(buffer, &[0x5c; 0x20]);
            t.set_u32(lock, 0x55 + c);
            t.set_u32(SCRATCH + 0x3f20, answers);
            let call = Call {
                ecx: owner,
                ..Call::default()
            };
            let args = [
                0x00a0_0000 + c,
                buffer,
                lo,
                hi,
                (1.5f32 + c as f32).to_bits(),
                (-0.25f32 * c as f32).to_bits(),
                size,
            ];
            t.check(f, call, &args, Eax::Checked, |g, img, fr| {
                o6_fam_pairs6::packed_sprite(g, img, fr)
            });
        }
    }
}

const BASIS_START: &[(u16, u32)] = &[(666, 0x1900), (664, 0x18d0), (665, 0x1b10)];

/// `o6_fam_pairs6::basis_start`: an object inside, just past and far
/// from the 0x80 bytes `magic_0664` tests (its kind), several scales
/// (`magic_0665`'s `K` negative too), slot 712 answering 0 and not. The vector slots write what they are handed: 236 three rows
/// from its words, 232 a half of its input, 325 a shuffled copy, 266 a
/// cross product, 235 a unit matrix, 231 a sum, 230 a copy; the matrix
/// globals point at scratch matrices.
#[test]
#[allow(clippy::too_many_lines)] // the vector stubs and their globals
fn fam2_basis_start() {
    for (n, rva) in copies("o6 fam pairs6 basis_start", BASIS_START) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        let rd = |m: &dyn magic::StubMem, at: u32| f32::from_bits(m.r32(at));
        t.stub(236, move |m, a| {
            for i in 0..12u32 {
                let x = (a[0] ^ a[1]).wrapping_add(i * 0x10_0001) % 97;
                m.w32(a[2] + 4 * i, (x as f32 * 0.125 - 5.0).to_bits());
            }
            0x236
        });
        t.stub(232, move |m, a| {
            // the input's bits hashed too (a zero's sign shows)
            let mut h = m.r32(SCRATCH + 0x3f30);
            for i in 0..4u32 {
                h = h.rotate_left(5) ^ m.r32(a[1] + 4 * i);
                let x = rd(m, a[1] + 4 * i) * 0.5 + 0.25;
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            m.w32(SCRATCH + 0x3f30, h);
            0x232
        });
        t.stub(325, move |m, a| {
            let v: Vec<f32> = (0..4u32).map(|i| rd(m, a[1] + 4 * i)).collect();
            for (i, x) in (0u32..).zip([v[1], v[2], v[0], v[3] + 1.0]) {
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x325
        });
        t.stub(266, move |m, a| {
            let p: Vec<f32> = (0..3u32).map(|i| rd(m, a[1] + 4 * i)).collect();
            let q: Vec<f32> = (0..3u32).map(|i| rd(m, a[2] + 4 * i)).collect();
            let c = [
                p[1] * q[2] - p[2] * q[1],
                p[2] * q[0] - p[0] * q[2],
                p[0] * q[1] - p[1] * q[0],
            ];
            for (i, x) in (0u32..).zip(c) {
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x266
        });
        t.stub(235, |m, a| {
            for i in 0..16u32 {
                m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
            }
            0x235
        });
        t.stub(231, move |m, a| {
            for i in 0..16u32 {
                let x = rd(m, a[1] + 4 * i) * 0.75 + rd(m, a[2] + 4 * i);
                m.w32(a[0] + 4 * i, x.to_bits());
            }
            0x231
        });
        t.stub(230, |m, a| {
            for i in 0..16u32 {
                let x = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, x);
            }
            0x230
        });
        t.stub(712, |m, a| {
            // the lock held while it runs
            let mut x = 0x712 ^ a[0] ^ a[1] ^ m.r32(SCRATCH + 0x3f04).rotate_left(9);
            for i in 0..16u32 {
                x = x.rotate_left(5) ^ m.r32(SCRATCH + 0x3400 + 4 * i);
            }
            m.w32(SCRATCH + 0x3f10, x);
            if m.r32(SCRATCH + 0x3f20) == 0 {
                0
            } else {
                x | 1
            }
        });
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        // the globals: (G2, G3, G4, G5) matrices, G6's word, the lock
        let (g1, tail, base) = match n {
            664 => (
                0x46u32,
                [0x255u32, 0x26e, 0x27a, 0x28d, 0x29a, 0x2a7],
                Some(w(&t, 0x1b)),
            ),
            665 => (0x28, [0x26c, 0x285, 0x291, 0x2ad, 0x2b4, 0x2bf], None),
            _ => (0x28, [0x23a, 0x253, 0x25f, 0x27b, 0x282, 0x28f], None),
        };
        t.set_u32(w(&t, g1), 0x0071_0000 + u32::from(n));
        for (k, &o) in tail[..4].iter().enumerate() {
            t.set_u32(w(&t, o), SCRATCH + 0x3400 + 0x40 * k as u32);
        }
        t.set_u32(w(&t, tail[4]), SCRATCH + 0x3f00);
        t.set_u32(w(&t, tail[5]), SCRATCH + 0x3f04);
        let saved = t.save();
        // (offset from 0664's base or scratch, scale, slot 712 answers,
        // 0665's `K`)
        let cases = [
            (None, 1.5f32, 1u32, 0.5f32),
            (Some(0x10u32), -0.75, 1, -1.25),
            (None, 2.0, 0, 3.0),
            (Some(0x10), 0.5, 0, 0.25),
            (Some(0x90), 1.25, 1, -0.5),
        ];
        for (c, (off, scale, answers, k)) in (0u32..).zip(cases) {
            t.restore(&saved);
            let p = match (off, base) {
                (Some(o), Some(b)) => b + o,
                _ => SCRATCH + 0x3000,
            };
            if n == 665 {
                t.set_u32(w(&t, 0x94), k.to_bits());
            }
            t.write(p, &[0x3c; 0x70]);
            t.set_u32(p + 0x60, scale.to_bits());
            for k in 0..4u32 {
                fill(&mut t, SCRATCH + 0x3400 + 0x40 * k, 0x10, 31 + k + c);
            }
            t.set_u32(SCRATCH + 0x3f00, 0x66);
            t.set_u32(SCRATCH + 0x3f04, 0x77 + c);
            t.set_u32(SCRATCH + 0x3f20, answers);
            t.check_entry(f, regs(rva, c), &[p], Eax::Checked);
        }
    }
}

const OVERLAY_SETUP: &[(u16, u32)] = &[(673, 0x1000), (253, 0x1000)];

/// `o6_fam_pairs6::overlay_setup`: two arguments, slot 790 answering two
/// floats, the cleared table and counts dirty before. The task ring and
/// (0253) the object block are set up as ported.
#[test]
fn fam2_overlay_setup() {
    for (n, rva) in copies("o6 fam pairs6 overlay_setup", OVERLAY_SETUP) {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs4(&mut t);
        t.stub_st0(790, |m, _| {
            let k = m.r32(SCRATCH + 0x3f40);
            m.w32(SCRATCH + 0x3f40, k + 1);
            1.25 + f64::from(k)
        });
        let f = t.at(rva);
        let block = n == 253;
        let sh = |o: u32| match (block, o < 0x84) {
            (false, _) => o,
            (true, true) => o + 6,
            (true, false) => o + 0xc,
        };
        let w = |t: &Twin, o: u32| t.img.word(f + sh(o));
        let saved = t.save();
        for c in 0..2u32 {
            t.restore(&saved);
            let arg = SCRATCH + 0x3000 + 0x100 * c;
            t.write(arg, &[0x4a; 0x40]);
            // the table and the counts dirty before
            let table = w(&t, 0x7b);
            for i in 0..0x3ffu32 {
                t.set_u32(table + 0xc * i, 0x5a5a_0000 + i + c);
            }
            t.set_u16(w(&t, 0x97), 7);
            t.set_u16(w(&t, 0x9f), 9);
            t.set_u32(w(&t, 0x104), 0x55);
            for o in [0x51u32, 0x5b, 0x65] {
                t.set_u32(w(&t, o), 0x4000_0000 + c);
            }
            t.check_entry(f, regs(rva, c), &[arg], Eax::Checked);
        }
    }
}

const SCENE_END: &[(u16, u32)] = &[(610, 0xab20), (434, 0x5b90)];

/// `o6_fam_pairs6::scene_end`: one and three actors; the skip flag set
/// (only the parts) and clear; the flag `*G_F` and the index `*G_I`
/// letting slots 217/255 run or not; the halfword `*G_H` 0 or not; a held
/// node or none. Slot 288 answers an empty list (the party and the
/// others run through no actors).
#[test]
fn fam2_scene_end() {
    for &(n, rva) in SCENE_END {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        registered(&t, rva, "o6 fam pairs6 scene_end");
        stubs4(&mut t);
        t.stub(288, |m, a| {
            m.w32(a[1], 0);
            0x288
        });
        let f = t.at(rva);
        let early = n == 434;
        let sh = |o: u32| match (early, o) {
            (false, _) | (true, ..0x96) => o,
            (true, 0x96..0x9b) => o + 5,
            (true, 0x9b..0x15c) => o + 0xf,
            (true, _) => o + 0xa,
        };
        let w = |t: &Twin, o: u32| t.img.word(f + sh(o));
        let ids = SCRATCH + 0x3000;
        t.write(ids, &[3, 1, 4, 1, 5, 2, 6, 0]);
        t.set_u32(w(&t, 0x11), ids);
        t.set_u32(w(&t, 0x6d), ids);
        let s = SCRATCH + 0x4000;
        t.set_u32(w(&t, 0x97), s);
        let skip_at = s + w(&t, 0x9d);
        let (node, done) = (SCRATCH + 0x3100, SCRATCH + 0x3200);
        t.set_u32(w(&t, 0x15d), done);
        if early {
            // the global 0434 clears after its early parts
            t.set_u32(t.img.word(f + 0xa2), 0x77);
        }
        let saved = t.save();
        // (actors, skip, *G_F, index, *G_H, a held node)
        let cases = [
            (1u32, 1u32, 0u32, 3u32, 0u16, false),
            (3, 0, 0, 3, 0, true),
            (3, 0, 0, 0x20, 1, false),
            (2, 0, 1, 3, 0, true),
            (3, 0, 0, 0xffff_fffe, 2, false),
        ];
        for (c, (actors, skip, flag, index, half, held)) in (0u32..).zip(cases) {
            t.restore(&saved);
            t.set_u32(w(&t, 0x59), actors);
            t.set_u32(skip_at, skip);
            t.set_u32(w(&t, 0xb5), flag);
            t.set_u32(w(&t, 0xc4), index);
            t.set_u32(w(&t, 0xbc), 0x00bc_0000 + c);
            t.set_u16(w(&t, 0x10a), half);
            t.set_u32(w(&t, 0x11b), 0x0011_b000 + c);
            t.set_u32(w(&t, 0x131), 0x31);
            rest_node(&mut t, node);
            t.set_u32(w(&t, 0x140), if held { node } else { 0 });
            t.set_u32(done, 0x55 + c);
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                o6_fam_pairs6::scene_end(g, img, fr)
            });
        }
    }
}
