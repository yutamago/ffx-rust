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
/// imm16` (`66 c7 05`), `mov byte [G], imm8` (`c6 05`), `mov [G], al`
/// (`a2`) and `mov [G], ax` (`66 a3`), `al`/`ax` from `eax`. Returns the
/// offset where the run stops (the first other instruction).
fn global_stores<G: Game>(g: &mut G, img: &Image, f: u32, from: u32, to: u32, eax: u32) -> u32 {
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
                g.write::<u8>(img.word(f + at + 1), eax as u8);
                at += 5;
            }
            (0x66, 0xa3, _) => {
                g.write::<u16>(img.word(f + at + 2), eax as u16);
                at += 6;
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
    global_stores(g, img, f, 0x205, 0x250, u32::from(al));
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

/// A port on its frame.
type Port<G> = fn(&mut G, &Image, &Frame) -> Ret;

/// Calls the internal port `port` of the function at `target` as the
/// original does: `args` pushed below `esp` (the first one on top), the
/// return address below them, `ecx` and `edx` as given.
fn sub_call<G: Game>(
    g: &mut G,
    port: Port<G>,
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

/// Where [`beams_spread`]'s copies differ: `magic_0644`'s has no height
/// step (slot 12), so its code from `+0x9d` on sits `shift` = 0x20 bytes
/// earlier, and its frame swaps `P` and `M`; the mean position's call sees
/// a different stack depth (`esp` at `S - mean_esp`).
pub struct Spread {
    shift: u32,
    p: u32,
    m: u32,
    mean_esp: u32,
    height: bool,
}

/// `magic_0446` 0x10004480 and `magic_0565` 0x100065e0.
const SPREAD_A: Spread = Spread {
    shift: 0,
    p: 0x68,
    m: 0x58,
    mean_esp: 0x1c,
    height: true,
};

/// `magic_0644` 0x1000a430.
const SPREAD_B: Spread = Spread {
    shift: 0x20,
    p: 0x58,
    m: 0x68,
    mean_esp: 4,
    height: false,
};

/// [`beams_spread`]'s operands for [`beams_tail`] and [`aim_at`] (the
/// `magic_0446` offsets less `shift` from `+0x9d` on).
fn spread_ops(shift: u32) -> BeamOps {
    let at = |o: u32| o - shift;
    BeamOps {
        table: 0x1b,
        n: 0,
        b: 0x29,
        c: 0x32,
        c_end: at(0x4c5),
        k1: 0,
        actor: (0, 0, 0),
        aim: [0x2ad, 0x2bc, 0x2c6, 0x2d0, 0x2dd].map(at),
    }
}

/// Three beams spread from the actors' mean position (cdecl, no
/// arguments; a /GS frame aligned to 8, `S` = `(ebp & !7) - 0xd0`: the
/// angle `t` at `S+0x24`, the offset `V` at `S+0x48`, the mean `M` and the
/// point `P` at `S+0x58`/`+0x68` ([`Spread`]), the turned offset `W` at
/// `S+0x78`, the matrix `X` at `S+0x88`): `P` = slot 237 `(slot 219
/// (*G_C, 0), 0, 0, P)` with `w` 1.0, `t = B[T]` (`B = *G_B`, `T` the
/// code's displacement), `V.w` = 1.0, `M` the actors' mean position
/// ([`mean_position`](crate::sb_misc::mean_position)), its `y` = slot 12
/// of the first entry's actor (ST0) where the copy has the height step.
/// For each beam `i` of 3 (`E = B + E0 + 0x520 i`, `E0` an immediate):
/// `E-0x18 = P`, `E-8 = M`; `V` = `(0, -25, 0)` for the first, `(40, 0,
/// slot 314 (15.0))` and `(-40, 0, slot 314 (15.0))` for the others (in
/// `magic_0446`; `magic_0644`'s are -15 and 10, the code's immediates); `X` =
/// slot 235, turned by slot 328 `(X, X, 0.0)` and 329 `(X, X, t)`; `W` =
/// slot 229 `(W, X, V)`, slot 248 `(E-8, E-8, W)`; `E+4 = 1.0`; the aim
/// ([`aim_at`]); the count `distance / KD` (1 when negative, 2 more for the
/// first) into `E+0x294` and `+0x298`; the object `E - 0x278` started at
/// the scale `K` with the program (an immediate)
/// ([`start_object_at`](crate::s4_misc::start_object_at)), `E-0x1b8.. =
/// E+8..`, `E+0x29c = i`, a task node taken and listed
/// ([`free_to_live`](crate::s7_misc::free_to_live)) with `+0x14 = E -
/// 0x278`. Then the object `O = B + O0` gets `O[W1] = 3`, `O[W2] = 0` and
/// leaves its list ([`unlist_object`]). Returns [`beams_tail`]'s `eax`.
///
/// `magic_0446` 0x10004480 (1,273 bytes; 10 DLLs).
pub fn beams_spread<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    beams_spread_body(g, img, fr, &SPREAD_A);
    beams_tail(g, img, fr.at, &spread_ops(0))
}

/// [`beams_spread`] followed by slot 38 `(slot 219 (*G_C, 0), 0.0, 1)`,
/// whose `eax` it returns.
fn spread_38<G: Game>(g: &mut G, img: &Image, fr: &Frame, l: &Spread) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o - l.shift);
    beams_spread_body(g, img, fr, l);
    beams_tail(g, img, f, &spread_ops(l.shift));
    let c = g.read::<u32>(w(0x4ea));
    let x = g
        .call(slot_target(g, op(img, f, 0x1b), 219), 0, &[c, 0])
        .eax;
    Ret::eax(g.call(slot_target(g, w(0x505), 38), 0, &[x, 0, 1]).eax)
}

/// [`beams_spread`] with slot 38 after the tail ([`spread_38`]; its
/// program 0x27 and object `B+0x2730`).
///
/// `magic_0565` 0x100065e0 (1,321 bytes; 5 DLLs).
pub fn beams_spread_38<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    spread_38(g, img, fr, &SPREAD_A)
}

/// [`beams_spread_38`] without the height step ([`SPREAD_B`]).
///
/// `magic_0644` 0x1000a430 (1,289 bytes; 5 DLLs).
pub fn beams_spread_flat<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    spread_38(g, img, fr, &SPREAD_B)
}

/// [`beams_spread`] up to its tail.
fn beams_spread_body<G: Game>(g: &mut G, img: &Image, fr: &Frame, l: &Spread) {
    use crate::sb_sse::div;
    let f = fr.at;
    let w = |o: u32| op(img, f, if o >= 0x9d { o - l.shift } else { o });
    let t = |g: &G, s: u32| slot_target(g, w(0x1b), s);
    let ops = spread_ops(l.shift);
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xd0);
    let (tilt, v, wv, x) = (s + 0x24, s + 0x48, s + 0x78, s + 0x88);
    let (p, m) = (s + l.p, s + l.m);
    let b = g.read::<u32>(w(0x29));
    let c = g.read::<u32>(w(0x32));
    let scene = g.call(t(g, 219), 0, &[c, 0]).eax;
    g.call(t(g, 237), 0, &[scene, 0, 0, p]);
    let k = g.read::<u32>(b.wrapping_add(w(0x57)));
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    g.write::<u32>(tilt, k);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    let mean = img.rel32_target(f + 0x79);
    crate::sb_misc::mean_position(g, img, mean, s.wrapping_sub(l.mean_esp), m);
    if l.height {
        let first = u32::from(g.read::<u8>(g.read::<u32>(w(0x7e))));
        let actor = g.read::<u32>(first.wrapping_mul(4).wrapping_add(w(0x88)));
        let y = crate::m0018::st0_f32(g.call(t(g, 12), 0, &[actor]).st0);
        g.write::<f32>(m + 4, y);
    }
    let mut e = b.wrapping_add(w(0xa1));
    let start = img.rel32_target(f + 0x374 - l.shift);
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
        // the offsets are the code's immediates (`mov [esp+n], imm32`)
        if i == 0 {
            g.write::<u32>(v, w(0x147));
            g.write::<u32>(v + 4, w(0x14f));
            g.write::<u32>(v + 8, w(0x157));
        } else {
            g.write::<u32>(v, w(if i == 1 { 0x119 } else { 0x10f }));
            g.write::<u32>(v + 4, w(0x127));
            let z = crate::m0018::st0_f32(g.call(t(g, 314), 0, &[w(0x134)]).st0);
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
        ]
        .map(|o| o - l.shift);
        let node = crate::s7_misc::free_to_live(g, img, f, ring);
        g.write::<u32>(node.wrapping_add(0x14), obj);
        e = e.wrapping_add(0x520);
    }
    let obj = b.wrapping_add(w(0x46c));
    g.write::<u32>(obj.wrapping_add(w(0x472)), 3);
    g.write::<u32>(obj.wrapping_add(w(0x47c)), 0);
    let list = [0x487, 0x48c, 0x495, 0x49c].map(|o| o - l.shift);
    unlist_object(g, img, f, obj, (list[0], list[1], list[2], list[3], 0x1b));
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

/// [`beams_spread_flat`] (the `rel32`s as [`BEAMS_SPREAD`]'s, 0x20 bytes
/// earlier from the square roots on).
pub const BEAMS_SPREAD_FLAT: Sig = Sig {
    len: 1289,
    hash: 0x8cae_44b7_7c19_daa9,
    calls: &[
        (0x79, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x22a, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x23d, Callee::Import("_CIatan2")),
        (0x27b, Callee::Import("_CIatan2")),
        (0x301, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x354, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x46c, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x501, Callee::Any),
    ],
    imports: &[],
};

/// The sparks that move a trail's head `p` (internal, `ecx` = `p`; the
/// function at `target`, called with `esp` above its return address): the
/// `0446` family's three, `0565`'s or `0644`'s. Returns its `eax`.
pub(crate) fn sparks_move<G: Game>(g: &mut G, img: &Image, target: u32, esp: u32, p: u32) -> u32 {
    use crate::s4_fam446::{
        SPARKS_A, SPARKS_B, SPARKS_C, SPARKS_D, sparks_a, sparks_b, sparks_c, sparks_d,
    };
    use crate::s7_misc::{
        FLAGGED_TWO_SPARKS, SPARK_AT_POINT, TWO_SPARKS, TWO_SPARKS_B, flagged_two_sparks,
        spark_at_point, two_sparks,
    };
    let fr = Frame {
        at: target,
        esp: esp.wrapping_sub(4),
        eax: 0,
        ecx: p,
        edx: 0,
        xmm: [0; 4],
    };
    let ports: [(&Sig, Port<G>); 8] = [
        (&SPARKS_A, sparks_a),
        (&SPARKS_B, sparks_b),
        (&SPARKS_C, sparks_c),
        (&SPARKS_D, sparks_d),
        (&TWO_SPARKS, |g, img, fr| Ret::eax(two_sparks(g, img, fr))),
        (&TWO_SPARKS_B, |g, img, fr| Ret::eax(two_sparks(g, img, fr))),
        (&SPARK_AT_POINT, |g, img, fr| {
            Ret::eax(spark_at_point(g, img, fr))
        }),
        (&FLAGGED_TWO_SPARKS, |g, img, fr| {
            Ret::eax(flagged_two_sparks(g, img, fr))
        }),
    ];
    ports
        .iter()
        .find(|(sig, _)| img.matches(target, sig))
        .map_or(0, |(_, port)| port(g, img, &fr).eax)
}

/// The trail's head moved and drawn (cdecl `(O)`; a /GS frame, `ebp` =
/// `esp - 4`, the point `P` at `ebp-0x14`, the matrix `M` at `ebp-0x54`;
/// `B = *G_B`): the wait `B[0x5c00]` counts down to 0; `P` = slot 237
/// `(slot 219 (*G_C, 0), 0, 0, P)`, `w` 1.0. While the wait runs, the
/// kind `B[0x5c04]` moves `P` (0: [`sparks_a`], 2: [`sparks_c`], 3:
/// [`sparks_d`], `ecx` = `P`; others nothing) and `P` is kept in
/// `B+0x30a0`; once it has run out, with the kind 0 and the count
/// `B[0x5bd8]` above 0 the count goes down and `P` is the kept one, else
/// `O` goes back to its pool ([`give_back`](crate::sa_pool::give_back))
/// and that is all. With the kind 0, `M` = slot 235 scaled to 0.2 on the
/// diagonal and the trail `B+0x59b0` drawn at `P` along `M`
/// ([`trail_draw`](crate::o6_fam_pairs6::trail_draw)). Returns the last
/// callee's `eax` (`kind - 3` for a kind without a move).
///
/// [`sparks_a`]: crate::s4_fam446::sparks_a
/// [`sparks_c`]: crate::s4_fam446::sparks_c
/// [`sparks_d`]: crate::s4_fam446::sparks_d
///
/// `magic_0446` 0x10001e80 (395 bytes; 6 DLLs), `magic_0544` 0x10001e80
/// (3 DLLs, its own trail draw), `magic_0565` 0x10003a20 (5 DLLs, its
/// own sparks and trail draw) and `magic_0644` 0x10007a40 (5 DLLs, its
/// own sparks and trail draw; the block's words 0x130 lower, read from
/// the code: `B[0x5ad0]` the wait, `B[0x5ad4]` the kind, `B[0x5aa8]` the
/// count, `B+0x2f70` the kept point, `B+0x5880` the trail).
pub fn trail_head<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let (p, m) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x54));
    let esp = ebp.wrapping_sub(0x60);
    let b = g.read::<u32>(w(0x14));
    // the block's words (displacements: `magic_0644`'s are 0x130 lower)
    let wait_at = b.wrapping_add(w(0x1b));
    let kind_at = b.wrapping_add(w(0x6d));
    let count_at = b.wrapping_add(w(0x76));
    let kept = b.wrapping_add(w(0x89));
    let wait = g.read::<i32>(wait_at);
    if wait > 0 {
        g.write::<i32>(wait_at, wait - 1);
    }
    let c = g.read::<u32>(w(0x36));
    let x = g.call(slot_target(g, w(0x2e), 219), 0, &[c, 0]).eax;
    g.call(slot_target(g, w(0x4c), 237), 0, &[x, 0, 0, p]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let kind = |g: &G| g.read::<u32>(kind_at);
    let eax = if g.read::<i32>(wait_at) > 0 {
        let k = kind(g);
        let call = match k {
            0 => Some(0xfe),
            2 => Some(0xf4),
            3 => Some(0xea),
            _ => None,
        };
        let eax = match call {
            Some(call) => sparks_move(g, img, img.rel32_target(f + call), esp, p),
            None => k.wrapping_sub(3),
        };
        for j in 0..4 {
            let v = g.read::<u32>(p + 4 * j);
            g.write::<u32>(kept + 4 * j, v);
        }
        eax
    } else {
        let left = g.read::<i32>(count_at);
        if kind(g) != 0 || left <= 0 {
            let back = img.rel32_target(f + 0xbe);
            return Ret::eax(crate::sa_pool::give_back(g, img, back, obj));
        }
        g.write::<i32>(count_at, left - 1);
        for j in 0..4 {
            let v = g.read::<u32>(kept + 4 * j);
            g.write::<u32>(p + 4 * j, v);
        }
        (left - 1).cast_unsigned()
    };
    if kind(g) != 0 {
        return Ret::eax(eax);
    }
    g.call(slot_target(g, w(0x144), 235), 0, &[m]);
    for o in [0x28u32, 0x14, 0] {
        g.write::<u32>(m + o, 0x3e4c_cccd);
    }
    let draw = (
        img.rel32_target(f + 0x173),
        esp.wrapping_sub(4),
        b.wrapping_add(w(0x159)),
        m,
    );
    sub_call(g, crate::o6_fam_pairs6::trail_draw, img, draw, &[p])
}

/// [`trail_head`] in `magic_0446` (the `rel32`s: the pool give-back,
/// the moves, the trail draw and `__security_check_cookie` twice).
pub const TRAIL_HEAD_0446: Sig = Sig {
    len: 395,
    hash: 0x7ccd_415a_28c9_5e5d,
    calls: &[
        (0xbe, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xcb, Callee::Any),
        (0xea, Callee::Sig(&crate::s4_fam446::SPARKS_D)),
        (0xf4, Callee::Sig(&crate::s4_fam446::SPARKS_C)),
        (0xfe, Callee::Sig(&crate::s4_fam446::SPARKS_A)),
        (
            0x173,
            Callee::Sig(&crate::o6_fam_pairs6::TRAIL_DRAW_0446_4E20),
        ),
        (0x183, Callee::Any),
    ],
    imports: &[],
};

/// [`trail_head`] in `magic_0544` (its trail draw).
pub const TRAIL_HEAD_0544: Sig = Sig {
    len: 395,
    hash: 0x7ccd_415a_28c9_5e5d,
    calls: &[
        (0xbe, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xcb, Callee::Any),
        (0xea, Callee::Sig(&crate::s4_fam446::SPARKS_D)),
        (0xf4, Callee::Sig(&crate::s4_fam446::SPARKS_C)),
        (0xfe, Callee::Sig(&crate::s4_fam446::SPARKS_A)),
        (
            0x173,
            Callee::Sig(&crate::o6_fam_pairs6::TRAIL_DRAW_0544_4CE0),
        ),
        (0x183, Callee::Any),
    ],
    imports: &[],
};

/// A light flown in and, at the end, its object handed on (cdecl `(O)`;
/// a /GS frame, `ebp` = `esp - 4`, the point `P` at `ebp-0x18`; `S =
/// O[0x14]`, `B = *G_B`): the count `S[0x50c]` goes down to 0; with `u =
/// count / S[0x510]` and `v = K - u`, `P = S[0x270..] v + S[0x260..] u`
/// (`w` 1.0); the light set up at `P` with the angles `S+0x280`
/// ([`light_setup`](crate::o6_fam_pairs5::light_setup), `n` 0x80) and `P`
/// kept in `S+0xd0`. Once the count is 0: the child started
/// ([`start_child_sound_b`](crate::s4_fam446::start_child_sound_b), `(S,
/// S+0x280, B[K])`), `S` taken off its list ([`unlist_object`]), `B[N]`
/// up by one (`K`, `N` the code's displacements: 0x8524/0x852c in
/// `magic_0565`, 0x83f4/0x83fc in `magic_0644`) and `O` given back to its pool
/// ([`give_back`](crate::sa_pool::give_back)). Returns the last callee's
/// `eax`.
///
/// `magic_0565` 0x10004750 and `magic_0644` 0x10008770 (395 bytes; 5 DLLs
/// each).
pub fn light_fly_in<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let obj = fr.arg(g, 0);
    let s = g.read::<u32>(obj.wrapping_add(0x14));
    let b = g.read::<u32>(w(0x21));
    let ebp = fr.esp.wrapping_sub(4);
    let esp = ebp.wrapping_sub(0x34);
    let left = g.read::<i32>(s + 0x50c);
    if left > 0 {
        g.write::<i32>(s + 0x50c, left - 1);
    }
    let u = div(cvt(g.read::<u32>(s + 0x50c)), cvt(g.read::<u32>(s + 0x510)));
    let v = sub(g.read::<f32>(w(0x4a)), u);
    let p = ebp.wrapping_sub(0x18);
    let mut at = [0f32; 3];
    for (i, x) in (0u32..).zip(&mut at) {
        let to = g.read::<f32>(s + 0x270 + 4 * i);
        let from = g.read::<f32>(s + 0x260 + 4 * i);
        *x = add(mul(to, v), mul(from, u));
        g.write::<f32>(p + 4 * i, *x);
    }
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let light = (img.rel32_target(f + 0xde), esp, s + 0x280, p);
    let mut eax = sub_call(g, crate::o6_fam_pairs5::light_setup, img, light, &[0x80]).eax;
    for (i, x) in (0u32..).zip(at) {
        g.write::<f32>(s + 0xd0 + 4 * i, x);
    }
    g.write::<u32>(s + 0xdc, 0x3f80_0000);
    if g.read::<i32>(s + 0x50c) <= 0 {
        // `push [edi + K]`, `inc [edi + N]`: the displacements differ per DLL
        let kind = g.read::<u32>(b.wrapping_add(w(0x121)));
        let child = (img.rel32_target(f + 0x12e), esp, s, s + 0x280);
        sub_call(
            g,
            crate::s4_fam446::start_child_sound_b,
            img,
            child,
            &[kind],
        );
        unlist_object(g, img, f, s, (0x138, 0x13d, 0x146, 0x14d, 0x152));
        let n = g.read::<u32>(b.wrapping_add(w(0x16f))).wrapping_add(1);
        g.write::<u32>(b.wrapping_add(w(0x16f)), n);
        let back = img.rel32_target(f + 0x176);
        eax = crate::sa_pool::give_back(g, img, back, obj);
    }
    Ret::eax(eax)
}

/// [`light_fly_in`] in `magic_0565` (the `rel32`s: the light set-up, the
/// child start, the list removal, the pool give-back and
/// `__security_check_cookie`).
pub const LIGHT_FLY_IN_0565: Sig = Sig {
    len: 395,
    hash: 0x3a22_0f26_8ba7_acfe,
    calls: &[
        (
            0xde,
            Callee::Sig(&crate::o6_fam_pairs5::LIGHT_SETUP_0644_B390),
        ),
        (0x12e, Callee::Sig(&crate::s4_fam446::START_CHILD_SOUND_B)),
        (0x13d, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x176, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x183, Callee::Any),
    ],
    imports: &[],
};

/// [`light_fly_in`] in `magic_0644`.
pub const LIGHT_FLY_IN_0644: Sig = Sig {
    len: 395,
    hash: 0xc8b0_f2aa_5d91_84ba,
    calls: &[
        (
            0xde,
            Callee::Sig(&crate::o6_fam_pairs5::LIGHT_SETUP_0644_B390),
        ),
        (0x12e, Callee::Sig(&crate::s4_fam446::START_CHILD_SOUND_B)),
        (0x13d, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x176, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x183, Callee::Any),
    ],
    imports: &[],
};

