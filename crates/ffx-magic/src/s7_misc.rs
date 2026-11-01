//! Shared functions of the shared-7 list (the rows of the shared-4 work
//! list that shared-6 left) without a family module yet. Each doc names
//! the DLL and address it was ported from, its size and how many DLLs
//! carry it; "`G`" is a global of the DLL (an operand of the code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
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
    let handlers: [(&Sig, fn(&mut G, &Image, &Frame) -> u32); 7] = [
        (&MOVING_LIGHT_NODE, moving_light_node),
        (&COUNT_FADE, count_fade),
        (&MODEL_DRAW_NODE, model_draw_node),
        (&LIGHT_NODE, light_node),
        (&HANDLES_REGISTERED, handles_registered),
        (&MODEL_POINTS_NODE, model_points_node),
        (&RING_KEY_FRAME, ring_key_frame),
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
///   - M[j] r`, `U[j+1] = (X[j+2] - X[j]) K - H[j] r`;
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
