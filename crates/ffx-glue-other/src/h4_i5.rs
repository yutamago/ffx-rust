//! Continuation round 7 of list `h4-glue-other`, batch 5: a particle
//! model's projected positions.

use ffx_bridge::x87::{div, load, mul, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{int_f32, pass};
use crate::h4_i1::{GET_PTR32, MUL_MATRIX, SCREEN_MATRIX};

/// `FFXVu0ApplyMatrix(out, matrix, in)` (cdecl).
pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
/// Bytes of [`project_positions`]'s frame, from `ebp - 0x64`: the matrix
/// (`+0`, 0x40 bytes), the input vector (`+0x40`) and the output vector
/// (`+0x50`).
pub const PROJECT_FRAME: u32 = 0x60;

/// `pppMdl_projectPositions_007161f0(part, job)` (cdecl, from the
/// particle fill jobs; `frame` see [`PROJECT_FRAME`]): the screen matrix
/// times the job's matrix `[job + 0x20]` (and again times `[job + 0x44]`
/// when set) into the frame's matrix; then each of the model's
/// (`get_ptr32_func([part + 0x1c])`) `i16` count at `+0x12` of positions
/// from `[job + 0x38]` (floats through `fld`/`fstp`, 16-byte stride,
/// when bit 0x400 of `[job]` is set; else `i16` triples, 6-byte stride)
/// is made a vector (w 1.0), transformed (`FFXVu0ApplyMatrix`) and
/// divided by its w (the reciprocal stored as f32; in the `i16` path a w
/// of 0 gives 0, `fucomp` + `test ah, 0x44; jp`), the three products
/// stored to the frame and to `[job + 4]` (16-byte stride) with w as is.
/// The count is read again after each vertex.
///
/// Returns the last count read; when the first is not positive, 0 for
/// the float path and the model's address for the `i16` path (the
/// original's `eax` there).
pub fn project_positions(h: &mut impl Host, frame: u32, part: u32, job: u32) -> u32 {
    let matrix = frame;
    let input = frame.wrapping_add(0x40);
    let output = frame.wrapping_add(0x50);
    let mut src: u32 = h.read(job.wrapping_add(0x38));
    let mut dst: u32 = h.read(job.wrapping_add(4));
    let handle: u32 = h.read(part.wrapping_add(0x1c));
    let model = h.call_sym(GET_PTR32, Conv::Cdecl, &[handle]);
    let screen = h.rebase(SCREEN_MATRIX);
    let m: u32 = h.read(job.wrapping_add(0x20));
    h.call_sym(MUL_MATRIX, Conv::Cdecl, &[matrix, screen, m]);
    let m: u32 = h.read(job.wrapping_add(0x44));
    if m != 0 {
        h.call_sym(MUL_MATRIX, Conv::Cdecl, &[matrix, screen, m]);
    }
    let floats = h.read::<u32>(job) & 0x400 != 0;
    let count_at = model.wrapping_add(0x12);
    let mut n = i32::from(h.read::<i16>(count_at));
    if n <= 0 {
        return if floats { 0 } else { model };
    }
    let one = 1.0f32.to_bits();
    let mut i = 0;
    while i < n {
        for k in 0..3 {
            let v = if floats {
                pass(h.read(src.wrapping_add(4 * k)))
            } else {
                int_f32(i32::from(h.read::<i16>(src.wrapping_add(2 * k))))
            };
            h.write(input.wrapping_add(4 * k), v);
        }
        h.write(input.wrapping_add(0xc), one);
        h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[output, matrix, input]);
        let w: u32 = h.read(output.wrapping_add(0xc));
        let wf = load(w);
        // the i16 path tests w against 0 first (equal, -0.0 too: 0)
        let r = if !floats && wf == 0.0 {
            0.0f32.to_bits()
        } else {
            store(div(1.0, wf))
        };
        let r = load(r);
        let x = load(h.read(output));
        let x = store(mul(x, r));
        h.write(output, x);
        h.write(dst, x);
        let y = load(h.read(output.wrapping_add(4)));
        let y = store(mul(y, r));
        h.write(output.wrapping_add(4), y);
        h.write(dst.wrapping_add(4), y);
        let z = load(h.read(output.wrapping_add(8)));
        let z = store(mul(r, z));
        h.write(output.wrapping_add(8), z);
        h.write(dst.wrapping_add(8), z);
        let w: u32 = h.read(output.wrapping_add(0xc));
        h.write(dst.wrapping_add(0xc), w);
        src = src.wrapping_add(if floats { 0x10 } else { 6 });
        dst = dst.wrapping_add(0x10);
        i += 1;
        n = i32::from(h.read::<i16>(count_at));
    }
    n as u32
}

