//! Functions several overlay families share, ported by the shared-B list
//! (batch 2). Each doc names the DLL and address it was ported from, its
//! size and how many DLLs carry it.

#![allow(clippy::similar_names)]

use crate::l5_objhost::{
    LIST_REMOVE, OBJ_BLOCK_INIT, ONE, TASKS_INIT, list_remove, obj_block_init, tasks_init,
};
use crate::m0153::{VECTOR_ANGLE_WY, VECTOR_ANGLE_XZ, vector_angle};
use crate::nodes::walk;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};
use ffx_bridge::x87::cvttsd2si;

fn f32_at<G: Game>(g: &G, a: u32) -> f32 {
    f32::from_bits(g.read::<u32>(a))
}

fn set_f32<G: Game>(g: &mut G, a: u32, x: f32) {
    g.write::<u32>(a, x.to_bits());
}

/// The effect's actors' mean position (`ecx` = the output, four floats,
/// `w` 1.0): for each actor `0..n` (the count at operand `+0x11`, read
/// again after each, `+0x81`; -1 would be the effect's own character,
/// `+0x46`, else the character table `+0x58` at the actor byte table's
/// entry, `+0x4d`) slot 262 `(chr, &x, &y, &z)` writes a position into
/// the frame (`esp - 0x18`, `- 0x14`, `- 0x10`, with `w` = 1.0 at `- 0xc`)
/// and the sums grow (`addss`, the new value first); the sums are divided
/// by `n` (`cvtdq2ps`). Returns `n`.
///
/// `magic_0052` 0x10001d80 (248 bytes, `ecx` = the output, 68 DLLs).
pub fn mean_position<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, out: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let ebp = esp.wrapping_sub(4);
    let (px, py, pz) = (
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x10),
        ebp.wrapping_sub(0xc),
    );
    let (mut sx, mut sy, mut sz) = (0.0f32, 0.0f32, 0.0f32);
    let mut n = g.read::<u32>(w(0x11));
    let mut i = 0u32;
    if n.cast_signed() > 0 {
        loop {
            let chr = if i == u32::MAX {
                g.read::<u32>(w(0x46))
            } else {
                let bytes = g.read::<u32>(w(0x4d));
                let b = u32::from(g.read::<u8>(bytes.wrapping_add(i)));
                g.read::<u32>(w(0x58).wrapping_add(4 * b))
            };
            let s262 = slot_target(g, w(0x6a), 262);
            g.call(s262, out, &[chr, px, py, pz]);
            n = g.read::<u32>(w(0x81));
            i = i.wrapping_add(1);
            g.write::<u32>(ebp.wrapping_sub(8), 0x3f80_0000);
            sx = add(f32_at(g, px), sx);
            sy = add(f32_at(g, py), sy);
            sz = add(f32_at(g, pz), sz);
            if i.cast_signed() >= n.cast_signed() {
                break;
            }
        }
    }
    let d = cvt(n);
    set_f32(g, out, div(sx, d));
    set_f32(g, out + 4, div(sy, d));
    set_f32(g, out + 8, div(sz, d));
    g.write::<u32>(out + 0xc, 0x3f80_0000);
    n
}

/// [`mean_position`].
pub const MEAN_POSITION: Sig = Sig {
    len: 248,
    hash: 0x8633_336c_8e28_46a8,
    // __security_check_cookie
    calls: &[(0xf0, Callee::Any)],
    imports: &[],
};

/// A free record of the DLL's pool (`n` records of 0x74 bytes at the
/// pointer `+0x37`/`+0x61`, `n` at `+0x0f`/`+0x97`), searched round-robin
/// from the saved index (`+0x08`, written back through `+0x105`): the
/// first whose `+0x6c` is 0 is reset (`+0x70` halfword, `+0x58`, `+0x48`,
/// `+0x60`, `+0x54` cleared, `+0x64`/`+0x68` = 0x80808080, `+0x40` = 1.0,
/// `+0x6c` = 1) and returned; 0 when all `n` are taken. Each time the
/// search wraps, the next 512 entries of the DLL's program cache (operand
/// `+0x7f`, from the page number `+0x2b`/`+0x78`, which goes up and is
/// stored through `+0xaa`) are filled with slot 941 `(record +0x30)` of
/// the records from a running index on.
///
/// `magic_0053` 0x10002410 (270 bytes, no arguments, 56 DLLs).
pub fn pool_take<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let mut idx = g.read::<u32>(w(0x08));
    let mut n = g.read::<u32>(w(0x0f));
    let mut l4 = 0u32;
    let mut tried = 0u32;
    if n.cast_signed() <= 0 {
        return 0;
    }
    let mut page = g.read::<u32>(w(0x2b));
    loop {
        let rec = idx.wrapping_mul(0x74).wrapping_add(g.read::<u32>(w(0x37)));
        idx = idx.wrapping_add(1);
        if idx.cast_signed() >= n.cast_signed() {
            let mut esi = page << 9;
            idx = 0;
            if esi.cast_signed() < esi.wrapping_add(0x200).cast_signed() {
                let mut r = l4.wrapping_mul(0x74);
                loop {
                    let t = g.read::<u32>(w(0x61));
                    let key = g.read::<u32>(r.wrapping_add(t).wrapping_add(0x30));
                    let s941 = slot_target(g, w(0x6a), 941);
                    let e = g.call(s941, 0, &[key]).eax;
                    page = g.read::<u32>(w(0x78));
                    g.write::<u32>(w(0x7f).wrapping_add(esi.wrapping_mul(4)), e);
                    esi = esi.wrapping_add(1);
                    l4 = l4.wrapping_add(1);
                    r = r.wrapping_add(0x74);
                    if esi.cast_signed() >= (page.wrapping_add(1) << 9).cast_signed() {
                        break;
                    }
                }
                n = g.read::<u32>(w(0x97));
            }
            page = page.wrapping_add(1);
            g.write::<u32>(w(0xaa), page);
        }
        if g.read::<u32>(rec + 0x6c) == 0 {
            g.write::<u16>(rec + 0x70, 0);
            g.write::<u32>(rec + 0x58, 0);
            g.write::<u32>(rec + 0x48, 0);
            g.write::<u32>(rec + 0x64, 0x8080_8080);
            g.write::<u32>(rec + 0x68, 0x8080_8080);
            g.write::<u32>(rec + 0x60, 0);
            g.write::<u32>(rec + 0x54, 0);
            g.write::<u32>(rec + 0x40, 0x3f80_0000);
            g.write::<u32>(rec + 0x6c, 1);
            g.write::<u32>(w(0x105), idx);
            return rec;
        }
        tried = tried.wrapping_add(1);
        if tried.cast_signed() >= n.cast_signed() {
            return 0;
        }
    }
}

/// [`pool_take`].
pub const POOL_TAKE: Sig = Sig {
    len: 270,
    hash: 0x5e4a_b106_2d3f_3ba9,
    calls: &[],
    imports: &[],
};

/// A node (`ecx`) with its own empty child list (`+0x1a` = 0xff,
/// `+0x10`/`+0xc` itself) appended to the list `edx` (when not 0:
/// `+0x14`, `+0x4`, `+0x8` the old last, the count `+0x18` up, the list's
/// `+0x10` the node), then the DLL's vtable (the immediate at `+0x2f`,
/// `+0x40` on the other path). Returns the old last (the list itself
/// when empty), or the caller's `eax` without a list.
///
/// `magic_0042` 0x10007d90 (69 bytes, `ecx`/`edx`, 53 DLLs).
pub fn node_append<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, list: u32, eax: u32) -> u32 {
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, list);
    if list == 0 {
        g.write::<u32>(node, img.word(f + 0x40));
        return eax;
    }
    g.write::<u32>(node + 4, list);
    g.write::<u32>(node + 8, g.read::<u32>(list + 0x10));
    let last = g.read::<u32>(list + 0x10);
    let n = g.read::<u16>(list + 0x18).wrapping_add(1);
    if last == list {
        g.write::<u16>(list + 0x18, n);
        g.write::<u32>(list + 0xc, node);
        g.write::<u32>(list + 0x10, node);
        g.write::<u32>(node, img.word(f + 0x2f));
    } else {
        g.write::<u32>(last + 4, node);
        let n = g.read::<u16>(list + 0x18).wrapping_add(1);
        g.write::<u16>(list + 0x18, n);
        g.write::<u32>(list + 0x10, node);
        g.write::<u32>(node, img.word(f + 0x40));
    }
    last
}

/// [`node_append`].
pub const NODE_APPEND: Sig = Sig {
    len: 69,
    hash: 0xd97f_80b5_19f3_75b1,
    calls: &[],
    imports: &[],
};

/// The particle set-up of the `magic_0162` family: `pppInitData(part,
/// program, 0)` (slot 711; the part at operand `+0x0a`, the program the
/// immediate `+0x25`) with the game lock (`+0x04`, `+0x2d`) set, then
/// slot 710 `(env, part, buf, 0x40000)` (`env` the immediate `+0x47`,
/// `part` again `+0x3d`, `buf` the stack word; slot 711's words stay on
/// the stack after them), and four of the DLL's words cleared (`+0x58`,
/// `+0x62`, `+0x6c`, `+0x76`). Returns slot 710's `eax`.
///
/// `magic_0162` 0x10009500 (129 bytes, one stack word, 50 DLLs).
pub fn part_setup<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let lock = g.read::<u32>(w(0x04));
    let part = g.read::<u32>(w(0x0a));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s711 = slot_target(g, w(0x18), 711);
    let a = [part, w(0x25), 0];
    g.call(s711, part, &a);
    let lock = g.read::<u32>(w(0x2d));
    let buf = g.read::<u32>(esp + 4);
    g.write::<u32>(lock, saved);
    let part = g.read::<u32>(w(0x3d));
    let s710 = slot_target(g, w(0x42), 710);
    let mut args = vec![w(0x47), part, buf, 0x40000];
    args.extend(a);
    // call width: 7 (slot 711's words stay on the stack)
    let eax = g.call(s710, part, &args).eax;
    for o in [0x58, 0x62, 0x6c, 0x76] {
        g.write::<u32>(w(o), 0);
    }
    eax
}

/// [`part_setup`].
pub const PART_SETUP: Sig = Sig {
    len: 129,
    hash: 0xf986_55ac_e322_b7af,
    calls: &[],
    imports: &[],
};

/// A four-step start-up (the DLL's state word, operand `+0x01`, read
/// again at `+0x31` and stored at `+0x37`/`+0x6f`): state 0 waits for
/// slot 225 `(2)`, 1 for slot 313 `(2)` (each step falls through to the
/// next once its slot says 0), 2 for slot 549 `(2)`, after which slot 226
/// `(G, 0)` runs (`G` the word at `+0x5e`) and the state becomes 4.
/// Returns 0.
///
/// `magic_0071` 0x100093c0 (122 bytes, no arguments, 41 DLLs; a table
/// entry).
pub fn start_steps<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let mut s = g.read::<u32>(w(0x01));
    loop {
        let slot = match s {
            0 => (225, 0x1b),
            1 => (313, 0x0e),
            2 => {
                let s549 = slot_target(g, w(0x41), 549);
                if g.call(s549, fr.ecx, &[2]).eax != 0 {
                    return Ret::eax(0);
                }
                let s226 = slot_target(g, w(0x58), 226);
                let a = [g.read::<u32>(w(0x5e)), 0];
                g.call(s226, fr.ecx, &a);
                g.write::<u32>(w(0x6f), 4);
                return Ret::eax(0);
            }
            _ => return Ret::eax(0),
        };
        let st = slot_target(g, w(slot.1), slot.0);
        if g.call(st, fr.ecx, &[2]).eax != 0 {
            return Ret::eax(0);
        }
        s = g.read::<u32>(w(0x31)).wrapping_add(1);
        g.write::<u32>(w(0x37), s);
    }
}

