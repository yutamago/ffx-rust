//! `ffx_prender::m5p_render_a*` (M5 stage 1, `PhyreEngine` list
//! `render-a`) against the original: on the image alone with synthetic
//! objects, and on the snapshots (`FFX_SNAPSHOT`). Results and every byte
//! written must agree. Needs `FFX_GAME_DIR`.
//!
//! None of these functions ran in a coverage run: every branch is reached
//! with synthetic data. Callees that stay original are replaced in both
//! oracles alike by recording stubs (tag, `ecx`, arguments, in order),
//! which pop what the original pops. Every copy in a table is run at its
//! own address.

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
use ffx_prender::m5p_render_a::{self as ma, bridge as a};

include!("m5p_render_a/twins.rs");

/// `Phyre_PClassDescriptor_Destruct`.
const DESCRIPTOR_DESTRUCT: u32 = 0x0043_b3e0;

/// A float worth storing: numbers of every size, zeros, infinities and
/// quiet NaNs (signalling NaNs stay out: the emulator keeps them).
fn float_bits(rng: &mut Rng) -> u32 {
    match rng.below(8) {
        0 => 0,
        1 => 0x8000_0000,
        2 => 0x7f80_0000 | (rng.next() & 0x8000_0000),
        3 => 0x7fc0_0000 | (rng.next() & 0x803f_ffff),
        4 => rng.next() & 0x807f_ffff, // denormal
        5 => (rng.below(60) as f32 - 30.0).to_bits(),
        _ => {
            let b = rng.next();
            if b & 0x7f80_0000 == 0x7f80_0000 {
                b | 0x0040_0000
            } else {
                b
            }
        }
    }
}

// ---- batch 1 ----

#[test]
fn fortype_dtors_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a001);
        for (k, &(addr, _)) in ma::FORTYPE_DTORS.iter().enumerate() {
            let this = t.zeros(0x80);
            let v = rng.next();
            t.w32(this, v);
            t.stub_sym(
                DESCRIPTOR_DESTRUCT,
                1,
                true,
                0,
                0,
                Ret::Val(k as u32 * 3 + 1),
            );
            t.check(
                &format!("{addr:#x}"),
                addr,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(a::fortype_dtor(g, this, ma::fortype_vtable(addr))),
            );
            assert_eq!(t.calls(), 1);
            t.reset();
        }
    });
}

/// An array header: owned or external, counts 0, 1, 2 or more, a block
/// or null.
fn header(t: &mut Twins, rng: &mut Rng, at: u32) {
    let count = match rng.below(4) {
        0 => 0,
        1 => 1,
        2 => 2 + rng.below(10),
        _ => rng.next() & 0x7fff_ffff,
    } | if rng.chance(3) { 0x8000_0000 } else { 0 };
    let data = if rng.chance(4) { 0 } else { t.zeros(0x10) };
    t.w32(at, count);
    t.w32(at + 4, data);
}

#[test]
fn resets_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a002);
        for case in 0..24 {
            for &(addr, off) in ma::RESET_OWNED {
                let this = t.zeros(0x40);
                header(t, &mut rng, this + off);
                t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(case + 7));
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(a::reset_owned(g, this, off)),
                );
                t.reset();
            }
            for &(addr, off) in ma::RESET_NONNULL {
                let this = t.zeros(0x40);
                header(t, &mut rng, this + off);
                t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(case + 9));
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(a::reset_owned_nonnull(g, this, off)),
                );
                t.reset();
            }
            for &(addr, check_null) in ma::RESET_SHARED {
                let this = t.zeros(0x40);
                header(t, &mut rng, this);
                t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(case + 11));
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(a::reset_shared(g, this, check_null)),
                );
                t.reset();
            }
            for &(addr, release) in ma::RESET_RELEASE {
                let this = t.zeros(0x40);
                header(t, &mut rng, this);
                t.stub_sym(release, 2, false, 2, 0, Ret::Val(case + 13));
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(a::reset_release(g, this, release)),
                );
                t.reset();
            }
        }
    });
}

