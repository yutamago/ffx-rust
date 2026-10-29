//! Shared functions without a family module yet, ported by the shared-4
//! list. Each doc names the DLL and address it was ported from, its size
//! and how many DLLs carry it.

use crate::l5_objhost::{OBJ_SCRIPT, POOL_ALLOC, obj_script, pool_alloc};
use crate::m0163::{TARGET_ANGLE, target_angle};
use crate::opu_parts::{START_PART, start_part};
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sa_misc::{LINK_PART, link_part};
use crate::sa_misc::{
    RESET_TRANSFORM, RESET_TRANSFORM_0, RESET_TRANSFORM_3, RESET_TRANSFORM_14, reset_transform,
    reset_transform_0, reset_transform_3, reset_transform_14,
};
use crate::sa_pool::{GIVE_BACK, give_back};
use crate::sb_common::{ANIM_STEP, SET_SPEED, anim_step, set_speed};
use crate::sb_misc::{
    ACTOR_SCALE, ANIM_RESET, EASE_ANGLE, EASE_SAMPLE, MEAN_POSITION, ROTATE_OFFSET, actor_scale,
    anim_reset, ease_angle, ease_sample, mean_position, rotate_offset,
};
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

fn f32_at<G: Game>(g: &G, a: u32) -> f32 {
    f32::from_bits(g.read::<u32>(a))
}

fn set_f32<G: Game>(g: &mut G, a: u32, x: f32) {
    g.write::<u32>(a, x.to_bits());
}

/// A pool node ([`pool_alloc`] at `pa`) holding `obj`, appended to the
/// DLL's object list: the operands are the offsets of the list's tail
/// (read when empty, `tail0`), head and tail stores and count when it
/// was empty (`head`, `tail1`, `count1`), and of the tail read, store and
/// count when it was not (`tail2`, `tail3`, `count2`). Nothing happens
/// when the pool is empty.
#[allow(clippy::too_many_arguments)] // the operands of the original's two branches
fn append_object<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    pa: u32,
    obj: u32,
    [tail0, head, tail1, count1, tail2, tail3, count2]: [u32; 7],
) {
    let w = |off: u32| img.word(f + off);
    let node = pool_alloc(g, img, pa);
    if node == 0 {
        return;
    }
    g.write::<u32>(node + 4, 0);
    let tail = g.read::<u32>(w(tail0));
    g.write::<u32>(node, tail);
    let count = if tail == 0 {
        g.write::<u32>(w(head), node);
        g.write::<u32>(w(tail1), node);
        count1
    } else {
        let tail = g.read::<u32>(w(tail2));
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(w(tail3), node);
        count2
    };
    g.write::<u32>(node + 8, obj);
    let n = g.read::<u32>(w(count));
    g.write::<u32>(w(count), n.wrapping_add(1));
}

/// The object start shared by [`start_object_at`] and
/// [`start_object_anim`]: the model words (scale `+0x60..+0x6c`), `*G_P
/// = ITEM` (operands `+0x04`, `+0x59`), the part `list[index]` of slot
/// 941 `(*G_D + 0x10)` (operands `+0x5e`, `+0x66`) started on it
/// ([`start_part`], `0x1000`, 1; its `rel32` at `part`), `reset` on the
/// object, then a pool node holding it appended to the object list
/// ([`pool_alloc`]'s `rel32` at `pool`, the list's operands `list`).
/// Returns the object.
#[allow(clippy::too_many_arguments)] // the original's registers, stack words and operands
fn start_object<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    scale: f32,
    index: u32,
    reset: impl FnOnce(&mut G),
    (part, pool, list): (u32, u32, [u32; 7]),
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    for o in [0x48u32, 0x44, 0x40] {
        g.write::<u32>(obj + o, 0);
    }
    g.write::<u32>(obj + 0x4c, 0x3f80_0000);
    for o in [0x5cu32, 0x58, 0x54, 0x50] {
        g.write::<u32>(obj + o, 0);
    }
    for o in [0x6cu32, 0x68, 0x64, 0x60] {
        g.write::<u32>(obj + o, scale.to_bits());
    }
    g.write::<u32>(p, w(0x59));
    let d = g.read::<u32>(g.read::<u32>(w(0x5e)).wrapping_add(0x10));
    let s941 = slot_target(g, w(0x66), 941);
    let table = g.call(s941, 0, &[d]).eax;
    let program = g.read::<u32>(table.wrapping_add(index.wrapping_mul(4)));
    start_part(g, img, img.rel32_target(f + part), obj, 0x1000, program, 1);
    reset(g);
    append_object(g, img, f, img.rel32_target(f + pool), obj, list);
    obj
}

/// Starts an effect object (internal: `edx` = the object, `xmm3` its
/// scale, the stack `(index, owner, -, owner_b)`): [`start_object`] with
/// its transform reset with the two owners ([`reset_transform`]: `edx` =
/// `owner`, `+0x100` = `owner_b`). Returns the object.
///
/// `magic_0018` 0x10003a10 (241 bytes; 76 DLLs).
#[allow(clippy::too_many_arguments)] // the original's registers and stack words
pub fn start_object_at<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    scale: f32,
    index: u32,
    owner: u32,
    owner_b: u32,
) -> u32 {
    // push owner_b; push ecx (not read); call with `edx` = owner
    let reset = |g: &mut G| {
        reset_transform(g, obj, owner, owner_b);
    };
    let list = [0xa8, 0xbb, 0xc1, 0xca, 0xd6, 0xdf, 0xe8];
    start_object(g, img, f, obj, scale, index, reset, (0x84, 0x9d, list))
}

/// [`start_object_at`].
pub const START_OBJECT_AT: Sig = Sig {
    len: 241,
    hash: 0x0131_a0f5_0281_290a,
    calls: &[
        (0x84, Callee::Sig(&START_PART)),
        (0x95, Callee::Sig(&RESET_TRANSFORM)),
        (0x9d, Callee::Sig(&POOL_ALLOC)),
    ],
    imports: &[],
};

/// Starts an effect object (internal: `edx` = the object, `xmm3` its
/// scale, the stack `(index, value)`): [`start_object`] with its
/// animation block reset ([`anim_reset`], `+0x114` = `value`). Returns
/// the object.
///
/// `magic_0054` 0x10001b80 (237 bytes; 23 DLLs).
pub fn start_object_anim<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    scale: f32,
    index: u32,
    value: u32,
) -> u32 {
    let reset = |g: &mut G| {
        anim_reset(g, obj, value);
    };
    let list = [0xa4, 0xb7, 0xbd, 0xc6, 0xd2, 0xdb, 0xe4];
    start_object(g, img, f, obj, scale, index, reset, (0x84, 0x99, list))
}

/// Starts an effect object (internal: `edx` = the object, `xmm3` its
/// scale, the stack `(index, owner, a, b)`): [`start_object`] with its
/// transform reset with three owners ([`reset_transform_3`]: `+0x114` =
/// `owner`, `+0x11c` = `a`, `+0x100` = `b`). Returns the object.
///
/// `magic_0092` 0x100027f0 (240 bytes; 20 DLLs).
#[allow(clippy::too_many_arguments)] // the original's registers and stack words
pub fn start_object_owners<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    scale: f32,
    index: u32,
    owner: u32,
    (a, b): (u32, u32),
) -> u32 {
    let reset = |g: &mut G| {
        reset_transform_3(g, obj, owner, a, b);
    };
    let list = [0xa7, 0xba, 0xc0, 0xc9, 0xd5, 0xde, 0xe7];
    start_object(g, img, f, obj, scale, index, reset, (0x84, 0x9c, list))
}

/// Starts an effect object (internal: `edx` = the object, `xmm3` its
/// scale, the stack `(index)`): [`start_object`] with its transform reset
/// with no owners ([`reset_transform_0`]). Returns the object.
///
/// `magic_0162` 0x100096a0 (234 bytes; 16 DLLs).
pub fn start_object_plain<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    scale: f32,
    index: u32,
) -> u32 {
    let reset = |g: &mut G| {
        reset_transform_0(g, obj);
    };
    let list = [0xa1, 0xb4, 0xba, 0xc3, 0xcf, 0xd8, 0xe1];
    start_object(g, img, f, obj, scale, index, reset, (0x84, 0x96, list))
}

/// Starts an effect object (internal: `edx` = the object, `xmm3` its
/// scale, the stack `(-, owner)`): [`start_object`] with the part
/// `list[0]` and its transform reset with the owner and the callback
/// `CB` (operand `+0x8b`; [`reset_transform_14`]). Returns the object.
///
/// `magic_0010` 0x10001cd0 (239 bytes; 11 DLLs).
pub fn start_object_first<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    scale: f32,
    owner: u32,
) -> u32 {
    let callback = img.word(f + 0x8b);
    let reset = |g: &mut G| {
        reset_transform_14(g, obj, owner, callback);
    };
    let list = [0xa6, 0xb9, 0xbf, 0xc8, 0xd4, 0xdd, 0xe6];
    start_object(g, img, f, obj, scale, 0, reset, (0x80, 0x9b, list))
}

/// [`start_object_first`].
pub const START_OBJECT_FIRST: Sig = Sig {
    len: 239,
    hash: 0xfbf5_d324_0967_22ae,
    calls: &[
        (0x80, Callee::Sig(&START_PART)),
        (0x93, Callee::Sig(&RESET_TRANSFORM_14)),
        (0x9b, Callee::Sig(&POOL_ALLOC)),
    ],
    imports: &[],
};

/// [`start_object_plain`].
pub const START_OBJECT_PLAIN: Sig = Sig {
    len: 234,
    hash: 0x72e6_9a9a_6c3a_e971,
    calls: &[
        (0x84, Callee::Sig(&START_PART)),
        (0x8e, Callee::Sig(&RESET_TRANSFORM_0)),
        (0x96, Callee::Sig(&POOL_ALLOC)),
    ],
    imports: &[],
};

