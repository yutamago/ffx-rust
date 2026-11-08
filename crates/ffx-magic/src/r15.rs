//! Round 15's own ports: start-ups and their helpers of overlays with
//! only a few functions left, written out for their DLL.

use crate::m0018::st0_f32;
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `magic_0686`'s light set-up (0x10001290, internal, no arguments; a
/// /GS frame of 0x58 bytes, `esp` at the return address): with `E` =
/// `*G_E` (`+0x13`) and the frame's matrix `M` (`ebp - 0x54`): slot 210
/// `(E, M)`; slot 232 `(r, r)` on its three rows `M`, `M+0x10`,
/// `M+0x20`; slot 19 `(E)`'s ST0 `r` (as f32) gives `s = -(r + K1)`
/// (`+0x76`, the sign flip a `xorps` with the mask `+0x89`), and the
/// third row scaled by `s` is the point `P` (`ebp - 0x14`); slot 248
/// `(V, V, P)` with `V` = `ebp - 0x24`, then `V.y -= K2` (`+0xca`); slot
/// 230 `(L, M)` (`L` the immediate `+0xd8`); slot 325 `(A, B)` (`+0xfa`,
/// `+0xef`), whose `eax` it returns. Each call sees the words the earlier
/// ones left on the stack.
pub fn light_setup_0686<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let ebp = esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let (m, v, p) = (at(0x54), at(0x24), at(0x14));
    let e = g.read::<u32>(w(0x13));
    let mut s = Pushes::default();
    s.push_all(&[m, e]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1c), 0x348 / 4), 0);
    for (row, op) in [(m, 0x2fu32), (at(0x44), 0x41), (at(0x34), 0x53)] {
        s.push_all(&[row, row]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(op), 0x3a0 / 4), 0);
    }
    s.push(e);
    // call width: 12 (Pushes caps the words at 12)
    let r = st0_f32(s.call(g, slot_target(g, w(0x60), 0x4c / 4), 0).st0);
    // fstp dword [ebp - 0x58]
    g.write::<f32>(at(0x58), r);
    let k1 = g.read::<f32>(w(0x76));
    let scale = f32::from_bits(add(r, k1).to_bits() ^ g.read::<u32>(w(0x89)));
    for k in 0..3u32 {
        let x = g.read::<f32>(at(0x34) + 4 * k);
        g.write::<f32>(p + 4 * k, mul(x, scale));
    }
    s.push_all(&[p, v, v]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x94), 0x3e0 / 4), 0);
    let y = sub(g.read::<f32>(v + 4), g.read::<f32>(w(0xca)));
    s.push_all(&[m, w(0xd8)]);
    g.write::<f32>(v + 4, y);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xd3), 0x398 / 4), 0);
    s.push_all(&[w(0xef), w(0xfa)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xea), 0x514 / 4), 0).eax
}

/// [`light_setup_0686`] (274 bytes, 1 overlay).
pub const LIGHT_SETUP_0686: Sig = Sig {
    len: 274,
    hash: 0x2ce8_654a_9610_657d,
    calls: &[
        // __security_check_cookie
        (0x10a, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0686`'s start-up (0x100020c0, an overlay table entry, cdecl
/// `(arg)`): a 16 MiB buffer from slot 803 `(0x1000000, 0x10)` cut into
/// four globals 0x80000 apart (`+0x19`, `+0x23`, `+0x2d`, `+0x37`), the
/// argument kept (`+0x3f`); the effect id `G_ID` (`+0x55`) = the
/// immediate `+0x59` and slot 718 `(id)`; slots 222 and 221 `(*G_ID, 0,
/// 1000, 0x2000)`; slot 719 `(*G_ID, 6, 0)`, `(*G_ID, 6, 1)` and `(*G_ID,
/// 8, 0)` into three globals; slot 711 `(R, S, 0)` (`R` the last, `S` the
/// immediate `+0xff`) with `*F` (`*G_F`, `+0xf1`) 1 around it; slot 710
/// `(T, R, *G_BUF, 0x80000)`; slot 941 `(R[5])`, `(R[6])`, `(R[7])` into
/// three globals; slots 4 `(0x3015)` and 27 `(0x3015, 1)`; slot 168
/// `(*G_ARG)` into `G_A`, slot 215 `(A)` into `G_B`, slot 412 `(*G_ARG,
/// C, 0)` into `G_C`; the light set-up ([`light_setup_0686`]); the
/// 0x110-byte block `+0x209` cleared (`memset`) and nine of its words
/// set. Returns 1.
pub fn start_up_0686<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let mut s = Pushes::default();
    s.push_all(&[0x10, 0x0100_0000]);
    // call width: 12 (Pushes caps the words at 12)
    let buf = s.call(g, slot_target(g, w(0x04), 0xc8c / 4), 0).eax;
    for (k, op) in (0u32..).zip([0x19u32, 0x23, 0x2d, 0x37]) {
        g.write::<u32>(w(op), buf.wrapping_add(0x8_0000 * k));
    }
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x3f), arg);
    s.push(w(0x49));
    g.write::<u32>(w(0x55), w(0x59));
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x44), 0xb38 / 4), 0);
    for (table, slot, id) in [(0x60u32, 0x378u32, 0x78u32), (0x7f, 0x374, 0x97)] {
        s.push_all(&[0x2000, 0x3e8, 0, g.read::<u32>(w(id))]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(table), slot / 4), 0);
    }
    for (table, kind, part, id, out) in [
        (0x9eu32, 6u32, 0u32, 0xaeu32, 0xb5u32),
        (0xba, 6, 1, 0xca, 0xd4),
    ] {
        s.push_all(&[part, kind, g.read::<u32>(w(id))]);
        // call width: 12 (Pushes caps the words at 12)
        let r = s.call(g, slot_target(g, w(table), 0xb3c / 4), 0).eax;
        g.write::<u32>(w(out), r);
    }
    s.pop(17);
    s.push_all(&[0, 8, g.read::<u32>(w(0xe9))]);
    // call width: 12 (Pushes caps the words at 12)
    let r = s.call(g, slot_target(g, w(0xd9), 0xb3c / 4), 0).eax;
    let flag = g.read::<u32>(w(0xf1));
    s.push(0);
    let saved = g.read::<u32>(flag);
    g.write::<u32>(w(0xfa), r);
    s.push_all(&[w(0xff), r]);
    g.write::<u32>(flag, 1);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x105), 0xb1c / 4), flag);
    let flag = g.read::<u32>(w(0x118));
    s.push_all(&[0x8_0000, g.read::<u32>(w(0x123))]);
    g.write::<u32>(flag, saved);
    s.push_all(&[g.read::<u32>(w(0x12b)), w(0x135)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x130), 0xb18 / 4), 0);
    for (rec, table, out, field) in [
        (0x142u32, 0x14au32, 0x157u32, 0x14u32),
        (0x15c, 0x164, 0x171, 0x18),
        (0x176, 0x17e, 0x18b, 0x1c),
    ] {
        let r = g.read::<u32>(w(rec));
        s.push(g.read::<u32>(r.wrapping_add(field)));
        // call width: 12 (Pushes caps the words at 12)
        let v = s.call(g, slot_target(g, w(table), 0xeb4 / 4), 0).eax;
        g.write::<u32>(w(out), v);
    }
    s.push(0x3015);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x190), 0x10 / 4), 0);
    s.push_all(&[1, 0x3015]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x19f), 0x6c / 4), 0);
    s.pop(16);
    s.push(g.read::<u32>(w(0x1bf)));
    // call width: 12 (Pushes caps the words at 12)
    let a = s.call(g, slot_target(g, w(0x1b0), 0x2a0 / 4), 0).eax;
    g.write::<u32>(w(0x1c6), a);
    s.push(a);
    // call width: 12 (Pushes caps the words at 12)
    let b = s.call(g, slot_target(g, w(0x1cc), 0x35c / 4), 0).eax;
    g.write::<u32>(w(0x1d9), b);
    s.push_all(&[0, w(0x1eb), g.read::<u32>(w(0x1f1))]);
    // call width: 12 (Pushes caps the words at 12)
    let c = s.call(g, slot_target(g, w(0x1de), 0x670 / 4), 0).eax;
    g.write::<u32>(w(0x1f8), c);
    // push ebp, push esi, five words; the return address
    light_setup_0686(g, img, img.rel32_target(f + 0x1fd), ebp.wrapping_sub(0x1c));
    g.write_bytes(w(0x209), &[0u8; 0x110]);
    g.write::<u32>(w(0x217), w(0x21b));
    for (op, v) in [(0x221u32, 7u32), (0x22b, 7)] {
        g.write::<u32>(w(op), v);
    }
    for op in [0x235u32, 0x23f, 0x249, 0x253, 0x25d, 0x267, 0x271] {
        g.write::<u32>(w(op), 1);
    }
    Ret::eax(1)
}

/// [`start_up_0686`] (641 bytes, 1 overlay).
pub const START_UP_0686: Sig = Sig {
    len: 641,
    hash: 0x091d_15d0_8c33_1516,
    calls: &[
        (0x1fd, Callee::Sig(&LIGHT_SETUP_0686)),
        (0x20e, Callee::Import("memset")),
    ],
    imports: &[],
};

/// `magic_0253`'s camera targets (0x10001710, an overlay table entry, no
/// arguments; a /GS frame of 0x14 bytes): slot 217 `(*G_A, P)` fills the
/// point `P` (`ebp - 0x14`); `G_P0` (`+0x5e`, `+0x3f`, `+0x66`, w 1.0) is
/// `P` and `G_P1` (`+0x7d`..) `P` with `y + K` (`+0x47`, also kept in
/// `P.y`); four more points `(0, 300, 0, 1)`, a count 50, a word 0 and
/// 0.02; slot 538 `(*G_B, 0)` and slot 602 `(*G_C, 0)` (the earlier
/// words under each), whose `eax` it returns.
pub fn camera_targets_0253<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x14);
    let mut s = Pushes::default();
    s.push_all(&[p, g.read::<u32>(w(0x1b))]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x15), 0x364 / 4), 0);
    let (x, y, z) = (g.read::<f32>(p), g.read::<f32>(p + 4), g.read::<f32>(p + 8));
    g.write::<f32>(w(0x3f), y);
    let y1 = add(y, g.read::<f32>(w(0x47)));
    s.push_all(&[0, g.read::<u32>(w(0x4f))]);
    g.write::<u32>(ebp.wrapping_sub(8), 0x3f80_0000);
    g.write::<f32>(w(0x5e), x);
    g.write::<f32>(w(0x66), z);
    g.write::<u32>(w(0x6c), 0x3f80_0000);
    g.write::<f32>(p + 4, y1);
    g.write::<f32>(w(0x7d), x);
    g.write::<f32>(w(0x85), y1);
    g.write::<f32>(w(0x8d), z);
    for (op, v) in [
        (0x93u32, 0x3f80_0000u32),
        (0x9d, 0),
        (0xa7, 0x4396_0000),
        (0xb1, 0),
        (0xbb, 0x3f80_0000),
        (0xc5, 0),
        (0xcf, 0x4396_0000),
        (0xd9, 0),
        (0xe3, 0x3f80_0000),
        (0xed, 0),
        (0xf7, 0x4396_0000),
        (0x101, 0),
        (0x10b, 0x3f80_0000),
        (0x115, 0),
        (0x11f, 0x4396_0000),
        (0x129, 0),
        (0x133, 0x3f80_0000),
        (0x13d, 0x32),
        (0x147, 0),
        (0x151, 0x3ca3_d70a),
    ] {
        g.write::<u32>(w(op), v);
    }
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x37), 0x868 / 4), 0);
    s.push_all(&[0, g.read::<u32>(w(0x16a))]);
    // call width: 12 (Pushes caps the words at 12)
    let eax = s.call(g, slot_target(g, w(0x162), 0x968 / 4), 0).eax;
    Ret::eax(eax)
}

/// [`camera_targets_0253`] (391 bytes, 1 overlay).
pub const CAMERA_TARGETS_0253: Sig = Sig {
    len: 391,
    hash: 0xb514_8686_ceea_ef2c,
    calls: &[
        // __security_check_cookie
        (0x17f, Callee::Any),
    ],
    imports: &[],
};

/// The angle wrap of [`objects_0253`]: `a - T` above `HI`, `a + T` below
/// `LO` (`comiss` + `jbe`: an unordered `a` stays).
fn wrap(a: f32, hi: f32, lo: f32, t_hi: f32, t_lo: f32) -> f32 {
    if a > hi {
        sub(a, t_hi)
    } else if lo > a {
        add(a, t_lo)
    } else {
        a
    }
}

