//! Round 6 shape families: exact pairs, fourth file (two DLLs' copies of
//! one function that differ only in their globals and immediates;
//! families of `~/ffx-work/m5/magic/r4/families.tsv`). Each port reads
//! its operands from the copy's code (`w(off)` = the word at `+off`).

use crate::o4_parts::disp8;
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Six pulsing lights (no arguments; a /GS frame of 0x1c bytes, `ebp` =
/// `esp - 4` at the entry; `magic_0249` 0x10001f30 and `magic_0562`
/// 0x10001f40, 418 bytes, the same code; family `F0249_10001f30`): 0
/// while `*G_ON` is 0. Else for each light (records of 0x34 bytes from
/// `L`, the game's light records of 0x10 bytes from `B` up to `END`;
/// `+0x34`, `+0x3f`, `+0x15a`):
/// - fading out (`+8` set): the level `-8` grows by `K5`; above `K6` it
///   is 100.0, else (NaN too) the result is 0;
/// - else its counter `+0` counts down; at 0 a new pulse: the peak `-4` =
///   slot 404 `(20.0)` + `K3` (ST0), the length `rand() % 21 + 20` into
///   `+0` and `+4`, and `+8` = 1 when `*G_F` is 1; then the level `-8` =
///   `peak * (1 - u) * u * K2` (`u` = counter / length, the other terms
///   of the curve times 0.0 as the original adds them) and the result is
///   0;
/// - slot 857 `(*G_A, B, &c0, &c1, &c2, &c3)` with the colour `(-0x18,
///   -0x14, -0x10, level / K4)` on the frame (`ebp - 0x14..`).
///
/// Returns 1 when no light took the second or third way, else 0.
pub fn pulse_lights<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    if g.read::<u32>(w(0x12)) == 0 {
        return Ret::eax(0);
    }
    let a = g.read::<u32>(w(0x18));
    let mut result = 1u32;
    let (mut light, mut rec) = (w(0x3a), w(0x3f));
    loop {
        let level_at = light.wrapping_sub(8);
        if g.read::<u32>(light.wrapping_add(8)) != 0 {
            let x = add(g.read::<f32>(level_at), k(g, 0x180));
            g.write::<f32>(level_at, x);
            if x > k(g, 0x187) {
                g.write::<u32>(level_at, w(0x199));
            } else {
                result = 0;
            }
        } else {
            let count = g.read::<u32>(light).wrapping_sub(1);
            g.write::<u32>(light, count);
            if count == 0 {
                // push ecx; mov dword [esp], 20.0
                // call width: 1
                let out = g.call(slot_target(g, w(0x52), w(0x59) / 4), fr.ecx, &[w(0x60)]);
                let peak = add(f32::from_bits(crate::sa_summon::st0_f32(out)), k(g, 0x72));
                g.write::<f32>(light.wrapping_sub(4), peak);
                let r = g.rand().cast_signed() % w(0x86).cast_signed();
                let len = r
                    .wrapping_add(disp8(img, f + 0x91).cast_signed())
                    .cast_unsigned();
                g.write::<u32>(light.wrapping_add(4), len);
                g.write::<u32>(light, len);
                if g.read::<u32>(w(0x94)) == disp8(img, f + 0x98) {
                    g.write::<u32>(light.wrapping_add(8), w(0xa3));
                }
            }
            let u = div(
                cvt(g.read::<u32>(light)),
                cvt(g.read::<u32>(light.wrapping_add(4))),
            );
            let zero = 0.0f32;
            let rest = sub(k(g, 0xc1), u);
            let y = mul(mul(rest, u), k(g, 0xd8));
            let t0 = mul(mul(rest, rest), zero);
            let y = mul(y, g.read::<f32>(light.wrapping_sub(4)));
            let t2 = mul(mul(u, u), zero);
            g.write::<f32>(level_at, add(add(y, t0), t2));
            result = 0;
        }
        let colour = ebp.wrapping_sub(0x14);
        for (i, o) in (0u32..).zip([0x18u32, 0x14, 0x10]) {
            let v = g.read::<u32>(light.wrapping_sub(o));
            g.write::<u32>(colour.wrapping_add(4 * i), v);
        }
        let c3 = div(g.read::<f32>(level_at), k(g, 0x123));
        g.write::<f32>(colour.wrapping_add(12), c3);
        let words = [a, rec, colour, colour + 4, colour + 8, colour + 12];
        // call width: 6
        g.call(slot_target(g, w(0x139), w(0x144) / 4), fr.ecx, &words);
        rec = rec.wrapping_add(disp8(img, f + 0x151));
        light = light.wrapping_add(disp8(img, f + 0x157));
        if rec.cast_signed() >= w(0x15a).cast_signed() {
            return Ret::eax(result);
        }
    }
}

