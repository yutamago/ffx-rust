//! Continuation round 11 of list `h4-glue-other`, batch 1: the movie skip
//! prompt's screen quads.

use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

/// `Phyre_PCluster_create(cluster; n, type, flags)` (thiscall, ret 0xc) ->
/// a `PArray` of objects (`+0` the count, bit 31 a flag; `+4` the data).
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
/// `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl).
pub const MALLOC: u32 = 0x0042_fb80;
/// `PMeshSegmentEntries_construct_00434430(data, n)` (cdecl).
pub const SEGMENTS_CONSTRUCT: u32 = 0x0043_4430;
/// `PMeshSegmentEntries_destroy_00434210(data, n)` (cdecl).
pub const SEGMENTS_DESTROY: u32 = 0x0043_4210;
/// `Phyre_PhyreFree(p)` (cdecl).
pub const FREE: u32 = 0x0042_fb20;
/// `Phyre_PCluster_allocateInstanceList(cluster; n, type, flags)`
/// (thiscall, ret 0xc) -> the list (`+0x34` its first instance).
pub const ALLOCATE_INSTANCES: u32 = 0x0044_0b70;
/// `PMeshInstance_ctor_004f27e0(instance; mesh)` (thiscall, ret 4).
pub const MESH_INSTANCE_CTOR: u32 = 0x004f_27e0;
/// `Phyre_Ctor_0055dc40(this)` (thiscall).
pub const STREAM_CTOR: u32 = 0x0055_dc40;
/// `PArray40_setBuffer_00434e40(array; data, count)` (thiscall, ret 8).
pub const SET_BUFFER: u32 = 0x0043_4e40;
/// `PhyreCtor_withSub8_004835d0(buffer; stride, count, kind)` (thiscall,
/// ret 0xc).
pub const BUFFER_CTOR: u32 = 0x0048_35d0;
/// `Phyre_PGeometry_PDataBlockNVN_isBound(buffer)` (thiscall) -> `al`.
pub const IS_BOUND: u32 = 0x0056_2da0;
/// `PVertexBufferD3D11_release_005646d0(buffer)` (thiscall).
pub const RELEASE: u32 = 0x0056_46d0;
/// `PRingBuffer_submit_00434c50(buffer; &data)` (thiscall, ret 4): maps
/// the buffer, its data pointer to `*data`.
pub const MAP: u32 = 0x0043_4c50;
/// `PIndexBufferD3D11_unmapCopy_00564cb0(buffer; slot, &data)` (thiscall,
/// ret 8) -> 0 when done.
pub const UNMAP: u32 = 0x0056_4cb0;
/// `g_PhyreScene` (`0x00ccb9d8`, a pointer): its cluster at `+0x1284`.
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// The type descriptors the cluster creates: the mesh (`0x00c99c80`), the
/// material list (`0x00c998e8`), the mesh instance (`0x00ca7228`).
pub const MESH_TYPE: u32 = 0x00c9_9c80;
/// See [`MESH_TYPE`].
pub const MATERIALS_TYPE: u32 = 0x00c9_98e8;
/// See [`MESH_TYPE`].
pub const INSTANCE_TYPE: u32 = 0x00ca_7228;
/// The vertex streams' semantics: position (`0x00c98620`), texture
/// coordinates (`0x00c98638`).
pub const POSITION_SEMANTIC: u32 = 0x00c9_8620;
/// See [`POSITION_SEMANTIC`].
pub const UV_SEMANTIC: u32 = 0x00c9_8638;
/// The screen size (`f64` 1920 at `0x00b41d68`, 1080 at `0x00b41d58`) and
/// 2 (`0x00b92270`).
pub const SCREEN_WIDTH: u32 = 0x00b4_1d68;
/// See [`SCREEN_WIDTH`].
pub const SCREEN_HEIGHT: u32 = 0x00b4_1d58;
/// See [`SCREEN_WIDTH`].
pub const TWO: u32 = 0x00b9_2270;
/// Bytes of [`build_quad`]'s frame, from the original's `ebp - 0x20`: the
/// texture coordinates' mapping (`+0`) and the positions' (`+8`).
pub const QUAD_FRAME: u32 = 0x10;

/// A `PArray` (`+0` the count, bit 31: the data is not owned; `+4` the
/// data, or the element itself for one) resized to one element:
/// `elements` is the data block when `Some`, else the array keeps one
/// zero word inline.
pub(crate) fn element(h: &impl Host, array: u32) -> Option<u32> {
    let n = h.read::<u32>(array) & 0x7fff_ffff;
    match n {
        0 => None,
        1 => Some(array + 4),
        _ => Some(h.read(array + 4)),
    }
}