/// A light flown along an arc (cdecl `(O)`; a /GS frame, `ebp` = `esp -
/// 4`, the point `P` at `ebp-0x14`; `S = O[0x14]`, `B = *G_B`): the count
/// `S[0x50c]` goes down to 0; when it reaches 8 the actor block at `G_1`
/// is filled (`{*G_X, ..}`: `G_2` the immediate, `G_3` = -1, `G_4` = slot
/// 413 or 215 `(*G_X)` by `*G_F`, `G_5` = -1, `G_6` = 0, `G_7` = 1, the
/// u16 `G_8` = `*G_F != 0`) and `B[W]` = the immediate (the wait); when
/// it reaches 0, slot 257 `(slot 219 (*G_X, 0), 0)`. With `u = count /
/// S[0x510]` and `v = K1 - u`, `P` = the arc from `S+0x260` to `S+0x270`:
/// `x`, `z` blended (`v` and `u`), `y = v u K2 (S.y0 - K3) + v^2 S.y0 + u^2
/// S.y1`, `w` 1.0. While the count runs the light is set up at `P`
/// ([`light_setup`](crate::o6_fam_pairs5::light_setup), angles
/// `S+0x280`, `n` 0x80); after, `B[W]` goes down, slot 268 `(*G_Y, 5, P)`,
/// and once `B[W]` is 0 too `B[N]` goes up and `O` goes back to its pool
/// ([`give_back`](crate::sa_pool::give_back)). Returns the last callee's
/// `eax`. (`W`, `N` are the code's displacements.)
///
/// `magic_0565` 0x10005220 and `magic_0644` 0x10009060 (588 bytes; 5 DLLs
/// each).
pub fn light_arc<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x50), s);
    let obj = fr.arg(g, 0);
    let s = g.read::<u32>(obj.wrapping_add(0x14));
    let b = g.read::<u32>(w(0x21));
    let ebp = fr.esp.wrapping_sub(4);
    let wait = b.wrapping_add(w(0xbd));
    let left = g.read::<i32>(s + 0x50c);
    if left > 0 {
        let left = left - 1;
        g.write::<i32>(s + 0x50c, left);
        if left == 8 {
            let x = g.read::<u32>(w(0x45));
            g.write::<u32>(w(0x4a), x);
            g.write::<u32>(w(0x56), w(0x5a));
            g.write::<u32>(w(0x60), u32::MAX);
            let slot = if g.read::<u32>(w(0x3f)) == 0 {
                215
            } else {
                413
            };
            let m = g.call(t(g, slot), 0, &[x]).eax;
            let flag = g.read::<u32>(w(0x7f)) != 0;
            g.write::<u32>(w(0x8a), m);
            g.write::<u32>(w(0x90), u32::MAX);
            g.write::<u32>(w(0x9a), 0);
            g.write::<u32>(w(0xa4), 1);
            g.write::<u16>(w(if flag { 0xb7 } else { 0xae }), u16::from(flag));
            g.write::<u32>(wait, w(0xc1));
        }
        if g.read::<u32>(s + 0x50c) == 0 {
            let x = g.read::<u32>(w(0xd7));
            let x = g.call(t(g, 219), 0, &[x, 0]).eax;
            g.call(t(g, 257), 0, &[x, 0]);
        }
    }
    let u = div(cvt(g.read::<u32>(s + 0x50c)), cvt(g.read::<u32>(s + 0x510)));
    let v = sub(g.read::<f32>(w(0x110)), u);
    let r = |g: &G, o: u32| g.read::<f32>(s + o);
    let p = ebp.wrapping_sub(0x14);
    let x = add(mul(r(g, 0x260), v), mul(r(g, 0x270), u));
    let rise = mul(mul(v, u), g.read::<f32>(w(0x146)));
    let z = add(mul(r(g, 0x268), v), mul(r(g, 0x278), u));
    let low = r(g, 0x264);
    let y = add(
        add(
            mul(rise, sub(low, g.read::<f32>(w(0x1ab)))),
            mul(mul(v, v), low),
        ),
        mul(mul(u, u), r(g, 0x274)),
    );
    for (i, c) in (0u32..).zip([x, y, z]) {
        g.write::<f32>(p + 4 * i, c);
    }
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    if g.read::<i32>(s + 0x50c) > 0 {
        let light = (
            img.rel32_target(f + 0x215),
            ebp.wrapping_sub(0x20),
            s + 0x280,
            p,
        );
        let eax = sub_call(g, crate::o6_fam_pairs5::light_setup, img, light, &[0x80]).eax;
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(wait).wrapping_sub(1);
    g.write::<u32>(b.wrapping_add(w(0x1d6)), n);
    let y = g.read::<u32>(w(0x1e7));
    let eax = g.call(t(g, 268), 0, &[y, 5, p]).eax;
    if g.read::<i32>(b.wrapping_add(w(0x227))) > 0 {
        return Ret::eax(eax);
    }
    let done = b.wrapping_add(w(0x230));
    let n = g.read::<u32>(done).wrapping_add(1);
    g.write::<u32>(done, n);
    let back = img.rel32_target(f + 0x237);
    Ret::eax(crate::sa_pool::give_back(g, img, back, obj))
}

/// [`light_arc`] in `magic_0565` (the `rel32`s: the light set-up, the
/// pool give-back and `__security_check_cookie`).
pub const LIGHT_ARC_0565: Sig = Sig {
    len: 588,
    hash: 0x5dd4_b294_f2ec_7f19,
    calls: &[
        (
            0x215,
            Callee::Sig(&crate::o6_fam_pairs5::LIGHT_SETUP_0644_B390),
        ),
        (0x237, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x244, Callee::Any),
    ],
    imports: &[],
};

/// [`light_arc`] in `magic_0644`.
pub const LIGHT_ARC_0644: Sig = Sig {
    len: 588,
    hash: 0x0337_1a48_769a_ce15,
    calls: LIGHT_ARC_0565.calls,
    imports: &[],
};

/// A light drifting and turning (cdecl `(O)`; `S = O[0x14]`, `B = *G_B`):
/// the count `S[0x50c]` goes down to 0; the drift `S+0x2a0..` (its `y`
/// growing by `K` first) moves the point `S+0x260..`; the two angles
/// `S+0x280`, `+0x284` turn by `S+0x290`, `+0x298`, brought back once by
/// `KT` when above `KH` or below `KL`; the light set up at the point
/// ([`light_setup`](crate::o6_fam_pairs5::light_setup), `n` = `count *
/// 128 / 20`, 0x80 from 20 on). Once the count is 0, `B[N]` goes up and
/// the original jumps to the pool give-back of `O`
/// ([`give_back`](crate::sa_pool::give_back)). Returns the last callee's
/// `eax`.
///
/// `magic_0644` 0x10008f20 (317 bytes; 5 DLLs).
pub fn light_drift<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let obj = fr.arg(g, 0);
    let s = g.read::<u32>(obj.wrapping_add(0x14));
    let b = g.read::<u32>(w(0x14));
    let left = g.read::<i32>(s + 0x50c);
    if left > 0 {
        g.write::<i32>(s + 0x50c, left - 1);
    }
    let r = |g: &G, o: u32| g.read::<f32>(s + o);
    let rise = add(r(g, 0x2a4), k(g, 0x2f));
    g.write::<f32>(s + 0x2a4, rise);
    g.write::<f32>(s + 0x260, add(r(g, 0x2a0), r(g, 0x260)));
    g.write::<f32>(s + 0x264, add(r(g, 0x264), r(g, 0x2a4)));
    g.write::<f32>(s + 0x268, add(r(g, 0x2a8), r(g, 0x268)));
    let (high, turn, low) = (k(g, 0x3d), k(g, 0x45), k(g, 0x4d));
    for (angle, speed) in [(0x280u32, 0x290u32), (0x284, 0x298)] {
        let a = add(r(g, speed), r(g, angle));
        let a = if a > high {
            sub(a, turn)
        } else if low > a {
            add(a, turn)
        } else {
            a
        };
        g.write::<f32>(s + angle, a);
    }
    let count = g.read::<i32>(s + 0x50c);
    let n = if count >= 0x14 {
        0x80
    } else {
        count.wrapping_shl(7) / 20
    };
    let esp = fr.esp.wrapping_sub(0x10);
    let light = (img.rel32_target(f + 0x117), esp, s + 0x280, s + 0x260);
    let eax = sub_call(
        g,
        crate::o6_fam_pairs5::light_setup,
        img,
        light,
        &[n.cast_unsigned()],
    )
    .eax;
    if g.read::<i32>(s + 0x50c) > 0 {
        return Ret::eax(eax);
    }
    let done = b.wrapping_add(w(0x129));
    let n = g.read::<u32>(done).wrapping_add(1);
    g.write::<u32>(done, n);
    let back = img.rel32_target(f + 0x134);
    Ret::eax(crate::sa_pool::give_back(g, img, back, obj))
}

/// [`light_drift`] (the `rel32`s: the light set-up and the pool
/// give-back it jumps to).
pub const LIGHT_DRIFT: Sig = Sig {
    len: 317,
    hash: 0x8896_079b_751c_bdf6,
    calls: &[
        (
            0x117,
            Callee::Sig(&crate::o6_fam_pairs5::LIGHT_SETUP_0644_B390),
        ),
        (0x134, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// Where [`light_land`]'s two copies differ: `magic_0565`'s picks the
/// child's program (0x19 after the sounds, 4 without them) and so sits
/// `shift` = 0xb bytes later from the object start on, its `K2` at
/// `k2`; `magic_0644`'s always starts program 4 (`push imm8` at
/// `program`).
pub struct LightLand {
    shift: u32,
    k2: u32,
    program: (u32, u32),
}

/// `magic_0644` 0x1000a180.
const LAND_0644: LightLand = LightLand {
    shift: 0,
    k2: 0x1dc,
    program: (0x1ec, 0x1ec),
};

/// `magic_0565` 0x10006330 (the `mov eax, imm32` after the sounds and
/// the one they skip to).
const LAND_0565: LightLand = LightLand {
    shift: 0xb,
    k2: 0x1e8,
    program: (0x1d9, 0x1e0),
};

/// A light landing (cdecl `(O)`; a /GS frame, `ebp` = `esp - 4`, the
/// point `P` at `ebp-0x18`; `S = O[0x14]`, `B = *G_B`): the count
/// `S[0x50c]` goes down to 0; with `u = count / S[0x510]` and `v = K1 -
/// u`, `P = S[0x260..] u + S[0x270..] v` (`w` 1.0), the light set up there
/// ([`light_setup`](crate::o6_fam_pairs5::light_setup), angles `S+0x280`,
/// `n` 0x80) and `P` kept in `S+0xd0`. Once the count is 0: unless
/// `S[0x514]` is set, slot 239 `(*G_C, (*G_L)[e], 0)` for each battle
/// number `e` of the list `L = B[D1]` (`L[4]` of them from `L+8`), slot
/// 306 `(7, 2, slot 719 (X1, 9, 1))` and slot 393 `(slot 719 (X2, 6,
/// 0x12), S+0x270, 0x100)`; the child `S+0x130` started at the scale `K2`
/// ([`start_object_at`](crate::s4_misc::start_object_at), the program by
/// [`LightLand`]) at `S+0x270` lowered by `K3`, its `+0xc4` = `B[D2]`; `S`
/// off its list ([`unlist_object`]), `B[D3]` up by one, `O` given back to
/// its pool ([`give_back`](crate::sa_pool::give_back)). Returns the last
/// callee's `eax`. (`D1`..`D3` are the code's displacements.)
fn light_land<G: Game>(g: &mut G, img: &Image, fr: &Frame, l: &LightLand) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let ws = |o: u32| op(img, f, o + l.shift);
    let t = |g: &G, s: u32| slot_target(g, w(0x161), s);
    let obj = fr.arg(g, 0);
    let b = g.read::<u32>(w(0x16));
    let s = g.read::<u32>(obj.wrapping_add(0x14));
    let ebp = fr.esp.wrapping_sub(4);
    let left = g.read::<i32>(s + 0x50c);
    if left > 0 {
        g.write::<i32>(s + 0x50c, left - 1);
    }
    let u = div(cvt(g.read::<u32>(s + 0x50c)), cvt(g.read::<u32>(s + 0x510)));
    let v = sub(g.read::<f32>(w(0x4d)), u);
    let p = ebp.wrapping_sub(0x18);
    let mut at = [0f32; 3];
    for (i, x) in (0u32..).zip(&mut at) {
        let from = g.read::<f32>(s + 0x260 + 4 * i);
        let to = g.read::<f32>(s + 0x270 + 4 * i);
        *x = add(mul(from, u), mul(v, to));
        g.write::<f32>(p + 4 * i, *x);
    }
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let light = (
        img.rel32_target(f + 0xdb),
        ebp.wrapping_sub(0x34),
        s + 0x280,
        p,
    );
    let eax = sub_call(g, crate::o6_fam_pairs5::light_setup, img, light, &[0x80]).eax;
    for (i, x) in (0u32..).zip(at) {
        g.write::<f32>(s + 0xd0 + 4 * i, x);
    }
    g.write::<u32>(s + 0xdc, 0x3f80_0000);
    if g.read::<i32>(s + 0x50c) > 0 {
        return Ret::eax(eax);
    }
    let program = if g.read::<u32>(s + 0x514) == 0 {
        let list = |g: &G| g.read::<u32>(b.wrapping_add(w(0x12f)));
        let mut i = 0u32;
        while i.cast_signed() < g.read::<i32>(list(g).wrapping_add(4)) {
            let e = u32::from(g.read::<u8>(list(g).wrapping_add(8 + i)));
            let c = g.read::<u32>(w(0x147));
            let n = u32::from(g.read::<u8>(g.read::<u32>(w(0x155)).wrapping_add(e)));
            g.call(t(g, 239), 0, &[c, n, 0]);
            i += 1;
        }
        let h = g.call(t(g, 719), 0, &[w(0x18d), 9, 1]).eax;
        g.call(t(g, 306), 0, &[7, 2, h]);
        let h = g.call(t(g, 719), 0, &[w(0x1b5), 6, 0x12]).eax;
        g.call(t(g, 393), 0, &[h, s + 0x270, 0x100]);
        w(l.program.0) & 0xff
    } else {
        w(l.program.1) & 0xff
    };
    let child = s + 0x130;
    let start = img.rel32_target(f + 0x1f0 + l.shift);
    let scale = g.read::<f32>(w(l.k2));
    crate::s4_misc::start_object_at(g, img, start, child, scale, program, 0, 0);
    for i in 0..4 {
        let x = g.read::<u32>(s + 0x270 + 4 * i);
        g.write::<u32>(child + 0xd0 + 4 * i, x);
    }
    let y = sub(g.read::<f32>(child + 0xd4), g.read::<f32>(ws(0x230)));
    let c4 = g.read::<u32>(b.wrapping_add(ws(0x236)));
    g.write::<u32>(child + 0xc4, c4);
    g.write::<f32>(child + 0xd4, y);
    let unlist = [0x240, 0x253, 0x25c, 0x263, 0x268].map(|o| o + l.shift);
    unlist_object(
        g,
        img,
        f,
        s,
        (unlist[0], unlist[1], unlist[2], unlist[3], unlist[4]),
    );
    let done = b.wrapping_add(ws(0x288));
    let n = g.read::<u32>(done).wrapping_add(1);
    g.write::<u32>(done, n);
    let back = img.rel32_target(f + 0x28d + l.shift);
    Ret::eax(crate::sa_pool::give_back(g, img, back, obj))
}

/// [`light_land`] in `magic_0644` (program 4).
///
/// `magic_0644` 0x1000a180 (674 bytes; 5 DLLs).
pub fn light_land_0644<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    light_land(g, img, fr, &LAND_0644)
}

/// [`light_land`] in `magic_0565` (program 0x19 after the sounds).
///
/// `magic_0565` 0x10006330 (685 bytes; 5 DLLs).
pub fn light_land_0565<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    light_land(g, img, fr, &LAND_0565)
}

/// [`light_land_0644`] (the `rel32`s: the light set-up, the object
/// start, the list removal, the pool give-back and
/// `__security_check_cookie`).
pub const LIGHT_LAND_0644: Sig = Sig {
    len: 674,
    hash: 0x8d96_735b_3fd7_0dbc,
    calls: &[
        (
            0xdb,
            Callee::Sig(&crate::o6_fam_pairs5::LIGHT_SETUP_0644_B390),
        ),
        (0x1f0, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x253, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x28d, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x29a, Callee::Any),
    ],
    imports: &[],
};

/// [`light_land_0565`] (the same `rel32`s, 0xb bytes later from the
/// object start on).
pub const LIGHT_LAND_0565: Sig = Sig {
    len: 685,
    hash: 0x42a3_3af1_0cc4_34a7,
    calls: &[
        (
            0xdb,
            Callee::Sig(&crate::o6_fam_pairs5::LIGHT_SETUP_0644_B390),
        ),
        (0x1fb, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x25e, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x298, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x2a5, Callee::Any),
    ],
    imports: &[],
};

/// Whether a bounce stops the axis: its new speed `v` is zeroed when it
/// keeps the sign the wall `wall` allows (`0 > wall` and `0 > v`, or
/// neither `0 > wall` nor `v < 0`, the `comiss` order of the original).
fn bounce_stops(wall: f32, v: f32) -> bool {
    if 0.0 > wall {
        0.0 > v
    } else {
        !(v < 0.0) && !v.is_nan()
    }
}

/// A light bouncing on the floor (cdecl `(O)`; `S = O[0x14]`, `B =
/// *G_B`): the count `S[0x50c]` goes down to 0; the speed `S+0x2a0..`
/// (its `y` growing by `K1` first) moves the point `S+0x260..`. At or
/// past the floor `F = B[D] - K2` the speed bounces (`x`, `z` times `K4`,
/// `y` times `K3`, kept in `S+0x270..` and back in `S+0x2a0..`; `y` stops
/// when above 0, `x`/`z` when they keep the side their wall `S+0x2b0` /
/// `+0x2b8` allows, [`bounce_stops`]; a `y` above 0 would also pull the
/// point back along its last step), the point sits on the floor, and the
/// turn speeds `S+0x290`, `+0x294` scale by `|speed| / S[0x27c]`
/// (`_libm_sse2_sqrt_precise`). The two angles `S+0x280`, `+0x284` turn by
/// `S+0x290`, `+0x298`, brought back once by `KT` when above `KH` or below
/// `KL`; the light set up at the point
/// ([`light_setup`](crate::o6_fam_pairs5::light_setup), `n` = `count *
/// 128 / 20`, 0x80 from 20 on). Once the count is 0, `B[N]` goes up and
/// `O` goes back to its pool ([`give_back`](crate::sa_pool::give_back)).
/// Returns the last callee's `eax`. (`D`, `N` are the code's
/// displacements.)
///
/// `magic_0565` 0x10004f00 (795 bytes; 5 DLLs).
pub fn light_bounce<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let obj = fr.arg(g, 0);
    let b = g.read::<u32>(w(0xe));
    let s = g.read::<u32>(obj.wrapping_add(0x14));
    let left = g.read::<i32>(s + 0x50c);
    if left > 0 {
        g.write::<i32>(s + 0x50c, left - 1);
    }
    let r = |g: &G, o: u32| g.read::<f32>(s + o);
    let (x0, y0, z0) = (r(g, 0x260), r(g, 0x264), r(g, 0x268));
    g.write::<f32>(s + 0x2a4, add(r(g, 0x2a4), k(g, 0x5b)));
    g.write::<f32>(s + 0x260, add(r(g, 0x260), r(g, 0x2a0)));
    g.write::<f32>(s + 0x264, add(r(g, 0x264), r(g, 0x2a4)));
    g.write::<f32>(s + 0x268, add(r(g, 0x2a8), r(g, 0x268)));
    let floor = sub(g.read::<f32>(b.wrapping_add(w(0xa5))), k(g, 0xad));
    if !(r(g, 0x264) < floor) && !r(g, 0x264).is_nan() && !floor.is_nan() {
        let vy = mul(r(g, 0x2a4), k(g, 0xd6));
        let vx = mul(r(g, 0x2a0), k(g, 0xe6));
        let vz = mul(r(g, 0x2a8), k(g, 0xee));
        g.write::<f32>(s + 0x270, vx);
        g.write::<f32>(s + 0x274, vy);
        g.write::<f32>(s + 0x278, vz);
        if vy > 0.0 {
            g.write::<u32>(s + 0x274, 0);
        }
        if bounce_stops(r(g, 0x2b0), vx) {
            g.write::<u32>(s + 0x270, 0);
        }
        if bounce_stops(r(g, 0x2b8), vz) {
            g.write::<u32>(s + 0x278, 0);
        }
        for i in 0..4 {
            let v = g.read::<u32>(s + 0x270 + 4 * i);
            g.write::<u32>(s + 0x2a0 + 4 * i, v);
        }
        if r(g, 0x274) > 0.0 {
            // (not reached: `y` was zeroed when above 0)
            let y = r(g, 0x264);
            let back = div(sub(y, floor), sub(y, y0));
            g.write::<f32>(s + 0x260, add(mul(sub(r(g, 0x260), x0), back), x0));
            g.write::<f32>(s + 0x268, add(mul(sub(r(g, 0x268), z0), back), z0));
        }
        g.write::<f32>(s + 0x264, floor);
        let (vx, vy, vz) = (r(g, 0x2a0), r(g, 0x2a4), r(g, 0x2a8));
        let sq = add(add(mul(vx, vx), mul(vy, vy)), mul(vz, vz));
        #[allow(clippy::cast_possible_truncation)] // `cvtsd2ss`
        let speed = f64::from(sq).sqrt() as f32;
        let ratio = div(speed, r(g, 0x27c));
        g.write::<f32>(s + 0x290, mul(r(g, 0x290), ratio));
        g.write::<f32>(s + 0x294, mul(r(g, 0x294), ratio));
    }
    let (high, turn, low) = (k(g, 0x267), k(g, 0x26f), k(g, 0x277));
    for (angle, speed) in [(0x280u32, 0x290u32), (0x284, 0x298)] {
        let a = add(r(g, speed), r(g, angle));
        let a = if a > high {
            sub(a, turn)
        } else if low > a {
            add(a, turn)
        } else {
            a
        };
        g.write::<f32>(s + angle, a);
    }
    let count = g.read::<i32>(s + 0x50c);
    let n = if count >= 0x14 {
        0x80
    } else {
        count.wrapping_shl(7) / 20
    };
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x18);
    let light = (img.rel32_target(f + 0x2f1), esp, s + 0x280, s + 0x260);
    let eax = sub_call(
        g,
        crate::o6_fam_pairs5::light_setup,
        img,
        light,
        &[n.cast_unsigned()],
    )
    .eax;
    if g.read::<i32>(s + 0x50c) > 0 {
        return Ret::eax(eax);
    }
    let done = b.wrapping_add(w(0x30b));
    let n = g.read::<u32>(done).wrapping_add(1);
    g.write::<u32>(done, n);
    let back = img.rel32_target(f + 0x310);
    Ret::eax(crate::sa_pool::give_back(g, img, back, obj))
}

