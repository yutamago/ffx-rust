//! The `magic_0162` family's script commands (`ffx_magic::sa_script`)
//! against `magic_0162.dll`. Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

mod magic;
#[path = "magic/sa.rs"]
mod sa;

use magic::{Call, Eax, SCRATCH, TRAPS, Twin};
use sa::{node, rec};

const N: u16 = 162;
const OBJ: u32 = SCRATCH + 0x1000;
const PC: u32 = SCRATCH + 0x1400;
const ACTORS: u32 = SCRATCH + 0x1800;
/// The actor indices the commands are tried with (-1 the caller).
const WHO: [i16; 6] = [-1, 0, 1, 2, 3, 4];

struct Globals {
    me: u32,
    count: u32,
    list: u32,
    battle: u32,
    me_obj: u32,
    objs: u32,
}

/// The DLL's actor globals (named by `actor_visible`'s and `show_all`'s
/// operands), filled: the caller 0x11, an actor list of party members
/// and others, objects `0x0b00_0000 + n`.
fn globals(t: &mut Twin) -> Globals {
    let f = t.at(0x62b0);
    let g = Globals {
        count: t.img.word(f + 0x08),
        battle: t.img.word(f + 0x12),
        list: t.img.word(f + 0x19),
        me: t.img.word(f + 0x20),
        me_obj: t.img.word(f + 0x48),
        objs: t.img.word(f + 0x55),
    };
    t.set_u32(g.me, 0x11);
    t.set_u32(g.list, ACTORS);
    t.write(ACTORS, &[0x05, 0x25, 0x1f, 0x20, 0xff, 0x00]);
    t.set_u32(g.me_obj, 0x0bad_0b1e);
    for n in 0..256 {
        t.set_u32(g.objs + 4 * n, 0x0b00_0000 + n);
    }
    t.set_u32(g.count, 5);
    g
}

fn twin() -> Option<(Twin, Globals)> {
    let mut t = sa::twin(N)?;
    let g = globals(&mut t);
    // every slot the commands call records its words and answers its
    // number and first word
    for (s, n) in [
        (313, 1),
        (378, 1),
        (295, 2),
        (719, 3),
        (551, 3),
        (220, 3),
        (546, 1),
        (525, 1),
        (6, 2),
        (547, 1),
        (548, 1),
        (296, 2),
        (257, 2),
        (541, 3),
        (38, 3),
        (713, 1),
        (293, 1),
        (413, 1),
        (215, 1),
    ] {
        rec(&mut t, s, n, move |m, a| match s {
            313 => m.r32(SCRATCH + 0x3000),
            293 => m.r32(SCRATCH + 0x3004),
            _ => s << 20 ^ a[0],
        });
    }
    Some((t, g))
}

/// A command at `rva` with the program words `words` (from `pc`).
fn cmd(t: &mut Twin, rva: u32, words: &[u32]) {
    t.write(OBJ, &[0x3c; 0x100]);
    t.set_u8(OBJ + 0x58, 3);
    for (i, w) in (0u32..).zip(words) {
        t.set_u32(PC + 4 * i, *w);
    }
    let f = t.at(rva);
    t.check_entry(f, Call::default(), &[OBJ, PC], Eax::Checked);
}

/// The first word: an opcode in the low half, the `u16`/`i16` operand
/// in the high half.
fn op(v: i16) -> u32 {
    0x0077 | u32::from(v as u16) << 16
}

#[test]
fn stack_and_wait_commands() {
    let Some((mut t, _)) = twin() else { return };
    for rva in [0x6dc0, 0x6d00, 0x6ce0, 0x6d40, 0x6c20] {
        for v in [0i16, 7, -1] {
            cmd(&mut t, rva, &[op(v), 0x1234_5678, 0x9abc_def0]);
        }
    }
}

#[test]
fn branch_on_a_callback() {
    let Some((mut t, _)) = twin() else { return };
    // the callback: a trap slot no overlay uses, answering the cell
    rec(&mut t, 0, 1, |m, _| m.r32(SCRATCH + 0x3008));
    for answer in [0u32, 1, 2, 0xffff_ffff] {
        t.set_u32(SCRATCH + 0x3008, answer);
        cmd(&mut t, 0x78f0, &[op(0), TRAPS, 0x00aa_bb00]);
    }
}

#[test]
fn record_commands() {
    let Some((mut t, _)) = twin() else { return };
    let v = SCRATCH + 0x2000;
    for (i, x) in [1.5f32, -2.0, 3.25].iter().enumerate() {
        t.set_u32(v + 4 * i as u32, x.to_bits());
    }
    for i in [0i16, 1, 7, -1] {
        cmd(&mut t, 0x76a0, &[op(i)]);
        cmd(&mut t, 0x7050, &[op(i), v]);
        cmd(&mut t, 0x7080, &[op(i), v]);
        for frames in [1u32, 3, 0, 0xffff_fff0] {
            cmd(&mut t, 0x7260, &[op(i), 0x1111, 7, frames]);
            for which in [0u32, 1] {
                cmd(&mut t, 0x7150, &[op(i), which, 0x2222, 9, frames]);
                cmd(&mut t, 0x71d0, &[op(i), which, 0x3333, 0xfffe, frames]);
            }
        }
    }
}

