//! Shared functions of the shared-5 list (rows 737 on of the shared-4
//! work list) without a family module yet. Each doc names the DLL and
//! address it was ported from, its size and how many DLLs carry it;
//! "`G`" is a global of the DLL (an operand of the code).

use crate::l5_objhost::{LIST_REMOVE, OBJ_BLOCK_INIT, TASKS_INIT, list_remove};
use crate::m0052::{PLACE, place};
use crate::m0596::{host_start, host_start_head};
use crate::ob_model::wrap_once;
use crate::opu_parts::{START_PART, start_part};
use crate::pushes::Pushes;
use crate::runtime::{slot_of_thunk, slot_target};
use crate::s4_fam446::{START_CHILD_B, start_child_b};
use crate::s4_misc::{SCALE_TASK, START_OBJECT_AT, scale_task, start_object_at};
use crate::s4_summon::{TARGETS_INIT, targets_init, trunc};
use crate::sa_misc::{TIMELINE, timeline};
use crate::sa_pool::{GIVE_BACK, TAKE, give_back, take};
use crate::sa_script::{ACTOR_MODE, actor_mode};
use crate::sb_misc::{TIMELINE_STOP_GLOBAL, timeline_stop_global};
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Out, Ret, Sig, Xmm};

/// The word at `f + off` of the code (an operand).
fn op(img: &Image, f: u32, off: u32) -> u32 {
    img.word(f + off)
}

/// Slot `slot` through the table pointer at operand `table`, with `args`.
fn slot_call<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (table, slot): (u32, u32),
    args: &[u32],
) -> Out {
    let t = slot_target(g, op(img, f, table), slot);
    g.call(t, 0, args)
}

/// The per-frame function of a "wot" effect (cdecl, no arguments). While
/// `*G_ON` is 0 it does nothing and returns 0. Otherwise: the pad history
/// (slot 302 `(0, 0, 0)`, its low half to `G_PAD`), `*G_M = G_REC`
/// (an immediate address), the screen and camera matrices copied (slots
/// 723 `(*G_S)` and 724 `(*G_C)`) and `**G_T = *G_V`. Effect flag 0
/// (slot 261) starts the "wot" once (`*G_ONCE`): `G_W = 0`, `G_ID` =
/// slot 377 `(callback, 0, 0, 0)` (the callback an immediate), slot 381
/// `(1)`. Effect flag 1 frees it (slot 376 `(G_ID)`, slot 381 `(0)`),
/// stops the timeline ([`timeline_stop_global`]) and ends the effect
/// (slot 214 `(*G_E)`); without it `G_WAIT` counts up. Returns 1.
///
/// `magic_0662` 0x100016c0 (296 bytes; 2 DLLs).
pub fn wot_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    if g.read::<u32>(w(0x2)) == 0 {
        return Ret::eax(0);
    }
    let pad = slot_call(g, img, f, (0xd, 302), &[0, 0, 0]).eax;
    g.write::<u32>(w(0x29), pad & 0xffff);
    let screen = g.read::<u32>(w(0x21));
    let m = g.read::<u32>(w(0x2e));
    g.write::<u32>(m, w(0x34));
    slot_call(g, img, f, (0x39, 723), &[screen, 0, 0, 0]);
    let camera = g.read::<u32>(w(0x4c));
    slot_call(g, img, f, (0x46, 724), &[camera, screen, 0, 0, 0]);
    let t = g.read::<u32>(w(0x5a));
    let v = g.read::<u32>(w(0x5f));
    g.write::<u32>(t, v);
    let flag0 = slot_call(g, img, f, (0x66, 261), &[0, camera, screen, 0, 0, 0]).eax;
    if flag0 != 0 && g.read::<u32>(w(0x7d)) == 0 {
        g.write::<u32>(w(0x9c), 0);
        let id = slot_call(g, img, f, (0x85, 377), &[w(0x96), 0, 0, 0]).eax;
        g.write::<u32>(w(0xa7), id);
        slot_call(g, img, f, (0xac, 381), &[1, w(0x96), 0, 0, 0]);
        let n = g.read::<u32>(w(0xbf));
        g.write::<u32>(w(0xbf), n.wrapping_add(1));
    }
    if slot_call(g, img, f, (0xc4, 261), &[1]).eax == 0 {
        let n = g.read::<u32>(w(0x11e));
        g.write::<u32>(w(0x11e), n.wrapping_add(1));
        return Ret::eax(1);
    }
    let id = g.read::<u32>(w(0xe0));
    slot_call(g, img, f, (0xda, 376), &[id]);
    slot_call(g, img, f, (0xed, 381), &[0, id]);
    timeline_stop_global(g, img, img.rel32_target(f + 0xfc));
    let e = g.read::<u32>(w(0x107));
    slot_call(g, img, f, (0x101, 214), &[e, 0, id]);
    Ret::eax(1)
}

/// [`wot_frame`].
pub const WOT_FRAME: Sig = Sig {
    len: 296,
    hash: 0xa5cb_15ab_b2fa_cb24,
    calls: &[(0xfc, Callee::Sig(&TIMELINE_STOP_GLOBAL))],
    imports: &[],
};

/// An object's camera-facing scale (cdecl `(obj)`, a /GS frame): the
/// screen and camera matrices into locals of the original's frame
/// (`ebp - 0x94`, `ebp - 0x54`; slots 723 and 724), slot 327 `(obj +
/// 0x80, camera)`, the point `(0, 0, 5, 1)` (local `ebp - 0x14`) through
/// it into `obj + 0xd0` (slot 229), `obj + 0xdc = 1.0`, the camera local
/// made a unit matrix (slot 235), turned about z by `K_ROT` (an
/// immediate; slot 330) and multiplied into `obj + 0x80` (slot 231).
/// The four scale words `obj + 0x60..0x6c` become `K / depth * K2`,
/// `depth` the screen depth (slot 282, ST0 stored as `f32`). Returns slot
/// 282's `eax`.
///
/// `magic_0434` 0x10007e90 (294 bytes; 2 DLLs).
pub fn facing_scale<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let screen = ebp.wrapping_sub(0x94);
    let camera = ebp.wrapping_sub(0x54);
    let point = ebp.wrapping_sub(0x14);
    let m = obj.wrapping_add(0x80);
    slot_call(g, img, f, (0x20, 723), &[screen]);
    slot_call(g, img, f, (0x31, 724), &[camera, screen]);
    slot_call(g, img, f, (0x42, 327), &[m, camera, camera, screen]);
    for (i, v) in (0u32..).zip([0u32, 0, 0x40a0_0000, 0x3f80_0000]) {
        g.write::<u32>(point + 4 * i, v);
    }
    let out = obj.wrapping_add(0xd0);
    slot_call(
        g,
        img,
        f,
        (0x62, 229),
        &[out, m, point, m, camera, camera, screen],
    );
    g.write::<u32>(obj + 0xdc, 0x3f80_0000);
    slot_call(
        g,
        img,
        f,
        (0x8f, 235),
        &[camera, out, m, point, m, camera, camera, screen],
    );
    let rot = 0x3f61_307a;
    slot_call(g, img, f, (0xb5, 330), &[camera, camera, rot]);
    slot_call(g, img, f, (0xc6, 231), &[m, m, camera, camera, camera, rot]);
    let depth = slot_call(g, img, f, (0xd5, 282), &[]);
    let d = depth.st0.map_or(f32::from_bits(0xffc0_0000), |v| v as f32);
    let k = g.read::<f32>(w(0xee));
    let k2 = g.read::<f32>(w(0x101));
    let s = mul(div(k, d), k2);
    for o in [0x6cu32, 0x68, 0x64, 0x60] {
        g.write::<f32>(obj + o, s);
    }
    Ret::eax(depth.eax)
}

/// [`facing_scale`] (the `rel32`: `__security_check_cookie`).
pub const FACING_SCALE: Sig = Sig {
    len: 294,
    hash: 0xeb47_50a2_adfe_e7fe,
    calls: &[(0x11e, Callee::Any)],
    imports: &[],
};

/// A motion started towards a point (`ecx` = `[obj, motion, to, len]`,
/// an aligned /GS frame). Nothing when bit 0 of the object's u16 `+0x90`
/// is set: the original then returns the frame's cookie word (`cookie ^
/// esp`). Otherwise the motion goes on the model (slot 228 `(obj+0x50,
/// motion)`), its result to `+0x5c` and as a float to `+0x70`; the
/// model's position into `obj + 0x1c` (slot 217), `d = to - pos` (slot
/// 234, a local of the aligned frame), `r = |d|`, `d` scaled by `(r -
/// len) / r` (slot 233) and added to the position into `obj + 0x2c` (slot
/// 248); flag bit 3 cleared, bit 2 set. Returns slot 248's `eax` with the
/// flags in its low half.
///
/// `magic_0201` 0x100025b0 (296 bytes, `ecx` = the record; 2 DLLs).
pub fn motion_towards<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let this = fr.ecx;
    // push ebp; and esp, -8; sub esp, 0x14
    let top = fr.esp.wrapping_sub(4) & !7;
    let d = top.wrapping_sub(0x14);
    let obj = g.read::<u32>(this);
    if g.read::<u16>(obj + 0x90) & 1 != 0 {
        return Ret::eax(g.read::<u32>(w(0xa)) ^ d);
    }
    let model = g.read::<u32>(obj + 0x50);
    let motion = g.read::<u32>(this + 4);
    let m = slot_call(g, img, f, (0x2c, 228), &[model, motion]).eax;
    g.write::<u32>(obj + 0x5c, m);
    // cvtdq2pd, plus 2^32 (the table's second double) when negative
    let bias = g.read::<f64>(w(0x51) + 8 * (m >> 31));
    g.write::<f32>(obj + 0x70, (f64::from(m.cast_signed()) + bias) as f32);
    let pos = obj.wrapping_add(0x1c);
    let model = g.read::<u32>(obj + 0x50);
    slot_call(g, img, f, (0x56, 217), &[model, pos, model, motion]);
    let to = g.read::<u32>(this + 8);
    slot_call(
        g,
        img,
        f,
        (0x79, 234),
        &[d, to, pos, model, pos, model, motion],
    );
    let [x, y, z] = [0u32, 4, 8].map(|o| g.read::<f32>(d + o));
    let s = add(add(mul(y, y), mul(x, x)), mul(z, z));
    let r = f64::from(s).sqrt() as f32;
    let k = div(sub(r, g.read::<f32>(this + 0xc)), r);
    slot_call(g, img, f, (0xd3, 233), &[d, d, k.to_bits()]);
    let out = obj.wrapping_add(0x2c);
    let e = slot_call(g, img, f, (0xed, 248), &[out, pos, d]).eax;
    let flags = (g.read::<u16>(obj + 0x90) & 0xfff7) | 4;
    g.write::<u16>(obj + 0x90, flags);
    Ret::eax((e & 0xffff_0000) | u32::from(flags))
}

/// [`motion_towards`] (the `rel32`s: the square root and
/// `__security_check_cookie`).
pub const MOTION_TOWARDS: Sig = Sig {
    len: 296,
    hash: 0x5dda_50ee_b459_dea9,
    calls: &[
        (0xb2, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x120, Callee::Any),
    ],
    imports: &[],
};

/// The effect's character set up (cdecl, no arguments; the character
/// number 0x3015 is an immediate). `*G_WAIT = 0`; when `*G_CH` is 0 the
/// character is read (slot 26 `(0x3015, 1)`), made (slot 0 `(0x3015)`,
/// into `*G_CH`) and set up: motion type 1 (slot 387), field mode 1
/// (slot 385), gravity mode 0 (slot 386), slot 922 `(ch[0x830], 0)`.
/// Then the record at `G_R`: its qword at `+0` cleared, `+4` = the
/// character, `+8` cleared, `+0` = `G_TABLE` (an immediate address); the
/// colour `G_C = (0.75, 0.75, 0.75, 1)`, the vector `G_V` cleared, and
/// slot 268 `(*G_O, 0x10, G_OUT)`. Returns 1.
///
/// `magic_0188` 0x100026d0 (300 bytes; 2 DLLs).
pub fn chara_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let mut ch = g.read::<u32>(w(0x1));
    g.write::<u32>(w(0x7), 0);
    if ch == 0 {
        let n = 0x3015;
        slot_call(g, img, f, (0x18, 26), &[n, 1]);
        ch = slot_call(g, img, f, (0x29, 0), &[n, n, 1]).eax;
        g.write::<u32>(w(0x37), ch);
        slot_call(g, img, f, (0x3f, 387), &[ch, 1, n, n, 1]);
        let c = g.read::<u32>(w(0x54));
        slot_call(g, img, f, (0x4c, 385), &[c, 1, ch, 1, n, n, 1]);
        let c2 = g.read::<u32>(w(0x69));
        slot_call(g, img, f, (0x61, 386), &[c2, 0, c, 1, ch, 1, n, n, 1]);
        let c3 = g.read::<u32>(w(0x76));
        let x = g.read::<u32>(c3.wrapping_add(0x830));
        slot_call(g, img, f, (0x83, 922), &[x, 0, c2, 0, c, 1, ch, 1, n, n, 1]);
        ch = g.read::<u32>(w(0x90));
    }
    g.write::<u64>(w(0x9e), 0);
    g.write::<u32>(w(0xa3), ch);
    let o = g.read::<u32>(w(0xb5));
    for (off, v) in [
        (0xbbu32, 0x3f40_0000u32),
        (0xc5, 0x3f40_0000),
        (0xcf, 0x3f40_0000),
        (0xd9, 0x3f80_0000),
        (0xe3, 0),
        (0xed, 0),
        (0xf7, 0),
        (0x101, 0),
    ] {
        g.write::<u32>(w(off), v);
    }
    g.write::<u64>(w(0x10d), 0);
    g.write::<u32>(w(0x113), w(0x117));
    slot_call(g, img, f, (0xa8, 268), &[o, 0x10, w(0xad)]);
    Ret::eax(1)
}

/// [`chara_setup`].
pub const CHARA_SETUP: Sig = Sig {
    len: 300,
    hash: 0xcbee_0f3c_eeb5_9064,
    calls: &[],
    imports: &[],
};

/// The targets' records started (cdecl `(recs)`, records of 0xa0 bytes):
/// `*G_N = *G_COUNT`, `*G_ON = 1`, and for each of the `*G_N` records:
/// the target number `ids[i]` (bytes at `*G_IDS`) at `+0`, slot 367
/// `(*G_H, id, &st)` (`st` the frame's one local, holding the entry's
/// `ecx` until the slot writes it), bit 0 of `st` at `+4`, `+8 = 0x23`,
/// the character's handle (slot 215 `(id)`) and its point 0x10 into `+0xc`
/// (slot 268), `*G_P = ITEM` (an immediate), the part of slot 941 `(*G_D +
/// 0x10)`'s `+4` started at `+0x20` ([`start_part`], `0x1000`, 1), `+0x60`
/// a copy of `+0xc` (slot 325), `+0x70..+0x7c = (0, 0, 0, 1)` and
/// `+0x80..+0x8c = (0.1875, 0.1875, 0.1875, 1.0)`. Returns 1.
///
/// `magic_0443` 0x10001330 (308 bytes; 2 DLLs).
pub fn targets_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    // push ecx: the local starts as the entry's ecx
    let st = fr.esp.wrapping_sub(8);
    g.write::<u32>(st, fr.ecx);
    let n = g.read::<u32>(w(0x5));
    g.write::<u32>(w(0xd), n);
    g.write::<u32>(w(0x13), 1);
    if n.cast_signed() <= 0 {
        return Ret::eax(1);
    }
    let mut rec = fr.arg(g, 0);
    let mut i = 0u32;
    loop {
        let id = u32::from(g.read::<u8>(g.read::<u32>(w(0x31)).wrapping_add(i)));
        let h = g.read::<u32>(w(0x4a));
        let mut s = Pushes::default();
        s.push_all(&[0, 0, 0, 0, 0, st, id, h]);
        g.write::<u32>(rec, id);
        // call width: 12 (the most `Pushes::words` holds)
        s.call(g, slot_target(g, w(0x52), 367), fr.ecx);
        let x = g.read::<u32>(st);
        let id = g.read::<u32>(rec);
        s.push(id);
        g.write::<u32>(rec + 4, x & 1);
        g.write::<u32>(rec + 8, 0x23);
        // call width: 12 (the most `Pushes::words` holds)
        let ch = s.call(g, slot_target(g, w(0x6b), 215), fr.ecx).eax;
        let pos = rec.wrapping_add(0xc);
        s.push_all(&[pos, 0x10, ch]);
        // call width: 12 (the most `Pushes::words` holds)
        s.call(g, slot_target(g, w(0x86), 268), fr.ecx);
        let p = g.read::<u32>(w(0x93));
        g.write::<u32>(p, w(0x99));
        let d = g.read::<u32>(g.read::<u32>(w(0x9e)).wrapping_add(0x10));
        s.push(d);
        // call width: 12 (the most `Pushes::words` holds)
        let table = s.call(g, slot_target(g, w(0xa6), 941), fr.ecx).eax;
        let program = g.read::<u32>(table.wrapping_add(4));
        start_part(
            g,
            img,
            img.rel32_target(f + 0xc2),
            rec.wrapping_add(0x20),
            0x1000,
            program,
            1,
        );
        let t = slot_target(g, w(0xcf), 325);
        g.call(t, 0, &[rec.wrapping_add(0x60), pos]);
        i += 1;
        for (o, v) in [
            (0x70u32, 0u32),
            (0x74, 0),
            (0x78, 0),
            (0x7c, 1),
            (0x80, 0x3e40_0000),
            (0x84, 0x3e40_0000),
            (0x88, 0x3e40_0000),
            (0x8c, 0x3f80_0000),
        ] {
            g.write::<u32>(rec + o, v);
        }
        rec = rec.wrapping_add(0xa0);
        if i.cast_signed() >= g.read::<i32>(w(0x11e)) {
            return Ret::eax(1);
        }
    }
}

/// [`targets_start`] (the `rel32`: [`start_part`]).
pub const TARGETS_START: Sig = Sig {
    len: 308,
    hash: 0xee0d_f5c5_cb6c_02fb,
    calls: &[(0xc2, Callee::Sig(&START_PART))],
    imports: &[],
};

/// `fstp dword` of a slot's ST0 (the indefinite when it left none).
fn st0_f32(out: Out) -> f32 {
    out.st0.map_or(f32::from_bits(0xffc0_0000), |v| v as f32)
}

