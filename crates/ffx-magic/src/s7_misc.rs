//! Shared functions of the shared-7 list (the rows of the shared-4 work
//! list that shared-6 left) without a family module yet. Each doc names
//! the DLL and address it was ported from, its size and how many DLLs
//! carry it; "`G`" is a global of the DLL (an operand of the code).

use crate::sb_sse::{cvt, div, mul, sub};
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Out, Ret, Sig};

/// The word at `f + off` of the code (an operand).
fn op(img: &Image, f: u32, off: u32) -> u32 {
    img.word(f + off)
}

/// A call through slot `s` of the import table whose global is the
/// operand at `f + t`.
fn slot<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, s: u32, ecx: u32, args: &[u32]) -> Out {
    let target = slot_target(g, op(img, f, t), s);
    g.call(target, ecx, args)
}

/// `cvttss2si`: truncated, `0x80000000` when out of range or NaN.
fn trunc(x: f32) -> i32 {
    crate::s4_summon::trunc(x).cast_signed()
}

/// The model's sound handles registered (internal: `ecx` points at the
/// model `M`; an aligned frame): slot 772 turns `(M, 0x5d40043)` and `(M,
/// 0x12d644)` into 64-bit handles (`edx:eax`) and slot 779 registers
/// each; then for each of the `*G_N` words from `M + 0x81c` its address
/// (sign-extended) goes to slot 779. Each slot 779 call runs with `**G_L`
/// set to 1 and put back after. Returns the last `*G_L` read.
///
/// `magic_0150` 0x10004110 (199 bytes; 2 DLLs).
pub fn handles_registered<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let m = g.read::<u32>(fr.ecx);
    // the words left on the stack from the calls before
    let h = slot(g, img, f, 0x8, 772, fr.ecx, &[m, 0x05d4_0043]);
    let lock = g.read::<u32>(w(0x21));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    slot(g, img, f, 0x2a, 779, lock, &[h.eax, h.edx, m, 0x05d4_0043]);
    let p = g.read::<u32>(w(0x3d));
    g.write::<u32>(p, saved);
    let h2 = slot(
        g,
        img,
        f,
        0x49,
        772,
        lock,
        &[m, 0x0012_d644, h.eax, h.edx, m, 0x05d4_0043],
    );
    let lock = g.read::<u32>(w(0x58));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let words = [h2.eax, h2.edx, m, 0x0012_d644, h.eax, h.edx, m, 0x05d4_0043];
    slot(g, img, f, 0x61, 779, lock, &words);
    let mut p = g.read::<u32>(w(0x74));
    g.write::<u32>(p, saved);
    if g.read::<i32>(w(0x81)) <= 0 {
        return p;
    }
    let mut i = 0i32;
    let mut at = m.wrapping_add(0x81c);
    loop {
        let saved = g.read::<u32>(p);
        g.write::<u32>(p, 1);
        // `cdq`: the address as a signed 64-bit value
        let high = if at.cast_signed() < 0 { u32::MAX } else { 0 };
        slot(g, img, f, 0x9e, 779, lock, &[at, high]);
        p = g.read::<u32>(w(0xab));
        i += 1;
        g.write::<u32>(p, saved);
        at = at.wrapping_add(4);
        if i >= g.read::<i32>(w(0xba)) {
            return p;
        }
    }
}

/// [`handles_registered`].
pub const HANDLES_REGISTERED: Sig = Sig {
    len: 199,
    hash: 0x6d88_f350_53d0_de08,
    calls: &[],
    imports: &[],
};

/// The word `O + 0x2c + 0x10 O[0xac]`: a part's counter.
fn counter<G: Game>(g: &G, o: u32) -> u32 {
    let i = g.read::<u32>(o.wrapping_add(0xac));
    o.wrapping_add(0x2c).wrapping_add(i << 4)
}

/// A part's count and fade (internal: `ecx` = the part `O`; its counter
/// `c` is the word `O + 0x2c + 0x10 O[0xac]`). One `rand()` in 16 (the
/// signed remainder 0) starts a node (slot 434 `(*G_X, CB)`) and stores
/// slot 942 `(O)`'s answer as that node's counter. Then with `c + 1`
/// and the limit `n = *G_N`, the fade `k = K1 K2`: at or past the limit
/// `O[0] |= 4` and `O+0x17c = k`; else the counter is `c + 1` and
/// `O+0x17c` is `k`, or `(n - c - 1) / K3 k` in the last five counts.
/// Returns what the original leaves in `eax`: twice the index when it
/// counts, else the remainder or slot 942's answer.
///
/// `magic_0384` 0x10001d30 (217 bytes; 3 DLLs).
pub fn count_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let o = fr.ecx;
    let k = mul(g.read::<f32>(w(0x8)), g.read::<f32>(w(0x10)));
    let c = g.read::<i32>(counter(g, o));
    let mut eax = (g.rand().cast_signed() % 16).cast_unsigned();
    if eax == 0 {
        let x = g.read::<u32>(w(0x50));
        let node = slot(g, img, f, 0x3e, 434, o, &[x, w(0x4a)]).eax;
        // `call ecx`: the slot's address is in `ecx`
        let t = slot_target(g, w(0x58), 942);
        eax = g.call(t, t, &[o, x, w(0x4a)]).eax;
        let at = counter(g, node);
        g.write::<u32>(at, eax);
    }
    let n = g.read::<i32>(w(0x79));
    let c = c.wrapping_add(1);
    if c >= n {
        let v = g.read::<u32>(o) | 4;
        g.write::<u32>(o, v);
        g.write::<f32>(o + 0x17c, k);
        return eax;
    }
    let fade = if c > n.wrapping_sub(5) {
        let left = cvt(n.wrapping_sub(c).cast_unsigned());
        mul(div(left, g.read::<f32>(w(0xac))), k)
    } else {
        k
    };
    let eax = g.read::<u32>(o + 0xac).wrapping_mul(2);
    let at = counter(g, o);
    g.write::<i32>(at, c);
    g.write::<f32>(o + 0x17c, fade);
    eax
}