// ---- GDraw's depth-stencil state cache

/// `"GDraw D3D out of memory in %s%s"` (`0x00b7a4ec`).
pub const D3D_OUT_OF_MEMORY: u32 = 0x00b7_a4ec;
/// `"GDraw D3D error in %s%s: 0x%08x"` (`0x00b7a50c`).
pub const D3D_ERROR: u32 = 0x00b7_a50c;
/// `"CreateDepthStencilState"` (`0x00b7a588`).
pub const CREATE_DEPTH_STENCIL_STATE: u32 = 0x00b7_a588;
/// The empty string (`0x00b8fbeb`).
pub const EMPTY_STRING: u32 = 0x00b8_fbeb;
/// Bytes of [`depth_stencil_state`]'s frame: the
/// `D3D11_DEPTH_STENCIL_DESC` at `ebp - 0x38`.
pub const DEPTH_FRAME: u32 = 0x34;

/// `gdraw_getDepthStencilState_00a3d020(depth, write, read_mask,
/// write_mask)` (cdecl; `desc` its descriptor at `ebp - 0x38`): `GDraw`'s
/// 32-entry cache (keys `+0x800`, states `+0x880`, use stamps `+0x900`,
/// the clock `+0x980` advanced each call) looked up by the key
/// `((write_mask << 8 | read_mask) << 7 | depth) * 2 | write`. A hit
/// renews its stamp. A miss takes the first empty entry or else the
/// least recently used (the largest unsigned age), releases its state,
/// copies the depth state `+0x7f0 + 4 (write + 2 depth)`'s descriptor
/// (`GetDesc`), sets the stencil on with both masks and both faces' ops
/// (keep, keep, keep, always), and creates the state on `GDraw`'s device
/// (`CreateDepthStencilState`, warning on failure). Returns the entry's
/// state.
pub fn depth_stencil_state(
    h: &mut impl Host,
    desc: u32,
    depth: u32,
    write: u32,
    read_mask: u32,
    write_mask: u32,
) -> u32 {
    let (rm, wm) = (read_mask & 0xff, write_mask & 0xff);
    let key =
        ((((wm << 8) | rm) << 7) | depth).wrapping_add((((wm << 8) | rm) << 7) | depth) | write;
    let gdraw_at = h.rebase(crate::h4_i3::GDRAW);
    let gd: u32 = h.read(gdraw_at);
    let clock: u32 = h.read(gd.wrapping_add(0x980));
    h.write(gd.wrapping_add(0x980), clock.wrapping_add(1));
    let stamp: u32 = h.read(gd.wrapping_add(0x980));
    for i in 0..0x20u32 {
        if h.read::<u32>(gd.wrapping_add(0x800 + 4 * i)) == key {
            h.write(gd.wrapping_add(0x900 + 4 * i), stamp);
            return h.read(gd.wrapping_add(0x880 + 4 * i));
        }
    }
    let mut slot = 0u32;
    let mut oldest = 0u32;
    for i in 0..0x20u32 {
        if h.read::<u32>(gd.wrapping_add(0x880 + 4 * i)) == 0 {
            slot = i;
            break;
        }
        let age = stamp.wrapping_sub(h.read(gd.wrapping_add(0x900 + 4 * i)));
        if age > oldest {
            oldest = age;
            slot = i;
        }
    }
    let state_off = 0x880 + 4 * slot;
    let mut gd = gd;
    let old: u32 = h.read(gd.wrapping_add(state_off));
    if old != 0 {
        let vtable: u32 = h.read(old);
        let release: u32 = h.read(vtable.wrapping_add(8));
        h.call(release, Conv::Stdcall, &[old]);
        let at = gd.wrapping_add(state_off);
        gd = h.read(gdraw_at);
        h.write(at, 0u32);
    }
    let base: u32 = h.read(
        gd.wrapping_add(0x7f0)
            .wrapping_add(write.wrapping_add(depth.wrapping_mul(2)).wrapping_mul(4)),
    );
    let vtable: u32 = h.read(base);
    let get_desc: u32 = h.read(vtable.wrapping_add(0x1c));
    h.call(get_desc, Conv::Stdcall, &[base, desc]);
    h.write(desc.wrapping_add(0x10), read_mask as u8);
    h.write(desc.wrapping_add(0x11), write_mask as u8);
    let gd: u32 = h.read(gdraw_at);
    h.write(desc.wrapping_add(0xc), 1u32);
    for (off, v) in [
        (0x14u32, 1u32),
        (0x18, 1),
        (0x1c, 3),
        (0x20, 3),
        (0x24, 1),
        (0x28, 1),
        (0x2c, 3),
        (0x30, 3),
    ] {
        h.write(desc.wrapping_add(off), v);
    }
    let device: u32 = h.read(gd);
    let vtable: u32 = h.read(device);
    let create: u32 = h.read(vtable.wrapping_add(0x54));
    let hr = h.call(
        create,
        Conv::Stdcall,
        &[device, desc, gd.wrapping_add(state_off)],
    );
    if (hr as i32) < 0 {
        let send: u32 = h.read(h.rebase(crate::h4_i1::SEND_WARNING_IAT));
        let what = h.rebase(CREATE_DEPTH_STENCIL_STATE);
        let empty = h.rebase(EMPTY_STRING);
        if hr == 0x8007_000e {
            let f = h.rebase(D3D_OUT_OF_MEMORY);
            h.call(send, Conv::Cdecl, &[0, f, what, empty]);
        } else {
            let f = h.rebase(D3D_ERROR);
            h.call(send, Conv::Cdecl, &[0, f, what, empty, hr]);
        }
    }
    let gd: u32 = h.read(gdraw_at);
    h.write(gd.wrapping_add(0x800 + 4 * slot), key);
    h.write(gd.wrapping_add(0x900 + 4 * slot), stamp);
    h.read(gd.wrapping_add(state_off))
}