/// A scene's three passes of a model with the camera matrices (internal:
/// `ecx` = `a`, `edx` = `b`, one stack word `c`; a /GS frame of 0x108
/// bytes, `ebp` = `esp - 4` at the entry; `magic_0162` 0x10001a80 and
/// `magic_0586` 0x10001a20, 448 bytes; family `F0162_10001a80`): with
/// the scene `S` = `*G_S` and the frame's matrices `A` (`ebp - 0x84`),
/// `B` (`ebp - 0x104`), `C` (`ebp - 0x44`), `D` (`ebp - 0xc4`): slot 235
/// `(A)`, slot 554 `(B, A, S + 0xa00)`, slot 235 `(A)`, slot 554 `(C, A,
/// S + 0xa10)`, slot 230 `(D, P)`, `D`'s translation (`ebp - 0x94..`) 0,
/// slot 231 `(A, C, D)`, slot 230 `(C, A)`; then with the game lock
/// `**G_L` set to 1 before each, slot 828 `(a, b, c, 0x50, 0x80, 0x20,
/// B, C, pass)` for the passes 0, 1, 2; the lock gets its old value. Each
/// call sees the words left before it (passed up to 12). Returns `*G_L`.
pub fn three_pass_model<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (a, b, c) = (fr.ecx, fr.edx, fr.arg(g, 0));
    let ebp = fr.esp.wrapping_sub(4);
    let (ma, mb, mc, md) = (
        ebp.wrapping_sub(0x84),
        ebp.wrapping_sub(0x104),
        ebp.wrapping_sub(0x44),
        ebp.wrapping_sub(0xc4),
    );
    let scene = g.read::<u32>(w(0x17));
    g.write::<u32>(ebp.wrapping_sub(0x108), b);
    let mut s = Pushes::default();
    s.push(ma);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x27), w(0x33) / 4), fr.ecx);
    s.push_all(&[scene.wrapping_add(w(0x3d)), ma, mb]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x51), w(0x57) / 4), fr.ecx);
    s.push(ma);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x65), w(0x6b) / 4), fr.ecx);
    s.push_all(&[scene.wrapping_add(w(0x73)), ma, mc]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x84), w(0x8a) / 4), fr.ecx);
    s.push_all(&[w(0x97), md]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x9d), w(0xa3) / 4), fr.ecx);
    s.push_all(&[md, mc, ma]);
    for (d, v) in [(0xc2u32, 0xc6u32), (0xcc, 0xd0), (0xd6, 0xda)] {
        g.write::<u32>(ebp.wrapping_add(w(d)), w(v));
    }
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xbc), w(0xe0) / 4), fr.ecx);
    s.push_all(&[ma, mc]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xf2), w(0xf8) / 4), fr.ecx);
    let lock = g.read::<u32>(w(0xff));
    let saved = g.read::<u32>(lock);
    // (the lock global, the `mov [eax], 1` value, `push pass`, `mov eax,
    // [T]`, `push 0x20`, the slot load, `push 0x80`, `push 0x50`)
    let passes = [
        (0xff, 0x109, 0x104, 0x119, 0x11e, 0x121, 0x126, 0x12b),
        (0x137, 0x140, 0x145, 0x152, 0x157, 0x15a, 0x15f, 0x164),
        (0x170, 0x178, 0x175, 0x188, 0x18d, 0x190, 0x195, 0x19a),
    ];
    for (k, &(lk, one, pass, t, k20, slot, k80, k50)) in passes.iter().enumerate() {
        let lock = g.read::<u32>(w(lk));
        g.write::<u32>(lock, w(one));
        if k == 1 {
            // add esp, 0x60: the first pass's words and all before go
            s.pop(24);
        }
        s.push_all(&[
            disp8(img, f + pass),
            mc,
            mb,
            disp8(img, f + k20),
            w(k80),
            disp8(img, f + k50),
            c,
            b,
            a,
        ]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(t), w(slot) / 4), fr.ecx);
    }
    let lock = g.read::<u32>(w(0x1a6));
    g.write::<u32>(lock, saved);
    Ret::eax(lock)
}