/// [`count_fade`] (the import: `rand`).
pub const COUNT_FADE: Sig = Sig {
    len: 217,
    hash: 0x1d59_faf3_6d99_34e9,
    calls: &[],
    imports: &[(0x2b, "rand")],
};

/// The pass over a ring's nodes the node handlers start with (`S` the
/// cursor: `S[0]` the ring `R`, byte `+4` "passed", byte `+5` the pass
/// number). When `R`'s byte `+0x1a` is not the pass, each node of `R`
/// (from `R+0x10`, on through `+8`) becomes `S[0]` and gets its handler
/// `[[node] + 0x1c] (S)`; then `S[0] = R`, `R+0x1a` = the pass and
/// "passed" is set (`true`), else "passed" is cleared. Also returns what
/// the original leaves in `eax` (from the entry's `eax`): the low byte
/// the pass test loaded over the entry's or the last handler's `eax`.
fn ring_pass<G: Game>(g: &mut G, s: u32, eax: u32) -> (bool, u32) {
    let r = g.read::<u32>(s);
    let low = |eax: u32, b: u8| (eax & !0xff) | u32::from(b);
    let mut eax = low(eax, g.read::<u8>(r + 0x1a));
    if g.read::<u8>(r + 0x1a) == g.read::<u8>(s + 5) {
        g.write::<u8>(s + 4, 0);
        return (false, eax);
    }
    let mut node = g.read::<u32>(r + 0x10);
    while node != r {
        g.write::<u32>(s, node);
        let cur = g.read::<u32>(s);
        node = g.read::<u32>(node + 8);
        let handler = g.read::<u32>(g.read::<u32>(cur).wrapping_add(0x1c));
        eax = g.call(handler, s, &[s]).eax;
    }
    eax = low(eax, g.read::<u8>(s + 5));
    g.write::<u32>(s, r);
    g.write::<u8>(r + 0x1a, eax as u8);
    g.write::<u8>(s + 4, 1);
    (true, eax)
}

/// A ring pass and a key frame (internal: `ecx` = the cursor `S`: `S[0]`
/// the ring `R`, byte `+4` "passed", byte `+5` the pass number). When
/// `R`'s byte `+0x1a` is not the pass, each node of `R` (from `R+0x10`,
/// on through `+8`) becomes `S[0]` and gets its handler `[[node] + 0x1c]
/// (S)`; then `S[0] = R`, `R+0x1a` = the pass and "passed" is set, else
/// "passed" is cleared and it ends. Passed: with `R`'s motion `m =
/// R+0x2c`, the key table `T` = slot 941 `(m+0x30)` and the frame `i =
/// trunc(R+0x30 / (m+0x40)+0x20)`: past the table's count `trunc(T+0x18)`
/// its last key (and `R+0x34 |= 1`), else key `i` (`R+0x34 &= !1`): three
/// words from `T + 0x40 + 0x30 i` into `R+0x1c..`. No table: `R+0x34 &=
/// !1` (the original tests a local it never writes: 0).
///
/// Returns what the original leaves in `eax` (the key's last word, 0
/// without a table; else the low byte the pass test loaded over the
/// handler's `eax`, or the entry's).
///
/// `magic_0578` 0x100066a0 (217 bytes; 2 DLLs).
pub fn ring_key_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let s = fr.ecx;
    let (passed, mut eax) = ring_pass(g, s, fr.eax);
    if !passed {
        return eax;
    }
    let r = g.read::<u32>(s);
    let m = g.read::<u32>(r + 0x2c);
    if m == 0 {
        return eax;
    }
    let len = g.read::<f32>(g.read::<u32>(m + 0x40).wrapping_add(0x20));
    let frame = trunc(div(g.read::<f32>(r + 0x30), len));
    let x = g.read::<u32>(m + 0x30);
    let table = slot(g, img, f, 0x65, 941, m, &[x]).eax;
    // junk read: [ebp-4] never written; 0 (brief 2026-10-24): no table
    // clears the flag
    let mut past = false;
    eax = 0;
    if table != 0 {
        let count = trunc(g.read::<f32>(table + 0x18));
        past = frame >= count;
        let key = if past { count } else { frame };
        let at = table
            .wrapping_add(0x40)
            .wrapping_add(key.cast_unsigned().wrapping_mul(0x30));
        for o in [0u32, 4, 8] {
            eax = g.read::<u32>(at.wrapping_add(o));
            g.write::<u32>(r + 0x1c + o, eax);
        }
    }
    let v = g.read::<u32>(r + 0x34);
    g.write::<u32>(r + 0x34, if past { v | 1 } else { v & !1 });
    eax
}

