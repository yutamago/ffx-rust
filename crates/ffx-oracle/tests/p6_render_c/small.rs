//! Continuation round 2, batch 11: the small engine functions
//! (`ffx_prender::p6_small`) against their originals.

use ffx_oracle::{Conv, Oracle};
use ffx_prender::p6_small as sm;

use super::encode::{answers, float_word, renderer, ring_check_stub};
use super::{G, Out, Rng, Twins, object};

impl Twins {
    /// A recording stub for `sym` that first runs `pre` (code that keeps
    /// `ecx` and the stack), then logs and returns as [`Twins::stub_code`].
    pub(super) fn stub_pre(
        &mut self,
        sym: u32,
        pre: &[u8],
        this: bool,
        n: u32,
        pops: u32,
        ret: u32,
    ) {
        let tail = self.stub_code(sym, this, n, pops, ret);
        let mut c = pre.to_vec();
        let at = self.a.alloc_code(&[0; 64]).unwrap();
        assert_eq!(self.b.alloc_code(&[0; 64]).unwrap(), at);
        c.push(0xe9);
        let next = at + c.len() as u32 + 4;
        c.extend(tail.wrapping_sub(next).to_le_bytes()); // jmp tail
        self.write(at, &c);
        self.jump(sym, at);
    }

    /// A stub whose `k`-th call returns `seq[k]` (the last one after
    /// that): logs as [`Twins::stub_code`] with `tag`.
    pub(super) fn stub_seq(&mut self, sym: u32, this: bool, n: u32, pops: u32, seq: &[u32]) {
        let tab = self.zeros(4 + 4 * seq.len());
        for (k, &v) in seq.iter().enumerate() {
            self.w32(tab + 4 + 4 * k as u32, v);
        }
        let last = seq.len() as u32 - 1;
        // the logging tail returns a dummy; the code after it is ours
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], tag
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xa1);
        c.extend(tab.to_le_bytes()); // mov eax, [tab]
        c.push(0x3d);
        c.extend(last.to_le_bytes()); // cmp eax, last
        c.extend([0x74, 0x06]); // je +6
        c.extend([0xff, 0x05]);
        c.extend(tab.to_le_bytes()); // inc dword [tab]
        c.extend([0x8b, 0x04, 0x85]);
        c.extend((tab + 4).to_le_bytes()); // mov eax, [tab+4+eax*4]
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let at = self.code(&c);
        self.jump(sym, at);
    }

    /// A stdcall stub (`n` words) put in the import slot `slot`.
    pub(super) fn import_stub(&mut self, slot: u32, n: u32, ret: u32) {
        let code = self.stub_code(slot, false, n, n, ret);
        let at = self.rebase(slot);
        self.w32(at, code);
    }

    /// A vtable of `n` slots whose `slot` is a recording stub; returns
    /// the vtable.
    pub(super) fn vtable(&mut self, n: u32, slots: &[(u32, u32, u32)]) -> u32 {
        let vt = self.zeros(4 * n as usize);
        for &(slot, words, ret) in slots {
            let s = self.stub_code(0x7700_0000 + slot, true, words, words, ret);
            self.w32(vt + 4 * slot, s);
        }
        vt
    }

    pub(super) fn reset_log(&mut self) {
        let log = self.log;
        self.w32(log, 0);
    }
}

/// `fld dword/qword [cell]` (code before a stub's logging).
pub(super) fn fld(cell: u32, qword: bool) -> Vec<u8> {
    let mut c = vec![if qword { 0xdd } else { 0xd9 }, 0x05];
    c.extend(cell.to_le_bytes());
    c
}