/// [`start_steps`].
pub const START_STEPS: Sig = Sig {
    len: 122,
    hash: 0x0665_4a18_1131_4913,
    calls: &[],
    imports: &[],
};

/// Slot 439 `(G)` (`G` the DLL's word at operand `+0x07`). Returns 0.
///
/// `magic_0187` 0x100014a0 (25 bytes, no arguments, 40 DLLs; a table
/// entry).
pub fn call_439<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let a = g.read::<u32>(img.word(fr.at + 0x07));
    let s = slot_target(g, img.word(fr.at + 0x01), 439);
    g.call(s, fr.ecx, &[a]);
    Ret::eax(0)
}

/// [`call_439`].
pub const CALL_439: Sig = Sig {
    len: 25,
    hash: 0x476d_b4c9_99c4_2689,
    calls: &[],
    imports: &[],
};

/// Slot 438 `(G)` ([`call_439`]'s twin). Returns 0.
///
/// `magic_0187` 0x100014c0 (25 bytes, no arguments, 40 DLLs; a table
/// entry).
pub fn call_438<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let a = g.read::<u32>(img.word(fr.at + 0x07));
    let s = slot_target(g, img.word(fr.at + 0x01), 438);
    g.call(s, fr.ecx, &[a]);
    Ret::eax(0)
}

/// [`call_438`].
pub const CALL_438: Sig = Sig {
    len: 25,
    hash: 0xa4f6_868c_6126_138d,
    calls: &[],
    imports: &[],
};

/// Stores the DLL's handler (the immediate at operand `+0x0d`) through
/// its pointer global (`+0x04`) and frees the part `(arg)` with
/// `pppAllFreePObject` (slot 713). Returns 1.
///
/// `magic_0020` 0x100016b0 (40 bytes, cdecl `(part)`, 39 DLLs; a table
/// entry).
pub fn free_part_1<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let part = fr.arg(g, 0);
    let p = g.read::<u32>(w(0x04));
    g.write::<u32>(p, w(0x0d));
    let s = slot_target(g, w(0x12), 713);
    g.call(s, fr.ecx, &[part]);
    Ret::eax(1)
}

/// [`free_part_1`].
pub const FREE_PART_1: Sig = Sig {
    len: 40,
    hash: 0x0b88_0c42_7b09_fdcb,
    calls: &[],
    imports: &[],
};

/// A load step: nothing (0) until slot 253 says ready; then, once each,
/// slot 225 `(0x2000)` (waits while it answers non-zero: 1), slot 226
/// `(G, 0)` and slot 227 `(G)` (waits while below 1: 1), each guarded by
/// its own DLL word (`+0x14`, `+0x3c`, `+0x67`; `G` at `+0x4c`/`+0x75`).
/// Returns the last slot's `eax` once everything ran.
///
/// `magic_0045` 0x10001e90 (150 bytes, no arguments, 38 DLLs; a table
/// entry).
pub fn load_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let s253 = slot_target(g, w(0x01), 253);
    let mut eax = g.call(s253, fr.ecx, &[]).eax;
    if eax == 0 {
        return Ret::eax(0);
    }
    if g.read::<u32>(w(0x14)) == 0 {
        let s225 = slot_target(g, w(0x1c), 225);
        eax = g.call(s225, fr.ecx, &[0x2000]).eax;
        if eax != 0 {
            return Ret::eax(1);
        }
        let v = g.read::<u32>(w(0x36)).wrapping_add(1);
        g.write::<u32>(w(0x36), v);
    }
    if g.read::<u32>(w(0x3c)) == 0 {
        let s226 = slot_target(g, w(0x44), 226);
        let a = [g.read::<u32>(w(0x4c)), 0];
        g.write::<u32>(w(0x58), 1);
        eax = g.call(s226, fr.ecx, &a).eax;
    }
    if g.read::<u32>(w(0x67)) != 0 {
        return Ret::eax(eax);
    }
    let s227 = slot_target(g, w(0x6f), 227);
    let a = g.read::<u32>(w(0x75));
    eax = g.call(s227, fr.ecx, &[a]).eax;
    if eax.cast_signed() < 1 {
        return Ret::eax(1);
    }
    let v = g.read::<u32>(w(0x8b)).wrapping_add(1);
    g.write::<u32>(w(0x8b), v);
    Ret::eax(eax)
}

/// [`load_step`].
pub const LOAD_STEP: Sig = Sig {
    len: 150,
    hash: 0xc500_5d85_55c0_a444,
    calls: &[],
    imports: &[],
};

/// `dst = a - b` (`subss`) for each `(a, b, dst)` operand triple.
fn sub_pairs<G: Game>(g: &mut G, img: &Image, f: u32, ops: &[(u32, u32, u32)]) {
    let w = |o: u32| img.word(f + o);
    for &(a, b, d) in ops {
        let x = sub(f32_at(g, w(a)), f32_at(g, w(b)));
        set_f32(g, w(d), x);
    }
}

/// Starts the DLL's camera move (`xmm2` = its time): the eight
/// differences of its start and end points (two quadruples of floats)
/// stored, the flags word (operand `+0x12`) loses bit 0, the time and a
/// cleared word stored. Returns the caller's `eax`.
///
/// `magic_0082` 0x10006710 (218 bytes, `xmm2`, 35 DLLs).
pub fn move_start<G: Game>(g: &mut G, img: &Image, f: u32, x: f32, eax: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let d = sub(f32_at(g, w(0x04)), f32_at(g, w(0x0c)));
    let fl = g.read::<u32>(w(0x12));
    g.write::<u32>(w(0x12), fl & !1);
    set_f32(g, w(0x1b), x);
    g.write::<u32>(w(0x21), 0);
    set_f32(g, w(0x2d), d);
    sub_pairs(
        g,
        img,
        f,
        &[
            (0x35, 0x3d, 0x45),
            (0x4d, 0x55, 0x5d),
            (0x65, 0x6d, 0x75),
            (0x7d, 0x85, 0x8d),
            (0x95, 0x9d, 0xa5),
            (0xad, 0xb5, 0xbd),
            (0xc5, 0xcd, 0xd5),
        ],
    );
    eax
}

/// [`move_start`].
pub const MOVE_START: Sig = Sig {
    len: 218,
    hash: 0x57a5_502b_cb10_eaf2,
    calls: &[],
    imports: &[],
};

/// [`move_start`] with a second time (`xmm3`, stored at operand
/// `+0x4e`) and the flags' bit 2 set. Returns the new flags (`eax`).
///
/// `magic_0082` 0x100067f0 (235 bytes, `xmm2`/`xmm3`, 31 DLLs).
pub fn move_start_2<G: Game>(g: &mut G, img: &Image, f: u32, x: f32, y: f32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let d = sub(f32_at(g, w(0x04)), f32_at(g, w(0x0c)));
    let fl = (g.read::<u32>(w(0x11)) & !1) | 4;
    set_f32(g, w(0x1f), x);
    set_f32(g, w(0x27), d);
    let d = sub(f32_at(g, w(0x2f)), f32_at(g, w(0x37)));
    g.write::<u32>(w(0x3d), 0);
    g.write::<u32>(w(0x46), fl);
    set_f32(g, w(0x4e), y);
    set_f32(g, w(0x56), d);
    sub_pairs(
        g,
        img,
        f,
        &[
            (0x5e, 0x66, 0x6e),
            (0x76, 0x7e, 0x86),
            (0x8e, 0x96, 0x9e),
            (0xa6, 0xae, 0xb6),
            (0xbe, 0xc6, 0xce),
            (0xd6, 0xde, 0xe6),
        ],
    );
    fl
}

/// [`move_start_2`].
pub const MOVE_START_2: Sig = Sig {
    len: 235,
    hash: 0xd5e1_086c_858d_bde2,
    calls: &[],
    imports: &[],
};

/// A smoothed sample of a key table (`ecx` = the floats, `edx` = the key
/// count `n`, the stack word = the stride in floats, `xmm0` = the time
/// `t`): the keys `k[i]` and `k[i + 1]` around `i = trunc(t)` (the last
/// key twice at the end) are each eased toward their outer neighbours by
/// the constants at `+0x39`, `+0x55`, `+0x93`, `+0xa6`, depending on
/// which side of the constant at `+0x39` the fraction is, then mixed by
/// the fraction (constant `+0xd5`). Returns the sample in `xmm0` (`eax`
/// is the original's scratch, which no caller reads).
///
/// `magic_0162` 0x100090a0 (250 bytes, `ecx`/`edx`/`xmm0` and one stack
/// word, 32 DLLs).
pub fn ease_sample<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    tab: u32,
    n: u32,
    t: f32,
) -> f32 {
    let w = |o: u32| img.word(f + o);
    let stride = g.read::<u32>(esp + 4);
    let i = cvttsd2si(f64::from(t));
    let key = |g: &G, j: u32| f32_at(g, tab.wrapping_add(j.wrapping_mul(stride).wrapping_mul(4)));
    let mut x5 = key(g, i);
    let mut x3 = if i == n.wrapping_sub(1) {
        x5
    } else {
        key(g, i.wrapping_add(1))
    };
    let c1 = f32_at(g, w(0x39));
    let frac = sub(t, cvt(i));
    if c1 > frac {
        if i != 0 {
            let x1 = mul(add(key(g, i.wrapping_sub(1)), x3), c1);
            let x6 = sub(c1, frac);
            let x1 = sub(x1, x5);
            let x0 = div(f32_at(g, w(0x55)), x6);
            x5 = add(div(x1, x0), x5);
        }
    } else {
        let j = if i.cast_signed() >= n.wrapping_sub(2).cast_signed() {
            n.wrapping_sub(1)
        } else {
            i.wrapping_add(2)
        };
        let x2 = key(g, j);
        let x1 = sub(frac, f32_at(g, w(0xa6)));
        let x2 = add(x2, x5);
        let x0 = div(f32_at(g, w(0x93)), x1);
        let x2 = sub(mul(x2, c1), x3);
        x3 = add(x3, div(x2, x0));
    }
    #[allow(clippy::float_cmp)] // ucomiss: exactly zero
    if frac == 0.0 {
        return add(0.0, x5);
    }
    let x0 = div(f32_at(g, w(0xd5)), frac);
    add(div(sub(x3, x5), x0), x5)
}

/// [`ease_sample`].
pub const EASE_SAMPLE: Sig = Sig {
    len: 250,
    hash: 0x1b2e_a9a7_2eab_1ca4,
    calls: &[],
    imports: &[],
};