#[test]
fn element_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a003);
        for case in 0..30 {
            for &addr in ma::WORD_ARRAY_ALLOCS {
                let n = rng.below(7);
                let count = if n == 0 {
                    rng.below(3)
                } else {
                    rng.below(n + 1)
                };
                let src: Vec<u32> = (0..8).map(|_| rng.next()).collect();
                let src = t.words(&src);
                let block = if rng.chance(5) { 0 } else { t.zeros(0x40) };
                if block != 0 {
                    for i in 0..16 {
                        let v = rng.next();
                        t.w32(block + 4 * i, v);
                    }
                }
                t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, Ret::Val(block));
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Cdecl,
                    &[n, src, count],
                    |g| Out::U32(a::word_array_alloc(g, n, src, count)),
                );
                t.reset();
            }
            for &(addr, words) in ma::COPY_ELEMENTS {
                let count = rng.below(5);
                let src: Vec<u32> = (0..24).map(|_| rng.next()).collect();
                let src = t.words(&src);
                let dst = if count == 1 && rng.chance(3) {
                    0
                } else {
                    t.zeros(0x80)
                };
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Cdecl,
                    &[dst, src, count],
                    |g| {
                        a::copy_elements(g, dst, src, count, words);
                        Out::None
                    },
                );
                t.reset();
            }
            for &addr in ma::ZERO_ELEMENTS20 {
                let count = rng.below(5);
                let fill: Vec<u32> = (0..24).map(|_| rng.next() | 1).collect();
                let data = if count == 1 && rng.chance(3) {
                    0
                } else {
                    t.words(&fill)
                };
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Cdecl,
                    &[data, count],
                    |g| {
                        a::zero_elements20(g, data, count);
                        Out::None
                    },
                );
                t.reset();
            }
            let _ = case;
        }
    });
}

/// A linked list of `n` nodes (next at `+link`), its head stored at the
/// global `head`; returns the nodes.
fn list(t: &mut Twins, rng: &mut Rng, head: u32, link: u32, n: u32) -> Vec<u32> {
    let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x20)).collect();
    for (i, &node) in nodes.iter().enumerate() {
        let next = nodes.get(i + 1).copied().unwrap_or(0);
        t.w32(node + link, next);
        let v = rng.next();
        t.w32(node + 0x10, v);
    }
    t.global(head, nodes.first().copied().unwrap_or(0));
    nodes
}

#[test]
fn lists_match_the_original() {
    // An item in the list (first, middle, last), not in it, an empty list.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a004);
        for case in 0..24 {
            let n = rng.below(4);
            for &addr in ma::UTILITY_DELETES {
                let nodes = list(t, &mut rng, ma::PUTILITY_LIST, 4, n);
                let this = if n > 0 && !rng.chance(4) {
                    nodes[rng.below(n) as usize]
                } else {
                    // not in the list: its own next must stay
                    let x = t.zeros(0x20);
                    let next = rng.word();
                    t.w32(x + 4, next);
                    x
                };
                let flags = rng.word();
                t.stub_sym(ma::OPERATOR_DELETE, 1, false, 1, 0, Ret::Val(case));
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Thiscall(this),
                    &[flags],
                    |g| Out::U32(a::utility_delete(g, this, flags)),
                );
                t.reset();
            }
            for &(addr, head, link) in ma::LIST_UNLINKS {
                let nodes = list(t, &mut rng, head, link, n);
                let item = if n > 0 && !rng.chance(4) {
                    nodes[rng.below(n) as usize]
                } else {
                    t.zeros(0x20)
                };
                t.check(&format!("{addr:#x}"), addr, Conv::Cdecl, &[item], |g| {
                    Out::U32(a::list_unlink(g, item, head, link))
                });
                t.reset();
            }
            for &addr in ma::CONTEXT_SWITCH_DELETES {
                let this = t.zeros(0x20);
                let size = rng.word();
                t.w32(this + 0x10, size);
                let total = rng.word();
                t.global(ma::CONTEXT_SWITCH_TOTAL, total);
                let flags = rng.word();
                t.stub_sym(ma::CONTEXT_SWITCH_UNLINK, 1, false, 1, 0, Ret::Val(0));
                t.stub_sym(ma::OPERATOR_DELETE, 2, false, 1, 0, Ret::Val(case));
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Thiscall(this),
                    &[flags],
                    |g| Out::U32(a::context_switch_delete(g, this, flags)),
                );
                t.reset();
            }
        }
    });
}