#[test]
fn flag_and_check_commands() {
    let Some((mut t, _)) = twin() else { return };
    let o = SCRATCH + 0x2400;
    for v in [0i16, 1, 2, -1] {
        t.set_u16(o + 0x124, 0x8421);
        cmd(&mut t, 0x6ed0, &[op(v), o]);
    }
    for answer in [0u32, 1, 0x100] {
        t.set_u32(SCRATCH + 0x3000, answer);
        cmd(&mut t, 0x6e40, &[op(0)]);
    }
}

#[test]
fn actor_commands() {
    let Some((mut t, g)) = twin() else { return };
    for battle in [0u32, 1] {
        t.set_u32(g.battle, battle);
        for i in WHO {
            cmd(&mut t, 0x75e0, &[op(i), 0x4444]);
            cmd(&mut t, 0x7530, &[op(i), 0x5555, 0x6666]);
            cmd(&mut t, 0x74e0, &[op(i), 0x7777, 0x8001_7ffe]);
            cmd(&mut t, 0x7700, &[op(i)]);
            cmd(&mut t, 0x7740, &[op(i)]);
            for b in [0u32, 1] {
                cmd(&mut t, 0x7470, &[op(i), b]);
                cmd(&mut t, 0x6f00, &[op(i), b]);
            }
            for x in [0u32, 3, 0xffff_fffd, 0x7fff_ffff] {
                cmd(&mut t, 0x73d0, &[op(i), x, 0x9999]);
            }
        }
    }
}

#[test]
fn release_from_the_owner() {
    let Some((mut t, _)) = twin() else { return };
    let f = t.at(0x6e80);
    let owner_g = t.img.word(f + 0x1e);
    t.set_u32(owner_g, SCRATCH + 0x2800);
    let owner = node(4);
    let list = owner + 0x30;
    let n = |i: u32| SCRATCH + 0x2c00 + 12 * i;
    let item = SCRATCH + 0x2e00;
    for present in [false, true] {
        // a pool-A list of two nodes; the second holds the item or not
        t.write(owner, &[0x11; 0x60]);
        t.set_u32(list, n(0));
        t.set_u32(list + 4, n(1));
        t.set_u32(list + 8, n(0));
        t.set_u32(list + 0xc, 2);
        t.set_u32(n(0), 0);
        t.set_u32(n(0) + 4, n(1));
        t.set_u32(n(0) + 8, 0x0123);
        t.set_u32(n(1), n(0));
        t.set_u32(n(1) + 4, 0);
        t.set_u32(n(1) + 8, if present { item } else { 0x0456 });
        t.set_u16(item + 0x124, 0xffff);
        cmd(&mut t, 0x6e80, &[op(0), owner, item]);
    }
}

#[test]
fn internal_actor_helpers() {
    let Some((mut t, g)) = twin() else { return };
    // each_actor: slot 288 writes the count and returns the list
    rec(&mut t, 288, 2, |m, a| {
        let n = m.r32(SCRATCH + 0x300c);
        m.w32(a[1], n);
        ACTORS
    });
    let f = t.at(0x63d0);
    for (party, count) in [(0u32, 0u32), (1, 3), (0, 5), (1, 0xffff_ffff)] {
        t.set_u32(SCRATCH + 0x3004, party);
        t.set_u32(SCRATCH + 0x300c, count);
        t.check(
            f,
            Call::this(0x00ec_0000),
            &[],
            Eax::Checked,
            |gm, img, fr| {
                ffx_magic::Ret::eax(ffx_magic::sa_script::each_actor(
                    gm, img, fr.at, fr.ecx, fr.esp,
                ))
            },
        );
    }
    let f = t.at(0x62b0);
    for (battle, count, show) in [(0u32, 5u32, 0u32), (1, 5, 1), (0, 0, 1), (0, 3, 7)] {
        t.set_u32(g.battle, battle);
        t.set_u32(g.count, count);
        let c = Call {
            eax: 0x00ea_0000,
            ecx: show,
            edx: 0,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, |gm, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::sa_script::show_all(
                gm, img, fr.at, fr.ecx, fr.eax,
            ))
        });
    }
    let f = t.at(0x6640);
    let out = SCRATCH + 0x2400;
    for battle in [0u32, 1] {
        t.set_u32(g.battle, battle);
        for i in [u32::MAX, 0, 1, 4] {
            t.write(out, &[0x66; 0x20]);
            let c = Call {
                eax: 0,
                ecx: i,
                edx: out,
                ..Call::default()
            };
            t.check(f, c, &[0x00a1_0000], Eax::Checked, |gm, img, fr| {
                ffx_magic::Ret::eax(ffx_magic::sa_script::actor_block(gm, img, fr))
            });
        }
    }
}
