//! Continuation round 2, batch 13: post-processing set-up, the camera
//! matrices, the render thread's constructor and loop, and a few
//! constructors (`ffx_prender::p6_post`) against their originals.

use ffx_oracle::Conv;
use ffx_prender::{p6_encode as enc, p6_post as po};

use super::encode::{answers, renderer, ring_check_stub};

use super::small::{float_bits, run_framed};
use super::{Out, Rng, Twins, object};

/// A texture or render-target object, or 0 when `none`.
fn maybe(t: &mut Twins, rng: &mut Rng, none: bool) -> u32 {
    object(t, rng, 0x10, none)
}

#[test]
fn post_targets_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0d01);
        t.stub(po::UPDATE_MAX_LEVEL, true, 0, 0, 0);
        t.stub(po::TARGET_SET_TEXTURE, true, 1, 1, 0);
        t.stub(po::GLOW_EFFECT, false, 1, 0, 0x00ef_0001);
        t.stub(po::GLOW_GPU_EFFECT, false, 1, 0, 0x00ef_0002);
        for i in 0..24u32 {
            // the cluster's creations, in call order; the target objects
            let texs: Vec<u32> = (0..4)
                .map(|k| maybe(&mut t, &mut rng, (i + k) % 9 == 8))
                .collect();
            t.stub_seq(po::CLUSTER_CREATE, true, 3, 3, &texs);
            let dims: Vec<u32> = (0..4).map(|k| u32::from((i + 2 * k) % 7 == 6)).collect();
            t.stub_seq(po::SET_DIMENSIONS, true, 4, 4, &dims);
            let rt = maybe(&mut t, &mut rng, i % 5 == 4);
            t.stub(po::CREATE_TARGET, true, 1, 1, rt);
            let objs: Vec<u32> = (0..3)
                .map(|k| {
                    let o = maybe(&mut t, &mut rng, (i + k) % 8 == 7);
                    if o != 0 {
                        let rt = maybe(&mut t, &mut rng, (i + k) % 6 == 5);
                        t.w32(o + 0x34, rt);
                    }
                    o
                })
                .collect();
            t.stub_seq(po::CLUSTER_CREATE2, true, 3, 3, &objs);
            let this = object(&mut t, &mut rng, 0x3f0, false);
            let (cluster, w, h) = (rng.next(), rng.below(0x1000), rng.below(0x1000));
            t.reset_log();
            t.check(
                &format!("{name} glow GPU target #{i}"),
                0x00a1_a2c0,
                Conv::Thiscall(this),
                &[cluster, w, h],
                |g| Out::U32(po::bridge::glow_gpu_setup(g, this, cluster, w, h)),
            );
            // the sequences start again
            t.unstub(po::CLUSTER_CREATE);
            t.unstub(po::SET_DIMENSIONS);
            t.unstub(po::CLUSTER_CREATE2);
            t.stub_seq(po::CLUSTER_CREATE, true, 3, 3, &texs);
            t.stub_seq(po::SET_DIMENSIONS, true, 4, 4, &dims);
            t.stub_seq(po::CLUSTER_CREATE2, true, 3, 3, &objs);
            t.reset_log();
            t.check(
                &format!("{name} glow base targets #{i}"),
                0x00a1_a190,
                Conv::Thiscall(this),
                &[cluster, w, h],
                |g| Out::U32(po::bridge::glow_base_setup(g, this, cluster, w, h)),
            );
            t.unstub(po::CLUSTER_CREATE);
            t.unstub(po::SET_DIMENSIONS);
            t.unstub(po::CLUSTER_CREATE2);
            // the glow's set-up, its two parts stubbed
            t.stub_seq(po::CLUSTER_CREATE, true, 3, 3, &texs);
            t.stub_seq(po::SET_DIMENSIONS, true, 4, 4, &dims);
            t.stub(
                po::GLOW_BASE_SETUP,
                true,
                3,
                3,
                if i % 6 == 1 { 0xd } else { 0 },
            );
            t.stub(po::GLOW_TARGETS, true, 4, 4, 0x0076_0000 + i);
            t.reset_log();
            t.check(
                &format!("{name} glow set-up #{i}"),
                0x00a1_76d0,
                Conv::Thiscall(this),
                &[cluster, w, h],
                |g| Out::U32(po::bridge::glow_setup(g, this, cluster, w, h)),
            );
            t.unstub(po::GLOW_BASE_SETUP);
            t.unstub(po::GLOW_TARGETS);
            t.unstub(po::CLUSTER_CREATE);
            t.unstub(po::SET_DIMENSIONS);
            t.unstub(po::CREATE_TARGET);
        }
        // PDeferredLightingBase's deleting destructor
        t.stub(po::DEFERRED_PARTS, true, 0, 0, 0);
        t.stub(po::MEMBER_DTOR, true, 0, 0, 0);
        t.stub(po::FREE, false, 1, 0, 0);
        for i in 0..4 {
            let this = object(&mut t, &mut rng, 0xfd0, false);
            let flags = rng.next() & !1 | (i & 1);
            t.reset_log();
            t.check(
                &format!("{name} deferred deleting dtor #{i}"),
                0x00a0_26d0,
                Conv::Thiscall(this),
                &[flags],
                |g| Out::U32(po::bridge::deferred_deleting_dtor(g, this, flags)),
            );
        }
        t.unstub(po::FREE);
    }
}