/// [`light_bounce`] (the `rel32`s: the square root, the light set-up and
/// the pool give-back).
pub const LIGHT_BOUNCE: Sig = Sig {
    len: 795,
    hash: 0x10ed_6256_6b19_3781,
    calls: &[
        (0x218, Callee::Import("_libm_sse2_sqrt_precise")),
        (
            0x2f1,
            Callee::Sig(&crate::o6_fam_pairs5::LIGHT_SETUP_0644_B390),
        ),
        (0x310, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// The 12-byte thunk over [`lit_colour_node`](crate::s7_misc::lit_colour_node)
/// (`ecx` = the first stack word, a jump with the caller's stack).
///
/// `magic_0152` 0x10001470 (12 bytes; 2 DLLs).
pub fn lit_colour_node_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::o6_fam_pairs3::thunk_to(g, img, fr, |g, img, fr| {
        Ret::eax(crate::s7_misc::lit_colour_node(g, img, fr))
    })
}

/// [`lit_colour_node_thunk`].
pub const LIT_COLOUR_NODE_THUNK: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&crate::s7_misc::LIT_COLOUR_NODE))],
    imports: &[],
};

/// The scrolled texture's fade (cdecl, no arguments; `V = *G_V`): while
/// `*G_A` is set and the clock `*G_T` is below 0x99 nothing happens
/// (returns 1). Else `V` drops by `K1` when slot 253 answers non-zero, and
/// is kept within `0..=K2` (stored when it was out); slot 38 `(*G_W, V,
/// 0)` and the texture scrolled
/// ([`scrolled_texture`](crate::o6_fam_pairs5::scrolled_texture)). Once
/// `V` equals `K3` exactly, `*G_D = 1` and it returns 0, else 1.
///
/// `magic_0188` 0x10001db0 (173 bytes; 2 DLLs).
pub fn texture_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x1a), s);
    if g.read::<u32>(w(0x2)) != 0 && g.read::<i32>(w(0xb)) < 0x99 {
        return Ret::eax(1);
    }
    let fade = w(0x2a);
    let down = g.call(t(g, 253), 0, &[]).eax;
    let mut v = g.read::<f32>(fade);
    if down != 0 {
        v = sub(v, g.read::<f32>(w(0x36)));
        g.write::<f32>(fade, v);
    }
    let top = g.read::<f32>(w(0x4e));
    if 0.0 > v {
        v = 0.0;
        g.write::<f32>(fade, v);
    } else if v > top {
        v = top;
        g.write::<f32>(fade, v);
    }
    let target = g.read::<u32>(w(0x77));
    g.call(t(g, 38), 0, &[target, v.to_bits(), 0]);
    let scroll = (img.rel32_target(f + 0x81), fr.esp, 0, 0);
    sub_call(g, crate::o6_fam_pairs5::scrolled_texture, img, scroll, &[]);
    if g.read::<f32>(fade) == g.read::<f32>(w(0x90)) {
        g.write::<u32>(w(0x9c), 1);
        return Ret::eax(0);
    }
    Ret::eax(1)
}

/// [`texture_fade`] (the `rel32`: the scrolled texture).
pub const TEXTURE_FADE: Sig = Sig {
    len: 173,
    hash: 0x8dfe_c3ce_7c1c_231e,
    calls: &[(
        0x81,
        Callee::Sig(&crate::o6_fam_pairs5::SCROLLED_TEXTURE_0188_1220),
    )],
    imports: &[],
};

/// The yaw toward the camera as the summon starts compute it: `KA` -
/// slot 333 `(*G_A)` (ST0, as f32), brought back once by `KT` when above
/// `KH` or below `KL` (the operands `[table, G_A, KA, KH, KT, KL, KT']`).
fn camera_yaw<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    [table, a, ka, kh, kt, kl, kt2]: [u32; 7],
) -> f32 {
    use crate::sb_sse::{add, sub};
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let actor = g.read::<u32>(w(a));
    let r = crate::m0018::st0_f32(g.call(slot_target(g, w(table), 333), 0, &[actor]).st0);
    let y = sub(k(g, ka), r);
    if y > k(g, kh) {
        sub(y, k(g, kt))
    } else if k(g, kl) > y {
        add(y, k(g, kt2))
    } else {
        y
    }
}

/// A node from the DLL's pool ([`take`](crate::sa_pool::take) at the
/// `rel32` `call`, owner the immediate at `owner`) appended to the busy
/// list (`G_FIRST` at `first`, `G_LAST` at `last`) with the byte `+0x18`
/// = `tag`; the list's last is updated only when `set_last` (the
/// original's first node of a pair leaves it for the second). Returns
/// the node.
fn busy_node<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (call, owner, tag): (u32, u32, u8),
    (first, last, first_w, last_w): (u32, u32, u32, Option<u32>),
) -> u32 {
    let w = |o: u32| op(img, f, o);
    let n = crate::sa_pool::take(g, img, img.rel32_target(f + call), w(owner));
    g.write::<u8>(n.wrapping_add(0x18), tag);
    g.write::<u32>(n.wrapping_add(4), 0);
    if g.read::<u32>(w(first)) == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(first_w), n);
    } else {
        let l = g.read::<u32>(w(last));
        g.write::<u32>(n, l);
        g.write::<u32>(l.wrapping_add(4), n);
    }
    g.write::<u32>(n.wrapping_add(8), n);
    if let Some(o) = last_w {
        g.write::<u32>(w(o), n);
    }
    n
}

/// A summon's two objects and its sparks started (cdecl, no arguments; a
/// /GS frame, `ebp` = `esp - 4`, the actors' mean `Q` at `ebp-0x14`):
/// slot 719 `(X, 8, 0)` into `*G_H` (three globals set to immediates
/// first), the part set up ([`part_init`](crate::l5_objhost::part_init));
/// the first object `O1` started at `K1` for the kind `*G_K`'s owner
/// `T[k] - 1` ([`start_object_at`](crate::s4_misc::start_object_at)),
/// its point `P1` = slot 262 `(*G_A, P1..)`, `w` 1.0, its yaw
/// ([`camera_yaw`]) and record (constant stores); `Q` the actors' mean
/// ([`mean_position`](crate::sb_misc::mean_position)); the second `O2`
/// at `K2` (owner `T[k] + 0x13`, the callback `CB`), its point from slot
/// 262 of the first entry's actor and then `Q` (also kept in a second
/// record with `k` and `T[k]`), its yaw and record; the effect's block
/// ([`me_block`](crate::sa_misc::me_block)); three busy nodes
/// ([`busy_node`], tags 1, 0 and 2) around the pool's reset
/// ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take)). The
/// original pushes the `ecx` its callees leave as the object starts'
/// unread third word. Returns 0.
///
/// `magic_0242` 0x10003680 (943 bytes; 5 DLLs).
pub fn summon_pair_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x11), s);
    let ebp = fr.esp.wrapping_sub(4);
    let q = ebp.wrapping_sub(0x14);
    global_stores(g, img, f, 0x26, 0x44, 0);
    let mut st = Pushes::default();
    st.push_all(&[0, 8, w(0x22)]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let h = st.call(g, t(g, 719), fr.ecx).eax;
    g.write::<u32>(w(0x4a), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x4f));
    let kind = g.read::<u32>(w(0x55));
    let owner = g.read::<u32>(kind.wrapping_mul(4).wrapping_add(w(0x64)));
    let start = img.rel32_target(f + 0x7a);
    let k1 = g.read::<f32>(w(0x5d));
    let o1 = w(0x75);
    crate::s4_misc::start_object_at(g, img, start, o1, k1, 0, owner.wrapping_sub(1), 0);
    let a = g.read::<u32>(w(0x9a));
    g.call(t(g, 262), 0, &[a, w(0x94), w(0x8f), w(0x84)]);
    g.write::<u32>(w(0xad), w(0xb1));
    let y = camera_yaw(g, img, f, [0xa1, 0xa7, 0xc4, 0xdb, 0xd1, 0xeb, 0xd1]);
    g.write::<f32>(w(0x101), y);
    global_stores(g, img, f, 0x105, 0x165, 0);
    let mean = img.rel32_target(f + 0x166);
    crate::sb_misc::mean_position(g, img, mean, ebp.wrapping_sub(0x24), q);
    let k2 = g.read::<f32>(w(0x16e));
    let o2 = w(0x17f);
    let start2 = img.rel32_target(f + 0x184);
    crate::s4_misc::start_object_at(
        g,
        img,
        start2,
        o2,
        k2,
        1,
        owner.wrapping_add(0x13),
        w(0x173),
    );
    let first = u32::from(g.read::<u8>(g.read::<u32>(w(0x189))));
    let actor = g.read::<u32>(first.wrapping_mul(4).wrapping_add(w(0x1a2)));
    g.call(t(g, 262), 0, &[actor, w(0x19b), w(0x196), w(0x18e)]);
    g.write::<u32>(w(0x1c0), w(0x1c4));
    let y = camera_yaw(g, img, f, [0x1b4, 0x1ba, 0x1d7, 0x1e6, 0x1f0, 0x1fa, 0x207]);
    let mean = [0, 1, 2, 3].map(|i| g.read::<u32>(q + 4 * i));
    g.write::<f32>(w(0x21e), y);
    for (i, &o) in [0x232u32, 0x23a, 0x242, 0x24a].iter().enumerate() {
        g.write::<u32>(w(o), mean[i]);
    }
    global_stores(g, img, f, 0x24e, 0x2ae, 0);
    for (i, &o) in [0x2b2u32, 0x2ba, 0x2c2, 0x2ca].iter().enumerate() {
        g.write::<u32>(w(o), mean[i]);
    }
    g.write::<u32>(w(0x2cf), 0);
    g.write::<u32>(w(0x2d5), kind);
    g.write::<u32>(w(0x2db), owner);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x2e0), w(0x22a));
    let n1 = busy_node(g, img, f, (0x2ed, 0x2e8, 1), (0x2f3, 0x30a, 0x31f, None));
    let had = g.read::<u32>(w(0x2f3)) != 0;
    let n2 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x32c), w(0x324));
    g.write::<u32>(n2.wrapping_add(4), 0);
    if had {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1.wrapping_add(4), n2);
    } else {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x349), n2);
    }
    g.write::<u32>(n2.wrapping_add(8), n2);
    g.write::<u32>(w(0x356), n2);
    crate::o6_fam_pool::pool_reset_take(g, img, img.rel32_target(f + 0x35b), w(0x34e));
    busy_node(
        g,
        img,
        f,
        (0x365, 0x360, 2),
        (0x36b, 0x381, 0x393, Some(0x39e)),
    );
    Ret::eax(0)
}

/// [`summon_pair_start`] (the `rel32`s: the part set-up, the object
/// starts, the mean position, the effect's block, the pool takes and
/// reset, `__security_check_cookie`).
pub const SUMMON_PAIR_START: Sig = Sig {
    len: 943,
    hash: 0x4fda_58c3_842e_bf85,
    calls: &[
        (0x4f, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x7a, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x166, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x184, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x2e0, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x2ed, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x32c, Callee::Sig(&crate::sa_pool::TAKE)),
        (
            0x35b,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0180_2140),
        ),
        (0x365, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x3a7, Callee::Any),
    ],
    imports: &[],
};

/// The summon's start task
/// ([`summon_start_task`](crate::s8_misc::summon_start_task)) with
/// [`summon_pair_start`].
///
/// `magic_0242` 0x10003550 (118 bytes; 5 DLLs).
pub fn summon_start_task_0242<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::s8_misc::summon_start_task(g, img, fr, summon_pair_start)
}

/// [`summon_start_task_0242`].
pub const SUMMON_START_TASK_0242: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&SUMMON_PAIR_START))],
    imports: &[],
};

/// A node taken off the DLL's free ring as the summon starts inline it
/// (the count u16 at the operand `count`, the head at `head`, the
/// stores at `head_w`/`head_0` (`None`: the same `mov [G], reg`), the
/// callback immediate `cb`, the count's store `count_w`): unlinked, the
/// head moved to its next (0 when it was the last), its words cleared,
/// `+8` itself, `+0x10` the callback, byte `+0x18` 0. Returns the node
/// (0 with none left; the original then writes near address 0).
fn ring_take<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (count, head, head_w, head_0, cb, count_w): (u32, u32, u32, Option<u32>, u32, u32),
) -> u32 {
    let w = |o: u32| op(img, f, o);
    let free = g.read::<u16>(w(count));
    if free == 0 {
        return 0;
    }
    let n = g.read::<u32>(w(head));
    let (prev, next) = (g.read::<u32>(n), g.read::<u32>(n + 4));
    g.write::<u32>(prev.wrapping_add(4), next);
    let (next, prev) = (g.read::<u32>(n + 4), g.read::<u32>(n));
    g.write::<u32>(next, prev);
    if free > 1 {
        let next = g.read::<u32>(n + 4);
        g.write::<u32>(w(head_w), next);
    } else {
        g.write::<u32>(w(head_0.unwrap_or(head_w)), 0);
    }
    for o in [0u32, 4, 0xc, 0x14] {
        g.write::<u32>(n + o, 0);
    }
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 0x10, w(cb));
    g.write::<u8>(n + 0x18, 0);
    g.write::<u16>(w(count_w), free - 1);
    n
}

/// A summon's two objects started over the actors (cdecl, no arguments;
/// a /GS frame, `ebp` = `esp - 4`, the point `Q` at `ebp-0x14`): slot 719
/// `(X, 8, 0)` into `*G_H` (two globals set first), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); the first object `O1`
/// at `K1` for the kind `k = *G_K` (owner `T[k] - 1`;
/// [`start_object_at`](crate::s4_misc::start_object_at)), its point `P1`
/// = slot 262 `(*G_A, P1..)` with `w` 1.0, its yaw ([`camera_yaw`]) and
/// record (constant stores). `Q` is the point (slot 262) of the last
/// battle number whose slot 390 answers 0x1066, else the actors' mean
/// ([`mean_position`](crate::sb_misc::mean_position)). The second object
/// `O2` at `K2` (owner `T[k] + 0x13`, the callback `CB`), its point `P2`
/// from slot 262 of the first entry's actor with `y` = slot 12's (ST0),
/// `w` 1.0, its yaw; then `P2 = (Q.x, P2.y - K3, Q.z)` kept with its
/// record, the actor block (slot 413 or 215 `(k)` by `*G_B`) filled
/// inline; two task nodes taken ([`ring_take`]) and listed (the list's
/// last updated only after the second). The original pushes the `ecx`
/// its callees leave as the object starts' unread third word. Returns 0.
///
/// `magic_0241` 0x10001340 (1,342 bytes; 5 DLLs).
pub fn summon_actor_pair_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x11), s);
    let ebp = fr.esp.wrapping_sub(4);
    let q = ebp.wrapping_sub(0x14);
    global_stores(g, img, f, 0x27, 0x3b, 0);
    let mut st = Pushes::default();
    st.push_all(&[0, 8, w(0x23)]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let h = st.call(g, t(g, 719), fr.ecx).eax;
    g.write::<u32>(w(0x41), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x46));
    let kind = g.read::<u32>(w(0x4b));
    let owner = g.read::<u32>(kind.wrapping_mul(4).wrapping_add(w(0x5a)));
    let start = img.rel32_target(f + 0x73);
    let k1 = g.read::<f32>(w(0x53));
    crate::s4_misc::start_object_at(g, img, start, w(0x6e), k1, 0, owner.wrapping_sub(1), 0);
    let a = g.read::<u32>(w(0x93));
    g.call(t(g, 262), 0, &[a, w(0x8d), w(0x88), w(0x7d)]);
    g.write::<u32>(w(0xa6), w(0xaa));
    let y = camera_yaw(g, img, f, [0x9a, 0xa0, 0xbd, 0xd4, 0xca, 0xe4, 0xca]);
    g.write::<f32>(w(0xfb), y);
    global_stores(g, img, f, 0xff, 0x15f, 0);
    // the actors: the last one whose slot 390 answers 0x1066 gives `Q`
    let mut found = false;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0x161)) {
        let list = g.read::<u32>(w(0x174));
        let n = u32::from(g.read::<u8>(list.wrapping_add(i)));
        if g.call(t(g, 390), 0, &[n]).eax == 0x1066 {
            let list = g.read::<u32>(w(0x1a1));
            let n = u32::from(g.read::<u8>(list.wrapping_add(i)));
            let actor = g.read::<u32>(n.wrapping_mul(4).wrapping_add(w(0x1ac)));
            g.call(t(g, 262), 0, &[actor, q, q + 4, q + 8]);
            g.write::<u32>(q + 0xc, 0x3f80_0000);
            found = true;
        }
        i += 1;
    }
    if !found {
        let mean = img.rel32_target(f + 0x1ea);
        crate::sb_misc::mean_position(g, img, mean, ebp.wrapping_sub(0x2c), q);
    }
    let k2 = g.read::<f32>(w(0x1f2));
    let start2 = img.rel32_target(f + 0x208);
    crate::s4_misc::start_object_at(
        g,
        img,
        start2,
        w(0x203),
        k2,
        1,
        owner.wrapping_add(0x13),
        w(0x1f7),
    );
    let first = u32::from(g.read::<u8>(g.read::<u32>(w(0x20d))));
    let actor = g.read::<u32>(first.wrapping_mul(4).wrapping_add(w(0x221)));
    g.call(t(g, 262), 0, &[actor, w(0x22b), w(0x21a), w(0x212)]);
    let h = crate::m0018::st0_f32(g.call(t(g, 12), 0, &[actor]).st0);
    g.write::<f32>(w(0x24a), h);
    g.write::<u32>(w(0x256), w(0x25a));
    let y2 = camera_yaw(g, img, f, [0x244, 0x250, 0x26d, 0x27c, 0x286, 0x290, 0x29d]);
    let lifted = sub(g.read::<f32>(w(0x2a5)), g.read::<f32>(w(0x2ad)));
    let qx = g.read::<u32>(q);
    let battle = g.read::<u32>(w(0x2ba)) != 0;
    g.write::<u16>(w(0x2c1), 0);
    g.write::<u32>(w(0x2c6), 0);
    g.write::<f32>(w(0x2d1), y2);
    let qz = g.read::<u32>(q + 8);
    g.write::<u32>(w(0x2db), kind);
    let k = g.read::<u32>(w(0x2e0));
    g.write::<u32>(w(0x2e8), qz);
    g.write::<u32>(w(0x2f0), qz);
    let pw = g.read::<u32>(w(0x2f8));
    g.write::<u32>(w(0x2fd), k);
    g.write::<u32>(w(0x30b), qx);
    g.write::<f32>(w(0x313), lifted);
    global_stores(g, img, f, 0x317, 0x371, 0);
    g.write::<u32>(w(0x375), qx);
    g.write::<f32>(w(0x37d), lifted);
    g.write::<u32>(w(0x385), pw);
    g.write::<u32>(w(0x38b), owner);
    global_stores(g, img, f, 0x38f, 0x3a3, 0);
    let model = g.call(t(g, if battle { 413 } else { 215 }), 0, &[k]).eax;
    let battle = g.read::<u32>(w(0x3ba)) != 0;
    g.write::<u32>(w(0x3c5), model);
    g.write::<u32>(w(0x3cb), u32::MAX);
    g.write::<u32>(w(0x3d5), 0);
    g.write::<u32>(w(0x3df), 1);
    g.write::<u16>(w(if battle { 0x3f2 } else { 0x3e9 }), u16::from(battle));
    // the two task nodes
    let n1 = ring_take(g, img, f, (0x3f9, 0x413, 0x438, None, 0x45d, 0x468));
    let had = g.read::<u32>(w(0x46e));
    g.write::<u32>(n1.wrapping_add(4), 0);
    if had == 0 {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x493), n1);
    } else {
        let last = g.read::<u32>(w(0x47e));
        g.write::<u32>(n1, last);
        g.write::<u32>(last.wrapping_add(4), n1);
    }
    g.write::<u32>(n1.wrapping_add(8), n1);
    let n2 = ring_take(g, img, f, (0x3f9, 0x413, 0x4bc, Some(0x4c4), 0x4f5, 0x500));
    g.write::<u32>(n2.wrapping_add(4), 0);
    if had == 0 && n1 == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x51e), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1.wrapping_add(4), n2);
    }
    g.write::<u32>(n2.wrapping_add(8), n2);
    g.write::<u32>(w(0x52e), n2);
    Ret::eax(0)
}

