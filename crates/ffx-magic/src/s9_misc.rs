//! Shared functions of the shared-9 list (round 6 of the shared work
//! list, `worklist5.tsv`: what the shared-8 round left) without a family
//! module yet. Each doc names the DLL and address it was ported from,
//! its size and how many DLLs carry it; "`G`" is a global of the DLL (an
//! operand of the code).

// The originals' float tests are `comiss`/`ucomiss` with their own
// unordered outcomes: `!(a >= b)` and exact `==` say what they do.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::float_cmp)]
// Long ports follow the original's order of stores and calls.
#![allow(clippy::too_many_lines)]

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::s8_misc::op;
use crate::{Callee, Frame, Game, Image, Out, Ret, Sig};

/// The original's run of constant stores to the DLL's globals from `f +
/// from` to `f + to`, done in order, their addresses and values read from
/// the instructions: `mov dword [G], imm32` (`c7 05`), `mov word [G],
/// imm16` (`66 c7 05`), `mov byte [G], imm8` (`c6 05`) and `mov [G], al`
/// (`a2`, the byte `al`). Returns the offset where the run stops (the
/// first other instruction).
fn global_stores<G: Game>(g: &mut G, img: &Image, f: u32, from: u32, to: u32, al: u8) -> u32 {
    let b = |o: u32| img.word(f + o) & 0xff;
    let mut at = from;
    while at < to {
        match (b(at), b(at + 1), b(at + 2)) {
            (0xc7, 0x05, _) => {
                g.write::<u32>(img.word(f + at + 2), img.word(f + at + 6));
                at += 10;
            }
            (0x66, 0xc7, 0x05) => {
                g.write::<u16>(img.word(f + at + 3), img.word(f + at + 7) as u16);
                at += 9;
            }
            (0xc6, 0x05, _) => {
                g.write::<u8>(img.word(f + at + 2), b(at + 6) as u8);
                at += 7;
            }
            (0xa2, _, _) => {
                g.write::<u8>(img.word(f + at + 1), al);
                at += 5;
            }
            _ => break,
        }
    }
    at
}

/// A summon's scene start (cdecl `(A)`): the targets
/// ([`targets_init_b`](crate::s4_summon::targets_init_b) of `A`), then the
/// DLL's node tree wired by its constant stores (three nodes after the
/// root, each first among its parent's children unless the parent test
/// fails, the count `*G_N` up by 3, the byte `*G_B` copied into three
/// nodes); the group of six immediates on the frame inserted
/// ([`node_insert`](crate::nodes::node_insert)); the effect `X` (the
/// immediate) registered (slot 718), its entry (slot 719 `(X, 8, 0)`)
/// kept in `*G_H` and handed to slot 711 `(entry, CB, 0)` with the lock
/// `*G_L` held at 1; slots 710 `(T, *G_H, D, 0x100000)`, 222 and 221
/// `(X, 0, 1000, 2)`, 551 `(slot 719 (X, 6, 1), 0, 2)`; then the start
/// ([`start_0083`](crate::m0078::start_0083)). Each call sees the words
/// the earlier ones left. Returns 0.
///
/// `magic_0086` 0x10006c50 (887 bytes; 2 DLLs).
pub fn scene_start_tree<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let tab = |g: &G, s: u32| slot_target(g, w(0x286), s);
    let ebp = fr.esp.wrapping_sub(4);
    // and esp, -8; sub esp, 0x1c; push esi
    let esp = (ebp & !7).wrapping_sub(0x20);
    let a = fr.arg(g, 0);
    crate::s4_summon::targets_init_b(g, img, img.rel32_target(f + 0xe), esp.wrapping_sub(4), a);
    let mut n = g.read::<u16>(w(0x14)).wrapping_add(1);
    let mut ecx = w(0x19);
    global_stores(g, img, f, 0x1f, 0x144, 0);
    global_stores(
        g,
        img,
        f,
        if ecx == w(0x146) { 0x14c } else { 0x158 },
        0x162,
        0,
    );
    ecx = w(0x163);
    n = n.wrapping_add(1);
    global_stores(g, img, f, 0x169, 0x1d6, 0);
    global_stores(
        g,
        img,
        f,
        if ecx == w(0x1d8) { 0x1de } else { 0x1ea },
        0x1f4,
        0,
    );
    n = n.wrapping_add(1);
    g.write::<u16>(w(0x1f8), n);
    let al = g.read::<u8>(w(0x1fd));
    global_stores(g, img, f, 0x205, 0x250, al);
    let group = esp.wrapping_add(8);
    for i in 0..6 {
        g.write::<u32>(group + 4 * i, w(0x254 + 8 * i));
    }
    let insert = Frame {
        at: img.rel32_target(f + 0x281),
        esp: esp.wrapping_sub(4),
        eax: u32::from(al),
        ecx: group,
        edx: fr.edx,
        xmm: [0; 4],
    };
    crate::nodes::node_insert(g, img, &insert);
    let mut st = Pushes::default();
    st.push(w(0x28b));
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, tab(g, 718), 0);
    st.pop(1);
    st.push_all(&[0, 8, w(0x2aa)]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let entry = st.call(g, slot_target(g, w(0x298), 719), 0).eax;
    st.pop(3);
    let lock = g.read::<u32>(w(0x2b2));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x2be), entry);
    st.push_all(&[0, w(0x2c3), entry]);
    g.write::<u32>(lock, 1);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, slot_target(g, w(0x2c9), 711), 0);
    let lock = g.read::<u32>(w(0x2dc));
    st.pop(3);
    g.write::<u32>(lock, saved);
    let h = g.read::<u32>(w(0x2fc));
    st.push_all(&[0x0010_0000, w(0x2f6), h, w(0x301)]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, slot_target(g, w(0x2e6), 710), 0);
    st.push_all(&[2, 0x3e8, 0, w(0x31c)]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, slot_target(g, w(0x308), 222), 0);
    st.push_all(&[2, 0x3e8, 0, w(0x337)]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, slot_target(g, w(0x323), 221), 0);
    st.push_all(&[1, 6, w(0x34d)]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let e2 = st.call(g, slot_target(g, w(0x33e), 719), 0).eax;
    st.pop(15);
    st.push_all(&[2, 0, e2]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, slot_target(g, w(0x35c), 551), 0);
    let start = Frame {
        at: img.rel32_target(f + 0x36c),
        esp: esp.wrapping_sub(4),
        eax: 0,
        ecx: fr.ecx,
        edx: fr.edx,
        xmm: [0; 4],
    };
    crate::m0078::start_0083(g, img, &start);
    Ret::eax(0)
}

/// [`scene_start_tree`] (the `rel32`s: the targets, the node insert and
/// the start).
pub const SCENE_START_TREE: Sig = Sig {
    len: 887,
    hash: 0xe66d_7c19_aba9_bf35,
    calls: &[
        (0xe, Callee::Sig(&crate::s4_summon::TARGETS_INIT_B)),
        (0x281, Callee::Sig(&crate::nodes::INSERT)),
        (0x36c, Callee::Sig(&crate::m0078::START_0086)),
    ],
    imports: &[],
};

/// The actor of entry `i` of a start's loop: `*G_NONE` for `i == -1` (the
/// loop never gets there), else `*(G_T + 4 * (*G_L)[i])` (the operands
/// `(none, list, table)`).
fn entry_actor<G: Game>(
    g: &G,
    img: &Image,
    f: u32,
    (none, list, table): (u32, u32, u32),
    i: u32,
) -> u32 {
    let w = |o: u32| op(img, f, o);
    if i == u32::MAX {
        return g.read::<u32>(w(none));
    }
    let n = g.read::<u8>(g.read::<u32>(w(list)).wrapping_add(i));
    g.read::<u32>(u32::from(n).wrapping_mul(4).wrapping_add(w(table)))
}