#[test]
fn post_camera_matrices_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0d02);
        // the matrix helpers: stand-ins that log; the inverse returns its
        // output, the copies fill their matrix with words from the source
        // FROM_NODE copies 16 words of its source into `ecx`
        let copy16 = [
            0x51, 0x56, 0x57, // push ecx; push esi; push edi
            0x8b, 0xf9, // mov edi, ecx
            0x8b, 0x74, 0x24, 0x10, // mov esi, [esp+0x10]
            0xb9, 0x10, 0x00, 0x00, 0x00, // mov ecx, 16
            0xf3, 0xa5, // rep movsd
            0x5f, 0x5e, 0x59, // pop edi; pop esi; pop ecx
        ];
        t.stub_pre(po::FROM_NODE, &copy16, true, 1, 1, 0);
        let inv = {
            // the output's first word set from the input's
            [
                0x8b, 0x44, 0x24, 0x04, // mov eax, [esp+4]
                0x8b, 0x54, 0x24, 0x08, // mov edx, [esp+8]
                0x8b, 0x12, // mov edx, [edx]
                0x89, 0x10, // mov [eax], edx
            ]
        };
        t.stub_pre(po::INVERSE, &inv, false, 2, 0, 0);
        t.stub(po::COPY, true, 1, 1, 0x0c09_0000);
        t.stub(po::COPY2, true, 1, 1, 0x0c09_0001);
        t.stub(po::MULTIPLY, false, 3, 0, 0x0c09_0002);
        t.stub(po::SET_VIEW, true, 1, 1, 0x0c09_0003);
        for i in 0..16u32 {
            let camera = object(&mut t, &mut rng, 0x30, false);
            let parent = object(&mut t, &mut rng, 0x10, false);
            for k in 0..0x10 {
                let (f, g) = (float_bits(&mut rng), float_bits(&mut rng));
                t.w32(camera + 4 * k, f);
                t.w32(parent + 4 * k, g);
            }
            t.w32(camera + 0xb0, if i % 2 == 0 { 0 } else { parent });
            let this = object(&mut t, &mut rng, 0x38, false);
            run_framed(
                &mut t,
                &format!("{name} setCamera #{i}"),
                0x005b_abd0,
                Conv::Thiscall(this),
                &[camera],
                0xc0,
                0xc0,
                |g, ebp| po::bridge::set_camera(g, this, ebp - 0xc0, camera),
            );
            run_framed(
                &mut t,
                &format!("{name} updateViewMatrices #{i}"),
                0x0067_20c0,
                Conv::Thiscall(camera),
                &[],
                0xc0,
                0xc0,
                |g, ebp| po::bridge::update_view_matrices(g, camera, ebp - 0xc0),
            );
        }
    }
}

