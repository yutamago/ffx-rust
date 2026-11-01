//! M5 stage 1, list `phyre-render-b`, round big-phyre2
//! (`ffx_prender::m5p_render_b_bp2`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_render_b.rs` (`bp2_*`).
//!
//! Functions that hand callees the addresses of their locals take `ebp`,
//! where the original's frame pointer is, and keep those locals at the
//! original's offsets below it; the shim passes the top of a
//! `StackFrame`, the tests the original's frame address.

/// `TrophyManager_Install` (thiscall `(this; arg)`, `xor eax, eax` on PC;
/// a folded body the texture parameter setters call first).
pub const TROPHY_INSTALL: u32 = 0x0059_e520;
/// `memcpy` (CRT thunk, cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// `PRenderSurface::PRenderSurface(texture, 0)` (thiscall, two words).
pub const SURFACE_CTOR: u32 = 0x005b_35c0;
/// `PRenderSurface::~PRenderSurface` (`ecx` only; a stub).
pub const SURFACE_DTOR: u32 = 0x005b_3640;
/// `PRenderer::setColorTarget(surface, slot)` (thiscall, two words).
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
/// `PRenderer::setDepthStencilTarget(target)` (thiscall, one word).
pub const SET_DEPTH_TARGET: u32 = 0x005b_b5b0;
/// `PRenderer::beginScene(word)` (thiscall, one word).
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
/// `PRenderer::endScene(word)` (thiscall, one word).
pub const END_SCENE: u32 = 0x005b_6950;
/// Render command 0x18 (thiscall `(byte, a, b)`).
pub const QUEUE_COMMAND_18: u32 = 0x005b_aa40;
/// `PRendererBase::renderMeshInstance(instance, contexts, x, y)`
/// (thiscall, four words).
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
/// The glow pass's render state object (`renderer + 0x48`).
pub const GLOW_STATE: u32 = 0x0194_49b4;

/// `0x00a139c0`'s frame below `ebp`: the surface at `ebp - 0x2c` and the
/// texture binding at `ebp - 0x1c`, in words.
pub const GLOW_MESH_FRAME_WORDS: u32 = 11;