/// [`ring_key_frame`].
pub const RING_KEY_FRAME: Sig = Sig {
    len: 217,
    hash: 0x6566_6d8c_9944_91d6,
    calls: &[],
    imports: &[],
};

/// Two scaled unit matrices multiplied (cdecl, no arguments; a /GS frame
/// aligned to 8): slot 719 `(*G_S, 2, 5)`; the locals `A`, `B` and `C`
/// made unit matrices (slot 235), `*G_H += *G_A + *G_B`, `A`'s diagonal
/// 0.001 and `C`'s 0.00075 (the first three terms), and slot 231 `(D,
/// G_M, C)` with `D` a fourth local. Returns slot 231's `eax`.
///
/// `magic_0443` 0x10001000 (226 bytes; 2 DLLs).
pub fn scaled_matrices<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x108);
    let (a, b, c, d) = (base + 0x40, base + 0x80, base, base + 0xc0);
    let s = g.read::<u32>(w(0x2b));
    let mut st = Pushes::default();
    st.push_all(&[5, 2, s]);
    st.call(g, slot_target(g, w(0x1b), 719), fr.ecx);
    st.push(a);
    st.call(g, slot_target(g, w(0x37), 235), fr.ecx);
    st.push(b);
    st.call(g, slot_target(g, w(0x4c), 235), fr.ecx);
    let add = g.read::<u32>(w(0x59)).wrapping_add(g.read::<u32>(w(0x5f)));
    g.write::<u32>(a, 0x3a83_126f);
    let h = g.read::<u32>(w(0x6d)).wrapping_add(add);
    g.write::<u32>(w(0x6d), h);
    g.write::<u32>(a + 0x14, 0x3a83_126f);
    g.write::<u32>(a + 0x28, 0x3a83_126f);
    st.push(c);
    st.call(g, slot_target(g, w(0x77), 235), fr.ecx);
    for o in [0x28u32, 0x14, 0] {
        g.write::<u32>(c + o, 0x3a44_9ba6);
    }
    st.push_all(&[c, w(0xa3), d]);
    Ret::eax(st.call(g, slot_target(g, w(0xa9), 231), fr.ecx).eax)
}

/// [`scaled_matrices`] (the `rel32`: `__security_check_cookie`).
pub const SCALED_MATRICES: Sig = Sig {
    len: 226,
    hash: 0xc873_9dde_8696_3a50,
    calls: &[(0xda, Callee::Any)],
    imports: &[],
};

/// Eight differences of the DLL's globals (internal: `edx` a flag,
/// `xmm2` a value): `*G_V = xmm2`, `*G_Z = 0`; with the flag, `*G_F |=
/// 1` and nothing else; else `*G_F &= !1` and the eight `*D_i = *X_i -
/// *Y_i`. Leaves `eax` as it was.
///
/// `magic_0042` 0x10008200 (230 bytes, `edx`/`xmm2`; 3 DLLs).
pub fn global_differences<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let r = |g: &G, o: u32| g.read::<f32>(w(o));
    g.write::<f32>(w(0x4), fr.xmm_f32(2));
    g.write::<u32>(w(0xa), 0);
    if fr.edx != 0 {
        let v = g.read::<u32>(w(0x18)) | 1;
        g.write::<u32>(w(0x18), v);
        return fr.eax;
    }
    // the first difference's subtraction comes before the flag's store
    let x = sub(r(g, 0x22), r(g, 0x2a));
    let v = g.read::<u32>(w(0x30)) & !1;
    g.write::<u32>(w(0x30), v);
    g.write::<f32>(w(0x39), x);
    for at in (0x3du32..=0xcd).step_by(0x18) {
        let x = sub(r(g, at + 4), r(g, at + 0xc));
        g.write::<f32>(w(at + 0x14), x);
    }
    fr.eax
}

/// [`global_differences`].
pub const GLOBAL_DIFFERENCES: Sig = Sig {
    len: 230,
    hash: 0x5f71_797f_8b52_ba15,
    calls: &[],
    imports: &[],
};