#[test]
fn post_render_thread_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0d03);
        // the constructor
        t.stub(po::THREAD_BASE, true, 0, 0, 0);
        t.stub(po::VECTOR_CTOR, false, 5, 5, 0);
        t.stub(po::SEMAPHORE_CTOR, true, 0, 0, 0);
        t.stub(po::MEMSET, false, 3, 0, 0);
        for i in 0..3 {
            let this = object(&mut t, &mut rng, 0x84, false);
            t.reset_log();
            t.check(
                &format!("{name} PRenderThread #{i}"),
                0x0061_4010,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(po::bridge::thread_ctor(g, this)),
            );
        }
        // the loop: each wait moves a script of states on (the state
        // word `+0xc` from a table), the last state 2
        t.stub(po::NAME_INIT, true, 2, 2, 0);
        t.stub(po::NAME_ADD, true, 1, 1, 0);
        t.stub(po::NAME_REMOVE, true, 1, 1, 0);
        t.stub(po::EVENT_RESET, true, 0, 0, 0);
        t.stub(po::PROCESS_COMMANDS, true, 2, 2, 0);
        t.stub(po::SEMAPHORE_SIGNAL, true, 2, 2, 0);
        t.stub(po::PRINT, false, 2, 0, 0);
        for i in 0..8u32 {
            let this = object(&mut t, &mut rng, 0x58, false);
            t.w32(this + 0x150, [0, 15, 14, 3][i as usize % 4]);
            for e in 0..16 {
                let entry = this + 0x14 * e;
                if rng.chance(3) {
                    t.w32(entry + 0x20, 0);
                }
            }
            // the wait stand-in: [this+0xc] = script[k++]
            let n = 1 + rng.below(5);
            let script = t.zeros(4 * (n as usize + 1) + 4);
            for k in 0..n {
                let s = if k + 1 == n {
                    2
                } else {
                    [0, 1, 1, 3][rng.below(4) as usize]
                };
                t.w32(script + 4 + 4 * k, s);
            }
            let mut pre = vec![0xa1];
            pre.extend(script.to_le_bytes()); // mov eax, [script]
            pre.extend([0xff, 0x05]);
            pre.extend(script.to_le_bytes()); // inc dword [script]
            pre.extend([0x8b, 0x04, 0x85]);
            pre.extend((script + 4).to_le_bytes()); // mov eax, [script+4+eax*4]
            pre.push(0xa3);
            pre.extend((this + 0xc).to_le_bytes()); // mov [this+0xc], eax
            t.stub_pre(po::EVENT_WAIT, &pre, true, 0, 0, 0);
            t.reset_log();
            t.check(
                &format!("{name} threadEntry #{i}"),
                0x0061_6010,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(po::bridge::thread_entry(g, this)),
            );
            t.unstub(po::EVENT_WAIT);
        }
    }
}

