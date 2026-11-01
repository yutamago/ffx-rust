//! `magic_0171.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Image, Sig};

/// The actor block (0x10002a00, internal: `edx` = the block, cdecl
/// `(a)`): `{a, -1, *ME, model (slot 413 in a battle, else 215), -1, 0,
/// 1, u16 battle flag}` as [`me_block`](crate::sa_misc::me_block) fills
/// its static one (`ME` at `+0x13`, the battle word `+0x1c`/`+0x57`).
/// Returns the model in a battle, else 0.
pub fn actor_block<G: crate::Game>(g: &mut G, img: &Image, f: u32, block: u32, a: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(block, a);
    g.write::<u32>(block + 4, u32::MAX);
    let me = g.read::<u32>(w(0x13));
    g.write::<u32>(block + 8, me);
    let battle = g.read::<u32>(w(0x1c)) != 0;
    let t = slot_target(g, w(0x23), if battle { 413 } else { 215 });
    let m = g.call(t, 0, &[me]).eax;
    g.write::<u32>(block + 0xc, m);
    g.write::<u32>(block + 0x10, u32::MAX);
    g.write::<u32>(block + 0x14, 0);
    g.write::<u32>(block + 0x18, 1);
    if g.read::<u32>(w(0x57)) != 0 {
        g.write::<u16>(block + 0x1c, 1);
        m
    } else {
        g.write::<u16>(block + 0x1c, 0);
        0
    }
}

/// [`actor_block`] (110 bytes, 1 overlay).
pub const ACTOR_BLOCK: Sig = Sig {
    len: 110,
    hash: 0xba9b_9b12_0a67_bbed,
    calls: &[],
    imports: &[],
};

/// The angles from `from` to `to` (0x100039a0, `ecx` = `out`, `edx` =
/// `from`, cdecl `(to)`; [`m0396::aim`](crate::m0396::aim)'s kin): the
/// pitch `atan2(dy, sqrt(dz dz + dx dx))` into `out.x`; the yaw `y = A -
/// atan2(dz, dx)` (`A` at `+0xbc`) brought back by `B` (`+0xd5`/`+0xfb`)
/// once when above `H` (`+0xcb`) or below `L` (`+0xee`) into `out.y`;
/// `out.z` = 0. The square root is `_libm_sse2_sqrt_precise`, the arcs
/// `_CIatan2`, each rounded to `f32`. (`eax` is what `_CIatan2` leaves:
/// the port gives `to`.)
pub fn aim<G: crate::Game>(g: &mut G, img: &Image, f: u32, out: u32, from: u32, to: u32) -> u32 {
    use crate::sb_sse::{add, mul, sub};
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, p: u32, k: u32| g.read::<f32>(p + 4 * k);
    let dy = sub(r(g, to, 1), r(g, from, 1));
    let dx = sub(r(g, to, 0), r(g, from, 0));
    let dz = sub(r(g, to, 2), r(g, from, 2));
    let s = add(mul(dz, dz), mul(dx, dx));
    let h = f64::from(s).sqrt();
    #[allow(clippy::cast_possible_truncation)]
    let pitch = g.atan2(f64::from(dy), h) as f32;
    g.write::<f32>(out, pitch);
    #[allow(clippy::cast_possible_truncation)]
    let yaw = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let y = sub(g.read::<f32>(w(0xbc)), yaw);
    let y = if y > g.read::<f32>(w(0xcb)) {
        sub(y, g.read::<f32>(w(0xd5)))
    } else if g.read::<f32>(w(0xee)) > y {
        add(y, g.read::<f32>(w(0xfb)))
    } else {
        y
    };
    g.write::<f32>(out + 4, y);
    g.write::<u32>(out + 8, 0);
    to
}

/// [`aim`] (272 bytes, 2 overlays: also `magic_0396` 0x10003910, whose
/// Sig ran 66 bytes into the next function until census2, 2026-11-01).
pub const AIM: Sig = Sig {
    len: 272,
    hash: 0xfe5c_16a6_9bb7_6c27,
    calls: &[
        (0x5a, crate::Callee::Import("_libm_sse2_sqrt_precise")),
        (0x6d, crate::Callee::Import("_CIatan2")),
        (0xaa, crate::Callee::Import("_CIatan2")),
    ],
    imports: &[],
};

