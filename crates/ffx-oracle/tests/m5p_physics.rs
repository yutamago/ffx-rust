//! `ffx_pother_b::m5p_physics` (M5 stage 1, `PhyreEngine` list
//! `phyre-physics`) against the original: on the image alone with
//! synthetic objects, and on the snapshots (`FFX_SNAPSHOT`). Results and
//! every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! None of these functions ran in a coverage run (`initialize` and slot 2
//! were never armed: not dump functions; the unlink helpers sit behind
//! pointer fields that are 0 on the default objects the game destroys), so
//! every branch is reached with synthetic data. Callees that stay original
//! (the binds, the type registration, `operator delete`, the Bullet
//! virtual methods) are recording stubs in both oracles alike (tag, `ecx`,
//! arguments, in order), popping what the original pops.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    dead_code
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pload::m5p_registry_load_a_misc::{self as ms, bridge as msb};
use ffx_pother_b::m5p_physics::{self as ph, Step, bridge as b};

include!("m5p_render_a/twins.rs");

/// Code that stores the word (or with `byte`, the low byte) in `cells + 4`
/// at the address in `cells` unless that is 0, then jumps to `then`: a
/// callee with a side effect on the caller's data.
fn store_then(t: &mut Twins, cells: u32, byte: bool, then: u32) -> u32 {
    let mut c = vec![0xa1];
    c.extend(cells.to_le_bytes()); // mov eax, [cells]
    c.extend([0x85, 0xc0, 0x74, 0x08]); // test eax, eax; je +8
    c.extend([0x8b, 0x15]);
    c.extend((cells + 4).to_le_bytes()); // mov edx, [cells+4]
    c.extend(if byte { [0x88, 0x10] } else { [0x89, 0x10] }); // mov [eax], dl/edx
    c.push(0x68);
    c.extend(then.to_le_bytes()); // push then
    c.push(0xc3); // ret
    let x = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), x);
    x
}

/// Links `nodes` through their word at `+next` and stores the first at
/// `head` (0 for none).
fn chain(t: &mut Twins, head: u32, next: u32, nodes: &[u32]) {
    for w in nodes.windows(2) {
        t.w32(w[0] + next, w[1]);
    }
    if let Some(&last) = nodes.last() {
        t.w32(last + next, 0);
    }
    t.w32(head, nodes.first().copied().unwrap_or(0));
}

/// `0x005e7250` `PUtilityPhysics`'s scalar deleting destructor: the same
/// code as `PUtility`'s (`0x00431e10`), whose port it runs.
#[test]
fn utility_dtor_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x005e_7250);
        let real_head: u32 = t.r32(t.rebase(ms::UTILITIES));
        for case in 0..60u32 {
            t.reset();
            t.stub_sym(ms::OPERATOR_DELETE, 1, false, 1, 0, Ret::Val(0x77));
            let this = t.zeros(0x10);
            t.w32(this, rng.next());
            t.w32(this + 4, rng.next());
            let n = rng.below(4) as usize;
            let mut nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x10)).collect();
            if case % 3 != 0 {
                let at = rng.below(n as u32 + 1) as usize;
                nodes.insert(at, this);
            }
            let head = t.rebase(ms::UTILITIES);
            if case % 7 == 6 && real_head != 0 {
                // the snapshot's own utilities (this not on them)
                t.patch(head, &real_head.to_le_bytes());
            } else {
                let old: u32 = t.r32(head);
                t.patch(head, &old.to_le_bytes());
                chain(t, head, 4, &nodes);
            }
            let flags = rng.word();
            t.check(
                &format!("utility dtor {case}"),
                0x005e_7250,
                Conv::Thiscall(this),
                &[flags],
                |g| Out::U32(msb::delete_utility(g, this, flags)),
            );
        }
    });
}

