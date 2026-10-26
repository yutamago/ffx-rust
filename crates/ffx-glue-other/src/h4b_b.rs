//! List `h4-glue-other`, second agent (h4b), batch 2: Iggy's `GDraw`
//! (Direct3D 11) callbacks for textures and vertex buffers.
//!
//! `GDraw`'s imports (`_IggyGDrawMallocAnnotated@12`, `_IggyGDrawFree@4`,
//! `IggyGDrawSendWarning`) are called through their import slots; the
//! device's `CreateBuffer` and the buffers' `Release` through their
//! vtables (stdcall, `this` first). Frames the originals hand the device
//! are laid out at the original's `ebp` offsets.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

/// `_IggyGDrawMallocAnnotated@12(size, file, line)` (import slot).
pub const GDRAW_MALLOC: Global<u32> = Global::new(0x00b0_c86c);
/// `_IggyGDrawFree@4(block)` (import slot).
pub const GDRAW_FREE: Global<u32> = Global::new(0x00b0_c870);
/// `IggyGDrawSendWarning(id, format, ...)` (import slot; cdecl, varargs).
pub const GDRAW_WARNING: Global<u32> = Global::new(0x00b0_c874);
/// `GDraw`'s state: `+0` the device, `+0xab0` the texture pool.
pub const GDRAW_STATE: Global<u32> = Global::new(0x01a8_58b4);
/// The source file `GDraw` names in its allocations.
pub const GDRAW_SOURCE: u32 = 0x00b7_a5e8;
/// `opd_config_memory(pool, size, c)` (cdecl).
pub const CONFIG_MEMORY: u32 = 0x00a3_b210;
/// `IggyHeap_moveBlock_00a39a90(block, size, owner, kind)` (cdecl).
pub const MOVE_BLOCK: u32 = 0x00a3_9a90;

/// `IggyGDrawSendWarning(0, format, words...)` through its slot.
fn warn(h: &mut impl Host, format: u32, words: &[u32]) -> u32 {
    let f = GDRAW_WARNING.get(h);
    let mut args = vec![0, h.rebase(format)];
    args.extend_from_slice(words);
    h.call(f, Conv::Cdecl, &args)
}

/// `gdraw_makeTextureBegin_00a3a070(owner, w, h, format, flags, info,
/// stats)` (stdcall, `ret 0x1c`): a texture's staging memory. Sizes from
/// 0x4000 up (signed) are refused (a warning, 0). `format` 0 is RGBA
/// (DXGI 0x1c, 4 bytes a texel), any other a single channel (0x3d, 1
/// byte). The size sums every mip level while bit 0 of `flags` asks for
/// mips (`w >> k` and `h >> k` at least 1, until both are 0; the shift
/// count wraps at 32 as `sar` does). The block (`GDraw` malloc, line
/// 0x1b2) goes to `info + 0`; without it a warning and 0. A record from
/// the pool (`opd_config_memory(state + 0xab0, size, stats)`, freed block
/// and 0 without one) gets `w`, `h` at `+0xc`/`+0x10` and three cleared
/// words; `info` the record (`+0x18`), the owner, flags, format, size,
/// texel size, `w`, the level count, the pitch `w * texel`, `h`: 1.
#[allow(clippy::too_many_arguments)]
pub fn make_texture_begin(
    h: &mut impl Host,
    owner: u32,
    w: u32,
    hh: u32,
    format: u32,
    flags: u32,
    info: u32,
    stats: u32,
) -> u32 {
    let (wi, hi) = (w.cast_signed(), hh.cast_signed());
    if wi >= 0x4000 || hi >= 0x4000 {
        warn(h, 0x00b7_a5a0, &[w, hh]);
        return 0;
    }
    let (dxgi, texel) = if format == 0 { (0x1c, 4) } else { (0x3d, 1) };
    let mut size = 0u32;
    let mut levels = 0u32;
    loop {
        let lw = wi.wrapping_shr(levels).max(1);
        let lh = hi.wrapping_shr(levels).max(1);
        size = size.wrapping_add(lh.wrapping_mul(lw).wrapping_mul(texel) as u32);
        levels += 1;
        if flags & 1 == 0 || (wi.wrapping_shr(levels) == 0 && hi.wrapping_shr(levels) == 0) {
            break;
        }
    }
    let malloc = GDRAW_MALLOC.get(h);
    let source = h.rebase(GDRAW_SOURCE);
    let block = h.call(malloc, Conv::Stdcall, &[size, source, 0x1b2]);
    h.write(info, block);
    if block == 0 {
        warn(h, 0x00b7_a638, &[w, hh]);
        return 0;
    }
    let state = GDRAW_STATE.get(h);
    let pool: u32 = h.read(state.wrapping_add(0xab0));
    let record = h.call_sym(CONFIG_MEMORY, Conv::Cdecl, &[pool, size, stats]);
    if record == 0 {
        let free = GDRAW_FREE.get(h);
        let first: u32 = h.read(info);
        h.call(free, Conv::Stdcall, &[first]);
        return 0;
    }
    h.write(record + 0xc, w);
    h.write(record + 0x10, hh);
    for off in [0, 4, 8] {
        h.write(record + off, 0u32);
    }
    h.write(info + 0x18, record);
    h.write(info + 0x1c, owner);
    h.write(info + 0x40, flags);
    h.write(info + 0x44, dxgi);
    h.write(info + 0x48, size);
    h.write(info + 0x50, texel as u32);
    h.write(info + 0x38, w);
    h.write(info + 0x4c, levels);
    h.write(info + 8, (texel as u32).wrapping_mul(w));
    h.write(info + 0xc, 0u32);
    h.write(info + 0x3c, hh);
    h.write(info + 4, hh);
    1
}