/// A summon's actors' paired objects started (cdecl, no arguments; a /GS
/// frame, `ebp` = `esp - 4` at the entry, the position `P` at
/// `ebp-0x14`): slot 719 `(X, 8, 0)` into `*G_H` (`*G_1`, `*G_2` set to
/// immediates first; two of its words stay on the stack), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)). For each of the `*G_N`
/// entries `i` (the actor `A` of the battle number `(*G_L)[i]`): slot
/// 262 `(A, P, P+4, P+8)`; the object `O = B0 + 0x260 i` started at
/// scale `K` ([`start_object_at`](crate::s4_misc::start_object_at),
/// index 1, owner `15 i`); slot 262 `(A, C, C+4, C+8)` and slot 268 `(A,
/// 0x11, C)` for the point `C = C0 + 0x260 i`, `C+0xc = 1.0`, `O+0xd4 =
/// P.y - K2`, the started object's `+0x64` scaled by `K3`; the record `E
/// = E0 + 0x34 i` cleared (`+0x20`, `+0x24` immediates) into `O+0x110`;
/// the second object `O+0x130` started (index 0, owner `15 i + 0x16`,
/// callback `CB`); the same two slots for `C+0x130`, `O+0x204 = P.y -
/// K4`, `O+0x250` = the battle number. Then a task node is taken and
/// listed ([`free_to_live`](crate::s7_misc::free_to_live)). The original
/// pushes the `ecx` slot 262 leaves as the first start's third word,
/// which [`start_object_at`](crate::s4_misc::start_object_at) does not
/// read. Returns 0.
///
/// `magic_0114` 0x10001370 (835 bytes; 2 DLLs).
pub fn paired_objects_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x14);
    let mut st = Pushes::default();
    st.push_all(&[0, 8, w(0x22)]);
    g.write::<u32>(w(0x28), w(0x2c));
    g.write::<u32>(w(0x32), w(0x36));
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let h = st.call(g, slot_target(g, w(0x11), 719), fr.ecx).eax;
    st.pop(1);
    g.write::<u32>(w(0x40), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x45));
    st.pop(2);
    let start = img.rel32_target(f + 0xce);
    let mut owner = 0u32;
    let mut obj = w(0x4f);
    let mut c = w(0x6e);
    let (mut c4, mut c8) = (w(0x75), w(0x7c));
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0x5c)) {
        let a = entry_actor(g, img, f, (0x86, 0x68, 0x93), i);
        st.push_all(&[p + 8, p + 4, p, a]);
        // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
        st.call(g, slot_target(g, w(0xa5), 262), p);
        st.pop(4);
        let scale = k(g, 0xb5);
        let o = crate::s4_misc::start_object_at(g, img, start, obj, scale, 1, owner, 0);
        g.write::<u32>(ebp.wrapping_sub(8), 0x3f80_0000);
        // the object and its point
        let a = entry_actor(g, img, f, (0xde, 0xe5, 0xf0), i);
        st.push_all(&[c8, c4, c, a]);
        // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
        st.call(g, slot_target(g, w(0x100), 262), fr.ecx);
        st.push_all(&[c, 0x11, a]);
        // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
        st.call(g, slot_target(g, w(0x10d), 268), fr.ecx);
        let y = sub(g.read::<f32>(p + 4), k(g, 0x128));
        g.write::<u32>(c + 0xc, 0x3f80_0000);
        let scale = k(g, 0x13a);
        c8 = c8.wrapping_add(0x130);
        g.write::<f32>(obj + 0xd4, y);
        let v = mul(g.read::<f32>(o + 0x64), k(g, 0x156));
        c4 = c4.wrapping_add(0x130);
        st.pop(7);
        g.write::<f32>(o + 0x64, v);
        // the record
        let e = i.wrapping_mul(0x34).wrapping_add(w(0x17a));
        g.write::<u32>(i.wrapping_mul(0x34).wrapping_add(w(0x170)), 0);
        for off in [0x14u32, 0x10, 0x1c] {
            g.write::<u32>(e.wrapping_add(off), 0);
        }
        g.write::<u32>(e.wrapping_add(0x20), w(0x198));
        g.write::<u32>(e.wrapping_add(0x24), w(0x19f));
        g.write::<u32>(e.wrapping_add(0x28), 0);
        g.write::<u32>(e.wrapping_add(0x2c), 0);
        g.write::<u16>(e.wrapping_add(0x30), 0);
        g.write::<u32>(obj + 0x110, e);
        // the second object and its point
        c = c.wrapping_add(0x130);
        let o2 = obj.wrapping_add(0x130);
        crate::s4_misc::start_object_at(
            g,
            img,
            start,
            o2,
            scale,
            0,
            owner.wrapping_add(0x16),
            w(0x1bf),
        );
        let a = entry_actor(g, img, f, (0x1e3, 0x1ea, 0x1f5), i);
        st.push_all(&[c8, c4, c, a]);
        // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
        st.call(g, slot_target(g, w(0x205), 262), fr.ecx);
        st.push_all(&[c, 0x11, a]);
        // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
        st.call(g, slot_target(g, w(0x212), 268), fr.ecx);
        let y = sub(g.read::<f32>(p + 4), k(g, 0x22d));
        let list = g.read::<u32>(w(0x233));
        let none = g.read::<u32>(w(0x238));
        st.pop(7);
        g.write::<u32>(c + 0xc, 0x3f80_0000);
        g.write::<f32>(obj + 0x204, y);
        let b = if i == u32::MAX {
            none
        } else {
            u32::from(g.read::<u8>(list.wrapping_add(i)))
        };
        c8 = c8.wrapping_add(0x130);
        c4 = c4.wrapping_add(0x130);
        owner = owner.wrapping_add(0xf);
        g.write::<u32>(obj + 0x250, b);
        i = i.wrapping_add(1);
        obj = obj.wrapping_add(0x260);
        c = c.wrapping_add(0x130);
    }
    crate::s7_misc::free_to_live(
        g,
        img,
        f,
        [
            0x28c, 0x29b, 0x2b8, 0x2c0, 0x2f1, 0x2fc, 0x302, 0x313, 0x334, 0x326, 0x334,
        ],
    );
    Ret::eax(0)
}

/// [`paired_objects_start`] (the `rel32`s: the part set-up, the object
/// starts and `__security_check_cookie`).
pub const PAIRED_OBJECTS_START: Sig = Sig {
    len: 835,
    hash: 0xd84b_cc69_62a1_c2df,
    calls: &[
        (0x45, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xce, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x1d6, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x33b, Callee::Any),
    ],
    imports: &[],
};

/// [`summon_load_objects`](crate::s7_misc::summon_load_objects)'s copy
/// that starts [`paired_objects_start`].
///
/// `magic_0114` 0x10001210 (119 bytes; 2 DLLs).
pub fn summon_load_paired<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::s7_misc::load_then_start(g, img, fr, paired_objects_start)
}

/// [`summon_load_paired`] (the `rel32`: [`paired_objects_start`]).
pub const SUMMON_LOAD_PAIRED: Sig = Sig {
    len: 119,
    hash: 0x2da6_1889_6875_2369,
    calls: &[(0x5c, Callee::Sig(&PAIRED_OBJECTS_START))],
    imports: &[],
};

/// Calls the internal port `port` of the function at `target` as the
/// original does: `args` pushed below `esp` (the first one on top), the
/// return address below them, `ecx` and `edx` as given.
fn sub_call<G: Game>(
    g: &mut G,
    port: fn(&mut G, &Image, &Frame) -> Ret,
    img: &Image,
    (target, esp, ecx, edx): (u32, u32, u32, u32),
    args: &[u32],
) -> Ret {
    let top = esp.wrapping_sub(4 * args.len() as u32);
    for (i, &x) in (0u32..).zip(args) {
        g.write::<u32>(top + 4 * i, x);
    }
    let fr = Frame {
        at: target,
        esp: top.wrapping_sub(4),
        eax: 0,
        ecx,
        edx,
        xmm: [0; 4],
    };
    port(g, img, &fr)
}

/// The reel game's draw slots: colour (811), vertex (809), end (812).
struct Lines {
    colour: u32,
    vertex: u32,
    end: u32,
}

impl Lines {
    fn new<G: Game>(g: &G, table: u32) -> Self {
        Lines {
            colour: slot_target(g, table, 811),
            vertex: slot_target(g, table, 809),
            end: slot_target(g, table, 812),
        }
    }

    /// One line from `a` to `b` (screen floats) in the colours `c` at `a`
    /// and `c2` at `b` (the three components and the alpha) and with the
    /// end's words `e`: slot 811 `(c, 0)`, 809 `(a, 0)`, 811 `(c2, 1)`,
    /// 809 `(b, 1)` and 812 `e`. Returns slot 812's `eax`.
    fn line<G: Game>(
        &self,
        g: &mut G,
        (c, c2): ([u32; 4], [u32; 4]),
        a: [f32; 2],
        b: [f32; 2],
        e: [u32; 4],
    ) -> u32 {
        g.call(self.colour, 0, &[c[0], c[1], c[2], c[3], 0]);
        g.call(self.vertex, 0, &[a[0].to_bits(), a[1].to_bits(), 0]);
        g.call(self.colour, 0, &[c2[0], c2[1], c2[2], c2[3], 1]);
        g.call(self.vertex, 0, &[b[0].to_bits(), b[1].to_bits(), 1]);
        g.call(self.end, 0, &e).eax
    }
}

/// Two outlined boxes (internal: `ecx` = `x`, `edx` = `y`, the stack `(w,
/// h, C1, C2, A)`): the outer box from `(x, y)` to `(x + w, y + h)` (the
/// grid units `16 n + 0x6ff8` across and `16 n + 0x72f8` down, made screen
/// integers by `K1`, `K2` and `KX`/`KY`:
/// [`screen_int`](crate::s4_fam446::screen_int)) in the grey `C1`, and the
/// inner one a unit (`0x10`) inside it in `C2`, both with the alpha `A`;
/// each edge a line one pixel (`KP`) short at its far end, the outer box's
/// clockwise and the inner box's counter-clockwise. The original keeps
/// `16 (y + h)` in its `h` word and the inner box's bottom (an integer) in
/// its `w` word. Returns the last slot 812's `eax`.
///
/// `magic_0565` 0x10002300 (1,777 bytes; 10 DLLs).
pub fn reel_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::s4_fam446::screen_int;
    use crate::sb_sse::{add, cvt, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let (x, y) = (fr.ecx, fr.edx);
    let [wd, ht, c1, c2, alpha] = [0, 1, 2, 3, 4].map(|i| fr.arg(g, i));
    let lines = Lines::new(g, w(0xce));
    let x16 = x << 4;
    let y16 = y << 4;
    let r16 = wd.wrapping_add(x) << 4;
    let b16 = ht.wrapping_add(y) << 4;
    g.write::<u32>(fr.esp + 8, b16);
    let at = |g: &G, n: u32, k: u32| screen_int(g, img, f, n, [0xa, 0x12, k]);
    let kp = g.read::<f32>(w(0x143));
    let ends = [0, 0, 1, 0];
    // the outer box
    let x0 = cvt(at(g, x16.wrapping_add(0x6ff8), 0x46));
    let y0 = cvt(at(g, y16.wrapping_add(0x72f8), 0x76));
    let x1 = cvt(at(g, r16.wrapping_add(0x6ff8), 0xa1));
    let bottom = at(g, b16.wrapping_add(0x72f8), 0xbc);
    g.write::<u32>(fr.esp + 4, bottom);
    let y1 = cvt(bottom);
    let c = ([c1, c1, c1, alpha], [c1, c1, c1, alpha]);
    lines.line(g, c, [x0, y0], [x1, add(y0, kp)], ends);
    lines.line(g, c, [x1, y0], [sub(x1, kp), y1], ends);
    lines.line(g, c, [x1, y1], [x0, sub(y1, kp)], ends);
    lines.line(g, c, [x0, y1], [add(x0, kp), y0], ends);
    // the inner box
    let x0 = cvt(at(g, x16.wrapping_add(0x7008), 0x3bd));
    let y0 = cvt(at(g, y16.wrapping_add(0x7308), 0x3e0));
    let x1 = cvt(at(g, r16.wrapping_add(0x6fe8), 0x403));
    let bottom = at(g, b16.wrapping_add(0x72e8), 0x41e);
    g.write::<u32>(fr.esp + 4, bottom);
    let y1 = cvt(bottom);
    let c = ([c2, c2, c2, alpha], [c2, c2, c2, alpha]);
    lines.line(g, c, [x0, y0], [x1, sub(y0, kp)], ends);
    lines.line(g, c, [x1, y0], [add(x1, kp), y1], ends);
    lines.line(g, c, [x1, y1], [x0, add(y1, kp)], ends);
    let eax = lines.line(g, c, [x0, y1], [sub(x0, kp), y0], ends);
    Ret::eax(eax)
}

