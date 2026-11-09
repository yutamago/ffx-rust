//! Round 31's ports (`ffx_magic::r31`) against each original, and the
//! step-0 sweep's copies with relocated immediates. Needs `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::type_complexity
)]

mod magic;

use magic::{Call, Eax, SCRATCH, Twin};

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, (0x5a5a_5a5a ^ o) | 1);
    }
}

/// `static_node_link_b` / `_c` in the copies whose last stores carry
/// relocated immediates (addresses; `magic_0152`/`0374`'s store 0, and
/// the `Sig` masks the difference): `magic_0155` 0x10009900 (one
/// address), `magic_0577` 0x1000a650 and 0x1000d870 (two each).
#[test]
fn static_links_store_their_immediates() {
    for (n, addrs, stores) in [
        (155u16, &[0x9900u32][..], &[0x4cu32, 0x56, 0x60][..]),
        (577, &[0xa650][..], &[0x4c, 0x56, 0x60][..]),
        (577, &[0xd870][..], &[0x4c, 0x56, 0x60, 0x6a][..]),
    ] {
        let Some(mut t) = twin(n) else { return };
        for &a in addrs {
            let f = t.at(a);
            let last = SCRATCH + 0x300;
            t.write(last, &[0x77; 0x10]);
            t.set_u32(t.img.word(f + 0x1), last);
            junk_ops(&mut t, f, &[0x7, 0x11, 0x1b, 0x24, 0x2a, 0x3b, 0x42]);
            junk_ops(&mut t, f, stores);
            assert!(
                stores.iter().any(|&o| t.img.word(f + o + 4) != 0),
                "magic_{n:04} {a:#x}: no relocated immediate"
            );
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}
