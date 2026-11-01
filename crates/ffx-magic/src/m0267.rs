//! `magic_0267.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::sb_sse::{div, mul};
use crate::{Frame, Game, Image, Ret, Sig};

/// Points through two transforms (0x10001340, internal: `ecx` = `b`,
/// `edx` = the points (16 bytes each), stack `(n, out, m)`): nothing for
/// `n <= 0` (the caller's `eax`). Else `out -= b` (in its argument word)
/// and for each point `p` (the i-th, `b` growing by 16): slot 229 `(p,
/// NAME, b)` (`NAME` at `+0x30`), slot 229 `(out + b, m, b)`, then `p`'s
/// x, y and z times `K / w` and `w` = `K / w` (`K` at `+0x52`); `n`
/// counts down in its argument word. Returns the last slot call's `eax`.
pub fn transform_points<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let n_at = fr.esp + 4;
    if g.read::<i32>(n_at) <= 0 {
        return Ret::eax(fr.eax);
    }
    let mut b = fr.ecx;
    let out = g.read::<u32>(fr.esp + 8).wrapping_sub(b);
    g.write::<u32>(fr.esp + 8, out);
    let mut p = fr.edx;
    let mut eax;
    loop {
        let s = slot_target(g, w(0x21), 0x394 / 4);
        g.call(s, 0, &[p, w(0x30), b]);
        // the argument words are read again each time
        let out = g.read::<u32>(fr.esp + 8);
        let m = g.read::<u32>(fr.esp + 0xc);
        let s = slot_target(g, w(0x42), 0x394 / 4);
        // call width: 6 (the earlier call's words under it)
        eax = g
            .call(s, 0, &[out.wrapping_add(b), m, b, p, w(0x30), b])
            .eax;
        let k = g.read::<f32>(w(0x52));
        let scale = div(k, g.read::<f32>(p + 0xc));
        for off in [0, 4, 8] {
            let v = mul(g.read::<f32>(p + off), scale);
            g.write::<f32>(p + off, v);
        }
        let v = div(k, g.read::<f32>(p + 0xc));
        g.write::<f32>(p + 0xc, v);
        b = b.wrapping_add(0x10);
        p = p.wrapping_add(0x10);
        let n = g.read::<u32>(n_at).wrapping_sub(1);
        g.write::<u32>(n_at, n);
        if n == 0 {
            break;
        }
    }
    Ret::eax(eax)
}

/// [`transform_points`] (170 bytes, 1 overlay).
pub const TRANSFORM_POINTS: Sig = Sig {
    len: 170,
    hash: 0xb2ab_4ffe_b669_936a,
    calls: &[],
    imports: &[],
};

/// A scale matrix (0x10003210, internal, `edx` = the matrix): slot 235
/// `(m)` resets it, the three diagonal words (`+0`, `+0x14`, `+0x28`)
/// become slot 277 `(*X)`'s float times `K` (`X` `+0x1d`, `K` `+0x35`) and
/// are then each multiplied by slot 15, 16 and 17 `(*Y)`'s floats (`Y`
/// `+0x4e`). Returns slot 17's `eax`. The slot calls pass the words the
/// original leaves on the stack.
pub fn scale_matrix<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let m = fr.edx;
    let mut st = crate::pushes::Pushes::default();
    st.push(m);
    let s = slot_target(g, w(0x05), 0x3ac / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    st.push(g.read::<u32>(w(0x1d)));
    let s = slot_target(g, w(0x17), 0x454 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let v = crate::m0018::st0_f32(st.call(g, s, 0).st0);
    let v = mul(v, g.read::<f32>(w(0x35)));
    for off in [0x28, 0x14, 0] {
        g.write::<f32>(m + off, v);
    }
    let y = g.read::<u32>(w(0x4e));
    let mut eax = 0;
    for (table, slot, off) in [(0x3a, 0x3c, 0), (0x65, 0x40, 0x14), (0x81, 0x44, 0x28)] {
        st.push(y);
        let s = slot_target(g, w(table), slot / 4);
        // call width: 12 (Pushes caps the words at 12)
        let out = st.call(g, s, 0);
        eax = out.eax;
        let k = crate::m0018::st0_f32(out.st0);
        let x = mul(g.read::<f32>(m + off), k);
        g.write::<f32>(m + off, x);
    }
    Ret::eax(eax)
}

/// [`scale_matrix`] (171 bytes, 1 overlay).
pub const SCALE_MATRIX: Sig = Sig {
    len: 171,
    hash: 0xb673_b784_1760_eb77,
    calls: &[],
    imports: &[],
};

/// The frame callback (0x10002910, cdecl `(task)`, a /GS frame of 0x14
/// bytes): slot 262 `(*X, &a, &b, &c)` (`X` `+0x28`, on the frame at
/// `ebp - 0x14`) puts the caster's position into the object `O` (`+0x57`:
/// `a`, `b`, `c`, 1.0 at `+0x3d`, `+0x4a`, `+0x66`, `+0x6c`), its script
/// runs ([`obj_script`](crate::l5_objhost::obj_script), `ecx` = `O`), the
/// motion steps ([`motion_step`](crate::s3_motion::motion_step), `+0x7a`)
/// and the objects run ([`run_objects`](crate::l5_objhost::run_objects),
/// `+0x81`); when both answer 1 the task gets state 3 and its record the
/// callback `+0x9b`. Returns the record, else the objects' answer.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let task = fr.arg(g, 0);
    let (pa, pb, pc) = (
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x10),
        ebp.wrapping_sub(0xc),
    );
    let s = slot_target(g, w(0x22), 0x418 / 4);
    let x = g.read::<u32>(w(0x28));
    g.call(s, 0, &[x, pa, pb, pc]);
    let a = g.read::<u32>(pa);
    g.write::<u32>(w(0x3d), a);
    let b = g.read::<u32>(pb);
    g.write::<u32>(w(0x4a), b);
    let c = g.read::<u32>(pc);
    g.write::<u32>(w(0x66), c);
    g.write::<u32>(w(0x6c), 0x3f80_0000);
    crate::l5_objhost::obj_script(g, img, img.rel32_target(f + 0x75), w(0x57));
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x7a));
    // push ebp; sub esp, 0x14; push esi; push edi; the call's return address
    let r = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x81),
        fr.esp.wrapping_sub(0x24),
    );
    if r != 1 || m != 1 {
        return Ret::eax(r);
    }
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(task + 0x14, 3);
    g.write::<u32>(rec + 0x10, w(0x9b));
    Ret::eax(rec)
}

/// [`frame`] (175 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 175,
    hash: 0xc97e_ae6d_f258_000f,
    calls: &[
        (0x75, crate::Callee::Sig(&crate::l5_objhost::OBJ_SCRIPT)),
        (0x7a, crate::Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x81, crate::Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        // __security_check_cookie
        (0xa7, crate::Callee::Any),
    ],
    imports: &[],
};