/// `b` brought within `k5` of `a` (`k6` a full turn): `b + k6` when `a`
/// is more than `k5` above it, `b - k6` when more than `k5` below.
fn unwrap_near(a: f32, b: f32, k5: f32, k6: f32) -> f32 {
    if a > b {
        if sub(a, b) > k5 { add(b, k6) } else { b }
    } else if sub(b, a) > k5 {
        sub(b, k6)
    } else {
        b
    }
}

/// [`ease_sample`] for angles: every key is offset by `k5` (operand
/// `+0x07`) and unwrapped toward its neighbour by a turn `k6` (`+0x0f`),
/// the result wrapped back into `-k5..k5` around a whole number of
/// turns. The fraction is stored over the stride's stack word, as the
/// original does. Returns the angle in `xmm0` (`eax` is scratch).
///
/// `magic_0162` 0x100091a0 (456 bytes, `ecx`/`edx`/`xmm0` and one stack
/// word, 31 DLLs).
pub fn ease_angle<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    tab: u32,
    n: u32,
    t: f32,
) -> f32 {
    let w = |o: u32| img.word(f + o);
    let k5 = f32_at(g, w(0x07));
    let k6 = f32_at(g, w(0x0f));
    let stride = g.read::<u32>(esp + 4);
    let i = cvttsd2si(f64::from(t));
    let key = |g: &G, j: u32| {
        add(
            f32_at(g, tab.wrapping_add(j.wrapping_mul(stride).wrapping_mul(4))),
            k5,
        )
    };
    let mut x4 = key(g, i);
    let mut x2 = if i == n.wrapping_sub(1) {
        x4
    } else {
        let b = key(g, i.wrapping_add(1));
        unwrap_near(x4, b, k5, k6)
    };
    let k7 = f32_at(g, w(0x78));
    let frac = sub(t, cvt(i));
    set_f32(g, esp + 4, frac);
    if k7 > frac {
        if i != 0 {
            let x1 = unwrap_near(x4, key(g, i.wrapping_sub(1)), k5, k6);
            let x1 = mul(add(x1, x2), k7);
            let k7b = sub(k7, frac);
            let x1 = sub(x1, x4);
            let x0 = div(f32_at(g, w(0xd3)), k7b);
            x4 = add(div(x1, x0), x4);
        }
    } else {
        let j = if i.cast_signed() >= n.wrapping_sub(2).cast_signed() {
            n.wrapping_sub(1)
        } else {
            i.wrapping_add(2)
        };
        let x3 = unwrap_near(x2, key(g, j), k5, k6);
        let x1 = sub(f32_at(g, esp + 4), f32_at(g, w(0x13e)));
        let x0 = f32_at(g, w(0x146));
        let x3 = add(x3, x4);
        let x0 = div(x0, x1);
        let x3 = sub(mul(x3, k7), x2);
        x2 = add(x2, div(x3, x0));
    }
    let frac = f32_at(g, esp + 4);
    #[allow(clippy::float_cmp)] // ucomiss: exactly zero
    let mut x2 = if frac == 0.0 {
        add(0.0, x4)
    } else {
        let x0 = div(f32_at(g, w(0x17a)), frac);
        add(div(sub(x2, x4), x0), x4)
    };
    let turns = cvttsd2si(f64::from(div(x2, k6)));
    if turns.cast_signed() > 0 {
        x2 = sub(x2, mul(cvt(turns), k6));
    }
    if 0.0 > x2 { add(x2, k5) } else { sub(x2, k5) }
}

/// [`ease_angle`].
pub const EASE_ANGLE: Sig = Sig {
    len: 456,
    hash: 0x94f3_fc0b_f8ce_ebdb,
    calls: &[],
    imports: &[],
};

/// Resets an object's animation block (`ecx`): `+0x114` = `edx`, the
/// words `+0x100..+0x120` and `+0xc0..+0xe8` cleared, `+0x126` cleared,
/// `+0x124` = 0x8000, `+0xdc`, `+0xec`, `+0xf0..+0xfc` = 1.0. Returns
/// 0x8000.
///
/// `magic_0054` 0x10001a70 (266 bytes, `ecx`/`edx`, 28 DLLs).
pub fn anim_reset<G: Game>(g: &mut G, obj: u32, v: u32) -> u32 {
    g.write::<u32>(obj + 0x114, v);
    for o in [0x11c, 0x100, 0x104, 0x108, 0x10c, 0x110, 0x118, 0x120] {
        g.write::<u32>(obj + o, 0);
    }
    g.write::<u8>(obj + 0x126, 0);
    g.write::<u16>(obj + 0x124, 0x8000);
    for o in [0xcc, 0xc8, 0xc4, 0xc0, 0xd8, 0xd4, 0xd0] {
        g.write::<u32>(obj + o, 0);
    }
    g.write::<u32>(obj + 0xdc, 0x3f80_0000);
    for o in [0xe8, 0xe4, 0xe0] {
        g.write::<u32>(obj + o, 0);
    }
    for o in [0xec, 0xfc, 0xf8, 0xf4, 0xf0] {
        g.write::<u32>(obj + o, 0x3f80_0000);
    }
    0x8000
}

/// [`anim_reset`].
pub const ANIM_RESET: Sig = Sig {
    len: 266,
    hash: 0x3203_f7f3_67dd_a384,
    calls: &[],
    imports: &[],
};

/// Script opcode `(ctx, ip)`: takes the part `[ip + 8]` out of the list
/// at `[ip + 4] + 0x30` (`objhost list_remove`, the `rel32` at `+0x15`);
/// when it was there, the owner `[ip + 4]` is stored through the DLL's
/// pointer global (`+0x1e`), the part freed (slot 713) and its flag
/// halfword `+0x124` loses bit 15. Returns `ip + 0xc`.
///
/// `magic_0390` 0x10002fc0 (73 bytes, cdecl `(ctx, ip)`, 26 DLLs; a
/// vtable entry).
pub fn op_free_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ip = fr.arg(g, 1);
    let owner = g.read::<u32>(ip + 4);
    let part = g.read::<u32>(ip + 8);
    let remove = img.rel32_target(fr.at + 0x15);
    if list_remove(g, img, remove, owner + 0x30, part) != 0 {
        let p = g.read::<u32>(img.word(fr.at + 0x1e));
        g.write::<u32>(p, owner);
        let s = slot_target(g, img.word(fr.at + 0x26), 713);
        g.call(s, fr.ecx, &[part]);
        let v = g.read::<u16>(part + 0x124);
        g.write::<u16>(part + 0x124, v & 0x7fff);
    }
    Ret::eax(ip + 0xc)
}

/// [`op_free_part`].
pub const OP_FREE_PART: Sig = Sig {
    len: 73,
    hash: 0xed4f_7858_4951_716c,
    calls: &[(0x15, Callee::Sig(&LIST_REMOVE))],
    imports: &[],
};

/// Runs the effect script (`ecx` = the context: `+0x0` the instruction
/// pointer, `+0x56` the state, `+0x54` a wait counter, `+0x24`/`+0x28`
/// two callbacks): while running (state 1) each opcode (the halfword at
/// `ip`) is dispatched through the DLL's opcode table (operand `+0x17`)
/// as `(ctx, ip)` and returns the next `ip`; then `ip` is saved and the
/// state decides: 0 (done) returns 1; 2 counts the wait down by the
/// game's frame step (the word the pointer global `+0x56` points at;
/// 0xffff waits forever) and runs again when it is used up; 3 calls
/// `+0x24`; states 2 and 3 then call `+0x28` when set. Returns 0 unless
/// done.
///
/// `magic_0162` 0x10007a00 (138 bytes, `ecx` = the context, 24 DLLs).
pub fn script_run<G: Game>(g: &mut G, img: &Image, f: u32, ctx: u32) -> u32 {
    let mut ip = g.read::<u32>(ctx);
    while g.read::<u8>(ctx + 0x56) == 1 {
        let op = u32::from(g.read::<u16>(ip));
        let h = g.read::<u32>(img.word(f + 0x17).wrapping_add(4 * op));
        ip = g.call(h, ip, &[ctx, ip]).eax;
    }
    g.write::<u32>(ctx, ip);
    match g.read::<u8>(ctx + 0x56) {
        0 => return 1,
        2 => {
            let c = u32::from(g.read::<u16>(ctx + 0x54));
            if c != 0xffff {
                let step = g.read::<u32>(g.read::<u32>(img.word(f + 0x56)));
                if c.cast_signed() > step.cast_signed() {
                    g.write::<u16>(ctx + 0x54, c.wrapping_sub(step) as u16);
                } else {
                    g.write::<u16>(ctx + 0x54, 0);
                    g.write::<u8>(ctx + 0x56, 1);
                }
            }
        }
        3 => {
            let cb = g.read::<u32>(ctx + 0x24);
            g.call(cb, ip, &[ctx]);
        }
        _ => return 0,
    }
    let cb = g.read::<u32>(ctx + 0x28);
    if cb != 0 {
        g.call(cb, ip, &[ctx]);
    }
    0
}

/// [`script_run`].
pub const SCRIPT_RUN: Sig = Sig {
    len: 138,
    hash: 0x4d48_b80f_d384_cc3f,
    calls: &[],
    imports: &[],
};

/// Runs a timeline (`ecx`: `+0x0` the time, `+0x4` the event count,
/// `+0x8` how many are live, `+0xc` a live word per event (8 bytes
/// apart), `+0x10c` the events, 0x18 bytes each: start, length (-1: until
/// its tick callback says 0), start, tick and end callbacks, their
/// argument). Each live event whose start has come gets its start
/// callback at the start time, its tick callback every time, and its end
/// callback when it is over (then it is no longer live; the last one
/// ending returns 0 at once). Then, if slot 253 says so, the time moves
/// on. Returns 1 while anything is live, else 0.
///
/// `magic_0020` 0x10002dd0 (211 bytes, `ecx` = the timeline, 22 DLLs).
pub fn timeline_run<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    if g.read::<u32>(this + 8) == 0 {
        return 0;
    }
    let mut ev = g.read::<u32>(this + 0x10c);
    let mut live = this + 0xc;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(this + 4).cast_signed() {
        if g.read::<u32>(live) != 0 {
            let now = g.read::<u32>(this);
            let start = g.read::<u32>(ev);
            if now.cast_signed() >= start.cast_signed() {
                let arg = |g: &G| g.read::<u32>(ev + 0x14);
                let mut r = 0x309;
                if now == start {
                    let cb = g.read::<u32>(ev + 8);
                    if cb != 0 {
                        let a = arg(g);
                        g.call(cb, start, &[a]);
                    }
                }
                let cb = g.read::<u32>(ev + 0xc);
                if cb != 0 {
                    let a = arg(g);
                    r = g.call(cb, start, &[a]).eax;
                }
                let len = g.read::<u32>(ev + 4);
                let over = if len == u32::MAX {
                    r == 0
                } else {
                    g.read::<u32>(this) == g.read::<u32>(ev).wrapping_sub(1).wrapping_add(len)
                };
                if over {
                    let cb = g.read::<u32>(ev + 0x10);
                    if cb != 0 {
                        let a = arg(g);
                        g.call(cb, len, &[a]);
                    }
                    g.write::<u32>(live, 0);
                    let n = g.read::<u32>(this + 8).wrapping_sub(1);
                    g.write::<u32>(this + 8, n);
                    if n == 0 {
                        return 0;
                    }
                }
            }
        }
        i = i.wrapping_add(1);
        live += 8;
        ev = ev.wrapping_add(0x18);
    }
    let s253 = slot_target(g, img.word(f + 0xac), 253);
    if g.call(s253, this, &[]).eax != 0 {
        let t = g.read::<u32>(this).wrapping_add(1);
        g.write::<u32>(this, t);
    }
    1
}