/// [`start_object_reset`](crate::s6_misc::start_object_reset)'s copy
/// whose state block reset is
/// [`RESET_STATE_1E`](crate::s6_misc::RESET_STATE_1E) (`magic_0101`
/// 0x10001a80, 231 bytes; 3 DLLs).
pub const START_OBJECT_RESET_1E: Sig = Sig {
    calls: &[
        (0x7d, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x8d, Callee::Sig(&crate::s6_misc::RESET_STATE_1E)),
        (0x95, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
    ],
    ..crate::s6_misc::START_OBJECT_RESET
};

/// A node taken from the DLL's free list and put on its live list
/// (internal: `ecx` = the owner `O`, kept in `*G_O`; `O+0x60`, `+0xc8`,
/// `+0x130`, `*G_P` zeroed, `*G_Q = 3`). The node `N = *G_F` (the free
/// count `*G_C`, u16) leaves its ring (`next.prev = prev`, `prev.next =
/// next`), `*G_F` its `prev` (0 when it was the last one) and the count
/// goes down; `N` is cleared (`+0`, `+4`, `+0xc`, `+0x14` 0, `+8 = N`,
/// `+0x10 = CB`, byte `+0x18` 0) and appended to the live list (`*G_L`
/// its first, `*G_T` its last; `N[0]` the old last). Returns `N`. With
/// none free `N` is 0 and the original writes to address 4 (a fault):
/// no test runs it.
///
/// `magic_0285` 0x100020b0 (236 bytes; 3 DLLs).
pub fn node_to_live<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let o = fr.ecx;
    let mut free = g.read::<u16>(w(0x4));
    g.write::<u32>(w(0xa), o);
    for off in [0x60u32, 0xc8, 0x130] {
        g.write::<u32>(o + off, 0);
    }
    g.write::<u32>(w(0x2b), 0);
    g.write::<u32>(w(0x35), 3);
    let mut n = 0;
    if free != 0 {
        n = g.read::<u32>(w(0x48));
        let (next, prev) = (g.read::<u32>(n), g.read::<u32>(n + 4));
        g.write::<u32>(next.wrapping_add(4), prev);
        let (prev, next) = (g.read::<u32>(n + 4), g.read::<u32>(n));
        g.write::<u32>(prev, next);
        if free > 1 {
            let prev = g.read::<u32>(n + 4);
            g.write::<u32>(w(0x65), prev);
        } else {
            g.write::<u32>(w(0x6d), 0);
        }
        free = free.wrapping_sub(1);
        for off in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(n + off, 0);
        }
        g.write::<u32>(n + 8, n);
        g.write::<u32>(n + 0x10, w(0x9e));
        g.write::<u8>(n + 0x18, 0);
        g.write::<u16>(w(0xa9), free);
    }
    let first = g.read::<u32>(w(0xaf));
    g.write::<u32>(n.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(0xdc), n);
        g.write::<u32>(n.wrapping_add(8), n);
        g.write::<u32>(w(0xe5), n);
    } else {
        let last = g.read::<u32>(w(0xbf));
        g.write::<u32>(n, last);
        g.write::<u32>(last.wrapping_add(4), n);
        g.write::<u32>(n.wrapping_add(8), n);
        g.write::<u32>(w(0xcd), n);
    }
    n
}

/// [`node_to_live`].
pub const NODE_TO_LIVE: Sig = Sig {
    len: 236,
    hash: 0xb3c0_4c2d_e8a3_549f,
    calls: &[],
    imports: &[],
};

/// A part's count down and its three children (cdecl `(O)`, the part's
/// timer the word `O + 0x30 + 0x10 O[0xac]`): the timer goes down; while
/// it is above 0 three nodes start (slot 434 `(*G_X, CB_i)`), each getting
/// slot 942 `(O)`'s answer as its counter; at 0 one node starts (`CB_E`)
/// with `O`'s counter as its own and `O[0] |= 4`. Returns the last node
/// (at 0) or slot 942's last answer.
///
/// `magic_0191` 0x10001620 (255 bytes; 3 DLLs).
pub fn countdown_children<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let o = fr.arg(g, 0);
    let timer = |g: &G| counter(g, o).wrapping_add(4);
    let at = timer(g);
    let t = g.read::<i32>(at).wrapping_sub(1);
    g.write::<i32>(at, t);
    let left = g.read::<i32>(timer(g));
    let start = slot_target(g, w(0x25), 434);
    if left <= 0 {
        let x = g.read::<u32>(w(0xd8));
        let node = g.call(start, fr.ecx, &[x, w(0xd2)]).eax;
        let c = g.read::<u32>(counter(g, o));
        let at = counter(g, node);
        g.write::<u32>(at, c);
        let v = g.read::<u32>(o) | 4;
        g.write::<u32>(o, v);
        return Ret::eax(node);
    }
    // (the callback, the table operands of slots 434 and 942)
    let mut st = Pushes::default();
    let mut ecx = fr.ecx;
    let mut eax = 0;
    for (k, (cb, gx, t434, t942)) in [
        (0x37u32, 0x3du32, 0x25u32, 0x45u32),
        (0x5d, 0x63, 0x6c, 0x7a),
        (0x92, 0x98, 0xa1, 0xaf),
    ]
    .into_iter()
    .enumerate()
    {
        let x = g.read::<u32>(w(gx));
        st.push_all(&[w(cb), x]);
        let start = if k == 0 { start } else { slot_target(g, w(t434), 434) };
        let node = st.call(g, start, ecx).eax;
        st.push(o);
        // `call ecx`: the slot's address is in `ecx`
        let t = slot_target(g, w(t942), 942);
        eax = st.call(g, t, t).eax;
        ecx = g.read::<u32>(node + 0xac).wrapping_mul(2);
        let at = counter(g, node);
        g.write::<u32>(at, eax);
    }
    Ret::eax(eax)
}

/// [`countdown_children`].
pub const COUNTDOWN_CHILDREN: Sig = Sig {
    len: 255,
    hash: 0xf533_e4c0_84fc_37b4,
    calls: &[],
    imports: &[],
};

/// Where a summon frame's variant has its operands (offsets in the
/// function).
struct SummonOps {
    /// The stage, `*P = ITEM` and the models' binding `rel32`.
    stage: u32,
    p: u32,
    item: u32,
    bind: u32,
    /// The bone runs before the ring (else after it).
    bone_first: bool,
    /// The bone: the pass, the bone `A` (an immediate) and its `rel32`.
    bone: (u32, u32, u32),
    /// The ring: skip, first, root (three compares), first again, the
    /// flags.
    ring: (u32, u32, [u32; 3], u32, u32),
    /// The two visits: the pass, the node (an immediate) and the `rel32`.
    visits: [(u32, u32, u32); 2],
    /// The flags' mask word, the flags, the word `W`, slot 214's table
    /// operand and its argument `*G_Z`.
    end: (u32, u32, u32, u32, u32),
}

