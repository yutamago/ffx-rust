//! `magic_0596.dll`'s own functions (besides the runtime and the object
//! host, [`crate::l5_objhost`]): the effect's start with its streamed
//! effect data, the part set-up with the caller's buffer, an object's
//! fields, the per-frame check that ends the effect and the two
//! characters' placement.

use crate::l5_objhost::{self as host, ONE};
use crate::ob_model::{Angle, ModelOps, Who, scale_model, st0_f32, who, wrap_once};
use crate::runtime::slot_target;
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The part's set-up (0x10005060, cdecl `(buf)`): `pppInitData(G_PART,
/// program, 0)` (slot 711, the game flag `*G_LOCK` set to 1 around it),
/// slot 710 `(env, G_PART, buf, 0x20000)` (the earlier call's words left
/// on the stack), the object list emptied. Returns slot 710's `eax`.
pub fn part_init<G: Game>(g: &mut G, img: &Image, f: u32, buf: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let lock = g.read::<u32>(w(0x04));
    let part = g.read::<u32>(w(0x0a));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s711 = slot_target(g, w(0x18), 711);
    let a = [part, w(0x25), 0];
    g.call(s711, 0, &a);
    let lock = g.read::<u32>(w(0x2d));
    g.write::<u32>(lock, saved);
    let part = g.read::<u32>(w(0x3d));
    let s710 = slot_target(g, w(0x42), 710);
    let mut args = vec![w(0x47), part, buf, 0x2_0000];
    args.extend(a);
    // call width: 7 at most (the earlier calls' words stay on the stack)
    let r = g.call(s710, 0, &args).eax;
    for off in [0x58, 0x62, 0x6c, 0x76] {
        g.write::<u32>(w(off), 0);
    }
    r
}

/// [`part_init`] (129 bytes, 1 overlay).
pub const PART_INIT: Sig = Sig {
    len: 129,
    hash: 0x3488_7923_81a4_f379,
    calls: &[],
    imports: &[],
};

