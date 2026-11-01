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

/// `0x00a13b90`'s callees: `PMatrix4_FromMatrix3x4` (thiscall `(out;
/// m)`), `sce::Vectormath::Aos::inverse` (cdecl `(out, m)`),
/// `PMatrix4_RowTimes` (thiscall `(m; out, v)` -> `out`),
/// `PMatrix4_FromRows` (thiscall `(out; r0, r1, r2, r3)`),
/// `PMatrix4_Copy_00453340` (thiscall `(dst; src)`).
pub const HISTORY_CALLS: [u32; 5] = [
    0x0045_1080,
    0x0057_3d00,
    0x004c_9c20,
    0x0045_0d40,
    0x0045_3340,
];
/// The words of `0x00a13b90`'s frame below `ebp`.
pub const HISTORY_FRAME_WORDS: u32 = 0x30c / 4;

pub mod history {
    //! `0x00a13b90` over game memory.

    use super::{HISTORY_CALLS, MINUS_ONE};
    use ffx_bridge::x87::{self, Reg};
    use ffx_graphic::host::{Conv, Host};

    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    fn copy_words(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        for k in 0..n {
            let w: u32 = h.read(src.wrapping_add(4 * k));
            h.write(dst.wrapping_add(4 * k), w);
        }
    }

    fn copy_floats(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        for k in 0..n {
            let x = ld(h, src.wrapping_add(4 * k));
            h.write(dst.wrapping_add(4 * k), x87::store(x));
        }
    }

    /// `m` times four row vectors (`PMatrix4_RowTimes` into `outs`, the
    /// last row first), the four results copied through the x87 into the
    /// rows at `dst`.
    fn times_rows(h: &mut impl Host, m: u32, outs: [u32; 4], rows: [u32; 4], dst: [u32; 4]) {
        let row_times = HISTORY_CALLS[2];
        let mut r = [0u32; 4];
        for k in (0..4).rev() {
            r[k] = h.call_sym(row_times, Conv::Thiscall(m), &[outs[k], rows[k]]);
        }
        for k in 0..4 {
            copy_floats(h, dst[k], r[k], 4);
        }
    }

