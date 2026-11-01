//! The load-step family (`ffx_magic::r9_load`) against each copy's
//! original: slot 225 busy, the first and a later request, the data not
//! loaded yet and loaded (the DLL's start-up then runs on both sides, its
//! pools given free nodes). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;
mod magic_r11;
mod magic_r9;

use ffx_magic::{Image, l5_objhost, m0032, sa_pool};
use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

/// What the start-up's callees need, found by their `Sig`s (`e8`
/// targets from `at` on, `depth` calls deep): pool takes (`count`, `head`
/// operands) and part set-ups (three buffer pointers).
#[derive(Default)]
struct Needs {
    rings: Vec<(u32, u32)>,
    parts: Vec<u32>,
}

fn needs(img: &Image, at: u32, len: u32, depth: u32, out: &mut Needs) {
    for p in at..at + len {
        if img.word(p).to_le_bytes()[0] != 0xe8 {
            continue;
        }
        let t = img.rel32_target(p + 1);
        if !img.in_text(t) {
            continue;
        }
        let ring = if img.matches(t, &sa_pool::TAKE) {
            Some((img.word(t + 0x04), img.word(t + 0x17)))
        } else if img.matches(t, &m0032::POP_NODE) {
            Some((img.word(t + 0x04), img.word(t + 0x13)))
        } else {
            None
        };
        if let Some(r) = ring {
            if !out.rings.contains(&r) {
                out.rings.push(r);
            }
        } else if img.matches(t, &l5_objhost::PART_INIT) {
            if !out.parts.contains(&t) {
                out.parts.push(t);
            }
        } else if depth > 0 {
            needs(img, t, 0x200, depth - 1, out);
        }
    }
}

/// A free ring of 8 nodes at `at` whose head and count are `(count,
/// head)`.
fn ring(t: &mut Twin, (count, head): (u32, u32), at: u32) {
    let node = |i: u32| at + 0x40 * i;
    for i in 0..8 {
        t.write(node(i), &[0x5a; 0x40]);
        t.set_u32(node(i), node((i + 7) % 8));
        t.set_u32(node(i) + 4, node((i + 1) % 8));
    }
    t.set_u32(head, node(0));
    t.set_u16(count, 8);
}

/// Free nodes for each pool the start-up takes from, and buffers for its
/// part set-ups.
fn pools(t: &mut Twin, start: u32) {
    let mut n = Needs::default();
    needs(&t.img, start, 0x300, 1, &mut n);
    for (k, r) in (0u32..).zip(n.rings) {
        ring(t, r, SCRATCH + 0x8000 + 0x400 * k);
    }
    for (k, p) in (0u32..).zip(n.parts) {
        for (j, o) in (0u32..).zip([0x01u32, 0x07, 0x11]) {
            let at = t.img.word(p + o);
            t.set_u32(at, SCRATCH + 0x4000 + 0x200 * k + 0x40 * j);
        }
    }
}

