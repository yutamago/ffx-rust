//! Continuation round 3, batch 15: the renderer's colour and depth-stencil
//! targets, a light's attenuation factors, the legacy glow's GPU targets
//! and shaders, and a packet's layout (`ffx_prender::p6_targets`) against
//! their originals.

use ffx_bridge::Mem;
use ffx_oracle::Conv;
use ffx_prender::p6_targets as tg;

use super::encode::{answers, ring_check_stub};
use super::small::{float_bits, run_framed};
use super::{Out, Rng, Twins, object};

/// `GetRenderStateCache`'s guard word, its object's constructor and the
/// CRT's `atexit` (the depth-stencil setter builds the cache inline).
const CACHE_GUARD: u32 = 0x00c9_628c;
const CACHE: u32 = 0x00c9_4f00;
const CACHE_CTOR: u32 = 0x004f_ee70;
const ATEXIT: u32 = 0x0094_9207;

/// A renderer of 0x50 random words: vtable slot 2 (`submitFrame`) the
/// stand-in `submit`; its rings' cursors and ends around each other;
/// `+0x4c` empty or not; the scene bit, the colour mask and the depth
/// flag random.
fn renderer(t: &mut Twins, rng: &mut Rng, submit: u32) -> u32 {
    let r = object(t, rng, 0x50, false);
    let vtable = t.zeros(0x10);
    t.w32(vtable + 8, submit);
    t.w32(r, vtable);
    t.w32(r + 4, rng.next() & !u32::from(rng.chance(4)));
    for (cur, end) in [(0x24, 0x28), (0x38, 0x3c)] {
        let base = 0x1000_0000 + (rng.below(16) << 8);
        let c = base + rng.below(0x200);
        let e = if rng.chance(4) {
            c.wrapping_sub(rng.below(0x40))
        } else {
            c + rng.below(0x200)
        };
        t.w32(r + cur, c);
        t.w32(r + end, e);
    }
    if rng.chance(2) {
        t.w32(r + 0x4c, 0);
    }
    let mask = if rng.chance(2) { 0 } else { rng.next() as u8 };
    t.write(r + 0x138, &[mask, u8::from(rng.chance(2))]);
    r
}

/// A 16-byte target: its texture (0 in one case of four) and three
/// words; or no target.
fn target(t: &mut Twins, rng: &mut Rng, none: bool) -> u32 {
    if none {
        return 0;
    }
    let tg = object(t, rng, 4, false);
    if rng.chance(4) {
        t.w32(tg, 0);
    }
    tg
}

/// The back buffer's size in the render state cache, and its guard bit
/// set or not.
fn cache(t: &mut Twins, rng: &mut Rng) {
    let c = t.rebase(CACHE);
    t.w32(c + 0x200, rng.next());
    t.w32(c + 0x204, rng.next());
    let g = t.rebase(CACHE_GUARD);
    t.w32(g, rng.next() & !1 | u32::from(rng.chance(2)));
}