/// An object's fields (0x100050f0, `ecx` = the object): the countdown
/// `+0x114` = 0x15, the frame counters and callbacks 0, mode 0, flags
/// 0x8000, angles and translation 0, the scale words `+0xdc`/`+0xec` and
/// the colour `+0xf0..+0xfc` 1.0. Returns 0x8000.
pub fn object_init<G: Game>(g: &mut G, obj: u32) -> u32 {
    for (off, v) in [
        (0x114, 0x15),
        (0x11c, 0),
        (0x100, 0),
        (0x104, 0),
        (0x108, 0),
        (0x10c, 0),
        (0x110, 0),
        (0x118, 0),
        (0x120, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    g.write::<u8>(obj + 0x126, 0);
    g.write::<u16>(obj + 0x124, 0x8000);
    for (off, v) in [
        (0xcc, 0),
        (0xc8, 0),
        (0xc4, 0),
        (0xc0, 0),
        (0xd8, 0),
        (0xd4, 0),
        (0xd0, 0),
        (0xdc, ONE),
        (0xe8, 0),
        (0xe4, 0),
        (0xe0, 0),
        (0xec, ONE),
        (0xfc, ONE),
        (0xf8, ONE),
        (0xf4, ONE),
        (0xf0, ONE),
    ] {
        g.write::<u32>(obj + off, v);
    }
    0x8000
}

/// [`object_init`] (270 bytes, 1 overlay).
pub const OBJECT_INIT: Sig = Sig {
    len: 270,
    hash: 0x21d5_038a_b0f9_c76b,
    calls: &[],
    imports: &[],
};

/// The object host's start as the summon overlays' starts begin it
/// (`f` the start, `arg` its argument): keeps `arg`, sets the object
/// block up ([`host::obj_block_init`]), three registrations (slots 556,
/// 555, 331 of the DLL's blocks), 1.0 into three data words, slot 790's
/// float into `G_SCALE`, the task ring ([`host::tasks_init`]), every
/// pool-A node free, then slot 720 `(0x3400, 0x800)`, 718, 222 and 221
/// `(E, 0, 1000, 1)` for the first effect data. Returns the words those
/// calls leave on the stack (the start's later calls see them).
pub fn host_start<G: Game>(g: &mut G, img: &Image, f: u32, arg: u32) -> Vec<u32> {
    let w = |off: u32| img.word(f + off);
    host_start_head(g, img, f, arg);
    let s720 = slot_target(g, w(0xb0), 720);
    let mut stack = vec![0x3400, 0x800];
    g.call(s720, 0, &stack);
    let s718 = slot_target(g, w(0xc7), 718);
    stack.insert(0, w(0xcc));
    // call width: 11 at most (the earlier calls' words stay on the stack)
    g.call(s718, 0, &stack);
    let s222 = slot_target(g, w(0xd9), 222);
    stack.splice(0..0, [w(0xed), 0, 1000, 1]);
    // call width: 11 at most (the earlier calls' words stay on the stack)
    g.call(s222, 0, &stack);
    let s221 = slot_target(g, w(0xf4), 221);
    stack.splice(0..0, [w(0x108), 0, 1000, 1]);
    // call width: 11 at most (the earlier calls' words stay on the stack)
    g.call(s221, 0, &stack);
    stack
}

/// [`host_start`] up to its first slot 720 call (the code through
/// `+0xaf`, which other starts share): `arg` kept, the object block, the
/// three registrations, the 1.0 words, slot 790's float, the task ring
/// and the free pool-A nodes. Leaves nothing on the stack.
pub fn host_start_head<G: Game>(g: &mut G, img: &Image, f: u32, arg: u32) {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(w(0x08), arg);
    host::obj_block_init(g, img, img.rel32_target(f + 0x0d), arg);
    let s556 = slot_target(g, w(0x12), 556);
    g.call(s556, 0, &[w(0x17)]);
    let s555 = slot_target(g, w(0x24), 555);
    g.call(s555, 0, &[w(0x29), w(0x17)]);
    let s331 = slot_target(g, w(0x36), 331);
    g.call(s331, 0, &[w(0x3b), w(0x29), w(0x17)]);
    let s790 = slot_target(g, w(0x48), 790);
    g.write::<u32>(w(0x57), ONE);
    g.write::<u32>(w(0x61), ONE);
    g.write::<u32>(w(0x6b), ONE);
    // fstp dword: ST0 rounded to f32 (an empty x87 stack stores the
    // indefinite NaN)
    let scale = g
        .call(s790, 0, &[])
        .st0
        .map_or(f32::from_bits(0xffc0_0000), |x| x as f32);
    g.write::<f32>(w(0x77), scale);
    host::tasks_init(g, img, img.rel32_target(f + 0x7c));
    // +0x80: mov eax, G_NODES + 8; 0x3ff payloads of 12-byte nodes
    let payloads = w(0x81);
    for i in 0..0x3ff {
        g.write::<u32>(payloads + 12 * i, 0);
    }
    g.write::<u16>(w(0xa3), 0x3ff);
    g.write::<u16>(w(0xab), 0);
}

/// Table `+0x0` (0x10006560, cdecl `(arg)`): the object host's start
/// ([`host_start`]: the object block, the three registrations, slot
/// 790's float, the task ring, the free nodes, slots 720, 718, 222, 221
/// for the first effect data), then the second effect data `E2`: slot
/// 718 `(E2)`, slot 221 `(E2, 0, 1000, 1)`, slot 718 `(E2)` again and its
/// stream cued (slot 551 `(slot 719 (E2, 6, 0), 0, 1)`); `G_DONE` = 0.
/// Each call sees the words the earlier ones left on the stack. Returns
/// 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let mut stack = host_start(g, img, f, arg);
    let s718 = slot_target(g, w(0x10f), 718);
    stack.insert(0, w(0x114));
    // call width: 16 at most (the earlier calls' words stay on the stack)
    g.call(s718, 0, &stack);
    let s221 = slot_target(g, w(0x121), 221);
    stack.splice(0..0, [w(0x135), 0, 1000, 1]);
    // call width: 16 (the earlier calls' words stay on the stack)
    g.call(s221, 0, &stack);
    let s718 = slot_target(g, w(0x13c), 718);
    let mut stack = vec![w(0x14a)];
    g.call(s718, 0, &stack);
    let s719 = slot_target(g, w(0x151), 719);
    stack.splice(0..0, [w(0x160), 6, 0]);
    let entry = g.call(s719, 0, &stack).eax;
    let s551 = slot_target(g, w(0x16c), 551);
    stack.splice(0..0, [entry, 0, 1]);
    g.call(s551, 0, &stack);
    g.write::<u32>(w(0x17d), 0);
    Ret::eax(0)
}

/// [`start`] (393 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 393,
    hash: 0xbfc5_6947_f82d_f03d,
    calls: &[
        (0x0d, Callee::Sig(&host::OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&host::TASKS_INIT)),
    ],
    imports: &[],
};