    /// The product the original computes inline: row `r` of `l` (rows at
    /// `l[r]`) times the columns of the rows at `rr`, each element
    /// `((l0 r0 + l1 r1) + l2 r2) + l3 r3` stored as `f32` into the rows at
    /// `out`.
    fn product(h: &mut impl Host, l: [u32; 4], rr: [u32; 4], out: [u32; 4]) {
        let mut res = [[0u32; 4]; 4];
        for (r, row) in res.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().enumerate() {
                let c4 = 4 * c as u32;
                let term = |h: &dyn Fn(u32) -> f64, k: usize| {
                    x87::mul(h(l[r] + 4 * k as u32), h(rr[k] + c4))
                };
                let get = |at: u32| ld(h, at);
                let s = x87::add(term(&get, 0), term(&get, 1));
                let s = x87::add(s, term(&get, 2));
                let s = x87::add(s, term(&get, 3));
                *cell = x87::store(s);
            }
        }
        for (r, row) in res.iter().enumerate() {
            h.write(out[r], *row);
        }
    }

    /// `Phyre_PPostProcessing_PMotionBlur_cameraHistory_00a13b90(effect;
    /// camera)` (thiscall, ret 4): the scale `S` (`ebp - 0x1cc`: `1 /
    /// camera[0x30]`, `1 / camera[0x44]`, -1.0, 1 on the diagonal); the
    /// camera's 3x4 as a 4x4 and its inverse `I` (`ebp - 0xd4`). For each
    /// of the 16 history slots: `Q` the inverse of the slot's current
    /// matrix (`+0xd4 + 0x40 i`, into `ebp - 0x20c`); `N` = the effect's
    /// `B = +0x54` times the rows of `A = +0x14` times the rows of the
    /// slot's previous matrix (`+0x4d4 + 0x40 i`; `RowTimes`, at `ebp -
    /// 0x18c` and `- 0x40`); `X = Q N` and `Y = I X` (inline, in the frame);
    /// `Y` times the rows of `S` (`RowTimes`) rebuilt by `FromRows` into
    /// `+0x8d4 + 0x40 i`; the previous matrix takes the current one.
    /// `eax` is the last `PMatrix4_Copy`'s.
    #[allow(clippy::too_many_lines)] // one original
    pub fn camera_history(h: &mut impl Host, ebp: u32, fx: u32, camera: u32) -> u32 {
        let [from_3x4, inverse, _, from_rows, copy] = HISTORY_CALLS;
        copy_words(h, ebp - 0xe4, camera + 0x40, 4);
        copy_words(h, ebp - 0x80, camera + 0x30, 4);
        let p0: u32 = h.read(ebp - 0x80);
        let p1: u32 = h.read(ebp - 0xe0);
        let a = x87::store(x87::div(1.0, x87::load(p0)));
        h.write(ebp - 0x80, a);
        let b = x87::store(Reg::ONE.div_m32(f32::from_bits(p1)).value());
        h.write(ebp - 0x7c, b);
        let minus_one: u32 = h.read(h.rebase(MINUS_ONE));
        let flip = x87::store(x87::load(minus_one));
        let one = 1.0f32.to_bits();
        let s = ebp - 0x1cc;
        h.write(s, [a, 0, 0, 0, 0, b, 0, 0, 0, 0, flip, 0, 0, 0, 0, one]);
        let m = ebp - 0x20c;
        h.call_sym(from_3x4, Conv::Thiscall(m), &[camera]);
        copy_words(h, ebp - 0x14c, m, 16);
        let inv = ebp - 0xd4;
        h.call_sym(inverse, Conv::Cdecl, &[inv, ebp - 0x14c]);
        let a_m = fx + 0x14;
        let b_m = fx + 0x54;
        let rows = |at: u32| [at, at + 0x10, at + 0x20, at + 0x30];
        let mut eax = 0;
        for i in 0..16u32 {
            let cur = fx + 0xd4 + 0x40 * i;
            let prev = fx + 0x4d4 + 0x40 * i;
            let q = ebp - 0x20c;
            h.call_sym(inverse, Conv::Cdecl, &[q, cur]);
            let n1 = rows(ebp - 0x18c);
            times_rows(
                h,
                a_m,
                [ebp - 0x21c, ebp - 0x23c, ebp - 0x22c, ebp - 0x25c],
                rows(prev),
                n1,
            );
            let n2 = rows(ebp - 0x40);
            times_rows(
                h,
                b_m,
                [ebp - 0x28c, ebp - 0x2bc, ebp - 0x29c, ebp - 0x27c],
                n1,
                n2,
            );
            let x = [ebp - 0x60, ebp - 0x70, ebp - 0x50, ebp - 0x94];
            product(h, rows(q), n2, x);
            let y = [ebp - 0x10c, ebp - 0xfc, ebp - 0xe4, ebp - 0x80];
            product(h, rows(inv), x, y);
            let z = ebp - 0x14c;
            for (k, &row) in y.iter().enumerate() {
                copy_floats(h, z + 0x10 * k as u32, row, 4);
            }
            let row_times = HISTORY_CALLS[2];
            let outs = [ebp - 0x2ac, ebp - 0x26c, ebp - 0x2cc, ebp - 0x24c];
            let s_rows = rows(s);
            let mut zr = [0u32; 4];
            for k in (0..4).rev() {
                zr[k] = h.call_sym(row_times, Conv::Thiscall(z), &[outs[k], s_rows[k]]);
            }
            h.call_sym(from_rows, Conv::Thiscall(ebp - 0x30c), &zr);
            h.call_sym(copy, Conv::Thiscall(fx + 0x8d4 + 0x40 * i), &[ebp - 0x30c]);
            eax = h.call_sym(copy, Conv::Thiscall(prev), &[cur]);
        }
        eax
    }
}