/// [`reel_frame`].
pub const REEL_FRAME: Sig = Sig {
    len: 1777,
    hash: 0x3c5e_7574_9933_1e81,
    calls: &[],
    imports: &[],
};

/// The three reels' current glyphs and their frame (internal: `ecx` = the
/// reels `R` (0x1c bytes each), `edx` = the row `y`): `R+0x6c` counts down
/// to 0, the alpha `A = (8 - R[0x6c]) * 128 / 8`; for each reel `i` the
/// glyph after its position (the u16 `+6` plus one, 0 past the strip's
/// length `*R[0]`), `(*R[0])[1][..]`, drawn at the row `y + 0x28 i`
/// ([`glyph_quad`](crate::s4_fam446::glyph_quad) `(0x4e0, 0x28, 0x19,
/// 0x80, A)`); then the frame ([`reel_frame`] at `(y - 2, 0x4c)`, `(0x7c,
/// 0x1d, 0x40, 0x20, A)`), whose `eax` it returns.
///
/// `magic_0565` 0x10002de0 (156 bytes; 10 DLLs).
pub fn reel_next_glyphs<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let (reels, row) = (fr.ecx, fr.edx);
    let left = g.read::<i32>(reels + 0x6c);
    if left > 0 {
        g.write::<i32>(reels + 0x6c, left - 1);
    }
    let alpha = (8i32.wrapping_sub(g.read::<i32>(reels + 0x6c)) << 7) / 8;
    let alpha = alpha.cast_unsigned();
    let esp = fr.esp.wrapping_sub(0x18);
    let quad = img.rel32_target(f + 0x67);
    for i in 0..3u32 {
        let r = reels + 0x1c * i;
        let strip = g.read::<u32>(r);
        let mut at = i32::from(g.read::<i16>(r + 6)) + 1;
        if at >= g.read::<i32>(strip) {
            at = 0;
        }
        let bytes = g.read::<u32>(strip + 4);
        let glyph = u32::from(g.read::<u8>(bytes.wrapping_add(at.cast_unsigned())));
        let y = row.wrapping_add(0x28 * i);
        sub_call(
            g,
            crate::s4_fam446::glyph_quad,
            img,
            (quad, esp, glyph, y),
            &[0x4e0, 0x28, 0x19, 0x80, alpha],
        );
    }
    let frame = img.rel32_target(f + 0x8e);
    let at = (frame, esp, row.wrapping_sub(2), 0x4c);
    sub_call(g, reel_frame, img, at, &[0x7c, 0x1d, 0x40, 0x20, alpha])
}

/// [`reel_next_glyphs`] (the `rel32`s: the glyph quad and the frame).
pub const REEL_NEXT_GLYPHS: Sig = Sig {
    len: 156,
    hash: 0xa717_705d_1db8_0132,
    calls: &[
        (0x67, Callee::Sig(&crate::s4_fam446::GLYPH_QUAD)),
        (0x8e, Callee::Sig(&REEL_FRAME)),
    ],
    imports: &[],
};

/// A reel's window (internal: `ecx` = the column `x`, `edx` = the row
/// `y`; the screen integers as [`reel_frame`]'s): its top and bottom
/// shades, two lines each from the left edge `16 x + 0x6ff8` to `16 x +
/// 0x7368`: from `16 y + 0x72f8` in black with the alpha 0x70 to `+0x76b8`
/// clear (slot 812 `(0, 0, 1, 2)`), from `+0x75d8` clear to `+0x7998` with
/// 0x70; then the reel's frame ([`reel_frame`] at `(x, y)`, `(0x37, 0x69,
/// 0xc0, 0x50, 0x80)`), whose `eax` it returns.
///
/// `magic_0565` 0x10002a00 (640 bytes; 10 DLLs).
pub fn reel_window<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::s4_fam446::screen_int;
    use crate::sb_sse::cvt;
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let (x, y) = (fr.ecx, fr.edx);
    let lines = Lines::new(g, w(0x56));
    let at = |g: &G, n: u32, k: u32| cvt(screen_int(g, img, f, n, [0x2a, 0x36, k]));
    let x16 = x << 4;
    let y16 = y << 4;
    let left = at(g, x16.wrapping_add(0x6ff8), 0x46);
    let right = at(g, x16.wrapping_add(0x7368), 0x77);
    let ends = [0, 0, 1, 2];
    let (dark, clear) = ([0, 0, 0, 0x70], [0, 0, 0, 0]);
    let top = at(g, y16.wrapping_add(0x72f8), 0xb4);
    let low = at(g, y16.wrapping_add(0x76b8), 0x125);
    lines.line(g, (dark, clear), [left, top], [right, low], ends);
    let top = at(g, y16.wrapping_add(0x75d8), 0x1a4);
    let low = at(g, y16.wrapping_add(0x7998), 0x215);
    lines.line(g, (clear, dark), [left, top], [right, low], ends);
    let esp = fr.esp.wrapping_sub(0x2c);
    let frame = (img.rel32_target(f + 0x272), esp, x, y);
    sub_call(g, reel_frame, img, frame, &[0x37, 0x69, 0xc0, 0x50, 0x80])
}

/// [`reel_window`] (the `rel32`: [`reel_frame`]).
pub const REEL_WINDOW: Sig = Sig {
    len: 640,
    hash: 0xb6dd_63c8_20d6_8064,
    calls: &[(0x272, Callee::Sig(&REEL_FRAME))],
    imports: &[],
};

/// One reel drawn (internal: `ecx` = the reel `R`, `edx` = the row `y`,
/// the stack `(x)`): the bounce `R+0x14` goes up by one (past 3 it starts
/// again at -3); slot 813 `(0, top, 512.0, bottom - KB)` clips to the
/// window (the screen floats of `16 x + 0x72f8` and `+0x7998`); four
/// glyphs of the strip `*R[0]` from the position `R[4] >> 16` (wrapping at
/// the strip's length) at the columns `16 x - (u16 R[4]) * 0x23 / 4096 +
/// 0x230 k` ([`glyph_quad`](crate::s4_fam446::glyph_quad) `(.., 0x37,
/// 0x23, colour, 0x80)`, the colour 0x80, or for glyph 10 `(0x2d0 - 0x70
/// |bounce|) / 3`); slot 814; then the window ([`reel_window`] at `(y,
/// x)`), whose `eax` it returns.
///
/// `magic_0565` 0x10002c80 (343 bytes; 10 DLLs).
pub fn reel_draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::s4_fam446::screen_f32;
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let (r, row) = (fr.ecx, fr.edx);
    let x = fr.arg(g, 0);
    let strip = g.read::<u32>(r);
    let bytes = g.read::<u32>(strip + 4);
    let pos = g.read::<u32>(r + 4);
    let bounce = g.read::<i32>(r + 0x14).wrapping_add(1);
    g.write::<i32>(r + 0x14, bounce);
    let x16 = x << 4;
    let mut col = x16.wrapping_sub(((pos & 0xffff) * 0x23) >> 12);
    let bounce = if bounce > 3 { -3 } else { bounce };
    g.write::<i32>(r + 0x14, bounce);
    let mut at = pos.cast_signed() >> 16;
    let ks = [0x5d, 0x6b, 0x76];
    let low = screen_f32(g, img, f, x16.wrapping_add(0x7998), ks);
    let low = sub(low, g.read::<f32>(w(0x7e)));
    let top = screen_f32(g, img, f, x16.wrapping_add(0x72f8), [0x9e, 0xae, 0xb6]);
    let clip = slot_target(g, w(0x8d), 813);
    g.call(clip, 0, &[0, top.to_bits(), 0x4400_0000, low.to_bits()]);
    let esp = fr.esp.wrapping_sub(0x28);
    let quad = img.rel32_target(f + 0x118);
    for _ in 0..4 {
        let glyph = u32::from(g.read::<u8>(at.cast_unsigned().wrapping_add(bytes)));
        let colour = if glyph == 10 {
            let b = g.read::<i32>(r + 0x14).wrapping_abs();
            (0x2d0i32.wrapping_sub(b.wrapping_mul(0x70)) / 3).cast_unsigned()
        } else {
            0x80
        };
        let words = [col, 0x37, 0x23, colour, 0x80];
        sub_call(
            g,
            crate::s4_fam446::glyph_quad,
            img,
            (quad, esp, glyph, row),
            &words,
        );
        at = at.wrapping_add(1);
        col = col.wrapping_add(0x230);
        if at >= g.read::<i32>(strip) {
            at = 0;
        }
    }
    g.call(slot_target(g, w(0x139), 814), 0, &[]);
    let window = (img.rel32_target(f + 0x14c), esp, row, x);
    sub_call(g, reel_window, img, window, &[])
}

/// [`reel_draw`] (the `rel32`s: the glyph quad and the window).
pub const REEL_DRAW: Sig = Sig {
    len: 343,
    hash: 0x121f_0168_81da_d198,
    calls: &[
        (0x118, Callee::Sig(&crate::s4_fam446::GLYPH_QUAD)),
        (0x14c, Callee::Sig(&REEL_WINDOW)),
    ],
    imports: &[],
};