#[test]
fn post_objects_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0d04);
        // the Iggy player
        t.stub(po::PLAYER_CTOR, true, 0, 0, 0);
        t.stub(po::PLAYER_DTOR, true, 0, 0, 0);
        t.stub(po::FREE, false, 1, 0, 0);
        for i in 0..6u32 {
            let p = if i % 3 == 0 { 0 } else { t.zeros(0x2c) };
            t.stub(po::MALLOC, false, 1, 0, p);
            // (the stub logs six of the seven words)
            t.stub(po::PLAYER_INIT, true, 6, 7, u32::from(i % 2 == 1));
            let args: [u32; 7] = std::array::from_fn(|_| rng.next());
            t.reset_log();
            t.check(
                &format!("{name} Iggy player #{i}"),
                0x00a3_e4f0,
                Conv::Cdecl,
                &args,
                |g| Out::U32(po::bridge::player_create(g, args)),
            );
            t.unstub(po::MALLOC);
            t.unstub(po::PLAYER_INIT);
        }
        // PUtilityText: the list by level
        let head = t.rebase(po::UTILITY_LIST);
        for i in 0..8u32 {
            let mut next = 0;
            let n = rng.below(4);
            // every second list ends below level 0xff
            let top = if i % 2 == 0 { 2 } else { 4 };
            let n = if i % 2 == 0 { n.max(1) } else { n };
            let mut levels: Vec<u32> = (0..n)
                .map(|_| [0x10, 0xfe, 0xff, 0x100][rng.below(top) as usize])
                .collect();
            levels.sort_unstable();
            for &lv in levels.iter().rev() {
                let u = object(&mut t, &mut rng, 4, false);
                t.w32(u + 0xc, lv);
                t.w32(u + 4, next);
                next = u;
            }
            t.w32(head, next);
            let this = object(&mut t, &mut rng, 4, false);
            t.reset_log();
            t.check(
                &format!("{name} PUtilityText #{i}"),
                0x00a3_e730,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(po::bridge::utility_text_ctor(g, this)),
            );
        }
        // the store constructors
        for (f, table) in [
            (0x0062_0590, po::STORES_620590),
            (0x0062_0780, po::STORES_620780),
        ] {
            let this = object(&mut t, &mut rng, 0x38, false);
            t.check(
                &format!("{name} ctor {f:#x}"),
                f,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(po::bridge::stores(g, this, table)),
            );
        }
        // the network's destructor
        t.stub(po::NETWORK_BASE_DTOR, true, 0, 0, 0x0b45_e000);
        for i in 0..12u32 {
            let net = object(&mut t, &mut rng, 0x10, false);
            for (k, off) in [0x34u32, 0x2c, 0x24, 0x1c].into_iter().enumerate() {
                let view = (i + k as u32).is_multiple_of(3);
                // bit 30 alone is no view
                let high = if (i + k as u32) % 5 == 2 {
                    0x4000_0000
                } else {
                    0
                };
                t.w32(
                    net + off,
                    rng.below(4) | high | if view { 0x8000_0000 } else { 0 },
                );
                if (i + k as u32) % 4 == 1 {
                    t.w32(net + off + 4, 0);
                }
            }
            t.reset_log();
            t.check(
                &format!("{name} network dtor #{i}"),
                0x005a_a4b0,
                Conv::Thiscall(net),
                &[],
                |g| Out::U32(po::bridge::network_dtor(g, net)),
            );
        }
        t.unstub(po::FREE);
        // a segment's packet
        t.stub(po::PACKET_SUBMIT, false, 1, 0, 0x0005_c1f9);
        for i in 0..12u32 {
            let this = object(&mut t, &mut rng, 0x10, false);
            let nseg = 3;
            let segs = object(&mut t, &mut rng, 4 * nseg, false);
            let infos = object(&mut t, &mut rng, 4 * nseg, false);
            t.w32(this + 0x38, segs);
            t.w32(this + 0x14, infos);
            let k = rng.below(nseg);
            let n = rng.below(5);
            let first = object(&mut t, &mut rng, 8, i % 7 == 6);
            if first != 0 {
                t.w32(first + 0x18, n | (rng.next() & 0x8000_0000));
            }
            t.w32(segs + 16 * k, first);
            t.w32(segs + 16 * k + 8, rng.below(4));
            t.w32(segs + 16 * k + 4, rng.next());
            let ids = t.zeros(4 * 16);
            let table = t.zeros(4 * 8);
            for j in 0..16 {
                let v = if rng.chance(4) {
                    0xffff_ffff
                } else {
                    rng.below(8)
                };
                t.w32(ids + 4 * j, v);
            }
            for j in 0..8 {
                let none = rng.chance(3);
                let o = object(&mut t, &mut rng, 0x10, none);
                t.w32(table + 4 * j, o);
            }
            t.w32(this + 0x30, ids);
            let packet = object(&mut t, &mut rng, 0x20, false);
            let p = object(&mut t, &mut rng, 4, false);
            t.w32(packet + 0x10, p);
            t.write(
                packet + 0x22,
                &(4 * rng.below(8) as u16 + 0x30).to_le_bytes(),
            );
            let x = rng.next();
            t.reset_log();
            t.check(
                &format!("{name} fill packet #{i}"),
                0x005a_e0f0,
                Conv::Thiscall(this),
                &[k, packet, table, x],
                |g| Out::U32(po::bridge::fill_packet(g, this, k, packet, table)),
            );
        }
    }
}