/// A callback (0x10007340, no arguments): the `+0x454` of `*G_STATE` (the
/// caster's height) into `+0x424` and `+0x3c4`. Returns the state.
pub fn copy_height<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let state = g.read::<u32>(img.word(fr.at + 0x01));
    let y = g.read::<u32>(state + 0x454);
    g.write::<u32>(state + 0x424, y);
    g.write::<u32>(state + 0x3c4, y);
    Ret::eax(state)
}

/// [`copy_height`] (30 bytes, 1 overlay).
pub const COPY_HEIGHT: Sig = Sig {
    len: 30,
    hash: 0x1540_4713_2758_4110,
    calls: &[],
    imports: &[],
};

/// The battle index of the null-matrix path: the byte at the selection
/// (`movzx esi, byte [ecx]`) or the word at a global (operand offset).
#[derive(Clone, Copy)]
enum Index {
    Byte(u32),
    Word(u32),
}

/// One model's placement in [`place_pair`]: the operand offsets of its
/// slot calls (each call reads the table global again) and globals.
struct PlaceOps {
    s210: u32,
    off: u32,
    s217: u32,
    s255: u32,
    s211: u32,
    model: ModelOps,
    index: Index,
}

/// Places one model (`chr`, the matrix at `m`; `pos` the local the
/// battle character's position goes through): the root matrix read
/// (slot 210) or, for a null matrix, the battle character `index`
/// (below 32, when `*G_OFF` is 0) moved to its own position (slots 217
/// and 255); the original matrix set (slot 211); the model matrix
/// ([`scale_model`]) turned by Z 0, X 0 and Y `ang`. Whether the battle
/// character was moved.
fn place_model<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    o: &PlaceOps,
    (chr, m, pos): (u32, u32, u32),
    ang: f32,
) -> bool {
    let w = |off: u32| img.word(f + off);
    let mut moved = false;
    if m == 0 {
        if g.read::<u32>(w(o.off)) == 0 {
            let i = match o.index {
                Index::Byte(p) => u32::from(g.read::<u8>(p)),
                Index::Word(op) => g.read::<u32>(w(op)),
            };
            if i.cast_signed() < 0x20 {
                let s217 = slot_target(g, w(o.s217), 217);
                g.call(s217, 0, &[i, pos]);
                let s255 = slot_target(g, w(o.s255), 255);
                g.call(s255, 0, &[i, pos, i, pos]);
                moved = true;
            }
        }
    } else {
        let s210 = slot_target(g, w(o.s210), 210);
        g.call(s210, 0, &[chr, m]);
    }
    let s211 = slot_target(g, w(o.s211), 211);
    g.call(s211, 0, &[chr, m]);
    let turns = [Angle::Bits(0), Angle::Bits(0), Angle::Bits(ang.to_bits())];
    let _ = scale_model(g, img, f, &o.model, m, &[chr, m], turns);
    moved
}