/// The reel game drawn (cdecl, no arguments; a frame aligned to 8): for
/// the texture ids 12 to 22, slot 209 `(slot 719 (T, 2, id), 0)` and slot
/// 208 `(slot 719 (T, 3, id), 0)`; then for each of the `*G_N` machines
/// `m` (0x78 bytes from `G_M`) whose `+0x18` is 1, its three reels
/// ([`reel_draw`] of `m - 0x5c + 0x1c i` at the row `m[-4] + 0x3f i` and
/// the column `m[0]`); then for the first `*G_N - 1` machines `i` below
/// `*G_K`, their next glyphs ([`reel_next_glyphs`] of `G_R + 0x78 i` at
/// the row `0x41 + 0x82 i`). Returns `*G_N - 1` (`*G_N` as last read).
///
/// `magic_0565` 0x10002e80 (265 bytes; 10 DLLs).
pub fn reels_draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let tex = w(0x1f);
    for id in 0xc..=0x16u32 {
        let h = g.call(slot_target(g, w(0x11), 719), 0, &[tex, 2, id]).eax;
        g.call(slot_target(g, w(0x2c), 209), 0, &[h, 0]);
        let h = g.call(slot_target(g, w(0x39), 719), 0, &[tex, 3, id]).eax;
        g.call(slot_target(g, w(0x54), 208), 0, &[h, 0]);
    }
    // ebp = esp - 4 aligned to 8; ecx, ebx, esi, edi pushed, one word
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x10);
    let mut n = g.read::<i32>(w(0x6b));
    let mut m = w(0x7a);
    let mut i = 0i32;
    while i < n {
        if g.read::<u32>(m + 0x18) == 1 {
            let draw = img.rel32_target(f + 0x9a);
            let mut reel = m.wrapping_sub(0x5c);
            for k in 0..3u32 {
                let row = g.read::<u32>(m.wrapping_sub(4)).wrapping_add(0x3f * k);
                let x = g.read::<u32>(m);
                sub_call(g, reel_draw, img, (draw, esp, reel, row), &[x]);
                reel = reel.wrapping_add(0x1c);
            }
            n = g.read::<i32>(w(0xb1));
        }
        i += 1;
        m = m.wrapping_add(0x78);
    }
    let mut reels = w(0xcb);
    let mut row = 0x41u32;
    let mut k = 0i32;
    while k < n.wrapping_sub(1) {
        if k < g.read::<i32>(w(0xd8)) {
            let next = img.rel32_target(f + 0xe4);
            sub_call(
                g,
                reel_next_glyphs,
                img,
                (next, esp, reels, row),
                &[n.cast_unsigned()],
            );
            n = g.read::<i32>(w(0xea));
        }
        k += 1;
        reels = reels.wrapping_add(0x78);
        row = row.wrapping_add(0x82);
    }
    Ret::eax(n.wrapping_sub(1).cast_unsigned())
}

/// [`reels_draw`] (the `rel32`s: [`reel_draw`] and
/// [`reel_next_glyphs`]).
pub const REELS_DRAW: Sig = Sig {
    len: 265,
    hash: 0x1583_4688_930b_f92f,
    calls: &[
        (0x9a, Callee::Sig(&REEL_DRAW)),
        (0xe4, Callee::Sig(&REEL_NEXT_GLYPHS)),
    ],
    imports: &[],
};

/// The packet words a sprite record's three colours go to (each block of 8
/// words from there is copied `0xc0` further).
const PACKET_COLOURS: [u32; 3] = [0x60, 0x90, 0xc0];

/// A model's sprite records written as GPU packets (internal: `ecx` = the
/// model `S`, the stack `(a, b, k)`; a /GS frame, `ebp` = `esp - 4`, the
/// matrix `M` at `ebp-0x48`): `M` = slot 235, slot 554 `(M, M, G_V)`;
/// then the groups from `S + S[4]` (`i16 S[0x10]` records in all): each
/// group of type 3 (byte `+1`; any other ends the function without the
/// store below) has `i16 +2` records of 0x28 bytes from `+0x10`, and each
/// record fills a 0x1a0-byte packet at `P = *G_P`: its constant words,
/// `P + 0x40` = the group's `+8`, `+0xc`, `P + 0x100` = `(a, b)`, and three
/// colours (bytes `0..4`, `4..8`, `8..12` of the record, the alpha scaled
/// by `k / 128`) at `P + 0x60`, `+0x90`, `+0xc0`, each block of 8 words
/// copied `0xc0` further. `*G_P` = the packet after the last. Returns
/// what the original leaves in `eax` (the last count read).
///
/// The original's depth sort (slot 581 with the depth from `M`) runs only
/// for a packet at address 0, which the packet's own stores fault on
/// before; the port leaves it out, and with it the stale stack word
/// `[ebp-0x70]` that decides only that path's argument.
///
/// `magic_0084` 0x10001fd0 (938 bytes; 10 DLLs).
pub fn sprite_packets<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let s = fr.ecx;
    let [a, b, k] = [0, 1, 2].map(|i| fr.arg(g, i));
    let ebp = fr.esp.wrapping_sub(4);
    let m = ebp.wrapping_sub(0x48);
    let mut base = g.read::<u32>(s + 4).wrapping_add(s);
    let mut p = g.read::<u32>(w(0x1a));
    g.call(slot_target(g, w(0x28), 235), 0, &[m]);
    g.call(slot_target(g, w(0x3f), 554), 0, &[m, m, w(0x38), m]);
    let mut left = i32::from(g.read::<i16>(s + 0x10));
    let mut eax = left.cast_unsigned();
    if left > 0 {
        loop {
            if g.read::<u8>(base + 1) != 3 {
                return Ret::eax(eax);
            }
            let mut rec = base + 0x10;
            let mut i = 0i32;
            while i < i32::from(g.read::<i16>(base + 2)) {
                packet(g, p, base, rec, (a, b, k));
                i += 1;
                rec = rec.wrapping_add(0x28);
                p = p.wrapping_add(0x1a0);
            }
            let n = i32::from(g.read::<i16>(base + 2));
            eax = n.cast_unsigned();
            left = left.wrapping_sub(n);
            base = rec;
            if left <= 0 {
                break;
            }
        }
    }
    g.write::<u32>(w(0x395), p);
    Ret::eax(eax)
}

/// One record's packet for [`sprite_packets`] (the group `grp`, the record
/// `rec`, the packet `p`).
fn packet<G: Game>(g: &mut G, p: u32, grp: u32, rec: u32, (a, b, k): (u32, u32, u32)) {
    for (o, x) in [
        (0x18u32, 0x0240_268e_u32),
        (0x1c, 0x4024),
        (0x20, 0x44),
        (0x24, 0x80),
        (0x28, 0x42),
        (0x2c, 0),
        (0x30, 0),
        (0x34, 0),
    ] {
        g.write::<u32>(p + o, x);
    }
    let (lo, hi) = (g.read::<u32>(grp + 8), g.read::<u32>(grp + 0xc));
    g.write::<u32>(p + 0x44, hi);
    g.write::<u32>(p + 0x40, lo);
    for (o, x) in [
        (0xe8u32, 0x4024_026e_u32),
        (0xec, 0x402),
        (0xf0, 0x48),
        (0xf4, 0x80),
        (0xf8, 0x42),
        (0xfc, 0),
    ] {
        g.write::<u32>(p + o, x);
    }
    g.write::<u32>(p + 0x100, a);
    g.write::<u32>(p + 0x104, b);
    for (j, &at) in (0u32..).zip(&PACKET_COLOURS) {
        let c = rec + 4 * j;
        for i in 0..3 {
            g.write::<u32>(p + at + 4 * i, u32::from(g.read::<u8>(c + i)));
        }
        let alpha = i32::from(g.read::<u8>(c + 3)).wrapping_mul(k.cast_signed()) / 128;
        g.write::<i32>(p + at + 0xc, alpha);
        for i in 0..8 {
            let v = g.read::<u32>(p + at + 4 * i);
            g.write::<u32>(p + at + 0xc0 + 4 * i, v);
        }
    }
}

/// [`sprite_packets`] (the `rel32`: `__security_check_cookie`).
pub const SPRITE_PACKETS: Sig = Sig {
    len: 938,
    hash: 0x8163_28b4_7dd1_13dd,
    calls: &[(0x3a2, Callee::Any)],
    imports: &[],
};

/// Where [`model_sprites`]'s copies keep their operands: the slot table,
/// the flag (`push imm8`), the texture `T`, the i16 field `F` of the
/// object, the global `G_X`, the model's offset `M` (a `disp8` or a
/// `disp32`), the sprite packets' `rel32`, the lock `G_L`, the offset `D`
/// added for slot 815 (an `imm8` or `imm32`).
pub struct ModelSprites {
    table: u32,
    flag: u32,
    tex: u32,
    field: u32,
    gx: u32,
    model: u32,
    call: u32,
    lock: u32,
    add: u32,
    wide: bool,
}

/// `magic_0084` 0x100017c0 (`disp8`s).
const SPRITES_A: ModelSprites = ModelSprites {
    table: 0x1b,
    flag: 0x2b,
    tex: 0x2f,
    field: 0x38,
    gx: 0x49,
    model: 0x54,
    call: 0x56,
    lock: 0xa1,
    add: 0xcc,
    wide: false,
};

/// `magic_0084` 0x100018c0, `magic_0085` 0x100034b0, `magic_0093`
/// 0x100034e0 (`disp32`s).
const SPRITES_B: ModelSprites = ModelSprites {
    table: 0x1b,
    flag: 0x2b,
    tex: 0x2f,
    field: 0x38,
    gx: 0x49,
    model: 0x54,
    call: 0x59,
    lock: 0xa7,
    add: 0xd2,
    wide: true,
};