/// [`timeline_run`].
pub const TIMELINE_RUN: Sig = Sig {
    len: 211,
    hash: 0x5aa0_4faf_65ff_8bdc,
    calls: &[],
    imports: &[],
};

/// Sets a sprite record up (`ecx`, `edx` its owner): `+0x8` = -1
/// (halfword), `+0x4` = 1, `+0xa` = 0x200, colour `+0x20c` = 0x80808080,
/// `+0x210`/`+0x224` cleared, `+0x214`/`+0x218` = 1.0, a random seed
/// (the game's `rand() & 0xffff`) at `+0x220` and 32 bytes from `+0x22c`
/// cleared. Returns the seed.
///
/// `magic_0271` 0x10002130 (132 bytes, `ecx`/`edx`, 21 DLLs).
pub fn sprite_init<G: Game>(g: &mut G, obj: u32, owner: u32) -> u32 {
    g.write::<u16>(obj + 8, 0xffff);
    g.write::<u32>(obj, owner);
    g.write::<u32>(obj + 4, 1);
    g.write::<u16>(obj + 0xa, 0x200);
    g.write::<u32>(obj + 0x20c, 0x8080_8080);
    g.write::<u32>(obj + 0x210, 0);
    g.write::<u32>(obj + 0x224, 0);
    g.write::<u32>(obj + 0x214, 0x3f80_0000);
    g.write::<u32>(obj + 0x218, 0x3f80_0000);
    let seed = g.rand() & 0xffff;
    g.write::<u32>(obj + 0x220, seed);
    for i in 0..8 {
        g.write::<u32>(obj + 0x22c + 4 * i, 0);
    }
    seed
}

/// [`sprite_init`].
pub const SPRITE_INIT: Sig = Sig {
    len: 132,
    hash: 0x0ab2_7901_3ff1_6044,
    calls: &[],
    imports: &[(0x50, "rand")],
};

/// Places a node's model: slot 237 `(m, 0, 0, node + 0xd0)` with the
/// matrix slot 219 `(G, 0)` gives (`G` the DLL's word at operand
/// `+0x16`; slot 219's words stay on the stack after them), then the
/// position's `w` (`+0xdc`) = 1.0. Returns slot 237's `eax`.
///
/// `magic_0446` 0x10005740 (66 bytes, cdecl `(node)`, 20 DLLs; a
/// callback the DLL hands the game).
pub fn place_model<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let pos = fr.arg(g, 0).wrapping_add(0xd0);
    let s219 = slot_target(g, w(0x04), 219);
    let a = [g.read::<u32>(w(0x16)), 0];
    let m = g.call(s219, fr.ecx, &a).eax;
    let s237 = slot_target(g, w(0x29), 237);
    let mut args = vec![m, 0, 0, pos];
    args.extend(a);
    // call width: 6 (slot 219's words stay on the stack)
    let eax = g.call(s237, fr.ecx, &args).eax;
    g.write::<u32>(pos + 0xc, 0x3f80_0000);
    Ret::eax(eax)
}

/// [`place_model`].
pub const PLACE_MODEL: Sig = Sig {
    len: 66,
    hash: 0xa643_856c_5b8e_bfa9,
    calls: &[],
    imports: &[],
};

/// An actor's scale (`ecx` = the actor index, -1 for the effect's own
/// character, operand `+0x0e`; else the character table `+0x20` at the
/// actor byte table's entry, `+0x15`): slot 16's float times slot 364's
/// (both `(chr)`, ST0 stored as `f32`). Returns it in `xmm0` (`eax` is
/// slot 16's).
///
/// `magic_0162` 0x10006180 (85 bytes, `ecx` = the index, 19 DLLs).
pub fn actor_scale<G: Game>(g: &mut G, img: &Image, f: u32, index: u32) -> (f32, u32) {
    let w = |o: u32| img.word(f + o);
    let chr = if index == u32::MAX {
        g.read::<u32>(w(0x0e))
    } else {
        let bytes = g.read::<u32>(w(0x15));
        let b = u32::from(g.read::<u8>(bytes.wrapping_add(index)));
        g.read::<u32>(w(0x20).wrapping_add(4 * b))
    };
    let st = |o: crate::Out| o.st0.map_or(f32::from_bits(0xffc0_0000), |v| v as f32);
    let s364 = slot_target(g, w(0x25), 364);
    let a = st(g.call(s364, index, &[chr]));
    let s16 = slot_target(g, w(0x36), 16);
    let r = g.call(s16, index, &[chr]);
    (mul(st(r), a), r.eax)
}

/// [`actor_scale`].
pub const ACTOR_SCALE: Sig = Sig {
    len: 85,
    hash: 0xfb64_bb6c_fe39_d9a6,
    calls: &[],
    imports: &[],
};

/// Looks up the effect's actors: the frame word (operand `+0x01`) copied
/// to `+0x06`, the mode word (`+0x18`) set, the character of the DLL's
/// own actor (slot 413 of the word at `+0x12`) stored at `+0x2e`, then
/// for each of the `n` actor bytes (`n` at `+0x34`/`+0x73`, the bytes at
/// `+0x4a`) its character (slot 215, or 413 while the mode word `+0x42`
/// is set) into the character table (`+0x6d`) at that byte; the byte
/// table's address (`+0x80`) is stored at `+0x7c`. Returns the last
/// slot's `eax`.
///
/// `magic_0197` 0x10001540 (134 bytes, no arguments, 16 DLLs).
pub fn lookup_actors<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    g.write::<u32>(w(0x06), g.read::<u32>(w(0x01)));
    let own = g.read::<u32>(w(0x12));
    g.write::<u32>(w(0x18), 1);
    let s413 = slot_target(g, w(0x0b), 413);
    let mut eax = g.call(s413, 0, &[own]).eax;
    g.write::<u32>(w(0x2e), eax);
    let mut i = 0u32;
    while i.cast_signed()
        < g.read::<u32>(if i == 0 { w(0x34) } else { w(0x73) })
            .cast_signed()
    {
        let slot = if g.read::<u32>(w(0x42)) == 0 {
            215
        } else {
            413
        };
        let b = u32::from(g.read::<u8>(w(0x4a).wrapping_add(i)));
        let s = slot_target(g, w(0x4f), slot);
        eax = g.call(s, i, &[b]).eax;
        i += 1;
        g.write::<u32>(w(0x6d).wrapping_add(4 * b), eax);
    }
    g.write::<u32>(w(0x7c), w(0x80));
    eax
}

/// [`lookup_actors`].
pub const LOOKUP_ACTORS: Sig = Sig {
    len: 134,
    hash: 0x212c_1b03_8dcf_6b4e,
    calls: &[],
    imports: &[],
};

/// Takes a node's model scale out of its matrix: slot 236 `(m, 0, node +
/// 0x80)` reads the matrix slot 219 `(G, 0)` gives into the node (`G` at
/// operand `+0x15`), slot 277 `(m)` its scale `s` (ST0, stored over the
/// argument word as the original does), and the node's three rows
/// (`+0x80`, `+0x90`, `+0xa0`, three floats each) are multiplied by
/// `K / s` (`K` at `+0x4d`); then [`place_model`]'s tail: slot 237 `(m2,
/// 0, 0, node + 0xd0)` with a fresh slot 219 matrix (`+0x109`) and `w` =
/// 1.0. Returns slot 237's `eax`.
///
/// `magic_0446` 0x100013f0 (305 bytes, cdecl `(node)`, 20 DLLs; a
/// callback the DLL hands the game).
pub fn place_model_unscaled<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let s219 = slot_target(g, w(0x04), 219);
    let m = g.call(s219, fr.ecx, &[g.read::<u32>(w(0x15)), 0]).eax;
    let node = fr.arg(g, 0);
    let rows = node + 0x80;
    let s236 = slot_target(g, w(0x20), 236);
    g.call(s236, fr.ecx, &[m, 0, rows]);
    let s277 = slot_target(g, w(0x39), 277);
    let s = g
        .call(s277, fr.ecx, &[m])
        .st0
        .map_or(f32::from_bits(0xffc0_0000), |v| v as f32);
    set_f32(g, fr.esp + 4, s);
    let k = div(f32_at(g, w(0x4d)), f32_at(g, fr.esp + 4));
    let first = mul(k, f32_at(g, rows));
    set_f32(g, rows, first);
    for o in [0x84, 0x88, 0x90, 0x94, 0x98] {
        let x = mul(f32_at(g, node + o), k);
        set_f32(g, node + o, x);
    }
    let x = mul(k, f32_at(g, node + 0xa0));
    set_f32(g, node + 0xa0, x);
    for o in [0xa4, 0xa8] {
        let x = mul(f32_at(g, node + o), k);
        set_f32(g, node + o, x);
    }
    let s219 = slot_target(g, w(0x57), 219);
    let m2 = g.call(s219, fr.ecx, &[g.read::<u32>(w(0x109)), 0]).eax;
    let pos = node + 0xd0;
    let s237 = slot_target(g, w(0x116), 237);
    let eax = g.call(s237, fr.ecx, &[m2, 0, 0, pos]).eax;
    g.write::<u32>(pos + 0xc, 0x3f80_0000);
    Ret::eax(eax)
}

/// [`place_model_unscaled`].
pub const PLACE_MODEL_UNSCALED: Sig = Sig {
    len: 305,
    hash: 0xc8e1_5e04_621b_f3f1,
    calls: &[],
    imports: &[],
};

/// The object host's start (`l5_objhost::start`, the same code up to its
/// slot 221 call) with two more steps: slot 719 `(data, 6, 1)` gives the
/// effect's entry, slot 551 `(entry, 0, 1)` registers it, and the DLL's
/// word at operand `+0x13b` is cleared. Returns 0.
///
/// `magic_0084` 0x100010a0 (327 bytes, cdecl `(arg)`, 19 DLLs; table
/// `+0x0`).
pub fn start_551<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x08), arg);
    obj_block_init(g, img, img.rel32_target(f + 0x0d), arg);
    let s556 = slot_target(g, w(0x12), 556);
    g.call(s556, 0, &[w(0x17)]);
    let s555 = slot_target(g, w(0x24), 555);
    g.call(s555, 0, &[w(0x29), w(0x17)]);
    let s331 = slot_target(g, w(0x36), 331);
    g.call(s331, 0, &[w(0x3b), w(0x29), w(0x17)]);
    let s790 = slot_target(g, w(0x48), 790);
    g.write::<u32>(w(0x57), ONE);
    g.write::<u32>(w(0x61), ONE);
    g.write::<u32>(w(0x6b), ONE);
    let scale = g
        .call(s790, 0, &[])
        .st0
        .map_or(f32::from_bits(0xffc0_0000), |x| x as f32);
    g.write::<f32>(w(0x77), scale);
    tasks_init(g, img, img.rel32_target(f + 0x7c));
    let payloads = w(0x81);
    for i in 0..0x3ff {
        g.write::<u32>(payloads + 12 * i, 0);
    }
    g.write::<u16>(w(0xa3), 0x3ff);
    g.write::<u16>(w(0xab), 0);
    let s720 = slot_target(g, w(0xb0), 720);
    g.call(s720, 0, &[0x3400, 0x800]);
    let data = w(0xcc);
    let s718 = slot_target(g, w(0xc7), 718);
    g.call(s718, 0, &[data, 0x3400, 0x800]);
    let s222 = slot_target(g, w(0xd9), 222);
    g.call(s222, 0, &[w(0xed), 0, 0x3e8, 1, data, 0x3400, 0x800]);
    let s221 = slot_target(g, w(0xf4), 221);
    g.call(
        s221,
        0,
        &[
            w(0x108),
            0,
            0x3e8,
            1,
            w(0xed),
            0,
            0x3e8,
            1,
            data,
            0x3400,
            0x800,
        ],
    );
    let s719 = slot_target(g, w(0x10f), 719);
    let e = g.call(s719, 0, &[w(0x11e), 6, 1]).eax;
    let s551 = slot_target(g, w(0x12a), 551);
    g.call(s551, 0, &[e, 0, 1]);
    g.write::<u32>(w(0x13b), 0);
    Ret::eax(0)
}