/// `gdraw_makeVertexBufferEnd_00a3a5f0(info, stats)` (stdcall, `ret 8`;
/// `stats` unread): creates the vertex and the index buffer of a vertex
/// buffer record (`info + 0x10`) from the staging blocks (`info + 0`,
/// `+4`; sizes `+8`, `+0xc`) with the device's `CreateBuffer` (vtable
/// `+0xc`; immutable, bind 1 then 2) into the record's `+0`/`+4`. The
/// descriptions and initial data live in the caller's frame (`frame`:
/// the original's `ebp - 0x4c` up to `ebp - 4`, 0x48 bytes). On success the record moves
/// to kind 2 (`IggyHeap_moveBlock(record, size, info + 0x14, 2)`);
/// on a failed call (negative `HRESULT`) both buffers are released, the
/// record goes back to the front of its heap's free ring (`[record +
/// 0x18] + 0x10`, links `+0x1c`/`+0x20`, state nibble cleared) and a
/// warning names the error (out of memory `0x8007000e` without the code).
/// Both staging blocks are freed. Returns the record, 0 on failure.
pub fn make_vertex_buffer_end(h: &mut impl Host, frame: u32, info: u32) -> u32 {
    let ebp = frame + 0x4c;
    let size0: u32 = h.read(info + 8);
    let data0: u32 = h.read(info);
    let size1: u32 = h.read(info + 0xc);
    let record: u32 = h.read(info + 0x10);
    let data1: u32 = h.read(info + 4);
    let words: [(u32, u32); 17] = [
        (0x34, size0),
        (0x4c, data0),
        (0x1c, size1),
        (0x40, data1),
        (0x30, 1),
        (0x2c, 1),
        (0x28, 0),
        (0x24, 0),
        (0x20, 0),
        (0x48, 0),
        (0x44, 0),
        (0x18, 1),
        (0x14, 2),
        (0x10, 0),
        (0xc, 0),
        (0x8, 0),
        (0x3c, 0),
    ];
    for (off, v) in words {
        h.write(ebp - off, v);
    }
    h.write(ebp - 0x38, 0u32);
    let device: u32 = h.read(GDRAW_STATE.get(h));
    let create: u32 = h.read(h.read::<u32>(device) + 0xc);
    let mut hr = h.call(
        create,
        Conv::Stdcall,
        &[device, ebp - 0x34, ebp - 0x4c, record],
    );
    if hr.cast_signed() >= 0 {
        let device: u32 = h.read(GDRAW_STATE.get(h));
        let create: u32 = h.read(h.read::<u32>(device) + 0xc);
        hr = h.call(
            create,
            Conv::Stdcall,
            &[device, ebp - 0x1c, ebp - 0x40, record + 4],
        );
    }
    let result = if hr.cast_signed() >= 0 {
        let size = h
            .read::<u32>(info + 8)
            .wrapping_add(h.read::<u32>(info + 0xc));
        let owner: u32 = h.read(info + 0x14);
        h.call_sym(MOVE_BLOCK, Conv::Cdecl, &[record, size, owner, 2]);
        record
    } else {
        for off in [0, 4] {
            let buffer: u32 = h.read(record + off);
            if buffer != 0 {
                let release: u32 = h.read(h.read::<u32>(buffer) + 8);
                h.call(release, Conv::Stdcall, &[buffer]);
                h.write(record + off, 0u32);
            }
        }
        requeue_free(h, record);
        let what = [0x00b7_a6e4, 0x00b7_a6cc];
        if hr == 0x8007_000e {
            let w = what.map(|s| h.rebase(s));
            warn(h, 0x00b7_a4ec, &w);
        } else {
            let w = what.map(|s| h.rebase(s));
            warn(h, 0x00b7_a50c, &[w[0], w[1], hr]);
        }
        0
    };
    let free = GDRAW_FREE.get(h);
    let first: u32 = h.read(info);
    h.call(free, Conv::Stdcall, &[first]);
    let second: u32 = h.read(info + 4);
    h.call(free, Conv::Stdcall, &[second]);
    result
}