/// `PRendererBase::renderMeshInstanceForPass_005b9a70` (thiscall
/// `(renderer; instance)`).
pub const RENDER_FOR_PASS: u32 = 0x005b_9a70;
/// `Phyre_PParameterBuffer_setTextureChecked_0056e670` (thiscall
/// `(buffer; param, binding)`).
pub const SET_TEXTURE_CHECKED: u32 = 0x0056_e670;
/// `ShaderParamBuffer_SetValue` (thiscall `(buffer; param, src)`).
pub const SET_VALUE: u32 = 0x0063_8780;
/// The depth-of-field passes' render states (`renderer + 0x48`): the
/// first pass, the second, the two blur passes, the final one.
pub const DOF_STATES: [u32; 4] = [0x0194_4d14, 0x0194_4934, 0x0194_35d8, 0x0194_4d24];
/// `0x00a1ca10`'s frame below `ebp` (five surfaces from `ebp - 0x8c`, the
/// bindings at `- 0x3c`, `- 0x2c`, `- 0x1c`), in words.
pub const DOF_FRAME_WORDS: u32 = 0x8c / 4;

pub mod dof {
    //! `0x00a1ca10` over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// The parameter buffer (`[[fx + 0xf3c] + 4]`).
    fn buffer(h: &impl Host, fx: u32) -> u32 {
        let params: u32 = h.read(fx + 0xf3c);
        h.read(params + 4)
    }

    /// Where a parameter's bytes go in `buffer`: `None` (3) when the
    /// offset is below 4 or they don't fit the buffer's size word.
    fn place(h: &impl Host, param: u32, buffer: u32) -> Option<(u32, u32)> {
        let offset = u32::from(h.read::<u16>(param + 8));
        let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
        if offset.wrapping_add(size) > h.read::<u32>(buffer) || offset < 4 {
            None
        } else {
            Some((offset, size))
        }
    }

