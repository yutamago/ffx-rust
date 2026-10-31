//! Families of two or three copies from the own work list (round 4,
//! second pass): part registration loops, flag-driven slot pairs, rcef
//! chains and pool records. Operands are read from each copy's code
//! (`w(off)` = the word at `+off`).

use crate::o4_parts::disp8;
use crate::o4_small::append_node;
use crate::runtime::slot_target;
use crate::sa_pool::take;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The slot of a `mov eax, [eax + disp32]` with the `disp32` at `at`.
fn slot_at(img: &Image, at: u32) -> u32 {
    img.word(at) / 4
}

/// Registers `this`'s parts (`magic_0171` 0x10002500, 80 bytes, internal:
/// `ecx` = `this`; 3 copies): for `i` in `0..N` (`N` the `imm8` at
/// `+0x49`, at least one pass), slot 772 `(this, i)` gives a 64-bit
/// handle, and slot 779 `(lo, hi, this, i)` registers it with the game
/// lock `**L` (`L` at `+0x21`, again at `+0x3d`) set to 1 around the
/// call. Returns `L`'s pointer as last read.
pub fn register_loop<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let n = disp8(img, f + 0x49).cast_signed();
    let mut i = 0u32;
    loop {
        let s772 = slot_target(g, w(0x11), slot_at(img, f + 0x18));
        let h = g.call(s772, this, &[this, i]);
        let lock = g.read::<u32>(w(0x21));
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        let s779 = slot_target(g, w(0x2a), slot_at(img, f + 0x36));
        g.call(s779, this, &[h.eax, h.edx, this, i]);
        let lock = g.read::<u32>(w(0x3d));
        g.write::<u32>(lock, saved);
        i += 1;
        if i.cast_signed() >= n {
            return lock;
        }
    }
}

/// No arguments (`magic_0252` 0x10001340, 96 bytes; 3 copies): while the
/// word `A` (`+0x02`) is 0 and the count `B` (`+0x0a`) is below 0x20,
/// slot 541 (`+0x25`) with `(B, 0, K)`; else slot 38 (`+0x34`) with
/// `(*C, 0, K)` (`C` at `+0x42`; `K` the `push imm8` at `+0x14`/`+0x31`);
/// then slot 525 (`+0x52`) with `(*C)` (`+0x58`). Returns its `eax`.
pub fn slot_by_flag<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let b = g.read::<u32>(w(0x0a));
    if g.read::<u32>(w(0x02)) == 0 && b.cast_signed() < 0x20 {
        let s = slot_target(g, w(0x1f), slot_at(img, f + 0x25));
        g.call(s, fr.ecx, &[b, 0, disp8(img, f + 0x14)]);
    } else {
        let c = g.read::<u32>(w(0x42));
        let s = slot_target(g, w(0x2c), slot_at(img, f + 0x34));
        g.call(s, fr.ecx, &[c, 0, disp8(img, f + 0x31)]);
    }
    let c = g.read::<u32>(w(0x58));
    let s = slot_target(g, w(0x49), slot_at(img, f + 0x52));
    Ret::eax(g.call(s, fr.ecx, &[c]).eax)
}

/// `(obj)` (`magic_0416` 0x10001d20, 95 bytes; 3 copies): slot 447
/// (`+0x1c`) with `(V, K)` (`V` the word at `*G + D`, `G` at `+0x04`, `D`
/// at `+0x0c`, `K` the `push imm32` at `+0x16`), `rcefTaskSetSeq(obj,
/// cb)` (`+0x2d`, `cb` at `+0x32`) and `rcefTaskCallSeq(obj, cb2)`
/// (`+0x45`, `+0x3f`), each with the earlier words after its own; then
/// `V` in the object's list `obj + 0x2c + 16 * i` (`i` the word at `obj +
/// 0xac`). Returns `2 * i`.
pub fn seq_keep_447_32<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0c)));
    let k = w(0x16);
    let s = slot_target(g, w(0x11), slot_at(img, f + 0x1c));
    g.call(s, fr.ecx, &[v, k]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x24), slot_at(img, f + 0x2d));
    g.call(s, fr.ecx, &[obj, w(0x32), v, k]);
    let s = slot_target(g, w(0x3a), slot_at(img, f + 0x45));
    g.call(s, fr.ecx, &[obj, w(0x3f), obj, w(0x32), v, k]);
    // add eax, eax; mov [esi + eax * 8 + 0x2c], edi
    let i = g.read::<u32>(obj.wrapping_add(0xac));
    let twice = i.wrapping_add(i);
    g.write::<u32>(
        obj.wrapping_add(twice.wrapping_mul(8)).wrapping_add(0x2c),
        v,
    );
    Ret::eax(twice)
}

