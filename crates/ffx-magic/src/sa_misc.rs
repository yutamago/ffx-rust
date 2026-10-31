//! Shared helpers of many overlays that belong to no larger family yet
//! (each doc names the DLL and address it was ported from, its size and
//! how many overlays carry it).

use crate::runtime::slot_target;
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

fn f32_at(g: &impl crate::Mem, addr: u32) -> f32 {
    f32::from_bits(g.read::<u32>(addr))
}

/// `cvtdq2ps`/`cvtsi2ss`: an `int` as the nearest float (the default
/// rounding the overlays run with).
#[allow(clippy::cast_precision_loss)] // cvtdq2ps rounds the same
pub(crate) fn cvt(i: i32) -> f32 {
    i as f32
}

/// `magic_0072` 0x10001000 (internal, `ecx` = four floats): each of the
/// four at or below 0 becomes `K` (0.01); slot 933 gets the four, the
/// first is kept in `G_LAST`, and slot 785 gets it scaled by `*G_SCALE`.
/// Returns slot 785's `eax`.
pub fn set_four<G: Game>(g: &mut G, img: &Image, f: u32, v: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    for i in 0..4 {
        if 0.0 >= f32_at(g, v + 4 * i) {
            g.write::<u32>(v + 4 * i, 0x3c23_d70a);
        }
    }
    let t = slot_target(g, w(0x3e), 933);
    let a: [u32; 4] = std::array::from_fn(|i| g.read::<u32>(v + 4 * i as u32));
    g.call(t, 0, &a);
    let x = f32_at(g, v);
    g.write::<u32>(w(0x7f), x.to_bits());
    let y = mul(x, f32_at(g, w(0x87)));
    let t = slot_target(g, w(0x77), 785);
    g.call(t, 0, &[y.to_bits()]).eax
}

/// `set_four` (160 bytes, 91 overlays).
pub const SET_FOUR: Sig = Sig {
    len: 160,
    hash: 0x3572_2f53_143c_e8a0,
    calls: &[],
    imports: &[],
};

/// `magic_0009` 0x100031a0 (internal, `ecx` = an object, `edx` and the
/// second stack word its two owners): resets the object's transform
/// block: `+0x100` the stack word, `+0x114` `edx`, the words from `+0xc0`
/// to `+0x120` cleared but for the scales (`+0xdc`, `+0xec`..`+0xfc`
/// 1.0), byte `+0x126` 0 and `u16 +0x124` 0x8000. Returns 0x8000.
pub fn reset_transform(g: &mut impl crate::Mem, obj: u32, edx: u32, arg1: u32) -> u32 {
    g.write::<u32>(obj + 0x100, arg1);
    g.write::<u32>(obj + 0x114, edx);
    for off in [0x11c, 0x104, 0x108, 0x10c, 0x110, 0x118, 0x120] {
        g.write::<u32>(obj + off, 0);
    }
    g.write::<u8>(obj + 0x126, 0);
    g.write::<u16>(obj + 0x124, 0x8000);
    for off in [0xcc, 0xc8, 0xc4, 0xc0, 0xd8, 0xd4, 0xd0] {
        g.write::<u32>(obj + off, 0);
    }
    g.write::<u32>(obj + 0xdc, 0x3f80_0000);
    for off in [0xe8, 0xe4, 0xe0] {
        g.write::<u32>(obj + off, 0);
    }
    for off in [0xec, 0xfc, 0xf8, 0xf4, 0xf0] {
        g.write::<u32>(obj + off, 0x3f80_0000);
    }
    0x8000
}

/// `reset_transform` (269 bytes, 80 overlays).
pub const RESET_TRANSFORM: Sig = Sig {
    len: 269,
    hash: 0x3002_f820_1219_c319,
    calls: &[],
    imports: &[],
};

/// `magic_0042` 0x100029b0 (internal, no arguments): the twelve words of
/// a static 3x4 block (`G_M + 0x0`..`+0x2c`, in the original's order)
/// become 1.0, and slot 790's float (ST0) goes to `G_M + 0x30`. Returns
/// slot 790's `eax`.
pub fn unit_block<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    for off in [
        0x07, 0x11, 0x1b, 0x25, 0x2f, 0x39, 0x43, 0x4d, 0x57, 0x61, 0x6b, 0x75,
    ] {
        g.write::<u32>(w(off), 0x3f80_0000);
    }
    let t = slot_target(g, w(0x01), 790);
    let out = g.call(t, 0, &[]);
    g.write::<u32>(w(0x87), crate::sa_summon::st0_f32(out));
    out.eax
}

/// `unit_block` (140 bytes, 73 overlays).
pub const UNIT_BLOCK: Sig = Sig {
    len: 140,
    hash: 0x39a8_000c_7337_3e37,
    calls: &[],
    imports: &[],
};

/// `magic_0042` 0x10008100 (internal, `ecx` = a node): a node with a
/// parent (`+0x14`) leaves it ([`crate::nodes`]' unlink: the parent's
/// count down, the sibling links joined, the node's own links to itself
/// and `+0x14` cleared). Returns the node's previous link, or `eax` as
/// given without a parent.
pub fn leave<G: Game>(g: &mut G, node: u32, eax: u32) -> u32 {
    let parent = g.read::<u32>(node + 0x14);
    if parent == 0 {
        return eax;
    }
    crate::nodes::leave_parent(g, node, parent)
}

/// `leave` (81 bytes, 64 overlays).
pub const LEAVE: Sig = Sig {
    len: 81,
    hash: 0x4ffa_e6a3_be00_ad27,
    calls: &[],
    imports: &[],
};

/// `magic_0053` 0x10002a00 (internal, reached by a tail jump; an aligned
/// frame): sends the effect's parts to the game: for each of the
/// `G_PARTS` (count) records (0x74 bytes from `*G_TABLE`), slot 941 maps
/// the record's `+0x30`, slot 772 turns it and the index into a 64-bit
/// handle (`edx:eax`) and slot 779 registers it, with `**G_LOCK` set to
/// 1 around the call; then for each of `G_BANKS` banks, 0x200 entries of
/// the static table go through slots 772/779 the same way (the table
/// read on across the banks). Returns the bank count after the banks
/// ran, else `*G_LOCK` after the parts, else `eax` as given.
pub fn register_parts<G: Game>(g: &mut G, img: &Image, f: u32, eax_in: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut eax = eax_in;
    let register =
        |g: &mut G, (o772, o779, lock_a, lock_b): (u32, u32, u32, u32), x: u32, i: u32| {
            let t = slot_target(g, w(o772), 772);
            let h = g.call(t, 0, &[x, i]);
            let lock = g.read::<u32>(w(lock_a));
            let saved = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            let t = slot_target(g, w(o779), 779);
            g.call(t, 0, &[h.eax, h.edx, x, i]);
            let lock = g.read::<u32>(w(lock_b));
            g.write::<u32>(lock, saved);
            lock
        };
    let mut i = 0u32;
    let mut rec = g.read::<u32>(w(0x16)).wrapping_add(0x30);
    while i.cast_signed() < g.read::<i32>(w(0x0e)) {
        let t = slot_target(g, w(0x21), 941);
        let x = g.read::<u32>(rec);
        let r = g.call(t, 0, &[x]).eax;
        eax = register(g, (0x35, 0x4c, 0x43, 0x5f), r, i);
        rec = rec.wrapping_add(0x74);
        i += 1;
    }
    let banks = g.read::<i32>(w(0x76));
    if banks <= 0 {
        return eax;
    }
    let mut entry = w(0x86);
    let mut k = 0i32;
    loop {
        for i in 0..0x200 {
            let x = g.read::<u32>(entry);
            register(g, (0x93, 0xad, 0xa4, 0xc0), x, i);
            entry = entry.wrapping_add(4);
        }
        k += 1;
        if k >= g.read::<i32>(w(0xe0)) {
            return k.cast_unsigned();
        }
    }
}

/// `register_parts` (237 bytes, 49 overlays).
pub const REGISTER_PARTS: Sig = Sig {
    len: 237,
    hash: 0x6b1d_c9da_d8d4_bea5,
    calls: &[],
    imports: &[],
};

/// `magic_0132` 0x100031d0 (internal, `ecx` = the record block): the
/// block is the effect's 0x200 records (0x74 bytes, `+0x6c` of each
/// cleared; `G_BLOCK` = it, `G_A`/`G_B` 0, `G_N` 0x200); a node is taken
/// off the free ring (`G_FREE`, u16 count `G_COUNT`, as the node pool's
/// take, its callback `+0x10` the DLL's default) and appended to the list
/// (`G_TAIL`, `G_HEAD` when it was empty). Returns the node. (With no
/// free node the original writes through address 0, as this does.)
pub fn records_init<G: Game>(g: &mut G, img: &Image, f: u32, block: u32) -> u32 {
    records_init_n(g, img, f, block, (0x6c, 0x74, 0x200))
}

/// [`records_init`] for `n` records of `stride` bytes, the word at
/// `field` cleared in each (the copies differ in these).
fn records_init_n<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    block: u32,
    (field, stride, n): (u32, u32, u32),
) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(w(0x02), block);
    for k in 0..n {
        g.write::<u32>(block + field + stride * k, 0);
    }
    let count = g.read::<u16>(w(0x20));
    g.write::<u32>(w(0x25), 0);
    g.write::<u32>(w(0x2b), n);
    g.write::<u32>(w(0x34), 0);
    let mut node = 0;
    if count != 0 {
        node = g.read::<u32>(w(0x3e));
        let prev = g.read::<u32>(node);
        let next = g.read::<u32>(node + 4);
        g.write::<u32>(prev + 4, next);
        let next = g.read::<u32>(node + 4);
        let prev = g.read::<u32>(node);
        g.write::<u32>(next, prev);
        if count > 1 {
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(w(0x5c), next);
        } else {
            g.write::<u32>(w(0x64), 0);
        }
        g.write::<u32>(node, 0);
        g.write::<u32>(node + 4, 0);
        g.write::<u32>(node + 0xc, 0);
        g.write::<u32>(node + 0x14, 0);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 0x10, w(0x95));
        g.write::<u8>(node + 0x18, 0);
        g.write::<u16>(w(0xa0), count.wrapping_sub(1));
    }
    let empty = g.read::<u32>(w(0xa6)) == 0;
    g.write::<u32>(node.wrapping_add(4), 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0xd0), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0xd8), node);
    } else {
        let tail = g.read::<u32>(w(0xb7));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0xc4), node);
    }
    node
}

/// `records_init` (221 bytes, 43 overlays).
pub const RECORDS_INIT: Sig = Sig {
    len: 221,
    hash: 0xbf8a_30d1_da29_f23f,
    calls: &[],
    imports: &[],
};

/// `magic_0187` 0x10001250 (a callback, cdecl `(p)`): sets bit 2 of
/// `*p`. Returns `p`.
pub fn set_bit2<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let p = fr.arg(g, 0);
    let v = g.read::<u32>(p);
    g.write::<u32>(p, v | 4);
    Ret::eax(p)
}

/// `set_bit2` (11 bytes, 40 overlays).
pub const SET_BIT2: Sig = Sig {
    len: 11,
    hash: 0xaaf4_c59b_2d02_be12,
    calls: &[],
    imports: &[],
};

/// One slot on the effect handle `*G_H` (the operand at `+0x7`), then 0
/// or 1 (`ret` value).
fn slot_on_handle<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, ret: u32) -> Ret {
    let t = slot_target(g, img.word(f + 0x01), s);
    let h = g.read::<u32>(img.word(f + 0x07));
    g.call(t, 0, &[h]);
    Ret::eax(ret)
}

/// `magic_0187` 0x10001480 (a table entry, no arguments): slot 435 on
/// the effect handle. Returns 0.
pub fn handle_435<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    slot_on_handle(g, img, fr.at, 435, 0)
}

/// `handle_435` (25 bytes, 40 overlays).
pub const HANDLE_435: Sig = Sig {
    len: 25,
    hash: 0xbada_09b9_cbc4_bcf9,
    calls: &[],
    imports: &[],
};

/// `magic_0187` 0x100014e0 (a table entry, no arguments): slot 437 on
/// the effect handle. Returns 0.
pub fn handle_437<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    slot_on_handle(g, img, fr.at, 437, 0)
}

/// `handle_437` (25 bytes, 40 overlays).
pub const HANDLE_437: Sig = Sig {
    len: 25,
    hash: 0xec9f_362f_806e_8191,
    calls: &[],
    imports: &[],
};

/// `magic_0045` 0x10001e60 (a table entry, no arguments): slot 804 on
/// the effect handle. Returns 1.
pub fn handle_804<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    slot_on_handle(g, img, fr.at, 804, 1)
}

/// `handle_804` (28 bytes, 34 overlays).
pub const HANDLE_804: Sig = Sig {
    len: 28,
    hash: 0x180d_bb5b_383e_0d77,
    calls: &[],
    imports: &[],
};

/// `magic_0009` 0x10001900 (a stored callback, no arguments): clears the
/// 0x30-byte block `G_B` (`memset`), sets bit 7 of its `u16 +0x2` and
/// copies `*G_SRC` into its `+0x14`. Returns that word.
pub fn block_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    g.write_bytes(w(0x05), &[0; 0x30]); // memset(G_B, 0, 0x30)
    let v = g.read::<u16>(w(0x16));
    g.write::<u16>(w(0x16), v | 0x80);
    let x = g.read::<u32>(w(0x1b));
    g.write::<u32>(w(0x23), x);
    Ret::eax(x)
}

/// `block_reset` (40 bytes, 38 overlays).
pub const BLOCK_RESET: Sig = Sig {
    len: 40,
    hash: 0xe531_f1aa_c815_ccf7,
    calls: &[(0x0a, Callee::Import("memset"))],
    imports: &[],
};

/// `magic_0052` 0x10002040 (internal, cdecl `(a)`): fills the static
/// block `G_BLK` as `sa_script::actor_block` does for `*G_ME`: `{a, -1,
/// *G_ME, model (slot 413 in a battle, else 215), -1, 0, 1, u16 battle
/// flag}`. Returns the model in a battle, else 0.
pub fn me_block<G: Game>(g: &mut G, img: &Image, f: u32, a: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let battle = g.read::<u32>(w(0x05)) != 0;
    g.write::<u32>(w(0x0e), a);
    let me = g.read::<u32>(w(0x13));
    g.write::<u32>(w(0x18), me);
    g.write::<u32>(w(0x24), u32::MAX);
    let t = slot_target(g, w(0x1e), if battle { 413 } else { 215 });
    let m = g.call(t, 0, &[me]).eax;
    let battle = g.read::<u32>(w(0x43)) != 0;
    g.write::<u32>(w(0x4e), m);
    g.write::<u32>(w(0x54), u32::MAX);
    g.write::<u32>(w(0x5e), 0);
    g.write::<u32>(w(0x68), 1);
    if battle {
        g.write::<u16>(w(0x7b), 1);
        m
    } else {
        g.write::<u16>(w(0x72), 0);
        0
    }
}

/// `me_block` (129 bytes, 35 overlays).
pub const ME_BLOCK: Sig = Sig {
    len: 129,
    hash: 0xab61_8bfd_e196_c22e,
    calls: &[],
    imports: &[],
};

/// The epilogue the census cuts off /GS functions (`call
/// __security_check_cookie; mov esp, ebp; pop ebp; ret`; `magic_0168`
/// 0x10002360, 9 bytes, 28 overlays): no function of its own, only the
/// end of the one before it, whose port does what it does (nothing the
/// game sees: the cookie is the CRT's). Listed so the work list counts
/// it covered.
pub const GS_EPILOGUE: Sig = Sig {
    len: 9,
    hash: 0x6693_97f6_8daf_2beb,
    calls: &[(0x01, Callee::Any)],
    imports: &[],
};

