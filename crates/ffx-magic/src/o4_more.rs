//! More single functions of the own work list (round 4): slot loops and
//! node set-ups on the DLL's globals. Operands are read from the code
//! (`w(off)` = the word at `+off`).

use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::sa_pool::take;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `this` in `ecx` (`magic_0165` 0x10005490, 83 bytes): for `i` in
/// `0..0xa0`, slot 772 (`+0x18`) with `(this, i)`; then, with the word
/// at `*B` (`B` at `+0x21`) set to 1, slot 779 (`+0x36`) with the first
/// slot's `eax`, its `edx` and the words still on the stack, the old word
/// put back after. Returns `*B` (the last one read).
pub fn slot_pair_loop<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let this = fr.ecx;
    let mut r = fr.eax;
    for i in 0..w(0x49) {
        let s = slot_target(g, w(0x11), w(0x18) / 4);
        let o = g.call(s, this, &[this, i]);
        let p = g.read::<u32>(w(0x21));
        let old = g.read::<u32>(p);
        g.write::<u32>(p, 1);
        let s = slot_target(g, w(0x2a), w(0x36) / 4);
        g.call(s, p, &[o.eax, o.edx, this, i]);
        r = g.read::<u32>(w(0x3d));
        g.write::<u32>(r, old);
    }
    Ret::eax(r)
}

/// No arguments (`magic_0165` 0x10002ff0, 90 bytes): the object `*O`
/// (`O` at `+0x03`) gets `+0x5f18` = 0x14 and `+0x5ee4` a pool node
/// ([`take`], `+0x17`, the owner the immediate at `+0x08`); the node's `+0x4` 0,
/// `+0x8` itself, put at the head `H` (`+0x34`) of the list whose count
/// word `N` is `+0x1d` (when `N` is 0: `+0x0` 0 and `N`, `H` the node).
/// Returns the node.
pub fn object_list_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let o = g.read::<u32>(w(0x03));
    g.write::<u32>(o.wrapping_add(w(0x0e)), w(0x12));
    let node = take(g, img, img.rel32_target(f + 0x17), w(0x08));
    let n = g.read::<u32>(w(0x1d));
    g.write::<u32>(o.wrapping_add(w(0x24)), node);
    g.write::<u32>(node.wrapping_add(4), 0);
    push_head(g, node, n == 0, (w(0x34), w(0x3e)), (w(0x4d), w(0x52)));
    Ret::eax(node)
}

/// A node put at the head of a list: when the list is empty, `+0x0` 0
/// and the words at `n_set` and `h_empty` the node; else `+0x0` the head
/// `*h`, the head's `+0x4` the node and `*h_set` the node. Then `+0x8`
/// the node itself.
fn push_head(
    g: &mut impl crate::Mem,
    node: u32,
    empty: bool,
    (h, h_set): (u32, u32),
    (n_set, h_empty): (u32, u32),
) {
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(n_set, node);
        g.write::<u32>(h_empty, node);
    } else {
        let head = g.read::<u32>(h);
        g.write::<u32>(node, head);
        g.write::<u32>(head.wrapping_add(4), node);
        g.write::<u32>(h_set, node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
}

/// `(a)` (`magic_0270` 0x10001b80, 80 bytes): a pool node ([`take`],
/// `+0x09`, the owner the immediate at `+0x04`), its `+0x14` = `a[0x2c]`
/// (`+0x19`) and `+0x4` 0, put at the head of the list (count word
/// `+0x0f`, head `+0x28`/`+0x32`, empty `+0x42`/`+0x47`). Returns the
/// node.
pub fn arg_list_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let node = take(g, img, img.rel32_target(f + 0x09), w(0x04));
    let n = g.read::<u32>(w(0x0f));
    let a = fr.arg(g, 0);
    let v = g.read::<u32>(a.wrapping_add(disp8(img, f + 0x19)));
    g.write::<u32>(node.wrapping_add(0x14), v);
    g.write::<u32>(node.wrapping_add(4), 0);
    push_head(g, node, n == 0, (w(0x28), w(0x32)), (w(0x42), w(0x47)));
    Ret::eax(node)
}

/// `(p)` (`magic_0301` 0x100022d0, 80 bytes): for `i` below the count at
/// `*A + 0x19c` (`A` at `+0x04`/`+0x33`, the count read again each
/// time), slot 446 (`+0x2c`) with `((*A)[0x1a0 + 4 i], 0)`; then the
/// word at `p` or 4. Returns `p`.
pub fn slot_each_flag<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut a = g.read::<u32>(w(0x04));
    let mut i = 0u32;
    let mut at = w(0x15);
    if g.read::<u32>(a.wrapping_add(w(0x0d))).cast_signed() > 0 {
        loop {
            let v = g.read::<u32>(at.wrapping_add(a));
            let s = slot_target(g, w(0x26), w(0x2c) / 4);
            g.call(s, fr.ecx, &[v, 0]);
            a = g.read::<u32>(w(0x33));
            i += 1;
            at = at.wrapping_add(disp8(img, f + 0x3d));
            if i.cast_signed() >= g.read::<u32>(a.wrapping_add(w(0x40))).cast_signed() {
                break;
            }
        }
    }
    let p = fr.arg(g, 0);
    let x = g.read::<u32>(p);
    g.write::<u32>(p, x | disp8(img, f + 0x4d));
    Ret::eax(p)
}