/// `PRendererBase::selectTechniquePass` (thiscall on `[renderer]`, eight
/// words).
pub const SELECT_TECHNIQUE_PASS: u32 = 0x005b_a690;
/// `Phyre_PRendering_PRendererBase_meshInstanceSizes_005b5150` (cdecl,
/// nine words; writes its first three through pointers).
pub const MESH_SIZES: u32 = 0x005b_5150;
/// `0x005b55a0`'s frame below `ebp` (the three sizes at `ebp - 0x2c`,
/// `- 0x44`, `- 0x3c`), in words.
pub const QUEUE_SEGMENTS_FRAME_WORDS: u32 = 18;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// `Phyre_PPostProcessing_PGlow_renderMesh_00a139c0(effect; renderer,
    /// texture)` (thiscall, ret 8): when the effect has a texture
    /// parameter (`+0x74`), the binding `{0x42, 0, effect + 0x38, texture}`
    /// at `ebp - 0x1c` goes into its parameter buffer (`[[+0x60] + 4]`) as
    /// `Phyre_PParameterBuffer_setTextureChecked_0056e670` does (5 for a
    /// parameter type other than 0x42, 0xd9..0xdc or 0x12; the
    /// `TrophyManager_Install` call's error; 3 when the offset is below 4
    /// or the bytes don't fit). Then a surface at `ebp - 0x2c` of the
    /// effect's target (`+0xa8`) becomes the colour target, no depth
    /// target, a scene with command 0x18 `(0, 6, 7)`, the glow state, the
    /// effect's mesh instance (`+0x5c`) rendered, the scene ended; the
    /// first error, or 0. The surface is destroyed on every path after
    /// it is made.
    pub fn glow_render_mesh(
        h: &mut impl Host,
        ebp: u32,
        effect: u32,
        renderer: u32,
        texture: u32,
    ) -> u32 {
        let param: u32 = h.read(effect + 0x74);
        if param != 0 {
            let params: u32 = h.read(effect + 0x60);
            let buffer: u32 = h.read(params + 4);
            let kind: u8 = h.read(param + 2);
            let owner = effect.wrapping_add(0x38);
            let binding = ebp - 0x1c;
            h.write(binding, [0x42u32, 0, owner, texture]);
            if !matches!(kind, 0x42 | 0xd9..=0xdc | 0x12) {
                return 5;
            }
            if owner != 0 && texture != 0 {
                let err = this_call(h, TROPHY_INSTALL, owner, &[texture]);
                if err != 0 {
                    return err;
                }
            }
            let offset = u32::from(h.read::<u16>(param + 8));
            let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
            if offset.wrapping_add(size) > h.read::<u32>(buffer) || offset < 4 {
                return 3;
            }
            h.call_sym(
                MEMCPY,
                Conv::Cdecl,
                &[offset.wrapping_add(buffer), binding, size],
            );
        }
        let surface = ebp - 0x2c;
        let target: u32 = h.read(effect + 0xa8);
        this_call(h, SURFACE_CTOR, surface, &[target, 0]);
        let err = glow_scene(h, effect, renderer, surface);
        this_call(h, SURFACE_DTOR, surface, &[]);
        err
    }

    /// The scene of [`glow_render_mesh`]: its first error, or 0.
    fn glow_scene(h: &mut impl Host, effect: u32, renderer: u32, surface: u32) -> u32 {
        let err = this_call(h, SET_COLOR_TARGET, renderer, &[surface, 0]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, SET_DEPTH_TARGET, renderer, &[0]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, BEGIN_SCENE, renderer, &[0]);
        if err != 0 {
            return err;
        }
        this_call(h, QUEUE_COMMAND_18, renderer, &[0, 6, 7]);
        let contexts: u32 = h.read(renderer + 0x13c);
        let state = h.rebase(GLOW_STATE);
        h.write(renderer + 0x48, state);
        let instance: u32 = h.read(effect + 0x5c);
        let list: u32 = h.read(instance + 0x10);
        let err = this_call(
            h,
            RENDER_MESH_INSTANCE,
            renderer,
            &[instance, list, 0, contexts],
        );
        if err != 0 {
            return err;
        }
        this_call(h, END_SCENE, renderer, &[1])
    }

    /// The record ring of `0x005b55a0`: 0x20-byte records from `+0xc` to
    /// `+0x200c`.
    const RECORDS: u32 = 0xc;
    const RECORDS_END: u32 = 0x200c;

    /// `Phyre_PRendering_PRendererBase_queueSegments_005b55a0(renderer;
    /// job, context)` (thiscall, ret 8): 7 when the job's mesh (`[job +
    /// 8]`) has no segment array. Else the open range record
    /// (`[renderer + 0x2310]`) takes the mesh, the first segment (`[job +
    /// 0xc]`, 16 bits) and the job's byte `+0x10`; then one 0x20-byte
    /// record per segment at the renderer's cursor (`+0x230c`) until the
    /// segments or the ring (`+0x200c`) run out: the previous record's
    /// words carried over (zeros for the ring's first), the segment's
    /// material entry, and when it has one `selectTechniquePass`, the
    /// segment's pass copied when its key matches, and for a record with
    /// a target whose segment is not skipped (`+0x14`) the sizes
    /// `meshInstanceSizes` reports (into `ebp - 0x2c`, `- 0x44`, `- 0x3c`)
    /// added to the two budgets (`+0x231c` against `+8`, `+0x2320`
    /// against `+4`); a budget overrun ends the loop before the record
    /// counts. When any record counted, the range closes (its end, the
    /// next range, the count and budgets stored) and the byte is
    /// cleared. The job keeps the next segment and the byte; 0 when all
    /// segments are queued, else 0xd.
    #[allow(clippy::too_many_lines)] // one original
    pub fn queue_segments(h: &mut impl Host, ebp: u32, r: u32, job: u32, ctx: u32) -> u32 {
        let mesh: u32 = h.read(job + 8);
        let segments: u32 = h.read(mesh);
        if segments == 0 {
            return 7;
        }
        let mut seg: u32 = h.read(job + 0xc);
        let mut flag: u8 = h.read(job + 0x10);
        let range: u32 = h.read(r + 0x2310);
        h.write(range, mesh);
        let range: u32 = h.read(r + 0x2310);
        h.write(range + 4, seg as u16);
        let range: u32 = h.read(r + 0x2310);
        h.write(range + 8, flag);
        let count = h.read::<u32>(segments) & 0x7fff_ffff;
        let next_range = h.read::<u32>(r + 0x2318).wrapping_add(count);
        let mut budget_a: u32 = h.read(r + 0x231c);
        let mut budget_b: u32 = h.read(r + 0x2320);
        let mut cursor: u32 = h.read(r + 0x230c);
        let list: u32 = h.read(mesh + 0x10);
        let n = h.read::<u32>(list) & 0x7fff_ffff;
        let entries = match n {
            0 => 0,
            1 => list + 4,
            _ => h.read(list + 4),
        };
        let pass_key: u32 = h.read(mesh + 0x18);
        let base: u32 = h.read(r);
        let state: u32 = h.read(base + 0x48);
        if seg >= count {
            return finish(h, job, seg, flag, count);
        }
        let mut any = false;
        let ring_first = r.wrapping_add(RECORDS);
        let ring_end = r.wrapping_add(RECORDS_END);
        while cursor < ring_end {
            let items: u32 = h.read(segments + 4);
            let item = items.wrapping_add(seg.wrapping_mul(0x6c));
            let index: u32 = h.read(item);
            let entry: u32 = h.read(entries.wrapping_add(index.wrapping_mul(4)));
            h.write(ctx + 0x1c, item);
            h.write(ctx + 0x20, seg);
            let words = cursor.wrapping_add(8);
            let last = if cursor == ring_first {
                h.write(words, [0u32; 3]);
                h.write(cursor, [0u32; 2]);
                h.write(words + 0xc, 0u32);
                0
            } else {
                let prev = cursor.wrapping_sub(0x20);
                let w: u32 = h.read(prev);
                h.write(cursor, w);
                let w: u32 = h.read(prev + 4);
                h.write(cursor + 4, w);
                for k in 0..4 {
                    let w: u32 = h.read(prev + 8 + 4 * k);
                    h.write(words + 4 * k, w);
                }
                h.read(prev + 0x18)
            };
            h.write(words + 0x10, last);
            h.write(words + 0x14, entry);
            if entry == 0 {
                h.write(cursor, 0u32);
                h.write(cursor + 4, 0u32);
            } else {
                let kind: u32 = h.read(entry);
                let base: u32 = h.read(r);
                h.write(ebp - 0x3c, kind);
                let segs: u32 = h.read(mesh + 0x28);
                let pass = segs.wrapping_add(seg.wrapping_mul(0x24));
                h.call_sym(
                    SELECT_TECHNIQUE_PASS,
                    Conv::Thiscall(base),
                    &[cursor, words, mesh, kind, entry, state, ctx, pass],
                );
                if h.read::<u32>(pass + 0x18) == kind {
                    for k in 0..5 {
                        let w: u32 = h.read(pass + 4 * k);
                        h.write(words + 4 * k, w);
                    }
                }
                let target: u32 = h.read(cursor + 4);
                if target != 0 && h.read::<u8>(item + 0x14) == 0 {
                    let extra = u32::from(h.read::<u16>(kind + 0x1c));
                    let target_word: u32 = h.read(target);
                    let entry_word: u32 = h.read(entry + 4);
                    h.write(ebp - 0x2c, 0u32);
                    h.write(ebp - 0x44, 0u32);
                    h.write(ebp - 0x3c, 0u32);
                    h.call_sym(
                        MESH_SIZES,
                        Conv::Cdecl,
                        &[
                            ebp - 0x2c,
                            ebp - 0x44,
                            ebp - 0x3c,
                            target_word,
                            entry_word,
                            kind,
                            target,
                            seg,
                            pass_key,
                        ],
                    );
                    let a = budget_a.wrapping_add(extra);
                    if a > h.read::<u32>(r + 8) {
                        break;
                    }
                    let b = h.read::<u32>(ebp - 0x2c).wrapping_add(budget_b);
                    if b > h.read::<u32>(r + 4) {
                        break;
                    }
                    budget_a = a;
                    budget_b = b;
                    any = true;
                }
            }
            cursor = cursor.wrapping_add(0x20);
            seg = seg.wrapping_add(1);
            if seg >= count {
                break;
            }
        }
        if any {
            let n: u32 = h.read(r + 0x2314);
            h.write(r + 0x2314, n.wrapping_add(1));
            let range: u32 = h.read(r + 0x2310);
            h.write(range + 6, seg as u16);
            let range: u32 = h.read(r + 0x2310);
            h.write(r + 0x2310, range.wrapping_add(0xc));
            h.write(r + 0x2318, next_range);
            h.write(r + 0x231c, budget_a);
            h.write(r + 0x2320, budget_b);
            h.write(r + 0x230c, cursor);
            flag = 0;
        }
        finish(h, job, seg, flag, count)
    }

    /// The end of [`queue_segments`]: the job's next segment and byte;
    /// 0 when every segment is queued, else 0xd.
    fn finish(h: &mut impl Host, job: u32, seg: u32, flag: u8, count: u32) -> u32 {
        h.write(job + 0xc, seg);
        h.write(job + 0x10, flag);
        if seg == count { 0 } else { 0xd }
    }
}