/// The particles thrown again (`ecx` = `[obj, a, b]`): the object's byte
/// `+0x1650` gets bit 1 cleared and bit 2 set; each of its `+0x163c`
/// particles (0x278 bytes from `+0x13c`) draws a random word (slot 324),
/// takes its owner's battle position when bit 0 of its flags is clear
/// (slot 531 `(p[-0x40], p + 0x19c)`), gets `+0x1a8 = 1.0`, its `+0x1a0`
/// lowered by `(frnd - K1) * K2 + K3` (slot 307, ST0), `+0x178 += 0x8000`,
/// `+0x1d8 = a`, u16 `+0x1dc = 0`, `+0x130 = G` (an immediate address)
/// and `+0x1e0` the random spin `(1 - (r & 2)) * (r & 0xfff)`. Then
/// `+0x1640 = b`, which it returns.
///
/// `magic_0155` 0x10007470 (311 bytes, `ecx` = the record; 2 DLLs).
pub fn particles_rethrow<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let this = fr.ecx;
    let obj = g.read::<u32>(this);
    let b = (g.read::<u8>(obj + 0x1650) & 0xfd) | 4;
    g.write::<u8>(obj + 0x1650, b);
    let mut p = obj.wrapping_add(0x13c);
    let mut i = 0i32;
    while i < g.read::<i32>(obj + 0x163c) {
        let r = slot_call(g, img, f, (0x41, 324), &[]).eax;
        if g.read::<u8>(p) & 1 == 0 {
            let owner = g.read::<u32>(p.wrapping_sub(0x40));
            slot_call(g, img, f, (0x5d, 531), &[owner, p.wrapping_add(0x19c)]);
        }
        g.write::<u32>(p + 0x1a8, 0x3f80_0000);
        let x = st0_f32(slot_call(g, img, f, (0x70, 307), &[]));
        let x = add(
            mul(sub(x, g.read::<f32>(w(0x92))), g.read::<f32>(w(0xb2))),
            g.read::<f32>(w(0xc2)),
        );
        let y = g.read::<f32>(p + 0x1a0);
        let s = g.read::<u32>(p + 0x178);
        g.write::<u32>(p + 0x178, s.wrapping_add(0x8000));
        let a = g.read::<u32>(this + 4);
        g.write::<u32>(p + 0x1d8, a);
        let spin = (1 - (r & 2).cast_signed()).wrapping_mul((r & 0xfff).cast_signed());
        g.write::<u16>(p + 0x1dc, 0);
        g.write::<u32>(p + 0x130, w(0xef));
        g.write::<i32>(p + 0x1e0, spin);
        g.write::<f32>(p + 0x1a0, sub(y, x));
        i += 1;
        p = p.wrapping_add(0x278);
    }
    let b = g.read::<u32>(this + 8);
    g.write::<u32>(obj + 0x1640, b);
    Ret::eax(b)
}

/// [`particles_rethrow`].
pub const PARTICLES_RETHROW: Sig = Sig {
    len: 311,
    hash: 0x57eb_1b6b_9961_ef4b,
    calls: &[],
    imports: &[],
};

/// A camera-rectangle task (cdecl `(task)`, `task[0]` its list node,
/// `task[1]` its record: count `+0x18`, state `+0x1c`, camera `+0x20`).
/// Each step while it runs: the two rectangles `*(rec+0x24) + 0x1c` and
/// `*(rec+0x28) + 0x1c` set on the camera (slot 284, 1 then 0), `+0x9c`
/// lowered by `K`, `+0x128 += 0x90`, the count down; it returns the last
/// slot's `eax`. State 0 starts the count (state 1, 0x28 steps); when
/// state 0 or 1 runs out, state 0x2710 sets the alpha (slot 693 `(0,
/// 0x1e)`) and starts state 0x2711 with 0x28 steps; when that runs out
/// the alpha is reset (slot 692 twice), the camera freed (slot 686) and
/// the node unlinked (pointing at itself). Returns the node's `+4`.
///
/// `magic_0152` 0x10008440 (318 bytes; 2 DLLs).
pub fn camera_rect_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let task = fr.arg(g, 0);
    let rec = g.read::<u32>(task + 4);
    let count = |g: &G| g.read::<i32>(rec + 0x18);
    let state = g.read::<i32>(rec + 0x1c);
    let run = match state {
        0 | 1 => {
            if state == 0 {
                g.write::<i32>(rec + 0x1c, 1);
                g.write::<u32>(rec + 0x18, 0x28);
            }
            if count(g) > 0 {
                true
            } else {
                g.write::<u32>(rec + 0x1c, 0x2710);
                slot_call(g, img, f, (0xb3, 693), &[0, 0x1e]);
                g.write::<u32>(rec + 0x1c, 0x2711);
                g.write::<u32>(rec + 0x18, 0x28);
                true
            }
        }
        0x2710 => {
            slot_call(g, img, f, (0xb3, 693), &[0, 0x1e]);
            g.write::<u32>(rec + 0x1c, 0x2711);
            g.write::<u32>(rec + 0x18, 0x28);
            true
        }
        0x2711 => count(g) > 0,
        _ => true,
    };
    if run {
        let cam = g.read::<u32>(rec + 0x20);
        let a = g.read::<u32>(rec + 0x24).wrapping_add(0x1c);
        slot_call(g, img, f, (0x3b, 284), &[cam, 1, a]);
        let b = g.read::<u32>(rec + 0x28).wrapping_add(0x1c);
        let cam2 = g.read::<u32>(rec + 0x20);
        let e = slot_call(g, img, f, (0x54, 284), &[cam2, 0, b, cam, 1, a]).eax;
        let x = sub(g.read::<f32>(rec + 0x9c), g.read::<f32>(w(0x71)));
        let n = g.read::<u32>(rec + 0x128);
        g.write::<u32>(rec + 0x128, n.wrapping_add(0x90));
        g.write::<f32>(rec + 0x9c, x);
        let c = g.read::<u32>(rec + 0x18);
        g.write::<u32>(rec + 0x18, c.wrapping_sub(1));
        return Ret::eax(e);
    }
    camera_rect_done(g, img, f, task, rec)
}

/// [`camera_rect_task`]'s end: the alpha reset, the camera freed, the
/// node unlinked. Returns the node's `+4`.
fn camera_rect_done<G: Game>(g: &mut G, img: &Image, f: u32, task: u32, rec: u32) -> Ret {
    let w = |o: u32| op(img, f, o);
    let s692 = slot_target(g, w(0xe8), 692);
    g.call(s692, 0, &[0, 1, 0]);
    let s692 = slot_target(g, w(0xfb), 692);
    g.call(s692, 0, &[1, 0, 1, 0, 1, 0]);
    let cam = g.read::<u32>(rec + 0x20);
    let s686 = slot_target(g, w(0x10e), 686);
    g.call(s686, 0, &[cam, 1, 0, 1, 0, 1, 0]);
    let node = g.read::<u32>(task);
    let prev = g.read::<u32>(node + 4);
    let next = g.read::<u32>(node + 8);
    g.write::<u32>(prev + 8, next);
    let next = g.read::<u32>(node + 8);
    let prev = g.read::<u32>(node + 4);
    g.write::<u32>(next + 4, prev);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    Ret::eax(prev)
}

/// [`camera_rect_task`].
pub const CAMERA_RECT_TASK: Sig = Sig {
    len: 318,
    hash: 0x8973_cbd6_b694_d7b5,
    calls: &[],
    imports: &[],
};

/// An object host's start with one character loaded (cdecl `(arg)`):
/// [`host_start_head`] (the code through `+0xaf` is `magic_0596`'s), slot
/// 720 `(0x3400, 0x800)`, the character 0x10af read (slot 4) and set up
/// (slot 27 `(0x10af, 1)`), the effect data `E` (an immediate) through
/// slots 718 `(E)`, 222 and 221 `(E, 0, 1000, 1)`; each call sees the
/// words the earlier ones left on the stack. `*G_DONE = 0`. Returns 0.
///
/// `magic_0349` 0x10004360 (319 bytes; 2 DLLs).
pub fn host_start_chara<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    host_start_head(g, img, f, fr.arg(g, 0));
    let mut s = Pushes::default();
    s.push_all(&[0x800, 0x3400]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xb0), 720), 0);
    s.push(0x10af);
    s.call(g, slot_target(g, w(0xc7), 4), 0);
    s.push_all(&[1, 0x10af]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xd6), 27), 0);
    s.push(w(0xec));
    s.call(g, slot_target(g, w(0xe7), 718), 0);
    s.push_all(&[1, 1000, 0, w(0x10d)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xf9), 222), 0);
    s.push_all(&[1, 1000, 0, w(0x128)]);
    s.call(g, slot_target(g, w(0x114), 221), 0);
    g.write::<u32>(w(0x133), 0);
    Ret::eax(0)
}

/// [`host_start_chara`] (the `rel32`s: [`obj_block_init`] and
/// [`tasks_init`]).
pub const HOST_START_CHARA: Sig = Sig {
    len: 319,
    hash: 0xf75f_0e15_60b1_4ce9,
    calls: &[
        (0xd, Callee::Sig(&OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&TASKS_INIT)),
    ],
    imports: &[],
};

/// A particle node set up and linked under its parent (`ecx` = `[node,
/// parent, a, b, c, d]`): u16 `+0x1a = 0xff`, an empty child ring (`+0xc`,
/// `+0x10` the node itself), `+0x14` the parent; with a parent the node
/// goes to the end of the parent's children (`+4` the parent, `+8` its
/// old last child, which points on to it; the parent's u16 `+0x18`
/// counts it). Then `+0x144 = c`, `+0x140 = d`, `+0x128 = a`, `+0x1ac =
/// b`, `+0x13c = G` (an immediate address), u16 `+0x148 = 0`, the scales
/// `+0x1a0..+0x1a8 = 1.0`, `+0xac..+0xb8 = (0, 1, 0, 0)`, `+0x11c..+0x124`
/// cleared, the matrix `+0x150` and `+0x5c` made unit (slot 235) and the
/// latter copied to `+0xdc` and `+0x1c` (slot 230), `+0x12c` a copy of
/// the vector `G_V` (slot 325). Returns slot 325's `eax`.
///
/// `magic_0573` 0x10003660 (319 bytes, `ecx` = the arguments; 2 DLLs).
pub fn node_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let args = fr.ecx;
    let parent = g.read::<u32>(args + 4);
    let node = g.read::<u32>(args);
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
    for (o, a) in [
        (0x144u32, 0xcu32),
        (0x140, 0x10),
        (0x128, 0x8),
        (0x1ac, 0x14),
    ] {
        let v = g.read::<u32>(args + a);
        g.write::<u32>(node + o, v);
    }
    g.write::<u32>(node + 0x13c, w(0x63));
    g.write::<u16>(node + 0x148, 0);
    for (o, v) in [
        (0x1a0u32, 0x3f80_0000u32),
        (0x1a8, 0x3f80_0000),
        (0x1a4, 0x3f80_0000),
        (0xac, 0),
        (0xb0, 0x3f80_0000),
        (0xb4, 0),
        (0xb8, 0),
        (0x124, 0),
        (0x120, 0),
        (0x11c, 0),
    ] {
        g.write::<u32>(node + o, v);
    }
    let a = node.wrapping_add(0x150);
    let m = node.wrapping_add(0x5c);
    slot_call(g, img, f, (0xbe, 235), &[a]);
    slot_call(g, img, f, (0xe9, 235), &[m, a]);
    slot_call(g, img, f, (0x102, 230), &[node.wrapping_add(0xdc), m, m, a]);
    slot_call(
        g,
        img,
        f,
        (0x114, 230),
        &[node.wrapping_add(0x1c), m, node.wrapping_add(0xdc), m, m, a],
    );
    let v = node.wrapping_add(0x12c);
    let e = slot_call(
        g,
        img,
        f,
        (0x12d, 325),
        &[
            v,
            w(0x127),
            node.wrapping_add(0x1c),
            m,
            node.wrapping_add(0xdc),
            m,
            m,
            a,
        ],
    );
    Ret::eax(e.eax)
}

/// [`node_setup`].
pub const NODE_SETUP: Sig = Sig {
    len: 319,
    hash: 0xa115_9fb5_0a1b_7e35,
    calls: &[],
    imports: &[],
};

/// A rising part run (cdecl `(obj)`): the position `obj + 0x40` a copy of
/// `G_V` (an immediate; slot 325), `y += K`, the scale `+0x60..+0x6c =
/// (0.0375, 0.0375, 0.0375, 1.0)`, the part's matrix set (slot 258) and
/// [`part_run_tail`] with the third program.
///
/// `magic_0443` 0x10001630 (320 bytes; 2 DLLs).
pub fn rising_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let obj = fr.arg(g, 0);
    let pos = obj.wrapping_add(0x40);
    let mut s = Pushes::default();
    s.push_all(&[w(0x9), pos]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x12), 325), 0);
    let y = add(g.read::<f32>(obj + 0x44), g.read::<f32>(w(0x27)));
    for o in [0x60u32, 0x64, 0x68] {
        g.write::<u32>(obj + o, 0x3d19_999a);
    }
    g.write::<f32>(obj + 0x44, y);
    g.write::<u32>(obj + 0x6c, 0x3f80_0000);
    s.push(obj);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x2c), 258), 0);
    part_run_tail(g, img, f + 0x5c, obj, 8, &mut s)
}

/// The end [`rising_part`] and [`moving_part`] share (`t` the address
/// of its first operand, `G_A` of the slot 231 call): the part's matrix multiplied and copied through
/// the DLL's matrices (slots 231 and 230 with `*G_A`, `*G_B`, `*G_C`,
/// `*G_D`), `*G_P = ITEM` and the part run (slot 712 `(obj, 0x30)`) with
/// the lock `*G_L` held at 1. When it ends, the part's objects are freed
/// (slot 713) and it starts again with the program at `+program` of slot
/// 941 `(*G_E + 0x10)` ([`start_part`], `0x1000`, 1). With `*G_FADE` set,
/// the u16 `+0x76` drops by 0x88 and it returns 0 once that is below 1;
/// else 1.
fn part_run_tail<G: Game>(
    g: &mut G,
    img: &Image,
    t: u32,
    obj: u32,
    program: u32,
    s: &mut Pushes,
) -> Ret {
    let w = |o: u32| img.word(t + o - 0x5c);
    let [a, b, c] = [0x5cu32, 0x67, 0x73].map(|o| g.read::<u32>(w(o)));
    s.push_all(&[a, b, c]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x61), 231), 0);
    let [c, d] = [0x7bu32, 0x86].map(|o| g.read::<u32>(w(o)));
    s.push_all(&[c, d]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x80), 230), 0);
    let p = g.read::<u32>(w(0x93));
    g.write::<u32>(p, w(0x9b));
    let lock = g.read::<u32>(w(0xa0));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    s.push_all(&[0x30, obj]);
    // call width: 12 (the most `Pushes::words` holds)
    let ended = s.call(g, slot_target(g, w(0xae), 712), 0).eax;
    let lock = g.read::<u32>(w(0xbc));
    g.write::<u32>(lock, saved);
    if ended != 0 {
        let p = g.read::<u32>(w(0xca));
        g.write::<u32>(p, w(0xd1));
        let t713 = slot_target(g, w(0xd6), 713);
        g.call(t713, 0, &[obj]);
        let p = g.read::<u32>(w(0xe3));
        g.write::<u32>(p, w(0xe9));
        let e = g.read::<u32>(g.read::<u32>(w(0xee)).wrapping_add(0x10));
        let t941 = slot_target(g, w(0xf6), 941);
        let table = g.call(t941, 0, &[e, obj]).eax;
        let program = g.read::<u32>(table.wrapping_add(program));
        let start = img.rel32_target(t + 0x111 - 0x5c);
        start_part(g, img, start, obj, 0x1000, program, 1);
    }
    if g.read::<u32>(w(0x11a)) != 0 {
        let v = g.read::<u16>(obj + 0x76).wrapping_add(0xff78);
        g.write::<u16>(obj + 0x76, v);
        if v.cast_signed() < 1 {
            return Ret::eax(0);
        }
    }
    Ret::eax(1)
}

/// [`rising_part`] (the `rel32`: [`start_part`]).
pub const RISING_PART: Sig = Sig {
    len: 320,
    hash: 0xc698_0a27_10d9_c2fa,
    calls: &[(0x111, Callee::Sig(&START_PART))],
    imports: &[],
};

/// An object host's start with a sound bank (cdecl `(arg)`):
/// [`host_start_head`], slot 720 `(0x3400, 0x800)`, slot 523 `(0, n)`
/// with `n` slot 627's answer (-1 when negative), the effect data `E`
/// (an immediate) through slots 718 `(E)`, 222 and 221 `(E, 0, 1000,
/// 1)`; each call sees the words the earlier ones left on the stack.
/// `*G_DONE = 0`. Returns 0.
///
/// `magic_0393` 0x100010a0 (324 bytes; 2 DLLs).
pub fn host_start_bank<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    host_start_head(g, img, f, fr.arg(g, 0));
    let mut s = Pushes::default();
    s.push_all(&[0x800, 0x3400]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xb0), 720), 0);
    let n = s.call(g, slot_target(g, w(0xc7), 627), 0).eax;
    // or ecx, -1; test eax, eax; cmovs eax, ecx
    let n = if n.cast_signed() < 0 { u32::MAX } else { n };
    s.push_all(&[n, 0]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xdd), 523), 0);
    s.push(w(0xf1));
    s.call(g, slot_target(g, w(0xec), 718), 0);
    s.push_all(&[1, 1000, 0, w(0x112)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xfe), 222), 0);
    s.push_all(&[1, 1000, 0, w(0x12d)]);
    s.call(g, slot_target(g, w(0x119), 221), 0);
    g.write::<u32>(w(0x138), 0);
    Ret::eax(0)
}

/// [`host_start_bank`] (the `rel32`s: [`obj_block_init`] and
/// [`tasks_init`]).
pub const HOST_START_BANK: Sig = Sig {
    len: 324,
    hash: 0x28c3_c4b0_24db_588b,
    calls: &[
        (0xd, Callee::Sig(&OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&TASKS_INIT)),
    ],
    imports: &[],
};

/// The object host's start of [`host_start_523_a`] and
/// [`host_start_523_b`], whose code differs only in the flag's bit
/// `shift`: [`host_start_head`], slot 720 `(0x3400, 0x800)`, slot 523
/// `(0, (*G_F >> shift) & 1)` and `(1, G_T[*G_BLOCK])` (`*G_BLOCK` the
/// object block's first word, slot 168's answer), the effect data `E`
/// (an immediate) through slots 718 `(E)`, 222 and 221 `(E, 0, 1000,
/// 1)`; each call sees the words the earlier ones left on the stack.
/// `*G_DONE = 0`. Returns 0.
fn host_start_523<G: Game>(g: &mut G, img: &Image, fr: &Frame, shift: u32) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    host_start_head(g, img, f, fr.arg(g, 0));
    let mut s = Pushes::default();
    s.push_all(&[0x800, 0x3400]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xb0), 720), 0);
    let flag = (g.read::<i32>(w(0xc7)) >> shift) & 1;
    s.push_all(&[flag.cast_unsigned(), 0]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xd3), 523), 0);
    let i = g.read::<u32>(w(0xe2));
    let v = g.read::<u32>(w(0xe9).wrapping_add(i.wrapping_mul(4)));
    s.push_all(&[v, 1]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xee), 523), 0);
    s.push(w(0x102));
    s.call(g, slot_target(g, w(0xfd), 718), 0);
    s.push_all(&[1, 1000, 0, w(0x123)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x10f), 222), 0);
    s.push_all(&[1, 1000, 0, w(0x13e)]);
    s.call(g, slot_target(g, w(0x12a), 221), 0);
    g.write::<u32>(w(0x149), 0);
    Ret::eax(0)
}

/// [`host_start_523`] with bit 10 (cdecl `(arg)`).
///
/// `magic_0053` 0x10001000 (341 bytes; 2 DLLs).
pub fn host_start_523_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    host_start_523(g, img, fr, 10)
}

/// [`host_start_523_a`] (the `rel32`s: [`obj_block_init`] and
/// [`tasks_init`]).
pub const HOST_START_523_A: Sig = Sig {
    len: 341,
    hash: 0x746b_de2d_98c6_b214,
    calls: &[
        (0xd, Callee::Sig(&OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&TASKS_INIT)),
    ],
    imports: &[],
};

