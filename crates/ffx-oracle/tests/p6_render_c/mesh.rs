//! Continuation round 4: `PRendererBase::renderMeshInstance`
//! (`ffx_prender::p6_mesh`) against its original.

use ffx_bridge::Mem;
use ffx_oracle::{Conv, Oracle};
use ffx_prender::p6_mesh as mh;

use super::{G, Rng, Twins, object};

/// A recording stub: logs `tag`, `ecx` when `this`, stack words
/// `first..first + n` (up to 12: words past 6 go into a second entry
/// tagged `tag | 0x8000_0000`); pops `pops` words; per call takes from
/// its table the result and then, per `fills` entry `(argument, words,
/// conditional)`, a flag word when conditional (0: no store) and the
/// words it stores through the pointer in that stack argument. Returns
/// its address and the table cursor.
pub(super) fn gen_stub(
    t: &mut Twins,
    tag: u32,
    this: bool,
    (first, n): (u32, u32),
    pops: u32,
    fills: &[(u32, u32, bool)],
) -> (u32, u32) {
    assert!(n <= 12);
    let cursor = t.zeros(4);
    let log = t.log;
    let mut c = vec![];
    for (part, (from, count)) in [(first, n.min(6)), (first + 6, n.saturating_sub(6))]
        .into_iter()
        .enumerate()
    {
        if part == 1 && count == 0 {
            break;
        }
        c.push(0xa1);
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        let tag = if part == 0 { tag } else { tag | 0x8000_0000 };
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..count {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * (from + i)) as u8]); // mov edx, [esp+4+4(from+i)]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
    }
    c.extend([0x8b, 0x15]);
    c.extend(cursor.to_le_bytes()); // mov edx, [cursor]
    let mut k = 1u32;
    for &(arg, words, conditional) in fills {
        let mut body = vec![0x8b, 0x4c, 0x24, (4 + 4 * arg) as u8]; // mov ecx, [esp+4+4arg]
        let flag = k;
        if conditional {
            k += 1;
        }
        for w in 0..words {
            body.extend([0x8b, 0x42, (4 * (k + w)) as u8]); // mov eax, [edx+4k]
            body.extend([0x89, 0x41, (4 * w) as u8]); // mov [ecx+4w], eax
        }
        if conditional {
            c.extend([0x8b, 0x42, (4 * flag) as u8]); // mov eax, [edx+4flag]
            c.extend([0x85, 0xc0, 0x74, body.len() as u8]); // test eax, eax; jz past
        }
        c.extend(body);
        k += words;
    }
    c.extend([0x8b, 0x02]); // mov eax, [edx]
    c.extend([0x83, 0x05]);
    c.extend(cursor.to_le_bytes());
    c.push((4 * k) as u8); // add dword [cursor], 4k
    if pops == 0 {
        c.push(0xc3);
    } else {
        c.push(0xc2);
        c.extend(((pops * 4) as u16).to_le_bytes());
    }
    (t.code(&c), cursor)
}

/// [`gen_stub`] placed at `sym`; returns the cursor.
pub(super) fn gen_at(
    t: &mut Twins,
    sym: u32,
    this: bool,
    log: (u32, u32),
    pops: u32,
    fills: &[(u32, u32, bool)],
) -> u32 {
    let (f, cursor) = gen_stub(t, sym, this, log, pops, fills);
    t.jump(sym, f);
    cursor
}

/// A float that is an ordinary number (no NaN, no denormal).
fn nice(rng: &mut Rng) -> f32 {
    let v = (f32::from(rng.below(2001) as u16) - 1000.0) / 64.0;
    if v == 0.0 { 0.5 } else { v }
}