/// The summon frame both variants share (an aligned frame; `E` = `esp`
/// after it and one push, the cursor at `E + 8`); [`summon_frame_bound`]
/// gives the order.
fn summon_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame, o: &SummonOps) -> Ret {
    let f = fr.at;
    let w = |off: u32| op(img, f, off);
    if g.read::<i32>(w(o.stage)) < 4 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(o.p));
    g.write::<u32>(p, w(o.item));
    crate::sa_summon::bind_models(g, img, img.rel32_target(f + o.bind));
    // push ebp; and esp, -8; sub esp, 0xc; push esi
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xc + 4);
    let cursor = e + 8;
    let bone = |g: &mut G| {
        let (pass, a, call) = o.bone;
        let pass = g.read::<u8>(w(pass));
        g.write::<u32>(cursor, w(a));
        g.write::<u8>(cursor + 5, pass);
        let bone = Frame {
            at: img.rel32_target(f + call),
            esp: e.wrapping_sub(4),
            ecx: cursor,
            ..*fr
        };
        crate::sa_summon::bone_plain(g, img, &bone);
    };
    if o.bone_first {
        bone(g);
    }
    let (skip, first, roots, first2, flags) = o.ring;
    let skip = g.read::<u32>(w(skip)) != 0;
    let mut node = g.read::<u32>(w(first));
    // an empty ring sets the flag at once; else the first node after the
    // walk (or the skip) is compared again
    let empty = if !skip && node == w(roots[0]) {
        true
    } else {
        if !skip {
            while node != w(roots[1]) {
                g.write::<u32>(cursor, node);
                let next = g.read::<u32>(node + 8);
                let method = g.read::<u32>(g.read::<u32>(node).wrapping_add(0x20));
                g.call(method, cursor, &[cursor]);
                node = next;
            }
            node = g.read::<u32>(w(first2));
        }
        node == w(roots[2])
    };
    if empty {
        let v = g.read::<u32>(w(flags)) | 1;
        g.write::<u32>(w(flags), v);
    }
    if !o.bone_first {
        bone(g);
    }
    for (pass, node, call) in o.visits {
        let pass = g.read::<u8>(w(pass));
        g.write::<u8>(cursor + 5, pass);
        g.write::<u32>(cursor, w(node));
        let esp = e.wrapping_sub(8);
        g.write::<u32>(esp + 4, cursor);
        let visit = Frame {
            at: img.rel32_target(f + call),
            esp,
            ecx: 0,
            ..*fr
        };
        crate::nodes::visit_gated(g, img, &visit);
    }
    let (mask, flags, word, table, z) = o.end;
    let bits = g.read::<u32>(w(mask)) & g.read::<u32>(w(flags));
    if bits & 1 != 0 && g.read::<u16>(w(word)) == 0 {
        let t = slot_target(g, w(table), 214);
        let z = g.read::<u32>(w(z));
        g.call(t, 0, &[z]);
    }
    Ret::eax(0)
}

/// A summon's frame with the models bound and the bone before the ring
/// (a table entry, no arguments; an 8-aligned frame, `E` = `esp` after it
/// and one push): nothing before stage 4 (`G_STAGE`). `*G_P = ITEM`; the
/// models bound ([`crate::sa_summon::bind_models`]); the bone `A` updated
/// ([`crate::sa_summon::bone_plain`]) with the pass `G_PASS` (a cursor on
/// the frame, `E + 8`); unless `G_SKIP` is set, each node of the root's
/// ring runs its method `+0x20` with the cursor, and an empty ring sets
/// bit 0 of `G_FLAGS`; the nodes `B` and `C` visited
/// ([`crate::nodes::visit_gated`]), each with the pass. With bit 0 of
/// `*G_M & G_FLAGS` and the word `G_W` 0, slot 214 `(*G_Z)` follows.
/// Returns 0.
///
/// `magic_0155` 0x1000c450 (259 bytes; 4 DLLs). [`summon_frame_ring`]
/// is the variant with the ring first.
pub fn summon_frame_bound<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = SummonOps {
        stage: 0xb,
        p: 0x18,
        item: 0x1e,
        bind: 0x23,
        bone_first: true,
        bone: (0x28, 0x34, 0x3d),
        ring: (0x43, 0x4a, [0x52, 0x72, 0x80], 0x7a, 0x88),
        visits: [(0x90, 0xa1, 0xa6), (0xad, 0xc1, 0xc6)],
        end: (0xcb, 0xd1, 0xdf, 0xe7, 0xed),
    };
    summon_frame(g, img, fr, &o)
}

/// [`summon_frame_bound`] (the `rel32`s: [`crate::sa_summon::bind_models`],
/// [`crate::sa_summon::bone_plain`] and two
/// [`crate::nodes::visit_gated`]).
pub const SUMMON_FRAME_BOUND: Sig = Sig {
    len: 259,
    hash: 0x1d10_fa3e_90b2_84a0,
    calls: &[
        (0x23, Callee::Sig(&crate::sa_summon::BIND_MODELS)),
        (0x3d, Callee::Sig(&crate::sa_summon::BONE)),
        (0xa6, Callee::Sig(&crate::nodes::GATED)),
        (0xc6, Callee::Sig(&crate::nodes::GATED)),
    ],
    imports: &[],
};