/// `magic_0009` 0x10002af0 (internal, `ecx` = a part record, `edx` = an
/// `i16` offset, stack `(owner, index)`): slot 941 maps the record's
/// `+0x0` (twice: once added to the offset, kept as `v`), slot 772 turns
/// the second and the index into a 64-bit handle and slot 773 asks
/// whether it is registered (`**G_LOCK` 1 around it). Registered: slot
/// 774 `(handle, record, owner, u16 record+8)` and slot 775 `(handle)`
/// with the lock held. Else: slot 776 on the byte at `[v + 4] + v + 4`,
/// slot 777 on the owner's `+0x1c`, and slot 778 registers `(handle,
/// record, owner, u16 record+8, the two answers or-ed)`. The lock word
/// is put back after each. Returns `*G_LOCK`.
pub fn link_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = fr.ecx;
    let owner = fr.arg(g, 0);
    let index = fr.arg(g, 1);
    let t = slot_target(g, w(0x07), 941);
    let x = g.read::<u32>(rec);
    let r1 = g.call(t, 0, &[x]).eax;
    let v = i32::from(g.read::<i16>(fr.edx))
        .cast_unsigned()
        .wrapping_add(r1);
    let t = slot_target(g, w(0x25), 941);
    let x = g.read::<u32>(rec);
    let r2 = g.call(t, 0, &[x]).eax;
    let t = slot_target(g, w(0x41), 772);
    let h = g.call(t, 0, &[r2, index]);
    let (lo, hi) = (h.eax, h.edx);
    let lock = g.read::<u32>(w(0x4f));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x61), 773);
    let known = g.call(t, 0, &[lo, hi, r2, index]).eax;
    let lock = g.read::<u32>(w(0x7a));
    g.write::<u32>(lock, saved);
    if known != 0 {
        let n = u32::from(g.read::<u16>(rec + 8));
        g.write::<u32>(lock, 1);
        let t = slot_target(g, w(0x93), 774);
        g.call(t, 0, &[lo, hi, rec, owner, n]);
        let lock = g.read::<u32>(w(0xa9));
        g.write::<u32>(lock, 1);
        let t = slot_target(g, w(0xb7), 775);
        g.call(t, 0, &[lo, hi]);
        let lock = g.read::<u32>(w(0xc6));
        g.write::<u32>(lock, saved);
        return lock;
    }
    let p = g.read::<u32>(v.wrapping_add(4));
    let b = u32::from(g.read::<u8>(p.wrapping_add(v).wrapping_add(4)));
    let t = slot_target(g, w(0xe6), 776);
    let a = g.call(t, 0, &[b]).eax;
    let t = slot_target(g, w(0xf7), 777);
    let y = g.read::<u32>(owner + 0x1c);
    let c = g.call(t, 0, &[y]).eax;
    let n = u32::from(g.read::<u16>(rec + 8));
    let lock = g.read::<u32>(w(0x115));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x12b), 778);
    g.call(t, 0, &[lo, hi, rec, owner, n, a | c]);
    let lock = g.read::<u32>(w(0x13b));
    g.write::<u32>(lock, saved);
    lock
}

/// `link_part` (331 bytes, 32 overlays).
pub const LINK_PART: Sig = Sig {
    len: 331,
    hash: 0x5f05_b40f_2cc6_afea,
    calls: &[],
    imports: &[],
};

/// `magic_0084` 0x100011f0 (a table entry, no arguments): the object
/// host's start-up calls on the DLL's three blocks (slots 353, 397, 332),
/// slot 267(4), the colour 1.0 x4 (slot 933), `G_ALPHA` 1.0, slot 785
/// twice with `*G_K`, slot 543(0), slot 250(0, 0) and, when slot 549(1)
/// answers non-zero, slot 550(1); each call's words stay on the stack
/// for the next until the original pops them. Returns 0.
pub fn host_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    host_start_calls(g, img, fr.at);
    Ret::eax(0)
}

/// The calls of [`host_start`] (`f` its address); whether slot 549
/// answered non-zero (slot 550 then ran).
fn host_start_calls<G: Game>(g: &mut G, img: &Image, f: u32) -> bool {
    let w = |off: u32| img.word(f + off);
    let one = 0x3f80_0000;
    let (a, b, c) = (w(0x06), w(0x18), w(0x2a));
    let t = slot_target(g, w(0x01), 353);
    g.call(t, 0, &[a]);
    let t = slot_target(g, w(0x13), 397);
    g.call(t, 0, &[b, a]);
    let t = slot_target(g, w(0x25), 332);
    g.call(t, 0, &[c, b, a]);
    let t = slot_target(g, w(0x37), 267);
    g.call(t, 0, &[4, c, b, a]);
    let t = slot_target(g, w(0x46), 933);
    g.call(t, 0, &[one, one, one, one]);
    let t = slot_target(g, w(0x72), 785);
    let k = g.read::<u32>(w(0x7a));
    g.write::<u32>(w(0x89), one);
    g.call(t, 0, &[k]);
    let t = slot_target(g, w(0x99), 785);
    let k = g.read::<u32>(w(0xa1));
    g.call(t, 0, &[k]);
    let t = slot_target(g, w(0xb3), 543);
    g.call(t, 0, &[0, k]);
    let t = slot_target(g, w(0xc2), 250);
    g.call(t, 0, &[0, 0]);
    let t = slot_target(g, w(0xde), 549);
    if g.call(t, 0, &[1, 0, 0]).eax == 0 {
        return false;
    }
    let t = slot_target(g, w(0xf4), 550);
    g.call(t, 0, &[1]);
    true
}

/// `magic_0165` 0x10001260 (a table entry, no arguments): [`host_start`]
/// followed by a second slot 549(1) check (and slot 550(1)), whatever
/// the first one answered. Returns 0.
pub fn host_start_d<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    host_start_calls(g, img, f);
    let t = slot_target(g, img.word(f + 0x106), 549);
    if g.call(t, 0, &[1]).eax != 0 {
        let t = slot_target(g, img.word(f + 0x11c), 550);
        g.call(t, 0, &[1]);
    }
    Ret::eax(0)
}

/// `host_start_d` (304 bytes, 5 overlays).
pub const HOST_START_D: Sig = Sig {
    len: 304,
    hash: 0x60dc_441b_a1d6_ade5,
    calls: &[],
    imports: &[],
};

/// `host_start` (264 bytes, 31 overlays).
pub const HOST_START: Sig = Sig {
    len: 264,
    hash: 0x4730_be2f_7b86_b2bc,
    calls: &[],
    imports: &[],
};

/// `magic_0188` 0x10003b30 (internal, no arguments): one frame of the
/// DLL's timeline (`G_LIVE` events still to end, `G_NOW` the frame):
/// each live event (flag `G_FLAGS[i]`, 8 bytes apart; the record
/// `*G_EVENTS + 0x18 * i`: start, length, on-start, on-frame, on-end,
/// argument) that has started runs its on-start callback at its start
/// frame and its on-frame callback every frame; it ends at its last
/// frame (start + length - 1), or, with length -1, when on-frame answers
/// 0 (no on-frame counts as 0x309): the on-end callback, the flag
/// cleared, `G_LIVE` down (0 left: returns 0). Then `G_NOW` advances
/// unless slot 253 answers 0. Returns 1 (0 with nothing live).
pub fn timeline<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x06)) == 0 {
        return 0;
    }
    let mut ev = g.read::<u32>(w(0x17));
    let mut flag = w(0x1f);
    let mut i = 0i32;
    while i < g.read::<i32>(w(0x25)) {
        let started = g.read::<u32>(flag) != 0 && g.read::<i32>(w(0x41)) >= g.read::<i32>(ev);
        if started {
            let start = g.read::<u32>(ev);
            let arg = |g: &G| g.read::<u32>(ev + 0x14);
            if g.read::<u32>(w(0x41)) == start {
                let cb = g.read::<u32>(ev + 8);
                if cb != 0 {
                    let a = arg(g);
                    g.call(cb, 0, &[a]);
                }
            }
            let mut edx = 0x309;
            let cb = g.read::<u32>(ev + 0xc);
            if cb != 0 {
                let a = arg(g);
                edx = g.call(cb, 0, &[a]).eax;
            }
            let len = g.read::<u32>(ev + 4);
            let ends = if len == u32::MAX {
                edx == 0
            } else {
                let last = g.read::<u32>(ev).wrapping_sub(1).wrapping_add(len);
                g.read::<u32>(w(0x7d)) == last
            };
            if ends {
                let cb = g.read::<u32>(ev + 0x10);
                if cb != 0 {
                    let a = arg(g);
                    g.call(cb, 0, &[a]);
                }
                g.write::<u32>(flag, 0);
                let live = g.read::<u32>(w(0x9e)).wrapping_sub(1);
                g.write::<u32>(w(0x9e), live);
                if live == 0 {
                    return 0;
                }
            }
        }
        i += 1;
        flag = flag.wrapping_add(8);
        ev = ev.wrapping_add(0x18);
    }
    let t = slot_target(g, w(0xb8), 253);
    if g.call(t, 0, &[]).eax != 0 {
        let now = g.read::<u32>(w(0xcf));
        g.write::<u32>(w(0xcf), now.wrapping_add(1));
    }
    1
}

/// `timeline` (227 bytes, 27 overlays).
pub const TIMELINE: Sig = Sig {
    len: 227,
    hash: 0x3948_02db_ccbe_59d6,
    calls: &[],
    imports: &[],
};

/// `magic_0162` 0x10005ea0 (internal, `ecx` = an object): the object's
/// drift: slot 314 (a random spread, ST0) of `+0x128` summed
/// `+0x130` times (f32, in the original's order) is added to each of
/// `+0x50..+0x58`, and of `+0x12c` summed `+0x134` times to
/// `+0x60..+0x68`. Returns the last call's `eax` (`eax` as given when
/// neither count is positive).
pub fn drift<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, eax_in: u32) -> u32 {
    let mut eax = eax_in;
    for (spread, count, at, op) in [(0x128, 0x130, 0x50, 0x31), (0x12c, 0x134, 0x60, 0xa4)] {
        let mut sum = 0.0f32;
        let mut i = 0i32;
        while i < g.read::<i32>(obj + count) {
            let t = slot_target(g, img.word(f + op), 314);
            let x = g.read::<u32>(obj + spread);
            let out = g.call(t, 0, &[x]);
            eax = out.eax;
            #[allow(clippy::assign_op_pattern)] // r + sum: the original's operand order
            {
                sum = add(f32::from_bits(crate::sa_summon::st0_f32(out)), sum);
            }
            i += 1;
        }
        for k in 0..3 {
            let v = add(f32_at(g, obj + at + 4 * k), sum);
            g.write::<u32>(obj + at + 4 * k, v.to_bits());
        }
    }
    eax
}

/// `drift` (271 bytes, 25 overlays).
pub const DRIFT: Sig = Sig {
    len: 271,
    hash: 0x9cd6_0d16_9f55_1b0a,
    calls: &[],
    imports: &[],
};

/// `magic_0162` 0x10009590 (internal, `ecx` = an object): the transform
/// reset of [`reset_transform`] with both owners 0. Returns 0x8000.
pub fn reset_transform_0(g: &mut impl crate::Mem, obj: u32) -> u32 {
    reset_transform(g, obj, 0, 0)
}

/// `reset_transform_0` (270 bytes, 24 overlays).
pub const RESET_TRANSFORM_0: Sig = Sig {
    len: 270,
    hash: 0x2595_a0d2_9b22_6052,
    calls: &[],
    imports: &[],
};

/// `magic_0042` 0x10007de0 (internal, `ecx` = a node, `edx` = its
/// parent): a node with the DLL's handler table, pass byte `+0x1a` 0xff
/// (as a u16), an empty child list and the parent `+0x14`; with a parent
/// it is appended to the parent's child list (`+0x4`/`+0x8` links, the
/// parent's `+0xc` first and `+0x10` last child, u16 count `+0x18`).
/// Returns the parent's last child before it (`eax` as given without a
/// parent).
pub fn node_append<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    parent: u32,
    eax: u32,
) -> u32 {
    g.write::<u32>(node, img.word(f + 0x02));
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent == 0 {
        return eax;
    }
    g.write::<u32>(node + 4, parent);
    let last = g.read::<u32>(parent + 0x10);
    g.write::<u32>(node + 8, last);
    let last = g.read::<u32>(parent + 0x10);
    if last != parent {
        g.write::<u32>(last + 4, node);
    }
    let n = g.read::<u16>(parent + 0x18);
    g.write::<u16>(parent + 0x18, n.wrapping_add(1));
    if last == parent {
        g.write::<u32>(parent + 0xc, node);
    }
    g.write::<u32>(parent + 0x10, node);
    last
}

/// `node_append` (63 bytes, 22 overlays).
pub const NODE_APPEND: Sig = Sig {
    len: 63,
    hash: 0xcd0f_b1f3_8c05_a2d1,
    calls: &[],
    imports: &[],
};

/// `magic_0353` 0x100023f0 (a stored callback, cdecl `(obj)`): one frame
/// of a fade out: the countdown `+0x120` drops, the four `u16` colour
/// words `+0x70..+0x76` become `countdown * 4096 / 30` (the original's
/// signed multiply-shift); at 0 the callback `+0x104` is cleared.
/// Returns the colour.
pub fn fade_step<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let obj = fr.arg(g, 0);
    let n = g.read::<u32>(obj + 0x120).wrapping_sub(1);
    g.write::<u32>(obj + 0x120, n);
    let x = g.read::<i32>(obj + 0x120).wrapping_shl(12);
    // imul 0x88888889; add edx, ecx; sar edx, 5; + the sign bit
    let hi = ((i64::from(x) * i64::from(0x8888_8889u32.cast_signed())) >> 32) as i32;
    let q = hi.wrapping_add(x) >> 5;
    let q = q
        .wrapping_add((q.cast_unsigned() >> 31).cast_signed())
        .cast_unsigned();
    for off in [0x76, 0x74, 0x72, 0x70] {
        g.write::<u16>(obj + off, q as u16);
    }
    if g.read::<u32>(obj + 0x120) == 0 {
        g.write::<u32>(obj + 0x104, 0);
    }
    Ret::eax(q)
}

/// `fade_step` (79 bytes, 21 overlays).
pub const FADE_STEP: Sig = Sig {
    len: 79,
    hash: 0x73de_edbf_9d84_6f59,
    calls: &[],
    imports: &[],
};

/// `magic_0197` 0x10001000 (a table entry, cdecl `(arg)`): the object
/// host's start-up: `G_ARG` = arg; slots 556, 555 and 331 on the DLL's
/// three blocks (each call's words left for the next), the three scales
/// 1.0 and slot 790's float into `G_F`; the task ring
/// ([`crate::l5_objhost::tasks_init`]); the 0x3ff nodes of pool A free
/// (payload 0, the free count 0x3ff, the rover 0); slot 720 (0x3400,
/// 0x800), slot 718 on the part, slots 222 and 221 on it `(part, 0,
/// 1000, 1)` (the earlier words staying below); `G_DONE` 0. Returns 0.
pub fn host_start_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let one = 0x3f80_0000;
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x07), arg);
    let (a, b, c) = (w(0x11), w(0x23), w(0x35));
    let t = slot_target(g, w(0x0c), 556);
    g.call(t, 0, &[a]);
    let t = slot_target(g, w(0x1e), 555);
    g.call(t, 0, &[b, a]);
    let t = slot_target(g, w(0x30), 331);
    g.call(t, 0, &[c, b, a]);
    let t = slot_target(g, w(0x42), 790);
    for off in [0x51, 0x5b, 0x65] {
        g.write::<u32>(w(off), one);
    }
    let out = g.call(t, 0, &[]);
    g.write::<u32>(w(0x71), crate::sa_summon::st0_f32(out));
    crate::l5_objhost::tasks_init(g, img, img.rel32_target(f + 0x76));
    for k in 0..0x3ff {
        g.write::<u32>(w(0x7b) + 12 * k, 0);
    }
    g.write::<u16>(w(0x97), 0x3ff);
    g.write::<u16>(w(0x9f), 0);
    let t = slot_target(g, w(0xa4), 720);
    g.call(t, 0, &[0x3400, 0x800]);
    let part = w(0xc0);
    let t = slot_target(g, w(0xbb), 718);
    g.call(t, 0, &[part, 0x3400, 0x800]);
    let t = slot_target(g, w(0xcd), 222);
    let mut stack = vec![w(0xe1), 0, 1000, 1, part, 0x3400, 0x800];
    // call width: 7 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xe8), 221);
    stack.splice(0..0, [w(0xfc), 0, 1000, 1]);
    // call width: 11 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    g.write::<u32>(w(0x107), 0);
    Ret::eax(0)
}

/// `host_start_b` (275 bytes, 21 overlays; the `rel32` is the task
/// ring's set-up).
pub const HOST_START_B: Sig = Sig {
    len: 275,
    hash: 0x1b38_9189_beb7_7a5d,
    calls: &[(0x76, Callee::Sig(&crate::l5_objhost::TASKS_INIT))],
    imports: &[],
};

/// `magic_0092` 0x100026e0 (internal, `ecx` = an object, `edx` and the
/// stack `(a, b)` its owners): [`reset_transform`] with `+0x100` = `b`,
/// `+0x114` = `edx` and `+0x11c` = `a`. Returns 0x8000.
pub fn reset_transform_3(g: &mut impl crate::Mem, obj: u32, edx: u32, a: u32, b: u32) -> u32 {
    reset_transform(g, obj, edx, b);
    g.write::<u32>(obj + 0x11c, a);
    0x8000
}

/// `reset_transform_3` (268 bytes, 20 overlays).
pub const RESET_TRANSFORM_3: Sig = Sig {
    len: 268,
    hash: 0x7f17_77d2_b826_9073,
    calls: &[],
    imports: &[],
};

/// The part refresh of `magic_0045` 0x10001510 and `magic_0341`
/// 0x100010d0 (`ops`: the operand offsets): slot 258 on the object,
/// slot 231 and slot 230 on the DLL's matrices (`*G_A`.. as words),
/// `**G_PART` = the DLL's part block, then slot 712 `(obj, 0x30, ...)`
/// with `**G_LOCK` 1 around it (every earlier call's words staying on
/// the stack). Returns slot 712's `eax`.
fn part_refresh<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, o: [u32; 14]) -> u32 {
    let w = |off: u32| img.word(f + off);
    let [
        t258,
        x68,
        t231,
        x74,
        x70,
        x70b,
        x60,
        t230,
        part_p,
        part,
        lock_a,
        t712,
        lock_b,
        _,
    ] = o;
    let t = slot_target(g, w(t258), 258);
    g.call(t, 0, &[obj]);
    let a = g.read::<u32>(w(x68));
    let t = slot_target(g, w(t231), 231);
    let b = g.read::<u32>(w(x74));
    let c = g.read::<u32>(w(x70));
    g.call(t, 0, &[c, b, a, obj]);
    let d = g.read::<u32>(w(x70b));
    let t = slot_target(g, w(t230), 230);
    let e = g.read::<u32>(w(x60));
    g.call(t, 0, &[e, d, c, b, a, obj]);
    let p = g.read::<u32>(w(part_p));
    g.write::<u32>(p, w(part));
    let lock = g.read::<u32>(w(lock_a));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(t712), 712);
    let r = g.call(t, 0, &[obj, 0x30, e, d, c, b, a, obj]).eax;
    let lock = g.read::<u32>(w(lock_b));
    g.write::<u32>(lock, saved);
    r
}

