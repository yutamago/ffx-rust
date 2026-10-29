//! The map effects' task functions (ffx-magic's `rcef.rs` and `m0334`..
//! `m0339.rs`), checked the same way in every DLL that carries them: the
//! harness runs the bound port of each copy against that copy's original.
//!
//! The game globals a function reads are found through the instruction
//! operand at `off` (the same offset in every carrier, since the code is
//! the same modulo relocations). The stubs for the `rcef*` slots read as
//! many argument words as the callee takes, so a port that passes fewer
//! words fails at the stub, and a port that passes others fails at the
//! call-out comparison. Some stubs also change the game global, so a port
//! that reads it at the wrong moment fails too.

use super::{Call, Eax, SCRATCH, StubMem, Twin};

/// The game word a function reads through its operand.
#[derive(Clone, Copy, Debug)]
pub enum Var {
    /// No game global.
    No,
    /// The value of the global at the operand (pushed as a word).
    Plain(u32),
    /// A pointer in the global at the operand; the word at `+0x1a0` of the
    /// object it points to is what the function pushes.
    Deref(u32),
}

/// The game global each port reads, by its name in `ffx_magic::PORTS`.
#[must_use]
pub fn var_for(name: &str) -> Option<Var> {
    Some(match name {
        "m0334 set seq, call seq" | "rcef effect ready seq" | "m0339 task seq, no args" => Var::No,
        "rcef create task wait" | "m0335 create task pair" => Var::Plain(0x26),
        "m0334 main init, create task" => Var::Plain(0x39),
        "m0336 create task wait" => Var::Plain(0xf),
        "m0336 damage seq" | "m0336 obj attach, set par C" | "m0339 obj attach, set par" => {
            Var::Deref(4)
        }
        "m0337 obj attach, set par B" => Var::Deref(0x14),
        _ => return None,
    })
}

/// The arity of every `rcef*` slot the map functions call (the words the
/// caller pushes, read from the original's `push`es).
const ARITY: [(u32, usize); 13] = [
    (433, 1),
    (434, 2),
    (436, 9),
    (440, 4),
    (442, 3),
    (443, 2),
    (444, 1),
    (446, 2),
    (449, 2),
    (450, 2),
    (451, 2),
    (472, 0),
    (473, 5),
];

/// A slot's result: a value of its arguments (the first `n`, read from the
/// original's logged words and the port's own: the same on both sides).
fn value(slot: u32, a: &[u32], n: usize) -> u32 {
    let mut x = slot.wrapping_mul(0x9e37_79b9);
    for (i, w) in a[..n].iter().enumerate() {
        x = x.rotate_left(5) ^ w.wrapping_add(i as u32);
    }
    x
}

/// The stubs for the slots above: each returns a value of its arguments.
pub fn stubs(t: &mut Twin) {
    for (slot, n) in ARITY {
        t.stub(slot, move |_, a| value(slot, a, n));
    }
}

/// The objects the pointer global moves to during the calls (see [`check`]):
/// slot 444 leaves `MOVED`, slot 443 leaves `MOVED_AGAIN`, so a read before
/// 443 and a read after it see different pointers.
const MOVED: u32 = SCRATCH + 0x4000;
const MOVED_AGAIN: u32 = SCRATCH + 0x5000;

/// Checks the function at `rva` of `t`'s DLL: the bound port against the
/// original, for three object pointers, two `ecx` values and two `eax`
/// values.
///
/// The stubs change the game global the function reads: for a plain word,
/// `rcefMainInit` (slot 436) writes a new value there, so a port that reads
/// it once fails; for a pointer, the calls that follow the first read
/// (slots 443 and 444) make it point at [`MOVED`], so a port that keeps the
/// old pointer fails.
pub fn check(t: &mut Twin, rva: u32, var: Var) {
    let f = t.at(rva);
    let p = SCRATCH + 0x2000;
    match var {
        Var::No => {}
        Var::Plain(off) => {
            let g = t.img.word(f + off);
            t.set_u32(g, 0x7000_0000 + rva);
            t.stub(436, move |m: &mut dyn StubMem, a: &[u32]| {
                let x = a[..9]
                    .iter()
                    .fold(0x5a5a_5a5a_u32, |x, w| x.rotate_left(3) ^ w);
                m.w32(g, x);
                value(436, a, 9)
            });
        }
        Var::Deref(off) => {
            let g = t.img.word(f + off);
            t.set_u32(g, p);
            t.set_u32(p + 0x1a0, 0x5a00_0000 + rva);
            t.set_u32(MOVED + 0x1a0, 0x6a00_0000 + rva);
            for (slot, n, to) in [(443, 2, MOVED_AGAIN), (444, 1, MOVED)] {
                t.stub(slot, move |m: &mut dyn StubMem, a: &[u32]| {
                    m.w32(g, to);
                    value(slot, a, n)
                });
            }
        }
    }
    for obj in [SCRATCH, SCRATCH + 0x800, SCRATCH + 0x3000] {
        for (ecx, eax) in [(0, 0), (0x4444, 0xeeee_0001)] {
            t.check_entry(
                f,
                Call {
                    eax,
                    ecx,
                    edx: 0x77,
                    ..Call::default()
                },
                &[obj],
                Eax::Checked,
            );
        }
    }
    // the next function starts from the plain stubs
    stubs(t);
}

/// Checks every copy of the ports in `t`'s DLL: each 16-byte aligned
/// address of the code the ports bind to. Returns how many were checked.
pub fn check_all(t: &mut Twin) -> usize {
    let (lo, hi) = t.img.text;
    let mut n = 0;
    let mut rva = (lo + 15) & !15;
    while rva < hi {
        let addr = t.at(rva);
        if let Some(var) =
            ffx_magic::bind(&t.img, addr).and_then(|i| var_for(ffx_magic::PORTS[i].0))
        {
            check(t, rva, var);
            n += 1;
        }
        rva += 16;
    }
    n
}