/// Places the caster and the summon (0x10006a20, no arguments; `esp`
/// the entry stack pointer): the caster (the battle character the byte
/// `*G_SEL` selects) at its position (slot 262 into the locals `ebp -
/// 0x14..-0xc`, kept in `*G_STATE +0x450`) faces `ang = A - Ch_GetRot`
/// (slot 333, wrapped once into -pi..pi); the state's two frames get the
/// position, the angle and (`+0x444`) the height + 47; the caster's
/// model and the summon's (`*G_SUMMON`, battle index `*G_INDEX`) are
/// placed ([`place_model`]) into `+0x390` and `+0x3d0`; the summon is
/// shown (slot 238 `(index, 1)`) and the caster hidden (slot 296 `(index,
/// 0)` with `*G_OFF` 0 and the index below 32, else slot 257 `(summon,
/// 1)`); the camera point `+0x460` is the position moved by `(0, 0, -5)`
/// turned by X 0 and Y `ang` (slots 235, 328, 329, 229 into the locals at
/// `ebp - 0x74`, `- 0x34`, `- 0x24`; slot 248 adds it), `w` = 1;
/// `+0x6e0` = 0 and two flags set. Returns slot 248's `eax`. (The
/// frame's stack cookie is the CRT's; nothing else reads it.) `magic_0304`
/// 0x100015d0 is the same code with every field of the state 0x1048
/// higher ([`PLACE_PAIR_0304`](crate::r21::PLACE_PAIR_0304)), read from the
/// first store's displacement.
#[allow(clippy::too_many_lines)] // one straight sequence of calls
pub fn place_pair<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let ebp = esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let sel = Who::Sel {
        sel: 0x11,
        at: 0,
        table: 0x30,
    };
    let chr = who(g, img, f, sel);
    // `magic_0304`'s copy (0x100015d0) has every state field 0x1048
    // higher: `S` moved by the first store's `[edi + 0x450]` less 0x450
    let state = g
        .read::<u32>(w(0x1d))
        .wrapping_add(w(0x4a).wrapping_sub(0x450));
    let s262 = slot_target(g, w(0x35), 262);
    let mut stack = vec![chr, l(0x14), l(0x10), l(0x0c)];
    g.call(s262, 0, &stack);
    for (off, src) in [(0x450, l(0x14)), (0x454, l(0x10)), (0x458, l(0x0c))] {
        let v = g.read::<u32>(src);
        g.write::<u32>(state + off, v);
    }
    g.write::<u32>(state + 0x45c, ONE);
    let chr2 = who(
        g,
        img,
        f,
        Who::Sel {
            sel: 0x73,
            at: 0,
            table: 0x84,
        },
    );
    let s333 = slot_target(g, w(0x89), 333);
    stack.insert(0, chr2);
    // call width: 5 at most (the earlier calls' words stay on the stack)
    let rot = st0_f32(g.call(s333, 0, &stack).st0);
    let ang = g.read::<f32>(w(0x9c)) - rot;
    let ang = wrap_once(g, img, f, ang, [0xab, 0xba, 0xc4, 0xd1]);
    // [ebp-0x18] is never written here: the stack's stale word, read as
    // it is (the only caller, `start2`, leaves its part set-up's buffer
    // there, and its port writes that word too)
    let leftover = g.read::<u32>(l(0x18));
    g.write::<u32>(state + 0x41c, leftover);
    g.write::<u32>(state + 0x410, 0);
    g.write::<f32>(state + 0x414, ang);
    g.write::<u32>(state + 0x418, 0);
    for (off, src) in [(0x420, l(0x14)), (0x424, l(0x10)), (0x428, l(0x0c))] {
        let v = g.read::<u32>(src);
        g.write::<u32>(state + off, v);
    }
    g.write::<u32>(state + 0x42c, ONE);
    let selp = g.read::<u32>(w(0x139));
    let k = u32::from(g.read::<u8>(selp));
    let chr = g.read::<u32>(w(0x149).wrapping_add(4 * k));
    let first = PlaceOps {
        s210: 0x157,
        off: 0x16a,
        s217: 0x17e,
        s255: 0x190,
        s211: 0x1a6,
        model: ModelOps {
            s235: 0x1b3,
            c277: Who::Sel {
                sel: 0x1c1,
                at: 0,
                table: 0x1cb,
            },
            s277: 0x1d0,
            k: 0x1e8,
            c: Who::Sel {
                sel: 0x1fb,
                at: 0,
                table: 0x205,
            },
            s15: 0x20a,
            s16: 0x221,
            s17: 0x23d,
            s330: 0x25a,
            s328: 0x278,
            s329: 0x296,
        },
        index: Index::Byte(selp),
    };
    let pos = l(0x24);
    place_model(
        g,
        img,
        f,
        &first,
        (chr, state.wrapping_add(0x390), pos),
        ang,
    );
    // the null-matrix path handed `ebp - 0x24` to its slots; its fourth
    // word is `ebp - 0x18` (else the stale word above)
    let l18 = g.read::<u32>(l(0x18));
    for k in 0..4 {
        let v = g.read::<u32>(state + 0x420 + 4 * k);
        g.write::<u32>(state + 0x3c0 + 4 * k, v);
    }
    g.write::<u32>(state + 0x430, 0);
    g.write::<f32>(state + 0x434, ang);
    g.write::<u32>(state + 0x43c, l18);
    g.write::<u32>(state + 0x438, 0);
    let x = g.read::<u32>(l(0x14));
    g.write::<u32>(state + 0x440, x);
    let z = g.read::<u32>(l(0x0c));
    g.write::<u32>(state + 0x448, z);
    g.write::<u32>(state + 0x44c, ONE);
    let y = g.read::<f32>(l(0x10)) + g.read::<f32>(w(0x33d));
    g.write::<f32>(state + 0x444, y);
    let summon = g.read::<u32>(w(0x350));
    let second = PlaceOps {
        s210: 0x35e,
        off: 0x371,
        s217: 0x388,
        s255: 0x39a,
        s211: 0x3b0,
        model: ModelOps {
            s235: 0x3bd,
            c277: Who::Word(0x3d1),
            s277: 0x3cb,
            k: 0x3e9,
            c: Who::Word(0x402),
            s15: 0x3ee,
            s16: 0x419,
            s17: 0x428,
            s330: 0x452,
            s328: 0x470,
            s329: 0x48e,
        },
        index: Index::Word(0x37a),
    };
    let m = state.wrapping_add(0x3d0);
    place_model(g, img, f, &second, (summon, m, pos), ang);
    for k in 0..4 {
        let v = g.read::<u32>(state + 0x440 + 4 * k);
        g.write::<u32>(state + 0x400 + 4 * k, v);
    }
    let s238 = slot_target(g, w(0x4d5), 238);
    let i = g.read::<u32>(w(0x4dd));
    // the second turn's three words stay on the stack
    g.call(s238, 0, &[i, 1, m, m, ang.to_bits()]);
    let off = g.read::<u32>(w(0x4ee)) != 0;
    let i = if off { 0 } else { g.read::<u32>(w(0x4f6)) };
    if !off && i.cast_signed() < 0x20 {
        let s296 = slot_target(g, w(0x503), 296);
        g.call(s296, 0, &[i, 0]);
    } else {
        let s257 = slot_target(g, w(0x510), 257);
        let summon = g.read::<u32>(w(0x518));
        g.call(s257, 0, &[summon, 1]);
    }
    g.write::<f32>(state + 0x6d4, ang);
    let cam = state + 0x460;
    for k in 0..4 {
        let v = g.read::<u32>(state + 0x420 + 4 * k);
        g.write::<u32>(cam + 4 * k, v);
    }
    let (mat, v, out) = (l(0x74), l(0x34), l(0x24));
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    g.write::<u32>(v + 8, 0xc0a0_0000);
    let s235 = slot_target(g, w(0x56f), 235);
    g.call(s235, 0, &[mat]);
    let s328 = slot_target(g, w(0x59d), 328);
    g.call(s328, 0, &[mat, mat, 0]);
    let s329 = slot_target(g, w(0x5bc), 329);
    g.call(s329, 0, &[mat, mat, ang.to_bits()]);
    g.write::<u32>(v + 0xc, ONE);
    let s229 = slot_target(g, w(0x5d5), 229);
    let a = [out, mat, v, mat, mat, ang.to_bits()];
    g.call(s229, 0, &a);
    let s248 = slot_target(g, w(0x5ed), 248);
    let mut args = vec![cam, cam, out];
    args.extend(a);
    // call width: 9 (the earlier calls' words stay on the stack)
    let r = g.call(s248, 0, &args).eax;
    g.write::<u32>(cam + 0xc, ONE);
    g.write::<u32>(state + 0x6e0, 0);
    g.write::<u32>(w(0x618), 1);
    g.write::<u32>(w(0x622), 1);
    r
}