/// `magic_0045` 0x10001510 (a table entry, cdecl `(obj)`): the part
/// refresh. Returns 1 when slot 712 answered 0, else 0.
pub fn part_refresh_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let obj = fr.arg(g, 0);
    let ops = [
        0x04, 0x16, 0x1b, 0x21, 0x2d, 0x35, 0x40, 0x3a, 0x4d, 0x58, 0x5d, 0x6a, 0x78, 0,
    ];
    let r = part_refresh(g, img, fr.at, obj, ops);
    Ret {
        edx: r,
        ..Ret::eax(u32::from(r == 0))
    }
}

/// `part_refresh_a` (141 bytes, 19 overlays).
pub const PART_REFRESH_A: Sig = Sig {
    len: 141,
    hash: 0x76ff_958c_973d_7097,
    calls: &[],
    imports: &[],
};

/// `magic_0341` 0x100010d0 (internal, `ecx` = an object): with the
/// object's flag `u16 +0x1e` set, the part refresh, and when slot 712
/// answered non-zero the flag is cleared. Returns 0 then (`eax` as given
/// with the flag clear).
pub fn part_refresh_b<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, eax: u32) -> u32 {
    if g.read::<u16>(obj + 0x1e) == 0 {
        return eax;
    }
    let ops = [
        0x0f, 0x1f, 0x24, 0x2a, 0x36, 0x3e, 0x49, 0x43, 0x56, 0x5e, 0x63, 0x71, 0x7f, 0,
    ];
    if part_refresh(g, img, f, obj, ops) != 0 {
        g.write::<u16>(obj + 0x1e, 0);
    }
    0
}

/// `part_refresh_b` (149 bytes, 16 overlays).
pub const PART_REFRESH_B: Sig = Sig {
    len: 149,
    hash: 0x04db_f4cf_0cb7_d772,
    calls: &[],
    imports: &[],
};

/// `magic_0195` 0x100013f0 (a callback, cdecl `(a)`): slot 432, then
/// slot 450 `(a, the DLL's callback)`. Returns slot 450's `eax`.
pub fn hook_callback<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let a = fr.arg(g, 0);
    let t = slot_target(g, img.word(f + 0x04), 432);
    g.call(t, 0, &[]);
    let t = slot_target(g, img.word(f + 0x11), 450);
    Ret::eax(g.call(t, 0, &[a, img.word(f + 0x16)]).eax)
}

/// `hook_callback` (42 bytes, 18 overlays).
pub const HOOK_CALLBACK: Sig = Sig {
    len: 42,
    hash: 0xf5a8_7f4b_7d30_3fb3,
    calls: &[],
    imports: &[],
};

/// `magic_0042` 0x10004ef0 (internal, `ecx` = the cursor): the walk;
/// when the cursor moved, the node's `+0x1c` is added to its parent's
/// (`+0x14`) `+0x20`. Returns the node, or the walk's `eax`.
pub fn add_to_parent<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let this = fr.ecx;
    let (eax, moved) = crate::nodes::walk(g, this, this, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(this);
    let p = g.read::<u32>(n + 0x14);
    let v = add(f32_at(g, n + 0x1c), f32_at(g, p + 0x20));
    g.write::<u32>(p + 0x20, v.to_bits());
    Ret::eax(n)
}

/// `add_to_parent` (96 bytes, 14 overlays).
pub const ADD_TO_PARENT: Sig = Sig {
    len: 96,
    hash: 0x50d8_b684_3ce0_ccac,
    calls: &[],
    imports: &[],
};

/// `magic_0042` 0x10007e20 (internal, `ecx` = a node, `edx` = its
/// parent, stack `(owner, a, b, c)`): [`node_append`]'s set-up (the
/// DLL's handler table, appended to the parent's children), then the
/// node's `+0x13c`/`+0x144`/`+0x140` take `a`/`b`/`c`, `+0x128` the
/// owner and byte `+0x148` 0. Returns the owner.
pub fn node_append_owned<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let (node, parent) = (fr.ecx, fr.edx);
    g.write::<u32>(node, img.word(fr.at + 0x05));
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
        g.write::<u32>(node + 4, parent);
        let last = g.read::<u32>(parent + 0x10);
        g.write::<u32>(node + 8, last);
        let last = g.read::<u32>(parent + 0x10);
        if last == parent {
            g.write::<u32>(parent + 0xc, node);
        } else {
            g.write::<u32>(last + 4, node);
        }
        let n = g.read::<u16>(parent + 0x18);
        g.write::<u16>(parent + 0x18, n.wrapping_add(1));
        g.write::<u32>(parent + 0x10, node);
    }
    let owner = fr.arg(g, 0);
    for (off, i) in [(0x13c, 1), (0x144, 2), (0x140, 3)] {
        let v = fr.arg(g, i);
        g.write::<u32>(node + off, v);
    }
    g.write::<u32>(node + 0x128, owner);
    g.write::<u8>(node + 0x148, 0);
    owner
}

/// `node_append_owned` (104 bytes, 14 overlays).
pub const NODE_APPEND_OWNED: Sig = Sig {
    len: 104,
    hash: 0x6a51_6fa0_aab6_6f11,
    calls: &[],
    imports: &[],
};

/// `magic_0010` 0x10001bc0 (internal, `ecx` = an object, `edx` and the
/// second stack word its owners): [`reset_transform`] with `+0x11c` 0x14.
/// Returns 0x8000.
pub fn reset_transform_14(g: &mut impl crate::Mem, obj: u32, edx: u32, b: u32) -> u32 {
    reset_transform_3(g, obj, edx, 0x14, b)
}

/// `reset_transform_14` (269 bytes, 14 overlays).
pub const RESET_TRANSFORM_14: Sig = Sig {
    len: 269,
    hash: 0x3002_2aaf_0172_852d,
    calls: &[],
    imports: &[],
};

/// `magic_0159` 0x10006270 (internal, `ecx` = a node, `edx` = its
/// parent, stack `(a, b, c, d)`): `node_insert` ([`crate::nodes`]) with
/// a group in this frame: `[node, parent, a, b, c, d]`. Returns its
/// `eax`.
pub fn insert_with<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    // push ebp; sub esp, 0x18: the group is at [ebp - 0x18]
    let group = fr.esp.wrapping_sub(0x1c);
    let words = [
        fr.ecx,
        fr.edx,
        fr.arg(g, 0),
        fr.arg(g, 1),
        fr.arg(g, 2),
        fr.arg(g, 3),
    ];
    for (i, v) in (0u32..).zip(words) {
        g.write::<u32>(group + 4 * i, v);
    }
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(fr.at + 0x28),
            esp: fr.esp.wrapping_sub(0x20),
            ecx: group,
            ..Frame::default()
        },
    )
    .eax
}

/// `insert_with` (48 bytes, 13 overlays).
pub const INSERT_WITH: Sig = Sig {
    len: 48,
    hash: 0xae19_98ec_ab87_ee8a,
    calls: &[(0x28, Callee::Sig(&crate::nodes::INSERT))],
    imports: &[],
};

/// `magic_0162` 0x100081f0 (internal, `ecx` = the record block):
/// [`records_init`] for 0x400 records.
pub fn records_init_400<G: Game>(g: &mut G, img: &Image, f: u32, block: u32) -> u32 {
    records_init_n(g, img, f, block, (0x6c, 0x74, 0x400))
}

/// `records_init_400` (221 bytes, 13 overlays).
pub const RECORDS_INIT_400: Sig = Sig {
    len: 221,
    hash: 0xe7a6_1582_5ca2_607b,
    calls: &[],
    imports: &[],
};

/// `magic_0242` 0x10001a60 (internal, no arguments): takes the next free
/// slot of the DLL's ring (`G_COUNT` entries of 0x50 bytes at `*G_BASE`,
/// searched from the rover `G_NEXT`; free when `+0x48` is 0): the rover
/// moves past it, the entry is reset (`+0x3c` 0, colours `+0x40`/`+0x44`
/// 0x80808080, `u16 +0x4c` 0) and marked used. Returns it, or 0 when the
/// ring is full.
pub fn ring_take<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut i = g.read::<u32>(w(0x01));
    let n = g.read::<i32>(w(0x09));
    if n <= 0 {
        return 0;
    }
    let base = g.read::<u32>(w(0x16));
    for _ in 0..n {
        let e = i.wrapping_mul(0x50).wrapping_add(base);
        i = i.wrapping_add(1);
        if i.cast_signed() >= n {
            i = 0;
        }
        if g.read::<u32>(e + 0x48) == 0 {
            g.write::<u32>(e + 0x3c, 0);
            g.write::<u32>(w(0x48), i);
            g.write::<u32>(e + 0x40, 0x8080_8080);
            g.write::<u32>(e + 0x44, 0x8080_8080);
            g.write::<u16>(e + 0x4c, 0);
            g.write::<u32>(e + 0x48, 1);
            return e;
        }
    }
    0
}

/// `ring_take` (105 bytes, 12 overlays).
pub const RING_TAKE: Sig = Sig {
    len: 105,
    hash: 0x7a83_9d56_7d5f_76e1,
    calls: &[],
    imports: &[],
};

/// `magic_0042` 0x10002a60 (internal, `ecx` = `[node, mode, target,
/// time]`): starts a node's move: `+0x9c` the target, `+0xa0` 0; mode
/// set: flags `+0xac` bits 0 and 2; else bit 0 cleared, the deltas
/// `+0x3c..+0x48 = +0x2c.. - +0x1c..` and `+0x6c..+0x78 = +0x5c.. -
/// +0x4c..` (interleaved as the original stores them) and bit 2 set;
/// `+0xa4` the time. Returns the time.
pub fn move_start<G: Game>(g: &mut G, args: u32) -> u32 {
    let n = g.read::<u32>(args);
    let target = g.read::<u32>(args + 8);
    g.write::<u32>(n + 0x9c, target);
    g.write::<u32>(n + 0xa0, 0);
    let flags = g.read::<u32>(n + 0xac);
    if g.read::<u32>(args + 4) == 0 {
        g.write::<u32>(n + 0xac, flags & !1);
        for i in 0..4 {
            let a = sub(f32_at(g, n + 0x2c + 4 * i), f32_at(g, n + 0x1c + 4 * i));
            g.write::<u32>(n + 0x3c + 4 * i, a.to_bits());
            let b = sub(f32_at(g, n + 0x5c + 4 * i), f32_at(g, n + 0x4c + 4 * i));
            g.write::<u32>(n + 0x6c + 4 * i, b.to_bits());
        }
    } else {
        g.write::<u32>(n + 0xac, flags | 1);
    }
    let time = g.read::<u32>(args + 0xc);
    let flags = g.read::<u32>(n + 0xac);
    g.write::<u32>(n + 0xac, flags | 4);
    g.write::<u32>(n + 0xa4, time);
    time
}

/// `move_start` (195 bytes, 12 overlays).
pub const MOVE_START: Sig = Sig {
    len: 195,
    hash: 0xa94b_da9e_385f_a53a,
    calls: &[],
    imports: &[],
};

/// `magic_0292` 0x10001210 (a table entry, no arguments): `**G_PART` =
/// the DLL's part block, slot 713 releases the object block. Returns 1.
pub fn release_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x01));
    g.write::<u32>(p, w(0x0c));
    let t = slot_target(g, w(0x11), 713);
    g.call(t, 0, &[w(0x06)]);
    Ret::eax(1)
}

/// `release_part` (38 bytes, 11 overlays).
pub const RELEASE_PART: Sig = Sig {
    len: 38,
    hash: 0x95ad_12c1_f2f0_23fa,
    calls: &[],
    imports: &[],
};

/// `magic_0206` 0x10002670 (internal, `ecx` = a 0x3c-byte sprite block,
/// `edx` = a resource): the block cleared (`memset`), `+0x0` the
/// resource's handle (slot 942), `+0x4` its data (slot 941) + 0x10,
/// `+0xc` the data plus the `i16` at the header's `+0x10`, `u16 +0x10`
/// the header's `+0x12`; offsets 0, scales 1.0, colour 0x80 x3 (each
/// call's words left on the stack until the end). Returns the header
/// with its `u16 +0x12` in the low half (the last `eax`).
pub fn sprite_init<G: Game>(g: &mut G, img: &Image, f: u32, out: u32, res: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write_bytes(out, &[0; 0x3c]); // memset(out, 0, 0x3c)
    let mut stack = vec![out, 0, 0x3c];
    let t = slot_target(g, w(0x11), 942);
    stack.insert(0, res);
    // call width: 4 (the earlier calls' words stay on the stack)
    let h = g.call(t, 0, &stack).eax;
    g.write::<u32>(out, h);
    stack.insert(0, h);
    let t = slot_target(g, w(0x22), 941);
    // call width: 5 (the earlier calls' words stay on the stack)
    let d = g.call(t, 0, &stack).eax;
    let h = g.read::<u32>(out);
    stack.insert(0, h);
    g.write::<u32>(out + 4, d.wrapping_add(0x10));
    let t = slot_target(g, w(0x37), 941);
    // call width: 6 (the earlier calls' words stay on the stack)
    let data = g.call(t, 0, &stack).eax;
    let t = slot_target(g, w(0x45), 941);
    let h = g.read::<u32>(out);
    stack.insert(0, h);
    // call width: 7 (the earlier calls' words stay on the stack)
    let hdr = g.call(t, 0, &stack).eax;
    let off = i32::from(g.read::<i16>(hdr + 0x10)).cast_unsigned();
    let t = slot_target(g, w(0x5a), 941);
    let h = g.read::<u32>(out);
    stack.insert(0, h);
    g.write::<u32>(out + 0xc, off.wrapping_add(data));
    // call width: 8 (the earlier calls' words stay on the stack)
    let hdr = g.call(t, 0, &stack).eax;
    let n = g.read::<u16>(hdr + 0x12);
    g.write::<u16>(out + 0x10, n);
    for (o, v) in [
        (0x14, 0),
        (0x18, 0),
        (0x1c, 0),
        (0x20, 0x3f80_0000),
        (0x24, 0x3f80_0000),
        (0x34, 0x80),
        (0x30, 0x80),
        (0x2c, 0x80),
        (0x38, 0x3f80_0000),
    ] {
        g.write::<u32>(out + o, v);
    }
    (hdr & 0xffff_0000) | u32::from(n)
}

/// `sprite_init` (186 bytes, 11 overlays).
pub const SPRITE_INIT: Sig = Sig {
    len: 186,
    hash: 0x0a3b_a89e_347d_b1c2,
    calls: &[(0x0c, Callee::Import("memset"))],
    imports: &[],
};

/// `magic_0206` 0x10002b20 (internal, `ecx` = a sprite fader): unless
/// flag bit 0 (`+0x2c`) holds it, the time `+0x28` counts down and at 0
/// the fader stops (returns 0). Then by the flags: bit 1 the phase
/// `+0x68 = time / length` (`+0x24`); bit 2 the colour `+0x5c..+0x64`
/// (ints) between the end `+0x6c..` and the start `+0x7c..` by `1 -
/// phase`, and the alpha `+0x68` likewise from `+0x78`/`+0x88`, divided
/// by `K2` and kept in 0..`K1`; bit 3 the scale `+0x54` between `+0x8c`
/// and `+0x90`. Slot 248 then moves the position `+0x4` by `+0x14`.
/// Returns 1.
pub fn sprite_fade<G: Game>(g: &mut G, img: &Image, f: u32, o: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let cvt = crate::sa_misc::cvt;
    let int = |g: &G, a: u32| cvt(g.read::<i32>(a));
    let flags = g.read::<u32>(o + 0x2c);
    if flags & 1 == 0 {
        let t = g.read::<u32>(o + 0x28).wrapping_sub(1);
        g.write::<u32>(o + 0x28, t);
        if g.read::<i32>(o + 0x28) <= 0 {
            return 0;
        }
    }
    if flags & 2 != 0 {
        let p = div(int(g, o + 0x28), int(g, o + 0x24));
        g.write::<u32>(o + 0x68, p.to_bits());
    }
    let k1 = f32_at(g, w(0x3a));
    if flags & 4 != 0 {
        let p = div(int(g, o + 0x28), int(g, o + 0x24));
        let q = sub(k1, p);
        let lerp = |g: &G, from: u32, to: u32| {
            let a = g.read::<u32>(o + from);
            let d = g.read::<u32>(o + to).wrapping_sub(a);
            add(mul(cvt(d.cast_signed()), q), cvt(a.cast_signed()))
        };
        for (dst, from, to) in [(0x5c, 0x6c, 0x7c), (0x60, 0x70, 0x80), (0x64, 0x74, 0x84)] {
            let v = lerp(g, from, to);
            g.write::<u32>(o + dst, ffx_bridge::x87::cvttsd2si(f64::from(v)));
        }
        let v = div(lerp(g, 0x78, 0x88), f32_at(g, w(0xff)));
        g.write::<u32>(o + 0x68, v.to_bits());
        if 0.0 > v {
            g.write::<u32>(o + 0x68, 0);
        }
        if f32_at(g, o + 0x68) > k1 {
            g.write::<u32>(o + 0x68, 0x3f80_0000);
        }
    }
    if flags & 8 != 0 {
        let p = div(int(g, o + 0x28), int(g, o + 0x24));
        let a = f32_at(g, o + 0x8c);
        let b = f32_at(g, o + 0x90);
        let v = add(mul(sub(k1, p), sub(b, a)), a);
        g.write::<u32>(o + 0x54, v.to_bits());
    }
    let t = slot_target(g, w(0x168), 248);
    g.call(t, 0, &[o + 4, o + 4, o + 0x14]);
    1
}

