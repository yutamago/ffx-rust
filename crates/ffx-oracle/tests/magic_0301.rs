//! `magic_0301.dll` against its Rust ports (`ffx_magic::m0301`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 301;

/// Every slot the three functions call records its words with a count.
fn stubs(t: &mut Twin) {
    for (s, n) in [
        (460u32, 4usize),
        (447, 2),
        (450, 2),
        (451, 2),
        (443, 2),
        (440, 4),
        (449, 2),
        (446, 2),
    ] {
        rec(t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
}

/// 0x100019c0, 0x10002190, 0x10002250: two contexts (indices and
/// counts), the object's words junk first.
#[test]
fn object_steps() {
    let Some(mut t) = twin(N) else { return };
    stubs(&mut t);
    let ctx = SCRATCH + 0x2000;
    let obj = SCRATCH + 0x1000;
    for k in 0..0x80 {
        t.set_u32(ctx + 0x180 + 4 * k, 0x0c00_0000 + k);
    }
    for (rva, gop) in [(0x19c0u32, 0x05u32), (0x2190, 0x23), (0x2250, 0x04)] {
        let at = t.at(rva);
        let w = |t: &Twin, off: u32| t.img.word(at + off);
        t.set_u32(w(&t, gop), ctx);
        for (i, n) in [(0u32, 0i32), (2, 3), (1, -1)] {
            t.set_u32(ctx + 0x198, 0x0198_0198);
            t.set_u32(
                ctx + 0x19c,
                if rva == 0x2250 { n.cast_unsigned() } else { i },
            );
            t.write(obj, &[0x5c; 0x140]);
            t.check_entry(at, Call::default(), &[obj], Eax::Checked);
        }
    }
}