    /// A texture parameter (`fx + slot`) set as `setTextureChecked` does,
    /// inline: the binding `{0x42, 0, fx + owner, texture}` at `binding`;
    /// 5 for a type other than 0x42, 0xd9..0xdc or 0x12,
    /// `TrophyManager_Install`'s error, 3 when it doesn't fit; 0.
    fn set_texture(
        h: &mut impl Host,
        fx: u32,
        slot: u32,
        owner: u32,
        texture: u32,
        binding: u32,
    ) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let buffer = buffer(h, fx);
        let kind: u8 = h.read(param + 2);
        let owner = fx.wrapping_add(owner);
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
        match place(h, param, buffer) {
            None => 3,
            Some((offset, size)) => {
                h.call_sym(
                    MEMCPY,
                    Conv::Cdecl,
                    &[offset.wrapping_add(buffer), binding, size],
                );
                0
            }
        }
    }

    /// A parameter (`fx + slot`) taking the effect's bytes at `fx + src`:
    /// 1 when that address is 0, 3 when they don't fit; 0.
    fn set_bytes(h: &mut impl Host, fx: u32, slot: u32, src: u32) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let buffer = buffer(h, fx);
        let src = fx.wrapping_add(src);
        if src == 0 {
            return 1;
        }
        match place(h, param, buffer) {
            None => 3,
            Some((offset, size)) => {
                h.call_sym(
                    MEMCPY,
                    Conv::Cdecl,
                    &[offset.wrapping_add(buffer), src, size],
                );
                0
            }
        }
    }

    /// A render target's texture for a binding: `[target]` unless its
    /// byte `+4` is set (0 then).
    fn texture_of(h: &impl Host, fx: u32, slot: u32) -> u32 {
        let target: u32 = h.read(fx + slot);
        if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        }
    }

    /// `setTextureChecked(buffer; [fx + slot], binding)` with the binding
    /// `{0x42, 0, fx + owner, texture}` at `ebp - 0x3c`, when the slot is
    /// set.
    fn bind(h: &mut impl Host, ebp: u32, fx: u32, slot: u32, owner: u32, texture: u32) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let b = ebp - 0x3c;
        h.write(b, [0x42u32, 0, fx.wrapping_add(owner), texture]);
        let buffer = buffer(h, fx);
        this_call(h, SET_TEXTURE_CHECKED, buffer, &[param, b])
    }

    /// `ShaderParamBuffer_SetValue(buffer; [fx + slot], fx + src)` when the
    /// slot is set.
    fn set_value(h: &mut impl Host, fx: u32, slot: u32, src: u32) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let buffer = buffer(h, fx);
        this_call(h, SET_VALUE, buffer, &[param, fx.wrapping_add(src)])
    }

    /// A pass into the surface at `surface` (made by [`open`]): the colour
    /// target, a scene with the
    /// mesh instance (`renderMeshInstance` for the first pass,
    /// `renderMeshInstanceForPass` after); the first error.
    fn pass(h: &mut impl Host, r: u32, fx: u32, surface: u32, first: bool) -> u32 {
        let err = this_call(h, SET_COLOR_TARGET, r, &[surface, 0]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, BEGIN_SCENE, r, &[0]);
        if err != 0 {
            return err;
        }
        let instance: u32 = h.read(fx + 0xf38);
        let err = if first {
            let contexts: u32 = h.read(r + 0x13c);
            let list: u32 = h.read(instance + 0x10);
            this_call(h, RENDER_MESH_INSTANCE, r, &[instance, list, 0, contexts])
        } else {
            this_call(h, RENDER_FOR_PASS, r, &[instance])
        };
        if err != 0 {
            return err;
        }
        this_call(h, END_SCENE, r, &[1])
    }

    /// The surfaces made so far (at most five), to destroy in reverse.
    struct Surfaces([u32; 5], usize);

    impl Surfaces {
        fn push(&mut self, at: u32) {
            if let Some(slot) = self.0.get_mut(self.1) {
                *slot = at;
                self.1 += 1;
            }
        }
    }

    /// Sets the render state and constructs the pass's surface of
    /// `[fx + target]`.
    fn open(h: &mut impl Host, r: u32, fx: u32, state: u32, target: u32, surface: u32) {
        let s = h.rebase(state);
        h.write(r + 0x48, s);
        let t: u32 = h.read(fx + target);
        this_call(h, SURFACE_CTOR, surface, &[t, 0]);
    }

    /// The passes after the setup; `surfaces` collects the ones to
    /// destroy.
    fn passes(
        h: &mut impl Host,
        ebp: u32,
        fx: u32,
        r: u32,
        tex: u32,
        out: u32,
        surfaces: &mut Surfaces,
    ) -> u32 {
        let [s1, s2, s3, s4] = DOF_STATES;
        let surface = ebp - 0x8c;
        open(h, r, fx, s1, 0xfb0, surface);
        surfaces.push(surface);
        let err = pass(h, r, fx, surface, true);
        if err != 0 {
            return err;
        }
        let surface = ebp - 0x7c;
        open(h, r, fx, s2, 0xf2c, surface);
        surfaces.push(surface);
        let err = pass(h, r, fx, surface, false);
        if err != 0 {
            return err;
        }
        let err = set_value(h, fx, 0xfa0, 0x51c);
        if err != 0 {
            return err;
        }
        let t = texture_of_if(h, fx, 0xf88, 0xf2c);
        let err = bind(h, ebp, fx, 0xf88, 0xf40, t);
        if err != 0 {
            return err;
        }
        let err = set_value(h, fx, 0xf9c, 0x50c);
        if err != 0 {
            return err;
        }
        let surface = ebp - 0x6c;
        open(h, r, fx, s3, 0xf30, surface);
        surfaces.push(surface);
        let err = pass(h, r, fx, surface, false);
        if err != 0 {
            return err;
        }
        let err = set_value(h, fx, 0xfa0, 0x58c);
        if err != 0 {
            return err;
        }
        let t = texture_of_if(h, fx, 0xf88, 0xf30);
        let err = bind(h, ebp, fx, 0xf88, 0xf64, t);
        if err != 0 {
            return err;
        }
        let surface = ebp - 0x5c;
        open(h, r, fx, s3, 0xf34, surface);
        surfaces.push(surface);
        let err = pass(h, r, fx, surface, false);
        if err != 0 {
            return err;
        }
        let err = bind(h, ebp, fx, 0xf88, 0xf64, tex);
        if err != 0 {
            return err;
        }
        for (slot, target) in [(0xf8c, 0xf2c), (0xf90, 0xf34), (0xf98, 0xfb0)] {
            let t = texture_of_if(h, fx, slot, target);
            let err = bind(h, ebp, fx, slot, 0xf64, t);
            if err != 0 {
                return err;
            }
        }
        for (slot, src) in [(0xfa4, 0x5fc), (0xfa8, 0xc1c), (0xfac, 0x90c)] {
            let err = set_value(h, fx, slot, src);
            if err != 0 {
                return err;
            }
        }
        // the final pass into `out` (its surface made even without one)
        let surface = ebp - 0x4c;
        this_call(h, SURFACE_CTOR, surface, &[out, 0]);
        surfaces.push(surface);
        let err = if out == 0 {
            this_call(h, SET_COLOR_TARGET, r, &[0, 0])
        } else {
            this_call(h, SET_COLOR_TARGET, r, &[surface, 0])
        };
        if err != 0 {
            return err;
        }
        let err = this_call(h, SET_DEPTH_TARGET, r, &[0]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, BEGIN_SCENE, r, &[0]);
        if err != 0 {
            return err;
        }
        let state = h.rebase(s4);
        h.write(r + 0x48, state);
        let instance: u32 = h.read(fx + 0xf38);
        let err = this_call(h, RENDER_FOR_PASS, r, &[instance]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, END_SCENE, r, &[1]);
        if err != 0 {
            return err;
        }
        this_call(h, SET_COLOR_TARGET, r, &[0, 0])
    }

    /// The binding texture of `[fx + target]`, read only when the slot is
    /// set (as the original reads it inside its test).
    fn texture_of_if(h: &impl Host, fx: u32, slot: u32, target: u32) -> u32 {
        if h.read::<u32>(fx + slot) == 0 {
            0
        } else {
            texture_of(h, fx, target)
        }
    }

    /// `Phyre_PPostProcessing_PDepthOfField_renderPass_00a1ca10(effect;
    /// renderer, a, near, far, out)` (thiscall, ret 0x14): 0x15 without
    /// the mesh instance (`+0xf38`); the two texture parameters (`+0xf88`
    /// with `near`, owner `+0xf64`, binding at `ebp - 0x1c`; `+0xf94` with
    /// `far`, owner `+0xf40`, at `ebp - 0x2c`) and three byte parameters
    /// (`+0xfb4`..`+0xfbc` from `+0x14`..`+0x1c`) into the parameter
    /// buffer (`[[+0xf3c] + 4]`), the first error returned; no depth
    /// target; then four passes into the effect's targets (`+0xfb0`,
    /// `+0xf2c`, `+0xf30`, `+0xf34`; surfaces from `ebp - 0x8c`) with
    /// their render states and parameters (`setTextureChecked` with
    /// bindings at `ebp - 0x3c`, `ShaderParamBuffer_SetValue`), and the
    /// last into `out` (or no colour target), the colour target cleared
    /// again. Every surface made is destroyed, the last first; the first
    /// error, or the last call's `eax`. `a` is never read.
    #[allow(clippy::too_many_arguments)] // the original's five words, the host and the frame
    pub fn dof_render_pass(
        h: &mut impl Host,
        ebp: u32,
        fx: u32,
        renderer: u32,
        a: u32,
        near: u32,
        far: u32,
        out: u32,
    ) -> u32 {
        let _ = a;
        if h.read::<u32>(fx + 0xf38) == 0 {
            return 0x15;
        }
        let err = set_texture(h, fx, 0xf88, 0xf64, near, ebp - 0x1c);
        if err != 0 {
            return err;
        }
        let err = set_texture(h, fx, 0xf94, 0xf40, far, ebp - 0x2c);
        if err != 0 {
            return err;
        }
        for (slot, src) in [(0xfb4, 0x14), (0xfb8, 0x18), (0xfbc, 0x1c)] {
            let err = set_bytes(h, fx, slot, src);
            if err != 0 {
                return err;
            }
        }
        let err = this_call(h, SET_DEPTH_TARGET, renderer, &[0]);
        if err != 0 {
            return err;
        }
        let mut surfaces = Surfaces([0; 5], 0);
        let err = passes(h, ebp, fx, renderer, near, out, &mut surfaces);
        let Surfaces(at, n) = surfaces;
        for &s in at.iter().take(n).rev() {
            this_call(h, SURFACE_DTOR, s, &[]);
        }
        err
    }
}