/// A battle effect's end (no arguments; a /GS frame of 0x14 bytes, `ebp`
/// = `esp - 4` at the entry; `magic_0434` 0x100095e0 and `magic_0610`
/// 0x1000d4b0, 327 bytes; family `F0434_100095e0`): with the scene `S` =
/// `*G_S`, its node `S + 0x1a64` goes back to the pool
/// ([`crate::sa_pool::give_back`]) and is cleared; unless `*G_B` is set,
/// an actor `i = *G_I` below 0x20 gets slot 217 `(i, &L)` and slot 255
/// `(i, &L)` (`L` on the frame at `ebp - 0x14`); slot 211 `(*G_C, 0)`;
/// slot 213 `(*G_V, 0)` while the u16 `*G_W` is 0; `*G_X` = 1; slot 296
/// `(i, 1)` for such an actor, else slot 257 `(*G_C, 0)`; the others
/// shown ([`crate::sb_vm::show_others`]) and the party's list run
/// ([`crate::sa_script::each_actor`]), both with 1; slot 334 `(0)`; the
/// node `*G_N` back to the pool and cleared too; `S + 0x1a88` = 1.
/// Returns the last call's `eax` (the cookie check leaves it).
pub fn battle_effect_end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    // sub esp, 0x14; push esi; the call's return address
    let callee_esp = ebp.wrapping_sub(0x14 + 4 + 4);
    let scene = g.read::<u32>(w(0x13));
    let node = g.read::<u32>(scene.wrapping_add(w(0x19)));
    if node != 0 {
        crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x22), node);
        g.write::<u32>(scene.wrapping_add(w(0x28)), w(0x2c));
    }
    let busy = g.read::<u32>(w(0x32)) != 0;
    let c = g.read::<u32>(w(0x3a));
    if !busy {
        let i = g.read::<u32>(w(0x43));
        if i.cast_signed() < disp8(img, f + 0x49).cast_signed() {
            let l = ebp.wrapping_sub(0x14);
            let mut stack = vec![i, l];
            // call width: 2
            g.call(slot_target(g, w(0x51), w(0x58) / 4), fr.ecx, &stack);
            stack.splice(0..0, [i, l]);
            // call width: 4 (the earlier call's words stay on the stack)
            g.call(slot_target(g, w(0x63), w(0x6a) / 4), fr.ecx, &stack);
        }
    }
    // call width: 2
    g.call(
        slot_target(g, w(0x75), w(0x7d) / 4),
        fr.ecx,
        &[c, disp8(img, f + 0x7a)],
    );
    if g.read::<u16>(w(0x8a)) == 0 {
        let v = g.read::<u32>(w(0x9b));
        // call width: 2
        g.call(
            slot_target(g, w(0x93), w(0xa1) / 4),
            fr.ecx,
            &[v, disp8(img, f + 0x98)],
        );
    }
    let busy = g.read::<u32>(w(0xac)) != 0;
    g.write::<u32>(w(0xb3), w(0xb7));
    let i = g.read::<u32>(w(0xbe));
    if !busy && i.cast_signed() < disp8(img, f + 0xc4).cast_signed() {
        // call width: 2
        g.call(
            slot_target(g, w(0xcb), w(0xd1) / 4),
            fr.ecx,
            &[i, disp8(img, f + 0xc8)],
        );
    } else {
        let c = g.read::<u32>(w(0xe0));
        // call width: 2
        g.call(
            slot_target(g, w(0xd8), w(0xe6) / 4),
            fr.ecx,
            &[c, disp8(img, f + 0xdd)],
        );
    }
    crate::sb_vm::show_others(g, img, img.rel32_target(f + 0xf5), callee_esp, w(0xf0));
    crate::sa_script::each_actor(g, img, img.rel32_target(f + 0xff), w(0xfa), callee_esp);
    // call width: 1
    let mut eax = g
        .call(
            slot_target(g, w(0x104), w(0x10c) / 4),
            fr.ecx,
            &[disp8(img, f + 0x109)],
        )
        .eax;
    let node = g.read::<u32>(w(0x114));
    if node != 0 {
        eax = crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x120), node);
        g.write::<u32>(w(0x126), w(0x12a));
    }
    g.write::<u32>(scene.wrapping_add(w(0x133)), w(0x137));
    Ret::eax(eax)
}