/// `magic_0253`'s objects (0x100013d0, an overlay table entry, no
/// arguments; a /GS frame of 0x18 bytes): five effect objects started
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), `edx` the
/// object, `xmm3` its scale, `(index, value)`): `(2, 0)`; then three
/// times the actor `*G_A`'s place: slot 262 `(A, &X, &Y, &Z)` into a
/// point of globals, slot 268 `(A, 6, P)` into the bone point `P`
/// (`ebp - 0x14`), `Y` = `P.y` (the first time minus `K` times
/// [`float_product`](crate::o4_tiny3::float_product)), `W` 1.0, and the
/// yaw `K' - r` (`r` slot 333 `(A)`) wrapped once into a global; between them
/// objects `(0, 0x30)` (after `Y -= K2` and two words), `(4, 0x30)`
/// (after `Y += K3` and a word) and `(1, 0x64)`; then three words and
/// `(5, 0x8a)`, and three more words. Returns the last object.
#[allow(clippy::too_many_lines)] // three places, five objects, in order
pub fn objects_0253<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::s4_misc::start_object_anim;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x14);
    let scale = k(g, 0x14);
    start_object_anim(g, img, call(0x26), w(0x1d), scale, 2, 0);
    // (table, actor, the three outputs (pushed last first))
    let place = |g: &mut G, table: u32, actor: u32, outs: [u32; 3]| {
        let a = g.read::<u32>(w(actor));
        let mut s = Pushes::default();
        s.push_all(&[w(outs[2]), w(outs[1]), w(outs[0]), a]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(table), 0x418 / 4), 0);
        (s, a)
    };
    let (mut s, a) = place(g, 0x2b, 0x31, [0x46, 0x41, 0x3c]);
    // push esi, 8 bytes and two words, four words; the return address
    let fp = Frame {
        at: call(0x4e),
        esp: ebp.wrapping_sub(0x40),
        ..*fr
    };
    let x = crate::o4_tiny3::float_product(g, img, &fp)
        .xmm0
        .map_or(0.0, |v| f32::from_bits(v.reg() as u32));
    s.push_all(&[p, 6, a]);
    g.write::<f32>(ebp.wrapping_sub(0x18), x);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x57), 0x430 / 4), 0);
    let x1 = mul(x, k(g, 0x74));
    let y = sub(g.read::<f32>(p + 4), x1);
    s.push(g.read::<u32>(w(0x84)));
    g.write::<u32>(w(0x8e), 0x3f80_0000);
    g.write::<f32>(w(0x9a), y);
    // call width: 12 (Pushes caps the words at 12)
    let r = st0_f32(s.call(g, slot_target(g, w(0x7e), 0x534 / 4), 0).st0);
    let yaw = wrap(
        sub(k(g, 0xad), r),
        k(g, 0xc4),
        k(g, 0xd4),
        k(g, 0xba),
        k(g, 0xba),
    );
    g.write::<f32>(w(0xed), yaw);
    let y = sub(g.read::<f32>(w(0xf5)), k(g, 0xfd));
    g.write::<f32>(w(0x111), y);
    g.write::<u32>(w(0x117), 0xc1a0_0000);
    g.write::<u32>(w(0x121), 0x41a0_0000);
    start_object_anim(g, img, call(0x12a), w(0x105), k(g, 0xe5), 0, 0x30);

    let (mut s, a) = place(g, 0x12f, 0x135, [0x14a, 0x145, 0x140]);
    s.push_all(&[p, 6, a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x156), 0x430 / 4), 0);
    let y = g.read::<f32>(p + 4);
    s.push(g.read::<u32>(w(0x171)));
    g.write::<f32>(w(0x179), y);
    g.write::<u32>(w(0x17f), 0x3f80_0000);
    // call width: 12 (Pushes caps the words at 12)
    let r = st0_f32(s.call(g, slot_target(g, w(0x16b), 0x534 / 4), 0).st0);
    let yaw = wrap(
        sub(k(g, 0x196), r),
        k(g, 0x1a5),
        k(g, 0x1b9),
        k(g, 0x1af),
        k(g, 0x1c6),
    );
    let scale = k(g, 0x1ce);
    g.write::<f32>(w(0x1d6), yaw);
    let y = add(g.read::<f32>(w(0x1de)), k(g, 0x1e6));
    g.write::<u32>(w(0x1f8), 0x41a0_0000);
    g.write::<f32>(w(0x204), y);
    start_object_anim(g, img, call(0x209), w(0x1ee), scale, 4, 0x30);

    let (mut s, a) = place(g, 0x20e, 0x214, [0x229, 0x224, 0x21f]);
    s.push_all(&[p, 6, a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x235), 0x430 / 4), 0);
    let y = g.read::<f32>(p + 4);
    s.push(g.read::<u32>(w(0x250)));
    g.write::<f32>(w(0x258), y);
    g.write::<u32>(w(0x25e), 0x3f80_0000);
    // call width: 12 (Pushes caps the words at 12)
    let r = st0_f32(s.call(g, slot_target(g, w(0x24a), 0x534 / 4), 0).st0);
    let yaw = wrap(
        sub(k(g, 0x275), r),
        k(g, 0x285),
        k(g, 0x299),
        k(g, 0x28f),
        k(g, 0x2a6),
    );
    let scale = k(g, 0x2ae);
    g.write::<f32>(w(0x2c2), yaw);
    start_object_anim(g, img, call(0x2c7), w(0x2b6), scale, 1, 0x64);
    let scale = k(g, 0x2cf);
    for (op, v) in [
        (0x2e4u32, 0xc342_0000u32),
        (0x2ee, 0x4248_0000),
        (0x2f8, 0x4270_0000),
    ] {
        g.write::<u32>(w(op), v);
    }
    let last = start_object_anim(g, img, call(0x301), w(0x2d7), scale, 5, 0x8a);
    for (op, v) in [
        (0x30fu32, 0xc360_0000u32),
        (0x319, 0x41f0_0000),
        (0x323, 0x4270_0000),
    ] {
        g.write::<u32>(w(op), v);
    }
    Ret::eax(last)
}

/// [`objects_0253`] (820 bytes, 1 overlay).
pub const OBJECTS_0253: Sig = Sig {
    len: 820,
    hash: 0xb767_977a_b494_bef8,
    calls: &[
        (0x26, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x4e, Callee::Sig(&crate::o4_tiny3::FLOAT_PRODUCT_0253_2BF0)),
        (0x12a, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x209, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x2c7, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x301, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        // __security_check_cookie
        (0x32c, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0104`'s first actor's position for a mode (0x100016f0,
/// internal: `edx` = four floats out, the stack word = the mode; a /GS
/// frame): [`actor_point`](crate::s4_misc::actor_point) for the first
/// actor (`G_CHRS[*G_MAP]`, operands `+0x11`, `+0x1d`), mode 2's scale
/// from [`scale_product`](crate::s4_small::scale_product) (`+0x78`, times
/// `K` `+0x9e`). Returns the last call's `eax` (the mode less 1 for any
/// other mode).
pub fn actor_point_0104<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let out = fr.edx;
    let mode = fr.arg(g, 0);
    let map = g.read::<u32>(w(0x11));
    let k = u32::from(g.read::<u8>(map));
    let chr = g.read::<u32>(w(0x1d).wrapping_add(4 * k));
    g.call(
        slot_target(g, w(0x29), 262),
        0,
        &[chr, out, out + 4, out + 8],
    );
    let local = fr.esp.wrapping_sub(4 + 0x14);
    let point = |g: &mut G, op: u32, p: u32, at: u32| {
        g.call(slot_target(g, w(op), 268), 0, &[chr, p, at]).eax
    };
    let ground = |g: &mut G, op: u32, words: &[u32]| {
        let o = g.call(slot_target(g, w(op), 12), 0, words);
        g.write::<f32>(out + 4, st0_f32(o.st0));
        o.eax
    };
    let eax = match mode {
        1 => {
            let eax = point(g, 0x57, 6, local);
            let y = g.read::<u32>(local + 4);
            g.write::<u32>(out + 4, y);
            eax
        }
        2 => {
            // push ebx, esi, edi under the frame; the return address
            let sp = Frame {
                at: img.rel32_target(f + 0x78),
                esp: fr.esp.wrapping_sub(4 + 0x18 + 0xc + 4),
                ..*fr
            };
            let scale = match crate::s4_small::scale_product(g, img, &sp).xmm0 {
                Some(crate::Xmm::F32(x)) => f32::from_bits(x),
                _ => 0.0,
            };
            let eax = point(g, 0x81, 6, local);
            let y = sub(g.read::<f32>(local + 4), mul(scale, g.read::<f32>(w(0x9e))));
            g.write::<f32>(out + 4, y);
            eax
        }
        3 => ground(g, 0xb5, &[chr]),
        4 => point(g, 0xc8, 0x10, out),
        5 => point(g, 0xc8, 0x11, out),
        6 | 7 => {
            let p = if mode == 6 { 0x10 } else { 0x11 };
            point(g, 0xe8, p, out);
            // slot 268's words stay below
            ground(g, 0xf6, &[chr, chr, p, out])
        }
        // dec eax; cmp eax, 6; ja: the mode less 1 is left
        _ => mode.wrapping_sub(1),
    };
    g.write::<u32>(out + 0xc, 0x3f80_0000);
    Ret::eax(eax)
}

/// [`actor_point_0104`] (316 bytes with its jump table, 1 overlay; the
/// `rel32`s: [`scale_product`](crate::s4_small::scale_product), a jump
/// inside, the cookie check).
pub const ACTOR_POINT_0104: Sig = Sig {
    len: 316,
    hash: 0x6699_d817_9be6_25d5,
    calls: &[
        (0x78, Callee::Sig(&crate::s4_small::SCALE_PRODUCT)),
        // __security_check_cookie
        (0x115, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0104`'s start-up (0x10001460, internal, no arguments; `push
/// esi; push edi`): slot 719 `(NAME, 8, 0)` into `G_PART` (two words
/// stored first), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); slot 390 `(*G_SEL)`'s
/// id looked up in the `-1`-ended table `+0x5b` (found: mode 5, else 7);
/// the object `+0x92` started
/// ([`start_object_reset`](crate::s6_misc::start_object_reset), scale
/// `+0x8a`) and placed at the actor
/// ([`actor_point_0104`], `edx` `+0x9d`, the mode); for the ids 0x1077,
/// 0x1158..0x115a slot 12 `(*A)`'s ground into `G_Y` first; `G_Y -= K`;
/// the record words, two fades
/// ([`fade`](crate::r11_fades::fade)), slot 271 `(4)`, the task node.
/// Returns 0.
pub fn start_up_0104<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::{fade, stores, task_node};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = fr.esp;
    let mut s = Pushes::default();
    s.push_all(&[0, 8, w(0x12)]);
    stores(g, img, f, &[0x16, 0x20]);
    // call width: 12 (Pushes caps the words at 12)
    let part = s.call(g, slot_target(g, w(0x01), 0xb3c / 4), 0).eax;
    s.pop(1);
    g.write::<u32>(w(0x30), part);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x35));
    let sel = g.read::<u32>(w(0x3a));
    s.push(u32::from(g.read::<u8>(sel)));
    // call width: 12 (Pushes caps the words at 12)
    let id = s.call(g, slot_target(g, w(0x43), 0x618 / 4), 0).eax;
    let mut at = w(0x5b);
    let mut mode = 7;
    loop {
        let v = g.read::<u32>(at);
        if v == u32::MAX {
            break;
        }
        if v == id {
            mode = 5;
            break;
        }
        at = at.wrapping_add(4);
    }
    // push esi, edi; sub esp, 0x10; the return address
    let start = Frame {
        at: img.rel32_target(f + 0x97),
        esp: e - 0x1c,
        edx: w(0x92),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x8a)))],
        ..*fr
    };
    crate::s6_misc::start_object_reset(g, img, &start);
    g.write::<u32>(e - 0x1c, mode);
    let place = Frame {
        at: img.rel32_target(f + 0xa2),
        esp: e - 0x20,
        edx: w(0x9d),
        ..*fr
    };
    actor_point_0104(g, img, &place);
    if matches!(id, 0x1077 | 0x1158 | 0x1159 | 0x115a) {
        let a = g.read::<u32>(w(0xd0));
        let r = g.call(slot_target(g, w(0xca), 0x30 / 4), 0, &[a]);
        g.write::<f32>(w(0xdb), st0_f32(r.st0));
    }
    let y = sub(g.read::<f32>(w(0xe6)), g.read::<f32>(w(0xee)));
    g.write::<f32>(w(0x111), y);
    stores(
        g,
        img,
        f,
        &[0x115, 0x11f, 0x129, 0x133, 0x13d, 0x147, 0x151, 0x15b],
    );
    g.write::<u16>(w(0x167), 0);
    stores(g, img, f, &[0x16b]);
    fade(g, img, fr, e - 0x10, (0xf6, 0xfe, 0, 8, 0x109, 0x176));
    let word = img.word(f + 0x18b) & 0xff;
    stores(g, img, f, &[0x196]);
    fade(
        g,
        img,
        fr,
        e - 0x14,
        (0x17e, 0x186, word, 0x10, 0x192, 0x1a1),
    );
    let s271 = g.read::<u32>(w(0x1a6));
    stores(g, img, f, &[0x1aa]);
    let mut s = Pushes::default();
    s.push_all(&[0, word, 4]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, g.read::<u32>(s271 + 0x43c), 0);
    task_node(g, img, f, 0x1c7, None, (0x1cd, 0x1df, 0x1ec, 0x1fa, 0x202));
    Ret::eax(0)
}