#[test]
fn post_mesh_buffers_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0d05);
        // the sizes
        t.stub(po::DATA_BLOCK_COUNT, true, 1, 1, 3);
        for i in 0..24u32 {
            // the smallest table (no pointers but the two) now and then
            let small = i % 6 == 0;
            let kinds = if small { 0 } else { rng.below(5) };
            t.stub(po::COUNT_KINDS, false, 1, 0, kinds);
            let outs = t.zeros(12);
            let (total, blocks, table) = (outs, outs + 4, outs + 8);
            let material = object(&mut t, &mut rng, 4, false);
            t.w32(material, rng.below(0x400));
            let mesh = object(&mut t, &mut rng, 0x10, false);
            let node = object(&mut t, &mut rng, 8, false);
            let n = if small { 0 } else { rng.below(6) };
            t.w32(node + 0x14, n | (rng.next() & 0x8000_0000));
            let segs = object(&mut t, &mut rng, 16, false);
            let seg = rng.below(4);
            if i % 3 == 0 {
                t.w32(segs + 16 * seg + 0xc, 0);
            }
            let dynamic = object(&mut t, &mut rng, 8, false);
            t.w32(dynamic + 0x14, segs);
            let instance = object(&mut t, &mut rng, 2, i % 4 == 3 || small);
            if instance != 0 {
                t.w32(instance + 4, if i % 5 == 1 { 0 } else { dynamic });
            }
            let flags = rng.next();
            let args = [
                total, blocks, table, flags, material, mesh, node, seg, instance,
            ];
            t.reset_log();
            t.check(
                &format!("{name} mesh sizes #{i}"),
                0x005b_5150,
                Conv::Cdecl,
                &args,
                |g| Out::U32(po::bridge::mesh_sizes(g, args)),
            );
            t.unstub(po::COUNT_KINDS);
        }
        t.unstub(po::DATA_BLOCK_COUNT);
        // the texture records
        t.stub(po::CUBE_RECORD, true, 1, 1, 0x0c0b_e000);
        t.stub(po::VOLUME_RECORD, true, 1, 1, 0x0003_d000);
        for i in 0..16u32 {
            let n = rng.below(5);
            let base = object(&mut t, &mut rng, 0x30, false);
            let entries = object(&mut t, &mut rng, 4 * n, false);
            let params = t.zeros(8);
            t.w32(params, n | (rng.next() & 0x8000_0000));
            t.w32(params + 4, entries);
            for k in 0..n {
                let e = entries + 16 * k;
                let kinds = [0x42u8, 0xd9, 0xda, 0xdb, 0xdc, 0x12, 0x43, 0x44, 0x41];
                t.write(e + 2, &[kinds[rng.below(9) as usize]]);
                let off = 0x10 * k;
                t.write(e + 8, &(off as u16).to_le_bytes());
                t.w32(
                    base + off,
                    [0x42, 0x42, 0x43, 0x44, 0x45][rng.below(5) as usize],
                );
                t.w32(base + off + 4, rng.below(4));
                let none = rng.chance(4);
                let tex = object(&mut t, &mut rng, 0x10, none);
                if tex != 0 {
                    t.w32(tex + 0x24, rng.below(3) | (rng.next() & 0x8000_0000));
                }
                t.w32(base + off + 0xc, tex);
            }
            let out = t.zeros(4 * 8);
            t.reset_log();
            let what = format!("{name} texture pointers #{i}");
            t.check(&what, 0x005b_53f0, Conv::Cdecl, &[base, params, out], |g| {
                Out::U32(po::bridge::texture_pointers(g, base, params, out))
            });
        }
        // the callback with a data block
        let tab = t.zeros(0x40);
        let check = ring_check_stub(&mut t, tab);
        t.jump(enc::RING_CHECK, check);
        let submit = t.stub_code(0x005b_6c50, true, 0, 0, 0x77);
        for i in 0..40u32 {
            let block = if i % 9 == 8 {
                0
            } else {
                t.zeros(0x20) + rng.below(16)
            };
            let cmd = if i % 7 == 6 { 0 } else { t.zeros(0x48) };
            t.stub_seq(po::RING_ALLOCATE, true, 1, 1, &[block, cmd]);
            let r = renderer(&mut t, &mut rng, submit);
            answers(&mut t, &mut rng, r, tab);
            let f = if i % 10 == 0 { 0 } else { rng.next() };
            let size = if i % 4 == 0 { 0 } else { rng.below(0x100) };
            let out = object(&mut t, &mut rng, 1, false);
            run_framed(
                &mut t,
                &format!("{name} data callback #{i}"),
                0x005b_5940,
                Conv::Thiscall(r),
                &[f, size, out],
                8,
                8,
                |g, ebp| {
                    let r = po::bridge::queue_data_callback(g, r, ebp - 8, f, size, out);
                    // the original keeps its adjusted size in its own argument
                    // slot (dead after its `ret 0xc`)
                    if f != 0 {
                        let s = size.wrapping_add(if size != 0 { 0xf } else { 0 });
                        ffx_bridge::Mem::write(g, ebp + 0xc, s);
                    }
                    r
                },
            );
            t.unstub(po::RING_ALLOCATE);
        }
    }
}

