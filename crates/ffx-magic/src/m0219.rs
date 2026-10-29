//! `magic_0219.dll`'s own functions on top of the object host
//! ([`crate::l5_objhost`]): the per-frame entry (the table's `+0x10`)
//! that waits for the effect's data, then creates the effect's one object
//! and installs the draw and update callbacks. Each is shared with one
//! other overlay.

use crate::l5_objhost::{self as host, ONE};
use crate::opu_parts;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Table `+0x10` (0x10001210, no arguments): nothing while slot 225
/// `(1)` is non-zero; the first time, slot 226 `(G_ARG, 1)`; once slot
/// 227 `(G_ARG)` reports the data loaded (1 or more), the effect starts
/// ([`boot`]) and the table's `+0xc` and `+0x14` become the draw and
/// update callbacks. Returns 0.
pub fn per_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s225 = slot_target(g, w(0x01), 225);
    if g.call(s225, 0, &[1]).eax != 0 {
        return Ret::eax(0);
    }
    if g.read::<u32>(w(0x18)) == 0 {
        let s226 = slot_target(g, w(0x1f), 226);
        let arg = g.read::<u32>(w(0x27));
        g.call(s226, 0, &[arg, 1]);
        g.write::<u32>(w(0x38), 1);
    }
    let s227 = slot_target(g, w(0x41), 227);
    let arg = g.read::<u32>(w(0x47));
    if g.call(s227, 0, &[arg]).eax.cast_signed() >= 1 {
        boot(g, img, img.rel32_target(f + 0x5c));
        g.write::<u32>(w(0x62), w(0x66));
        g.write::<u32>(w(0x6c), w(0x70));
    }
    Ret::eax(0)
}

/// `per_frame` (119 bytes, 2 overlays).
pub const PER_FRAME: Sig = Sig {
    len: 119,
    hash: 0x2da6_1889_6875_2369,
    calls: &[(0x5c, Callee::Sig(&BOOT))],
    imports: &[],
};

/// Starts the effect (0x10001370, no arguments): the part's entry (slot
/// 719 `(data, 8, 0)`) into `G_PART`, its buffers, the part set up
/// ([`host::part_init`]), the object made ([`make_object`]), the
/// character's two registrations (slots 262 and 268 of the battle
/// character the control block's first byte selects), the effect's
/// state block cleared, and a task appended to the task list. Returns 0.
pub fn boot<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let s719 = slot_target(g, w(0x01), 719);
    g.write::<u32>(w(0x17), w(0x1b));
    g.write::<u32>(w(0x21), w(0x25));
    let part = g.call(s719, 0, &[w(0x11), 8, 0]).eax;
    g.write::<u32>(w(0x2f), part);
    host::part_init(g, img, img.rel32_target(f + 0x34));
    // movss xmm3, [G_SCALE0]; mov edx, G_OBJ
    let scale = g.read::<u32>(w(0x3c));
    make_object(g, img, img.rel32_target(f + 0x49), w(0x44), scale);
    let sel = g.read::<u32>(w(0x4e));
    let k = u32::from(g.read::<u8>(sel));
    let chr = g.read::<u32>(w(0x62).wrapping_add(4 * k));
    let s262 = slot_target(g, w(0x67), 262);
    let a = [chr, w(0x6c), w(0x5b), w(0x53)];
    g.call(s262, 0, &a);
    let s268 = slot_target(g, w(0x7a), 268);
    let mut args = vec![chr, 0x11, w(0x7f)];
    args.extend(a);
    // call width: 7 (the words this call leaves on the stack at most)
    g.call(s268, 0, &args);
    g.write::<u32>(w(0x95), ONE);
    for off in [0x9f, 0xa9, 0xb3, 0xbd] {
        g.write::<u32>(w(off), 0);
    }
    g.write::<u32>(w(0xc7), w(0xcb));
    g.write::<u32>(w(0xd1), w(0xd5));
    g.write::<u32>(w(0xdb), 0);
    g.write::<u32>(w(0xe5), 0);
    g.write::<u16>(w(0xef), 0);
    g.write::<u32>(w(0xf5), w(0xf9));
    let task = host::task_alloc(g, img, img.rel32_target(f + 0xfe));
    let head = g.read::<u32>(w(0x104));
    g.write::<u32>(task + 4, 0);
    if head == 0 {
        g.write::<u32>(task, 0);
        g.write::<u32>(w(0x130), task);
    } else {
        let tail = g.read::<u32>(w(0x115));
        g.write::<u32>(task, tail);
        g.write::<u32>(tail + 4, task);
    }
    g.write::<u32>(task + 8, task);
    g.write::<u32>(w(0x122), task);
    0
}

