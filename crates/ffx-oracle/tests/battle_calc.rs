//! `MsCalcCommand`, its target step and `MsActTargetSearch`
//! (`ffx_battle::stat`), and the command execution's pieces
//! (`ffx_battle::exec`) against the original, as in `battle.rs`: the
//! shared battle world with random actors, commands and cue entries, on the
//! image alone and on both snapshots. `TOGetFFXLang` and the achievement
//! unlock (main's) are the same stubs in both; the trigonometric and square
//! root imports the same stand-ins (`battle_stat.rs`); the CD library's
//! slots the same recording stubs.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::crt::{patch, stand_in};
use battle_common::world::{ACHIEVEMENT, ACTORS, TO_GET_FFX_LANG, World, world};
use battle_common::*;
use ffx_battle::stat::bridge as s;
use ffx_oracle::{Conv, Oracle};

fn lang(m: &mut Oracle) -> u32 {
    let at = m.rebase(TO_GET_FFX_LANG);
    m.call(at, Conv::Cdecl, &[]).unwrap().eax
}

fn achievement(m: &mut Oracle, n: u32) {
    let at = m.rebase(ACHIEVEMENT);
    m.call(at, Conv::Cdecl, &[n]).unwrap();
}

/// What targeting reads beyond the world's shuffle: the cursor and reach
/// bytes, reflect, the action buffers' free marks, spoils (synthetic).
fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, spoils: u32) {
    for i in 0..ACTORS {
        let p = w.actor(i);
        for (at, len) in [
            (0x4ff, 1),
            (0x6d3, 2),
            (0xdd9, 1),
            (0x62c, 1),
            (0x60d, 1),
            (0x6c0, 1),
        ] {
            fill(a, b, rng, p + at, len);
        }
        if rng.below(2) == 0 {
            poke(a, b, p + 0x60d, &[0]);
            poke(a, b, p + 0x62c, &[0]);
        }
        for o in [0x776, 0xa4e] {
            let v = if rng.below(3) == 0 { rng.byte() } else { 0xff };
            poke(a, b, p + o, &[v]);
        }
        // groups (MsGetRandomGroupTarget)
        poke(a, b, p + 0x504, &[rng.byte()]);
        if w.synthetic {
            // some formulas divide by the maxima (a fault on 0)
            for at in [0x594, 0x598] {
                if u32::from_le_bytes(b.read(p + at, 4).unwrap().try_into().unwrap()) == 0 {
                    poke(a, b, p + at, &1u32.to_le_bytes());
                }
            }
            let sp = if rng.below(2) == 0 { 0 } else { spoils };
            poke(a, b, p + 0xf88, &sp.to_le_bytes());
        }
    }
    if w.synthetic {
        fill(a, b, rng, spoils, 0x120);
    }
    // a guard now and then (any actor, Tidus often): able, status 2 0x1000
    // or 0x2000, nothing in the way of an automatic command
    for _ in 0..rng.below(3) {
        let i = if rng.below(2) == 0 {
            0
        } else {
            rng.below(ACTORS)
        };
        let p = w.actor(i);
        poke(a, b, p + 0xdc8, &[1]);
        poke(a, b, p + 0xdcc, &[0, 0, 0]);
        poke(a, b, p + 0x606, &[0, 0, 0]);
        poke(a, b, p + 0x617, &[[0x10, 0x20][rng.below(2) as usize]]);
    }
    set(
        a,
        b,
        ffx_battle::save::bridge::STEAL_RULE,
        u8::try_from(rng.below(3)).unwrap(),
    );
}

fn id(rng: &mut Xorshift) -> u16 {
    let v = match rng.below(4) {
        0 => 0x2000 + rng.below(0x40),
        1 => 0xff,
        _ => 0x3000 + rng.below(0x40),
    };
    u16::try_from(v).unwrap()
}

fn selector(rng: &mut Xorshift) -> u32 {
    match rng.below(4) {
        0 => rng.range(-26, 0).cast_unsigned(),
        1 => rng.below(0x24),
        _ => rng.next() & 0x7fff_ffff,
    }
}