/// [`start_551`].
pub const START_551: Sig = Sig {
    len: 327,
    hash: 0xcaa5_806d_3dd0_e4ab,
    calls: &[
        (0x0d, Callee::Sig(&OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&TASKS_INIT)),
    ],
    imports: &[],
};

/// [`effect_reset`]'s stores: (operand, size, value), in the original's
/// order (three records of the DLL's 0x148-byte table reset).
const STORES: [(u32, u8, u32); 59] = [
    (0x5e, 2, 0x0),
    (0x66, 1, 0x0),
    (0x6d, 4, 0x0),
    (0x77, 4, 0x3f80_0000),
    (0x81, 4, 0x3f80_0000),
    (0x8b, 4, 0x0),
    (0x95, 4, 0x0),
    (0x9f, 4, 0x0),
    (0xa9, 4, 0x0),
    (0xb3, 4, 0x0),
    (0xbd, 4, 0x0),
    (0xc7, 4, 0x0),
    (0xd1, 4, 0x0),
    (0xdb, 4, 0x0),
    (0xe5, 4, 0x0),
    (0xef, 4, 0x0),
    (0xf9, 4, 0x0),
    (0x103, 4, 0x0),
    (0x10d, 4, 0x4480_0000),
    (0x118, 2, 0x0),
    (0x120, 1, 0x0),
    (0x127, 4, 0x0),
    (0x131, 4, 0x3f80_0000),
    (0x13b, 4, 0x3f80_0000),
    (0x145, 4, 0x0),
    (0x14f, 4, 0x0),
    (0x159, 4, 0x0),
    (0x163, 4, 0x0),
    (0x16d, 4, 0x0),
    (0x177, 4, 0x0),
    (0x181, 4, 0x0),
    (0x18b, 4, 0x0),
    (0x195, 4, 0x0),
    (0x19f, 4, 0x0),
    (0x1a9, 4, 0x0),
    (0x1b3, 4, 0x0),
    (0x1bd, 4, 0x0),
    (0x1c7, 4, 0x4480_0000),
    (0x1d2, 2, 0x0),
    (0x1da, 1, 0x0),
    (0x1e1, 4, 0x0),
    (0x1eb, 4, 0x3f80_0000),
    (0x1f5, 4, 0x3f80_0000),
    (0x1ff, 4, 0x0),
    (0x209, 4, 0x0),
    (0x213, 4, 0x0),
    (0x21d, 4, 0x0),
    (0x227, 4, 0x0),
    (0x231, 4, 0x0),
    (0x23b, 4, 0x0),
    (0x245, 4, 0x0),
    (0x24f, 4, 0x0),
    (0x259, 4, 0x0),
    (0x263, 4, 0x0),
    (0x26d, 4, 0x0),
    (0x277, 4, 0x0),
    (0x281, 4, 0x4480_0000),
    (0x28b, 4, 0x0),
    (0x295, 4, 0x0),
];

/// Resets the effect's camera state: slot 287 `(0, 0, 7)` gives a
/// handle (stored at operand `+0x14`), slot 283 `(handle, 0)` and
/// `(handle, 1)`, slot 281 `(1)` unless the DLL's mode word (`+0x42`) is
/// set, then three records of the DLL's table reset ([`STORES`]).
/// Returns the last slot's `eax`.
///
/// `magic_0162` 0x100050d0 (670 bytes, no arguments, 23 DLLs).
pub fn effect_reset<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let s287 = slot_target(g, w(0x01), 287);
    let h = g.call(s287, 0, &[0, 0, 7]).eax;
    g.write::<u32>(w(0x14), h);
    let s283 = slot_target(g, w(0x1c), 283);
    g.call(s283, 0, &[h, 0]);
    let s283 = slot_target(g, w(0x29), 283);
    let h = g.read::<u32>(w(0x31));
    let mut eax = g.call(s283, 0, &[h, 1]).eax;
    if g.read::<u32>(w(0x42)) == 0 {
        let s281 = slot_target(g, w(0x4a), 281);
        eax = g.call(s281, 0, &[1]).eax;
    }
    for (o, size, v) in STORES {
        match size {
            1 => g.write::<u8>(w(o), v as u8),
            2 => g.write::<u16>(w(o), v as u16),
            _ => g.write::<u32>(w(o), v),
        }
    }
    eax
}

/// [`effect_reset`].
pub const EFFECT_RESET: Sig = Sig {
    len: 670,
    hash: 0x9302_ec07_1c73_df2c,
    calls: &[],
    imports: &[],
};

/// Draws a model of the `magic_0446` family (`ecx` = the object: `+0x0`
/// the key for slot 941, `+0x8` a halfword; `edx` = its frame entry, a
/// halfword offset into the slot 941 table; the stack word = the draw
/// record): slot 772 `(941(key), 0)` gives the model as `eax`:`edx`;
/// with the game lock (operand `+0x4e`) set slot 773 asks whether it is
/// ready: then slot 774 `(model, obj, rec, obj +0x8)` and slot 775
/// `(model)`, else slot 778 `(model, obj, rec, obj +0x8, 776(byte) |
/// 777(rec +0x1c))` (the byte from the frame entry: `[e + 4 + [e + 4]]`).
/// The lock word is put back. Returns the lock pointer.
///
/// `magic_0446` 0x100057c0 (330 bytes, `ecx`/`edx` and one stack word,
/// 16 DLLs).
pub fn draw_frame<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, obj: u32, entry: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let s941 = slot_target(g, w(0x07), 941);
    let t = g.call(s941, obj, &[g.read::<u32>(obj)]).eax;
    let e = i32::from(g.read::<u16>(entry).cast_signed())
        .cast_unsigned()
        .wrapping_add(t);
    let s941 = slot_target(g, w(0x25), 941);
    let t2 = g.call(s941, e, &[g.read::<u32>(obj)]).eax;
    let s772 = slot_target(g, w(0x40), 772);
    let m = g.call(s772, e, &[t2, 0]);
    let (lo, hi) = (m.eax, m.edx);
    let p = g.read::<u32>(w(0x4e));
    let saved = g.read::<u32>(p);
    g.write::<u32>(p, 1);
    let s773 = slot_target(g, w(0x60), 773);
    let ready = g.call(s773, p, &[lo, hi]).eax;
    let lock = g.read::<u32>(w(0x79));
    g.write::<u32>(lock, saved);
    let rec = g.read::<u32>(esp + 4);
    if ready != 0 {
        g.write::<u32>(lock, 1);
        let n = u32::from(g.read::<u16>(obj + 8));
        let s774 = slot_target(g, w(0x92), 774);
        g.call(s774, saved, &[lo, hi, obj, rec, n]);
        let lock = g.read::<u32>(w(0xa8));
        g.write::<u32>(lock, 1);
        let s775 = slot_target(g, w(0xb6), 775);
        g.call(s775, saved, &[lo, hi]);
        let lock = g.read::<u32>(w(0xc5));
        g.write::<u32>(lock, saved);
        return lock;
    }
    let b = u32::from(g.read::<u8>(g.read::<u32>(e + 4).wrapping_add(e).wrapping_add(4)));
    let s776 = slot_target(g, w(0xe5), 776);
    let x = g.call(s776, e, &[b]).eax;
    let s777 = slot_target(g, w(0xf6), 777);
    let y = g.call(s777, s777, &[g.read::<u32>(rec + 0x1c)]).eax;
    let n = u32::from(g.read::<u16>(obj + 8));
    let lock = g.read::<u32>(w(0x114));
    let saved2 = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s778 = slot_target(g, w(0x12a), 778);
    g.call(s778, n, &[lo, hi, obj, rec, n, x | y]);
    let lock = g.read::<u32>(w(0x13a));
    g.write::<u32>(lock, saved2);
    lock
}

/// [`draw_frame`].
pub const DRAW_FRAME: Sig = Sig {
    len: 330,
    hash: 0x66ab_c9ca_c457_0698,
    calls: &[],
    imports: &[],
};

/// Script opcode `(ctx)` of the `magic_0446` family: when two words of
/// the DLL's state block (the pointer at operand `+0x05`; the
/// displacements in the instructions at `+0x0b`, `+0x11`) are equal, a
/// third (`+0x1c`) is set and the context's state byte `+0x56` becomes 1
/// (returns `ctx`); else returns the first word.
pub fn state_match<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let p = g.read::<u32>(w(0x05));
    let a = g.read::<u32>(p.wrapping_add(w(0x0b)));
    if a != g.read::<u32>(p.wrapping_add(w(0x11))) {
        return Ret::eax(a);
    }
    let ctx = fr.arg(g, 0);
    g.write::<u32>(p.wrapping_add(w(0x1c)), 1);
    g.write::<u8>(ctx + 0x56, 1);
    Ret::eax(ctx)
}

/// [`state_match`] with the words at `+0x8528`..`+0x8530`.
///
/// `magic_0446` 0x10003020 (42 bytes, cdecl `(ctx)`, 15 DLLs).
pub const STATE_MATCH_8528: Sig = Sig {
    len: 42,
    hash: 0xea1a_49e3_17c8_7b22,
    calls: &[],
    imports: &[],
};

/// [`state_match`] with the words at `+0xebe0`..`+0xebe8`.
///
/// `magic_0446` 0x10004980 (42 bytes, cdecl `(ctx)`, 15 DLLs).
pub const STATE_MATCH_EBE0: Sig = Sig {
    len: 42,
    hash: 0xc5df_1fb4_cc9f_006c,
    calls: &[],
    imports: &[],
};

