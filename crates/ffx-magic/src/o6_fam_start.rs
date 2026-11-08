//! Round 6 shape families: the object host's start-up with its two
//! particle parts (families `F0162_100010a0` and `F0568_100010a0` of
//! `~/ffx-work/m5/magic/r4/families.tsv`; 8 copies in `magic_0162`,
//! `0165`, `0304`, `0430`, `0568`, `0586`, `0588`, `0606`). The copies
//! differ in their globals, a few immediates (some `push imm8` where
//! others have `push imm32`) and in how many part motions they register
//! (one or two), so the port walks the code with a [`Cursor`].

use crate::l5_objhost::{obj_block_init, tasks_init};
use crate::o6_fam_cursor::Cursor;
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// One `mov eax, [T]; push ..; mov eax, [eax + S]; push ..; call eax`
/// step: `before` pushes ahead of the slot load, `after` behind it.
/// Returns the call's `eax`.
fn slot_step<G: Game>(
    g: &mut G,
    c: &mut Cursor,
    s: &mut Pushes,
    t: u32,
    before: usize,
    after: usize,
    ecx: u32,
) -> u32 {
    c.skip(5);
    for _ in 0..before {
        s.push(c.push_imm());
    }
    let slot = c.slot_load();
    for _ in 0..after {
        s.push(c.push_imm());
    }
    c.skip(2);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, t, slot), ecx).eax
}

/// The start-up (a table entry, cdecl `(arg)`; `magic_0162` 0x100010a0,
/// 433 bytes, and `magic_0568` 0x100010a0, 393 bytes, with their copies):
/// `G_ARG` = `arg` and the object block set up ([`obj_block_init`]);
/// slots 556, 555 and 331 on the DLL's three blocks; three scales 1.0 and
/// slot 790's float (ST0) into `G_F`; the task ring ([`tasks_init`]); the
/// 0x3ff nodes of pool `A` free (payload 0, the free count 0x3ff, the
/// rover 0); slot 720 `(0x3400, 0x800)`; slot 718 and 222 `(part, 0, K,
/// 1)` on part `A`; slot 718 and 221 `(part, 0, 1000, 1)` on part `B`;
/// slot 718 on `B` again; then once or twice slot 719 `(B, 6, M)` and
/// slot 551 `(its eax, I, 1)`; `G_DONE` 0. Each call sees the words the
/// earlier ones left (until the `add esp`). Returns 0. `magic_0598`'s
/// (0x10006740) and `magic_0600`'s (0x10006140) copies call slot 4
/// `(id)` and slot 27 `(id, 1)` after slot 720.
pub fn host_start_parts<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ecx = fr.ecx;
    let arg = fr.arg(g, 0);
    let mut c = Cursor::new(img, fr.at + 6);
    // mov [G_ARG], ecx; call obj_block_init
    g.write::<u32>(c.word(2, 6), arg);
    obj_block_init(g, img, img.rel32_target(c.at + 1), arg);
    c.skip(5);
    // mov eax, [T] (the slot table global, the same in every step)
    let t = img.word(c.at + 1);
    let mut s = Pushes::default();
    for _ in 0..3 {
        slot_step(g, &mut c, &mut s, t, 1, 0, ecx);
    }
    // mov eax, [T]; add esp, 0xc; mov eax, [eax + S]; three 1.0 stores;
    // call eax; fstp dword [G_F]
    c.skip(5 + 3);
    s.pop(3);
    let table = g.read::<u32>(t);
    let slot = c.slot_load();
    for _ in 0..3 {
        let at = c.word(2, 10);
        g.write::<u32>(at, img.word(c.at - 4));
    }
    c.skip(2);
    let target = g.read::<u32>(table.wrapping_add(4 * slot));
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, target, ecx);
    g.write::<u32>(c.word(2, 6), crate::sa_summon::st0_f32(out));
    // call tasks_init
    tasks_init(g, img, img.rel32_target(c.at + 1));
    c.skip(5);
    // mov eax, A; mov ecx, N; lea ebx, [ebx]; the clearing loop
    let a = c.word(1, 5);
    let n = c.word(1, 5);
    c.skip(6 + 6);
    let stride = c.disp8(2, 3);
    c.skip(3);
    let mut at = a;
    for _ in 0..n {
        g.write::<u32>(at, 0);
        at = at.wrapping_add(stride);
    }
    // mov eax, N2; mov [G_FREE], ax; xor eax, eax; mov [G_ROVER], ax
    let free = c.word(1, 5);
    g.write::<u16>(c.word(2, 6), free as u16);
    c.skip(2);
    g.write::<u16>(c.word(2, 6), 0);
    // slot 720 (one push before the load, one after)
    slot_step(g, &mut c, &mut s, t, 1, 1, ecx);
    // `magic_0598`/`0600`: slot 4 `(id)` and slot 27 `(id, 1)` (`mov eax,
    // [eax + 0x10]` behind the first push)
    if img.word(c.at + 10) & 0x00ff_ffff == 0x0010_408b {
        slot_step(g, &mut c, &mut s, t, 1, 0, ecx);
        slot_step(g, &mut c, &mut s, t, 1, 1, ecx);
    }
    // part A: 718 (1 push), 222 (1 + 3)
    slot_step(g, &mut c, &mut s, t, 1, 0, ecx);
    slot_step(g, &mut c, &mut s, t, 1, 3, ecx);
    slot_step(g, &mut c, &mut s, t, 1, 3, ecx);
    // part B: 718, 221
    slot_step(g, &mut c, &mut s, t, 1, 0, ecx);
    slot_step(g, &mut c, &mut s, t, 1, 3, ecx);
    // mov eax, [T]; add esp, n; mov eax, [eax + S]; push B; call eax
    let n = img.word(c.at + 7) & 0xff;
    c.skip(5 + 3);
    s.pop(n as usize / 4);
    let table = g.read::<u32>(t);
    let slot = c.slot_load();
    s.push(c.push_imm());
    c.skip(2);
    let target = g.read::<u32>(table.wrapping_add(4 * slot));
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, target, ecx);
    // one or two motions: 719 (1 + 2 pushes), then `push i; push j; push
    // eax` and 551
    while img.word(c.at) as u8 == 0xa1 {
        let h = slot_step(g, &mut c, &mut s, t, 1, 2, ecx);
        s.push(c.push_imm());
        s.push(c.push_imm());
        s.push(h);
        c.skip(1);
        slot_step(g, &mut c, &mut s, t, 0, 0, ecx);
    }
    // add esp, n; mov [G_DONE], 0
    c.skip(3);
    g.write::<u32>(c.word(2, 10), img.word(c.at - 4));
    Ret::eax(0)
}

include!("o6_fam_start_table.rs.in");