fn calls(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, cue: u32) {
    let out = w.out;
    // the search
    let chr = rng.below(0x24);
    let sel = selector(rng);
    let kind = [0x5c, 0, 1, 9, 0x16, 0x1c, 0x1e, 0x50, 0xfb, 0x11b, 0x159][rng.below(11) as usize];
    let which = rng.below(0x44);
    let mode = [0, 1, 2, 3, 0x80, 0x81, 0x82][rng.below(7) as usize];
    let resolved = rng.below(2);
    let pick = if rng.below(4) == 0 { 0 } else { out + 0x80 };
    let args = [chr, sel, kind, rng.next(), which, mode, resolved, pick];
    twin("MsActTargetSearch", a, b, 0x007a_cd40, &args, ALL, |m| {
        s::ms_act_target_search(m, &stand_in, chr, sel, kind, which, mode, resolved, pick)
    });
    same("MsActTargetSearch", a, b);
    // command data, equipment names, default targets
    if w.synthetic {
        // equipment of names the fake table has
        let mut slots = Vec::new();
        slots.push((ffx_battle::save::bridge::EQUIPMENT.ptr(b).addr, 200));
        for g in [
            ffx_battle::save::bridge::SPOILS_EQUIPMENT,
            ffx_battle::save::bridge::EQUIPMENT_7000,
        ] {
            slots.push((g.ptr(b).addr, 8));
        }
        for (at, n) in slots {
            fill(a, b, rng, at, n * 0x16);
            for i in 0..n {
                let id = u16::try_from(rng.below(0x40)).unwrap();
                poke(a, b, at + i * 0x16, &id.to_le_bytes());
            }
        }
        let menu = rng.next();
        set(a, b, ffx_battle::save::bridge::MENU, menu);
    }
    let data = (rng.below(0xe) << 12) | [rng.below(0x40), rng.below(0x1000)][rng.below(2) as usize];
    let data = if rng.below(10) == 0 { 0xff } else { data };
    let name = if rng.below(3) == 0 { 0 } else { out + 0x90 };
    twin("MsGetComData", a, b, 0x0079_a4b0, &[data, name], ALL, |m| {
        s::ms_get_com_data(m, data, name)
    });
    same("MsGetComData", a, b);
    // a field of the record (round 28): every selector and beyond, an
    // actor whose +0x5c1/+0x5d9 the 0x40000 commands read
    let i = rng.below(0x1f);
    let what = [rng.below(11), rng.below(11), 11, rng.next()][rng.below(4) as usize];
    twin(
        "MsGetComInfo",
        a,
        b,
        0x0078_cd30,
        &[i, data, what],
        ALL,
        |m| s::ms_get_com_info(m, i, data, what),
    );
    same("MsGetComInfo", a, b);
    // a record at address 0 (an item table whose data offset wraps the
    // record round to 0): no record
    if rng.below(4) == 0 {
        let table = out + 0xc0;
        let mut t = [0u8; 0x14];
        t[..2].copy_from_slice(&1u16.to_le_bytes());
        t[0xa..0xc].copy_from_slice(&0xfffu16.to_le_bytes());
        t[0x10..].copy_from_slice(&table.wrapping_neg().to_le_bytes());
        poke(a, b, table, &t);
        let items = ffx_kernel::bridge::ITEM;
        let kept = items.get(b);
        set(a, b, items, table);
        let id = 0x2000 | rng.below(0x1000);
        twin(
            "MsGetComInfo",
            a,
            b,
            0x0078_cd30,
            &[i, id, what],
            ALL,
            |m| s::ms_get_com_info(m, i, id, what),
        );
        set(a, b, items, kept);
        same("MsGetComInfo", a, b);
    }
    twin(
        "MsGetSaveWeapon",
        a,
        b,
        0x007a_bbd0,
        &[data, name],
        ALL,
        |m| ffx_battle::save::bridge::ms_get_save_weapon(m, data, name),
    );
    same("MsGetSaveWeapon", a, b);
    twin(
        "MsGetSaveWeaponName",
        a,
        b,
        0x007a_bdf0,
        &[data],
        ALL,
        |m| ffx_battle::save::bridge::ms_get_save_weapon_name(m, data),
    );
    same("MsGetComData", a, b);
    let bits = rng.next() & 0x7fff_ffff;
    let bits = [bits, bits & rng.next(), 0, 1 << rng.below(0x1f)][rng.below(4) as usize];
    twin(
        "MsGetRandomGroupTarget",
        a,
        b,
        0x0079_9910,
        &[bits],
        ALL,
        |m| s::ms_get_random_group_target(m, bits),
    );
    same("MsGetRandomGroupTarget", a, b);
    let chr = rng.below(0x24);
    let cid = u32::from(id(rng));
    let side = rng.range(-2, 3);
    let args = [chr, cid, side.cast_unsigned(), rng.next()];
    twin("MsGetCommandTarget", a, b, 0x0079_96d0, &args, ALL, |m| {
        s::ms_get_command_target(m, &stand_in, chr, cid, side, args[3])
    });
    same("MsGetCommandTarget", a, b);
    // the target step
    let user = rng.below(0x1f);
    let cmd = w.command(a, b, rng);
    // hit counts the game has: more than 16 hits overrun a target's
    // results block (0x2d8 bytes) into the actor's spoils pointer
    poke(a, b, cmd + 0x2b, &[u8::try_from(rng.below(5)).unwrap()]);
    let event = u32::from(id(rng));
    poke(a, b, out + 0x40, &(rng.next() & 0x7fff_ffff).to_le_bytes());
    poke(a, b, out + 0x44, &rng.below(100).to_le_bytes());
    let args = [user, out, out + 0x20, cmd, event, out + 0x40, out + 0x44];
    twin("MsCalcCommandTarget", a, b, 0x0078_ba70, &args, ALL, |m| {
        s::ms_calc_command_target(
            m,
            &stand_in,
            user,
            out,
            out + 0x20,
            cmd,
            event,
            out + 0x40,
            out + 0x44,
        )
    });
    same("MsCalcCommandTarget", a, b);
    // a whole command
    let n = rng.below(3);
    let mut c = vec![0u8; 0x40];
    c[0] = u8::try_from(rng.below(0x1f)).unwrap();
    c[3] = u8::try_from(n).unwrap();
    for k in 0..2usize {
        let e = k * 16;
        c[e + 8..e + 10].copy_from_slice(&id(rng).to_le_bytes());
        c[e + 10..e + 12].copy_from_slice(&id(rng).to_le_bytes());
        // up to three targets: hits beyond 16 on one target (guards and
        // reflection gather them) overrun its results block
        let bits = match rng.below(3) {
            0 => 1 << rng.below(0x1f),
            1 => 0,
            _ => (1 << rng.below(0x1f)) | (1 << rng.below(0x1f)) | (1 << rng.below(0x1f)),
        };
        c[e + 0x10..e + 0x14].copy_from_slice(&u32::to_le_bytes(bits));
    }
    poke(a, b, cue, &c);
    let k = rng.below(3);
    let cue_at = if rng.below(10) == 0 { 0 } else { cue };
    twin("MsCalcCommand", a, b, 0x0078_92e0, &[cue_at, k], ALL, |m| {
        s::ms_calc_command(m, &stand_in, cue_at, k, lang, achievement)
    });
    same("MsCalcCommand", a, b);
}