/// One vertex stream: built with `stride` bytes per vertex for six
/// vertices, its storage released when the size changed and it was
/// bound, its first element tagged with `kind` and `semantic`.
pub(crate) fn setup_stream(h: &mut impl Host, stream: u32, stride: u32, kind: u8, semantic: u32) {
    h.call_sym(BUFFER_CTOR, Conv::Thiscall(stream), &[stride, 6, 1]);
    let size = h.read::<u32>(stream + 4).wrapping_mul(h.read(stream));
    if h.read::<u32>(stream + 0x38) != size {
        if h.call_sym(IS_BOUND, Conv::Thiscall(stream), &[]) & 0xff != 0 {
            h.call_sym(RELEASE, Conv::Thiscall(stream), &[]);
        }
        h.write(stream + 0x38, size);
    }
    let desc: u32 = h.read(stream + 0xc);
    h.write(desc + 8, kind);
    let semantic = h.rebase(semantic);
    h.write(desc + 4, semantic);
    h.write(desc, 0u32);
}

/// The stream's unmap: slot `+0x10` + 1 (0 once it reaches the count at
/// `+0x18`); stored with `+0x14` cleared when the unmap returns 0.
/// Returns the unmap's result.
pub(crate) fn unmap(h: &mut impl Host, stream: u32, data: u32) -> u32 {
    let next = h.read::<u32>(stream + 0x10).wrapping_add(1);
    let count = h.read::<u32>(stream + 0x18) & 0x7fff_ffff;
    let slot = if next == count { 0 } else { next };
    let r = h.call_sym(UNMAP, Conv::Thiscall(stream), &[slot, data]);
    if r == 0 {
        h.write(stream + 0x10, slot);
        h.write(stream + 0x14, 0u8);
    }
    r
}

/// A `PArray` of one-word elements just created (`array`) made to hold
/// one zero word inline, unless its count word is 1: the old data freed
/// when owned (bit 31 clear) and not inline.
pub(crate) fn one_word(h: &mut impl Host, array: u32) {
    let word: u32 = h.read(array);
    if word != 1 {
        let old = element(h, array).unwrap_or(0);
        let inline = array + 4;
        h.write(inline, 0u32);
        if old != inline && h.read::<u32>(array) >> 31 == 0 {
            h.call_sym(FREE, Conv::Cdecl, &[old]);
        }
        h.write(array, 1u32);
    }
}

/// A mesh's segment `PArray` just created (`array`) made to hold one
/// fresh 0x6c-byte segment, unless its count word is 1: the old segments
/// destroyed when owned and not the new block; nothing when the
/// allocation fails.
pub(crate) fn one_segment(h: &mut impl Host, array: u32) {
    if h.read::<u32>(array) != 1 {
        let data = h.call_sym(MALLOC, Conv::Cdecl, &[0x6c, 4]);
        if data != 0 {
            h.call_sym(SEGMENTS_CONSTRUCT, Conv::Cdecl, &[data, 1]);
            let word: u32 = h.read(array);
            let old: u32 = h.read(array + 4);
            if old != data && word >> 31 == 0 {
                h.call_sym(SEGMENTS_DESTROY, Conv::Cdecl, &[old, word & 0x7fff_ffff]);
            }
            h.write(array + 4, data);
            h.write(array, 1u32);
        }
    }
}

/// A segment's two six-vertex streams (`+0x28`, made when it does not
/// hold 2): positions (12 bytes, kind 2) and texture coordinates (8 bytes,
/// kind 1). The segment's `+0x10` becomes 2. Returns the two streams.
pub(crate) fn segment_streams(h: &mut impl Host, segment: u32) -> (u32, u32) {
    h.write(segment + 0x10, 2u32);
    if h.read::<u32>(segment + 0x28) != 2 {
        let data = h.call_sym(MALLOC, Conv::Cdecl, &[0x80, 4]);
        if data != 0 {
            for k in 0..2u32 {
                let at = data.wrapping_add(k << 6);
                if at != 0 {
                    h.call_sym(STREAM_CTOR, Conv::Thiscall(at), &[]);
                }
            }
            h.call_sym(SET_BUFFER, Conv::Thiscall(segment + 0x28), &[data, 2]);
        }
    }
    let pos: u32 = h.read(segment + 0x2c);
    setup_stream(h, pos, 0xc, 2, POSITION_SEMANTIC);
    let uv = h.read::<u32>(segment + 0x2c).wrapping_add(0x40);
    setup_stream(h, uv, 8, 1, UV_SEMANTIC);
    (pos, uv)
}