/// A model's flag step (`magic_0202` 0x10006d90, 103 bytes, internal:
/// `ecx` = `this`; 3 copies): with bit 0 of the byte `this + 0x90`, slot
/// 30 (`+0x16`) with `(this[0x54])` and slot 36 (`+0x28`) with
/// `(this[0x54], K)` (read again; the first call's word after them), and
/// the halfword `+0x90` gets 0x200; without it, slot 228 (`+0x41`) with
/// `(this[0x50], K)`. Both paths then give the halfword 0x100 and
/// `this + 0x60` = `K` (`+0x61`; `K` the immediates at `+0x1f`, `+0x46`).
/// Returns 0x100.
pub fn flag_slots<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let table = w(0x01);
    let set = |g: &mut G, bit: u16| {
        let v = g.read::<u16>(this + 0x90);
        g.write::<u16>(this + 0x90, v | bit);
    };
    if g.read::<u8>(this + 0x90) & 1 != 0 {
        let a = g.read::<u32>(this + 0x54);
        let s = slot_target(g, table, disp8(img, f + 0x16) / 4);
        g.call(s, this, &[a]);
        let b = g.read::<u32>(this + 0x54);
        let s = slot_target(g, w(0x1a), slot_at(img, f + 0x28));
        g.call(s, this, &[b, w(0x1f), a]);
        set(g, 0x200);
    } else {
        let a = g.read::<u32>(this + 0x50);
        let s = slot_target(g, table, slot_at(img, f + 0x41));
        g.call(s, this, &[a, w(0x46)]);
    }
    // jmp: the first path joins the second's `or`
    set(g, 0x100);
    g.write::<u32>(this + 0x60, w(0x61));
    0x100
}

/// No arguments (`magic_0397` 0x100063f0, 90 bytes; 2 copies): `p` =
/// slot 288 (`+0x14`) with `(9, &n)` (`n` a local at `ebp - 4`, the
/// immediate at `+0x11`); for each of the `n` bytes `c` of `p` other than
/// the byte `B` (`+0x1f`), the slot at `+0x42` (635 or 636) with `(c)`.
/// Returns the last call's `eax` (`p` when none ran).
pub fn byte_list_slot<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    // ebp = esp - 4; n at ebp - 4
    let n_at = fr.esp.wrapping_sub(8);
    let s = slot_target(g, w(0x0c), slot_at(img, f + 0x14));
    let p = g.call(s, fr.ecx, &[disp8(img, f + 0x11), n_at]).eax;
    let skip = g.read::<u8>(w(0x1f));
    let mut eax = p;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(n_at).cast_signed() {
        let c = g.read::<u8>(p.wrapping_add(i));
        if c != skip {
            let s = slot_target(g, w(0x3c), slot_at(img, f + 0x42));
            eax = g.call(s, fr.ecx, &[u32::from(c)]).eax;
        }
        i += 1;
    }
    Ret::eax(eax)
}

/// `(obj)` (`magic_0313` 0x10001920, 89 bytes; 2 copies): `r` =
/// `rcefMainCreateTask(*G, cb)` (`+0x19`, `G` at `+0x0f`, `cb` at
/// `+0x09`); `*H` (`+0x23`) into `r`'s list `r + 0x2c + 16 * r[0xac]`;
/// `rcefTaskSetSeq(obj, cb2)` (`+0x3c`, `+0x2a`) and `rcefTaskWait(obj,
/// T)` (`+0x4e`, `T` at `+0x48`), each with the earlier words after its
/// own. Returns the last slot's `eax`.
pub fn task_list_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let v = g.read::<u32>(w(0x0f));
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x15));
    let r = g.call(s, fr.ecx, &[v, w(0x09)]).eax;
    let i = g.read::<u32>(r.wrapping_add(0xac));
    let h = g.read::<u32>(w(0x23));
    let obj = fr.arg(g, 0);
    g.write::<u32>(
        r.wrapping_add(i.wrapping_add(i).wrapping_mul(8))
            .wrapping_add(0x2c),
        h,
    );
    let s = slot_target(g, w(0x36), slot_at(img, f + 0x3c));
    g.call(s, fr.ecx, &[obj, w(0x2a), v, w(0x09)]);
    let t = disp8(img, f + 0x48);
    let s = slot_target(g, w(0x43), slot_at(img, f + 0x4e));
    Ret::eax(g.call(s, fr.ecx, &[obj, t, obj, w(0x2a), v, w(0x09)]).eax)
}