#[test]
fn render_targets_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0f01);
        t.stub(CACHE_CTOR, true, 0, 0, 0);
        t.stub(ATEXIT, false, 1, 0, 0);
        let submit = t.stub_code(0x005b_6c50, true, 0, 0, 0x77);
        let tab = t.zeros(0x40);
        let check = ring_check_stub(&mut t, tab);
        t.jump(0x0061_3f00, check);
        let mut seen = [0u32; 4];
        for i in 0..160u32 {
            let errors = [0, 0, 0, 7];
            let v = errors[rng.below(4) as usize];
            t.stub(tg::VALIDATE, true, 0, 0, v);
            let e = if rng.chance(5) { 0x21 } else { 0 };
            t.stub(tg::VIEWPORT_SIZE, true, 2, 2, e);
            let room = [0, 1, 0x1234_5600, 0x1234_5601][rng.below(4) as usize];
            t.stub(tg::HAS_ROOM, true, 2, 2, room);
            let blocks: Vec<u32> = (0..2)
                .map(|k| if (i + k) % 7 == 6 { 0 } else { t.zeros(0x48) })
                .collect();
            t.stub_seq(tg::RING_ALLOCATE, true, 1, 1, &blocks);
            cache(&mut t, &mut rng);
            let r = renderer(&mut t, &mut rng, submit);
            let tgt = target(&mut t, &mut rng, i % 3 == 2);
            let slot = if rng.chance(8) {
                4 + rng.below(3)
            } else {
                rng.below(4)
            };
            t.reset_log();
            t.check(
                &format!("{name} setColorTarget #{i}"),
                0x005b_b130,
                Conv::Thiscall(r),
                &[tgt, slot],
                |g| Out::U32(tg::bridge::set_color_target(g, r, tgt, slot)),
            );
            let ra: u32 = Mem::read(&t.a, t.log);
            seen[0] += u32::from(ra > 3);
            t.unstub(tg::RING_ALLOCATE);

            t.stub_seq(tg::RING_ALLOCATE, true, 1, 1, &blocks);
            cache(&mut t, &mut rng);
            let r = renderer(&mut t, &mut rng, submit);
            answers(&mut t, &mut rng, r, tab);
            let tgt = target(&mut t, &mut rng, i % 3 == 1);
            run_framed(
                &mut t,
                &format!("{name} setDepthStencilTarget #{i}"),
                0x005b_b5b0,
                Conv::Thiscall(r),
                &[tgt],
                0x1c,
                8,
                |g, ebp| tg::bridge::set_depth_stencil_target(g, r, ebp - 0x1c, tgt),
            );
            let ra: u32 = Mem::read(&t.a, t.log);
            seen[1] += u32::from(ra > 4);
            t.unstub(tg::RING_ALLOCATE);
            t.unstub(tg::VALIDATE);
            t.unstub(tg::VIEWPORT_SIZE);
            t.unstub(tg::HAS_ROOM);
        }
        // most cases reach the command
        assert!(seen[0] > 25 && seen[1] > 25, "{name}: {seen:?}");
    }
}

/// `fld qword [esp+4]; call [esp+0xc]; ret` and at `+16` `fstp st0;
/// ret`: `Host::call_intrinsic`'s trampoline, at `super::CAVE`.
#[test]
fn light_factors_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0f02);
        t.trampoline();
        let mut types = [0u32; 3];
        for i in 0..300u32 {
            let light = object(&mut t, &mut rng, 0x10, false);
            let ty = match i % 3 {
                0 => t.rebase(tg::SPOT_TYPE),
                1 => t.rebase(tg::POINT_TYPE),
                _ => rng.next(),
            };
            types[(i % 3) as usize] += 1;
            t.w32(light + 0x18, ty);
            for off in [0x1c, 0x20] {
                let a = if rng.chance(6) {
                    float_bits(&mut rng)
                } else {
                    (f32::from(rng.below(3000) as u16) / 1000.0).to_bits()
                };
                t.w32(light + off, a);
            }
            for off in [0x28, 0x2c] {
                let d = if rng.chance(6) {
                    float_bits(&mut rng)
                } else {
                    (f64::from(rng.below(100_000)) as f32 / 97.0).to_bits()
                };
                t.w32(light + off, d);
            }
            if i % 50 == 49 {
                // equal distances: an infinite factor
                let n: u32 = Mem::read(&t.b, light + 0x28);
                t.w32(light + 0x2c, n);
            }
            let out: Vec<u32> = (0..4).map(|_| t.zeros(4)).collect();
            let args = [out[0], out[1], out[2], out[3], light];
            run_framed(
                &mut t,
                &format!("{name} light factors #{i}"),
                0x005b_5ef0,
                Conv::Cdecl,
                &args,
                0xc,
                0,
                |g, ebp| {
                    let (eax, slot) =
                        tg::bridge::light_factors(g, [out[0], out[1], out[2], out[3]], light);
                    g.write(ebp + 0x18, slot);
                    eax
                },
            );
        }
        assert!(types.iter().all(|&n| n == 100), "{name}");
    }
}

#[test]
fn glow_gpu_initialize_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0f03);
        t.stub(0x004c_4250, true, 0, 0, 0);
        t.stub(0x0058_3a60, true, 1, 1, 0);
        for i in 0..20u32 {
            // every creation succeeds: a failed one makes the original
            // name a target at address 0xc
            let texs: Vec<u32> = (0..6).map(|_| object(&mut t, &mut rng, 8, false)).collect();
            t.stub_seq(0x0044_1010, true, 3, 3, &texs);
            t.stub(0x004c_40c0, true, 4, 4, 0);
            let objs: Vec<u32> = (0..6)
                .map(|_| {
                    let o = object(&mut t, &mut rng, 0x10, false);
                    let rt = object(&mut t, &mut rng, 8, false);
                    t.w32(o + 0x34, rt);
                    o
                })
                .collect();
            t.stub_seq(0x0044_0b70, true, 3, 3, &objs);
            let this = object(&mut t, &mut rng, 0x30, false);
            let (cluster, w, h) = (rng.next(), rng.next(), rng.next());
            t.reset_log();
            t.check(
                &format!("{name} glow GPU initialize #{i}"),
                0x00a1_7bd0,
                Conv::Thiscall(this),
                &[cluster, w, h],
                |g| Out::U32(tg::bridge::glow_gpu_initialize(g, this, cluster, w, h)),
            );
            assert_eq!(t.log_entries(), 6 * 5, "{name} #{i}: the calls");
            t.unstub(0x0044_1010);
            t.unstub(0x004c_40c0);
            t.unstub(0x0044_0b70);
        }
    }
}