/// A heap block back into its heap's free ring: unlinked (links `+0x1c`
/// next, `+0x20` prev), linked in before the ring's head
/// `[block + 0x18] + 0x10`, the state nibble of `+0x30` cleared.
fn requeue_free(h: &mut impl Host, block: u32) {
    let prev: u32 = h.read(block + 0x20);
    let next: u32 = h.read(block + 0x1c);
    let heap: u32 = h.read(block + 0x18);
    h.write(prev + 0x1c, next);
    let next: u32 = h.read(block + 0x1c);
    let prev: u32 = h.read(block + 0x20);
    h.write(next + 0x20, prev);
    let head = heap.wrapping_add(0x10);
    h.write(block + 0x1c, head);
    let first: u32 = h.read(head + 0x20);
    h.write(block + 0x20, first);
    h.write(first + 0x1c, block);
    let head: u32 = h.read(block + 0x1c);
    h.write(head + 0x20, block);
    let v: u32 = h.read(block + 0x30);
    h.write(block + 0x30, v & 0x0fff_ffff);
}

/// `obj->Release()` (vtable `+8`, stdcall `this`) when `obj` is set:
/// `Some(eax)`.
fn release(h: &mut impl Host, obj: u32) -> Option<u32> {
    if obj == 0 {
        return None;
    }
    let f: u32 = h.read(h.read::<u32>(obj) + 8);
    Some(h.call(f, Conv::Stdcall, &[obj]))
}

/// The out-of-memory or error warning after a failed device call:
/// `IggyGDrawSendWarning(0, format, call, what[, hr])`.
fn warn_hr(h: &mut impl Host, hr: u32, call: u32, what: u32) -> u32 {
    let (c, w) = (h.rebase(call), h.rebase(what));
    if hr == 0x8007_000e {
        warn(h, 0x00b7_a4ec, &[c, w])
    } else {
        warn(h, 0x00b7_a50c, &[c, w, hr])
    }
}

/// `gdraw_createRowTexture_00a3a9f0(w, texels)` (stdcall, `ret 8`): the
/// state's `w` x 1 RGBA texture (`+0x798`) and its shader resource view
/// (`+0x79c`): both released first, then the device's `CreateTexture2D`
/// (vtable `+0x14`) from the texels (pitch `4 w`) and
/// `CreateShaderResourceView` (`+0x1c`, no description). The description
/// and the initial data live in the caller's frame (`frame`: the
/// original's `ebp - 0x3c`, 0x38 bytes). A failure is reported
/// (`IggyGDrawSendWarning`); a failed view releases the texture again.
/// Returns the original's `eax`: the view's `HRESULT`, or the warning's
/// or the release's.
pub fn create_row_texture(h: &mut impl Host, frame: u32, w: u32, texels: u32) -> u32 {
    let ebp = frame + 0x3c;
    for off in [0x79c, 0x798] {
        let state = GDRAW_STATE.get(h);
        if release(h, h.read(state + off)).is_some() {
            h.write(state + off, 0u32);
        }
    }
    let state = GDRAW_STATE.get(h);
    let desc: [(u32, u32); 14] = [
        (0x30, w),
        (0x38, w.wrapping_shl(2)),
        (0x2c, 1),
        (0x28, 1),
        (0x24, 1),
        (0x20, 0x1c),
        (0x1c, 1),
        (0x18, 0),
        (0x14, 1),
        (0x10, 8),
        (0xc, 0),
        (0x8, 0),
        (0x3c, texels),
        (0x34, 0),
    ];
    for (off, v) in desc {
        h.write(ebp - off, v);
    }
    let device: u32 = h.read(state);
    let create: u32 = h.read(h.read::<u32>(device) + 0x14);
    let hr = h.call(
        create,
        Conv::Stdcall,
        &[device, ebp - 0x30, ebp - 0x3c, state + 0x798],
    );
    if hr.cast_signed() < 0 {
        return warn_hr(h, hr, 0x00b7_a688, 0x00b8_fbeb);
    }
    let state = GDRAW_STATE.get(h);
    let device: u32 = h.read(state);
    let view: u32 = h.read(h.read::<u32>(device) + 0x1c);
    let texture: u32 = h.read(state + 0x798);
    let hr = h.call(view, Conv::Stdcall, &[device, texture, 0, state + 0x79c]);
    if hr.cast_signed() >= 0 {
        return hr;
    }
    let mut eax = warn_hr(h, hr, 0x00b7_a698, 0x00b7_a6b4);
    let state = GDRAW_STATE.get(h);
    if let Some(r) = release(h, h.read(state + 0x798)) {
        h.write(state + 0x798, 0u32);
        eax = r;
    }
    eax
}