/// [`summon_frame_bound`]'s variant with the ring walked before the bone
/// (`magic_0150` 0x1000dfa0, 263 bytes; 4 DLLs).
pub fn summon_frame_ring<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = SummonOps {
        stage: 0xb,
        p: 0x18,
        item: 0x1e,
        bind: 0x23,
        bone_first: false,
        bone: (0x78, 0x84, 0x8d),
        ring: (0x29, 0x30, [0x38, 0x5a, 0x68], 0x62, 0x70),
        visits: [(0x94, 0xa5, 0xaa), (0xb1, 0xc5, 0xca)],
        end: (0xcf, 0xd5, 0xe3, 0xeb, 0xf1),
    };
    summon_frame(g, img, fr, &o)
}

/// [`summon_frame_ring`] (the `rel32`s as [`SUMMON_FRAME_BOUND`]'s).
pub const SUMMON_FRAME_RING: Sig = Sig {
    len: 263,
    hash: 0xeca5_9b9e_01f7_a322,
    calls: &[
        (0x23, Callee::Sig(&crate::sa_summon::BIND_MODELS)),
        (0x8d, Callee::Sig(&crate::sa_summon::BONE)),
        (0xaa, Callee::Sig(&crate::nodes::GATED)),
        (0xca, Callee::Sig(&crate::nodes::GATED)),
    ],
    imports: &[],
};

/// A camera's settings put (cdecl, no arguments; one stack word): slots
/// 783, 782 and 784 with one constant each, slot 781 with three, slot
/// 785 with one, slot 801 with six copies of `K / *G_D`, then `*G_D =
/// 1.0` and slot 785 with its constant again. Each constant is a word of
/// the DLL's. Returns the last slot's `eax`.
///
/// `magic_0098` 0x10001090 (271 bytes; 4 DLLs).
pub fn camera_settings<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<u32>(w(o));
    for (table, s, c) in [(0x1u32, 783u32, 0x9u32), (0x1c, 782, 0x24), (0x36, 784, 0x3e)] {
        let t = slot_target(g, w(table), s);
        let x = k(g, c);
        g.call(t, fr.ecx, &[x]);
    }
    let (a, b) = (k(g, 0x53), k(g, 0x65));
    let t = slot_target(g, w(0x58), 781);
    let c = k(g, 0x7c);
    g.call(t, fr.ecx, &[c, b, a]);
    let t = slot_target(g, w(0x88), 785);
    let x = k(g, 0x90);
    g.call(t, fr.ecx, &[x]);
    let x = div(g.read::<f32>(w(0xa8)), g.read::<f32>(w(0xb0))).to_bits();
    let t = slot_target(g, w(0xb5), 801);
    g.call(t, fr.ecx, &[x; 6]);
    let t = slot_target(g, w(0xe7), 785);
    let x = k(g, 0xef);
    g.write::<u32>(w(0xfe), 0x3f80_0000);
    Ret::eax(g.call(t, fr.ecx, &[x]).eax)
}

/// [`camera_settings`].
pub const CAMERA_SETTINGS: Sig = Sig {
    len: 271,
    hash: 0xc5ca_4e86_44be_828a,
    calls: &[],
    imports: &[],
};

/// A ring node's handler that sets its model's points (internal: `ecx` =
/// the cursor `S`; a ring pass first, [`ring_pass`]). Passed, with `R =
/// S[0]` and its flags byte `R+0x240`: without bit 0, slot 229 `(R+0x1c,
/// R+0xc8, R+0x218)` and slot 284 `(R[0x23c], 0, R+0xf8)` then `(R[0x23c],
/// 1, R+0x1c)`; bits 0 and 1, slot 284 `(R[0x23c], 0, R+0x22c)`; bit 0
/// only, the child `C = R+0x228` (none: nothing) gets its handler with a
/// cursor of its own on the frame (`C`, pass byte `+5`) and slot 284
/// `(R[0x23c], 0, C+0x1c)` follows. Returns the last slot's `eax`, else
/// what the pass or the flags test left.
///
/// `magic_0150` 0x10002d30 (293 bytes; 2 DLLs).
pub fn model_points_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let s = fr.ecx;
    let (passed, eax) = ring_pass(g, s, fr.eax);
    if !passed {
        return eax;
    }
    let r = g.read::<u32>(s);
    let bits = g.read::<u8>(r + 0x240);
    let eax = (eax & !0xff) | u32::from(bits);
    let model = |g: &G| g.read::<u32>(r + 0x23c);
    let mut st = Pushes::default();
    if bits & 1 == 0 {
        st.push_all(&[r + 0x218, r + 0xc8, r + 0x1c]);
        st.call(g, slot_target(g, w(0xd7), 229), fr.ecx);
        let m = model(g);
        st.push_all(&[r + 0xf8, 0, m]);
        st.call(g, slot_target(g, w(0xeb), 284), fr.ecx);
        st.pop(6);
        let m = model(g);
        st.push_all(&[r + 0x1c, 1, m]);
        return st.call(g, slot_target(g, w(0x107), 284), fr.ecx).eax;
    }
    if bits & 2 != 0 {
        let m = model(g);
        st.push_all(&[r + 0x22c, 0, m]);
        return st.call(g, slot_target(g, w(0x66), 284), fr.ecx).eax;
    }
    let c = g.read::<u32>(r + 0x228);
    if c == 0 {
        return eax;
    }
    // the child's cursor at `ebp - 8`
    let cursor = fr.esp.wrapping_sub(0xc);
    let pass = g.read::<u8>(s + 5);
    g.write::<u32>(cursor, c);
    g.write::<u8>(cursor + 5, pass);
    let handler = g.read::<u32>(g.read::<u32>(c).wrapping_add(0x1c));
    st.push(cursor);
    st.call(g, handler, cursor);
    let at = g.read::<u32>(r + 0x228).wrapping_add(0x1c);
    let m = model(g);
    st.push_all(&[at, 0, m]);
    st.call(g, slot_target(g, w(0xab), 284), fr.ecx).eax
}

