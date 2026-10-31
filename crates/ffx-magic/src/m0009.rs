//! `magic_0009.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
use crate::{Callee, Game, Image, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Sets up the four 0x250-byte records of the effect block `G_FX`
/// (operand `+0x11`) and its model (0x100019d0, internal: `ecx` = a
/// position `pos` (4 words), `edx` = another position `from`, `arg` the
/// stack argument, a rotation (3 floats); `esp` at the return address).
/// Two slot 719 lookups `(NAME, 9, 0)` and `(NAME, 9, 1)` (the first
/// call's words left on the stack) give the records' two handles; each
/// record gets them, `pos`, flags and colours, 32 copies of `pos`, and a
/// word of the table at operand `+0x64`. Then slot 235 `(G_FX)` and the
/// rotations 330, 328, 329 `(G_FX, G_FX, arg[2] / arg[0] / arg[1])`,
/// `pos` at `G_FX + 0x60`, the distance `|pos - from|` (`sqrt`) at
/// `+0x58` and the rest of the model's words.
pub fn setup<G: Game>(g: &mut G, img: &Image, f: u32, pos: u32, from: u32, arg: u32) {
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x11));
    let name = w(0x29);
    let s719 = slot_target(g, w(0x1a), 719);
    let a = g.call(s719, pos, &[name, 9, 0]).eax;
    let s719 = slot_target(g, w(0x3b), 719);
    let name2 = w(0x4a);
    // call width: 6 (three words and the first call's three)
    let b = g.call(s719, pos, &[name2, 9, 1, name, 9, 0]).eax;
    let p = [0u32, 4, 8, 12].map(|o| g.read::<u32>(pos + o));
    let table = w(0x64);
    let end = w(0x171);
    let mut rec = fx.wrapping_add(0x2a2);
    let mut at = table;
    loop {
        g.write::<u32>(rec - 0x232, a);
        g.write::<u16>(rec - 0x22a, 0xffff);
        g.write::<u32>(rec - 0x22e, 1);
        g.write::<u16>(rec - 0x228, 0x200);
        for (i, v) in (0u32..).zip(p) {
            g.write::<u32>(rec - 0x226 + 4 * i, v);
        }
        g.write::<u32>(rec - 0x216, 0x8080_8080);
        g.write::<u32>(rec - 0x212, 0x3e4c_cccd);
        g.write::<u32>(rec - 0x20e, b);
        g.write::<u16>(rec - 0x206, 0xffff);
        g.write::<u32>(rec - 0x20a, 1);
        g.write::<u16>(rec - 0x204, 0x200);
        g.write::<u32>(rec - 0x2, 0x8080_8080);
        g.write::<u32>(rec + 0x2, 0);
        g.write::<u32>(rec + 0x12, 0);
        g.write::<u32>(rec + 0x6, ONE);
        g.write::<u32>(rec + 0xa, ONE);
        for k in 0..32u32 {
            for (i, v) in (0u32..).zip(p) {
                g.write::<u32>(rec - 0x202 + 0x10 * k + 4 * i, v);
            }
        }
        g.write::<u32>(rec - 0x2, 0x3050_7896);
        g.write::<u32>(rec + 0x2, 0x0864_3c78);
        g.write::<u32>(rec + 0x16, 0x46);
        g.write::<u32>(rec + 0xe, 0x3f0c_cccd);
        let v = g.read::<u32>(at);
        g.write::<u32>(rec + 0x1a, v);
        at = at.wrapping_add(4);
        rec = rec.wrapping_add(0x250);
        // cmp ebx, END; jl
        if at.cast_signed() >= end.cast_signed() {
            break;
        }
    }
    let s235 = slot_target(g, w(0x17c), 235);
    g.call(s235, rec, &[fx]);
    for (t, slot, o) in [(0x192, 330u32, 8u32), (0x1af, 328, 0), (0x1cb, 329, 4)] {
        let s = slot_target(g, w(t), slot);
        let r = g.read::<u32>(arg + o);
        g.call(s, rec, &[fx, fx, r]);
    }
    for (i, v) in (0u32..).zip(p) {
        g.write::<u32>(fx + 0x60 + 4 * i, v);
    }
    for (o, v) in [
        (0x40u32, 0u32),
        (0x44, 0),
        (0x48, 0),
        (0x4c, ONE),
        (0x50, 0),
        (0x54, 0),
    ] {
        g.write::<u32>(fx + o, v);
    }
    let d = |o: u32| sub(g.read::<f32>(pos + o), g.read::<f32>(from + o));
    let (dx, dy, dz) = (d(0), d(4), d(8));
    let s = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
    g.write::<f32>(fx + 0x58, f64::from(s).sqrt() as f32);
    g.write::<u32>(fx + 0x5c, ONE);
    g.write::<u32>(fx + 0x9b0, 0);
    g.write::<u32>(fx + 0x9b4, 0);
    g.write::<u32>(fx + 0x9b8, 0x14);
}

