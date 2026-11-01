//! `magic_0298.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// Models to a list (0x10001fe0, internal: `ecx` = the model bytes,
/// `edx` = their count, one stack word `list`): for each byte `b` the
/// model of slot 215 `(b)` goes to slot 276 `(model, list)`, and slot
/// 219 `(b, 0)` and `(b, 1)`'s answers likewise when non-zero. Returns
/// the last slot call's `eax` (the count when there are none).
pub fn models_to_list<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (bytes, n) = (fr.ecx, fr.edx);
    let list = fr.arg(g, 0);
    let mut eax = n;
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let b = u32::from(g.read::<u8>(bytes.wrapping_add(i)));
        let s = slot_target(g, w(0x25), 0x35c / 4);
        let model = g.call(s, 0, &[b]).eax;
        let s = slot_target(g, w(0x35), 0x450 / 4);
        // call width: 3 (the earlier call's word under it)
        g.call(s, 0, &[model, list, b]);
        let s = slot_target(g, w(0x42), 0x36c / 4);
        // call width: 5 (the earlier calls' words under it)
        let x = g.call(s, 0, &[b, 0, model, list, b]).eax;
        if x != 0 {
            let s = slot_target(g, w(0x5b), 0x450 / 4);
            g.call(s, 0, &[x, list]);
        }
        let s = slot_target(g, w(0x6b), 0x36c / 4);
        eax = g.call(s, 0, &[b, 1]).eax;
        if eax != 0 {
            let s = slot_target(g, w(0x84), 0x450 / 4);
            eax = g.call(s, 0, &[eax, list]).eax;
        }
        i += 1;
    }
    Ret::eax(eax)
}

/// [`models_to_list`] (163 bytes, 1 overlay).
pub const MODELS_TO_LIST: Sig = Sig {
    len: 163,
    hash: 0x6471_3a73_0816_9360,
    calls: &[],
    imports: &[],
};

/// A task step (0x10001da0, cdecl `(task)`): the motion script of the
/// record at `+0x07` ([`motion_script`](crate::s4_misc::motion_script)),
/// the objects run ([`run_objects`](crate::l5_objhost::run_objects)) and
/// slot 275 `(1)`; when both give 1 and all of the first `*G_N` (`+0x37`)
/// records (0x54 bytes from the immediate `+0x42`) have a non-zero first
/// word, the models of slot 288 `(9, &n)` and `(11, &n)` (the count `n`
/// a local at `ebp - 4`) go to the list 0
/// ([`models_to_list`](crate::m0298::models_to_list); the earlier words
/// left under each call), the task's state is 3 and its record's `+0x10`
/// the immediate `+0xa8`. Returns the record, else slot 275's `eax` or
/// the records counted.
pub fn models_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let m = crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0xc), w(0x7));
    // push ebp; push ecx; push esi; push edi; the call
    let r =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x13), ebp.wrapping_sub(0x10));
    let t = slot_target(g, w(0x19), 275);
    let eax = g.call(t, 0, &[1]).eax;
    if r != 1 || m != 1 {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(w(0x37)).cast_signed();
    let mut i = 0i32;
    let mut rec = w(0x42);
    while i < n {
        if g.read::<u32>(rec) == 0 {
            break;
        }
        i += 1;
        rec = rec.wrapping_add(0x54);
    }
    if i != n {
        return Ret::eax(i.cast_unsigned());
    }
    let cnt = ebp.wrapping_sub(4);
    let t = slot_target(g, w(0x5c), 288);
    let b = g.call(t, 0, &[9, cnt]).eax;
    let list = |g: &mut G, b: u32, esp: u32, at: u32| {
        g.write::<u32>(esp + 4, 0);
        let callee = Frame {
            at: img.rel32_target(f + at),
            esp,
            ecx: b,
            edx: g.read::<u32>(cnt),
            ..*fr
        };
        crate::m0298::models_to_list(g, img, &callee);
    };
    g.write::<u32>(ebp.wrapping_sub(0x10), cnt);
    g.write::<u32>(ebp.wrapping_sub(0x14), 9);
    list(g, b, ebp.wrapping_sub(0x1c), 0x72);
    let t = slot_target(g, w(0x7b), 288);
    let b = g.call(t, 0, &[0xb, cnt, 0, 9, cnt]).eax;
    g.write::<u32>(ebp.wrapping_sub(0x1c), cnt);
    g.write::<u32>(ebp.wrapping_sub(0x20), 0xb);
    list(g, b, ebp.wrapping_sub(0x28), 0x91);
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), w(0xa8));
    Ret::eax(rec)
}

/// [`models_step`] (178 bytes, 1 overlay).
pub const MODELS_STEP: Sig = Sig {
    len: 178,
    hash: 0xb08b_1f20_d803_ade7,
    calls: &[
        (0xc, crate::Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x13, crate::Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x72, crate::Callee::Sig(&MODELS_TO_LIST)),
        (0x91, crate::Callee::Sig(&MODELS_TO_LIST)),
    ],
    imports: &[],
};