/// [`model_points_node`].
pub const MODEL_POINTS_NODE: Sig = Sig {
    len: 293,
    hash: 0xe442_fa48_524e_cd74,
    calls: &[],
    imports: &[],
};

/// A ring node's handler that sets a light (internal: `ecx` = the cursor
/// `S`; a ring pass first, [`ring_pass`], whose result it ignores). With
/// `R = S[0]` and its count byte `n = R+0x1c` (0: nothing): slot 245
/// `(L)` (two words on the frame); `x = R+0x20` (not above 0: nothing),
/// `L[1] = K / x`, slot 821 `(0)`, slot 822 `(1, 1, 1, n K2)`, slot 823
/// `(L[1], L[1])` and slot 824 `(0.4990, 0.7320, 0, 0)`. Returns the last
/// slot's `eax`, else `R`.
///
/// `magic_0150` 0x10001000 (303 bytes; 2 DLLs).
pub fn light_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let s = fr.ecx;
    ring_pass(g, s, fr.eax);
    let r = g.read::<u32>(s);
    if g.read::<u8>(r + 0x1c) == 0 {
        return r;
    }
    let local = fr.esp.wrapping_sub(0xc);
    slot(g, img, f, 0x56, 245, fr.ecx, &[local]);
    let r = g.read::<u32>(s);
    let x = g.read::<f32>(r + 0x20);
    let n = u32::from(g.read::<u8>(r + 0x1c));
    // `comiss`/`jbe`: not above 0 (or unordered) ends it
    if x.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater) {
        return r;
    }
    let inv = div(g.read::<f32>(w(0x80)), x);
    let t = slot_target(g, w(0x85), 821);
    g.write::<f32>(local + 4, inv);
    g.call(t, fr.ecx, &[0]);
    let t = slot_target(g, w(0x9d), 822);
    let v = mul(cvt(n), g.read::<f32>(w(0xb5))).to_bits();
    let one = 0x3f80_0000;
    g.call(t, fr.ecx, &[one, one, one, v]);
    let t = slot_target(g, w(0xde), 823);
    let inv = g.read::<u32>(local + 4);
    g.call(t, fr.ecx, &[inv, inv]);
    let t = slot_target(g, w(0xf9), 824);
    g.call(t, fr.ecx, &[0x3eff_8000, 0x3f3b_6276, 0, 0]).eax
}

/// [`light_node`].
pub const LIGHT_NODE: Sig = Sig {
    len: 303,
    hash: 0x6dfe_0e26_71c6_d3d4,
    calls: &[],
    imports: &[],
};