/// A free record of the DLL's pool (`n` at operand `+0x09` records of
/// 0x140 bytes at the pointer `+0x16`), searched round-robin from the
/// saved index (`+0x01`, written back through `+0x5e`): the first whose
/// `+0x130` is 0 is taken (set to 1); one whose halfword `+0x124` lacks
/// bit 15 is returned as it is. Returns the record, or 0.
///
/// `magic_0269` 0x10003490 (102 bytes, no arguments, 14 DLLs).
pub fn pool_take_140<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let mut idx = g.read::<u32>(w(0x01));
    let n = g.read::<u32>(w(0x09));
    if n.cast_signed() <= 0 {
        return 0;
    }
    let base = g.read::<u32>(w(0x16));
    let mut tried = 0u32;
    loop {
        let rec = idx.wrapping_mul(0x140).wrapping_add(base);
        idx = idx.wrapping_add(1);
        if idx.cast_signed() >= n.cast_signed() {
            idx = 0;
        }
        if g.read::<u32>(rec + 0x130) == 0 {
            g.write::<u32>(rec + 0x130, 1);
            g.write::<u32>(w(0x5e), idx);
            return rec;
        }
        if g.read::<u16>(rec + 0x124) & 0x8000 == 0 {
            g.write::<u32>(w(0x5e), idx);
            return rec;
        }
        tried = tried.wrapping_add(1);
        if tried.cast_signed() >= n.cast_signed() {
            return 0;
        }
    }
}

/// [`pool_take_140`].
pub const POOL_TAKE_140: Sig = Sig {
    len: 102,
    hash: 0xf8b6_d00e_2739_639d,
    calls: &[],
    imports: &[],
};

/// [`sprite_init`]'s variant with a given colour: the same header (no
/// seed; `+0x220` cleared) and the 16 bytes at the stack word copied into
/// all 32 entries from `+0xc` (16 bytes apart). Returns the colour's last
/// word.
///
/// `magic_0132` 0x10002640 (127 bytes, `ecx`/`edx` and one stack word,
/// 14 DLLs).
pub fn sprite_init_colour<G: Game>(g: &mut G, esp: u32, obj: u32, owner: u32) -> u32 {
    g.write::<u16>(obj + 8, 0xffff);
    g.write::<u32>(obj, owner);
    let src = g.read::<u32>(esp + 4);
    g.write::<u32>(obj + 4, 1);
    g.write::<u16>(obj + 0xa, 0x200);
    g.write::<u32>(obj + 0x20c, 0x8080_8080);
    g.write::<u32>(obj + 0x210, 0);
    g.write::<u32>(obj + 0x220, 0);
    g.write::<u32>(obj + 0x214, 0x3f80_0000);
    g.write::<u32>(obj + 0x218, 0x3f80_0000);
    let mut eax = 0;
    for k in 0..0x20u32 {
        for j in 0..4 {
            eax = g.read::<u32>(src.wrapping_add(4 * j));
            g.write::<u32>(obj + 0xc + 16 * k + 4 * j, eax);
        }
    }
    eax
}

/// [`sprite_init_colour`].
pub const SPRITE_INIT_COLOUR: Sig = Sig {
    len: 127,
    hash: 0x1650_4d2f_81f5_3f4f,
    calls: &[],
    imports: &[],
};

/// Ends every live event of a timeline ([`timeline_run`]'s layout, `ecx`):
/// each live one gets its end callback (when set) and is no longer live.
/// Returns how many events there are (0 when none is live).
///
/// `magic_0020` 0x10002eb0 (89 bytes, `ecx` = the timeline, 13 DLLs).
pub fn timeline_stop<G: Game>(g: &mut G, this: u32) -> u32 {
    if g.read::<u32>(this + 8) == 0 {
        return 0;
    }
    let mut cb = g.read::<u32>(this + 0x10c).wrapping_add(0x10);
    let mut live = this + 0xc;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(this + 4).cast_signed() {
        if g.read::<u32>(live) != 0 {
            let f = g.read::<u32>(cb);
            if f != 0 {
                let a = g.read::<u32>(cb + 4);
                g.call(f, f, &[a]);
            }
            g.write::<u32>(live, 0);
            let n = g.read::<u32>(this + 8).wrapping_sub(1);
            g.write::<u32>(this + 8, n);
        }
        i += 1;
        live += 8;
        cb = cb.wrapping_add(0x18);
    }
    i
}

/// [`timeline_stop`].
pub const TIMELINE_STOP: Sig = Sig {
    len: 89,
    hash: 0xbbb4_420b_4cf1_6c2a,
    calls: &[],
    imports: &[],
};

/// [`timeline_stop`] on the DLL's own timeline (its words are globals:
/// live count `+0x02`/`+0x43`, event count `+0x0d`/`+0x48`, the events
/// pointer `+0x18`, the live words the immediate `+0x1d`). Returns the
/// event count as last read, or 0 when none is live.
///
/// `magic_0206` 0x100030f0 (91 bytes, no arguments, 12 DLLs).
pub fn timeline_stop_global<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    if g.read::<u32>(w(0x02)) == 0 {
        return 0;
    }
    let mut n = g.read::<u32>(w(0x0d));
    let mut cb = g.read::<u32>(w(0x18)).wrapping_add(0x10);
    let mut live = w(0x1d);
    let mut i = 0u32;
    if n.cast_signed() <= 0 {
        return n;
    }
    loop {
        if g.read::<u32>(live) != 0 {
            let fp = g.read::<u32>(cb);
            if fp != 0 {
                let a = g.read::<u32>(cb + 4);
                g.call(fp, i, &[a]);
            }
            g.write::<u32>(live, 0);
            let c = g.read::<u32>(w(0x43)).wrapping_sub(1);
            g.write::<u32>(w(0x43), c);
            n = g.read::<u32>(w(0x48));
        }
        i += 1;
        live += 8;
        cb = cb.wrapping_add(0x18);
        if i.cast_signed() >= n.cast_signed() {
            return n;
        }
    }
}

/// [`timeline_stop_global`].
pub const TIMELINE_STOP_GLOBAL: Sig = Sig {
    len: 91,
    hash: 0xee82_41f1_9aaf_85e7,
    calls: &[],
    imports: &[],
};

/// [`timeline_stop_global`] as the census cuts it in `magic_0255`
/// (0x10002430, 106 bytes, 12 DLLs): the same code with the CRT's
/// `__security_check_cookie` after it (its `rel32` at `+0x66`).
pub const TIMELINE_STOP_GLOBAL_CRT: Sig = Sig {
    len: 106,
    hash: 0xbbf7_66a8_b4c3_782d,
    calls: &[(0x66, Callee::Any)],
    imports: &[],
};

/// [`static_nodes_init`]'s stores: (destination operand, size, value or
/// its operand, whether the value is an operand), in order.
const STORES_7E90: [(u32, u8, u32, bool); 36] = [
    (0x3, 2, 0xff, false),
    (0xb, 4, 0xf, true),
    (0x15, 4, 0x19, true),
    (0x1f, 4, 0x0, false),
    (0x29, 4, 0x2d, true),
    (0x33, 4, 0x0, false),
    (0x3d, 4, 0x0, false),
    (0x47, 4, 0x0, false),
    (0x51, 4, 0x3f80_0000, false),
    (0x5b, 4, 0x5f, true),
    (0x66, 2, 0xff, false),
    (0x6e, 4, 0x72, true),
    (0x78, 4, 0x7c, true),
    (0x82, 4, 0x0, false),
    (0x8c, 4, 0x0, false),
    (0x96, 4, 0x0, false),
    (0xa0, 4, 0x0, false),
    (0xab, 2, 0xff, false),
    (0xb3, 4, 0xb7, true),
    (0xbd, 4, 0xc1, true),
    (0xc7, 4, 0x0, false),
    (0xd1, 4, 0xd5, true),
    (0xdb, 4, 0x3f80_0000, false),
    (0xe5, 4, 0x3f80_0000, false),
    (0xef, 4, 0x3f80_0000, false),
    (0xf9, 4, 0x3f80_0000, false),
    (0x103, 4, 0x107, true),
    (0x10e, 2, 0xff, false),
    (0x116, 4, 0x11a, true),
    (0x120, 4, 0x124, true),
    (0x12a, 4, 0x0, false),
    (0x134, 4, 0x138, true),
    (0x13e, 4, 0x142, true),
    (0x148, 4, 0x14c, true),
    (0x152, 4, 0x0, false),
    (0x15c, 1, 0x0, false),
];

/// Writes a table of constant stores ([`static_nodes_init`]'s shape).
fn store_table<G: Game>(g: &mut G, img: &Image, f: u32, table: &[(u32, u8, u32, bool)]) {
    for &(d, size, v, op) in table {
        let dst = img.word(f + d);
        let v = if op { img.word(f + v) } else { v };
        match size {
            1 => g.write::<u8>(dst, v as u8),
            2 => g.write::<u16>(dst, v as u16),
            _ => g.write::<u32>(dst, v),
        }
    }
}

/// Sets the DLL's four static nodes up (their own empty lists, vtables,
/// 1.0 scales) and links three of them into a fourth's slots, all as
/// constant stores ([`STORES_7E90`]). Returns the caller's `eax`.
///
/// `magic_0042` 0x10007e90 (354 bytes, no arguments, 14 DLLs).
pub fn static_nodes_init<G: Game>(g: &mut G, img: &Image, f: u32, eax: u32) -> u32 {
    store_table(g, img, f, &STORES_7E90);
    eax
}

/// [`static_nodes_init`].
pub const STATIC_NODES_INIT: Sig = Sig {
    len: 354,
    hash: 0x86c3_1d8a_946c_6882,
    calls: &[],
    imports: &[],
};

/// The model draw `(lo, hi)` with the game lock (`*G_LOCK`) set: slot
/// 779, the lock word put back.
fn draw_locked<G: Game>(g: &mut G, img: &Image, f: u32, ops: [u32; 3], m: crate::Out) -> u32 {
    let lock = g.read::<u32>(img.word(f + ops[0]));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s779 = slot_target(g, img.word(f + ops[1]), 779);
    g.call(s779, lock, &[m.eax, m.edx]);
    let lock = g.read::<u32>(img.word(f + ops[2]));
    g.write::<u32>(lock, saved);
    lock
}

/// Draws the DLL's models: for each of `n` records (0x74 bytes from the
/// pointer `+0x13`, `n` at `+0x0b`/`+0x6e`) slot 772 `(941(+0x30), i)`,
/// then for each of `m` pages (`+0x76`/`+0xde`) 512 table words (from the
/// immediate `+0x85`, one running pointer) slot 772 `(word, j)`; each
/// model drawn with slot 779 under the game lock. Returns the caller's
/// `eax`, the lock pointer after the first loop, or the page count after
/// the second.
///
/// `magic_0132` 0x100038a0 (235 bytes, no arguments, 13 DLLs).
pub fn draw_all<G: Game>(g: &mut G, img: &Image, f: u32, eax: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let mut eax = eax;
    let mut i = 0u32;
    if g.read::<u32>(w(0x0b)).cast_signed() > 0 {
        let mut rec = g.read::<u32>(w(0x13)).wrapping_add(0x30);
        loop {
            let s941 = slot_target(g, w(0x21), 941);
            let t = g.call(s941, i, &[g.read::<u32>(rec)]).eax;
            let s772 = slot_target(g, w(0x35), 772);
            let m = g.call(s772, i, &[t, i]);
            eax = draw_locked(g, img, f, [0x43, 0x4c, 0x5f], m);
            i = i.wrapping_add(1);
            rec = rec.wrapping_add(0x74);
            if i.cast_signed() >= g.read::<u32>(w(0x6e)).cast_signed() {
                break;
            }
        }
    }
    if g.read::<u32>(w(0x76)).cast_signed() > 0 {
        let mut p = w(0x85);
        let mut page = 0u32;
        loop {
            for j in 0..0x200 {
                let s772 = slot_target(g, w(0x93), 772);
                let m = g.call(s772, j, &[g.read::<u32>(p), j]);
                draw_locked(g, img, f, [0xa4, 0xad, 0xc0], m);
                p = p.wrapping_add(4);
            }
            page = page.wrapping_add(1);
            eax = page;
            if page.cast_signed() >= g.read::<u32>(w(0xde)).cast_signed() {
                break;
            }
        }
    }
    eax
}

