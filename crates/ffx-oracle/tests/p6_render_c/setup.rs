//! Continuation round 2, batch 12: the renderer's and render thread's
//! set-up, the viewport setters, the capture buffer and the visibility
//! check's jobs (`ffx_prender::p6_setup`) against their originals.

use ffx_oracle::Conv;
use ffx_prender::{p6_descriptor as descriptor, p6_encode as enc, p6_setup as su};

use super::encode::{answers, renderer, ring_check_stub};
use super::small::{float_bits, run_framed};
use super::{G, Out, Rng, Twins, object};

impl Twins {
    /// Page 0 mapped in both oracles (the size asserts store to address
    /// 0; an empty texture array reads its slot there).
    pub(super) fn map_page0(&mut self) {
        self.a.map_zeroed(0, 0x1000).unwrap();
        self.b.map_zeroed(0, 0x1000).unwrap();
    }

    /// A cdecl stub (`n` words logged) put in the import slot `slot`.
    fn import_cdecl(&mut self, slot: u32, n: u32, ret: u32) {
        let code = self.stub_code(slot, false, n, 0, ret);
        let at = self.rebase(slot);
        self.w32(at, code);
    }

    /// `fprintf`'s stand-in in its import slot: logs the file and the
    /// format, and the two numbers after the size messages' formats.
    fn fprintf_stub(&mut self) {
        let two = self.stub_code(su::FPRINTF, false, 2, 0, 0);
        let four = self.stub_code(su::FPRINTF + 1, false, 4, 0, 0);
        let at = self.a.alloc_code(&[0; 64]).unwrap();
        assert_eq!(self.b.alloc_code(&[0; 64]).unwrap(), at);
        let mut c = vec![];
        for msg in [su::RENDERER_VIEWPORT_MSG, su::THREAD_VIEWPORT_MSG] {
            c.extend([0x81, 0x7c, 0x24, 0x08]);
            c.extend(self.rebase(msg).to_le_bytes()); // cmp dword [esp+8], msg
            c.extend([0x0f, 0x84]);
            let next = at + c.len() as u32 + 4;
            c.extend(four.wrapping_sub(next).to_le_bytes()); // je four
        }
        c.push(0xe9);
        let next = at + c.len() as u32 + 4;
        c.extend(two.wrapping_sub(next).to_le_bytes()); // jmp two
        self.write(at, &c);
        let slot = self.rebase(su::FPRINTF);
        self.w32(slot, at);
    }

    /// A `memcpy(dst, src, n)` stand-in that copies and logs.
    fn memcpy_stub(&mut self) {
        // push esi; push edi; mov edi,[esp+0xc]; mov esi,[esp+0x10];
        // mov ecx,[esp+0x14]; rep movsb; pop edi; pop esi
        let pre = [
            0x56, 0x57, 0x8b, 0x7c, 0x24, 0x0c, 0x8b, 0x74, 0x24, 0x10, 0x8b, 0x4c, 0x24, 0x14,
            0xf3, 0xa4, 0x5f, 0x5e,
        ];
        self.stub_pre(su::MEMCPY, &pre, false, 3, 0, 0x0d57);
    }
}