// ---- Distortion_setupDistortionGrid

/// `Phyre_PhyreMalloc_0042fb80(n, align)` (cdecl).
pub const PHYRE_MALLOC_ALIGNED: u32 = 0x0042_fb80;
/// `PMeshSegmentEntries_construct_00434430(p, n)` (cdecl).
pub const SEGMENTS_CTOR: u32 = 0x0043_4430;
/// `PMeshSegmentEntries_destroy_00434210(p, n)` (cdecl).
pub const SEGMENTS_FREE: u32 = 0x0043_4210;
/// `Phyre_Ctor_0055dc40(stream)` (thiscall): a 0x40-byte vertex
/// stream's constructor.
pub const STREAM_CTOR: u32 = 0x0055_dc40;
/// `PArray40_setBuffer_00434e40(array; p, n)` (thiscall, ret 8).
pub const STREAMS_ADOPT: u32 = 0x0043_4e40;
/// `operator_new_006304c0(n)` (cdecl).
pub const OPERATOR_NEW_ARRAY: u32 = crate::h4_i2::OPERATOR_NEW_ARRAY;
/// The grid's four fills, in call order (thiscall on the distortion):
/// `Distortion_fillGridPositions_006c2ca0`, `Distortion_fillGridOnes_006c28e0`,
/// `Distortion_fillGridUV_006c2bb0`, `Distortion_buildGridIndices_006c2990`.
pub const GRID_SETUPS: [u32; 4] = [0x006c_2ca0, 0x006c_28e0, 0x006c_2bb0, 0x006c_2990];
/// `Phyre_PGeometry_FormatSize_0055bc40(format)` (cdecl).
pub const INDEX_SIZE: u32 = 0x0055_bc40;
/// `Phyre_PGeometry_PIndexDataBlockNVN_isBound(block)` (thiscall): `al`.
pub const INDEX_IS_BOUND: u32 = 0x0056_2db0;
/// `PIndexBufferD3D11_release_005648c0(block)` (thiscall).
pub const INDEX_UNBIND: u32 = 0x0056_48c0;