/// [`host_start_523`] with bit 11 (cdecl `(arg)`).
///
/// `magic_0052` 0x10001000 (341 bytes; 2 DLLs).
pub fn host_start_523_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    host_start_523(g, img, fr, 11)
}

/// [`host_start_523_b`] (the `rel32`s: [`obj_block_init`] and
/// [`tasks_init`]).
pub const HOST_START_523_B: Sig = Sig {
    len: 341,
    hash: 0x5dff_957e_78d3_d71d,
    calls: &[
        (0xd, Callee::Sig(&OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&TASKS_INIT)),
    ],
    imports: &[],
};

/// An object host's start with two effect data and two streams (cdecl
/// `(arg)`): [`host_start_head`], slot 720 `(0x3400, 0x800)`, the first
/// data `E1` (an immediate) through slots 718 `(E1)`, 222 `(E1, 0, 4,
/// 1)` and 221 `(E1, 0, 1000, 1)`, the second `E2` through 718 and its
/// streams 0 and 1 cued (slot 551 `(slot 719 (E2, 6, k), k, 1)`); each
/// call sees the words the earlier ones left on the stack. `*G_DONE =
/// 0`. Returns 0.
///
/// `magic_0168` 0x100010a0 (385 bytes; 2 DLLs).
pub fn host_start_streams<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    host_start_head(g, img, f, fr.arg(g, 0));
    let mut s = Pushes::default();
    s.push_all(&[0x800, 0x3400]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xb0), 720), 0);
    s.push(w(0xcc));
    s.call(g, slot_target(g, w(0xc7), 718), 0);
    s.push_all(&[1, 4, 0, w(0xea)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xd9), 222), 0);
    s.push_all(&[1, 1000, 0, w(0x105)]);
    s.call(g, slot_target(g, w(0xf1), 221), 0);
    s.push(w(0x111));
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x10c), 718), 0);
    s.push_all(&[0, 6, w(0x12d)]);
    let a = s.call(g, slot_target(g, w(0x11e), 719), 0).eax;
    s.push_all(&[1, 0, a]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x139), 551), 0);
    let mut s = Pushes::default();
    s.push_all(&[1, 6, w(0x158)]);
    // call width: 12 (the most `Pushes::words` holds)
    let b = s.call(g, slot_target(g, w(0x146), 719), 0).eax;
    s.push_all(&[1, 1, b]);
    s.call(g, slot_target(g, w(0x164), 551), 0);
    g.write::<u32>(w(0x175), 0);
    Ret::eax(0)
}

/// [`host_start_streams`] (the `rel32`s: [`obj_block_init`] and
/// [`tasks_init`]).
pub const HOST_START_STREAMS: Sig = Sig {
    len: 385,
    hash: 0x2437_187f_dc85_c3ac,
    calls: &[
        (0xd, Callee::Sig(&OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&TASKS_INIT)),
    ],
    imports: &[],
};

/// An object host's start with one stream (cdecl `(arg)`): `magic_0596`'s
/// [`host_start`], then the effect data's stream 2 cued (slot 551 `(slot
/// 719 (E, 6, 2), 0, 1)`); each call sees the words the earlier ones
/// left on the stack. `*G_DONE = 0`. Returns 0.
///
/// `magic_0102` 0x100010a0 (327 bytes; 4 DLLs).
pub fn host_start_stream<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let mut stack = host_start(g, img, f, fr.arg(g, 0));
    stack.splice(0..0, [w(0x11e), 6, 2]);
    let t = slot_target(g, w(0x10f), 719);
    // call width: 12 at most (the earlier calls' words stay on the stack)
    let a = g.call(t, 0, &stack[..stack.len().min(12)]).eax;
    stack.splice(0..0, [a, 0, 1]);
    let t = slot_target(g, w(0x12a), 551);
    // call width: 12 at most (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack[..stack.len().min(12)]);
    g.write::<u32>(w(0x13b), 0);
    Ret::eax(0)
}

/// [`host_start_stream`] (the `rel32`s: [`obj_block_init`] and
/// [`tasks_init`]).
pub const HOST_START_STREAM: Sig = Sig {
    len: 327,
    hash: 0xcd95_907d_a2b6_70a6,
    calls: &[
        (0xd, Callee::Sig(&OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&TASKS_INIT)),
    ],
    imports: &[],
};

/// An object host's start with a second effect data (cdecl `(arg)`):
/// `magic_0596`'s [`host_start`], then the second data `E2` (an
/// immediate) through slots 718 `(E2)` and 221 `(E2, 0, 1000, 1)`; each
/// call sees the words the earlier ones left on the stack. `*G_DONE =
/// 0`. Returns 0.
///
/// `magic_0511` 0x100010a0 (332 bytes; 4 DLLs).
pub fn host_start_two<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let mut stack = host_start(g, img, f, fr.arg(g, 0));
    stack.insert(0, w(0x114));
    let t = slot_target(g, w(0x10f), 718);
    // call width: 12 at most (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack[..stack.len().min(12)]);
    stack.splice(0..0, [w(0x135), 0, 1000, 1]);
    let t = slot_target(g, w(0x121), 221);
    // call width: 12 at most (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack[..stack.len().min(12)]);
    g.write::<u32>(w(0x140), 0);
    Ret::eax(0)
}

/// [`host_start_two`] (the `rel32`s: [`obj_block_init`] and
/// [`tasks_init`]).
pub const HOST_START_TWO: Sig = Sig {
    len: 332,
    hash: 0x1fd0_19ed_f5f9_4cfc,
    calls: &[
        (0xd, Callee::Sig(&OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&TASKS_INIT)),
    ],
    imports: &[],
};

/// The character turned to face the map centre (cdecl, no arguments; an
/// aligned /GS frame): the position of `*G_CH` (slot 262 into three
/// locals `x, y, z`, `w = 1`) and the map centre (slot 292, a local
/// vector), `yaw = atan2(c.z - z, c.x - x)` (`_CIatan2`, rounded to
/// `f32`), `A - yaw` wrapped once, `B - that` wrapped once ([`wrap_once`]:
/// `x - T` above `HI`, `x + T` below `LO`), and the angle set (slot 576
/// `(*G_M, 1, angle)`). Returns slot 576's `eax`.
///
/// `magic_0307` 0x100013f0 (287 bytes; 2 DLLs).
pub fn face_map_centre<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    // push ebp; and esp, -8; sub esp, 0x30
    let top = fr.esp.wrapping_sub(4) & !7;
    let [x, y, z] = [0x24u32, 0x20, 0x1c].map(|o| top.wrapping_sub(o));
    let centre = top.wrapping_sub(0x14);
    let ch = g.read::<u32>(w(0x2a));
    slot_call(g, img, f, (0x24, 262), &[ch, x, y, z]);
    g.write::<u32>(top.wrapping_sub(0x18), 0x3f80_0000);
    slot_call(g, img, f, (0x3c, 292), &[centre, ch, x, y, z]);
    let dz = sub(g.read::<f32>(centre + 8), g.read::<f32>(z));
    let dx = sub(g.read::<f32>(centre), g.read::<f32>(x));
    g.write::<u32>(top.wrapping_sub(8), 0x3f80_0000);
    let yaw = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let ops = [0xa2, 0xaa, 0xb2, 0xaa];
    let a = wrap_once(g, img, f, sub(g.read::<f32>(w(0x9a)), yaw), ops);
    let b = wrap_once(g, img, f, sub(g.read::<f32>(w(0xd6)), a), ops);
    let m = g.read::<u32>(w(0x107));
    let e = slot_call(g, img, f, (0xf3, 576), &[m, 1, b.to_bits()]).eax;
    Ret::eax(e)
}

/// [`face_map_centre`] (the `rel32`s: `_CIatan2` and
/// `__security_check_cookie`).
pub const FACE_MAP_CENTRE: Sig = Sig {
    len: 287,
    hash: 0x8dae_0046_eb51_e3d2,
    calls: &[(0x8a, Callee::Import("_CIatan2")), (0x117, Callee::Any)],
    imports: &[],
};

/// An object host's start that turns its character to the map centre
/// (cdecl `(arg)`): [`host_start_head`], slot 720 `(0x3400, 0x800)`,
/// [`face_map_centre`], the first effect data `E1` (an immediate) through
/// slots 718 `(E1)`, 222 and 221 `(E1, 0, 1000, 1)`, the second `E2`
/// through 718 and 221; each call sees the words the earlier ones left
/// on the stack. `*G_DONE = 0`. Returns 0.
///
/// `magic_0307` 0x100010a0 (337 bytes; 2 DLLs).
pub fn host_start_facing<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    host_start_head(g, img, f, fr.arg(g, 0));
    let mut s = Pushes::default();
    s.push_all(&[0x800, 0x3400]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xb0), 720), 0);
    // push ebp and the two words: the callee's esp at its return address
    let face = Frame {
        at: img.rel32_target(f + 0xc7),
        esp: fr.esp.wrapping_sub(0x10),
        ..*fr
    };
    face_map_centre(g, img, &face);
    s.push(w(0xd1));
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xcc), 718), 0);
    s.push_all(&[1, 1000, 0, w(0xf2)]);
    s.call(g, slot_target(g, w(0xde), 222), 0);
    s.push_all(&[1, 1000, 0, w(0x10d)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xf9), 221), 0);
    s.push(w(0x119));
    s.call(g, slot_target(g, w(0x114), 718), 0);
    s.push_all(&[1, 1000, 0, w(0x13a)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x126), 221), 0);
    g.write::<u32>(w(0x145), 0);
    Ret::eax(0)
}

/// [`host_start_facing`] (the `rel32`s: [`obj_block_init`],
/// [`tasks_init`] and [`face_map_centre`]).
pub const HOST_START_FACING: Sig = Sig {
    len: 337,
    hash: 0x3e4e_76f7_3a7b_d05c,
    calls: &[
        (0xd, Callee::Sig(&OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&TASKS_INIT)),
        (0xc7, Callee::Sig(&FACE_MAP_CENTRE)),
    ],
    imports: &[],
};

/// A model drawn through the camera (internal: `ecx` = the model, `edx`
/// = its matrix, `esp` the frame's entry; a /GS frame). The model's
/// 64-bit handle `h` (slot 772 `(model, 0)`, `edx:eax`), the screen and
/// camera matrices into locals (`ebp - 0xc8`, `ebp - 0x48`; slots 723 and
/// 724), the camera times the matrix into the local `ebp - 0x88` (slot
/// 231), which `*G_B + 0x3b50` points at. With the lock `*G_L` held at 1
/// slot 773 `(h)` says whether the model is ready; then the lock is 1
/// and the record `ebp - 0xf0` (its `+0x1c` the model) goes to slot 819
/// `(h, rec, b)` and slot 870 `(h)` (lock 1 again), or else to slot 871
/// `(h, rec, b, 0x248)`, `b` = `*G_B + 0x3b30`. The lock is restored.
/// Returns the lock's address (`G_L`).
///
/// `magic_0563` 0x10002060 (327 bytes; 2 DLLs).
pub fn model_draw<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, model: u32, m: u32) -> u32 {
    let w = |o: u32| op(img, f, o);
    let ebp = esp.wrapping_sub(4);
    let [screen, cam, out, rec] = [0xc8u32, 0x48, 0x88, 0xf0].map(|o| ebp.wrapping_sub(o));
    let h = slot_call(g, img, f, (0x22, 772), &[model, 0]);
    let (lo, hi) = (h.eax, h.edx);
    slot_call(g, img, f, (0x32, 723), &[screen, model, 0]);
    slot_call(g, img, f, (0x51, 724), &[cam]);
    slot_call(g, img, f, (0x6d, 231), &[out, cam, m]);
    let b = g.read::<u32>(w(0x7b)).wrapping_add(0x3b30);
    g.write::<u32>(b + 0x20, out);
    let lock = g.read::<u32>(w(0x8f));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let ready = slot_call(g, img, f, (0x9f, 773), &[lo, hi]).eax;
    let lock = g.read::<u32>(w(0xb5));
    g.write::<u32>(lock, saved);
    g.write::<u32>(rec + 0x1c, model);
    g.write::<u32>(lock, 1);
    if ready != 0 {
        slot_call(g, img, f, (0xe3, 819), &[lo, hi, rec, b]);
        let lock = g.read::<u32>(w(0xf2));
        g.write::<u32>(lock, 1);
        slot_call(g, img, f, (0xfe, 870), &[lo, hi, lo, hi, rec, b]);
    } else {
        slot_call(g, img, f, (0x118, 871), &[lo, hi, rec, b, 0x248]);
    }
    let lock = g.read::<u32>(w(0x12a));
    g.write::<u32>(lock, saved);
    lock
}

/// [`model_draw`] (the `rel32`: `__security_check_cookie`).
pub const MODEL_DRAW: Sig = Sig {
    len: 327,
    hash: 0x52a1_9783_1986_3cb0,
    calls: &[(0x13f, Callee::Any)],
    imports: &[],
};

/// `+= 1` on the word at `a`.
fn bump<G: Game>(g: &mut G, a: u32) {
    let v = g.read::<u32>(a);
    g.write::<u32>(a, v.wrapping_add(1));
}

/// `|= bits` on the byte at `a`.
fn set_bits<G: Game>(g: &mut G, a: u32, bits: u8) {
    let v = g.read::<u8>(a);
    g.write::<u8>(a, v | bits);
}

/// A four-part effect's script task (cdecl `(task)`, `task[0]` its list
/// node, `task[1]` its record: state `+0x18`, parts `a, b, c, d` at
/// `+0x1c..+0x28`, `p` at `+0x2c`, counters `+0x3c` and `+0x40`). Every
/// step counts both counters up (state 12 restarts `+0x40` first). The
/// states: 0 sets the two parts' speeds (`a`: 30, 1, 5; `b`: 30, 1, 20)
/// and the flags of `c` (0x18) and `d` (0x10); 10 sets bit 0 of `c`'s
/// byte `+0x54`, 11 and 12 that of `d`; 13 waits for `+0x40 >= 5` and
/// goes to 0x14; 0x1e and 0x1f raise `p`'s `+0x30` by `K`; 0x2710
/// unlinks the node. States 0, 10, 11, 12, 13 (done waiting), 0x14 and
/// 0x1e go on to the next. Returns what the original leaves in `eax`:
/// the state minus the cases it tested (0x1e/0x1f: `p`; 0x2710: the
/// node's old `prev`).
///
/// `magic_0155` 0x10009970 (335 bytes; 2 DLLs).
pub fn four_part_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let task = fr.arg(g, 0);
    let rec = g.read::<u32>(task + 4);
    let p = g.read::<u32>(rec + 0x2c);
    let [a, b, c, d] = [0x1cu32, 0x20, 0x24, 0x28].map(|o| g.read::<u32>(rec + o));
    let mut state = g.read::<u32>(rec + 0x18);
    let counters = |g: &mut G| {
        bump(g, rec + 0x3c);
        bump(g, rec + 0x40);
    };
    let next = |g: &mut G| bump(g, rec + 0x18);
    if state <= 10 {
        if state == 10 {
            set_bits(g, c + 0x54, 1);
            next(g);
        } else if state == 0 {
            for (o, v) in [
                (0x2cu32, 0x41f0_0000u32),
                (0x30, 0x3f80_0000),
                (0x34, 0x40a0_0000),
            ] {
                g.write::<u32>(a + o, v);
            }
            for (o, v) in [
                (0x2cu32, 0x41f0_0000u32),
                (0x30, 0x3f80_0000),
                (0x34, 0x41a0_0000),
            ] {
                g.write::<u32>(b + o, v);
            }
            set_bits(g, c + 0x34, 0x18);
            set_bits(g, d + 0x34, 0x10);
            next(g);
        }
        counters(g);
        return Ret::eax(state);
    }
    loop {
        if state > 0x14 {
            return match state {
                0x1e | 0x1f => {
                    if state == 0x1e {
                        next(g);
                    }
                    let y = add(g.read::<f32>(p + 0x30), g.read::<f32>(op(img, f, 0xde)));
                    g.write::<f32>(p + 0x30, y);
                    counters(g);
                    Ret::eax(p)
                }
                0x2710 => {
                    let node = g.read::<u32>(task);
                    let prev = g.read::<u32>(node + 4);
                    let after = g.read::<u32>(node + 8);
                    g.write::<u32>(prev + 8, after);
                    let after = g.read::<u32>(node + 8);
                    let prev = g.read::<u32>(node + 4);
                    g.write::<u32>(after + 4, prev);
                    g.write::<u32>(node + 8, node);
                    g.write::<u32>(node + 4, node);
                    counters(g);
                    Ret::eax(prev)
                }
                _ => {
                    counters(g);
                    Ret::eax(state.wrapping_sub(0x2710))
                }
            };
        }
        match state {
            0x14 => next(g),
            11 => {
                set_bits(g, d + 0x54, 1);
                next(g);
                counters(g);
                return Ret::eax(0);
            }
            12 => {
                set_bits(g, d + 0x54, 1);
                next(g);
                bump(g, rec + 0x3c);
                g.write::<u32>(rec + 0x40, 1);
                return Ret::eax(0);
            }
            13 if g.read::<i32>(rec + 0x40) >= 5 => {
                g.write::<u32>(rec + 0x18, 0x14);
                state = 0x14;
                continue;
            }
            _ => {
                counters(g);
                return Ret::eax(state - 13);
            }
        }
        counters(g);
        return Ret::eax(state);
    }
}

/// [`four_part_task`].
pub const FOUR_PART_TASK: Sig = Sig {
    len: 335,
    hash: 0x1c11_5b28_95ab_3bc7,
    calls: &[],
    imports: &[],
};

/// A part moved by a matrix (cdecl `(obj)`, a /GS frame): the point `G_V`
/// through the matrix `G_M` (both immediates; slot 229) into a local
/// (`ebp - 0x14`), copied to the position `obj + 0x40` (slot 325), the
/// part's matrix set (slot 258) and [`part_run_tail`] with the fourth
/// program.
///
/// `magic_0443` 0x10001860 (337 bytes; 2 DLLs).
pub fn moving_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let obj = fr.arg(g, 0);
    let v = fr.esp.wrapping_sub(0x18);
    let mut s = Pushes::default();
    s.push_all(&[w(0x16), w(0x1e), v]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x24), 229), 0);
    s.push_all(&[v, obj.wrapping_add(0x40)]);
    s.call(g, slot_target(g, w(0x39), 325), 0);
    s.push(obj);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x46), 258), 0);
    part_run_tail(g, img, f + 0x55, obj, 0xc, &mut s)
}

/// [`moving_part`] (the `rel32`s: [`start_part`] and
/// `__security_check_cookie`).
pub const MOVING_PART: Sig = Sig {
    len: 337,
    hash: 0x8d9f_a79b_1b31_2a9a,
    calls: &[
        (0x10a, Callee::Sig(&START_PART)),
        (0x134, Callee::Any),
        (0x149, Callee::Any),
    ],
    imports: &[],
};

