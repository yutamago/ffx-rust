//! Round 11's object set-up between two actors (`ffx_magic::r11_actors`,
//! `magic_0356` 0x10001260 and `magic_0357` 0x10001320) against each
//! original. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{dump, rec, twin};

/// Eight runs (three `rand`s each, so both callbacks come up), the
/// actors' floats changing per run (one run with a NaN), the object's
/// words junk first.
#[test]
fn between_actors() {
    for (dll, rva) in [(356u16, 0x1260u32), (357, 0x1320)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
        let gp = SCRATCH + 0x100;
        t.set_u32(op(&t, 0x1d), gp);
        t.set_u32(gp + 0x198, 0x00a1_0000);
        t.set_u32(gp + 0x1a0, 0x00b1_0000);
        let obj = SCRATCH + 0x1000;
        // slot 463 fills a matrix from the actor
        rec(&mut t, 463, 3, |m, a| {
            for i in 0..16 {
                m.w32(a[2] + 4 * i, 0x3f00_0000 + (a[0] >> 12) + 0x1_0000 * i);
            }
            0x463
        });
        // slot 229 `(out, m, in)`: out = in's words plus the matrix's first
        rec(&mut t, 229, 3, |m, a| {
            dump(m, 229, a[2], 4);
            for i in 0..3 {
                let v = m.r32(a[2] + 4 * i) ^ (m.r32(a[1] + 4 * i) & 0x0000_ff00);
                m.w32(a[0] + 4 * i, v);
            }
            0x229
        });
        let run = SCRATCH + 0x80;
        t.stub_st0(488, move |m, a| {
            let k = m.r32(run);
            if k == 5 {
                f64::NAN
            } else {
                f64::from(a[0] >> 16) * 0.125 + f64::from(k)
            }
        });
        t.stub_st0(489, move |m, a| {
            f64::from(a[0] >> 16) * -0.375 + f64::from(m.r32(run)) * 0.5
        });
        rec(&mut t, 490, 3, |_, a| a[1] ^ 0x490);
        rec(&mut t, 450, 2, |_, a| a[1] ^ 0x450);
        rec(&mut t, 451, 2, |_, a| a[1] ^ 0x451);
        for k in 0..8u32 {
            t.set_u32(run, k);
            t.write(obj, &[0x5d; 0x400]);
            t.set_u32(obj + 0xbc, 0x0101_0101 * k);
            t.check_entry(f, Call::default(), &[obj], Eax::Checked);
        }
    }
}