/// `Distortion_setupDistortionGrid(this)` (`0x006c2ff0`, thiscall; the SEH
/// frame is not kept): the mesh `[this + 0x34]` given one segment (a new
/// 0x6c-byte block, the old one freed when owned) unless it has one;
/// the segment (`[mesh + 4]`) reset (`+0` 0, `+0x10` 2) and given three
/// vertex streams (0xc0 bytes) unless it has them (`+0x28` 3); the grid's
/// arrays allocated (`+0x68`..`+0x74`: 0x3cc, 0x510, 0x288 and 0x600
/// bytes) and filled by its four setups; the index block (`+0x30`) set to
/// 0x180 indices of format 8 from 0 (count 0x50), its byte size
/// (`+0x64`) updated, unbinding it first when bound and the size
/// changed.
pub fn setup_distortion_grid(h: &mut impl Host, this: u32) {
    let mesh: u32 = h.read(this.wrapping_add(0x34));
    if h.read::<u32>(mesh) != 1 {
        let block = h.call_sym(PHYRE_MALLOC_ALIGNED, Conv::Cdecl, &[0x6c, 4]);
        if block != 0 {
            h.call_sym(SEGMENTS_CTOR, Conv::Cdecl, &[block, 1]);
            let count: u32 = h.read(mesh);
            let old: u32 = h.read(mesh.wrapping_add(4));
            if old != block && count & 0x8000_0000 == 0 {
                h.call_sym(SEGMENTS_FREE, Conv::Cdecl, &[old, count & 0x7fff_ffff]);
            }
            h.write(mesh.wrapping_add(4), block);
            h.write(mesh, 1u32);
        }
    }
    let mesh: u32 = h.read(this.wrapping_add(0x34));
    let segment: u32 = h.read(mesh.wrapping_add(4));
    h.write(segment, 0u32);
    h.write(segment.wrapping_add(0x10), 2u32);
    if h.read::<u32>(segment.wrapping_add(0x28)) != 3 {
        let streams = h.call_sym(PHYRE_MALLOC_ALIGNED, Conv::Cdecl, &[0xc0, 4]);
        if streams != 0 {
            for k in 0..3u32 {
                let s = streams.wrapping_add(0x40 * k);
                if s != 0 {
                    h.call_sym(STREAM_CTOR, Conv::Thiscall(s), &[]);
                }
            }
            h.call_sym(
                STREAMS_ADOPT,
                Conv::Thiscall(segment.wrapping_add(0x28)),
                &[streams, 3],
            );
        }
    }
    for (off, n) in [
        (0x68u32, 0x3ccu32),
        (0x6c, 0x510),
        (0x70, 0x288),
        (0x74, 0x600),
    ] {
        let p = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[n]);
        h.write(this.wrapping_add(off), p);
    }
    for f in GRID_SETUPS {
        h.call_sym(f, Conv::Thiscall(this), &[]);
    }
    h.write(segment.wrapping_add(0x38), 0x180u32);
    h.write(segment.wrapping_add(0x3c), 8u8);
    h.write(segment.wrapping_add(0x30), 0u32);
    h.write(segment.wrapping_add(0x34), 0x50u32);
    let size = h
        .call_sym(INDEX_SIZE, Conv::Cdecl, &[8])
        .wrapping_mul(0x180);
    if h.read::<u32>(segment.wrapping_add(0x64)) != size {
        let block = segment.wrapping_add(0x30);
        if h.call_sym(INDEX_IS_BOUND, Conv::Thiscall(block), &[]) as u8 != 0 {
            h.call_sym(INDEX_UNBIND, Conv::Thiscall(block), &[]);
        }
        h.write(segment.wrapping_add(0x64), size);
    }
}

// ---- GDraw's depth buffer

/// `"CreateTexture2D"` (`0x00b7a688`).
pub const CREATE_TEXTURE_2D: u32 = 0x00b7_a688;
/// `"CreateDepthStencilView while creating rendertarget"` (`0x00b7a7a4`).
pub const CREATE_DSV: u32 = 0x00b7_a7a4;
/// Bytes of [`depth_buffer`]'s frame: the `D3D11_TEXTURE2D_DESC` at
/// `ebp - 0x30`.
pub const TEXTURE_FRAME: u32 = 0x2c;

/// One `IUnknown::Release` (vtable `+8`, stdcall) of `obj`.
fn release(h: &mut impl Host, obj: u32) {
    let vtable: u32 = h.read(obj);
    let f: u32 = h.read(vtable.wrapping_add(8));
    h.call(f, Conv::Stdcall, &[obj]);
}