/// [`summon_actor_pair_start`] (the `rel32`s: the part set-up, the
/// object starts, the mean position, `__security_check_cookie`).
pub const SUMMON_ACTOR_PAIR_START: Sig = Sig {
    len: 1342,
    hash: 0x326d_0b69_fbb9_e426,
    calls: &[
        (0x46, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x73, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x1ea, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x208, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x536, Callee::Any),
    ],
    imports: &[],
};

/// The summon's start task
/// ([`summon_start_task`](crate::s8_misc::summon_start_task)) with
/// [`summon_actor_pair_start`].
///
/// `magic_0241` 0x10001210 (118 bytes; 5 DLLs).
pub fn summon_start_task_0241<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::s8_misc::summon_start_task(g, img, fr, summon_actor_pair_start)
}

/// [`summon_start_task_0241`].
pub const SUMMON_START_TASK_0241: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&SUMMON_ACTOR_PAIR_START))],
    imports: &[],
};

/// An object carried to its place and then launched (cdecl `(O)`; a /GS
/// frame aligned to 8, `S` = `(ebp & !7) - 0xd0` after the saved
/// registers: the yaw at `S+8`, the pitch (f64) at `S+0x18`, the point
/// `P` at `S+0x28`, the turned point `W` at `S+0x38`, the matrices `M`,
/// `M2` at `S+0x48`, `S+0x88`; `X = O[0x14]`, `B = *G_B`): the count
/// `X[0x50c]` goes down to 0; with `u = count / X[0x510]` and `v = K -
/// u`, `P` = the blend of `X+0x260..` and `X+0x270..` (`w` 1.0, each
/// coordinate in the original's operand order) and the part placed there
/// ([`part_matrix`](crate::sa_misc::part_matrix), or the weapon's matrix
/// in `magic_0555`: [`swing_matrix`](crate::s5_misc::swing_matrix);
/// angles `X+0x280`, `B` on the stack). Once the count is 0: the child started
/// ([`start_child_c`](crate::s4_fam446::start_child_c)); `X+0x260 = P`,
/// `X+0x2a0` = 0 (`w` 1.0); the launch `P = (r1, -(r2 + K2), -(z K6))`
/// with `r1`, `r2`, `r3` = slot 314 `(1.0)`, `(1.0)`, `(0.2)` and
/// `z = r3 + K3`, or `K3` itself when `K5 < z < K4`; `M` = slot 235 turned by
/// slots 328/329 `(M, M, X.280 / X.284)`, `W` = slot 229 `(W, M, P)`,
/// slot 248 `(V, V, W)` (`V = X+0x2a0`, `w` 1.0); `P`'s aim: the pitch
/// `atan2(P.y, |P.xz|)` and the yaw `KA - atan2(P.z, P.x)` brought back
/// once by `KT` (`_libm_sse2_sqrt_precise`, `_CIatan2`);
/// `P = (0, 0, -0.7, 1)`, `U = X+0x2b0` = 0 (`w` 1.0), `M2` = slot 235 turned by the
/// pitch and the yaw, `W` = slot 229 `(W, M2, P)`, slot 248 `(U, U, W)`;
/// `X+0x270 = V`, `X+0x27c = |V|`; the spins `X+0x290` = `(slot 314
/// (0.1) + K4) KH`, `X+0x294` = `(slot 404 (0.1) + K4) KL`, `X+0x298` =
/// 0; the count 40. With `B[0xd748]` 1 the trail record `B+0x2f90` is set up and
/// gets a task node ([`take`](crate::sa_pool::take), listed last). At the
/// end `O[8]`'s `+0x10` = the next step `CB`. Returns `O[8]` (or the
/// part's `eax` while the count runs).
///
/// `magic_0544` 0x10003470 (1,571 bytes; 6 DLLs) and `magic_0555`
/// 0x10003470 (2 DLLs).
pub fn launch_after_carry<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, s: u32| slot_target(g, w(0x103), s);
    let st0 = |o: Out| crate::m0018::st0_f32(o.st0);
    let obj = fr.arg(g, 0);
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xd0);
    let (yaw_at, pitch_at, p, wv, m, m2) =
        (s + 8, s + 0x18, s + 0x28, s + 0x38, s + 0x48, s + 0x88);
    let x = g.read::<u32>(obj.wrapping_add(0x14));
    let b = g.read::<u32>(w(0x1f));
    let left = g.read::<i32>(x + 0x50c);
    if left > 0 {
        g.write::<i32>(x + 0x50c, left - 1);
    }
    let r = |g: &G, o: u32| g.read::<f32>(x + o);
    let u = div(cvt(g.read::<u32>(x + 0x50c)), cvt(g.read::<u32>(x + 0x510)));
    let v = sub(k(g, 0x5b), u);
    let px = add(mul(v, r(g, 0x270)), mul(r(g, 0x260), u));
    let py = add(mul(r(g, 0x264), u), mul(v, r(g, 0x274)));
    let pz = add(mul(r(g, 0x268), u), mul(v, r(g, 0x278)));
    for (i, c) in (0u32..).zip([px, py, pz, 1.0]) {
        g.write::<f32>(p + 4 * i, c);
    }
    // the part's matrix (`magic_0544`) or the weapon's (`magic_0555`),
    // `B` on the stack
    g.write::<u32>(s.wrapping_sub(4), b);
    let place = img.rel32_target(f + 0xdb);
    let eax = crate::s5_misc::swing_matrix(g, img, place, s.wrapping_sub(8), x + 0x280, p);
    if g.read::<i32>(x + 0x50c) > 0 {
        return Ret::eax(eax);
    }
    let child = (img.rel32_target(f + 0xf8), s, x, x + 0x280);
    sub_call(g, crate::s4_fam446::start_child_c, img, child, &[]);
    for i in 0..4 {
        let c = g.read::<u32>(p + 4 * i);
        g.write::<u32>(x + 0x260 + 4 * i, c);
    }
    for (i, c) in (0u32..).zip([0, 0, 0, 0x3f80_0000]) {
        g.write::<u32>(x + 0x2a0 + 4 * i, c);
    }
    // the launch speed
    let r1 = st0(g.call(t(g, 314), 0, &[0x3f80_0000]));
    g.write::<f32>(p, r1);
    let r2 = st0(g.call(t(g, 314), 0, &[0x3f80_0000]));
    g.write::<f32>(p + 4, -add(r2, k(g, 0x197)));
    let r3 = st0(g.call(t(g, 314), 0, &[0x3e4c_cccd]));
    let k3 = k(g, 0x1ca);
    let z = add(r3, k3);
    let z = if k(g, 0x1d2) > z && z > k(g, 0x1e5) {
        k3
    } else {
        z
    };
    g.write::<f32>(p + 8, -mul(z, k(g, 0x1f2)));
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    g.call(t(g, 235), 0, &[m]);
    g.call(t(g, 328), 0, &[m, m, r(g, 0x280).to_bits()]);
    g.call(t(g, 329), 0, &[m, m, r(g, 0x284).to_bits()]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    g.call(t(g, 229), 0, &[wv, m, p]);
    let vel = x + 0x2a0;
    g.call(t(g, 248), 0, &[vel, vel, wv]);
    g.write::<u32>(vel + 0xc, 0x3f80_0000);
    // the launch's aim from the origin
    let (dx, dy, dz) = (
        sub(g.read::<f32>(p), 0.0),
        sub(g.read::<f32>(p + 4), 0.0),
        sub(g.read::<f32>(p + 8), 0.0),
    );
    let h = f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt();
    let pitch = g.atan2(f64::from(dy), h);
    g.write::<f64>(pitch_at, pitch);
    #[allow(clippy::cast_possible_truncation)] // `cvtpd2ps`
    let yaw = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let y = sub(k(g, 0x34c), yaw);
    g.write::<f32>(yaw_at, y);
    if y > k(g, 0x357) {
        g.write::<f32>(yaw_at, sub(y, k(g, 0x367)));
    } else if k(g, 0x371) > y {
        g.write::<f32>(yaw_at, add(y, k(g, 0x37e)));
    }
    for (i, c) in (0u32..).zip([0, 0, 0xbf33_3333, 0x3f80_0000]) {
        g.write::<u32>(p + 4 * i, c);
    }
    let up = x + 0x2b0;
    for (i, c) in (0u32..).zip([0, 0, 0, 0x3f80_0000]) {
        g.write::<u32>(up + 4 * i, c);
    }
    g.call(t(g, 235), 0, &[m2]);
    #[allow(clippy::cast_possible_truncation)] // `cvtpd2ps`
    let pitch = g.read::<f64>(pitch_at) as f32;
    g.call(t(g, 328), 0, &[m2, m2, pitch.to_bits()]);
    let yaw = g.read::<u32>(yaw_at);
    g.call(t(g, 329), 0, &[m2, m2, yaw]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    g.call(t(g, 229), 0, &[wv, m2, p]);
    g.call(t(g, 248), 0, &[up, up, wv]);
    g.write::<u32>(up + 0xc, 0x3f80_0000);
    for i in 0..4 {
        let c = g.read::<u32>(vel + 4 * i);
        g.write::<u32>(x + 0x270 + 4 * i, c);
    }
    let (vx, vy, vz) = (r(g, 0x2a0), r(g, 0x2a4), r(g, 0x2a8));
    let sq = add(add(mul(vx, vx), mul(vy, vy)), mul(vz, vz));
    #[allow(clippy::cast_possible_truncation)] // `cvtsd2ss`
    let speed = f64::from(sq).sqrt() as f32;
    g.write::<f32>(x + 0x27c, speed);
    let r4 = st0(g.call(t(g, 314), 0, &[0x3dcc_cccd]));
    g.write::<f32>(x + 0x290, mul(add(r4, k(g, 0x50e)), k(g, 0x516)));
    let r5 = st0(g.call(t(g, 404), 0, &[0x3dcc_cccd]));
    let spin = add(r5, k(g, 0x53d));
    g.write::<u32>(x + 0x298, 0);
    g.write::<f32>(x + 0x294, mul(spin, k(g, 0x556)));
    g.write::<u32>(x + 0x50c, 0x28);
    if g.read::<u32>(b.wrapping_add(w(0x56e))) == 1 {
        let rec = b.wrapping_add(w(0x57b));
        for (o, c) in [(0x80u32, 0x8080_8080u32), (0x84, 0x0080_8080), (0x88, 0)] {
            g.write::<u32>(rec + o, c);
        }
        g.write::<u16>(rec + 0x98, 0x48);
        for (o, c) in [(0x90u32, 0x14u32), (0x8c, 0x14), (0x94, 0)] {
            g.write::<u32>(rec + o, c);
        }
        let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x5ca), w(0x580));
        g.write::<u32>(node.wrapping_add(0x14), rec);
        g.write::<u32>(node.wrapping_add(4), 0);
        if g.read::<u32>(w(0x5d0)) == 0 {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(0x5f5), node);
        } else {
            let last = g.read::<u32>(w(0x5e3));
            g.write::<u32>(node, last);
            g.write::<u32>(last.wrapping_add(4), node);
        }
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x5fd), node);
    }
    let next = g.read::<u32>(obj.wrapping_add(8));
    g.write::<u32>(next.wrapping_add(0x10), w(0x60b));
    Ret::eax(next)
}

/// [`launch_after_carry`] in `magic_0544` (the `rel32`s: the part's
/// placing, the child start, the square roots, the arcs, the pool take
/// and `__security_check_cookie`).
pub const LAUNCH_AFTER_CARRY_0544: Sig = Sig {
    len: 1571,
    hash: 0x1b0f_7e50_f94f_16cf,
    calls: &[
        (0xdb, Callee::Sig(&crate::sa_misc::PART_MATRIX)),
        (0xf8, Callee::Sig(&crate::s4_fam446::START_CHILD_C)),
        (0x2f4, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x307, Callee::Import("_CIatan2")),
        (0x336, Callee::Import("_CIatan2")),
        (0x4d2, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x5ca, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x61b, Callee::Any),
    ],
    imports: &[],
};

/// [`launch_after_carry`] in `magic_0555` (the weapon's matrix).
pub const LAUNCH_AFTER_CARRY_0555: Sig = Sig {
    len: 1571,
    hash: 0x1b0f_7e50_f94f_16cf,
    calls: &[
        (0xdb, Callee::Sig(&crate::s5_misc::WEAPON_MATRIX)),
        (0xf8, Callee::Sig(&crate::s4_fam446::START_CHILD_C)),
        (0x2f4, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x307, Callee::Import("_CIatan2")),
        (0x336, Callee::Import("_CIatan2")),
        (0x4d2, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x5ca, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x61b, Callee::Any),
    ],
    imports: &[],
};

/// The camera matrix aimed (cdecl, no arguments; an `ebp` frame: the
/// matrix `M` at `ebp-0x84`, the scale `M3` at `ebp-0x104`; `C = G_M`
/// the DLL's camera matrix): `C`, `M`, `M3` = slot 235; the two id lists'
/// answers ([`id_list`](crate::s4_small::id_list) of `*G_1`, `*G_2`).
/// Both 1: the direction from slot 325 `(T, G_T)` (with `T.y` = `*G_Y`)
/// to `G_V`; both 0: `M` is the look matrix
/// ([`look_matrix`](crate::s7_misc::look_matrix)) from `T` to `G_V`;
/// else from the second list's point (slot 217) to the first's. For a
/// direction `d`: `Z` = slot 232 `(d)`, `X` = slot 266 `(U, Z)` with `U
/// = (0, 1, 0, 1)`, the third row slot 266 `(Z, X)`, both normalised
/// (slot 232); `M`'s rows `(X, that, Z)` with `w` 0 and the last row `(0,
/// 0, 0, 1)`. Then `C` = slot 231 `(C, M, C)`, slot 395 `(C, C, G_V)`,
/// `M3`'s diagonal `G_S[0..3] * K`, `C` = slot 231 `(C, C, M3)`, whose
/// `eax` it returns.
///
/// `magic_0188` 0x10001940 (1,054 bytes; 2 DLLs).
pub fn camera_aimed<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::mul;
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x14), s);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |d: u32| ebp.wrapping_sub(d);
    let (m, m3, tp, dir, c1, c2) = (at(0x84), at(0x104), at(0x94), at(0xa4), at(0xb4), at(0xc4));
    let (x, r2, r3, up) = (at(0x14), at(0x24), at(0x34), at(0x44));
    let cam = w(0x20);
    g.call(t(g, 235), 0, &[cam]);
    g.call(t(g, 235), 0, &[m]);
    g.call(t(g, 235), 0, &[m3]);
    let esp = ebp.wrapping_sub(0x108);
    let ids = |g: &mut G, img: &Image, op_list: u32, call: u32| {
        let list = g.read::<u32>(w(op_list));
        sub_call(
            g,
            crate::s4_small::id_list,
            img,
            (img.rel32_target(f + call), esp, list, 0),
            &[],
        )
        .eax
    };
    let first = ids(g, img, 0x50, 0x58);
    let second = ids(g, img, 0x5e, 0x65);
    let set_up = |g: &mut G| {
        for (i, c) in (0u32..).zip([0, 0x3f80_0000, 0, 0x3f80_0000]) {
            g.write::<u32>(up + 4 * i, c);
        }
    };
    // the direction's rows: (`Z` slot, cross slot, the order of `M`'s rows)
    let rows = if first == 1 && second == 1 {
        g.call(t(g, 325), 0, &[tp, w(0x81)]);
        let y = g.read::<u32>(w(0x97));
        g.write::<u32>(tp + 4, y);
        set_up(g);
        g.call(t(g, 234), 0, &[dir, w(0xa9), tp]);
        Some((r2, r3))
    } else if first == 0 && second == 0 {
        g.call(t(g, 325), 0, &[tp, w(0x1bc)]);
        let y = g.read::<u32>(w(0x1d2));
        g.write::<u32>(tp + 4, y);
        let look = Frame {
            at: img.rel32_target(f + 0x1f0),
            esp: ebp.wrapping_sub(0x118),
            eax: 0,
            ecx: m,
            edx: tp,
            xmm: [0; 4],
        };
        g.write::<u32>(ebp.wrapping_sub(0x114), w(0x1d7));
        crate::s7_misc::look_matrix(g, img, &look);
        None
    } else {
        let l2 = g.read::<u32>(w(0x20a));
        g.call(t(g, 217), 0, &[l2, c2]);
        let l1 = g.read::<u32>(w(0x224));
        g.call(t(g, 217), 0, &[l1, c1]);
        set_up(g);
        g.call(t(g, 234), 0, &[dir, c1, c2]);
        Some((r3, r2))
    };
    if let Some((z, y)) = rows {
        g.call(t(g, 232), 0, &[z, dir]);
        g.call(t(g, 266), 0, &[x, up, z]);
        g.call(t(g, 266), 0, &[y, z, x]);
        g.call(t(g, 232), 0, &[x, x]);
        g.call(t(g, 232), 0, &[y, y]);
        for (row, src) in [(0u32, x), (1, y), (2, z)] {
            for i in 0..3 {
                let v = g.read::<u32>(src + 4 * i);
                g.write::<u32>(m + 0x10 * row + 4 * i, v);
            }
        }
        for (o, c) in [
            (0x3cu32, 0x3f80_0000u32),
            (0x38, 0),
            (0x34, 0),
            (0x30, 0),
            (0x2c, 0),
            (0x1c, 0),
            (0xc, 0),
        ] {
            g.write::<u32>(m + o, c);
        }
    }
    g.call(t(g, 231), 0, &[cam, m, cam]);
    g.call(t(g, 395), 0, &[cam, cam, w(0x394)]);
    let k = g.read::<f32>(w(0x3ae));
    for (i, o) in [(0u32, 0x3b6u32), (5, 0x3d6), (10, 0x3f4)] {
        let s = g.read::<f32>(w(o));
        g.write::<f32>(m3 + 4 * i, mul(s, k));
    }
    Ret::eax(g.call(t(g, 231), 0, &[cam, cam, m3]).eax)
}

/// [`camera_aimed`] (the `rel32`s: the id lists, the look matrix,
/// `__security_check_cookie`).
pub const CAMERA_AIMED: Sig = Sig {
    len: 1054,
    hash: 0xd649_cd1e_a425_7567,
    calls: &[
        (0x58, Callee::Sig(&crate::s4_small::ID_LIST)),
        (0x65, Callee::Sig(&crate::s4_small::ID_LIST)),
        (0x1f0, Callee::Sig(&crate::s7_misc::LOOK_MATRIX)),
        (0x416, Callee::Any),
    ],
    imports: &[],
};

/// The camera's frame (cdecl, no arguments): the scale `G_S` = `(0.75,
/// 0.75, 0.75, 1)` (the immediates), the camera aimed ([`camera_aimed`]),
/// slot 211 `(*G_W, G_M)`, the motion script stepped
/// ([`motion_script_step`](crate::s6_misc::motion_script_step)). Returns
/// whether `*G_D` is 0.
///
/// `magic_0188` 0x10002800 (89 bytes; 2 DLLs).
pub fn camera_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    global_stores(g, img, f, 0, 0x28, 0);
    let aim = (img.rel32_target(f + 0x29), fr.esp, fr.ecx, fr.edx);
    sub_call(g, camera_aimed, img, aim, &[]);
    let wv = g.read::<u32>(w(0x39));
    g.call(slot_target(g, w(0x2e), 211), 0, &[wv, w(0x33)]);
    let step = (img.rel32_target(f + 0x49), fr.esp, 0, 0);
    sub_call(g, crate::s6_misc::motion_script_step, img, step, &[]);
    Ret::eax(u32::from(g.read::<u32>(w(0x51)) == 0))
}

/// [`camera_frame`] (the `rel32`s: [`camera_aimed`] and the motion
/// script step).
pub const CAMERA_FRAME: Sig = Sig {
    len: 89,
    hash: 0xb10c_5c13_4aa8_a5d3,
    calls: &[
        (0x29, Callee::Sig(&CAMERA_AIMED)),
        (0x49, Callee::Sig(&crate::s6_misc::MOTION_SCRIPT_STEP)),
    ],
    imports: &[],
};

/// [`trail_draw`](crate::o6_fam_pairs6::trail_draw)'s copy without `M4`
/// (`magic_0565` 0x10006ea0, 1,392 bytes; 5 DLLs).
pub const TRAIL_DRAW_0565: Sig = Sig {
    len: 1392,
    hash: 0xce7c_8d3c_b669_efb7,
    calls: &[
        (0xdd, Callee::Sig(&crate::sb_common::ANIM_STEP)),
        (0x1b1, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x531, Callee::Sig(&crate::sb_misc::DRAW_FRAME)),
        (0x568, Callee::Any),
    ],
    imports: &[],
};

/// [`trail_head`] in `magic_0565` (its sparks and trail draw).
pub const TRAIL_HEAD_0565: Sig = Sig {
    len: 395,
    hash: 0x7ccd_415a_28c9_5e5d,
    calls: &[
        (0xbe, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xcb, Callee::Any),
        (0xea, Callee::Sig(&crate::s7_misc::FLAGGED_TWO_SPARKS)),
        (0xf4, Callee::Sig(&crate::s7_misc::TWO_SPARKS)),
        (0xfe, Callee::Sig(&crate::s4_fam446::SPARKS_B)),
        (0x173, Callee::Sig(&TRAIL_DRAW_0565)),
        (0x183, Callee::Any),
    ],
    imports: &[],
};