/// `SkipPrompt_buildQuad_006e7940(x, y, w, h, &mesh, &instance,
/// &materials, material)` (`0x006e7940`, stdcall, ret 0x20; `frame` see
/// [`QUAD_FRAME`]): creates in the scene's cluster (`+0x1284`) what is
/// missing (a mesh with one 0x6c-byte segment, a one-word material list,
/// a mesh instance), sets the material, gives the segment two 0x40-byte
/// vertex streams (positions: 12 bytes, texture coordinates: 8, six
/// vertices each) and fills them with the screen rectangle (x, y, w, h in
/// 1920x1080 pixels) as two triangles. Returns the second unmap's result.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the original's eight words
pub fn build_quad(
    h: &mut impl Host,
    frame: u32,
    rect: [u32; 4],
    mesh: u32,
    instance: u32,
    materials: u32,
    material: u32,
) -> u32 {
    let uv_data = frame;
    let pos_data = frame + 8;
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let cluster = scene.wrapping_add(0x1284);
    // the mesh, one segment
    if h.read::<u32>(mesh) == 0 {
        let t = h.rebase(MESH_TYPE);
        let array = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, t, 0]);
        h.write(mesh, array);
        one_segment(h, array);
    }
    // the material list, one word
    if h.read::<u32>(materials) == 0 {
        let t = h.rebase(MATERIALS_TYPE);
        let array = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, t, 0]);
        h.write(materials, array);
        one_word(h, array);
    }
    let list: u32 = h.read(materials);
    if let Some(slot) = element(h, list) {
        h.write(slot, material);
    }
    // the instance
    if h.read::<u32>(instance) == 0 {
        let mesh_array: u32 = h.read(mesh);
        let t = h.rebase(INSTANCE_TYPE);
        let made = h.call_sym(ALLOCATE_INSTANCES, Conv::Thiscall(cluster), &[1, t, 0]);
        let first = if made == 0 { 0 } else { h.read(made + 0x34) };
        let v = if first == 0 {
            0
        } else {
            h.call_sym(MESH_INSTANCE_CTOR, Conv::Thiscall(first), &[mesh_array])
        };
        h.write(instance, v);
    }
    let list: u32 = h.read(materials);
    let inst: u32 = h.read(instance);
    let target = if list == 0 {
        h.read::<u32>(inst).wrapping_add(0x30)
    } else {
        list
    };
    h.write(inst + 0x10, target);
    // the segment's two streams
    let segment: u32 = {
        let array: u32 = h.read(mesh);
        h.read(array + 4)
    };
    let (pos, uv) = segment_streams(h, segment);
    h.call_sym(MAP, Conv::Thiscall(pos), &[pos_data]);
    h.call_sym(MAP, Conv::Thiscall(uv), &[uv_data]);
    // the rectangle in clip space
    let width = f64::from_bits(h.read(h.rebase(SCREEN_WIDTH)));
    let height = f64::from_bits(h.read(h.rebase(SCREEN_HEIGHT)));
    let two = f64::from_bits(h.read(h.rebase(TWO)));
    let [x, y, w, hh] = rect.map(load);
    let left = store(sub(1.0, mul(div(add(x, w), width), two)));
    let right = store(sub(1.0, mul(div(x, width), two)));
    let bottom = store(sub(mul(div(add(y, hh), height), two), 1.0));
    let top = store(sub(mul(div(y, height), two), 1.0));
    let one = 1f32.to_bits();
    let p: u32 = h.read(pos_data);
    let vertices = [
        left, bottom, one, left, top, one, right, top, one, right, top, one, right, bottom, one,
        left, bottom, one,
    ];
    for (k, v) in (0u32..).zip(vertices) {
        h.write(p.wrapping_add(4 * k), v);
    }
    let t: u32 = h.read(uv_data);
    let uvs = [0, one, 0, 0, one, 0, one, 0, one, one, 0, one];
    for (k, v) in (0u32..).zip(uvs) {
        h.write(t.wrapping_add(4 * k), v);
    }
    unmap(h, pos, pos_data);
    unmap(h, uv, uv_data)
}
