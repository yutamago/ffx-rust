//! Shared functions of the shared-7 list (the rows of the shared-4 work
//! list that shared-6 left) without a family module yet. Each doc names
//! the DLL and address it was ported from, its size and how many DLLs
//! carry it; "`G`" is a global of the DLL (an operand of the code).

// The originals' float tests are `comiss`/`ucomiss` with their own
// unordered outcomes: `!(a >= b)` and exact `==` say what they do.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::float_cmp)]
// Long ports follow the original's order of stores and calls.
#![allow(clippy::too_many_lines)]

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Out, Ret, Sig};

/// A handler's port on the frame its thunk hands it.
type Handler<G> = fn(&mut G, &Image, &Frame) -> u32;

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
        let start = if k == 0 {
            start
        } else {
            slot_target(g, w(t434), 434)
        };
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
    /// What runs before `*P = ITEM`.
    pre: SummonPre,
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

/// What a summon frame's variant does before its models are bound.
#[derive(Clone, Copy)]
enum SummonPre {
    None,
    /// The pass byte `G_PASS` goes up (before `*P = ITEM`) and `G_X =
    /// *G_Y` (after it): the operands of the pass, `G_X` and `G_Y`.
    PassAndCopy(u32, u32, u32),
    /// Slot 673 first: its table operand.
    Slot673(u32),
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
    if let SummonPre::Slot673(t) = o.pre {
        slot(g, img, f, t, 673, fr.ecx, &[]);
    }
    let p = g.read::<u32>(w(o.p));
    if let SummonPre::PassAndCopy(pass, _, _) = o.pre {
        let v = g.read::<u8>(w(pass)).wrapping_add(1);
        g.write::<u8>(w(pass), v);
    }
    g.write::<u32>(p, w(o.item));
    if let SummonPre::PassAndCopy(_, x, y) = o.pre {
        let v = g.read::<u32>(w(y));
        g.write::<u32>(w(x), v);
    }
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
        pre: SummonPre::None,
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
        pre: SummonPre::None,
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
    for (table, s, c) in [
        (0x1u32, 783u32, 0x9u32),
        (0x1c, 782, 0x24),
        (0x36, 784, 0x3e),
    ] {
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
/// R+0xc8, R+0x218)` and slot 284 `(R[0x23c], 0, R+0xf8)`; bits 0 and 1,
/// slot 284 `(R[0x23c], 0, R+0x22c)`; bit 0 only, the child `C =
/// R+0x228` (if any) gets its handler with a cursor of its own on the
/// frame (`C`, pass byte `+5`) and slot 284 `(R[0x23c], 0, C+0x1c)`
/// follows. Then slot 284 `(R[0x23c], 1, R+0x1c)`, whose `eax` it
/// returns; not passed, what the pass left.
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
    let model = |g: &G| g.read::<u32>(r + 0x23c);
    let mut st = Pushes::default();
    if bits & 1 == 0 {
        st.push_all(&[r + 0x218, r + 0xc8, r + 0x1c]);
        st.call(g, slot_target(g, w(0xd7), 229), fr.ecx);
        let m = model(g);
        st.push_all(&[r + 0xf8, 0, m]);
        st.call(g, slot_target(g, w(0xeb), 284), fr.ecx);
    } else if bits & 2 != 0 {
        let m = model(g);
        st.push_all(&[r + 0x22c, 0, m]);
        st.call(g, slot_target(g, w(0x66), 284), fr.ecx);
    } else {
        let c = g.read::<u32>(r + 0x228);
        if c != 0 {
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
            st.call(g, slot_target(g, w(0xab), 284), fr.ecx);
        }
    }
    // every way ends with the model's own point (the words before are
    // popped)
    let m = model(g);
    let mut last = Pushes::default();
    last.push_all(&[r + 0x1c, 1, m]);
    last.call(g, slot_target(g, w(0x107), 284), fr.ecx).eax
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
    for (k, (node, call)) in [(0x113u32, 0x118u32), (0x12b, 0x130)]
        .into_iter()
        .enumerate()
    {
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
    let base = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0xc8);
    // the frame's cookie word (`cookie ^ esp`) is the `eax` the pass
    // test loads its byte over
    let cookie = g.read::<u32>(w(0xd)) ^ base;
    let (passed, eax) = ring_pass(g, s, cookie);
    if !passed {
        return eax;
    }
    let r = g.read::<u32>(s);
    let m = g.read::<u32>(r + 0x20);
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

/// A look matrix from two points (internal: `ecx` = the matrix `O`,
/// `edx` and the stack word the two points; a /GS frame, `ebp` = `esp -
/// 4` at the entry). With the frame's vectors `A` (`ebp-0x54`), the up
/// vector `U = (0, 1, 0, 1)` (`ebp-0x44`), `C` (`ebp-0x34`), `D`
/// (`ebp-0x24`) and `X` (`ebp-0x14`): `A` = slot 234 `(A, arg, edx)`,
/// `D` = slot 232 `(D, A)`, `X` = slot 266 `(X, U, D)`, `C` = slot 266
/// `(C, D, X)`, then slot 232 `(X, X)` and `(C, C)`; `O`'s rows are `X`,
/// `C`, `D` (three terms each, the fourth 0) and `(0, 0, 0, 1)`. Each
/// call sees the words the earlier ones left. Returns the last slot's
/// `eax`.
///
/// `magic_0188` 0x10003c20 (332 bytes, `ecx`/`edx`; 2 DLLs).
pub fn look_matrix<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let o = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let at = |d: u32| ebp.wrapping_sub(d);
    let (a, u, c, d, x) = (at(0x54), at(0x44), at(0x34), at(0x24), at(0x14));
    let mut st = Pushes::default();
    let arg = fr.arg(g, 0);
    st.push_all(&[fr.edx, arg, a]);
    for (i, v) in (0u32..).zip([0, 0x3f80_0000, 0, 0x3f80_0000]) {
        g.write::<u32>(u + 4 * i, v);
    }
    st.call(g, slot_target(g, w(0x1b), 234), o);
    st.push_all(&[a, d]);
    st.call(g, slot_target(g, w(0x4e), 232), o);
    st.push_all(&[d, u, x]);
    st.call(g, slot_target(g, w(0x67), 266), o);
    st.push_all(&[x, d, c]);
    st.call(g, slot_target(g, w(0x80), 266), o);
    st.push_all(&[x, x]);
    st.call(g, slot_target(g, w(0x92), 232), o);
    st.push_all(&[c, c]);
    let eax = st.call(g, slot_target(g, w(0xa4), 232), o).eax;
    for (row, v) in [(0u32, x), (0x10, c)] {
        for i in 0..3 {
            let t = g.read::<u32>(v + 4 * i);
            g.write::<u32>(o + row + 4 * i, t);
        }
    }
    let (d0, d1, d2) = (g.read::<u32>(d), g.read::<u32>(d + 4), g.read::<u32>(d + 8));
    g.write::<u32>(o + 0x20, d0);
    g.write::<u32>(o + 0x24, d1);
    g.write::<u32>(o + 0xc, 0);
    g.write::<u32>(o + 0x1c, 0);
    g.write::<u32>(o + 0x28, d2);
    for off in [0x2cu32, 0x30, 0x34, 0x38] {
        g.write::<u32>(o + off, 0);
    }
    g.write::<u32>(o + 0x3c, 0x3f80_0000);
    eax
}

/// [`look_matrix`] (the `rel32`: `__security_check_cookie`).
pub const LOOK_MATRIX: Sig = Sig {
    len: 332,
    hash: 0x3083_01fe_9784_181a,
    calls: &[(0x144, Callee::Any)],
    imports: &[],
};

/// An emitter's landing check and puff (cdecl `(E)`): unless its point
/// `E+0xc` is inside the box (`K1 < x < K2`, `z = E+0x14 > K3`, `y =
/// E+0x10 >= 0`; unordered fails), nothing (the entry's `eax`). Else
/// `E+0x284 |= 2`; with a rate `v = E+0x260` other than 0 the point
/// moves back along `E+0x25c`/`E+0x264` by `1 - (-(y - v) / v)` (`K4`
/// for the 1); `y = 0`; the point goes to `E+0x24c..`, an object
/// `G_OBJ + 0x130 E[0x280]` starts there
/// ([`start_object_at`](crate::s4_misc::start_object_at): scale `K5`,
/// part 2) with `+0xd0..` = the point, `E+0x26c = 0`, and it jumps to
/// the smoke puff ([`smoke_puff`](crate::s6_misc::smoke_puff), `ecx` =
/// `E`), whose `eax` it returns.
///
/// `magic_0285` 0x10004cc0 (339 bytes; 2 DLLs).
pub fn landing_puff<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let e = fr.arg(g, 0);
    let x = g.read::<f32>(e + 0xc);
    if !(x > k(g, 0xf) && k(g, 0x1d) > x) {
        return Ret::eax(fr.eax);
    }
    if !(g.read::<f32>(e + 0x14) > k(g, 0x32)) {
        return Ret::eax(fr.eax);
    }
    let y = g.read::<f32>(e + 0x10);
    if !(y >= 0.0) {
        return Ret::eax(fr.eax);
    }
    let fl = g.read::<u16>(e + 0x284) | 2;
    g.write::<u16>(e + 0x284, fl);
    let v = g.read::<f32>(e + 0x260);
    // `ucomiss`/`lahf`/`jnp`: only an ordered 0 skips the move
    if v != 0.0 {
        let neg = f32::from_bits(div(sub(y, v), v).to_bits() ^ g.read::<u32>(w(0x81)));
        let back = sub(k(g, 0x6e), neg);
        let dx = mul(g.read::<f32>(e + 0x25c), back);
        let nx = sub(g.read::<f32>(e + 0xc), dx);
        g.write::<f32>(e + 0xc, nx);
        let dz = mul(g.read::<f32>(e + 0x264), back);
        let nz = sub(g.read::<f32>(e + 0x14), dz);
        g.write::<f32>(e + 0x14, nz);
    }
    g.write::<u32>(e + 0x10, 0);
    for off in [0u32, 4, 8, 0xc] {
        let p = g.read::<u32>(e + 0xc + off);
        g.write::<u32>(e + 0x24c + off, p);
    }
    let obj = g
        .read::<u32>(e + 0x280)
        .wrapping_mul(0x130)
        .wrapping_add(w(0xfa));
    let start = img.rel32_target(f + 0x105);
    crate::s4_misc::start_object_at(g, img, start, obj, k(g, 0xc3), 2, 0, 0);
    for off in [0u32, 4, 8, 0xc] {
        let p = g.read::<u32>(e + 0x24c + off);
        g.write::<u32>(obj + 0xd0 + off, p);
    }
    g.write::<u32>(e + 0x26c, 0);
    let puff = Frame {
        at: img.rel32_target(f + 0x14c),
        ecx: e,
        ..*fr
    };
    Ret::eax(crate::s6_misc::smoke_puff(g, img, &puff))
}

/// [`landing_puff`] (the `rel32`s:
/// [`start_object_at`](crate::s4_misc::start_object_at) and the tail
/// jump to [`smoke_puff`](crate::s6_misc::smoke_puff)).
pub const LANDING_PUFF: Sig = Sig {
    len: 339,
    hash: 0xdfdd_c19b_b6a7_598d,
    calls: &[
        (0x105, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x14c, Callee::Sig(&crate::s6_misc::SMOKE_PUFF)),
    ],
    imports: &[],
};

/// A ring node's handler that moves a light (internal: `ecx` = the
/// cursor `S`; a ring pass first, [`ring_pass`], over the frame's cookie
/// word; a /GS frame, `ebp` = `esp - 4` at the entry). Passed, with `R =
/// S[0]`: with no steps left (`R+0x54`), a node whose bit 0 of `R+0x58`
/// is set leaves its parent ([`leave`](crate::sa_misc::leave)). Else the
/// point `P = R+0x1c` moves by `R+0x3c.. * R+0x50` (four terms), the
/// clock `R+0x4c += R+0x50`; at `K` or past it slot 325 `(P, R+0x2c)`,
/// one step less and the clock 0. Then slot 915 `(P.x / K2, P.y / K2,
/// P.z / K2, R+0x28 K3, slot 776 (u16 R+0x5a))`, whose `eax` it
/// returns.
///
/// `magic_0155` 0x10004720 (354 bytes; 4 DLLs).
pub fn moving_light_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = fr.ecx;
    let cookie = g.read::<u32>(w(0x7)) ^ fr.esp.wrapping_sub(4);
    let (passed, eax) = ring_pass(g, s, cookie);
    if !passed {
        return eax;
    }
    let r = g.read::<u32>(s);
    if g.read::<u32>(r + 0x54) == 0 {
        if g.read::<u8>(r + 0x58) & 1 != 0 {
            return crate::sa_misc::leave(g, r, eax);
        }
        return eax;
    }
    let p = r + 0x1c;
    for i in 0..4u32 {
        let d = mul(g.read::<f32>(p + 0x20 + 4 * i), g.read::<f32>(r + 0x50));
        let v = add(d, g.read::<f32>(p + 4 * i));
        g.write::<f32>(p + 4 * i, v);
    }
    let clock = add(g.read::<f32>(r + 0x50), g.read::<f32>(r + 0x4c));
    g.write::<f32>(r + 0x4c, clock);
    // `comiss`/`jb`: below `K` or unordered goes on
    if clock >= k(g, 0x9a) {
        slot(g, img, f, 0xaa, 325, p + 0x10, &[p, r + 0x2c]);
        let n = g.read::<u32>(r + 0x54).wrapping_sub(1);
        g.write::<u32>(r + 0x54, n);
        g.write::<u32>(r + 0x4c, 0);
    }
    let id = u32::from(g.read::<u16>(r + 0x5a));
    let id = slot(g, img, f, 0xca, 776, fr.ecx, &[id]).eax;
    let by = k(g, 0xe7);
    let z = mul(g.read::<f32>(r + 0x28), k(g, 0xdf)).to_bits();
    let c = div(g.read::<f32>(r + 0x24), by).to_bits();
    let b = div(g.read::<f32>(r + 0x20), by).to_bits();
    let a = div(g.read::<f32>(p), by).to_bits();
    slot(g, img, f, 0xf3, 915, fr.ecx, &[a, b, c, z, id]).eax
}

/// [`moving_light_node`] (the `rel32`s: [`leave`](crate::sa_misc::leave)
/// and two `__security_check_cookie`s).
pub const MOVING_LIGHT_NODE: Sig = Sig {
    len: 354,
    hash: 0x22b9_764e_30e8_31c1,
    calls: &[
        (0x13c, Callee::Any),
        (0x14d, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x15a, Callee::Any),
    ],
    imports: &[],
};

/// A summon's scripted frame (cdecl `(node)`; `B = *G_B`, its state
/// `B+0xeb70`; the offsets are `magic_0644`'s, `magic_0565`'s block sits
/// 0x130 further, an operand). First the record `REC` (0x30 bytes) is cleared, its u16
/// `+2` gets bit 7, `*G_1 = *G_2 = 0` and `*G_3 = *G_4`. Then by the
/// state (past 3: nothing): 0 starts the script `SCRIPT` (`*G_PC`, the
/// wait `*G_W` 0, u16 `*G_X` 0, the mode byte `*G_M` 1) and goes to 1;
/// 1 reads the pads and runs the script ([`read_pads`], [`run`]); when it
/// ends, slot 647 `(*G_Q)` and the records of the four kinds (`magic_0644`:
/// [`kind_records`]; `magic_0565`: [`selection_groups`]) and state 2; 2
/// starts the script of the next record `B + 0xeac4 + 0x28 B[0xeb6c]`
/// (kept in `B+0xeb64`; `*G_PC` = the DLL's table at its first word),
/// counts it and goes to 3, then runs it as 3 does; 3 runs the script,
/// and when it ends goes back to 2 while records are left (`B+0xeb6c <
/// B+0xeb68`), else `B+0xeb74 = 1`, the closing slots
/// ([`call_628_213`]) and the node goes back to its pool
/// ([`give_back`]). Every way ends in a jump to the motion step
/// ([`motion_step`]), whose `eax` it returns.
///
/// [`read_pads`]: crate::sb_misc::read_pads
/// [`run`]: crate::sa_script::run
/// [`kind_records`]: crate::sa_misc::kind_records
/// [`selection_groups`]: crate::s6_misc::selection_groups
/// [`call_628_213`]: crate::sb_misc::call_628_213
/// [`give_back`]: crate::sa_pool::give_back
/// [`motion_step`]: crate::s3_motion::motion_step
///
/// `magic_0644` 0x1000b8f0 (367 bytes; 5 DLLs) and `magic_0565`
/// 0x10007a50 (352 bytes; 5 DLLs).
pub fn summon_script_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let call = |o: u32| img.rel32_target(f + o);
    let b = g.read::<u32>(w(0x6));
    g.write::<u32>(w(0x15), 0);
    g.write::<u32>(w(0x1f), 0);
    // memset(REC, 0, 0x30)
    for i in 0..0x30 {
        g.write::<u8>(w(0xf) + i, 0);
    }
    let v = g.read::<u16>(w(0x34)) | 0x80;
    g.write::<u16>(w(0x34), v);
    let x = g.read::<u32>(w(0x39));
    g.write::<u32>(w(0x3e), x);
    // the state word `B + S` (0xeb70 in `magic_0644`, 0xeca0 in `magic_0565`; an
    // operand), the other words of the block at fixed distances from it
    let st = w(0x44);
    let at = |d: u32| b.wrapping_add(st).wrapping_sub(d);
    let state = at(0);
    // a script started: its pc, the wait, the u16 and the mode byte
    let start = |g: &mut G, pc: u32, (o_pc, o_w, o_x, o_m): (u32, u32, u32, u32)| {
        g.write::<u32>(w(o_pc), pc);
        g.write::<u32>(w(o_w), 0);
        g.write::<u16>(w(o_x), 0);
        g.write::<u8>(w(o_m), 1);
    };
    let script = |g: &mut G, o: u32| crate::sa_script::run(g, img, call(o));
    let now = g.read::<u32>(state);
    if now == 0 {
        start(g, w(0x5d), (0x59, 0x63, 0x6e, 0x76));
        g.write::<u32>(state, 1);
    } else if now == 1 {
        crate::sb_misc::read_pads(g, img, call(0x8d));
        if script(g, 0x92) == 1 {
            let q = g.read::<u32>(w(0xa2));
            slot(g, img, f, 0x9c, 647, fr.ecx, &[q]);
            let records = call(0xb2);
            // the callee's entry `esp`: two pushes and the call's
            let esp = fr.esp.wrapping_sub(0xc);
            if img.matches(records, &crate::sa_misc::KIND_RECORDS) {
                crate::sa_misc::kind_records(g, img, records, esp);
            } else {
                let groups = Frame {
                    at: records,
                    esp,
                    ..*fr
                };
                crate::s6_misc::selection_groups(g, img, &groups);
            }
            g.write::<u32>(state, 2);
        }
    } else if now == 2 || now == 3 {
        if now == 2 {
            let n = g.read::<u32>(at(4));
            let rec = at(0xac).wrapping_add(n.wrapping_mul(0x28));
            g.write::<u32>(at(0xc), rec);
            let first = g.read::<u32>(rec);
            let pc = g.read::<u32>(first.wrapping_mul(4).wrapping_add(w(0xee)));
            start(g, pc, (0xf3, 0xe3, 0xfa, 0x102));
            let n = g.read::<u32>(at(4)).wrapping_add(1);
            g.write::<u32>(at(4), n);
            g.write::<u32>(state, 3);
        }
        if script(g, 0x118) == 1 {
            if g.read::<i32>(at(4)) < g.read::<i32>(at(8)) {
                g.write::<u32>(state, 2);
            } else {
                g.write::<u32>(at(0).wrapping_add(4), 1);
                crate::sb_misc::call_628_213(g, img, call(0x13a));
                let node = fr.arg(g, 0);
                crate::sa_pool::give_back(g, img, call(0x142), node);
            }
        }
    }
    Ret::eax(crate::s3_motion::motion_step(g, img, call(0x88)))
}