/// [`trail_task`](crate::o6_fam_pairs6::trail_task) in `magic_0565`
/// (its sparks, a light placed instead of the part, the child with its
/// sound, the object off its list before the mode is asked).
pub const TRAIL_TASK_0565: Sig = Sig {
    len: 641,
    hash: 0xd618_c58b_219b_340c,
    calls: &[
        (0xf9, Callee::Sig(&crate::s7_misc::FLAGGED_TWO_SPARKS)),
        (0x103, Callee::Sig(&crate::s7_misc::TWO_SPARKS)),
        (0x10d, Callee::Sig(&crate::s4_fam446::SPARKS_B)),
        (
            0x120,
            Callee::Sig(&crate::o6_fam_pairs5::LIGHT_SETUP_0644_B390),
        ),
        (0x174, Callee::Sig(&crate::s4_fam446::START_CHILD_SOUND_A)),
        (0x183, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x24b, Callee::Sig(&TRAIL_DRAW_0565)),
        (0x25b, Callee::Any),
        (0x26c, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x279, Callee::Any),
    ],
    imports: &[],
};

/// [`trail_draw`](crate::o6_fam_pairs6::trail_draw) in `magic_0644`
/// (0x1000acd0, 1497 bytes; 5 DLLs: `0644`, `0692`, `0694`, `0696`,
/// `0698`): its own frame layout, `M4` and `M3` also through slot 231
/// with `M1`, the loop's last `eax` returned (the `rel32`s: the
/// animation step, the square root, the frame draw and the cookie check).
pub const TRAIL_DRAW_0644: Sig = Sig {
    len: 1497,
    hash: 0x35c9_a9cd_b0db_5aa6,
    calls: &[
        (0xd9, Callee::Sig(&crate::sb_common::ANIM_STEP)),
        (0x1c2, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x59f, Callee::Sig(&crate::sb_misc::DRAW_FRAME)),
        (0x5d1, Callee::Any),
    ],
    imports: &[],
};

/// [`trail_head`] in `magic_0644` (0x10007a40, 395 bytes; 5 DLLs: its
/// block words, sparks and trail draw).
pub const TRAIL_HEAD_0644: Sig = Sig {
    len: 395,
    hash: 0x2999_09f2_a574_5f38,
    calls: &[
        (0xbe, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xcb, Callee::Any),
        (0xea, Callee::Sig(&crate::s7_misc::SPARK_AT_POINT)),
        (0xf4, Callee::Sig(&crate::s7_misc::TWO_SPARKS_B)),
        (0xfe, Callee::Sig(&crate::s4_fam446::SPARKS_B)),
        (0x173, Callee::Sig(&TRAIL_DRAW_0644)),
        (0x183, Callee::Any),
    ],
    imports: &[],
};

/// [`trail_task`](crate::o6_fam_pairs6::trail_task) in `magic_0644`
/// (0x10007e60, 641 bytes; 5 DLLs): `magic_0565`'s with the block's
/// words 0x130 lower, its own sparks and trail draw.
pub const TRAIL_TASK_0644: Sig = Sig {
    len: 641,
    hash: 0x7230_e812_ce74_e02e,
    calls: &[
        (0xf9, Callee::Sig(&crate::s7_misc::SPARK_AT_POINT)),
        (0x103, Callee::Sig(&crate::s7_misc::TWO_SPARKS_B)),
        (0x10d, Callee::Sig(&crate::s4_fam446::SPARKS_B)),
        (
            0x120,
            Callee::Sig(&crate::o6_fam_pairs5::LIGHT_SETUP_0644_B390),
        ),
        (0x174, Callee::Sig(&crate::s4_fam446::START_CHILD_SOUND_A)),
        (0x183, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x24b, Callee::Sig(&TRAIL_DRAW_0644)),
        (0x25b, Callee::Any),
        (0x26c, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x279, Callee::Any),
    ],
    imports: &[],
};

/// A summon's six objects per actor started around it (cdecl, no
/// arguments; a /GS frame aligned to 8, `S` = `(ebp & !7) - 0x58` after
/// the saved registers: the actor's offset `D` at `S+0x24`, the origin
/// `P` at `S+0x34`, the turned point `W` at `S+0x44`): slot 719 `(X, 8,
/// 0)` into `*G_H` (two globals set first), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)), `P` = slot 262 `(*G_A,
/// P..)` (`w` 1.0). For each of the `*G_N` actors `i` (the battle number's
/// actor `a`): `D` = slot 262 `(a, ..)` less `P` (`x`, `z`), the program
/// index `0x54`, or `0x5c` when `sqrt(dz dz + dx dx)` is not above `K`
/// (`_libm_sse2_sqrt_precise`); then six objects `O = B + 0x720 i + 0x130
/// j` (their records `E = R + 0x138 i + 0x34 j`): started at `K2` with
/// that index ([`start_object_first`](crate::o6_fam_object::start_object_first)),
/// slot 262 `(a, O+0xd0, O+0xd4, O+0xd8)` and slot 268 `(a, 6, W)`, `O+0xd4
/// = W.y`, `O+0xdc = 1.0`, `O+0xc0 += K3`, `O+0xd0..0xd8` each moved by slot
/// 314 `(5.0)` (ST0), `O+0xd4+0x4c` = the actor's byte
/// ([`pick`](crate::m0074::pick)), the record cleared (`+0xc`, `+0x10`
/// immediates) into `O+0x110`, the index for the next one `+ 8 + rand() %
/// 4`. Then the task (`task_start`) with `CB`, the actor block (slot 413
/// or 215 `(*G_K)` by `*G_B`) filled inline and a task node taken and
/// listed ([`free_to_live`](crate::s7_misc::free_to_live)). Returns 0.
///
/// `magic_0247` 0x10001340 (1,110 bytes; 2 DLLs).
pub fn summon_ring_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x15), s);
    let st0 = |o: Out| crate::m0018::st0_f32(o.st0);
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x58);
    let (d, p, wv) = (s + 0x24, s + 0x34, s + 0x44);
    global_stores(g, img, f, 0x2b, 0x3f, 0);
    let mut st = Pushes::default();
    st.push_all(&[0, 8, w(0x27)]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let h = st.call(g, t(g, 719), fr.ecx).eax;
    g.write::<u32>(w(0x45), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x4a));
    let a0 = g.read::<u32>(w(0x64));
    g.call(t(g, 262), 0, &[a0, p, p + 4, p + 8]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let start = img.rel32_target(f + 0x166);
    let pick = img.rel32_target(f + 0x265);
    let mut index = 0u32;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0x7f)) {
        let actor = entry_actor(g, img, f, (0x96, 0x9d, 0xa8), i);
        g.call(t(g, 262), 0, &[actor, d, d + 4, d + 8]);
        let dx = sub(g.read::<f32>(d), g.read::<f32>(p));
        let dz = sub(g.read::<f32>(d + 8), g.read::<f32>(p + 8));
        g.write::<u32>(d + 0xc, 0x3f80_0000);
        g.write::<f32>(d + 8, dz);
        g.write::<f32>(d, dx);
        #[allow(clippy::cast_possible_truncation)] // `cvtsd2ss`
        let dist = f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt() as f32;
        let near = !(dist > g.read::<f32>(w(0x115)));
        let mut rec = i.wrapping_mul(0x138).wrapping_add(w(0x12d));
        let mut obj = i.wrapping_mul(0x720).wrapping_add(w(0x13a));
        index = if near { 0x5c } else { 0x54 };
        for _ in 0..6 {
            // the object start (`edx` the object, `xmm3` the scale, the
            // index twice on the stack, two words below them)
            let o = obj.wrapping_sub(0xd4);
            let call = s.wrapping_sub(0x14);
            g.write::<u32>(call + 4, index);
            g.write::<u32>(call + 8, index);
            let scale = g.read::<u32>(w(0x156));
            let fr = Frame {
                at: start,
                esp: call,
                eax: 0,
                ecx: 0,
                edx: o,
                xmm: [0, 0, 0, u128::from(scale)],
            };
            crate::o6_fam_object::start_object_first(g, img, start, &fr);
            let actor = entry_actor(g, img, f, (0x173, 0x17a, 0x185), i);
            let at = obj.wrapping_sub(4);
            g.call(t(g, 262), 0, &[actor, at, obj, obj + 4]);
            g.call(t(g, 268), 0, &[actor, 6, wv]);
            let y = g.read::<u32>(wv + 4);
            g.write::<u32>(obj, y);
            g.write::<u32>(obj + 8, 0x3f80_0000);
            let v = add(
                g.read::<f32>(obj.wrapping_sub(0x14)),
                g.read::<f32>(w(0x1dd)),
            );
            g.write::<f32>(obj.wrapping_sub(0x14), v);
            for c in [at, obj, obj + 4] {
                let r = st0(g.call(t(g, 314), 0, &[0x40a0_0000]));
                let v = add(g.read::<f32>(c), r);
                g.write::<f32>(c, v);
            }
            let n = crate::m0074::pick(g, img, pick, i);
            g.write::<u32>(obj + 0x4c, n);
            for o in [4u32, 0, 0xffff_fffc, 8] {
                g.write::<u32>(rec.wrapping_add(o), 0);
            }
            g.write::<u16>(rec.wrapping_add(0x1c), 0);
            g.write::<u32>(rec.wrapping_add(0xc), w(0x297));
            g.write::<u32>(rec.wrapping_add(0x10), w(0x29e));
            g.write::<u32>(rec.wrapping_add(0x14), 0);
            g.write::<u32>(rec.wrapping_add(0x18), 0);
            g.write::<u32>(obj + 0x3c, rec.wrapping_sub(0x14));
            let r = g.rand().cast_signed() % 4;
            index = index.wrapping_add(8).wrapping_add(r.cast_unsigned());
            obj = obj.wrapping_add(0x130);
            rec = rec.wrapping_add(0x34);
        }
        i += 1;
    }
    g.write::<u32>(w(0x302), 0);
    let task = (img.rel32_target(f + 0x30b), s.wrapping_sub(4), 0, 0);
    sub_call(
        g,
        |g, img, fr| Ret::eax(crate::s3_misc::task_start(g, img, fr)),
        img,
        task,
        &[w(0x2fc)],
    );
    let _ = index;
    // the actor block
    let k = g.read::<u32>(w(0x310));
    let battle = g.read::<u32>(w(0x319)) != 0;
    g.write::<u32>(w(0x31f), k);
    global_stores(g, img, f, 0x329, 0x347, 0);
    let model = g.call(t(g, if battle { 413 } else { 215 }), 0, &[k]).eax;
    let battle = g.read::<u32>(w(0x35e)) != 0;
    g.write::<u32>(w(0x369), model);
    g.write::<u32>(w(0x36f), u32::MAX);
    g.write::<u32>(w(0x379), 0);
    g.write::<u32>(w(0x383), 1);
    g.write::<u16>(w(if battle { 0x396 } else { 0x38d }), u16::from(battle));
    crate::s7_misc::free_to_live(
        g,
        img,
        f,
        [
            0x39d, 0x3ac, 0x3c9, 0x3d1, 0x402, 0x40d, 0x413, 0x422, 0x447, 0x435, 0x447,
        ],
    );
    Ret::eax(0)
}

/// [`summon_ring_start`] (the `rel32`s: the part set-up, the square root,
/// the object start, the pick, the task start, `__security_check_cookie`;
/// `rand` through the IAT).
pub const SUMMON_RING_START: Sig = Sig {
    len: 1110,
    hash: 0x3b9c_85db_b8bb_2a18,
    calls: &[
        (0x4a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x108, Callee::Import("_libm_sse2_sqrt_precise")),
        (
            0x166,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0247_25E0),
        ),
        (0x265, Callee::Sig(&crate::m0074::PICK)),
        (0x30b, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0x44e, Callee::Any),
    ],
    imports: &[(0x2b5, "rand")],
};

/// The summon's start task
/// ([`summon_start_task`](crate::s8_misc::summon_start_task)) with
/// [`summon_ring_start`].
///
/// `magic_0247` 0x10001210 (118 bytes; 2 DLLs).
pub fn summon_start_task_0247<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::s8_misc::summon_start_task(g, img, fr, summon_ring_start)
}

/// [`summon_start_task_0247`].
pub const SUMMON_START_TASK_0247: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&SUMMON_RING_START))],
    imports: &[],
};

/// A node taken off the DLL's free ring as the original inlines it: `n`
/// unlinked (its `+0` the previous, `+4` the next), the head `*G_H` =
/// `n+4` while more than one was free (else 0), the count less one; the
/// node cleared (`+0`, `+4`, `+0xc`, `+0x14`), `n+8 = n`, its callback
/// `cb` at `+0x10`, `byte n+0x18 = 0`.
fn ring_node<G: Game>(g: &mut G, n: u32, free: u16, (head, count, cb): (u32, u32, u32)) {
    let (prev, next) = (g.read::<u32>(n), g.read::<u32>(n + 4));
    g.write::<u32>(prev.wrapping_add(4), next);
    let (next, prev) = (g.read::<u32>(n + 4), g.read::<u32>(n));
    g.write::<u32>(next, prev);
    let h = if free > 1 { g.read::<u32>(n + 4) } else { 0 };
    g.write::<u32>(head, h);
    for o in [0u32, 4, 0xc, 0x14] {
        g.write::<u32>(n + o, 0);
    }
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 0x10, cb);
    g.write::<u8>(n + 0x18, 0);
    g.write::<u16>(count, free.wrapping_sub(1));
}

/// A summon's five task nodes started (cdecl, no arguments): slot 719
/// `(X, 8, 0)` into `*G_H` (three globals set first; two of its words
/// stay on the stack), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)), `*G_R` = slot 291
/// `(*G_A, *G_B, &a, &b, &c)`, three globals set. Then the nodes, each
/// taken off the free ring (`*G_F`, `u16 *G_C`; with none free the
/// original writes through address 0, as this does):
/// - 1: callback `CB1`, `+0x14` = `D1`, appended to the task list (`*G_L`
///   last; `*G_T` when it is empty, `*G_T` not moved on); then the first
///   block's constants;
/// - 2: callback `CB2`, byte `+0x18` 1, `+0x14` = `D2`, linked after node
///   1 (or the list's first when it was empty); the second block's
///   constants;
/// - 3: the same with `D3`, linked after node 2 and made the last
///   (`*G_L`); slot 271 `(4)`;
/// - 4: taken and listed ([`free_to_live`](crate::s7_misc::free_to_live)),
///   byte `+0x18` 1; the records set up
///   ([`records_init`](crate::sa_misc::records_init) of the block `B`);
/// - 5: the same, byte `+0x18` 2.
///
/// Returns 0.
///
/// `magic_0132` 0x100013e0 (1,304 bytes; 4 DLLs).
pub fn summon_nodes_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x5), s);
    global_stores(g, img, f, 0x1b, 0x39, 0);
    let h = g.call(t(g, 719), fr.ecx, &[w(0x17), 8, 0]).eax;
    g.write::<u32>(w(0x3f), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x44));
    let (a, b) = (g.read::<u32>(w(0x6a)), g.read::<u32>(w(0x64)));
    let r = g.call(t(g, 291), 0, &[a, b, w(0x5e), w(0x59), w(0x4e)]).eax;
    g.write::<u32>(w(0x7b), r);
    global_stores(g, img, f, 0x7f, 0x9d, 0);
    let (head, count, first, last) = (w(0xa9), w(0x73), w(0x10e), w(0x124));
    // node 1
    let mut free = g.read::<u16>(count);
    let n1 = if free == 0 {
        0
    } else {
        let n = g.read::<u32>(head);
        ring_node(g, n, free, (head, count, w(0xfd)));
        free -= 1;
        n
    };
    g.write::<u32>(n1.wrapping_add(0x14), w(0x116));
    g.write::<u32>(n1.wrapping_add(4), 0);
    if g.read::<u32>(first) == 0 {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x137), n1);
    } else {
        let l = g.read::<u32>(last);
        g.write::<u32>(n1, l);
        g.write::<u32>(l.wrapping_add(4), n1);
    }
    g.write::<u32>(n1.wrapping_add(8), n1);
    global_stores(g, img, f, 0x13e, 0x1c0, 0);
    // node 2 (the head as node 1 left it)
    let n2 = if free == 0 {
        0
    } else {
        let n = g.read::<u32>(head);
        ring_node(g, n, free, (head, count, w(0x21e)));
        free -= 1;
        n
    };
    let listed = g.read::<u32>(first) != 0;
    g.write::<u8>(n2.wrapping_add(0x18), img.word(f + 0x237) as u8);
    g.write::<u32>(n2.wrapping_add(0x14), w(0x23b));
    g.write::<u32>(n2.wrapping_add(4), 0);
    let mut p = n1;
    if listed {
        g.write::<u32>(n2, p);
        g.write::<u32>(p.wrapping_add(4), n2);
        p = g.read::<u32>(first);
    } else {
        p = n2;
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x25f), p);
    }
    g.write::<u32>(n2.wrapping_add(8), n2);
    global_stores(g, img, f, 0x266, 0x2f2, 0);
    // node 3
    let n3 = if free == 0 {
        0
    } else {
        let n = g.read::<u32>(head);
        ring_node(g, n, free, (head, count, w(0x34d)));
        n
    };
    g.write::<u8>(n3.wrapping_add(0x18), img.word(f + 0x35f) as u8);
    g.write::<u32>(n3.wrapping_add(0x14), w(0x363));
    g.write::<u32>(n3.wrapping_add(4), 0);
    if p == 0 {
        g.write::<u32>(n3, 0);
        g.write::<u32>(w(0x381), n3);
    } else {
        g.write::<u32>(n3, n2);
        g.write::<u32>(n2.wrapping_add(4), n3);
    }
    g.write::<u32>(n3.wrapping_add(8), n3);
    g.write::<u32>(w(0x397), n3);
    g.call(slot_target(g, w(0x386), 271), 0, &[4]);
    // nodes 4 and 5
    let n4 = crate::s7_misc::free_to_live(
        g,
        img,
        f,
        [
            0x3a0, 0x3b7, 0x3d4, 0x3dc, 0x40d, 0x418, 0x41e, 0x431, 0x452, 0x444, 0x452,
        ],
    );
    g.write::<u8>(n4.wrapping_add(0x18), img.word(f + 0x426) as u8);
    crate::sa_misc::records_init(g, img, img.rel32_target(f + 0x457), w(0x449));
    let n5 = crate::s7_misc::free_to_live(
        g,
        img,
        f,
        [
            0x45e, 0x46d, 0x48a, 0x492, 0x4be, 0x4c9, 0x4cf, 0x4e5, 0x4f3, 0x505, 0x50e,
        ],
    );
    g.write::<u8>(n5.wrapping_add(0x18), img.word(f + 0x4d9) as u8);
    Ret::eax(0)
}

/// [`summon_nodes_start`] (the `rel32`s: the part set-up, the records'
/// set-up).
pub const SUMMON_NODES_START: Sig = Sig {
    len: 1304,
    hash: 0x8e72_2275_c2fb_dfc6,
    calls: &[
        (0x44, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x457, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
    ],
    imports: &[],
};

/// The summon's start task
/// ([`summon_start_task`](crate::s8_misc::summon_start_task), the copy
/// with `MsEffectReady(*G_A, 1)`) with [`summon_nodes_start`].
///
/// `magic_0132` 0x100012b0 (119 bytes; 4 DLLs).
pub fn summon_start_task_0132<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::s8_misc::summon_start_task(g, img, fr, summon_nodes_start)
}

/// [`summon_start_task_0132`].
pub const SUMMON_START_TASK_0132: Sig = Sig {
    len: 119,
    hash: 0x2da6_1889_6875_2369,
    calls: &[(0x5c, Callee::Sig(&SUMMON_NODES_START))],
    imports: &[],
};