/// The character's battle thickness plus a margin by its model
/// (internal, no arguments; the float in `xmm0`): the model of `*G_CH`
/// (slot 390), its handle (slot 215) and thickness (slot 19, ST0 stored
/// as `f32`), plus the margin of models 0x1015, 0x102d, 0x108c, 0x111a,
/// 0x1127 and 0x112c (constants of the DLL); other models get none.
/// `eax` is slot 19's.
///
/// `magic_0188` 0x100014b0 (343 bytes; 2 DLLs).
pub fn thickness_margin<G: Game>(g: &mut G, img: &Image, f: u32) -> Ret {
    let w = |o: u32| op(img, f, o);
    let ch = g.read::<u32>(w(0xc));
    let model = slot_call(g, img, f, (0x5, 390), &[ch]).eax;
    let h = slot_call(g, img, f, (0x1c, 215), &[ch, ch]).eax;
    let (table, k) = match model {
        0x108c => (0x58, Some(0x6d)),
        0x102d => (0x7c, Some(0x91)),
        0x1015 => (0xa0, Some(0xb5)),
        0x111a => (0xc4, Some(0xd9)),
        0x112c => (0x111, Some(0x126)),
        0x1127 => (0x135, Some(0x14a)),
        _ => (0xf5, None),
    };
    let out = slot_call(g, img, f, (table, 19), &[h]);
    let x = st0_f32(out);
    let x = k.map_or(x, |k| add(x, g.read::<f32>(w(k))));
    Ret::xmm0(Xmm::f32(x), out.eax)
}

/// [`thickness_margin`].
pub const THICKNESS_MARGIN: Sig = Sig {
    len: 343,
    hash: 0x250a_c0c5_3ca4_a0d7,
    calls: &[],
    imports: &[],
};

/// A particle effect's data set up (cdecl `(arg)`): `*G_ARG = arg`, three
/// pointers (`G_A`, `G_B`, `G_E` = the effect data `E`, immediates), `E`
/// through slots 718 `(E)` and 222 `(E, 0, 1000, 0x2000)`, its part data
/// `D` = slot 719 `(E, 8, 0)` into `*G_D` and initialised (slot 711 `(D,
/// G_I, 0)`, with the lock `*G_L` held at 1), the environment `G_ENV`
/// (slot 710 `(G_ENV, D, *G_A, 0x80000)`), the three programs of slot 941
/// `(D[5..7])` into `G_P0..G_P2`, the block `G_BLK` cleared (0x110 bytes,
/// `memset`) and its words set (`+0x10c` = `G_T`, an immediate; three
/// flags 1). Each call sees the words the earlier ones left on the
/// stack. Returns 1.
///
/// `magic_0292` 0x10001240 (345 bytes; 2 DLLs).
pub fn ppp_effect_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    g.write::<u32>(w(0x7), fr.arg(g, 0));
    for (dst, v) in [(0x1eu32, 0x22u32), (0x28, 0x2c), (0x32, 0x36)] {
        g.write::<u32>(w(dst), w(v));
    }
    let mut s = Pushes::default();
    s.push(w(0x18));
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xc), 718), 0);
    let e = g.read::<u32>(w(0x55));
    s.push_all(&[0x2000, 1000, 0, e]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x3d), 222), 0);
    let e = g.read::<u32>(w(0x6c));
    s.push_all(&[0, 8, e]);
    // call width: 12 (the most `Pushes::words` holds)
    let d = s.call(g, slot_target(g, w(0x5c), 719), 0).eax;
    let lock = g.read::<u32>(w(0x74));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x7d), d);
    g.write::<u32>(lock, 1);
    s.push_all(&[0, w(0x82), d]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x88), 711), 0);
    let lock = g.read::<u32>(w(0x9b));
    let a = g.read::<u32>(w(0xa6));
    g.write::<u32>(lock, saved);
    let d = g.read::<u32>(w(0xae));
    s.push_all(&[0x80000, a, d, w(0xb8)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xb3), 710), 0);
    let x = g.read::<u32>(g.read::<u32>(w(0xc5)).wrapping_add(0x14));
    s.push(x);
    // call width: 12 (the most `Pushes::words` holds)
    let p = s.call(g, slot_target(g, w(0xcd), 941), 0).eax;
    g.write::<u32>(w(0xda), p);
    let mut s = Pushes::default();
    for (k, (src, table, dst)) in [(0xdfu32, 0xeau32, 0xf7u32), (0xfc, 0x104, 0x11d)]
        .into_iter()
        .enumerate()
    {
        let x = g.read::<u32>(g.read::<u32>(w(src)).wrapping_add(0x18 + 4 * k as u32));
        s.push(x);
        // call width: 12 (the most `Pushes::words` holds)
        let p = s.call(g, slot_target(g, w(table), 941), 0).eax;
        g.write::<u32>(w(dst), p);
    }
    g.write_bytes(w(0x118), &[0u8; 0x110]);
    g.write::<u32>(w(0x12b), w(0x12f));
    for o in [0x135u32, 0x13f, 0x149] {
        g.write::<u32>(w(o), 1);
    }
    Ret::eax(1)
}

/// [`ppp_effect_setup`] (the `rel32`: `memset`).
pub const PPP_EFFECT_SETUP: Sig = Sig {
    len: 345,
    hash: 0x17f2_3cf8_8ed1_b258,
    calls: &[(0x122, Callee::Import("memset"))],
    imports: &[],
};

/// A particle effect's data set up with six buffers (cdecl `(arg)`):
/// `*G_ARG = arg`, seven pointers stored (six buffers and `G_E` = the
/// effect data `E`, immediates), `E` through slot 718, its part data `D`
/// = slot 719 `(E, 8, 0)` into `*G_D` and initialised (slot 711 `(D,
/// G_I, 0)`, with the lock `*G_L` held at 1), the environment `G_ENV`
/// (slot 710 `(G_ENV, D, *G_A, 0x80000)`), the three programs of slot 941
/// `(D[5..7])` into `G_P0..G_P2`, the block `G_BLK` cleared (0x110 bytes,
/// `memset`) and its words set (`+0x10c` = `G_T`, an immediate; three
/// flags 1). Each call sees the words the earlier ones left on the
/// stack. Returns 1.
///
/// `magic_0662` 0x100014b0 (351 bytes; 2 DLLs).
pub fn ppp_effect_setup_6<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    g.write::<u32>(w(0x7), fr.arg(g, 0));
    for (dst, v) in [
        (0x1eu32, 0x22u32),
        (0x28, 0x2c),
        (0x32, 0x36),
        (0x3c, 0x40),
        (0x46, 0x4a),
        (0x50, 0x54),
        (0x5a, 0x5e),
    ] {
        g.write::<u32>(w(dst), w(v));
    }
    let mut s = Pushes::default();
    s.push(w(0x18));
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xc), 718), 0);
    let e = g.read::<u32>(w(0x75));
    s.push_all(&[0, 8, e]);
    // call width: 12 (the most `Pushes::words` holds)
    let d = s.call(g, slot_target(g, w(0x65), 719), 0).eax;
    let lock = g.read::<u32>(w(0x7d));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x86), d);
    g.write::<u32>(lock, 1);
    s.push_all(&[0, w(0x8b), d]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x91), 711), 0);
    let lock = g.read::<u32>(w(0xa4));
    let a = g.read::<u32>(w(0xaf));
    g.write::<u32>(lock, saved);
    let d = g.read::<u32>(w(0xb7));
    s.push_all(&[0x80000, a, d, w(0xc1)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xbc), 710), 0);
    for (k, (src, table, dst)) in [
        (0xceu32, 0xd6u32, 0xe3u32),
        (0xe8, 0xf0, 0xfd),
        (0x102, 0x10a, 0x123),
    ]
    .into_iter()
    .enumerate()
    {
        let x = g.read::<u32>(g.read::<u32>(w(src)).wrapping_add(0x14 + 4 * k as u32));
        s.push(x);
        // call width: 12 (the most `Pushes::words` holds)
        let p = s.call(g, slot_target(g, w(table), 941), 0).eax;
        g.write::<u32>(w(dst), p);
    }
    g.write_bytes(w(0x11e), &[0u8; 0x110]);
    g.write::<u32>(w(0x131), w(0x135));
    for o in [0x13bu32, 0x145, 0x14f] {
        g.write::<u32>(w(o), 1);
    }
    Ret::eax(1)
}

/// [`ppp_effect_setup_6`] (the `rel32`: `memset`).
pub const PPP_EFFECT_SETUP_6: Sig = Sig {
    len: 351,
    hash: 0x1230_4794_f9c4_69d0,
    calls: &[(0x128, Callee::Import("memset"))],
    imports: &[],
};

/// A packed texture drawn on a model (`ecx` = the model, cdecl `(-, a,
/// bits: u64, x, y, n)`, a /GS frame). `bits` packs the texture: its
/// number (bits 0..13), a format (bits 20..25) and two sizes as powers of
/// two (bits 26..29 and 30..33). Slot 885 `(number, format, 0, 0, 1 <<
/// s26, 1 << s30, buf)` fills the local buffer `ebp - 0x104` with the
/// lock `*G_L` held at 1; when it answers non-zero, slot 905 `(model[0x830],
/// buf, a, x, y, 0, 0, &q, slot 776 (0x42))` draws it, `q` the local quad
/// `(0, 1, 1, (n << 7) / 128 * K)` at `ebp - 0x114`. Returns 0, or slot
/// 905's `eax`.
///
/// `magic_0348` 0x100051f0 (360 bytes; 2 DLLs).
pub fn packed_texture<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let ebp = fr.esp.wrapping_sub(4);
    let buf = ebp.wrapping_sub(0x104);
    let quad = ebp.wrapping_sub(0x114);
    let a = fr.arg(g, 1);
    let model = g.read::<u32>(fr.ecx.wrapping_add(0x830));
    let bits = u64::from(fr.arg(g, 2)) | (u64::from(fr.arg(g, 3)) << 32);
    let n = fr.arg(g, 6).cast_signed();
    // shl 7, then a signed division by 128 (cdq; and edx, 0x7f; sar 7)
    let k = n.wrapping_shl(7) / 128;
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let size = mul(k as f32, g.read::<f32>(w(0x83)));
    let lock = g.read::<u32>(w(0x3c));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s30 = 1u16 << ((bits >> 30) & 0xf);
    let s26 = 1u16 << ((bits >> 26) & 0xf);
    for (o, v) in [(0u32, 0u32), (8, 0x3f80_0000), (4, 0x3f80_0000)] {
        g.write::<u32>(quad + o, v);
    }
    let format = u32::from((bits >> 20) as u16 & 0x3f);
    let number = bits as u32 & 0x3fff;
    g.write::<f32>(quad + 0xc, size);
    let filled = slot_call(
        g,
        img,
        f,
        (0xc5, 885),
        &[number, format, 0, 0, s26.into(), s30.into(), buf],
    )
    .eax;
    let lock = g.read::<u32>(w(0xe5));
    g.write::<u32>(lock, saved);
    if filled == 0 {
        return Ret::eax(0);
    }
    let q = slot_call(g, img, f, (0xf6, 776), &[0x42]).eax;
    let [x, y] = [fr.arg(g, 4), fr.arg(g, 5)];
    let e = slot_call(
        g,
        img,
        f,
        (0x145, 905),
        &[model, buf, a, x, y, 0, 0, quad, q],
    );
    Ret::eax(e.eax)
}

/// [`packed_texture`] (the `rel32`: `__security_check_cookie`).
pub const PACKED_TEXTURE: Sig = Sig {
    len: 360,
    hash: 0x5dec_4eb0_1647_bf92,
    calls: &[(0x160, Callee::Any)],
    imports: &[],
};

/// Two lights started and a scale task (cdecl, no arguments): the state
/// `*G_S`'s light records `+0x1590` (colour bytes `0xff, 0xff, 0xff, 0x80`
/// at `+0x84`, `+0x80 = 0`, count 0x14) and `+0x1630` (`0xff, 0xff, 0xff,
/// 0x80` at `+0x80`, the bytes `+0x85..+0x87` cleared, count 0x33), each
/// with u16 `+0x98 = 0x48`, `+0x8c`/`+0x90 = 0x1e`, `+0x94 = 0`, and a
/// task node of its own ([`take`] with the callback `CB`, an immediate)
/// appended to the DLL's task list (the first record's `+0x94` then 1);
/// then [`scale_task`] on `+0x17d8` with the scales `K1`, `K2` and 0x28
/// steps. Returns its node.
///
/// `magic_0434` 0x10009450 (355 bytes; 2 DLLs).
pub fn two_lights_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let a = g.read::<u32>(w(0x4)).wrapping_add(0x1590);
    g.write::<u32>(a + 0x80, 0);
    g.write::<u16>(a + 0x85, 0xffff);
    g.write::<u8>(a + 0x84, 0xff);
    g.write::<u8>(a + 0x87, 0x80);
    g.write::<u32>(a + 0x88, 0x14);
    g.write::<u16>(a + 0x98, 0x48);
    g.write::<u32>(a + 0x90, 0x1e);
    g.write::<u32>(a + 0x8c, 0x1e);
    g.write::<u32>(a + 0x94, 0);
    let first = take(g, img, img.rel32_target(f + 0x67), w(0x10));
    let mut head = g.read::<u32>(w(0x6d));
    g.write::<u32>(first + 0x14, a);
    g.write::<u32>(first + 4, 0);
    if head == 0 {
        head = first;
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0x98), first);
    } else {
        let tail = g.read::<u32>(w(0x83));
        g.write::<u32>(first, tail);
        g.write::<u32>(tail + 4, first);
    }
    g.write::<u32>(first + 8, first);
    g.write::<u32>(a + 0x94, 1);
    let b = g.read::<u32>(w(0xab)).wrapping_add(0x1630);
    g.write::<u32>(b + 0x81, 0x0080_ffff);
    g.write::<u8>(b + 0x80, 0xff);
    g.write::<u16>(b + 0x86, 0);
    g.write::<u8>(b + 0x85, 0);
    g.write::<u32>(b + 0x88, 0x33);
    g.write::<u16>(b + 0x98, 0x48);
    g.write::<u32>(b + 0x90, 0x1e);
    g.write::<u32>(b + 0x8c, 0x1e);
    g.write::<u32>(b + 0x94, 0);
    let second = take(g, img, img.rel32_target(f + 0x10d), w(0xb6));
    g.write::<u32>(second + 0x14, b);
    g.write::<u32>(second + 4, 0);
    // `head` is the list's head as read before the first append, or the
    // first node: the second node goes after the first one
    if head == 0 {
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x12d), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first + 4, second);
    }
    let c = g.read::<u32>(w(0x133)).wrapping_add(0x17d8);
    let k2 = g.read::<u32>(w(0x13b));
    let k1 = g.read::<u32>(w(0x143));
    g.write::<u32>(second + 8, second);
    g.write::<u32>(w(0x153), second);
    // push ebx, esi, edi; push 0x28; call: the callee's frame
    let esp = fr.esp.wrapping_sub(0x14);
    g.write::<u32>(esp + 4, 0x28);
    let callee = Frame {
        at: img.rel32_target(f + 0x158),
        esp,
        ecx: c,
        xmm: [0, u128::from(k1), u128::from(k2), 0],
        ..*fr
    };
    scale_task(g, img, &callee)
}

/// [`two_lights_start`] (the `rel32`s: two [`take`]s and [`scale_task`]).
pub const TWO_LIGHTS_START: Sig = Sig {
    len: 355,
    hash: 0x8bf0_0470_6ca2_2035,
    calls: &[
        (0x67, Callee::Sig(&TAKE)),
        (0x10d, Callee::Sig(&TAKE)),
        (0x158, Callee::Sig(&SCALE_TASK)),
    ],
    imports: &[],
};

/// The mean offset of the live targets added to a point (`ecx` = `out`,
/// a /GS frame): `v` = a copy of `G_V` (an immediate; slot 325, a local),
/// and for each of the `*G_N` targets `G_IDS[i]` (bytes) that slot 684
/// says is live, its battle position (slot 531, a local) minus `v` is
/// summed; `out = v + sum * (K / n)` (`n` the live count), `w = 1`.
/// Returns the last slot's `eax`.
///
/// `magic_0150` 0x1000afc0 (366 bytes, `ecx` = `out`; 2 DLLs).
pub fn targets_mean_offset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let out = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    let p = ebp.wrapping_sub(0x24);
    let mut eax = slot_call(g, img, f, (0x1d, 325), &[v, w(0x17)]).eax;
    let (mut sx, mut sy, mut sz) = (0.0f32, 0.0f32, 0.0f32);
    let mut n = 0i32;
    let mut i = 0i32;
    while i < g.read::<i32>(w(0x4c)) {
        let t = u32::from(g.read::<u8>(w(0x63).wrapping_add(i.cast_unsigned())));
        eax = slot_call(g, img, f, (0x69, 684), &[t]).eax;
        if eax != 0 {
            let t = u32::from(g.read::<u8>(w(0x83).wrapping_add(i.cast_unsigned())));
            eax = slot_call(g, img, f, (0x89, 531), &[t, p]).eax;
            let [vx, vy, vz] = [0u32, 4, 8].map(|o| g.read::<f32>(v + o));
            let dy = sub(g.read::<f32>(p + 4), vy);
            let dz = sub(g.read::<f32>(p + 8), vz);
            let dx = sub(g.read::<f32>(p), vx);
            n += 1;
            g.write::<f32>(p + 4, dy);
            sy = add(dy, sy);
            g.write::<f32>(p + 8, dz);
            sz = add(dz, sz);
            g.write::<f32>(p, dx);
            sx = add(dx, sx);
        }
        i += 1;
    }
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let k = div(g.read::<f32>(w(0x10c)), n as f32);
    let [vx, vy, vz] = [0u32, 4, 8].map(|o| g.read::<f32>(v + o));
    g.write::<f32>(out, add(vx, mul(k, sx)));
    g.write::<u32>(out + 0xc, 0x3f80_0000);
    g.write::<f32>(out + 4, add(vy, mul(k, sy)));
    g.write::<f32>(out + 8, add(vz, mul(k, sz)));
    Ret::eax(eax)
}

/// [`targets_mean_offset`] (the `rel32`: `__security_check_cookie`).
pub const TARGETS_MEAN_OFFSET: Sig = Sig {
    len: 366,
    hash: 0x2834_50da_5a8a_7566,
    calls: &[(0x166, Callee::Any)],
    imports: &[],
};

/// Three copies of character 0x10af made and three actors reset (cdecl,
/// no arguments): three times slot 0 `(0x10af)`, slot 26 `(0x10af, 1)`,
/// the character's byte `G_K[k] = 0x20 + k` and handle `G_CH[k]`, slot 10
/// `(ch, 0)`; each call sees the words the earlier ones left on the stack.
/// Then for the actors 0..3 of the state `*G_S`: [`actor_mode`] (`ecx` =
/// the actor, `edx` = its record `+0x18f0 + 0x40 i`), two vectors
/// `+0x19b0 + 0x10 i = 0` and `+0x19e0 + 0x10 i = (0, 0, 0, 1)`, and slot
/// 257 `(G_A[G_SEL[i]], 1)` (the original's `i == -1` case, `*G_ONE`,
/// cannot happen). Returns slot 257's `eax`.
///
/// `magic_0349` 0x10004980 (373 bytes; 2 DLLs).
pub fn three_charas<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let s = g.read::<u32>(w(0xb));
    let mut p = Pushes::default();
    for (k, (t0, t26, key, store, t10)) in [
        (0x1u32, 0x18u32, 0x31u32, 0x38u32, 0x2bu32),
        (0x45, 0x54, 0x6d, 0x74, 0x67),
        (0x81, 0x90, 0xa9, 0xb0, 0xa3),
    ]
    .into_iter()
    .enumerate()
    {
        p.push(0x10af);
        // call width: 12 (the most `Pushes::words` holds)
        let ch = p.call(g, slot_target(g, w(t0), 0), 0).eax;
        p.push_all(&[1, 0x10af]);
        // call width: 12 (the most `Pushes::words` holds)
        p.call(g, slot_target(g, w(t26), 26), 0);
        g.write::<u8>(w(key), 0x20 + k as u8);
        g.write::<u32>(w(store), ch);
        p.push_all(&[0, ch]);
        // call width: 12 (the most `Pushes::words` holds)
        p.call(g, slot_target(g, w(t10), 10), 0);
    }
    let mut eax = 0;
    for i in 0..3u32 {
        // push esi, edi; call: the callee's frame
        let callee = Frame {
            at: img.rel32_target(f + 0xd1),
            esp: fr.esp.wrapping_sub(0xc),
            ecx: i,
            edx: s.wrapping_add(0x18f0 + 0x40 * i),
            ..*fr
        };
        actor_mode(g, img, &callee);
        for (o, v) in [
            (0x19b0u32, 0u32),
            (0x19b4, 0),
            (0x19b8, 0),
            (0x19bc, 0),
            (0x19e0, 0),
            (0x19e4, 0),
            (0x19e8, 0),
            (0x19ec, 0x3f80_0000),
        ] {
            g.write::<u32>(s + o + 0x10 * i, v);
        }
        let sel = u32::from(g.read::<u8>(g.read::<u32>(w(0x146)).wrapping_add(i)));
        let a = g.read::<u32>(w(0x151).wrapping_add(4 * sel));
        eax = slot_call(g, img, f, (0x159, 257), &[a, 1]).eax;
    }
    Ret::eax(eax)
}