/// `this` in `ecx`, `k` in `edx` (`magic_0519` 0x10002750, 77 bytes,
/// internal): slot 336 (`+0x0c`) with `(k)`, its `eax` stored at `this`;
/// slot 333 (`+0x1f`) with `(that, k)`, its float (kept in a local)
/// `* *M / *D` (`M` at `+0x31`, `D` at `+0x3c`) truncated (`cvttss2si`)
/// into the halfword at `this + 4` (`+0x47`). Returns the truncated
/// value.
pub fn float_ratio_half<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    this: u32,
    k: u32,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(0x05), w(0x0c) / 4);
    let r = g.call(s, this, &[k]).eax;
    g.write::<u32>(this, r);
    let s = slot_target(g, w(0x19), w(0x1f) / 4);
    let x = g.call(s, this, &[r, k]).st0.unwrap_or(0.0) as f32;
    // ebp = esp - 4; the float at ebp - 4
    g.write::<u32>(esp.wrapping_sub(8), x.to_bits());
    let m = f32::from_bits(g.read::<u32>(w(0x31)));
    let d = f32::from_bits(g.read::<u32>(w(0x3c)));
    let v = ffx_bridge::x87::cvttsd2si(f64::from(x * m / d));
    g.write::<u16>(this.wrapping_add(disp8(img, f + 0x47)), v as u16);
    v
}

/// No arguments (`magic_0297` 0x10008020, 85 bytes): the record `*R`
/// (`R` at `+0x01`) gets byte `+0x180` 1 and three floats (`+0x0c`,
/// `+0x10`, `+0x14`, the immediates at `+0x0f`, `+0x16`, `+0x1d`); slot
/// 856 (`+0x28`), then, when the handle `*H` (`H` at `+0x2f`) is set,
/// slot 804 (`+0x3f`) with it and `*H` 0. Returns 0.
pub fn reset_record_handle<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = g.read::<u32>(w(0x01));
    g.write::<u8>(r.wrapping_add(w(0x07)), 1);
    for at in [0x0c, 0x13, 0x1a] {
        g.write::<u32>(r.wrapping_add(disp8(img, f + at + 2)), w(at + 3));
    }
    let s = slot_target(g, w(0x22), w(0x28) / 4);
    g.call(s, fr.ecx, &[]);
    let h = g.read::<u32>(w(0x2f));
    if h != 0 {
        let s = slot_target(g, w(0x39), w(0x3f) / 4);
        g.call(s, fr.ecx, &[h]);
        g.write::<u32>(w(0x4a), 0);
    }
    Ret::eax(0)
}

/// [`slot_pair_loop`] in `magic_0165` (0x10005490, 83 bytes).
pub const SLOT_PAIR_LOOP_0165_5490: Sig = Sig {
    len: 83,
    hash: 0x0f0e_afa3_ac26_fe1b,
    calls: &[],
    imports: &[],
};

/// [`object_list_node`] in `magic_0165` (0x10002ff0, 90 bytes).
pub const OBJECT_LIST_NODE_0165_2FF0: Sig = Sig {
    len: 90,
    hash: 0xf1f2_47f6_e0c9_b91d,
    calls: &[(0x17, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`reset_record_handle`] in `magic_0297` (0x10008020, 85 bytes).
pub const RESET_RECORD_HANDLE_0297_8020: Sig = Sig {
    len: 85,
    hash: 0x3f7c_9608_fdf6_db6e,
    calls: &[],
    imports: &[],
};

/// [`arg_list_node`] in `magic_0270` (0x10001b80, 80 bytes).
pub const ARG_LIST_NODE_0270_1B80: Sig = Sig {
    len: 80,
    hash: 0xe4b2_07de_4b64_1d0a,
    calls: &[(0x9, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`slot_each_flag`] in `magic_0301` (0x100022d0, 80 bytes).
pub const SLOT_EACH_FLAG_0301_22D0: Sig = Sig {
    len: 80,
    hash: 0x9858_514b_b3f6_50c2,
    calls: &[],
    imports: &[],
};

/// [`float_ratio_half`] in `magic_0519` (0x10002750, 77 bytes).
pub const FLOAT_RATIO_HALF_0519_2750: Sig = Sig {
    len: 77,
    hash: 0xe52f_3d03_029b_2c53,
    calls: &[],
    imports: &[],
};