/// [`summon_script_frame`] in `magic_0644` (the `rel32`s: `memset`, the
/// pads, the script (twice), the kind records, the closing slots, the
/// pool's give-back and the motion step's tail jumps). Its 367 bytes
/// run on into `__security_check_cookie` (no padding between them), whose
/// tail jump is the last `rel32`.
pub const SUMMON_SCRIPT_FRAME: Sig = Sig {
    len: 367,
    hash: 0x2ecb_93ce_9a75_d25d,
    calls: &[
        (0x28, Callee::Import("memset")),
        (0x88, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x8d, Callee::Sig(&crate::sb_misc::READ_PADS)),
        (0x92, Callee::Sig(&crate::sa_script::RUN)),
        (0xb2, Callee::Sig(&crate::sa_misc::KIND_RECORDS)),
        (0xc3, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x118, Callee::Sig(&crate::sa_script::RUN)),
        (0x13a, Callee::Sig(&crate::sb_misc::CALL_628_213)),
        (0x142, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x149, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x16b, Callee::Any),
    ],
    imports: &[],
};

/// [`summon_script_frame`] in `magic_0565`, with the selection groups
/// ([`selection_groups`](crate::s6_misc::selection_groups)) for the kind
/// records.
pub const SUMMON_SCRIPT_FRAME_B: Sig = Sig {
    len: 352,
    hash: 0x8cbb_d52c_255b_702a,
    calls: &[
        (0x28, Callee::Import("memset")),
        (0x88, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x8d, Callee::Sig(&crate::sb_misc::READ_PADS)),
        (0x92, Callee::Sig(&crate::sa_script::RUN)),
        (0xb2, Callee::Sig(&crate::s6_misc::SELECTION_GROUPS)),
        (0xc3, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x118, Callee::Sig(&crate::sa_script::RUN)),
        (0x13a, Callee::Sig(&crate::sb_misc::CALL_628_213)),
        (0x142, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x149, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
    ],
    imports: &[],
};

/// The 12-byte `mov ecx, [ebp + 8]; jmp X` thunks (the node tables'
/// entries, `crate::s3_wrap`) over this list's handlers: `X` runs with
/// the cursor in `ecx` and the caller's stack. A target that is none of
/// them cannot bind (each thunk's `Sig` names its callee).
pub fn ecx_jmp<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ecx = fr.arg(g, 0);
    let target = img.rel32_target(fr.at + 8);
    let fr = Frame {
        at: target,
        ecx,
        ..*fr
    };
    let handlers: [(&Sig, Handler<G>); 10] = [
        (&MOVING_LIGHT_NODE, moving_light_node),
        (&COUNT_FADE, count_fade),
        (&MODEL_DRAW_NODE, model_draw_node),
        (&LIGHT_NODE, light_node),
        (&HANDLES_REGISTERED, handles_registered),
        (&MODEL_POINTS_NODE, model_points_node),
        (&RING_KEY_FRAME, ring_key_frame),
        (&THUNDER_NODE, thunder_node),
        (&SPRITE_FADE_NODE, sprite_fade_node),
        (&SHAKE_NODE, shake_node),
    ];
    for (sig, port) in handlers {
        if img.matches(target, sig) {
            return Ret::eax(port(g, img, &fr));
        }
    }
    Ret::eax(fr.eax)
}

/// `magic_0155` 0x10004890 (4 DLLs) over [`moving_light_node`].
pub const JMP_MOVING_LIGHT_NODE: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&MOVING_LIGHT_NODE))],
    imports: &[],
};

/// `magic_0384` 0x10001f10 (3 DLLs) over [`count_fade`].
pub const JMP_COUNT_FADE: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&COUNT_FADE))],
    imports: &[],
};

/// `magic_0478` 0x10004050 (3 DLLs) over [`model_draw_node`].
pub const JMP_MODEL_DRAW_NODE: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&MODEL_DRAW_NODE))],
    imports: &[],
};

/// `magic_0150` 0x10001140 (2 DLLs) over [`light_node`].
pub const JMP_LIGHT_NODE: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&LIGHT_NODE))],
    imports: &[],
};

/// `magic_0150` 0x10006e60 (2 DLLs) over [`handles_registered`].
pub const JMP_HANDLES_REGISTERED: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&HANDLES_REGISTERED))],
    imports: &[],
};

/// `magic_0150` 0x10002e60 (2 DLLs) over [`model_points_node`].
pub const JMP_MODEL_POINTS_NODE: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&MODEL_POINTS_NODE))],
    imports: &[],
};

/// `magic_0578` 0x10006780 (2 DLLs) over [`ring_key_frame`].
pub const JMP_RING_KEY_FRAME: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&RING_KEY_FRAME))],
    imports: &[],
};

/// The camera settings put after slot 267 `(4)` (cdecl, no arguments):
/// [`camera_settings`]. Returns 1.
///
/// `magic_0098` 0x100020d0 (29 bytes; 4 DLLs).
pub fn camera_settings_4<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    slot(g, img, f, 0x1, 267, fr.ecx, &[4]);
    let cam = Frame {
        at: img.rel32_target(f + 0x13),
        esp: fr.esp.wrapping_sub(4),
        ..*fr
    };
    camera_settings(g, img, &cam);
    Ret::eax(1)
}

/// [`camera_settings_4`] (the `rel32`: [`camera_settings`]).
pub const CAMERA_SETTINGS_4: Sig = Sig {
    len: 29,
    hash: 0xec46_de87_64d6_4017,
    calls: &[(0x13, Callee::Sig(&CAMERA_SETTINGS))],
    imports: &[],
};

/// A fading level and the scaled matrices (cdecl, no arguments): nothing
/// (1) unless `*G_ON`. The level `V = *G_V` goes down by `K` while slot
/// 253 says so, kept in `0..=K2` (stored when it moved or was clamped;
/// unordered stays), slot 38 `(*G_S, V, 0)`, the matrices
/// ([`scaled_matrices`]); a level now at `K3` (ordered) sets `*G_F` and
/// returns 0, else 1.
///
/// `magic_0443` 0x100019d0 (161 bytes; 2 DLLs).
pub fn fading_level<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    if g.read::<u32>(w(0x2)) == 0 {
        return Ret::eax(1);
    }
    let down = slot(g, img, f, 0xe, 253, fr.ecx, &[]).eax;
    let mut v = k(g, 0x1e);
    if down != 0 {
        v = sub(v, k(g, 0x2a));
        g.write::<f32>(w(0x32), v);
    }
    // `comiss 0, v; ja`, then `comiss v, K2; jbe`
    if 0.0 > v {
        v = 0.0;
        g.write::<f32>(w(0x52), v);
    } else if v > k(g, 0x42) {
        v = k(g, 0x42);
        g.write::<f32>(w(0x52), v);
    }
    let s = g.read::<u32>(w(0x6b));
    slot(g, img, f, 0x57, 38, fr.ecx, &[s, v.to_bits(), 0]);
    let m = Frame {
        at: img.rel32_target(f + 0x75),
        esp: fr.esp.wrapping_sub(4),
        ..*fr
    };
    scaled_matrices(g, img, &m);
    // `ucomiss`/`lahf`/`jp`: only an ordered equality sets the flag
    if k(g, 0x7d) == k(g, 0x84) {
        g.write::<u32>(w(0x90), 1);
        return Ret::eax(0);
    }
    Ret::eax(1)
}

/// [`fading_level`] (the `rel32`: [`scaled_matrices`]).
pub const FADING_LEVEL: Sig = Sig {
    len: 161,
    hash: 0xc695_01e0_bbaa_a2bd,
    calls: &[(0x75, Callee::Sig(&SCALED_MATRICES))],
    imports: &[],
};

/// The near points marked, then the part's count and fade (cdecl `(O)`;
/// a /GS frame, `ebp` = `esp - 4` at the entry). For each of the `*G_N`
/// points `P_i` (16 bytes from `T - 8`, `T` an immediate): the frame's
/// vector `V = P_i - (O+0xd0, 0, O+0xd8, O+0xdc)` (`ebp-0x14`), its
/// length slot 453 `(V)` (ST0, as `f32`), and below `K` the flag
/// `F[i] = 1`. Then [`count_fade`] with `ecx` = `O`, whose `eax` it
/// returns.
///
/// `magic_0384` 0x10001e10 (244 bytes; 3 DLLs).
pub fn near_points_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let o = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    let (x, z, q) = (
        g.read::<f32>(o + 0xd0),
        g.read::<f32>(o + 0xd8),
        g.read::<f32>(o + 0xdc),
    );
    let mut i = 0i32;
    let mut p = w(0x4c);
    while i < g.read::<i32>(w(if i == 0 { 0x40 } else { 0xd2 })) {
        let t = slot_target(g, w(0x61), 453);
        g.write::<f32>(v, sub(g.read::<f32>(p.wrapping_sub(8)), x));
        g.write::<f32>(v + 4, sub(g.read::<f32>(p.wrapping_sub(4)), 0.0));
        g.write::<f32>(v + 8, sub(g.read::<f32>(p), z));
        g.write::<f32>(v + 0xc, sub(g.read::<f32>(p.wrapping_add(4)), q));
        let len = crate::m0018::st0_f32(g.call(t, fr.ecx, &[v]).st0);
        // `comiss K, len; jbe`: below `K` (ordered) marks it
        if g.read::<f32>(w(0xa2)) > len {
            let at = i.cast_unsigned().wrapping_mul(4).wrapping_add(w(0xb2));
            g.write::<u32>(at, 1);
        }
        i += 1;
        p = p.wrapping_add(0x10);
    }
    let fade = Frame {
        at: img.rel32_target(f + 0xe0),
        ecx: o,
        ..*fr
    };
    Ret::eax(count_fade(g, img, &fade))
}

/// [`near_points_fade`] (the `rel32`s: [`count_fade`] and
/// `__security_check_cookie`).
pub const NEAR_POINTS_FADE: Sig = Sig {
    len: 244,
    hash: 0x85f4_90f5_4dae_1029,
    calls: &[(0xe0, Callee::Sig(&COUNT_FADE)), (0xec, Callee::Any)],
    imports: &[],
};

/// A spark record started at a point (internal: `ecx` = the point `P`):
/// nothing while bit 0 or 1 of `**G_F` is set (`*G_F` returned); else a
/// record `R` from the pool ([`pool_take`](crate::sb_misc::pool_take);
/// none: 0) gets u16 `+0x38 = -1`, `+0x3a = 0x200`, `+0x30 = *G_C +
/// 0x28`'s word, `+0x34 = 1`, the position `(P.x + r(2), P.y, P.z +
/// r(2), 1)` (`r` slot 314's random float, ST0), `+0x10 = r(0.1)`,
/// `+0x14 = slot 404 (0.05) + K1`, `+0x18 = r(0.1)`, `+0x1c = r(pi)`, the
/// step `(0, -0.01, 0, 1)`, `+0x4c = +0x50 = 0x20`, bytes `+0x67 = 0xa0`
/// and `+0x6b = 0`, and `+0x3c = (r(0.2) + K2) K3`. Returns the last
/// slot's `eax`.
///
/// `magic_0644` 0x100078c0 (378 bytes; 5 DLLs).
pub fn spark_at_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let p = fr.ecx;
    let flags = g.read::<u32>(w(0x5));
    if g.read::<u8>(flags) & 3 != 0 {
        return flags;
    }
    let r = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0x17));
    if r == 0 {
        return 0;
    }
    let c = g.read::<u32>(g.read::<u32>(w(0x27)).wrapping_add(0x28));
    g.write::<u16>(r + 0x38, 0xffff);
    g.write::<u16>(r + 0x3a, 0x200);
    g.write::<u32>(r + 0x30, c);
    g.write::<u32>(r + 0x34, 1);
    // slot 314 or 404 with one float word: its ST0 as `f32`, and its `eax`
    let rnd = |g: &mut G, t: u32, s: u32, x: f32| {
        let o = slot(g, img, f, t, s, c, &[x.to_bits()]);
        (crate::m0018::st0_f32(o.st0), o.eax)
    };
    let (a, _) = rnd(g, 0x3f, 314, 2.0);
    let x = add(g.read::<f32>(p), a);
    g.write::<f32>(r, x);
    let y = g.read::<u32>(p + 4);
    g.write::<u32>(r + 4, y);
    let (b, _) = rnd(g, 0x7b, 314, 2.0);
    let z = add(g.read::<f32>(p + 8), b);
    g.write::<u32>(r + 0xc, 0x3f80_0000);
    g.write::<f32>(r + 8, z);
    let (v, _) = rnd(g, 0x95, 314, 0.1);
    g.write::<f32>(r + 0x10, v);
    let (v, _) = rnd(g, 0xb8, 404, 0.05);
    g.write::<f32>(r + 0x14, add(v, k(g, 0xd7)));
    let (v, _) = rnd(g, 0xdc, 314, 0.1);
    g.write::<f32>(r + 0x18, v);
    let (v, _) = rnd(g, 0xf8, 314, core::f32::consts::PI);
    g.write::<f32>(r + 0x1c, v);
    for (o, v) in [
        (0x20u32, 0u32),
        (0x24, 0xbc23_d70a),
        (0x28, 0),
        (0x2c, 0x3f80_0000),
    ] {
        g.write::<u32>(r + o, v);
    }
    let (v, eax) = rnd(g, 0x10f, 314, 0.2);
    let v = add(v, k(g, 0x14a));
    g.write::<u32>(r + 0x4c, 0x20);
    g.write::<u32>(r + 0x50, 0x20);
    g.write::<u8>(r + 0x67, 0xa0);
    let v = mul(v, k(g, 0x167));
    g.write::<u8>(r + 0x6b, 0);
    g.write::<f32>(r + 0x3c, v);
    eax
}

/// [`spark_at_point`] (the `rel32`: the pool's take).
pub const SPARK_AT_POINT: Sig = Sig {
    len: 378,
    hash: 0x29b6_4b54_3365_c05a,
    calls: &[(0x17, Callee::Sig(&crate::sb_misc::POOL_TAKE))],
    imports: &[],
};

/// A ring node taken out of its ring (`prev.next = next`, `next.prev =
/// prev`, each read again after the store before) and closed on itself.
/// Returns what the original leaves in `eax`: the `prev` read last.
fn unring<G: Game>(g: &mut G, node: u32) -> u32 {
    let prev = g.read::<u32>(node + 4);
    let next = g.read::<u32>(node + 8);
    g.write::<u32>(prev.wrapping_add(8), next);
    let next = g.read::<u32>(node + 8);
    let prev = g.read::<u32>(node + 4);
    g.write::<u32>(next.wrapping_add(4), prev);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    prev
}

/// A two-stage fade task (cdecl `(task)`, `task[0]` its ring node,
/// `O = task[1]`): the clock `O+0x2c4 += K` and the stage u16 `O+0x2c8`.
/// Stage 0: once the clock is at `K0` (ordered), slot 239 `(*G_S, -5,
/// 0)` and the next stage. Stage 1: once it is at `K1`, `*G_A = 0`, eight
/// targets `*D_i = K2 - *X_i` (and one `*X - *Y`), seven words 1.0, one
/// 0.05, `*G_F = (*G_F & !1) | 4`, and the task's node leaves its ring.
/// Other stages: nothing. Returns what the original leaves in `eax`
/// (slot 239's, the ring's `prev`, else the stage minus 1 or 0).
///
/// `magic_0083` 0x100066c0 (413 bytes; 5 DLLs; `K1` and `K2` the same
/// constant) and `magic_0091` 0x100066c0 (417 bytes; 3 DLLs; two).
pub fn two_stage_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // `movss xmm1, [K]; comiss xmm0, xmm1` (0083) or `comiss xmm0, [K1];
    // movss xmm1, [K2]` (0091): the code after it sits 4 bytes later
    let two = img.word(f + 0x39) & 0xff == 0x0f;
    let d = if two { 4 } else { 0 };
    let w = |o: u32| op(img, f, o);
    let wd = |o: u32| op(img, f, o + d);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 4);
    let clock = add(g.read::<f32>(o + 0x2c4), k(g, 0x16));
    let stage = g.read::<u16>(o + 0x2c8);
    g.write::<f32>(o + 0x2c4, clock);
    match stage {
        0 => {
            // `comiss`/`jb`: below or unordered waits
            if !(clock >= g.read::<f32>(wd(0x173))) {
                return Ret::eax(0);
            }
            let s = g.read::<u32>(wd(0x18a));
            let r = slot(
                g,
                img,
                f,
                0x17a + d,
                239,
                fr.ecx,
                &[s, (-5i32).cast_unsigned(), 0],
            );
            let n = g.read::<u16>(o + 0x2c8).wrapping_add(1);
            g.write::<u16>(o + 0x2c8, n);
            Ret::eax(r.eax)
        }
        1 => {
            // the compare's constant: `movss`'s operand (0083) or
            // `comiss`'s (0091)
            if !(clock >= k(g, if two { 0x3c } else { 0x3d })) {
                return Ret::eax(0);
            }
            let k2 = if two { k(g, 0x4a) } else { k(g, 0x3d) };
            let diff = |g: &G, x: u32| sub(k2, g.read::<f32>(wd(x)));
            let x = diff(g, 0x51);
            g.write::<u32>(wd(0x56), 0);
            let flags = g.read::<u32>(wd(0x5b));
            for at in [0x61u32, 0x6b] {
                g.write::<u32>(wd(at), 0x3f80_0000);
            }
            g.write::<f32>(wd(0x77), x);
            let x = diff(g, 0x82);
            for at in [0x88u32, 0x92, 0x9c, 0xa6] {
                g.write::<u32>(wd(at), 0x3f80_0000);
            }
            g.write::<f32>(wd(0xb2), x);
            let x = diff(g, 0xbd);
            g.write::<u32>(wd(0xc3), 0x3f80_0000);
            g.write::<u32>(wd(0xcd), wd(0xd1));
            g.write::<u32>(wd(0xd7), 0x3f80_0000);
            g.write::<f32>(wd(0xe6), x);
            let x = diff(g, 0xf1);
            g.write::<u32>(wd(0xf9), (flags & !1) | 4);
            g.write::<f32>(wd(0x101), x);
            let x = diff(g, 0x10c);
            g.write::<f32>(wd(0x114), x);
            let x = diff(g, 0x11f);
            let y = diff(g, 0x127);
            g.write::<f32>(wd(0x12f), x);
            let z = sub(g.read::<f32>(wd(0x137)), g.read::<f32>(wd(0x13f)));
            g.write::<f32>(wd(0x147), y);
            g.write::<f32>(wd(0x14f), z);
            let node = g.read::<u32>(task);
            Ret::eax(unring(g, node))
        }
        s => Ret::eax(u32::from(s).wrapping_sub(1)),
    }
}