/// [`three_charas`] (the `rel32`: [`actor_mode`]).
pub const THREE_CHARAS: Sig = Sig {
    len: 373,
    hash: 0x1546_aef2_1279_1887,
    calls: &[(0xd1, Callee::Sig(&ACTOR_MODE))],
    imports: &[],
};

/// A fading model task (cdecl `(task)`, `task[0]` its list node,
/// `task[1]` the record: state `+0x18`, timer `+0x34`, model `+0x44`,
/// fade byte `+0xc3`). The state (a jump table in the original):
/// - 0: the speeds `+0x30 = -0x9c40`, `+0x38 = 0x13880`, accelerations
///   `+0x3c`, `+0x40` cleared, the shade `(3, 3, 3)` over 0x3c frames
///   (slot 256 on the model's `+0x54`), then state 1 at once;
/// - 1: the fade byte up by 4; at 0x40 state 2 with the timer 0x28;
/// - 2: when the timer has run out, the shade back to `(1, 1, 1)` over
///   0x28 frames and state 3;
/// - 3: the fade byte down by 4; at 0 state 10 at once;
/// - 10: bit 4 of u16 `+0xec` set, the timer 2, state 11 at once;
/// - 11: when the timer has run out, the node unlinked and state 0x4e20.
///
/// Every step then moves the record: `+0xe0 += +0x30`, `+0xe4 += +0x38`,
/// the speeds grow by the accelerations, the timer counts down. Returns
/// the new `+0x38`.
///
/// `magic_0578` 0x10007e30 (376 bytes; 2 DLLs).
pub fn fading_model_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let shade = |g: &mut G, off: u32, v: u32, frames: u32| {
        let m = g.read::<u32>(g.read::<u32>(r + 0x44).wrapping_add(0x54));
        slot_call(g, img, f, (off, 256), &[m, v, v, v, frames]);
    };
    let mut state = g.read::<u32>(r + 0x18);
    loop {
        match state {
            0 | 1 => {
                if state == 0 {
                    g.write::<u32>(r + 0x30, 0xffff_63c0);
                    g.write::<u32>(r + 0x38, 0x13880);
                    g.write::<u32>(r + 0x3c, 0);
                    g.write::<u32>(r + 0x40, 0);
                    shade(g, 0x77, 0x4040_0000, 0x3c);
                    bump(g, r + 0x18);
                }
                let b = g.read::<u8>(r + 0xc3).wrapping_add(4);
                g.write::<u8>(r + 0xc3, b);
                if b == 0x40 {
                    bump(g, r + 0x18);
                    g.write::<u32>(r + 0x34, 0x28);
                }
            }
            2 => {
                if g.read::<i32>(r + 0x34) <= 0 {
                    shade(g, 0xd2, 0x3f80_0000, 0x28);
                    bump(g, r + 0x18);
                }
            }
            3 => {
                let b = g.read::<u8>(r + 0xc3).wrapping_add(0xfc);
                g.write::<u8>(r + 0xc3, b);
                if b == 0 {
                    g.write::<u32>(r + 0x18, 10);
                    state = 10;
                    continue;
                }
            }
            10 | 11 => {
                if state == 10 {
                    let v = g.read::<u16>(r + 0xec) | 0x10;
                    g.write::<u16>(r + 0xec, v);
                    bump(g, r + 0x18);
                    g.write::<u32>(r + 0x34, 2);
                }
                if g.read::<i32>(r + 0x34) <= 0 {
                    let node = g.read::<u32>(task);
                    let prev = g.read::<u32>(node + 4);
                    let next = g.read::<u32>(node + 8);
                    g.write::<u32>(prev + 8, next);
                    let next = g.read::<u32>(node + 8);
                    let prev = g.read::<u32>(node + 4);
                    g.write::<u32>(next + 4, prev);
                    g.write::<u32>(node + 8, node);
                    g.write::<u32>(node + 4, node);
                    g.write::<u32>(r + 0x18, 0x4e20);
                }
            }
            _ => {}
        }
        break;
    }
    let vx = g.read::<u32>(r + 0x30);
    let ax = g.read::<u32>(r + 0x3c);
    let vy = g.read::<u32>(r + 0x38);
    let x = g.read::<u32>(r + 0xe0);
    g.write::<u32>(r + 0xe0, x.wrapping_add(vx));
    let y = g.read::<u32>(r + 0xe4);
    g.write::<u32>(r + 0xe4, y.wrapping_add(vy));
    g.write::<u32>(r + 0x30, ax.wrapping_add(vx));
    let vy = g.read::<u32>(r + 0x40).wrapping_add(vy);
    let t = g.read::<u32>(r + 0x34);
    g.write::<u32>(r + 0x34, t.wrapping_sub(1));
    g.write::<u32>(r + 0x38, vy);
    Ret::eax(vy)
}

/// [`fading_model_task`].
pub const FADING_MODEL_TASK: Sig = Sig {
    len: 376,
    hash: 0x632a_e5e3_4338_5219,
    calls: &[],
    imports: &[],
};

/// A target's record measured (`ecx` = the record, `+0` the target, a
/// /GS frame; nothing for target 0xff, which it returns): its handle
/// (slot 215) into `+4`, its position (slot 217) into `+0x2c` and a copy
/// at `+0x3c`, its root matrix (slot 210) into `+0x4c` and a copy at
/// `+0x8c` (slot 230), its battle direction (slot 169) `* K1 / K2`
/// truncated into the halves `+0x14`/`+0x16`, its width (slot 251) into
/// `+0x1c` and height (slot 216) into `+0x20`; the larger one `r` gives
/// `+0x28 = KD / r` (in double) above `K3`, else 0.001; then that times
/// the scale's `y` (slot 290, a local vector) times the data scale (slot
/// 277) into `+0x28` and `+0x24`. Returns slot 277's `eax`.
///
/// `magic_0667` 0x100029a0 (380 bytes, `ecx` = the record; 2 DLLs).
pub fn target_measure<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let kd = g.read::<f64>(op(img, f, 0x10e));
    measure(g, img, fr, (0x130, 0x13f), |_, big| {
        (kd / f64::from(big)) as f32
    })
}

/// [`target_measure`]'s steps, with the inverse radius of the larger size
/// above `K3` from `inverse` (the tail's operands: the table loads of the
/// slot 290 and 277 calls).
fn measure<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    (t290, t277): (u32, u32),
    inverse: impl FnOnce(&G, f32) -> f32,
) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let r = fr.ecx;
    let id = g.read::<u32>(r);
    if id == 0xff {
        return Ret::eax(id);
    }
    let h = slot_call(g, img, f, (0x23, 215), &[id]).eax;
    g.write::<u32>(r + 4, h);
    let pos = r.wrapping_add(0x2c);
    let id = g.read::<u32>(r);
    slot_call(g, img, f, (0x33, 217), &[id, pos, id]);
    let lo = g.read::<u64>(pos);
    g.write::<u64>(r + 0x3c, lo);
    let hi = g.read::<u64>(pos + 8);
    let m = r.wrapping_add(0x4c);
    let h = g.read::<u32>(r + 4);
    g.write::<u64>(r + 0x44, hi);
    slot_call(g, img, f, (0x4a, 210), &[h, m, id, pos, id]);
    slot_call(
        g,
        img,
        f,
        (0x75, 230),
        &[r.wrapping_add(0x8c), m, h, m, id, pos, id],
    );
    let id = g.read::<u32>(r);
    let dir = st0_f32(slot_call(g, img, f, (0x82, 169), &[id]));
    let a = div(mul(dir, g.read::<f32>(w(0x9c))), g.read::<f32>(w(0xa6)));
    let a = trunc(a) as u16;
    g.write::<u16>(r + 0x16, a);
    g.write::<u16>(r + 0x14, a);
    let id = g.read::<u32>(r);
    let width = st0_f32(slot_call(g, img, f, (0xb7, 251), &[id]));
    g.write::<f32>(r + 0x1c, width);
    let id2 = g.read::<u32>(r);
    let height = st0_f32(slot_call(g, img, f, (0xc7, 216), &[id2, id]));
    let width = g.read::<f32>(r + 0x1c);
    g.write::<f32>(r + 0x20, height);
    // comiss width, height; jbe: the height unless the width is larger
    let big = if width > height { width } else { height };
    g.write::<f32>(r + 0x28, big);
    let big = g.read::<f32>(r + 0x28);
    if big > g.read::<f32>(w(0x104)) {
        let k = inverse(g, big);
        g.write::<f32>(r + 0x28, k);
    } else {
        g.write::<u32>(r + 0x28, 0x3a83_126f);
    }
    let scale = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
    let id = g.read::<u32>(r);
    slot_call(g, img, f, (t290, 290), &[id, scale]);
    let h = g.read::<u32>(r + 4);
    let out = slot_call(g, img, f, (t277, 277), &[h, id, scale]);
    let s = st0_f32(out);
    let x = mul(mul(g.read::<f32>(scale + 4), s), g.read::<f32>(r + 0x28));
    g.write::<f32>(r + 0x28, x);
    g.write::<f32>(r + 0x24, x);
    Ret::eax(out.eax)
}

/// [`target_measure`] (the `rel32`: `__security_check_cookie`).
pub const TARGET_MEASURE: Sig = Sig {
    len: 380,
    hash: 0x4497_dde3_b042_2dfb,
    calls: &[(0x174, Callee::Any)],
    imports: &[],
};

/// A weapon's matrix built and set (internal: `ecx` = three angles,
/// `edx` = a position; `esp` the entry's; an aligned /GS frame): slot 562,
/// the weapon `w` of `*G_CH` (slot 219 `(ch, 0)`), the local matrix `M`
/// made a unit one (slot 235), its diagonal the data scale (slot 277)
/// times the weapon's own scales (slots 15, 16, 17; ST0 each), turned
/// about z, x and y by the angles `+8`, `+0`, `+4` (slots 330, 328, 329),
/// its last row the position, and slot 913 `(slot 825 (w), M, -1)`.
/// Returns slot 913's `eax`.
///
/// `magic_0555` 0x100052a0 (422 bytes; 2 DLLs).
pub fn weapon_matrix<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, ang: u32, pos: u32) -> u32 {
    let w = |o: u32| op(img, f, o);
    // push ebp; and esp, -8; sub esp, 0x50
    let top = esp.wrapping_sub(4) & !7;
    let m = top.wrapping_sub(0x48);
    let at = |i: u32| m + 4 * i;
    g.write::<u32>(top.wrapping_sub(0x4c), pos);
    slot_call(g, img, f, (0x15, 562), &[]);
    let ch = g.read::<u32>(w(0x32));
    let weapon = slot_call(g, img, f, (0x2a, 219), &[ch, 0]).eax;
    slot_call(g, img, f, (0x40, 235), &[m, ch, 0]);
    let s = st0_f32(slot_call(g, img, f, (0x55, 277), &[weapon, m, ch, 0]));
    for i in [10u32, 5, 0] {
        g.write::<f32>(at(i), s);
    }
    for (i, (table, slot)) in [(0u32, (0x6du32, 15u32)), (5, (0x9a, 16)), (10, (0xbb, 17))] {
        let k = st0_f32(slot_call(g, img, f, (table, slot), &[weapon]));
        let v = mul(g.read::<f32>(at(i)), k);
        g.write::<f32>(at(i), v);
    }
    for (table, slot, a) in [(0xf5u32, 330u32, 8u32), (0x114, 328, 0), (0x134, 329, 4)] {
        let x = g.read::<u32>(ang + a);
        slot_call(g, img, f, (table, slot), &[m, m, x]);
    }
    let pos = g.read::<u32>(top.wrapping_sub(0x4c));
    for i in 0..4u32 {
        let v = g.read::<u32>(pos + 4 * i);
        g.write::<u32>(at(12 + i), v);
    }
    let x = slot_call(g, img, f, (0x16a, 825), &[weapon]).eax;
    slot_call(g, img, f, (0x186, 913), &[x, m, u32::MAX]).eax
}

/// [`weapon_matrix`] (the `rel32`: `__security_check_cookie`).
pub const WEAPON_MATRIX: Sig = Sig {
    len: 422,
    hash: 0xfe78_a60e_b923_9efc,
    calls: &[(0x19e, Callee::Any)],
    imports: &[],
};

/// A weapon's swing step (cdecl `(task)`, `task[5]` the object, a /GS
/// frame): the count `+0x50c` down to 0, `t = count / +0x510`, the
/// position `p = +0x270..+0x278 * (K - t) + +0x260..+0x268 * t` (`w` 1, a
/// local) through [`weapon_matrix`] with the angles `+0x280`, and `p`
/// into `+0xd0..+0xdc`. Returns [`weapon_matrix`]'s `eax` while the count
/// runs; at 0 the object's child starts ([`start_child_b`] with `*G_S`'s
/// `+0x8524`), the object leaves the DLL's list ([`list_remove`]; when it
/// was there, `*G_P = ITEM`, its parts are freed (slot 713) and bit 15 of
/// u16 `+0x124` cleared), `*G_S`'s `+0x852c` counts it and the task's
/// node goes back to its pool ([`give_back`]), whose `eax` it returns.
///
/// `magic_0555` 0x10002a90 (391 bytes; 2 DLLs).
pub fn weapon_swing<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let task = fr.arg(g, 0);
    let obj = g.read::<u32>(task + 0x14);
    let n = g.read::<i32>(obj + 0x50c);
    let s = g.read::<u32>(w(0x21));
    if n > 0 {
        g.write::<i32>(obj + 0x50c, n - 1);
    }
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let t = div(
        g.read::<i32>(obj + 0x50c) as f32,
        g.read::<i32>(obj + 0x510) as f32,
    );
    let u = sub(g.read::<f32>(w(0x4a)), t);
    let mix = |g: &G, a: u32, b: u32| {
        add(
            mul(g.read::<f32>(obj + a), u),
            mul(g.read::<f32>(obj + b), t),
        )
    };
    let (x, y, z) = (
        mix(g, 0x270, 0x260),
        mix(g, 0x274, 0x264),
        mix(g, 0x278, 0x268),
    );
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x18);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    for (o, v) in [(0u32, x), (4, y), (8, z)] {
        g.write::<f32>(p + o, v);
    }
    // push ecx; call: the callee's frame is 0x40 below the entry
    let eax = weapon_matrix(
        g,
        img,
        img.rel32_target(f + 0xda),
        fr.esp.wrapping_sub(0x40),
        obj.wrapping_add(0x280),
        p,
    );
    for (o, v) in [(0xd0u32, x), (0xd4, y), (0xd8, z)] {
        g.write::<f32>(obj + o, v);
    }
    g.write::<u32>(obj + 0xdc, 0x3f80_0000);
    if g.read::<i32>(obj + 0x50c) > 0 {
        return Ret::eax(eax);
    }
    // push the word; call: the callee's frame is 0x40 below the entry
    let esp = fr.esp.wrapping_sub(0x40);
    g.write::<u32>(esp + 4, g.read::<u32>(s + 0x8524));
    let child = Frame {
        at: img.rel32_target(f + 0x12a),
        esp,
        ecx: obj,
        edx: obj.wrapping_add(0x280),
        ..*fr
    };
    start_child_b(g, img, &child);
    if list_remove(g, img, img.rel32_target(f + 0x139), w(0x134), obj) != 0 {
        let p = g.read::<u32>(w(0x142));
        g.write::<u32>(p, w(0x149));
        slot_call(g, img, f, (0x14e, 713), &[obj]);
        let v = g.read::<u16>(obj + 0x124) & 0x7fff;
        g.write::<u16>(obj + 0x124, v);
    }
    bump(g, s + 0x852c);
    Ret::eax(give_back(g, img, img.rel32_target(f + 0x172), task))
}

/// [`weapon_swing`] (the `rel32`s: [`weapon_matrix`], [`start_child_b`],
/// [`list_remove`], [`give_back`], `__security_check_cookie`).
pub const WEAPON_SWING: Sig = Sig {
    len: 391,
    hash: 0x4a9f_6e7c_ba4e_2b95,
    calls: &[
        (0xda, Callee::Sig(&WEAPON_MATRIX)),
        (0x12a, Callee::Sig(&START_CHILD_B)),
        (0x139, Callee::Sig(&LIST_REMOVE)),
        (0x172, Callee::Sig(&GIVE_BACK)),
        (0x17f, Callee::Any),
    ],
    imports: &[],
};

/// The targets' shade reset and a colour task started (cdecl, no
/// arguments): every target `G_A[G_SEL[i]]` of the `*G_N` selected gets
/// the shade `(1, 1, 1)` over 0x3c frames (slot 256; the original's `i ==
/// -1` case, `*G_ONE`, cannot happen). The state `*G_S`'s colour record
/// `+0x5a30` gets `(0.5, 0.5, 0.5, 1)`, `(1, 1, 1, 1)`, the count 0x32,
/// `+0x34 = 0`, `+0x2c`/`+0x30 = 0x1e`, `G_A = 0.5`, `G_B = 1.0`; a node
/// taken from the free ring (`*G_FREE`, its count u16 `*G_COUNT`; the
/// original inlines the take: unlinked, cleared, its callback `CB` an
/// immediate) runs it (kind 1), appended to the task list. Returns the
/// list's old tail, or what the original leaves in `eax` (0xffff after a
/// take, else slot 256's `eax` or the entry's).
///
/// `magic_0165` 0x10003410 (395 bytes; 2 DLLs).
pub fn shade_reset_colour_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let s = g.read::<u32>(w(0x6));
    let mut eax = fr.eax;
    let mut i = 0i32;
    while i < g.read::<i32>(w(0xc)) {
        let sel = u32::from(g.read::<u8>(g.read::<u32>(w(0x1f)).wrapping_add(i.cast_unsigned())));
        let a = g.read::<u32>(w(0x2a).wrapping_add(4 * sel));
        let one = 0x3f80_0000;
        eax = slot_call(g, img, f, (0x4c, 256), &[a, one, one, one, 0x3c]).eax;
        i += 1;
    }
    let r = s.wrapping_add(0x5a30);
    let mut count = g.read::<u16>(w(0x6d));
    g.write::<u32>(w(0x73), 0x3f00_0000);
    g.write::<u32>(w(0x7d), 0x3f80_0000);
    for (o, v) in [
        (8u32, 0x3f00_0000u32),
        (4, 0x3f00_0000),
        (0, 0x3f00_0000),
        (0xc, 0x3f80_0000),
        (0x18, 0x3f80_0000),
        (0x14, 0x3f80_0000),
        (0x10, 0x3f80_0000),
        (0x1c, 0x3f80_0000),
        (0x28, 0x32),
        (0x34, 0),
        (0x30, 0x1e),
        (0x2c, 0x1e),
    ] {
        g.write::<u32>(r + o, v);
    }
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0xe3));
        let prev = g.read::<u32>(node);
        let next = g.read::<u32>(node + 4);
        g.write::<u32>(prev + 4, next);
        let next = g.read::<u32>(node + 4);
        let prev = g.read::<u32>(node);
        g.write::<u32>(next, prev);
        if count > 1 {
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(w(0x100), next);
        } else {
            g.write::<u32>(w(0x108), 0);
        }
        eax = 0xffff;
        count = count.wrapping_sub(1);
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node + o, 0);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 0x10, w(0x139));
        g.write::<u8>(node + 0x18, 0);
        g.write::<u16>(w(0x144), count);
        node
    };
    let tail = g.read::<u32>(w(0x14a));
    g.write::<u32>(node.wrapping_add(0x14), r);
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    if tail != 0 {
        let last = g.read::<u32>(w(0x162));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
        g.write::<u32>(w(0x16d), node);
        g.write::<u32>(node.wrapping_add(8), node);
        return Ret::eax(last);
    }
    g.write::<u32>(node, 0);
    g.write::<u32>(w(0x17d), node);
    g.write::<u32>(w(0x183), node);
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(eax)
}