/// `GDraw`'s viewport setter `0x00a3ce00(x, y, w, h)` (cdecl).
pub const SET_VIEWPORT: u32 = 0x00a3_ce00;
/// `GDraw`'s projection setter `0x00a3c450(x, w, h, y)` (cdecl).
pub const SET_PROJECTION: u32 = 0x00a3_c450;
/// `GDraw`'s quad draw `0x00a36090(colour, a, x0, y0, x1, y1)` (cdecl).
pub const DRAW_QUAD: u32 = 0x00a3_6090;
/// The constant colour / blend factor (`f32` x 4).
pub const QUAD_COLOUR: u32 = 0x00b7_a3f8;
/// `crt_memset_thunk(p, c, n)`.
pub const MEMSET: u32 = 0x0094_964c;

/// The context's blend and depth-stencil states for a quad (vtable
/// `+0x8c`, `+0x90`): `OMSetBlendState(state + 0x7d4, colour, ~0)`,
/// `OMSetDepthStencilState(depth, 0)`; then `+0x9c8` cleared.
fn quad_states(h: &mut impl Host, ctx: u32, depth: Option<u32>) {
    let state = GDRAW_STATE.get(h);
    let blend: u32 = h.read(state + 0x7d4);
    let colour = h.rebase(QUAD_COLOUR);
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x8c);
    h.call(f, Conv::Stdcall, &[ctx, blend, colour, 0xffff_ffff]);
    let depth = match depth {
        Some(d) => d,
        None => {
            let state = GDRAW_STATE.get(h);
            h.read(state + 0x7f0)
        }
    };
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x90);
    h.call(f, Conv::Stdcall, &[ctx, depth, 0]);
}

/// The shared tail of the quad draws: the pixel shader (vtable `+0x24`,
/// `shader`), the constant buffer `state + 0x988` mapped (`+0x38`,
/// write-discard, the mapping in the frame at `mapped`) and cleared (64
/// bytes), unmapped (`+0x3c`) and bound (`+0x40`, slot 0). `Err(eax)`
/// when it stops: a failed map (the warning's `eax`) or no data (0).
fn quad_constants(h: &mut impl Host, ctx: u32, shader: u32, mapped: u32) -> Result<(), u32> {
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x24);
    h.call(f, Conv::Stdcall, &[ctx, shader, 0, 0]);
    let state = GDRAW_STATE.get(h);
    let c: u32 = h.read(state + 4);
    let buffer: u32 = h.read(state + 0x988);
    let map: u32 = h.read(h.read::<u32>(c) + 0x38);
    let hr = h.call(map, Conv::Stdcall, &[c, buffer, 0, 4, 0, mapped]);
    if hr.cast_signed() < 0 {
        return Err(warn_hr(h, hr, 0x00b6_868c, 0x00b7_a408));
    }
    let data: u32 = h.read(mapped);
    if data == 0 {
        return Err(0);
    }
    h.call_sym(MEMSET, Conv::Cdecl, &[data, 0, 0x40]);
    let state = GDRAW_STATE.get(h);
    let c: u32 = h.read(state + 4);
    let buffer: u32 = h.read(state + 0x988);
    let unmap: u32 = h.read(h.read::<u32>(c) + 0x3c);
    h.call(unmap, Conv::Stdcall, &[c, buffer, 0]);
    let state = GDRAW_STATE.get(h);
    let bind: u32 = h.read(h.read::<u32>(ctx) + 0x40);
    h.call(bind, Conv::Stdcall, &[ctx, 0, 1, state + 0x988]);
    Ok(())
}