/// [`start_up_0104`] (521 bytes, 1 overlay).
pub const START_UP_0104: Sig = Sig {
    len: 521,
    hash: 0xe856_c40c_9356_58b0,
    calls: &[
        (0x35, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x97, Callee::Sig(&crate::s7_misc::START_OBJECT_RESET_1E)),
        (0xa2, Callee::Sig(&ACTOR_POINT_0104)),
        (0x176, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x1a1, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x1c7, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// A pool node ([`take`](crate::sa_pool::take), `rel32` `call`, owner the
/// `mov ecx, imm32` operand `owner`) with its byte `+0x18` = `flag` (when
/// given) and `+0x4` 0.
fn node<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (call, owner): (u32, u32),
    flag: Option<u8>,
) -> u32 {
    let n = crate::sa_pool::take(g, img, img.rel32_target(f + call), img.word(f + owner));
    if let Some(b) = flag {
        g.write::<u8>(n.wrapping_add(0x18), b);
    }
    g.write::<u32>(n.wrapping_add(4), 0);
    n
}

/// `magic_0053`'s colour start-up (0x100017c0, internal, no arguments; a
/// frame with the index `k = *G_I` (`+0x05`) as its local): as
/// [`colour_start_0065`](crate::r11_misc::colour_start_0065): `E =
/// TABLE[k]` (`+0x0d`), `b` = bit 10 of `*G_F` (`+0x14`), the words, the
/// colour task when `b` ([`colour_task`](crate::s4_small::colour_task)),
/// the state task ([`state_task`](crate::s4_misc::state_task), `ecx`
/// `+0xa4`, the record `+0x95` or `+0x9c`), the actor block
/// ([`me_block`](crate::sa_misc::me_block), `+0xb1`); then task nodes:
/// the first (byte 1) last on the list; for `k` 0 a second one (byte 1)
/// after it; the records block
/// ([`records_init_80`](crate::sa_misc::records_init_80), `+0x150`); when
/// `b` a third (no byte) at the end without becoming the last; and a
/// fourth (byte 2) after the third or the last. Returns 0.
#[allow(clippy::too_many_lines)] // four nodes, each list case in order
pub fn colour_start_0053<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = fr.esp;
    let k = g.read::<u32>(w(0x05));
    let entry = g.read::<u32>(w(0x0d).wrapping_add(k.wrapping_mul(4)));
    let bit = (g.read::<u32>(w(0x14)).cast_signed() >> 10) & 1 != 0;
    stores(g, img, f, &[0x1e]);
    g.write::<u32>(w(0x2a), u32::from(bit));
    // the frame's local: the index
    g.write::<u32>(e - 8, k);
    stores(g, img, f, &[0x31]);
    g.write::<u32>(w(0x3d), entry);
    let state = if bit {
        stores(g, img, f, &[0x43, 0x4d, 0x57, 0x61]);
        g.write::<u32>(w(0x6d), entry);
        stores(g, img, f, &[0x71, 0x7b, 0x85]);
        // push ebp, ecx, esi, edi; the return address
        let colour = Frame {
            at: img.rel32_target(f + 0x90),
            esp: e - 0x14,
            ..*fr
        };
        crate::s4_small::colour_task(g, img, &colour);
        w(0x95)
    } else {
        w(0x9c)
    };
    // push ebx; push ecx; push eax: the state task's word
    g.write::<u32>(e - 0x1c, state);
    g.write::<u32>(state.wrapping_add(4), entry);
    let task = Frame {
        at: img.rel32_target(f + 0xac),
        esp: e - 0x20,
        ecx: w(0xa4),
        ..*fr
    };
    crate::s4_misc::state_task(g, img, &task);
    stores(g, img, f, &[0xb5]);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0xc0), w(0xb1));
    stores(g, img, f, &[0xcc]);
    let n1 = node(g, img, f, (0xd7, 0xc8), Some(1));
    let mut head = g.read::<u32>(w(0xdd));
    if head == 0 {
        head = n1;
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x109), n1);
    } else {
        let tail = g.read::<u32>(w(0xf4));
        g.write::<u32>(n1, tail);
        g.write::<u32>(tail + 4, n1);
    }
    g.write::<u32>(n1.wrapping_add(8), n1);
    g.write::<u32>(w(0x116), n1);
    if k == 0 {
        let n2 = node(g, img, f, (0x122, 0x11d), Some(1));
        // (the head is the first node at least: the empty case never runs)
        if head == 0 {
            g.write::<u32>(n2, 0);
            g.write::<u32>(w(0x143), n2);
        } else {
            g.write::<u32>(n2, n1);
            g.write::<u32>(n1 + 4, n2);
        }
        g.write::<u32>(n2.wrapping_add(8), n2);
        g.write::<u32>(w(0x14b), n2);
    }
    crate::sa_misc::records_init_80(g, img, img.rel32_target(f + 0x155), w(0x150));
    // the list's first and the node the fourth goes after
    let (head, prev) = if bit {
        let n3 = node(g, img, f, (0x165, 0x160), None);
        let head = g.read::<u32>(w(0x16b));
        let head = if head == 0 {
            g.write::<u32>(n3, 0);
            g.write::<u32>(w(0x196), n3);
            n3
        } else {
            let tail = g.read::<u32>(w(0x17e));
            g.write::<u32>(n3, tail);
            g.write::<u32>(tail + 4, n3);
            head
        };
        g.write::<u32>(n3.wrapping_add(8), n3);
        (head, n3)
    } else {
        (g.read::<u32>(w(0x1a1)), g.read::<u32>(w(0x1a7)))
    };
    let n4 = node(g, img, f, (0x1b1, 0x1ac), Some(2));
    if head == 0 {
        g.write::<u32>(n4, 0);
        g.write::<u32>(w(0x1e1), n4);
        g.write::<u32>(n4.wrapping_add(8), n4);
        g.write::<u32>(w(0x1e9), n4);
    } else {
        g.write::<u32>(n4, prev);
        g.write::<u32>(prev + 4, n4);
        g.write::<u32>(n4.wrapping_add(8), n4);
        g.write::<u32>(w(0x1ce), n4);
    }
    Ret::eax(0)
}

/// [`colour_start_0053`] (500 bytes, 1 overlay).
pub const COLOUR_START_0053: Sig = Sig {
    len: 500,
    hash: 0x4b38_b17f_16c7_f87b,
    calls: &[
        (0x90, Callee::Sig(&crate::s4_small::COLOUR_TASK)),
        (0xac, Callee::Sig(&crate::s4_misc::STATE_TASK)),
        (0xc0, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0xd7, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x122, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x155, Callee::Sig(&crate::sa_misc::RECORDS_INIT_80)),
        (0x165, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x1b1, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0053`'s load step (0x10001250, a table entry, no arguments;
/// 145 bytes): nothing more while slot 225 `(1)` is busy; while `*G_I`
/// (`+0x18`) is 0 also while slot 593 `()` answers non-zero; then as
/// [`load_step`](crate::r9_load::load_step): the first time slot 226
/// `(*G_RES, 0)` and `G_ASKED` = 1 (`+0x31`, `+0x52`), once slot 227
/// `(*G_RES)` reports the data loaded the start-up
/// ([`colour_start_0053`]) and two table words. Returns 0.
pub fn load_0053<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    if g.call(slot_target(g, w(0x01), 0x384 / 4), 0, &[1]).eax != 0 {
        return Ret::eax(0);
    }
    if g.read::<u32>(w(0x18)) == 0 && g.call(slot_target(g, w(0x1f), 0x944 / 4), 0, &[]).eax != 0 {
        return Ret::eax(0);
    }
    if g.read::<u32>(w(0x31)) == 0 {
        let res = g.read::<u32>(w(0x41));
        g.call(slot_target(g, w(0x39), 0x388 / 4), 0, &[res, 0]);
        g.write::<u32>(w(0x52), 1);
    }
    let res = g.read::<u32>(w(0x61));
    if g.call(slot_target(g, w(0x5b), 0x38c / 4), 0, &[res])
        .eax
        .cast_signed()
        < 1
    {
        return Ret::eax(0);
    }
    // the call's return address goes below the entry's
    let start = Frame {
        at: img.rel32_target(f + 0x76),
        esp: fr.esp.wrapping_sub(4),
        ..*fr
    };
    colour_start_0053(g, img, &start);
    g.write::<u32>(w(0x7c), w(0x80));
    g.write::<u32>(w(0x86), w(0x8a));
    Ret::eax(0)
}

/// [`load_0053`] (145 bytes, 1 overlay).
pub const LOAD_0053: Sig = Sig {
    len: 145,
    hash: 0x0d85_95f9_510d_117b,
    calls: &[(0x76, Callee::Sig(&COLOUR_START_0053))],
    imports: &[],
};

/// Where [`summon_task`]'s two copies differ: the operand offsets from
/// the state-1 way on (`magic_0095`'s shift as its point call does) and
/// the point and height calls themselves.
struct SummonTaskLayout {
    /// The state halfword in the summon block (`+0x814` or `+0x810`).
    state: u32,
    /// The state-0 way's threshold: the block's `+0x810` (`None`) or a
    /// global (`Some(operand)`).
    thr0: Option<u32>,
    /// Slot 325 `(B + p, B + 0x394)` (`magic_0094`) or slot 268 `(*A,
    /// 0x11, B + p)` then `w` 1.0 (`magic_0095`): the table operand and,
    /// for slot 268, the actor's.
    point: [(u32, Option<u32>); 2],
    /// The state-0 height step: `model_direct`'s `rel32` and the byte
    /// operand, then the two constants (one when there is no choice).
    height: (Option<(u32, u32)>, u32, u32),
    /// The state-1 operands: threshold, list, append `rel32`, `K`, list,
    /// append `rel32`, the alpha immediate, the insert record's two
    /// words, insert `rel32`, the particle record's global, vtable, two
    /// words, its index immediate, the particle node `rel32`.
    one: [u32; 16],
    /// The state-0 operands, as `one` without the threshold, then slot
    /// 239's table and word.
    zero: [u32; 17],
}

/// `magic_0094` 0x10006780.
const SUMMON_TASK_0094: SummonTaskLayout = SummonTaskLayout {
    state: 0x814,
    thr0: None,
    point: [(0x1b5, None), (0x2bd, None)],
    height: (Some((0x2d4, 0x2cc)), 0x2e8, 0x2f2),
    one: [
        0x18a, 0x19b, 0x1a2, 0x1cd, 0x1db, 0x1e8, 0x21e, 0x22e, 0x23a, 0x23f, 0x244, 0x254, 0x260,
        0x270, 0x278, 0x27d,
    ],
    zero: [
        0x2a3, 0x2aa, 0x2fd, 0x30a, 0x340, 0x350, 0x35c, 0x361, 0x366, 0x376, 0x382, 0x392, 0x39a,
        0x39f, 0x3a4, 0x3b4, 0,
    ],
};

/// `magic_0095` 0x10006710.
const SUMMON_TASK_0095: SummonTaskLayout = SummonTaskLayout {
    state: 0x810,
    thr0: Some(0x29a),
    point: [(0x1a7, Some(0x1bc)), (0x2b7, Some(0x2cc))],
    height: (None, 0x2e5, 0x2e5),
    one: [
        0x18a, 0x19b, 0x1a2, 0x1d5, 0x1e3, 0x1f0, 0x226, 0x236, 0x242, 0x247, 0x24c, 0x25c, 0x268,
        0x278, 0x280, 0x285,
    ],
    zero: [
        0x2ab, 0x2b2, 0x2f3, 0x300, 0x336, 0x346, 0x352, 0x357, 0x35c, 0x36c, 0x378, 0x388, 0x390,
        0x395, 0x39a, 0x3aa, 0,
    ],
};

/// `comiss t, k; jb`: below, or unordered.
fn below(t: f32, k: f32) -> bool {
    matches!(t.partial_cmp(&k), None | Some(core::cmp::Ordering::Less))
}

/// The summon's three-state task (a task callback, cdecl `(task)`; an
/// 8-aligned frame of 0x1c bytes, `L` its six-word record at `E + 0x10`
/// with `E` the frame after three pushes): with `B = task[1]` the timer
/// `B + 0x80c` moves on by `K` (`+0x1d`), then by the state `B + S`:
/// - 2, once the timer reaches `T2` (`+0x4b`): fifteen globals (the
///   colours `K1 - c`, a few 1.0, 0.05, a word 0, a flag word with bit 0
///   cleared and bit 2 set) and the task's node unlinked (its `next`
///   returned);
/// - 1, once the timer reaches `T1`: the node `B + 0x320` appended to the
///   list `G_L` ([`node_append`](crate::sb_misc::node_append)), its point
///   (slot 325 or 268), `B + 0x340 -= K`, the node `B + 0x3fc` appended,
///   its colour (`B + 0x418..`, the alpha an immediate), then
///   [`node_insert`](crate::nodes::node_insert) and
///   [`particle_node`](crate::sb_nodes::particle_node) through `L`, the
///   state up by one (the particle node's `eax` returned);
/// - 0, once the timer reaches `T0`: the same for `B + 0x34c` and `B +
///   0x3d0` (the height step `+0x36c` through
///   [`model_direct`](crate::s6_misc::model_direct) in `magic_0094`), slot
///   239 `(*G, -5, 0)`, the state 1 (1 returned);
/// - else nothing (the state less 2 returned; 0 before a threshold).
#[allow(clippy::too_many_lines)] // three states, in the original's order
fn summon_task<G: Game>(g: &mut G, img: &Image, fr: &Frame, lay: &SummonTaskLayout) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let b = g.read::<u32>(task + 4);
    // push ebp; and esp, -8; sub esp, 0x1c; three pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x1c + 12);
    let rec = e + 0x10;
    let timer = add(g.read::<f32>(b + 0x80c), k(g, 0x1d));
    let state = u32::from(g.read::<u16>(b + lay.state));
    g.write::<f32>(b + 0x80c, timer);
    // the point call of a way: slot 325 `(B + p, B + 0x394)` or slot 268
    // `(*A, 0x11, B + p)` and `w` 1.0
    let point = |g: &mut G, (table, actor): (u32, Option<u32>), p: u32| match actor {
        None => {
            g.call(slot_target(g, w(table), 0x514 / 4), 0, &[b + p, b + 0x394]);
        }
        Some(a) => {
            let a = g.read::<u32>(w(a));
            g.call(slot_target(g, w(table), 0x430 / 4), 0, &[a, 0x11, b + p]);
            g.write::<u32>(b + p + 0xc, 0x3f80_0000);
        }
    };
    // the two nodes' records through `L` (`o` from the alpha on): the
    // insert's (`[e + 0x24]` the second node, `ecx` after its append),
    // then the particle node's
    let records =
        |g: &mut G, o: &[u32], (first, second, part, node): (u32, u32, u32, u32), list: u32| {
            g.write::<u32>(rec + 0x14, b + second);
            let es = b + part;
            for (i, v) in (0u32..).zip([es, list, w(o[1]), b + first, w(o[2])]) {
                g.write::<u32>(rec + 4 * i, v);
            }
            // `lea ecx, [esp + 0x10]`; the return address below the frame
            let ins = Frame {
                at: call(o[3]),
                esp: e - 4,
                ecx: rec,
                ..*fr
            };
            crate::nodes::node_insert(g, img, &ins);
            let a = g.read::<u32>(w(o[4]));
            let node = b + node;
            g.write::<u32>(rec, node);
            g.write::<u32>(node, w(o[5]));
            for (i, v) in (1u32..).zip([w(o[6]), es, a, w(o[7]), w(o[8])]) {
                g.write::<u32>(rec + 4 * i, v);
            }
            crate::sb_nodes::particle_node(g, img, call(o[9]), rec)
        };
    match state {
        2 => {
            if below(timer, k(g, 0x4b)) {
                return Ret::eax(0);
            }
            let k1 = k(g, 0x59);
            let c = sub(k1, g.read::<f32>(w(0x64)));
            g.write::<u32>(w(0x69), 0);
            let flags = g.read::<u32>(w(0x6e));
            stores(g, img, f, &[0x72, 0x7c]);
            g.write::<f32>(w(0x8a), c);
            let c = sub(k1, g.read::<f32>(w(0x95)));
            stores(g, img, f, &[0x99, 0xa3, 0xad, 0xb7]);
            g.write::<f32>(w(0xc5), c);
            let c = sub(k1, g.read::<f32>(w(0xd0)));
            stores(g, img, f, &[0xd4, 0xde, 0xe8]);
            g.write::<f32>(w(0xf9), c);
            let c = sub(k1, g.read::<f32>(w(0x104)));
            g.write::<u32>(w(0x10c), (flags & !1) | 4);
            g.write::<f32>(w(0x114), c);
            let c = sub(k1, g.read::<f32>(w(0x11f)));
            g.write::<f32>(w(0x127), c);
            let c0 = sub(k1, g.read::<f32>(w(0x132)));
            let c1 = sub(k1, g.read::<f32>(w(0x13a)));
            g.write::<f32>(w(0x142), c0);
            let c2 = sub(g.read::<f32>(w(0x14a)), g.read::<f32>(w(0x152)));
            g.write::<f32>(w(0x15a), c1);
            g.write::<f32>(w(0x162), c2);
            // the task's node leaves its ring
            let n = g.read::<u32>(task);
            let next = g.read::<u32>(n + 4);
            let prev = g.read::<u32>(n + 8);
            g.write::<u32>(next + 8, prev);
            let prev = g.read::<u32>(n + 8);
            let next = g.read::<u32>(n + 4);
            g.write::<u32>(prev + 4, next);
            g.write::<u32>(n + 8, n);
            g.write::<u32>(n + 4, n);
            Ret::eax(next)
        }
        1 => {
            let o = &lay.one;
            if below(timer, k(g, o[0])) {
                return Ret::eax(0);
            }
            let list = w(o[1]);
            crate::sb_misc::node_append(g, img, call(o[2]), b + 0x320, list, 0);
            point(g, lay.point[0], 0x33c);
            let y = sub(g.read::<f32>(b + 0x340), k(g, o[3]));
            let list = w(o[4]);
            g.write::<f32>(b + 0x340, y);
            crate::sb_misc::node_append(g, img, call(o[5]), b + 0x3fc, list, 0);
            for c in [0x418u32, 0x41c, 0x420] {
                g.write::<u32>(b + c, 0x3f80_0000);
            }
            g.write::<u32>(b + 0x424, w(o[6]));
            let r = records(g, &o[6..], (0x320, 0x3fc, 0x6c0, 0x220), list);
            let s = g.read::<u16>(b + lay.state).wrapping_add(1);
            g.write::<u16>(b + lay.state, s);
            Ret::eax(r)
        }
        0 => {
            let o = &lay.zero;
            let thr = match lay.thr0 {
                None => g.read::<f32>(b + 0x810),
                Some(op) => k(g, op),
            };
            if below(timer, thr) {
                return Ret::eax(0);
            }
            let list = w(o[0]);
            crate::sb_misc::node_append(g, img, call(o[1]), b + 0x34c, list, 0);
            point(g, lay.point[1], 0x368);
            let x = g.read::<f32>(b + 0x36c);
            let step = match lay.height.0 {
                Some((md, byte)) => {
                    let h = u32::from(g.read::<u8>(w(byte)));
                    let direct = Frame {
                        at: call(md),
                        esp: e - 4,
                        ecx: h,
                        ..*fr
                    };
                    if crate::s6_misc::model_direct(g, img, &direct).eax != 0 {
                        lay.height.1
                    } else {
                        lay.height.2
                    }
                }
                None => lay.height.1,
            };
            let x = add(x, k(g, step));
            let list = w(o[2]);
            g.write::<f32>(b + 0x36c, x);
            crate::sb_misc::node_append(g, img, call(o[3]), b + 0x3d0, list, 0);
            for c in [0x3ecu32, 0x3f0, 0x3f4] {
                g.write::<u32>(b + c, 0x3f80_0000);
            }
            g.write::<u32>(b + 0x3f8, w(o[4]));
            records(g, &o[4..], (0x34c, 0x3d0, 0x574, 0x120), list);
            let v = g.read::<u32>(w(o[15]));
            // call width: 3
            g.call(
                slot_target(g, w(o[14]), 0x3bc / 4),
                0,
                &[v, (-5i32).cast_unsigned(), 0],
            );
            g.write::<u16>(b + lay.state, 1);
            Ret::eax(1)
        }
        // dec eax; dec eax: what is left of the state
        _ => Ret::eax(state.wrapping_sub(2)),
    }
}

/// [`summon_task`] in `magic_0094` (0x10006780, 976 bytes).
pub fn summon_task_0094<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    summon_task(g, img, fr, &SUMMON_TASK_0094)
}

/// [`summon_task`] in `magic_0095` (0x10006710, 966 bytes).
pub fn summon_task_0095<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    summon_task(g, img, fr, &SUMMON_TASK_0095)
}