/// `0x005ffe40` `PUtilityPhysics::initialize`: the start-up objects while
/// the byte stays clear (one of them sets it in some cases), then every
/// bind and declaration in order.
#[test]
fn utility_initialize_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x005f_fe40);
        let started = t.rebase(ph::STARTED);
        let cells = t.zeros(8);
        for case in 0..24u32 {
            t.reset();
            t.w32(cells, 0);
            let mut tag = 0x100;
            for &step in ph::INIT_STEPS {
                if let Step::Call(f) = step {
                    tag += 1;
                    let v = rng.next();
                    t.stub_sym(f, tag, false, 0, 0, Ret::Val(v));
                }
            }
            let v = rng.next();
            t.stub_sym(ph::REGISTER_TYPE, 1, false, 1, 0, Ret::Val(v));
            t.stub_sym(ph::ADD_TO_NAMESPACE, 2, true, 0, 0, Ret::Val(v ^ 1));
            // the start-up objects: a vtable whose slot 1 records (the
            // `k`th sets the byte when the case says so)
            let setter_at = rng.below(3);
            for (k, &obj) in ph::STARTUP_OBJECTS.iter().enumerate() {
                let rec = t.stub_code(0x10 + k as u32, true, 0, 0, Ret::Val(rng.next()), false);
                let slot1 = if case % 4 == 1 && k as u32 == setter_at {
                    store_then(t, cells, true, rec)
                } else {
                    rec
                };
                let vt = t.words(&[0, slot1]);
                t.global(obj, vt);
            }
            if case % 4 == 1 {
                t.w32(cells, started);
                t.w32(cells + 4, 1 + rng.below(0xff));
            }
            let s = u8::from(case % 4 == 2) * (1 + rng.below(0xff) as u8);
            t.patch(started, &[s]);
            t.check(
                &format!("physics initialize {case}"),
                0x005f_fe40,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b::utility_initialize(g, ph::INIT_STEPS)),
            );
            let calls = t.calls();
            assert!(calls >= 40, "initialize {case}: {calls} calls");
        }
    });
}

/// `0x006016c0` (world, controller): the controller on the world's list or
/// not, at each place; with and without a dynamics world, which the first
/// removal can replace (the original reads it again).
#[test]
fn world_remove_controller_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0060_16c0);
        let cells = t.zeros(8);
        for case in 0..80u32 {
            t.reset();
            t.w32(cells, 0);
            let world = t.zeros(0x60);
            let ctrl = t.zeros(0x120);
            let n = rng.below(4) as usize;
            let mut nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x120)).collect();
            if case % 4 != 0 {
                let at = rng.below(n as u32 + 1) as usize;
                nodes.insert(at, ctrl);
            }
            chain(t, world + 0x4c, 0, &nodes);
            t.w32(ctrl + 0x110, rng.word());
            t.w32(ctrl + 0x114, rng.word());
            let remove_action = t.stub_code(1, true, 1, 1, Ret::Val(rng.next()), false);
            let remove_object = t.stub_code(2, true, 1, 1, Ret::Val(rng.next()), false);
            let mut vt = vec![0u32; 16];
            vt[15] = remove_action;
            vt[8] = remove_object;
            let other = t.object(&vt, 0x10);
            if case % 3 == 1 {
                // removeAction replaces the controller's dynamics world
                vt[15] = store_then(t, cells, false, remove_action);
                t.w32(cells, ctrl + 0x10c);
                t.w32(cells + 4, other);
            }
            let dynamics = t.object(&vt, 0x10);
            if case % 5 != 4 {
                t.w32(ctrl + 0x10c, dynamics);
            }
            t.check(
                &format!("remove controller {case}"),
                0x0060_16c0,
                Conv::Thiscall(world),
                &[ctrl],
                |g| Out::U32(b::world_remove_controller(g, world, ctrl)),
            );
        }
    });
}