/// Runs the original at `f` (`conv`, `args`) on `a` and `port` on `b`
/// with the original's `ebp` (the port's frame at the same address, its
/// calls below `lo`); compares `eax`, the bytes `[lo, ebp)` and the
/// argument slots `[ebp + 8, ebp + 8 + 4 * args)`, and every byte
/// written.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_framed(
    t: &mut Twins,
    what: &str,
    f: u32,
    conv: Conv,
    args: &[u32],
    below: u32,
    cmp: usize,
    port: impl FnOnce(&mut G, u32) -> u32,
) {
    let top = t.a.stack_range().end - 0x100;
    let ebp = top - 4 * args.len() as u32 - 8;
    let lo = ebp - below;
    let junk: Vec<u8> = (0..below + 8 + 4 * args.len() as u32)
        .map(|k| (k * 37 + 11) as u8)
        .collect();
    t.write(lo, &junk);
    t.reset_log();
    let addr = t.rebase(f);
    let ra =
        t.a.call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
    // the port's side never pushed the arguments: as the original's
    for (k, &w) in args.iter().enumerate() {
        t.b.write(ebp + 8 + 4 * k as u32, &w.to_le_bytes()).unwrap();
    }
    t.b.set_call_top(Some(lo - 0x100));
    let rb = port(&mut G(&mut t.b), ebp);
    t.b.set_call_top(None);
    assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
    assert_eq!(ra.eax, rb, "{what}: eax");
    let fa = Oracle::read(&mut t.a, lo, cmp).unwrap();
    let fb = Oracle::read(&mut t.b, lo, cmp).unwrap();
    assert_eq!(fa, fb, "{what}: the frame");
    let n = 4 * args.len();
    let sa = Oracle::read(&mut t.a, ebp + 8, n).unwrap();
    let sb = Oracle::read(&mut t.b, ebp + 8, n).unwrap();
    assert_eq!(sa, sb, "{what}: the argument slots");
    t.diff(what);
}

/// Bits of a float the originals copy or add: ordinary values mostly.
pub(super) fn float_bits(rng: &mut Rng) -> u32 {
    if rng.chance(4) {
        float_word(rng)
    } else {
        let e = 0x3c00_0000 + (rng.below(0x0800) << 16);
        e | (rng.next() & 0x00ff_ffff) | (rng.next() & 0x8000_0000)
    }
}

#[test]
fn small_thunks_and_forwarders_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0b11);
        for &(f, target, words) in sm::THUNKS {
            t.stub(target, true, words, words, 0x5150_0000 | f >> 4);
            for i in 0..4 {
                let this = object(&mut t, &mut rng, 2, i == 3);
                let args: Vec<u32> = (0..words).map(|_| rng.next()).collect();
                t.reset_log();
                t.check(
                    &format!("{name} thunk {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(this),
                    &args,
                    |g| Out::U32(sm::bridge::thunk(g, target, this, &args)),
                );
            }
            t.unstub(target);
        }
        // isKeyDown: a float through the application's input +0x2b8
        let cell = t.zeros(4);
        let code = fld(cell, false);
        t.stub_pre(sm::KEY_VALUE, &code, true, 2, 2, 0x99);
        for i in 0..12 {
            let app = object(&mut t, &mut rng, 0xb0, false);
            let v = float_bits(&mut rng);
            t.w32(cell, v);
            let (key, k) = (rng.next(), rng.below(8));
            t.reset_log();
            let what = format!("{name} isKeyDown #{i}");
            let addr = t.rebase(0x0062_8460);
            let ra = t.a.call(addr, Conv::Thiscall(app), &[key, k]).unwrap();
            let rb = sm::bridge::is_key_down(&mut G(&mut t.b), app, key, k);
            let st0 = ra.st0_f32().expect("ST0");
            assert_eq!(st0.to_bits(), rb.to_bits(), "{what}: ST0");
            t.diff(&what);
        }
        t.unstub(sm::KEY_VALUE);
    }
}

#[test]
fn small_sync_and_ring_room_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0b12);
        // sync: Sleep moves the counter on
        let counter = t.zeros(4);
        let mut pre = vec![0xff, 0x05];
        pre.extend(counter.to_le_bytes()); // inc dword [counter]
        t.stub_pre(sm::SLEEP, &pre, false, 1, 0, 0x44);
        for i in 0..12u32 {
            let mesh = object(&mut t, &mut rng, 0xc, i == 0);
            if mesh != 0 {
                let c0 = rng.next();
                t.w32(counter, c0);
                let p = if i % 4 == 1 { 0 } else { counter };
                t.w32(mesh + 0x28, p);
                t.w32(mesh + 0x2c, c0.wrapping_add(i % 3));
            }
            t.reset_log();
            let addr = t.rebase(0x005a_fca0);
            t.a.call(addr, Conv::Stdcall, &[mesh])
                .unwrap_or_else(|e| panic!("sync #{i}: {e}"));
            sm::bridge::sync(&mut G(&mut t.b), mesh);
            t.diff(&format!("{name} sync #{i}"));
        }
        t.unstub(sm::SLEEP);

        // the renderer's room check
        let tab = t.zeros(0x40);
        let check = ring_check_stub(&mut t, tab);
        t.jump(sm::RING_CHECK, check);
        for i in 0..60 {
            let r = renderer(&mut t, &mut rng, 0);
            // a ring whose cursor is at its end
            for (cur, end) in [(0x24, 0x28), (0x38, 0x3c)] {
                if rng.chance(4) {
                    let c: u32 = super::Mem::read(&t.b, r + cur);
                    t.w32(r + end, c);
                }
            }
            answers(&mut t, &mut rng, r, tab);
            let (bytes, n) = (rng.below(0x400), rng.below(5));
            run_framed(
                &mut t,
                &format!("{name} hasRoom #{i}"),
                0x005b_6a70,
                Conv::Thiscall(r),
                &[bytes, n],
                4,
                4,
                |g, ebp| sm::bridge::has_room(g, r, ebp - 4, ebp + 8, bytes, n),
            );
        }
        t.unstub(sm::RING_CHECK);
    }
}