/// [`draw_all`].
pub const DRAW_ALL: Sig = Sig {
    len: 235,
    hash: 0xaf30_23e1_8d4d_a8ec,
    calls: &[],
    imports: &[],
};

/// Deals a shuffled sequence (`ecx` = the output bytes): `total` bytes
/// (operand `+0x1e`) drawn from `n` kinds (`+0x17`), kind `i` available
/// as often as the word at `+0x46 + 0x1c * i` says. Each draw lays the
/// remaining kinds out in a frame buffer (`ebp - 0x104`, the counts at
/// `ebp - 0x124`), picks the slot 324 random number modulo their total
/// (`idiv`, signed) and takes that one. With `n` at most 1 the output is
/// cleared instead. Returns `total`. (When every count is used up before
/// `total` draws the original divides by zero and faults; the port
/// draws index 0 there.)
///
/// `magic_0269` 0x10002b10 (351 bytes, `ecx` = the output, 12 DLLs).
pub fn deal<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, out: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let ebp = esp.wrapping_sub(4);
    let counts = ebp.wrapping_sub(0x124);
    let pool = ebp.wrapping_sub(0x104);
    let n = g.read::<u32>(w(0x17));
    let total = g.read::<u32>(w(0x1e));
    if n.cast_signed() <= 1 {
        for i in 0..total.cast_signed().max(0).cast_unsigned() {
            g.write::<u8>(out.wrapping_add(i), 0);
        }
        return total;
    }
    for i in 0..n {
        let c = g.read::<u32>(w(0x46).wrapping_add(0x1c * i));
        g.write::<u32>(counts.wrapping_add(4 * i), c);
    }
    let mut k = 0u32;
    while k.cast_signed() < total.cast_signed() {
        let mut len = 0u32;
        for i in 0..n.cast_signed().max(0).cast_unsigned() {
            let c = g.read::<u32>(counts.wrapping_add(4 * i));
            if c.cast_signed() > 0 {
                for j in 0..c {
                    g.write::<u8>(pool.wrapping_add(len).wrapping_add(j), i as u8);
                }
                len = len.wrapping_add(c);
            }
        }
        let s324 = slot_target(g, w(0xce), 324);
        let r = g.call(s324, n, &[]).eax;
        let at = r.cast_signed().checked_rem(len.cast_signed()).unwrap_or(0);
        k += 1;
        let b = g.read::<u8>(pool.wrapping_add(at.cast_unsigned()));
        g.write::<u8>(out.wrapping_add(k - 1), b);
        let kind = i32::from(b.cast_signed()).cast_unsigned();
        let ca = counts.wrapping_add(kind.wrapping_mul(4));
        let c = g.read::<u32>(ca).wrapping_sub(1);
        g.write::<u32>(ca, c);
    }
    total
}

/// [`deal`].
pub const DEAL: Sig = Sig {
    len: 351,
    hash: 0x8356_38da_9c31_3bbf,
    // __security_check_cookie, three times
    calls: &[
        (0x11a, Callee::Any),
        (0x12d, Callee::Any),
        (0x157, Callee::Any),
    ],
    imports: &[],
};

/// Draws each record's models: for each of `n` records (0x50 bytes from
/// the pointer `+0x0f`; `n` at `+0x08`, read again at `+0x77`) and each
/// `j` in `0..n`, slot 772 `(941(record +0x0), j)` drawn with slot 779
/// under the game lock (`+0x53`, `+0x6f`). Returns `n` as last read.
///
/// `magic_0171` 0x10003900 (153 bytes, no arguments, 11 DLLs).
pub fn draw_grid<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let mut n = g.read::<u32>(w(0x08));
    let mut rec = g.read::<u32>(w(0x0f));
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let mut j = 0u32;
        while j.cast_signed() < n.cast_signed() {
            let s941 = slot_target(g, w(0x31), 941);
            let t = g.call(s941, i, &[g.read::<u32>(rec)]).eax;
            let s772 = slot_target(g, w(0x45), 772);
            let m = g.call(s772, i, &[t, j]);
            draw_locked(g, img, f, [0x53, 0x5c, 0x6f], m);
            j += 1;
            n = g.read::<u32>(w(0x77));
        }
        i += 1;
        rec = rec.wrapping_add(0x50);
    }
    n
}

/// [`draw_grid`].
pub const DRAW_GRID: Sig = Sig {
    len: 153,
    hash: 0xade4_92bd_b64d_6292,
    calls: &[],
    imports: &[],
};

/// [`anim_reset`] with the two values on the stack: `+0x11c` and
/// `+0x100` from them, `+0x114` cleared instead. Returns 0x8000.
///
/// `magic_0102` 0x10002030 (272 bytes, `ecx` and two stack words, 11
/// DLLs).
pub fn anim_reset_2<G: Game>(g: &mut G, esp: u32, obj: u32) -> u32 {
    g.write::<u32>(obj + 0x11c, g.read::<u32>(esp + 4));
    g.write::<u32>(obj + 0x100, g.read::<u32>(esp + 8));
    for o in [0x114, 0x104, 0x108, 0x10c, 0x110, 0x118, 0x120] {
        g.write::<u32>(obj + o, 0);
    }
    g.write::<u8>(obj + 0x126, 0);
    g.write::<u16>(obj + 0x124, 0x8000);
    for o in [0xcc, 0xc8, 0xc4, 0xc0, 0xd8, 0xd4, 0xd0] {
        g.write::<u32>(obj + o, 0);
    }
    g.write::<u32>(obj + 0xdc, 0x3f80_0000);
    for o in [0xe8, 0xe4, 0xe0] {
        g.write::<u32>(obj + o, 0);
    }
    for o in [0xec, 0xfc, 0xf8, 0xf4, 0xf0] {
        g.write::<u32>(obj + o, 0x3f80_0000);
    }
    0x8000
}

/// [`anim_reset_2`].
pub const ANIM_RESET_2: Sig = Sig {
    len: 272,
    hash: 0xa90a_acf0_9743_ce05,
    calls: &[],
    imports: &[],
};

/// Shows the character of the matrix slot 219 `(G, 0)` gives
/// (`Ch_SetHideEffect(m, 0)`, slot 257; `G` at operand `+0x09`; slot
/// 219's words stay on the stack). Returns slot 257's `eax`.
///
/// `magic_0446` 0x10005790 (41 bytes, no arguments, 10 DLLs; a vtable
/// entry).
pub fn show_219<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let a = [g.read::<u32>(w(0x09)), 0];
    let s219 = slot_target(g, w(0x01), 219);
    let m = g.call(s219, fr.ecx, &a).eax;
    let s257 = slot_target(g, w(0x19), 257);
    Ret::eax(g.call(s257, fr.ecx, &[m, 0, a[0], 0]).eax)
}

/// [`show_219`].
pub const SHOW_219: Sig = Sig {
    len: 41,
    hash: 0xbe05_fdba_c4e2_aa14,
    calls: &[],
    imports: &[],
};

/// Slot 220 `(719(ecx, 6, edx), a, b)` (the two stack words; slot 719's
/// words stay on the stack after them). Returns slot 220's `eax`.
///
/// `magic_0565` 0x10008610 (45 bytes, `ecx`/`edx` and two stack words,
/// 10 DLLs).
pub fn call_220<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, ecx: u32, edx: u32) -> u32 {
    let s719 = slot_target(g, img.word(f + 0x04), 719);
    let e = g.call(s719, ecx, &[ecx, 6, edx]).eax;
    let s220 = slot_target(g, img.word(f + 0x1c), 220);
    let a = [
        e,
        g.read::<u32>(esp + 4),
        g.read::<u32>(esp + 8),
        ecx,
        6,
        edx,
    ];
    g.call(s220, ecx, &a).eax
}

/// [`call_220`].
pub const CALL_220: Sig = Sig {
    len: 45,
    hash: 0x495d_4fab_9820_e41d,
    calls: &[],
    imports: &[],
};

/// Slot 628 `(0)`; when the DLL's halfword (operand `+0x15`) is 0, slot
/// 213 `(G, 0)` (`G` at `+0x25`) and the DLL's flag (`+0x36`) set.
/// Returns the last slot's `eax`.
///
/// `magic_0565` 0x10006dd0 (63 bytes, no arguments, 10 DLLs).
pub fn call_628_213<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let s628 = slot_target(g, w(0x01), 628);
    let eax = g.call(s628, 0, &[0]).eax;
    if g.read::<u16>(w(0x15)) != 0 {
        return eax;
    }
    let s213 = slot_target(g, w(0x1d), 213);
    let eax = g.call(s213, 0, &[g.read::<u32>(w(0x25)), 0]).eax;
    g.write::<u32>(w(0x36), 1);
    eax
}

/// [`call_628_213`].
pub const CALL_628_213: Sig = Sig {
    len: 63,
    hash: 0xae75_8e7c_dbb5_6292,
    calls: &[],
    imports: &[],
};

/// Frees the DLL's own part when its list holds it: `objhost
/// list_remove(list, part)` (the `rel32` at `+0x15`; the list the
/// immediate `+0x10`, the part the DLL's block (pointer `+0x03`) plus
/// 0x1300); then the handler (immediate `+0x25`) is stored through the
/// pointer global `+0x1e`, the part freed (slot 713) and its halfword
/// `+0x124` loses bit 15. Returns 0x7fff, or 0 when the list did not
/// hold it.
///
/// `magic_0446` 0x10004080 (71 bytes, no arguments, 10 DLLs; a vtable
/// entry).
pub fn free_own_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let part = g.read::<u32>(w(0x03)).wrapping_add(0x1300);
    let remove = img.rel32_target(fr.at + 0x15);
    if list_remove(g, img, remove, w(0x10), part) == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0x1e));
    g.write::<u32>(p, w(0x25));
    let s713 = slot_target(g, w(0x2a), 713);
    g.call(s713, fr.ecx, &[part]);
    let v = g.read::<u16>(part + 0x124);
    g.write::<u16>(part + 0x124, v & 0x7fff);
    Ret::eax(0x7fff)
}

/// [`free_own_part`].
pub const FREE_OWN_PART: Sig = Sig {
    len: 71,
    hash: 0x3773_65f2_492c_2388,
    calls: &[(0x15, Callee::Sig(&LIST_REMOVE))],
    imports: &[],
};