/// A summon's task nodes per target and four more (cdecl, no arguments;
/// `ebp` = `esp - 4` at the entry, the count `n` at `ebp-0xc`): slot 719
/// `(X, 8, 0)` into `*G_H` (three globals set first; two of its words
/// stay on the stack), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)), the list `L` = slot 617
/// `(*G_A, &n)`. For each of the `n` targets `i` (`c` from 1, up by 0x19):
/// the index `j` of `L[i]` among the `*G_M` bytes at `*G_T` (0 when it is
/// not there) and the record `R = R0 + 0x520 i` (`R-4 = j`, `R = 0xc`,
/// `R+4 = c`); a node taken off the free ring (callback `CB`, `+0x14` =
/// `R - 0x508`) and appended to the task list. Then `*G_C = c - 0x19`,
/// the actor block (slot 413 or 215 `(*G_K)` by `*G_B`, `+0` the first or
/// second word by `c - 0x19 > 0`) filled inline, the first block's
/// constants, a node (byte `+0x18` 1, `+0x14` = `D1`) appended (`*G_T`
/// not moved on), the second block's constants with `c + 0x14`, a node
/// (`D2`) linked after it and made the last, slot 271 `(4)`; two nodes
/// taken and listed ([`free_to_live`](crate::s7_misc::free_to_live)),
/// bytes `+0x18` 1 and 2, the records set up
/// ([`records_init`](crate::sa_misc::records_init) of the block `B`)
/// between them. With no free node the original writes through address
/// 0, as this does. Returns 0.
///
/// `magic_0347` 0x100035c0 (1,558 bytes; 2 DLLs).
pub fn summon_targets_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x7), s);
    global_stores(g, img, f, 0x1d, 0x3b, 0);
    let h = g.call(t(g, 719), fr.ecx, &[w(0x19), 8, 0]).eax;
    g.write::<u32>(w(0x41), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x46));
    let n_at = fr.esp.wrapping_sub(0x10);
    let a = g.read::<u32>(w(0x55));
    let picked = g.call(t(g, 617), 0, &[a, n_at]).eax;
    let (head, count, first, last) = (w(0x86), w(0xd1), w(0x151), w(0x169));
    let n = g.read::<i32>(n_at);
    let mut c = 1u32;
    let mut rec = w(0x8b);
    let mut i = 0u32;
    while i.cast_signed() < n {
        let m = g.read::<i32>(w(0x92));
        let mut j = 0u32;
        if m > 0 {
            let x = g.read::<u8>(picked.wrapping_add(i));
            let table = g.read::<u32>(w(0xb6));
            j = (0..m.cast_unsigned())
                .find(|&k| g.read::<u8>(table.wrapping_add(k)) == x)
                .unwrap_or(0);
        }
        let free = g.read::<u16>(count);
        g.write::<u32>(rec.wrapping_sub(4), j);
        g.write::<u32>(rec, w(0xde));
        g.write::<u32>(rec.wrapping_add(4), c);
        let node = if free == 0 {
            0
        } else {
            let node = g.read::<u32>(head);
            ring_node(g, node, free, (head, count, w(0x141)));
            node
        };
        let listed = g.read::<u32>(first) != 0;
        g.write::<u32>(node.wrapping_add(0x14), rec.wrapping_add(w(0x158)));
        g.write::<u32>(node.wrapping_add(4), 0);
        if listed {
            let l = g.read::<u32>(last);
            g.write::<u32>(node, l);
            g.write::<u32>(l.wrapping_add(4), node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(0x17c), node);
        }
        c = c.wrapping_add(img.word(f + 0x185) & 0xff);
        i += 1;
        g.write::<u32>(node.wrapping_add(8), node);
        rec = rec.wrapping_add(w(0x192));
        g.write::<u32>(w(0x198), node);
    }
    // the actor block
    let left = c.wrapping_sub(0x19);
    g.write::<u32>(w(0x1b6), left);
    let k = g.read::<u32>(w(0x1bb));
    let pick = if left.cast_signed() > 0 {
        w(0x1c0)
    } else {
        w(0x1c5)
    };
    let battle = g.read::<u32>(w(0x1ce)) != 0;
    g.write::<u32>(w(0x1d4), k);
    g.write::<u32>(w(0x1e0), pick);
    global_stores(g, img, f, 0x1e4, 0x1ee, 0);
    let model = g.call(t(g, if battle { 413 } else { 215 }), 0, &[k]).eax;
    let battle = g.read::<u32>(w(0x205)) != 0;
    g.write::<u32>(w(0x210), model);
    global_stores(g, img, f, 0x214, 0x228, 0);
    g.write::<u32>(w(0x22a), 1);
    g.write::<u16>(w(if battle { 0x23d } else { 0x234 }), u16::from(battle));
    // the two blocks' nodes
    let mut free = g.read::<u16>(count);
    global_stores(g, img, f, 0x248, 0x2c0, 0);
    let na = if free == 0 {
        0
    } else {
        let node = g.read::<u32>(head);
        ring_node(g, node, free, (head, count, w(0x31b)));
        free -= 1;
        node
    };
    let mut p = g.read::<u32>(first);
    g.write::<u8>(na.wrapping_add(0x18), img.word(f + 0x333) as u8);
    g.write::<u32>(na.wrapping_add(0x14), w(0x337));
    g.write::<u32>(na.wrapping_add(4), 0);
    if p == 0 {
        p = na;
        g.write::<u32>(na, 0);
        g.write::<u32>(w(0x35c), p);
    } else {
        let l = g.read::<u32>(last);
        g.write::<u32>(na, l);
        g.write::<u32>(l.wrapping_add(4), na);
    }
    g.write::<u32>(na.wrapping_add(8), na);
    global_stores(g, img, f, 0x369, 0x3cd, 0);
    g.write::<u32>(w(0x3ce), c.wrapping_add(img.word(f + 0x368) & 0xff));
    global_stores(g, img, f, 0x3d2, 0x3f0, 0);
    let nb = if free == 0 {
        0
    } else {
        let node = g.read::<u32>(head);
        ring_node(g, node, free, (head, count, w(0x44b)));
        node
    };
    g.write::<u8>(nb.wrapping_add(0x18), img.word(f + 0x45d) as u8);
    g.write::<u32>(nb.wrapping_add(0x14), w(0x461));
    g.write::<u32>(nb.wrapping_add(4), 0);
    if p == 0 {
        g.write::<u32>(nb, 0);
        g.write::<u32>(w(0x47f), nb);
    } else {
        g.write::<u32>(nb, na);
        g.write::<u32>(na.wrapping_add(4), nb);
    }
    g.write::<u32>(nb.wrapping_add(8), nb);
    g.write::<u32>(w(0x495), nb);
    g.call(slot_target(g, w(0x484), 271), 0, &[4]);
    // two more, the records between them
    let n3 = crate::s7_misc::free_to_live(
        g,
        img,
        f,
        [
            0x49e, 0x4b5, 0x4d2, 0x4da, 0x50b, 0x516, 0x51c, 0x52f, 0x550, 0x542, 0x550,
        ],
    );
    g.write::<u8>(n3.wrapping_add(0x18), img.word(f + 0x524) as u8);
    crate::sa_misc::records_init(g, img, img.rel32_target(f + 0x555), w(0x547));
    let n4 = crate::s7_misc::free_to_live(
        g,
        img,
        f,
        [
            0x55c, 0x56b, 0x588, 0x590, 0x5bc, 0x5c7, 0x5cd, 0x5e3, 0x5f1, 0x603, 0x60c,
        ],
    );
    g.write::<u8>(n4.wrapping_add(0x18), img.word(f + 0x5d7) as u8);
    Ret::eax(0)
}

/// [`summon_targets_start`] (the `rel32`s: the part set-up, the
/// records' set-up).
pub const SUMMON_TARGETS_START: Sig = Sig {
    len: 1558,
    hash: 0x21ff_0880_2ada_b60e,
    calls: &[
        (0x46, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x555, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
    ],
    imports: &[],
};

/// The summon's start task
/// ([`summon_start_task`](crate::s8_misc::summon_start_task)) with
/// [`summon_targets_start`].
///
/// `magic_0347` 0x10003490 (118 bytes; 2 DLLs).
pub fn summon_start_task_0347<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::s8_misc::summon_start_task(g, img, fr, summon_targets_start)
}

/// [`summon_start_task_0347`].
pub const SUMMON_START_TASK_0347: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&SUMMON_TARGETS_START))],
    imports: &[],
};

/// `Z * X * Y` as [`rotation_4250`](crate::s3_matrix::rotation_4250)
/// computes it, the same products and sums in the same order (its parity
/// test runs that port on this code), compiled two bytes shorter: the
/// rotation `magic_0070`'s family calls (`ecx` = the matrix, `edx` = the
/// angles, the sine table's global at `+0x0c`).
///
/// `magic_0070` 0x10003650 (1,380 bytes; 8 DLLs).
pub const ROTATION_3650: Sig = Sig {
    len: 1380,
    hash: 0x2714_ac72_7d96_743a,
    calls: &[],
    imports: &[],
};

/// A node's eased move (thiscall, `ecx` = the group `C`; a /GS frame,
/// `ebp` = `esp - 4` at the entry): the walk
/// ([`walk`](crate::nodes::walk), `eax` the cookie word; nothing more when
/// it stops). Then, with `N` the owner:
/// - the objects `N+0x124` and `N+0x128`, each visited through its
///   vtable's `+0x1c` with a cursor in the frame (`[ebp-0x5c]`,
///   `[ebp-0xc]`: the object, `+5` the group's mode; `+4` is left as the
///   frame holds it, and the callee may read it), their `+0x1c..+0x28`
///   then copied to `N+0x2c` and `N+0x4c`;
/// - by the byte `N+0x136`: 0 eases `N+0x1c..+0x24` towards `N+0x2c` by
///   the rate `N+0x5c` (`(x - t) * r + t`; `+0x28` = `+0x38`); 1 counts
///   the timer `N+0x11c` down by `K` (to 0 below 0), slot 233 `(&V,
///   N+0x10c, timer / N+0x120)` and slot 234 `(N+0x1c, N+0xfc, &V)`,
///   `N+0x28 = 1.0`;
/// - unless the byte `N+0x135`: `N+0x3c..+0x44` eased towards `+0x4c` by
///   `+0x60` (`+0x48` = `+0x58`);
/// - unless the byte `N+0x137`: the scales `+0xdc..+0xe4` multiplied by
///   `+0xec..+0xf4`, `+0xac..+0xb4` = `+0xcc.. * scale + +0xbc..` (`+0xb8`
///   = `+0xc8`); unless the `u16 N+0x98`: the same for `+0xe8`, `+0x64`;
/// - by the byte `N+0x134`: 1 moves the angles `+0x6c..+0x74` by
///   `-+0x7c..` and stops each at 0 when it crosses it, 0 scales them by
///   `+0x7c..`;
/// - the angle words `a_i = s_i - trunc(s_i * x_i)` (`s_i` the `i16`
///   `+0x8c..` negated when `x_i = +0x6c..` is below 0), `+0x78 = a_2 *
///   K1 * K2 + +0x88`, `a_2 = 0`; the rotation `M` of `a`
///   ([`rotation_4250`](crate::s3_matrix::rotation_4250)); with the flag
///   word `+0x12c`'s bits 8 and 1 set, `+0x4c..+0x54` = `+0x1c..` less
///   the rows of `M` dotted with `+0x9c..+0xa4` (`magic_0570`'s copy: its
///   columns, each product taken off in turn; `+0x58` = `+0xa8`);
/// - slots 284 `(*N+0x130, 1, N+0x3c)`, 284 `(.., 0, N+0x1c)`, 285
///   `(.., +0x78)`, 674 `(.., +0xac, +0xb0)` and 286 `(.., +0x64)`.
///
/// Returns slot 286's `eax`, or the walk's.
///
/// `magic_0155` 0x100031d0 (1,514 bytes; 4 DLLs), `magic_0570` 0x100040e0
/// (1,514 bytes; 2 DLLs).
pub fn node_ease<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, mul, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    let (eax, owner) = crate::nodes::walk(g, fr.ecx, fr.ecx, cookie);
    let Some(n) = owner else {
        return Ret::eax(eax);
    };
    let rf = |g: &G, o: u32| g.read::<f32>(n.wrapping_add(o));
    let t = |g: &G, s: u32| slot_target(g, w(0x135), s);
    for (obj_at, local, to) in [(0x124u32, 0x5cu32, 0x2cu32), (0x128, 0xc, 0x4c)] {
        let obj = g.read::<u32>(n + obj_at);
        if obj == 0 {
            continue;
        }
        let cur = ebp.wrapping_sub(local);
        let mode = g.read::<u8>(fr.ecx + 5);
        g.write::<u32>(cur, obj);
        g.write::<u8>(cur + 5, mode);
        let method = g.read::<u32>(g.read::<u32>(obj) + 0x1c);
        g.call(method, cur, &[cur]);
        let obj = g.read::<u32>(cur);
        for i in 0..4 {
            let x = g.read::<u32>(obj.wrapping_add(0x1c + 4 * i));
            g.write::<u32>(n + to + 4 * i, x);
        }
    }
    let v = ebp.wrapping_sub(0x54);
    let ease = |g: &mut G, (x, to, rate): (u32, u32, u32)| {
        for i in 0..3 {
            let d = sub(rf(g, x + 4 * i), rf(g, to + 4 * i));
            let e = add(mul(d, rf(g, rate)), rf(g, to + 4 * i));
            g.write::<f32>(n + x + 4 * i, e);
        }
        let last = g.read::<u32>(n + to + 0xc);
        g.write::<u32>(n + x + 0xc, last);
    };
    match g.read::<u8>(n + 0x136) {
        0 => ease(g, (0x1c, 0x2c, 0x5c)),
        1 => {
            let left = sub(rf(g, 0x11c), g.read::<f32>(w(0xf9)));
            g.write::<f32>(n + 0x11c, left);
            if 0.0 > left {
                g.write::<u32>(n + 0x11c, 0);
            }
            let k = crate::sb_sse::div(rf(g, 0x11c), rf(g, 0x120));
            g.call(t(g, 233), 0, &[v, n + 0x10c, k.to_bits()]);
            g.call(t(g, 234), 0, &[n + 0x1c, n + 0xfc, v]);
            g.write::<u32>(n + 0x28, 0x3f80_0000);
        }
        _ => {}
    }
    if g.read::<u8>(n + 0x135) == 0 {
        ease(g, (0x3c, 0x4c, 0x60));
    }
    // the scales: `(scale, step, base, out)` per component
    let scale = |g: &mut G, (s, k, a, b, out): (u32, u32, u32, u32, u32)| {
        let x = mul(rf(g, k), rf(g, s));
        let y = add(mul(rf(g, a), x), rf(g, b));
        g.write::<f32>(n + s, x);
        g.write::<f32>(n + out, y);
    };
    if g.read::<u8>(n + 0x137) == 0 {
        for i in 0..3 {
            let o = 4 * i;
            scale(g, (0xdc + o, 0xec + o, 0xcc + o, 0xbc + o, 0xac + o));
        }
        let last = g.read::<u32>(n + 0xc8);
        g.write::<u32>(n + 0xb8, last);
    }
    if g.read::<u16>(n + 0x98) == 0 {
        scale(g, (0xe8, 0xf8, 0xd8, 0xc8, 0x64));
    }
    match g.read::<u8>(n + 0x134) {
        1 => {
            for i in 0..3 {
                let step = rf(g, 0x7c + 4 * i);
                let x = sub(rf(g, 0x6c + 4 * i), step);
                g.write::<f32>(n + 0x6c + 4 * i, x);
                // stopped at 0 once it crosses it (a NaN step as a
                // negative one)
                let crossed = if step > 0.0 { x <= 0.0 } else { x > 0.0 };
                if crossed {
                    g.write::<u32>(n + 0x6c + 4 * i, 0);
                }
            }
        }
        0 => {
            for i in 0..3 {
                let x = mul(rf(g, 0x7c + 4 * i), rf(g, 0x6c + 4 * i));
                g.write::<f32>(n + 0x6c + 4 * i, x);
            }
        }
        _ => {}
    }
    let angles = ebp.wrapping_sub(0x14);
    for i in 0..3 {
        let x = rf(g, 0x6c + 4 * i);
        let s = i32::from(g.read::<i16>(n + 0x8c + 2 * i)).cast_unsigned();
        let a = if 0.0 > x { s.wrapping_neg() } else { s };
        let p = crate::s4_summon::trunc(mul(cvt(s), x));
        g.write::<u32>(angles + 4 * i, a.wrapping_sub(p));
    }
    let a2 = cvt(g.read::<u32>(angles + 8));
    let k = mul(mul(a2, g.read::<f32>(w(0x438))), g.read::<f32>(w(0x44a)));
    g.write::<u32>(angles + 8, 0);
    g.write::<f32>(n + 0x78, add(k, rf(g, 0x88)));
    crate::s3_matrix::rotation_4250(g, img, img.rel32_target(f + 0x45c), v, angles);
    let flags = g.read::<u32>(n + 0x12c);
    if flags & 8 != 0 && flags & 1 != 0 {
        let m = |g: &G, i: u32| g.read::<f32>(v + 4 * i);
        let (kx, ky, kz) = (rf(g, 0x9c), rf(g, 0xa0), rf(g, 0xa4));
        // `magic_0570`'s copy (`movss xmm0, [esi+0x9c]` at `+0x476`) takes
        // the columns of `M`, one product off at a time
        if img.word(f + 0x476) >> 24 == 0x86 {
            for (j, (to, from)) in (0u32..).zip([(0x4c, 0x1c), (0x50, 0x20), (0x54, 0x24)]) {
                let mut x = rf(g, from);
                for (i, k) in (0u32..).zip([kx, ky, kz]) {
                    x = sub(x, mul(k, m(g, j + 4 * i)));
                }
                g.write::<f32>(n + to, x);
            }
        } else {
            let r0 = add(add(mul(m(g, 0), kx), mul(ky, m(g, 1))), mul(m(g, 2), kz));
            g.write::<f32>(n + 0x4c, sub(rf(g, 0x1c), r0));
            let r1 = add(add(mul(m(g, 4), kx), mul(ky, m(g, 5))), mul(m(g, 6), kz));
            g.write::<f32>(n + 0x50, sub(rf(g, 0x20), r1));
            let r2 = add(add(mul(m(g, 8), kx), mul(ky, m(g, 9))), mul(m(g, 10), kz));
            g.write::<f32>(n + 0x54, sub(rf(g, 0x24), r2));
        }
        let last = g.read::<u32>(n + 0xa8);
        g.write::<u32>(n + 0x58, last);
    }
    let h = g.read::<u32>(n + 0x130);
    g.call(t(g, 284), 0, &[h, 1, n + 0x3c]);
    let h = g.read::<u32>(n + 0x130);
    g.call(t(g, 284), 0, &[h, 0, n + 0x1c]);
    let h = g.read::<u32>(n + 0x130);
    g.call(t(g, 285), 0, &[h, g.read::<u32>(n + 0x78)]);
    let h = g.read::<u32>(n + 0x130);
    let (x, y) = (g.read::<u32>(n + 0xac), g.read::<u32>(n + 0xb0));
    g.call(t(g, 674), 0, &[h, x, y]);
    let h = g.read::<u32>(n + 0x130);
    Ret::eax(g.call(t(g, 286), 0, &[h, g.read::<u32>(n + 0x64)]).eax)
}

/// [`node_ease`] (the `rel32`s: the rotation, `__security_check_cookie`).
pub const NODE_EASE: Sig = Sig {
    len: 1514,
    hash: 0x8213_4ecf_4407_072f,
    calls: &[(0x45c, Callee::Sig(&ROTATION_3650)), (0x5e2, Callee::Any)],
    imports: &[],
};

/// [`node_ease`] in `magic_0570` (the target set back by the matrix's
/// columns).
pub const NODE_EASE_B: Sig = Sig {
    hash: 0x3ccf_28f8_43ec_2d2e,
    ..NODE_EASE
};

/// The DLL's sprite records drawn (internal: `xmm3` = the time step `dt`;
/// a /GS frame aligned to 8, `E` = `((esp - 4) & !7) - 0x28` after the
/// saved registers, `dt` kept at `E+0x10`, the word `x` at `E+0xc`): `x`
/// starts as `*G_X`; for each of the `*G_N` records `R = *G_R + 0x94 i`
/// with `R+0` set: slot 325 `(R+0x44, P)` with `P` = `R+4`, or, while
/// `*G_F` is set, slot 229's `(L, V, R+4)` result `L` (`E+0x14`); then
/// `x` = the record `R+0x30`'s frame
/// ([`sprite_frame`](crate::o6_fam_pairs5::sprite_frame), `xmm3` = `dt`,
/// stack `(x, i)`). Returns `x`.
///
/// `magic_0206` 0x10002f30 (200 bytes; 4 DLLs).
pub fn sprite_frames<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x72), s);
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let dt = fr.xmm_f32(3).to_bits();
    let mut x = g.read::<u32>(w(0x1b));
    let base = g.read::<u32>(w(0x22));
    g.write::<u32>(e + 0x10, dt);
    g.write::<u32>(e + 0xc, x);
    let frame_at = img.rel32_target(f + 0x91);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0x15)) {
        let rec = base.wrapping_add(i.wrapping_mul(0x94));
        if g.read::<u32>(rec) != 0 {
            let p = if g.read::<u32>(w(0x47)) == 0 {
                rec.wrapping_add(4)
            } else {
                let l = e + 0x14;
                g.call(
                    slot_target(g, w(0x5d), 229),
                    0,
                    &[l, w(0x57), rec.wrapping_add(4)],
                );
                l
            };
            g.call(t(g, 325), 0, &[rec.wrapping_add(0x44), p]);
            g.write::<u32>(e.wrapping_sub(8), x);
            g.write::<u32>(e.wrapping_sub(4), i);
            let sub = Frame {
                at: frame_at,
                esp: e.wrapping_sub(0xc),
                eax: 0,
                ecx: rec.wrapping_add(0x30),
                edx: 0,
                xmm: [0, 0, 0, u128::from(dt)],
            };
            x = crate::o6_fam_pairs5::sprite_frame(g, img, &sub).eax;
            g.write::<u32>(e + 0xc, x);
        }
        i += 1;
    }
    Ret::eax(x)
}

/// [`sprite_frames`] (the `rel32`s: the record's frame,
/// `__security_check_cookie`).
pub const SPRITE_FRAMES: Sig = Sig {
    len: 200,
    hash: 0x6606_ddc1_2a24_1818,
    calls: &[
        (
            0x91,
            Callee::Sig(&crate::o6_fam_pairs5::SPRITE_FRAME_0206_2730),
        ),
        (0xc0, Callee::Any),
    ],
    imports: &[],
};