/// Takes the free ring's head `node` (count `free` > 0) as the original
/// inlines it: unlinked (each link re-read), the new head is `node`'s
/// next (0 when it was the last), its links, `+0xc`, `+0x14` cleared,
/// `+8` itself, `+0x10` the owner, byte `+0x18` 0 (the caller clears it).
/// Returns the new head
/// and count; the caller stores them.
fn ring_take<G: Game>(g: &mut G, node: u32, free: u16) -> (u32, u16) {
    let prev = g.read::<u32>(node);
    let next = g.read::<u32>(node.wrapping_add(4));
    g.write::<u32>(prev.wrapping_add(4), next);
    let next = g.read::<u32>(node.wrapping_add(4));
    let prev = g.read::<u32>(node);
    g.write::<u32>(next, prev);
    let head = if free > 1 {
        g.read::<u32>(node.wrapping_add(4))
    } else {
        0
    };
    (head, free.wrapping_sub(1))
}

/// A camera block's reset with two list nodes (no arguments; `magic_0307`
/// 0x10006bc0 and `magic_0605` 0x1000a100, 653 bytes; family
/// `F0307_10006bc0`): with `B` = `*G_B`, `B + 0x1e34` = 0; the angle
/// `B[0xc24] - K1` brought into range once (`x > K2`: `x - K3`; else
/// `K4 > x`: `x + K3`) into `B + 0x1e38`, `+0x1e3c..+0x1e50` 0, `+0x1e64`
/// = 0x5b; the 0x12 records of 0x60 bytes from `B + 0x16d4` get their
/// first 20 words cleared, `B + 0x1e68` = 0; then two nodes from the free
/// ring (count `G_FREE` u16, head `G_HEAD`; callbacks `CB1`, `CB2`): the
/// first appended to the list (`G_FIRST`, `G_LAST`, which it does not
/// move) and kept in `B + 0x2864`, the second after it (it becomes the
/// last). An empty ring gives node 0, which the original writes through
/// (it faults; so does the port). Returns 0.
pub fn camera_reset_nodes<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let b = g.read::<u32>(w(0x05));
    let at = |o: u32| b.wrapping_add(w(o));
    g.write::<u32>(at(0x0b), w(0x0f));
    let mut x = sub(g.read::<f32>(at(0x17)), k(g, 0x1f));
    if x > k(g, 0x26) {
        x = sub(x, k(g, 0x30));
    } else if k(g, 0x3a) > x {
        x = add(x, k(g, 0x47));
    }
    g.write::<f32>(at(0x4f), x);
    for (d, v) in [
        (0x55u32, 0x59u32),
        (0x5f, 0x63),
        (0x69, 0x6d),
        (0x73, 0x77),
        (0x7d, 0x81),
        (0x87, 0x8b),
        (0x91, 0x95),
    ] {
        g.write::<u32>(at(d), w(v));
    }
    let mut rec = at(0x9b);
    for _ in 0..w(0xa0) {
        for i in 0..20u32 {
            g.write::<u32>(rec.wrapping_sub(4).wrapping_add(4 * i), 0);
        }
        rec = rec.wrapping_add(disp8(img, f + 0x13d));
    }
    g.write::<u32>(at(0x14e), 0);
    let clear = |g: &mut G, node: u32, owner: u32| {
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node.wrapping_add(o), 0);
        }
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(node.wrapping_add(0x10), owner);
        g.write::<u8>(node.wrapping_add(0x18), 0);
    };
    let mut free = g.read::<u16>(w(0x148));
    let (first_node, mut head) = if free == 0 {
        (0, g.read::<u32>(w(0x159)))
    } else {
        let node = g.read::<u32>(w(0x163));
        let (head, left) = ring_take(g, node, free);
        g.write::<u32>(w(0x18d), head);
        clear(g, node, w(0x1b2));
        free = left;
        g.write::<u16>(w(0x1bd), free);
        (node, head)
    };
    let empty = g.read::<u32>(w(0x1c3)) == 0;
    g.write::<u32>(first_node.wrapping_add(4), 0);
    if empty {
        g.write::<u32>(first_node, 0);
        g.write::<u32>(w(0x1e5), first_node);
    } else {
        let last = g.read::<u32>(w(0x1d2));
        g.write::<u32>(first_node, last);
        g.write::<u32>(last.wrapping_add(4), first_node);
    }
    g.write::<u32>(first_node.wrapping_add(8), first_node);
    g.write::<u32>(b.wrapping_add(w(0x1ee)), first_node);
    if free == 0 {
        head = 0;
    } else {
        let (next, left) = ring_take(g, head, free);
        g.write::<u32>(if free > 1 { w(0x214) } else { w(0x21c) }, next);
        clear(g, head, w(0x24d));
        g.write::<u16>(w(0x258), left);
    }
    let second = head;
    g.write::<u32>(second.wrapping_add(0x14), 0);
    g.write::<u32>(second.wrapping_add(4), 0);
    g.write::<u32>(w(0x266), second);
    if g.read::<u32>(w(0x26c)) != 0 {
        g.write::<u32>(second, first_node);
        g.write::<u32>(first_node.wrapping_add(4), second);
    } else {
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x284), second);
    }
    g.write::<u32>(second.wrapping_add(8), second);
    Ret::eax(0)
}