/// [`summon_task_0094`].
pub const SUMMON_TASK_0094_SIG: Sig = Sig {
    len: 976,
    hash: 0x0f2c_3e2f_07ac_08fc,
    calls: &[
        (0x1a2, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1e8, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x23f, Callee::Sig(&crate::nodes::INSERT)),
        (0x27d, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x2aa, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x2d4, Callee::Sig(&crate::s6_misc::MODEL_DIRECT)),
        (0x30a, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x361, Callee::Sig(&crate::nodes::INSERT)),
        (0x39f, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// [`summon_task_0095`].
pub const SUMMON_TASK_0095_SIG: Sig = Sig {
    len: 966,
    hash: 0x7d0b_885e_7cd1_de85,
    calls: &[
        (0x1a2, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1f0, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x247, Callee::Sig(&crate::nodes::INSERT)),
        (0x285, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x2b2, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x300, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x357, Callee::Sig(&crate::nodes::INSERT)),
        (0x395, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// A task node ([`node`]) appended to the task list: `first` the head
/// word tested, `last` the tail read; an empty list gets the node as its
/// head `first_w`; either way the tail `last_w` is the node.
fn append_node<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    n: u32,
    (first, last, first_w, last_w): (u32, u32, u32, u32),
) {
    let w = |o: u32| img.word(f + o);
    let head = g.read::<u32>(w(first));
    if head == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(first_w), n);
    } else {
        let tail = g.read::<u32>(w(last));
        g.write::<u32>(n, tail);
        g.write::<u32>(tail + 4, n);
    }
    g.write::<u32>(n.wrapping_add(8), n);
    g.write::<u32>(w(last_w), n);
}

/// `magic_0303`'s start-up (0x100017a0, internal, no arguments; a /GS
/// frame of 0x24 bytes, `P` the point at `ebp - 0x24` and `Q` the one at
/// `ebp - 0x14`): five words, the actors looked up
/// ([`lookup_actors`](crate::sb_misc::lookup_actors)), slot 719 `(NAME,
/// 8, 0)` into `G_PART`, the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); the object `+0x77`
/// started ([`start_object_anim_2`](crate::s6_misc::start_object_anim_2),
/// scale `+0x6f`, `(1, -, 0, 0)`); slots 262 and 268 for the actor `*A`
/// (`+0x8e`) into three globals and `P`, `Y` = `P.y`; slot 413 or 215
/// (by `*G_F`, `+0xcd`) of `*G_N` into a record with its words and flag
/// halfword; the camera records
/// ([`camera_records_start`](crate::s5_misc::camera_records_start)); slots
/// 262 and 268 for `*A` again into `Q` and `P`, a record cleared by the
/// index `*G_I` (`+0x1d6`, 0x148 bytes each), the camera's point and
/// words; the task started ([`task_start`](crate::s3_misc::task_start),
/// the immediate `+0x20a`); two task nodes (bytes 1 and 2) last on the
/// list around the pool's reset take
/// ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take)). Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0303<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let (p, q) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    stores(g, img, f, &[0x11, 0x1b, 0x25, 0x2f, 0x39]);
    crate::sb_misc::lookup_actors(g, img, call(0x44));
    let part = g
        .call(slot_target(g, w(0x49), 0xb3c / 4), 0, &[w(0x58), 8, 0])
        .eax;
    g.write::<u32>(w(0x62), part);
    crate::l5_objhost::part_init(g, img, call(0x67));
    // push esi; four words; the return address
    let esp = ebp.wrapping_sub(0x3c);
    for (i, v) in (1u32..).zip([1, 0, 0, 0]) {
        g.write::<u32>(esp + 4 * i, v);
    }
    let start = Frame {
        at: call(0x83),
        esp,
        edx: w(0x77),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x6f)))],
        ..*fr
    };
    crate::s6_misc::start_object_anim_2(g, img, &start);
    let a = g.read::<u32>(w(0x8e));
    // (the object start's words stay under these: `ecx` among them, as
    // part_init left it; the calls pass only their own)
    let mut s = Pushes::default();
    s.push_all(&[w(0x99), w(0x9e), w(0xa3), a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x88), 0x418 / 4), 0);
    s.push_all(&[p, 6, a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xaf), 0x430 / 4), 0);
    let id = g.read::<u32>(w(0xbf));
    let y = g.read::<f32>(p + 4);
    let flag = g.read::<u32>(w(0xcd));
    g.write::<u32>(w(0xd3), id);
    g.write::<f32>(w(0xe1), y);
    stores(g, img, f, &[0xe5, 0xef, 0xf9]);
    let slot = if flag == 0 { 0x35c } else { 0x674 };
    let r = g.call(slot_target(g, w(0xd9), slot / 4), 0, &[id]).eax;
    let flag = g.read::<u32>(w(0x11a));
    g.write::<u32>(w(0x125), r);
    stores(g, img, f, &[0x129, 0x133]);
    g.write::<u32>(w(0x13f), 1);
    if flag == 0 {
        g.write::<u16>(w(0x149), 0);
    } else {
        g.write::<u16>(w(0x152), 1);
    }
    stores(g, img, f, &[0x156, 0x160]);
    g.write::<u16>(w(0x16d), 0);
    g.write::<u8>(w(0x175), 1);
    stores(g, img, f, &[0x179, 0x183]);
    g.write::<u16>(w(0x190), 0);
    g.write::<u8>(w(0x198), 1);
    // push esi; the return address
    let cam = Frame {
        at: call(0x19d),
        esp: ebp.wrapping_sub(0x2c),
        ..*fr
    };
    crate::s5_misc::camera_records_start(g, img, &cam);
    let a = g.read::<u32>(w(0x1a3));
    let mut s = Pushes::default();
    s.push_all(&[q + 8, q + 4, q, a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1b4), 0x418 / 4), 0);
    s.push_all(&[p, 6, a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1c6), 0x430 / 4), 0);
    let i = g.read::<u32>(w(0x1d6));
    let x0 = g.read::<f32>(q);
    g.write::<u32>(w(0x1ec).wrapping_add(i.wrapping_mul(0x148)), 0);
    let x1 = g.read::<f32>(p + 4);
    g.write::<f32>(w(0x1f8), x0);
    let z = g.read::<f32>(q + 8);
    g.write::<f32>(q + 4, x1);
    g.write::<u32>(q + 0xc, 0x3f80_0000);
    stores(g, img, f, &[0x215, 0x21f, 0x229, 0x233, 0x23d]);
    g.write::<f32>(w(0x24b), x1);
    g.write::<f32>(w(0x253), z);
    // push esi; one word left, one pushed; the return address
    let esp = ebp.wrapping_sub(0x34);
    g.write::<u32>(esp + 4, w(0x20a));
    g.write::<u32>(esp + 8, q + 8);
    let task = Frame {
        at: call(0x258),
        esp,
        ..*fr
    };
    crate::s3_misc::task_start(g, img, &task);
    stores(g, img, f, &[0x264]);
    let n = node(g, img, f, (0x26f, 0x260), Some(1));
    append_node(g, img, f, n, (0x275, 0x28a, 0x29c, 0x2a9));
    crate::o6_fam_pool::pool_reset_take(g, img, call(0x2ae), w(0x2a1));
    let n = node(g, img, f, (0x2b8, 0x2b3), Some(2));
    append_node(g, img, f, n, (0x2be, 0x2d2, 0x2e4, 0x2ef));
    g.write::<u32>(w(0x2f7), 0);
    Ret::eax(0)
}

/// [`start_up_0303`] (778 bytes, 1 overlay).
pub const START_UP_0303: Sig = Sig {
    len: 778,
    hash: 0xc846_7913_d04e_2741,
    calls: &[
        (0x44, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x67, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x83, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x19d, Callee::Sig(&crate::s5_misc::CAMERA_RECORDS_START)),
        (0x258, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0x26f, Callee::Sig(&crate::sa_pool::TAKE)),
        (
            0x2ae,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0243_2800),
        ),
        (0x2b8, Callee::Sig(&crate::sa_pool::TAKE)),
        // __security_check_cookie
        (0x302, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0176`'s start-up (0x10001390, internal, no arguments; an
/// 8-aligned /GS frame of 0x2c bytes, `E` its base, the point `P` at `E +
/// 0x14`): slot 719 `(NAME, 8, 0)` into `G_PART` (two words stored
/// first), the part set up ([`part_init`](crate::l5_objhost::part_init));
/// the object `+0x58` started
/// ([`start_object_at`](crate::s4_misc::start_object_at), scale `+0x50`,
/// `(0, 0, -, 0)`); slot 262 `(*A, X, Y, Z)` into three globals, `W` 1.0,
/// the yaw `K - ` slot 333 `(*A)` wrapped once into `+0x101`; the record
/// words; the second object `+0xf9` (scale `+0xe8`, `(1, 0x17, -, CB)`);
/// for the first actor `C = CHRS[*SEL]` slot 262 into `Q` and slot 268
/// into `P`, `Q.y = P.y - K2`, `Q.w` 1.0, -2.0 into a word; with `d = Q -
/// X` the pitch `atan2(dy, sqrt(dz dz + dx dx))` and the yaw `K3 -
/// atan2(dz, dx)` wrapped once, plus pi, wrapped again (the square root
/// is `_libm_sse2_sqrt_precise`, the arcs `_CIatan2`, each rounded to
/// `f32`); the actor block ([`me_block`](crate::sa_misc::me_block)); a
/// free-list node ([`pop_node`](crate::m0032::pop_node)) last on the task
/// list. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0176<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x2c
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x2c);
    let p = e + 0x14;
    stores(g, img, f, &[0x29, 0x33]);
    let part = g
        .call(slot_target(g, w(0x15), 0xb3c / 4), 0, &[w(0x25), 8, 0])
        .eax;
    g.write::<u32>(w(0x43), part);
    crate::l5_objhost::part_init(g, img, call(0x48));
    crate::s4_misc::start_object_at(g, img, call(0x64), w(0x58), k(g, 0x50), 0, 0, 0);
    let a = g.read::<u32>(w(0x84));
    // call width: 4
    g.call(
        slot_target(g, w(0x69), 0x418 / 4),
        0,
        &[a, w(0x7e), w(0x79), w(0x6e)],
    );
    let a = g.read::<u32>(w(0x91));
    stores(g, img, f, &[0x95]);
    let r = st0_f32(g.call(slot_target(g, w(0x8b), 0x534 / 4), 0, &[a]).st0);
    let t = k(g, 0xbd);
    let yaw = sub(k(g, 0xaf), r);
    let yaw = if yaw > k(g, 0xc7) {
        sub(yaw, t)
    } else if k(g, 0xd7) > yaw {
        add(yaw, t)
    } else {
        yaw
    };
    g.write::<f32>(w(0x101), yaw);
    stores(
        g,
        img,
        f,
        &[
            0x105, 0x10f, 0x119, 0x123, 0x12d, 0x137, 0x141, 0x14b, 0x155,
        ],
    );
    g.write::<u16>(w(0x161), 0);
    stores(g, img, f, &[0x165]);
    crate::s4_misc::start_object_at(g, img, call(0x170), w(0xf9), k(g, 0xe8), 1, 0x17, w(0xed));
    let sel = g.read::<u32>(w(0x175));
    let b = u32::from(g.read::<u8>(sel));
    let c = g.read::<u32>(w(0x189).wrapping_add(4 * b));
    // call width: 4
    g.call(
        slot_target(g, w(0x18e), 0x418 / 4),
        0,
        &[c, w(0x193), w(0x182), w(0x17a)],
    );
    // call width: 3
    g.call(slot_target(g, w(0x1a6), 0x430 / 4), 0, &[c, 6, p]);
    let y = sub(g.read::<f32>(p + 4), k(g, 0x1bf));
    let dx = sub(k(g, 0x1c7), k(g, 0x1cf));
    let dz = sub(k(g, 0x1d7), k(g, 0x1df));
    g.write::<f32>(w(0x1e7), y);
    let dy = sub(y, k(g, 0x1ef));
    stores(g, img, f, &[0x1f6, 0x20a]);
    let s = add(mul(dz, dz), mul(dx, dx));
    let h = f64::from(s).sqrt();
    let pitch = g.atan2(f64::from(dy), h);
    #[allow(clippy::cast_possible_truncation)]
    let yaw0 = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let (hi, lo, turn) = (k(g, 0x295), k(g, 0x29d), k(g, 0x2b0));
    let wrap = |x: f32| {
        if x > hi {
            sub(x, turn)
        } else if lo > x {
            add(x, turn)
        } else {
            x
        }
    };
    let yaw = wrap(sub(k(g, 0x28d), yaw0));
    #[allow(clippy::cast_possible_truncation)]
    g.write::<f32>(w(0x2d7), pitch as f32);
    let yaw = wrap(add(yaw, hi));
    g.write::<f32>(w(0x2f8), yaw);
    crate::sa_misc::me_block(g, img, call(0x2fd), w(0x2f0));
    let n = crate::m0032::pop_node(g, img, call(0x305));
    let head = g.read::<u32>(w(0x30b));
    g.write::<u32>(n.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(0x32d), n);
    } else {
        let tail = g.read::<u32>(w(0x31b));
        g.write::<u32>(n, tail);
        g.write::<u32>(tail + 4, n);
    }
    g.write::<u32>(n.wrapping_add(8), n);
    g.write::<u32>(w(0x33a), n);
    Ret::eax(0)
}