/// `0x006018b0` (world, model): the model without a world, off the list,
/// on it; its bodies on the world's body list or not, owned by this model,
/// another (whose world shares the dynamics world or not) or none, with
/// and without a Bullet body.
#[test]
fn world_remove_model_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0060_18b0);
        for case in 0..120u32 {
            t.reset();
            let remove_body = t.stub_code(1, true, 1, 1, Ret::Val(rng.next()), false);
            let mut vt = vec![0u32; 21];
            vt[20] = remove_body;
            let dynamics = t.object(&vt, 0x10);
            let world = t.zeros(0x60);
            if case % 6 != 5 {
                t.w32(world + 0x54, dynamics);
            }
            // another world: the same dynamics world or a different one
            let world2 = t.zeros(0x60);
            let d2 = if rng.chance(2) { dynamics } else { rng.word() };
            t.w32(world2 + 0x54, d2);
            let model = t.zeros(0x10);
            let model2 = t.zeros(0x10);
            t.w32(model2 + 8, if rng.chance(3) { 0 } else { world2 });
            if case % 8 != 7 {
                t.w32(model + 8, if rng.chance(2) { world } else { world2 });
            }
            let n = rng.below(3) as usize;
            let mut models: Vec<u32> = (0..n).map(|_| t.zeros(0x10)).collect();
            if case % 5 != 0 {
                let at = rng.below(n as u32 + 1) as usize;
                models.insert(at, model);
            }
            chain(t, world + 0x48, 0, &models);
            let k = rng.below(4) as usize;
            let bodies: Vec<u32> = (0..k).map(|_| t.zeros(0x100)).collect();
            chain(t, model + 4, 0, &bodies);
            let mut listed: Vec<u32> = (0..rng.below(3)).map(|_| t.zeros(0x100)).collect();
            for &body in &bodies {
                let owner = match rng.below(4) {
                    0 => 0,
                    1 => model2,
                    _ => model,
                };
                t.w32(body + 0xb0, owner);
                t.w32(body + 0xdc, if rng.chance(4) { 0 } else { rng.next() | 1 });
                if !rng.chance(3) {
                    let at = rng.below(listed.len() as u32 + 1) as usize;
                    listed.insert(at, body);
                }
            }
            chain(t, world + 0x50, 0xb8, &listed);
            t.check(
                &format!("remove model {case}"),
                0x0060_18b0,
                Conv::Thiscall(world),
                &[model],
                |g| Out::U32(b::world_remove_model(g, world, model)),
            );
        }
    });
}

/// `0x00601a20` (body, &head): on the list (at each place) or not.
#[test]
fn unlink_world_body_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0060_1a20);
        for case in 0..60u32 {
            t.reset();
            let head = t.zeros(4);
            let body = t.zeros(0x100);
            t.w32(body + 0xb8, rng.word());
            let n = rng.below(4) as usize;
            let mut nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x100)).collect();
            if case % 3 != 0 {
                let at = rng.below(n as u32 + 1) as usize;
                nodes.insert(at, body);
            }
            chain(t, head, 0xb8, &nodes);
            t.check(
                &format!("unlink world body {case}"),
                0x0060_1a20,
                Conv::Thiscall(body),
                &[head],
                |g| Out::U32(b::unlink_world_body(g, body, head)),
            );
        }
    });
}

/// `0x00601a60` (model, body): on the model's list (at each place) or not.
#[test]
fn model_remove_body_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0060_1a60);
        for case in 0..60u32 {
            t.reset();
            let model = t.zeros(0x10);
            let body = t.zeros(0x100);
            t.w32(body + 0xb0, rng.word());
            t.w32(body, rng.word());
            let n = rng.below(4) as usize;
            let mut nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x100)).collect();
            if case % 3 != 0 {
                let at = rng.below(n as u32 + 1) as usize;
                nodes.insert(at, body);
            }
            chain(t, model + 4, 0, &nodes);
            t.check(
                &format!("model remove body {case}"),
                0x0060_1a60,
                Conv::Thiscall(model),
                &[body],
                |g| Out::U32(b::model_remove_body(g, model, body)),
            );
        }
    });
}

/// `0x00605720` `PUtilityPhysics` slot 2 (host only): `eax = 0`.
#[test]
fn utility_slot2_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0060_5720);
        for case in 0..4u32 {
            let this = rng.next();
            t.check(
                &format!("slot 2 {case}"),
                0x0060_5720,
                Conv::Thiscall(this),
                &[],
                |_| Out::U32(ffx_pcore::m5p_stubs::bridge::zero()),
            );
        }
    });
}