/// `magic_0440` 0x100034d0 (`disp8`s, after the four stores).
const SPRITES_C: ModelSprites = ModelSprites {
    table: 0x23,
    flag: 0x2d,
    tex: 0x5d,
    field: 0x66,
    gx: 0x77,
    model: 0x82,
    call: 0x84,
    lock: 0xcf,
    add: 0xfa,
    wide: false,
};

/// `magic_0440` 0x10003600 (`disp32`s, after the four stores).
const SPRITES_D: ModelSprites = ModelSprites {
    table: 0x23,
    flag: 0x2d,
    tex: 0x5d,
    field: 0x66,
    gx: 0x77,
    model: 0x82,
    call: 0x87,
    lock: 0xd5,
    add: 0x100,
    wide: true,
};

/// A model's sprites drawn (cdecl `(O)`; a /GS frame aligned to 16, `A`
/// = `ebp & !15`, the draw record `R` at `A - 0x80`): the texture handle
/// `H` = slot 719 `(T, 2, flag)`; the model `m = *(*(**G_X + 0x20) + M)`
/// and its packets ([`sprite_packets`] `(H[0], H[1], O.F >> 7)`, `ecx` =
/// `m`); `R` gets `+0x1b` = the byte `O.F >> 7` (16-bit), u16 `+0x18` =
/// 0xffff, `+0x1a` = 0xff, `+0x10`, `+0x14`, `+0`, `+0x44` = 0, byte `+8` =
/// `m[0x16]`, `+0x20` = `O + 0x50` (its other words are the original's
/// stale stack); slot 815 `(O, *(**G_X + 0x20) + D, R)` with the lock
/// `**G_L` held at 1. Returns `*G_L`.
fn model_sprites<G: Game>(g: &mut G, img: &Image, fr: &Frame, l: &ModelSprites) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    // the `disp8`/`imm8` operands are sign-extended
    let imm = |o: u32| {
        if l.wide {
            w(o)
        } else {
            i32::from((img.word(f + o) as u8).cast_signed()).cast_unsigned()
        }
    };
    let obj = fr.arg(g, 0);
    let a = fr.esp.wrapping_sub(4) & !0xf;
    let flag = w(l.flag) & 0xff;
    let h = g
        .call(slot_target(g, w(l.table), 719), 0, &[w(l.tex), 2, flag])
        .eax;
    let field = obj.wrapping_add(w(l.field));
    let k = (i32::from(g.read::<i16>(field)) >> 7).cast_unsigned();
    let words = [g.read::<u32>(h), g.read::<u32>(h.wrapping_add(4)), k];
    let gx = w(l.gx);
    let c = g.read::<u32>(g.read::<u32>(g.read::<u32>(gx)).wrapping_add(0x20));
    let model = g.read::<u32>(c.wrapping_add(imm(l.model)));
    let packets = (img.rel32_target(f + l.call), a.wrapping_sub(0x94), model, 0);
    sub_call(g, sprite_packets, img, packets, &words);
    let r = a.wrapping_sub(0x80);
    g.write::<u8>(r + 0x1b, (g.read::<i16>(field) >> 7).to_le_bytes()[0]);
    g.write::<u16>(r + 0x18, 0xffff);
    g.write::<u8>(r + 0x1a, 0xff);
    g.write::<u32>(r + 0x10, 0);
    g.write::<u32>(r + 0x14, 0);
    let c = g.read::<u32>(g.read::<u32>(g.read::<u32>(gx)).wrapping_add(0x20));
    let m = g.read::<u32>(c.wrapping_add(imm(l.model)));
    g.write::<u8>(r + 8, g.read::<u8>(m.wrapping_add(0x16)));
    g.write::<u32>(r + 0x20, obj.wrapping_add(0x50));
    let lock = g.read::<u32>(w(l.lock));
    g.write::<u32>(r + 0x44, 0);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(r, 0);
    g.write::<u32>(lock, 1);
    let target = slot_target(g, w(l.table), 815);
    g.call(target, 0, &[obj, c.wrapping_add(imm(l.add)), r]);
    let lock = g.read::<u32>(w(l.lock));
    g.write::<u32>(lock, saved);
    Ret::eax(lock)
}

/// [`model_sprites`] with the model at a `disp8` (`magic_0084`
/// 0x100017c0, 245 bytes; 3 DLLs).
pub fn model_sprites_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    model_sprites(g, img, fr, &SPRITES_A)
}

/// [`model_sprites`] with the model at a `disp32` (`magic_0084`
/// 0x100018c0, 254 bytes; 3 DLLs, and the copies in `magic_0085`,
/// `0093`, `0619`, `0681`).
pub fn model_sprites_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    model_sprites(g, img, fr, &SPRITES_B)
}

/// The four stores before [`model_sprites`] in `magic_0440`'s copies:
/// `*G_1` = the immediate, `*G_2 = *K`, `*G_3 = *G_4 = 0`.
fn sprite_globals<G: Game>(g: &mut G, img: &Image, f: u32) {
    let w = |o: u32| op(img, f, o);
    let k = g.read::<u32>(w(0x1e));
    g.write::<u32>(w(0x30), w(0x34));
    g.write::<u32>(w(0x3c), k);
    g.write::<u32>(w(0x42), 0);
    g.write::<u32>(w(0x4c), 0);
}

/// [`model_sprites`] after [`sprite_globals`], the model at a `disp8`
/// (`magic_0440` 0x100034d0, 291 bytes; 2 DLLs).
pub fn model_sprites_c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    sprite_globals(g, img, fr.at);
    model_sprites(g, img, fr, &SPRITES_C)
}

/// [`model_sprites`] after [`sprite_globals`], the model at a `disp32`
/// (`magic_0440` 0x10003600, 300 bytes; 2 DLLs).
pub fn model_sprites_d<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    sprite_globals(g, img, fr.at);
    model_sprites(g, img, fr, &SPRITES_D)
}

/// [`model_sprites_a`] (the `rel32`s: [`sprite_packets`] and
/// `__security_check_cookie`).
pub const MODEL_SPRITES_A: Sig = Sig {
    len: 245,
    hash: 0x8342_5855_29c0_0926,
    calls: &[(0x56, Callee::Sig(&SPRITE_PACKETS)), (0xed, Callee::Any)],
    imports: &[],
};

/// [`model_sprites_b`] in `magic_0084`.
pub const MODEL_SPRITES_B: Sig = Sig {
    len: 254,
    hash: 0x22f7_08f5_89db_9931,
    calls: &[(0x59, Callee::Sig(&SPRITE_PACKETS)), (0xf6, Callee::Any)],
    imports: &[],
};

/// [`model_sprites_b`] in `magic_0085` (the model at `+0xfc`).
pub const MODEL_SPRITES_B_0085: Sig = Sig {
    len: 254,
    hash: 0xe2fa_8412_4700_1942,
    calls: MODEL_SPRITES_B.calls,
    imports: &[],
};

/// [`model_sprites_b`] in `magic_0093` (the model at `+0x15c`, flag 0).
pub const MODEL_SPRITES_B_0093: Sig = Sig {
    len: 254,
    hash: 0x35b6_400c_407e_cdc8,
    calls: MODEL_SPRITES_B.calls,
    imports: &[],
};

/// [`model_sprites_c`].
pub const MODEL_SPRITES_C: Sig = Sig {
    len: 291,
    hash: 0x82ca_c495_e9bc_e62b,
    calls: &[(0x84, Callee::Sig(&SPRITE_PACKETS)), (0x11b, Callee::Any)],
    imports: &[],
};

/// [`model_sprites_d`].
pub const MODEL_SPRITES_D: Sig = Sig {
    len: 300,
    hash: 0x976d_e7de_b69e_d550,
    calls: &[(0x87, Callee::Sig(&SPRITE_PACKETS)), (0x124, Callee::Any)],
    imports: &[],
};

/// The aim from the point `from` at `to`, as the beam starts compute it:
/// `dx`, `dy`, `dz` from `from` to `to`; the pitch
/// `atan2(dy, sqrt(dz dz + dx dx))` into `out`, the yaw `KA - atan2(dz, dx)` brought back once
/// by `KT` when above `KH` or below `KL` into `out+4`, `out+8` = 0 (the
/// operands `[KA, KH, KT, KL, KT']`). The square root is
/// `_libm_sse2_sqrt_precise`, the arcs `_CIatan2`, each rounded to `f32`.
fn aim_at<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (from, to, out): (u32, u32, u32),
    [ka, kh, kt, kl, kt2]: [u32; 5],
) {
    use crate::sb_sse::{add, mul, sub};
    let k = |g: &G, o: u32| g.read::<f32>(op(img, f, o));
    let r = |g: &G, p: u32, i: u32| g.read::<f32>(p.wrapping_add(4 * i));
    let dy = sub(r(g, to, 1), r(g, from, 1));
    let dx = sub(r(g, to, 0), r(g, from, 0));
    let dz = sub(r(g, to, 2), r(g, from, 2));
    let h = f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt();
    #[allow(clippy::cast_possible_truncation)] // `cvtpd2ps`
    let pitch = g.atan2(f64::from(dy), h) as f32;
    g.write::<f32>(out, pitch);
    #[allow(clippy::cast_possible_truncation)] // `cvtpd2ps`
    let yaw = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let y = sub(k(g, ka), yaw);
    let y = if y > k(g, kh) {
        sub(y, k(g, kt))
    } else if k(g, kl) > y {
        add(y, k(g, kt2))
    } else {
        y
    };
    g.write::<f32>(out + 4, y);
    g.write::<u32>(out + 8, 0);
}

/// The distance from `from` to `to` as the beam starts compute it (the
/// differences `from - to`, `y`, `x`, `z` squared and summed in that
/// order, `_libm_sse2_sqrt_precise`, rounded to `f32`).
fn distance<G: Game>(g: &G, from: u32, to: u32) -> f32 {
    use crate::sb_sse::{add, mul, sub};
    let r = |p: u32, i: u32| g.read::<f32>(p.wrapping_add(4 * i));
    let dy = sub(r(from, 1), r(to, 1));
    let dx = sub(r(from, 0), r(to, 0));
    let dz = sub(r(from, 2), r(to, 2));
    let d = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
    #[allow(clippy::cast_possible_truncation)] // `cvtsd2ss`
    let d = f64::from(d).sqrt() as f32;
    d
}