#[test]
fn small_methods_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a005);
        for case in 0..20u32 {
            for &addr in ma::TREE_PAIR_DTORS {
                let this = t.zeros(0x20);
                let seq = t.seq(&[case, case + 5]);
                t.stub_sym(ma::TREE_HALF_DTOR, 1, true, 0, 0, seq);
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(a::tree_pair_dtor(g, this)),
                );
                t.reset();
            }
            for &(addr, skip_null) in ma::SET_EXTERNAL {
                let this = t.zeros(0x10);
                header(t, &mut rng, this);
                let old = t.r32(this + 4);
                let buffer = if rng.chance(4) { old } else { rng.word() };
                let count = rng.word();
                t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(case));
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Thiscall(this),
                    &[buffer, count],
                    |g| Out::U32(a::set_external(g, this, buffer, count, skip_null)),
                );
                t.reset();
            }
            // the tree: a sentinel and up to 7 nodes in key order
            {
                let tree = t.zeros(0x10);
                let n = rng.below(8);
                let keys: Vec<u32> = (0..n).map(|i| 10 * i + 5).collect();
                let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x10)).collect();
                fn build(t: &mut Twins, nodes: &[u32], keys: &[u32], sentinel: u32) -> u32 {
                    if nodes.is_empty() {
                        return sentinel;
                    }
                    let m = nodes.len() / 2;
                    let l = build(t, &nodes[..m], &keys[..m], sentinel);
                    let r = build(t, &nodes[m + 1..], &keys[m + 1..], sentinel);
                    t.w32(nodes[m], l);
                    t.w32(nodes[m] + 4, r);
                    t.w32(nodes[m] + 0xc, keys[m]);
                    nodes[m]
                }
                let root = build(t, &nodes, &keys, tree);
                t.w32(tree + 0xc, root);
                let k = if rng.chance(2) && n > 0 {
                    keys[rng.below(n) as usize]
                } else {
                    rng.below(90)
                };
                let key = t.words(&[k]);
                t.check(
                    "Phyre_PRedBlackTree_find_004da4e0",
                    0x004d_a4e0,
                    Conv::Thiscall(tree),
                    &[key],
                    |g| Out::U32(a::tree_find(g, tree, key)),
                );
                t.reset();
            }
            let this = t.zeros(0x100);
            for i in 0..0x40 {
                let v = rng.next();
                t.w32(this + 4 * i, v);
            }
            t.check(
                "Phyre_PPair_init_004d7b60",
                0x004d_7b60,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(a::pair_init(g, this)),
            );
            let text: Vec<u8> = (0..rng.below(12))
                .map(|_| (rng.below(200) + 1) as u8)
                .chain([0])
                .collect();
            let s = t.alloc(&text);
            let sp = t.words(&[s]);
            let out = t.zeros(8);
            let len = t.zeros(8);
            t.check(
                "StringWithLength_004dec30",
                0x004d_ec30,
                Conv::Cdecl,
                &[sp, out, len],
                |g| Out::U32(a::string_with_length(g, sp, out, len)),
            );
            t.check(
                "Phyre_PIChildClassVisitor_dtor_004d6290",
                0x004d_6290,
                Conv::Thiscall(this),
                &[],
                |g| {
                    a::store_vtable(g, this, ma::CHILD_VISITOR_VTABLE);
                    Out::None
                },
            );
            t.check(
                "Phyre_PAnimation_PBufferAcceptance_dtor_00616140",
                0x0061_6140,
                Conv::Thiscall(this),
                &[],
                |g| {
                    a::store_vtable(g, this, ma::BUFFER_ACCEPTANCE_VTABLE);
                    Out::None
                },
            );
            let flags = rng.word();
            t.stub_sym(ma::OPERATOR_DELETE, 1, false, 1, 0, Ret::Val(case));
            t.check(
                "Phyre_PIChildClassVisitor_scalarDeletingDtor_004d63f0",
                0x004d_63f0,
                Conv::Thiscall(this),
                &[flags],
                |g| Out::U32(a::visitor_delete(g, this, flags)),
            );
            t.stub_sym(0x0043_3dd0, 2, true, 0, 0, Ret::Val(1));
            t.check(
                "Phyre_PSprite_PUtilitySprite_scalarDeletingDtor",
                0x0043_40a0,
                Conv::Thiscall(this),
                &[flags],
                |g| {
                    Out::U32(a::delete_with(
                        g,
                        this,
                        flags,
                        0x0043_3dd0,
                        ma::OPERATOR_DELETE,
                    ))
                },
            );
            t.stub_sym(0x0049_69f0, 3, true, 0, 0, Ret::Val(2));
            t.stub_sym(ma::PHYRE_FREE, 4, false, 1, 0, Ret::Val(3));
            t.check(
                "Phyre_PGeometry_PDynamicMesh_scalarDeletingDtor",
                0x0049_6d90,
                Conv::Thiscall(this),
                &[flags],
                |g| Out::U32(a::delete_with(g, this, flags, 0x0049_69f0, ma::PHYRE_FREE)),
            );
            t.stub_sym(ma::SPRITE_ARRAYS_RELEASE, 5, true, 0, 0, Ret::Val(case + 1));
            t.check(
                "Phyre_PSprite_PUtilitySprite_terminate",
                0x0043_5090,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(a::sprite_terminate(g, this)),
            );
            t.stub_sym(ma::CLUSTER_CLEAR, 6, true, 0, 0, Ret::Val(case + 2));
            t.check(
                "RenderTarget_release_00440ea0",
                0x0044_0ea0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(a::forward0(g, this, ma::CLUSTER_CLEAR)),
            );
            t.stub_sym(
                ma::RECURSIVE_LOCK_RELEASE,
                7,
                true,
                0,
                0,
                Ret::Val(case + 3),
            );
            t.check(
                "PRecursiveLock_releaseHeld_00463e60",
                0x0046_3e60,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(a::release_held_lock(g, this)),
            );
            t.stub_sym(0x005c_9930, 8, false, 1, 0, Ret::Val(case + 4));
            let item = rng.word();
            t.check(
                "List_unlinkForward_005c9920",
                0x005c_9920,
                Conv::Cdecl,
                &[item],
                |g| Out::U32(a::forward_cdecl1(g, 0x005c_9930, item)),
            );
            for (addr, off, target) in [
                (0x0056_e570, 0, ma::PSTRING_ASSIGN),
                (0x0056_e510, 0x14, ma::PSTRING_ASSIGN),
                (0x0054_6450, 4, ma::PSTRING_ASSIGN),
                (0x0055_b630, 0xc, ma::PMATRIX4_COPY),
            ] {
                t.stub_sym(target, 9, true, 1, 1, Ret::Val(case + 5));
                let arg = rng.word();
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Thiscall(this),
                    &[arg],
                    |g| Out::U32(a::forward1(g, this, off, target, arg)),
                );
            }
            assert!(t.calls() >= 9);
            t.reset();
        }
    });
}