/// `0x007ceb30 + 0x340` (unreachable debug code, as
/// `battle_common::STUBS`): where the CD library stubs go in both oracles
/// (the heap is not executable).
const CD_STUBS: u32 = 0x007c_ee70;

/// The CD library's slots 4, 5 and 0 (the read queue) as stubs in one
/// oracle, recording their arguments (and the read block's `+0xc`) at
/// `rec`.
fn cd_stubs(o: &mut Oracle, rec: u32) {
    let at = o.rebase(CD_STUBS);
    let store = |code: &mut Vec<u8>, arg: u8, to: u32| {
        code.extend([0x8b, 0x44, 0x24, arg]);
        code.push(0xa3);
        code.extend(to.to_le_bytes());
    };
    let mut code = Vec::new();
    store(&mut code, 4, rec);
    code.push(0xc3);
    code.resize(0x10, 0xcc);
    store(&mut code, 4, rec + 4);
    code.push(0xc3);
    code.resize(0x20, 0xcc);
    store(&mut code, 4, rec + 8);
    store(&mut code, 8, rec + 0xc);
    store(&mut code, 0xc, rec + 0x10);
    code.extend([0x8b, 0x44, 0x24, 0x10, 0x8b, 0x40, 0x0c]);
    code.push(0xa3);
    code.extend((rec + 0x14).to_le_bytes());
    code.push(0xc3);
    o.write(at, &code).unwrap();
    let lib = ffx_battle::exec::bridge::CD_LIB.ptr(&*o).addr;
    for (slot, off) in [(0, 0x20), (4, 0), (5, 0x10)] {
        o.write(lib + 4 * slot, &(at + off).to_le_bytes()).unwrap();
    }
}