/// Where the beam starts keep their shared operands: the slot table, the
/// count `*G_N`, the block `*G_B`, the scene `*G_C` (and again at the
/// end), `K1`, the actor lookup's three and the aim's five.
struct BeamOps {
    table: u32,
    n: u32,
    b: u32,
    c: u32,
    c_end: u32,
    k1: u32,
    actor: (u32, u32, u32),
    aim: [u32; 5],
}

/// The beam starts' head: `V` = slot 237 `(slot 219 (*G_C, 0), 0, 0, V)`
/// (`V` at `A - 0x14`, `A` = `ebp & !7`). Returns `(n, B, V)`.
fn beams_head<G: Game>(g: &mut G, img: &Image, fr: &Frame, o: &BeamOps) -> (i32, u32, u32) {
    let w = |x: u32| op(img, fr.at, x);
    let v = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x14);
    let n = g.read::<i32>(w(o.n));
    let b = g.read::<u32>(w(o.b));
    let c = g.read::<u32>(w(o.c));
    let x = g.call(slot_target(g, w(o.table), 219), 0, &[c, 0]).eax;
    g.call(slot_target(g, w(o.table), 237), 0, &[x, 0, 0, v]);
    (n, b, v)
}

/// One beam's point `E` aimed (entry `i`): `E-0x18 = V`, `E-0xc = K1`;
/// slot 262 `(a, E-8, E-4, E)`, slot 268 `(a, 0x10, E-8)` for the actor
/// `a` of the battle number `(*G_L)[i]` ([`entry_actor`]), `E+4 = 1.0`;
/// the aim from `E-0x18` at `E-8` into `E+8` ([`aim_at`]). Returns the
/// distance between them ([`distance`]).
fn beam_aim<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    o: &BeamOps,
    (e, v, i): (u32, u32, u32),
) -> f32 {
    let t = |g: &G, s: u32| slot_target(g, op(img, f, o.table), s);
    for j in 0..3 {
        let x = g.read::<u32>(v + 4 * j);
        g.write::<u32>(e.wrapping_sub(0x18 - 4 * j), x);
    }
    let k1 = g.read::<u32>(op(img, f, o.k1));
    g.write::<u32>(e.wrapping_sub(0xc), k1);
    let actor = entry_actor(g, img, f, o.actor, i);
    let (p, q) = (e.wrapping_sub(8), e.wrapping_sub(4));
    g.call(t(g, 262), 0, &[actor, p, q, e]);
    g.call(t(g, 268), 0, &[actor, 0x10, p]);
    g.write::<u32>(e + 4, 0x3f80_0000);
    let from = e.wrapping_sub(0x18);
    aim_at(g, img, f, (from, p, e + 8), o.aim);
    distance(g, from, p)
}

/// A beam's count: `trunc(x)` into `E+0x294` and `+0x298` (1 when
/// negative).
fn beam_count<G: Game>(g: &mut G, e: u32, x: f32) {
    let count = crate::s4_summon::trunc(x);
    let count = if count.cast_signed() < 0 { 1 } else { count };
    g.write::<u32>(e + 0x294, count);
    g.write::<u32>(e + 0x298, count);
}

/// A beam's object `obj` started at the kind `kind`'s scale and program
/// (the tables at the operands `t1`, `t2`;
/// [`start_object_at`](crate::s4_misc::start_object_at) at the `rel32`
/// `call`) and the aim kept (`E+8..+0x18` into `E-0x1b8..`).
fn beam_object<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (e, obj, kind): (u32, u32, u32),
    (t1, t2, call): (u32, u32, u32),
) {
    let k = kind.wrapping_mul(4);
    let scale = g.read::<f32>(k.wrapping_add(op(img, f, t1)));
    let program = g.read::<u32>(k.wrapping_add(op(img, f, t2)));
    let start = img.rel32_target(f + call);
    crate::s4_misc::start_object_at(g, img, start, obj, scale, program, 0, 0);
    for j in 0..4 {
        let x = g.read::<u32>(e + 8 + 4 * j);
        g.write::<u32>(e.wrapping_sub(0x1b8 - 4 * j), x);
    }
}

/// The beam starts' end: slot 257 `(slot 219 (*G_C, 0), 1)`, its `eax`.
fn beams_tail<G: Game>(g: &mut G, img: &Image, f: u32, o: &BeamOps) -> Ret {
    let w = |x: u32| op(img, f, x);
    let c = g.read::<u32>(w(o.c_end));
    let x = g.call(slot_target(g, w(o.table), 219), 0, &[c, 0]).eax;
    Ret::eax(g.call(slot_target(g, w(o.table), 257), 0, &[x, 1]).eax)
}

/// [`beams_aimed`]'s operands.
const BEAMS_AIMED_OPS: BeamOps = BeamOps {
    table: 0x15,
    n: 0x1c,
    b: 0x29,
    c: 0x32,
    c_end: 0x3c9,
    k1: 0x5b,
    actor: (0xad, 0xb4, 0xbf),
    aim: [0x1aa, 0x1b9, 0x1c3, 0x1cd, 0x1da],
};

/// The beams aimed at the actors and started (cdecl, no arguments; a /GS
/// frame aligned to 8): the head ([`beams_head`]); for each of the `n =
/// *G_N` entries `i` (0x520 bytes from `E = *G_B + 0x5e98`) the aim
/// ([`beam_aim`]), the count `distance * KS` ([`beam_count`]), the object
/// `O = E - 0x278` started at the kind `B[0x8524]` ([`beam_object`]),
/// `E+0x29c = i`, a task node taken and listed
/// ([`free_to_live`](crate::s7_misc::free_to_live)) with `+0x14 = O`. Then
/// `B[0x8528] = n`, `B[0x852c] = 0`; the object `B[0x8520]` taken off the
/// list `G_O` ([`list_remove`](crate::l5_objhost::list_remove)): if it was
/// there, `**G_P = G_Q`, slot 713 `(it)` and its u16 `+0x124` loses bit
/// 15. Returns [`beams_tail`]'s `eax`.
///
/// `magic_0446` 0x10002c20 (1,018 bytes; 10 DLLs).
pub fn beams_aimed<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::mul;
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let ops = &BEAMS_AIMED_OPS;
    let (n, b, v) = beams_head(g, img, fr, ops);
    let mut e = b.wrapping_add(0x5e98);
    let mut i = 0i32;
    while i < n {
        let d = beam_aim(g, img, f, ops, (e, v, i.cast_unsigned()));
        beam_count(g, e, mul(d, g.read::<f32>(w(0x22c))));
        let kind = g.read::<u32>(b.wrapping_add(0x8524));
        let o = e.wrapping_sub(0x278);
        beam_object(g, img, f, (e, o, kind), (0x262, 0x26b, 0x272));
        g.write::<i32>(e + 0x29c, i);
        let ring = [
            0x27c, 0x2b5, 0x2d2, 0x2da, 0x30b, 0x316, 0x31c, 0x334, 0x35b, 0x347, 0x35b,
        ];
        let node = crate::s7_misc::free_to_live(g, img, f, ring);
        g.write::<u32>(node.wrapping_add(0x14), o);
        i += 1;
        e = e.wrapping_add(0x520);
    }
    g.write::<i32>(b.wrapping_add(0x8528), n);
    g.write::<u32>(b.wrapping_add(0x852c), 0);
    let obj = g.read::<u32>(b.wrapping_add(0x8520));
    let remove = img.rel32_target(f + 0x390);
    if crate::l5_objhost::list_remove(g, img, remove, w(0x38b), obj) != 0 {
        g.write::<u32>(g.read::<u32>(w(0x399)), w(0x3a0));
        g.call(slot_target(g, w(ops.table), 713), 0, &[obj]);
        let fl = g.read::<u16>(obj.wrapping_add(0x124));
        g.write::<u16>(obj.wrapping_add(0x124), fl & 0x7fff);
    }
    beams_tail(g, img, f, ops)
}

/// [`beams_sprites`]' operands.
const BEAMS_SPRITES_OPS: BeamOps = BeamOps {
    table: 0x2d,
    n: 0x1d,
    b: 0x15,
    c: 0x35,
    c_end: 0x3d0,
    k1: 0x64,
    actor: (0xad, 0xb4, 0xbf),
    aim: [0x1ae, 0x1bd, 0x1c7, 0x1d1, 0x1de],
};