/// [`place_pair`] (1,588 bytes, 1 overlay).
pub const PLACE_PAIR: Sig = Sig {
    len: 1588,
    hash: 0xa9be_d27c_c6bc_370d,
    // __security_check_cookie
    calls: &[(0x62c, Callee::Any)],
    imports: &[],
};

/// An object placed with a particle part (0x10005200, internal: `edx` =
/// the object, `xmm3` = its scale): translation `+0x40..+0x48` 0 and
/// `+0x4c` 1.0, `+0x50..+0x5c` 0, the scale in `+0x60..+0x6c`; `*G_P =
/// ITEM` (operands `+0x01`, `+0x56`), part `list[2]` (slot 941 of `*G_D +
/// 0x10`, `G_D` at `+0x5b`) started on it (`pppStartPart(obj, 0x1000,
/// program, 1)`, the wrapper's `rel32` at `+0x7e`), its fields set
/// ([`object_init`], `+0x88`); then a node of the object pool
/// ([`host::pool_alloc`], `+0x90`) appended to the object list (`G_FIRST`
/// `+0xae`, `G_LAST` `+0x9b`/`+0xb4`/`+0xc8`/`+0xd1`), its `+0x8` the
/// object, the count (`+0xbd`/`+0xda`) up by one. Without a free node the
/// object stays off the list. Returns the object.
pub fn place_object<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, scale: f32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x01));
    for off in [0x48, 0x44, 0x40] {
        g.write::<u32>(obj + off, 0);
    }
    g.write::<u32>(obj + 0x4c, ONE);
    for off in [0x5c, 0x58, 0x54, 0x50] {
        g.write::<u32>(obj + off, 0);
    }
    for off in [0x6c, 0x68, 0x64, 0x60] {
        g.write::<u32>(obj + off, scale.to_bits());
    }
    g.write::<u32>(p, w(0x56));
    let d = g.read::<u32>(g.read::<u32>(w(0x5b)).wrapping_add(0x10));
    let s941 = slot_target(g, w(0x63), 941);
    let programs = g.call(s941, 0, &[d]).eax;
    let program = g.read::<u32>(programs.wrapping_add(8));
    crate::opu_parts::start_part(g, img, img.rel32_target(f + 0x7e), obj, 0x1000, program, 1);
    object_init(g, obj);
    let node = host::pool_alloc(g, img, img.rel32_target(f + 0x90));
    if node == 0 {
        return obj;
    }
    let last = g.read::<u32>(w(0x9b));
    g.write::<u32>(node + 4, 0);
    if last == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0xae), node);
        g.write::<u32>(w(0xb4), node);
    } else {
        g.write::<u32>(node, last);
        let last = g.read::<u32>(w(0xc8));
        g.write::<u32>(last + 4, node);
        g.write::<u32>(w(0xd1), node);
    }
    g.write::<u32>(node + 8, obj);
    let count = if last == 0 { w(0xbd) } else { w(0xda) };
    let n = g.read::<u32>(count);
    g.write::<u32>(count, n.wrapping_add(1));
    obj
}

