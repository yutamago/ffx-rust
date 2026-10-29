//! Shared functions without a family module yet, ported by the shared-4
//! list. Each doc names the DLL and address it was ported from, its size
//! and how many DLLs carry it.

use crate::l5_objhost::{POOL_ALLOC, pool_alloc};
use crate::opu_parts::{START_PART, start_part};
use crate::runtime::slot_target;
use crate::sa_misc::{RESET_TRANSFORM, reset_transform};
use crate::sb_common::{ANIM_STEP, anim_step};
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

/// Starts an effect object (internal: `edx` = the object, `xmm3` its
/// scale, the stack `(index, owner, -, owner_b)`): the model words (scale
/// `+0x60..+0x6c`), `*G_P = ITEM` (operands `+0x04`, `+0x59`), the part
/// `list[index]` of slot 941 `(*G_D + 0x10)` started on it
/// ([`start_part`], `0x1000`, 1), its transform reset with the two owners
/// ([`reset_transform`]: `edx` = `owner`, `+0x100` = `owner_b`), then a
/// pool node holding it appended to the object list. Returns the object.
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
    let list = g.call(s941, 0, &[d]).eax;
    let program = g.read::<u32>(list.wrapping_add(index.wrapping_mul(4)));
    start_part(g, img, img.rel32_target(f + 0x84), obj, 0x1000, program, 1);
    // push owner_b; push ecx (not read); call with `edx` = owner
    reset_transform(g, obj, owner, owner_b);
    append_object(
        g,
        img,
        f,
        img.rel32_target(f + 0x9d),
        obj,
        [0xa8, 0xbb, 0xc1, 0xca, 0xd6, 0xdf, 0xe8],
    );
    obj
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
            g.write::<u8>(w(op), c as u8);
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
    let mut live = 0u32;
    if g.read::<i32>(w(0x90)) > 0 {
        let mut b = list.wrapping_add(0x28);
        let mut i = 0u32;
        loop {
            if g.read::<u32>(b + 0x44) != 0 {
                live = live.wrapping_add(1);
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
    g.write::<u32>(w(0xcc), live);
    Ret::eax(live)
}

/// [`sprites_run`] (the `rel32`s: the cookie check and [`anim_step`]).
pub const SPRITES_RUN: Sig = Sig {
    len: 1243,
    hash: 0xff0f_41cd_ae14_c472,
    calls: &[(0xdd, Callee::Any), (0x343, Callee::Sig(&ANIM_STEP))],
    imports: &[],
};