#[test]
fn setters_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a006);
        for case in 0..60u32 {
            let this = t.zeros(0x100);
            for i in 0..0x40 {
                let v = rng.next();
                t.w32(this + 4 * i, v);
            }
            for (addr, off, n) in [
                (0x0049_f170, 0, 4),
                (0x0049_f240, 0x1c, 1),
                (0x0049_f250, 0x24, 1),
                (0x0049_f270, 0x20, 1),
                (0x0049_f280, 0x28, 2),
                (0x0049_f2a0, 0x30, 1),
                (0x0056_ece0, 0x18, 4),
            ] {
                let v: Vec<u32> = (0..n).map(|_| float_bits(&mut rng)).collect();
                t.check(&format!("{addr:#x}"), addr, Conv::Thiscall(this), &v, |g| {
                    a::set_floats(g, this, off, &v);
                    Out::None
                });
            }
            // y 0 (either sign) with x -0: the two orders differ in the sign
            let (x, y) = match case % 10 {
                0 => (0x8000_0000, 0),
                1 => (0x8000_0000, 0x8000_0000),
                2 => (0, 0x8000_0000),
                3 => (float_bits(&mut rng), 0),
                _ => (float_bits(&mut rng), float_bits(&mut rng)),
            };
            t.check(
                "SetAngleRange_0049f190",
                0x0049_f190,
                Conv::Thiscall(this),
                &[x, y],
                |g| {
                    a::set_angle_range(g, this, x, y);
                    Out::None
                },
            );
            let p = if case % 7 == 0 { 0 } else { t.words(&[x, y]) };
            t.check(
                "SetAngleRangeFrom_0049f1e0",
                0x0049_f1e0,
                Conv::Thiscall(this),
                &[p],
                |g| {
                    a::set_angle_range_from(g, this, p);
                    Out::None
                },
            );
            let v = rng.word();
            t.check(
                "SetWord10_0049f260",
                0x0049_f260,
                Conv::Thiscall(this),
                &[v],
                |g| Out::U32(a::set_word(g, this, 0x10, v)),
            );
            t.check(
                "SetWord0_00546440",
                0x0054_6440,
                Conv::Thiscall(this),
                &[v],
                |g| Out::U32(a::set_word(g, this, 0, v)),
            );
            let b = rng.next();
            t.check(
                "SetByte68_0055b620",
                0x0055_b620,
                Conv::Thiscall(this),
                &[b],
                |g| Out::U8(a::set_byte68(g, this, b)),
            );
            let out = t.zeros(8);
            t.check(
                "GetPair4C_0055b560",
                0x0055_b560,
                Conv::Thiscall(this),
                &[out],
                |g| Out::U32(a::get_pair4c(g, this, out)),
            );
            t.check(
                "Phyre_PAnimation_PNetworkOptimizeBufferAllocator_ctor_00616120",
                0x0061_6120,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(a::buffer_allocator_ctor(g, this)),
            );
            let block = if case % 3 == 0 { 0 } else { t.zeros(8) };
            t.w32(this + 4, block);
            t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(case + 1));
            t.check(
                "Phyre_PAnimation_PNetworkOptimizeBufferAllocator_dtor_00616150",
                0x0061_6150,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(a::buffer_allocator_dtor(g, this)),
            );
            let fill: Vec<u32> = (0..16).map(|_| rng.next() | 0x0101_0101).collect();
            let slots = t.words(&fill);
            t.w32(this + 4, slots);
            let i = rng.below(8);
            t.check(
                "Phyre_PAnimation_PNetworkOptimizeBufferAllocator_clearSlot_00616310",
                0x0061_6310,
                Conv::Thiscall(this),
                &[i],
                |g| Out::U32(a::clear_slot_flag(g, this, i)),
            );
            let frames: Vec<u32> = (0..8).map(|_| rng.word()).collect();
            let limit = if case % 3 == 0 {
                frames[i as usize]
            } else {
                rng.word()
            };
            let frames = t.words(&frames);
            t.w32(this + 0xc8, frames);
            t.w32(this + 0x20, limit);
            t.check(
                "PSprite_frameBelowLimit_0056dc70",
                0x0056_dc70,
                Conv::Thiscall(this),
                &[i],
                |g| Out::U32(a::frame_below(g, this, i)),
            );
            if case % 2 == 0 {
                t.w32(this + 0xb8, 0);
            }
            t.check(
                "PSprite_defaultB8_0056dca0",
                0x0056_dca0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(a::default_b8(g, this)),
            );
            // count 0 reads address 4i (unmapped here): left out
            let words: Vec<u32> = (0..8).map(|_| rng.next()).collect();
            let data = t.words(&words);
            let count = [1, 2, 7, 0x8000_0001, 0x8000_0005][case as usize % 5];
            t.w32(this + 4, count);
            t.w32(
                this + 8,
                if count & 0x7fff_ffff > 1 {
                    data
                } else {
                    rng.next()
                },
            );
            t.check(
                "PSharray_word_0055b380",
                0x0055_b380,
                Conv::Thiscall(this),
                &[i % 2],
                |g| Out::U32(a::sharray_word(g, this, i % 2)),
            );
            let arg = rng.word();
            let o = if case % 3 == 0 { t.zeros(8) } else { 0 };
            let m = if case % 2 == 0 { t.zeros(8) } else { 0 };
            let p = if case % 5 == 0 { 0 } else { t.words(&[0, m]) };
            t.w32(this + 0x58, o);
            t.w32(this + 4, p);
            t.stub_sym(ma::SCENE_00507410, 2, true, 1, 1, Ret::Val(case + 2));
            t.stub_sym(ma::MATRIX_TO_WORLD_3X4, 3, true, 1, 1, Ret::Val(case + 3));
            t.check(
                "PGameplay_forward58_0055b780",
                0x0055_b780,
                Conv::Thiscall(this),
                &[arg],
                |g| Out::U32(a::forward_58(g, this, arg)),
            );
            t.reset();
        }
    });
}

// ---- batch 2 ----

include!("m5p_render_a/b2.rs");