#[test]
#[allow(clippy::too_many_lines)]
fn render_mesh_instance_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_1301);
        let ring = object(&mut t, &mut rng, 4, false);
        t.stub(mh::RING_SINGLETON, false, 0, 0, ring);
        let ring_alloc = t.seq_stub(mh::RING_ALLOCATE, true, 1, 1);
        // computeJointMatrices(inst; mats): fills the matrices from `src`
        // (`n` words), as the original does after `_alloca`
        let src = t.zeros(4);
        let words = t.zeros(4);
        let mut c = t.log_code(mh::COMPUTE_JOINTS, true, 1);
        c.extend([0x56, 0x57]); // push esi; push edi
        c.extend([0x8b, 0x7c, 0x24, 0x0c]); // mov edi, [esp+0xc]
        c.extend([0x8b, 0x35]);
        c.extend(src.to_le_bytes()); // mov esi, [src]
        c.extend([0x8b, 0x0d]);
        c.extend(words.to_le_bytes()); // mov ecx, [words]
        c.extend([0xf3, 0xa5]); // rep movsd
        c.extend([0x5f, 0x5e]); // pop edi; pop esi
        c.extend([0xc2, 0x04, 0x00]); // ret 4
        let joints_fn = t.code(&c);
        t.jump(mh::COMPUTE_JOINTS, joints_fn);
        let upload = t.seq_stub(mh::UPLOAD_SKIN, true, 1, 1);
        let select = gen_at(&mut t, mh::SELECT_PASS, true, (0, 8), 8, &[(0, 2, false)]);
        let sizes = gen_at(
            &mut t,
            mh::INSTANCE_SIZES,
            false,
            (0, 9),
            0,
            &[(0, 1, false), (1, 1, false), (2, 1, false)],
        );
        let room = t.seq_stub(mh::HAS_ROOM, true, 2, 2);
        let fill = gen_at(&mut t, mh::FILL_BUFFERS, true, (0, 8), 8, &[(4, 1, true)]);
        let data = t.seq_stub(mh::DATA_ALLOCATE, true, 1, 1);
        let queue = gen_at(&mut t, mh::QUEUE_COMMANDS, true, (0, 8), 8, &[]);
        let flush = t.stub_code(0x7700_0002, true, 0, 0, 0x3e50_0003);
        let vt = t.zeros(16);
        t.w32(vt + 8, flush);
        let skip_effect = t.rebase(mh::SKIP_EFFECT);
        let skip_context = t.rebase(mh::SKIP_CONTEXT);
        let mut results = std::collections::BTreeSet::new();
        for i in 0..80u32 {
            let r = object(&mut t, &mut rng, 0x30, false);
            t.w32(r, vt);
            t.w32(r + 4, u32::from(i % 20 != 1) | (rng.next() & !1));
            let effect = match i % 10 {
                2 => 0,
                3 | 4 => skip_effect,
                _ => rng.next() | 1,
            };
            t.w32(r + 0x48, effect);
            let joints = match i % 16 {
                7 => 0x80 + rng.below(4),
                15 => 0x7f,
                _ => 1 + rng.below(6),
            };
            let segs = rng.below(6);
            let mesh = object(&mut t, &mut rng, 8, false);
            t.w32(mesh, segs | (rng.next() & 0x8000_0000));
            t.w32(mesh + 0x18, joints | (rng.next() & 0x8000_0000));
            let nctx = if i % 20 == 5 { 0 } else { 1 + rng.below(4) };
            let ctxs: Vec<u32> = (0..nctx)
                .map(|_| {
                    if rng.chance(12) {
                        return 0;
                    }
                    let c = object(&mut t, &mut rng, 4, false);
                    if rng.chance(2) {
                        t.w32(c + 8, skip_context);
                        let w = if rng.chance(2) {
                            skip_context
                        } else {
                            rng.next()
                        };
                        t.w32(c + 0xc, w);
                    }
                    c
                })
                .collect();
            let contexts = t.zeros(8);
            t.w32(contexts, nctx | (rng.next() & 0x8000_0000));
            if nctx == 1 {
                t.w32(contexts + 4, ctxs[0]);
            } else if nctx > 1 {
                let words: Vec<u8> = ctxs.iter().flat_map(|w| w.to_le_bytes()).collect();
                let at = t.alloc(&words);
                t.w32(contexts + 4, at);
            }
            let seg_block = t.zeros(0x6c * segs as usize + 4);
            for k in 0..segs {
                let s = seg_block + 0x6c * k;
                t.w32(s, rng.below(nctx.max(1)));
                t.w32(s + 4, rng.below(joints));
                t.write(s + 0x14, &[u8::from(rng.chance(6))]);
                t.w32(s + 0x1c, rng.below(5));
            }
            t.w32(mesh + 4, seg_block);
            let inst = object(&mut t, &mut rng, 0x24, false);
            t.w32(inst, mesh);
            let per_seg = object(&mut t, &mut rng, 9 * segs + 9, false);
            t.w32(inst + 0x28, per_seg);
            let y = rng.below(3);
            t.w32(inst + 0x80, rng.below(3));
            let x = if rng.chance(3) { 0 } else { rng.next() };
            // per-call tables
            let block = object(&mut t, &mut rng, 16 * (joints + 1) + 8, false);
            t.seq_set(ring_alloc, &[block]);
            let up = if i % 9 == 4 { 0x51 } else { 0 };
            t.seq_set(upload, &[up]);
            let mut sel = vec![];
            let mut sz = vec![];
            let mut rooms = vec![];
            let mut fills = vec![];
            let mut datas = vec![];
            for k in 0..4 * segs {
                let pass = if rng.chance(5) {
                    0
                } else {
                    let p = object(&mut t, &mut rng, 2, false);
                    t.w32(p + 4, rng.below(4) | (rng.next() & 0x8000_0000));
                    p
                };
                sel.extend([rng.next(), rng.next(), pass]);
                sz.extend([
                    rng.next(),
                    rng.below(0x400),
                    rng.below(0x400),
                    rng.below(0x400),
                ]);
                rooms.push((rng.next() & !0xff) | u32::from(rng.chance(2)));
                let e = if (i + k) % 23 == 11 { 0x33 } else { 0 };
                fills.extend([e, u32::from(!rng.chance(3)), rng.below(16)]);
                datas.push(rng.next());
            }
            t.seq_set(select, &sel);
            t.seq_set(sizes, &sz);
            t.seq_set(room, &rooms);
            t.seq_set(fill, &fills);
            t.seq_set(data, &datas);
            t.seq_set(queue, &vec![0x3e50_0002; 4 * segs as usize]);
            // the call, with the frame and the matrix array at the
            // original's addresses
            let args = [inst, contexts, x, y];
            let top = t.a.stack_range().end - 0x100;
            let ebp = top - 4 * args.len() as u32 - 8;
            let count = joints + 1;
            let base = (ebp - 0x198 - (count << 6)) & !0xf;
            let lo = base - 0x40;
            let junk: Vec<u8> = (0..ebp + 0x18 - lo).map(|k| (k * 37 + 11) as u8).collect();
            t.write(lo, &junk);
            let floats: Vec<u8> = (0..16 * (count - 1))
                .flat_map(|_| nice(&mut rng).to_le_bytes())
                .collect();
            let from = t.alloc(&floats);
            t.w32(src, from);
            t.w32(words, if count > 0x80 { 0 } else { 16 * (count - 1) });
            t.reset_log();
            let what = format!("{name} render mesh instance #{i}");
            let addr = t.rebase(0x005b_9a90);
            let ra =
                t.a.call(addr, Conv::Thiscall(r), &args)
                    .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
            for (k, &w) in args.iter().enumerate() {
                t.b.write(ebp + 8 + 4 * k as u32, &w.to_le_bytes()).unwrap();
            }
            t.b.set_call_top(Some(lo - 0x100));
            let mut alloca = |n: u32| (ebp - 0x198 - (n << 6)) & !0xf;
            let rb = mh::bridge::render_mesh_instance(
                &mut G(&mut t.b),
                r,
                ebp - 0x188,
                args,
                &mut alloca,
            );
            t.b.set_call_top(None);
            assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
            assert_eq!(ra.eax, rb, "{what}: eax");
            results.insert(rb);
            let fa = Oracle::read(&mut t.a, ebp - 0x188, 0x178).unwrap();
            let fb = Oracle::read(&mut t.b, ebp - 0x188, 0x178).unwrap();
            if fa != fb {
                let d: Vec<usize> = (0..fa.len()).filter(|&k| fa[k] != fb[k]).collect();
                panic!(
                    "{what}: the frame differs at ebp-{:#x}..",
                    0x188 - d[0] as u32
                );
            }
            let ma = Oracle::read(&mut t.a, base, 64 * count as usize).unwrap();
            let mb = Oracle::read(&mut t.b, base, 64 * count as usize).unwrap();
            // (the ring path takes no _alloca: that is the call stack)
            if count <= 0x80 && ma != mb {
                let d: Vec<usize> = (0..ma.len()).filter(|&k| ma[k] != mb[k]).collect();
                panic!(
                    "{what}: the matrices differ at base+{:#x}.. ({} bytes, count {count}, base {base:#x}, ebp {ebp:#x})",
                    d[0],
                    d.len()
                );
            }
            let _ = Mem::read::<u32>(&t.b, ebp);
            t.diff(&what);
        }
        for want in [0, 1, 7, 0x15, 0x16, 0x33, 0x51] {
            assert!(
                results.contains(&want),
                "{name}: no result {want:#x}: {results:x?}"
            );
        }
    }
}