/// `sprite_fade` (387 bytes, 11 overlays).
pub const SPRITE_FADE: Sig = Sig {
    len: 387,
    hash: 0xa80d_e3fc_7274_bb5d,
    calls: &[],
    imports: &[],
};

/// `magic_0201` 0x100048e0 (a table entry, no arguments): slot 250
/// `(0, 0)`. Returns its `eax`.
pub fn slot250_zero<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let t = slot_target(g, img.word(fr.at + 0x01), 250);
    Ret::eax(g.call(t, 0, &[0, 0]).eax)
}

/// `slot250_zero` (35 bytes, 10 overlays).
pub const SLOT250_ZERO: Sig = Sig {
    len: 35,
    hash: 0x3590_98f3_f94c_c9c8,
    calls: &[],
    imports: &[],
};

/// `magic_0565` 0x10006110 (a table entry, no arguments): slot 719 finds
/// the DLL's effect (kind 6, 0xc) and slot 663 gives it 0x78 (slot 719's
/// words left below). Returns slot 663's `eax`.
pub fn effect_78<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x01), 719);
    let e = w(0x10);
    let r = g.call(t, 0, &[e, 6, 0xc]).eax;
    let t = slot_target(g, w(0x1a), 663);
    Ret::eax(g.call(t, 0, &[r, 0x78, e, 6, 0xc]).eax)
}

/// `effect_78` (42 bytes, 10 overlays).
pub const EFFECT_78: Sig = Sig {
    len: 42,
    hash: 0xa867_e45c_87e1_f972,
    calls: &[],
    imports: &[],
};

/// `magic_0565` 0x100017b0 (a table entry, no arguments): slot 628(0),
/// slot 311 on the DLL's handle `*G_H` (slot 628's word left below),
/// then `*G_H` -1. Returns slot 311's `eax`.
pub fn handle_close<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x01), 628);
    g.call(t, 0, &[0]);
    let t = slot_target(g, w(0x10), 311);
    let h = g.read::<u32>(w(0x16));
    let r = g.call(t, 0, &[h, 0]).eax;
    g.write::<u32>(w(0x27), u32::MAX);
    Ret::eax(r)
}

/// `handle_close` (48 bytes, 10 overlays).
pub const HANDLE_CLOSE: Sig = Sig {
    len: 48,
    hash: 0x9646_4b6f_8151_8d52,
    calls: &[],
    imports: &[],
};

/// `magic_0565` 0x100019b0 (internal, `ecx` = three 0x1c-byte timers):
/// each timer's `+0x8` 0, `+0xc` 20, `+0x18` 1. Returns `eax` as given.
pub fn timers_reset(g: &mut impl crate::Mem, t: u32, eax: u32) -> u32 {
    for k in 0..3 {
        g.write::<u32>(t + 0x1c * k + 8, 0);
        g.write::<u32>(t + 0x1c * k + 0xc, 0x14);
        g.write::<u32>(t + 0x1c * k + 0x18, 1);
    }
    eax
}

/// `timers_reset` (64 bytes, 10 overlays).
pub const TIMERS_RESET: Sig = Sig {
    len: 64,
    hash: 0xd6b5_aff6_2ded_27b9,
    calls: &[],
    imports: &[],
};

/// `magic_0565` 0x100078b0 (a table entry, no arguments): the effect
/// object `*G_OBJ`'s part 0 (slot 219) is shown (slot 257, 0), then
/// part 0 again (each call's words left below) gets slot 38 `(part,
/// 1.0, 10)`. Returns slot 38's `eax`.
pub fn part_show<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x01), 219);
    let o = g.read::<u32>(w(0x09));
    let p = g.call(t, 0, &[o, 0]).eax;
    let t = slot_target(g, w(0x19), 257);
    g.call(t, 0, &[p, 0, o, 0]);
    let t = slot_target(g, w(0x26), 219);
    let o2 = g.read::<u32>(w(0x2e));
    let p2 = g.call(t, 0, &[o2, 0, p, 0, o, 0]).eax;
    let t = slot_target(g, w(0x49), 38);
    Ret::eax(g.call(t, 0, &[p2, 0x3f80_0000, 0xa]).eax)
}

/// `part_show` (89 bytes, 10 overlays).
pub const PART_SHOW: Sig = Sig {
    len: 89,
    hash: 0x4942_16f2_98ad_621b,
    calls: &[],
    imports: &[],
};

/// `magic_0565` 0x10006c90 (internal, no arguments): the effect object
/// `*G_OBJ`'s part 0 (slot 219) gets slots 357 `(part, -1, 0x3c00)`,
/// 356 `(part, -1, 0x2ee0)`, 357 and 356 `(part, -1, -1)`, every call's
/// words left on the stack for the next. Returns the last `eax`.
pub fn part_ranges<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x01), 219);
    let o = g.read::<u32>(w(0x10));
    let p = g.call(t, 0, &[o, 0]).eax;
    let mut stack = vec![o, 0];
    let mut eax = p;
    for (op, s, x) in [
        (0x18, 357, 0x3c00),
        (0x2f, 356, 0x2ee0),
        (0x44, 357, u32::MAX),
        (0x56, 356, u32::MAX),
    ] {
        let t = slot_target(g, w(op), s);
        stack.splice(0..0, [p, u32::MAX, x]);
        // call width: 14 at the last (the earlier calls' words stay)
        eax = g.call(t, 0, &stack).eax;
    }
    eax
}

/// `part_ranges` (108 bytes, 10 overlays).
pub const PART_RANGES: Sig = Sig {
    len: 108,
    hash: 0x6517_a332_6fe4_2107,
    calls: &[],
    imports: &[],
};

/// `magic_0446` 0x10004c70 (internal, no arguments): the effect's camera
/// target (`*G_FX + 0x2f80`): slot 262 reads the model `*G_M`'s position
/// into it (w 1.0), slot 333's angle (ST0) gives `a = K - angle` folded
/// into range (minus `K3` above `K2`, plus `K3` below `K4`) into
/// `+0xebf0`, the next three words 0 and `G_READY` 1. Returns slot 333's
/// `eax`.
pub fn camera_target<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let o = CameraOps {
        fx: 0x08,
        t262: 0x1b,
        model: 0x22,
        t333: 0x2f,
        model2: 0x3c,
        k: [0x4f, 0x5e, 0x68, 0x72, 0x7f],
        vec: 0x2f80,
        out: [0xebf0, 0xebf8, 0xebf4, 0xebfc],
    };
    let (eax, _) = camera_fold(g, img, f, &o);
    g.write::<u32>(img.word(f + 0xac), 1);
    eax
}

/// `camera_target` (185 bytes, 10 overlays).
pub const CAMERA_TARGET: Sig = Sig {
    len: 185,
    hash: 0xfd69_2f6f_5a45_cf75,
    calls: &[],
    imports: &[],
};

/// `magic_0070` 0x10008640 (internal, `ecx` = `[node, parent, a, b, c,
/// d, e]`): the node appended to the parent's children (as in
/// [`crate::m0003::node_init`], no handler table), then `+0x4c` a,
/// `+0x44` b, `+0x50` c, `+0x40` d, `+0x48` e, byte `+0x54` 0, the
/// colour `+0x1c..+0x28` 1.0, and slot 678 on `+0x30`. Returns slot
/// 678's `eax`.
pub fn colour_node_init<G: Game>(g: &mut G, img: &Image, f: u32, args: u32) -> u32 {
    let node = g.read::<u32>(args);
    let parent = g.read::<u32>(args + 4);
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
        g.write::<u32>(node + 4, parent);
        let last = g.read::<u32>(parent + 0x10);
        g.write::<u32>(node + 8, last);
        let last = g.read::<u32>(parent + 0x10);
        if last == parent {
            g.write::<u32>(parent + 0xc, node);
        } else {
            g.write::<u32>(last + 4, node);
        }
        let n = g.read::<u16>(parent + 0x18);
        g.write::<u16>(parent + 0x18, n.wrapping_add(1));
        g.write::<u32>(parent + 0x10, node);
    }
    for (src, dst) in [
        (8, 0x4c),
        (0xc, 0x44),
        (0x18, 0x48),
        (0x10, 0x50),
        (0x14, 0x40),
    ] {
        let v = g.read::<u32>(args + src);
        g.write::<u32>(node + dst, v);
    }
    g.write::<u8>(node + 0x54, 0);
    for off in [0x28, 0x24, 0x20, 0x1c] {
        g.write::<u32>(node + off, 0x3f80_0000);
    }
    let t = slot_target(g, img.word(f + 0x5f), 678);
    g.call(t, 0, &[node + 0x30]).eax
}

/// `colour_node_init` (140 bytes, 10 overlays).
pub const COLOUR_NODE_INIT: Sig = Sig {
    len: 140,
    hash: 0xb5e5_a6f1_6f3d_38be,
    calls: &[],
    imports: &[],
};

/// `magic_0070` 0x100088d0 (a node handler, cdecl `(cursor)`): the walk
/// (`ecx` as given); when the cursor moved, the rings of the node's
/// objects `+0x3c` and (when set) `+0x40` are walked with a cursor in
/// the frame; slot 942 on the `+0x40` object's `+0x1c` (or 0) gives
/// `+0x38`, and the node's `+0x1c..+0x24` take the `+0x3c` object's.
/// Returns the last word copied, or the walk's `eax`.
pub fn follow_object<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let this = fr.arg(g, 0);
    let (eax, moved) = crate::nodes::walk(g, this, fr.ecx, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(this);
    // push ebp; sub esp, 8: the cursor is at [ebp - 8]
    let cursor = fr.esp.wrapping_sub(0xc);
    let enter = |g: &mut G, obj: u32| {
        let pass = g.read::<u8>(this + 5);
        g.write::<u32>(cursor, obj);
        g.write::<u8>(cursor + 5, pass);
        let h = g.read::<u32>(g.read::<u32>(obj) + 0x1c);
        g.call(h, cursor, &[cursor]);
    };
    let a = g.read::<u32>(n + 0x3c);
    enter(g, a);
    let b = g.read::<u32>(n + 0x40);
    let r = if b == 0 {
        let t = slot_target(g, w(0xa0), 942);
        g.call(t, 0, &[0]).eax
    } else {
        enter(g, b);
        let x = g.read::<u32>(n + 0x40).wrapping_add(0x1c);
        let t = slot_target(g, w(0x8e), 942);
        g.call(t, 0, &[x, cursor]).eax
    };
    g.write::<u32>(n + 0x38, r);
    let mut v = 0;
    for off in [0x1c, 0x20, 0x24] {
        let o = g.read::<u32>(n + 0x3c);
        v = g.read::<u32>(o + off);
        g.write::<u32>(n + off, v);
    }
    Ret::eax(v)
}

/// `follow_object` (214 bytes, 10 overlays).
pub const FOLLOW_OBJECT: Sig = Sig {
    len: 214,
    hash: 0xcb10_9da5_fff0_beff,
    calls: &[],
    imports: &[],
};

/// `magic_0446` 0x10001000 (a table entry, cdecl `(arg)`): the object
/// host's start-up with two parts: `G_ARG` = arg, the object block
/// ([`crate::l5_objhost::obj_block_init`]), slots 556/555/331 on the
/// three blocks, the three scales 1.0 and slot 790's float, the task
/// ring, pool A free, slot 720 (0x3400, 0x800), then slot 718 and slot
/// 222 `(part, 0, 11, 1)` on part D and slot 718, 222 and 221 `(part, 0,
/// 1000, 1)` on part E, every call's words left below until the end;
/// `G_DONE` 0. Returns 0.
pub fn host_start_c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let one = 0x3f80_0000;
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x08), arg);
    crate::l5_objhost::obj_block_init(g, img, img.rel32_target(f + 0x0d), arg);
    let (a, b, c) = (w(0x17), w(0x29), w(0x3b));
    let t = slot_target(g, w(0x12), 556);
    g.call(t, 0, &[a]);
    let t = slot_target(g, w(0x24), 555);
    g.call(t, 0, &[b, a]);
    let t = slot_target(g, w(0x36), 331);
    g.call(t, 0, &[c, b, a]);
    let t = slot_target(g, w(0x48), 790);
    for off in [0x57, 0x61, 0x6b] {
        g.write::<u32>(w(off), one);
    }
    let out = g.call(t, 0, &[]);
    g.write::<u32>(w(0x77), crate::sa_summon::st0_f32(out));
    crate::l5_objhost::tasks_init(g, img, img.rel32_target(f + 0x7c));
    for k in 0..0x3ff {
        g.write::<u32>(w(0x81) + 12 * k, 0);
    }
    g.write::<u16>(w(0xa3), 0x3ff);
    g.write::<u16>(w(0xab), 0);
    let t = slot_target(g, w(0xb0), 720);
    let mut stack = vec![0x3400, 0x800];
    // call width: 2 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let d = w(0xcc);
    let t = slot_target(g, w(0xc7), 718);
    stack.insert(0, d);
    // call width: 3 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xd9), 222);
    stack.splice(0..0, [w(0xea), 0, 0xb, 1]);
    // call width: 7 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xf1), 718);
    stack.insert(0, w(0xf6));
    // call width: 8 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x103), 222);
    stack.splice(0..0, [w(0x117), 0, 1000, 1]);
    // call width: 12 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x11e), 221);
    stack.splice(0..0, [w(0x132), 0, 1000, 1]);
    // call width: 16 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    g.write::<u32>(w(0x13d), 0);
    Ret::eax(0)
}

/// `host_start_c` (329 bytes, 10 overlays; the `rel32`s are the object
/// block's and the task ring's set-ups).
pub const HOST_START_C: Sig = Sig {
    len: 329,
    hash: 0x38e0_8e7b_b008_1246,
    calls: &[
        (0x0d, Callee::Sig(&crate::l5_objhost::OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&crate::l5_objhost::TASKS_INIT)),
    ],
    imports: &[],
};

/// `magic_0543` 0x10005840 (internal, no arguments): registers the
/// effect's 16 part handles: for each index, slot 941 maps the effect's
/// `+0x59b0`, slot 772 turns it and the index into a 64-bit handle and
/// slot 779 registers it with `**G_LOCK` 1 around the call. Returns
/// `*G_LOCK`.
pub fn register_16<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x03));
    let mut lock = 0;
    for i in 0..0x10 {
        let t = slot_target(g, w(0x11), 941);
        let x = g.read::<u32>(fx + 0x59b0);
        let r = g.call(t, 0, &[x]).eax;
        let t = slot_target(g, w(0x29), 772);
        let h = g.call(t, 0, &[r, i]);
        let l = g.read::<u32>(w(0x37));
        let saved = g.read::<u32>(l);
        g.write::<u32>(l, 1);
        let t = slot_target(g, w(0x40), 779);
        g.call(t, 0, &[h.eax, h.edx, r, i]);
        lock = g.read::<u32>(w(0x53));
        g.write::<u32>(lock, saved);
    }
    lock
}

/// `register_16` (102 bytes, 9 overlays).
pub const REGISTER_16: Sig = Sig {
    len: 102,
    hash: 0x7d2b_e3f5_c665_7465,
    calls: &[],
    imports: &[],
};

/// `magic_0006` 0x100065b0 (internal, `ecx` = an object, `edx` = a
/// block): the block's `+0x0`/`+0x8` 0 and `+0x4` the object's facing
/// as an int: `(K - angle) * K2 / K3` (SSE, truncated), the angle from
/// slot 333 on the model `+0x54` (ST0) with flag bit 0 (`+0x90`), else
/// from slot 601 on the object `+0x50` into a local (`[ebp - 4]`,
/// holding the object until the slot writes it: `push ecx`). Returns
/// the int.
pub fn facing_int<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, out: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let local = esp.wrapping_sub(8);
    g.write::<u32>(local, obj);
    g.write::<u32>(out + 8, 0);
    g.write::<u32>(out, 0);
    let x = if g.read::<u8>(obj + 0x90) & 1 != 0 {
        let t = slot_target(g, w(0x1e), 333);
        let m = g.read::<u32>(obj + 0x54);
        let a = crate::sa_summon::st0_f32(g.call(t, 0, &[m]));
        g.write::<u32>(local, a);
        sub(f32_at(g, w(0x34)), f32::from_bits(a))
    } else {
        let t = slot_target(g, w(0x47), 601);
        let o = g.read::<u32>(obj + 0x50);
        g.call(t, 0, &[o, local]);
        sub(f32_at(g, w(0x5a)), f32_at(g, local))
    };
    let v = div(mul(x, f32_at(g, w(0x6a))), f32_at(g, w(0x72)));
    let n = ffx_bridge::x87::cvttsd2si(f64::from(v));
    g.write::<u32>(out + 4, n);
    n
}