/// [`start_up_0176`] (843 bytes, 1 overlay).
pub const START_UP_0176: Sig = Sig {
    len: 843,
    hash: 0x6e45_5101_5600_840a,
    calls: &[
        (0x48, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x64, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x170, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x239, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x24c, Callee::Import("_CIatan2")),
        (0x27b, Callee::Import("_CIatan2")),
        (0x2fd, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x305, Callee::Sig(&crate::m0032::POP_NODE)),
        // __security_check_cookie
        (0x343, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0367`'s sparks at a point (0x100015c0, internal, `ecx` = the
/// point `P`, four words; a frame of 0xc bytes): `1 + rand() % 3` small
/// sparks, then three big ones, each in the next free record of the
/// 0x50-byte table `*G_T` (`+0x47`; the count `*G_N` `+0x32`, the cursor
/// `G_C` `+0x37` searched on from where it is and wrapping at the count;
/// without a free one the search's cursor is returned): the colours
/// 0x80808080, the part from slot 719 `(NAME, 9, big)`, the point, the
/// three spreads (slot 314 of 0.4 each, or 2.0, 2.0 kept and slot 314 of
/// 2.0), the angle (slot 314 of 2 pi), the speed `(slot 404 (0.5) + K) *
/// K'` (the two `K'`s differ), the life words, the flag byte and `+0x30`
/// -10.0. Returns 1 (the cursor when the table is empty).
#[allow(clippy::too_many_lines)] // two loops, written out as the original
pub fn sparks_0367<G: Game>(g: &mut G, img: &Image, f: u32, point: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    // (count, cursor, table operands) of each loop's search
    let search = |g: &mut G, (n, c, t): (u32, u32, u32)| -> Result<u32, u32> {
        let count = g.read::<i32>(w(n));
        let mut at = g.read::<u32>(w(c));
        if count <= 0 {
            return Err(at);
        }
        let table = g.read::<u32>(w(t));
        for _ in 0..count {
            let rec = table.wrapping_add(at.wrapping_mul(0x50));
            at = at.wrapping_add(1);
            if at.cast_signed() >= count {
                at = 0;
            }
            if g.read::<u32>(rec + 0x48) == 0 {
                g.write::<u32>(w(c), at);
                return Ok(rec);
            }
        }
        Err(at)
    };
    // the record's head: colours, part, point
    let head = |g: &mut G, rec: u32, (table, name): (u32, u32), big: u32| {
        g.write::<u32>(rec + 0x40, 0x8080_8080);
        g.write::<u32>(rec + 0x44, 0x8080_8080);
        g.write::<u32>(rec + 0x3c, 0);
        g.write::<u16>(rec + 0x4c, 0);
        g.write::<u32>(rec + 0x48, 1);
        let part = g
            .call(slot_target(g, w(table), 0xb3c / 4), 0, &[w(name), 9, big])
            .eax;
        g.write::<u32>(rec, part);
        g.write::<u16>(rec + 8, 0xffff);
        g.write::<u32>(rec + 4, 1);
        g.write::<u16>(rec + 0xa, 0x200);
        for k in 0..4 {
            let v = g.read::<u32>(point + 4 * k);
            g.write::<u32>(rec + 0xc + 4 * k, v);
        }
    };
    // slot 314 or 404 of one float (the word the earlier call left,
    // overwritten), rounded to f32
    let float = |g: &mut G, table: u32, slot: u32, x: u32| {
        st0_f32(g.call(slot_target(g, w(table), slot), 0, &[x]).st0)
    };
    let r = g.rand().cast_signed();
    let small = r.wrapping_rem(3) + 1;
    for _ in 0..small {
        let rec = match search(g, (0x32, 0x37, 0x47)) {
            Ok(rec) => rec,
            Err(at) => return at,
        };
        head(g, rec, (0x80, 0xae), 0);
        for (o, table) in [(0x1cu32, 0xe8u32), (0x20, 0x102), (0x24, 0x119)] {
            let x = float(g, table, 0x4e8 / 4, 0x3ecc_cccd);
            g.write::<f32>(rec + o, x);
        }
        let x = float(g, 0x130, 0x4e8 / 4, 0x40c9_0fdb);
        g.write::<f32>(rec + 0x28, x);
        let r = float(g, 0x147, 0x650 / 4, 0x3f00_0000);
        let x = add(r, g.read::<f32>(w(0x166)));
        g.write::<u32>(rec + 0x34, u32::MAX);
        g.write::<u32>(rec + 0x38, 0);
        g.write::<u8>(rec + 0x47, 0x20);
        let x = mul(x, g.read::<f32>(w(0x185)));
        g.write::<u16>(rec + 0x4c, 1);
        g.write::<u32>(rec + 0x30, 0xc120_0000);
        g.write::<f32>(rec + 0x2c, x);
    }
    for _ in 0..3 {
        let rec = match search(g, (0x1b2, 0x1b7, 0x1c7)) {
            Ok(rec) => rec,
            Err(at) => return at,
        };
        head(g, rec, (0x1fe, 0x22f), 1);
        let x = float(g, 0x269, 0x4e8 / 4, 0x4000_0000);
        g.write::<f32>(rec + 0x1c, x);
        g.write::<u32>(rec + 0x20, 0x4000_0000);
        let x = float(g, 0x283, 0x4e8 / 4, 0x4000_0000);
        g.write::<f32>(rec + 0x24, x);
        let x = float(g, 0x2a1, 0x4e8 / 4, 0x40c9_0fdb);
        g.write::<f32>(rec + 0x28, x);
        let r = float(g, 0x2b8, 0x650 / 4, 0x3f00_0000);
        let x = add(r, g.read::<f32>(w(0x2d7)));
        g.write::<u32>(rec + 0x34, 0x10);
        let x = mul(x, g.read::<f32>(w(0x2ef)));
        g.write::<u32>(rec + 0x38, 0x10);
        g.write::<u8>(rec + 0x47, 0);
        g.write::<f32>(rec + 0x2c, x);
        g.write::<u16>(rec + 0x4c, 1);
        g.write::<u32>(rec + 0x30, 0xc120_0000);
    }
    1
}

/// [`sparks_0367`] (798 bytes, 1 overlay).
pub const SPARKS_0367: Sig = Sig {
    len: 798,
    hash: 0x61e0_4f6e_70d4_fbbf,
    calls: &[],
    imports: &[(0xe, "rand")],
};

/// `magic_0367`'s sparks by the actor's motion (0x100018e0, an overlay
/// table entry, no arguments; a /GS frame, `P` the point at `ebp -
/// 0x14`): the motion `slot 22 (*A) >> 8` (signed); for 2 the bones 5
/// and 7, for 8 the bones 4 and 8: slot 268 `(*A, bone, P)`, `P.w` 1.0
/// and [`sparks_0367`] at `P` each time (the first call's words under the
/// second's). Returns the last sparks' answer (the motion less 2 for any
/// other motion).
pub fn motion_sparks_0367<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x14);
    let mut eax = 0;
    let a = g.read::<u32>(w(0x17));
    let m = (g
        .call(slot_target(g, w(0x11), 0x58 / 4), 0, &[a])
        .eax
        .cast_signed()
        >> 8)
        .wrapping_sub(2);
    // (first bone, second bone, the first call's table, actor and
    // sparks operands)
    let (b1, b2, (table, actor, call)) = match m {
        0 => (5, 7, (0x71u32, 0x79u32, 0x90u32)),
        6 => (4, 8, (0x41, 0x49, 0x60)),
        _ => return Ret::eax(m.cast_unsigned()),
    };
    let mut s = Pushes::default();
    for (bone, (table, actor, call)) in [(b1, (table, actor, call)), (b2, (0x9b, 0xa1, 0xbb))] {
        let a = g.read::<u32>(w(actor));
        s.push_all(&[p, bone, a]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(table), 0x430 / 4), 0);
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        eax = sparks_0367(g, img, img.rel32_target(f + call), p);
    }
    Ret::eax(eax)
}

/// [`motion_sparks_0367`] (205 bytes, 1 overlay).
pub const MOTION_SPARKS_0367: Sig = Sig {
    len: 205,
    hash: 0x6c84_550f_8700_36b8,
    calls: &[
        (0x60, Callee::Sig(&SPARKS_0367)),
        (0x90, Callee::Sig(&SPARKS_0367)),
        (0xbb, Callee::Sig(&SPARKS_0367)),
        // __security_check_cookie
        (0xc5, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0349`'s objects of a set (internal, no arguments, no frame
/// words but its locals): with `E = *G_E` (`+0x09`/`+0x0b`), `n` objects
/// at `E + 0x4c0 + 0x130 i` started
/// ([`start_object_at`](crate::s4_misc::start_object_at), the index
/// `index`, the owner the next word of the table `T` (`ops.1`), scale
/// `(slot 404 (0.5) + K) * K'`), each given `E + 0x1a14` and `E +
/// 0x1a20..0x1a2c` at `+0xc4`/`+0xd0..` and the next vector of `V`
/// (`ops.2`, up to `ops.3`) at `+0xe0..`. Returns `E`.
fn objects_0349<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    index: u32,
    (e_op, t_op, v_op, end_op, slot_op, k1, k2, call): (u32, u32, u32, u32, u32, u32, u32, u32),
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let e = g.read::<u32>(w(e_op));
    let mut obj = e + 0x4c0;
    let mut owner = w(t_op);
    let mut v = w(v_op);
    loop {
        let r = st0_f32(
            g.call(slot_target(g, w(slot_op), 0x650 / 4), 0, &[0x3f00_0000])
                .st0,
        );
        let scale = mul(add(r, g.read::<f32>(w(k1))), g.read::<f32>(w(k2)));
        let o = g.read::<u32>(owner);
        crate::s4_misc::start_object_at(
            g,
            img,
            img.rel32_target(f + call),
            obj,
            scale,
            index,
            o,
            0,
        );
        let x = g.read::<u32>(e + 0x1a14);
        g.write::<u32>(obj + 0xc4, x);
        for k in 0..4u32 {
            let x = g.read::<u32>(e + 0x1a20 + 4 * k);
            g.write::<u32>(obj + 0xd0 + 4 * k, x);
        }
        owner = owner.wrapping_add(4);
        for k in 0..4u32 {
            let x = g.read::<u32>(v.wrapping_sub(4).wrapping_add(4 * k));
            g.write::<u32>(obj + 0xe0 + 4 * k, x);
        }
        v = v.wrapping_add(0x10);
        obj += 0x130;
        if v.cast_signed() >= w(end_op).cast_signed() {
            return e;
        }
    }
}

/// The free ring's head `node` taken (the count `count` already read):
/// unlinked, the head `*G_FREE` its next (0 with one left; `next_op` /
/// `zero_op` the stores), the count one less into `count_op`, its words
/// cleared, `+8` itself, `+0x10` the callback immediate `cb_op`, byte
/// `+0x18` 0. Returns the new head and the count.
fn ring_take<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    count: u16,
    (next_op, zero_op, cb_op, count_op): (u32, u32, u32, u32),
) -> (u32, u16) {
    let w = |o: u32| img.word(f + o);
    let (prev, next) = (g.read::<u32>(node), g.read::<u32>(node + 4));
    g.write::<u32>(prev.wrapping_add(4), next);
    let (next, prev) = (g.read::<u32>(node + 4), g.read::<u32>(node));
    g.write::<u32>(next, prev);
    let head = if count > 1 {
        let next = g.read::<u32>(node + 4);
        g.write::<u32>(w(next_op), next);
        next
    } else {
        g.write::<u32>(w(zero_op), 0);
        0
    };
    let count = count.wrapping_sub(1);
    for o in [0u32, 4, 0xc, 0x14] {
        g.write::<u32>(node + o, 0);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 0x10, w(cb_op));
    g.write::<u8>(node + 0x18, 0);
    g.write::<u16>(w(count_op), count);
    (head, count)
}

/// The colour block `B = E + 0x1a60`'s words (`first` at `+0..+8`,
/// `second` at `+0x10..+0x18`, 1.0 at `+0xc` and `+0x1c`) after the two
/// globals `ats` (as `mov dword [G], imm`).
fn colour_block_0349<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    b: u32,
    ats: [u32; 2],
    first: u32,
    second: u32,
) {
    crate::r11_fades::stores(g, img, f, &ats);
    for o in [8u32, 4, 0] {
        g.write::<u32>(b + o, first);
    }
    g.write::<u32>(b + 0xc, 0x3f80_0000);
    for o in [0x18u32, 0x14, 0x10] {
        g.write::<u32>(b + o, second);
    }
    g.write::<u32>(b + 0x1c, 0x3f80_0000);
}