#[test]
fn post_effect_ctors_and_dtors_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0d06);
        t.stub(po::SAMPLER_CTOR, true, 0, 0, 0);
        for &(f, c) in po::POST_CTORS {
            for i in 0..6u32 {
                let this = object(&mut t, &mut rng, 0x28, false);
                // the samplers' flags (bit 7) set or not
                for (k, s) in c.samplers.into_iter().enumerate() {
                    let fl = rng.next() & !0x80 | if (i >> k) & 1 == 1 { 0x80 } else { 0 };
                    t.w32(this + s + 0x1c, fl);
                }
                t.reset_log();
                t.check(
                    &format!("{name} post ctor {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(po::bridge::post_ctor(g, this, &c)),
                );
            }
        }
        t.unstub(po::SAMPLER_CTOR);
        // the destructors
        t.stub(po::TARGET_RELEASE, false, 1, 0, 0x88);
        t.stub(po::SAMPLER_RELEASE, true, 0, 0, 0x89);
        t.stub(po::EFFECT_RELEASE, true, 0, 0, 0x8a);
        t.stub(po::MEMBER_DTOR, true, 0, 0, 0x8b);
        for k in 0..3 {
            t.stub(0x0059_5f90 + 0x10 * k, true, 0, 0, 0x0101 * (k + 1));
        }
        t.stub(0x0059_cc40, true, 0, 0, 0x70);
        t.stub(0x0059_cf00, true, 0, 0, 0x71);
        t.stub(0x0059_d0f0, true, 0, 0, 0x72);
        for &(f, d) in po::POST_DTORS {
            for i in 0..6u32 {
                let this = object(&mut t, &mut rng, 0x3f0, false);
                if let Some(samplers) = d.samplers {
                    for (k, s) in samplers.into_iter().enumerate() {
                        let fl = rng.next() & !0x80 | if (i >> k) & 1 == 1 { 0x80 } else { 0 };
                        t.w32(this + s + 0x1c, fl);
                    }
                }
                if let Some(e) = d.effect {
                    let eff = object(&mut t, &mut rng, 1, i % 3 == 2);
                    t.w32(this + e, eff);
                }
                for &off in d.inline {
                    let tex = object(&mut t, &mut rng, 2, i % 4 == 3);
                    let target = object(&mut t, &mut rng, 2, i % 5 == 4);
                    if target != 0 {
                        t.w32(target, tex);
                        t.write(target + 4, &[(i % 4) as u8]);
                    }
                    t.w32(this + off, target);
                }
                t.reset_log();
                t.check(
                    &format!("{name} post dtor {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(po::bridge::post_dtor(g, this, &d)),
                );
            }
        }
        // PDeferredLightingD3D11's destructor
        t.stub(po::DEFERRED_LIGHTING_PARTS, true, 0, 0, 0);
        t.stub(po::DEFERRED_PARTS, true, 0, 0, 0);
        for i in 0..2 {
            let this = object(&mut t, &mut rng, 0xff4, false);
            t.reset_log();
            t.check(
                &format!("{name} deferred lighting dtor #{i}"),
                0x009f_dc00,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(po::bridge::deferred_dtor(g, this)),
            );
        }
        // two targets and an array
        t.stub(po::FREE, false, 1, 0, 0x0f4e_e000);
        for i in 0..6u32 {
            let obj = object(&mut t, &mut rng, 4, false);
            t.w32(obj, rng.below(4) | if i % 2 == 0 { 0x8000_0000 } else { 0 });
            if i % 3 == 1 {
                t.w32(obj + 4, 0);
            }
            t.reset_log();
            t.check(
                &format!("{name} targets array dtor #{i}"),
                0x009f_e790,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(po::bridge::targets_array_dtor(g, obj)),
            );
        }
        t.unstub(po::FREE);
        // the camera-like defaults: the identity in the frame
        t.stub(po::SET_VIEW, true, 1, 1, 0);
        for i in 0..2 {
            let this = object(&mut t, &mut rng, 0x20, false);
            run_framed(
                &mut t,
                &format!("{name} camera defaults #{i}"),
                0x009f_c6f0,
                Conv::Thiscall(this),
                &[],
                0x40,
                0x40,
                |g, ebp| po::bridge::camera_defaults(g, this, ebp - 0x40),
            );
        }
    }
}