/// `facing_int` (130 bytes, 9 overlays).
pub const FACING_INT: Sig = Sig {
    len: 130,
    hash: 0x9753_60d3_512c_bfa7,
    calls: &[],
    imports: &[],
};

/// `magic_0155` 0x10003ec0 (internal, `ecx` = `[obj, out]`): `out` = the
/// object's model id: slot 20 on the model `+0x54` with flag bit 0
/// (`+0x90`), else slot 411 on the object `+0x50`. Returns it.
pub fn model_id<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let o = g.read::<u32>(this);
    let r = if g.read::<u8>(o + 0x90) & 1 != 0 {
        let x = g.read::<u32>(o + 0x54);
        let t = slot_target(g, img.word(f + 0x12), 20);
        g.call(t, 0, &[x]).eax
    } else {
        let x = g.read::<u32>(o + 0x50);
        let t = slot_target(g, img.word(f + 0x27), 411);
        g.call(t, 0, &[x]).eax
    };
    g.write::<u32>(this + 4, r);
    r
}

/// `model_id` (59 bytes, 8 overlays).
pub const MODEL_ID: Sig = Sig {
    len: 59,
    hash: 0xd1ea_d4c0_c060_e5a2,
    calls: &[],
    imports: &[],
};

/// `magic_0126` 0x10001a60 (internal, `ecx` = an actor, -1 the caller):
/// slot 390 gives the actor's kind; a kind in the DLL's list (words
/// ending with -1) answers 1, any other makes slot 534 on the actor.
/// Returns slot 534's `eax`.
pub fn actor_kind_check<G: Game>(g: &mut G, img: &Image, f: u32, i: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let who = |g: &G, me: u32, list: u32| {
        if i == u32::MAX {
            g.read::<u32>(w(me))
        } else {
            let l = g.read::<u32>(w(list));
            u32::from(g.read::<u8>(l.wrapping_add(i)))
        }
    };
    let n = who(g, 0x09, 0x10);
    let t = slot_target(g, w(0x1a), 390);
    let kind = g.call(t, 0, &[n]).eax;
    let mut at = w(0x30);
    let mut k = g.read::<u32>(w(0x28));
    while k != u32::MAX {
        if k == kind {
            return 1;
        }
        k = g.read::<u32>(at + 4);
        at += 4;
    }
    let n = who(g, 0x55, 0x74);
    let t = slot_target(g, w(if i == u32::MAX { 0x5b } else { 0x7e }), 534);
    g.call(t, 0, &[n]).eax
}

/// `actor_kind_check` (143 bytes, 8 overlays).
pub const ACTOR_KIND_CHECK: Sig = Sig {
    len: 143,
    hash: 0x0796_d46a_6ebf_01ab,
    calls: &[],
    imports: &[],
};

/// `magic_0165` 0x10006b70 (internal, `edx` = a matrix, the stack word a
/// vector): slot 235 resets the matrix; slot 277's scale (ST0) times `K`
/// goes on its diagonal (`+0x0`, `+0x14`, `+0x28`), each then scaled by
/// slots 15, 16, 17 (ST0) of the handle `*G_H`; slots 330, 328 and 329
/// rotate it by the vector's `+0x8`, `+0x0`, `+0x4` (each call's words
/// left below until the original pops them). Returns slot 329's `eax`.
pub fn scale_rotate<G: Game>(g: &mut G, img: &Image, f: u32, m: u32, v: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let st = |out: crate::Out| f32::from_bits(crate::sa_summon::st0_f32(out));
    let t = slot_target(g, w(0x05), 235);
    g.call(t, 0, &[m]);
    let h = g.read::<u32>(w(0x1d));
    let t = slot_target(g, w(0x17), 277);
    let s = mul(st(g.call(t, 0, &[h, m])), f32_at(g, w(0x35)));
    for off in [0x28, 0x14, 0] {
        g.write::<u32>(m + off, s.to_bits());
    }
    let h2 = g.read::<u32>(w(0x4e));
    let mut stack = vec![h, m];
    for (op, slot, off) in [(0x3a, 15, 0), (0x65, 16, 0x14), (0x81, 17, 0x28)] {
        let t = slot_target(g, w(op), slot);
        stack.insert(0, h2);
        // call width: 5 (the earlier calls' words stay on the stack)
        let a = st(g.call(t, 0, &stack));
        let x = mul(f32_at(g, m + off), a);
        g.write::<u32>(m + off, x.to_bits());
    }
    let mut eax = 0;
    for (op, slot, i) in [(0xa1, 330, 2), (0xc6, 328, 0), (0xe2, 329, 1)] {
        let t = slot_target(g, w(op), slot);
        let x = g.read::<u32>(v + 4 * i);
        eax = g.call(t, 0, &[m, m, x]).eax;
    }
    eax
}

/// `scale_rotate` (257 bytes, 8 overlays).
pub const SCALE_ROTATE: Sig = Sig {
    len: 257,
    hash: 0x660e_bec7_294f_264d,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x100026b0 (internal, `ecx` = a sound record, `edx` =
/// a value): a record whose `+0x0` is not 0xff starts: `+0x10` 0, `+0x8`
/// the value, `+0xc` slot 228 `(id, value)`. Returns that, or 0.
pub fn sound_start<G: Game>(g: &mut G, img: &Image, f: u32, p: u32, x: u32) -> u32 {
    let v = g.read::<u32>(p);
    if v == 0xff {
        return 0;
    }
    g.write::<u32>(p + 0x10, 0);
    g.write::<u32>(p + 8, x);
    let t = slot_target(g, img.word(f + 0x13), 228);
    let r = g.call(t, 0, &[v, x]).eax;
    g.write::<u32>(p + 0xc, r);
    r
}

/// `sound_start` (49 bytes, 7 overlays).
pub const SOUND_START: Sig = Sig {
    len: 49,
    hash: 0xd237_1b00_5c32_6912,
    calls: &[],
    imports: &[],
};

/// `magic_0251` 0x10001480 (internal, `ecx` = an actor, `edx` = a
/// matrix): slot 235 resets the matrix; slot 277's scale of the actor's
/// object (ST0) times `K` goes on its diagonal, each then scaled by
/// slots 15, 16, 17 (ST0) of the object (their words left on the stack
/// for the next). Returns slot 17's `eax`.
pub fn actor_scale<G: Game>(g: &mut G, img: &Image, f: u32, i: u32, m: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let st = |out: crate::Out| f32::from_bits(crate::sa_summon::st0_f32(out));
    let obj = |g: &G, me: u32, list: u32, objs: u32| {
        if i == u32::MAX {
            g.read::<u32>(w(me))
        } else {
            let l = g.read::<u32>(w(list));
            let n = u32::from(g.read::<u8>(l.wrapping_add(i)));
            g.read::<u32>(w(objs).wrapping_add(4 * n))
        }
    };
    let t = slot_target(g, w(0x05), 235);
    g.call(t, 0, &[m]);
    let o = obj(g, 0x21, 0x28, 0x33);
    let t = slot_target(g, w(0x39), 277);
    let s = mul(st(g.call(t, 0, &[o])), f32_at(g, w(0x51)));
    for off in [0x28, 0x14, 0] {
        g.write::<u32>(m + off, s.to_bits());
    }
    let o2 = obj(g, 0x6d, 0x74, 0x7f);
    let mut stack = Vec::new();
    let mut eax = 0;
    for (op, slot, off) in [(0x84, 15, 0), (0x9b, 16, 0x14), (0xb7, 17, 0x28)] {
        let t = slot_target(g, w(op), slot);
        stack.push(o2);
        // call width: 3 (the earlier calls' words stay on the stack)
        let out = g.call(t, 0, &stack);
        eax = out.eax;
        let x = mul(f32_at(g, m + off), st(out));
        g.write::<u32>(m + off, x.to_bits());
    }
    eax
}

/// `actor_scale` (225 bytes, 7 overlays).
pub const ACTOR_SCALE: Sig = Sig {
    len: 225,
    hash: 0xe6b4_5c65_8c47_0daa,
    calls: &[],
    imports: &[],
};

/// `magic_0070` 0x10001000 (internal, `ecx` = the cursor; reached by a
/// tail jump): the walk, then (moved or not) a node with a count byte
/// `+0x1c` above 1 and a positive time `+0x20`: slot 245 into a local
/// (`[ebp - 8]`), slot 821(0), slot 822 `(1.0, 1.0, 1.0, count * K2)`,
/// slot 823 `(K / time, K / time)` and slot 824 `(0.4990, 0.7320, 0,
/// 0)` (the original's literal words), each reusing the earlier call's
/// stack words as the original does. Returns slot 824's `eax`, or the
/// node.
pub fn blur_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.ecx;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    crate::nodes::walk(g, this, this, fr.eax);
    let n = g.read::<u32>(this);
    if g.read::<u8>(n + 0x1c) == 0 {
        return Ret::eax(n);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let t = slot_target(g, w(0x56), 245);
    g.call(t, 0, &[ebp.wrapping_sub(8)]);
    let n = g.read::<u32>(this);
    let time = f32_at(g, n + 0x20);
    let k = g.read::<u8>(n + 0x1c);
    let positive = time > 0.0;
    if !positive || k <= 1 {
        return Ret::eax(n);
    }
    let x0 = div(f32_at(g, w(0x89)), time);
    g.write::<u32>(ebp.wrapping_sub(4), x0.to_bits());
    let t = slot_target(g, w(0x8e), 821);
    g.call(t, 0, &[0]);
    let t = slot_target(g, w(0xa6), 822);
    let xk = mul(cvt(i32::from(k)), f32_at(g, w(0xbe)));
    let one = 0x3f80_0000;
    g.call(t, 0, &[one, one, one, xk.to_bits()]);
    let t = slot_target(g, w(0xe7), 823);
    g.call(t, 0, &[x0.to_bits(), x0.to_bits()]);
    let t = slot_target(g, w(0x102), 824);
    Ret::eax(g.call(t, 0, &[0x3eff_8000, 0x3f3b_6276, 0, 0]).eax)
}

/// `blur_setup` (312 bytes, 6 overlays).
pub const BLUR_SETUP: Sig = Sig {
    len: 312,
    hash: 0xf976_1bc1_ad6a_8dee,
    calls: &[],
    imports: &[],
};

/// `magic_0446` 0x100049b0 (a table entry, no arguments): the effect's
/// `+0xd748` 1. Returns the effect.
pub fn fx_flag<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let fx = g.read::<u32>(img.word(fr.at + 0x01));
    g.write::<u32>(fx + 0xd748, 1);
    Ret::eax(fx)
}

/// `fx_flag` (16 bytes, 6 overlays).
pub const FX_FLAG: Sig = Sig {
    len: 16,
    hash: 0xccab_a2b3_18b8_0cf0,
    calls: &[],
    imports: &[],
};

/// `magic_0155` 0x10008e70 (internal, `ecx` = an object, `edx` = a
/// vector): slot 262 reads the model `+0x54`'s position into the vector
/// and slot 12's height (ST0) into its `+0x4` (slot 262's words left
/// below). Returns slot 12's `eax`.
pub fn model_position<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, v: u32) -> u32 {
    let t = slot_target(g, img.word(f + 0x07), 262);
    let m = g.read::<u32>(obj + 0x54);
    g.call(t, 0, &[m, v, v + 4, v + 8]);
    let t = slot_target(g, img.word(f + 0x1e), 12);
    let m2 = g.read::<u32>(obj + 0x54);
    let out = g.call(t, 0, &[m2, m, v, v + 4, v + 8]);
    g.write::<u32>(v + 4, crate::sa_summon::st0_f32(out));
    out.eax
}

/// `model_position` (50 bytes, 6 overlays).
pub const MODEL_POSITION: Sig = Sig {
    len: 50,
    hash: 0xb446_1e98_a2e1_c7ba,
    calls: &[],
    imports: &[],
};

/// `magic_0098` 0x10003260 (a table entry, no arguments): slot 384 on
/// the handle `*G_H`, slot 25 `(0x3015, 1)` and slot 804 on `*G_X` (each
/// call's words left below). Returns 1.
pub fn handle_3015<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let h = g.read::<u32>(w(0x07));
    let t = slot_target(g, w(0x01), 384);
    g.call(t, 0, &[h]);
    let t = slot_target(g, w(0x14), 25);
    g.call(t, 0, &[0x3015, 1, h]);
    let t = slot_target(g, w(0x25), 804);
    let x = g.read::<u32>(w(0x2b));
    g.call(t, 0, &[x, 0x3015, 1, h]);
    Ret::eax(1)
}

/// `handle_3015` (64 bytes, 6 overlays).
pub const HANDLE_3015: Sig = Sig {
    len: 64,
    hash: 0xfb09_fcda_9f54_7937,
    calls: &[],
    imports: &[],
};

/// `magic_0393` 0x100015a0 (a table entry, no arguments): slot 628(0),
/// slot 311 on the handle `*G_H` (then -1), and slot 622 `(*G_X, the
/// byte **G_P)` into `G_R` (the earlier words left below). Returns it.
pub fn handle_swap<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x01), 628);
    g.call(t, 0, &[0]);
    let t = slot_target(g, w(0x10), 311);
    let h = g.read::<u32>(w(0x16));
    g.call(t, 0, &[h, 0]);
    let p = g.read::<u32>(w(0x23));
    g.write::<u32>(w(0x29), u32::MAX);
    let b = u32::from(g.read::<u8>(p));
    let t = slot_target(g, w(0x36), 622);
    let x = g.read::<u32>(w(0x3c));
    let r = g.call(t, 0, &[x, b, h, 0]).eax;
    g.write::<u32>(w(0x4c), r);
    Ret::eax(r)
}

/// `handle_swap` (81 bytes, 6 overlays).
pub const HANDLE_SWAP: Sig = Sig {
    len: 81,
    hash: 0xabd5_330a_a9a9_c9e9,
    calls: &[],
    imports: &[],
};

/// `magic_0376` 0x10002670 (a table entry, no arguments): the loading
/// steps: step A waits for slot 225(0x2000) to say 0; then, once, slot
/// 226 `(*G_FILE, 0)` (flag `G_STARTED`); step B waits for slot
/// 227(`*G_FILE`) to reach 1. Returns 1 while waiting, else the last
/// call's `eax` (`eax` as given when everything is done).
pub fn load_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut eax = fr.eax;
    if g.read::<u32>(w(0x02)) == 0 {
        let t = slot_target(g, w(0x0a), 225);
        eax = g.call(t, 0, &[0x2000]).eax;
        if eax != 0 {
            return Ret::eax(1);
        }
        let v = g.read::<u32>(w(0x24));
        g.write::<u32>(w(0x24), v.wrapping_add(1));
    }
    if g.read::<u32>(w(0x2a)) == 0 {
        let t = slot_target(g, w(0x32), 226);
        let file = g.read::<u32>(w(0x3a));
        g.write::<u32>(w(0x46), 1);
        eax = g.call(t, 0, &[file, 0]).eax;
    }
    if g.read::<u32>(w(0x55)) != 0 {
        return Ret::eax(eax);
    }
    let t = slot_target(g, w(0x5d), 227);
    let file = g.read::<u32>(w(0x63));
    let r = g.call(t, 0, &[file]).eax;
    if r.cast_signed() < 1 {
        return Ret::eax(1);
    }
    let v = g.read::<u32>(w(0x79));
    g.write::<u32>(w(0x79), v.wrapping_add(1));
    Ret::eax(r)
}

/// `load_wait` (132 bytes, 6 overlays).
pub const LOAD_WAIT: Sig = Sig {
    len: 132,
    hash: 0x8783_7a66_6bd1_5692,
    calls: &[],
    imports: &[],
};