/// The float of slot `slot` (through the table global `t`) for `x`
/// (ST0 as `f32`, its bits).
fn slot_float<G: Game>(g: &mut G, t: u32, slot: u32, x: u32, ecx: u32) -> (u32, u32) {
    // call width: 1
    let out = g.call(slot_target(g, t, slot), ecx, &[x]);
    (crate::sa_summon::st0_f32(out), out.eax)
}

/// A spark task's frame (cdecl `(task)`; a /GS frame of 0x14 bytes,
/// `ebp` = `esp - 4` at the entry; `magic_0249` 0x10001b20 and
/// `magic_0562` 0x10001b30, 591 bytes, the same code; family
/// `F0249_10001b20`): while the u16 delay `task + 0x14` runs it counts
/// down (returns the new value); else its life `+0x16` counts down, and
/// at 0 the task goes back to the pool ([`crate::sa_pool::give_back`]).
/// Otherwise four sparks from the ring ([`crate::sa_misc::ring_take`];
/// none left: return 0): each gets slot 719 `(P, 9, 0)` (the first) or
/// `(P, 9, 1)`, u16 `+8` 0xffff, `+0xa` 0x200, `+4` 1, its position from
/// slot 237 `(*G_A, 0x45, V, spark + 0xc)` with `V` on the frame (`ebp -
/// 0x14`: 300, then 0 0 for the first, slot 314 `(500)` twice for the
/// others, and 1.0), `+0x18` 1.0, `+0x1c..+0x24` (0 for the first, slot
/// 314 `(0.3)` each for the others), `+0x28` slot 314 `(pi)`, `+0x2c`
/// (0.5, then 0.1), `+0x34` 0x20, `+0x38` 0, byte `+0x47` 0x40. The slot
/// calls see the one word the earlier calls leave. Returns the last
/// call's `eax`.
pub fn spark_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let task = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let delay = g.read::<u16>(task.wrapping_add(0x14));
    if delay != 0 {
        let d = u32::from(delay).wrapping_sub(1);
        g.write::<u16>(task.wrapping_add(0x14), d as u16);
        return Ret::eax(d);
    }
    let life = g.read::<u16>(task.wrapping_add(0x16)).wrapping_sub(1);
    g.write::<u16>(task.wrapping_add(0x16), life);
    if life == 0 {
        return Ret::eax(crate::sa_pool::give_back(
            g,
            img,
            img.rel32_target(f + 0x36),
            task,
        ));
    }
    let take = img.rel32_target(f + 0x4a);
    let v = ebp.wrapping_sub(0x14);
    let t = w(0x93);
    let s314 = w(0xe9) / 4;
    let a = w(0x9b);
    let start =
        |g: &mut G, spark: u32, (tp, sp, push_p, push_9, push_k): (u32, u32, u32, u32, u32)| {
            let tt = g.read::<u32>(w(tp));
            let target = g.read::<u32>(tt.wrapping_add(w(sp)));
            let words = [w(push_p), disp8(img, f + push_9), disp8(img, f + push_k)];
            // call width: 3
            // `call ecx`: ecx holds the target
            let h = g.call(target, target, &words).eax;
            g.write::<u32>(spark, h);
            g.write::<u16>(spark.wrapping_add(8), 0xffff);
            g.write::<u16>(spark.wrapping_add(0xa), 0x200);
        };
    let place = |g: &mut G, spark: u32| {
        let words = [
            g.read::<u32>(a),
            disp8(img, f + 0x98),
            v,
            spark.wrapping_add(0xc),
        ];
        // call width: 4
        g.call(slot_target(g, t, w(0xbd) / 4), fr.ecx, &words);
    };
    // the first spark
    let spark = crate::sa_misc::ring_take(g, img, take);
    if spark == 0 {
        return Ret::eax(0);
    }
    start(g, spark, (0x5a, 0x61, 0x6b, 0x69, 0x67));
    g.write::<u32>(spark.wrapping_add(4), w(0x8a));
    for (i, at) in (0u32..).zip([0xa2u32, 0xa9, 0xb0, 0xb7]) {
        g.write::<u32>(v.wrapping_add(4 * i), w(at));
    }
    place(g, spark);
    g.write::<u32>(spark.wrapping_add(0x18), w(0xcb));
    for o in [0x24u32, 0x20, 0x1c] {
        g.write::<u32>(spark.wrapping_add(o), 0);
    }
    let (pi, mut eax) = slot_float(g, t, s314, w(0xf0), fr.ecx);
    g.write::<u32>(spark.wrapping_add(0x28), pi);
    for (o, at) in [(0x2cu32, 0xffu32), (0x34, 0x106), (0x38, 0x10d)] {
        g.write::<u32>(spark.wrapping_add(o), w(at));
    }
    g.write::<u8>(spark.wrapping_add(0x47), w(0x114) as u8);
    for _ in 0..disp8(img, f + 0x237) {
        let spark = crate::sa_misc::ring_take(g, img, img.rel32_target(f + 0x118));
        if spark == 0 {
            return Ret::eax(0);
        }
        start(g, spark, (0x128, 0x130, 0x137, 0x135, 0x12d));
        g.write::<u32>(spark.wrapping_add(4), w(0x157));
        g.write::<u32>(v, w(0x161));
        let (y, _) = slot_float(g, t, s314, w(0x16e), fr.ecx);
        g.write::<u32>(v.wrapping_add(4), y);
        let (z, _) = slot_float(g, t, s314, w(0x17f), fr.ecx);
        g.write::<u32>(v.wrapping_add(8), z);
        g.write::<u32>(v.wrapping_add(12), w(0x1a6));
        place(g, spark);
        g.write::<u32>(spark.wrapping_add(0x18), w(0x1bd));
        for (o, at) in [
            (0x1cu32, 0x1ca),
            (0x20, 0x1db),
            (0x24, 0x1f2),
            (0x28, 0x209),
        ] {
            let (x, e) = slot_float(g, t, s314, w(at), fr.ecx);
            g.write::<u32>(spark.wrapping_add(o), x);
            eax = e;
        }
        for (o, at) in [(0x2cu32, 0x21f), (0x34, 0x226), (0x38, 0x22d)] {
            g.write::<u32>(spark.wrapping_add(o), w(at));
        }
        g.write::<u8>(spark.wrapping_add(0x47), w(0x234) as u8);
    }
    Ret::eax(eax)
}

include!("o6_fam_pairs4_table.rs.in");