#[test]
fn glow_initialize_shaders_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0f04);
        t.stub(tg::FREE, false, 1, 0, 0);
        t.stub(tg::SET_WORD, true, 2, 2, 0);
        let mut paths = [0u32; 4];
        for i in 0..120u32 {
            let fx = object(&mut t, &mut rng, 8, i % 10 == 9);
            t.stub(tg::GLOW_GPU_EFFECT, false, 1, 0, fx);
            let material = object(&mut t, &mut rng, 4, false);
            t.stub(tg::CREATE_MATERIAL, true, 3, 3, material);
            let params: Vec<u32> = (0..6)
                .map(|k| {
                    if rng.chance(if k == 5 { 3 } else { 12 }) {
                        0
                    } else {
                        rng.next() | 1
                    }
                })
                .collect();
            t.stub_seq(tg::FIND_PARAMETER, true, 1, 1, &params);
            // the state array: its count word (1, inline, owned or not)
            let array = t.zeros(0x10);
            let count = match rng.below(5) {
                0 => 1,
                1 => 0,
                2 => rng.below(0x100) | 0x8000_0000,
                _ => (2 + rng.below(0x100)) | (rng.next() & 0x8000_0000),
            };
            t.w32(array, count);
            t.w32(array + 4, rng.next());
            t.stub(tg::CLUSTER_CREATE, true, 3, 3, array);
            let e = if rng.chance(5) { 0x41 } else { 0 };
            t.stub(tg::SET_VALUE, true, 2, 2, e);
            let regs: Vec<u32> = (0..2).map(|_| u32::from(rng.chance(4)) * 0x42).collect();
            t.stub_seq(tg::REGISTER, true, 0, 0, &regs);
            let this = object(&mut t, &mut rng, 0x20, false);
            let (cluster, effect) = (rng.next(), rng.next());
            run_framed(
                &mut t,
                &format!("{name} glow shaders #{i}"),
                0x00a1_ab00,
                Conv::Thiscall(this),
                &[cluster, effect],
                0x1c,
                0x10,
                |g, ebp| {
                    let r =
                        tg::bridge::glow_initialize_shaders(g, this, ebp - 0x1c, cluster, effect);
                    // the original keeps the second parameter in the
                    // effect's slot (`ret 8`: the callee's word)
                    if g.read::<u32>(this + 0x48) != 0 {
                        let p: u32 = g.read(this + 0x64);
                        g.write(ebp + 0xc, p);
                    }
                    r
                },
            );
            let ra: u32 = Mem::read(&t.a, t.log);
            paths[match ra {
                0..=2 => 0,
                3..=9 => 1,
                10..=12 => 2,
                _ => 3,
            }] += 1;
            for f in [
                tg::GLOW_GPU_EFFECT,
                tg::CREATE_MATERIAL,
                tg::FIND_PARAMETER,
                tg::CLUSTER_CREATE,
                tg::SET_VALUE,
                tg::REGISTER,
            ] {
                t.unstub(f);
            }
        }
        assert!(paths.iter().all(|&n| n > 3), "{name}: {paths:?}");
    }
}

