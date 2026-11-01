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

include!("o6_fam_pairs4_table.rs.in");