/// [`start_object_owners`].
pub const START_OBJECT_OWNERS: Sig = Sig {
    len: 240,
    hash: 0x794c_89b3_4294_d9a2,
    calls: &[
        (0x84, Callee::Sig(&START_PART)),
        (0x94, Callee::Sig(&RESET_TRANSFORM_3)),
        (0x9c, Callee::Sig(&POOL_ALLOC)),
    ],
    imports: &[],
};

/// [`start_object_anim`].
pub const START_OBJECT_ANIM: Sig = Sig {
    len: 237,
    hash: 0x8bc5_2ce5_0a68_533f,
    calls: &[
        (0x84, Callee::Sig(&START_PART)),
        (0x91, Callee::Sig(&ANIM_RESET)),
        (0x99, Callee::Sig(&POOL_ALLOC)),
    ],
    imports: &[],
};

/// The frame's matrices of [`sprites_run`] (`E` = `esp` after the
/// aligned frame and the three pushes): `E + 0x28` the scale (slot 235's
/// unit matrix, its diagonal set per sprite), `E + 0x68` the rotation,
/// `E + 0x98` the placed one, `E + 0xa8` a copy, `E + 0xe8` the camera,
/// `E + 0x128` the screen.
struct SpriteFrame {
    e: u32,
}

impl SpriteFrame {
    fn at(&self, off: u32) -> u32 {
        self.e.wrapping_add(off)
    }
}

/// One sprite of [`sprites_run`] (`b` = the record + 0x28, `i` its
/// index): its motion, scale, roll, matrices, colour and animation frame
/// (see there).
#[allow(clippy::too_many_lines)] // the original's steps in order
fn sprite<G: Game>(g: &mut G, img: &Image, f: u32, fs: &SpriteFrame, b: u32, i: u32) {
    let w = |o: u32| img.word(f + o);
    if g.read::<u8>(b + 0x48) & 2 != 0 {
        // the velocity `-0x8..+0x0` damped by `+0x4`
        let k = f32_at(g, b + 4);
        let x = mul(f32_at(g, b - 8), k);
        set_f32(g, b - 8, x);
        let x = mul(f32_at(g, b - 4), k);
        set_f32(g, b - 4, x);
        let x = mul(k, f32_at(g, b));
        set_f32(g, b, x);
    }
    // the speed `-0x18..-0x10` grows by the velocity, the position
    // `-0x28..-0x20` by the speed (operands in the original's order)
    let x = add(f32_at(g, b - 8), f32_at(g, b - 0x18));
    set_f32(g, b - 0x18, x);
    let x = add(f32_at(g, b - 4), f32_at(g, b - 0x14));
    set_f32(g, b - 0x14, x);
    let x = add(f32_at(g, b - 0x10), f32_at(g, b));
    set_f32(g, b - 0x10, x);
    let x = add(f32_at(g, b - 0x18), f32_at(g, b - 0x28));
    set_f32(g, b - 0x28, x);
    let x = add(f32_at(g, b - 0x14), f32_at(g, b - 0x24));
    set_f32(g, b - 0x24, x);
    let x = add(f32_at(g, b - 0x20), f32_at(g, b - 0x10));
    set_f32(g, b - 0x20, x);
    // the scale: `+0x18` blended towards `K1` over `+0x30` frames
    // (`+0x34` the frame), times `+0x14`
    let total = g.read::<u32>(b + 0x30);
    let size = f32_at(g, b + 0x14);
    let mut scale = f32_at(g, b + 0x18);
    if total.cast_signed() > 0 {
        let n = g.read::<u32>(b + 0x34);
        let r = div(cvt(n), cvt(total));
        let k1 = f32_at(g, w(0x199));
        scale = mul(add(mul(scale, r), sub(k1, r)), size);
        let n = n.wrapping_add(1);
        g.write::<u32>(b + 0x34, n);
        if n.cast_signed() > total.cast_signed() {
            g.write::<u32>(b + 0x30, 0);
        }
    } else {
        scale = mul(scale, size);
    }
    for off in [0x50, 0x3c, 0x28] {
        set_f32(g, fs.at(off), scale);
    }
    // the roll `-0xc` by `+0x20`, wrapped by `K4` past `K2` or below `K3`
    let step = f32_at(g, w(0x9a));
    let mut roll = add(f32_at(g, b + 0x20), f32_at(g, b - 0xc));
    if roll > f32_at(g, w(0x1e0)) {
        roll = sub(roll, step);
    } else if f32_at(g, w(0x1f0)) > roll {
        roll = add(roll, step);
    }
    set_f32(g, b - 0xc, roll);
    let t = slot_target(g, w(0x20e), 330);
    g.call(t, 0, &[fs.at(0x68), fs.at(0x28), roll.to_bits()]);
    let t = slot_target(g, w(0x234), 229);
    g.call(t, 0, &[fs.at(0x98), fs.at(0xe8), b - 0x28]);
    g.write::<u32>(w(0x245), 0);
    if g.read::<u8>(b + 0x48) & 1 != 0 {
        let t = slot_target(g, w(0x261), 230);
        g.call(t, 0, &[fs.at(0xa8), fs.at(0x68)]);
        let z = add(f32_at(g, b + 0x1c), f32_at(g, fs.at(0xe0)));
        g.write::<u32>(w(0x286), fs.at(0xa8));
        set_f32(g, fs.at(0xe0), z);
    }
    let total = g.read::<i32>(b + 0x28);
    if total > 0 {
        // the colour: bytes `+0x3c` (from) to `+0x40` (to) over `+0x28`
        // frames, `|+0x2c|` the frame (`idiv`)
        let at = g.read::<i32>(b + 0x2c).wrapping_abs();
        for (k, op) in [(0u32, 0x2bf), (1, 0x2dd), (2, 0x2fb), (3, 0x31d)] {
            let total = if k == 0 {
                total
            } else {
                g.read::<i32>(b + 0x28)
            };
            let from = i32::from(g.read::<u8>(b + 0x3c + k));
            let to = i32::from(g.read::<u8>(b + 0x40 + k));
            let n = from
                .wrapping_mul(total.wrapping_sub(at))
                .wrapping_add(to.wrapping_mul(at));
            // idiv: a zero total (or an overflow) faults in the original
            let c = n.checked_div(total).unwrap_or(0);
            g.write::<u8>(w(op), c.cast_unsigned() as u8);
        }
        let n = g.read::<u32>(b + 0x2c).wrapping_add(1);
        g.write::<u32>(b + 0x2c, n);
        if n.cast_signed() > g.read::<i32>(b + 0x28) {
            g.write::<u32>(b + 0x28, 0);
        }
    } else {
        let c = g.read::<u32>(b + 0x40);
        g.write::<u32>(w(0x339), c);
    }
    let anim = b + 8;
    let entry = anim_step(g, img, img.rel32_target(f + 0x343), anim);
    if g.read::<i32>(b + 0x24) < 0 && g.read::<u32>(b + 0xc) != 1 {
        g.write::<u32>(b + 0x24, 1);
        return;
    }
    // the frame's sprite: its record in the table slot 941 gives
    let t = slot_target(g, w(0x364), 941);
    let table = g.call(t, 0, &[g.read::<u32>(anim)]).eax;
    let rec = i32::from(g.read::<u16>(entry).cast_signed())
        .cast_unsigned()
        .wrapping_add(table);
    let t = slot_target(g, w(0x381), 941);
    let table = g.call(t, 0, &[g.read::<u32>(anim)]).eax;
    let t = slot_target(g, w(0x397), 772);
    let out = g.call(t, 0, &[table, i]);
    let (lo, hi) = (out.eax, out.edx);
    let lock_at = g.read::<u32>(w(0x3a5));
    let saved = g.read::<u32>(lock_at);
    g.write::<u32>(lock_at, 1);
    let t = slot_target(g, w(0x3b7), 773);
    let known = g.call(t, 0, &[lo, hi, table, i]).eax;
    let lock_at = g.read::<u32>(w(0x3d3));
    g.write::<u32>(lock_at, saved);
    let colour = w(0x3ef);
    let kind = u32::from(g.read::<u16>(b + 0x10));
    if known != 0 {
        g.write::<u32>(lock_at, 1);
        let t = slot_target(g, w(0x3ea), 774);
        g.call(t, 0, &[lo, hi, anim, colour, kind]);
        let lock_at = g.read::<u32>(w(0x409));
        g.write::<u32>(lock_at, 1);
        let t = slot_target(g, w(0x417), 775);
        g.call(t, 0, &[lo, hi]);
        let lock_at = g.read::<u32>(w(0x426));
        g.write::<u32>(lock_at, saved);
    } else {
        let off = g.read::<u32>(rec + 4);
        let byte = u32::from(g.read::<u8>(off.wrapping_add(rec).wrapping_add(4)));
        let t = slot_target(g, w(0x443), 776);
        let a = g.call(t, 0, &[byte]).eax;
        let t = slot_target(g, w(0x451), 777);
        let p = g.read::<u32>(w(0x460));
        let flags = a | g.call(t, 0, &[p]).eax;
        let kind = u32::from(g.read::<u16>(b + 0x10));
        let lock_at = g.read::<u32>(w(0x46e));
        let saved = g.read::<u32>(lock_at);
        g.write::<u32>(lock_at, 1);
        let t = slot_target(g, w(0x489), 778);
        g.call(t, 0, &[lo, hi, anim, w(0x480), kind, flags]);
        let lock_at = g.read::<u32>(w(0x4a1));
        g.write::<u32>(lock_at, saved);
    }
}