/// [`setup`] (662 bytes, 1 overlay).
pub const SETUP: Sig = Sig {
    len: 662,
    hash: 0x956c_ecbc_7857_627b,
    calls: &[(0x25d, Callee::Import("_libm_sse2_sqrt_precise"))],
    imports: &[],
};

/// Fills 0x46 entries of a model (0x10002aa0, internal: `ecx` = the
/// model): for each index, slot 772 `(model, i)` gives a 64-bit value
/// (`eax:edx`) that goes to slot 779 `(lo, hi, model, i)` (the first
/// call's words left on the stack), the game flag `*G_LOCK` (operand
/// `+0x21`) set to 1 around the call and restored after.
pub fn fill<G: Game>(g: &mut G, img: &Image, f: u32, model: u32) {
    let w = |off: u32| img.word(f + off);
    for i in 0..0x46u32 {
        let s772 = slot_target(g, w(0x11), 772);
        let r = g.call(s772, model, &[model, i]);
        let lock = g.read::<u32>(w(0x21));
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        let s779 = slot_target(g, w(0x2a), 779);
        g.call(s779, lock, &[r.eax, r.edx, model, i]);
        let lock = g.read::<u32>(w(0x3d));
        g.write::<u32>(lock, saved);
    }
}

/// [`fill`] (80 bytes, 1 overlay).
pub const FILL: Sig = Sig {
    len: 80,
    hash: 0xefad_cf98_910f_56c6,
    calls: &[],
    imports: &[],
};

/// Sets up a part object (0x100032b0, internal: `edx` = the object, the
/// float `xmm3` its four scales, stack words `index` and `owner`):
/// `+0x40..+0x48` and `+0x50..+0x5c` 0, `+0x4c` 1.0, `+0x60..+0x6c` the
/// scale; the word the pointer global (operand `+0x04`) points at gets
/// the immediate at `+0x59`; slot 941 `(*O + 0x10)` (`O` at `+0x5e`)
/// gives the program table and the `pppStartPart` wrapper starts the
/// object (`ecx`) with `(0x1000, table[index], 1)`; the transform reset
/// with `owner` and the immediate at `+0x8f`; then a pool node
/// ([`pool_alloc`](crate::l5_objhost::pool_alloc)), when one is free,
/// holds the object (`+0x8`) at the tail of the list (head `+0xbd`, tail
/// `+0xaa`, count `+0xcc`; `+0x4` 0, `+0x0` the old tail). Returns the
/// object.
pub fn part_object<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    scale: u32,
    index: u32,
    owner: u32,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    for off in [0x48, 0x44, 0x40] {
        g.write::<u32>(obj + off, 0);
    }
    g.write::<u32>(obj + 0x4c, ONE);
    for off in [0x5c, 0x58, 0x54, 0x50] {
        g.write::<u32>(obj + off, 0);
    }
    for off in [0x6c, 0x68, 0x64, 0x60] {
        g.write::<u32>(obj + off, scale);
    }
    g.write::<u32>(p, w(0x59));
    let o = g.read::<u32>(w(0x5e));
    let s941 = slot_target(g, w(0x66), 941);
    let table = g.call(s941, 0, &[g.read::<u32>(o + 0x10)]).eax;
    let program = g.read::<u32>(table.wrapping_add(index.wrapping_mul(4)));
    let wrapper = img.rel32_target(f + 0x84);
    crate::opu_parts::start_part(g, img, wrapper, obj, 0x1000, program, 1);
    crate::sa_misc::reset_transform(g, obj, owner, w(0x8f));
    let node = crate::l5_objhost::pool_alloc(g, img, img.rel32_target(f + 0x9f));
    if node == 0 {
        return obj;
    }
    let tail = g.read::<u32>(w(0xaa));
    g.write::<u32>(node + 4, 0);
    let count = if tail == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0xbd), node);
        g.write::<u32>(w(0xc3), node);
        w(0xcc)
    } else {
        g.write::<u32>(node, tail);
        let tail = g.read::<u32>(w(0xd8));
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(w(0xe1), node);
        w(0xea)
    };
    g.write::<u32>(node + 8, obj);
    let n = g.read::<u32>(count);
    g.write::<u32>(count, n.wrapping_add(1));
    obj
}

