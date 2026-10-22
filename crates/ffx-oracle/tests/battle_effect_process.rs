//! `MsEffectProcess` (`ffx_battle::minor`, battle round 36), the effects'
//! frame, against the original. `FlushCache`,
//! `graphicVFXSetCurrentVFXType`, `MagicFile_GetEffectOverlayTable` and
//! the overlay tables' functions are logging stand-ins (the functions log
//! `g_VfxPass` as they see it); on the battle snapshot some rounds keep
//! the battle overlay's own table (`op_effect_run_before`/`_after`, run
//! as the originals on both sides).

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::*;
use ffx_battle::minor::bridge as mn;
use ffx_oracle::Oracle;

fn call(m: &mut Oracle, f: u32, a: &[u32]) -> u32 {
    m.call_cdecl(f, a).unwrap()
}

/// A logging stand-in at new code: appends `id`, its first argument and
/// `g_VfxPass` at the cursor `log`, returns the word at `ret`.
fn logger(a: &mut Oracle, b: &mut Oracle, log: u32, id: u32, ret: u32) -> u32 {
    let pass = a.rebase(0x0113_335c);
    let mut c = vec![0x8b, 0x0d]; // mov ecx, [log]
    c.extend(log.to_le_bytes());
    c.extend([0xc7, 0x01]); // mov dword [ecx], id
    c.extend(id.to_le_bytes());
    c.extend([0x8b, 0x44, 0x24, 0x04, 0x89, 0x41, 0x04]); // [ecx + 4] = arg 0
    c.push(0xa1); // mov eax, [pass]
    c.extend(pass.to_le_bytes());
    c.extend([0x89, 0x41, 0x08, 0x83, 0x05]); // [ecx + 8] = pass; add [log], 12
    c.extend(log.to_le_bytes());
    c.extend([0x0c, 0xa1]); // mov eax, [ret]
    c.extend(ret.to_le_bytes());
    c.push(0xc3);
    let at = a.alloc_code(&c).unwrap();
    assert_eq!(b.alloc_code(&c).unwrap(), at);
    at
}

fn jump(a: &mut Oracle, b: &mut Oracle, sym: u32, to: u32) {
    for o in [a, b] {
        let at = o.rebase(sym);
        let rel = to.wrapping_sub(at + 5);
        o.write(at, &[[0xe9].as_slice(), &rel.to_le_bytes()].concat())
            .unwrap();
    }
}

fn cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let world = battle_common::world::world(a, b, rng, synthetic);
    let own_funcs = mn::OVERLAY_FUNCS.get(b);
    let real = !synthetic && own_funcs != 0 && a.read(own_funcs, 0x14).is_ok();
    let log = scratch(a, b, 4);
    let log_ring = scratch(a, b, 0x4000);
    let magic_ret = scratch(a, b, 4);
    let zero = scratch(a, b, 4);
    let flush = logger(a, b, log, 1, zero);
    jump(a, b, 0x0062_f4c0, flush);
    let kind = logger(a, b, log, 2, zero);
    jump(a, b, mn::SET_CURRENT_VFX_TYPE, kind);
    let magic = logger(a, b, log, 3, magic_ret);
    jump(a, b, mn::MAGIC_FILE_OVERLAY_TABLE, magic);
    // tables of logging functions (ids 0x10 + 2k: +0xc, 0x11 + 2k: +0x10)
    let tables: Vec<u32> = (0..6u32)
        .map(|k| {
            let t = scratch(a, b, 0x20);
            let before = logger(a, b, log, 0x10 + 2 * k, zero);
            let after = logger(a, b, log, 0x11 + 2 * k, zero);
            poke(a, b, t + 0xc, &before.to_le_bytes());
            poke(a, b, t + 0x10, &after.to_le_bytes());
            t
        })
        .collect();
    // The overlay's own functions leave stack-dependent bytes in globals:
    // the port calls them through trampolines that give them the stack
    // depth the original's frame does (its argument, return address,
    // three saved registers).
    let mut tramps = Vec::new();
    if real {
        for slot in [0xc, 0x10] {
            let target: u32 = ffx_bridge::Mem::read(&*b, own_funcs + slot);
            let at = a.alloc_code(&[0; 16]).unwrap();
            assert_eq!(b.alloc_code(&[0; 16]).unwrap(), at);
            let mut code = vec![0x83, 0xec, 0x0c, 0x6a, 0x00, 0xe8];
            code.extend(target.wrapping_sub(at + 10).to_le_bytes());
            code.extend([0x83, 0xc4, 0x10, 0xc3]);
            poke(a, b, at, &code);
            tramps.push((target, at));
        }
    }
    let mut c = |m: &mut Oracle, f: u32, args: &[u32]| {
        let f = tramps.iter().find(|t| t.0 == f).map_or(f, |t| t.1);
        call(m, f, args)
    };
    let mut reach = [0u32; 4];
    for round in 0..120u32 {
        poke(a, b, log, &log_ring.to_le_bytes());
        let phase = [0, 1, 0, 1, 2, 0x8000_0000][rng.below(6) as usize];
        let own = real && round % 10 == 3 && phase < 2;
        let funcs = if own { own_funcs } else { tables[0] };
        set(a, b, mn::OVERLAY_FUNCS, funcs);
        // actors 4 or 5 at times, with a table (null only where the
        // phase reads none)
        for i in 0..0x1f {
            let actor = world.actor(i);
            let state = [0u8, 4, 5, 3, 6][rng.below(5) as usize];
            let table = if phase > 1 && rng.below(4) == 0 {
                0
            } else {
                tables[1 + rng.below(4) as usize]
            };
            // the overlay's own functions run alone
            poke(a, b, actor + 0xdfb, &[if own { 0 } else { state }]);
            poke(a, b, actor + 0xf8c, &table.to_le_bytes());
        }
        let stat = if own {
            0
        } else {
            [4i8, 5, 0, 3][rng.below(4) as usize]
        };
        set(a, b, ffx_battle::state::bridge::EVENT_EFFECT_STAT, stat);
        set(a, b, mn::EVENT_EFFECT_TABLE, tables[5]);
        // the magic file's table: one of the actors' or the event's
        let m = if rng.below(3) == 0 {
            0
        } else {
            tables[1 + rng.below(5) as usize]
        };
        poke(a, b, magic_ret, &m.to_le_bytes());
        poke(a, b, mn::VFX_PASS.ptr(b).addr, &rng.below(2).to_le_bytes());
        twin(
            &format!("MsEffectProcess {round} phase {phase:#x}"),
            a,
            b,
            0x0078_7e00,
            &[phase],
            0,
            |m| {
                mn::ms_effect_process(m, &mut c, phase);
                0
            },
        );
        same(&format!("MsEffectProcess {round}"), a, b);
        reach[usize::from(own)] += 1;
        reach[2] += u32::from(matches!(stat, 4 | 5));
        reach[3] += u32::from(phase > 1);
    }
    eprintln!("MsEffectProcess reach {reach:?} (fake, own table, event, other phase)");
}

#[test]
fn effect_process_matches_the_original_on_the_image() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    let mut rng = Xorshift(0x0036_7e00);
    cases(&mut a, &mut b, &mut rng, true);
}

#[test]
fn effect_process_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x0036_7e01);
    cases(&mut a, &mut b, &mut rng, false);
}