#[test]
fn packet_layout_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0f05);
        for i in 0..80u32 {
            let n = if i % 9 == 8 { 0 } else { 1 + rng.below(9) };
            let records = t.zeros(0x14 * n as usize + 4);
            for k in 0..n {
                let ty = t.zeros(0x30);
                let rets = [rng.below(0x80), rng.below(0x80)];
                let header = if rng.chance(2) {
                    t.stub_code(0x7100 + k, false, 3, 0, rets[0])
                } else {
                    0
                };
                let plain = if rng.chance(3) {
                    0
                } else {
                    t.stub_code(0x7200 + k, false, 2, 0, rets[1])
                };
                t.w32(ty + 0x1c, header);
                t.w32(ty + 0x20, plain);
                t.w32(records + 0x14 * k, ty);
            }
            let net = object(&mut t, &mut rng, 0x10, false);
            t.w32(net + 4, n | (rng.next() & 0x8000_0000));
            t.w32(net + 8, records);
            t.w32(net + 0x18, rng.below(0x20) | (rng.next() & 0x8000_0000));
            if rng.chance(3) {
                t.w32(net + 0x28, 0);
            } else {
                let shared = object(&mut t, &mut rng, 2, false);
                t.w32(net + 0x28, shared);
            }
            let ctx = object(&mut t, &mut rng, 4, false);
            if rng.chance(3) {
                t.w32(ctx + 8, 0);
            } else {
                let seg = object(&mut t, &mut rng, 8, false);
                t.w32(ctx + 8, seg);
            }
            // a 16-aligned packet, or one 2 or 8 bytes past it (padding)
            // (filled with junk, so that every store shows)
            let out = object(&mut t, &mut rng, 0x400, false) + [0, 2, 8][rng.below(3) as usize];
            t.reset_log();
            t.check(
                &format!("{name} packet layout #{i}"),
                0x005c_6c30,
                Conv::Thiscall(net),
                &[out, ctx],
                |g| Out::U32(tg::bridge::packet_layout(g, net, out, ctx)),
            );
        }
    }
}

#[test]
fn common_shaders_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0f06);
        t.stub(tg::FREE, false, 1, 0, 0);
        let mut paths = [0u32; 3];
        for (f, c) in [
            (0x00a1_8870, tg::FXAA_COMMON),
            (0x00a1_85a0, tg::DOF_COMMON),
        ] {
            for i in 0..60u32 {
                let this = object(&mut t, &mut rng, 0x3f0, false);
                let fx = object(&mut t, &mut rng, 8, i % 15 == 14);
                t.w32(this + c.effect, fx);
                // the material's parameter buffer: its size word, a layout
                // word at +8
                let buffer = object(&mut t, &mut rng, 0x20, false);
                let room = 0x20 + rng.below(0x60);
                t.w32(buffer, room);
                let layout = rng.next();
                t.w32(buffer + 8, layout);
                let material = object(&mut t, &mut rng, 4, false);
                t.w32(material + 4, buffer);
                t.stub(tg::CREATE_MATERIAL, true, 3, 3, material);
                // the parent's buffer, of the same layout or not
                if rng.chance(3) {
                    t.w32(this + 8, 0);
                } else {
                    let src = object(&mut t, &mut rng, 8, false);
                    t.w32(src, 12 + rng.below(16));
                    let l = if rng.chance(2) {
                        layout ^ 0x8000_0000
                    } else {
                        rng.next()
                    };
                    t.w32(src + 8, l);
                    let parent = object(&mut t, &mut rng, 2, false);
                    t.w32(parent + 4, src);
                    t.w32(this + 8, parent);
                }
                let mut seq: Vec<u32> = c.params.iter().map(|_| rng.next()).collect();
                for _ in 0..2 {
                    if rng.chance(4) {
                        seq.push(0);
                    } else {
                        let p = t.zeros(0x10);
                        let size = rng.below(0x10) | (rng.next() & 0xe000);
                        // in the header, inside, past the end, or ending
                        // right at the end (or one byte past it)
                        let edge = room - (size & 0x1fff) + rng.below(2);
                        let off = [
                            rng.below(4),
                            4 + rng.below(0x40),
                            0x60 + rng.below(0x40),
                            edge,
                        ][rng.below(4) as usize];
                        t.write(p + 8, &(off as u16).to_le_bytes());
                        t.write(p + 0xa, &(size as u16).to_le_bytes());
                        seq.push(p);
                    }
                }
                t.stub_seq(tg::FIND_PARAMETER, true, 1, 1, &seq);
                let array = t.zeros(0x10);
                let count = match rng.below(4) {
                    0 => 1,
                    1 => 0x8000_0001,
                    _ => (2 + rng.below(4)) | (rng.next() & 0x8000_0000),
                };
                t.w32(array, count);
                let data = t.zeros(0x10);
                t.w32(array + 4, data);
                t.stub(tg::CLUSTER_CREATE, true, 3, 3, array);
                let regs: Vec<u32> = (0..2).map(|_| u32::from(rng.chance(6)) * 0x42).collect();
                t.stub_seq(tg::REGISTER, true, 0, 0, &regs);
                let (cluster, effect) = (rng.next(), rng.next());
                run_framed(
                    &mut t,
                    &format!("{name} common shaders {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(this),
                    &[cluster, effect],
                    0x20,
                    0x20,
                    |g, ebp| {
                        let (r, slot) =
                            tg::bridge::common_shaders(g, this, ebp - 0x20, cluster, effect, &c);
                        if let Some(v) = slot {
                            g.write(ebp + 0xc, v);
                        }
                        r
                    },
                );
                let calls: u32 = Mem::read(&t.a, t.log);
                paths[match calls {
                    0 => 0,
                    1..=12 => 1,
                    _ => 2,
                }] += 1;
                for s in [
                    tg::CREATE_MATERIAL,
                    tg::FIND_PARAMETER,
                    tg::CLUSTER_CREATE,
                    tg::REGISTER,
                ] {
                    t.unstub(s);
                }
            }
        }
        assert!(paths.iter().all(|&n| n > 3), "{name}: {paths:?}");
    }
}