/// A summon's frame that loads first (a table entry, no arguments; `E` =
/// the entry's `esp`, the cursor at `E - 0xc`). The load state `*G_S`:
/// 0 and 1 wait on slots 225 and 313 `(2)` (busy: return), each going to
/// the next state when done; 2 waits on slot 549 `(2)`, then slot 226
/// `(*G_Y, 0)` and state 4; from there (any other state) the frame runs:
/// slot 673, `*G_P = ITEM`, `G_X = *G_Z`, the pass byte up; unless
/// `G_SKIP`, each node of the root's ring runs its method `+0x20` with
/// the cursor; an empty ring sets bit 0 of `G_FLAGS`; the nodes `B` and
/// `C` visited ([`crate::nodes::visit_gated`]) with the pass (the walk's
/// pass read again, else the new one). Returns 0.
///
/// `magic_0152` 0x1000c7b0 (317 bytes; 5 DLLs).
pub fn summon_load_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    loop {
        let state = g.read::<u32>(w(0x4));
        let (table, s) = match state {
            0 => (0x26, 225),
            1 => (0x19, 313),
            2 => {
                if slot(g, img, f, 0x50, 549, fr.ecx, &[2]).eax != 0 {
                    return Ret::eax(0);
                }
                let y = g.read::<u32>(w(0x71));
                slot(g, img, f, 0x6b, 226, fr.ecx, &[y, 0]);
                g.write::<u32>(w(0x82), 4);
                break;
            }
            _ => break,
        };
        if slot(g, img, f, table, s, fr.ecx, &[2]).eax != 0 {
            return Ret::eax(0);
        }
        let v = g.read::<u32>(w(0x40)).wrapping_add(1);
        g.write::<u32>(w(0x46), v);
    }
    slot(g, img, f, 0x8b, 673, fr.ecx, &[]);
    let p = g.read::<u32>(w(0x99));
    g.write::<u32>(p, w(0x9f));
    let z = g.read::<u32>(w(0xa4));
    let mut node = g.read::<u32>(w(0xaa));
    g.write::<u32>(w(0xaf), z);
    let mut pass = g.read::<u8>(w(0xb4)).wrapping_add(1);
    let skip = g.read::<u32>(w(0xbc)) != 0;
    g.write::<u8>(w(0xc2), pass);
    let cursor = fr.esp.wrapping_sub(0xc);
    if !skip && node != w(0xca) {
        while node != w(0xe8) {
            g.write::<u32>(cursor, node);
            let method = g.read::<u32>(g.read::<u32>(node).wrapping_add(0x20));
            node = g.read::<u32>(node + 8);
            g.call(method, cursor, &[cursor]);
        }
        pass = g.read::<u8>(w(0xef));
        node = g.read::<u32>(w(0xf5));
    }
    // the empty ring's test (an empty ring before the walk takes it too)
    if node == w(0xfb) {
        let v = g.read::<u32>(w(0x104)) | 1;
        g.write::<u32>(w(0x104), v);
    }
    for (k, (node, call)) in [(0x113u32, 0x118u32), (0x12b, 0x130)].into_iter().enumerate() {
        if k == 1 {
            pass = g.read::<u8>(w(0x11d));
        }
        g.write::<u8>(cursor + 5, pass);
        let esp = fr.esp.wrapping_sub(0x14 + 4 * k as u32);
        g.write::<u32>(esp + 4, cursor);
        g.write::<u32>(cursor, w(node));
        let visit = Frame {
            at: img.rel32_target(f + call),
            esp,
            ecx: 0,
            ..*fr
        };
        crate::nodes::visit_gated(g, img, &visit);
    }
    Ret::eax(0)
}

/// [`summon_load_frame`] (the `rel32`s: two
/// [`crate::nodes::visit_gated`]).
pub const SUMMON_LOAD_FRAME: Sig = Sig {
    len: 317,
    hash: 0xf1be_6350_1100_8604,
    calls: &[
        (0x118, Callee::Sig(&crate::nodes::GATED)),
        (0x130, Callee::Sig(&crate::nodes::GATED)),
    ],
    imports: &[],
};

/// A ring node's handler that draws its model (internal: `ecx` = the
/// cursor `S`; a ring pass first, [`ring_pass`]; a /GS frame aligned to
/// 16). Passed, with `R = S[0]` and its model `M = R+0x20`: the matrix
/// `X` (a local) is slot 231 `(X, G_M, R[0x1c] + 0x9c)`, and the draw
/// record `L` (a local) gets `0x1c`, `**G_A`, the u16 `M+0x16`, zeros,
/// `R+0x24`, `X` four times, `*G_V` and the model's two tables `M + M[8]`
/// and `M + M[0xc]`; slot 308 `(M, L)`'s answer goes to `*G_V` and is
/// returned. Else returns what the pass left.
///
/// `magic_0478` 0x10003f00 (324 bytes; 3 DLLs).
pub fn model_draw_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let s = fr.ecx;
    let (passed, eax) = ring_pass(g, s, fr.eax);
    if !passed {
        return eax;
    }
    let r = g.read::<u32>(s);
    let m = g.read::<u32>(r + 0x20);
    let base = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0xc8);
    let (l, x) = (base + 8, base + 0x80);
    g.write::<u32>(l, 0x1c);
    let kind = g.read::<u16>(m.wrapping_add(0x16));
    g.write::<u16>(l + 8, kind);
    g.write::<u32>(l + 0x1c, 0);
    for (at, o) in [(0x38u32, 8u32), (0x3c, 0xc)] {
        let v = g.read::<u32>(m.wrapping_add(o)).wrapping_add(m);
        g.write::<u32>(l + at, v);
    }
    let a = g.read::<u32>(g.read::<u32>(w(0xa1)));
    g.write::<u32>(l + 4, a);
    g.write::<u32>(l + 0xc, 0);
    g.write::<u32>(l + 0x10, 0);
    let v = g.read::<u32>(r + 0x24);
    g.write::<u32>(l + 0x18, v);
    g.write::<u32>(l + 0x40, 0);
    let mut st = Pushes::default();
    let src = g.read::<u32>(r + 0x1c).wrapping_add(0x9c);
    st.push_all(&[src, w(0xdb), x]);
    st.call(g, slot_target(g, w(0xe1), 231), r);
    for at in [0x20u32, 0x24, 0x28, 0x2c] {
        g.write::<u32>(l + at, x);
    }
    let v = g.read::<u32>(w(0x105));
    g.write::<u32>(l + 0x34, v);
    let t = slot_target(g, w(0x113), 308);
    g.write::<u32>(l + 0x44, 0);
    st.push_all(&[l, m]);
    let eax = st.call(g, t, r).eax;
    g.write::<u32>(w(0x12c), eax);
    eax
}

/// [`model_draw_node`] (the `rel32`: `__security_check_cookie`).
pub const MODEL_DRAW_NODE: Sig = Sig {
    len: 324,
    hash: 0xadc7_a54f_f4a0_d8f0,
    calls: &[(0x13c, Callee::Any)],
    imports: &[],
};