/// [`place_object`] (226 bytes, 1 overlay).
pub const PLACE_OBJECT: Sig = Sig {
    len: 226,
    hash: 0xf5ee_280a_b104_4879,
    calls: &[
        (0x7e, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x88, Callee::Sig(&OBJECT_INIT)),
        (0x90, Callee::Sig(&host::POOL_ALLOC)),
    ],
    imports: &[],
};

/// A task node (the pool take at the `rel32` `call`, owner `CB` at
/// `cb`) of kind 1 or 2 appended to the task list (operands `ops`, as
/// [`crate::r12h_start::append`] takes them). Returns the node.
fn task_node<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (cb, call, kind): (u32, u32, u8),
    ops: [u32; 5],
) -> u32 {
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + call), img.word(f + cb));
    g.write::<u8>(node.wrapping_add(0x18), kind);
    crate::r12h_start::append(g, img, f, node, ops);
    node
}

/// The 32-bit stores `mov dword [G], imm32` (10 bytes each) at the
/// offsets `at`: `G` (`+2`) = the immediate (`+6`).
fn stores<G: Game>(g: &mut G, img: &Image, f: u32, at: &[u32]) {
    for &o in at {
        g.write::<u32>(img.word(f + o + 2), img.word(f + o + 6));
    }
}

/// The effect's second start (0x10007360, no arguments): four globals
/// from immediates; slot 719 `(P, 8, 0)` into `G_H`; the part set up
/// ([`part_init`] `(BUF)`); the pair placed ([`place_pair`]); the
/// effect's turned matrix ([`crate::o4_effect::start_turned`]); the two
/// script blocks' callbacks, words, halfwords and bytes; slot 271 `(4)`;
/// two task nodes (kinds 1 and 2) appended to the task list, the records
/// set up ([`crate::sa_misc::records_init_400`]) between them. Returns 0.
pub fn start2<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start2_with(
        g,
        img,
        fr,
        |g, img, at, esp| {
            let buf = g.read::<u32>(esp + 4);
            part_init(g, img, at, buf);
        },
        |g, img, at, esp| {
            place_pair(g, img, at, esp);
        },
    )
}