#[test]
fn mesh_commands_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0f07);
        t.import_stub(tg::INTERLOCKED_ADD, 2, 0x11);
        let mut cmds_seen = 0;
        for i in 0..60u32 {
            let out = object(&mut t, &mut rng, 0x40, false);
            t.stub(
                tg::GATHER_RECORDS,
                false,
                3,
                0,
                out + 4 + 4 * (2 + rng.below(4)),
            );
            t.stub(
                tg::TEXTURE_POINTERS,
                false,
                3,
                0,
                out + 4 + 4 * (6 + rng.below(4)),
            );
            t.stub(tg::STATE_ELEMENT, true, 1, 1, rng.next());
            let n = rng.below(4);
            let blocks: Vec<u32> = (0..n.max(1))
                .map(|_| object(&mut t, &mut rng, 0x12, false))
                .collect();
            t.stub_seq(tg::RING_ALLOCATE, true, 1, 1, &blocks);
            let r = object(&mut t, &mut rng, 0x18, false);
            if rng.chance(2) {
                t.w32(r + 0x4c, 0);
            }
            let instances = object(&mut t, &mut rng, 8, false);
            t.w32(instances + 4, n | (rng.next() & 0x8000_0000));
            let refs = object(&mut t, &mut rng, 8, false);
            for k in [0u32, 1, 3] {
                if rng.chance(3) {
                    t.w32(refs + 4 * k, 0);
                }
            }
            // the state object (read at +8), or none
            let no_state = rng.chance(2);
            let state = object(&mut t, &mut rng, 4, no_state);
            t.w32(refs + 0x18, state);
            let material = object(&mut t, &mut rng, 0x10, false);
            if rng.chance(2) {
                t.w32(material + 0x38, 0);
            }
            if rng.chance(2) {
                t.w32(material + 0x28, rng.next() & 0x8000_0000);
            } else {
                let p = object(&mut t, &mut rng, 2, false);
                t.w32(material + 0x2c, p);
            }
            let no_extra = rng.chance(3);
            let extra = object(&mut t, &mut rng, 4, no_extra);
            if extra != 0 && rng.chance(3) {
                t.w32(extra + 4, 0);
            }
            let a = tg::MeshCommands {
                context: rng.next(),
                instances,
                extra,
                material,
                tag: rng.next(),
                refs,
                out,
                callback: if rng.chance(2) { 0 } else { rng.next() },
            };
            let args = [
                a.context,
                a.instances,
                a.extra,
                a.material,
                a.tag,
                a.refs,
                a.out,
                a.callback,
            ];
            run_framed(
                &mut t,
                &format!("{name} mesh commands #{i}"),
                0x005b_a380,
                Conv::Thiscall(r),
                &args,
                0x14,
                0,
                |g, ebp| {
                    let (eax, slot) = tg::bridge::mesh_commands(g, r, &a);
                    g.write(ebp + 0xc, slot);
                    eax
                },
            );
            cmds_seen += n;
            for s in [
                tg::GATHER_RECORDS,
                tg::TEXTURE_POINTERS,
                tg::STATE_ELEMENT,
                tg::RING_ALLOCATE,
            ] {
                t.unstub(s);
            }
        }
        assert!(cmds_seen > 40, "{name}: {cmds_seen}");
    }
}