/// `PParameterBuffer_writeParameter_004d57d0` (thiscall `(buffer; param,
/// binding)`).
pub const WRITE_PARAMETER: u32 = 0x004d_57d0;
/// The screen-space reflection passes' render states (`renderer + 0x48`).
pub const SSR_STATES: [u32; 4] = [0x0194_4f08, 0x0194_4f18, 0x0194_49e4, 0x0194_4f28];
/// `0x00a220b0`'s frame below `ebp` (three surfaces from `ebp - 0x7c`, the
/// fourth over the binding at `- 0x4c`, the bindings at `- 0x3c`,
/// `- 0x2c`, `- 0x1c`), in words.
pub const SSR_FRAME_WORDS: u32 = 0x7c / 4;

pub mod ssr {
    //! `0x00a220b0` over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// The parameter buffer (`[[fx + 0x74] + 4]`).
    fn buffer(h: &impl Host, fx: u32) -> u32 {
        let params: u32 = h.read(fx + 0x74);
        h.read(params + 4)
    }

    fn place(h: &impl Host, param: u32, buffer: u32) -> Option<(u32, u32)> {
        let offset = u32::from(h.read::<u16>(param + 8));
        let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
        if offset.wrapping_add(size) > h.read::<u32>(buffer) || offset < 4 {
            None
        } else {
            Some((offset, size))
        }
    }