/// The sprites' frame (a table entry, no arguments; an 8-aligned /GS
/// frame). Nothing while the game is paused (slot 561 `Sg_GetPause`
/// non-zero, returned). Else the screen and camera matrices (slots 723,
/// 724) and a unit matrix (slot 235) go into the frame, and each record
/// of `G_LIST` (operand `+0x34`; `G_N` of them, 0x74 bytes, `b` = the
/// record + 0x28) that is live (`+0x44`) and not waiting (`+0x38` counts
/// down first) is moved: its velocity damped (flag `+0x48` bit 1), its
/// speed and position stepped, its scale (`+0x18` towards `K1` over
/// `+0x30` frames, times `+0x14`) set on the unit matrix's diagonal, its
/// roll `-0xc` stepped by `+0x20` and wrapped, slot 330 rolls the matrix,
/// slot 229 places it with the camera; `G_P` is 0, or (flag bit 0) a copy
/// of the rolled matrix (slot 230) with `+0x1c` added to its `z`. Its
/// colour goes into `G_COL` (faded from `+0x3c` to `+0x40` over `+0x28`
/// frames, or `+0x40` as it is), its animation steps
/// ([`anim_step`]); then, unless a negative life `+0x24` is held (`+0xc`
/// not 1: it becomes 1), the frame's handle (slot 772 of the frame
/// table and the index) is drawn: slots 774 and 775 when slot 773 knows
/// it, else slot 778 with the flags of slots 776 and 777, `**G_LOCK` 1
/// around them. A positive life counts down; at 0 the record dies.
/// `G_COUNT` and the return value are the number of live records.
///
/// `magic_0053` 0x10002520 (1243 bytes; 58 DLLs).
pub fn sprites_run<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = slot_target(g, w(0x1b), 561);
    let paused = g.call(t, 0, &[]).eax;
    if paused != 0 {
        return Ret::eax(paused);
    }
    // push ebp; and esp, -8; sub esp, 0x164; three pushes
    let ebp = fr.esp.wrapping_sub(4);
    let fs = SpriteFrame {
        e: (ebp & !7).wrapping_sub(0x164 + 12),
    };
    let list = g.read::<u32>(w(0x34));
    let t = slot_target(g, w(0x41), 723);
    g.call(t, 0, &[fs.at(0x128)]);
    let t = slot_target(g, w(0x59), 724);
    g.call(t, 0, &[fs.at(0xe8)]);
    // `lea eax, [esp + 0x6c]` with the argument still pushed
    g.write::<u32>(w(0x6a), fs.at(0x68));
    let t = slot_target(g, w(0x77), 235);
    g.call(t, 0, &[fs.at(0x28)]);
    let mut count = 0u32;
    if g.read::<i32>(w(0x90)) > 0 {
        let mut b = list.wrapping_add(0x28);
        let mut i = 0u32;
        loop {
            if g.read::<u32>(b + 0x44) != 0 {
                count = count.wrapping_add(1);
                let wait = g.read::<i32>(b + 0x38);
                if wait > 0 {
                    g.write::<i32>(b + 0x38, wait - 1);
                } else {
                    sprite(g, img, f, &fs, b, i);
                    let life = g.read::<i32>(b + 0x24);
                    if life > 0 {
                        g.write::<i32>(b + 0x24, life - 1);
                        if life == 1 {
                            g.write::<u32>(b + 0x44, 0);
                        }
                    }
                }
            }
            i += 1;
            b = b.wrapping_add(0x74);
            if i.cast_signed() >= g.read::<i32>(w(0xc5)) {
                break;
            }
        }
    }
    g.write::<u32>(w(0xcc), count);
    Ret::eax(count)
}

/// [`sprites_run`] (the `rel32`s: the cookie check and [`anim_step`]).
pub const SPRITES_RUN: Sig = Sig {
    len: 1243,
    hash: 0xff0f_41cd_ae14_c472,
    calls: &[(0xdd, Callee::Any), (0x343, Callee::Sig(&ANIM_STEP))],
    imports: &[],
};

/// `|a| > |b|` as the original compares them: both widened to `f64`, the
/// sign cleared with the DLL's mask (`andpd`), `comisd` (unordered: no).
fn wider(mask: u64, a: f32, b: f32) -> bool {
    let abs = |x: f32| f64::from_bits(f64::from(x).to_bits() & mask);
    abs(a) > abs(b)
}

/// A shake task of the `magic_0162` family (cdecl `(task)`, its state
/// `s` at `+0x14`; an 8-aligned frame). A delay `+0x24` counts down first
/// (returned). While the time left `+0x28` counts down, the curve's time
/// `t` (`+0x1c`) steps by `+0x20` and two quadratic curves of `u = K - t`
/// and `t` (`K` operand `+0x49`, the middle terms times `K2`, operands
/// `+0x6c`/`+0x9d`) give `x` (from `+0x4`, `+0x8`, `+0xc`) and `y` (from
/// `+0x10`, `+0x14`, `+0x18`); flags `+0x2c` bit 0/1 put `x`/`y` through
/// slot 314 (ST0). At the end the callback `+0x0` runs `(s)` and, unless
/// flag bit 2 is set, the task is marked (bit 15) and given back
/// ([`give_back`]); `x` and `y` are then 0. An odd time left scales both
/// by `K3` (operands `+0x12c`, `+0x134`). `G_X` and `G_Y` keep the larger
/// in size. Returns the delay left, the time left, slot 314's `eax`, the
/// flags or [`give_back`]'s `eax`, as the original leaves it.
///
/// `magic_0162` 0x10007d10 (471 bytes; 27 DLLs).
pub fn shake_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 0x14);
    let delay = g.read::<i32>(s + 0x24);
    if delay > 0 {
        g.write::<i32>(s + 0x24, delay - 1);
        return Ret::eax((delay - 1).cast_unsigned());
    }
    let mut eax = g.read::<u32>(s + 0x28);
    if eax.cast_signed() > 0 {
        eax -= 1;
        g.write::<u32>(s + 0x28, eax);
    }
    let (x, y) = if g.read::<i32>(s + 0x28) > 0 {
        let flags = g.read::<u8>(s + 0x2c);
        let t = add(f32_at(g, s + 0x20), f32_at(g, s + 0x1c));
        let k = f32_at(g, w(0x49));
        let u = sub(k, t);
        set_f32(g, s + 0x1c, t);
        let curve = |g: &G, k2: u32, o: u32| {
            let mid = mul(mul(mul(u, t), f32_at(g, w(k2))), f32_at(g, s + o + 4));
            let first = mul(mul(u, u), f32_at(g, s + o));
            add(add(mid, first), mul(mul(t, t), f32_at(g, s + o + 8)))
        };
        let mut x = curve(g, 0x6c, 0x4);
        let mut y = curve(g, 0x9d, 0x10);
        if flags & 1 != 0 {
            let t = slot_target(g, w(0xcb), 314);
            let out = g.call(t, 0, &[x.to_bits()]);
            eax = out.eax;
            x = f32::from_bits(crate::sa_summon::st0_f32(out));
        }
        if g.read::<u8>(s + 0x2c) & 2 != 0 {
            let t = slot_target(g, w(0xfd), 314);
            let out = g.call(t, 0, &[y.to_bits()]);
            eax = out.eax;
            y = f32::from_bits(crate::sa_summon::st0_f32(out));
        }
        (x, y)
    } else {
        let callback = g.read::<u32>(s);
        if callback != 0 {
            // its eax is overwritten at once
            g.call(callback, fr.ecx, &[s]);
        }
        let flags = u32::from(g.read::<u16>(s + 0x2c));
        eax = flags;
        if flags & 4 == 0 {
            g.write::<u16>(s + 0x2c, (flags | 0x8000) as u16);
            eax = give_back(g, img, img.rel32_target(f + 0x1ce), task);
        }
        // junk read: xmm2 (x) never written on this path; 0 (brief
        // 2026-10-24)
        (0.0, 0.0)
    };
    let (mut x, mut y) = (x, y);
    if g.read::<u8>(s + 0x28) & 1 != 0 {
        x = mul(x, f32_at(g, w(0x12c)));
        y = mul(y, f32_at(g, w(0x134)));
    }
    let mask = g.read::<u64>(w(0x144));
    if wider(mask, x, f32_at(g, w(0x13c))) {
        set_f32(g, w(0x160), x);
    }
    if wider(mask, y, f32_at(g, w(0x168))) {
        set_f32(g, w(0x188), y);
    }
    Ret::eax(eax)
}

/// [`shake_task`] (the `rel32`: [`give_back`]; the jump back is inside).
pub const SHAKE_TASK: Sig = Sig {
    len: 471,
    hash: 0x1e1a_5c75_1f8c_62f4,
    calls: &[(0x1ce, Callee::Sig(&GIVE_BACK))],
    imports: &[],
};

/// A task placing an object on its target (cdecl `(task)`, its state `s`
/// at `+0x14`; a /GS frame). By the mode `+0x44`:
/// - 1: the target `+0x48` (-1 the caster `G_SELF`, operand `+0x38`,
///   else `G_CHRS[G_MAP[i]]`, operands `+0x3f`, `+0x4a`) gives its
///   position (slot 262 into `[ebp - 0x14]`, `w` 1.0); with flag `+0x40`
///   bit 0 the offset `s` is turned by the target's angle
///   ([`target_angle`], the angles `(0, a, 0)` at `[ebp - 0x24]`) and
///   added to it ([`rotate_offset`]);
/// - 2: the actors' mean position ([`mean_position`]);
/// - else the object keeps its place.
///
/// The position goes into the object `s + 0x10`, whose script runs
/// ([`obj_script`]); when it ends (flag bit 15) with bit 14 set, the task
/// is marked (`+0x4c` bit 15) and given back ([`give_back`]). Returns
/// the script's end flag, 0x4000, or [`give_back`]'s `eax`.
///
/// `magic_0052` 0x10002730 (258 bytes; 26 DLLs).
pub fn follow_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let pos = ebp.wrapping_sub(0x14);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 0x14);
    let placed = match g.read::<u32>(s + 0x44) {
        1 => {
            let i = g.read::<u32>(s + 0x48);
            let chr = if i == u32::MAX {
                g.read::<u32>(w(0x38))
            } else {
                let map = g.read::<u32>(w(0x3f));
                let k = u32::from(g.read::<u8>(map.wrapping_add(i)));
                g.read::<u32>(w(0x4a).wrapping_add(4 * k))
            };
            let t = slot_target(g, w(0x5c), 262);
            g.call(t, 0, &[chr, pos, pos + 4, pos + 8]);
            g.write::<u32>(pos + 0xc, 0x3f80_0000);
            if g.read::<u8>(s + 0x40) & 1 != 0 {
                let angles = ebp.wrapping_sub(0x24);
                g.write::<u32>(angles, 0);
                let i = g.read::<u32>(s + 0x48);
                let a = target_angle(g, img, img.rel32_target(f + 0x83), i);
                set_f32(g, angles + 4, a);
                g.write::<u32>(angles + 8, 0);
                // sub esp, 0x24; push esi; push edi; push s; call
                let esp = ebp.wrapping_sub(0x34);
                g.write::<u32>(esp + 4, s);
                rotate_offset(g, img, img.rel32_target(f + 0x9b), esp, angles, pos);
            }
            true
        }
        2 => {
            // sub esp, 0x24; push esi; push edi; call
            mean_position(
                g,
                img,
                img.rel32_target(f + 0x29),
                ebp.wrapping_sub(0x30),
                pos,
            );
            true
        }
        _ => false,
    };
    if placed {
        for i in 0..4 {
            let x = g.read::<u32>(pos + 4 * i);
            g.write::<u32>(s + 0x10 + 4 * i, x);
        }
    }
    let obj = s + 0x10;
    obj_script(g, img, img.rel32_target(f + 0xce), obj);
    // the script's `eax`: its flags' bit 15
    let ended = u32::from(g.read::<u16>(obj + 0x30)) >> 15;
    if ended != 1 {
        return Ret::eax(ended);
    }
    if g.read::<u16>(s + 0x40) & 0x4000 == 0 {
        return Ret::eax(0x4000);
    }
    let flags = g.read::<u16>(s + 0x4c) | 0x8000;
    g.write::<u16>(s + 0x4c, flags);
    Ret::eax(give_back(g, img, img.rel32_target(f + 0xee), task))
}