#[test]
fn small_objects_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0b13);
        // the delete-children visit: the visitor in the frame
        t.stub(sm::VISITOR_SINGLETON, true, 0, 0, 0x0bad_5eed);
        for i in 0..6 {
            let vt = t.vtable(16, &[(15, 2, 0x1234_5600 | i)]);
            let node = object(&mut t, &mut rng, 4, false);
            t.w32(node, vt);
            let a = rng.next();
            run_framed(
                &mut t,
                &format!("{name} visitDeleteChildren #{i}"),
                0x005c_5e50,
                Conv::Thiscall(node),
                &[a],
                0x14,
                8,
                |g, ebp| sm::bridge::visit_delete(g, node, ebp - 0x14, a),
            );
        }
        t.unstub(sm::VISITOR_SINGLETON);

        // GetType<PModifier>
        t.stub(sm::NAMED_SEMANTIC_CTOR, true, 5, 5, 0x66);
        t.stub(sm::ATEXIT, false, 1, 0, 0);
        for i in 0..6 {
            let guard = t.rebase(sm::MODIFIER_GUARD);
            t.w32(guard, rng.next() & !1 | u32::from(i % 2 == 1));
            t.reset_log();
            t.check(
                &format!("{name} GetType<PModifier> #{i}"),
                0x005c_7fe0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(sm::bridge::modifier_type(g)),
            );
        }
        t.unstub(sm::NAMED_SEMANTIC_CTOR);

        // the file seek
        t.stub(sm::VFILE_SEEK, true, 2, 2, 0x1111);
        t.import_stub(sm::SET_FILE_POINTER, 4, 0x2222);
        for i in 0..9 {
            let file = object(&mut t, &mut rng, 2, false);
            let v = if i % 3 == 0 {
                object(&mut t, &mut rng, 1, false)
            } else {
                0
            };
            t.w32(file + 4, v);
            if i % 3 == 1 {
                t.w32(file, u32::MAX);
            }
            let (d, m) = (rng.next(), 1 + rng.below(2));
            t.reset_log();
            t.check(
                &format!("{name} file seek #{i}"),
                0x0060_80e0,
                Conv::Thiscall(file),
                &[d, m],
                |g| Out::U32(sm::bridge::file_seek(g, file, d, m)),
            );
        }
        t.unstub(sm::VFILE_SEEK);

        // the 0x40-byte element array
        t.stub(sm::ELEMENT_DTOR, true, 0, 0, 0x33);
        t.stub(sm::FREE, false, 1, 0, 0x4567_89ab);
        for i in 0..12 {
            let arr = object(&mut t, &mut rng, 2, false);
            let n = rng.below(4);
            let data = if i % 4 == 0 {
                0
            } else {
                t.zeros(0x40 * n as usize + 4)
            };
            // bit 30 alone: owned (only with no data: a count that large)
            let view = match i % 3 {
                0 => 0x8000_0000,
                1 if data == 0 => 0x4000_0000,
                _ => 0,
            };
            t.w32(arr, n | view);
            t.w32(arr + 4, data);
            t.reset_log();
            t.check(
                &format!("{name} array destroy #{i}"),
                0x0061_41a0,
                Conv::Thiscall(arr),
                &[],
                |g| Out::U32(sm::bridge::array_destroy(g, arr)),
            );
        }
        t.unstub(sm::ELEMENT_DTOR);

        // the release list
        t.stub(sm::FINISH, true, 1, 1, 0x55);
        for i in 0..8 {
            let list = t.zeros(0x2010);
            for k in 0..4 {
                t.w32(list + 4 * k, rng.next());
            }
            t.write(list + 1, &[u8::from(i % 4 != 0)]);
            let n = rng.below(4);
            t.w32(list + 0x2008, n);
            for k in 0..n {
                let vt = t.vtable(8, &[(7, 1, 0)]);
                let obj = object(&mut t, &mut rng, 2, false);
                t.w32(obj, vt);
                t.w32(list + 8 + 8 * k, obj);
                t.w32(list + 12 + 8 * k, rng.next());
            }
            let ctx = object(&mut t, &mut rng, 1, false);
            t.reset_log();
            t.check(
                &format!("{name} flush #{i}"),
                0x0061_4800,
                Conv::Thiscall(list),
                &[ctx],
                |g| Out::U32(sm::bridge::flush(g, list, ctx)),
            );
        }

        // a new 3-word node
        for i in 0..4 {
            let node = t.zeros(0x14);
            t.stub(sm::NEW, false, 1, 0, node);
            let value = object(&mut t, &mut rng, 1, false);
            t.reset_log();
            t.check(
                &format!("{name} new node #{i}"),
                0x0062_5880,
                Conv::Thiscall(value),
                &[],
                |g| Out::U32(sm::bridge::new_node(g, value)),
            );
            t.unstub(sm::NEW);
        }

        // the list entry handed on
        for (i, found) in [0u32, 4, 0x0100_0004, 0x2000_0000].into_iter().enumerate() {
            t.stub(sm::LOOKUP, true, 0, 0, found);
            t.stub(sm::LOOKUP_USE, true, 1, 1, 0x7777_7700 + i as u32);
            for p in [0, 0x00c0_ffee] {
                t.reset_log();
                t.check(
                    &format!("{name} lookupUse {found:#x} {p:#x}"),
                    0x0062_6c40,
                    Conv::Stdcall,
                    &[p],
                    |g| Out::U32(sm::bridge::lookup_use(g, p)),
                );
            }
            t.unstub(sm::LOOKUP);
            t.unstub(sm::LOOKUP_USE);
        }

        // the deleting destructors
        for &(f, vtable, field, base) in sm::DELETING {
            t.stub(sm::TARGET_RELEASE, false, 1, 0, 0x88);
            t.stub(base, true, 0, 0, 0x99);
            for i in 0..4 {
                let this = object(&mut t, &mut rng, field / 4 + 1, false);
                let flags = rng.next() & !1 | (i & 1);
                t.reset_log();
                t.check(
                    &format!("{name} deleting dtor {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(this),
                    &[flags],
                    |g| {
                        Out::U32(sm::bridge::deleting_dtor(
                            g,
                            this,
                            flags,
                            (vtable, field, base),
                        ))
                    },
                );
            }
            t.unstub(base);
            t.unstub(sm::TARGET_RELEASE);
        }
        t.unstub(sm::FREE);
        t.unstub(sm::ATEXIT);
    }
}