/// `PRendererD3D11_updateDepthTransform_0059a390` (thiscall `(quad;
/// renderer, m, colour)`).
pub const UPDATE_DEPTH_TRANSFORM: u32 = 0x0059_a390;
/// `PRenderStateBlockD3D11_create_0058d4e0` (thiscall on the pass state).
pub const CREATE_PASS_STATE: u32 = 0x0058_d4e0;
/// `PFullscreenQuadD3D11_drawSetup_00592e90` (thiscall `(quad; renderer,
/// primitive, slot, n)`).
pub const DRAW_SETUP: u32 = 0x0059_2e90;
/// The most vertices one mapped buffer takes.
pub const QUAD_CHUNK: u32 = 0x1000;
/// `0x00591dd0`'s frame below `ebp` (the matrix at `ebp - 0xa0`, the
/// colour and the mapped resource at `ebp - 0x20`), in words.
pub const DRAW_POINTS_FRAME_WORDS: u32 = 40;

pub mod points {
    //! `0x00591dd0` over game memory.

    use super::{CREATE_PASS_STATE, DRAW_SETUP, QUAD_CHUNK, UPDATE_DEPTH_TRANSFORM};
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// COM method `off` of `obj` with `args` after it (stdcall).
    fn com(h: &mut impl Host, obj: u32, off: u32, args: &[u32]) -> u32 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt.wrapping_add(off));
        let mut words = [0u32; 6];
        words[0] = obj;
        words[1..=args.len()].copy_from_slice(args);
        h.call(f, Conv::Stdcall, &words[..=args.len()]) // call width: 6
    }

    /// Four floats through the x87 (`fld`/`fstp`).
    fn copy4(h: &mut impl Host, dst: u32, src: u32) {
        for k in 0..4 {
            let bits: u32 = h.read(src.wrapping_add(4 * k));
            h.write(dst.wrapping_add(4 * k), x87::store(x87::load(bits)));
        }
    }

    /// `PRendererD3D11_drawPoints_00591dd0(renderer; primitive, points,
    /// colour, indices, n, m)` (thiscall, ret 0x18): the full-screen quad
    /// helper (`renderer + 0x394`) takes `m` (the identity without one,
    /// at `ebp - 0xa0`) and the colour `{1, 1, 1, 1}` (`ebp - 0x20`)
    /// (`updateDepthTransform`, its error returned), releases its four
    /// buffers (`+0x1c8`, `+0x1cc`, `+0x1d4`, `+0x1d0`), copies the
    /// renderer's pass state (`+0x570`, 0x5f words) and creates it (its
    /// error returned). Then the `n` points go out in chunks of 0x1000:
    /// the next of 16 dynamic buffers (`+0x40`) mapped (`WRITE_DISCARD`,
    /// the result at `ebp - 0x1c`; a failed map writes through a null
    /// pointer as the original does), each vertex its point (`points[i]`
    /// or `points[indices[i]]`, 16 bytes) and `colour`, 0x28 bytes apart;
    /// unmapped, the buffer index advanced, `drawSetup` (its error
    /// returned). 0.
    #[allow(clippy::too_many_arguments)] // the original's six words, the host and the frame
    pub fn draw_points(
        h: &mut impl Host,
        ebp: u32,
        r: u32,
        primitive: u32,
        points: u32,
        colour: u32,
        indices: u32,
        n: u32,
        m: u32,
    ) -> u32 {
        let matrix = ebp - 0xa0;
        if m == 0 {
            for k in 0..16u32 {
                h.write(
                    matrix + 4 * k,
                    if k % 5 == 0 { 1.0f32 } else { 0.0 }.to_bits(),
                );
            }
        } else {
            for k in 0..16 {
                let w: u32 = h.read(m.wrapping_add(4 * k));
                h.write(matrix + 4 * k, w);
            }
        }
        // the copy at `ebp - 0x60` the original builds first is the same
        let one = 1.0f32.to_bits();
        h.write(ebp - 0x20, [one; 4]);
        let q = r.wrapping_add(0x394);
        let err = h.call_sym(
            UPDATE_DEPTH_TRANSFORM,
            Conv::Thiscall(q),
            &[r, matrix, ebp - 0x20],
        );
        if err != 0 {
            return err;
        }
        for off in [0x1c8, 0x1cc, 0x1d4, 0x1d0] {
            let obj: u32 = h.read(q.wrapping_add(off));
            if obj != 0 {
                com(h, obj, 8, &[]);
                h.write(q.wrapping_add(off), 0u32);
            }
        }
        for k in 0..0x5f {
            let w: u32 = h.read(r.wrapping_add(0x570 + 4 * k));
            h.write(q.wrapping_add(0x60 + 4 * k), w);
        }
        let err = h.call_sym(CREATE_PASS_STATE, Conv::Thiscall(q.wrapping_add(0x60)), &[]);
        if err != 0 {
            return err;
        }
        h.write(q.wrapping_add(0x58), [0u32; 2]);
        if n == 0 {
            return 0;
        }
        let mut done = 0u32;
        let mut src = points;
        let mut index_at = indices;
        loop {
            let chunk = (n.wrapping_sub(done)).min(QUAD_CHUNK);
            let ctx: u32 = h.read(r.wrapping_add(0x70));
            let slot: u32 = h.read(q.wrapping_add(0x40));
            let buffer: u32 = h.read(q.wrapping_add(slot.wrapping_mul(4)));
            let hr = com(h, ctx, 0x38, &[buffer, 0, 4, 0, ebp - 0x1c]);
            let mapped: u32 = h.read(ebp - 0x1c);
            #[allow(clippy::cast_possible_wrap)] // an HRESULT
            let mut dst = if (hr as i32) < 0 { 0 } else { mapped };
            for k in 0..chunk {
                let p = if indices == 0 {
                    src.wrapping_add(16 * k)
                } else {
                    let i: u32 = h.read(index_at.wrapping_add(4 * k));
                    (i << 4).wrapping_add(points)
                };
                copy4(h, dst, p);
                copy4(h, dst.wrapping_add(0x10), colour);
                dst = dst.wrapping_add(0x28);
            }
            let ctx: u32 = h.read(r.wrapping_add(0x70));
            let slot: u32 = h.read(q.wrapping_add(0x40));
            let buffer: u32 = h.read(q.wrapping_add(slot.wrapping_mul(4)));
            com(h, ctx, 0x3c, &[buffer, 0]);
            let slot: u32 = h.read(q.wrapping_add(0x40));
            done = done.wrapping_add(QUAD_CHUNK);
            src = src.wrapping_add(0x10000);
            index_at = index_at.wrapping_add(0x4000);
            h.write(q.wrapping_add(0x40), slot.wrapping_add(1) & 0xf);
            let err = h.call_sym(DRAW_SETUP, Conv::Thiscall(q), &[r, primitive, slot, chunk]);
            if err != 0 {
                return err;
            }
            if done >= n {
                return 0;
            }
        }
    }
}