/// [`follow_task`].
pub const FOLLOW_TASK: Sig = Sig {
    len: 258,
    hash: 0x0715_19c5_1314_f28a,
    calls: &[
        (0x29, Callee::Sig(&MEAN_POSITION)),
        (0x83, Callee::Sig(&TARGET_ANGLE)),
        (0x9b, Callee::Sig(&ROTATE_OFFSET)),
        (0xce, Callee::Sig(&OBJ_SCRIPT)),
        (0xee, Callee::Sig(&GIVE_BACK)),
        (0xfa, Callee::Any),
    ],
    imports: &[],
};

/// The operand offsets of one [`angle_track`] copy: the three easing
/// calls' `rel32`s and the table operands of slots 235, 330, 328, 329,
/// 229.
struct AngleOps {
    calls: [u32; 3],
    slots: [u32; 5],
}

/// One step of an angle track (`ecx` = the track: the keys `+0x20`,
/// three floats per key, `n` keys `+0x38`, the time `+0x2c` and its step
/// `+0x30`, the steps left `+0x34`, the base `+0x24`; `edx` = four floats
/// out; a /GS frame). On the last step the last key is taken and `+0x3c`
/// cleared; otherwise the time moves on and, at it (or at the quartic
/// curve of it, `curve` the operands of `K1` and `K2`,
/// [`crate::s3_misc::quartic`]), the keys give two angles
/// ([`ease_angle`]) and a distance ([`ease_sample`]; stride 3, each call
/// seeing the strides the earlier ones pushed). The distance `(0, 0, d,
/// 1)` on the frame (`[ebp - 0x14]`) is turned by a matrix (`[ebp -
/// 0x54]`: slot 235's unit, rolled by 0 (330), the first angle (328), the
/// second (329); slot 229) and added to the base; the base's `w` is
/// copied and returned.
fn angle_track<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    curve: Option<(u32, u32)>,
    o: &AngleOps,
) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (s, out) = (fr.ecx, fr.edx);
    let ebp = fr.esp.wrapping_sub(4);
    let left = g.read::<u32>(s + 0x34).wrapping_sub(1);
    g.write::<u32>(s + 0x34, left);
    let keys = g.read::<u32>(s + 0x20);
    let (a0, a1, d) = if left == 0 {
        let n = g.read::<u32>(s + 0x38);
        let last = keys.wrapping_add(n.wrapping_mul(12)).wrapping_sub(12);
        let key = (f32_at(g, last), f32_at(g, last + 4), f32_at(g, last + 8));
        g.write::<u32>(s + 0x3c, 0);
        key
    } else {
        let t = add(f32_at(g, s + 0x30), f32_at(g, s + 0x2c));
        let x = match curve {
            Some((k1, k2)) => {
                let c = g.read::<u32>(s + 0x28);
                crate::s3_misc::quartic(g, c, t, f32_at(g, w(k1)), f32_at(g, w(k2)))
            }
            None => t,
        };
        let n = g.read::<u32>(s + 0x38);
        set_f32(g, s + 0x2c, t);
        // sub esp, 0x64; three pushes; then a stride pushed per call
        let mut esp = ebp.wrapping_sub(0x64 + 12);
        let mut ease = |g: &mut G, i: u32, angle: bool| {
            esp = esp.wrapping_sub(4);
            g.write::<u32>(esp, 3);
            let target = img.rel32_target(f + o.calls[i as usize]);
            let at = esp.wrapping_sub(4);
            if angle {
                ease_angle(g, img, target, at, keys + 4 * i, n, x)
            } else {
                ease_sample(g, img, target, at, keys + 4 * i, n, x)
            }
        };
        let a0 = ease(g, 0, true);
        let a1 = ease(g, 1, true);
        let d = ease(g, 2, false);
        (a0, a1, d)
    };
    let m = ebp.wrapping_sub(0x54);
    let v = ebp.wrapping_sub(0x14);
    for (i, x) in [0, 0, d.to_bits(), 0x3f80_0000].into_iter().enumerate() {
        g.write::<u32>(v + 4 * i as u32, x);
    }
    let [s235, s330, s328, s329, s229] = o.slots;
    let mut st = Pushes::default();
    st.push(m);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(s235), 235), 0);
    st.set_top(0);
    st.push_all(&[m, m]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(s330), 330), 0);
    st.pop(2);
    st.set_top(a0.to_bits());
    st.push_all(&[m, m]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(s328), 328), 0);
    st.pop(2);
    st.set_top(a1.to_bits());
    st.push_all(&[m, m]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(s329), 329), 0);
    st.push_all(&[v, m, v]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(s229), 229), 0);
    let base = g.read::<u32>(s + 0x24);
    for i in 0..3 {
        let x = add(f32_at(g, base + 4 * i), f32_at(g, v + 4 * i));
        set_f32(g, out + 4 * i, x);
    }
    let last = g.read::<u32>(base + 0xc);
    g.write::<u32>(out + 0xc, last);
    last
}

/// [`angle_track`] at the track's own time.
///
/// `magic_0162` 0x10005b00 (391 bytes, `ecx`/`edx`; 24 DLLs).
pub fn angle_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let o = AngleOps {
        calls: [0x40, 0x56, 0x6c],
        slots: [0xa9, 0xdc, 0xfb, 0x11a, 0x133],
    };
    angle_track(g, img, fr, None, &o)
}

/// [`angle_step`].
pub const ANGLE_STEP: Sig = Sig {
    len: 391,
    hash: 0xb735_9d3a_2c6e_ca06,
    calls: &[
        (0x40, Callee::Sig(&EASE_ANGLE)),
        (0x56, Callee::Sig(&EASE_ANGLE)),
        (0x6c, Callee::Sig(&EASE_SAMPLE)),
        (0x17f, Callee::Any),
    ],
    imports: &[],
};

/// [`angle_track`] through the quartic curve (`K1` operand `+0x14`, `K2`
/// `+0x3c`).
///
/// `magic_0162` 0x10005c90 (520 bytes, `ecx`/`edx`; 24 DLLs).
pub fn angle_step_curved<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let o = AngleOps {
        calls: [0xc1, 0xd7, 0xed],
        slots: [0x12a, 0x15d, 0x17c, 0x19b, 0x1b4],
    };
    angle_track(g, img, fr, Some((0x14, 0x3c)), &o)
}

/// [`angle_step_curved`].
pub const ANGLE_STEP_CURVED: Sig = Sig {
    len: 520,
    hash: 0x088c_5e9c_2474_39d6,
    calls: &[
        (0xc1, Callee::Sig(&EASE_ANGLE)),
        (0xd7, Callee::Sig(&EASE_ANGLE)),
        (0xed, Callee::Sig(&EASE_SAMPLE)),
        (0x200, Callee::Any),
    ],
    imports: &[],
};