/// A cue of `+3` entries (0-3), first entry mostly (`+1`, `+2` 0), each
/// with a command.
fn exec_cue(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, at: u32) {
    fill(a, b, rng, at, 0x48);
    if rng.below(4) != 0 {
        poke(a, b, at + 1, &[0, 0]);
    }
    let n = u8::try_from(rng.below(4)).unwrap();
    poke(
        a,
        b,
        at + 2,
        &[u8::try_from(rng.below(u32::from(n.max(1)))).unwrap(), n],
    );
    for k in 0..4 {
        for o in [8, 10] {
            let v = match rng.below(8) {
                0 => 0x3028u16,
                1 => 0x3029,
                2 => 0x311e,
                _ => id(rng),
            };
            // a first ID the kernel has (`MsSetChrMimic` reads the record
            // `MsGetCommand` finds, null for none)
            let v = if o == 8 && v == 0xff { 0x3000 } else { v };
            poke(a, b, at + k * 0x10 + o, &v.to_le_bytes());
        }
    }
}

/// The calls the CD and message window paths make, as the originals in
/// the Rust's oracle; the read block in `block`.
fn exec_call(block: u32) -> impl FnMut(&mut Oracle, u32, &[u32]) -> u32 {
    move |m, f, args| {
        if let [file, dest, done, i] = *args {
            m.write(block + 0xc, &i.to_le_bytes()).unwrap();
            let r = m
                .call(f, Conv::Cdecl, &[file, dest, done, block])
                .unwrap()
                .eax;
            m.write(block + 0xc, &[0; 4]).unwrap();
            r
        } else {
            m.call(f, Conv::Cdecl, args).unwrap().eax
        }
    }
}