    /// The type check and `TrophyManager_Install` of a texture binding
    /// `{0x42, 0, fx + owner, texture}` written at `binding`: 5 for a type
    /// other than 0x42, 0xd9..0xdc or 0x12, the install's error; 0.
    fn check_texture(
        h: &mut impl Host,
        fx: u32,
        param: u32,
        owner: u32,
        texture: u32,
        binding: u32,
    ) -> u32 {
        let kind: u8 = h.read(param + 2);
        let owner = fx.wrapping_add(owner);
        h.write(binding, [0x42u32, 0, owner, texture]);
        if !matches!(kind, 0x42 | 0xd9..=0xdc | 0x12) {
            return 5;
        }
        if owner != 0 && texture != 0 {
            return this_call(h, TROPHY_INSTALL, owner, &[texture]);
        }
        0
    }

    /// A texture parameter (`fx + slot`) set inline as `setTextureChecked`
    /// does (3 when it doesn't fit).
    fn set_texture(
        h: &mut impl Host,
        fx: u32,
        slot: u32,
        owner: u32,
        texture: u32,
        binding: u32,
    ) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let buffer = buffer(h, fx);
        let err = check_texture(h, fx, param, owner, texture, binding);
        if err != 0 {
            return err;
        }
        match place(h, param, buffer) {
            None => 3,
            Some((offset, size)) => {
                h.call_sym(
                    MEMCPY,
                    Conv::Cdecl,
                    &[offset.wrapping_add(buffer), binding, size],
                );
                0
            }
        }
    }

    /// A byte parameter (`fx + slot`) from `fx + src`: 1 for a null
    /// source, 3 when it doesn't fit.
    fn set_bytes(h: &mut impl Host, fx: u32, slot: u32, src: u32) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let buffer = buffer(h, fx);
        let src = fx.wrapping_add(src);
        if src == 0 {
            return 1;
        }
        match place(h, param, buffer) {
            None => 3,
            Some((offset, size)) => {
                h.call_sym(
                    MEMCPY,
                    Conv::Cdecl,
                    &[offset.wrapping_add(buffer), src, size],
                );
                0
            }
        }
    }

    fn texture_of(h: &impl Host, fx: u32, target: u32) -> u32 {
        let t: u32 = h.read(fx + target);
        if h.read::<u8>(t + 4) == 0 {
            h.read(t)
        } else {
            0
        }
    }

    /// A binding's texture: a fixed one, or the one of the render target
    /// at `fx + offset` (read only when the binding is made).
    #[derive(Clone, Copy)]
    enum Tex {
        Is(u32),
        Of(u32),
    }

    /// `setTextureChecked(buffer; [fx + slot], ebp - 0x4c)` with the
    /// binding `{0x42, 0, fx + owner, texture}`, when the slot is set.
    fn bind(h: &mut impl Host, ebp: u32, fx: u32, slot: u32, owner: u32, tex: Tex) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let t = match tex {
            Tex::Is(t) => t,
            Tex::Of(target) => texture_of(h, fx, target),
        };
        let b = ebp - 0x4c;
        h.write(b, [0x42u32, 0, fx.wrapping_add(owner), t]);
        let buffer = buffer(h, fx);
        this_call(h, SET_TEXTURE_CHECKED, buffer, &[param, b])
    }

    /// A pass: colour target `target` (a surface or 0), no depth target,
    /// the state, a scene with the mesh instance, the colour target
    /// cleared; the first error.
    fn pass(h: &mut impl Host, r: u32, fx: u32, target: u32, state: u32, first: bool) -> u32 {
        let err = this_call(h, SET_COLOR_TARGET, r, &[target, 0]);
        if err != 0 {
            return err;
        }
        finish(h, r, fx, state, first)
    }

    /// [`pass`] after its colour target.
    fn finish(h: &mut impl Host, r: u32, fx: u32, state: u32, first: bool) -> u32 {
        let err = this_call(h, SET_DEPTH_TARGET, r, &[0]);
        if err != 0 {
            return err;
        }
        if state != 0 {
            let s = h.rebase(state);
            h.write(r + 0x48, s);
        }
        let err = this_call(h, BEGIN_SCENE, r, &[0]);
        if err != 0 {
            return err;
        }
        let instance: u32 = h.read(fx + 0x1c);
        let err = if first {
            let contexts: u32 = h.read(r + 0x13c);
            let list: u32 = h.read(instance + 0x10);
            this_call(h, RENDER_MESH_INSTANCE, r, &[instance, list, 0, contexts])
        } else {
            this_call(h, RENDER_FOR_PASS, r, &[instance])
        };
        if err != 0 {
            return err;
        }
        let err = this_call(h, END_SCENE, r, &[1]);
        if err != 0 {
            return err;
        }
        this_call(h, SET_COLOR_TARGET, r, &[0, 0])
    }

    /// The passes; `made` counts the surfaces at `ebp - 0x7c`, `- 0x6c`,
    /// `- 0x5c` made so far.
    #[allow(clippy::too_many_arguments)] // the original's words, the host and the frame
    fn passes(
        h: &mut impl Host,
        ebp: u32,
        fx: u32,
        r: u32,
        out: u32,
        a: u32,
        b: u32,
        made: &mut u32,
    ) -> u32 {
        let [s1, s2, s3, s4] = SSR_STATES;
        for (k, (target, state)) in [(0x68u32, s1), (0x6c, s2)].into_iter().enumerate() {
            let surface = ebp - 0x7c + 0x10 * k as u32;
            let t: u32 = h.read(fx + target);
            this_call(h, SURFACE_CTOR, surface, &[t, 0]);
            *made += 1;
            let err = pass(h, r, fx, surface, state, k == 0);
            if err != 0 {
                return err;
            }
        }
        let err = bind(h, ebp, fx, 0x78, 0x44, Tex::Of(0x68));
        if err != 0 {
            return err;
        }
        let err = bind(h, ebp, fx, 0x80, 0x20, Tex::Of(0x6c));
        if err != 0 {
            return err;
        }
        let surface = ebp - 0x5c;
        let t: u32 = h.read(fx + 0x70);
        this_call(h, SURFACE_CTOR, surface, &[t, 0]);
        *made += 1;
        let err = pass(h, r, fx, surface, s3, false);
        if err != 0 {
            return err;
        }
        for (slot, owner, tex) in [(0x78, 0x44, a), (0x7c, 0x20, b)] {
            let err = bind(h, ebp, fx, slot, owner, Tex::Is(tex));
            if err != 0 {
                return err;
            }
        }
        let err = bind(h, ebp, fx, 0x88, 0x44, Tex::Of(0x70));
        if err != 0 {
            return err;
        }
        let state = h.rebase(s4);
        h.write(r + 0x48, state);
        let err = if out == 0 {
            this_call(h, SET_COLOR_TARGET, r, &[0, 0])
        } else {
            let surface = ebp - 0x4c;
            this_call(h, SURFACE_CTOR, surface, &[out, 0]);
            let err = this_call(h, SET_COLOR_TARGET, r, &[surface, 0]);
            this_call(h, SURFACE_DTOR, surface, &[]);
            err
        };
        if err != 0 {
            return err;
        }
        finish(h, r, fx, 0, false)
    }

    /// `Phyre_PPostProcessing_PScreenSpaceReflection_renderPass_00a220b0(effect;
    /// renderer, out, a, b, c)` (thiscall, ret 0x14): 0x15 without the
    /// mesh instance (`+0x1c`); texture parameters `+0x78` (`a`, owner
    /// `+0x44`, binding at `ebp - 0x1c`) and `+0x7c` (`b`, owner `+0x20`,
    /// `ebp - 0x2c`) inline, `+0x84` (`c`, owner `+0x20`, `ebp - 0x3c`)
    /// through `writeParameter`, byte parameters `+0x8c`/`+0x90` from
    /// `+0x14`/`+0x18` (buffer `[[+0x74] + 4]`); three passes into
    /// `+0x68`, `+0x6c`, `+0x70` (surfaces at `ebp - 0x7c`, `- 0x6c`,
    /// `- 0x5c`; states `0x01944f08`, `0x01944f18`, `0x019449e4`) with the
    /// texture bindings between them (`setTextureChecked`, at `ebp - 0x4c`),
    /// and a last one into `out` (its surface over the binding, destroyed at
    /// once) or no colour target; the surfaces destroyed in reverse; the
    /// first error or the last colour-target call's `eax`.
    #[allow(clippy::too_many_arguments)] // the original's five words, the host and the frame
    pub fn ssr_render_pass(
        h: &mut impl Host,
        ebp: u32,
        fx: u32,
        renderer: u32,
        out: u32,
        a: u32,
        b: u32,
        c: u32,
    ) -> u32 {
        if h.read::<u32>(fx + 0x1c) == 0 {
            return 0x15;
        }
        let err = set_texture(h, fx, 0x78, 0x44, a, ebp - 0x1c);
        if err != 0 {
            return err;
        }
        let err = set_texture(h, fx, 0x7c, 0x20, b, ebp - 0x2c);
        if err != 0 {
            return err;
        }
        let param: u32 = h.read(fx + 0x84);
        if param != 0 {
            let buffer = buffer(h, fx);
            let err = check_texture(h, fx, param, 0x20, c, ebp - 0x3c);
            if err != 0 {
                return err;
            }
            let err = this_call(h, WRITE_PARAMETER, buffer, &[param, ebp - 0x3c]);
            if err != 0 {
                return err;
            }
        }
        for (slot, src) in [(0x8c, 0x14), (0x90, 0x18)] {
            let err = set_bytes(h, fx, slot, src);
            if err != 0 {
                return err;
            }
        }
        let mut made = 0u32;
        let err = passes(h, ebp, fx, renderer, out, a, b, &mut made);
        for k in (0..made).rev() {
            this_call(h, SURFACE_DTOR, ebp - 0x7c + 0x10 * k, &[]);
        }
        err
    }
}