/// [`shade_reset_colour_task`].
pub const SHADE_RESET_COLOUR_TASK: Sig = Sig {
    len: 395,
    hash: 0xf03a_2a6d_3a38_b631,
    calls: &[],
    imports: &[],
};

/// An effect's end: its character shown again (cdecl, no arguments; an
/// aligned frame): slot 6 `(*G_CH, 0)`, the draw flags restored (slot
/// 547), the shadow back (slot 295 `(*G_ID, -1)`), the transparency 1.0
/// (slot 541 `(*G_ID, 1.0, 1)` for a party member `*G_ID < 0x20` without
/// `*G_ALT`, else slot 38 `(*G_CH, 1.0, 1)`), the held node given back
/// ([`give_back`]) and `*G_NODE = 0`, slots 568 `(1)`, 566 `(1)`, 574,
/// 546 `(*G_CH)` and 271 `(1)`; then for the four parts of `*G_S +
/// 0xea8` (0x8a8 bytes each) each of their `+0x824` models is released
/// (slot 779 with slot 772 `(slot 941 (part[0]), (k << 16) + i)`'s
/// 64-bit handle) with the lock `*G_L` held at 1. Returns the lock's
/// address after a release, else slot 271's `eax`.
///
/// `magic_0168` 0x10001cc0 (407 bytes; 2 DLLs).
pub fn effect_end_show<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let ch = g.read::<u32>(w(0x13));
    slot_call(g, img, f, (0x8, 6), &[ch, 0]);
    slot_call(g, img, f, (0x1e, 547), &[ch, ch, 0]);
    let id = g.read::<u32>(w(0x3d));
    slot_call(g, img, f, (0x2c, 295), &[id, u32::MAX]);
    let one = 0x3f80_0000;
    let id = g.read::<i32>(w(0x50));
    if g.read::<u32>(w(0x48)) == 0 && id < 0x20 {
        slot_call(g, img, f, (0x65, 541), &[id.cast_unsigned(), one, 1]);
    } else {
        let ch = g.read::<u32>(w(0x88));
        slot_call(g, img, f, (0x72, 38), &[ch, one, 1]);
    }
    let held = g.read::<u32>(w(0x90));
    if held != 0 {
        give_back(g, img, img.rel32_target(f + 0x9c), held);
        g.write::<u32>(w(0xa2), 0);
    }
    slot_call(g, img, f, (0xab, 568), &[1]);
    slot_call(g, img, f, (0xba, 566), &[1]);
    slot_call(g, img, f, (0xcc, 574), &[]);
    let ch = g.read::<u32>(w(0xe2));
    slot_call(g, img, f, (0xdc, 546), &[ch]);
    let mut eax = slot_call(g, img, f, (0xef, 271), &[1]).eax;
    let mut part = g.read::<u32>(w(0x102)).wrapping_add(0xea8);
    for k in 0..4u32 {
        let mut i = 0i32;
        while i < g.read::<i32>(part + 0x824) {
            let d = g.read::<u32>(part);
            let t = slot_call(g, img, f, (0x121, 941), &[d]).eax;
            let h = slot_call(
                g,
                img,
                f,
                (0x138, 772),
                &[t, (k << 16).wrapping_add(i.cast_unsigned())],
            );
            let lock = g.read::<u32>(w(0x146));
            let saved = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            let args = [h.eax, h.edx, t, (k << 16).wrapping_add(i.cast_unsigned())];
            slot_call(g, img, f, (0x14f, 779), &args);
            eax = g.read::<u32>(w(0x162));
            g.write::<u32>(eax, saved);
            i += 1;
        }
        part = part.wrapping_add(0x8a8);
    }
    Ret::eax(eax)
}

/// [`effect_end_show`] (the `rel32`: [`give_back`]).
pub const EFFECT_END_SHOW: Sig = Sig {
    len: 407,
    hash: 0x89a7_9b28_c818_d798,
    calls: &[(0x9c, Callee::Sig(&GIVE_BACK))],
    imports: &[],
};

/// The screen fade stepped (cdecl, no arguments; a /GS frame): the level
/// `*G_X` rises by `K_UP` while `*G_T > 0xaf`, else falls by `K_DN`, held
/// at `LO` from below; rising, it ends at `HI`: above it the level becomes
/// 1.0 and the function returns 0, exactly at it it returns 0 too.
/// Falling, it is held at `HI` from above. Then the colour scale `(x, x,
/// x, x)` (slot 353) and the offset `(0x80, 0x80, 0x80, 0x80)` (slot 397;
/// locals of the frame), `y = max(*G_X, LO)` (`LO` when `LO >= y`), slot
/// 801 with six times `y / *G_Z`, `*G_Z = y` and slot 785 `(K * y)`.
/// Returns 1.
///
/// `magic_0188` 0x100023b0 (423 bytes; 2 DLLs).
pub fn fade_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let lo = k(g, 0x26);
    let mut x = k(g, 0x1e);
    if g.read::<i32>(w(0x12)) > 0xaf {
        x = add(x, k(g, 0x30));
        g.write::<f32>(w(0x3b), x);
        if lo > x {
            x = lo;
            g.write::<f32>(w(0xaa), x);
        } else {
            let hi = k(g, 0x45);
            if x > hi {
                g.write::<u32>(w(0x50), 0x3f80_0000);
                return Ret::eax(0);
            }
            // ucomiss; lahf; test ah, 0x44; jp: on only when not equal
            #[allow(clippy::float_cmp)] // the original's exact test
            if x == hi {
                return Ret::eax(0);
            }
        }
    } else {
        x = sub(x, k(g, 0x85));
        g.write::<f32>(w(0x90), x);
        if lo > x {
            x = lo;
            g.write::<f32>(w(0xaa), x);
        } else {
            let hi = k(g, 0x9a);
            if x > hi {
                x = hi;
                g.write::<f32>(w(0xaa), x);
            }
        }
    }
    let ebp = fr.esp.wrapping_sub(4);
    let colour = ebp.wrapping_sub(0x14);
    let offset = ebp.wrapping_sub(0x24);
    for i in 0..4u32 {
        g.write::<f32>(colour + 4 * i, x);
        g.write::<u32>(offset + 4 * i, 0x80);
    }
    slot_call(g, img, f, (0xb3, 353), &[colour]);
    slot_call(g, img, f, (0xf4, 397), &[offset, colour]);
    let y = k(g, 0x104);
    let m = k(g, 0x10c);
    // comiss m, y; jb: `y` kept when above `m` or unordered
    let y = if m >= y { m } else { y };
    let z = div(y, k(g, 0x12c));
    slot_call(g, img, f, (0x131, 801), &[z.to_bits(); 6]);
    let v = mul(k(g, 0x16c), y);
    g.write::<f32>(w(0x186), y);
    slot_call(g, img, f, (0x171, 785), &[v.to_bits()]);
    Ret::eax(1)
}

/// [`fade_step`] (the `rel32`s: three `__security_check_cookie`s).
pub const FADE_STEP: Sig = Sig {
    len: 423,
    hash: 0x4b80_e637_ec22_c6e6,
    calls: &[
        (0x60, Callee::Any),
        (0x79, Callee::Any),
        (0x19f, Callee::Any),
    ],
    imports: &[],
};

/// `b` brought within `PI` of `a` (the overlays' `comiss` pair, each
/// `jbe` taken when below, equal or unordered): `b + TAU` when `a > b`
/// by more than `PI`, `b - TAU` when `b > a` by more than `PI`.
fn near_angle(a: f32, b: f32, pi: f32, tau: f32) -> f32 {
    if a > b {
        if sub(a, b) > pi { add(b, tau) } else { b }
    } else if sub(b, a) > pi {
        sub(b, tau)
    } else {
        b
    }
}

/// An angle key curve sampled (internal: `ecx` = six keys 16 bytes apart,
/// `xmm0` = the time `t`; the result in `xmm0`): the keys around `i =
/// trunc(t)` (each `+ PI`, brought within `PI` of its neighbour), eased
/// with the one before (the first half of a step, `frac < K_H`) or after
/// it, then `a + (b - a) / (K / frac)` (`a` when `frac` is 0), wrapped
/// into `[-PI, PI)` by whole turns. Returns the angle and what the
/// original leaves in `eax` (the turns).
///
/// `magic_0055` 0x100031a0 (427 bytes; 2 DLLs).
pub fn angle_keys<G: Game>(g: &mut G, img: &Image, f: u32, keys: u32, t: f32) -> (f32, u32) {
    let k = |g: &G, o: u32| g.read::<f32>(op(img, f, o));
    let (pi, tau) = (k(g, 0x4), k(g, 0xc));
    let key = |g: &G, i: i32| {
        add(
            g.read::<f32>(keys.wrapping_add((16 * i).cast_unsigned())),
            pi,
        )
    };
    let i = trunc(t).cast_signed();
    let mut a = key(g, i);
    let mut b = if i == 5 {
        a
    } else {
        let b = key(g, i.wrapping_add(1));
        near_angle(a, b, pi, tau)
    };
    let half = k(g, 0x67);
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let frac = sub(t, i as f32);
    if half > frac {
        if i != 0 {
            let c = near_angle(a, key(g, i.wrapping_sub(1)), pi, tau);
            let c = sub(mul(add(c, b), half), a);
            let d = div(k(g, 0xba), sub(half, frac));
            a = add(div(c, d), a);
        }
    } else {
        let j = if i >= 4 { 5 } else { i + 2 };
        let d = near_angle(b, key(g, j), pi, tau);
        let u = sub(frac, k(g, 0x12f));
        let d = add(d, a);
        let x = div(k(g, 0x124), u);
        let d = sub(mul(d, k(g, 0x13f)), b);
        b = add(b, div(d, x));
    }
    // ucomiss frac, 0; lahf; test ah, 0x44; jnp: equal goes to 0
    let step = if frac == 0.0 {
        0.0
    } else {
        div(sub(b, a), div(k(g, 0x15f), frac))
    };
    let mut r = add(step, a);
    let turns = trunc(div(r, tau));
    if turns.cast_signed() > 0 {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
        let whole = turns.cast_signed() as f32;
        r = sub(r, mul(whole, tau));
    }
    let r = if 0.0 > r { add(r, pi) } else { sub(r, pi) };
    (r, turns)
}

/// [`angle_keys`].
pub const ANGLE_KEYS: Sig = Sig {
    len: 427,
    hash: 0x68c2_d9a8_e212_ef37,
    calls: &[],
    imports: &[],
};

/// A summon's particle start (cdecl `(actor)`): the targets
/// ([`targets_init`]), the DLL's node rings and records set up (their
/// pointers and callbacks immediates of the code), the effect data `E`
/// through slot 718, its part data `D` = slot 719 `(E, 8, 0)` into
/// `*G_D` and initialised (slot 711 `(D, G_I, 0)` with the lock `*G_L`
/// held at 1), the environment (slot 710 `(G_ENV, D, G_B, 0x80000)`),
/// slots 222 and 221 `(E, 0, 1000, 2)` ([`summon_ppp_head`]); each call
/// sees the words the earlier ones left on the stack. Then the record
/// `G_R` goes in front
/// of the ring `G_RING` (its `+4` and the ring's tail), bit 0 of the
/// ring's flags cleared, `G_DONE = 0`. Returns 0.
///
/// `magic_0342` 0x100071b0 (437 bytes; 2 DLLs).
pub fn summon_ppp_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    summon_ppp_head(g, img, fr);
    let tail = g.read::<u32>(w(0x15a));
    g.write::<u32>(w(0x162), tail);
    for (dst, v) in [
        (0x168u32, 0x16cu32),
        (0x172, 0x176),
        (0x17c, 0x180),
        (0x186, 0x18a),
    ] {
        g.write::<u32>(w(dst), w(v));
    }
    g.write::<u32>(tail.wrapping_add(4), w(0x191));
    let flags = g.read::<u32>(w(0x197)) & 0xffff_fffe;
    g.write::<u32>(w(0x197), flags);
    g.write::<u32>(w(0x19e), w(0x1a2));
    g.write::<u32>(w(0x1a8), 0);
    Ret::eax(0)
}

/// The code [`summon_ppp_start`] and [`summon_ppp_start_stream`] share
/// (through `+0x158`, the slot 221 call; the environment's size is the
/// immediate at `+0x100`, the kind slot 719 gets the byte at `+0xc2`).
/// Returns the words left on the stack.
fn summon_ppp_head<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Pushes {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    // push ebp, esi; call: the callee's frame
    targets_init(
        g,
        img,
        img.rel32_target(f + 0x8),
        fr.esp.wrapping_sub(0xc),
        fr.arg(g, 0),
    );
    for (dst, v) in [
        (0x13u32, w(0x17)),
        (0x1d, w(0x21)),
        (0x27, w(0x2b)),
        (0x31, w(0x35)),
        (0x3b, 1),
        (0x45, w(0x49)),
    ] {
        g.write::<u32>(w(dst), v);
    }
    g.write::<u16>(w(0x50), 0x1ff);
    for (dst, v) in [
        (0x58u32, w(0x5c)),
        (0x62, w(0x66)),
        (0x6c, 0),
        (0x76, w(0x7a)),
    ] {
        g.write::<u32>(w(dst), v);
    }
    g.write::<u16>(w(0x81), 0xff);
    for (dst, v) in [
        (0x89u32, w(0x8d)),
        (0x93, w(0x97)),
        (0x9d, 0),
        (0xb2, w(0xb6)),
    ] {
        g.write::<u32>(w(dst), v);
    }
    let mut s = Pushes::default();
    s.push(w(0xac));
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xd), 718), 0);
    // push imm8 at +0xc1: the part data's kind (0 or 3 so far)
    let kind = u32::from(img.word(f + 0xc2) as u8);
    s.push_all(&[kind, 8, w(0xcc)]);
    // call width: 12 (the most `Pushes::words` holds)
    let d = s.call(g, slot_target(g, w(0xbd), 719), 0).eax;
    let lock = g.read::<u32>(w(0xd4));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0xdd), d);
    g.write::<u32>(lock, 1);
    s.push_all(&[0, w(0xe2), d]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xe8), 711), 0);
    let lock = g.read::<u32>(w(0xfb));
    g.write::<u32>(lock, saved);
    let d = g.read::<u32>(w(0x112));
    s.push_all(&[w(0x100), w(0x10c), d, w(0x11d)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x107), 710), 0);
    s.push_all(&[2, 1000, 0, w(0x138)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x124), 222), 0);
    s.push_all(&[2, 1000, 0, w(0x153)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x13f), 221), 0);
    s
}

/// [`summon_ppp_start`] (the `rel32`: [`targets_init`]).
pub const SUMMON_PPP_START: Sig = Sig {
    len: 437,
    hash: 0x1700_3c8f_ff91_dc8e,
    calls: &[(0x8, Callee::Sig(&TARGETS_INIT))],
    imports: &[],
};

/// [`summon_ppp_start`] as `magic_0411` binds it (part data kind 3, an
/// environment of 0x100000 bytes; 437 bytes, 3 DLLs).
pub fn summon_ppp_start_k3<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    summon_ppp_start(g, img, fr)
}

/// [`summon_ppp_start_k3`] (`magic_0411` 0x100075e0; the `rel32`:
/// [`targets_init`]).
pub const SUMMON_PPP_START_K3: Sig = Sig {
    len: 437,
    hash: 0xdffb_4020_9afe_e0bf,
    calls: &[(0x8, Callee::Sig(&TARGETS_INIT))],
    imports: &[],
};

/// [`summon_ppp_start`] as `magic_0272` binds it (an environment of
/// 0x200000 bytes; 437 bytes, 3 DLLs).
pub fn summon_ppp_start_big<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    summon_ppp_start(g, img, fr)
}

/// [`summon_ppp_start_big`] (`magic_0272` 0x10007130; the `rel32`:
/// [`targets_init`]).
pub const SUMMON_PPP_START_BIG: Sig = Sig {
    len: 437,
    hash: 0xc2ad_10ae_ba75_eeb6,
    calls: &[(0x8, Callee::Sig(&TARGETS_INIT))],
    imports: &[],
};

/// [`summon_ppp_start`] with an environment of 0x100000 bytes and one
/// more flag cleared at the end (`*G_F = 0`).
///
/// `magic_0071` 0x100091f0 (447 bytes; 8 DLLs).
pub fn summon_ppp_start_flag<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let r = summon_ppp_start(g, img, fr);
    g.write::<u32>(op(img, fr.at, 0x1b2), 0);
    r
}

/// [`summon_ppp_start_flag`] (the `rel32`: [`targets_init`]).
pub const SUMMON_PPP_START_FLAG: Sig = Sig {
    len: 447,
    hash: 0x5e21_50da_30c8_764e,
    calls: &[(0x8, Callee::Sig(&TARGETS_INIT))],
    imports: &[],
};

/// A slot thunk's `Sig` (`mov eax, [G]; mov eax, [eax + 4 * slot]; jmp
/// eax`, 13 bytes; the slot offset is part of the bytes).
const fn thunk_sig(hash: u64) -> Sig {
    Sig {
        len: 13,
        hash,
        calls: &[],
        imports: &[],
    }
}

/// The thunk of slot 421 (`FlushCache`).
pub const THUNK_421: Sig = thunk_sig(0xcccc_47fe_39ac_7272);
/// The thunk of slot 423 (`sceGsExecLoadImage`).
pub const THUNK_423: Sig = thunk_sig(0xebb5_8666_cd2f_9c3a);
/// The thunk of slot 425 (`sceGsSetDefLoadImage`).
pub const THUNK_425: Sig = thunk_sig(0x27b8_118a_d5eb_2782);

/// A call through the slot thunk the `rel32` at `f + call` reaches (the
/// thunk jumps to its slot with the caller's stack), with `p`'s words.
fn thunk_call<G: Game>(g: &mut G, img: &Image, f: u32, call: u32, p: &Pushes) -> Out {
    let t = img.rel32_target(f + call);
    let table = img.word(t + 1);
    let slot = slot_of_thunk(img, t, table).unwrap_or(0);
    // call width: 12 (the most `Pushes::words` holds)
    p.call(g, slot_target(g, table, slot), 0)
}