/// [`two_stage_fade`] in `magic_0083`.
pub const TWO_STAGE_FADE: Sig = Sig {
    len: 413,
    hash: 0xef0f_794d_141b_de93,
    calls: &[],
    imports: &[],
};

/// [`two_stage_fade`] in `magic_0091`.
pub const TWO_STAGE_FADE_B: Sig = Sig {
    len: 417,
    hash: 0xd7c2_4f1a_07d1_2e81,
    calls: &[],
    imports: &[],
};

/// A part started at its parent with random scales (cdecl `(O)`; `P` the
/// parent `O + 0x2c + 0x10 O[0xac]`): `r1 = rand()`, `s = r1 / K1 K2`
/// (into the argument's word), `r2 = rand()`, `O+0xbc |= 0x4000`; `O`'s
/// point `+0xd0..` = `P`'s (`w` 0), its `+0xe0..` = `P`'s times `K3` (`w`
/// 0), `+0x100 = +0x104 = 0`, `+0x108 = s`, `+0x10c = 0`, the scale
/// `+0x130..+0x138 = r2 / K1 K4 + K5` (`+0x13c` 0) and the colour
/// `(1.42, 1.11, 0.8, 1)`; then slot 458 `(*G_X, O, 1)`, slot 450 `(O,
/// CB)` and slot 451 `(O, 10)`, whose `eax` it returns.
///
/// `magic_0191` 0x100019a0 (418 bytes; 3 DLLs).
pub fn random_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let o = fr.arg(g, 0);
    let p = g.read::<u32>(counter(g, o));
    let r1 = g.rand();
    let s = mul(div(cvt(r1), k(g, 0x25)), k(g, 0x2d));
    g.write::<f32>(fr.esp + 4, s);
    let r2 = g.rand();
    let v = g.read::<u32>(o + 0xbc) | 0x4000;
    g.write::<u32>(o + 0xbc, v);
    let pf = |g: &G, at: u32| g.read::<f32>(p.wrapping_add(at));
    let by = k(g, 0x91);
    let far = [0xe0u32, 0xe4, 0xe8].map(|at| mul(pf(g, at), by));
    let near = [0xd0u32, 0xd4, 0xd8].map(|at| pf(g, at));
    let scale = add(mul(div(cvt(r2), k(g, 0x71)), k(g, 0x89)), k(g, 0x9e));
    for (at, x) in [0xd0u32, 0xd4, 0xd8].into_iter().zip(near) {
        g.write::<f32>(o + at, x);
    }
    g.write::<u32>(o + 0xdc, 0);
    for (at, x) in [0xe0u32, 0xe4, 0xe8].into_iter().zip(far) {
        g.write::<f32>(o + at, x);
    }
    for at in [0xecu32, 0x100, 0x104] {
        g.write::<u32>(o + at, 0);
    }
    g.write::<f32>(o + 0x108, s);
    g.write::<u32>(o + 0x10c, 0);
    for at in [0x130u32, 0x134, 0x138] {
        g.write::<f32>(o + at, scale);
    }
    g.write::<u32>(o + 0x13c, 0);
    for (at, v) in [
        (0x170u32, 0x3fb5_c28fu32),
        (0x174, 0x3f8e_147b),
        (0x178, 0x3f4c_cccd),
        (0x17c, 0x3f80_0000),
    ] {
        g.write::<u32>(o + at, v);
    }
    let mut st = Pushes::default();
    let x = g.read::<u32>(w(0xb8));
    st.push_all(&[1, o, x]);
    st.call(g, slot_target(g, w(0x96), 458), fr.ecx);
    st.push_all(&[w(0x17e), o]);
    st.call(g, slot_target(g, w(0x179), 450), fr.ecx);
    st.push_all(&[10, o]);
    Ret::eax(st.call(g, slot_target(g, w(0x18c), 451), fr.ecx).eax)
}

/// [`random_part`] (the imports: `rand`, twice).
pub const RANDOM_PART: Sig = Sig {
    len: 418,
    hash: 0x2a1e_ab84_9e89_eae8,
    calls: &[],
    imports: &[(0x16, "rand"), (0x38, "rand")],
};

/// A stream's next block sent (internal: `ecx` = the state `S` (u16 `+0`
/// the block, u16 `+2` the bytes in it), `edx` = the table `T` (i16
/// `+6` the count; entry `i` at `T + 8 (i + 2)`: i16 `+0` its data's
/// offset from `T`, u16 `+2` its length), the stack word `A`; a
/// frame, `ebp` = `esp - 4` at the entry, the local `L = T` at
/// `ebp-0x10`). `S` is a 64-bit handle `(S, sign)`; every slot call runs
/// with `**G_LOCK` 1 and puts the word back after (except the second of
/// a pair). Past the last block: slot 779 `(S, sign)` and 0. Else with
/// the block's data `E`: slot 773 `(S, sign)` says whether it is open;
/// open, slot 774 `(S, sign, &L, A, block)` and slot 775 `(S, sign)`;
/// not, the rate byte `E + E[1] + 4` (0: 0x48) through slot 776 and
/// slot 777 `(A+0x1c)` give flags `F` and slot 778 `(S, sign, &L, A,
/// block, F)`. Then `S+2 += 0x100`; at the block's length or past it the
/// block goes up and `S+2` keeps the rest, and past the count the state
/// is cleared. Returns 1.
///
/// `magic_0150` 0x10003f60 (422 bytes, `ecx`/`edx`; 5 DLLs).
pub fn stream_block<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let (s, t) = (fr.ecx, fr.edx);
    let a = fr.arg(g, 0);
    // `cdq`: the state's address as a signed 64-bit handle
    let hi = if s.cast_signed() < 0 { u32::MAX } else { 0 };
    let ebp = fr.esp.wrapping_sub(4);
    let local = ebp.wrapping_sub(0x10);
    let count = i32::from(g.read::<i16>(t + 6));
    let block = g.read::<u16>(s);
    let lock = |g: &G, o: u32| g.read::<u32>(w(o));
    // a slot call with the lock word 1, the old word returned
    let locked = |g: &mut G, o: u32| {
        let l = lock(g, o);
        let saved = g.read::<u32>(l);
        g.write::<u32>(l, 1);
        saved
    };
    if i32::from(block) >= count {
        let saved = locked(g, 0x27);
        let l = lock(g, 0x27);
        slot(g, img, f, 0x2d, 779, l, &[s, hi]);
        let l = lock(g, 0x42);
        g.write::<u32>(l, saved);
        return 0;
    }
    let entry = t.wrapping_add(8 * (u32::from(block) + 2));
    let e = i32::from(g.read::<i16>(entry))
        .cast_unsigned()
        .wrapping_add(t);
    g.write::<u32>(local, t);
    let saved = locked(g, 0x68);
    let open = slot(g, img, f, 0x77, 773, saved, &[s, hi]).eax;
    let l = lock(g, 0x88);
    g.write::<u32>(l, saved);
    if open != 0 {
        let block = u32::from(g.read::<u16>(s));
        g.write::<u32>(l, 1);
        slot(g, img, f, 0xa7, 774, saved, &[s, hi, local, a, block]);
        let l = lock(g, 0xbe);
        g.write::<u32>(l, 1);
        slot(g, img, f, 0xcc, 775, saved, &[s, hi]);
        let l = lock(g, 0xdd);
        g.write::<u32>(l, saved);
    } else {
        let rate = g.read::<u8>(g.read::<u32>(e + 4).wrapping_add(e).wrapping_add(4));
        let rate = if rate == 0 { 0x48 } else { u32::from(rate) };
        let x = slot(g, img, f, 0x100, 776, saved, &[rate]).eax;
        let y = g.read::<u32>(a + 0x1c);
        let flags = x | slot(g, img, f, 0x112, 777, saved, &[y]).eax;
        let block = u32::from(g.read::<u16>(s));
        let saved = locked(g, 0x12a);
        let args = [s, hi, local, a, block, flags];
        slot(g, img, f, 0x149, 778, saved, &args);
        let l = lock(g, 0x159);
        g.write::<u32>(l, saved);
    }
    let n = g.read::<u16>(s + 2).wrapping_add(0x100);
    g.write::<u16>(s + 2, n);
    let len = g.read::<u16>(entry + 2);
    if i32::from(n) >= i32::from(len.cast_signed()) {
        let b = g.read::<u16>(s).wrapping_add(1);
        g.write::<u16>(s, b);
        g.write::<u16>(s + 2, n.wrapping_sub(len));
        if i32::from(b) >= i32::from(g.read::<i16>(t + 6)) {
            g.write::<u32>(s, 0);
        }
    }
    1
}

/// [`stream_block`].
pub const STREAM_BLOCK: Sig = Sig {
    len: 422,
    hash: 0xf565_585c_b593_e295,
    calls: &[],
    imports: &[],
};

/// A summon's loading and fade-in frame (cdecl, no arguments). With `S =
/// *G_S`, the steps each run once (their flag word set after): `*G_A`:
/// slot 225 `(S)` until it is 0; `*G_B`: slot 226 `(S, 1)`; `*G_C`: slot
/// 227 `(S)` until it is at least 1; then while `*G_D` is negative,
/// slot 347 `(S)`'s bit 0 waits, slot 239 `(S, -5, 0)` and slot 220
/// `(*G_S2, 0x40, 0x100)` (with `*G_D = 0`). Then slot 349 `(*G_X,
/// *G_Y)` and, for `D = *G_D` in 1..=15, slot 348 `(*G_X, *G_Y, 0, -q,
/// q, 0)` with `q = 64 D / 15`; for `D` in 16..=45, the same with `q =
/// 64 - 64 (D - 15) / 30`. Returns 0.
///
/// `magic_0701` 0x10001320 (422 bytes; 3 DLLs).
pub fn summon_fade_in<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let v = |g: &G, o: u32| g.read::<u32>(w(o));
    let ecx = fr.ecx;
    if v(g, 0x2) == 0 {
        let s = v(g, 0x10);
        if slot(g, img, f, 0xa, 225, ecx, &[s]).eax != 0 {
            return Ret::eax(0);
        }
        let n = v(g, 0x29).wrapping_add(1);
        g.write::<u32>(w(0x29), n);
    }
    if v(g, 0x2f) == 0 {
        let s = v(g, 0x3f);
        let t = slot_target(g, w(0x37), 226);
        g.write::<u32>(w(0x4b), 1);
        g.call(t, ecx, &[s, 1]);
    }
    if v(g, 0x5a) == 0 {
        let s = v(g, 0x68);
        if slot(g, img, f, 0x62, 227, ecx, &[s]).eax.cast_signed() < 1 {
            return Ret::eax(0);
        }
        let n = v(g, 0x82).wrapping_add(1);
        g.write::<u32>(w(0x82), n);
    }
    if v(g, 0x88).cast_signed() < 0 {
        let s = v(g, 0x96);
        if slot(g, img, f, 0x90, 347, ecx, &[s]).eax & 1 == 0 {
            return Ret::eax(0);
        }
        let s = v(g, 0xbe);
        slot(g, img, f, 0xae, 239, ecx, &[s, (-5i32).cast_unsigned(), 0]);
        let t = slot_target(g, w(0xc5), 220);
        let s2 = v(g, 0xd8);
        g.write::<u32>(w(0xde), 0);
        g.call(t, ecx, &[s2, 0x40, 0x100]);
    }
    let (y, x) = (v(g, 0xed), v(g, 0xf8));
    slot(g, img, f, 0xf2, 349, ecx, &[x, y]);
    let d = v(g, 0x106).cast_signed();
    if (1..=15).contains(&d) {
        let q = (d << 6) / 15;
        let (y, x) = (v(g, 0x13a), v(g, 0x146));
        let args = [x, y, 0, (-q).cast_unsigned(), q.cast_unsigned(), 0];
        slot(g, img, f, 0x132, 348, ecx, &args);
    }
    let e = v(g, 0x14e).cast_signed().wrapping_sub(15);
    if (1..=30).contains(&e) {
        let c = 0x40 - (e << 6) / 30;
        let (y, x) = (v(g, 0x193), v(g, 0x199));
        let args = [x, y, 0, (-c).cast_unsigned(), c.cast_unsigned(), 0];
        slot(g, img, f, 0x181, 348, ecx, &args);
    }
    Ret::eax(0)
}

/// [`summon_fade_in`].
pub const SUMMON_FADE_IN: Sig = Sig {
    len: 422,
    hash: 0xe1a8_5681_4004_5fe7,
    calls: &[],
    imports: &[],
};

/// A ring node's handler that fades a sprite in and out (internal: `ecx` =
/// the cursor `S`; a ring pass first, [`ring_pass`]; `ebp` = `esp - 4` at
/// the entry, the sprite record `O` a local at `ebp-0x54`). Passed, with
/// `R = S[0]` and bit 0 of its byte `R+0x2b` set: three u16 counters
/// give the level `v`: `R+0x26` (fading in: `(R+0x1c - n) / R+0x1c`),
/// else `R+0x24` (held: `K`), else `R+0x28` (fading out: `K - (R+0x20 -
/// n) / R+0x20`), each counting down; none left: bit 0 of `R+0x2b` is
/// cleared, and with bit 1 the node leaves its parent
/// ([`leave`](crate::sa_misc::leave)). The alpha `a = R+0x2a v` (in the
/// frame at `ebp-4`); above `K2`: slot 245 `(ebp-8)`, the sprite `O` =
/// `(0x80, 0x80, 0x80, trunc(a))`, corners 255.5 and 111.5, `+0x24 =
/// 0x44`, the zoom `R+0x2c`, byte `+0x48 = 0` and the buffer `*G_B + 8`,
/// its packet ([`sprite_gs_packet`](crate::s6_misc::sprite_gs_packet)),
/// slot 581 `(*G_B, n, 0x1000, 0)` with `n` the 16-byte words written,
/// and `*G_B` the new buffer end, slot 581's `eax` returned. Else what
/// the original leaves (the counter minus 1, the level byte, the flags
/// byte).
///
/// The packet reads `O+0x20` and `O+0x28`, which the original never
/// writes (junk read: [ebp-0x34] and [ebp-0x2c] never written; 0 (brief
/// 2026-10-24)).
///
/// `magic_0202` 0x100012c0 (423 bytes; 2 DLLs).
pub fn sprite_fade_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = fr.ecx;
    let (passed, eax) = ring_pass(g, s, fr.eax);
    if !passed {
        return eax;
    }
    let r = g.read::<u32>(s);
    let bits = g.read::<u8>(r + 0x2b);
    let eax = (eax & !0xff) | u32::from(bits);
    if bits & 1 == 0 {
        return eax;
    }
    let counter = |g: &mut G, at: u32| {
        let n = g.read::<u16>(r + at);
        if n != 0 {
            g.write::<u16>(r + at, n.wrapping_sub(1));
        }
        n
    };
    let v = {
        let a = counter(g, 0x26);
        if a != 0 {
            let len = g.read::<f32>(r + 0x1c);
            div(sub(len, f32::from(a.cast_signed())), len)
        } else if counter(g, 0x24) != 0 {
            k(g, 0x92)
        } else {
            let c = counter(g, 0x28);
            if c == 0 {
                let bits = bits & !1;
                g.write::<u8>(r + 0x2b, bits);
                let eax = (eax & !0xff) | u32::from(bits);
                if bits & 2 == 0 {
                    return eax;
                }
                return crate::sa_misc::leave(g, r, eax);
            }
            let len = g.read::<f32>(r + 0x20);
            sub(k(g, 0xbf), div(sub(len, f32::from(c.cast_signed())), len))
        }
    };
    let level = g.read::<u8>(r + 0x2a);
    let alpha = mul(cvt(u32::from(level)), v);
    let ebp = fr.esp.wrapping_sub(4);
    g.write::<f32>(ebp.wrapping_sub(4), alpha);
    // `comiss`/`jbe`: at or below `K2` (or unordered) ends it
    if !(alpha > k(g, 0xe9)) {
        return u32::from(level);
    }
    slot(g, img, f, 0xfd, 245, fr.ecx, &[ebp.wrapping_sub(8)]);
    let buf = g.read::<u32>(w(0x10b));
    let o = ebp.wrapping_sub(0x54);
    g.write::<u32>(o + 0x34, buf.wrapping_add(8));
    let alpha = crate::s4_summon::trunc(g.read::<f32>(ebp.wrapping_sub(4))) & 0xff;
    let zoom = g.read::<u32>(r + 0x2c);
    g.write::<u32>(o + 0x24, 0x44);
    g.write::<u8>(o + 0x48, 0);
    g.write::<u32>(o + 0x10, 0x437f_8000);
    g.write::<u32>(o + 0x14, 0x42df_0000);
    for at in [0u32, 4, 8] {
        g.write::<u32>(o + at, 0x80);
    }
    g.write::<u32>(o + 0xc, alpha);
    g.write::<u32>(o + 0x2c, zoom);
    // junk read: [ebp-0x34] never written; 0 (brief 2026-10-24)
    g.write::<u32>(o + 0x20, 0);
    // junk read: [ebp-0x2c] never written; 0 (brief 2026-10-24)
    g.write::<u32>(o + 0x28, 0);
    let packet = Frame {
        at: img.rel32_target(f + 0x15c),
        esp: ebp.wrapping_sub(0x64),
        ecx: o,
        ..*fr
    };
    crate::s6_misc::sprite_gs_packet(g, img, &packet);
    let end = g.read::<u32>(o + 0x34);
    let n = end.wrapping_sub(buf) >> 4;
    let r = slot(g, img, f, 0x173, 581, o, &[buf, n, 0x1000, 0]).eax;
    g.write::<u32>(w(0x185), end);
    r
}

