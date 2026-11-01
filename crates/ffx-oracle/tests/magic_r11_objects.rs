//! Round 11's object start-ups (`ffx_magic::r11_objects`) against each
//! copy's original. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;
mod magic_r11;

use ffx_magic::r11_objects;
use magic::{Call, Eax};
use magic_owna::twin;

/// The start-up with three task nodes (`magic_0110`, `0112` 0x10001410,
/// `0121` 0x10002ec0): no, one and three actors; three and five free
/// nodes (the third take leaves the ring empty or not); the task list
/// empty and not.
#[test]
fn three_nodes() {
    for (dll, rva) in [(110u16, 0x1410u32), (112, 0x1410), (121, 0x2ec0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        for (n, free, listed) in [
            (0u32, 3u16, false),
            (1, 5, true),
            (3, 3, true),
            (2, 5, false),
        ] {
            magic_r11::three_nodes_setup(&mut t, f, n, free, listed);
            t.check(
                f,
                Call::default(),
                &[],
                Eax::Checked,
                r11_objects::three_nodes,
            );
        }
    }
}

/// The start-up whose points take slot 12's height (`magic_0120`
/// 0x10001410): as [`three_nodes`].
#[test]
fn three_nodes_height() {
    let Some(mut t) = twin(120) else { return };
    let f = t.at(0x1410);
    for (n, free, listed) in [
        (0u32, 3u16, false),
        (1, 5, true),
        (3, 3, true),
        (2, 5, false),
    ] {
        magic_r11::three_nodes_height_setup(&mut t, f, n, free, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            r11_objects::three_nodes_height,
        );
    }
}

/// The scaled start-ups (`magic_0126` 0x10001460 with the kind check and
/// the height, `magic_0127` 0x10002d30): no, one and three actors (scales
/// below, inside and above the clamp; kinds listed and not), the ring and
/// the list as in [`three_nodes`].
#[test]
fn three_nodes_scaled() {
    let cases = [
        (0u32, 3u16, false),
        (1, 5, true),
        (3, 3, true),
        (2, 5, false),
    ];
    if let Some(mut t) = twin(126) {
        let f = t.at(0x1460);
        for (n, free, listed) in cases {
            magic_r11::three_nodes_kind_setup(&mut t, f, n, free, listed);
            t.check(
                f,
                Call::default(),
                &[],
                Eax::Checked,
                r11_objects::three_nodes_kind,
            );
        }
    }
    if let Some(mut t) = twin(127) {
        let f = t.at(0x2d30);
        for (n, free, listed) in cases {
            magic_r11::three_nodes_scaled_setup(&mut t, f, n, free, listed);
            t.check(
                f,
                Call::default(),
                &[],
                Eax::Checked,
                r11_objects::three_nodes_scaled,
            );
        }
    }
}

/// The start-up with two objects per actor (`magic_0116` 0x10001460,
/// `0117` 0x10002f00): no, one and three actors (one actor picks the
/// other callback), three and five free nodes, the task list empty and
/// not.
#[test]
fn two_per_actor() {
    for (dll, rva) in [(116u16, 0x1460u32), (117, 0x2f00)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        for (n, free, listed) in [
            (0u32, 3u16, false),
            (1, 5, true),
            (3, 3, true),
            (2, 5, false),
        ] {
            magic_r11::two_per_actor_setup(&mut t, f, n, free, listed);
            t.check(
                f,
                Call::default(),
                &[],
                Eax::Checked,
                r11_objects::two_per_actor,
            );
        }
    }
}