#[test]
fn small_input_and_timer_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0b14);
        // PInput::Update
        let head = t.rebase(sm::INPUT_LIST);
        for i in 0..12u32 {
            let dirty = t.rebase(sm::INPUT_DIRTY);
            t.write(dirty, &[u8::from(i % 2 == 0)]);
            let com = t.stub_code(
                0x00c0_0010,
                false,
                5,
                5,
                [0x8000_0005, 0, 1, 1][(i as usize / 2) % 4],
            );
            let cvt = t.zeros(0x20);
            t.w32(cvt + 0x10, com);
            let device = object(&mut t, &mut rng, 1, false);
            t.w32(device, cvt);
            let dev = t.rebase(sm::INPUT_DEVICE);
            t.w32(dev, device);
            // a chain of inputs (links at +4), ended by the head, 0 or 4
            let n = rng.below(4);
            let end = match i % 3 {
                0 => head,
                1 => 0,
                _ => 4,
            };
            let mut next = end;
            for _ in 0..n {
                let vt = t.vtable(6, &[(5, 0, 0)]);
                let input = object(&mut t, &mut rng, 2, false);
                t.w32(input, vt);
                t.w32(input + 4, next);
                next = input + 4;
            }
            t.w32(head, next);
            t.reset_log();
            t.check(
                &format!("{name} PInput::Update #{i}"),
                0x0062_5770,
                Conv::Cdecl,
                &[],
                |g| Out::U32(sm::bridge::input_update(g)),
            );
        }

        // the timer step
        let now = t.zeros(8);
        let code = fld(now, true);
        t.stub_pre(sm::GET_TIME, &code, false, 0, 0, 0);
        for i in 0..40 {
            let timer = object(&mut t, &mut rng, 0x4a, false);
            let fixed = match i % 4 {
                0 => float_bits(&mut rng) & 0x7fff_ffff,
                1 => float_bits(&mut rng) | 0x8000_0000,
                2 => 0,
                _ => float_bits(&mut rng),
            };
            t.w32(timer + 0x3c, fixed);
            t.w32(timer + 0x20, float_bits(&mut rng));
            t.w32(timer + 0x28, float_bits(&mut rng));
            let start = t.rebase(sm::START_TIME);
            let s = f64::from(rng.below(1 << 20)) * 0.001_953_125;
            t.write(start, &s.to_bits().to_le_bytes());
            let v = s + f64::from(rng.below(1 << 24)) / 4096.0;
            t.write(now, &v.to_bits().to_le_bytes());
            t.reset_log();
            let addr = t.rebase(0x0062_8df0);
            let ra = t.a.call(addr, Conv::Thiscall(timer), &[]).unwrap();
            assert!(ra.st0_raw.is_none(), "timer: ST0 left");
            sm::bridge::timer_step(&mut G(&mut t.b), timer);
            t.diff(&format!("{name} timer step #{i}"));
        }
        t.unstub(sm::GET_TIME);
    }
}