/// [`sprite_fade_node`] (the `rel32`s:
/// [`sprite_gs_packet`](crate::s6_misc::sprite_gs_packet) and
/// [`leave`](crate::sa_misc::leave)).
pub const SPRITE_FADE_NODE: Sig = Sig {
    len: 423,
    hash: 0x8675_917e_9ea9_9368,
    calls: &[
        (0x15c, Callee::Sig(&crate::s6_misc::SPRITE_GS_PACKET)),
        (0x19c, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};

/// A spline's second derivatives for one component (internal: `ecx` the
/// points `P` (16 bytes each, the component at word `c`), `edx` the
/// knots `X`; the stack `(M, H, U, c, n)`: the results `M`, the steps
/// `H` and the work row `U`, `n` the knot count, `m = n - 1`). The
/// tridiagonal system solved as the original does, each step one `f32`
/// operation in its order (`K` the DLL's constant, 2):
/// - `M[0] = M[m] = 0`; for `i < m`: `H[i] = X[i+1] - X[i]`, `U[i+1] =
///   (P[i+1] - P[i]) / H[i]`;
/// - `M[1] = U[2] - U[1]`, `U[1] = (X[2] - X[0]) K`;
/// - for `j` in `1..m-1`: `r = H[j] / U[j]`, `M[j+1] = (U[j+2] - U[j+1])
///   M[j] r`, `U[j+1] = (X[j+2] - X[j]) K - H[j] r`;
/// - `M[m-1] -= H[m-1] M[m]`, and for `k` from `m - 1` down to 1: `M[k] =
///   (M[k] - H[k] M[k+1]) / U[k]`.
///
/// Returns what the original leaves in `eax`: `&U[1]` after the last
/// loop ran, else `&U[2]`.
///
/// `magic_0432` 0x10004ef0 (437 bytes, `ecx`/`edx`; 3 DLLs).
pub fn spline_moments<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let k = g.read::<f32>(op(img, fr.at, 0x9f));
    let (p, x) = (fr.ecx, fr.edx);
    let [m_, h, u, c, n] = [0, 1, 2, 3, 4].map(|i| fr.arg(g, i));
    let at = |base: u32, i: u32| base.wrapping_add(i.wrapping_mul(4));
    let rd = |g: &G, base: u32, i: u32| g.read::<f32>(at(base, i));
    let m = n.wrapping_sub(1);
    g.write::<u32>(m_, 0);
    g.write::<u32>(at(m_, m), 0);
    let comp = p.wrapping_add(c.wrapping_mul(4));
    // the original keeps scratch in its argument word `n` (`ebp+0x18`):
    // `X - U` for the first loop, the second loop's count after
    let n_word = fr.esp + 0x14;
    if m.cast_signed() > 0 {
        g.write::<u32>(n_word, x.wrapping_sub(u));
        for i in 0..m {
            let step = sub(rd(g, x, i + 1), rd(g, x, i));
            g.write::<f32>(at(h, i), step);
            let pi = |i: u32| comp.wrapping_add(i.wrapping_mul(0x10));
            let d = sub(g.read::<f32>(pi(i + 1)), g.read::<f32>(pi(i)));
            g.write::<f32>(at(u, i + 1), div(d, step));
        }
    }
    let v = sub(rd(g, u, 2), rd(g, u, 1));
    g.write::<f32>(at(m_, 1), v);
    let v = mul(sub(rd(g, x, 2), rd(g, x, 0)), k);
    g.write::<f32>(at(u, 1), v);
    let last = m.wrapping_sub(1);
    if last.cast_signed() > 1 {
        // counted down to 0
        g.write::<u32>(n_word, 0);
        for j in 1..last {
            let r = div(rd(g, h, j), rd(g, u, j));
            let a = sub(rd(g, u, j + 2), rd(g, u, j + 1));
            let v = sub(a, mul(rd(g, m_, j), r));
            g.write::<f32>(at(m_, j + 1), v);
            let a = mul(sub(rd(g, x, j + 2), rd(g, x, j)), k);
            let v = sub(a, mul(rd(g, h, j), r));
            g.write::<f32>(at(u, j + 1), v);
        }
    }
    let v = sub(
        rd(g, m_, m.wrapping_sub(1)),
        mul(rd(g, h, m.wrapping_sub(1)), rd(g, m_, m)),
    );
    g.write::<f32>(at(m_, m.wrapping_sub(1)), v);
    if last.cast_signed() <= 0 {
        return at(u, 2);
    }
    for i in (1..=last).rev() {
        let v = div(
            sub(rd(g, m_, i), mul(rd(g, h, i), rd(g, m_, i + 1))),
            rd(g, u, i),
        );
        g.write::<f32>(at(m_, i), v);
    }
    at(u, 1)
}

/// [`spline_moments`].
pub const SPLINE_MOMENTS: Sig = Sig {
    len: 437,
    hash: 0xa09e_d671_1264_35e8,
    calls: &[],
    imports: &[],
};

/// A spline set up through points (internal: `ecx` the work row `W`,
/// `edx` the points `Q` (16 bytes each); the stack `(T, MX, MY, MZ, n)`;
/// an aligned frame). The knots `T` are the chord lengths summed (`T[0]
/// = 0`, `T[i] = sqrt(dy^2 + dx^2 + dz^2) + T[i-1]`, the squares added
/// in that order, the root in `f64`) and then divided by the total
/// `T[n-1]`; the second derivatives of the three components are
/// [`spline_moments`] `(Q, T; MX/MY/MZ, W, W + 4n, 0/1/2, n)`, the last
/// one's `eax` returned.
///
/// `magic_0432` 0x10004b00 (437 bytes, `ecx`/`edx`; 3 DLLs).
pub fn spline_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let (w, q) = (fr.ecx, fr.edx);
    let t = fr.arg(g, 0);
    let n = fr.arg(g, 4);
    let at = |i: u32| t.wrapping_add(i.wrapping_mul(4));
    g.write::<u32>(t, 0);
    let pt = |i: u32, c: u32| q.wrapping_add(i.wrapping_mul(0x10)).wrapping_add(4 * c);
    let mut i = 1u32;
    while i.cast_signed() < n.cast_signed() {
        let d = |g: &G, c: u32| sub(g.read::<f32>(pt(i, c)), g.read::<f32>(pt(i - 1, c)));
        let (dy, dx, dz) = (d(g, 1), d(g, 0), d(g, 2));
        let s = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
        // `cvtps2pd`, `_libm_sse2_sqrt_precise`, `cvtsd2ss`
        let len = f64::from(s).sqrt() as f32;
        let v = add(len, g.read::<f32>(at(i - 1)));
        g.write::<f32>(at(i), v);
        i += 1;
    }
    let total = g.read::<f32>(at(n.wrapping_sub(1)));
    let mut i = 1u32;
    while i.cast_signed() < n.cast_signed() {
        let v = div(g.read::<f32>(at(i)), total);
        g.write::<f32>(at(i), v);
        i += 1;
    }
    // the three calls' frames: after `push ebp; and esp, -8; sub esp, 0xc`
    // and three saved registers, five argument words each
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x18);
    let u = w.wrapping_add(n.wrapping_mul(4));
    let mut eax = 0;
    for (k, call) in [0x17bu32, 0x191, 0x1a7].into_iter().enumerate() {
        let k = k as u32;
        let m = fr.arg(g, 1 + k);
        let esp = base.wrapping_sub(0x18 + 0x14 * k);
        for (j, v) in (1u32..).zip([m, w, u, k, n]) {
            g.write::<u32>(esp + 4 * j, v);
        }
        let moments = Frame {
            at: img.rel32_target(f + call),
            esp,
            ecx: q,
            edx: t,
            ..*fr
        };
        eax = spline_moments(g, img, &moments);
    }
    eax
}

/// [`spline_setup`] (the `rel32`s: `_libm_sse2_sqrt_precise`'s thunk and
/// three [`spline_moments`]).
pub const SPLINE_SETUP: Sig = Sig {
    len: 437,
    hash: 0xf2d0_056a_849b_2488,
    calls: &[
        (0x75, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x17b, Callee::Sig(&SPLINE_MOMENTS)),
        (0x191, Callee::Sig(&SPLINE_MOMENTS)),
        (0x1a7, Callee::Sig(&SPLINE_MOMENTS)),
    ],
    imports: &[],
};

/// A ring node's handler that shakes the screen (internal: `ecx` = the
/// cursor `S`; a ring pass first, [`ring_pass`]; `ebp` = `esp - 4` at
/// the entry). Passed, with `R = S[0]`: without bit 0 of the byte
/// `R+0x40`, slot 250 `(0, 0)`. Else the timer `t = R+0x2c` goes down by
/// `K`; a timer that was 0 clears bit 0, and with bit 1 the node leaves
/// its parent ([`leave`](crate::sa_misc::leave)) and slot 250 `(0, 0)`
/// follows. Otherwise, with `v = r - K2` (`r` slot 307's ST0) and two
/// random signs `s1`, `s2` (slot 324's bit 1: -1, else 1): the fade `f
/// = R+0x38 - t / R+0x30 R+0x3c` (into `R+0x34`), and slot 250 with
/// `f (R+0x1c + R+0x20 v) s1` and `(R+0x24 + R+0x28 v) s2 f`. Returns
/// slot 250's `eax`, else what the pass or the flags left.
///
/// `magic_0513` 0x10004510 (370 bytes; 2 DLLs).
pub fn shake_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = fr.ecx;
    let (passed, eax) = ring_pass(g, s, fr.eax);
    if !passed {
        return eax;
    }
    let r = g.read::<u32>(s);
    let bits = g.read::<u8>(r + 0x40);
    let shake = |g: &mut G, x: u32, y: u32| slot(g, img, f, 0x15c, 250, s, &[x, y]).eax;
    if bits & 1 == 0 {
        return shake(g, 0, 0);
    }
    let t = g.read::<f32>(r + 0x2c);
    g.write::<f32>(r + 0x2c, sub(t, k(g, 0x6a)));
    // `ucomiss`/`lahf`/`test ah, 0x44`/`jnp`: only an ordered 0 ends it
    if t == 0.0 {
        let bits = bits & !1;
        g.write::<u8>(r + 0x40, bits);
        // `ah`: `lahf`'s flags of the equal compare (ZF and bit 1)
        let eax = (eax & !0xffff) | 0x4200 | u32::from(bits);
        if bits & 2 == 0 {
            return eax;
        }
        crate::sa_misc::leave(g, r, eax);
        return shake(g, 0, 0);
    }
    let rnd = slot(g, img, f, 0x7e, 307, s, &[]);
    let v = sub(crate::m0018::st0_f32(rnd.st0), k(g, 0x96));
    let sign = |e: u32| cvt(1u32.wrapping_sub(e & 2));
    let e1 = slot(g, img, f, 0x9b, 324, s, &[]).eax;
    let a = mul(
        add(mul(g.read::<f32>(r + 0x20), v), g.read::<f32>(r + 0x1c)),
        sign(e1),
    );
    let e2 = slot(g, img, f, 0xc8, 324, s, &[]).eax;
    let ratio = mul(
        div(g.read::<f32>(r + 0x2c), g.read::<f32>(r + 0x30)),
        g.read::<f32>(r + 0x3c),
    );
    let b = add(mul(g.read::<f32>(r + 0x28), v), g.read::<f32>(r + 0x24));
    let fade = sub(g.read::<f32>(r + 0x38), ratio);
    g.write::<f32>(r + 0x34, fade);
    let y = mul(mul(b, sign(e2)), fade);
    let x = mul(fade, a);
    shake(g, x.to_bits(), y.to_bits())
}

/// [`shake_node`] (the `rel32`: [`leave`](crate::sa_misc::leave)).
pub const SHAKE_NODE: Sig = Sig {
    len: 370,
    hash: 0x3666_4bb9_9fb5_4a61,
    calls: &[(0x145, Callee::Sig(&crate::sa_misc::LEAVE))],
    imports: &[],
};

/// [`sprites_step`](crate::s6_misc::sprites_step) on a system in the
/// DLL's globals (cdecl, no arguments; an aligned frame): the count
/// `G_N`, the sprites `*G_P`, gravity `G_A` and its step `G_B`, the cap
/// `G_C` and its limit `G_D` (the operands; one block in the DLLs seen).
///
/// `magic_0206` 0x10002d30 (376 bytes; 4 DLLs).
pub fn sprites_step_global<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let words = crate::s6_misc::SpriteSystem {
        count: w(0x16),
        sprites: w(0x10),
        gravity: w(0x97),
        step: w(0xa7),
        cap: w(0xb2),
        limit: w(0x121),
    };
    Ret::eax(crate::s6_misc::sprites_step_on(g, img, f, fr.eax, &words))
}

/// [`sprites_step_global`] (the `rel32`s: the sprite fade, `memset` and
/// the square root).
pub const SPRITES_STEP_GLOBAL: Sig = Sig {
    len: 376,
    hash: 0xcacc_9289_f665_9304,
    calls: &[
        (0x2c, Callee::Sig(&crate::sa_misc::SPRITE_FADE)),
        (0x83, Callee::Import("memset")),
        (0x103, Callee::Import("_libm_sse2_sqrt_precise")),
    ],
    imports: &[],
};

/// A ring node's handler that draws a thunder strip (internal: `ecx` =
/// the cursor `S`; a ring pass first, [`ring_pass`], over the frame's
/// cookie word; a /GS frame aligned to 16). Passed, with `R = S[0]` and
/// the frame's vector `V`: `V` = slot 233 `(V, R[0x48] + 0xbc, -1.0)`,
/// normalised (slot 232 `(V, V)`), `w` 1.0; with bit 0 of the byte
/// `R+0x54`, slot 677 `(R+0x30, R[0x50], R[0x44] + 0x1c, V)` and the bit
/// cleared; the record `L = (R+0x30, R[0x4c] + 0x1c, R[0x40], R[0x50],
/// R[0x44] + 0x1c, V)` (a local) goes to slot 679 while `*G_OFF` is 0;
/// then slot 680 `(R+0x30, G_T, &(1.0, 1.0867), R[0x40], G_K, 0x48, 0,
/// 0, 0, R+0x1c, R)`, whose `eax` it returns.
///
/// `magic_0387` 0x10007ca0 (406 bytes; 3 DLLs).
pub fn thunder_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let s = fr.ecx;
    let top = fr.esp.wrapping_sub(4) & !0xf;
    // `sub esp, 0x58`, then `push esi; push edi`: the locals from `e`
    let e = top.wrapping_sub(0x60);
    let cookie = g.read::<u32>(w(0xa)) ^ top.wrapping_sub(0x58);
    let (v, pair, rec) = (e + 0x4c, e + 0x18, e + 0x20);
    for (i, x) in (0u32..).zip([0, 0, 0xbf80_0000, 0x3f80_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    g.write::<u32>(pair, 0x3f80_0000);
    g.write::<u32>(pair + 4, 0x3f8a_aaab);
    let r = g.read::<u32>(s);
    g.write::<u32>(e + 0x14, r);
    let (passed, eax) = ring_pass(g, s, cookie);
    if !passed {
        return eax;
    }
    let r = g.read::<u32>(s);
    let src = g.read::<u32>(r + 0x48).wrapping_add(0xbc);
    slot(g, img, f, 0xac, 233, r, &[v, src, 0xbf80_0000]);
    slot(g, img, f, 0xc2, 232, r, &[v, v]);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    if g.read::<u8>(r + 0x54) & 1 != 0 {
        let at = g.read::<u32>(r + 0x44).wrapping_add(0x1c);
        let x = g.read::<u32>(r + 0x50);
        slot(g, img, f, 0xf3, 677, r, &[r + 0x30, x, at, v]);
        let b = g.read::<u8>(r + 0x54) & !1;
        g.write::<u8>(r + 0x54, b);
    }
    let words = [
        r + 0x30,
        g.read::<u32>(r + 0x4c).wrapping_add(0x1c),
        g.read::<u32>(r + 0x40),
        g.read::<u32>(r + 0x50),
        g.read::<u32>(r + 0x44).wrapping_add(0x1c),
        v,
    ];
    for (i, x) in (0u32..).zip(words) {
        g.write::<u32>(rec + 4 * i, x);
    }
    if g.read::<u32>(w(0x12d)) == 0 {
        slot(g, img, f, 0x146, 679, r, &[rec]);
    }
    let x40 = g.read::<u32>(r + 0x40);
    let args = [
        r + 0x30,
        w(0x175),
        pair,
        x40,
        w(0x163),
        0x48,
        0,
        0,
        0,
        r + 0x1c,
        r,
    ];
    // call width: 11 words
    slot(g, img, f, 0x170, 680, r, &args).eax
}

/// [`thunder_node`] (the `rel32`: `__security_check_cookie`).
pub const THUNDER_NODE: Sig = Sig {
    len: 406,
    hash: 0xc0d4_84f1_bd49_7891,
    calls: &[(0x18e, Callee::Any)],
    imports: &[],
};

/// `magic_0387` 0x10007e40 (3 DLLs) over [`thunder_node`].
pub const JMP_THUNDER_NODE: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&THUNDER_NODE))],
    imports: &[],
};

/// `magic_0202` 0x10001480 (2 DLLs) over [`sprite_fade_node`].
pub const JMP_SPRITE_FADE_NODE: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&SPRITE_FADE_NODE))],
    imports: &[],
};

/// `magic_0513` 0x100046c0 (2 DLLs) over [`shake_node`].
pub const JMP_SHAKE_NODE: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&SHAKE_NODE))],
    imports: &[],
};

/// A spline object set up and its sound handle registered (internal:
/// `ecx` = the object `O`, `edx` = the points `P` (16 bytes each); the
/// stack `(n, kind, b, reverse, M, sx, sy, sz, w1, w2, b2)`; an aligned
/// frame). `O+0x80` = the motion's data `D = M + (i16) M+0x10`, bytes
/// `+0x84 = n`, `+0x85 = b2`, `+0x86 = b`, u16 `+0x88 = 0`, `+0x8a =
/// kind`, `+0x8c = w1`, `+0x90 = w2`, `+0x94..+0x9c` the scales, `+0xa0
/// = 0.005`; the `n` points copied to `O` (reversed with `reverse`).
/// Then the handle `h` = slot 772 `(O, 0)`; slot 773 `(h)` says whether
/// it is open (`**G_LOCK` 1 around it), its flags `F` = slot 776 (the
/// rate byte `D + D[1] + 4`); an open one is closed (slot 779 `(h)`);
/// `r` = slot 942 `(M)` (a local) and slot 907 `(h, &r, 0x40, F)`; then
/// slot 269 `(O+0x40, O, n)`, whose `eax` it returns. The original keeps
/// `n - 1` in its argument word `n`.
///
/// `magic_0515` 0x10002dc0 (480 bytes, `ecx`/`edx`; 4 DLLs).
pub fn spline_object<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let (o, p) = (fr.ecx, fr.edx);
    let a = |g: &G, i: u32| fr.arg(g, i);
    let top = fr.esp.wrapping_sub(4) & !7;
    let m = a(g, 4);
    let d = i32::from(g.read::<i16>(m.wrapping_add(0x10)))
        .cast_unsigned()
        .wrapping_add(m);
    g.write::<u32>(o + 0x80, d);
    g.write::<u8>(o + 0x86, a(g, 2) as u8);
    g.write::<u8>(o + 0x85, a(g, 10) as u8);
    g.write::<u16>(o + 0x8a, a(g, 1) as u16);
    let n = a(g, 0);
    g.write::<u8>(o + 0x84, n as u8);
    g.write::<u16>(o + 0x88, 0);
    let x = a(g, 5);
    g.write::<u32>(o + 0x94, x);
    g.write::<u32>(fr.esp + 4, n.wrapping_sub(1));
    for (at, i) in [(0x98u32, 6u32), (0x8c, 8), (0x9c, 7)] {
        let v = a(g, i);
        g.write::<u32>(o + at, v);
    }
    g.write::<u32>(o + 0xa0, 0x3ba3_d70a);
    let v = a(g, 9);
    g.write::<u32>(o + 0x90, v);
    let reverse = a(g, 3) != 0;
    let mut i = n.wrapping_sub(1);
    let mut src = p;
    loop {
        let dst = o.wrapping_add(i << 4);
        let from = if reverse { src } else { p.wrapping_add(i << 4) };
        let lo = g.read::<u64>(from);
        g.write::<u64>(dst, lo);
        let hi = g.read::<u64>(from.wrapping_add(8));
        g.write::<u64>(dst + 8, hi);
        src = src.wrapping_add(0x10);
        if i == 0 {
            break;
        }
        i = i.wrapping_sub(1);
    }
    let d = g.read::<u32>(o + 0x80);
    let d2 = g.read::<u32>(d.wrapping_add(4)).wrapping_add(d);
    let h = slot(g, img, f, 0xea, 772, o, &[o, 0]);
    let lock = |g: &G, at: u32| g.read::<u32>(w(at));
    let l = lock(g, 0xf8);
    let saved = g.read::<u32>(l);
    g.write::<u32>(l, 1);
    let open = slot(g, img, f, 0x107, 773, o, &[h.eax, h.edx, o, 0]).eax;
    g.write::<u32>(lock(g, 0x11e), saved);
    let rate = u32::from(g.read::<u8>(d2.wrapping_add(4)));
    let flags = slot(g, img, f, 0x12d, 776, o, &[rate, h.eax, h.edx, o, 0]).eax;
    if open != 0 {
        let l = lock(g, 0x14e);
        let saved = g.read::<u32>(l);
        g.write::<u32>(l, 1);
        slot(g, img, f, 0x153, 779, o, &[h.eax, h.edx]);
        g.write::<u32>(lock(g, 0x16a), saved);
    }
    let r = slot(g, img, f, 0x174, 942, o, &[m]).eax;
    let local = top.wrapping_sub(0xc);
    g.write::<u32>(local, r);
    let l = lock(g, 0x188);
    let saved = g.read::<u32>(l);
    g.write::<u32>(l, 1);
    slot(
        g,
        img,
        f,
        0x1a3,
        907,
        o,
        &[h.eax, h.edx, local, 0x40, flags],
    );
    g.write::<u32>(lock(g, 0x1b2), saved);
    let n = g.read::<u32>(fr.esp + 4).wrapping_add(1);
    slot(g, img, f, 0x1ca, 269, o, &[o + 0x40, o, n]).eax
}