/// Runs a motion script (internal, `ecx` = the script: `+0x0` the
/// commands, 8 bytes each (an operation and its argument); `+0x4` the
/// actor's index, `+0x8` its model, `+0xc` its character; `+0x10` the
/// current command; `+0x14` done; `u16 +0x18` a wait, `+0x1a` a frame
/// count, `+0x1c` the mode: 0 a model, else a character).
///
/// Done scripts return 1. A wait counts down (the script goes on when it
/// runs out); without one the frame count goes up and the script goes on
/// only when the motion has ended (slot 411 `MsEffectCheckMotion` of the
/// model non-zero, or slot 20 `Ch_GetWaitMotion` of the character zero).
/// Then the commands run from the next one: -2 sets the actor's speed
/// ([`set_speed`]), -3 slot 23 `(chr, 0)`, -4 slot 3 on the actor's
/// character (`G_SELF` or `G_CHRS[G_MAP[i]]`, operands `+0xb1`, `+0xc3`,
/// `+0xb8`), each going on to the next; -5 slot 529 `(*G_X)` and -1 stop
/// there; any other operation starts a motion (mode 0: slot 228 `(model,
/// motion)`; 1: slots 30, 36 and 18 on the character; 2: slots 30, 37
/// `(chr, 1, motion)` and 18; else nothing). The command's argument then
/// says what follows: -7 waits for the motion (the command again, the
/// wait 1 or the motion's answer; a model without one gets motion 0x10),
/// -6, -5, -3, -1 end the script (a model gets slot 213 with 2, 3, 1 or
/// 0), -4 ends it, -2 clears the wait and the count, anything else is the
/// wait. Returns the done word (1 when it ends).
///
/// `magic_0162` 0x100066e0 (640 bytes with its two jump tables, `ecx`;
/// 20 DLLs).
#[allow(clippy::too_many_lines)] // the original's two dispatches
pub fn motion_script<G: Game>(g: &mut G, img: &Image, f: u32, s: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    if g.read::<u32>(s + 0x14) == 1 {
        return 1;
    }
    let wait = g.read::<u16>(s + 0x18);
    if wait != 0 {
        let left = wait - 1;
        g.write::<u16>(s + 0x18, left);
        if left != 0 {
            return g.read::<u32>(s + 0x14);
        }
    } else {
        let n = g.read::<u16>(s + 0x1a).wrapping_add(1);
        g.write::<u16>(s + 0x1a, n);
        let ended = if g.read::<u16>(s + 0x1c) == 0 {
            let t = slot_target(g, w(0x23), 411);
            g.call(t, 0, &[g.read::<u32>(s + 8)]).eax != 0
        } else {
            let t = slot_target(g, w(0x23), 20);
            g.call(t, 0, &[g.read::<u32>(s + 0xc)]).eax == 0
        };
        if !ended {
            return g.read::<u32>(s + 0x14);
        }
    }
    let chr = |g: &G| g.read::<u32>(s + 0xc);
    let (cmd, answer) = loop {
        let i = g.read::<u32>(s + 0x10).wrapping_add(1);
        g.write::<u32>(s + 0x10, i);
        let cmd = g.read::<u32>(s).wrapping_add(8u32.wrapping_mul(i));
        match g.read::<u32>(cmd).cast_signed() {
            -2 => {
                let (index, speed) = (g.read::<u32>(s + 4), g.read::<u32>(cmd + 4));
                set_speed(g, img, img.rel32_target(f + 0x8e), index, speed);
            }
            -3 => {
                let t = slot_target(g, w(0x95), 23);
                g.call(t, 0, &[chr(g), 0]);
            }
            -4 => {
                let i = g.read::<u32>(s + 4);
                let actor = if i == u32::MAX {
                    g.read::<u32>(w(0xb1))
                } else {
                    let map = g.read::<u32>(w(0xb8));
                    let k = u32::from(g.read::<u8>(map.wrapping_add(i)));
                    g.read::<u32>(w(0xc3).wrapping_add(4 * k))
                };
                let t = slot_target(g, w(0xc9), 3);
                g.call(t, 0, &[actor]);
            }
            -5 => {
                let t = slot_target(g, w(0x108), 529);
                g.call(t, 0, &[g.read::<u32>(w(0x10e))]);
                break (cmd, 1);
            }
            -1 => break (cmd, 1),
            _ => {
                let answer = match g.read::<u16>(s + 0x1c).cast_signed() {
                    0 => {
                        let t = slot_target(g, w(0x18b), 228);
                        let (model, motion) = (g.read::<u32>(s + 8), g.read::<u32>(cmd));
                        g.call(t, 0, &[model, motion]).eax
                    }
                    1 => {
                        let c = chr(g);
                        let t = slot_target(g, w(0x158), 30);
                        g.call(t, 0, &[c]);
                        let motion = g.read::<u32>(cmd);
                        let c = chr(g);
                        let t = slot_target(g, w(0x167), 36);
                        g.call(t, 0, &[c, motion, c]);
                        let c = chr(g);
                        let t = slot_target(g, w(0x177), 18);
                        g.call(t, 0, &[c, c, motion, c]).eax
                    }
                    2 => {
                        let c = chr(g);
                        let t = slot_target(g, w(0x125), 30);
                        g.call(t, 0, &[c]);
                        let motion = g.read::<u32>(cmd);
                        let c = chr(g);
                        let t = slot_target(g, w(0x134), 37);
                        g.call(t, 0, &[c, 1, motion, c]);
                        let c = chr(g);
                        let t = slot_target(g, w(0x146), 18);
                        g.call(t, 0, &[c, c, 1, motion, c]).eax
                    }
                    _ => 1,
                };
                break (cmd, answer);
            }
        }
    };
    let arg = g.read::<u32>(cmd + 4);
    let end = |g: &mut G, model_arg: Option<u32>| {
        if let Some(x) = model_arg
            && g.read::<u16>(s + 0x1c) == 0
        {
            let t = slot_target(g, w(0x1eb), 213);
            g.call(t, 0, &[g.read::<u32>(s + 8), x]);
        }
        g.write::<u32>(s + 0x14, 1);
        g.read::<u32>(s + 0x14)
    };
    match arg.cast_signed() {
        -7 => {
            let mut answer = answer;
            if answer.cast_signed() <= 0 {
                answer = 1;
                if g.read::<u16>(s + 0x1c) == 0 {
                    let t = slot_target(g, w(0x216), 228);
                    let a = g.call(t, 0, &[g.read::<u32>(s + 8), 0x10]).eax;
                    if a.cast_signed() > 0 {
                        answer = a;
                    }
                }
            }
            let i = g.read::<u32>(s + 0x10).wrapping_sub(1);
            g.write::<u32>(s + 0x10, i);
            let done = g.read::<u32>(s + 0x14);
            g.write::<u16>(s + 0x18, answer as u16);
            done
        }
        -6 => end(g, Some(2)),
        -5 => end(g, Some(3)),
        -4 => end(g, None),
        -3 => end(g, Some(1)),
        -2 => {
            g.write::<u32>(s + 0x18, 0);
            g.read::<u32>(s + 0x14)
        }
        -1 => end(g, Some(0)),
        _ => {
            g.write::<u16>(s + 0x18, arg as u16);
            g.read::<u32>(s + 0x14)
        }
    }
}

/// [`motion_script`] (the `rel32`s: [`set_speed`] and a jump inside).
pub const MOTION_SCRIPT: Sig = Sig {
    len: 640,
    hash: 0x150f_79fd_9639_53ba,
    calls: &[(0x8e, Callee::Sig(&SET_SPEED))],
    imports: &[],
};

