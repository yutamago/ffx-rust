//! Round 6 shape families: an effect DLL's start-up with its work
//! buffers, the particle part and the three program lists (fuzzy family
//! `F0368_10001140` of `~/ffx-work/m5/magic/r4/families.tsv`; 7 copies in
//! `magic_0368`, `0527`, `0255`, `0617`, `0372`, `0669`, `0206`,
//! 360 to 445 bytes). The copies share the middle and differ at both
//! ends: how many buffers the block is cut into (and whether `arg` is
//! stored first), and the flag stores and slot-260 calls at the end. The
//! port reads each copy's layout from its code with a [`Cursor`].

use crate::o6_fam_cursor::Cursor;
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The start-up (a table entry, cdecl `(arg)`; `magic_0368` 0x10001140):
/// 1. `G_ARG` = `arg` (first in `magic_0527`, else after the buffers);
/// 2. slot 803 `(0x1000000, 0x10)` allocates the block; its address and
///    each `+ K` step after it go to the buffer globals in the copy's
///    order (`mov [G], eax` / `add eax, K`);
/// 3. the part `P`: `G_PART` = `P`, slot 718 `(P)`, slot 222 `(P, 0,
///    1000, 0x2000)`, slot 719 `(P, 8, 0)` gives the handle `H` (`G_H`);
/// 4. with the game lock `*G_LOCK` set to 1 around it, slot 711 `(H, Q,
///    0)`; then slot 710 `(R, H, block, 0x80000)`;
/// 5. slot 941 on `H[0x14]`, `H[0x18]`, `H[0x1c]` into three list globals
///    (`H` re-read from `G_H` each time);
/// 6. `memset(M, 0, 0x110)`;
/// 7. the copy's tail: flag stores (`mov [G], imm`) and slot 260 `(K)`
///    calls whose result goes to a global (`magic_0641`'s: a second
///    `memset` and other slots).
///
/// Each slot call sees the words the earlier ones left until the copy's
/// `add esp`. Returns 1.
#[allow(clippy::too_many_lines)] // one step per call of the original, operands read in order
pub fn effect_boot<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ecx = fr.ecx;
    let arg = fr.arg(g, 0);
    let mut c = Cursor::new(img, fr.at + 3);
    let mut s = Pushes::default();
    // mov eax, [ebp + 8]; mov [G_ARG], eax (magic_0527)
    if img.word(c.at) as u8 == 0x8b {
        c.skip(3);
        g.write::<u32>(c.word(1, 5), arg);
    }
    // mov eax, [T]; push esi; mov eax, [eax + S]; push 0x10; push 0x1000000; call eax
    let t = c.word(1, 5);
    c.skip(1);
    let slot = c.slot_load();
    s.push(c.push_imm());
    s.push(c.push_imm());
    c.skip(2);
    // call width: 12 (Pushes caps the words at 12)
    let block = s.call(g, slot_target(g, t, slot), ecx).eax;
    // the buffers: `mov [G], eax`, `add eax, K` and `mov eax, [ebp + 8]`
    // (then the next store is G_ARG) until the next `mov eax, [T]`
    let mut eax = block;
    loop {
        match img.word(c.at) as u8 {
            0xa3 => g.write::<u32>(c.word(1, 5), eax),
            0x05 => eax = eax.wrapping_add(c.word(1, 5)),
            0x8b => {
                c.skip(3);
                eax = arg;
            }
            _ => break,
        }
    }
    // mov eax, [T]; push P; mov eax, [eax + S]; mov [G_PART], P; call eax
    c.skip(5);
    s.push(c.push_imm());
    let slot = c.slot_load();
    let part_global = c.word(2, 10);
    g.write::<u32>(part_global, img.word(c.at - 4));
    c.skip(2);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, t, slot), ecx);
    // mov eax, [T]; push 0x2000; mov eax, [eax + S]; push 1000; push 0;
    // push [G_PART]; call eax
    c.skip(5);
    s.push(c.push_imm());
    let slot = c.slot_load();
    s.push(c.push_imm());
    s.push(c.push_imm());
    s.push(g.read::<u32>(c.word(2, 6)));
    c.skip(2);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, t, slot), ecx);
    // mov eax, [T]; push 0; mov eax, [eax + S]; push 8; push [G_PART]; call eax
    c.skip(5);
    s.push(c.push_imm());
    let slot = c.slot_load();
    s.push(c.push_imm());
    s.push(g.read::<u32>(c.word(2, 6)));
    c.skip(2);
    // call width: 12 (Pushes caps the words at 12)
    let h = s.call(g, slot_target(g, t, slot), ecx).eax;
    // mov ecx, [G_LOCK]; push 0; mov esi, [ecx]; mov [G_H], eax; push Q;
    // push eax; mov eax, [T]; mov dword [ecx], 1; mov eax, [eax + S]; call eax
    let lock = g.read::<u32>(c.word(2, 6));
    s.push(c.push_imm());
    c.skip(2);
    let saved = g.read::<u32>(lock);
    let h_global = c.word(1, 5);
    g.write::<u32>(h_global, h);
    s.push(c.push_imm());
    c.skip(1);
    s.push(h);
    c.skip(5);
    g.write::<u32>(lock, c.word(2, 6));
    let slot = c.slot_load();
    c.skip(2);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, t, slot), ecx);
    // mov eax, [G_LOCK]; push 0x80000; push [G_BLOCK]; mov [eax], esi;
    // push [G_H]; mov eax, [T]; push R; mov eax, [eax + S]; call eax
    let lock = g.read::<u32>(c.word(1, 5));
    s.push(c.push_imm());
    s.push(g.read::<u32>(c.word(2, 6)));
    c.skip(2);
    g.write::<u32>(lock, saved);
    s.push(g.read::<u32>(c.word(2, 6)));
    c.skip(5);
    s.push(c.push_imm());
    let slot = c.slot_load();
    c.skip(2);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, t, slot), ecx);
    // mov eax, [G_H]; add esp, n
    let mut handle = g.read::<u32>(c.word(1, 5));
    let n = c.disp8(2, 3);
    s.pop((n / 4) as usize);
    // three lists: push [eax + d]; mov eax, [T]; mov eax, [eax + S]; call
    // eax; the first two stored (mov [G], eax; mov eax, [G_H])
    let mut list = 0;
    for k in 0..3 {
        let d = c.disp8(2, 3);
        s.push(g.read::<u32>(handle.wrapping_add(d)));
        c.skip(5);
        let slot = c.slot_load();
        c.skip(2);
        // call width: 12 (Pushes caps the words at 12)
        list = s.call(g, slot_target(g, t, slot), ecx).eax;
        if k < 2 {
            g.write::<u32>(c.word(1, 5), list);
            handle = g.read::<u32>(c.word(1, 5));
        }
    }
    // push 0x110; push 0; push M; mov [G_L3], eax; call memset
    let len = c.push_imm();
    let fill = c.push_imm();
    let m = c.push_imm();
    s.push_all(&[len, fill, m]);
    g.write::<u32>(c.word(1, 5), list);
    g.write_bytes(m, &vec![fill as u8; len as usize]);
    c.skip(5);
    // the tail, up to `mov eax, 1`: flag stores, `add esp`, and slot 260
    // calls (`mov eax, [T]`, the slot load and its push in either order,
    // `call eax`, the result stored)
    let mut slot = 0;
    let mut out = 0;
    loop {
        match img.word(c.at) as u8 {
            0xc7 => {
                let at = c.word(2, 10);
                g.write::<u32>(at, img.word(c.at - 4));
            }
            0xa1 => {
                c.skip(5);
            }
            0x8b => slot = c.slot_load(),
            0x6a | 0x68 => s.push(c.push_imm()),
            0xff => {
                c.skip(2);
                // call width: 12 (Pushes caps the words at 12)
                out = s.call(g, slot_target(g, t, slot), ecx).eax;
            }
            0x83 => {
                let n = c.disp8(2, 3);
                s.pop((n / 4) as usize);
            }
            0xa3 => g.write::<u32>(c.word(1, 5), out),
            // `magic_0641`'s second table: `call memset` on the three
            // words just pushed (`(M, 0, 0x110)`)
            0xe8 => {
                let ws = s.words();
                let (m, fill, len) = (ws[0], ws[1], ws[2]);
                g.write_bytes(m, &vec![fill as u8; len as usize]);
                c.skip(5);
            }
            _ => break,
        }
    }
    // mov eax, 1
    Ret::eax(img.word(c.at + 1))
}

include!("o6_fam_boot_table.rs.in");