/// [`spline_object`].
pub const SPLINE_OBJECT: Sig = Sig {
    len: 480,
    hash: 0xadc0_93d4_b7d4_28ba,
    calls: &[],
    imports: &[],
};

/// An emitter aimed at its target and a task node for it (internal:
/// `ecx` = the emitter `E`, `xmm1` = the speed `v`; a /GS frame aligned
/// to 8, the matrix `M` a local). `E+0x25c..` = `(0, 0, -v, 0)` (the
/// sign flipped by the DLL's mask) and `E+0x274 = v`; with `d` the
/// target `E+0x24c..` less the position `E+0xc..`: the pitch
/// `atan2(dy, sqrt(dz dz + dx dx))` and the yaw `K - atan2(dz, dx)`
/// (brought back by `T` once when above `H` or below
/// `L`), the arcs `_CIatan2`'s and the root `f64`; `M` a unit matrix
/// (slot 235) turned by the pitch (slot 328) and the yaw (slot 329), and
/// `E+0x25c` = slot 229 `(E+0x25c, M, E+0x25c)`. A node from the pool
/// ([`take`](crate::sa_pool::take), owner `CB`) with `+0x14 = E` is
/// appended to the DLL's task list (`*G_HEAD`, `*G_TAIL`). Returns the
/// node; with none left the original writes to address 0x14 (not run).
///
/// `magic_0284` 0x10003a40 (518 bytes, `ecx`/`xmm1`; 4 DLLs).
pub fn aimed_emitter<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let lay = AimLayout {
        vel: 0x25c,
        target: 0x24c,
        pos: 0xc,
        speed: 0x274,
        angles: None,
        k: [0xfe, 0x10d, 0x11d, 0x127, 0x134],
        slots: [0x144, 0x166, 0x187, 0x19a],
        list: [0x1b1, 0x1be, 0x1c4, 0x1d7, 0x1e9, 0x1f6],
    };
    aim_core(g, img, fr, &lay)
}

/// Where an aimed emitter's variant keeps its words: the fields of `E`
/// (the velocity, the target, the position, the speed, the two angles
/// when it keeps them) and the code's operands (the yaw's constants
/// `K, H, T, L, T`; the slot tables of 235, 328, 329 and 229; the
/// owner, the take's `rel32` and the list's four globals).
struct AimLayout {
    vel: u32,
    target: u32,
    pos: u32,
    speed: u32,
    angles: Option<(u32, u32)>,
    k: [u32; 5],
    slots: [u32; 4],
    list: [u32; 6],
}

/// [`aimed_emitter`] on the layout `lay`.
fn aim_core<G: Game>(g: &mut G, img: &Image, fr: &Frame, lay: &AimLayout) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, i: usize| g.read::<f32>(w(lay.k[i]));
    let e = fr.ecx;
    let speed = fr.xmm[1] as u32;
    let neg = speed ^ g.read::<u32>(w(0x1a));
    let v = e + lay.vel;
    g.write::<u32>(v + 8, neg);
    for at in [0u32, 4, 0xc] {
        g.write::<u32>(v + at, 0);
    }
    let d = |g: &G, c: u32| {
        sub(
            g.read::<f32>(e + lay.target + c),
            g.read::<f32>(e + lay.pos + c),
        )
    };
    let (dy, dx, dz) = (d(g, 4), d(g, 0), d(g, 8));
    let s = add(mul(dz, dz), mul(dx, dx));
    // `_libm_sse2_sqrt_precise`, then `_CIatan2` on x87
    let h = f64::from(s).sqrt();
    let pitch = g.atan2(f64::from(dy), h) as f32;
    let yaw = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let mut y = sub(k(g, 0), yaw);
    if y > k(g, 1) {
        y = sub(y, k(g, 2));
    } else if k(g, 3) > y {
        y = add(y, k(g, 4));
    }
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x70);
    let m = base + 0x28;
    let [s235, s328, s329, s229] = lay.slots;
    slot(g, img, f, s235, 235, e, &[m]);
    slot(g, img, f, s328, 328, e, &[m, m, pitch.to_bits()]);
    slot(g, img, f, s329, 329, e, &[m, m, y.to_bits()]);
    slot(g, img, f, s229, 229, e, &[v, m, v]);
    if let Some((a, b)) = lay.angles {
        g.write::<f32>(e + a, pitch);
        g.write::<f32>(e + b, y);
    }
    g.write::<u32>(e + lay.speed, speed);
    let [owner, take, head, tail_r, head_w, tail_w] = lay.list;
    let n = crate::sa_pool::take(g, img, img.rel32_target(f + take), w(owner));
    g.write::<u32>(n.wrapping_add(0x14), e);
    g.write::<u32>(n.wrapping_add(4), 0);
    if g.read::<u32>(w(head)) != 0 {
        let tail = g.read::<u32>(w(tail_r));
        g.write::<u32>(n, tail);
        g.write::<u32>(tail.wrapping_add(4), n);
    } else {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(head_w), n);
    }
    g.write::<u32>(w(tail_w), n);
    g.write::<u32>(n.wrapping_add(8), n);
    n
}

/// [`aimed_emitter`]'s variant that keeps the two angles (`E+0xc0` the
/// pitch, `+0xc4` the yaw) and its fields further on: velocity
/// `+0x368`, target `+0x358`, position `+0xd0`, speed `+0x384`
/// (`magic_0018` 0x100031e0, 559 bytes; 5 DLLs).
pub fn aimed_emitter_angles<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let lay = AimLayout {
        vel: 0x368,
        target: 0x358,
        pos: 0xd0,
        speed: 0x384,
        angles: Some((0xc0, 0xc4)),
        k: [0x107, 0x116, 0x126, 0x130, 0x13d],
        slots: [0x14d, 0x16f, 0x190, 0x1a3],
        list: [0x1da, 0x1e7, 0x1ed, 0x200, 0x212, 0x21f],
    };
    aim_core(g, img, fr, &lay)
}

/// [`aimed_emitter_angles`] (the `rel32`s as [`AIMED_EMITTER`]'s).
pub const AIMED_EMITTER_ANGLES: Sig = Sig {
    len: 559,
    hash: 0xd6fc_a2cf_b5c5_1ed0,
    calls: &[
        (0xb3, Callee::Import("_libm_sse2_sqrt_precise")),
        (0xc6, Callee::Import("_CIatan2")),
        (0xf5, Callee::Import("_CIatan2")),
        (0x1e7, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x227, Callee::Any),
    ],
    imports: &[],
};

/// [`aimed_emitter`] (the `rel32`s: the square root's and `_CIatan2`'s
/// thunks, the pool's take and `__security_check_cookie`).
pub const AIMED_EMITTER: Sig = Sig {
    len: 518,
    hash: 0xa960_562b_d1bd_2d25,
    calls: &[
        (0xaa, Callee::Import("_libm_sse2_sqrt_precise")),
        (0xbd, Callee::Import("_CIatan2")),
        (0xec, Callee::Import("_CIatan2")),
        (0x1be, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x1fe, Callee::Any),
    ],
    imports: &[],
};

/// [`summon_frame_ring`]'s variant whose pass byte goes up first and
/// that copies `G_X = *G_Y` (`magic_0577` 0x1000dd60, 279 bytes).
pub fn summon_frame_pass<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = SummonOps {
        pre: SummonPre::PassAndCopy(0x1e, 0x2e, 0x29),
        stage: 0xb,
        p: 0x18,
        item: 0x24,
        bind: 0x33,
        bone_first: false,
        bone: (0x88, 0x94, 0x9d),
        ring: (0x39, 0x40, [0x48, 0x6a, 0x78], 0x72, 0x80),
        visits: [(0xa4, 0xb5, 0xba), (0xc1, 0xd5, 0xda)],
        end: (0xdf, 0xe5, 0xf3, 0xfb, 0x101),
    };
    summon_frame(g, img, fr, &o)
}

/// [`summon_frame_pass`] (the `rel32`s as [`SUMMON_FRAME_BOUND`]'s).
pub const SUMMON_FRAME_PASS: Sig = Sig {
    len: 279,
    hash: 0x693c_6f3b_3f0c_b805,
    calls: &[
        (0x33, Callee::Sig(&crate::sa_summon::BIND_MODELS)),
        (0x9d, Callee::Sig(&crate::sa_summon::BONE)),
        (0xba, Callee::Sig(&crate::nodes::GATED)),
        (0xda, Callee::Sig(&crate::nodes::GATED)),
    ],
    imports: &[],
};

/// [`summon_frame_ring`]'s variant that calls slot 673 first
/// (`magic_0603` 0x1000da40, 279 bytes).
pub fn summon_frame_673<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = SummonOps {
        pre: SummonPre::Slot673(0x18),
        stage: 0xb,
        p: 0x25,
        item: 0x2b,
        bind: 0x30,
        bone_first: false,
        bone: (0x88, 0x94, 0x9d),
        ring: (0x36, 0x3d, [0x45, 0x6a, 0x78], 0x72, 0x80),
        visits: [(0xa4, 0xb5, 0xba), (0xc1, 0xd5, 0xda)],
        end: (0xdf, 0xe5, 0xf3, 0xfb, 0x101),
    };
    summon_frame(g, img, fr, &o)
}

/// [`summon_frame_673`] (the `rel32`s as [`SUMMON_FRAME_BOUND`]'s).
pub const SUMMON_FRAME_673: Sig = Sig {
    len: 279,
    hash: 0x49cd_5a89_d5a6_bd3b,
    calls: &[
        (0x30, Callee::Sig(&crate::sa_summon::BIND_MODELS)),
        (0x9d, Callee::Sig(&crate::sa_summon::BONE)),
        (0xba, Callee::Sig(&crate::nodes::GATED)),
        (0xda, Callee::Sig(&crate::nodes::GATED)),
    ],
    imports: &[],
};