/// A trail of sprites (internal, `ecx` = the trail, `edx` = four matrix
/// rows' floats (`+0x0..+0x28`), the stack word = the newest point; an
/// 16-aligned /GS frame, `E` = `esp` after it and two pushes). The
/// screen and camera matrices (slots 723, 724) go into the frame, the
/// trail's animation steps ([`anim_step`], `+0x0` cursor), `G_P`
/// (operand `+0x66`) points at the sprite matrix `E + 0x78` and `G_X`
/// (`+0x6c`) is 0. The point goes into the ring of 32 (`+0xc`, 16 bytes
/// each, the newest index `+0x220`). Then `+0x224` sprites are placed
/// along the ring, `+0x21c` apart: walking back from the newest point,
/// segment by segment (slot 234 the step, its length by
/// `_libm_sse2_sqrt_precise`; at most 32), each sprite `i` at the
/// distance reached on its segment (slot 248; the segment's start when
/// the distance is past it), coloured from `+0x20c` to `+0x210` by
/// `i / n` (`idiv`, into `G_COL`), scaled by `(K - f) +0x214 + f
/// +0x218` (`f = (i + 1) / n`, `K` operand `+0x320`) on the rows of
/// `edx`, placed with the camera (slot 229) and, when the colour's
/// alpha is not 0, drawn by its handle as [`sprites_run`] draws (slots
/// 772 to 778 with `**G_LOCK` 1 around them). A zero-length segment
/// skips its sprite. Returns 0x1f, or the point's last word when there
/// are no sprites.
///
/// `magic_0018` 0x10002580 (1462 bytes; 20 DLLs).
#[allow(clippy::too_many_lines)] // the original's walk and draw, step by step
pub fn trail_draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (obj, rows) = (fr.ecx, fr.edx);
    // push ebp; and esp, -16; sub esp, 0x138; two pushes
    let e = (fr.esp.wrapping_sub(4) & !15).wrapping_sub(0x138 + 8);
    let at = |o: u32| e.wrapping_add(o);
    let t = slot_target(g, w(0x25), 723);
    g.call(t, 0, &[at(0xf8)]);
    let t = slot_target(g, w(0x47), 724);
    g.call(t, 0, &[at(0xb8)]);
    let entry = anim_step(g, img, img.rel32_target(f + 0x59), obj);
    g.write::<u32>(w(0x66), at(0x78));
    g.write::<u32>(w(0x6c), 0);
    let mut newest = g.read::<u32>(obj + 0x220).wrapping_add(1);
    g.write::<u32>(obj + 0x220, newest);
    if newest.cast_signed() >= 0x20 {
        newest = 0;
        g.write::<u32>(obj + 0x220, 0);
    }
    let point = fr.arg(g, 0);
    let ring = |i: u32| obj + 0xc + 16 * i;
    let mut last = 0;
    for k in 0..4 {
        last = g.read::<u32>(point + 4 * k);
        g.write::<u32>(ring(newest) + 4 * k, last);
    }
    let total = g.read::<i32>(obj + 0x224);
    if total <= 0 {
        return last;
    }
    let (cur_pt, delta, p) = (at(0x60), at(0x50), at(0x40));
    let mut cur = newest;
    let mut walked = 0.0f32;
    let mut seg = 0.0f32;
    let mut left = 0x20i32;
    let mut i = 0u32;
    loop {
        // comiss walked, seg; jb: still on this segment
        if walked >= seg {
            // the next segment back
            // cmovs: a negative index wraps to the last
            let prev = cur.wrapping_sub(1);
            let prev = if prev.cast_signed() < 0 { 0x1f } else { prev };
            walked = sub(walked, seg);
            for k in 0..4 {
                let x = g.read::<u32>(ring(cur) + 4 * k);
                g.write::<u32>(cur_pt + 4 * k, x);
            }
            let t = slot_target(g, w(0x158), 234);
            g.call(t, 0, &[delta, ring(prev), cur_pt]);
            let (dx, dy, dz) = (f32_at(g, delta), f32_at(g, delta + 4), f32_at(g, delta + 8));
            let sq = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
            seg = f64::from(sq).sqrt() as f32;
            cur = prev;
            left -= 1;
            if left <= 0 {
                return 0x1f;
            }
            // ucomiss seg, 0; lahf; test ah, 0x44; jnp: equal skips
            if seg == 0.0 {
                i += 1;
                if i.cast_signed() >= g.read::<i32>(obj + 0x224) {
                    return 0x1f;
                }
                continue;
            }
        }
        for (k, op) in [(0u32, 0x201), (1, 0x22d), (2, 0x259), (3, 0x288)] {
            let n = g.read::<i32>(obj + 0x224);
            let from = i32::from(g.read::<u8>(obj + 0x20c + k));
            let to = i32::from(g.read::<u8>(obj + 0x210 + k));
            let x = from
                .wrapping_mul(n.wrapping_sub(i.cast_signed()))
                .wrapping_add(to.wrapping_mul(i.cast_signed()));
            // idiv: a zero count (or an overflow) faults in the original
            let c = x.checked_div(n).unwrap_or(0);
            g.write::<u8>(w(op), c.cast_unsigned() as u8);
        }
        if walked < seg || walked.is_nan() || seg.is_nan() {
            let r = div(walked, seg);
            let d = [f32_at(g, delta), f32_at(g, delta + 4), f32_at(g, delta + 8)];
            for (k, x) in (0u32..).zip(d) {
                set_f32(g, p + 4 * k, mul(r, x));
            }
            let t = slot_target(g, w(0x2d1), 248);
            g.call(t, 0, &[p, cur_pt, p]);
            walked = add(f32_at(g, obj + 0x21c), walked);
        } else {
            for k in 0..4 {
                let x = g.read::<u32>(cur_pt + 4 * k);
                g.write::<u32>(p + 4 * k, x);
            }
        }
        let n = cvt(g.read::<u32>(obj + 0x224));
        let frac = div(cvt(i.wrapping_add(1)), n);
        let k = f32_at(g, w(0x320));
        let scale = add(
            mul(sub(k, frac), f32_at(g, obj + 0x214)),
            mul(f32_at(g, obj + 0x218), frac),
        );
        set_f32(g, p + 0xc, 1.0);
        let m = at(0x78);
        let t = slot_target(g, w(0x346), 235);
        g.call(t, 0, &[m]);
        for row in 0..3 {
            for col in 0..3 {
                let x = mul(f32_at(g, rows + 0x10 * row + 4 * col), scale);
                set_f32(g, m + 0x10 * row + 4 * col, x);
            }
        }
        let t = slot_target(g, w(0x419), 229);
        g.call(t, 0, &[at(0xa8), at(0xb8), p]);
        if g.read::<u8>(w(0x433)) != 0 {
            let t = slot_target(g, w(0x43f), 941);
            let table = g.call(t, 0, &[g.read::<u32>(obj)]).eax;
            let rec = i32::from(g.read::<u16>(entry).cast_signed())
                .cast_unsigned()
                .wrapping_add(table);
            let t = slot_target(g, w(0x45a), 772);
            let out = g.call(t, 0, &[obj, i]);
            let (lo, hi) = (out.eax, out.edx);
            let lock_at = g.read::<u32>(w(0x46e));
            let saved = g.read::<u32>(lock_at);
            g.write::<u32>(lock_at, 1);
            let t = slot_target(g, w(0x480), 773);
            let known = g.call(t, 0, &[lo, hi, obj, i]).eax;
            let lock_at = g.read::<u32>(w(0x49c));
            g.write::<u32>(lock_at, saved);
            let colour = w(0x4b8);
            if known != 0 {
                let kind = u32::from(g.read::<u16>(obj + 8));
                g.write::<u32>(lock_at, 1);
                let t = slot_target(g, w(0x4b3), 774);
                g.call(t, 0, &[lo, hi, obj, colour, kind]);
                let lock_at = g.read::<u32>(w(0x4d1));
                g.write::<u32>(lock_at, 1);
                let t = slot_target(g, w(0x4df), 775);
                g.call(t, 0, &[lo, hi]);
                let lock_at = g.read::<u32>(w(0x4f1));
                g.write::<u32>(lock_at, saved);
            } else {
                let off = g.read::<u32>(rec + 4);
                let byte = u32::from(g.read::<u8>(off.wrapping_add(rec).wrapping_add(4)));
                let t = slot_target(g, w(0x50e), 776);
                let a = g.call(t, 0, &[byte]).eax;
                let t = slot_target(g, w(0x51c), 777);
                let x = g.read::<u32>(w(0x52b));
                let flags = a | g.call(t, 0, &[x]).eax;
                let kind = u32::from(g.read::<u16>(obj + 8));
                let lock_at = g.read::<u32>(w(0x539));
                let saved = g.read::<u32>(lock_at);
                g.write::<u32>(lock_at, 1);
                let t = slot_target(g, w(0x545), 778);
                g.call(t, 0, &[lo, hi, obj, w(0x54c), kind, flags]);
                let lock_at = g.read::<u32>(w(0x56b));
                g.write::<u32>(lock_at, saved);
            }
        }
        i += 1;
        if i.cast_signed() >= g.read::<i32>(obj + 0x224) {
            return 0x1f;
        }
    }
}

/// [`trail_draw`] (the `rel32`s: [`anim_step`], the square root and the
/// cookie check).
pub const TRAIL_DRAW: Sig = Sig {
    len: 1462,
    hash: 0xd093_6870_caca_e3cf,
    calls: &[
        (0x59, Callee::Sig(&ANIM_STEP)),
        (0x191, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x5ae, Callee::Any),
    ],
    imports: &[],
};

/// An actor's position for a mode (internal, `ecx` = the actor's index,
/// -1 the caster; `edx` = four floats out; the stack word = the mode; a
/// /GS frame): the character (`G_SELF`, or `G_CHRS[G_MAP[i]]`, operands
/// `+0x1f`, `+0x31`, `+0x26`) gives its position (slot 262 into `out`);
/// then by mode: 1 the height of its point 6 (slot 268 into the frame,
/// `[ebp - 0x14]`); 2 the same less the actor's scale
/// ([`actor_scale`]) times `K` (operand `+0xb2`); 3 its ground (slot 12,
/// ST0); 4 or 5 its point 0x10 or 0x11 into `out`; 6 or 7 the same with
/// the ground as the height. `w` is 1.0. Returns the last call's `eax`
/// (the mode less 1 for any other mode).
///
/// `magic_0162` 0x10006030 (336 bytes with its jump table; 11 DLLs).
pub fn actor_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (index, out) = (fr.ecx, fr.edx);
    let mode = fr.arg(g, 0);
    let chr = if index == u32::MAX {
        g.read::<u32>(w(0x1f))
    } else {
        let map = g.read::<u32>(w(0x26));
        let k = u32::from(g.read::<u8>(map.wrapping_add(index)));
        g.read::<u32>(w(0x31).wrapping_add(4 * k))
    };
    let t = slot_target(g, w(0x3a), 262);
    g.call(t, 0, &[chr, out, out + 4, out + 8]);
    let eax;
    let local = fr.esp.wrapping_sub(4 + 0x14);
    // slot 268 `(chr, point, at)`, then slot 12's ground into `out + 4`
    let point = |g: &mut G, op: u32, p: u32, at: u32| {
        let t = slot_target(g, w(op), 268);
        g.call(t, 0, &[chr, p, at]).eax
    };
    let ground = |g: &mut G, op: u32, words: &[u32]| {
        let t = slot_target(g, w(op), 12);
        let o = g.call(t, 0, words);
        g.write::<u32>(out + 4, crate::sa_summon::st0_f32(o));
        o.eax
    };
    match mode {
        1 => {
            eax = point(g, 0x68, 6, local);
            let y = g.read::<u32>(local + 4);
            g.write::<u32>(out + 4, y);
        }
        2 => {
            let (scale, _) = actor_scale(g, img, img.rel32_target(f + 0x8c), index);
            eax = point(g, 0x95, 6, local);
            let y = sub(f32_at(g, local + 4), mul(scale, f32_at(g, w(0xb2))));
            set_f32(g, out + 4, y);
        }
        3 => eax = ground(g, 0xc9, &[chr]),
        4 => eax = point(g, 0xdc, 0x10, out),
        5 => eax = point(g, 0xdc, 0x11, out),
        6 | 7 => {
            let p = if mode == 6 { 0x10 } else { 0x11 };
            point(g, 0xfc, p, out);
            // slot 268's words stay below
            eax = ground(g, 0x10a, &[chr, chr, p, out]);
        }
        // dec eax; cmp eax, 6; ja: the mode less 1 is left
        _ => eax = mode.wrapping_sub(1),
    }
    g.write::<u32>(out + 0xc, 0x3f80_0000);
    Ret::eax(eax)
}

/// [`actor_point`] (the `rel32`s: [`actor_scale`], the cookie check and
/// two jumps inside).
pub const ACTOR_POINT: Sig = Sig {
    len: 336,
    hash: 0x281b_4b1b_d20a_b4d4,
    calls: &[(0x8c, Callee::Sig(&ACTOR_SCALE)), (0x129, Callee::Any)],
    imports: &[],
};