/// No arguments (`magic_0307` 0x10002990, 86 bytes; 2 copies): eight
/// words of `*G` (`+0x01`) set (`mov dword [eax + disp32], imm32` from
/// `+0x05`, 10 bytes each). Returns `*G`.
pub fn ptr_stores8<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = g.read::<u32>(img.word(f + 0x01));
    for k in 0..8 {
        let insn = f + 0x05 + 10 * k;
        g.write::<u32>(p.wrapping_add(img.word(insn + 2)), img.word(insn + 6));
    }
    Ret::eax(p)
}

/// `(obj)` (`magic_0382` 0x100010a0, 86 bytes; 2 copies): the slot at
/// `+0x0c` (444) with the `push imm8` at `+0x09`,
/// `rcefMainCreateTask(*G, cb)` (`+0x24`, `G` at `+0x1e`, `cb` at
/// `+0x18`), `rcefTaskSetSeq(obj, cb2)` (`+0x39`, `+0x30`) and
/// `rcefTaskWait(obj, T)` (`+0x4b`, `T` at `+0x45`), each with the earlier
/// words after its own. Returns the last slot's `eax`.
pub fn slot_task_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = disp8(img, f + 0x09);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x0c));
    g.call(s, fr.ecx, &[k]);
    let v = g.read::<u32>(w(0x1e));
    let s = slot_target(g, w(0x13), slot_at(img, f + 0x24));
    g.call(s, fr.ecx, &[v, w(0x18), k]);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x2b), slot_at(img, f + 0x39));
    g.call(s, fr.ecx, &[obj, w(0x30), v, w(0x18), k]);
    let t = disp8(img, f + 0x45);
    let s = slot_target(g, w(0x40), slot_at(img, f + 0x4b));
    let r = g.call(s, fr.ecx, &[obj, t, obj, w(0x30), v, w(0x18), k]);
    Ret::eax(r.eax)
}

/// No arguments (`magic_0353` 0x10002390, 85 bytes; 2 copies): a word of
/// `*G` set (`mov dword [eax + disp32], imm32` at `+0x0a`); a pool node
/// taken ([`take`], `+0x15`, the owner the operand at `+0x06`), its byte
/// `+0x18` 1 and `+0x4` 0, appended to the list ([`append_node`]: head
/// `+0x1b`/`+0x48`, tail `+0x2f`, `+0x39`, `+0x4d`). Returns the node.
pub fn store_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x01));
    g.write::<u32>(p.wrapping_add(w(0x0c)), w(0x10));
    let node = take(g, img, img.rel32_target(f + 0x15), w(0x06));
    let empty = g.read::<u32>(w(0x1b)) == 0;
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    append_node(g, node, empty, (w(0x2f), w(0x39)), (w(0x48), w(0x4d)));
    Ret::eax(node)
}

/// `(obj)` (`magic_0356` 0x10001840, 40 bytes; 2 copies): the slot at
/// `+0x17` (446 or 447) with `(V, K)` (`V` the word at `*G + D`, `G` at
/// `+0x04`, `D` at `+0x0c`, `K` the `push imm8` at `+0x09`), then `obj`
/// gets bit 2. Returns `obj`.
pub fn field_slot_flag8<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0c)));
    let s = slot_target(g, w(0x11), slot_at(img, f + 0x17));
    g.call(s, fr.ecx, &[v, disp8(img, f + 0x09)]);
    let obj = fr.arg(g, 0);
    let x = g.read::<u32>(obj);
    g.write::<u32>(obj, x | 4);
    Ret::eax(obj)
}