/// `magic_0349`'s first set (0x10004ed0, internal, no arguments; a frame
/// of 8 bytes): five objects ([`objects_0349`], index 3); the colour
/// block `B` (`E + 0x1a60`: 0.5s, then 1.0s) with its counts; a node of
/// the free ring (`*G_FREE` `+0x15c`, the u16 count `+0xe6`; none taken
/// when it is 0, and the original then writes through address 0) with
/// byte 1 and `B` last on the task list (`+0x1c3`, `+0x1d9`, `+0x1ec`,
/// `+0x1fe`); `B + 4` 0.65. Returns the list's old tail (0xffff when it
/// was empty).
pub fn first_set_0349<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = objects_0349(
        g,
        img,
        f,
        3,
        (0x0b, 0x12, 0x1d, 0xd3, 0x22, 0x42, 0x54, 0x5e),
    );
    let b = e + 0x1a60;
    let count = g.read::<u16>(w(0xe6));
    colour_block_0349(g, img, f, b, [0xea, 0xf4], 0x3f00_0000, 0x3f80_0000);
    for (o, v) in [(0x28u32, 0x28u32), (0x34, 0), (0x30, 0x1e), (0x2c, 0x1e)] {
        g.write::<u32>(b + o, v);
    }
    let mut eax = 0;
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0x15c));
        ring_take(g, img, f, node, count, (0x179, 0x181, 0x1b2, 0x1bd));
        eax = 0xffff;
        node
    };
    let head = g.read::<u32>(w(0x1c3));
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(0x14), b);
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x1ec), node);
    } else {
        let tail = g.read::<u32>(w(0x1d9));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        eax = tail;
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(b + 4, 0x3f26_6666);
    g.write::<u32>(w(0x1fe), node);
    Ret::eax(eax)
}

/// [`first_set_0349`] (519 bytes, 1 overlay).
pub const FIRST_SET_0349: Sig = Sig {
    len: 519,
    hash: 0xc74f_d876_7898_7cb4,
    calls: &[(0x5e, Callee::Sig(&crate::s4_misc::START_OBJECT_AT))],
    imports: &[],
};

/// `magic_0349`'s second set (0x10004b00, internal, no arguments; a frame
/// of 0xc bytes): seven objects ([`objects_0349`], index 2); the block
/// `A = E + 0x1a30` (words, a halfword 0) and a free-ring node for it
/// (byte 1) last on the task list; `A`'s rest (0.2, 5.0, 5.0, flags `|=
/// 3`, 12); the colour block `B` (1.0s, then 0.5s) and a second node
/// for it (byte 1) after the first, `B + 0x14` 0.65. With an empty ring
/// the take is skipped (the original then writes through address 0).
/// Returns 0xffff after both takes (else the list's old tail, or 0).
#[allow(clippy::too_many_lines)] // two nodes, each list case in order
pub fn second_set_0349<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = objects_0349(
        g,
        img,
        f,
        2,
        (0x09, 0x1b, 0x20, 0xd6, 0x25, 0x45, 0x57, 0x61),
    );
    let a = e + 0x1a30;
    let count = g.read::<u16>(w(0xe9));
    let mut eax = 0;
    g.write::<u32>(a, 0);
    g.write::<u32>(a + 0x1c, 0);
    g.write::<u32>(a + 0x20, 0x3d23_d70a);
    g.write::<u32>(a + 0x24, 0);
    g.write::<u32>(a + 0x28, 0x19);
    g.write::<u16>(a + 0x2c, 0);
    let (first, free, count) = if count == 0 {
        (0, g.read::<u32>(w(0x11c)), 0)
    } else {
        let node = g.read::<u32>(w(0x126));
        let (prev, next) = (g.read::<u32>(node), g.read::<u32>(node + 4));
        g.write::<u32>(prev.wrapping_add(4), next);
        let (next, prev) = (g.read::<u32>(node + 4), g.read::<u32>(node));
        g.write::<u32>(next, prev);
        let free = if count > 1 {
            g.read::<u32>(node + 4)
        } else {
            0
        };
        eax = 0xffff;
        let count = count.wrapping_sub(1);
        g.write::<u32>(w(0x150), free);
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node + o, 0);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 0x10, w(0x175));
        g.write::<u8>(node + 0x18, 0);
        g.write::<u16>(w(0x180), count);
        (node, free, count)
    };
    let head = g.read::<u32>(w(0x186));
    g.write::<u8>(first.wrapping_add(0x18), 1);
    g.write::<u32>(first.wrapping_add(0x14), a);
    g.write::<u32>(first.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0x1af), first);
    } else {
        let tail = g.read::<u32>(w(0x19c));
        g.write::<u32>(first, tail);
        g.write::<u32>(tail.wrapping_add(4), first);
        eax = tail;
    }
    g.write::<u32>(first.wrapping_add(8), first);
    for (o, v) in [
        (4u32, 0u32),
        (8, 0x3e4c_cccd),
        (0xc, 0),
        (0x10, 0x40a0_0000),
        (0x14, 0x40a0_0000),
        (0x18, 0),
    ] {
        g.write::<u32>(a + o, v);
    }
    let fl = g.read::<u16>(a + 0x2c) | 3;
    g.write::<u16>(a + 0x2c, fl);
    g.write::<u32>(a + 0x24, 0xc);
    let b = e + 0x1a60;
    colour_block_0349(g, img, f, b, [0x1f5, 0x1ff], 0x3f80_0000, 0x3f00_0000);
    for (o, v) in [(0x28u32, 0u32), (0x34, 0), (0x30, 0x1e), (0x2c, 0x1e)] {
        g.write::<u32>(b + o, v);
    }
    let second = if count == 0 {
        0
    } else {
        ring_take(g, img, f, free, count, (0x27e, 0x286, 0x2b7, 0x2c2));
        eax = 0xffff;
        free
    };
    let head = g.read::<u32>(w(0x2c8));
    g.write::<u8>(second.wrapping_add(0x18), 1);
    g.write::<u32>(second.wrapping_add(0x14), b);
    g.write::<u32>(second.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x2ec), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first.wrapping_add(4), second);
    }
    g.write::<u32>(second.wrapping_add(8), second);
    g.write::<u32>(b + 0x14, 0x3f26_6666);
    g.write::<u32>(w(0x2fe), second);
    Ret::eax(eax)
}

/// [`second_set_0349`] (775 bytes, 1 overlay).
pub const SECOND_SET_0349: Sig = Sig {
    len: 775,
    hash: 0x3aee_23f0_3956_e36d,
    calls: &[(0x61, Callee::Sig(&crate::s4_misc::START_OBJECT_AT))],
    imports: &[],
};

/// `magic_0325`'s delayed start (0x10001510, a task callback, cdecl
/// `(task)`; a /GS frame of 0x1c bytes, `P` the point at `ebp - 0x14`):
/// while `task[5]` counts down above 0 nothing more (the task returned).
/// Then the yaw `K - ` slot 333 `(*A)` wrapped once; the object `+0x8a`
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), `(0, 0)`)
/// at the actor `*A`: slot 262 into three globals, slot 268 `(A, 6, P)`,
/// `Y = P.y`, two words and the yaw; six objects `R_i - 0xd4` (`R_i` =
/// `+0x100 + 0x130 i`, `(1, V[i])`), each at the actor (slot 262 into
/// `R_i - 4..`, `R_i = P.y`) with its words from four tables; the object
/// `+0x1c0` (`(2, 0xb4)`) the same way with its record's words; a node of
/// the free ring ([`ring_take`], none for an empty ring: the original
/// writes through address 0) for the record last on the task list; the
/// task started ([`task_start`](crate::s3_misc::task_start)); slot 413
/// or 215 (by `*G_F`) of `*G_N` into a record with its words and flag
/// halfword; `task[2] + 0x10` the next callback. Returns `task[2]`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn delayed_start_0325<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    use crate::s4_misc::start_object_anim;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x14);
    let task = fr.arg(g, 0);
    let left = g.read::<i32>(task + 0x14).wrapping_sub(1);
    g.write::<i32>(task + 0x14, left);
    if left > 0 {
        return Ret::eax(task);
    }
    let a = g.read::<u32>(w(0x2a));
    let r = st0_f32(g.call(slot_target(g, w(0x24), 0x534 / 4), 0, &[a]).st0);
    let yaw = sub(k(g, 0x3d), r);
    let yaw = if yaw > k(g, 0x4c) {
        sub(yaw, k(g, 0x5b))
    } else if k(g, 0x65) > yaw {
        add(yaw, k(g, 0x72))
    } else {
        yaw
    };
    // slots 262 `(A, x, y, z)` and 268 `(A, 6, P)` (operands: the
    // tables and the actor), each over the earlier words
    let place = |g: &mut G, (t1, t2, actor): (u32, u32, u32), xyz: [u32; 3]| {
        let a = g.read::<u32>(w(actor));
        // call width: 4
        g.call(
            slot_target(g, w(t1), 0x418 / 4),
            0,
            &[a, xyz[0], xyz[1], xyz[2]],
        );
        // call width: 3
        g.call(slot_target(g, w(t2), 0x430 / 4), 0, &[a, 6, p]);
        g.read::<f32>(p + 4)
    };
    start_object_anim(g, img, call(0x93), w(0x8a), k(g, 0x7f), 0, 0);
    let y = place(g, (0x98, 0xbf, 0x9e), [w(0xb3), w(0xae), w(0xa9)]);
    g.write::<f32>(w(0xd7), y);
    stores(g, img, f, &[0xe3, 0xed]);
    g.write::<f32>(w(0xfb), yaw);
    let mut rec = w(0x100);
    for i in 0..6u32 {
        let v = g.read::<u32>(w(0x11d) + 4 * i);
        start_object_anim(g, img, call(0x12a), rec - 0xd4, k(g, 0x114), 1, v);
        let y = place(g, (0x13e, 0x150, 0x130), [rec - 4, rec, rec + 4]);
        let c = g.read::<u32>(w(0x166) + 4 * i);
        g.write::<f32>(rec, y);
        let x = add(g.read::<f32>(w(0x172) + 4 * i), k(g, 0x17a));
        g.write::<u32>(rec + 8, 0x3f80_0000);
        g.write::<u32>(rec + 0xc, c);
        let d = g.read::<u32>(w(0x18a) + 4 * i);
        g.write::<u32>(rec + 0x10, d);
        let e = g.read::<u32>(w(0x193) + 4 * i);
        g.write::<f32>(rec + 0x14, x);
        g.write::<u32>(rec - 0x10, e);
        rec += 0x130;
    }
    start_object_anim(g, img, call(0x1cc), w(0x1c0), k(g, 0x1b8), 2, 0xb4);
    let y = place(g, (0x1d1, 0x1f8, 0x1d7), [w(0x1ec), w(0x1e7), w(0x1e2)]);
    let count = g.read::<u16>(w(0x20f));
    g.write::<f32>(w(0x217), y);
    stores(g, img, f, &[0x223]);
    g.write::<f32>(w(0x231), yaw);
    stores(g, img, f, &[0x235, 0x23f]);
    g.write::<u8>(w(0x24b), 0xff);
    g.write::<u16>(w(0x253), 0);
    g.write::<u8>(w(0x25b), 0);
    stores(g, img, f, &[0x260]);
    g.write::<u16>(w(0x26d), 0x48);
    stores(g, img, f, &[0x273, 0x27d, 0x287]);
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0x29c));
        ring_take(g, img, f, node, count, (0x2b9, 0x2c1, 0x2f2, 0x2fd));
        node
    };
    let head = g.read::<u32>(w(0x303));
    g.write::<u32>(node.wrapping_add(0x14), w(0x30d));
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x32f), node);
    } else {
        let tail = g.read::<u32>(w(0x31c));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x33e), node);
    stores(g, img, f, &[0x342]);
    // push ecx (not read); push imm; the return address
    let esp = ebp.wrapping_sub(0x28);
    g.write::<u32>(esp + 4, w(0x335));
    let ts = Frame {
        at: call(0x34d),
        esp,
        ..*fr
    };
    crate::s3_misc::task_start(g, img, &ts);
    let id = g.read::<u32>(w(0x352));
    let flag = g.read::<u32>(w(0x35b));
    g.write::<u32>(w(0x361), id);
    stores(g, img, f, &[0x36b, 0x375, 0x37f]);
    let slot = if flag == 0 { 0x35c } else { 0x674 };
    let r = g.call(slot_target(g, w(0x367), slot / 4), 0, &[id]).eax;
    let flag = g.read::<u32>(w(0x3a0));
    g.write::<u32>(w(0x3ab), r);
    stores(g, img, f, &[0x3af, 0x3b9]);
    g.write::<u32>(w(0x3c5), 1);
    if flag == 0 {
        g.write::<u16>(w(0x3cf), 0);
    } else {
        g.write::<u16>(w(0x3d8), 1);
    }
    let next = g.read::<u32>(task + 8);
    g.write::<u32>(next.wrapping_add(0x10), w(0x3e5));
    Ret::eax(next)
}