/// `magic_0393` 0x100014f0 (a script command, cdecl `(obj)`): a small
/// state machine on `G_STATE` with the counter `G_N` and the handle
/// `*G_H`: 0 counts down and at 0 closes (slot 630 `(h, 0)`, state 1);
/// 1 waits for slot 629(h) to be non-negative (counter 0, state 2); 2
/// counts up to 10, then state 3, slot 802(-1) and the object's mode
/// `+0x56` 1. Returns the last value the original leaves in `eax` (the
/// counter, the call's answer, the object, or the state minus 2 for an
/// unknown state).
pub fn fade_states<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let state = g.read::<u32>(w(0x04));
    let h = g.read::<u32>(w(0x0a));
    match state {
        0 => {
            let c = g.read::<u32>(w(0x7e)).wrapping_sub(1);
            g.write::<u32>(w(0x84), c);
            if c.cast_signed() > 0 {
                return Ret::eax(c);
            }
            let t = slot_target(g, w(0x8d), 630);
            let r = g.call(t, 0, &[h, 0]).eax;
            g.write::<u32>(w(0xa1), 1);
            Ret::eax(r)
        }
        1 => {
            let t = slot_target(g, w(0x53), 629);
            let r = g.call(t, 0, &[h]).eax;
            if r.cast_signed() >= 0 {
                g.write::<u32>(w(0x69), 0);
                g.write::<u32>(w(0x73), 2);
            }
            Ret::eax(r)
        }
        2 => {
            let c = g.read::<u32>(w(0x1e)).wrapping_add(1);
            g.write::<u32>(w(0x24), c);
            if c.cast_signed() < 0xa {
                return Ret::eax(c);
            }
            let t = slot_target(g, w(0x2e), 802);
            g.write::<u32>(w(0x3c), 3);
            g.call(t, 0, &[u32::MAX]);
            let obj = fr.arg(g, 0);
            g.write::<u8>(obj + 0x56, 1);
            Ret::eax(obj)
        }
        _ => Ret::eax(state.wrapping_sub(2)),
    }
}

/// `fade_states` (171 bytes, 6 overlays).
pub const FADE_STATES: Sig = Sig {
    len: 171,
    hash: 0x433a_a71b_9e1c_dda0,
    calls: &[],
    imports: &[],
};

/// `magic_0171` 0x10003510 (internal, `ecx` = the record block):
/// [`records_init`] for 0x100 records of 0x50 bytes (`+0x48` cleared).
pub fn records_init_100<G: Game>(g: &mut G, img: &Image, f: u32, block: u32) -> u32 {
    records_init_n(g, img, f, block, (0x48, 0x50, 0x100))
}

/// `magic_0053` 0x10002330 (internal, `ecx` = the record block):
/// [`records_init`] for 0x80 records of 0x74 bytes (`+0x6c` cleared).
pub fn records_init_80<G: Game>(g: &mut G, img: &Image, f: u32, block: u32) -> u32 {
    records_init_n(g, img, f, block, (0x6c, 0x74, 0x80))
}

/// `records_init_80` (221 bytes, 2 overlays).
pub const RECORDS_INIT_80: Sig = Sig {
    len: 221,
    hash: 0x8b9d_8567_38f4_2f5b,
    calls: &[],
    imports: &[],
};

/// `records_init_100` (221 bytes, 6 overlays).
pub const RECORDS_INIT_100: Sig = Sig {
    len: 221,
    hash: 0xcc49_e2a5_c29d_386d,
    calls: &[],
    imports: &[],
};

/// `magic_0042` 0x100019c0 (internal, `ecx` = a request): the ring
/// reservation of [`crate::m0070::ring_take`] as this copy computes it:
/// a wrapping take gives the second piece the old head as its length
/// (`cap - (cap - head)`, the original's arithmetic). Returns the ring's
/// base for a wrapping span, else its length (0 for a request larger
/// than the ring).
pub fn ring_reserve(g: &mut impl crate::Mem, req: u32) -> u32 {
    let ring = g.read::<u32>(req);
    let n = g.read::<u16>(req + 4);
    let cap = g.read::<u16>(ring + 0x1c);
    if n > cap {
        g.write::<u16>(req + 6, 0);
        return 0;
    }
    let head = g.read::<u16>(ring + 0x30);
    let end = n.wrapping_add(head);
    let stride = u32::from(g.read::<u16>(ring + 0x1e));
    let base = g.read::<u32>(ring + 0x20);
    g.write::<u32>(req + 0xc, (u32::from(head) * stride).wrapping_add(base));
    if end > cap {
        let head = g.read::<u16>(ring + 0x30);
        g.write::<u32>(req + 0x10, base);
        let first = cap.wrapping_sub(head);
        g.write::<u16>(req + 6, first);
        g.write::<u16>(req + 8, cap.wrapping_sub(first));
        g.write::<u16>(ring + 0x30, end.wrapping_sub(cap));
    } else {
        let n = g.read::<u16>(req + 4);
        g.write::<u16>(req + 6, n);
        g.write::<u16>(req + 8, 0);
        g.write::<u16>(ring + 0x30, if end == cap { 0 } else { end });
    }
    let used = g.read::<u16>(ring + 0x32);
    if used < cap {
        let used = g.read::<u16>(req + 4).wrapping_add(used);
        g.write::<u16>(ring + 0x32, used);
        if used > cap {
            g.write::<u16>(ring + 0x32, cap);
        }
    } else {
        let head = g.read::<u16>(ring + 0x30);
        g.write::<u16>(ring + 0x34, head);
    }
    let stride = u32::from(g.read::<u16>(ring + 0x1e));
    let start = g.read::<u16>(ring + 0x34);
    let at = (u32::from(start) * stride).wrapping_add(g.read::<u32>(ring + 0x20));
    g.write::<u32>(ring + 0x24, at);
    let start = g.read::<u16>(ring + 0x34);
    let used = g.read::<u16>(ring + 0x32);
    let cap = g.read::<u16>(ring + 0x1c);
    if start.wrapping_add(used) > cap {
        let first = cap.wrapping_sub(start);
        g.write::<u16>(ring + 0x2c, first);
        let rest = g.read::<u16>(ring + 0x32).wrapping_sub(first);
        g.write::<u16>(ring + 0x2e, rest);
        let base = g.read::<u32>(ring + 0x20);
        g.write::<u32>(ring + 0x28, base);
        base
    } else {
        g.write::<u16>(ring + 0x2c, used);
        g.write::<u16>(ring + 0x2e, 0);
        let stride = u32::from(g.read::<u16>(ring + 0x1e));
        let used = u32::from(g.read::<u16>(ring + 0x32));
        let after = (stride * used).wrapping_add(g.read::<u32>(ring + 0x20));
        g.write::<u32>(ring + 0x28, after);
        used
    }
}

/// `ring_reserve` (247 bytes, 6 overlays).
pub const RING_RESERVE: Sig = Sig {
    len: 247,
    hash: 0xfb23_3fc7_412f_736d,
    calls: &[],
    imports: &[],
};

/// `magic_0544` 0x10005280 (internal, `ecx` = a rotation vector, `edx`
/// = a position; an aligned /GS frame): slot 562, then the effect object
/// `*G_OBJ`'s part 0 (slot 219): a matrix in the frame (`M`, at the
/// aligned `esp + 0x18`) reset (slot 235), its diagonal the part's scale
/// (slot 277, ST0) times slots 15/16/17 (ST0), rotated by the vector's
/// `+0x8`, `+0x0`, `+0x4` (slots 330, 328, 329), the position's four
/// words as its last row; slot 825 on the part and slot 843 `(that,
/// M)`. Every call passes the words the original's stack holds there.
/// Returns slot 843's `eax`.
pub fn part_matrix<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let st = |out: crate::Out| f32::from_bits(crate::sa_summon::st0_f32(out));
    let (v, p) = (fr.ecx, fr.edx);
    // push ebp; and esp, -8; sub esp, 0x50; push esi; push edi
    let a = ((fr.esp.wrapping_sub(4)) & !7).wrapping_sub(0x58);
    let m = a + 0x10;
    g.write::<u32>(a + 0xc, p);
    let t = slot_target(g, w(0x15), 562);
    g.call(t, 0, &[]);
    let o = g.read::<u32>(w(0x32));
    let t = slot_target(g, w(0x2a), 219);
    let r = g.call(t, 0, &[o, 0]).eax;
    let mut stack = vec![m, o, 0];
    let t = slot_target(g, w(0x40), 235);
    // call width: 3 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    stack.insert(0, r);
    let t = slot_target(g, w(0x55), 277);
    // call width: 4 (the earlier calls' words stay on the stack)
    let s = crate::sa_summon::st0_f32(g.call(t, 0, &stack));
    g.write::<u32>(a + 8, s);
    for off in [0x28, 0x14, 0] {
        g.write::<u32>(m + off, s);
    }
    for (op, slot, off) in [(0x6d, 15, 0), (0x9a, 16, 0x14), (0xbb, 17, 0x28)] {
        let t = slot_target(g, w(op), slot);
        stack.insert(0, r);
        // call width: 7 (the earlier calls' words stay on the stack)
        let x = st(g.call(t, 0, &stack));
        g.write::<u32>(a + 8, x.to_bits());
        let y = mul(f32_at(g, m + off), x);
        g.write::<u32>(m + off, y.to_bits());
    }
    for (op, slot, i) in [(0xf5, 330, 2), (0x114, 328, 0), (0x134, 329, 1)] {
        let t = slot_target(g, w(op), slot);
        let x = g.read::<u32>(v + 4 * i);
        g.write::<u32>(a.wrapping_sub(4), x);
        g.call(t, 0, &[m, m, x]);
    }
    let p = g.read::<u32>(a + 0xc);
    for i in 0..4 {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(m + 0x30 + 4 * i, x);
    }
    let t = slot_target(g, w(0x16a), 825);
    let v1 = g.read::<u32>(v + 4);
    let q = g.call(t, 0, &[r, m, m, v1]).eax;
    let t = slot_target(g, w(0x184), 843);
    g.call(t, 0, &[q, m, r, m, m, v1]).eax
}

/// `part_matrix` (420 bytes, 6 overlays; the `rel32` is
/// `__security_check_cookie`).
pub const PART_MATRIX: Sig = Sig {
    len: 420,
    hash: 0x1d0e_f194_b185_f910,
    calls: &[(0x19c, Callee::Any)],
    imports: &[],
};

/// `magic_0644` 0x1000b7b0 (a table entry, no arguments): the effect's
/// `+0xd618` 1. Returns the effect.
pub fn fx_flag_d618<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let fx = g.read::<u32>(img.word(fr.at + 0x01));
    g.write::<u32>(fx + 0xd618, 1);
    Ret::eax(fx)
}

/// `fx_flag_d618` (16 bytes, 5 overlays).
pub const FX_FLAG_D618: Sig = Sig {
    len: 16,
    hash: 0x12ea_fff1_ba9a_658d,
    calls: &[],
    imports: &[],
};

/// `magic_0644` 0x10009f70 (a table entry, no arguments): takes the
/// effect's object (`*G_FX + 0x11d0`) out of the DLL's list (the object
/// host's `list_remove`); when it was there, `**G_PART` = the DLL's part
/// block, slot 713 releases the object and bit 15 of its flags `+0x124`
/// is cleared. Returns 0x7fff then, else 0.
pub fn release_fx_object<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let o = g.read::<u32>(w(0x03)).wrapping_add(0x11d0);
    let rm = img.rel32_target(f + 0x15);
    if crate::l5_objhost::list_remove(g, img, rm, w(0x10), o) == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0x1e));
    g.write::<u32>(p, w(0x25));
    let t = slot_target(g, w(0x2a), 713);
    g.call(t, 0, &[o]);
    let v = g.read::<u16>(o + 0x124);
    g.write::<u16>(o + 0x124, v & 0x7fff);
    Ret::eax(0x7fff)
}

/// `release_fx_object` (71 bytes, 5 overlays).
pub const RELEASE_FX_OBJECT: Sig = Sig {
    len: 71,
    hash: 0x4b47_fcef_1baf_16a6,
    calls: &[(0x15, Callee::Sig(&crate::l5_objhost::LIST_REMOVE))],
    imports: &[],
};

/// `magic_0384` 0x100023b0 (a table entry, cdecl `(arg)`): slot 436
/// opens the effect's memory (`*G_H`, arg, the DLL's two blocks, 0xc0000
/// bytes from the arena end - 0x1c0000, 1, two more of the DLL's words)
/// and slot 434 sets its callback (slot 436's words left below).
/// Returns 0.
pub fn mem_open<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let arg = fr.arg(g, 0);
    let t = slot_target(g, w(0x1b), 436);
    let h = g.read::<u32>(w(0x39));
    let words = [
        h,
        arg,
        w(0x30),
        w(0x2b),
        0xc_0000,
        w(0x10).wrapping_sub(0x1c_0000),
        1,
        w(0x09),
        w(0x04),
    ];
    g.call(t, 0, &words);
    let t = slot_target(g, w(0x40), 434);
    let h = g.read::<u32>(w(0x4b));
    let mut stack = vec![h, w(0x45)];
    stack.extend(words);
    // call width: 11 (slot 436's words stay on the stack)
    g.call(t, 0, &stack);
    Ret::eax(0)
}

/// `mem_open` (94 bytes, 5 overlays).
pub const MEM_OPEN: Sig = Sig {
    len: 94,
    hash: 0x24cf_b8fd_f411_6941,
    calls: &[],
    imports: &[],
};

/// `magic_0565` 0x10005b00 (internal, no arguments): when the effect
/// (`*G_FX`) has its `+0xd748` flag at 1, part 0 of the object `*G_OBJ`
/// (slot 219) is shown (slot 257, 1); then the effect's `+0xd740`,
/// `+0xd744` and `+0xd750` are cleared and `+0xd74c` takes the word at
/// `+0x4` of its `+0xec94` record. Returns that word.
pub fn fx_reset_d74c<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x03));
    if g.read::<u32>(fx + 0xd748) == 1 {
        let t = slot_target(g, w(0x11), 219);
        let o = g.read::<u32>(w(0x19));
        let p = g.call(t, 0, &[o, 0]).eax;
        let t = slot_target(g, w(0x29), 257);
        g.call(t, 0, &[p, 1, o, 0]);
    }
    g.write::<u32>(fx + 0xd740, 0);
    g.write::<u32>(fx + 0xd744, 0);
    let rec = g.read::<u32>(fx + 0xec94);
    let v = g.read::<u32>(rec + 4);
    g.write::<u32>(fx + 0xd74c, v);
    g.write::<u32>(fx + 0xd750, 0);
    v
}

/// `fx_reset_d74c` (103 bytes, 5 overlays).
pub const FX_RESET_D74C: Sig = Sig {
    len: 103,
    hash: 0x91be_f4ca_3f39_cc89,
    calls: &[],
    imports: &[],
};

/// `magic_0173` 0x10006a10 (internal, `ecx` = a node): the node is
/// appended to the DLL's static parent node `P` (pass byte 0xff, an empty
/// child list, links, `P`'s count up), takes the DLL's handler table and
/// `+0x1c`, `+0x20`, `+0x24`, `+0x2c` 0. Returns `P`'s last child before
/// it.
pub fn static_child<G: Game>(g: &mut G, img: &Image, f: u32, n: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let p = w(0x0f);
    g.write::<u16>(n + 0x1a, 0xff);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, p);
    g.write::<u32>(n + 4, w(0x16));
    let last = g.read::<u32>(w(0x1b));
    g.write::<u32>(n + 8, last);
    let last = g.read::<u32>(w(0x23));
    if last == w(0x28) {
        g.write::<u32>(w(0x30), n);
    } else {
        g.write::<u32>(last + 4, n);
    }
    let c = g.read::<u16>(w(0x3c));
    g.write::<u16>(w(0x3c), c.wrapping_add(1));
    g.write::<u32>(w(0x42), n);
    g.write::<u32>(n + 0x2c, 0);
    g.write::<u32>(n + 0x20, 0);
    g.write::<u32>(n, w(0x56));
    g.write::<u32>(n + 0x1c, 0);
    g.write::<u32>(n + 0x24, 0);
    last
}

/// `static_child` (105 bytes, 5 overlays).
pub const STATIC_CHILD: Sig = Sig {
    len: 105,
    hash: 0x7667_a5b9_2f73_fb48,
    calls: &[],
    imports: &[],
};

/// `magic_0644` 0x10005d20 (internal, `ecx` = three reels of 0x1c
/// bytes: `+0x0` a strip `{count, symbols}`, `i16 +0x6` the position):
/// whether the symbol after each reel's position (wrapping at the strip's
/// count) is the same on all three: `+0x70` 1 and returns 1, else 0.
pub fn reels_match<M: crate::Mem>(g: &mut M, o: u32) -> u32 {
    let sym = |g: &M, row: u32| {
        let strip = g.read::<u32>(row);
        let mut i = i32::from(g.read::<i16>(row + 6)) + 1;
        if i >= g.read::<i32>(strip) {
            i = 0;
        }
        let syms = g.read::<u32>(strip + 4);
        g.read::<u8>(syms.wrapping_add(i.cast_unsigned()))
    };
    let first = sym(g, o);
    for k in 1..3 {
        if sym(g, o + 0x1c * k) != first {
            g.write::<u32>(o + 0x70, 0);
            return 0;
        }
    }
    g.write::<u32>(o + 0x70, 1);
    1
}

/// `reels_match` (113 bytes, 5 overlays).
pub const REELS_MATCH: Sig = Sig {
    len: 113,
    hash: 0x30f1_453b_5eab_4be7,
    calls: &[],
    imports: &[],
};

/// The operand offsets and fields of the two camera targets.
struct CameraOps {
    fx: u32,
    t262: u32,
    model: u32,
    t333: u32,
    model2: u32,
    /// `K`, `K2` (upper bound), `K3` (the turn, twice), `K4` (lower bound).
    k: [u32; 5],
    /// The vector's offset in the effect.
    vec: u32,
    /// The angle's field and the three cleared after it, in order.
    out: [u32; 4],
}