#[test]
fn post_texture_unbind_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0d07);
        t.stub(po::FREE, false, 1, 0, 0x0f4e_e000);
        let release = t.stub_code(0x0c0f_0002, false, 1, 1, 0);
        t.map_page0();
        for &(f, u) in po::UNBINDS {
            for i in 0..16u32 {
                let n = [0, 1, 2, 3][i as usize % 4];
                t.stub(u.count, true, 0, 0, n);
                t.stub(
                    po::UNBIND_OWNER,
                    true,
                    0,
                    0,
                    if i % 7 == 6 { 0x15 } else { 0 },
                );
                let tex = object(&mut t, &mut rng, 0x20, false);
                let owner = object(&mut t, &mut rng, 1, i % 3 == 0);
                t.w32(tex + 0x18, owner);
                // the records: inline for one, else a block
                let recs = t.zeros((u.stride * n.max(1)) as usize);
                // an array emptied under a count of records (page 0 is read)
                let count = if i == 15 {
                    0
                } else {
                    n | (rng.next() & 0x8000_0000)
                };
                t.w32(tex + u.array, count);
                let inline = n == 1;
                let base = if inline { tex + u.array + 4 } else { recs };
                if !inline {
                    t.w32(tex + u.array + 4, recs);
                }
                for k in 0..n {
                    for &fld in u.release {
                        let obj = if rng.chance(3) {
                            0
                        } else {
                            let vt = t.zeros(0x10);
                            t.w32(vt + 8, release);
                            let o = object(&mut t, &mut rng, 2, false);
                            t.w32(o, vt);
                            o
                        };
                        t.w32(base + u.stride * k + fld, obj);
                    }
                    if let Some(fr) = u.free {
                        let blk = if rng.chance(3) { 0 } else { rng.next() | 1 };
                        t.w32(base + u.stride * k + fr, blk);
                    }
                }
                t.reset_log();
                t.check(
                    &format!("{name} unbind {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(tex),
                    &[],
                    |g| Out::U32(po::bridge::unbind(g, tex, &u)),
                );
                t.unstub(u.count);
                t.unstub(po::UNBIND_OWNER);
            }
        }
    }
}