/// No arguments (`magic_0165` 0x100033a0, 100 bytes; 2 copies): for each
/// actor `i` in `0..N` (`N` at `+0x05`, read again at `+0x5c`): its
/// character `TABLE[B[i]]` (`B` the byte list `*L`, `L` at `+0x1d`;
/// `TABLE` at `+0x28`; the effect's own `*E` for index -1, `E` at
/// `+0x16`), and slot 256 (`+0x50`) with `(chr, x, y, z, K)` (the floats
/// at `+0x44`, `+0x3d`, `+0x35`, `K` the `push imm8` at `+0x2d`). Returns
/// the last call's `eax` (`eax` as given when `N` is not above 0).
pub fn scale_actors<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut eax = fr.eax;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(w(0x05)).cast_signed() {
        // cmp esi, -1: never true here (i counts up from 0), kept as the
        // original has it
        let chr = if i == u32::MAX {
            g.read::<u32>(w(0x16))
        } else {
            let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x1d)).wrapping_add(i)));
            g.read::<u32>(w(0x28).wrapping_add(4 * b))
        };
        let s = slot_target(g, w(0x4a), slot_at(img, f + 0x50));
        let args = [chr, w(0x44), w(0x3d), w(0x35), disp8(img, f + 0x2d)];
        eax = g.call(s, fr.ecx, &args).eax;
        i += 1;
    }
    Ret::eax(eax)
}

/// [`register_loop`] in `magic_0171` (0x10002500, 80 bytes).
pub const REGISTER_LOOP_0171_2500: Sig = Sig {
    len: 80,
    hash: 0xbe23_b9c9_1517_15cc,
    calls: &[],
    imports: &[],
};

/// [`register_loop`] in `magic_0180` (0x10001870, 80 bytes).
pub const REGISTER_LOOP_0180_1870: Sig = Sig {
    len: 80,
    hash: 0xb136_52fc_d662_6954,
    calls: &[],
    imports: &[],
};

/// [`register_loop`] in `magic_0392` (0x10004830, 80 bytes).
pub const REGISTER_LOOP_0392_4830: Sig = Sig {
    len: 80,
    hash: 0x48cb_49cc_61b0_654a,
    calls: &[],
    imports: &[],
};

/// [`slot_by_flag`] in `magic_0252` (0x10001340, 96 bytes).
pub const SLOT_BY_FLAG_0252_1340: Sig = Sig {
    len: 96,
    hash: 0x7a59_d69a_a198_b236,
    calls: &[],
    imports: &[],
};

/// [`slot_by_flag`] in `magic_0303` (0x10001420, 96 bytes).
pub const SLOT_BY_FLAG_0303_1420: Sig = Sig {
    len: 96,
    hash: 0xa955_a65d_c579_83c6,
    calls: &[],
    imports: &[],
};

/// [`slot_by_flag`] in `magic_0318` (0x10001540, 96 bytes).
pub const SLOT_BY_FLAG_0318_1540: Sig = Sig {
    len: 96,
    hash: 0x1943_23fd_0b43_f06e,
    calls: &[],
    imports: &[],
};

/// [`seq_keep_447_32`] in `magic_0416` (0x10001d20, 95 bytes).
pub const SEQ_KEEP_447_32_0416_1D20: Sig = Sig {
    len: 95,
    hash: 0x7375_aa5b_506f_a110,
    calls: &[],
    imports: &[],
};

/// [`seq_keep_447_32`] in `magic_0510` (0x10001430, 95 bytes).
pub const SEQ_KEEP_447_32_0510_1430: Sig = Sig {
    len: 95,
    hash: 0x990e_7d4e_8bc7_b162,
    calls: &[],
    imports: &[],
};

/// [`seq_keep_447_32`] in `magic_0525` (0x100013b0, 95 bytes).
pub const SEQ_KEEP_447_32_0525_13B0: Sig = Sig {
    len: 95,
    hash: 0xb80d_40e3_a6d0_5918,
    calls: &[],
    imports: &[],
};

/// [`flag_slots`] in `magic_0202` (0x10006d90, 103 bytes).
pub const FLAG_SLOTS_0202_6D90: Sig = Sig {
    len: 103,
    hash: 0x7a55_fa45_ada1_9d95,
    calls: &[],
    imports: &[],
};

/// [`flag_slots`] in `magic_0317` (0x10006a40, 103 bytes).
pub const FLAG_SLOTS_0317_6A40: Sig = Sig {
    len: 103,
    hash: 0x9aba_08a9_44a0_a4c5,
    calls: &[],
    imports: &[],
};

/// [`flag_slots`] in `magic_0597` (0x10008e80, 103 bytes).
pub const FLAG_SLOTS_0597_8E80: Sig = Sig {
    len: 103,
    hash: 0x43ee_118b_6a85_8357,
    calls: &[],
    imports: &[],
};

/// [`byte_list_slot`] in `magic_0397` (0x100063f0, 90 bytes).
pub const BYTE_LIST_SLOT_0397_63F0: Sig = Sig {
    len: 90,
    hash: 0x7706_4a57_4d68_373d,
    calls: &[],
    imports: &[],
};