/// `0x00a25410`'s callees: the pass's camera setup `0x00a13b90` (thiscall
/// `(effect; camera)`), `PMatrix4_FromMatrix3x4` (thiscall `(out; m)`),
/// `sce::Vectormath::Aos::inverse` (cdecl `(out, m)`),
/// `PMatrix4_RowTimes` (thiscall `(m; out, v)` -> `out`), `PMatrix4_Copy`
/// (thiscall `(dst; src)`).
pub const BLUR_MATRICES_CALLS: [u32; 5] = [
    0x00a1_3b90,
    0x0045_1080,
    0x0057_3d00,
    0x004c_9c20,
    0x0045_3340,
];
/// The `f32` -1.0 the projection's z takes.
pub const MINUS_ONE: u32 = 0x00b9_2340;
/// The words of `0x00a25410`'s frame below `ebp`.
pub const BLUR_MATRICES_FRAME_WORDS: u32 = 0x1c4 / 4;

pub mod blur_matrices {
    //! `0x00a25410` over game memory.

    use super::{BLUR_MATRICES_CALLS, MINUS_ONE};
    use ffx_bridge::x87::{self, Reg};
    use ffx_graphic::host::{Conv, Host};

    fn copy_floats(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        for k in 0..n {
            let bits: u32 = h.read(src.wrapping_add(4 * k));
            h.write(dst.wrapping_add(4 * k), x87::store(x87::load(bits)));
        }
    }