/// `ffx_battle::exec`: gil, Mimic, Mix, the system message gate, the
/// action request and the small checks around them.
fn exec(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, cue: u32, block: u32) {
    use ffx_battle::exec::bridge as e;
    use ffx_battle::save::bridge::GIL;
    use ffx_battle::state::bridge as st;
    // gil
    let gil = match rng.below(4) {
        0 => rng.next(),
        1 => 999_999_998 + rng.below(3),
        _ => rng.below(100_000),
    };
    set(a, b, GIL, gil);
    let n = match rng.below(4) {
        0 => rng.next(),
        1 => gil.wrapping_sub(1).wrapping_add(rng.below(3)),
        _ => rng.below(100_000),
    };
    twin("MsPayGIL", a, b, 0x0078_59a0, &[n], ALL, |m| {
        e::ms_pay_gil(m, n.cast_signed()).cast_unsigned()
    });
    same("MsPayGIL", a, b);
    twin("MsSetGIL", a, b, 0x0078_5c20, &[n], 0, |m| {
        e::ms_set_gil(m, n.cast_signed());
        0
    });
    same("MsSetGIL", a, b);
    // actors
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    for at in [0x4b6, 0x40a, 0x40b, 0x714, 0x715, 0x5bc, 0x5bd, 0x6e1] {
        let v = if rng.below(3) == 0 { 0 } else { rng.byte() };
        poke(a, b, actor + at, &[v]);
    }
    fill(a, b, rng, actor + 0x726, 4);
    twin(
        "MsCheckChrHold",
        a,
        b,
        0x0079_1170,
        &[i, actor, cue],
        ALL,
        |m| e::ms_check_chr_hold(m, actor),
    );
    let bits = rng.next() & 0x7fff_ffff;
    let bits = [bits, bits & rng.next(), 0, 1 << rng.below(0x1f)][rng.below(4) as usize];
    twin("MsCountChrReady", a, b, 0x0079_2880, &[bits], ALL, |m| {
        e::ms_count_chr_ready(m, bits)
    });
    let j = rng.below(0x60);
    twin(
        "MsCheckSameSide",
        a,
        b,
        0x0079_95c0,
        &[j, bits],
        ALL,
        |_| e::ms_check_same_side(j, bits),
    );
    let cmd = w.command(a, b, rng);
    // one actor able, KO'd or not
    let k = rng.below(ACTORS);
    poke(a, b, w.actor(k) + 0xdc8, &[1]);
    poke(a, b, w.actor(k) + 0xdd9, &[1]);
    poke(a, b, w.actor(k) + 0xdcc, &[u8::from(rng.below(2) == 0)]);
    let bits = if rng.below(3) == 0 { 1 << k } else { bits };
    twin(
        "MsCheckTargetsGone",
        a,
        b,
        0x0079_9610,
        &[bits, cmd],
        ALL,
        |m| e::ms_check_targets_gone(m, bits, cmd),
    );
    let kind = rng.below(8);
    twin(
        "MsCommandKindMode",
        a,
        b,
        0x007a_e120,
        &[kind, cmd],
        ALL,
        |m| e::ms_command_kind_mode(m, kind, cmd),
    );
    twin("MsAddChrGain", a, b, 0x007a_fb60, &[i, actor], 0, |m| {
        e::ms_add_chr_gain(m, actor);
        0
    });
    same("MsAddChrGain", a, b);
    // the defense positions
    set(
        a,
        b,
        ffx_battle::stat::bridge::GUARD_TOOK,
        u8::from(rng.below(2) == 0),
    );
    for k in 0..ACTORS {
        let v = if rng.below(3) == 0 { 0xff } else { rng.byte() };
        poke(a, b, w.actor(k) + 0x6d4, &[v]);
    }
    twin("MsDefensePosOffAll", a, b, 0x007a_f800, &[], 0, |m| {
        e::ms_defense_pos_off_all(m);
        0
    });
    same("MsDefensePosOffAll", a, b);
    // the action request
    let act = if rng.below(2) == 0 {
        -1
    } else {
        rng.range(-3, 5)
    };
    set(a, b, e::ACTION_REQUEST, rng.next());
    set(a, b, e::ACTION_REQUEST_STEP, rng.next());
    set(a, b, ffx_battle::misc::bridge::ACTION, act);
    let k = rng.next();
    twin("MsActionRequestSet", a, b, 0x007a_cb00, &[k], ALL, |m| {
        e::ms_action_request_set(m, k.to_le_bytes()[0]).cast_unsigned()
    });
    same("MsActionRequestSet", a, b);
    // command IDs
    let ids = if rng.below(5) == 0 { 0 } else { w.out };
    for o in [0, 2] {
        let v: u16 = if rng.below(3) == 0 { 0x3029 } else { id(rng) };
        poke(a, b, w.out + o, &v.to_le_bytes());
    }
    twin("MsCheckCommand3029", a, b, 0x007a_e0f0, &[ids], ALL, |m| {
        e::ms_check_command_3029(m, ids)
    });
    // a command on the script's targets, into random cues
    set(
        a,
        b,
        st::INPUT_CUE_COUNT,
        i8::try_from(rng.range(-1, 0x40)).unwrap(),
    );
    set(
        a,
        b,
        st::COMMAND_CUE_COUNT,
        i8::try_from(rng.range(-1, 0x30)).unwrap(),
    );
    fill(a, b, rng, st::INPUT_CUE.ptr(b).addr, 0x40 * 8);
    fill(a, b, rng, st::COMMAND_CUE.ptr(b).addr, 0x30 * st::CUE_SIZE);
    set(a, b, e::SCRIPT_TARGETS, rng.next());
    let after = rng.below(ACTORS + 2);
    let cid = rng.next();
    twin(
        "MsInsertScriptCommand",
        a,
        b,
        0x007a_e2e0,
        &[i, after, cid],
        ALL,
        |m| e::ms_insert_script_command(m, i, after, u16::try_from(cid & 0xffff).unwrap()),
    );
    same("MsInsertScriptCommand", a, b);
    // the system message gate: the window functions (system's) stubbed
    // alike in both, with random states
    let (st7, win7) = (rng.below(5), rng.below(4));
    for o in [&mut *a, &mut *b] {
        for (f, v) in [(0x008b_7ca0, st7), (0x0090_6c20, win7), (0x008a_b8f0, 0)] {
            let at = o.rebase(f);
            o.stub_return(at, v).unwrap();
        }
    }
    twin("MsSystemMessageCheck", a, b, 0x0079_d050, &[], ALL, |m| {
        e::ms_system_message_check(m, &mut exec_call(block)).cast_unsigned()
    });
    same("MsSystemMessageCheck", a, b);
    // Mix
    exec_cue(a, b, rng, cue);
    twin(
        "MsCheckRikkuLimit",
        a,
        b,
        0x007a_fbd0,
        &[i, actor, cue],
        ALL,
        |m| e::ms_check_rikku_limit(m, &mut exec_call(block), i, actor, cue),
    );
    same("MsCheckRikkuLimit", a, b);
    // Mimic: the saved cue, the gil it threw
    exec_cue(a, b, rng, e::MIMIC.ptr(b).addr);
    exec_cue(a, b, rng, cue);
    set(a, b, e::MIMIC_GIL, rng.range(-5, 200_000));
    set(
        a,
        b,
        ffx_battle::damage::bridge::GIL_AMOUNT,
        rng.range(-5, 200_000),
    );
    let i = if rng.below(4) == 0 {
        rng.below(ACTORS)
    } else {
        rng.below(8)
    };
    let actor = w.actor(i);
    twin(
        "MsSetChrMimic",
        a,
        b,
        0x0079_9b40,
        &[i, actor, cue],
        ALL,
        |m| e::ms_set_chr_mimic(m, &stand_in, i, actor, cue),
    );
    same("MsSetChrMimic", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    patch(a);
    patch(b);
    let w = world(a, b, rng, synthetic);
    for o in [&mut *a, &mut *b] {
        let at = o.rebase(ACHIEVEMENT);
        o.stub_return(at, 0).unwrap();
        if w.synthetic {
            let at = o.rebase(TO_GET_FFX_LANG);
            o.stub_return(at, 1).unwrap();
        }
    }
    if w.synthetic {
        // the fake kernel records' hit counts as the game's (0-4)
        for g in [ffx_kernel::bridge::COMMAND, ffx_kernel::bridge::ITEM] {
            let t = g.get(b);
            for i in 0..0x40 {
                poke(
                    a,
                    b,
                    t + 0x20 + i * 0x60 + 0x2b,
                    &[u8::try_from(rng.below(5)).unwrap()],
                );
            }
        }
    }
    // kernel tables where there are none
    for (g, stride) in [
        (ffx_battle::menu::bridge::WEAPON_NAMES, 0x60),
        (ffx_battle::minor::bridge::IMPORTANT_ROM, 0x10),
        (ffx_kernel::bridge::A_ABILITY, 0x40),
    ] {
        if w.synthetic || !readable(a, g.get(b), 0x20) {
            let t = battle_common::world::fake_records(a, b, rng, 0x40, stride);
            set(a, b, g, t);
        }
    }
    let spoils = scratch(a, b, 0x120);
    let cue = scratch(a, b, 0x40);
    let ecue = scratch(a, b, 0x50);
    let block = scratch(a, b, 0x20);
    let rec = scratch(a, b, 0x20);
    cd_stubs(a, rec);
    cd_stubs(b, rec);
    for round in 0..1500 {
        w.shuffle(a, b, rng, round);
        dress(a, b, rng, &w, spoils);
        calls(a, b, rng, &w, cue);
        exec(a, b, rng, &w, ecue, block);
    }
}

#[test]
fn commands_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0xca1c_0001), true);
}

#[test]
fn commands_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0xca1c_0002), false);
}