/// [`start2`] with the part set-up (`+0x4c`) and the placement (`+0x51`)
/// given (each gets its address and the `esp` at its call, the return
/// address there): `magic_0598`'s copy calls
/// [`part_setup`](crate::sb_misc::part_setup) and its own placement
/// ([`place_pair_0598`](crate::r23::place_pair_0598)).
pub(crate) fn start2_with<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    part: impl FnOnce(&mut G, &Image, u32, u32),
    place: impl FnOnce(&mut G, &Image, u32, u32),
) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    stores(g, img, f, &[0x14, 0x1e, 0x28, 0x32]);
    let t = slot_target(g, w(0x01), 719);
    // call width: 3
    let h = g.call(t, 0, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x42), h);
    // slot 719's 0 left; push BUF; the return address
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 8, 0);
    g.write::<u32>(esp + 4, w(0x47));
    part(g, img, img.rel32_target(f + 0x4c), esp);
    // the part set-up's frame leaves `BUF` in the word the placement
    // reads without writing (its `ebp - 0x18`)
    g.write::<u32>(esp.wrapping_sub(0x1c), w(0x47));
    place(g, img, img.rel32_target(f + 0x51), esp);
    let callee = Frame {
        at: img.rel32_target(f + 0x56),
        esp,
        ..Frame::default()
    };
    crate::o4_effect::start_turned(g, img, &callee);
    g.write::<u32>(w(0x61), w(0x65));
    g.write::<u32>(w(0x6b), w(0x6f));
    g.write::<u16>(w(0x76), w(0x7a) as u16);
    g.write::<u8>(w(0x7e), w(0x82) as u8);
    g.write::<u32>(w(0x85), w(0x89));
    g.write::<u32>(w(0x8f), w(0x93));
    g.write::<u16>(w(0x9a), w(0x9e) as u16);
    g.write::<u8>(w(0xa2), w(0xa6) as u8);
    let t = slot_target(g, w(0x5b), 271);
    // call width: 3 (the part set-up's words under it)
    g.call(t, 0, &[4, w(0x47), 0]);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xba), w(0xb5));
    g.write::<u8>(node.wrapping_add(0x18), 1);
    crate::r12h_start::append(g, img, f, node, [0xc0, 0xd4, 0xf3, 0xe6, 0xf3]);
    crate::sa_misc::records_init_400(g, img, img.rel32_target(f + 0xf8), w(0xeb));
    task_node(
        g,
        img,
        f,
        (0xfd, 0x102, 2),
        [0x108, 0x11c, 0x129, 0x137, 0x13f],
    );
    Ret::eax(0)
}