/// [`sprite_frames`] over a block (internal: `ecx` = the block `B`,
/// `xmm3` = the time step `dt`): the count at `B+0`, the records at
/// `*(B+4)`, the flag at `B+0x18`, slot 229's vector `B+0x1c`, `x`
/// from `*G_X`. `magic_0375`'s copy keeps its words on an `ebp` frame
/// (`dt` at `ebp-0x20`, `x` at `-0x1c`, `L` at `-0x18`, `B` at `-0x24`),
/// `magic_0371`'s on a frame aligned to 8 (`E` = `((esp - 4) & !7) -
/// 0x30`: `dt` at `E+0x14`, `x` at `+0x10`, `L` at `+0x1c`, `B` at
/// `+0x18`). Returns `x`.
///
/// `magic_0375` 0x10003890 (168 bytes; 3 DLLs), `magic_0371` 0x10002bf0
/// (181 bytes; 3 DLLs).
pub fn block_sprite_frames<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let aligned = img.word(f + 3) & 0xff00 == 0xe400;
    // (x's global, slot 229's table, slot 325's table, the frame's call)
    let ops = if aligned {
        [0x15, 0x4f, 0x64, 0x83]
    } else {
        [0x11, 0x48, 0x5c, 0x79]
    };
    let ebp = fr.esp.wrapping_sub(4);
    // (dt, x, L, B, the frame's `esp`)
    let [dt_at, x_at, l, b_at, call] = if aligned {
        let e = (ebp & !7).wrapping_sub(0x30);
        [e + 0x14, e + 0x10, e + 0x1c, e + 0x18, e.wrapping_sub(0xc)]
    } else {
        let s = |o: u32| ebp.wrapping_sub(o);
        [s(0x20), s(0x1c), s(0x18), s(0x24), s(0x40)]
    };
    let b = fr.ecx;
    let dt = fr.xmm_f32(3).to_bits();
    let mut x = g.read::<u32>(w(ops[0]));
    let base = g.read::<u32>(b.wrapping_add(4));
    g.write::<u32>(dt_at, dt);
    g.write::<u32>(b_at, b);
    g.write::<u32>(x_at, x);
    let frame_at = img.rel32_target(f + ops[3]);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(b) {
        let rec = base.wrapping_add(i.wrapping_mul(0x94));
        if g.read::<u32>(rec) != 0 {
            let p = if g.read::<u32>(b.wrapping_add(0x18)) == 0 {
                rec.wrapping_add(4)
            } else {
                let v = b.wrapping_add(0x1c);
                g.call(
                    slot_target(g, w(ops[1]), 229),
                    0,
                    &[l, v, rec.wrapping_add(4)],
                );
                l
            };
            g.call(
                slot_target(g, w(ops[2]), 325),
                0,
                &[rec.wrapping_add(0x44), p],
            );
            g.write::<u32>(call + 4, x);
            g.write::<u32>(call + 8, i);
            let sub = Frame {
                at: frame_at,
                esp: call,
                eax: 0,
                ecx: rec.wrapping_add(0x30),
                edx: 0,
                xmm: [0, 0, 0, u128::from(dt)],
            };
            x = crate::o6_fam_pairs5::sprite_frame(g, img, &sub).eax;
            g.write::<u32>(x_at, x);
        }
        i += 1;
    }
    Ret::eax(x)
}

/// [`block_sprite_frames`] in `magic_0375` (the `rel32`s: the record's
/// frame, `__security_check_cookie`).
pub const BLOCK_SPRITE_FRAMES: Sig = Sig {
    len: 168,
    hash: 0xa702_d613_9ae4_f596,
    calls: &[
        (
            0x79,
            Callee::Sig(&crate::o6_fam_pairs5::SPRITE_FRAME_0206_2730),
        ),
        (0xa0, Callee::Any),
    ],
    imports: &[],
};

/// [`block_sprite_frames`] in `magic_0371` (the frame aligned to 8).
pub const BLOCK_SPRITE_FRAMES_B: Sig = Sig {
    len: 181,
    hash: 0xfa84_9339_b622_a4fd,
    calls: &[
        (
            0x83,
            Callee::Sig(&crate::o6_fam_pairs5::SPRITE_FRAME_0206_2730),
        ),
        (0xad, Callee::Any),
    ],
    imports: &[],
};

/// A node's sparks spawned, moved and drawn (thiscall, `ecx` = the group
/// `C`; a /GS frame aligned to 16, `P` = `((esp - 4) & !0xf) - 0x100`
/// after the saved registers): the walk ([`walk`](crate::nodes::walk),
/// `eax` the cookie word, `ecx` the owner for every method; nothing more
/// when it stops). Then, with `N` the owner:
/// - when `K0 > N+0x60`: a request (`P+0x1c`: `N`, a count of `slot 324
///   & 0x1f + 0x10`) takes records off `N`'s ring
///   ([`ring_take`](crate::m0070::ring_take), one or two pieces); each
///   record `R` (0x40 bytes) gets `f` = slot 307 (ST0) `- K1`, `R+0x12`/
///   `+0x30` = slot 324, the kind `R+0x24` = whether its low two bits are
///   0 (colour `R+0x28` 0x80808080, else 0x40808080), the size `R+0x20 =
///   f * K2 + K3`, the live byte `R+0x34 = 1`, `R+0x2c = 0`, slot 325
///   `(R, N+0x38)`, `R+0..+8 += K4 - (slot 307 - K1) * K5` each, and the
///   speed `R+0x14..+0x1c = N+0x48.. * K6 * f`;
/// - else, with no sparks counted (`N+0x64`), the node is released
///   ([`node_release`](crate::s6_misc::node_release), `ecx` = `&N`) and
///   that is all.
///
/// Then the draw: a render state at `P+0x30` (`+2` = 0x80, `+0x10` and
/// `+0x1c` its matrix `P+0xb8`, `+0x14` = `*G_Z`, its words cleared) and
/// the local matrix at `P+0x68`; `N+0x64 = 0`. For both lists (records
/// at `N+0x24 + 4j`, u16 counts at `N+0x2c + 2j`) each record with its
/// live byte and `+0x2b` set: slot 229 `(P+0x98, V, R)`; the scale `x =
/// R+0x20 * R+0x2c` on the diagonal; the first two rows turned by the
/// angle word `R+0x30` (the sine table; the terms with the rows as the
/// last record left them) and set by slot 325; slot 230 `(P+0xb8,
/// P+0x68)`; the fade `R+0x2c` grows by `K8` below `K7` (capped at 4.0);
/// the colour into the render state; the record's sequence step
/// ([`motion_sequence_step`](crate::s6_misc::motion_sequence_step),
/// `ecx` = `R+0x10`, `edx` = `N+0x58`'s entry of its kind, the stack
/// `(P+0x30)`) gives the live byte; `R+0x2b -= 4`; the position moves by
/// the speed, the speed's `x` and `z` scale by `K9`, its `y` drops by
/// `K10`; `N+0x64` counts it. `*G_Z` = the render state's `+0x14`, `N+0x60
/// += K11`. Returns that word, the release's `eax` or the walk's.
///
/// `magic_0070` 0x100018e0 (1,584 bytes; 3 DLLs).
#[allow(clippy::too_many_lines)] // the original's three phases, in order
pub fn node_sparks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, s: u32| slot_target(g, w(0x85), s);
    let st0 = |o: Out| crate::m0018::st0_f32(o.st0);
    let s = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0xf8);
    let p = s.wrapping_sub(8);
    let cookie = g.read::<u32>(w(0xd)) ^ s;
    let (eax, owner) = crate::nodes::walk(g, fr.ecx, g.read::<u32>(fr.ecx), cookie);
    let Some(n) = owner else {
        return Ret::eax(eax);
    };
    let rf = |g: &G, o: u32| g.read::<f32>(n.wrapping_add(o));
    if k(g, 0x76) > rf(g, 0x60) {
        g.write::<u32>(p + 0x1c, n);
        let r = g.call(t(g, 324), 0, &[]).eax;
        g.write::<u16>(p + 0x20, ((r & 0x1f) + 0x10) as u16);
        crate::m0070::ring_take(g, p + 0x1c);
        for j in 0..2u32 {
            let mut rec = g.read::<u32>(p + 0x28 + 4 * j);
            let count = g.read::<u16>(p + 0x22 + 2 * j);
            for _ in 0..count {
                g.write::<u32>(rec + 0x10, 0);
                let fv = sub(st0(g.call(t(g, 307), 0, &[])), k(g, 0xe5));
                let r = g.call(t(g, 324), 0, &[]).eax;
                // `test al, 3; sete`: the low two bits clear
                let kind = u32::from(r.trailing_zeros() >= 2);
                g.write::<u16>(rec + 0x12, r as u16);
                g.write::<u32>(rec + 0x30, r & 0xffff);
                g.write::<u32>(rec + 0x20, 0x3d4c_cccd);
                g.write::<u32>(rec + 0x24, kind);
                g.write::<u32>(
                    rec + 0x28,
                    if kind != 0 { 0x8080_8080 } else { 0x4080_8080 },
                );
                let size = add(mul(fv, k(g, 0x138)), k(g, 0x144));
                g.write::<u8>(rec + 0x34, 1);
                g.write::<u32>(rec + 0x2c, 0);
                g.write::<f32>(rec + 0x20, size);
                g.call(t(g, 325), 0, &[rec, n + 0x38]);
                for o in [0u32, 4, 8] {
                    let v = sub(st0(g.call(t(g, 307), 0, &[])), k(g, 0xe5));
                    let d = sub(k(g, 0x18c), mul(v, k(g, 0x199)));
                    let x = add(d, g.read::<f32>(rec + o));
                    g.write::<f32>(rec + o, x);
                }
                for o in [0u32, 4, 8] {
                    let v = mul(mul(rf(g, 0x48 + o), k(g, 0x20c)), fv);
                    g.write::<f32>(rec + 0x14 + o, v);
                }
                rec = rec.wrapping_add(0x40);
            }
        }
    } else if g.read::<u32>(n + 0x64) == 0 {
        g.write::<u32>(p + 0xc, n);
        let release = Frame {
            at: img.rel32_target(f + 0x618),
            esp: p.wrapping_sub(4),
            eax: 0,
            ecx: p + 0xc,
            edx: 0,
            xmm: [0; 4],
        };
        return Ret::eax(crate::s6_misc::node_release(g, img, &release));
    }
    // the draw
    let l = |g: &G, o: u32| g.read::<f32>(p + o);
    let m = p + 0xb8;
    g.write::<u32>(p + 0x4c, m);
    g.write::<u32>(p + 0x40, m);
    g.write::<u16>(p + 0x32, 0x80);
    let z = g.read::<u32>(w(0x29b));
    g.write::<u32>(p + 0x44, z);
    for o in [
        0x6cu32, 0x70, 0x74, 0x78, 0x80, 0x84, 0x88, 0x8c, 0x94, 0x38,
    ] {
        g.write::<u32>(p + o, 0);
    }
    g.write::<u8>(p + 0x48, 0);
    g.write::<u32>(n + 0x64, 0);
    let table = |g: &G| g.read::<u32>(w(0x370));
    let step_at = img.rel32_target(f + 0x531);
    for j in 0..2u32 {
        let mut rec = g.read::<u32>(n + 0x24 + 4 * j);
        let counts = n + 0x2c + 2 * j;
        let mut i = 0u32;
        while i < u32::from(g.read::<u16>(counts)) {
            if g.read::<u8>(rec + 0x34) != 0 && g.read::<u8>(rec + 0x2b) != 0 {
                g.call(t(g, 229), 0, &[p + 0x98, w(0x352), rec]);
                let x3 = mul(g.read::<f32>(rec + 0x20), g.read::<f32>(rec + 0x2c));
                let tb = table(g);
                for o in [0x90u32, 0x7c, 0x68] {
                    g.write::<f32>(p + o, x3);
                }
                let a = g.read::<u32>(rec + 0x30);
                let sine = |g: &G, a: u32| g.read::<f32>(tb.wrapping_add(4 * ((a >> 4) & 0xfff)));
                let (sn, cs) = (sine(g, a), sine(g, a.wrapping_add(0x4000)));
                let row = [
                    (0xa8u32, add(mul(cs, x3), mul(sn, l(g, 0x78)))),
                    (0x1c, sub(mul(cs, l(g, 0x78)), mul(sn, x3))),
                    (0xac, add(mul(sn, x3), mul(cs, l(g, 0x6c)))),
                    (0x20, sub(mul(cs, x3), mul(sn, l(g, 0x6c)))),
                    (0xb0, add(mul(sn, l(g, 0x80)), mul(cs, l(g, 0x70)))),
                    (0x24, sub(mul(cs, l(g, 0x80)), mul(sn, l(g, 0x70)))),
                    (0xb4, add(mul(sn, l(g, 0x84)), mul(cs, l(g, 0x74)))),
                    (0x28, sub(mul(cs, l(g, 0x84)), mul(sn, l(g, 0x74)))),
                ];
                for (o, v) in row {
                    g.write::<f32>(p + o, v);
                }
                g.call(t(g, 325), 0, &[p + 0x68, p + 0xa8]);
                g.call(t(g, 325), 0, &[p + 0x78, p + 0x1c]);
                g.call(t(g, 230), 0, &[p + 0xb8, p + 0x68]);
                let fade = g.read::<f32>(rec + 0x2c);
                let top = k(g, 0x4f2);
                if top > fade {
                    let v = add(fade, k(g, 0x502));
                    g.write::<f32>(rec + 0x2c, v);
                    if v > top {
                        g.write::<u32>(rec + 0x2c, 0x4080_0000);
                    }
                }
                let colour = g.read::<u32>(rec + 0x28);
                g.write::<u32>(p + 0x34, colour);
                let kind = g.read::<u32>(rec + 0x24);
                let q = g.read::<u32>(kind.wrapping_mul(4).wrapping_add(n + 0x58));
                g.write::<u32>(p.wrapping_sub(0xc), p + 0x30);
                let step = Frame {
                    at: step_at,
                    esp: p.wrapping_sub(0x10),
                    eax: 0,
                    ecx: rec + 0x10,
                    edx: q,
                    xmm: [0; 4],
                };
                let live = crate::s6_misc::motion_sequence_step(g, img, &step);
                g.write::<u8>(rec + 0x34, live as u8);
                let (vx, x0, vy, vz) = (
                    g.read::<f32>(rec + 0x14),
                    g.read::<f32>(rec),
                    g.read::<f32>(rec + 0x18),
                    g.read::<f32>(rec + 0x1c),
                );
                let b = g.read::<u8>(rec + 0x2b).wrapping_add(0xfc);
                g.write::<u8>(rec + 0x2b, b);
                g.write::<f32>(rec, add(x0, vx));
                let y = add(vy, g.read::<f32>(rec + 4));
                g.write::<f32>(rec + 0x14, mul(vx, k(g, 0x55f)));
                g.write::<f32>(rec + 4, y);
                let zz = add(g.read::<f32>(rec + 8), vz);
                g.write::<f32>(rec + 0x18, sub(vy, k(g, 0x576)));
                g.write::<f32>(rec + 8, zz);
                g.write::<f32>(rec + 0x1c, mul(vz, k(g, 0x55f)));
                let c = g.read::<u32>(n + 0x64).wrapping_add(1);
                g.write::<u32>(n + 0x64, c);
            }
            i += 1;
            rec = rec.wrapping_add(0x40);
        }
    }
    let z = g.read::<u32>(p + 0x44);
    g.write::<u32>(w(0x5db), z);
    g.write::<f32>(n + 0x60, add(rf(g, 0x60), k(g, 0x5e8)));
    Ret::eax(z)
}

/// [`node_sparks`] (the `rel32`s: the ring take, the sequence step,
/// `__security_check_cookie` twice, the release).
pub const NODE_SPARKS: Sig = Sig {
    len: 1584,
    hash: 0x54c9_f75a_27b0_7666,
    calls: &[
        (0xa5, Callee::Sig(&crate::m0070::RING_TAKE)),
        (0x531, Callee::Sig(&crate::s6_misc::MOTION_SEQUENCE_STEP)),
        (0x5fd, Callee::Any),
        (0x618, Callee::Sig(&crate::s6_misc::NODE_RELEASE)),
        (0x628, Callee::Any),
    ],
    imports: &[],
};

/// The host's objects run and drawn (internal: `ecx` = the host `C`,
/// `edx` = a context word; an `ebp` frame: the matrix `M` at `ebp-0x94`,
/// the vector `D` at `ebp-0x54`, the object's matrix `O` at `ebp-0x44`,
/// its place `T` at `ebp-0x14`). The list `L = C+0x30` restarts (`L+8 =
/// L+0`), `*G_HOST = C`, `*G_CTX = edx` unless 0; slots 723 `(*G_A)`, 724
/// `(*G_B)`, 235 `(M)`. For each object `X` of the list
/// ([`list_cur`](crate::l5_objhost::list_cur)):
/// - not live (`u16 X+0x124` without 0x8000): skipped, not counted; with
///   bit 1, or while its wait `X+0x114` counts down: counted;
/// - its update `X+0x104` and, when its frame `X+0x11c` is its count
///   `X+0x118`, its hook `X+0x100` (cdecl `(X)`); by the mode byte
///   `X+0x126`: 1 slot 554 `(O, M, X+0xc0)`; 2 `O` = `X+0x80..+0xac`, `T`
///   = `M`'s last row; else slots 330 `(O, M, X+0xc8)`, 328 `(O, O,
///   X+0xc0)`, 329 `(O, O, X+0xc4)`;
/// - slot 229 `(D, O, X+0xe0)`, `O`'s rows scaled by `X+0x60`, `+0x64`,
///   `+0x68`, `T` = `X+0xd0.. + D`, slot 552 `(X+0x40, T)`, slot 230
///   `(*G_O, O)`; `*G_P = X+0x108` unless 0; `X+0xf0..+0xfc` into `*G_V`;
///   slots 231 `(*G_C, *G_D, *G_O)` and 230 `(*G_E, *G_C)`;
/// - flagged 0x4000 (the flag dropped), or when slot 712 `(X, 0x30)`
///   answers 1 (with `*G_LOCK` 1 around it): `*G_HOST = C`, slot 713
///   `(X)`, the flag 0x8000 dropped and `X` off the list
///   ([`list_remove`](crate::l5_objhost::list_remove)); otherwise slot
///   160 `(X, X+0x10c, X)` when `X+0x10c` is set;
/// - its target `X+0x110` (when set) takes `T` (its last word 0 until a
///   mode 2 object sets it: the original's frame word there is never
///   written before) and runs its script
///   ([`obj_script`](crate::l5_objhost::obj_script)); the count `X+0x118`
///   goes up; counted.
///
/// After each object the list's cursor moves on when it still points at
/// it. Returns 1 when no object was counted, else 0.
///
/// `magic_0244` 0x10001af0 (1,262 bytes; 3 DLLs).
#[allow(clippy::too_many_lines)] // the original's per-object steps, in order
pub fn host_objects_run<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x3a), s);
    let host = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let (m, d, o, tv) = (
        ebp.wrapping_sub(0x94),
        ebp.wrapping_sub(0x54),
        ebp.wrapping_sub(0x44),
        ebp.wrapping_sub(0x14),
    );
    // `T`'s last word is written only by mode 2; before that the
    // original copies whatever the frame holds there into a target. The
    // port uses 0 there (the never-written stack read rule).
    g.write::<u32>(tv + 0xc, 0);
    let list = host.wrapping_add(0x30);
    let first = g.read::<u32>(list);
    g.write::<u32>(list + 8, first);
    g.write::<u32>(ebp.wrapping_sub(0x98), host);
    let gh = g.read::<u32>(w(0x22));
    g.write::<u32>(gh, host);
    if fr.edx != 0 {
        let gc = g.read::<u32>(w(0x33));
        g.write::<u32>(gc, fr.edx);
    }
    let b = g.read::<u32>(w(0x4c));
    let a = g.read::<u32>(w(0x40));
    g.call(t(g, 723), 0, &[a]);
    g.call(t(g, 724), 0, &[b]);
    g.call(t(g, 235), 0, &[m]);
    let mut counted = 0u32;
    let rf = |g: &G, x: u32, off: u32| g.read::<f32>(x.wrapping_add(off));
    let mut x = crate::l5_objhost::list_cur(g, list);
    while x != 0 {
        let flags = g.read::<u16>(x + 0x124);
        let mut count = true;
        if flags & 0x8000 == 0 {
            count = false;
        } else if flags & 1 != 0 {
        } else if g.read::<u32>(x + 0x114) != 0 {
            let left = g.read::<u32>(x + 0x114).wrapping_sub(1);
            g.write::<u32>(x + 0x114, left);
        } else {
            let update = g.read::<u32>(x + 0x104);
            if update != 0 {
                g.call(update, 0, &[x]);
            }
            let hook = g.read::<u32>(x + 0x100);
            if hook != 0 && g.read::<u32>(x + 0x11c) == g.read::<u32>(x + 0x118) {
                g.call(hook, hook, &[x]);
            }
            match g.read::<u8>(x + 0x126) {
                1 => {
                    g.call(t(g, 554), 0, &[o, m, x + 0xc0]);
                }
                2 => {
                    for i in 0..12 {
                        let v = g.read::<u32>(x + 0x80 + 4 * i);
                        g.write::<u32>(o + 4 * i, v);
                    }
                    for i in 0..4 {
                        let v = g.read::<u32>(m + 0x30 + 4 * i);
                        g.write::<u32>(tv + 4 * i, v);
                    }
                }
                _ => {
                    let c8 = g.read::<u32>(x + 0xc8);
                    g.call(t(g, 330), 0, &[o, m, c8]);
                    let c0 = g.read::<u32>(x + 0xc0);
                    g.call(t(g, 328), 0, &[o, o, c0]);
                    let c4 = g.read::<u32>(x + 0xc4);
                    g.call(t(g, 329), 0, &[o, o, c4]);
                }
            }
            g.call(t(g, 229), 0, &[d, o, x + 0xe0]);
            for (row, k) in [(0u32, 0x60u32), (0x10, 0x64), (0x20, 0x68)] {
                let s = rf(g, x, k);
                for i in 0..3 {
                    let v = mul(g.read::<f32>(o + row + 4 * i), s);
                    g.write::<f32>(o + row + 4 * i, v);
                }
            }
            for i in 0..3 {
                let v = add(rf(g, x, 0xd0 + 4 * i), g.read::<f32>(d + 4 * i));
                g.write::<f32>(tv + 4 * i, v);
            }
            g.call(t(g, 552), 0, &[x + 0x40, tv]);
            let go = g.read::<u32>(w(0x337));
            g.call(t(g, 230), 0, &[go, o]);
            let target = g.read::<u32>(x + 0x108);
            if target != 0 {
                let gp = g.read::<u32>(w(0x351));
                g.write::<u32>(gp, target);
            }
            let gv = g.read::<u32>(w(0x35f));
            for i in 0..4 {
                let v = g.read::<u32>(x + 0xf0 + 4 * i);
                g.write::<u32>(gv + 4 * i, v);
            }
            let (gd, gc, go) = (
                g.read::<u32>(w(0x382)),
                g.read::<u32>(w(0x373)),
                g.read::<u32>(w(0x365)),
            );
            g.call(t(g, 231), 0, &[gd, gc, go]);
            let (ge, gd) = (g.read::<u32>(w(0x3ac)), g.read::<u32>(w(0x3a1)));
            g.call(t(g, 230), 0, &[ge, gd]);
            let flags = g.read::<u16>(x + 0x124);
            let leave = if flags & 0x4000 != 0 {
                g.write::<u16>(x + 0x124, flags & 0xbfff);
                true
            } else {
                let gh = g.read::<u32>(w(0x3ca));
                g.write::<u32>(gh, host);
                let lock = g.read::<u32>(w(0x3d7));
                g.write::<u32>(lock, 1);
                let r = g.call(t(g, 712), 0, &[x, 0x30]).eax;
                let lock = g.read::<u32>(w(0x3f3));
                g.write::<u32>(lock, 0);
                if r != 1 {
                    let cb = g.read::<u32>(x + 0x10c);
                    if cb != 0 {
                        g.call(t(g, 160), 0, &[x, cb, x]);
                    }
                }
                r == 1
            };
            if leave {
                let gh = g.read::<u32>(w(0x431));
                g.write::<u32>(gh, host);
                g.call(t(g, 713), 0, &[x]);
                let fl = g.read::<u16>(x + 0x124) & 0x7fff;
                g.write::<u16>(x + 0x124, fl);
                crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x45f), list, x);
            }
            let target = g.read::<u32>(x + 0x110);
            if target != 0 {
                for i in 0..4 {
                    let v = g.read::<u32>(tv + 4 * i);
                    g.write::<u32>(target + 4 * i, v);
                }
                let target = g.read::<u32>(x + 0x110);
                crate::l5_objhost::obj_script(g, img, img.rel32_target(f + 0x49b), target);
            }
            let n = g.read::<u32>(x + 0x118).wrapping_add(1);
            g.write::<u32>(x + 0x118, n);
        }
        if count {
            counted += 1;
        }
        if x == crate::l5_objhost::list_cur(g, list) {
            let cur = g.read::<u32>(list + 8);
            if cur != 0 {
                let next = g.read::<u32>(cur + 4);
                g.write::<u32>(list + 8, next);
            }
        }
        x = crate::l5_objhost::list_cur(g, list);
    }
    Ret::eax(u32::from(counted == 0))
}