/// The two 0x100-square images saved and loaded back (cdecl, no
/// arguments; the GS image calls of the PS2 code): for the images at
/// 0x2e00 and 0x3200 (records `G_S1`, `G_S2`), slot 426 sets up a store,
/// the cache is flushed (slot 421) and slot 424 stores into `*G_B1` /
/// `*G_B2`; slot 222 `(*G_E, 0, n, 0x2000)` with `n` 1000 once `*G_N >=
/// 0x12e`, else 0xe; slots 378, 380, 379 `(0x2000)`; `**G_P = *G_V`; the
/// timeline runs ([`timeline`]); then both images are loaded back (slots
/// 425, 421, 382 `(0, 0)`, 423 from the buffers, records `G_L1`, `G_L2`)
/// and synced (slot 382); `*G_N` counts up. Each call sees the words the
/// earlier ones left on the stack. Returns the last slot's `eax`.
///
/// `magic_0662` 0x10001000 (448 bytes; 2 DLLs).
pub fn images_store_load<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let mut s = Pushes::default();
    s.push_all(&[0x100, 0x100, 0, 0, 0, 4, 0x2e00, w(0x23)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x1), 426), 0);
    s.push(0);
    thunk_call(g, img, f, 0x2c, &s);
    s.push_all(&[g.read::<u32>(w(0x32)), w(0x3c)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x37), 424), 0);
    s.push_all(&[0x100, 0x100, 0, 0, 0, 4, 0x3200, w(0x6b)]);
    s.call(g, slot_target(g, w(0x49), 426), 0);
    let mut s = Pushes::default();
    s.push(0);
    thunk_call(g, img, f, 0x77, &s);
    s.push_all(&[g.read::<u32>(w(0x7d)), w(0x87)]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x82), 424), 0);
    let n = if g.read::<i32>(w(0x9d)) < 0x12e {
        0xe
    } else {
        1000
    };
    let mut s = Pushes::default();
    s.push_all(&[0x2000, n, 0, g.read::<u32>(w(0xbf))]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x94), 222), 0);
    let mut s = Pushes::default();
    for (table, slot) in [(0xc6u32, 378u32), (0xdb, 380), (0xed, 379)] {
        s.push(0x2000);
        // call width: 12 (the most `Pushes::words` holds)
        s.call(g, slot_target(g, w(table), slot), 0);
    }
    let p = g.read::<u32>(w(0x100));
    let v = g.read::<u32>(w(0x105));
    g.write::<u32>(p, v);
    timeline(g, img, img.rel32_target(f + 0x10c));
    s.push_all(&[0x100, 0x100, 0, 0, 0, 4, 0x2e00, w(0x128)]);
    thunk_call(g, img, f, 0x12d, &s);
    s.push(0);
    thunk_call(g, img, f, 0x134, &s);
    s.push_all(&[0, 0]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x139), 382), 0);
    s.push_all(&[g.read::<u32>(w(0x14b)), w(0x150)]);
    thunk_call(g, img, f, 0x155, &s);
    let mut s = Pushes::default();
    s.push_all(&[0x100, 0x100, 0, 0, 0, 4, 0x3200, w(0x174)]);
    thunk_call(g, img, f, 0x179, &s);
    s.push(0);
    thunk_call(g, img, f, 0x180, &s);
    s.push_all(&[0, 0]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x185), 382), 0);
    s.push_all(&[g.read::<u32>(w(0x197)), w(0x19c)]);
    thunk_call(g, img, f, 0x1a1, &s);
    s.push_all(&[0, 0]);
    // call width: 12 (the most `Pushes::words` holds)
    let e = s.call(g, slot_target(g, w(0x1a6), 382), 0).eax;
    bump(g, w(0x1b8));
    Ret::eax(e)
}

/// [`images_store_load`] (the `rel32`s: the slot thunks and
/// [`timeline`]).
pub const IMAGES_STORE_LOAD: Sig = Sig {
    len: 448,
    hash: 0xc8b6_5fda_8348_6848,
    calls: &[
        (0x2c, Callee::Sig(&THUNK_421)),
        (0x77, Callee::Sig(&THUNK_421)),
        (0x10c, Callee::Sig(&TIMELINE)),
        (0x12d, Callee::Sig(&THUNK_425)),
        (0x134, Callee::Sig(&THUNK_421)),
        (0x155, Callee::Sig(&THUNK_423)),
        (0x179, Callee::Sig(&THUNK_425)),
        (0x180, Callee::Sig(&THUNK_421)),
        (0x1a1, Callee::Sig(&THUNK_423)),
    ],
    imports: &[],
};

/// [`target_measure`] with the radius by mode: `KD / r` while `*G_A` or
/// `*G_B` is set, else 0.0005 with `*G_C`, else `KD / r * KM` (in
/// double).
///
/// `magic_0515` 0x100029a0 (450 bytes, `ecx` = the record; 2 DLLs).
pub fn target_measure_modes<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    measure(g, img, fr, (0x176, 0x185), |g, big| {
        if g.read::<u32>(w(0x10c)) != 0 || g.read::<u32>(w(0x115)) != 0 {
            (g.read::<f64>(w(0x120)) / f64::from(big)) as f32
        } else if g.read::<u32>(w(0x138)) != 0 {
            f32::from_bits(0x3a03_126f)
        } else {
            (g.read::<f64>(w(0x14c)) / f64::from(big) * g.read::<f64>(w(0x15b))) as f32
        }
    })
}

/// [`target_measure_modes`] (the `rel32`: `__security_check_cookie`).
pub const TARGET_MEASURE_MODES: Sig = Sig {
    len: 450,
    hash: 0x63a3_75f7_fc34_7aef,
    calls: &[(0x1ba, Callee::Any)],
    imports: &[],
};

/// A model's parts drawn with their own or the shared matrix (internal:
/// `ecx` = the model object, `edx` = the matrix; a /GS frame with a
/// 0x80-word table `ebp - 0x244`, cleared with `memset`). With `d` the
/// object's `+0x1c0` (part count u16 `+6`, entry count u16 `+8`, `+0x14`
/// the entries' data for slot 941), a part `j` is marked when some entry
/// `k` (12 bytes) of `slot 941 (d[0x14])` has u16 `+2 == j`. When every
/// part is marked (or there are none), slot 843 `(obj[0x830], matrix)`
/// draws the model whole and its `eax` is returned. Otherwise each part
/// `i` is drawn by slot 842 `(obj[0x830], m, i)`: `m` the matrix for a
/// marked part, else the matrix times the bone `obj[0x328] + 0x88 +
/// 0x160 * n` (`n` the u16 at `*(obj[0x1bc] + 0x38 i)`; slot 231 into
/// the local `ebp - 0x44`). Returns the part count as read last.
///
/// `magic_0434` 0x10003ce0 (451 bytes; 2 DLLs).
pub fn model_parts_draw<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    obj: u32,
    mtx: u32,
) -> u32 {
    let ebp = esp.wrapping_sub(4);
    let marks = ebp.wrapping_sub(0x244);
    let local = ebp.wrapping_sub(0x44);
    g.write_bytes(marks, &[0u8; 0x200]);
    let d = |g: &G| g.read::<u32>(obj + 0x1c0);
    let parts = |g: &G| u32::from(g.read::<u16>(d(g) + 6));
    let mut all = true;
    let mut j = 0u32;
    if parts(g) != 0 {
        loop {
            let mut k = 0u32;
            while k < u32::from(g.read::<u16>(d(g) + 8)) {
                let data = g.read::<u32>(d(g) + 0x14);
                let t = slot_call(g, img, f, (0x74, 941), &[data]).eax;
                if j == u32::from(g.read::<u16>(t.wrapping_add(12 * k + 2))) {
                    g.write::<u32>(marks + 4 * j, 1);
                    break;
                }
                k += 1;
            }
            if g.read::<u32>(marks.wrapping_add(4 * j)) == 0 {
                all = false;
            }
            j += 1;
            if j.cast_signed() >= parts(g).cast_signed() {
                break;
            }
        }
    }
    if all {
        let m = g.read::<u32>(obj + 0x830);
        return slot_call(g, img, f, (0x19d, 843), &[m, mtx]).eax;
    }
    if parts(g) == 0 {
        return d(g);
    }
    let mut i = 0u32;
    loop {
        if g.read::<u32>(marks.wrapping_add(4 * i)) != 0 {
            let m = g.read::<u32>(obj + 0x830);
            slot_call(g, img, f, (0xff, 842), &[m, mtx, i]);
        } else {
            let e = g.read::<u32>(g.read::<u32>(obj + 0x1bc).wrapping_add(0x38 * i));
            let n = u32::from(g.read::<u16>(e));
            let bone = g
                .read::<u32>(obj + 0x328)
                .wrapping_add(0x88)
                .wrapping_add(n.wrapping_mul(0x160));
            slot_call(g, img, f, (0x148, 231), &[local, mtx, bone]);
            let m = g.read::<u32>(obj + 0x830);
            slot_call(g, img, f, (0x15a, 842), &[m, local, i, local, mtx, bone]);
        }
        i += 1;
        let n = parts(g);
        if i.cast_signed() >= n.cast_signed() {
            return n;
        }
    }
}

/// [`model_parts_draw`] (the `rel32`s: `memset` and two
/// `__security_check_cookie`s).
pub const MODEL_PARTS_DRAW: Sig = Sig {
    len: 451,
    hash: 0x8405_5d31_7cc3_2deb,
    calls: &[
        (0x37, Callee::Import("memset")),
        (0x18e, Callee::Any),
        (0x1bb, Callee::Any),
    ],
    imports: &[],
};

/// A part run at a point of the character's root (cdecl `(obj)`, a /GS
/// frame with two local matrices `R` (`ebp - 0xa4`) and `S` (`ebp -
/// 0x54`) and three vectors): `R` = the root matrix of `*G_CH` (slot 210)
/// scaled by 1000 (`S` a unit matrix, slot 235, its diagonal 1000; slot
/// 231), the point `(0, -10, -29, 1)` (or `(0, -7.9, -38.6, 1)` with
/// `*G_ALT`) through it (slot 229) and copied (slot 325); then `R` scaled
/// by 0.32 the same way, copied into `*G_A` (slot 230), the DLL's
/// matrices multiplied and copied (slots 231 `(*G_B, *G_C, *G_A)` and 230
/// `(*G_D, *G_B)`), the object's scale `+0x60..+0x6c = (0.32, 0.32, 0.32,
/// 1)`, `*G_P = ITEM` and the part run (slot 712 `(obj, 0x30)`) with the
/// lock `*G_L` held at 1. Returns 1 when the run ended (slot 712 answered
/// 0), else 0.
///
/// `magic_0530` 0x100014e0 (470 bytes; 2 DLLs).
pub fn root_point_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let [r, s, v, p, q] = [0xa4u32, 0x54, 0x14, 0x64, 0x74].map(|o| ebp.wrapping_sub(o));
    let diagonal = |g: &mut G, x: u32| {
        for o in [0x28u32, 0x14, 0] {
            g.write::<u32>(s + o, x);
        }
    };
    let mut st = Pushes::default();
    let ch = g.read::<u32>(w(0x26));
    st.push_all(&[r, ch]);
    st.call(g, slot_target(g, w(0x20), 210), 0);
    st.push(s);
    st.call(g, slot_target(g, w(0x37), 235), 0);
    st.push_all(&[s, r, r]);
    diagonal(g, 0x447a_0000);
    st.call(g, slot_target(g, w(0x50), 231), 0);
    let (y, z) = if g.read::<u32>(w(0x76)) == 0 {
        (0xc120_0000u32, 0xc1e8_0000u32)
    } else {
        (0xc0fc_ccc2, 0xc21a_662a)
    };
    for (o, x) in [(0u32, 0u32), (0xc, 0x3f80_0000), (4, y), (8, z)] {
        g.write::<u32>(v + o, x);
    }
    let mut st = Pushes::default();
    st.push_all(&[v, r, p]);
    st.call(g, slot_target(g, w(0xb9), 229), 0);
    st.push_all(&[p, q]);
    st.call(g, slot_target(g, w(0xce), 325), 0);
    st.push(s);
    st.call(g, slot_target(g, w(0xdf), 235), 0);
    st.push_all(&[s, r, r]);
    diagonal(g, 0x3ea3_d70a);
    st.call(g, slot_target(g, w(0xf8), 231), 0);
    st.push_all(&[r, g.read::<u32>(w(0x127))]);
    st.call(g, slot_target(g, w(0x121), 230), 0);
    let [a, b, c] = [0x135u32, 0x140, 0x14c].map(|o| g.read::<u32>(w(o)));
    st.push_all(&[a, b, c]);
    for o in [0x60u32, 0x64, 0x68] {
        g.write::<u32>(obj + o, 0x3ea3_d70a);
    }
    g.write::<u32>(obj + 0x6c, 0x3f80_0000);
    st.call(g, slot_target(g, w(0x13a), 231), 0);
    let [b, d] = [0x170u32, 0x17b].map(|o| g.read::<u32>(w(o)));
    st.push_all(&[b, d]);
    st.call(g, slot_target(g, w(0x175), 230), 0);
    let ptr = g.read::<u32>(w(0x188));
    g.write::<u32>(ptr, w(0x191));
    let lock = g.read::<u32>(w(0x196));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let ran = slot_call(g, img, f, (0x1a5, 712), &[obj, 0x30]).eax;
    let lock = g.read::<u32>(w(0x1b4));
    g.write::<u32>(lock, saved);
    Ret::eax(u32::from(ran == 0))
}

/// [`root_point_part`] (the `rel32`: `__security_check_cookie`).
pub const ROOT_POINT_PART: Sig = Sig {
    len: 470,
    hash: 0x2039_1561_d9e6_6b22,
    calls: &[(0x1ce, Callee::Any)],
    imports: &[],
};

/// [`summon_ppp_start`] with a larger environment (0x100000 or 0x200000
/// bytes) and a stream `k` of the effect data cued (slot 551 `(slot 719
/// (E, 6, k), 0, 2)`; `k` the immediate at `+0x168`) before the record
/// `G_R` goes in front of the ring `G_RING`. Returns 0.
///
/// `magic_0186` 0x10008230 (480 bytes; 2 DLLs; stream 2) and
/// `magic_0173` 0x10007170 (480 bytes; 2 DLLs; stream 1, its own
/// [`SUMMON_PPP_START_STREAM_1`]).
pub fn summon_ppp_start_stream<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    summon_ppp_head(g, img, fr);
    // push imm8 at +0x167: the stream (sign-extended; 1 or 2 so far)
    let stream = u32::from(img.word(f + 0x168) as u8 as i8 as u8);
    let mut s = Pushes::default();
    s.push_all(&[stream, 6, w(0x16c)]);
    // call width: 12 (the most `Pushes::words` holds)
    let a = s.call(g, slot_target(g, w(0x15a), 719), 0).eax;
    s.push_all(&[2, 0, a]);
    s.call(g, slot_target(g, w(0x178), 551), 0);
    let tail = g.read::<u32>(w(0x185));
    for (dst, v) in [(0x18eu32, 0x192u32), (0x198, 0x19c), (0x1a2, 0x1a6)] {
        g.write::<u32>(w(dst), w(v));
    }
    g.write::<u32>(w(0x1ab), tail);
    g.write::<u32>(w(0x1b1), w(0x1b5));
    g.write::<u32>(tail.wrapping_add(4), w(0x1bc));
    let flags = g.read::<u32>(w(0x1c2)) & 0xffff_fffe;
    g.write::<u32>(w(0x1c2), flags);
    g.write::<u32>(w(0x1c9), w(0x1cd));
    g.write::<u32>(w(0x1d3), 0);
    Ret::eax(0)
}

/// [`summon_ppp_start_stream`] (the `rel32`: [`targets_init`]).
pub const SUMMON_PPP_START_STREAM: Sig = Sig {
    len: 480,
    hash: 0xb4ab_b93c_c6de_fbf8,
    calls: &[(0x8, Callee::Sig(&TARGETS_INIT))],
    imports: &[],
};

/// [`summon_ppp_start_stream`] as `magic_0173` binds it (stream 1).
pub fn summon_ppp_start_stream_1<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    summon_ppp_start_stream(g, img, fr)
}

/// [`summon_ppp_start_stream_1`] (the `rel32`: [`targets_init`]).
pub const SUMMON_PPP_START_STREAM_1: Sig = Sig {
    len: 480,
    hash: 0xd4de_b5c5_1a46_b303,
    calls: &[(0x8, Callee::Sig(&TARGETS_INIT))],
    imports: &[],
};

/// A task node taken from the DLL's free ring as the overlays inline it
/// (the operands, in `f`'s code: the u16 free count `count` and its store
/// `count_store`, the free head `head`, its stores `next` and `emptied`,
/// the callback immediate `cb`): none when the count is 0; else the head
/// leaves the ring (its neighbours linked), the head moves on (0 when it
/// was the last), the node is cleared (`+8` itself, `+0x10` the
/// callback) and the count drops.
fn inline_take<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    [count, head, next, emptied, cb, count_store]: [u32; 6],
) -> u32 {
    let w = |o: u32| op(img, f, o);
    let n = g.read::<u16>(w(count));
    if n == 0 {
        return 0;
    }
    let node = g.read::<u32>(w(head));
    let prev = g.read::<u32>(node);
    let after = g.read::<u32>(node + 4);
    g.write::<u32>(prev + 4, after);
    let after = g.read::<u32>(node + 4);
    let prev = g.read::<u32>(node);
    g.write::<u32>(after, prev);
    if n > 1 {
        let after = g.read::<u32>(node + 4);
        g.write::<u32>(w(next), after);
    } else {
        g.write::<u32>(w(emptied), 0);
    }
    for o in [0u32, 4, 0xc, 0x14] {
        g.write::<u32>(node + o, 0);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 0x10, w(cb));
    g.write::<u8>(node + 0x18, 0);
    g.write::<u16>(w(count_store), n - 1);
    node
}