/// Starts the effect's sequence: four DLL words set (`+0x18` = 10,
/// `+0x22`, `+0x2c`, `+0x36` cleared), slot 312 `(fn, 0, 0, 0)` (`fn` the
/// immediate `+0x12`) gives a handle stored at `+0x41`, then slot 371
/// `(0, 0)` and `(1, 0)`. Returns the last slot's `eax`.
///
/// `magic_0565` 0x10001650 (107 bytes, no arguments, 10 DLLs; a table
/// entry).
pub fn start_seq<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    g.write::<u32>(w(0x18), 0xa);
    g.write::<u32>(w(0x22), 0);
    g.write::<u32>(w(0x2c), 0);
    g.write::<u32>(w(0x36), 0);
    let s312 = slot_target(g, w(0x01), 312);
    let h = g.call(s312, fr.ecx, &[w(0x12), 0, 0, 0]).eax;
    g.write::<u32>(w(0x41), h);
    let s371 = slot_target(g, w(0x46), 371);
    g.call(s371, fr.ecx, &[0, 0]);
    let s371 = slot_target(g, w(0x57), 371);
    Ret::eax(g.call(s371, fr.ecx, &[1, 0]).eax)
}

/// [`start_seq`].
pub const START_SEQ: Sig = Sig {
    len: 107,
    hash: 0x226f_dde8_2854_5472,
    calls: &[],
    imports: &[],
};

/// Sets the light ranges of the matrix slot 219 `(G, 0)` gives (`G` at
/// operand `+0x10`): slot 357 `(m, -1, 0x3c00)`, slot 356 `(m, -1,
/// 0x2ee0)`, slot 362 `(H, m, 1, 0)` (`H` at `+0x55`), slot 357 and 356
/// `(m, -1, -1)`. Returns the last slot's `eax`.
///
/// `magic_0446` 0x10004be0 (132 bytes, no arguments, 10 DLLs).
pub fn light_ranges<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let s219 = slot_target(g, w(0x01), 219);
    let m = g.call(s219, 0, &[g.read::<u32>(w(0x10)), 0]).eax;
    let s357 = slot_target(g, w(0x18), 357);
    g.call(s357, s357, &[m, u32::MAX, 0x3c00]);
    let s356 = slot_target(g, w(0x2f), 356);
    g.call(s356, s357, &[m, u32::MAX, 0x2ee0]);
    let s362 = slot_target(g, w(0x44), 362);
    g.call(s362, s357, &[g.read::<u32>(w(0x55)), m, 1, 0]);
    let s357 = slot_target(g, w(0x5c), 357);
    g.call(s357, s357, &[m, u32::MAX, u32::MAX]);
    let s356 = slot_target(g, w(0x6e), 356);
    g.call(s356, s357, &[m, u32::MAX, u32::MAX]).eax
}

/// [`light_ranges`].
pub const LIGHT_RANGES: Sig = Sig {
    len: 132,
    hash: 0x6a52_53de_4a50_6c78,
    calls: &[],
    imports: &[],
};

/// Picks the effect's variant: the DLL's entry table (operand `+0x16`)
/// at slot 646 `(G)` (`G` at `+0x07`) into `+0x1e`; one slot in use
/// (`+0x24` = 1, `+0x2e`/`+0x38` cleared), the first word of each slot
/// record (0x78 bytes from the immediate `+0x43`; the count read at
/// `+0x5c`) cleared; slot 312 `(fn, 0, 0, 0)` (`fn` the immediate
/// `+0x74`) gives a handle stored at `+0x7e`. Returns the handle.
///
/// `magic_0565` 0x100017e0 (131 bytes, no arguments, 10 DLLs).
pub fn pick_variant<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let s646 = slot_target(g, w(0x01), 646);
    let e = g.call(s646, 0, &[g.read::<u32>(w(0x07))]).eax;
    let v = g.read::<u32>(w(0x16).wrapping_add(e.wrapping_mul(4)));
    g.write::<u32>(w(0x1e), v);
    g.write::<u32>(w(0x24), 1);
    g.write::<u32>(w(0x2e), 0);
    g.write::<u32>(w(0x38), 0);
    let mut rec = w(0x43);
    let mut i = 0u32;
    loop {
        g.write::<u32>(rec, 0);
        i = i.wrapping_add(1);
        rec = rec.wrapping_add(0x78);
        if i.cast_signed() >= g.read::<u32>(w(0x5c)).cast_signed() {
            break;
        }
    }
    let s312 = slot_target(g, w(0x63), 312);
    let h = g.call(s312, i, &[w(0x74), 0, 0, 0]).eax;
    g.write::<u32>(w(0x7e), h);
    h
}

/// [`pick_variant`].
pub const PICK_VARIANT: Sig = Sig {
    len: 131,
    hash: 0x88c0_a393_6a42_f39f,
    calls: &[],
    imports: &[],
};

/// Reads the two pads: for pad `p` in 0..2 and each port from `1 - r`
/// (`r` from slot 305) up to 0, slots 302, 303 and 304 `(p, 0, port)`
/// are OR-ed into three halfwords
/// stored in the DLL's tables (operands `+0xa3`, `+0xae`, `+0xb6`, at
/// `2 * p`). Returns the original's `eax` (the last halfword in the low
/// half of the previous result).
///
/// `magic_0565` 0x1000a310 (203 bytes, no arguments, 10 DLLs).
pub fn read_pads<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let s305 = slot_target(g, w(0x07), 305);
    let mut eax = g.call(s305, 0, &[]).eax;
    let first = 1u32.wrapping_sub(eax);
    for p in 0..2u32 {
        let mut port = first;
        let (mut a, mut b, mut c) = (0u16, 0u16, 0u16);
        eax &= 0xffff_0000;
        if port.cast_signed() <= 0 {
            loop {
                let s302 = slot_target(g, w(0x52), 302);
                a |= g.call(s302, p, &[p, 0, port]).eax as u16;
                let s303 = slot_target(g, w(0x67), 303);
                b |= g.call(s303, p, &[p, 0, port]).eax as u16;
                let s304 = slot_target(g, w(0x7c), 304);
                let r = g.call(s304, p, &[p, 0, port]).eax;
                port = port.wrapping_add(1);
                c |= r as u16;
                if port.cast_signed() > 0 {
                    eax = (r & 0xffff_0000) | u32::from(b);
                    break;
                }
            }
        }
        g.write::<u16>(w(0xa3).wrapping_add(2 * p), a);
        g.write::<u16>(w(0xae).wrapping_add(2 * p), eax as u16);
        g.write::<u16>(w(0xb6).wrapping_add(2 * p), c);
    }
    eax
}

/// [`read_pads`].
pub const READ_PADS: Sig = Sig {
    len: 203,
    hash: 0xb07e_e06e_ad3e_f9d8,
    calls: &[],
    imports: &[],
};

/// The walk (from the frame cookie's `eax`; `B` = `((esp - 4) & !7) -
/// 0x30`, the frame after its pushes), then the owner's nodes `a`
/// (`+0x2c`) and `b` (`+0x30`) visited with a cursor at `B + 0xc` and
/// slot 234 `(V, a + 0x1c, b + 0x1c)` filling a vector `V` at `B + 0x1c`.
/// Its horizontal length `sqrt(x*x + z*z)` (MSVCR110's sqrt, exact) goes
/// into `V.w`; the pitch (`m0153` vector angle on `+0xc`/`+0x4`, the
/// `rel32` at `+0xef`) and `K3 -` the yaw (`+0x0`/`+0x8`, `+0x110`) are
/// scaled to angle units (`* K1 / K2`, `cvttss2si`; constants at `+0xf7`,
/// `+0x103`, `+0x118`, `+0x137`, `+0x13f`) into three frame words (the
/// third 0). For each axis `k` (2, 1, 0) the word gains the owner's
/// `+0x34 + 4k`, and the parent's word `+0x1c + 4k` moves toward it by
/// the owner's float `+0x40 + 4k` times the halfword difference. Returns
/// the last step (`eax`), or the walk's `eax`.
///
/// `magic_0042` 0x10004fe0 (446 bytes, thiscall `ecx` = the cursor, 10
/// DLLs).
pub fn aim_parent<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, this: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let e = (esp.wrapping_sub(4) & !7).wrapping_sub(0x24);
    let b = e.wrapping_sub(0xc);
    let owner0 = g.read::<u32>(this);
    let eax = g.read::<u32>(w(0x0a)) ^ e;
    let (eax, tail) = walk(g, this, owner0, eax);
    let Some(o) = tail else {
        return eax;
    };
    let cursor = b + 0xc;
    let pass = g.read::<u8>(this + 5);
    let na = g.read::<u32>(o + 0x2c);
    let parent = g.read::<u32>(o + 0x14);
    g.write::<u32>(cursor, na);
    g.write::<u8>(cursor + 5, pass);
    let m = g.read::<u32>(g.read::<u32>(na) + 0x1c);
    g.call(m, cursor, &[cursor]);
    let pass = g.read::<u8>(this + 5);
    let nb = g.read::<u32>(o + 0x30);
    g.write::<u32>(cursor, nb);
    g.write::<u8>(cursor + 5, pass);
    let m = g.read::<u32>(g.read::<u32>(nb) + 0x1c);
    g.call(m, cursor, &[cursor]);
    let v = b + 0x1c;
    let pb = g.read::<u32>(o + 0x30) + 0x1c;
    let pa = g.read::<u32>(o + 0x2c) + 0x1c;
    let s234 = slot_target(g, w(0xb1), 234);
    g.call(s234, cursor, &[v, pa, pb]);
    let z = f32_at(g, v + 8);
    let x = f32_at(g, v);
    let h = add(mul(z, z), mul(x, x));
    let d = f64::from(h).sqrt() as f32;
    set_f32(g, v + 0xc, d);
    let pitch = vector_angle(g, img, img.rel32_target(f + 0xef), v, true);
    let a1 = cvttsd2si(f64::from(div(
        mul(pitch, f32_at(g, w(0xf7))),
        f32_at(g, w(0x103)),
    )));
    g.write::<u32>(cursor, a1);
    let yaw = vector_angle(g, img, img.rel32_target(f + 0x110), v, false);
    let x1 = sub(f32_at(g, w(0x118)), yaw);
    g.write::<u32>(cursor + 8, 0);
    let x1 = div(mul(x1, f32_at(g, w(0x137))), f32_at(g, w(0x13f)));
    g.write::<u32>(cursor + 4, cvttsd2si(f64::from(x1)));
    let mut eax = 0;
    for k in (0..3u32).rev() {
        let slot = cursor + 4 * k;
        let v = g
            .read::<u32>(slot)
            .wrapping_add(g.read::<u32>(o + 0x34 + 4 * k));
        g.write::<u32>(slot, v);
        let cur = i32::from(g.read::<u16>(slot).cast_signed());
        let p = parent + 0x1c + 4 * k;
        let diff = cur - i32::from(g.read::<u16>(p).cast_signed());
        let step = mul(f32_at(g, o + 0x40 + 4 * k), cvt(diff.cast_unsigned()));
        eax = cvttsd2si(f64::from(step));
        let n = g.read::<u32>(p).wrapping_add(eax);
        g.write::<u32>(p, n);
    }
    eax
}

/// [`aim_parent`].
pub const AIM_PARENT: Sig = Sig {
    len: 446,
    hash: 0x2441_3768_6f17_938f,
    calls: &[
        (0xdc, Callee::Import("_libm_sse2_sqrt_precise")),
        (0xef, Callee::Sig(&VECTOR_ANGLE_WY)),
        (0x110, Callee::Sig(&VECTOR_ANGLE_XZ)),
        // __security_check_cookie
        (0x1b6, Callee::Any),
    ],
    imports: &[],
};