/// The camera target of [`camera_target`] and [`camera_target_b`]:
/// slot 262 reads the model's position into the effect's vector (w
/// 1.0), slot 333's angle (ST0) gives `a = K - angle` folded into range.
/// Returns slot 333's `eax`.
fn camera_fold<G: Game>(g: &mut G, img: &Image, f: u32, o: &CameraOps) -> (u32, u32) {
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(o.fx));
    let v = fx + o.vec;
    let t = slot_target(g, w(o.t262), 262);
    let m = g.read::<u32>(w(o.model));
    g.call(t, 0, &[m, v, v + 4, v + 8]);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    let t = slot_target(g, w(o.t333), 333);
    let m2 = g.read::<u32>(w(o.model2));
    let out = g.call(t, 0, &[m2, m, v, v + 4, v + 8]);
    let angle = f32::from_bits(crate::sa_summon::st0_f32(out));
    let mut a = sub(f32_at(g, w(o.k[0])), angle);
    if a > f32_at(g, w(o.k[1])) {
        a = sub(a, f32_at(g, w(o.k[2])));
    } else if f32_at(g, w(o.k[3])) > a {
        a = add(a, f32_at(g, w(o.k[4])));
    }
    g.write::<u32>(fx + o.out[0], a.to_bits());
    for &off in &o.out[1..] {
        g.write::<u32>(fx + off, 0);
    }
    (out.eax, fx)
}

/// `magic_0644` 0x1000ab30 (internal, no arguments): the camera target
/// of [`camera_target`] at the effect's `+0x2e50` into `+0xeac0`, then
/// `G_READY` 1 and `G_A`/`G_B` -1. Returns slot 333's `eax`.
pub fn camera_target_b<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let o = CameraOps {
        fx: 0x08,
        t262: 0x1b,
        model: 0x22,
        t333: 0x2f,
        model2: 0x3c,
        k: [0x4f, 0x5e, 0x68, 0x72, 0x7f],
        vec: 0x2e50,
        out: [0xeac0, 0xeb70, 0xeb6c, 0xeb74],
    };
    let (eax, _) = camera_fold(g, img, f, &o);
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(w(0xac), 1);
    g.write::<u32>(w(0xb6), u32::MAX);
    g.write::<u32>(w(0xc0), u32::MAX);
    eax
}

/// `camera_target_b` (205 bytes, 5 overlays).
pub const CAMERA_TARGET_B: Sig = Sig {
    len: 205,
    hash: 0xce5a_1272_728c_07bf,
    calls: &[],
    imports: &[],
};

/// `magic_0565` 0x10006d00 (internal, no arguments): [`camera_target_b`]
/// with the `magic_0446` block layout: the vector at `+0x2f80`, the angle
/// into `+0xebf0` and `+0xeca0`, `+0xec9c`, `+0xeca4` cleared; then
/// `G_READY` 1 and `G_A`/`G_B` -1. Returns slot 333's `eax`.
pub fn camera_target_c<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let o = CameraOps {
        fx: 0x08,
        t262: 0x1b,
        model: 0x22,
        t333: 0x2f,
        model2: 0x3c,
        k: [0x4f, 0x5e, 0x68, 0x72, 0x7f],
        vec: 0x2f80,
        out: [0xebf0, 0xeca0, 0xec9c, 0xeca4],
    };
    let (eax, _) = camera_fold(g, img, f, &o);
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(w(0xac), 1);
    g.write::<u32>(w(0xb6), u32::MAX);
    g.write::<u32>(w(0xc0), u32::MAX);
    eax
}

/// `camera_target_c` (205 bytes, 5 overlays).
pub const CAMERA_TARGET_C: Sig = Sig {
    len: 205,
    hash: 0x35ff_ab2a_e2f8_6da4,
    calls: &[],
    imports: &[],
};

/// `magic_0250` 0x10002300 (internal, no arguments): registers `N x N`
/// part handles (`N` = `G_N`, read again after every inner step): for
/// record `k` (0x68 bytes from `*G_TABLE`) and index `i`, slot 941 maps
/// the record's `+0x0`, slot 772 turns it and `i` into a 64-bit handle
/// and slot 779 registers it with `**G_LOCK` 1 around the call. Returns
/// the last `N` read.
pub fn register_grid<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut n = g.read::<i32>(w(0x05));
    let mut rec = g.read::<u32>(w(0x0c));
    let mut k = 0i32;
    while k < n {
        let mut i = 0i32;
        while i < n {
            let t = slot_target(g, w(0x31), 941);
            let x = g.read::<u32>(rec);
            let r = g.call(t, 0, &[x]).eax;
            let t = slot_target(g, w(0x45), 772);
            let h = g.call(t, 0, &[r, i.cast_unsigned()]);
            let lock = g.read::<u32>(w(0x53));
            let saved = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            let t = slot_target(g, w(0x5c), 779);
            g.call(t, 0, &[h.eax, h.edx, r, i.cast_unsigned()]);
            let lock = g.read::<u32>(w(0x6f));
            g.write::<u32>(lock, saved);
            n = g.read::<i32>(w(0x77));
            i += 1;
        }
        k += 1;
        rec = rec.wrapping_add(0x68);
    }
    n.cast_unsigned()
}

/// `register_grid` (151 bytes, 5 overlays).
pub const REGISTER_GRID: Sig = Sig {
    len: 151,
    hash: 0x1399_70f3_49a1_305f,
    calls: &[],
    imports: &[],
};

/// `magic_0303` 0x10002cb0 (internal, `edx` = a vector): outside a battle
/// (`*G_BATTLE` 0) a party member caller (`*G_ME` below 0x20) gets slot
/// 255 `(me, vector)`; anyone else's object `*G_ME_OBJ` is placed: slot
/// 33 `(obj, x, y, z)`, or slot 32 `(obj, x, z)` when slot 11 says it is
/// grounded. Returns the last call's `eax`.
pub fn place_me<G: Game>(g: &mut G, img: &Image, f: u32, v: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x02)) == 0 {
        let me = g.read::<u32>(w(0x0d));
        if me.cast_signed() < 0x20 {
            let t = slot_target(g, w(0x19), 255);
            return g.call(t, 0, &[me, v]).eax;
        }
    }
    let t = slot_target(g, w(0x2b), 11);
    let o = g.read::<u32>(w(0x32));
    let grounded = g.call(t, 0, &[o]).eax != 0;
    let z = g.read::<u32>(v + 8);
    if grounded {
        let t = slot_target(g, w(0x47), 32);
        let x = g.read::<u32>(v);
        g.call(t, 0, &[o, x, z]).eax
    } else {
        let t = slot_target(g, w(0x47), 33);
        let (x, y) = (g.read::<u32>(v), g.read::<u32>(v + 4));
        g.call(t, 0, &[o, x, y, z]).eax
    }
}

/// `place_me` (154 bytes, 5 overlays).
pub const PLACE_ME: Sig = Sig {
    len: 154,
    hash: 0xe0b5_63f6_5f9a_6e94,
    calls: &[],
    imports: &[],
};

/// `magic_0644` 0x1000b2b0 (a callback, cdecl `(part)`): one step of the
/// DLL's fade table: slot 843 sets the part's matrix to entry `G_STEP`
/// (64 bytes each), slot 833 gives part 0 of `*G_OBJ` (slot 219) the
/// table's float for the step; the step advances, and at `G_END` or past
/// 0x14 the part is reset (slot 847, part 0 alpha 1.0 through slot 833)
/// and the step and end go back to 0 (each call's words left below until
/// the original pops them). Returns the step, or slot 833's `eax` after a
/// reset.
pub fn fade_table<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let a = fr.arg(g, 0);
    let i = g.read::<u32>(w(0x04));
    let m = (i << 6).wrapping_add(w(0x0c));
    let t = slot_target(g, w(0x16), 843);
    g.call(t, 0, &[a, m]);
    let t = slot_target(g, w(0x24), 219);
    let o = g.read::<u32>(w(0x2c));
    let p = g.call(t, 0, &[o, 0, a, m]).eax;
    let i = g.read::<u32>(w(0x3a));
    let x = g.read::<u32>(w(0x46).wrapping_add(4 * i));
    let t = slot_target(g, w(0x51), 833);
    g.call(t, 0, &[a, p, x]);
    let i = g.read::<u32>(w(0x5f)).wrapping_add(1);
    g.write::<u32>(w(0x68), i);
    if i.cast_signed() < g.read::<i32>(w(0x6e)) && i.cast_signed() < 0x15 {
        return Ret::eax(i);
    }
    let t = slot_target(g, w(0x7a), 847);
    g.call(t, 0, &[a, 0]);
    let t = slot_target(g, w(0x8a), 219);
    let o = g.read::<u32>(w(0x92));
    let p = g.call(t, 0, &[o, 0, a, 0]).eax;
    let t = slot_target(g, w(0xaa), 833);
    let r = g.call(t, 0, &[a, p, 0x3f80_0000]).eax;
    g.write::<u32>(w(0xbc), 0);
    g.write::<u32>(w(0xc6), 0);
    Ret::eax(r)
}

/// `fade_table` (209 bytes, 5 overlays).
pub const FADE_TABLE: Sig = Sig {
    len: 209,
    hash: 0xd372_87ca_a280_291b,
    calls: &[],
    imports: &[],
};

/// `magic_0644` 0x1000a9d0 (internal, no arguments): for each of the 4
/// kinds, slot 645 lists the effect's entries of that kind (a byte list,
/// its length and a word through locals at `[ebp - 0xc]`/`[ebp - 0x14]`);
/// a kind with a word fills the next of the effect's records (0x28 bytes
/// from `+0xeac4`): the word, the length, and per entry the index of the
/// actor in the DLL's actor list (`G_COUNT` entries) with that battle
/// number (left as it was when none has it). The record count goes to
/// `+0xeb68`. Returns it.
pub fn kind_records<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x07));
    let mut rec = fx + 0xeac4;
    let n = g.read::<i32>(w(0x18));
    let ebp = esp.wrapping_sub(4);
    let (len, word) = (ebp.wrapping_sub(0xc), ebp.wrapping_sub(0x14));
    let mut count = 0u32;
    for k in 0..4 {
        let t = slot_target(g, w(0x3a), 645);
        let h = g.read::<u32>(w(0x40));
        let list = g.call(t, 0, &[h, len, k, word]).eax;
        let c = g.read::<u32>(word);
        if c == 0 {
            continue;
        }
        g.write::<u32>(rec, c);
        let l = g.read::<u32>(len);
        g.write::<u32>(rec + 4, l);
        let mut j = 0i32;
        while j < g.read::<i32>(len) {
            let b = u32::from(g.read::<u8>(list.wrapping_add(j.cast_unsigned())));
            let actors = g.read::<u32>(w(0x8b));
            if let Some(i) = (0..n)
                .find(|&i| u32::from(g.read::<u8>(actors.wrapping_add(i.cast_unsigned()))) == b)
            {
                g.write::<u8>(rec + 8 + j.cast_unsigned(), i.cast_unsigned() as u8);
            }
            j += 1;
        }
        count += 1;
        rec += 0x28;
    }
    g.write::<u32>(fx + 0xeb68, count);
    count
}

/// `kind_records` (230 bytes, 5 overlays).
pub const KIND_RECORDS: Sig = Sig {
    len: 230,
    hash: 0x26c3_0e63_c8c2_5080,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x100020c0 (internal, `ecx` = an object): the object's
/// jitter: each of `+0x90..+0x98` gets slot 314 (a random spread of
/// `+0x16c`, ST0) added `+0x174` times, and each of `+0xa0..+0xa8` the
/// spread of `+0x170` `+0x178` times. Returns `+0x178` as last read.
pub fn jitter<G: Game>(g: &mut G, img: &Image, f: u32, o: u32) -> u32 {
    let mut eax = 0;
    for (base, spread, count, op) in [(0x90, 0x16c, 0x174, 0x31), (0xa0, 0x170, 0x178, 0xa1)] {
        eax = g.read::<u32>(o + count);
        for c in 0..3 {
            let mut i = 0i32;
            while i < eax.cast_signed() {
                let t = slot_target(g, img.word(f + op), 314);
                let x = g.read::<u32>(o + spread);
                let r = f32::from_bits(crate::sa_summon::st0_f32(g.call(t, 0, &[x])));
                let v = add(f32_at(g, o + base + 4 * c), r);
                g.write::<u32>(o + base + 4 * c, v.to_bits());
                eax = g.read::<u32>(o + count);
                i += 1;
            }
        }
    }
    eax
}

/// `jitter` (236 bytes, 5 overlays).
pub const JITTER: Sig = Sig {
    len: 236,
    hash: 0xa707_7a21_b5b0_9ac1,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x10009ed0 (internal, no arguments): slot 331 on the
/// DLL's block. Returns its `eax`.
pub fn block_331<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let t = slot_target(g, img.word(f + 0x01), 331);
    g.call(t, 0, &[img.word(f + 0x06)]).eax
}

/// `block_331` (20 bytes, 4 overlays).
pub const BLOCK_331: Sig = Sig {
    len: 20,
    hash: 0xd0a0_4957_64d5_fc57,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x10009d90 (internal, `edx` = a value): slot 306 `(7,
/// value, *G_X)`. Returns its `eax`.
pub fn slot306<G: Game>(g: &mut G, img: &Image, f: u32, x: u32) -> u32 {
    let t = slot_target(g, img.word(f + 0x07), 306);
    let v = g.read::<u32>(img.word(f + 0x02));
    g.call(t, 0, &[7, x, v]).eax
}

/// `slot306` (26 bytes, 4 overlays).
pub const SLOT306: Sig = Sig {
    len: 26,
    hash: 0x698e_cf42_c5eb_4e1b,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x10002640 (internal, `ecx` = a record, `edx` = a
/// value): a record whose `+0x0` is not 0xff calls slot 296 `(id,
/// value)`. Returns its `eax`, or 0xff.
pub fn record_296<G: Game>(g: &mut G, img: &Image, f: u32, p: u32, x: u32) -> u32 {
    let id = g.read::<u32>(p);
    if id == 0xff {
        return id;
    }
    let t = slot_target(g, img.word(f + 0x0c), 296);
    g.call(t, 0, &[id, x]).eax
}

/// `record_296` (28 bytes, 4 overlays).
pub const RECORD_296: Sig = Sig {
    len: 28,
    hash: 0xbd46_7ed6_7f05_c10c,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x10003d50 (internal, no arguments): the first word of
/// each of the DLL's 0x100 records (0x4c bytes) cleared. Returns the
/// address past the last record.
pub fn records_clear(g: &mut impl crate::Mem, img: &Image, f: u32) -> u32 {
    let mut p = img.word(f + 0x06);
    for _ in 0..0x100 {
        g.write::<u32>(p, 0);
        p = p.wrapping_add(0x4c);
    }
    p
}

/// `records_clear` (29 bytes, 4 overlays).
pub const RECORDS_CLEAR: Sig = Sig {
    len: 29,
    hash: 0xf824_396b_e1a7_da52,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x10002da0 (internal, no arguments): the first of the
/// DLL's records (0xa8 bytes, from `A` to `B`) whose `+0x80` is 0, or 0
/// when none is.
pub fn record_free(g: &impl crate::Mem, img: &Image, f: u32) -> u32 {
    let end = img.word(f + 0x14);
    let mut p = img.word(f + 0x01);
    loop {
        if g.read::<u32>(p + 0x80) == 0 {
            return p;
        }
        p = p.wrapping_add(0xa8);
        if p >= end {
            return 0;
        }
    }
}

/// `record_free` (29 bytes, 4 overlays).
pub const RECORD_FREE: Sig = Sig {
    len: 29,
    hash: 0xdd46_561a_3da5_cced,
    calls: &[],
    imports: &[],
};

/// `magic_0098` 0x100020f0 (a table entry, no arguments): slot 239
/// `(*G_A, *G_B, 0)`. Returns 1.
pub fn slot239_pair<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x01), 239);
    let (a, b) = (g.read::<u32>(w(0x15)), g.read::<u32>(w(0x09)));
    g.call(t, 0, &[a, b, 0]);
    Ret::eax(1)
}

/// `slot239_pair` (36 bytes, 4 overlays).
pub const SLOT239_PAIR: Sig = Sig {
    len: 36,
    hash: 0x6e9b_423c_d7a5_9036,
    calls: &[],
    imports: &[],
};

/// `magic_0351` 0x100015c0 (a table entry, no arguments): slot 399
/// `(*G_H, 0)`, then `G_DONE` 0. Returns 1.
pub fn slot399_done<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x01), 399);
    let h = g.read::<u32>(w(0x09));
    g.call(t, 0, &[h, 0]);
    g.write::<u32>(w(0x1a), 0);
    Ret::eax(1)
}