/// A two-stage summon set-up task (cdecl `(task)`, `task[0]` its ring
/// node, `O = task[1]`; an aligned frame, the record `L` a local): the
/// clock `O+0x568 += K` and the stage `O+0x574`. Stage 0, once the clock
/// is at `K0` (ordered): the nodes `A = O+0x24c` and `B = O+0x2a4`
/// appended to the DLL's list `G_L`
/// ([`node_append`](crate::sb_misc::node_append)), slot 325 `(O+0x268,
/// O+0x23c)`, `O+0x26c += K1` or `K2` by the model's way
/// ([`model_direct`](crate::s6_misc::model_direct) of `(O+0x578)+0x50`),
/// `B`'s colour `(1, 1, 1, O+0x56c K3 K4)`; the node `N = O+0x41c`
/// inserted ([`node_insert`](crate::nodes::node_insert): `(N, G_L, G_V,
/// A, G_W, B)`), the particle `O+0x120` (vtable `VT`) set up
/// ([`particle_node`](crate::sb_nodes::particle_node): `(p, G_O, N, *G_D,
/// G_E, 0)`), slot 239 `(*G_S, (O+0x578)+0x50, 0x20)` and the next stage.
/// Stage 1, once bit 2 of both `O+0x108` and `O+0x208` is set: seven of
/// its nodes leave their parents ([`leave`](crate::sa_misc::leave)),
/// the task's node leaves its ring and the stage is 0xff. Returns what
/// the original leaves in `eax` (slot 239's, the ring's `prev`, else
/// the stage minus 1).
///
/// `magic_0439` 0x10006a90 (556 bytes; 2 DLLs).
pub fn summon_setup_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 4);
    let clock = add(g.read::<f32>(o + 0x568), k(g, 0x1e));
    let stage = g.read::<u32>(o + 0x574);
    g.write::<f32>(o + 0x568, clock);
    match stage {
        0 => {}
        1 => {
            if g.read::<u8>(o + 0x108) & 4 == 0 || g.read::<u8>(o + 0x208) & 4 == 0 {
                return Ret::eax(0);
            }
            let mut eax = 0;
            for at in [0x220u32, 0x278, 0x2d0, 0x24c, 0x2a4, 0x41c, 0x57c] {
                eax = crate::sa_misc::leave(g, o + at, eax);
            }
            let node = g.read::<u32>(task);
            let prev = unring(g, node);
            g.write::<u32>(o + 0x574, 0xff);
            return Ret::eax(prev);
        }
        s => return Ret::eax(s.wrapping_sub(1)),
    }
    // `comiss`/`jb`: below `K0` or unordered waits
    if !(clock >= k(g, 0xd5)) {
        return Ret::eax(0);
    }
    let (a, b) = (o + 0x24c, o + 0x2a4);
    crate::sb_misc::node_append(g, img, call(0xed), a, w(0xe6), 0);
    slot(g, img, f, 0x100, 325, a, &[o + 0x268, o + 0x23c]);
    let user = g.read::<u32>(g.read::<u32>(o + 0x578).wrapping_add(0x50));
    let direct = Frame {
        at: call(0x119),
        ecx: user,
        ..*fr
    };
    let way = crate::s6_misc::model_direct(g, img, &direct).eax;
    let step = if way != 0 { k(g, 0x12d) } else { k(g, 0x137) };
    let v = add(g.read::<f32>(o + 0x26c), step);
    g.write::<f32>(o + 0x26c, v);
    crate::sb_misc::node_append(g, img, call(0x14f), b, w(0x142), 0);
    for at in [0x2c0u32, 0x2c4, 0x2c8] {
        g.write::<u32>(o + at, 0x3f80_0000);
    }
    let alpha = mul(mul(g.read::<f32>(o + 0x56c), k(g, 0x17d)), k(g, 0x18f));
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let rec = base + 0x10;
    let n = o + 0x41c;
    g.write::<u32>(rec + 0x14, b);
    g.write::<u32>(rec, n);
    g.write::<f32>(o + 0x2cc, alpha);
    for (i, v) in (1u32..).zip([w(0x142), w(0x1ab), a, w(0x1b7)]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let insert = Frame {
        at: call(0x1bc),
        esp: base.wrapping_sub(4),
        ecx: rec,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &insert);
    let d = g.read::<u32>(w(0x1c1));
    let p = o + 0x120;
    g.write::<u32>(rec, p);
    g.write::<u32>(p, w(0x1d1));
    for (i, v) in (1u32..).zip([w(0x1dd), n, d, w(0x1ed), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::particle_node(g, img, call(0x1fa), rec);
    let user = g.read::<u32>(g.read::<u32>(o + 0x578).wrapping_add(0x50));
    let s = g.read::<u32>(w(0x210));
    let eax = slot(g, img, f, 0x20a, 239, rec, &[s, user, 0x20]).eax;
    let st = g.read::<u32>(o + 0x574).wrapping_add(1);
    g.write::<u32>(o + 0x574, st);
    Ret::eax(eax)
}

/// [`summon_setup_task`] (the `rel32`s: seven
/// [`leave`](crate::sa_misc::leave)s, two
/// [`node_append`](crate::sb_misc::node_append)s,
/// [`model_direct`](crate::s6_misc::model_direct),
/// [`node_insert`](crate::nodes::node_insert) and
/// [`particle_node`](crate::sb_nodes::particle_node)).
pub const SUMMON_SETUP_TASK: Sig = Sig {
    len: 556,
    hash: 0xda92_eea7_2f2a_849b,
    calls: &[
        (0x61, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x6c, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x77, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x82, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x8d, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x98, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xa3, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xed, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x119, Callee::Sig(&crate::s6_misc::MODEL_DIRECT)),
        (0x14f, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1bc, Callee::Sig(&crate::nodes::INSERT)),
        (0x1fa, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// A ring node's handler that places its model's two groups of items
/// (cdecl `(S)`, the cursor; a ring pass first, [`ring_pass`], over the
/// frame's cookie word; a /GS frame aligned to 8, `F` the frame after
/// the saved registers). Passed, with `R = S[0]` and its model `M =
/// R[0x1c]`: the frame's draw record is set (zeros, the screen sizes
/// 160, 126, 126, 128 and 512, 256, 1791.5, 1839.5, the u16 `R+0x20` as
/// a float twice, `G_M2` twice, `(0, -0.0, 0)`, a byte 0, `F+0x18 = R`)
/// and slot 245 `(F+0x1c)` called; then for the groups `j` = 0, 1 (the
/// items at `M[9 + j]`, 16 bytes each, their count the u16 `M + 0x2c +
/// 2j`), each item `p`: `X` = slot 229 `(F+0xc0, G_M, p)`, slot 325
/// `(F+0x20, X)`, three words 1.0 and slot 231 `(F+0xd0, G_M2, F+0x90)`,
/// `F+0x84 = *G_P + 0x68`. Returns what the original leaves in `eax`:
/// the last group's count (0 for none), else what the pass left.
///
/// `magic_0042` 0x10004c90 (606 bytes; 4 DLLs).
pub fn model_items_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let s = fr.arg(g, 0);
    let top = fr.esp.wrapping_sub(4) & !7;
    let cookie = g.read::<u32>(w(0xd)) ^ top.wrapping_sub(0x10c);
    let (passed, eax) = ring_pass(g, s, cookie);
    if !passed {
        return Ret::eax(eax);
    }
    let r = g.read::<u32>(s);
    let fb = top.wrapping_sub(0x118);
    for at in [0x94u32, 0x98, 0x9c, 0xa0, 0xa8, 0xac, 0xb0, 0xb4, 0xbc] {
        g.write::<u32>(fb + at, 0);
    }
    for (at, v) in [
        (0x30u32, 0x4320_0000u32),
        (0x34, 0x42fc_0000),
        (0x38, 0x42fc_0000),
        (0x3c, 0x4300_0000),
        (0x48, w(0xec)),
        (0x80, w(0xf7)),
    ] {
        g.write::<u32>(fb + at, v);
    }
    let n = cvt(u32::from(g.read::<u16>(r + 0x20)));
    g.write::<f32>(fb + 0x7c, n);
    g.write::<f32>(fb + 0x78, n);
    for (at, v) in [
        (0x70u32, 0x44df_f000u32),
        (0x74, 0x44e5_f000),
        (0x68, 0x4400_0000),
        (0x6c, 0x4380_0000),
        (0x18, r),
    ] {
        g.write::<u32>(fb + at, v);
    }
    slot(g, img, f, 0x10c, 245, s, &[fb + 0x1c]);
    for (at, v) in [(0x50u32, 0u32), (0x54, 0x8000_0000), (0x58, 0)] {
        g.write::<u32>(fb + at, v);
    }
    g.write::<u8>(fb + 0x88, 0);
    let mut eax = 0;
    for (items, count) in [(0x24u32, 0x2cu32), (0x28, 0x2e)] {
        let m = g.read::<u32>(r + 0x1c);
        let mut p = g.read::<u32>(m.wrapping_add(items));
        eax = 0;
        let mut i = 0u32;
        while i < u32::from(g.read::<u16>(g.read::<u32>(r + 0x1c).wrapping_add(count))) {
            let x = fb + 0xc0;
            slot(g, img, f, 0x1a0, 229, s, &[x, w(0x19a), p]);
            slot(g, img, f, 0x1ba, 325, s, &[fb + 0x20, x]);
            for at in [0xb8u32, 0xa4, 0x90] {
                g.write::<u32>(fb + at, 0x3f80_0000);
            }
            slot(g, img, f, 0x1dc, 231, s, &[fb + 0xd0, w(0x1d6), fb + 0x90]);
            let v = g.read::<u32>(w(0x20a)).wrapping_add(0x68);
            g.write::<u32>(fb + 0x84, v);
            i += 1;
            p = p.wrapping_add(0x10);
            let m = g.read::<u32>(g.read::<u32>(fb + 0x18).wrapping_add(0x1c));
            eax = u32::from(g.read::<u16>(m.wrapping_add(count)));
        }
    }
    Ret::eax(eax)
}

/// [`model_items_node`] (the `rel32`: `__security_check_cookie`).
pub const MODEL_ITEMS_NODE: Sig = Sig {
    len: 606,
    hash: 0xebac_c9e4_a523_d94f,
    calls: &[(0x256, Callee::Any)],
    imports: &[],
};

/// A model's sound record sent (cdecl, no arguments; a frame aligned to
/// 16 through `ebx`, `B` its base, the matrix `M` at `B-0x48` and the
/// record `L` at `B-0xc0`). With `C = (**G_A)+0x20 + 0x8e0` and its model
/// `D = C[0x1c]`: `M` = slot 298 `(M)` with its diagonal 1.0 and the
/// point `(*G_X, *G_Y + K, *G_Z, 1)`, then slot 231 `(M, *G_W, M)`; `L` =
/// 0xc, `*G_P`, the u16 `D+0x16`, `M`, zeros. The handle `h` = slot 772
/// `(0x203, 0)`; slot 773 `(h)` says whether it is open; open: slot 819
/// `(h, C, &L)` and slot 870 `(h)`; not: the flags `F` = slot 776 (the
/// low byte of `D+0x16`) | slot 777 `(0)` | 0x40 and slot 871 `(h, C, &L,
/// F)`. Every call after the handle runs with `**G_LOCK` 1 and puts the
/// word back. Returns the lock's pointer `*G_LOCK`.
///
/// `magic_0515` 0x10004a40 (657 bytes; 4 DLLs).
pub fn sound_record<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let b = fr.esp.wrapping_sub(0xc) & !0xf;
    let (m, l) = (b.wrapping_sub(0x48), b.wrapping_sub(0xc0));
    let c = g
        .read::<u32>(g.read::<u32>(g.read::<u32>(w(0x27))).wrapping_add(0x20))
        .wrapping_add(0x8e0);
    let d = g.read::<u32>(c.wrapping_add(0x1c));
    slot(g, img, f, 0x45, 298, 0, &[m]);
    g.write::<f32>(m + 0x30, k(g, 0x55));
    let y = add(k(g, 0x62), k(g, 0x6a));
    g.write::<f32>(m + 0x34, y);
    let wm = g.read::<u32>(w(0x77));
    for at in [0x28u32, 0x14, 0] {
        g.write::<u32>(m + at, 0x3f80_0000);
    }
    g.write::<f32>(m + 0x38, k(g, 0x84));
    g.write::<u32>(m + 0x3c, 0x3f80_0000);
    slot(g, img, f, 0x8a, 231, 0, &[m, wm, m]);
    g.write::<u8>(l + 0x1b, 0);
    g.write::<u32>(l, 0xc);
    let kind = g.read::<u16>(d.wrapping_add(0x16));
    g.write::<u16>(l + 8, kind);
    g.write::<u8>(l + 0xa, 0);
    for at in [0x1cu32, 0x3c, 0x38] {
        g.write::<u32>(l + at, 0);
    }
    let p = g.read::<u32>(g.read::<u32>(w(0xd4)));
    g.write::<u32>(l + 4, p);
    g.write::<u32>(l + 0x20, m);
    for at in [0xcu32, 0x10, 0x40, 0x44, 0x2c, 0x28, 0x24] {
        g.write::<u32>(l + at, 0);
    }
    let h = slot(g, img, f, 0x112, 772, 0, &[0x203, 0]);
    let lock = |g: &G, at: u32| g.read::<u32>(w(at));
    let lp = lock(g, 0x167);
    let saved = g.read::<u32>(lp);
    g.write::<u32>(lp, 1);
    let open = slot(g, img, f, 0x179, 773, 0, &[h.eax, h.edx, 0x203, 0]).eax;
    let lp = lock(g, 0x19b);
    g.write::<u32>(lp, saved);
    if open != 0 {
        g.write::<u32>(lp, 1);
        slot(g, img, f, 0x1bc, 819, 0, &[h.eax, h.edx, c, l]);
        g.write::<u32>(lock(g, 0x1d1), 1);
        let words = [h.eax, h.edx, h.eax, h.edx, c, l];
        slot(g, img, f, 0x1dd, 870, 0, &words);
        let lp = lock(g, 0x1eb);
        g.write::<u32>(lp, saved);
        return Ret::eax(lp);
    }
    let x = slot(g, img, f, 0x216, 776, 0, &[u32::from(kind as u8)]).eax;
    let y = g.read::<u32>(l + 0x44);
    let flags = x | slot(g, img, f, 0x224, 777, 0, &[y]).eax | 0x40;
    let lp = lock(g, 0x23e);
    let saved = g.read::<u32>(lp);
    g.write::<u32>(lp, 1);
    slot(g, img, f, 0x25c, 871, 0, &[h.eax, h.edx, c, l, flags]);
    let lp = lock(g, 0x275);
    g.write::<u32>(lp, saved);
    Ret::eax(lp)
}

/// [`sound_record`] (the `rel32`s: two `__security_check_cookie`s).
pub const SOUND_RECORD: Sig = Sig {
    len: 657,
    hash: 0x97d0_bbca_fcca_22c4,
    calls: &[(0x202, Callee::Any), (0x286, Callee::Any)],
    imports: &[],
};

/// A summon's falling-sparks frame (cdecl `(node)`; `ebp` = `esp - 4` at
/// the entry, a scratch float at `ebp-4`). While the delay `*G_C` is
/// above 0 it counts down (still above: it returns it); when it reaches
/// 0, slot 237 `(*G_S, T[*G_I], 0, G_PT)` and `*G_W = 1.0`. Then slot
/// 326 `(G_A, G_B, G_PT, v)` with `v = *G_V`; the height `*G_Y -= v (K1 -
/// v) K2 K3 + (K1 - v)^2 0 + v^2 0` (each product kept, the zeros
/// included) and `*G_V = v + K4`. Below `K1` (or unordered) four sparks
/// start: each a record `p` from the ring ([`ring_take`]; none ends it,
/// returning 0): u16 `+0xa = 0x200`, `+8 = -1`, `+0 = **G_Q`, `+4 = 1`,
/// its point (slot 325 `(p+0xc, G_PT)`) moved by three random floats
/// (slot 314 `(1.0)`, ST0), `+0x1c..+0x24 = (r - K5) K6` (slot 307,
/// ST0), `+0x28 = 0`, `+0x2c = 0.01`, `+0x34 = +0x38 = 0x18`, byte
/// `+0x47 = 0` (the last slot's `eax` returned). Else the object `G_O`
/// leaves the list `G_L` ([`list_remove`]); when it was there `*G_P =
/// ITEM`, slot 713 `(G_O)` and u16 `*G_F &= 0x7fff`; then the node goes
/// back to its pool ([`give_back`]), whose `eax` it returns.
///
/// [`ring_take`]: crate::sa_misc::ring_take
/// [`list_remove`]: crate::l5_objhost::list_remove
/// [`give_back`]: crate::sa_pool::give_back
///
/// `magic_0242` 0x10003aa0 (689 bytes; 5 DLLs).
pub fn falling_sparks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let c = g.read::<i32>(w(0x5));
    if c > 0 {
        let c = c - 1;
        g.write::<i32>(w(0xf), c);
        if c > 0 {
            return Ret::eax(c.cast_unsigned());
        }
        let i = g.read::<u32>(w(0x1c));
        let tv = g.read::<u32>(i.wrapping_mul(4).wrapping_add(w(0x2a)));
        let s = g.read::<u32>(w(0x35));
        slot(g, img, f, 0x2f, 237, fr.ecx, &[s, tv, 0, w(0x21)]);
        g.write::<u32>(w(0x46), w(0x4a));
    }
    let v = g.read::<u32>(w(0x52));
    slot(
        g,
        img,
        f,
        0x57,
        326,
        fr.ecx,
        &[w(0x72), w(0x6d), w(0x68), v],
    );
    let v = k(g, 0x7c);
    let left = sub(k(g, 0x84), v);
    let a = mul(mul(mul(v, left), k(g, 0xa1)), k(g, 0xad));
    let a = add(a, mul(mul(left, left), 0.0));
    let b = mul(mul(v, v), 0.0);
    let next = add(v, k(g, 0xc0));
    let a = add(a, b);
    let y = g.read::<f32>(w(0xd7));
    g.write::<f32>(w(0xdf), next);
    g.write::<f32>(w(0xeb), sub(y, a));
    // `comiss`/`jb`: below `K1` or unordered starts the sparks
    if next >= k(g, 0xcb) {
        let removed = crate::l5_objhost::list_remove(g, img, call(0xfc), w(0xf7), w(0xf2));
        if removed != 0 {
            let p = g.read::<u32>(w(0x105));
            g.write::<u32>(p, w(0x110));
            slot(g, img, f, 0x115, 713, fr.ecx, &[w(0x10a)]);
            let v = g.read::<u16>(w(0x12c)) & 0x7fff;
            g.write::<u16>(w(0x12c), v);
        }
        let node = fr.arg(g, 0);
        return Ret::eax(crate::sa_pool::give_back(g, img, call(0x134), node));
    }
    let scratch = fr.esp.wrapping_sub(8);
    let mut eax = 0;
    for _ in 0..4 {
        let p = crate::sa_misc::ring_take(g, img, call(0x145));
        if p == 0 {
            return Ret::eax(0);
        }
        let q = g.read::<u32>(g.read::<u32>(w(0x155)));
        g.write::<u16>(p + 0xa, 0x200);
        g.write::<u16>(p + 8, 0xffff);
        g.write::<u32>(p, q);
        g.write::<u32>(p + 4, 1);
        slot(g, img, f, 0x165, 325, fr.ecx, &[p + 0xc, w(0x180)]);
        let rnd = |g: &mut G, t: u32, s: u32, args: &[u32]| {
            let o = slot(g, img, f, t, s, fr.ecx, args);
            let x = crate::m0018::st0_f32(o.st0);
            g.write::<f32>(scratch, x);
            (x, o.eax)
        };
        let one = [0x3f80_0000];
        let (r, _) = rnd(g, 0x188, 314, &one);
        let x = add(g.read::<f32>(p + 0xc), r);
        g.write::<f32>(p + 0xc, x);
        let (r, _) = rnd(g, 0x1ab, 314, &one);
        let y = add(r, g.read::<f32>(p + 0x10));
        g.write::<f32>(p + 0x10, y);
        let (r, _) = rnd(g, 0x1d0, 314, &one);
        let z = add(r, g.read::<f32>(p + 0x14));
        g.write::<f32>(p + 0x14, z);
        // (the slot table, the two constants' operands)
        for (t, ks, km, at) in [
            (0x1f6u32, 0x216u32, 0x223u32, 0x1cu32),
            (0x21b, 0x240, 0x24d, 0x20),
        ] {
            let (r, _) = rnd(g, t, 307, &[]);
            g.write::<f32>(p + at, mul(sub(r, k(g, ks)), k(g, km)));
        }
        let (r, e) = rnd(g, 0x245, 307, &[]);
        eax = e;
        let d = sub(r, k(g, 0x26a));
        g.write::<u32>(p + 0x28, 0);
        g.write::<u32>(p + 0x2c, 0x3c23_d70a);
        g.write::<u32>(p + 0x34, 0x18);
        g.write::<u32>(p + 0x38, 0x18);
        g.write::<u8>(p + 0x47, 0);
        g.write::<f32>(p + 0x24, mul(d, k(g, 0x288)));
    }
    Ret::eax(eax)
}

/// [`falling_sparks`] (the `rel32`s: [`list_remove`](crate::l5_objhost::list_remove),
/// [`give_back`](crate::sa_pool::give_back) and
/// [`ring_take`](crate::sa_misc::ring_take)).
pub const FALLING_SPARKS: Sig = Sig {
    len: 689,
    hash: 0xa0db_6813_4541_7448,
    calls: &[
        (0xfc, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x134, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x145, Callee::Sig(&crate::sa_misc::RING_TAKE)),
    ],
    imports: &[],
};

/// A camera path task started (cdecl `(P, a, b, n)`, `xmm3` a scale; a
/// /GS frame, `ebp` = `esp - 4` at the entry, the vectors `L1` at
/// `ebp-0x24` and `L2` at `ebp-0x14`). With `A = *G_A`: `L1` = slot 217
/// `(A, L1)` into `G1`; `P` into `G2`; `L2` = slot 234 `(L2, P, L1)` into
/// `G3`; `G4` zeroed; slot 233 `(H1, L2, scale)`, `(H2, L2, a)`, `(H3,
/// L2, b)`; five words 1.0 (the last over `G3`'s `w`), `*G_E = A`, `*G_N
/// = n`, the step `K / n`, the state zeroed. A node is taken from the
/// DLL's free list (as [`node_to_live`]; none: the original writes to
/// address 0x18, not run), set up (`+0x10 = CB`, byte `+0x18 = 1`,
/// `+0x14 = TASK`) and appended to the task list. Returns what the
/// original leaves in `eax`: the old last node, or 0xffff on an empty
/// list.
///
/// `magic_0055` 0x100028c0 (699 bytes; 4 DLLs).
pub fn camera_path_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let ebp = fr.esp.wrapping_sub(4);
    let (l1, l2) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    let p = fr.arg(g, 0);
    let scale = fr.xmm[3] as u32;
    let a = g.read::<u32>(w(0x17));
    slot(g, img, f, 0x20, 217, 0, &[a, l1]);
    for (i, at) in (0u32..).zip([0x3bu32, 0x48, 0x55, 0x62]) {
        let v = g.read::<u32>(l1 + 4 * i);
        g.write::<u32>(w(at), v);
    }
    for (i, at) in (0u32..).zip([0x6eu32, 0x7f, 0x8f, 0xa3]) {
        let v = g.read::<u32>(p.wrapping_add(4 * i));
        g.write::<u32>(w(at), v);
    }
    slot(g, img, f, 0x9b, 234, 0, &[l2, p, l1]);
    for (i, at) in (0u32..).zip([0xb8u32, 0xc5, 0xd2, 0xdf]) {
        let v = g.read::<u32>(l2 + 4 * i);
        g.write::<u32>(w(at), v);
    }
    for at in [0xfbu32, 0x105, 0x10f] {
        g.write::<u32>(w(at), 0);
    }
    let (x, y) = (fr.arg(g, 1), fr.arg(g, 2));
    for (t, h, v) in [
        (0xf5u32, 0x11e, scale),
        (0x136, 0x13b, x),
        (0x159, 0x15e, y),
    ] {
        slot(g, img, f, t, 233, 0, &[w(h), l2, v]);
    }
    for at in [0x177u32, 0x181, 0x18b, 0x195, 0x19f] {
        g.write::<u32>(w(at), 0x3f80_0000);
    }
    g.write::<u32>(w(0x1a9), a);
    let n = fr.arg(g, 3);
    let mut free = g.read::<u16>(w(0x1ba));
    let step = div(g.read::<f32>(w(0x16e)), cvt(n));
    g.write::<u32>(w(0x1c3), n);
    g.write::<u32>(w(0x1cb), 0);
    g.write::<f32>(w(0x1d7), step);
    for at in [0x1ddu32, 0x1e7, 0x1f1] {
        g.write::<u32>(w(at), 0);
    }
    g.write::<u16>(w(0x1fb), 0);
    let mut node = 0;
    let mut eax = 0;
    if free != 0 {
        node = g.read::<u32>(w(0x20a));
        let (next, prev) = (g.read::<u32>(node), g.read::<u32>(node + 4));
        g.write::<u32>(next.wrapping_add(4), prev);
        let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node));
        g.write::<u32>(prev, next);
        if free > 1 {
            let prev = g.read::<u32>(node + 4);
            g.write::<u32>(w(0x227), prev);
        } else {
            g.write::<u32>(w(0x22f), 0);
        }
        eax = 0xffff;
        free = free.wrapping_sub(1);
        for off in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node + off, 0);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 0x10, w(0x260));
        g.write::<u8>(node + 0x18, 0);
        g.write::<u16>(w(0x26b), free);
    }
    let head = g.read::<u32>(w(0x271));
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(0x14), w(0x27e));
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x2a0), node);
    } else {
        let tail = g.read::<u32>(w(0x28d));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        eax = tail;
    }
    g.write::<u32>(w(0x2ab), node);
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(eax)
}

/// [`camera_path_task`] (the `rel32`: `__security_check_cookie`).
pub const CAMERA_PATH_TASK: Sig = Sig {
    len: 699,
    hash: 0x2597_5249_c58e_1500,
    calls: &[(0x2b3, Callee::Any)],
    imports: &[],
};