#[test]
fn small_visibility_jobs_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0b15);
        // the job creation, waiting for room
        let pool_vt = t.vtable(8, &[(6, 0, 0), (7, 1, 0)]);
        let pool = object(&mut t, &mut rng, 1, false);
        t.w32(pool, pool_vt);
        t.stub(sm::THREAD_POOL, false, 0, 0, pool);
        t.stub(sm::SLEEP, false, 1, 0, 0);
        t.stub(sm::RING_SINGLETON, false, 0, 0, 0x00c0_ffee);
        for i in 0..6 {
            let block = t.zeros(0x48);
            let seq: Vec<u32> = (0..i % 3).map(|_| 0).chain([block]).collect();
            t.stub_seq(sm::RING_ALLOCATE, true, 1, 1, &seq);
            t.reset_log();
            t.check(
                &format!("{name} job create #{i}"),
                0x009f_09e0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(sm::bridge::job_create(g)),
            );
            t.unstub(sm::RING_ALLOCATE);
        }

        // the starters: the advance stub counts the cursor down
        let dec = [0xff, 0x49, 0x18]; // dec dword [ecx+0x18]
        for (f, count, advance, start) in [
            (
                0x009f_20f0,
                sm::OCCLUDER_COUNT,
                sm::OCCLUDER_ADVANCE,
                sm::OCCLUDER_START,
            ),
            (
                0x009f_2250,
                sm::OBJECT_COUNT,
                sm::OBJECT_ADVANCE,
                sm::OBJECT_START,
            ),
        ] {
            t.stub(count, true, 0, 0, 0x30);
            t.stub_pre(advance, &dec, true, 1, 1, 0x31);
            t.stub(start, true, 3, 3, 0x0003_2000 + f);
            for i in 0..8 {
                let this = object(&mut t, &mut rng, 0xc, false);
                let (c1, c2) = (
                    object(&mut t, &mut rng, 1, false),
                    object(&mut t, &mut rng, 1, false),
                );
                t.w32(this + 0x24, c1);
                t.w32(this + 0x28, c2);
                let cursor = object(&mut t, &mut rng, 8, false);
                let n = i % 4;
                t.w32(cursor + 0x18, n);
                let q = object(&mut t, &mut rng, 8, false);
                let p = object(&mut t, &mut rng, 4, false);
                t.w32(p + 0xc, q);
                t.w32(cursor + 8, p);
                t.reset_log();
                let what = format!("{name} start jobs {f:#x} #{i}");
                if f == 0x009f_20f0 && n == 0 {
                    // eax is the caller's then (its caller does not read it)
                    let addr = t.rebase(f);
                    t.a.call(addr, Conv::Thiscall(this), &[cursor]).unwrap();
                    sm::bridge::start_occluder_jobs(&mut G(&mut t.b), this, cursor);
                    t.diff(&what);
                    continue;
                }
                t.check(&what, f, Conv::Thiscall(this), &[cursor], |g| {
                    Out::U32(if f == 0x009f_20f0 {
                        sm::bridge::start_occluder_jobs(g, this, cursor)
                    } else {
                        sm::bridge::start_jobs_for_all_bounds(g, this, cursor)
                    })
                });
            }
            t.unstub(count);
            t.unstub(advance);
            t.unstub(start);
        }

        // the occluder cursor
        t.stub(sm::CURSOR_FIRST, true, 0, 0, 0);
        t.stub(sm::CURSOR_NOTE, true, 1, 1, 0);
        t.stub(sm::CURSOR_SKIP, true, 0, 0, 0);
        for i in 0..6 {
            let block = object(&mut t, &mut rng, 8, i % 3 == 0);
            t.stub(sm::CURSOR_LOOKUP, true, 1, 1, block);
            let cursor = object(&mut t, &mut rng, 8, false);
            let world = object(&mut t, &mut rng, 1, false);
            t.reset_log();
            t.check(
                &format!("{name} occluder cursor #{i}"),
                0x009f_23f0,
                Conv::Thiscall(cursor),
                &[world],
                |g| Out::U32(sm::bridge::occluder_cursor(g, cursor, world)),
            );
            t.unstub(sm::CURSOR_LOOKUP);
        }

        // the jobs released, and the destructor
        for i in 0..8 {
            let this = object(&mut t, &mut rng, 0x10, false);
            let mut next = 0;
            for _ in 0..rng.below(4) {
                let job = object(&mut t, &mut rng, 0x11, false);
                let none = rng.chance(3);
                let owner = object(&mut t, &mut rng, 2, none);
                t.w32(job + 0xc, owner);
                t.w32(job + 0x40, next);
                next = job;
            }
            t.w32(this + 0x18, next);
            t.reset_log();
            t.check(
                &format!("{name} release jobs #{i}"),
                0x009f_3590,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(sm::bridge::release_jobs(g, this)),
            );
        }
        t.stub(sm::RELEASE_JOBS, true, 0, 0, 0);
        for i in 0..8 {
            let this = object(&mut t, &mut rng, 0x10, false);
            for list in [0x1c, 0x20] {
                let mut next = 0;
                for _ in 0..rng.below(4) {
                    let node = object(&mut t, &mut rng, 2, false);
                    let obj = object(&mut t, &mut rng, 2, false);
                    t.w32(node, next);
                    t.w32(node + 4, obj);
                    next = node;
                }
                t.w32(this + list, next);
            }
            let entry = object(&mut t, &mut rng, 2, i % 3 == 0);
            t.w32(this + 0x38, entry);
            t.reset_log();
            t.check(
                &format!("{name} visibility dtor #{i}"),
                0x009f_26c0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(sm::bridge::visibility_dtor(g, this)),
            );
        }
        t.unstub(sm::RELEASE_JOBS);
        t.unstub(sm::THREAD_POOL);
        t.unstub(sm::SLEEP);
        t.unstub(sm::RING_SINGLETON);
    }
}