/// Sparks started in pairs (cdecl, no arguments): for each `i` below
/// `*G_N - 3`, two task nodes ([`inline_take`]) with the bytes `+0x14..`
/// `(r % 6 + 10 i, i, i, i)` and `(r' % 6 + 10 (i + 3), i, i, i + 3)` (`r`,
/// `r'` from `rand`), kind 1, each appended to the task list. Returns
/// the list's old tail after a pair, or the last `i` (the entry's `eax`
/// when there is none).
///
/// `magic_0349` 0x100054c0 (480 bytes; 2 DLLs).
pub fn spark_pairs<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let n = g.read::<i32>(w(0x8)).wrapping_sub(3);
    let mut eax = fr.eax;
    let append = |g: &mut G, node: u32, (head, tail, first, last): (u32, u32, u32, u32)| {
        let mut eax = None;
        if g.read::<u32>(w(head)) != 0 {
            let t = g.read::<u32>(w(tail));
            g.write::<u32>(node, t);
            g.write::<u32>(t.wrapping_add(4), node);
            eax = Some(t);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(first), node);
        }
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(last), node);
        eax
    };
    let mut i = 0i32;
    while i < n {
        // the bytes of `i` (`mov al, bl`)
        let b = i.cast_unsigned() as u8;
        let node = inline_take(g, img, f, [0x23, 0x32, 0x4f, 0x57, 0x88, 0x93]);
        // idiv: the remainder of rand by 6 (rand is never negative, so
        // the unsigned remainder is the same)
        let r = (g.rand() % 6) as u8;
        for o in [0x15u32, 0x16, 0x17] {
            g.write::<u8>(node.wrapping_add(o), b);
        }
        g.write::<u8>(node.wrapping_add(0x18), 1);
        g.write::<u32>(node.wrapping_add(4), 0);
        let head = g.read::<u32>(w(0xc7));
        g.write::<u8>(node.wrapping_add(0x14), r.wrapping_add(b.wrapping_mul(10)));
        if head != 0 {
            let t = g.read::<u32>(w(0xd2));
            g.write::<u32>(node, t);
            g.write::<u32>(t.wrapping_add(4), node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(0xe5), node);
        }
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0xf5), node);
        let node = inline_take(g, img, f, [0xec, 0x104, 0x121, 0x129, 0x15a, 0x165]);
        let b3 = b.wrapping_add(3);
        let r = (g.rand() % 6) as u8;
        g.write::<u8>(node.wrapping_add(0x17), b3);
        g.write::<u8>(node.wrapping_add(0x18), 1);
        g.write::<u32>(node.wrapping_add(4), 0);
        eax = i.cast_unsigned();
        g.write::<u8>(node.wrapping_add(0x14), r.wrapping_add(b3.wrapping_mul(10)));
        g.write::<u8>(node.wrapping_add(0x15), b);
        g.write::<u8>(node.wrapping_add(0x16), b);
        if let Some(t) = append(g, node, (0x199, 0x1aa, 0x1bd, 0x1ca)) {
            eax = t;
        }
        i += 1;
    }
    Ret::eax(eax)
}

/// [`spark_pairs`] (the imports: `rand`, twice).
pub const SPARK_PAIRS: Sig = Sig {
    len: 480,
    hash: 0xc95c_b0cb_0da4_5750,
    calls: &[],
    imports: &[(0x99, "rand"), (0x16e, "rand")],
};

/// Three points eased between their two ends (cdecl, no arguments; the
/// state `*G_S`): for `k` in 0..3, the count `+0x2268 + 4 k` down to 0,
/// `t = count / D_k` (`D_k` constants), `u = K - t`, and the point
/// `+0x19b0 + 0x20 k = (b.x u + a.x t, b.y, b.z u + a.z t, 1)` with `a` at
/// `+0x1a70 + 0x10 k` and `b` at `+0x1aa0 + 0x10 k`. Returns the last
/// count.
///
/// `magic_0511` 0x10005200 (481 bytes; 2 DLLs).
pub fn three_points_ease<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let s = g.read::<u32>(w(0x5));
    let k = g.read::<f32>(w(0x2c));
    let mut count = 0;
    let mut points = [[0f32; 3]; 3];
    for (i, d) in [0x34u32, 0xac, 0x121].into_iter().enumerate() {
        let i = i as u32;
        let c = s + 0x2268 + 4 * i;
        count = g.read::<i32>(c);
        if count > 0 {
            count -= 1;
            g.write::<i32>(c, count);
        }
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
        let t = div(g.read::<i32>(c) as f32, g.read::<f32>(w(d)));
        let u = sub(k, t);
        let a = s + 0x1a70 + 0x10 * i;
        let b = s + 0x1aa0 + 0x10 * i;
        let r = |g: &G, p: u32| g.read::<f32>(p);
        points[i as usize] = [
            add(mul(r(g, b), u), mul(r(g, a), t)),
            r(g, b + 4),
            add(mul(r(g, b + 8), u), mul(r(g, a + 8), t)),
        ];
    }
    for (i, [x, y, z]) in (0u32..).zip(points) {
        let p = s + 0x19b0 + 0x20 * i;
        g.write::<f32>(p, x);
        g.write::<f32>(p + 4, y);
        g.write::<f32>(p + 8, z);
        g.write::<u32>(p + 0xc, 0x3f80_0000);
    }
    Ret::eax(count.cast_unsigned())
}

/// [`three_points_ease`].
pub const THREE_POINTS_EASE: Sig = Sig {
    len: 481,
    hash: 0xaeca_650f_c53c_dc6d,
    calls: &[],
    imports: &[],
};

/// A model carried along an arc from its character (cdecl, no
/// arguments; a /GS frame). With `n = *G_T - 15` above 0x32 it is placed
/// at `G_HOME` ([`place`], an immediate position). Otherwise: the
/// character `G_CHRS[*G_SEL]`'s position (slot 262 into three locals, `w
/// = 1`) and turn (slot 333, ST0), the angle `A - turn` wrapped once
/// (`x - T` above `HI`, `x + T` below `LO`), `t = n / D`, `s = K - t`, the
/// height `q = s t K1 K2 + s s 0 + t t 0` (doubled by `K3` on odd
/// `*G_T`), the point `(q, 0, 0, 1)` turned by a unit matrix rotated by 0
/// about x and by the angle about y (slots 235, 328, 329, 229), added to
/// the position (slot 248) and the model placed there ([`place`]).
/// Returns [`place`]'s `eax`.
///
/// `magic_0052` 0x10001890 (485 bytes; 2 DLLs).
pub fn arc_carry<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let n = g.read::<i32>(w(0x13)).wrapping_sub(15);
    if n > 0x32 {
        return Ret::eax(place(g, img, img.rel32_target(f + 0x25), w(0x20)));
    }
    let ebp = fr.esp.wrapping_sub(4);
    let [p, m, r, v] = [0x14u32, 0x74, 0x34, 0x24].map(|o| ebp.wrapping_sub(o));
    let chr = |g: &G, sel: u32, table: u32| {
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(sel))));
        g.read::<u32>(w(table).wrapping_add(4 * i))
    };
    let ch = chr(g, 0x39, 0x4f);
    slot_call(g, img, f, (0x54, 262), &[ch, p, p + 4, p + 8]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let ch2 = chr(g, 0x61, 0x72);
    let turn = st0_f32(slot_call(
        g,
        img,
        f,
        (0x77, 333),
        &[ch2, ch, p, p + 4, p + 8],
    ));
    let a = sub(k(g, 0x8a), turn);
    let a = if a > k(g, 0x99) {
        sub(a, k(g, 0xa8))
    } else if k(g, 0xb2) > a {
        add(a, k(g, 0xbf))
    } else {
        a
    };
    let odd = g.read::<u8>(w(0xca)) & 1 != 0;
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let t = div(n as f32, k(g, 0xe2));
    let s = sub(k(g, 0xda), t);
    for (o, x) in [(4u32, 0u32), (8, 0), (0xc, 0x3f80_0000)] {
        g.write::<u32>(v + o, x);
    }
    let q = mul(mul(mul(s, t), k(g, 0x111)), k(g, 0x11d));
    let q = add(add(q, mul(mul(s, s), 0.0)), mul(mul(t, t), 0.0));
    let q = if odd { mul(q, k(g, 0x13c)) } else { q };
    g.write::<f32>(v, q);
    slot_call(g, img, f, (0x14a, 235), &[m]);
    slot_call(g, img, f, (0x163, 328), &[m, m, 0]);
    slot_call(g, img, f, (0x182, 329), &[m, m, a.to_bits()]);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    slot_call(g, img, f, (0x19b, 229), &[r, m, v, m, m, a.to_bits()]);
    slot_call(
        g,
        img,
        f,
        (0x1b8, 248),
        &[p, p, r, r, m, v, m, m, a.to_bits()],
    );
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    Ret::eax(place(g, img, img.rel32_target(f + 0x1d2), p))
}

/// [`arc_carry`] (the `rel32`s: two [`place`]s and two
/// `__security_check_cookie`s).
pub const ARC_CARRY: Sig = Sig {
    len: 485,
    hash: 0xe9b6_d791_9f3d_c72d,
    calls: &[
        (0x25, Callee::Sig(&PLACE)),
        (0x30, Callee::Any),
        (0x1d2, Callee::Sig(&PLACE)),
        (0x1dd, Callee::Any),
    ],
    imports: &[],
};

/// Eight objects started around the character (cdecl, no arguments; a
/// /GS frame): the position of `G_CHRS[*G_SEL]` (slot 262 into locals)
/// and its point 0x10 there (slot 268); for each of the eight words `e`
/// of the table `G_T` (immediates bound the loop), the object `*G_S +
/// 0x11d0 + 0x130 k` is started ([`start_object_at`]: part 6, owner `e -
/// 0xc3`, the owner word `G_O` an immediate, scale `(frnd(0.2) + K1) *
/// K2`), placed at the point (`+0xd0..+0xdc`, `w` 1), given three random
/// turns `frnd(pi)` (`+0xc0..+0xc8`) and nudged by `frnd(2)` on each
/// axis. `frnd` is slot 314 (ST0). Returns slot 314's last `eax`.
///
/// `magic_0393` 0x10002180 (485 bytes; 2 DLLs).
pub fn objects_around<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x14);
    let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x11))));
    let ch = g.read::<u32>(w(0x1d).wrapping_add(4 * i));
    let base = g.read::<u32>(w(0x23));
    slot_call(g, img, f, (0x34, 262), &[ch, p, p + 4, p + 8]);
    slot_call(g, img, f, (0x46, 268), &[ch, 0x10, p, ch, p, p + 4, p + 8]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let frnd = |g: &mut G, table: u32, x: u32| {
        let out = slot_call(g, img, f, (table, 314), &[x]);
        (st0_f32(out), out.eax)
    };
    let pi = 0x4049_0fdb;
    let two = 0x4000_0000;
    let mut obj = base.wrapping_add(0x11d0);
    let mut e = w(0x66);
    let mut eax = 0;
    while e.cast_signed() < w(0x1cb).cast_signed() {
        let (r, _) = frnd(g, 0x71, 0x3e4c_cccd);
        let scale = mul(add(r, g.read::<f32>(w(0x91))), g.read::<f32>(w(0xa8)));
        let owner = g.read::<u32>(e).wrapping_sub(0xc3);
        start_object_at(
            g,
            img,
            img.rel32_target(f + 0xb3),
            obj,
            scale,
            6,
            owner,
            w(0xa0),
        );
        for o in [0xd0u32, 0xd4, 0xd8, 0xdc] {
            let v = g.read::<u32>(p + o - 0xd0);
            g.write::<u32>(obj + o, v);
        }
        for (table, o) in [(0xbdu32, 0xc0u32), (0x109, 0xc4), (0x123, 0xc8)] {
            let (r, _) = frnd(g, table, pi);
            g.write::<f32>(obj + o, r);
        }
        for (table, o) in [(0x13d, 0xd0u32), (0x161, 0xd4), (0x18d, 0xd8)] {
            let (r, a) = frnd(g, table, two);
            let v = add(g.read::<f32>(obj + o), r);
            g.write::<f32>(obj + o, v);
            eax = a;
        }
        e = e.wrapping_add(4);
        obj = obj.wrapping_add(0x130);
    }
    Ret::eax(eax)
}

/// [`objects_around`] (the `rel32`s: [`start_object_at`] and
/// `__security_check_cookie`).
pub const OBJECTS_AROUND: Sig = Sig {
    len: 485,
    hash: 0xb168_5f54_528f_f5d9,
    calls: &[(0xb3, Callee::Sig(&START_OBJECT_AT)), (0x1dd, Callee::Any)],
    imports: &[],
};

/// `cvtdq2ps` of a `rand` result.
#[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
fn rand_f32<G: Game>(g: &mut G) -> f32 {
    g.rand().cast_signed() as f32
}

/// A map task scattered from its model (cdecl `(task)`, a /GS frame):
/// the model `m = *(*G_M + 0x1a0)`'s matrix into a local `M` (slot 463
/// `(m, 0, M)`), its two sizes `a` (slot 488) and `b` (slot 489, ST0
/// each), the point `(rand / R * b * K1 - b, -(rand / R * a), b * K2, 1)`
/// through `M` (slot 229, in place), the task's position `+0xd0` that
/// point (`w` 0), `+0x100..+0x10c = (0, 0, rand / R * K3, 0)`, the scale
/// `+0x130..+0x138 = rand / R * K4 + K4` (`w` 0), bit 14 of `+0xbc` set;
/// then slot 458 `(*G_M, task, 0)` and the task's sequences (slots 450
/// and 449 with immediates). Each call sees the words the earlier ones
/// left on the stack. Returns slot 449's `eax`.
///
/// `magic_0356` 0x10001580 (494 bytes; 2 DLLs).
pub fn task_scatter<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let task = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let m_local = ebp.wrapping_sub(0x54);
    let v = ebp.wrapping_sub(0x14);
    let m = g.read::<u32>(g.read::<u32>(w(0x11)).wrapping_add(0x1a0));
    let mut s = Pushes::default();
    s.push_all(&[m_local, 0, m]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x25), 463), 0);
    s.push(m);
    let a = st0_f32(s.call(g, slot_target(g, w(0x35), 488), 0));
    s.push(m);
    // call width: 12 (the most `Pushes::words` holds)
    let b = st0_f32(s.call(g, slot_target(g, w(0x46), 489), 0));
    let x = sub(mul(mul(div(rand_f32(g), k(g, 0x69)), b), k(g, 0x76)), b);
    g.write::<f32>(v, x);
    let y = mul(div(rand_f32(g), k(g, 0x94)), a);
    // xorps with the sign mask: the negated value, NaNs included
    let mask = g.read::<u32>(w(0xa5));
    g.write::<u32>(v + 4, y.to_bits() ^ mask);
    g.write::<f32>(v + 8, mul(b, k(g, 0xb7)));
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    s.push_all(&[v, m_local, v]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0xc0), 229), 0);
    let q = mul(div(rand_f32(g), k(g, 0xe5)), k(g, 0xed));
    let sc = add(mul(div(rand_f32(g), k(g, 0x10d)), k(g, 0x11a)), k(g, 0x12f));
    let flags = g.read::<u32>(task + 0xbc) | 0x4000;
    g.write::<u32>(task + 0xbc, flags);
    for o in [0u32, 4, 8] {
        let c = g.read::<u32>(v + o);
        g.write::<u32>(task + 0xd0 + o, c);
    }
    for (o, x) in [
        (0xdcu32, 0u32),
        (0x100, 0),
        (0x104, 0),
        (0x108, q.to_bits()),
        (0x10c, 0),
        (0x130, sc.to_bits()),
        (0x134, sc.to_bits()),
        (0x138, sc.to_bits()),
        (0x13c, 0),
    ] {
        g.write::<u32>(task + o, x);
    }
    s.push_all(&[0, task, g.read::<u32>(w(0x1a9))]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, slot_target(g, w(0x1a2), 458), 0);
    s.push_all(&[w(0x1bb), task]);
    s.call(g, slot_target(g, w(0x1b6), 450), 0);
    s.push_all(&[w(0x1ce), task]);
    // call width: 12 (the most `Pushes::words` holds)
    let e = s.call(g, slot_target(g, w(0x1c9), 449), 0).eax;
    Ret::eax(e)
}

/// [`task_scatter`] (the `rel32`: `__security_check_cookie`; the import
/// `rand`, called through `esi`).
pub const TASK_SCATTER: Sig = Sig {
    len: 494,
    hash: 0x8ef7_535b_7089_7db8,
    calls: &[(0x1e6, Callee::Any)],
    imports: &[(0x58, "rand")],
};

/// A thrown node set up and aimed (`ecx` = `[node, parent, target, a,
/// grav, flag, limit, out]`; an aligned frame). The node is linked under
/// its parent as in [`node_setup`] (u16 `+0x1a = 0xff`, its child ring
/// empty, the end of the parent's children). Then, with `d = target -
/// parent+0x1c` and `h = grav / 2`, it tries the upward speeds `v = 5,
/// 4.5, ..` (32 tries): when `v v - 4 h (-d.y)` is not negative, its root
/// `r` (`_libm_sse2_sqrt_precise`) gives the two flight times `t1 = (r -
/// v) / 2h` and `t2 = (-v - r) / 2h`; the first positive one (`t1` first
/// when `flag` says `t1 > t2`, or not `flag` and `t2 > t1`; else `t2`
/// first) is the time, taken when it is above `limit`. Without one the
/// time is 1.0 (and `v` the last tried less 0.5). `*out = v`; the node's
/// speed `(d.x / t, v, d.z / t)` at `+0x1c..+0x24`, `+0x3c` the time,
/// `+0x40 = 0`, u16 `+0x44 = 0`, `+0x34 = grav`, `+0x38 = a`, which it
/// returns. (The frame word `[esp + 0x30]` it copies into its best time
/// is never written; the copy never reaches a store.)
///
/// `magic_0178` 0x10005e20 (509 bytes, `ecx` = the arguments; 2 DLLs).
pub fn throw_aim<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let args = fr.ecx;
    let parent = g.read::<u32>(args + 4);
    let node = g.read::<u32>(args);
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
    let to = g.read::<u32>(args + 8);
    let p = |g: &G, o: u32| g.read::<f32>(parent.wrapping_add(o));
    let dz = sub(g.read::<f32>(to.wrapping_add(8)), p(g, 0x24));
    let dx = sub(g.read::<f32>(to), p(g, 0x1c));
    let dy = sub(g.read::<f32>(to.wrapping_add(4)), p(g, 0x20));
    let h = mul(g.read::<f32>(args + 0x10), k(g, 0x81));
    let c = mul(
        mul(h, k(g, 0x9c)),
        f32::from_bits(dy.to_bits() ^ 0x8000_0000),
    );
    let half = k(g, 0x1e4);
    let one = k(g, 0x68);
    let mut v = k(g, 0x4d);
    let mut time = None;
    for _ in 0..32 {
        let disc = sub(mul(v, v), c);
        if 0.0 <= disc || disc.is_nan() {
            let r = f64::from(disc).sqrt() as f32;
            let inv = div(one, mul(h, k(g, 0x118)));
            let t1 = mul(sub(r, v), inv);
            let t2 = mul(sub(f32::from_bits(v.to_bits() ^ 0x8000_0000), r), inv);
            let t1_first = if g.read::<u32>(args + 0x14) != 0 {
                t1 > t2
            } else {
                t2 > t1
            };
            let (a, b) = if t1_first { (t1, t2) } else { (t2, t1) };
            let x = if a > 0.0 {
                Some(a)
            } else if b > 0.0 {
                Some(b)
            } else {
                None
            };
            if let Some(x) = x
                && x > g.read::<f32>(args + 0x18)
            {
                time = Some(x);
                break;
            }
        }
        v = sub(v, half);
    }
    let t = time.unwrap_or(one);
    let out = g.read::<u32>(args + 0x1c);
    g.write::<f32>(out, v);
    g.write::<f32>(node + 0x1c, div(dx, t));
    g.write::<f32>(node + 0x3c, t);
    let grav = g.read::<u32>(args + 0x10);
    g.write::<u32>(node + 0x40, 0);
    g.write::<u16>(node + 0x44, 0);
    let a = g.read::<u32>(args + 0xc);
    g.write::<f32>(node + 0x24, div(dz, t));
    g.write::<f32>(node + 0x20, v);
    g.write::<u32>(node + 0x34, grav);
    g.write::<u32>(node + 0x38, a);
    Ret::eax(a)
}

/// [`throw_aim`] (the `rel32`: the square root).
pub const THROW_AIM: Sig = Sig {
    len: 509,
    hash: 0x603c_dfcb_4fce_6311,
    calls: &[(0xe7, Callee::Import("_libm_sse2_sqrt_precise"))],
    imports: &[],
};