/// [`part_object`] (243 bytes, 1 overlay).
pub const PART_OBJECT: Sig = Sig {
    len: 243,
    hash: 0x59cf_2bf2_6360_f1b6,
    calls: &[
        (0x84, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x97, Callee::Sig(&crate::sa_misc::RESET_TRANSFORM)),
        (0x9f, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
    ],
    imports: &[],
};

/// The objects' frame (0x10001930, cdecl `(task)`, an aligned frame):
/// [`run_objects`](crate::l5_objhost::run_objects); when it says 1 (no
/// object ran), the task's state `+0x14` becomes 3 and its record
/// (`task[8]`) gets the callback at `+0x2d` (`+0x10`); then each of the
/// four 0x250-byte records from `*R + 0x70` (`R` at `+0x19`): slot 941
/// `(*rec)` gives a model, slot 772 `(model, i)` a 64-bit value that goes
/// to slot 779 `(lo, hi, model, i)` (the earlier words left on the stack)
/// with the game flag `*L` (`+0x56`) 1 around it, and [`fill`] of the
/// record's `+0x24`. Returns `run_objects`' answer, or the lock pointer
/// [`fill`] leaves in `eax` after the fourth record.
pub fn objects_frame<G: Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> crate::Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    // push ebp; and esp, -8; four pushes: the call's esp
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(16 + 4);
    let run = img.rel32_target(f + 0xb);
    let r = crate::l5_objhost::run_objects(g, img, run, esp);
    if r != 1 {
        return crate::Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x2d));
    let mut at = g.read::<u32>(w(0x19)).wrapping_add(0x70);
    let fill_f = img.rel32_target(f + 0x7f);
    for i in 0..4u32 {
        let s941 = slot_target(g, w(0x34), 941);
        let model = g.call(s941, 0, &[g.read::<u32>(at)]).eax;
        let s772 = slot_target(g, w(0x48), 772);
        let v = g.call(s772, 0, &[model, i]);
        let lock = g.read::<u32>(w(0x56));
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        let s779 = slot_target(g, w(0x5f), 779);
        g.call(s779, 0, &[v.eax, v.edx, model, i]);
        let lock = g.read::<u32>(w(0x72));
        g.write::<u32>(lock, saved);
        fill(g, img, fill_f, at + 0x24);
        at = at.wrapping_add(0x250);
    }
    crate::Ret::eax(g.read::<u32>(img.word(fill_f + 0x3d)))
}

/// [`objects_frame`] (150 bytes, 1 overlay).
pub const OBJECTS_FRAME: Sig = Sig {
    len: 150,
    hash: 0xc64c_aa89_daa7_4bdf,
    calls: &[
        (0xb, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x7f, Callee::Sig(&FILL)),
    ],
    imports: &[],
};
