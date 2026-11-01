//! Round 6 shape families: the effect object set-up with its constant
//! stores (families `F0361_100016a0` and `F0193_10001660` of
//! `~/ffx-work/m5/magic/r4/families.tsv`; 8 copies in `magic_0361`,
//! `magic_0384`, `magic_0415` with twelve stores and `magic_0193`,
//! `magic_0382` with eight). The copies differ in their constants, the
//! number of stores, the slot of the last call and in whether two of the
//! pushes are `imm8` or `imm32`, so the port reads every operand with a
//! [`Cursor`].

use crate::o6_fam_cursor::Cursor;
use crate::pushes::Pushes;
use crate::{Frame, Game, Image, Ret, Sig};

/// `(obj)`, cdecl (`magic_0361` 0x100016a0, 241 bytes; the copies 201 to
/// 244 bytes): with `G` (the first `mov eax, [G]`) and the slot table
/// `T` (the next `mov eax, [T]`), each call seeing the words the earlier
/// ones left:
/// 1. slot `S1` (441) `(obj, (*G)[D], K1)` (`D` the `push [eax + D]`
///    displacement, 0x198);
/// 2. slot `S2` (443) `(obj, K2)`;
/// 3. `*G` read, then the copy's `obj + d = v` stores (twelve or eight),
///    then slot `S3` (440)
///    `(*G, obj, K4, K3)`;
/// 4. slot `S4` (450) `(obj, CB)` (a callback in the DLL);
/// 5. slot `S5` (451 with a small `K5`, 449 with a pointer) `(obj, K5)`.
///
/// Returns the last call's `eax`.
pub fn object_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let obj = fr.arg(g, 0);
    let mut c = Cursor::new(img, fr.at + 3);
    // mov eax, [G]; push esi; mov esi, [ebp + 8]
    let gp = c.word(1, 5);
    c.skip(4);
    let first = g.read::<u32>(gp);
    let k1 = c.push_imm();
    // push dword [eax + D]
    let d = c.word(2, 6);
    let mut s = Pushes::default();
    s.push(k1);
    s.push(g.read::<u32>(first.wrapping_add(d)));
    // mov eax, [T]; push esi; mov eax, [eax + S1]; call eax
    let t = c.word(1, 5);
    c.skip(1);
    let s1 = c.slot_load();
    c.skip(2);
    s.push(obj);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, crate::runtime::slot_target(g, t, s1), fr.ecx);
    // mov eax, [T]; push K2; mov eax, [eax + S2]; push esi; call eax
    c.skip(5);
    let k2 = c.push_imm();
    let s2 = c.slot_load();
    c.skip(3);
    s.push_all(&[k2, obj]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, crate::runtime::slot_target(g, t, s2), fr.ecx);
    // mov eax, [T]; push K3; push K4; push esi; push dword [G]
    c.skip(5);
    let table = g.read::<u32>(t);
    let k3 = c.push_imm();
    let k4 = c.push_imm();
    c.skip(1);
    let g2 = c.word(2, 6);
    s.push_all(&[k3, k4, obj, g.read::<u32>(g2)]);
    // mov dword [esi + d], v (c7 86) up to the slot load
    while img.word(c.at) as u8 == 0xc7 {
        let (d, v) = c.store_imm();
        g.write::<u32>(obj.wrapping_add(d), v);
    }
    // mov eax, [eax + S3]; call eax
    let s3 = c.slot_load();
    c.skip(2);
    let target = g.read::<u32>(table.wrapping_add(4 * s3));
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, target, fr.ecx);
    // mov eax, [T]; push CB; mov eax, [eax + S4]; push esi; call eax
    c.skip(5);
    let cb = c.push_imm();
    let s4 = c.slot_load();
    c.skip(3);
    s.push_all(&[cb, obj]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, crate::runtime::slot_target(g, t, s4), fr.ecx);
    // mov eax, [T]; push K5; mov eax, [eax + S5]; push esi; call eax
    c.skip(5);
    let k5 = c.push_imm();
    let s5 = c.slot_load();
    s.push_all(&[k5, obj]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, crate::runtime::slot_target(g, t, s5), fr.ecx).eax)
}

include!("o6_fam_setup_table.rs.in");
