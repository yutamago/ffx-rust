//! Every copy of the map effects' shared task ports (`ffx-magic`'s `rcef.rs`)
//! in the overlays that carry them, whatever list owns the DLL: each copy the
//! ports bind in is run against its own original (`magic::map::check_all`).
//! The DLLs are those `magic-sig --bind` reports (`carriers-all`, docs/
//! formats/magic.md). Needs `FFX_GAME_DIR`.
//!
//! One test per DLL: a second `Twin::load` on the same thread clears the
//! harness heap, which holds that thread's Unicorn bookkeeping, so each DLL
//! gets a test thread of its own.

mod magic;

use magic::Twin;
use magic::map;

fn check_dll(n: u16) {
    let Some(mut t) = Twin::load(n) else { return };
    t.init();
    map::stubs(&mut t);
    let checked = map::check_all(&mut t);
    assert!(checked > 0, "magic_{n:04}: no copy of the ports bound");
    eprintln!("magic_{n:04}: {checked} copies checked");
}

macro_rules! carriers {
    ($($name:ident: $n:literal,)*) => {
        $(
            #[test]
            fn $name() {
                check_dll($n);
            }
        )*
    };
}

carriers! {
    magic_0187: 187,
    magic_0191: 191,
    magic_0193: 193,
    magic_0194: 194,
    magic_0196: 196,
    magic_0300: 300,
    magic_0301: 301,
    magic_0312: 312,
    magic_0313: 313,
    magic_0334: 334,
    magic_0335: 335,
    magic_0336: 336,
    magic_0337: 337,
    magic_0339: 339,
    magic_0356: 356,
    magic_0357: 357,
    magic_0359: 359,
    magic_0360: 360,
    magic_0361: 361,
    magic_0362: 362,
    magic_0363: 363,
    magic_0364: 364,
    magic_0366: 366,
    magic_0378: 378,
    magic_0382: 382,
    magic_0415: 415,
    magic_0416: 416,
    magic_0507: 507,
    magic_0510: 510,
    magic_0521: 521,
    magic_0522: 522,
    magic_0525: 525,
    magic_0526: 526,
    magic_0529: 529,
    magic_0534: 534,
}