/// `boot` (319 bytes, 2 overlays).
pub const BOOT: Sig = Sig {
    len: 319,
    hash: 0x20ef_39c3_853f_8f8c,
    calls: &[
        (0x34, Callee::Sig(&host::PART_INIT)),
        (0x49, Callee::Sig(&MAKE_OBJECT)),
        (0xfe, Callee::Sig(&host::TASK_ALLOC)),
    ],
    imports: &[],
};

/// Makes the effect's object at `obj` (0x10001b30, `edx` = the object,
/// `xmm3` = the scale's bits): position and rotation 0 with `w` = 1, the
/// scale in `+0x60..+0x6c`, the game's draw context, the part started
/// (`pppStartPart(obj, 0x1000, *slot941(G_PART +0x10), 1)` through the
/// wrapper), the fields ([`object_init`], the finish callback at
/// `+0x85`), and appended to the object list. Returns `obj`.
pub fn make_object<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, scale: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let ctx = g.read::<u32>(w(0x01));
    for (off, v) in [
        (0x48, 0),
        (0x44, 0),
        (0x40, 0),
        (0x4c, ONE),
        (0x5c, 0),
        (0x58, 0),
        (0x54, 0),
        (0x50, 0),
        (0x6c, scale),
        (0x68, scale),
        (0x64, scale),
        (0x60, scale),
    ] {
        g.write::<u32>(obj + off, v);
    }
    g.write::<u32>(ctx, w(0x56));
    let part = g.read::<u32>(w(0x5b));
    let a = g.read::<u32>(part + 0x10);
    let s941 = slot_target(g, w(0x63), 941);
    let p = g.call(s941, 0, &[a]).eax;
    let a0 = g.read::<u32>(p);
    opu_parts::start_part(g, img, img.rel32_target(f + 0x7d), obj, 0x1000, a0, 1);
    object_init(g, obj, w(0x85));
    let node = host::pool_alloc(g, img, img.rel32_target(f + 0x95));
    if node != 0 {
        let tail = g.read::<u32>(w(0xa0));
        g.write::<u32>(node + 4, 0);
        if tail == 0 {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(0xb3), node);
            g.write::<u32>(w(0xb9), node);
        } else {
            g.write::<u32>(node, tail);
            let tail = g.read::<u32>(w(0xcd));
            g.write::<u32>(tail + 4, node);
            g.write::<u32>(w(0xd6), node);
        }
        g.write::<u32>(node + 8, obj);
        let count = if tail == 0 { w(0xc2) } else { w(0xdf) };
        let n = g.read::<u32>(count);
        g.write::<u32>(count, n.wrapping_add(1));
    }
    obj
}

/// `make_object` (231 bytes, 2 overlays).
pub const MAKE_OBJECT: Sig = Sig {
    len: 231,
    hash: 0xa281_110d_314d_b0b4,
    calls: &[
        (0x7d, Callee::Sig(&opu_parts::START_PART)),
        (0x8d, Callee::Sig(&OBJECT_INIT)),
        (0x95, Callee::Sig(&host::POOL_ALLOC)),
    ],
    imports: &[],
};

/// An object's fields (0x10001a10, `ecx` = the object, the second stack
/// word = the finish callback): the callback at `+0x100`, the frame
/// counters and other callbacks 0 (`+0x11c` = 15), mode 0, flags 0x8000,
/// angles and translation 0, the scale words `+0xdc`/`+0xec` and the
/// colour `+0xf0..+0xfc` 1.0. Returns 0x8000.
pub fn object_init<G: Game>(g: &mut G, obj: u32, finish: u32) -> u32 {
    g.write::<u32>(obj + 0x100, finish);
    for (off, v) in [
        (0x114, 0),
        (0x11c, 0xf),
        (0x104, 0),
        (0x108, 0),
        (0x10c, 0),
        (0x110, 0),
        (0x118, 0),
        (0x120, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    g.write::<u8>(obj + 0x126, 0);
    g.write::<u16>(obj + 0x124, 0x8000);
    for (off, v) in [
        (0xcc, 0),
        (0xc8, 0),
        (0xc4, 0),
        (0xc0, 0),
        (0xd8, 0),
        (0xd4, 0),
        (0xd0, 0),
        (0xdc, ONE),
        (0xe8, 0),
        (0xe4, 0),
        (0xe0, 0),
        (0xec, ONE),
        (0xfc, ONE),
        (0xf8, ONE),
        (0xf4, ONE),
        (0xf0, ONE),
    ] {
        g.write::<u32>(obj + off, v);
    }
    0x8000
}

/// `object_init` (273 bytes, 2 overlays).
pub const OBJECT_INIT: Sig = Sig {
    len: 273,
    hash: 0xe7c7_41a3_f5a8_48fe,
    calls: &[],
    imports: &[],
};