/// [`delayed_start_0325`] (1015 bytes, 1 overlay).
pub const DELAYED_START_0325: Sig = Sig {
    len: 1015,
    hash: 0xf8db_1bed_690b_020b,
    calls: &[
        (0x93, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x12a, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x1cc, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x34d, Callee::Sig(&crate::s3_misc::TASK_START)),
        // __security_check_cookie
        (0x3ef, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0195`'s scatter (0x10001000, a table entry, cdecl `(arg)`; a
/// /GS frame of 0x1c bytes, `L` the vector at `ebp - 0x14`): two blocks
/// of seven flags and seven words cleared; then seven times a slot `i =
/// rand() % 7` whose flag is clear (drawn again while it is set): the
/// `k`-th time while `k` is below the effect's count `E[0x19c]` (`E =
/// *G_E`), slot 457 `(E[0x1a0 + 4k], 6, P_i)` (`P_i` the `i`-th point of
/// `+0xb6`), the flag 1 and the word `E[0x1a0 + 4k]`; past the count a
/// random entry `j = rand() % E[0x19c]`: slot 457 `(E[0x1a0 + 4j], 0,
/// P_i)`, slot 469 `(L)`, the flag 2, the word -1 and `P_i += L s` with
/// `s = rand() / K K' + K'`. Then slots 450 `(arg, CB1)` and 449 `(arg,
/// CB2)` (the first's words under the second's), whose `eax` it returns.
pub fn scatter_0195<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    use crate::sb_sse::{cvt, div};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let l = ebp.wrapping_sub(0x14);
    let arg = fr.arg(g, 0);
    stores(g, img, f, &[0x1c, 0x26]);
    for o in [0x34u32, 0x3c, 0x44] {
        g.write::<u64>(w(o), 0);
    }
    stores(g, img, f, &[0x48]);
    for o in [0x56u32, 0x5e, 0x66] {
        g.write::<u64>(w(o), 0);
    }
    stores(g, img, f, &[0x6a]);
    for n in 0..7u32 {
        let edi = 0x1a0 + 4 * n;
        // a slot whose flag is clear (`jne` draws again)
        let i = loop {
            let i = g.rand().cast_signed().wrapping_rem(7).cast_unsigned();
            if g.read::<u32>(w(0x99).wrapping_add(4 * i)) == 0 {
                break i;
            }
        };
        let e = g.read::<u32>(w(0xa2));
        if n.cast_signed() < g.read::<i32>(e + 0x19c) {
            let p = w(0xb6).wrapping_add(16 * i);
            let id = g.read::<u32>(e + edi);
            // call width: 3
            g.call(slot_target(g, w(0xbc), 0x724 / 4), 0, &[id, 6, p]);
            let e = g.read::<u32>(w(0xce));
            g.write::<u32>(w(0xd4).wrapping_add(4 * i), 1);
            let v = g.read::<u32>(e + edi);
            g.write::<u32>(w(0xe4).wrapping_add(4 * i), v);
        } else {
            let r = g.rand().cast_signed();
            let e = g.read::<u32>(w(0xf5));
            let j = r.wrapping_rem(g.read::<i32>(e + 0x19c)).cast_unsigned();
            let p = w(0x110).wrapping_add(16 * i);
            let id = g.read::<u32>(e.wrapping_add(4 * j).wrapping_add(0x1a0));
            let mut s = Pushes::default();
            s.push_all(&[p, 0, id]);
            // call width: 12 (Pushes caps the words at 12)
            s.call(g, slot_target(g, w(0x101), 0x724 / 4), 0);
            s.push(l);
            // call width: 12 (Pushes caps the words at 12)
            s.call(g, slot_target(g, w(0x125), 0x754 / 4), 0);
            let r = g.rand();
            let s = add(mul(div(cvt(r), k(g, 0x14a)), k(g, 0x157)), k(g, 0x169));
            g.write::<u32>(w(0x16f).wrapping_add(4 * i), 2);
            g.write::<u32>(w(0x179).wrapping_add(4 * i), u32::MAX);
            let d: [f32; 4] = core::array::from_fn(|c| mul(g.read::<f32>(l + 4 * c as u32), s));
            for (c, x) in (0u32..).zip(d) {
                g.write::<f32>(l + 4 * c, x);
            }
            // `addss xmm0, xmm1` for x, `addss xmm2, [esi + 4]` (the step
            // first) for the rest
            let x = add(g.read::<f32>(p), d[0]);
            g.write::<f32>(p, x);
            for c in 1..4u32 {
                let x = add(d[c as usize], g.read::<f32>(p + 4 * c));
                g.write::<f32>(p + 4 * c, x);
            }
        }
    }
    let mut s = Pushes::default();
    s.push_all(&[w(0x1f0), arg]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1e2), 0x708 / 4), 0);
    s.push_all(&[w(0x1fd), arg]);
    // call width: 12 (Pushes caps the words at 12)
    let eax = s.call(g, slot_target(g, w(0x1f8), 0x704 / 4), 0).eax;
    Ret::eax(eax)
}

/// [`scatter_0195`] (542 bytes, 1 overlay; the `rel32` at `+0xe9` is a
/// jump inside).
pub const SCATTER_0195: Sig = Sig {
    len: 542,
    hash: 0x189a_8901_a13d_f8fe,
    calls: &[
        // __security_check_cookie
        (0x216, Callee::Any),
    ],
    imports: &[(0x82, "rand"), (0xef, "rand"), (0x136, "rand")],
};

/// `magic_0195`'s aim of an object (0x10001520, a table entry, cdecl
/// `(obj)`; a /GS frame of 0x30 bytes, `L` at `ebp - 0x14` and `D` at
/// `ebp - 0x24`): with `i = *G_I` the object's next key slots get `i` and
/// `*G_B` (at `+0x2c + 16 n` and `+0x30 + 16 n`, `n = obj[0xac]`) and
/// `+0xbc` bit 13; slot 456 `(E[0x198], T[i], L)` gives the start `L`;
/// with the point `P = PTS[i]`, `D = P - L` goes through slot 464 `(D)`;
/// then `L' = L - D K` into `obj + 0xd0` (w 0) and `((P - D K2) - L') K3`
/// into `obj + 0xe0` (w 0), the rest the code's immediates; slots 450
/// `(obj, CB)` and 451 `(obj, 10)` (the earlier words under each), whose
/// `eax` it returns.
pub fn aim_0195<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let (l, d) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    let obj = fr.arg(g, 0);
    let i = g.read::<u32>(w(0x12));
    let n = g.read::<u32>(obj + 0xac);
    g.write::<u32>(obj.wrapping_add(n.wrapping_mul(16)).wrapping_add(0x2c), i);
    let n = g.read::<u32>(obj + 0xac);
    let b = g.read::<u32>(w(0x2d));
    g.write::<u32>(obj.wrapping_add(n.wrapping_add(3).wrapping_mul(16)), b);
    let fl = g.read::<u32>(obj + 0xbc) | 0x2000;
    g.write::<u32>(obj + 0xbc, fl);
    let e = g.read::<u32>(w(0x48));
    let t = g.read::<u32>(w(0x4f).wrapping_add(4 * i));
    let mut s = Pushes::default();
    s.push_all(&[l, t, g.read::<u32>(e + 0x198)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x5a), 0x720 / 4), 0);
    let i = g.read::<u32>(w(0x67));
    let pt = w(0x6f).wrapping_add(16 * i);
    let p: [f32; 4] = core::array::from_fn(|c| g.read::<f32>(pt + 4 * c as u32));
    for c in 0..4u32 {
        let x = sub(p[c as usize], g.read::<f32>(l + 4 * c));
        g.write::<f32>(d + 4 * c, x);
    }
    s.push(d);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x9a), 0x740 / 4), 0);
    let dv: [f32; 4] = core::array::from_fn(|c| g.read::<f32>(d + 4 * c as u32));
    let (k1, k2, k3) = (k(g, 0xdd), k(g, 0x14f), k(g, 0x17d));
    let lv: [f32; 4] =
        core::array::from_fn(|c| sub(g.read::<f32>(l + 4 * c as u32), mul(dv[c], k1)));
    for (c, x) in (0u32..).zip(lv) {
        g.write::<f32>(l + 4 * c, x);
    }
    let a: [f32; 3] = core::array::from_fn(|c| mul(sub(sub(p[c], mul(dv[c], k2)), lv[c]), k3));
    for (c, x) in (0u32..).zip(&lv[..3]) {
        g.write::<f32>(obj + 0xd0 + 4 * c, *x);
    }
    g.write::<u32>(obj + 0xdc, 0);
    for (c, x) in (0u32..).zip(a) {
        g.write::<f32>(obj + 0xe0 + 4 * c, x);
    }
    for (o, v) in [
        (0xecu32, 0u32),
        (0x100, 0x42b4_0000),
        (0x104, 0),
        (0x108, 0),
        (0x10c, 0),
        (0x130, 0x3d4c_cccd),
        (0x134, 0x3ecc_cccd),
        (0x138, 0x3d4c_cccd),
        (0x13c, 0),
        (0x140, 0xba00_7358),
        (0x144, 0x3c23_d70a),
        (0x148, 0xbba0_902e),
        (0x14c, 0),
        (0x180, 0),
        (0x184, 0),
        (0x188, 0),
        (0x18c, 0xbdb8_51ec),
    ] {
        g.write::<u32>(obj + o, v);
    }
    s.push_all(&[w(0x285), obj]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x182), 0x708 / 4), 0);
    s.push_all(&[10, obj]);
    // call width: 12 (Pushes caps the words at 12)
    let eax = s.call(g, slot_target(g, w(0x28d), 0x70c / 4), 0).eax;
    Ret::eax(eax)
}

/// [`aim_0195`] (686 bytes, 1 overlay).
pub const AIM_0195: Sig = Sig {
    len: 686,
    hash: 0xcd46_0b21_1152_6b2e,
    calls: &[
        // __security_check_cookie
        (0x2a6, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0314`'s start-up (0x10001340, internal, no arguments; a /GS
/// frame of 0x18 bytes, `P` the point at `ebp - 0x14`): slot 719 `(NAME,
/// 8, 0)` into `G_PART` (two words stored first), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); the yaw `y = K - ` slot
/// 333 `(*A)` wrapped once; the object `+0xb0`
/// ([`start_object_at`](crate::s4_misc::start_object_at), `(1, 0, -,
/// 0)`) at the actor (slot 262 into three globals, slot 268 into `P`),
/// `y` and `P.y + K2` into its record; the object `+0x121` (`(0, 0x42, -,
/// CB)`) after the record words; for the first actor `C = CHRS[*SEL]`
/// slot 262 into three globals, slot 12's ground into one, `y + K3`
/// wrapped once, the height less `K4`; six objects `R_i - 0x104` (`R_i =
/// +0x241 + 0x130 i`, `(2, 0, -, 0)`) with `R_i[7] = i`, `R_i[-0x10] = y`
/// and `R_i[0]` a callback; slot 413 or 215 (by `*G_F`) of `*G_N` into a
/// record with its words and flag halfword; a node of the free ring
/// ([`ring_take`], none for an empty ring: the original writes through
/// address 0) last on the task list. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0314<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    use crate::s4_misc::start_object_at;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x14);
    stores(g, img, f, &[0x24, 0x2e]);
    let part = g
        .call(slot_target(g, w(0x11), 0xb3c / 4), 0, &[w(0x20), 8, 0])
        .eax;
    g.write::<u32>(w(0x3e), part);
    crate::l5_objhost::part_init(g, img, call(0x43));
    let a = g.read::<u32>(w(0x4e));
    let r = st0_f32(g.call(slot_target(g, w(0x48), 0x534 / 4), 0, &[a]).st0);
    let t = k(g, 0x6e);
    let yaw = sub(k(g, 0x61), r);
    let yaw = if yaw > k(g, 0x78) {
        sub(yaw, t)
    } else if k(g, 0x8d) > yaw {
        add(yaw, t)
    } else {
        yaw
    };
    start_object_at(g, img, call(0xb5), w(0xb0), k(g, 0xa3), 1, 0, 0);
    let a = g.read::<u32>(w(0xc0));
    // call width: 4
    g.call(
        slot_target(g, w(0xba), 0x418 / 4),
        0,
        &[a, w(0xd5), w(0xd0), w(0xcb)],
    );
    // call width: 3
    g.call(slot_target(g, w(0xe1), 0x430 / 4), 0, &[a, 6, p]);
    g.write::<f32>(w(0x104), yaw);
    let y = add(g.read::<f32>(p + 4), k(g, 0x111));
    stores(g, img, f, &[0x125]);
    g.write::<f32>(w(0x133), y);
    stores(
        g,
        img,
        f,
        &[
            0x137, 0x141, 0x14b, 0x155, 0x15f, 0x169, 0x173, 0x17d, 0x187,
        ],
    );
    g.write::<u16>(w(0x193), 0);
    stores(g, img, f, &[0x197]);
    start_object_at(g, img, call(0x1a2), w(0x121), k(g, 0xf9), 0, 0x42, w(0x116));
    let sel = g.read::<u32>(w(0x1a7));
    let b = u32::from(g.read::<u8>(sel));
    let c = g.read::<u32>(w(0x1bb).wrapping_add(4 * b));
    let mut s = Pushes::default();
    s.push_all(&[w(0x1ac), w(0x1b4), w(0x1c5), c]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1c0), 0x418 / 4), 0);
    s.push(c);
    // call width: 12 (Pushes caps the words at 12)
    let h = st0_f32(s.call(g, slot_target(g, w(0x1d3), 0x30 / 4), 0).st0);
    g.write::<f32>(w(0x1e4), h);
    let y2 = add(yaw, k(g, 0x1ec));
    stores(g, img, f, &[0x1f3]);
    let y2 = if y2 > k(g, 0x200) {
        sub(y2, k(g, 0x20a))
    } else if k(g, 0x214) > y2 {
        add(y2, k(g, 0x221))
    } else {
        y2
    };
    g.write::<f32>(w(0x229), y2);
    let v = sub(g.read::<f32>(w(0x231)), k(g, 0x239));
    g.write::<f32>(w(0x249), v);
    let mut rec = w(0x241);
    for i in 0..6u32 {
        start_object_at(g, img, call(0x266), rec - 0x104, k(g, 0x254), 2, 0, 0);
        g.write::<u32>(rec + 0x1c, i);
        g.write::<f32>(rec - 0x40, yaw);
        g.write::<u32>(rec, w(0x279));
        rec += 0x130;
    }
    let flag = g.read::<u32>(w(0x28e));
    let id = g.read::<u32>(w(0x294));
    g.write::<u32>(w(0x29a), id);
    stores(g, img, f, &[0x2a4, 0x2ae]);
    let slot = if flag == 0 { 0x35c } else { 0x674 };
    let r = g.call(slot_target(g, w(0x2a0), slot / 4), 0, &[id]).eax;
    let flag = g.read::<u32>(w(0x2cf));
    g.write::<u32>(w(0x2da), r);
    stores(g, img, f, &[0x2de, 0x2e8]);
    g.write::<u32>(w(0x2f4), 1);
    if flag == 0 {
        g.write::<u16>(w(0x2fe), 0);
    } else {
        g.write::<u16>(w(0x307), 1);
    }
    let count = g.read::<u16>(w(0x30e));
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0x31d));
        ring_take(g, img, f, node, count, (0x33a, 0x342, 0x373, 0x37e));
        node
    };
    let head = g.read::<u32>(w(0x384));
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x3a7), node);
    } else {
        let tail = g.read::<u32>(w(0x394));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x3b5), node);
    Ret::eax(0)
}