/// [`beams_aimed`]'s kin with a sprite kind (cdecl, no arguments): the
/// entries `E` 0x520 bytes apart from `*G_B + 0x3328` (their objects `O
/// = E - 0x278`); the count is `distance / KD`; with the kind `B[0x5c04]`
/// 0 the point's sprite is set up instead of an object
/// ([`sprite_init`](crate::sb_misc::sprite_init) of `E+0x48` for
/// `*(*G_Y + 0x24)`; `E+0x270 = 0x10`, `+0x264 = 1.0`, `+0x260 = 0.5`, byte
/// `+0x257 = 0x40`). Then `B[0x5c08] = n`, `B[0x5c0c] = 0` and the tail
/// ([`beams_tail`]).
///
/// `magic_0446` 0x10002500 (1,025 bytes; 10 DLLs).
pub fn beams_sprites<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::div;
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let ops = &BEAMS_SPRITES_OPS;
    let (n, b, v) = beams_head(g, img, fr, ops);
    let mut o = b.wrapping_add(0x30b0);
    let mut i = 0i32;
    while i < n {
        let e = o.wrapping_add(0x278);
        let d = beam_aim(g, img, f, ops, (e, v, i.cast_unsigned()));
        beam_count(g, e, div(d, g.read::<f32>(w(0x22f))));
        let kind = g.read::<u32>(b.wrapping_add(0x5c04));
        if kind == 0 {
            let owner = g.read::<u32>(g.read::<u32>(w(0x25f)).wrapping_add(0x24));
            crate::sb_misc::sprite_init(g, e + 0x48, owner);
            g.write::<u32>(e + 0x270, 0x10);
            g.write::<u32>(e + 0x264, 0x3f80_0000);
            g.write::<u32>(e + 0x260, 0x3f00_0000);
            g.write::<u8>(e + 0x257, 0x40);
        } else {
            beam_object(g, img, f, (e, o, kind), (0x29a, 0x2a6, 0x2ad));
        }
        g.write::<i32>(e + 0x29c, i);
        let ring = [
            0x2db, 0x2f0, 0x30d, 0x315, 0x346, 0x351, 0x357, 0x36d, 0x396, 0x380, 0x396,
        ];
        let node = crate::s7_misc::free_to_live(g, img, f, ring);
        g.write::<u32>(node.wrapping_add(0x14), o);
        i += 1;
        o = o.wrapping_add(0x520);
    }
    g.write::<i32>(b.wrapping_add(0x5c08), n);
    g.write::<u32>(b.wrapping_add(0x5c0c), 0);
    beams_tail(g, img, f, ops)
}

/// [`beams_sprites`] (the `rel32`s: the square root, the arcs, the
/// sprite set-up, the object start and `__security_check_cookie`).
pub const BEAMS_SPRITES: Sig = Sig {
    len: 1025,
    hash: 0x59ca_2d54_76d6_d406,
    calls: &[
        (0x14b, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x15e, Callee::Import("_CIatan2")),
        (0x19c, Callee::Import("_CIatan2")),
        (0x223, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x26a, Callee::Sig(&crate::sb_misc::SPRITE_INIT)),
        (0x2ad, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x3f9, Callee::Any),
    ],
    imports: &[],
};

/// [`beams_aimed`] (the `rel32`s: the square root, the arcs, the object
/// start, the list removal and `__security_check_cookie`).
pub const BEAMS_AIMED: Sig = Sig {
    len: 1018,
    hash: 0xf8d8_8370_2157_fadb,
    calls: &[
        (0x147, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x15a, Callee::Import("_CIatan2")),
        (0x198, Callee::Import("_CIatan2")),
        (0x21e, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x272, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x390, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x3f2, Callee::Any),
    ],
    imports: &[],
};

/// The beams launched one by one (cdecl `(O)`; a /GS frame aligned to 8,
/// `A` = `ebp & !7`, the point `P` at `A - 0x24`, the angles `Q` at `A -
/// 0x50`, the offset `U` at `A - 0x14`; `R = *G_B + 0x8540`): while the
/// wait `R[0x5200]` is above 0 it counts down (returning the new count).
/// Then `P` is the block's `B+0x2f80` (with `R[0x5208]` 0) or slot 237
/// `(slot 219 (*G_C, 0), 0, 0, P)` with `w` 1.0; `Q = (0, B[0xebf0], 0,
/// 0)`, `U = (0, 0, 0, 1)`; with `R[0x5208]` 0, `U.x` = slot 314 `(5.0)`,
/// `U.y` = slot 314 `(2.0)` - `K`, and `P` moved by `U` turned by `Q`
/// ([`turn_point`](crate::s4_small::turn_point)). The next beam `D = R +
/// 0x520 R[0x5204]`: `D+0x260 = P`; its target `T = D+0x270` from slot
/// 262 and 268 of the first entry's actor, `T.w` 1.0, with `R[0x5208]` 0
/// each coordinate moved by slot 314 `(4.0)`; the aim from `P` to `T`
/// into `D+0x280` ([`aim_at`]); `D+0x514` = 0, the count `distance / KD`
/// into `D+0x50c` and `+0x510` (1 when negative); a task node (owner
/// `CB`; [`take`](crate::sa_pool::take)) gets `+0x14 = D` and goes to the
/// live list's end. The wait becomes `slot 324 () % 8 + 4`; `R[0x5204]`
/// goes up, and once it reaches `R[0x520c]` the block's `B+0x1404` = the
/// next step `S`, `B+0x1420` = 60 and byte `O+0x56` = 1 (returning `O`;
/// else the new `R[0x5204]`).
///
/// `magic_0446` 0x10003b90 (1,216 bytes; 10 DLLs).
pub fn beams_one_by_one<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let st0 = |o: Out| crate::m0018::st0_f32(o.st0);
    let obj = fr.arg(g, 0);
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x60);
    let b = g.read::<u32>(w(0x1b));
    let r = b.wrapping_add(0x8540);
    let wait = g.read::<i32>(b.wrapping_add(0xd740));
    if wait > 0 {
        g.write::<i32>(r + 0x5200, wait - 1);
        return Ret::eax((wait - 1).cast_unsigned());
    }
    let p = s + 0x3c;
    if g.read::<u32>(r + 0x5208) == 0 {
        for j in 0..4 {
            let x = g.read::<u32>(b.wrapping_add(0x2f80 + 4 * j));
            g.write::<u32>(p + 4 * j, x);
        }
    } else {
        let c = g.read::<u32>(w(0x9d));
        let x = g.call(slot_target(g, w(0x95), 219), 0, &[c, 0]).eax;
        g.call(slot_target(g, w(0xb4), 237), 0, &[x, 0, 0, p]);
        g.write::<u32>(p + 0xc, 0x3f80_0000);
    }
    let d = g
        .read::<u32>(r + 0x5204)
        .wrapping_mul(0x520)
        .wrapping_add(r);
    let (q, u) = (s + 0x10, s + 0x4c);
    let tilt = g.read::<u32>(b.wrapping_add(0xebf0));
    for (j, x) in (0u32..).zip([0, tilt, 0, 0]) {
        g.write::<u32>(q + 4 * j, x);
    }
    for (j, x) in (0u32..).zip([0, 0, 0, 0x3f80_0000]) {
        g.write::<u32>(u + 4 * j, x);
    }
    if g.read::<u32>(r + 0x5208) == 0 {
        let x = st0(g.call(slot_target(g, w(0x129), 314), 0, &[0x40a0_0000]));
        g.write::<f32>(u, x);
        let z = st0(g.call(slot_target(g, w(0x142), 314), 0, &[0x4000_0000]));
        g.write::<f32>(u + 4, sub(z, g.read::<f32>(w(0x163))));
        let turn = (img.rel32_target(f + 0x17b), s.wrapping_sub(4), q, p);
        sub_call(g, crate::s4_small::turn_point, img, turn, &[u]);
    }
    for j in 0..4 {
        let x = g.read::<u32>(p + 4 * j);
        g.write::<u32>(d + 0x260 + 4 * j, x);
    }
    let first = u32::from(g.read::<u8>(g.read::<u32>(w(0x1bb))));
    let actor = g.read::<u32>(first.wrapping_mul(4).wrapping_add(w(0x1ce)));
    let tgt = d + 0x270;
    g.call(
        slot_target(g, w(0x1d7), 262),
        0,
        &[actor, tgt, tgt + 4, tgt + 8],
    );
    g.call(slot_target(g, w(0x1ee), 268), 0, &[actor, 0x10, tgt]);
    g.write::<u32>(tgt + 0xc, 0x3f80_0000);
    if g.read::<u32>(r + 0x5208) == 0 {
        for (j, t) in (0u32..).zip([0x21b, 0x23e, 0x268]) {
            let x = st0(g.call(slot_target(g, w(t), 314), 0, &[0x4080_0000]));
            let v = add(x, g.read::<f32>(tgt + 4 * j));
            g.write::<f32>(tgt + 4 * j, v);
        }
    }
    let from = d + 0x260;
    aim_at(
        g,
        img,
        f,
        (from, tgt, d + 0x280),
        [0x35b, 0x36a, 0x374, 0x37e, 0x38b],
    );
    let dist = distance(g, from, tgt);
    let x = div(dist, g.read::<f32>(w(0x3f6)));
    g.write::<u32>(d + 0x514, 0);
    let count = crate::s4_summon::trunc(x);
    let count = if count.cast_signed() < 0 { 1 } else { count };
    g.write::<u32>(d + 0x50c, count);
    g.write::<u32>(d + 0x510, count);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x421), w(0x410));
    g.write::<u32>(node.wrapping_add(0x14), d);
    g.write::<u32>(node.wrapping_add(4), 0);
    if g.read::<u32>(w(0x427)) == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x44c), node);
    } else {
        let last = g.read::<u32>(w(0x43a));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x454), node);
    let roll = g
        .call(slot_target(g, w(0x459), 324), 0, &[])
        .eax
        .cast_signed()
        % 8;
    let next = g.read::<u32>(r + 0x5204).wrapping_add(1);
    g.write::<u32>(r + 0x5204, next);
    g.write::<i32>(r + 0x5200, roll + 4);
    let next = g.read::<i32>(r + 0x5204);
    if next < g.read::<i32>(r + 0x520c) {
        return Ret::eax(next.cast_unsigned());
    }
    g.write::<u32>(b.wrapping_add(0x1404), w(0x498));
    g.write::<u32>(b.wrapping_add(0x1420), 0x3c);
    g.write::<u8>(obj.wrapping_add(0x56), 1);
    Ret::eax(obj)
}

/// [`beams_one_by_one`] (the `rel32`s: `__security_check_cookie` twice,
/// the point turn, the square roots, the arcs and the pool take).
pub const BEAMS_ONE_BY_ONE: Sig = Sig {
    len: 1216,
    hash: 0xbc64_721a_c140_c3b2,
    calls: &[
        (0x49, Callee::Any),
        (0x17b, Callee::Sig(&crate::s4_small::TURN_POINT)),
        (0x2f5, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x308, Callee::Import("_CIatan2")),
        (0x349, Callee::Import("_CIatan2")),
        (0x3e5, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x421, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x4b8, Callee::Any),
    ],
    imports: &[],
};