/// An `i32` through `fild` and `fstp dword`.
fn int_f32(n: u32) -> u32 {
    crate::h4_e1::int_f32(n.cast_signed())
}

/// `gdraw_drawRectQuad_00a3bd40(rect, a)` (cdecl): a constant-colour quad
/// over `rect` (four `i32`: x0, y0, x1, y1) on the whole target: the
/// quad states (depth-stencil `state + 0x7f0`), the viewport (0, 0, the
/// target's `+0x6c0` x `+0x6c4`) and projection, pixel shader `+0x25c`,
/// the cleared constants (mapping at `mapped`, the original's `ebp -
/// 0x10`), then `0x00a36090(colour, a, f32 rect...)`. Returns its `eax`
/// (the warning's after a failed map, 0 without data).
pub fn draw_rect_quad(h: &mut impl Host, mapped: u32, rect: u32, a: u32) -> u32 {
    let state = GDRAW_STATE.get(h);
    let ctx: u32 = h.read(state + 4);
    quad_states(h, ctx, None);
    let state = GDRAW_STATE.get(h);
    h.write(state + 0x9c8, 0u32);
    let (w, hh): (u32, u32) = (h.read(state + 0x6c0), h.read(state + 0x6c4));
    h.call_sym(SET_VIEWPORT, Conv::Cdecl, &[0, 0, w, hh]);
    let state = GDRAW_STATE.get(h);
    let (w, hh): (u32, u32) = (h.read(state + 0x6c0), h.read(state + 0x6c4));
    h.call_sym(SET_PROJECTION, Conv::Cdecl, &[0, w, hh, 0]);
    let shader: u32 = h.read(state + 0x25c);
    if let Err(eax) = quad_constants(h, ctx, shader, mapped) {
        return eax;
    }
    let r: [u32; 4] = std::array::from_fn(|k| int_f32(h.read(rect + 4 * k as u32)));
    let colour = h.rebase(QUAD_COLOUR);
    h.call_sym(DRAW_QUAD, Conv::Cdecl, &[colour, a, r[0], r[1], r[2], r[3]])
}

/// `gdraw_drawTargetQuad_00a3bed0(a, depth, sub)` (cdecl): a quad over
/// the target's size (`+0x6e0` x `+0x6e4`, from 0): the quad states with
/// depth-stencil `depth`, the viewport (with `sub` the sub-rectangle
/// `+0x6c8`, `+0x6cc`, `+0x6d8`, `+0x6dc`, else 0, 0 and the size), the
/// projection, no pixel shader, the cleared constants (mapping at
/// `mapped`, the original's `ebp - 0xc`), then `0x00a36090(colour, a, 0,
/// 0, f32 w, f32 h)`. The original stores those two floats in its third
/// argument slot (`slots`: the caller's first argument word), the width
/// last. Returns as [`draw_rect_quad`].
pub fn draw_target_quad(
    h: &mut impl Host,
    mapped: u32,
    a: u32,
    depth: u32,
    sub: u32,
    slots: Option<u32>,
) -> u32 {
    let state = GDRAW_STATE.get(h);
    let ctx: u32 = h.read(state + 4);
    quad_states(h, ctx, Some(depth));
    let state = GDRAW_STATE.get(h);
    h.write(state + 0x9c8, 0u32);
    let view: [u32; 4] = if sub != 0 {
        [0x6c8, 0x6cc, 0x6d8, 0x6dc].map(|o| h.read(state + o))
    } else {
        [0, 0, h.read(state + 0x6e0), h.read(state + 0x6e4)]
    };
    h.call_sym(SET_VIEWPORT, Conv::Cdecl, &view);
    let state = GDRAW_STATE.get(h);
    let (w, hh): (u32, u32) = (h.read(state + 0x6c0), h.read(state + 0x6c4));
    h.call_sym(SET_PROJECTION, Conv::Cdecl, &[0, w, hh, 0]);
    if let Err(eax) = quad_constants(h, ctx, 0, mapped) {
        return eax;
    }
    let state = GDRAW_STATE.get(h);
    let fh = int_f32(h.read(state + 0x6e4));
    let fw = int_f32(h.read(state + 0x6e0));
    if let Some(s) = slots {
        h.write(s + 8, fw);
    }
    let colour = h.rebase(QUAD_COLOUR);
    h.call_sym(DRAW_QUAD, Conv::Cdecl, &[colour, a, 0, 0, fw, fh])
}