#[test]
fn setup_small_objects_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0c01);
        t.map_page0();
        // a texture array's slot
        for i in 0..24u32 {
            let arr = object(&mut t, &mut rng, 6, false);
            let n = [0, 1, 3, 2][i as usize % 4];
            t.w32(arr + 0xc, n | (rng.next() & 0x8000_0000));
            if n > 1 {
                let slots = object(&mut t, &mut rng, 4 * n, false);
                if i % 3 == 0 {
                    t.w32(slots + 16 * (i % n), 0);
                }
                t.w32(arr + 0x10, slots);
            } else if n == 1 && i % 3 == 0 {
                t.w32(arr + 0x10, 0);
            } else if n == 0 {
                t.w32(0, if i % 8 == 0 { 0 } else { 0x55 });
                t.w32(8, 0x77);
            }
            let k = if n == 0 { (i / 4) % 2 } else { rng.below(4) };
            t.reset_log();
            t.check(
                &format!("{name} mark slot #{i}"),
                0x0059_d320,
                Conv::Thiscall(arr),
                &[k, rng.next()],
                |g| Out::U32(su::bridge::mark_slot(g, arr, k)),
            );
        }
        // two arrays freed
        t.stub(su::FREE, false, 1, 0, 0x0f4e_e000);
        for i in 0..12u32 {
            let obj = object(&mut t, &mut rng, 5, false);
            t.w32(
                obj + 0xc,
                rng.below(3) | if i % 3 == 0 { 0x8000_0000 } else { 0 },
            );
            t.w32(
                obj + 4,
                rng.below(3) | if i % 4 == 1 { 0x8000_0000 } else { 0 },
            );
            if i % 5 == 2 {
                t.w32(obj + 0x10, 0);
            }
            if i % 6 == 3 {
                t.w32(obj + 8, 0);
            }
            t.reset_log();
            t.check(
                &format!("{name} free arrays #{i}"),
                0x005b_0e40,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(su::bridge::free_arrays(g, obj)),
            );
        }
        t.unstub(su::FREE);
        // constructors
        t.stub(su::RING_CTOR, true, 0, 0, 0x11);
        for i in 0..3 {
            let this = object(&mut t, &mut rng, 0x18, false);
            let dev = rng.next();
            t.reset_log();
            t.check(
                &format!("{name} PRendererBase #{i}"),
                0x005b_4650,
                Conv::Thiscall(this),
                &[dev],
                |g| Out::U32(su::bridge::renderer_base_ctor(g, this, dev)),
            );
        }
        t.unstub(su::RING_CTOR);
        t.stub(0x005b_4650, true, 1, 1, 0x22);
        t.stub(su::SEMAPHORE_CTOR, true, 0, 0, 0x33);
        t.stub(su::VECTOR_CTOR, false, 5, 5, 0x44);
        t.stub(su::SURFACE_CTOR, true, 0, 0, 0x55);
        for i in 0..3 {
            let this = object(&mut t, &mut rng, 0x50, false);
            let dev = rng.next();
            t.reset_log();
            t.check(
                &format!("{name} PRenderer #{i}"),
                0x005b_45b0,
                Conv::Thiscall(this),
                &[dev],
                |g| Out::U32(su::bridge::renderer_ctor(g, this, dev)),
            );
        }
        t.unstub(0x005b_4650);
        t.unstub(su::SEMAPHORE_CTOR);
        t.unstub(su::VECTOR_CTOR);
        t.unstub(su::SURFACE_CTOR);
        // PModifierNetworkDynamicMesh's constructInstance
        t.stub(su::MESH_BASE_CTOR, true, 0, 0, 0x6600);
        for i in 0..4 {
            let obj = object(&mut t, &mut rng, 0x10, i == 0);
            t.reset_log();
            t.check(
                &format!("{name} mesh constructInstance #{i}"),
                0x005a_da70,
                Conv::Thiscall(super::DESC),
                &[obj],
                |g| {
                    Out::U8(descriptor::bridge::construct_with_zeroed(
                        g,
                        obj,
                        su::MESH_BASE_CTOR,
                        Some(su::MESH_VTABLE),
                        None,
                        &su::MESH_ZEROS,
                    ))
                },
            );
        }
        t.unstub(su::MESH_BASE_CTOR);
        // a node's three arrays visited
        for i in 0..12u32 {
            let skip = if i % 6 == 5 { 0x0101 } else { 0x0100 };
            let vt = t.vtable(4, &[(3, 0, skip), (1, 4, 0x1000 + i)]);
            let visitor = object(&mut t, &mut rng, 1, false);
            t.w32(visitor, vt);
            let node = object(&mut t, &mut rng, 4, false);
            t.w32(node + 4, rng.below(3));
            t.w32(node + 8, rng.below(3));
            let c = rng.below(2) | rng.below(2) << 16;
            t.w32(node + 0xc, c);
            t.reset_log();
            t.check(
                &format!("{name} visit arrays #{i}"),
                0x005c_5170,
                Conv::Cdecl,
                &[node, visitor],
                |g| Out::U32(su::bridge::visit_arrays(g, node, visitor)),
            );
        }
    }
}