/// [`host_objects_run`] (the `rel32`s: the list's current object three
/// times, the removal, the script, `__security_check_cookie`).
pub const HOST_OBJECTS_RUN: Sig = Sig {
    len: 1262,
    hash: 0x17c9_8375_34be_2ce4,
    calls: &[
        (0x7a, Callee::Sig(&crate::l5_objhost::LIST_CUR)),
        (0x45f, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x49b, Callee::Sig(&crate::l5_objhost::OBJ_SCRIPT)),
        (0x4b2, Callee::Sig(&crate::l5_objhost::LIST_CUR)),
        (0x4c8, Callee::Sig(&crate::l5_objhost::LIST_CUR)),
        (0x4e6, Callee::Any),
    ],
    imports: &[],
};

/// One of [`effect_group_init`]'s two node lists: `node` put last on the
/// DLL's list (its `+0x1a` 0xff, its own child ring empty, parent `R`,
/// the list's tail `*G_T` and first `*G_F` when the list was empty, the
/// count `u16 *G_C` up by one) and given the vtable `vt`. `ops` are the
/// operands of the original's inlined copy: `(R, R again, tail read,
/// tail read again, R compared, first, count, tail written, vtable)`.
fn list_node<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, ops: [u32; 9]) {
    let w = |o: u32| op(img, f, o);
    let [r, r2, tail, tail2, root, first, count, tail_w, vt] = ops;
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, w(r));
    g.write::<u32>(node + 4, w(r2));
    let t = g.read::<u32>(w(tail));
    g.write::<u32>(node + 8, t);
    let t = g.read::<u32>(w(tail2));
    if t == w(root) {
        g.write::<u32>(w(first), node);
    } else {
        g.write::<u32>(t.wrapping_add(4), node);
    }
    let n = g.read::<u16>(w(count)).wrapping_add(1);
    g.write::<u16>(w(count), n);
    g.write::<u32>(w(tail_w), node);
    g.write::<u32>(node, w(vt));
}

/// An effect object `X` set up with its nodes (internal: `ecx` = `X`,
/// `edx` = the definition `D`, `xmm2` = a scale `s`; a frame aligned to
/// 8, `P` = `((esp - 4) & !7) - 0x30` after the saved registers, the
/// set-up record at `P+0x18`). By slot 390 `(D+0x50)` less 0x1071 (the
/// DLL's byte table, or slot 534 `(D+0x50)` when the table does not
/// pick the first kind) the object takes one of two copies of the same
/// code, which differ in the callback `CB` and the record's count `k`
/// (3 or 2): `X` joins the DLL's object ring (vtable, `CB`, the ring's
/// last `*G_L`, the flag bit 0 of `*G_FL` cleared); its node `X+0x220`
/// goes on the node list ([`list_node`]); slot 268 `(D+0x54, 0x11,
/// X+0x23c)`, `X+0x248 = 1.0`; the node `X+0x278` the same; `X+0x294..`
/// = `(1.0, 1.0, 1.0, s)`; the node `X+0x2d0` inserted under `X+0x220`
/// ([`node_insert`](crate::nodes::node_insert)). Then the particle node
/// `X+0x20` ([`particle_node`](crate::sb_nodes::particle_node), program
/// index `k`), the effect node `X+0x57c`
/// ([`effect_node`](crate::sb_nodes::effect_node)) with its vtable,
/// `+0x2c |= 1`, `+0x30 = 0`; `X+0x23c..+0x244` each move by a random
/// step (slot 307, ST0); `X+0x578 = D`, `X+0x568`/`+0x570`/`+0x574` 0,
/// `X+0x56c = s`. Returns `D`.
///
/// `magic_0411` 0x10006d30 (1,244 bytes with its jump tables; 2 DLLs).
#[allow(clippy::too_many_lines)] // the original's set-up, store by store
pub fn effect_group_init<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x15), s);
    let st0 = |o: Out| crate::m0018::st0_f32(o.st0);
    let (x, d) = (fr.ecx, fr.edx);
    let s = fr.xmm_f32(2);
    let p = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x30);
    g.write::<u32>(p + 0x10, d);
    g.write::<f32>(p + 0xc, s);
    let id = g.read::<u32>(d + 0x50);
    let k = g.call(t(g, 390), 0, &[id]).eax.wrapping_sub(0x1071);
    // the switch: the byte table (in the code, read from the image) has
    // 0 for the first copy at once; any other entry (or `k` past it) asks
    // slot 534 first
    let first_kind = (k <= 0x1b && img.word(w(0x3b).wrapping_add(k)) as u8 == 0)
        || g.call(t(g, 534), 0, &[id]).eax != 0;
    // the copy's operands: `+0x5f` on, or the same 0x171 bytes later
    let b = if first_kind { 0 } else { 0x171 };
    let v = |o: u32| w(o + b);
    g.write::<u32>(x + 8, x);
    g.write::<u32>(x + 4, x);
    g.write::<u32>(x, v(0x67));
    g.write::<u32>(x + 0x10, v(0x6e));
    g.write::<u32>(x + 0x14, x);
    let last = g.read::<u32>(v(0x76));
    g.write::<u32>(x + 8, last);
    g.write::<u32>(x + 4, v(0x80));
    let last = g.read::<u32>(v(0x85));
    let node = x + 0x220;
    g.write::<u32>(last.wrapping_add(4), x);
    let fl = g.read::<u32>(v(0x94)) & !1;
    g.write::<u32>(v(0x94), fl);
    g.write::<u32>(v(0x9b), x);
    let ops = |o: [u32; 9]| o.map(|o| o + b);
    list_node(
        g,
        img,
        f,
        node,
        ops([0xae, 0xb5, 0xba, 0xc2, 0xc7, 0xcf, 0xdb, 0xeb, 0xf2]),
    );
    let h = g.read::<u32>(d + 0x54);
    g.call(t(g, 268), 0, &[h, 0x11, x + 0x23c]);
    g.write::<u32>(x + 0x248, 0x3f80_0000);
    let second = x + 0x278;
    let ops2 = ops([
        0x127, 0x12e, 0x133, 0x13b, 0x140, 0x148, 0x154, 0x160, 0x166,
    ]);
    list_node(g, img, f, second, ops2);
    let rec = p + 0x18;
    g.write::<u32>(rec + 0x14, second);
    for o in [0x294u32, 0x298, 0x29c] {
        g.write::<u32>(x + o, 0x3f80_0000);
    }
    g.write::<f32>(x + 0x2a0, s);
    let words = [x + 0x2d0, v(0x1a6), v(0x1ae), node, v(0x1ba)];
    for (i, word) in (0u32..).zip(words) {
        g.write::<u32>(rec + 4 * i, word);
    }
    let insert = Frame {
        at: img.rel32_target(f + b + 0x1bf),
        esp: p.wrapping_sub(4),
        ecx: rec,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &insert);
    g.write::<u32>(rec + 0x14, v(0x1c7));
    // the particle node and the effect node
    let part = g.read::<u32>(w(0x33d));
    g.write::<u32>(rec, x + 0x20);
    g.write::<u32>(x + 0x20, w(0x34a));
    let words = [w(0x356), x + 0x2d0, part, w(0x366)];
    for (i, word) in (1u32..).zip(words) {
        g.write::<u32>(rec + 4 * i, word);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x36b), rec);
    let fx = x + 0x57c;
    let words = [fx, w(0x381), w(0x389), node, w(0x395), w(0x39d)];
    for (i, word) in (0u32..).zip(words) {
        g.write::<u32>(rec + 4 * i, word);
    }
    crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x3a2), rec);
    let n = g.read::<u32>(rec);
    g.write::<u32>(n, w(0x3ac));
    let fl = g.read::<u32>(fx + 0x2c) | 1;
    g.write::<u32>(fx + 0x2c, fl);
    g.write::<u32>(fx + 0x30, 0);
    // the random steps
    for (o, (k1, k2, k3)) in [
        (0x23cu32, (0x3d6u32, 0x3e3u32, 0x3ebu32)),
        (0x240, (0x415, 0x422, 0x42a)),
        (0x244, (0x454, 0x47a, 0x48c)),
    ] {
        let r = sub(st0(g.call(t(g, 307), 0, &[])), g.read::<f32>(w(k1)));
        let r = sub(mul(r, g.read::<f32>(w(k2))), g.read::<f32>(w(k3)));
        if o == 0x244 {
            g.write::<u32>(x + 0x578, d);
            for z in [0x568u32, 0x570, 0x574] {
                g.write::<u32>(x + z, 0);
            }
        }
        let v = add(r, g.read::<f32>(x + o));
        g.write::<f32>(x + o, v);
    }
    g.write::<f32>(x + 0x56c, s);
    Ret::eax(d)
}

/// [`effect_group_init`] (the `rel32`s: the node insert in both copies,
/// the particle node, the effect node).
pub const EFFECT_GROUP_INIT: Sig = Sig {
    len: 1244,
    hash: 0x06fd_c15a_df90_fa75,
    calls: &[
        (0x1bf, Callee::Sig(&crate::nodes::INSERT)),
        (0x330, Callee::Sig(&crate::nodes::INSERT)),
        (0x36b, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x3a2, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// `KA - a` brought back once into the range: less `KB` above `KC`, plus
/// `KB` below `KD` (the operands `(KA, KB, KC, KD)` of `f`).
fn fold_angle(g: &impl crate::Mem, img: &Image, f: u32, a: f32, ops: [u32; 4]) -> f32 {
    use crate::sb_sse::{add, sub};
    let k = |o: u32| g.read::<f32>(op(img, f, o));
    let [ka, kb, kc, kd] = ops;
    let x = sub(k(ka), a);
    if x > k(kc) {
        sub(x, k(kb))
    } else if k(kd) > x {
        add(x, k(kb))
    } else {
        x
    }
}

/// A summon object `X` started between two actors (thiscall, `ecx` =
/// `X`; a /GS frame aligned to 8, `P` = `((esp - 4) & !7) - 0x98` after
/// the saved registers): `V0` (`P+0x20`, `w` 1.0) = slot 262 then 268
/// `(.., 0x11, V0)` of `X`'s actor (`X+0x504`'s battle number, or the
/// none actor for -1), `V1` (`P+0x30`, `w` 1.0) the same of the none
/// actor; the object started at `V0` with scale `K`
/// ([`start_object_first`](crate::o6_fam_object::start_object_first),
/// stack `(0, w, 0x10, P+0x28)`: `w` is the `ecx` slot 268 leaves, which
/// the port takes as 0) and `X+0xd0..` = `V0`. Then the yaw of the first
/// entry's actor (slot 333, ST0) folded ([`fold_angle`]) into `X+0xc4`;
/// `X+0x120` = the battle number (or the none word); the record `R =
/// X+0x488` reset (callback and data immediates) into `X+0x110`; the step
/// `d = (V1 - V0) / K5` (`P+0x1c`, `+0x18`, `+0x14`); `X+0x4bc..+0x4c8` =
/// `V0`; `W` = `(slot 314 (40.0), slot 404 (20.0), 0, 1.0)` (ST0s) turned
/// by the matrix `M` (slot 235, slot 329 `(M, M, a)`, `a` the folded
/// `atan2(dz, dx)` of `V1 - V0`) with slot 229 `(W, M, W)`; `X+0x4cc..` =
/// `V0 + d + W`, its `y` lowered to the actor's height (slot 12, ST0) less
/// `K6` when above it; `X+0x4dc..` = `d K7 + V0 - W`; `X+0x4ec..` = `V1`;
/// a few words; the sprite set up with the colour `V0`
/// ([`sprite_init_colour`](crate::sb_misc::sprite_init_colour), `X+0x260`,
/// `(*G_S)+0x10`). Returns its `eax`.
///
/// `magic_0134` 0x10001980 (1,331 bytes; 2 DLLs).
#[allow(clippy::too_many_lines)] // the original's set-up, in order
pub fn summon_object_between<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let t = |g: &G, s: u32| slot_target(g, w(0x51), s);
    let st0 = |o: Out| crate::m0018::st0_f32(o.st0);
    let x = fr.ecx;
    let p = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x98);
    let lf = |g: &G, o: u32| g.read::<f32>(p + o);
    let i = g.read::<u32>(x + 0x504);
    let actor = entry_actor(g, img, f, (0x2b, 0x32, 0x3d), i);
    let (v0, v1) = (p + 0x20, p + 0x30);
    g.call(t(g, 262), 0, &[actor, v0, v0 + 4, v0 + 8]);
    g.call(t(g, 268), 0, &[actor, 0x11, v0]);
    let none = g.read::<u32>(w(0x75));
    g.write::<u32>(p + 0x2c, 0x3f80_0000);
    g.call(t(g, 262), 0, &[none, v1, v1 + 4, v1 + 8]);
    g.call(t(g, 268), 0, &[none, 0x11, v1]);
    g.write::<u32>(p + 0x3c, 0x3f80_0000);
    // the object start: `(0, w, 0x10, P+0x28)`, `w` taken as 0
    let call = p.wrapping_sub(0x14);
    for (k, word) in (1u32..).zip([0, 0, 0x10, p + 0x28]) {
        g.write::<u32>(call + 4 * k, word);
    }
    let start = img.rel32_target(f + 0xce);
    let scale = g.read::<u32>(w(0xb7));
    let sub_fr = Frame {
        at: start,
        esp: call,
        eax: 0,
        ecx: 0,
        edx: x,
        xmm: [0, 0, 0, u128::from(scale)],
    };
    crate::o6_fam_object::start_object_first(g, img, start, &sub_fr);
    for k in 0..4 {
        let v = g.read::<u32>(v0 + 4 * k);
        g.write::<u32>(x + 0xd0 + 4 * k, v);
    }
    let first = entry_actor(g, img, f, (0x2b, 0x10b, 0x115), 0);
    let yaw = st0(g.call(t(g, 333), 0, &[first]));
    let fold = [0x12e, 0x13c, 0x146, 0x156];
    let yaw = fold_angle(g, img, f, yaw, fold);
    g.write::<f32>(x + 0xc4, yaw);
    let n = if i == u32::MAX {
        g.read::<u32>(w(0x177))
    } else {
        u32::from(g.read::<u8>(g.read::<u32>(w(0x17e)).wrapping_add(i)))
    };
    g.write::<u32>(x + 0x120, n);
    let k5 = g.read::<f32>(w(0x196));
    let r = x + 0x488;
    let d = [
        div(sub(lf(g, 0x30), lf(g, 0x20)), k5),
        div(sub(lf(g, 0x34), lf(g, 0x24)), k5),
        div(sub(lf(g, 0x38), lf(g, 0x28)), k5),
    ];
    for (o, v) in [
        (0x18u32, 0),
        (0x14, 0),
        (0x10, 0),
        (0x1c, 0),
        (0x28, 0),
        (0x2c, 0),
    ] {
        g.write::<u32>(r + o, v);
    }
    g.write::<u32>(r + 0x20, w(0x1e9));
    g.write::<u32>(r + 0x24, w(0x1f4));
    g.write::<u16>(r + 0x30, 0);
    g.write::<u32>(x + 0x110, r);
    for (k, o) in [(0u32, 0x1cu32), (1, 0x18), (2, 0x14)] {
        g.write::<f32>(p + o, d[k as usize]);
    }
    for k in 0..4 {
        let v = g.read::<u32>(v0 + 4 * k);
        g.write::<u32>(x + 0x4bc + 4 * k, v);
    }
    let r1 = st0(g.call(t(g, 314), 0, &[0x4220_0000]));
    let r2 = st0(g.call(t(g, 404), 0, &[0x41a0_0000]));
    let wv = p + 0x40;
    g.write::<f32>(wv, r1);
    g.write::<f32>(wv + 4, r2);
    g.write::<u32>(wv + 8, 0);
    g.write::<u32>(wv + 0xc, 0x3f80_0000);
    let dz = sub(lf(g, 0x38), lf(g, 0x28));
    let dx = sub(lf(g, 0x30), lf(g, 0x20));
    #[allow(clippy::cast_possible_truncation)] // `cvtpd2ps`
    let a = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let a = fold_angle(g, img, f, a, [0x2db, 0x2fa, 0x2ea, 0x304]);
    let m = p + 0x50;
    g.call(t(g, 235), 0, &[m]);
    g.call(t(g, 329), 0, &[m, m, a.to_bits()]);
    g.call(t(g, 229), 0, &[wv, m, wv]);
    g.write::<u32>(x + 0x4d8, 0x3f80_0000);
    for (k, dd) in [(0u32, 0x1cu32), (1, 0x18), (2, 0x14)] {
        let v = add(add(lf(g, 0x20 + 4 * k), lf(g, dd)), lf(g, 0x40 + 4 * k));
        g.write::<f32>(x + 0x4cc + 4 * k, v);
    }
    let i = g.read::<u32>(x + 0x504);
    let actor = entry_actor(g, img, f, (0x3ce, 0x3d5, 0x3e0), i);
    let h = st0(g.call(t(g, 12), 0, &[actor]));
    if g.read::<f32>(x + 0x4d0) > h {
        g.write::<f32>(x + 0x4d0, sub(h, g.read::<f32>(w(0x40d))));
    }
    let k7 = g.read::<f32>(w(0x41d));
    g.write::<u32>(x + 0x4e8, 0x3f80_0000);
    for (k, dd) in [(0u32, 0x1cu32), (1, 0x18), (2, 0x14)] {
        let v = sub(
            add(mul(lf(g, dd), k7), lf(g, 0x20 + 4 * k)),
            lf(g, 0x40 + 4 * k),
        );
        g.write::<f32>(x + 0x4dc + 4 * k, v);
    }
    for k in 0..4 {
        let v = g.read::<u32>(v1 + 4 * k);
        g.write::<u32>(x + 0x4ec + 4 * k, v);
    }
    g.write::<u32>(x + 0x4fc, 0);
    g.write::<u32>(x + 0x500, w(0x4d8));
    g.write::<u32>(x + 0x508, 0xc);
    g.write::<u32>(x + 0x510, 0x10);
    let owner = g.read::<u32>(g.read::<u32>(w(0x4f2)) + 0x10);
    g.write::<u32>(p.wrapping_sub(4), v0);
    let eax = crate::sb_misc::sprite_init_colour(g, p.wrapping_sub(8), x + 0x260, owner);
    g.write::<u32>(x + 0x484, 0x80);
    g.write::<u32>(x + 0x47c, 0x3f00_0000);
    g.write::<u32>(x + 0x46c, 0x4040_4040);
    Ret::eax(eax)
}

/// [`summon_object_between`] (the `rel32`s: the object start, `_CIatan2`
/// through its thunk, the sprite set-up, `__security_check_cookie`).
pub const SUMMON_OBJECT_BETWEEN: Sig = Sig {
    len: 1331,
    hash: 0xabcb_be7b_d127_fc6d,
    calls: &[
        (
            0xce,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0132_3CC0),
        ),
        (0x2c9, Callee::Any),
        (0x4fa, Callee::Sig(&crate::sb_misc::SPRITE_INIT_COLOUR)),
        (0x52b, Callee::Any),
    ],
    imports: &[],
};