/// [`start2`] (326 bytes, 1 overlay).
pub const START2: Sig = Sig {
    len: 326,
    hash: 0xb0df_13a1_ef72_7e74,
    calls: &[
        (0x4c, Callee::Sig(&PART_INIT)),
        (0x51, Callee::Sig(&PLACE_PAIR)),
        (0x56, Callee::Sig(&crate::o4_effect::START_TURNED_0596_7060)),
        (0xba, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xf8, Callee::Sig(&crate::sa_misc::RECORDS_INIT_400)),
        (0x102, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The summon's objects started (0x10007150, no arguments): with the
/// record `R = *G_R`, the object `O = R + 0x260` placed
/// ([`place_object`], scale `K`; four never-written stack words under
/// it, which it does not read), its `+0xd0..+0xdc` from `R + 0x450..`,
/// `+0xc4` from `R + 0x6d4`, `+0xd4 -= D`; `R + 0x444` = `R + 0x454`,
/// `R + 0x424` and `+0x3c4` = it less `E`; the timer block `T = R +
/// 0x48c` (0, 40, 0, 50, 1, 1, a halfword 0) on a task node (kind 1,
/// callback `CB`, `+0x14` = `T`) appended to the task list; the record
/// node `R + 0x4d8` ([`crate::o4_small::record_node`]) with flags 3 and
/// its scale and count; the object `R + 0x578`'s colour and fields and
/// its pool node ([`crate::s8_misc::object_node`]); the fade task on
/// `R + 0x540` ([`crate::s3_misc::fade_task`], `edx` 16, two scales,
/// 180). Returns the fade task's node.
pub fn objects_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let scale = k(g, 0x04);
    let r = g.read::<u32>(w(0x0c));
    let obj = r.wrapping_add(0x260);
    place_object(g, img, img.rel32_target(f + 0x1c), obj, scale);
    for o in [0u32, 4, 8, 0xc] {
        let v = g.read::<u32>(r.wrapping_add(0x450 + o));
        g.write::<u32>(obj.wrapping_add(0xd0 + o), v);
    }
    let x = sub(g.read::<f32>(obj + 0xd4), k(g, 0x62));
    let v = g.read::<u32>(r.wrapping_add(0x6d4));
    g.write::<u32>(obj + 0xc4, v);
    g.write::<f32>(obj + 0xd4, x);
    let y = g.read::<f32>(r.wrapping_add(0x454));
    let low = sub(y, k(g, 0x8d));
    g.write::<f32>(r.wrapping_add(0x444), y);
    g.write::<f32>(r.wrapping_add(0x424), low);
    g.write::<f32>(r.wrapping_add(0x3c4), low);
    let timer = r.wrapping_add(0x48c);
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x28),
        (8, 0),
        (0xc, 0x32),
        (0x14, 1),
        (0x10, 1),
    ] {
        g.write::<u32>(timer + o, v);
    }
    g.write::<u16>(timer + 0x18, 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xdd), w(0x72));
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(0x14), timer);
    crate::r12h_start::append(g, img, f, node, [0xe3, 0xfa, 0x11c, 0x10c, 0x11c]);
    let rn = r.wrapping_add(0x4d8);
    // push esi; push edi; the return address
    let esp = fr.esp.wrapping_sub(0xc);
    let callee = Frame {
        at: img.rel32_target(f + 0x121),
        esp,
        ecx: rn,
        ..Frame::default()
    };
    crate::o4_small::record_node(g, img, &callee);
    let v = g.read::<u16>(rn + 0x2c);
    g.write::<u16>(rn + 0x2c, v | 3);
    let o2 = r.wrapping_add(0x578);
    for (o, v) in [
        (4u32, 0x3f80_0000u32),
        (8, 0x3f80_0000),
        (0xc, 0),
        (0x10, 0x4080_0000),
        (0x14, 0x4080_0000),
        (0x18, 0),
        (0x24, 0x16),
    ] {
        g.write::<u32>(rn + o, v);
    }
    g.write::<u32>(o2 + 0xf0, 0x4080_8080);
    g.write::<u16>(o2 + 0x124, 2);
    for (o, v) in [
        (0x11cu32, u32::MAX),
        (0x120, 0x5c),
        (0x110, 0x76),
        (0xf8, 0),
        (0xfc, 0x3d4c_cccd),
        (0x100, 0),
    ] {
        g.write::<u32>(o2 + o, v);
    }
    let callee = Frame {
        at: img.rel32_target(f + 0x1b4),
        esp,
        ecx: o2,
        ..Frame::default()
    };
    crate::s8_misc::object_node(g, img, &callee);
    // push 0xb4 under the return address
    let esp = fr.esp.wrapping_sub(0x10);
    g.write::<u32>(esp + 4, 0xb4);
    let callee = Frame {
        at: img.rel32_target(f + 0x1d9),
        esp,
        ecx: r.wrapping_add(0x540),
        edx: 0x10,
        xmm: [
            0,
            u128::from(k(g, 0x1c4).to_bits()),
            u128::from(k(g, 0x1bc).to_bits()),
            0,
        ],
        ..Frame::default()
    };
    Ret::eax(crate::s3_misc::fade_task(g, img, &callee))
}

/// [`objects_start`] (483 bytes, 1 overlay).
pub const OBJECTS_START: Sig = Sig {
    len: 483,
    hash: 0xab2a_3ffb_7ea7_33ad,
    calls: &[
        (0x1c, Callee::Sig(&PLACE_OBJECT)),
        (0xdd, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x121, Callee::Sig(&crate::o4_small::RECORD_NODE_0596_3900)),
        (0x1b4, Callee::Sig(&crate::s8_misc::OBJECT_NODE)),
        (0x1d9, Callee::Sig(&crate::s3_misc::FADE_TASK)),
    ],
    imports: &[],
};