#[test]
fn setup_capture_buffer_and_rings_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0c02);
        t.memcpy_stub();
        // the material capture buffer
        for i in 0..16u32 {
            let size = 0x60 + 4 * rng.below(0x10);
            let dst = if i % 7 == 6 { 0 } else { t.zeros(0x100) };
            t.stub(su::RING_ALLOC, true, 1, 1, dst);
            let desc = object(&mut t, &mut rng, size / 4, false);
            t.w32(desc, size);
            let n = rng.below(5);
            let entries = object(&mut t, &mut rng, 4 * n, false);
            t.w32(desc + 8, n | (rng.next() & 0x8000_0000));
            t.w32(desc + 0xc, entries);
            for k in 0..n {
                let e = entries + 16 * k;
                let kinds = [0x42u8, 0xd9, 0xda, 0xdb, 0xdc, 0x12, 0x43, 0x44, 0x41, 0x13];
                t.write(e + 2, &[kinds[rng.below(10) as usize]]);
                // slots apart (a texture word under another slot's kind
                // word would be a wild pointer)
                let off = 0x10 + 0x10 * k + 4 * rng.below(2);
                t.write(e + 8, &(off as u16).to_le_bytes());
                // the slot (in the copy): a kind word and a texture
                t.w32(desc + off, 0x41 + rng.below(5));
                let none = rng.chance(3);
                let tex = object(&mut t, &mut rng, 3, none);
                t.w32(desc + off + 0xc, tex);
            }
            let this = object(&mut t, &mut rng, 0x10, false);
            t.reset_log();
            t.check(
                &format!("{name} material capture #{i}"),
                0x005b_7f30,
                Conv::Thiscall(this),
                &[desc],
                |g| Out::U32(su::bridge::material_capture(g, this, desc)),
            );
            t.unstub(su::RING_ALLOC);
        }
        // the rings' set-up
        t.stub(su::SEMAPHORE_WAIT, true, 0, 0, 0);
        t.stub(su::RING_NEXT, true, 0, 0, 0);
        for i in 0..12u32 {
            let seq: &[u32] = match i % 4 {
                0 => &[5],
                1 => &[0, 9],
                _ => &[0, 0],
            };
            t.stub_seq(su::RING_INIT, true, 3, 3, seq);
            let this = object(&mut t, &mut rng, 0x18, false);
            if i % 3 == 0 {
                t.w32(this + 0x44, 0);
            }
            let args = [
                rng.next(),
                rng.next(),
                rng.below(0x10_0000),
                rng.below(0x10_0000),
                1 + rng.below(4),
            ];
            t.reset_log();
            t.check(
                &format!("{name} init rings #{i}"),
                0x005b_8e80,
                Conv::Thiscall(this),
                &args,
                |g| {
                    Out::U32(su::bridge::init_rings(
                        g, this, args[0], args[1], args[2], args[3], args[4],
                    ))
                },
            );
            t.unstub(su::RING_INIT);
        }
        t.unstub(su::SEMAPHORE_WAIT);
        t.unstub(su::MEMCPY);
    }
}

#[test]
fn setup_render_thread_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0c03);
        t.map_page0();
        // initialize: the thread's set-up in the frame
        t.stub(su::EVENT_INIT, true, 1, 1, 0);
        t.stub(su::THREAD_INIT_CTOR, true, 1, 1, 0x7100);
        for i in 0..10u32 {
            let seq: &[u32] = match i % 5 {
                0 => &[3],
                1 => &[0, 4],
                _ => &[0, 0],
            };
            t.stub_seq(su::SEMAPHORE_INIT, true, 2, 2, seq);
            t.stub(su::THREAD_START, true, 1, 1, if i % 5 == 2 { 6 } else { 0 });
            let this = object(&mut t, &mut rng, 0x5c, false);
            t.w32(this + 0xc, if i % 5 == 4 { 1 } else { rng.next() & !1 });
            run_framed(
                &mut t,
                &format!("{name} initialize #{i}"),
                0x0061_5140,
                Conv::Thiscall(this),
                &[],
                0x18,
                0x18,
                |g, ebp| su::bridge::thread_initialize(g, this, ebp - 0x18),
            );
            t.unstub(su::SEMAPHORE_INIT);
            t.unstub(su::THREAD_START);
        }
        // sync: the command in the frame
        t.stub(su::GET_EVENT_FLAG, true, 0, 0, 0x00f1_a600);
        t.stub(su::SEMAPHORE_WAIT, true, 0, 0, 0);
        t.stub(su::EVENT_SIGNAL, true, 0, 0, 0);
        t.stub(su::EVENT_WAIT, true, 0, 0, 0);
        t.stub(su::RELEASE_EVENT_FLAG, true, 1, 1, 0);
        for i in 0..10u32 {
            let this = object(&mut t, &mut rng, 0x58, false);
            t.w32(this + 0xc, if i % 4 == 3 { 2 } else { 1 });
            t.w32(this + 0x154, [0, 15, 7, 14][i as usize % 4]);
            run_framed(
                &mut t,
                &format!("{name} sync #{i}"),
                0x0061_5ea0,
                Conv::Thiscall(this),
                &[],
                0x4c,
                0x48,
                |g, ebp| su::bridge::thread_sync(g, this, ebp - 0x4c),
            );
        }
        // the state's viewport, with the size assert
        let iob = t.zeros(0x80);
        t.import_cdecl(su::IOB, 0, iob);
        t.fprintf_stub();
        t.import_cdecl(su::FFLUSH, 1, 0);
        t.stub(su::SET_VIEWPORT_STATE, true, 4, 4, 0x5e7);
        for i in 0..12u32 {
            let state = object(&mut t, &mut rng, 4, false);
            let sides = [0x8000, 0x8001, 0x10, 0xffff_ffff];
            let (w, h) = (sides[i as usize % 4], sides[(i as usize / 4) % 4]);
            let (x, y) = (rng.next(), rng.next());
            t.w32(0, 0xdead);
            t.reset_log();
            t.check(
                &format!("{name} thread viewport #{i}"),
                0x0061_5de0,
                Conv::Thiscall(state),
                &[x, y, w, h],
                |g| Out::U32(su::bridge::thread_set_viewport(g, state, x, y, w, h)),
            );
        }
        // the renderer's viewport: a command, as the encoders
        let tab = t.zeros(0x40);
        let check = ring_check_stub(&mut t, tab);
        t.jump(enc::RING_CHECK, check);
        let submit = t.stub_code(0x005b_6c50, true, 0, 0, 0x77);
        for i in 0..30u32 {
            let cmd = if i % 7 == 6 { 0 } else { t.zeros(0x48) };
            t.stub(enc::RING_ALLOCATE, true, 1, 1, cmd);
            let r = renderer(&mut t, &mut rng, submit);
            answers(&mut t, &mut rng, r, tab);
            let w = if i % 5 == 4 {
                0x9000
            } else {
                rng.below(0x8001)
            };
            let args = [rng.next(), rng.next(), w, rng.below(0x8001)];
            t.w32(0, 0xdead);
            run_framed(
                &mut t,
                &format!("{name} renderer viewport #{i}"),
                0x005b_c060,
                Conv::Thiscall(r),
                &args,
                8,
                8,
                |g, ebp| su::bridge::renderer_set_viewport(g, r, ebp - 8, args),
            );
            t.unstub(enc::RING_ALLOCATE);
        }
    }
}