/// `gdraw_createDepthBuffer_00a3b840(stats)` (cdecl; `desc` its texture
/// descriptor at `ebp - 0x30`): once (`GDraw`'s view `+0x788` is 0) a
/// D24S8 depth texture of `GDraw`'s size (`+0x6c0` x `+0x6c4`) made on the
/// device (`CreateTexture2D` into `+0x794`) with its view
/// (`CreateDepthStencilView` into `+0x788`); on success the stats get
/// flag 4 (`u16 +0`), one more texture (`u16 +0xa`) and its bytes (`+0x1c`
/// += w h 4) and the view is cleared (depth 1.0, stencil 0); on failure a
/// warning and both released. Returns the view (`+0x788`), 0 when the
/// failure released none.
pub fn depth_buffer(h: &mut impl Host, desc: u32, stats: u32) -> u32 {
    let gdraw_at = h.rebase(crate::h4_i3::GDRAW);
    let gd: u32 = h.read(gdraw_at);
    let view: u32 = h.read(gd.wrapping_add(0x788));
    if view != 0 {
        return view;
    }
    let w: u32 = h.read(gd.wrapping_add(0x6c0));
    h.write(desc, w);
    let hh: u32 = h.read(gd.wrapping_add(0x6c4));
    h.write(desc.wrapping_add(4), hh);
    for (off, v) in [
        (8u32, 1u32),
        (0xc, 1),
        (0x10, 0x2d),
        (0x14, 1),
        (0x18, 0),
        (0x1c, 0),
        (0x20, 0x40),
        (0x24, 0),
        (0x28, 0),
    ] {
        h.write(desc.wrapping_add(off), v);
    }
    let device: u32 = h.read(gd);
    let vtable: u32 = h.read(device);
    let create: u32 = h.read(vtable.wrapping_add(0x14));
    let mut hr = h.call(
        create,
        Conv::Stdcall,
        &[device, desc, 0, gd.wrapping_add(0x794)],
    );
    let mut what = CREATE_TEXTURE_2D;
    if hr as i32 >= 0 {
        let gd: u32 = h.read(gdraw_at);
        let device: u32 = h.read(gd);
        let vtable: u32 = h.read(device);
        let create: u32 = h.read(vtable.wrapping_add(0x28));
        let texture: u32 = h.read(gd.wrapping_add(0x794));
        hr = h.call(
            create,
            Conv::Stdcall,
            &[device, texture, 0, gd.wrapping_add(0x788)],
        );
        what = CREATE_DSV;
        if hr as i32 >= 0 {
            let gd: u32 = h.read(gdraw_at);
            let f: u16 = h.read(stats);
            h.write(stats, f | 4);
            let n: u16 = h.read(stats.wrapping_add(0xa));
            h.write(stats.wrapping_add(0xa), n.wrapping_add(1));
            let hh: u32 = h.read(gd.wrapping_add(0x6c4));
            let w: u32 = h.read(gd.wrapping_add(0x6c0));
            let total: u32 = h.read(stats.wrapping_add(0x1c));
            h.write(
                stats.wrapping_add(0x1c),
                total.wrapping_add(hh.wrapping_mul(w) << 2),
            );
            let context: u32 = h.read(gd.wrapping_add(4));
            let vtable: u32 = h.read(context);
            let clear: u32 = h.read(vtable.wrapping_add(0xd4));
            let view: u32 = h.read(gd.wrapping_add(0x788));
            h.call(
                clear,
                Conv::Stdcall,
                &[context, view, 3, 1.0f32.to_bits(), 0],
            );
            let gd: u32 = h.read(gdraw_at);
            return h.read(gd.wrapping_add(0x788));
        }
    }
    let send: u32 = h.read(h.rebase(crate::h4_i1::SEND_WARNING_IAT));
    let what = h.rebase(what);
    let empty = h.rebase(EMPTY_STRING);
    if hr == 0x8007_000e {
        let f = h.rebase(D3D_OUT_OF_MEMORY);
        h.call(send, Conv::Cdecl, &[0, f, what, empty]);
    } else {
        let f = h.rebase(D3D_ERROR);
        h.call(send, Conv::Cdecl, &[0, f, what, empty, hr]);
    }
    let gd: u32 = h.read(gdraw_at);
    let texture: u32 = h.read(gd.wrapping_add(0x794));
    let gd = if texture == 0 {
        gd
    } else {
        release(h, texture);
        h.write(gd.wrapping_add(0x794), 0u32);
        h.read(gdraw_at)
    };
    let view: u32 = h.read(gd.wrapping_add(0x788));
    if view == 0 {
        return 0;
    }
    release(h, view);
    let fresh: u32 = h.read(gdraw_at);
    h.write(gd.wrapping_add(0x788), 0u32);
    h.read(fresh.wrapping_add(0x788))
}
