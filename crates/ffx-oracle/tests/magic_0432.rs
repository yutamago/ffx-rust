//! `magic_0432.dll` against its Rust ports (`ffx_magic::m0432`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::{rec, twin};

const N: u16 = 432;

/// 0x10005240 (internal): the sound's start with the lock word seen during slot 711.
#[test]
fn sound_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5240);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    let lock = SCRATCH + 0x600;
    // slot 711 copies the lock word out (its answer is not used)
    rec(&mut t, 711, 3, move |m: &mut dyn StubMem, a: &[u32]| {
        let v = m.r32(lock);
        m.w32(lock + 0x40, v);
        v ^ a[0]
    });
    rec(&mut t, 710, 7, |_, a| 0x0710_0000 ^ a[2]);
    t.set_u32(w(&t, 0x04), lock);
    t.set_u32(w(&t, 0x0a), 0x00ab_000a);
    for id in [0u32, 0x1234] {
        t.set_u32(lock, 0x77 + id);
        for off in [0x58, 0x62, 0x6c, 0x76] {
            t.set_u32(w(&t, off), 0x5555_0000 + off);
        }
        t.check(
            at,
            Call::default(),
            &[id],
            Eax::Checked,
            ffx_magic::m0432::sound_start,
        );
    }
}