/// One copy at `rva` (`one`: the 119-byte shape).
fn copy(n: u16, rva: u32, one: bool, extra: fn(&mut Twin, u32)) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let d = u32::from(one);
    let w = |t: &Twin, o: u32| t.img.word(at + o);
    let busy = SCRATCH + 0x100;
    let loaded = SCRATCH + 0x104;
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    // the start-up's own slots: a pointer to junk for whatever asks
    let data = SCRATCH + 0x2000;
    t.write(data, &[0x40; 0x400]);
    for s in [719u32, 718, 215, 413, 219, 941] {
        rec(&mut t, s, 1, move |_, _| data);
    }
    for s in [790u32, 12, 333, 314] {
        t.stub_st0(s, move |_, _| f64::from(s) * 0.25);
    }
    t.set_u32(w(&t, 0x26 + d), 0x0000_5e5e);
    let start = t.img.rel32_target(at + 0x5b + d);
    pools(&mut t, start);
    extra(&mut t, start);
    for (b, asked, l) in [
        (1u32, 0u32, 1u32),
        (0, 0, 0),
        (0, 1, 0xffff_ffff),
        (0, 0, 1),
        (0, 1, 2),
    ] {
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, 0x18), asked);
        for o in [0x61u32, 0x6b] {
            t.set_u32(w(&t, o + d), 0x0bad_0000 + o);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// The objects' start-up (`r9_misc::objects_start`): one actor, a
/// three free nodes.
fn objects(t: &mut Twin, start: u32) {
    magic_r9::objects_setup(t, start, 1, 3, false);
}

/// Nothing more to set up.
fn none(_: &mut Twin, _: u32) {}

/// `m0018::start`'s inline take (count `+0xf4`, head `+0x10e`).
fn inline_0018(t: &mut Twin, start: u32) {
    let r = (t.img.word(start + 0xf4), t.img.word(start + 0x10e));
    ring(t, r, SCRATCH + 0xa000);
}

/// `m0046::start_up`'s selector byte (`+0x4e`) and its callee's two
/// pointers.
fn sel_0046(t: &mut Twin, start: u32) {
    let so = t.img.rel32_target(start + 0x49);
    t.set_u32(t.img.word(so + 0x01), SCRATCH + 0x200);
    t.set_u32(t.img.word(so + 0x5b), SCRATCH + 0x300);
    let b = SCRATCH + 0x500;
    t.set_u32(t.img.word(start + 0x4e), b);
    t.write(b, &[2, 0x40]);
}

/// `m0390`'s facing: the selector byte.
fn sel_0390(t: &mut Twin, start: u32) {
    let face = t.img.rel32_target(start + 0x49);
    t.set_u32(t.img.word(face + 0x8c), SCRATCH + 0x180);
    t.set_u32(t.img.word(face + 0x08), SCRATCH + 0x5000);
}

#[test]
fn load_0018() {
    copy(18, 0x1210, false, inline_0018);
}

#[test]
fn load_0019() {
    copy(19, 0x3720, false, inline_0018);
}

#[test]
fn load_0046() {
    copy(46, 0x1210, true, sel_0046);
}

#[test]
fn load_0064() {
    copy(64, 0x26d0, false, none);
}

#[test]
fn load_0171() {
    copy(171, 0x1210, false, none);
}

#[test]
fn load_0367() {
    copy(367, 0x1210, false, none);
}

#[test]
fn load_0369() {
    copy(369, 0x1210, false, none);
}

#[test]
fn load_0383() {
    copy(383, 0x1210, false, none);
}

#[test]
fn load_0390() {
    copy(390, 0x12b0, false, sel_0390);
}

#[test]
fn load_0395() {
    copy(395, 0x3df0, false, none);
}

#[test]
fn load_0447() {
    copy(447, 0x49b0, false, sel_0390);
}

#[test]
fn load_0108() {
    copy(108, 0x1210, true, objects);
}

#[test]
fn load_0221() {
    copy(221, 0x1210, true, objects);
}

#[test]
fn load_0229() {
    copy(229, 0x1210, true, objects);
}

/// The one-object start-up (`r9_misc::effect_start`): three free nodes.
fn effect(t: &mut Twin, start: u32) {
    magic_r9::effect_setup(t, start, 2.0, 3, false);
}

#[test]
fn load_0166() {
    copy(166, 0x1210, false, effect);
}

#[test]
fn load_0308() {
    copy(308, 0x1210, false, effect);
}

/// Round 11's start-up with three task nodes
/// (`r11_objects::three_nodes`): two actors, enough free nodes for both
/// runs.
fn three_nodes(t: &mut Twin, start: u32) {
    magic_r11::three_nodes_setup(t, start, 2, 7, false);
}

#[test]
fn load_0110() {
    copy(110, 0x12b0, true, three_nodes);
}

#[test]
fn load_0112() {
    copy(112, 0x12b0, true, three_nodes);
}

#[test]
fn load_0121() {
    copy(121, 0x2d60, true, three_nodes);
}

/// Round 11's start-up with two objects per actor
/// (`r11_objects::two_per_actor`).
fn two_per_actor(t: &mut Twin, start: u32) {
    magic_r11::two_per_actor_setup(t, start, 2, 7, false);
}

/// Round 11's start-up with slot 12's heights
/// (`r11_objects::three_nodes_height`).
fn three_nodes_height(t: &mut Twin, start: u32) {
    magic_r11::three_nodes_height_setup(t, start, 2, 7, false);
}

#[test]
fn load_0120() {
    copy(120, 0x12b0, true, three_nodes_height);
}

/// Round 11's scaled start-ups (`r11_objects::three_nodes_kind`,
/// `three_nodes_scaled`).
fn three_nodes_kind(t: &mut Twin, start: u32) {
    magic_r11::three_nodes_kind_setup(t, start, 2, 7, false);
}

fn three_nodes_scaled(t: &mut Twin, start: u32) {
    magic_r11::three_nodes_scaled_setup(t, start, 2, 7, false);
}

#[test]
fn load_0126() {
    copy(126, 0x1300, true, three_nodes_kind);
}

#[test]
fn load_0127() {
    copy(127, 0x2bd0, true, three_nodes_scaled);
}

/// Round 11's start-up with an object and two fades
/// (`r11_fades::object_two_fades`).
fn object_two_fades(t: &mut Twin, start: u32) {
    magic_r11::object_two_fades_setup(t, start, false);
}

#[test]
fn load_0105() {
    copy(105, 0x2fd0, true, object_two_fades);
}

#[test]
fn load_0116() {
    copy(116, 0x1300, true, two_per_actor);
}

#[test]
fn load_0117() {
    copy(117, 0x2da0, true, two_per_actor);
}

#[test]
fn load_fades() {
    for (n, rva) in [
        (101u16, 0x2e70u32),
        (93, 0x3120),
        (85, 0x30a0),
        (103, 0x3220),
    ] {
        copy(n, rva, true, magic_r11::fades_row);
    }
}

#[test]
fn load_0252() {
    copy(252, 0x1210, false, |t, start| {
        magic_r11::effect_flat_setup(t, start, 1.0, false);
    });
}

#[test]
fn load_0365() {
    copy(365, 0x2ab0, true, |t, start| {
        magic_r11::effect_height_setup(t, start, 1.0, false);
    });
}

#[test]
fn load_0197() {
    copy(197, 0x1210, false, |t, start| {
        magic_r11::effect_lookup_setup(t, start, 1.0, false, false);
    });
}

#[test]
fn load_0065() {
    copy(65, 0x2e90, false, |t, start| {
        magic_r11::colour_start_setup(t, start, true, false);
    });
}

#[test]
fn load_0054() {
    copy(54, 0x1210, true, |t, start| {
        magic_r11::two_anim_setup(t, start, false);
    });
}

#[test]
fn load_0231() {
    copy(231, 0x1210, true, |t, start| {
        magic_r11::effect_kind_setup(t, start, 1.0, 0, false);
    });
}

#[test]
fn load_0233() {
    copy(233, 0x1210, true, |t, start| {
        magic_r11::sized_actors_setup(t, start, 2, 4, false);
    });
}

#[test]
fn load_0100() {
    copy(100, 0x12b0, true, |t, start| {
        magic_r11::fades_setup(
            t,
            start,
            668,
            &[0x5d, 0xe8],
            (0x62, 0x76),
            &[0x14e, 0x20a, 0x221],
            (0x25c, 0x272, false),
        );
        let sl = t.img.rel32_target(start + 0x3d);
        t.set_u32(t.img.word(sl + 0x01), magic::SCRATCH + 0x180);
        magic_owna::rec(t, 534, 1, |_, _| 0);
        t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.25);
    });
}

#[test]
fn load_0084() {
    copy(84, 0x1300, true, |t, start| {
        magic_r11::fades_setup(
            t,
            start,
            730,
            &[0x6b, 0x106],
            (0x70, 0x84),
            &[0x1e2, 0x1dc, 0x271],
            (0x298, 0x2aa, false),
        );
        let sl = t.img.rel32_target(start + 0x4b);
        magic_r11::state_slots(t, sl, false, 1.0);
    });
}

#[test]
fn load_0102() {
    copy(102, 0x1300, true, |t, start| {
        magic_r11::kinds_setup(t, start, true, false, false);
    });
}

#[test]
fn load_0274() {
    copy(274, 0x32d0, false, |t, start| {
        magic_r11::two_objects_setup(t, start, 1.0, false);
    });
}