    fn copy_words(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        for k in 0..n {
            let w: u32 = h.read(src.wrapping_add(4 * k));
            h.write(dst.wrapping_add(4 * k), w);
        }
    }

    /// `m` times four row vectors (`PMatrix4_RowTimes`, into the outs, in
    /// the original's order: the last row first), the four results copied
    /// through the x87 into the 4x4 at `dst`.
    fn times_rows(h: &mut impl Host, m: u32, outs: [u32; 4], rows: [u32; 4], dst: u32) {
        let row_times = BLUR_MATRICES_CALLS[3];
        let mut r = [0u32; 4];
        for k in (0..4).rev() {
            r[k] = h.call_sym(row_times, Conv::Thiscall(m), &[outs[k], rows[k]]);
        }
        for (k, &p) in r.iter().enumerate() {
            copy_floats(h, dst + 0x10 * k as u32, p, 4);
        }
    }

    /// `Phyre_PPostProcessing_PMotionBlur_setupMatrices_00a25410(effect;
    /// camera)` (thiscall, ret 4): the camera setup `0x00a13b90`; the
    /// camera's 3x4 as a 4x4 `M` (`ebp - 0xe4`) and its inverse `I`
    /// (`ebp - 0x124`, from a copy at `ebp - 0x40`); the scale `S` (`ebp -
    /// 0xa4`): `1 / camera[0x30]`, `1 / camera[0x44]`, -1.0 on the
    /// diagonal, then 1 in the corner; the effect's `B = +0x54` times the
    /// rows of `A = +0x14`, then of `I`, then of `S` (each product's rows
    /// through the x87 into `ebp - 0x40`) goes to `+0x94`, `M` to `+0x14`,
    /// and `B` takes the camera's projection (`camera + 0x30`,
    /// `PMatrix4_Copy`, its `eax` returned).
    pub fn blur_matrices(h: &mut impl Host, ebp: u32, fx: u32, camera: u32) -> u32 {
        let [setup, from_3x4, inverse, _, copy] = BLUR_MATRICES_CALLS;
        h.call_sym(setup, Conv::Thiscall(fx), &[camera]);
        copy_words(h, ebp - 0x64, camera + 0x40, 4);
        copy_words(h, ebp - 0x54, camera + 0x30, 4);
        let p0: u32 = h.read(ebp - 0x54);
        let p1: u32 = h.read(ebp - 0x60);
        let a = x87::store(x87::div(1.0, x87::load(p0)));
        h.write(ebp - 0x54, a);
        let b = x87::store(Reg::ONE.div_m32(f32::from_bits(p1)).value());
        h.write(ebp - 0x50, b);
        let minus_one: u32 = h.read(h.rebase(MINUS_ONE));
        let flip = x87::store(x87::load(minus_one));
        let one = 1.0f32.to_bits();
        let s = ebp - 0xa4;
        h.write(s, [a, 0, 0, 0, 0, b, 0, 0, 0, 0, flip, 0, 0, 0, 0, one]);
        let m = ebp - 0xe4;
        h.call_sym(from_3x4, Conv::Thiscall(m), &[camera]);
        let n = ebp - 0x40;
        copy_words(h, n, m, 16);
        let inv = ebp - 0x124;
        h.call_sym(inverse, Conv::Cdecl, &[inv, n]);
        let a_rows = fx + 0x14;
        let b_m = fx + 0x54;
        times_rows(
            h,
            b_m,
            [ebp - 0x164, ebp - 0x154, ebp - 0x54, ebp - 0x64],
            [a_rows, a_rows + 0x10, a_rows + 0x20, a_rows + 0x30],
            n,
        );
        times_rows(
            h,
            n,
            [ebp - 0x134, ebp - 0x1c4, ebp - 0x1a4, ebp - 0x184],
            [inv, inv + 0x10, inv + 0x20, inv + 0x30],
            n,
        );
        times_rows(
            h,
            n,
            [ebp - 0x144, ebp - 0x1b4, ebp - 0x174, ebp - 0x194],
            [s, s + 0x10, s + 0x20, s + 0x30],
            n,
        );
        copy_floats(h, fx + 0x94, n, 16);
        copy_floats(h, fx + 0x14, m, 16);
        h.call_sym(copy, Conv::Thiscall(b_m), &[camera + 0x30])
    }
}