#[test]
fn setup_visibility_jobs_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0c04);
        t.memcpy_stub();
        let pool_vt = t.vtable(8, &[(5, 1, 0)]);
        // slot 7 (the wait) changes the job's link word: the unlink
        // stores the word as it is after the wait
        let wait = t.stub_code(0x7700_0007, true, 1, 1, 0);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0x83, 0x70, 0x40, 0x10]); // xor dword [eax+0x40], 0x10
        c.push(0xe9);
        let at = t.a.alloc_code(&[0; 16]).unwrap();
        assert_eq!(t.b.alloc_code(&[0; 16]).unwrap(), at);
        c.extend(wait.wrapping_sub(at + 13).to_le_bytes()); // jmp wait
        t.write(at, &c);
        t.w32(pool_vt + 4 * 7, at);
        let pool = object(&mut t, &mut rng, 1, false);
        t.w32(pool, pool_vt);
        t.stub(su::THREAD_POOL, false, 0, 0, pool);
        t.stub(su::RING_SINGLETON, false, 0, 0, 0x00c0_ffee);
        // the occluders' build
        t.stub(su::SORT_OCCLUDERS, false, 5, 0, 0x50);
        t.stub(su::BUILD_OCCLUDERS, false, 4, 0, 0x51);
        for i in 0..10u32 {
            let this = object(&mut t, &mut rng, 0x10, false);
            let mut next = 0;
            let mut total = 0;
            for _ in 0..[0, 1, 3, 2][i as usize % 4] {
                let node = object(&mut t, &mut rng, 4, false);
                let n = if i % 5 == 4 { 0 } else { rng.below(4) };
                total += n;
                let entries = object(&mut t, &mut rng, 2 * n, false);
                let owner = object(&mut t, &mut rng, 2, false);
                t.w32(node, next);
                t.w32(node + 4, owner);
                t.w32(node + 8, n | (rng.next() & 0x8000_0000));
                t.w32(node + 0xc, entries);
                next = node;
            }
            t.w32(this + 0x20, next);
            let (b1, b2) = (t.zeros(0x10 + 8 * total as usize), t.zeros(0x420));
            t.w32(b1 + 4, 0x1234);
            t.w32(b1 + 8, 0x5678);
            t.stub_seq(su::RING_ALLOCATE, true, 1, 1, &[b1, b2]);
            t.reset_log();
            t.check(
                &format!("{name} build occluders #{i}"),
                0x009f_2fd0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(su::bridge::build_occluders(g, this)),
            );
            t.unstub(su::RING_ALLOCATE);
        }
        // the jobs' wait and count
        for i in 0..10u32 {
            let this = object(&mut t, &mut rng, 0x10, false);
            let mut next = 0;
            for _ in 0..rng.below(5) {
                let job = object(&mut t, &mut rng, 0x11, false);
                let end = rng.next();
                let d = match rng.below(4) {
                    0 => 0,
                    1 => 12 * rng.below(50),
                    2 => 0u32.wrapping_sub(12 * rng.below(50)),
                    _ => rng.below(100),
                };
                t.w32(job + 0x30, end);
                t.w32(job + 0x2c, end.wrapping_add(d));
                let none = rng.chance(3);
                let owner = object(&mut t, &mut rng, 2, none);
                t.w32(job + 0xc, owner);
                t.w32(job + 0x40, next);
                next = job;
            }
            t.w32(this + 0x18, next);
            t.reset_log();
            t.check(
                &format!("{name} wait and count #{i}"),
                0x009f_30d0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(su::bridge::wait_and_count(g, this)),
            );
        }
        // the object jobs
        let bias = t.rebase(su::DEPTH_BIAS_COUNT);
        for (i, n) in [0u32, 5, 399, 400, 401, 850].into_iter().enumerate() {
            t.w32(bias, rng.next());
            let jobs: Vec<u32> = (0..3).map(|_| t.zeros(0x48)).collect();
            t.stub_seq(su::JOB_CREATE, false, 0, 0, &jobs);
            let block = t.zeros(0x20);
            t.stub(su::RING_ALLOCATE, true, 1, 1, block);
            let this = object(&mut t, &mut rng, 0x10, false);
            for k in 2..6 {
                let f = float_bits(&mut rng);
                t.w32(this + 4 * k, f);
            }
            let (c1, c2) = (
                object(&mut t, &mut rng, 1, false),
                object(&mut t, &mut rng, 1, false),
            );
            t.w32(this + 0x24, c1);
            t.w32(this + 0x28, c2);
            let (objects, stride) = (rng.next(), rng.below(0x100));
            t.reset_log();
            let what = format!("{name} start jobs for objects #{i}");
            let addr = t.rebase(0x009f_4000);
            t.a.call(addr, Conv::Thiscall(this), &[objects, n, stride])
                .unwrap_or_else(|e| panic!("{what}: {e}"));
            su::bridge::start_jobs_for_objects(&mut G(&mut t.b), this, objects, n, stride);
            t.diff(&what);
            t.unstub(su::RING_ALLOCATE);
            t.unstub(su::JOB_CREATE);
        }
        // the cursor's advance
        t.stub(su::SKIP_EMPTY, true, 0, 0, 0);
        for i in 0..24u32 {
            let stride = [8u32, 12, 16][i as usize % 3];
            let (k, m) = (1 + rng.below(4), 1 + rng.below(4));
            let region = t.zeros((stride * (k + m + 3)) as usize);
            let node1 = region + stride * k;
            let node2 = node1 + stride * (m + 1);
            t.w32(node1, node2);
            t.w32(node2, 0);
            let cursor = object(&mut t, &mut rng, 8, false);
            // at the block's end now and then (its count is then `left`)
            let skip = if i % 8 == 3 { k } else { rng.below(k) };
            let (end, left, n) = match i % 6 {
                // past the last block: all that is left at once
                5 => {
                    let left = rng.below(5);
                    (0, left, left)
                }
                // as many left as asked for (some ending on a block end)
                1 | 4 => {
                    let n = 1 + rng.below(k - skip.min(k - 1) + m);
                    (node1, n, n)
                }
                _ => (node1, 100, rng.below(k - skip.min(k - 1) + m + 1)),
            };
            t.w32(cursor + 0xc, region + stride * skip);
            t.w32(cursor + 0x10, end);
            t.w32(cursor + 0x14, stride);
            t.w32(cursor + 0x18, left);
            t.reset_log();
            let what = format!("{name} advance #{i}");
            let addr = t.rebase(0x009f_3dc0);
            t.a.call(addr, Conv::Thiscall(cursor), &[n])
                .unwrap_or_else(|e| panic!("{what}: {e}"));
            su::bridge::advance(&mut G(&mut t.b), cursor, n);
            t.diff(&what);
        }
        t.unstub(su::SKIP_EMPTY);
        t.unstub(su::THREAD_POOL);
        t.unstub(su::RING_SINGLETON);
        t.unstub(su::MEMCPY);
    }
}