/// [`start_up_0314`] (964 bytes, 1 overlay).
pub const START_UP_0314: Sig = Sig {
    len: 964,
    hash: 0x31f4_0ad7_05db_6b7e,
    calls: &[
        (0x43, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xb5, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x1a2, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x266, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        // __security_check_cookie
        (0x3bc, Callee::Any),
    ],
    imports: &[],
};

/// The yaw `K - r` wrapped once (`comiss` + `jbe`: an unordered yaw
/// stays): above `HI` less `T`, below `LO` plus `T'` (operands).
fn yaw_wrap<G: Game>(
    g: &G,
    img: &Image,
    f: u32,
    r: f32,
    (k, hi, t, lo, t2): (u32, u32, u32, u32, u32),
) -> f32 {
    let c = |o: u32| g.read::<f32>(img.word(f + o));
    let y = sub(c(k), r);
    if y > c(hi) {
        sub(y, c(t))
    } else if c(lo) > y {
        add(y, c(t2))
    } else {
        y
    }
}

/// `magic_0381`'s start-up (0x100014f0, internal, no arguments; a frame
/// with one local): five words, the actors looked up
/// ([`lookup_actors`](crate::sb_misc::lookup_actors)), slot 719 `(NAME,
/// 8, 0)` into `G_PART`, the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); two objects
/// ([`start_object_owners`](crate::s4_misc::start_object_owners): `+0x70`
/// `(1, 0x78, 0x6e, CB)` and `+0x12e` `(0, 0, 0, 0)`), each at the actor
/// `*A` (slot 262 into three globals; the first also slot 12's ground),
/// `W` 1.0, the yaw `K - ` slot 333 `(*A)` wrapped once and the height
/// less `K2`; two fades ([`fade`](crate::r11_fades::fade)), slot 271
/// `(4)`, slot 220 `(slot 719 (NAME, 6, 0), 0x40, 0x100)`; slot 413 or 215
/// (by `*G_F`) of `*G_N` and of the first actor's byte into two records
/// with their words and flag halfwords; a task node last on the list.
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0381<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::{fade, stores, task_node};
    use crate::s4_misc::start_object_owners;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let e = fr.esp;
    stores(g, img, f, &[0x05, 0x0f, 0x19]);
    g.write::<u8>(w(0x25), 0x15);
    stores(g, img, f, &[0x2a]);
    crate::sb_misc::lookup_actors(g, img, call(0x35));
    let part = g
        .call(slot_target(g, w(0x3a), 0xb3c / 4), 0, &[w(0x49), 8, 0])
        .eax;
    g.write::<u32>(w(0x53), part);
    crate::l5_objhost::part_init(g, img, call(0x58));
    start_object_owners(
        g,
        img,
        call(0x75),
        w(0x70),
        k(g, 0x60),
        1,
        0x78,
        (0x6e, w(0x65)),
    );
    let a = g.read::<u32>(w(0x80));
    // call width: 4
    g.call(
        slot_target(g, w(0x7a), 0x418 / 4),
        0,
        &[a, w(0x95), w(0x90), w(0x8b)],
    );
    let h = st0_f32(g.call(slot_target(g, w(0x9d), 0x30 / 4), 0, &[a]).st0);
    g.write::<f32>(w(0xa9), h);
    let a = g.read::<u32>(w(0xb4));
    stores(g, img, f, &[0xb8]);
    let r = st0_f32(g.call(slot_target(g, w(0xae), 0x534 / 4), 0, &[a]).st0);
    let yaw = yaw_wrap(g, img, f, r, (0xd1, 0xe8, 0xde, 0xf8, 0xde));
    let scale = k(g, 0x109);
    g.write::<f32>(w(0x111), yaw);
    let y = sub(k(g, 0x119), k(g, 0x121));
    g.write::<f32>(w(0x136), y);
    start_object_owners(g, img, call(0x13b), w(0x12e), scale, 0, 0, (0, 0));
    let a = g.read::<u32>(w(0x15b));
    // call width: 4
    g.call(
        slot_target(g, w(0x140), 0x418 / 4),
        0,
        &[a, w(0x155), w(0x150), w(0x145)],
    );
    let a = g.read::<u32>(w(0x168));
    stores(g, img, f, &[0x16c]);
    let r = st0_f32(g.call(slot_target(g, w(0x162), 0x534 / 4), 0, &[a]).st0);
    let yaw = yaw_wrap(g, img, f, r, (0x185, 0x194, 0x19e, 0x1a8, 0x1b5));
    g.write::<f32>(w(0x1cd), yaw);
    let y = sub(k(g, 0x1d5), k(g, 0x1bd));
    g.write::<f32>(w(0x1f0), y);
    // push ebp, ecx, esi; the word; the return address
    fade(g, img, fr, e - 0x14, (0x1c5, 0x1bd, 0, 0x5a, 0x1e8, 0x1f5));
    stores(g, img, f, &[0x213]);
    fade(
        g,
        img,
        fr,
        e - 0x18,
        (0x1bd, 0x1fd, 0x118, 0x20, 0x20f, 0x21e),
    );
    stores(g, img, f, &[0x227]);
    let mut s = Pushes::default();
    s.push_all(&[0, 0x118, 4]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x223), 0x43c / 4), 0);
    s.push_all(&[0, 6, w(0x24b)]);
    // call width: 12 (Pushes caps the words at 12)
    let h = s.call(g, slot_target(g, w(0x23c), 0xb3c / 4), 0).eax;
    s.push_all(&[0x100, 0x40, h]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x25a), 0x370 / 4), 0);
    let id = g.read::<u32>(w(0x267));
    let flag = g.read::<u32>(w(0x270));
    g.write::<u32>(w(0x276), id);
    stores(g, img, f, &[0x280, 0x28a]);
    let slot = if flag == 0 { 0x35c } else { 0x674 };
    let r = g.call(slot_target(g, w(0x27c), slot / 4), 0, &[id]).eax;
    let flag = g.read::<u32>(w(0x2a8));
    g.write::<u32>(w(0x2b5), r);
    stores(g, img, f, &[0x2b9, 0x2c3]);
    g.write::<u32>(w(0x2cf), 1);
    if flag == 0 {
        g.write::<u16>(w(0x2db), 0);
    } else {
        g.write::<u16>(w(0x2e4), 1);
    }
    let sel = g.read::<u32>(w(0x2e9));
    stores(g, img, f, &[0x2ed, 0x2f7]);
    let b = u32::from(g.read::<u8>(sel));
    g.write::<u32>(w(0x305), b);
    let slot = if flag == 0 { 0x35c } else { 0x674 };
    let r = g.call(slot_target(g, w(0x30b), slot / 4), 0, &[b]).eax;
    let flag = g.read::<u32>(w(0x328));
    g.write::<u32>(w(0x32e), r);
    stores(g, img, f, &[0x332, 0x33c]);
    g.write::<u32>(w(0x348), 1);
    if flag == 0 {
        g.write::<u16>(w(0x352), 0);
    } else {
        g.write::<u16>(w(0x35b), 1);
    }
    task_node(g, img, f, 0x365, None, (0x36b, 0x37c, 0x389, 0x39a, 0x3a2));
    Ret::eax(0)
}

/// [`start_up_0381`] (940 bytes, 1 overlay).
pub const START_UP_0381: Sig = Sig {
    len: 940,
    hash: 0x2f2e_5f27_1e65_2389,
    calls: &[
        (0x35, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x58, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x75, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x13b, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x1f5, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x21e, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x365, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0245`'s start-up (0x10001340, internal, no arguments; an
/// 8-aligned /GS frame of 0x1c bytes, `P` the point at its `+0x14`):
/// slot 719 `(NAME, 8, 0)` into `G_PART` (two words stored first), the
/// parts buffer ([`parts_buffer`](crate::s4_small::parts_buffer)); the
/// object `+0x5a`
/// ([`start_object_reset_bare`](crate::r9_misc::start_object_reset_bare),
/// scale `+0x52`) at the actor `*A` (slot 262 into three globals, `W`
/// 1.0), the yaw `K - ` slot 333 `(*A)` wrapped once, `Y -= K2`, its
/// `+0x64` 0.8; the actor block ([`me_block`](crate::sa_misc::me_block));
/// for each of the `*G_N` actors `C = CHRS[SEL[i]]` slot 262 into `P` and
/// `D[i + 1] = 0x22 - trunc(sqrt(dz dz + dx dx) K3)` (`d = P - X`, `P.w`
/// 1.0; the root `_libm_sse2_sqrt_precise`); the record words; two nodes
/// of the free ring (none for an empty ring: the original then writes
/// through address 0): the first (the record) last on the task list, the
/// second after it. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0245<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x1c; three pushes
    let base = fr.esp.wrapping_sub(4) & !7;
    let p = base.wrapping_sub(0x14);
    stores(g, img, f, &[0x2b, 0x35]);
    let part = g
        .call(slot_target(g, w(0x15), 0xb3c / 4), 0, &[w(0x27), 8, 0])
        .eax;
    g.write::<u32>(w(0x45), part);
    // the three pushes, two of slot 719's words; the return address
    let pb = Frame {
        at: call(0x4a),
        esp: base.wrapping_sub(0x34),
        ..*fr
    };
    crate::s4_small::parts_buffer(g, img, &pb);
    let start = Frame {
        at: call(0x5f),
        esp: base.wrapping_sub(0x3c),
        edx: w(0x5a),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x52)))],
        ..*fr
    };
    let obj = crate::r9_misc::start_object_reset_bare(g, img, &start);
    let a = g.read::<u32>(w(0x80));
    // call width: 4
    g.call(
        slot_target(g, w(0x65), 0x418 / 4),
        0,
        &[a, w(0x7a), w(0x75), w(0x6a)],
    );
    let a = g.read::<u32>(w(0x8f));
    stores(g, img, f, &[0x93]);
    let r = st0_f32(g.call(slot_target(g, w(0x89), 0x534 / 4), 0, &[a]).st0);
    let yaw = sub(k(g, 0xad), r);
    let yaw = if yaw > k(g, 0xbd) {
        sub(yaw, k(g, 0xc7))
    } else if k(g, 0xd1) > yaw {
        add(yaw, k(g, 0xde))
    } else {
        yaw
    };
    let y = sub(k(g, 0xe6), k(g, 0xee));
    g.write::<f32>(w(0xf6), yaw);
    g.write::<f32>(w(0x103), y);
    g.write::<u32>(obj + 0x64, 0x3f4c_cccd);
    crate::sa_misc::me_block(g, img, call(0x10f), w(0xfb));
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0x11a)) {
        // (`cmp esi, -1`: never, the loop counts from 0)
        let sel = g.read::<u32>(w(0x131));
        let b = u32::from(g.read::<u8>(sel.wrapping_add(i)));
        let c = g.read::<u32>(w(0x13c).wrapping_add(4 * b));
        // call width: 4
        g.call(
            slot_target(g, w(0x151), 0x418 / 4),
            0,
            &[c, p, p + 4, p + 8],
        );
        let dx = sub(g.read::<f32>(p), k(g, 0x175));
        let dz = sub(g.read::<f32>(p + 8), k(g, 0x16d));
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        g.write::<f32>(p + 8, dz);
        g.write::<f32>(p, dx);
        let s = add(mul(dz, dz), mul(dx, dx));
        #[allow(clippy::cast_possible_truncation)]
        let d = mul(f64::from(s).sqrt() as f32, k(g, 0x1b1));
        // cvttss2si: out of range or NaN gives 0x80000000
        #[allow(clippy::cast_possible_truncation)]
        let t = if d.is_nan() || d.abs() >= 2_147_483_648.0 {
            i32::MIN
        } else {
            d as i32
        };
        i = i.wrapping_add(1);
        g.write::<i32>(w(0x1bf).wrapping_add(4 * i), 0x22i32.wrapping_sub(t));
    }
    let count = g.read::<u16>(w(0x1d2));
    stores(
        g,
        img,
        f,
        &[0x1d8, 0x1e2, 0x1ec, 0x1f6, 0x200, 0x20a, 0x214, 0x21e],
    );
    g.write::<u16>(w(0x22a), 0);
    stores(g, img, f, &[0x22e]);
    g.write::<u16>(w(0x23a), 0);
    let (first, free, count) = if count == 0 {
        (0, g.read::<u32>(w(0x24a)), 0)
    } else {
        let node = g.read::<u32>(w(0x254));
        let (prev, next) = (g.read::<u32>(node), g.read::<u32>(node + 4));
        g.write::<u32>(prev.wrapping_add(4), next);
        let (next, prev) = (g.read::<u32>(node + 4), g.read::<u32>(node));
        g.write::<u32>(next, prev);
        let free = if count > 1 {
            g.read::<u32>(node + 4)
        } else {
            0
        };
        let count = count.wrapping_sub(1);
        g.write::<u32>(w(0x279), free);
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node + o, 0);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 0x10, w(0x29e));
        g.write::<u8>(node + 0x18, 0);
        g.write::<u16>(w(0x2a9), count);
        (node, free, count)
    };
    let mut head = g.read::<u32>(w(0x2af));
    g.write::<u32>(first.wrapping_add(0x14), w(0x2b6));
    g.write::<u32>(first.wrapping_add(4), 0);
    if head == 0 {
        head = first;
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0x2db), first);
    } else {
        let tail = g.read::<u32>(w(0x2c6));
        g.write::<u32>(first, tail);
        g.write::<u32>(tail.wrapping_add(4), first);
    }
    g.write::<u32>(first.wrapping_add(8), first);
    stores(g, img, f, &[0x2e2]);
    let second = if count == 0 {
        0
    } else {
        ring_take(g, img, f, free, count, (0x30e, 0x316, 0x347, 0x352));
        free
    };
    g.write::<u32>(second.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x370), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first.wrapping_add(4), second);
    }
    g.write::<u32>(second.wrapping_add(8), second);
    g.write::<u32>(w(0x382), second);
    Ret::eax(0)
}

/// [`start_up_0245`] (913 bytes, 1 overlay).
pub const START_UP_0245: Sig = Sig {
    len: 913,
    hash: 0xb165_0bf0_407a_6810,
    calls: &[
        (0x4a, Callee::Sig(&crate::s4_small::PARTS_BUFFER_50000)),
        (0x5f, Callee::Sig(&crate::r9_misc::START_OBJECT_BARE_0245)),
        (0x10f, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x1a0, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x389, Callee::Any),
    ],
    imports: &[],
};