/// The sprites' frame of the `magic_0171` family (a table entry, no
/// arguments; an 8-aligned /GS frame, `E` = `esp` after it and three
/// pushes): as [`sprites_run`] with its own record (0x50 bytes; `b` =
/// the record + 0x14: the position `-0x8..+0x0` stepped by `+0x8..+0x10`,
/// the roll `+0x14` as it is, the scale `+0x18`, the copy's `z` `+0x1c`,
/// a life `+0x20`, the colour `+0x24..+0x33`, the flags byte `+0x38`,
/// live `+0x34`; the animation cursor at `-0x14`, stepped inline as
/// [`anim_step`] does) and the drawing through [`link_part`] (the owner
/// block at operand `+0x2af`, whose `+0x4` is `G_COL` and `+0x1c` `G_P`).
/// Returns the number of live records (also into `G_COUNT`).
///
/// `magic_0171` 0x100035f0 (778 bytes; 12 DLLs).
pub fn sprites_run_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; and esp, -8; sub esp, 0x154; three pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x154 + 12);
    let at = |o: u32| e.wrapping_add(o);
    let list = g.read::<u32>(w(0x1f));
    let t = slot_target(g, w(0x2c), 723);
    g.call(t, 0, &[at(0x118)]);
    let t = slot_target(g, w(0x44), 724);
    g.call(t, 0, &[at(0xd8)]);
    // `lea eax, [esp + 0x5c]` with the argument still pushed
    g.write::<u32>(w(0x55), at(0x58));
    let t = slot_target(g, w(0x62), 235);
    g.call(t, 0, &[at(0x18)]);
    let mut count = 0u32;
    if g.read::<i32>(w(0x7b)) > 0 {
        let mut b = list.wrapping_add(0x14);
        let mut i = 0u32;
        loop {
            if g.read::<u32>(b + 0x34) != 0 {
                count = count.wrapping_add(1);
                sprite_b(g, img, f, &at, b, i);
                let life = g.read::<i32>(b + 0x20);
                if life > 0 {
                    g.write::<i32>(b + 0x20, life - 1);
                    if life == 1 {
                        g.write::<u32>(b + 0x34, 0);
                    }
                }
            }
            i += 1;
            b = b.wrapping_add(0x50);
            if i.cast_signed() >= g.read::<i32>(w(0x2e6)) {
                break;
            }
        }
    }
    g.write::<u32>(w(0x2fd), count);
    Ret::eax(count)
}

/// One record of [`sprites_run_b`].
fn sprite_b<G: Game>(g: &mut G, img: &Image, f: u32, at: &impl Fn(u32) -> u32, b: u32, i: u32) {
    let w = |o: u32| img.word(f + o);
    for (p, v) in [(b - 8, b + 8), (b - 4, b + 0xc), (b, b + 0x10)] {
        let x = add(f32_at(g, v), f32_at(g, p));
        set_f32(g, p, x);
    }
    let scale = g.read::<u32>(b + 0x18);
    for off in [0x40, 0x2c, 0x18] {
        g.write::<u32>(at(off), scale);
    }
    let roll = g.read::<u32>(b + 0x14);
    let t = slot_target(g, w(0xf9), 330);
    g.call(t, 0, &[at(0x58), at(0x18), roll]);
    let t = slot_target(g, w(0x11a), 229);
    g.call(t, 0, &[at(0x88), at(0xd8), b - 8]);
    g.write::<u32>(w(0x12b), 0);
    if g.read::<u8>(b + 0x38) & 1 != 0 {
        let t = slot_target(g, w(0x147), 230);
        g.call(t, 0, &[at(0x98), at(0x58)]);
        let z = add(f32_at(g, b + 0x1c), f32_at(g, at(0xd0)));
        g.write::<u32>(w(0x16c), at(0x98));
        set_f32(g, at(0xd0), z);
    }
    let total = g.read::<i32>(b + 0x24);
    if total > 0 {
        let k = g.read::<i32>(b + 0x28).wrapping_abs();
        for (c, op) in [(0u32, 0x1a5), (1, 0x1c3), (2, 0x1e1), (3, 0x203)] {
            let total = if c == 0 {
                total
            } else {
                g.read::<i32>(b + 0x24)
            };
            let from = i32::from(g.read::<u8>(b + 0x2c + c));
            let to = i32::from(g.read::<u8>(b + 0x30 + c));
            let n = from
                .wrapping_mul(total.wrapping_sub(k))
                .wrapping_add(to.wrapping_mul(k));
            // idiv: a zero total (or an overflow) faults in the original
            let x = n.checked_div(total).unwrap_or(0);
            g.write::<u8>(w(op), x.cast_unsigned() as u8);
        }
        let n = g.read::<u32>(b + 0x28).wrapping_add(1);
        g.write::<u32>(b + 0x28, n);
        if n.cast_signed() > g.read::<i32>(b + 0x24) {
            g.write::<u32>(b + 0x24, 0);
        }
    } else {
        let c = g.read::<u32>(b + 0x30);
        g.write::<u32>(w(0x21f), c);
    }
    let cursor = b - 0x14;
    let entry = crate::sb_common::anim_step_with(g, w(0x224), cursor);
    if g.read::<i32>(b + 0x20) < 0 && g.read::<u32>(b - 0x10) != 1 {
        g.write::<u32>(b + 0x20, 1);
        return;
    }
    // push index; push owner; ecx = the cursor, edx = the frame's entry
    let esp = at(0).wrapping_sub(12);
    g.write::<u32>(esp + 4, w(0x2af));
    g.write::<u32>(esp + 8, i);
    let callee = Frame {
        at: img.rel32_target(f + 0x2b7),
        esp,
        ecx: cursor,
        edx: entry,
        ..Frame::default()
    };
    link_part(g, img, &callee);
}

/// [`sprites_run_b`] (the `rel32`s: [`link_part`] and the cookie check).
pub const SPRITES_RUN_B: Sig = Sig {
    len: 778,
    hash: 0xb47d_67e6_e58b_4868,
    calls: &[(0x2b7, Callee::Sig(&LINK_PART)), (0x302, Callee::Any)],
    imports: &[],
};

/// Where an object of the `magic_0162` family is on the screen
/// (internal, `ecx` = the object; a /GS frame). By its mode `u16 +0x124`:
/// 1 the point of actor `+0x11c` for the mode `+0x120`
/// ([`actor_point`]); 2 and 3 the actor's vector `(+0x120, 0 or the
/// object's +0xe0)` ([`crate::sa_script::actor_vector`]); 4 the record
/// `+0x120` (-1: `G_IDX`, operand `+0x96`) of `G_RECS` (`+0xa2`, 0x148
/// bytes each): its `+0x60` as it is, or placed by its matrix (slot 229)
/// when its byte `+0x146` is set. The point (`[ebp - 0x14]`) goes into
/// `G_V` (operand `+0xfc`, slot 325, `w` 1.0) and through the screen and
/// camera matrices (slots 723, 724, 231) to the screen (slot 565); the
/// object's `+0x114`/`+0x118` take `x / 16 - 0x700` and `y / 16 - 0x730`.
/// Mode 5 runs the callback `+0xf4` `(object)` instead; then (and for any
/// other mode) `G_V` is set back from them: `(x K1, y / K2, 0, 0)`
/// (operands `+0x1c6`, `+0x1ed`). Returns the screen `y` word, the
/// callback's `eax`, or the mode less 1.
///
/// `magic_0162` 0x10008e70 (548 bytes with its jump table; 11 DLLs).
#[allow(clippy::too_many_lines)] // the original's modes
pub fn screen_place<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let obj = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    let mode = i32::from(g.read::<u16>(obj + 0x124).cast_signed()) - 1;
    let gv = w(0xfc);
    let actor = g.read::<u32>(obj + 0x11c);
    let placed = match mode {
        0 => {
            // sub esp, 0xe4; push esi; push the mode word; call
            let esp = ebp.wrapping_sub(0xf0);
            g.write::<u32>(esp + 4, g.read::<u32>(obj + 0x120));
            let callee = Frame {
                at: img.rel32_target(f + 0x3e),
                esp,
                ecx: actor,
                edx: v,
                ..Frame::default()
            };
            actor_point(g, img, &callee);
            true
        }
        1 | 2 => {
            let a = if mode == 1 { 0 } else { obj + 0xe0 };
            let esp = ebp.wrapping_sub(0xf4);
            g.write::<u32>(esp + 4, a);
            g.write::<u32>(esp + 8, v);
            let callee = Frame {
                at: img.rel32_target(f + if mode == 1 { 0x5d } else { 0x81 }),
                esp,
                ecx: actor,
                edx: g.read::<u32>(obj + 0x120),
                ..Frame::default()
            };
            crate::sa_script::actor_vector(g, img, &callee);
            true
        }
        3 => {
            let mut i = g.read::<u32>(obj + 0x120);
            if i == u32::MAX {
                i = g.read::<u32>(w(0x96));
            }
            let rec = i.wrapping_mul(0x148).wrapping_add(w(0xa2));
            if g.read::<u8>(rec + 0x146) == 0 {
                for k in 0..4 {
                    let x = g.read::<u32>(rec + 0x60 + 4 * k);
                    g.write::<u32>(v + 4 * k, x);
                }
            } else {
                let t = slot_target(g, w(0xe3), 229);
                g.call(t, 0, &[v, rec, rec + 0x60]);
            }
            true
        }
        4 => {
            let callback = g.read::<u32>(obj + 0xf4);
            let eax = g.call(callback, obj, &[obj]).eax;
            reset_view(g, img, f, obj);
            return Ret::eax(eax);
        }
        _ => false,
    };
    if !placed {
        reset_view(g, img, f, obj);
        return Ret::eax(mode.cast_unsigned());
    }
    let (screen, camera, m, out) = (
        ebp.wrapping_sub(0xa4),
        ebp.wrapping_sub(0x64),
        ebp.wrapping_sub(0xe4),
        ebp.wrapping_sub(0x24),
    );
    let mut s = Pushes::default();
    s.push_all(&[v, gv]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xf7), 325), 0);
    s.push(screen);
    g.write::<u32>(gv + 0xc, 0x3f80_0000);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x110), 723), 0);
    s.push(camera);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x12b), 724), 0);
    s.push_all(&[camera, screen, m]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x14a), 231), 0);
    s.push_all(&[0, v, m, out]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x168), 565), 0);
    let x = (g.read::<i32>(out) >> 4).wrapping_sub(0x700);
    g.write::<i32>(obj + 0x114, x);
    let y = (g.read::<i32>(out + 4) >> 4).wrapping_sub(0x730);
    g.write::<i32>(obj + 0x118, y);
    Ret::eax(y.cast_unsigned())
}