/// `slot399_done` (40 bytes, 4 overlays).
pub const SLOT399_DONE: Sig = Sig {
    len: 40,
    hash: 0x5a00_c02a_0a0c_6997,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x10001b40 (internal, `ecx` = an object, `xmm1`/`xmm2`
/// two spreads): `+0x16c`/`+0x170` the spreads, `+0x174`/`+0x178` 1 and
/// byte `+0x188` 1. Returns `eax` as given.
pub fn spread_set(g: &mut impl crate::Mem, fr: &Frame) -> u32 {
    let o = fr.ecx;
    g.write::<u32>(o + 0x16c, fr.xmm[1] as u32);
    g.write::<u32>(o + 0x170, fr.xmm[2] as u32);
    g.write::<u32>(o + 0x174, 1);
    g.write::<u32>(o + 0x178, 1);
    g.write::<u8>(o + 0x188, 1);
    fr.eax
}

/// `spread_set` (44 bytes, 4 overlays).
pub const SPREAD_SET: Sig = Sig {
    len: 44,
    hash: 0x0151_f92d_2ea0_abeb,
    calls: &[],
    imports: &[],
};

/// `magic_0356` 0x10001000 (a callback, cdecl `(a)`): `G_FLAG` 0, slot
/// 450 `(a, the DLL's first callback)` and slot 449 `(a, the second)`.
/// Returns slot 449's `eax`.
pub fn hook_pair<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let a = fr.arg(g, 0);
    let t = slot_target(g, w(0x04), 450);
    g.write::<u32>(w(0x18), 0);
    g.call(t, 0, &[a, w(0x09)]);
    let t = slot_target(g, w(0x23), 449);
    Ret::eax(g.call(t, 0, &[a, w(0x28), a, w(0x09)]).eax)
}

/// `hook_pair` (60 bytes, 4 overlays).
pub const HOOK_PAIR: Sig = Sig {
    len: 60,
    hash: 0x5498_7758_7b60_3660,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x10002490 (internal, no arguments): `*G_N` times, slot
/// 309 on `*G_X` unless the sound id `*G_ID` is 0xff. Returns the last
/// call's `eax` (`eax` as given when none ran).
pub fn repeat_309<G: Game>(g: &mut G, img: &Image, f: u32, eax_in: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut eax = eax_in;
    let n = g.read::<u32>(w(0x07));
    for _ in 0..n {
        if g.read::<u32>(w(0x13)) != 0xff {
            let t = slot_target(g, w(0x1e), 309);
            let x = g.read::<u32>(w(0x24));
            eax = g.call(t, 0, &[x]).eax;
        }
    }
    eax
}

/// `repeat_309` (60 bytes, 4 overlays).
pub const REPEAT_309: Sig = Sig {
    len: 60,
    hash: 0xba8a_3b29_94e5_7eff,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x10003940 (a stored callback, cdecl `(obj, anim)`): the
/// object's sub-frame counter `u16 +0x48` advances by 0x200; at the
/// animation's frame length (`u16 anim+0x2`, compared as `i16`) it wraps
/// and the frame `u16 +0x4a` advances; at the animation's `+0x28`
/// record's frame count (`i16 +0x6`) the record `+0x28` is cleared.
/// Returns the length, or the record as it was.
pub fn anim_step<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let o = fr.arg(g, 0);
    let anim = fr.arg(g, 1);
    let c = g.read::<u16>(o + 0x48).wrapping_add(0x200);
    g.write::<u16>(o + 0x48, c);
    let len = g.read::<u16>(anim + 2);
    if c.cast_signed() < len.cast_signed() {
        return Ret {
            edx: o,
            ..Ret::eax(u32::from(len))
        };
    }
    let fno = g.read::<u16>(o + 0x4a).wrapping_add(1);
    g.write::<u16>(o + 0x4a, fno);
    let rec = g.read::<u32>(o + 0x28);
    g.write::<u16>(o + 0x48, c.wrapping_sub(len));
    let fno = g.read::<u16>(o + 0x4a);
    if fno.cast_signed() >= g.read::<i16>(rec + 6) {
        g.write::<u32>(o + 0x28, 0);
    }
    Ret {
        edx: o,
        ..Ret::eax(rec)
    }
}

/// `anim_step` (63 bytes, 4 overlays).
pub const ANIM_STEP: Sig = Sig {
    len: 63,
    hash: 0x50ca_d7e4_f41b_c783,
    calls: &[],
    imports: &[],
};

/// `magic_0155` 0x10008cc0 (internal, `ecx` = a value; an aligned frame):
/// slot 771 turns the value into a 64-bit handle and slot 779 registers
/// it with `**G_LOCK` 1 around the call. Returns `*G_LOCK`.
pub fn register_one<G: Game>(g: &mut G, img: &Image, f: u32, x: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x08), 771);
    let h = g.call(t, 0, &[x]);
    let lock = g.read::<u32>(w(0x18));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x24), 779);
    g.call(t, 0, &[h.eax, h.edx]);
    let lock = g.read::<u32>(w(0x37));
    g.write::<u32>(lock, saved);
    lock
}

/// `register_one` (69 bytes, 4 overlays).
pub const REGISTER_ONE: Sig = Sig {
    len: 69,
    hash: 0x0f08_0af3_347c_149b,
    calls: &[],
    imports: &[],
};

/// `magic_0262` 0x10006aa0 (internal, `ecx` = an object): unless flag
/// bit 0 (`+0x90`) is set, slot 211 `(model +0x54, 0)`, flag bit 4
/// cleared, and slot 255 puts the object `+0x50` at its node's (`+0x4c`)
/// `+0xcc`. Returns slot 255's `eax`, or `eax` as given.
pub fn object_to_node<G: Game>(g: &mut G, img: &Image, f: u32, o: u32, eax: u32) -> u32 {
    if g.read::<u8>(o + 0x90) & 1 != 0 {
        return eax;
    }
    let t = slot_target(g, img.word(f + 0x0d), 211);
    let m = g.read::<u32>(o + 0x54);
    g.call(t, 0, &[m, 0]);
    let v = g.read::<u16>(o + 0x90);
    g.write::<u16>(o + 0x90, v & 0xffef);
    let p = g.read::<u32>(o + 0x4c).wrapping_add(0xcc);
    let t = slot_target(g, img.word(f + 0x34), 255);
    let x = g.read::<u32>(o + 0x50);
    g.call(t, 0, &[x, p, m, 0]).eax
}

/// `object_to_node` (72 bytes, 4 overlays).
pub const OBJECT_TO_NODE: Sig = Sig {
    len: 72,
    hash: 0xadab_9d08_d591_5a24,
    calls: &[],
    imports: &[],
};

/// `magic_0070` 0x100099a0 (internal, `ecx` = a node, `xmm1` a rate):
/// slot 234 `(+0x3c, +0x2c, +0x1c)`, then flag bit 0 of `u16 +0x58`,
/// `+0x54` 1, `+0x50` the rate and `+0x4c` 0. Returns slot 234's `eax`.
pub fn node_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let n = fr.ecx;
    let t = slot_target(g, img.word(fr.at + 0x19), 234);
    let r = g.call(t, 0, &[n + 0x3c, n + 0x2c, n + 0x1c]).eax;
    let v = g.read::<u16>(n + 0x58);
    g.write::<u16>(n + 0x58, v | 1);
    g.write::<u32>(n + 0x54, 1);
    g.write::<u32>(n + 0x50, fr.xmm[1] as u32);
    g.write::<u32>(n + 0x4c, 0);
    r
}

/// `node_start` (74 bytes, 4 overlays).
pub const NODE_START: Sig = Sig {
    len: 74,
    hash: 0x0956_e5f0_a5a2_51ef,
    calls: &[],
    imports: &[],
};

/// Slot 772 turns `(x, i)` into a 64-bit handle and slot 779 registers
/// it with `**G_LOCK` 1 around the call (the operands at `o772`, `lock`
/// (read twice: `lock_a` before, `lock_b` after) and `o779`). Returns
/// `*G_LOCK` as read after.
fn register_handle<G: Game>(g: &mut G, img: &Image, f: u32, ops: [u32; 4], x: u32, i: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let [o772, lock_a, o779, lock_b] = ops;
    let t = slot_target(g, w(o772), 772);
    let h = g.call(t, 0, &[x, i]);
    let lock = g.read::<u32>(w(lock_a));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(o779), 779);
    g.call(t, 0, &[h.eax, h.edx, x, i]);
    let lock = g.read::<u32>(w(lock_b));
    g.write::<u32>(lock, saved);
    lock
}

/// `magic_0284` 0x10002a90 (internal, `ecx` = a value): its 8 part
/// handles registered ([`register_handle`] with indices 0..8). Returns
/// `*G_LOCK`.
pub fn register_8<G: Game>(g: &mut G, img: &Image, f: u32, x: u32) -> u32 {
    let mut r = 0;
    for i in 0..8 {
        r = register_handle(g, img, f, [0x11, 0x21, 0x2a, 0x3d], x, i);
    }
    r
}

/// `register_8` (80 bytes, 4 overlays).
pub const REGISTER_8: Sig = Sig {
    len: 80,
    hash: 0x2352_d7c2_11c6_dd7c,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x10003e90 (internal, no arguments): the handle (index
/// 0) of each of the DLL's records (0x4c bytes, from `A` to `B`)
/// registered. Returns `*G_LOCK`.
pub fn register_records<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let end = img.word(f + 0x4c);
    let mut p = img.word(f + 0x04);
    loop {
        let r = register_handle(g, img, f, [0x11, 0x22, 0x2b, 0x3e], p, 0);
        p = p.wrapping_add(0x4c);
        if p.cast_signed() >= end.cast_signed() {
            return r;
        }
    }
}

/// `register_records` (86 bytes, 4 overlays).
pub const REGISTER_RECORDS: Sig = Sig {
    len: 86,
    hash: 0x3142_d0f6_1ef8_15f7,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x100034b0 (internal, no arguments): for each of the
/// DLL's records (0xa8 bytes) its handles 0..=count (`i32 +0xa4`, none
/// when negative) registered. Returns the last record that had any, or
/// `eax` as given.
pub fn register_counted<G: Game>(g: &mut G, img: &Image, f: u32, eax_in: u32) -> u32 {
    let end = img.word(f + 0x5d);
    let mut c = img.word(f + 0x04);
    let mut eax = eax_in;
    loop {
        let mut i = 0i32;
        if g.read::<i32>(c) >= 0 {
            loop {
                let base = c.wrapping_sub(0xa4);
                register_handle(g, img, f, [0x17, 0x25, 0x2e, 0x41], base, i.cast_unsigned());
                i += 1;
                eax = base;
                if i > g.read::<i32>(c) {
                    break;
                }
            }
        }
        c = c.wrapping_add(0xa8);
        if c.cast_signed() >= end.cast_signed() {
            return eax;
        }
    }
}

/// `register_counted` (103 bytes, 4 overlays).
pub const REGISTER_COUNTED: Sig = Sig {
    len: 103,
    hash: 0x4200_88cb_9306_6a8d,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x100028f0 (internal, `ecx` = records of 0xcc bytes,
/// `edx` = their count): each record's object `+0x0` is put at its
/// `+0x2c` (slot 255) and given the angle `i16 +0x14` times `K1 * K2`
/// (slot 170; slot 255's word left below). Returns slot 170's `eax`, or
/// `eax` as given for none.
pub fn place_records<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    recs: u32,
    n: u32,
    eax_in: u32,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut eax = eax_in;
    let mut p = recs;
    for _ in 0..n {
        let t = slot_target(g, w(0x15), 255);
        let o = g.read::<u32>(p);
        g.call(t, 0, &[o, p + 0x2c]);
        let a = crate::sa_misc::cvt(i32::from(g.read::<i16>(p + 0x14)));
        let x = crate::sb_sse::mul(
            crate::sb_sse::mul(a, f32_at(g, w(0x3d))),
            f32_at(g, w(0x48)),
        );
        let t = slot_target(g, w(0x2c), 170);
        let o = g.read::<u32>(p);
        eax = g.call(t, 0, &[o, x.to_bits()]).eax;
        p = p.wrapping_add(0xcc);
    }
    eax
}

/// `place_records` (100 bytes, 4 overlays).
pub const PLACE_RECORDS: Sig = Sig {
    len: 100,
    hash: 0x3a39_5fd0_dadb_975e,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x10002560 (internal, no arguments): slot 295 `(object,
/// 0)` for the first `*G_N` records of table A and every record of table
/// B (0xcc bytes each, the object at `+0x0`). Returns the last call's
/// `eax`.
pub fn records_295<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut p = w(0x0d);
    let mut i = 0i32;
    while i < g.read::<i32>(w(0x05)) {
        let t = slot_target(g, w(0x12), 295);
        let o = g.read::<u32>(p);
        g.call(t, 0, &[o, 0]);
        i += 1;
        p = p.wrapping_add(0xcc);
    }
    let end = w(0x5c);
    let mut p = w(0x36);
    loop {
        let t = slot_target(g, w(0x41), 295);
        let o = g.read::<u32>(p);
        let r = g.call(t, 0, &[o, 0]).eax;
        p = p.wrapping_add(0xcc);
        if p.cast_signed() >= end.cast_signed() {
            return r;
        }
    }
}

/// `records_295` (100 bytes, 4 overlays).
pub const RECORDS_295: Sig = Sig {
    len: 100,
    hash: 0x7e64_2a74_45ad_3e84,
    calls: &[],
    imports: &[],
};

/// `magic_0284` 0x10003510 (internal, no arguments): [`ring_take`] for a
/// ring of 0x68-byte entries (free when `+0x60` is 0): the entry is reset
/// (`+0x54` 0, colours `+0x58`/`+0x5c` 0x80808080, scale `+0x38` 1.0),
/// marked used and the rover moved past it. Returns it, or 0 when the
/// ring is full.
pub fn ring_take_68<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut i = g.read::<u32>(w(0x02));
    let n = g.read::<i32>(w(0x0a));
    if n <= 0 {
        return 0;
    }
    let base = g.read::<u32>(w(0x17));
    for _ in 0..n {
        let e = i.wrapping_mul(0x68).wrapping_add(base);
        i = i.wrapping_add(1);
        if i.cast_signed() >= n {
            i = 0;
        }
        if g.read::<u32>(e + 0x60) == 0 {
            g.write::<u32>(e + 0x54, 0);
            g.write::<u32>(e + 0x58, 0x8080_8080);
            g.write::<u32>(e + 0x5c, 0x8080_8080);
            g.write::<u32>(e + 0x38, 0x3f80_0000);
            g.write::<u32>(e + 0x60, 1);
            g.write::<u32>(w(0x62), i);
            return e;
        }
    }
    0
}

/// `ring_take_68` (104 bytes, 4 overlays).
pub const RING_TAKE_68: Sig = Sig {
    len: 104,
    hash: 0xff7f_a4bb_bfe0_f625,
    calls: &[],
    imports: &[],
};

/// `magic_0515` 0x10003ef0 (internal, `edx` = an angle): `G_B` 1, slot
/// 247 turns the DLL's matrix `M` by `-0x4000 - angle`, its last row takes
/// the four words of `A`, and slot 327 applies it to the vector `V` (the
/// earlier words left below). Returns slot 327's `eax`.
pub fn turn_matrix<G: Game>(g: &mut G, img: &Image, f: u32, angle: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let x = 0xffff_c000u32.wrapping_sub(angle);
    g.write::<u8>(w(0x0f), 1);
    let t = slot_target(g, w(0x09), 247);
    let m = w(0x1b);
    g.call(t, 0, &[m, x]);
    for (src, dst) in [(0x25, 0x32), (0x3a, 0x42), (0x4a, 0x52), (0x5a, 0x62)] {
        let v = g.read::<u32>(w(src));
        g.write::<u32>(w(dst), v);
    }
    let t = slot_target(g, w(0x2a), 327);
    g.call(t, 0, &[w(0x72), w(0x6d), m, x]).eax
}

/// `turn_matrix` (124 bytes, 4 overlays).
pub const TURN_MATRIX: Sig = Sig {
    len: 124,
    hash: 0x16c3_3e31_ca87_a91e,
    calls: &[],
    imports: &[],
};

/// `magic_0042` 0x10008470 (internal, no arguments): the DLL's static
/// node `A` set up under its static parent `P` (the handler table, pass
/// byte 0xff, an empty child list, appended to `P`'s children), with
/// `+0x70`/`+0x6c` two of the DLL's blocks, byte `+0x74` 0 and `+0x68` 2.
/// Returns `P`'s last child before it.
pub fn static_node<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let last = g.read::<u32>(w(0x01));
    g.write::<u32>(w(0x07), w(0x0b));
    g.write::<u16>(w(0x12), 0xff);
    for (dst, v) in [
        (0x1a, w(0x1e)),
        (0x24, w(0x28)),
        (0x2e, w(0x32)),
        (0x38, w(0x3c)),
        (0x41, last),
    ] {
        g.write::<u32>(w(dst), v);
    }
    if last == w(0x46) {
        g.write::<u32>(w(0x4e), w(0x52));
    } else {
        g.write::<u32>(last + 4, w(0x5b));
    }
    let c = g.read::<u16>(w(0x62));
    g.write::<u16>(w(0x62), c.wrapping_add(1));
    for (dst, v) in [(0x68, w(0x6c)), (0x72, w(0x76)), (0x7c, w(0x80))] {
        g.write::<u32>(w(dst), v);
    }
    g.write::<u8>(w(0x86), 0);
    g.write::<u32>(w(0x8d), 2);
    last
}

/// `static_node` (150 bytes, 4 overlays).
pub const STATIC_NODE: Sig = Sig {
    len: 150,
    hash: 0x82d4_dbc5_3c59_69e5,
    calls: &[],
    imports: &[],
};