/// Two sparks started at a point (internal: `ecx` = the point `P`; `ebp`
/// = `esp - 4` at the entry, a scratch float at `ebp-4`). Each spark is a
/// record `R` from the pool ([`pool_take`](crate::sb_misc::pool_take);
/// none: 0) with u16 `+0x38 = -1`, `+0x3a = 0x200`, `+0x34 = 1`, `+0x30`
/// a word of the DLL's `*G_C` block, the position `(P.x + r, P.y, P.z +
/// r, 1)`, `+0x10` and `+0x18` random, `+0x14 = -(r404 + K)` (the sign by
/// the DLL's mask), `+0x1c` a random angle, the step `(0, D, 0, 1)` and
/// `+0x3c = (r + K2) K3`, `+0x4c = -1`, `+0x50 = 0` (`r` slot 314's ST0,
/// `r404` slot 404's, each with the code's immediate); the first also
/// `+0x68 = C`. Between the two, a set bit 0..2 of `**G_F` ends it
/// (returning `*G_F`). Returns slot 314's last `eax`.
///
/// `magic_0565` 0x10003450 (729 bytes; 5 DLLs) and `magic_0644`
/// 0x100075e0 (730 bytes; 5 DLLs: its first block word at `+4`, which
/// moves its code on by a byte, and the `z` sum the other way round).
pub fn two_sparks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    // `mov eax, [eax]` (0565) or `mov eax, [eax + 4]` (0644)
    let d = u32::from(img.word(f + 0xa) & 0xff00 == 0x4000);
    let w = |o: u32| op(img, f, o + d);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let p = fr.ecx;
    let scratch = fr.esp.wrapping_sub(8);
    let block = |g: &G| g.read::<u32>(op(img, f, 0x5));
    let field = if d == 0 { 0 } else { img.word(f + 0xc) & 0xff };
    let first = g.read::<u32>(block(g).wrapping_add(field));
    // slot `s` through the table operand `t` with the immediate at `x`:
    // its ST0 as `f32` (kept in the scratch word) and its `eax`
    let rnd = |g: &mut G, t: u32, s: u32, x: u32| {
        let o = slot(g, img, f, t + d, s, p, &[w(x)]);
        let v = crate::m0018::st0_f32(o.st0);
        g.write::<f32>(scratch, v);
        (v, o.eax)
    };
    let z_first = d == 0;
    // (take, the block word, tables, immediates, K, mask, K2, K3, the
    // step's `y`)
    let mut eax = 0;
    for spark in 0..2 {
        let (take, t, x, kk, mask, k2, k3, dy) = if spark == 0 {
            (
                0x13u32,
                [0x38u32, 0x71, 0x8b, 0xae, 0xd2, 0xf5, 0x10c],
                [0x4du32, 0x62, 0x99, 0xb5, 0xd9, 0xfc, 0x135],
                0xcdu32,
                0xe0u32,
                0x147u32,
                0x165u32,
                0x11au32,
            )
        } else {
            (
                0x17f,
                [0x1a7, 0x1e3, 0x1fd, 0x220, 0x243, 0x25f, 0x276],
                [0x1bf, 0x1d4, 0x20b, 0x227, 0x255, 0x266, 0x29f],
                0,
                0x23e,
                0x2b1,
                0x2ca,
                0x284,
            )
        };
        let r = crate::sb_misc::pool_take(g, img, img.rel32_target(f + take + d));
        if r == 0 {
            return 0;
        }
        let word = if spark == 0 {
            first
        } else {
            let at = img.word(f + 0x198 + d) & 0xff;
            let b2 = g.read::<u32>(w(0x18f));
            g.read::<u32>(b2.wrapping_add(at))
        };
        g.write::<u16>(r + 0x38, 0xffff);
        g.write::<u16>(r + 0x3a, 0x200);
        g.write::<u32>(r + 0x30, word);
        g.write::<u32>(r + 0x34, 1);
        let (a, _) = rnd(g, t[0], 314, x[0]);
        g.write::<f32>(r, add(g.read::<f32>(p), a));
        let y = g.read::<u32>(p + 4);
        g.write::<u32>(r + 4, y);
        let (b, _) = rnd(g, t[1], 314, x[1]);
        let pz = g.read::<f32>(p + 8);
        let z = if z_first { add(pz, b) } else { add(b, pz) };
        g.write::<u32>(r + 0xc, 0x3f80_0000);
        g.write::<f32>(r + 8, z);
        let (v, _) = rnd(g, t[2], 314, x[2]);
        g.write::<f32>(r + 0x10, v);
        let (v, _) = rnd(g, t[3], 404, x[3]);
        let v = if spark == 0 { add(v, k(g, kk)) } else { v };
        let v = f32::from_bits(v.to_bits() ^ g.read::<u32>(w(mask)));
        g.write::<f32>(r + 0x14, v);
        let (v, _) = rnd(g, t[4], 314, x[4]);
        g.write::<f32>(r + 0x18, v);
        let (v, _) = rnd(g, t[5], 314, x[5]);
        g.write::<f32>(r + 0x1c, v);
        for (at, v) in [
            (0x20u32, 0u32),
            (0x24, w(dy)),
            (0x28, 0),
            (0x2c, 0x3f80_0000),
        ] {
            g.write::<u32>(r + at, v);
        }
        let (v, e) = rnd(g, t[6], 314, x[6]);
        eax = e;
        let v = add(v, k(g, k2));
        let flags = if spark == 0 {
            g.read::<u32>(w(0x14c))
        } else {
            0
        };
        g.write::<u32>(r + 0x4c, 0xffff_ffff);
        g.write::<u32>(r + 0x50, 0);
        let v = mul(v, k(g, k3));
        if spark == 0 {
            g.write::<u32>(r + 0x68, w(0x16c));
        }
        g.write::<f32>(r + 0x3c, v);
        if spark == 0 && g.read::<u8>(flags) & 7 != 0 {
            return flags;
        }
    }
    eax
}

/// [`two_sparks`] in `magic_0565` (the `rel32`s: two of the pool's take).
pub const TWO_SPARKS: Sig = Sig {
    len: 729,
    hash: 0xc74e_44ed_5ba5_1685,
    calls: &[
        (0x13, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x17f, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
    ],
    imports: &[],
};

/// [`two_sparks`] in `magic_0644`.
pub const TWO_SPARKS_B: Sig = Sig {
    len: 730,
    hash: 0x949d_27e8_6f81_8d51,
    calls: &[
        (0x14, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x180, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
    ],
    imports: &[],
};

/// A node appended to the DLL's parent `P` (its first `*G_PF`, last
/// `*G_PL`, count u16 `*G_PC`), as the original inlines it: u16 `+0x1a
/// = 0xff`, its own child links, `+0x14 = +4 = P`, `+8` the old last,
/// the first set when `P` was empty (else the old last's `+4`), and the
/// vtable `VT`. The operands: `(+0x14's P, +4's P, last, last, P, first,
/// count, last, VT)`.
fn parent_append<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, ops: [u32; 9]) {
    let w = |o: u32| op(img, f, o);
    let [p14, p4, l1, l2, p, first, count, l3, vt] = ops;
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, w(p14));
    g.write::<u32>(node + 4, w(p4));
    let last = g.read::<u32>(w(l1));
    g.write::<u32>(node + 8, last);
    let last = g.read::<u32>(w(l2));
    if last == w(p) {
        g.write::<u32>(w(first), node);
    } else {
        g.write::<u32>(last.wrapping_add(4), node);
    }
    let n = g.read::<u16>(w(count)).wrapping_add(1);
    g.write::<u16>(w(count), n);
    g.write::<u32>(w(l3), node);
    g.write::<u32>(node, w(vt));
}

/// A summon object set up and listed (internal: `ecx` = the object `O`,
/// `edx` = a record `R`, `xmm2` a scale `s`; an aligned frame, the
/// set-up record `L` a local). `O` joins the DLL's task ring (vtable
/// `VT`, callback `CB`; bit 0 of `*G_F` cleared); its nodes `B = O+0x120`
/// and `C = O+0x14c` are appended to the parent `P` ([`parent_append`]),
/// `B`'s point slot 268 `(R[0x54], 0x11, O+0x13c)` (`w` 1.0) and `C`'s
/// colour `(1, 1, 1, s K)`; the node `N = O+0x178` inserted
/// ([`node_insert`](crate::nodes::node_insert): `(N, P, G_V, B, G_W,
/// C)`), the particle `O+0x20` set up
/// ([`particle_node`](crate::sb_nodes::particle_node): `(p, G_O, N,
/// *G_D, G_E, 0)`), the effect node `E = O+0x2d8`
/// ([`effect_node`](crate::sb_nodes::effect_node): `(E, P, G_V, B, CB2,
/// 0)`, then vtable `VT4`, `+0x2c |= 1`, `+0x30 = 0`), slot 239 `(*G_S,
/// R[0x50], 0x20)`; `O+0x13c..` moved by three random offsets (`(r -
/// K) K' - K''`, `r` slot 307's ST0), `O+0x2d4 = R`, `+0x2c8 = s` and
/// `+0x2c4`, `+0x2cc`, `+0x2d0` 0. Returns slot 307's last `eax`.
///
/// `magic_0412` 0x10006a60 (796 bytes, `ecx`/`edx`/`xmm2`; 4 DLLs).
pub fn summon_object_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let (o, r) = (fr.ecx, fr.edx);
    let s = fr.xmm_f32(2);
    g.write::<u32>(o + 8, o);
    g.write::<u32>(o + 4, o);
    g.write::<u32>(o, w(0x1c));
    g.write::<u32>(o + 0x10, w(0x23));
    g.write::<u32>(o + 0x14, o);
    let last = g.read::<u32>(w(0x2b));
    g.write::<u32>(o + 8, last);
    g.write::<u32>(o + 4, w(0x35));
    let last = g.read::<u32>(w(0x3a));
    g.write::<u32>(last.wrapping_add(4), o);
    let v = g.read::<u32>(w(0x49)) & !1;
    g.write::<u32>(w(0x49), v);
    g.write::<u32>(w(0x50), o);
    let (b, c) = (o + 0x120, o + 0x14c);
    parent_append(
        g,
        img,
        f,
        b,
        [0x63, 0x6a, 0x6f, 0x77, 0x80, 0x88, 0x94, 0xa5, 0xac],
    );
    let h = g.read::<u32>(r.wrapping_add(0x54));
    slot(g, img, f, 0x99, 268, b, &[h, 0x11, o + 0x13c]);
    g.write::<u32>(o + 0x148, 0x3f80_0000);
    parent_append(
        g,
        img,
        f,
        c,
        [0xdc, 0xe3, 0xe8, 0xf0, 0xf5, 0xfd, 0x109, 0x11d, 0x123],
    );
    let alpha = mul(s, k(g, 0x117));
    for at in [0x168u32, 0x16c, 0x170] {
        g.write::<u32>(o + at, 0x3f80_0000);
    }
    g.write::<f32>(o + 0x174, alpha);
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x30);
    let rec = base + 0x18;
    let set = |g: &mut G, words: [u32; 6]| {
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(rec + 4 * i, v);
        }
    };
    let n = o + 0x178;
    set(g, [n, w(0x163), w(0x16b), b, w(0x177), c]);
    let insert = Frame {
        at: call(0x17c),
        esp: base.wrapping_sub(4),
        ecx: rec,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &insert);
    let d = g.read::<u32>(w(0x181));
    let q = o + 0x20;
    g.write::<u32>(rec, q);
    g.write::<u32>(q, w(0x18e));
    for (i, v) in (1u32..).zip([w(0x19a), n, d, w(0x1aa), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::particle_node(g, img, call(0x1b7), rec);
    let e = o + 0x2d8;
    set(g, [e, w(0x1cd), w(0x1d5), b, w(0x1e1), 0]);
    crate::sb_nodes::effect_node(g, img, call(0x1ee), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x1fa));
    let v = g.read::<u32>(e + 0x2c) | 1;
    g.write::<u32>(e + 0x2c, v);
    g.write::<u32>(e + 0x30, 0);
    // `[esp + 0x18]` after a push: the local holding `R`, not the record
    let x = g.read::<u32>(r.wrapping_add(0x50));
    let sv = g.read::<u32>(w(0x21d));
    slot(g, img, f, 0x203, 239, rec, &[sv, x, 0x20]);
    let local = base + 0x14;
    let mut eax = 0;
    // (the slot table, K, K', K'', the field)
    for (t, k1, k2, k3, at) in [
        (0x224u32, 0x241u32, 0x24eu32, 0x256u32, 0x13cu32),
        (0x246, 0x280, 0x28d, 0x295, 0x140),
        (0x285, 0x2bf, 0x2c7, 0x2cf, 0x144),
    ] {
        let rnd = slot(g, img, f, t, 307, rec, &[]);
        eax = rnd.eax;
        let v = crate::m0018::st0_f32(rnd.st0);
        g.write::<f32>(local, v);
        let v = sub(mul(sub(v, k(g, k1)), k(g, k2)), k(g, k3));
        let v = add(v, g.read::<f32>(o + at));
        g.write::<f32>(o + at, v);
    }
    g.write::<u32>(o + 0x2d4, r);
    g.write::<f32>(o + 0x2c8, s);
    for at in [0x2c4u32, 0x2cc, 0x2d0] {
        g.write::<u32>(o + at, 0);
    }
    eax
}

/// [`summon_object_setup`] (the `rel32`s:
/// [`node_insert`](crate::nodes::node_insert),
/// [`particle_node`](crate::sb_nodes::particle_node) and
/// [`effect_node`](crate::sb_nodes::effect_node)).
pub const SUMMON_OBJECT_SETUP: Sig = Sig {
    len: 796,
    hash: 0x8fe8_82ab_ab67_f324,
    calls: &[
        (0x17c, Callee::Sig(&crate::nodes::INSERT)),
        (0x1b7, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x1ee, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// A ring node's handler that walks its two children, scatters sparks and
/// moves its model's two groups of items (cdecl `(S)`, the cursor; a
/// ring pass first, [`ring_pass`], over the frame's cookie word; a /GS
/// frame aligned to 8, `B` its base). Passed, with `R = S[0]`: the
/// children `R[0x38]` and `R[0x3c]` get their handlers with a cursor at
/// `B+8` (the pass byte at `+5`); the sparks of the request `(R, 1)` at
/// `B+0x94` ([`ring_sparks`](crate::s6_misc::ring_sparks)); the draw
/// record (zeros, the sizes 150, 126, 126, 128, 512, 256, 1791.5,
/// 1839.5, 11, 11, `G_M` twice, `(0, -0.0, 0)`, a byte 0) and slot 245
/// `(B+8)`. Then for the two groups `j` (the items at `R[9 + j]`, 0x30
/// bytes each, their count u16 `R + 0x2c + 2j`), each item `p`: `X` =
/// slot 229 `(B+0xd8, G_M2, p)`, slot 325 `(B+0x20, X)`, its scale
/// `p+0x20` three times and slot 231 `(B+0xe8, G_M, B+0xa8)`; `p+0 +=
/// p+0x14`, `p+4 += R[0x3c]+0x20 + p+0x18`, `p+0x18 -= K`, `p+8 +=
/// p+0x1c`, `B+0x84 = *G_P + 0x70`. `*G_P` is written back as it was.
/// Returns `&R[0xb]` (passed), else what the pass left.
///
/// `magic_0042` 0x10001c10 (836 bytes; 4 DLLs).
pub fn children_items_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let s = fr.arg(g, 0);
    let b = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x130);
    let cookie = g.read::<u32>(w(0xd)) ^ b;
    let r = g.read::<u32>(s);
    g.write::<u32>(b + 4, r);
    let (passed, eax) = ring_pass(g, s, cookie);
    if !passed {
        return Ret::eax(eax);
    }
    let r = g.read::<u32>(s);
    let cursor = b + 8;
    for at in [0x38u32, 0x3c] {
        let pass = g.read::<u8>(s + 5);
        let c = g.read::<u32>(r + at);
        g.write::<u32>(cursor, c);
        g.write::<u8>(cursor + 5, pass);
        if at == 0x38 {
            // `R` kept in a local the loop reads back
            g.write::<u32>(b + 0x14, r);
        }
        let handler = g.read::<u32>(g.read::<u32>(c).wrapping_add(0x1c));
        g.call(handler, cursor, &[cursor]);
    }
    let q = b + 0x94;
    g.write::<u32>(q, r);
    g.write::<u16>(q + 4, 1);
    let sparks = Frame {
        at: img.rel32_target(f + 0xc8),
        esp: b.wrapping_sub(0x14),
        ecx: q,
        ..*fr
    };
    crate::s6_misc::ring_sparks(g, img, &sparks);
    let saved = g.read::<u32>(w(0xcd));
    g.write::<u32>(b + 0x18, saved);
    for at in [0xacu32, 0xb0, 0xb4, 0xb8, 0xc0, 0xc4, 0xc8, 0xcc, 0xd4] {
        g.write::<u32>(b + at, 0);
    }
    for (at, v) in [
        (0x30u32, 0x4316_0000u32),
        (0x34, 0x42fc_0000),
        (0x38, 0x42fc_0000),
        (0x3c, 0x4300_0000),
        (0x48, w(0x166)),
        (0x80, w(0x171)),
        (0x7c, 0x4130_0000),
        (0x78, 0x4130_0000),
        (0x70, 0x44df_f000),
        (0x74, 0x44e5_f000),
        (0x68, 0x4400_0000),
        (0x6c, 0x4380_0000),
    ] {
        g.write::<u32>(b + at, v);
    }
    slot(g, img, f, 0xdb, 245, cursor, &[cursor]);
    for (at, v) in [(0x50u32, 0u32), (0x54, 0x8000_0000), (0x58, 0)] {
        g.write::<u32>(b + at, v);
    }
    g.write::<u8>(b + 0x88, 0);
    for j in 0..2u32 {
        g.write::<u32>(b + 4, 2 - j);
        let list = r + 0x24 + 4 * j;
        g.write::<u32>(b + 0x1c, list);
        let count = r + 0x2c + 2 * j;
        let mut p = g.read::<u32>(list);
        let mut i = 0u32;
        g.write::<u32>(b + 0x10, 0);
        while i < u32::from(g.read::<u16>(count)) {
            let x = b + 0xd8;
            slot(g, img, f, 0x22f, 229, cursor, &[x, w(0x229), p]);
            slot(g, img, f, 0x249, 325, cursor, &[b + 0x20, x]);
            let sc = g.read::<u32>(p.wrapping_add(0x20));
            for at in [0xd0u32, 0xbc, 0xa8] {
                g.write::<u32>(b + at, sc);
            }
            slot(
                g,
                img,
                f,
                0x270,
                231,
                cursor,
                &[b + 0xe8, w(0x26a), b + 0xa8],
            );
            let rd = |g: &G, o: u32| g.read::<f32>(p.wrapping_add(o));
            let x0 = add(rd(g, 0x14), rd(g, 0));
            let y = rd(g, 0x18);
            g.write::<f32>(p, x0);
            let r3c = g.read::<u32>(g.read::<u32>(b + 0x14).wrapping_add(0x3c));
            i += 1;
            let up = add(g.read::<f32>(r3c.wrapping_add(0x20)), y);
            let dy = sub(y, g.read::<f32>(w(0x2c6)));
            let up = add(up, rd(g, 4));
            g.write::<f32>(p.wrapping_add(0x18), dy);
            g.write::<u32>(b + 0x10, i);
            g.write::<f32>(p.wrapping_add(4), up);
            let z = add(rd(g, 0x1c), rd(g, 8));
            g.write::<f32>(p.wrapping_add(8), z);
            let e = g.read::<u32>(b + 0x18).wrapping_add(0x70);
            g.write::<u32>(b + 0x84, e);
            p = p.wrapping_add(0x30);
        }
    }
    let saved = g.read::<u32>(b + 0x18);
    g.write::<u32>(w(0x32c), saved);
    Ret::eax(r + 0x2c)
}

/// [`children_items_node`] (the `rel32`s:
/// [`ring_sparks`](crate::s6_misc::ring_sparks) and
/// `__security_check_cookie`).
pub const CHILDREN_ITEMS_NODE: Sig = Sig {
    len: 836,
    hash: 0x4757_7089_3464_2368,
    calls: &[
        (0xc8, Callee::Sig(&crate::s6_misc::RING_SPARKS)),
        (0x33c, Callee::Any),
    ],
    imports: &[],
};

/// A node off the DLL's free ring (count u16 `*G_C`, head `*G_F`) and
/// onto its live list (`*G_L` first, `*G_T` last), as the original
/// inlines it (see [`node_to_live`]): the operands `(count, head, head
/// write, head clear, callback, count write, first, last read, last
/// write, first write, last write 2)`. With none free the node is 0 and
/// the original writes to address 4 (not run).
fn free_to_live<G: Game>(g: &mut G, img: &Image, f: u32, ops: [u32; 11]) -> u32 {
    let w = |o: u32| op(img, f, o);
    let [
        count,
        head,
        head_w,
        head_0,
        cb,
        count_w,
        first,
        last_r,
        last_w,
        first_w,
        last_w2,
    ] = ops;
    let mut free = g.read::<u16>(w(count));
    let mut n = 0;
    if free != 0 {
        n = g.read::<u32>(w(head));
        let (next, prev) = (g.read::<u32>(n), g.read::<u32>(n + 4));
        g.write::<u32>(next.wrapping_add(4), prev);
        let (prev, next) = (g.read::<u32>(n + 4), g.read::<u32>(n));
        g.write::<u32>(prev, next);
        if free > 1 {
            let prev = g.read::<u32>(n + 4);
            g.write::<u32>(w(head_w), prev);
        } else {
            g.write::<u32>(w(head_0), 0);
        }
        free = free.wrapping_sub(1);
        for off in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(n + off, 0);
        }
        g.write::<u32>(n + 8, n);
        g.write::<u32>(n + 0x10, w(cb));
        g.write::<u8>(n + 0x18, 0);
        g.write::<u16>(w(count_w), free);
    }
    let any = g.read::<u32>(w(first));
    g.write::<u32>(n.wrapping_add(4), 0);
    if any == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(first_w), n);
        g.write::<u32>(n.wrapping_add(8), n);
        g.write::<u32>(w(last_w2), n);
    } else {
        let last = g.read::<u32>(w(last_r));
        g.write::<u32>(n, last);
        g.write::<u32>(last.wrapping_add(4), n);
        g.write::<u32>(n.wrapping_add(8), n);
        g.write::<u32>(w(last_w), n);
    }
    n
}