/// `G_V` from the object's screen place (the tail of [`screen_place`]).
fn reset_view<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32) {
    let w = |o: u32| img.word(f + o);
    let x = mul(cvt(g.read::<u32>(obj + 0x114)), f32_at(g, w(0x1c6)));
    set_f32(g, w(0x1d0), x);
    let y = cvt(g.read::<u32>(obj + 0x118));
    g.write::<u32>(w(0x1e1), 0);
    let y = div(y, f32_at(g, w(0x1ed)));
    g.write::<u32>(w(0x1f3), 0);
    set_f32(g, w(0x1ff), y);
}

/// [`screen_place`] (the `rel32`s: [`actor_point`], two
/// [`crate::sa_script::actor_vector`]s, the cookie check, three jumps
/// inside).
pub const SCREEN_PLACE: Sig = Sig {
    len: 548,
    hash: 0xd65e_f67a_21c8_0115,
    calls: &[
        (0x3e, Callee::Sig(&ACTOR_POINT)),
        (0x5d, Callee::Sig(&crate::sa_script::ACTOR_VECTOR)),
        (0x81, Callee::Sig(&crate::sa_script::ACTOR_VECTOR)),
        (0x1a0, Callee::Any),
        (0x205, Callee::Any),
    ],
    imports: &[],
};

/// The effect's frame step (a table entry, no arguments): nothing before
/// it is ready (`G_READY`, operand `+0x2`, 0 returned). `*G_P = ITEM`
/// (operands `+0xd`, `+0x19`); the screen and camera matrices go to the
/// DLL's (slots 723, 724 with `*G_A`, `*G_B`); `*G_C = G_D`. When slot
/// 261 `(0)` answers, the timeline runs ([`crate::sa_misc::timeline`]);
/// when slot 261 `(1)` answers, the timeline stops
/// ([`crate::sb_misc::timeline_stop_global`]) and slot 214 `(*G_E)`
/// (operand `+0x87`) follows; else the count `G_N` (`+0x9e`) goes up.
/// Returns 1.
///
/// `magic_0255` 0x10001540 (168 bytes; 10 DLLs).
pub fn frame_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    if g.read::<u32>(w(0x2)) == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0xd));
    let a = g.read::<u32>(w(0x13));
    g.write::<u32>(p, w(0x19));
    let mut s = Pushes::default();
    s.push(a);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1e), 723), 0);
    s.push(g.read::<u32>(w(0x31)));
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x2b), 724), 0);
    let c = g.read::<u32>(w(0x3f));
    g.write::<u32>(c, g.read::<u32>(w(0x44)));
    s.push(0);
    // call width: 12 (Pushes caps the words at 12)
    if s.call(g, slot_target(g, w(0x4b), 261), 0).eax != 0 {
        crate::sa_misc::timeline(g, img, img.rel32_target(f + 0x61));
    }
    let t = slot_target(g, w(0x66), 261);
    if g.call(t, 0, &[1]).eax == 0 {
        let n = g.read::<u32>(w(0x9e));
        g.write::<u32>(w(0x9e), n.wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop_global(g, img, img.rel32_target(f + 0x7c));
    let t = slot_target(g, w(0x81), 214);
    g.call(t, 0, &[g.read::<u32>(w(0x87))]);
    Ret::eax(1)
}

/// [`frame_step`].
pub const FRAME_STEP: Sig = Sig {
    len: 168,
    hash: 0x3f08_9d62_2aa9_2b90,
    calls: &[
        (0x61, Callee::Sig(&crate::sa_misc::TIMELINE)),
        (0x7c, Callee::Sig(&crate::sb_misc::TIMELINE_STOP_GLOBAL)),
    ],
    imports: &[],
};

/// A task waiting on its time (cdecl `(task)`, the state `s` at `+0x4`):
/// the time `+0x2c4` grows by `K` (operand `+0x17`). In mode 1 (`+0x2d0`)
/// with flag bit 2 (`+0x108`) the state's four nodes leave their parents
/// ([`crate::sa_misc::leave`]: `+0x120`, `+0x14c`, `+0x178`, `+0x2d8`),
/// the task leaves its ring and the mode is 0xff. In mode 0, once the
/// time reaches `K2` (`+0x99`), slot 239 `(*G, the object's +0x50, 0x20)`
/// starts (the object at `+0x2d4`, `G` operand `+0xb1`) and the mode goes
/// up. Returns the ring's next link, slot 239's `eax`, or the mode less
/// 1 (0 in modes 0 and 1).
///
/// `magic_0414` 0x100069d0 (202 bytes; 10 DLLs).
pub fn timed_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 4);
    let time = add(f32_at(g, s + 0x2c4), f32_at(g, w(0x17)));
    let mode = g.read::<u32>(s + 0x2d0);
    set_f32(g, s + 0x2c4, time);
    if mode == 1 && g.read::<u8>(s + 0x108) & 4 != 0 {
        for off in [0x120, 0x14c, 0x178, 0x2d8] {
            crate::sa_misc::leave(g, s + off, 0);
        }
        let node = g.read::<u32>(task);
        let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
        g.write::<u32>(prev + 8, next);
        let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
        g.write::<u32>(next + 4, prev);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 4, node);
        g.write::<u32>(s + 0x2d0, 0xff);
        return Ret::eax(prev);
    }
    // comiss time, K2; jb: not yet
    if mode != 0 || !(time >= f32_at(g, w(0x99))) {
        return Ret::eax(if mode == 0 { 0 } else { mode.wrapping_sub(1) });
    }
    let obj = g.read::<u32>(s + 0x2d4);
    let t = slot_target(g, w(0xab), 239);
    let x = g.read::<u32>(w(0xb1));
    let eax = g.call(t, 0, &[x, g.read::<u32>(obj + 0x50), 0x20]).eax;
    let m = g.read::<u32>(s + 0x2d0).wrapping_add(1);
    g.write::<u32>(s + 0x2d0, m);
    Ret::eax(eax)
}

/// [`timed_task`] (the `rel32`s: four [`crate::sa_misc::leave`]s).
pub const TIMED_TASK: Sig = Sig {
    len: 202,
    hash: 0xeac4_4b58_7fa5_3ce0,
    calls: &[
        (0x49, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x54, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x5f, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x6a, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};

/// A task node for the state `s` ([`crate::sa_pool::take`] through the
/// `rel32` at `f + call`, its callback the operand at `cb`; `+0x14` the
/// state, with `kind` its byte `+0x18`) appended to the DLL's task list
/// (`head`/`tail` operands). Returns the node.
fn task_for<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (call, cb): (u32, u32),
    s: u32,
    kind: Option<u8>,
    (head, tail): (u32, u32),
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + call), w(cb));
    let empty = g.read::<u32>(w(head)) == 0;
    g.write::<u32>(node + 0x14, s);
    if let Some(k) = kind {
        g.write::<u8>(node + 0x18, k);
    }
    g.write::<u32>(node + 4, 0);
    crate::s3_misc::append_task(g, node, w(head), w(tail), empty);
    node
}

/// Starts a two-colour scale task (internal, `ecx` = the state, `xmm1`,
/// `xmm2` two scales, the stack word a count): a scale not above 0 is
/// `K` (operand `+0x7`); `G_A`/`G_B` (`+0x2d`, `+0x35`) and the state's
/// two vectors `+0x0`, `+0x10` (`w` 1.0) take them; `+0x28` the count,
/// `+0x34` 0, `+0x30`/`+0x2c` 0x1e. A task node (kind 1) runs it. Returns
/// the node.
///
/// `magic_0162` 0x10007ef0 (195 bytes; 9 DLLs).
pub fn scale_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = fr.ecx;
    let k = f32_at(g, w(0x7));
    // comiss 0, x; jb: kept when above 0 (or unordered)
    let keep = |x: f32| if 0.0 < x || x.is_nan() { x } else { k };
    let (a, b) = (keep(fr.xmm_f32(1)), keep(fr.xmm_f32(2)));
    set_f32(g, w(0x2d), a);
    set_f32(g, w(0x35), b);
    for o in [8, 4, 0] {
        set_f32(g, s + o, a);
    }
    g.write::<u32>(s + 0xc, 0x3f80_0000);
    for o in [0x18, 0x14, 0x10] {
        set_f32(g, s + o, b);
    }
    g.write::<u32>(s + 0x1c, 0x3f80_0000);
    g.write::<u32>(s + 0x28, fr.arg(g, 0));
    g.write::<u32>(s + 0x34, 0);
    g.write::<u32>(s + 0x30, 0x1e);
    g.write::<u32>(s + 0x2c, 0x1e);
    Ret::eax(task_for(g, img, f, (0x7d, 0x25), s, Some(1), (0x83, 0x9b)))
}

/// [`scale_task`].
pub const SCALE_TASK: Sig = Sig {
    len: 195,
    hash: 0x4f79_da4c_36e9_c1c2,
    calls: &[(0x7d, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// Starts a state's task (internal, `ecx` = the state, the stack word a
/// value): `+0x20..+0x2c`, `+0x38`, `+0x3c` and the halfwords `+0x40`,
/// `+0x4c` cleared, `+0x34` the value, `+0x30` the step (operand
/// `+0x32`), `+0x44` 1; a task node runs it. Returns the node.
///
/// `magic_0053` 0x10002c30 (150 bytes; 8 DLLs).
pub fn state_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = fr.ecx;
    for o in [0x28, 0x24, 0x20, 0x2c] {
        g.write::<u32>(s + o, 0);
    }
    g.write::<u32>(s + 0x34, fr.arg(g, 0));
    g.write::<u32>(s + 0x30, w(0x32));
    g.write::<u32>(s + 0x38, 0);
    g.write::<u32>(s + 0x3c, 0);
    g.write::<u16>(s + 0x40, 0);
    g.write::<u32>(s + 0x44, 1);
    g.write::<u16>(s + 0x4c, 0);
    Ret::eax(task_for(g, img, f, (0x54, 0xa), s, None, (0x5a, 0x6e)))
}

/// [`state_task`].
pub const STATE_TASK: Sig = Sig {
    len: 150,
    hash: 0x363f_1ddd_6eaa_2c9f,
    calls: &[(0x54, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};