/// The start-up (0x10001340, no arguments): three words get their
/// blocks (`+0x1d`.. `+0x31`); slot 407 `(*G, NAME, 0)` (`G` `+0x08`,
/// `NAME` `+0x16`) gives a byte list for
/// [`obj_block_fill`](crate::l5_objhost::obj_block_fill) (`ecx` = `*G`,
/// `edx` = the block `+0x3d`); slot 719 `(F, 8, 0)` (`+0x58`) into `+0x62`;
/// the part's set-up ([`part_init`](crate::l5_objhost::part_init)); a
/// u16 and three words cleared and `+0x8c` = 1. Then three nodes from the
/// pool ([`take`](crate::sa_pool::take), owners the callbacks `+0x71`,
/// `+0xcc`, `+0x108`) go on the task list (first `+0x9b`, last `+0xb2`):
/// the first (byte `+0x18` = 1) after the old last without moving the
/// list's last word, the second right after it (the list's last), the
/// records block `+0xf6` set up
/// ([`records_init_100`](crate::sa_misc::records_init_100)), the third
/// (byte 2) at the end. Returns 0.
pub fn start_up<G: crate::Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> crate::Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s407 = slot_target(g, w(0x01), 0x65c / 4);
    let obj = g.read::<u32>(w(0x08));
    for (dst, val) in [(0x1d, 0x21), (0x27, 0x2b), (0x31, 0x35)] {
        g.write::<u32>(w(dst), w(val));
    }
    let bytes = g.call(s407, 0, &[obj, w(0x16), 0]).eax;
    crate::l5_objhost::obj_block_fill(g, img, img.rel32_target(f + 0x44), obj, w(0x3d), bytes);
    let s719 = slot_target(g, w(0x49), 0xb3c / 4);
    let h = g.call(s719, 0, &[w(0x58), 8, 0]).eax;
    g.write::<u32>(w(0x62), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x67));
    g.write::<u16>(w(0x77), 0);
    for at in [0x7c, 0x81, 0x86] {
        g.write::<u32>(w(at), 0);
    }
    g.write::<u32>(w(0x8c), 1);
    let first = crate::sa_pool::take(g, img, img.rel32_target(f + 0x95), w(0x71));
    let head = g.read::<u32>(w(0x9b));
    g.write::<u8>(first + 0x18, 1);
    g.write::<u32>(first + 4, 0);
    // `edi`: the old head, or the first node when the list was empty
    let edi = if head == 0 {
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0xc7), first);
        first
    } else {
        let last = g.read::<u32>(w(0xb2));
        g.write::<u32>(first, last);
        g.write::<u32>(last + 4, first);
        head
    };
    g.write::<u32>(first + 8, first);
    let second = crate::sa_pool::take(g, img, img.rel32_target(f + 0xd4), w(0xcc));
    g.write::<u32>(second + 4, 0);
    if edi == 0 {
        // only for a null first node
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0xf1), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first + 4, second);
    }
    g.write::<u32>(second + 8, second);
    g.write::<u32>(w(0xfe), second);
    crate::sa_misc::records_init_100(g, img, img.rel32_target(f + 0x103), w(0xf6));
    let third = crate::sa_pool::take(g, img, img.rel32_target(f + 0x10d), w(0x108));
    let head = g.read::<u32>(w(0x113));
    g.write::<u8>(third + 0x18, 2);
    g.write::<u32>(third + 4, 0);
    if head == 0 {
        g.write::<u32>(third, 0);
        g.write::<u32>(w(0x144), third);
        g.write::<u32>(third + 8, third);
        g.write::<u32>(w(0x14c), third);
    } else {
        let last = g.read::<u32>(w(0x129));
        g.write::<u32>(third, last);
        g.write::<u32>(last + 4, third);
        g.write::<u32>(third + 8, third);
        g.write::<u32>(w(0x136), third);
    }
    crate::Ret::eax(0)
}

/// [`start_up`] (339 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 339,
    hash: 0x0613_38ca_d8b6_2033,
    calls: &[
        (0x44, crate::Callee::Sig(&crate::l5_objhost::OBJ_BLOCK_FILL)),
        (0x67, crate::Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x95, crate::Callee::Sig(&crate::sa_pool::TAKE)),
        (0xd4, crate::Callee::Sig(&crate::sa_pool::TAKE)),
        (0x103, crate::Callee::Sig(&crate::sa_misc::RECORDS_INIT_100)),
        (0x10d, crate::Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// A task step (0x10001a80, cdecl `(task)`): the motion script of the
/// record at `+0x05` ([`motion_script`](crate::s4_misc::motion_script))
/// and the objects run ([`run_objects`](crate::l5_objhost::run_objects));
/// when both give 1 and `*G_D` (`+0x20`) is 1, the grid is drawn
/// ([`draw_grid`](crate::sb_misc::draw_grid)), the task's state is 3 and
/// its record's `+0x10` the immediate `+0x3b`. Returns the record, else
/// the objects' result.
pub fn grid_step<G: crate::Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> crate::Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let m = crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0xa), w(0x5));
    // push ebp; push esi; call
    let r =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x11), fr.esp.wrapping_sub(12));
    if r != 1 || m != 1 || g.read::<u32>(w(0x20)) != 1 {
        return crate::Ret::eax(r);
    }
    crate::sb_misc::draw_grid(g, img, img.rel32_target(f + 0x27));
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), w(0x3b));
    crate::Ret::eax(rec)
}

/// [`grid_step`] (66 bytes, 1 overlay).
pub const GRID_STEP: Sig = Sig {
    len: 66,
    hash: 0x5461_8f50_1ab1_90d6,
    calls: &[
        (0xa, crate::Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x11, crate::Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x27, crate::Callee::Sig(&crate::sb_misc::DRAW_GRID)),
    ],
    imports: &[],
};