/// A summon's actors' objects started (cdecl, no arguments; `ebp` =
/// `esp - 4` at the entry, the count `c` and the list `L` locals): slot
/// 719 `(G_D, 8, 0)` into `*G_H` (`*G_1`, `*G_2` set to immediates first;
/// two of its words stay on the stack), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)), slot 412 `(*G_X, &c,
/// 0)`. For each of the `c` entries `i`: the battle number `L[i]` is
/// looked up in the DLL's byte list `G_B` (`*G_N` entries; index `j`,
/// else 0); the object `O = B0 + 0x130 i` started at scale `K`
/// ([`start_object_first`](crate::s4_misc::start_object_first), owner
/// `15 i`); with the actor `A = *(G_T + 4 G_B[j])`, slot 262 `(A, O+0xd0,
/// O+0xd4, O+0xd8)` and slot 268 `(A, 0x11, O+0xd0)`, `O+0xdc = 1.0`,
/// `O+0xd4 -= K2`; the yaw `K3 - slot 333 (A)` (ST0), brought back by
/// `T` once when above `H` or below `Lo`, into `O+0xc4`; `O+0x120 =
/// G_B[j]`; the record `E = E0 + 0x34 i` cleared (vtables `VT`, `VT2`)
/// and `O+0x110 = E`. (The code's `obj` is `O + 0xd4`, `rec` is `E +
/// 0x14`.) Then a task node is taken and listed
/// ([`free_to_live`]). Returns 0.
///
/// `magic_0010` 0x10001370 (748 bytes; 2 DLLs).
pub fn actors_objects_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let (count, list) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x18));
    let mut st = Pushes::default();
    st.push_all(&[0, 8, w(0x17)]);
    g.write::<u32>(w(0x1d), w(0x21));
    g.write::<u32>(w(0x27), w(0x2b));
    let h = st.call(g, slot_target(g, w(0x7), 719), fr.ecx).eax;
    st.pop(1);
    g.write::<u32>(w(0x35), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x3a));
    let x = g.read::<u32>(w(0x4b));
    st.push_all(&[0, count, x]);
    let l = st.call(g, slot_target(g, w(0x45), 412), fr.ecx).eax;
    st.pop(5);
    g.write::<u32>(list, l);
    let mut owner = 0u32;
    let mut i = 0u32;
    let (mut obj, mut rec) = (w(0x75), w(0x7a));
    let actor = |g: &G, j: u32, none: u32, b: u32, t: u32| {
        if j == u32::MAX {
            g.read::<u32>(w(none))
        } else {
            let n = g.read::<u8>(g.read::<u32>(w(b)).wrapping_add(j));
            g.read::<u32>(u32::from(n).wrapping_mul(4).wrapping_add(w(t)))
        }
    };
    while i.cast_signed() < g.read::<i32>(count) {
        let n = g.read::<i32>(w(0x82));
        let mut found = 0u32;
        // (the start's first stack word, the entry or the last number
        // compared, is not read)
        if n > 0 {
            let want = u32::from(g.read::<u8>(g.read::<u32>(list).wrapping_add(i)));
            let mut j = 0u32;
            loop {
                let have = if j == u32::MAX {
                    g.read::<u32>(w(0xa7))
                } else {
                    u32::from(g.read::<u8>(g.read::<u32>(w(0xaf)).wrapping_add(j)))
                };
                if want == have {
                    found = j;
                    break;
                }
                j = j.wrapping_add(1);
                if j.cast_signed() >= n {
                    break;
                }
            }
        }
        let start = img.rel32_target(f + 0xde);
        let o = obj.wrapping_sub(0xd4);
        crate::s4_misc::start_object_first(g, img, start, o, k(g, 0xcc), owner);
        let a = actor(g, found, 0xec, 0xf3, 0xfe);
        let p = obj.wrapping_sub(4);
        st.push_all(&[obj + 4, obj, p, a]);
        st.call(g, slot_target(g, w(0x107), 262), fr.ecx);
        st.push_all(&[p, 0x11, a]);
        st.call(g, slot_target(g, w(0x11d), 268), fr.ecx);
        st.pop(7);
        g.write::<u32>(obj + 8, 0x3f80_0000);
        let v = sub(g.read::<f32>(obj), k(g, 0x141));
        g.write::<f32>(obj, v);
        let a = actor(g, found, 0x152, 0x159, 0x164);
        let t = slot(g, img, f, 0x16a, 333, fr.ecx, &[a]);
        let t = crate::m0018::st0_f32(t.st0);
        g.write::<f32>(ebp.wrapping_sub(0xc), t);
        let mut y = sub(k(g, 0x17d), t);
        if y > k(g, 0x18c) {
            y = sub(y, k(g, 0x196));
        } else if k(g, 0x1a0) > y {
            y = add(y, k(g, 0x1ad));
        }
        g.write::<f32>(obj.wrapping_sub(0x10), y);
        let b = if found == u32::MAX {
            g.read::<u32>(w(0x1bc))
        } else {
            u32::from(g.read::<u8>(g.read::<u32>(w(0x1c3)).wrapping_add(found)))
        };
        owner = owner.wrapping_add(0xf);
        for off in [4u32, 0, 0xffff_fffc, 8] {
            g.write::<u32>(rec.wrapping_add(off), 0);
        }
        g.write::<u32>(obj + 0x4c, b);
        g.write::<u16>(rec + 0x1c, 0);
        g.write::<u32>(rec + 0xc, w(0x1fd));
        g.write::<u32>(rec + 0x10, w(0x204));
        g.write::<u32>(rec + 0x14, 0);
        g.write::<u32>(rec + 0x18, 0);
        g.write::<u32>(obj + 0x3c, rec.wrapping_sub(0x14));
        obj = obj.wrapping_add(0x130);
        rec = rec.wrapping_add(0x34);
        i += 1;
    }
    free_to_live(
        g,
        img,
        f,
        [
            0x233, 0x242, 0x25f, 0x267, 0x298, 0x2a3, 0x2a9, 0x2b9, 0x2c7, 0x2d9, 0x2e2,
        ],
    );
    Ret::eax(0)
}

/// [`actors_objects_start`] (the `rel32`s:
/// [`part_init`](crate::l5_objhost::part_init) and
/// [`start_object_first`](crate::s4_misc::start_object_first)).
pub const ACTORS_OBJECTS_START: Sig = Sig {
    len: 748,
    hash: 0xe0c9_6ed6_38d7_d57d,
    calls: &[
        (0x3a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xde, Callee::Sig(&crate::s4_misc::START_OBJECT_FIRST)),
    ],
    imports: &[],
};

/// One step of a timed blend (`timer` counts down when above 0): the
/// weight `t = timer / length` (both words as floats) and `1 - t` (`K0`
/// the code's 1.0).
fn blend_weight<G: Game>(g: &mut G, at: u32, one: f32) -> (f32, f32) {
    let v = g.read::<i32>(at);
    if v > 0 {
        g.write::<i32>(at, v - 1);
    }
    let t = div(cvt(g.read::<u32>(at)), cvt(g.read::<u32>(at + 4)));
    (t, sub(one, t))
}

/// Three camera angle blends and four point blends sent (cdecl `(task)`,
/// `S = task[0x14]`; a /GS frame, `ebp` = `esp - 4` at the entry, the
/// matrices `M` (`ebp-0xa4`) and `M2` (`ebp-0x64`), the vector `V`
/// (`ebp-0x24`) and `U = (0, 0, 1, 1)` (`ebp-0x14`) locals). `M` = slot
/// 235. For each of three blocks `i` (0x38 bytes from `S+0x30`): its
/// timer gives `t` ([`blend_weight`]); its two angles go from `b` to `a`
/// as `(1 - t) b + a t`, `b` first brought round by a turn when the two
/// lie on the far sides of 0 more than `K1` apart, the result kept in
/// `-K1..K1` (by `K2`); `M2` = `M` turned by the two angles (slots 328,
/// 329), `V` = slot 229 `(V, M2, U)` and slot 590 `(i, V)`. Then the
/// block's point at `P = S+0xa8 + 0x38 i` (and a fourth after the
/// three) blends `+0x20..` into `+0x10..` by its own timer (`w` 1.0) and
/// goes to slot 589 `(i, P)`, whose last `eax` it returns.
///
/// `magic_0348` 0x10003330 (750 bytes; 5 DLLs).
pub fn camera_blends<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = g.read::<u32>(fr.arg(g, 0).wrapping_add(0x14));
    let ebp = fr.esp.wrapping_sub(4);
    let at = |d: u32| ebp.wrapping_sub(d);
    let (m, m2, v, u) = (at(0xa4), at(0x64), at(0x24), at(0x14));
    for (i, x) in (0u32..).zip([0, 0, 0x3f80_0000, 0x3f80_0000]) {
        g.write::<u32>(u + 4 * i, x);
    }
    slot(g, img, f, 0x24, 235, 0, &[m]);
    let one = k(g, 0x8c);
    let (k1, k2, k5) = (k(g, 0x56), k(g, 0x5e), k(g, 0xad));
    // a point blended by its own timer and sent
    let point = |g: &mut G, p: u32, i: u32, table: u32| {
        let (t, u) = blend_weight(g, p + 0x30, one);
        for c in 0..3u32 {
            let a = mul(g.read::<f32>(p + 0x20 + 4 * c), u);
            let b = mul(g.read::<f32>(p + 0x10 + 4 * c), t);
            g.write::<f32>(p + 4 * c, add(a, b));
        }
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        slot(g, img, f, table, 589, 0, &[i, p]).eax
    };
    let mut blk = s + 0x30;
    let mut p = s + 0xa8;
    for i in 0..3u32 {
        let (t, rest) = blend_weight(g, blk, one);
        for c in 0..2u32 {
            let a = g.read::<f32>(blk - 0x20 + 4 * c);
            let mut b = g.read::<f32>(blk - 0x10 + 4 * c);
            if 0.0 > a {
                if b > 0.0 && sub(b, a) > k1 {
                    b = sub(k5, sub(k1, b));
                }
            } else if 0.0 > b && sub(a, b) > k1 {
                b = add(add(b, k1), k1);
            }
            let mut x = add(mul(rest, b), mul(a, t));
            if x > k1 {
                x = sub(x, k2);
            } else if k5 > x {
                x = add(x, k2);
            }
            g.write::<f32>(blk - 0x30 + 4 * c, x);
        }
        let mut st = Pushes::default();
        let yaw = g.read::<u32>(blk - 0x30);
        st.push_all(&[yaw, m, m2]);
        st.call(g, slot_target(g, w(0x13c), 328), 0);
        st.pop(2);
        let pitch = g.read::<u32>(blk - 0x2c);
        st.set_top(pitch);
        st.push_all(&[m2, m2]);
        st.call(g, slot_target(g, w(0x15b), 329), 0);
        st.push_all(&[u, m2, v]);
        st.call(g, slot_target(g, w(0x174), 229), 0);
        st.push_all(&[v, i]);
        st.call(g, slot_target(g, w(0x185), 590), 0);
        blk += 0x38;
        point(g, p, i, 0x1d2);
        p += 0x38;
    }
    Ret::eax(point(g, p, 3, 0x283))
}

/// [`camera_blends`] (the `rel32`: `__security_check_cookie`).
pub const CAMERA_BLENDS: Sig = Sig {
    len: 750,
    hash: 0x4311_2989_63cf_417c,
    calls: &[(0x2e6, Callee::Any)],
    imports: &[],
};

/// A thrown object's flight step (internal: `ecx` = the flight `E`,
/// `edx` = the object's point `P`, the stack word `A` (three angles); a
/// /GS frame aligned to 8, `B` the frame after its pushes, the vector
/// `U` (`B+0x24`) and the matrix `M` (`B+0x34`) locals). The first time
/// (`E+0x28` 0): with `d = P+0x10..`, the pitch `atan2(dy, sqrt(dz dz +
/// dx dx))` and the yaw `K - atan2(dz, dx)` (brought back by `T` once
/// above `H` or below `L`); `M` a unit matrix (slot 235) turned by the
/// pitch times `K2` (slot 328) and the yaw (slot 329); `U = (0, 0, r -
/// K3, 1)` (`r` slot 314 `(10.0)`'s ST0) turned by `M` (slot 229 `(U, M,
/// U)`); `E` = `U`'s three terms, 0, the step `(U.x K4, 4.0, U.x K4, 0)`
/// and three random spins `rand() % 0x801 - 0x400` (u16 `+0x20..`),
/// `E+0x28 = 1`. Every time: `E` moves by its step, `P` by `E`, and `A`'s
/// three words by the spins (16 bits each). Returns `A[2]`.
///
/// `magic_0304` 0x100023b0 (767 bytes, `ecx`/`edx`; 2 DLLs).
pub fn flight_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let (e, p) = (fr.ecx, fr.edx);
    let a = fr.arg(g, 0);
    let b = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x7c);
    let (u, m) = (b + 0x24, b + 0x34);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    if g.read::<u32>(e + 0x28) == 0 {
        let dy = sub(rf(g, p + 0x14), 0.0);
        let dz = sub(rf(g, p + 0x18), 0.0);
        let dx = sub(rf(g, p + 0x10), 0.0);
        for (i, v) in (0u32..).zip([0, 0, 0, 0x3f80_0000]) {
            g.write::<u32>(u + 4 * i, v);
        }
        let s = add(mul(dz, dz), mul(dx, dx));
        // `_libm_sse2_sqrt_precise`, then `_CIatan2` on x87
        let h = f64::from(s).sqrt();
        let pitch = g.atan2(f64::from(dy), h);
        let yaw = g.atan2(f64::from(dz), f64::from(dx)) as f32;
        let mut y = sub(k(g, 0xe9), yaw);
        if y > k(g, 0xf8) {
            y = sub(y, k(g, 0x108));
        } else if k(g, 0x112) > y {
            y = add(y, k(g, 0x11f));
        }
        let pitch = mul(pitch as f32, k(g, 0x13b));
        let mut st = Pushes::default();
        st.push(m);
        st.call(g, slot_target(g, w(0x141), 235), a);
        st.set_top(pitch.to_bits());
        st.push_all(&[m, m]);
        st.call(g, slot_target(g, w(0x165), 328), a);
        st.pop(2);
        st.set_top(y.to_bits());
        st.push_all(&[m, m]);
        st.call(g, slot_target(g, w(0x186), 329), a);
        st.pop(2);
        st.set_top(0x4120_0000);
        let r = st.call(g, slot_target(g, w(0x193), 314), a);
        let r = sub(crate::m0018::st0_f32(r.st0), k(g, 0x1b7));
        g.write::<f32>(u + 8, r);
        st.push_all(&[u, m, u]);
        st.call(g, slot_target(g, w(0x1cb), 229), a);
        let x = rf(g, u);
        g.write::<f32>(e, x);
        let sx = mul(x, k(g, 0x1f7));
        let v = g.read::<u32>(u + 4);
        g.write::<u32>(e + 4, v);
        let v = g.read::<u32>(u + 8);
        g.write::<u32>(e + 8, v);
        g.write::<u32>(e + 0xc, 0);
        g.write::<f32>(e + 0x10, sx);
        g.write::<u32>(e + 0x14, 0x4080_0000);
        g.write::<f32>(e + 0x18, sx);
        g.write::<u32>(e + 0x1c, 0);
        for at in [0x20u32, 0x22, 0x24] {
            let r = g.rand().cast_signed() % 0x801 - 0x400;
            g.write::<u16>(e + at, r.cast_unsigned() as u16);
        }
        g.write::<u32>(e + 0x28, 1);
    }
    for c in [0u32, 4, 8] {
        let v = add(rf(g, e + 0x10 + c), rf(g, e + c));
        g.write::<f32>(e + c, v);
    }
    let v = add(rf(g, e), rf(g, p));
    g.write::<f32>(p, v);
    for c in [4u32, 8] {
        let v = add(rf(g, p + c), rf(g, e + c));
        g.write::<f32>(p + c, v);
    }
    let mut eax = 0;
    for (i, at) in (0u32..).zip([0x20u32, 0x22, 0x24]) {
        let v = u32::from(g.read::<u16>(e + at)).wrapping_add(g.read::<u32>(a + 4 * i)) & 0xffff;
        g.write::<u32>(a + 4 * i, v);
        eax = v;
    }
    eax
}

/// [`flight_step`] (the `rel32`s: the square root's and `_CIatan2`'s
/// thunks and `__security_check_cookie`; the import: `rand`, through a
/// register).
pub const FLIGHT_STEP: Sig = Sig {
    len: 767,
    hash: 0x0d07_77e6_f11c_2748,
    calls: &[
        (0x95, Callee::Import("_libm_sse2_sqrt_precise")),
        (0xa8, Callee::Import("_CIatan2")),
        (0xd7, Callee::Import("_CIatan2")),
        (0x301, Callee::Any),
    ],
    imports: &[(0x1eb, "rand")],
};
