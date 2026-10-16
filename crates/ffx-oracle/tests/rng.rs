//! The random number generators (`ffx_battle::rng`) against the original,
//! on two oracles started alike: one runs `FFX.exe`'s code, the other the
//! Rust bridge on the same memory; results and every written byte must
//! agree. Needs `FFX_GAME_DIR` (and `FFX_SNAPSHOT` for the snapshot case).

use std::path::PathBuf;

use ffx_battle::rng::bridge;
use ffx_oracle::{Conv, Oracle, Pages};

const RND: u32 = 0x0079_89a0;
const BRND: u32 = 0x0079_88f0;
const MS_RAND_MIZE: u32 = 0x0079_8890;
const RNG_CLOCK_SEED: u32 = 0x0079_8950;

fn exe() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    Some(dir?.join("FFX.exe"))
}

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

/// Runs the original in `a` and the bridge in `b` from the same state;
/// checks results and memory.
fn twin(
    what: &str,
    a: &mut Oracle,
    b: &mut Oracle,
    original: impl Fn(&mut Oracle) -> u32,
    rust: impl Fn(&mut Oracle) -> u32,
) {
    let ra = original(a);
    let rb = rust(b);
    assert_eq!(ra, rb, "{what}: results");
    assert_eq!(a.diff(b).unwrap(), [], "{what}: memory");
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    for _ in 0..500 {
        let seed = rng.next();
        for o in [&mut *a, &mut *b] {
            let at = o.rebase(bridge::SEED.sym);
            o.write(at, &seed.to_le_bytes()).unwrap();
        }
        twin(
            &format!("rnd seed {seed:#x}"),
            a,
            b,
            |o| o.call(o.rebase(RND), Conv::Cdecl, &[]).unwrap().eax,
            bridge::rnd,
        );
    }
    for _ in 0..500 {
        let index = rng.next() % 68;
        let state = rng.next();
        for o in [&mut *a, &mut *b] {
            let at = o.rebase(bridge::STATES.sym) + 4 * index;
            o.write(at, &state.to_le_bytes()).unwrap();
        }
        twin(
            &format!("brnd {index} state {state:#x}"),
            a,
            b,
            |o| o.call(o.rebase(BRND), Conv::Cdecl, &[index]).unwrap().eax,
            |m| bridge::brnd(m, index),
        );
    }
    for _ in 0..50 {
        let (clock, n) = (rng.next() & 0xff, rng.next() % 4);
        let stub = a.rebase(RNG_CLOCK_SEED);
        a.stub_return(stub, clock).unwrap();
        twin(
            &format!("MsRandMize clock {clock} n {n}"),
            a,
            b,
            |o| {
                o.call(o.rebase(MS_RAND_MIZE), Conv::Cdecl, &[n]).unwrap();
                o.read_u32(o.rebase(bridge::SEED.sym))
            },
            |m| {
                bridge::ms_rand_mize(m, clock, n);
                bridge::SEED.get(m)
            },
        );
    }
}

trait ReadU32 {
    fn read_u32(&mut self, at: u32) -> u32;
}

impl ReadU32 for Oracle {
    fn read_u32(&mut self, at: u32) -> u32 {
        u32::from_le_bytes(self.read(at, 4).unwrap().try_into().unwrap())
    }
}

#[test]
fn generators_match_the_original() {
    let Some(exe) = exe() else { return };
    let mut a = Oracle::load(&exe).unwrap();
    let mut b = Oracle::load(&exe).unwrap();
    check(&mut a, &mut b, &mut Xorshift(0x2545_f491));
}

#[test]
fn generators_match_the_original_on_a_snapshot() {
    let Some(exe) = exe() else { return };
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; skipping");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
    let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
    // the game's own state first, untouched
    twin(
        "rnd on the game's seed",
        &mut a,
        &mut b,
        |o| o.call(o.rebase(RND), Conv::Cdecl, &[]).unwrap().eax,
        bridge::rnd,
    );
    assert_ne!(a.image_base(), 0);
    let _ = b.image_base();
    check(&mut a, &mut b, &mut Xorshift(0x1234_5678));
}