/// The object `obj` taken off the list at the operand `list`
/// ([`list_remove`](crate::l5_objhost::list_remove) at the `rel32`
/// `call`); if it was there, `**G_P = Q` (the operands `gp`, `q`), slot
/// 713 `(obj)` and its u16 `+0x124` loses bit 15.
fn unlist_object<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    (list, call, gp, q, table): (u32, u32, u32, u32, u32),
) {
    let w = |o: u32| op(img, f, o);
    let remove = img.rel32_target(f + call);
    if crate::l5_objhost::list_remove(g, img, remove, w(list), obj) != 0 {
        g.write::<u32>(g.read::<u32>(w(gp)), w(q));
        g.call(slot_target(g, w(table), 713), 0, &[obj]);
        let fl = g.read::<u16>(obj.wrapping_add(0x124));
        g.write::<u16>(obj.wrapping_add(0x124), fl & 0x7fff);
    }
}

/// [`beams_spread`]'s operands (for [`beams_tail`]).
const BEAMS_SPREAD_OPS: BeamOps = BeamOps {
    table: 0x1b,
    n: 0,
    b: 0x29,
    c: 0x32,
    c_end: 0x4c5,
    k1: 0,
    actor: (0, 0, 0),
    aim: [0x2ad, 0x2bc, 0x2c6, 0x2d0, 0x2dd],
};

/// Three beams spread from the actors' mean position (cdecl, no
/// arguments; a /GS frame aligned to 8, `S` = `(ebp & !7) - 0xd0`: the
/// angle `t` at `S+0x24`, the offset `V` at `S+0x48`, the mean `M` at
/// `S+0x58`, the point `P` at `S+0x68`, the turned offset `W` at `S+0x78`,
/// the matrix `X` at `S+0x88`): `P` = slot 237 `(slot 219 (*G_C, 0), 0,
/// 0, P)` with `w` 1.0, `t = B[0xebf0]` (`B = *G_B`), `V.w` = 1.0, `M` the
/// actors' mean position ([`mean_position`](crate::sb_misc::mean_position))
/// with `M.y` = slot 12 of the first entry's actor (ST0). For each beam
/// `i` of 3 (`E = B + 0xd9d8 + 0x520 i`): `E-0x18 = P`, `E-8 = M`; `V` =
/// `(0, -25, 0)` for the first, `(40, 0, slot 314 (15.0))` and `(-40, 0,
/// slot 314 (15.0))` for the others; `X` = slot 235, turned by slot 328
/// `(X, X, 0.0)` and 329 `(X, X, t)`; `W` = slot 229 `(W, X, V)`, slot 248
/// `(E-8, E-8, W)`; `E+4 = 1.0`; the aim ([`aim_at`]); the count
/// `distance / KD` (1 when negative, 2 more for the first) into `E+0x294`
/// and `+0x298`; the object `E - 0x278` started at the scale `K` with the
/// program 3 ([`start_object_at`](crate::s4_misc::start_object_at)),
/// `E-0x1b8.. = E+8..`, `E+0x29c = i`, a task node taken and listed
/// ([`free_to_live`](crate::s7_misc::free_to_live)) with `+0x14 = E -
/// 0x278`. Then `B[0xebe0] = 3`, `B[0xebe4] = 0`, the object `B+0x2860`
/// off its list ([`unlist_object`]). Returns [`beams_tail`]'s `eax`.
///
/// `magic_0446` 0x10004480 (1,273 bytes; 10 DLLs).
pub fn beams_spread<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = beams_spread_body(g, img, fr);
    beams_tail(g, img, f, &BEAMS_SPREAD_OPS)
}

/// [`beams_spread`] with the program `0x27`, the object at `B+0x2730`
/// (its words `+0xc4b0`, `+0xc4b4`) and, after the tail, slot 38 `(slot
/// 219 (*G_C, 0), 0.0, 1)`, whose `eax` it returns (the program, the
/// object's offset and its words' are the code's immediates).
///
/// `magic_0565` 0x100065e0 (1,321 bytes; 5 DLLs).
pub fn beams_spread_38<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = beams_spread_body(g, img, fr);
    let w = |o: u32| op(img, f, o);
    beams_tail(g, img, f, &BEAMS_SPREAD_OPS);
    let c = g.read::<u32>(w(0x4ea));
    let x = g.call(slot_target(g, w(0x1b), 219), 0, &[c, 0]).eax;
    Ret::eax(g.call(slot_target(g, w(0x505), 38), 0, &[x, 0, 1]).eax)
}

/// [`beams_spread`] up to its tail; returns the function's address.
fn beams_spread_body<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    use crate::sb_sse::div;
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x1b), s);
    let ops = &BEAMS_SPREAD_OPS;
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xd0);
    let (tilt, v, m, p, wv, x) = (s + 0x24, s + 0x48, s + 0x58, s + 0x68, s + 0x78, s + 0x88);
    let b = g.read::<u32>(w(0x29));
    let c = g.read::<u32>(w(0x32));
    let scene = g.call(t(g, 219), 0, &[c, 0]).eax;
    g.call(t(g, 237), 0, &[scene, 0, 0, p]);
    let k = g.read::<u32>(b.wrapping_add(0xebf0));
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    g.write::<u32>(tilt, k);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    let mean = img.rel32_target(f + 0x79);
    crate::sb_misc::mean_position(g, img, mean, s.wrapping_sub(0x1c), m);
    let first = u32::from(g.read::<u8>(g.read::<u32>(w(0x7e))));
    let actor = g.read::<u32>(first.wrapping_mul(4).wrapping_add(w(0x88)));
    let y = crate::m0018::st0_f32(g.call(t(g, 12), 0, &[actor]).st0);
    g.write::<f32>(m + 4, y);
    let mut e = b.wrapping_add(0xd9d8);
    let start = img.rel32_target(f + 0x374);
    for i in 0..3u32 {
        for j in 0..4 {
            let a = g.read::<u32>(p + 4 * j);
            g.write::<u32>(e.wrapping_sub(0x18 - 4 * j), a);
        }
        let to = e.wrapping_sub(8);
        for j in 0..4 {
            let a = g.read::<u32>(m + 4 * j);
            g.write::<u32>(to + 4 * j, a);
        }
        if i == 0 {
            g.write::<u32>(v, 0);
            g.write::<u32>(v + 4, 0xc1c8_0000);
            g.write::<u32>(v + 8, 0);
        } else {
            g.write::<u32>(v, if i == 1 { 0x4220_0000 } else { 0xc220_0000 });
            g.write::<u32>(v + 4, 0);
            let z = crate::m0018::st0_f32(g.call(t(g, 314), 0, &[0x4170_0000]).st0);
            g.write::<f32>(v + 8, z);
        }
        g.call(t(g, 235), 0, &[x]);
        g.call(t(g, 328), 0, &[x, x, 0]);
        g.call(t(g, 329), 0, &[x, x, k]);
        g.write::<u32>(v + 0xc, 0x3f80_0000);
        g.call(t(g, 229), 0, &[wv, x, v]);
        g.call(t(g, 248), 0, &[to, to, wv]);
        g.write::<u32>(e + 4, 0x3f80_0000);
        let from = e.wrapping_sub(0x18);
        aim_at(g, img, f, (from, to, e + 8), ops.aim);
        let d = div(distance(g, from, to), g.read::<f32>(w(0x32d)));
        beam_count(g, e, d);
        if i == 0 {
            let n = g.read::<u32>(e + 0x294).wrapping_add(2);
            g.write::<u32>(e + 0x294, n);
            g.write::<u32>(e + 0x298, n);
        }
        let obj = e.wrapping_sub(0x278);
        let scale = g.read::<f32>(w(0x35c));
        let program = w(0x36c) & 0xff;
        crate::s4_misc::start_object_at(g, img, start, obj, scale, program, 0, 0);
        for j in 0..4 {
            let a = g.read::<u32>(e + 8 + 4 * j);
            g.write::<u32>(e.wrapping_sub(0x1b8 - 4 * j), a);
        }
        g.write::<u32>(e + 0x29c, i);
        let ring = [
            0x37e, 0x3b7, 0x3d4, 0x3dc, 0x40d, 0x418, 0x41e, 0x436, 0x459, 0x449, 0x459,
        ];
        let node = crate::s7_misc::free_to_live(g, img, f, ring);
        g.write::<u32>(node.wrapping_add(0x14), obj);
        e = e.wrapping_add(0x520);
    }
    let obj = b.wrapping_add(w(0x46c));
    g.write::<u32>(obj.wrapping_add(w(0x472)), 3);
    g.write::<u32>(obj.wrapping_add(w(0x47c)), 0);
    unlist_object(g, img, f, obj, (0x487, 0x48c, 0x495, 0x49c, 0x1b));
    f
}

/// [`beams_spread`] (the `rel32`s: the mean position, the square roots,
/// the arcs, the object start, the list removal and
/// `__security_check_cookie`).
pub const BEAMS_SPREAD: Sig = Sig {
    len: 1273,
    hash: 0x58ac_d4b5_c09b_9bd3,
    calls: &[
        (0x79, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x24a, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x25d, Callee::Import("_CIatan2")),
        (0x29b, Callee::Import("_CIatan2")),
        (0x321, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x374, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x48c, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x4f1, Callee::Any),
    ],
    imports: &[],
};

/// [`beams_spread_38`] (the `rel32`s as [`BEAMS_SPREAD`]'s).
pub const BEAMS_SPREAD_38: Sig = Sig {
    len: 1321,
    hash: 0xf355_3e49_1b00_3aaf,
    calls: &[
        (0x79, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x24a, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x25d, Callee::Import("_CIatan2")),
        (0x29b, Callee::Import("_CIatan2")),
        (0x321, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x374, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x48c, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x521, Callee::Any),
    ],
    imports: &[],
};