#[test]
fn small_render_targets_and_text_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0b16);
        // resize
        for i in 0..24u32 {
            let bound = if i % 2 == 0 { 0x0100 } else { 0x0101 };
            t.stub(sm::IS_BOUND[0], true, 0, 0, bound);
            t.stub(sm::UNBIND[0], true, 0, 0, if i % 4 == 1 { 7 } else { 0 });
            t.stub(
                sm::SET_DIMENSIONS,
                true,
                4,
                4,
                if i % 3 == 0 { 5 } else { 0 },
            );
            let tex = object(&mut t, &mut rng, 9, i % 11 == 10);
            let target = object(&mut t, &mut rng, 2, i == 23);
            if target != 0 {
                t.w32(target, tex);
                t.write(target + 4, &[u8::from(i % 7 == 6)]);
            }
            let (w, h) = (rng.below(3), rng.below(3));
            if tex != 0 {
                let same = i % 3 != 0;
                t.w32(tex + 0x1c, if same { w } else { rng.below(3) });
                t.w32(tex + 0x20, if same { h } else { rng.below(3) });
            }
            let force = [0, 1, 0x100, 0][i as usize % 4];
            t.reset_log();
            t.check(
                &format!("{name} target resize #{i}"),
                0x00a0_e4b0,
                Conv::Cdecl,
                &[target, w, h, force],
                |g| Out::U32(sm::bridge::target_resize(g, target, w, h, force)),
            );
            t.unstub(sm::IS_BOUND[0]);
            t.unstub(sm::UNBIND[0]);
            t.unstub(sm::SET_DIMENSIONS);
        }
        // release
        for i in 0..24u32 {
            for k in 0..3 {
                let bound = if (i + k) % 2 == 0 { 0x0200 } else { 0x0201 };
                t.stub(sm::IS_BOUND[k as usize], true, 0, 0, bound);
                t.stub(sm::UNBIND[k as usize], true, 0, 0, 0x0300 + k);
            }
            let tex = object(&mut t, &mut rng, 2, i % 6 == 5);
            let target = object(&mut t, &mut rng, 2, i == 0);
            if target != 0 {
                t.w32(target, tex);
                t.write(target + 4, &[(i % 5) as u8]);
            }
            t.reset_log();
            t.check(
                &format!("{name} target release #{i}"),
                0x00a0_e520,
                Conv::Cdecl,
                &[target],
                |g| Out::U32(sm::bridge::target_release(g, target)),
            );
            for k in 0..3 {
                t.unstub(sm::IS_BOUND[k]);
                t.unstub(sm::UNBIND[k]);
            }
        }
        // the depth of field's parameters
        for i in 0..4 {
            t.stub(sm::DOF_COMMON, true, 2, 2, if i == 1 { 0x15 } else { 0 });
            t.stub(sm::FIND_PARAMETER, true, 1, 1, 0x00ab_cdef);
            let this = object(&mut t, &mut rng, 0x3f0, false);
            let (a, effect) = (rng.next(), object(&mut t, &mut rng, 1, false));
            t.reset_log();
            t.check(
                &format!("{name} dof shaders #{i}"),
                0x00a1_a6c0,
                Conv::Thiscall(this),
                &[a, effect],
                |g| Out::U32(sm::bridge::dof_shaders(g, this, a, effect)),
            );
            t.unstub(sm::DOF_COMMON);
            t.unstub(sm::FIND_PARAMETER);
        }
        // the text colour
        t.stub(sm::SET_VALUE, true, 2, 2, 0x00_5e7);
        for i in 0..9 {
            t.stub(
                sm::FIND_PARAMETER,
                true,
                1,
                1,
                if i % 3 == 1 { 0 } else { 0x0bad_f00d },
            );
            let fx = object(&mut t, &mut rng, 2, i % 3 == 2);
            let material = object(&mut t, &mut rng, 1, false);
            t.w32(material, fx);
            let rgb = t.zeros(12);
            for k in 0..3 {
                let f = float_word(&mut rng);
                t.w32(rgb + 4 * k, f);
            }
            run_framed(
                &mut t,
                &format!("{name} text colour #{i}"),
                0x00a2_a650,
                Conv::Thiscall(material),
                &[rgb],
                0x10,
                0x10,
                |g, ebp| sm::bridge::text_color(g, material, ebp - 0x10, rgb),
            );
            t.unstub(sm::FIND_PARAMETER);
        }
        // the text set
        for i in 0..4 {
            t.stub(sm::TEXT_SET, true, 1, 1, if i == 1 { 3 } else { 0 });
            t.stub(sm::CREATE_MESH, true, 1, 1, 0x00e5_4000);
            let text = object(&mut t, &mut rng, 0x59, false);
            let s = t.alloc(
                &b"hello, Spira\0"[..(i as usize * 3)]
                    .iter()
                    .copied()
                    .chain([0])
                    .collect::<Vec<u8>>(),
            );
            t.w32(text + 0x160, s);
            let a = rng.next();
            t.reset_log();
            t.check(
                &format!("{name} text set #{i}"),
                0x00a2_ad80,
                Conv::Thiscall(text),
                &[a],
                |g| Out::U32(sm::bridge::text_set(g, text, a)),
            );
            t.unstub(sm::TEXT_SET);
            t.unstub(sm::CREATE_MESH);
        }
        // the Iggy start-up
        t.import_stub(sm::IGGY_INIT, 1, 0);
        t.import_stub(sm::IGGY_WARNING, 2, 0);
        t.import_stub(sm::IGGY_TRACE, 2, 0);
        t.reset_log();
        t.check(
            &format!("{name} Iggy init"),
            0x00a3_e6d0,
            Conv::Cdecl,
            &[],
            |g| Out::U32(sm::bridge::iggy_init(g)),
        );
    }
}