/// [`byte_list_slot`] in `magic_0397` (0x10006450, 90 bytes).
pub const BYTE_LIST_SLOT_0397_6450: Sig = Sig {
    len: 90,
    hash: 0x0e1f_b3c8_a339_ad49,
    calls: &[],
    imports: &[],
};

/// [`task_list_seq_wait`] in `magic_0313` (0x10001920, 89 bytes).
pub const TASK_LIST_SEQ_WAIT_0313_1920: Sig = Sig {
    len: 89,
    hash: 0x7ca0_baf2_f254_9e69,
    calls: &[],
    imports: &[],
};

/// [`task_list_seq_wait`] in `magic_0313` (0x10001980, 89 bytes).
pub const TASK_LIST_SEQ_WAIT_0313_1980: Sig = Sig {
    len: 89,
    hash: 0x08e8_78e8_2a50_13ee,
    calls: &[],
    imports: &[],
};

/// [`ptr_stores8`] in `magic_0307` (0x10002990, 86 bytes).
pub const PTR_STORES8_0307_2990: Sig = Sig {
    len: 86,
    hash: 0x7da1_e9de_373c_627b,
    calls: &[],
    imports: &[],
};

/// [`ptr_stores8`] in `magic_0563` (0x10002490, 86 bytes).
pub const PTR_STORES8_0563_2490: Sig = Sig {
    len: 86,
    hash: 0xa35d_e407_5cdd_f413,
    calls: &[],
    imports: &[],
};

/// [`slot_task_seq_wait`] in `magic_0382` (0x100010a0, 86 bytes).
pub const SLOT_TASK_SEQ_WAIT_0382_10A0: Sig = Sig {
    len: 86,
    hash: 0x81cc_f41c_0c7a_c089,
    calls: &[],
    imports: &[],
};

/// [`slot_task_seq_wait`] in `magic_0522` (0x10001090, 86 bytes).
pub const SLOT_TASK_SEQ_WAIT_0522_1090: Sig = Sig {
    len: 86,
    hash: 0x41ca_fe8e_217f_ed9c,
    calls: &[],
    imports: &[],
};

/// [`store_node`] in `magic_0353` (0x10002390, 85 bytes).
pub const STORE_NODE_0353_2390: Sig = Sig {
    len: 85,
    hash: 0xf23d_6a14_9761_dbaa,
    calls: &[(0x15, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`store_node`] in `magic_0672` (0x10001cf0, 85 bytes).
pub const STORE_NODE_0672_1CF0: Sig = Sig {
    len: 85,
    hash: 0x638f_5613_0d22_b468,
    calls: &[(0x15, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`field_slot_flag8`] in `magic_0356` (0x10001840, 40 bytes).
pub const FIELD_SLOT_FLAG8_0356_1840: Sig = Sig {
    len: 40,
    hash: 0x877a_e9e7_4486_710d,
    calls: &[],
    imports: &[],
};

/// [`field_slot_flag8`] in `magic_0529` (0x10001620, 40 bytes).
pub const FIELD_SLOT_FLAG8_0529_1620: Sig = Sig {
    len: 40,
    hash: 0x612c_d2a4_b4e4_26b4,
    calls: &[],
    imports: &[],
};

/// [`scale_actors`] in `magic_0165` (0x100033a0, 100 bytes).
pub const SCALE_ACTORS_0165_33A0: Sig = Sig {
    len: 100,
    hash: 0x4ad9_c613_af01_7084,
    calls: &[],
    imports: &[],
};

/// [`scale_actors`] in `magic_0606` (0x10007810, 100 bytes).
pub const SCALE_ACTORS_0606_7810: Sig = Sig {
    len: 100,
    hash: 0x78a5_fd45_e73d_d7b3,
    calls: &[],
    imports: &[],
};

/// [`main_init_create_task`] in `magic_0195` (0x100019e0, 94 bytes).
pub const MAIN_INIT_0195_19E0: Sig = Sig {
    len: 94,
    hash: 0x57a7_a2b9_d945_5061,
    calls: &[],
    imports: &[],
};

/// [`main_init_create_task`] in `magic_0507` (0x100010d0, 94 bytes).
pub const MAIN_INIT_0507_10D0: Sig = Sig {
    len: 94,
    hash: 0xb5c6_dec3_5a3d_2214,
    calls: &[],
    imports: &[],
};
