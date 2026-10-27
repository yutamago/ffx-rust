//! Continuation round 11 of list `h4-glue-other`, batch 1: the loading
//! screen's meshes.

use ffx_bridge::x87::{load, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4_m2::{
    CLUSTER_CREATE, MAP, MATERIALS_TYPE, MESH_TYPE, element, one_segment, one_word,
    segment_streams, unmap,
};

/// `FullscreenMesh_createColoredTriangle_00a06120(cluster)` (cdecl) -> the
/// background's mesh instance.
pub const CREATE_TRIANGLE: u32 = 0x00a0_6120;
/// `Phyre_PRendering_PEffectVariant_createMaterial(variant; cluster, a,
/// b)` (thiscall, ret 0xc).
pub const CREATE_MATERIAL: u32 = 0x004e_6a40;
/// `PSharray_setWord_0048c540(array; index, value)` (thiscall, ret 8).
pub const SET_WORD: u32 = 0x0048_c540;
/// `ShaderParams_FindByName(variant; name)` (thiscall, ret 4).
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `Phyre_PParameterBuffer_setTextureChecked_0056e670(buffer; param,
/// record)` (thiscall, ret 8).
pub const SET_TEXTURE: u32 = 0x0056_e670;
/// `ShaderParamBuffer_SetValue(buffer; param, src)` (thiscall, ret 8).
pub const SET_VALUE: u32 = 0x0063_8780;
/// `Phyre_PCluster_allocateMeshInstance_00566680(cluster; mesh)`
/// (thiscall, ret 4) -> the instance.
pub const ALLOCATE_INSTANCE: u32 = 0x0056_6680;
/// `PhyreScene_getLinearSampler(scene; which)` (thiscall, ret 4).
pub const LINEAR_SAMPLER: u32 = 0x0065_e050;
/// `g_PhyreScene` (`0x00ccb9d8`, a pointer).
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// The parameter names: the texture (`0x00b425d8`), the colour
/// (`0x00b47a48`), the texture coordinates' scale (`0x00b425e4`).
pub const TEXTURE_PARAM: u32 = 0x00b4_25d8;
/// See [`TEXTURE_PARAM`].
pub const COLOUR_PARAM: u32 = 0x00b4_7a48;
/// See [`TEXTURE_PARAM`].
pub const UV_SCALE_PARAM: u32 = 0x00b4_25e4;
/// `f32` constants: the colour's alpha 0.8 (`0x00b34254`), -1
/// (`0x00b92340`), the bar's corners 0.7375 (`0x00b4b36c`), 0.8708
/// (`0x00b4b370`), -0.6407 (`0x00b4b380`), -0.8778 (`0x00b4b384`).
pub const ALPHA: u32 = 0x00b3_4254;
/// See [`ALPHA`].
pub const MINUS_ONE: u32 = 0x00b9_2340;
/// See [`ALPHA`].
pub const LEFT: u32 = 0x00b4_b36c;
/// See [`ALPHA`].
pub const RIGHT: u32 = 0x00b4_b370;
/// See [`ALPHA`].
pub const TOP: u32 = 0x00b4_b380;
/// See [`ALPHA`].
pub const BOTTOM: u32 = 0x00b4_b384;
/// Bytes of [`prepare_mesh`]'s frame, from the original's `ebp - 0x60`:
/// the second texture record (`+0`), the first texture record, later the
/// scale (`+0x10`), the colour (`+0x20`), the texture coordinates'
/// mapping (`+0x30`) and the positions' (`+0x38`).
pub const MESH_FRAME: u32 = 0x40;

/// `f32` bits as the original's `fld dword` / `fst dword` copies them.
fn copy(h: &impl Host, sym: u32) -> u32 {
    store(load(h.read(h.rebase(sym))))
}

/// The first material of the instance's material list (`+0x10`): its
/// parameter buffer (`+4`).
fn first_buffer(h: &impl Host, instance: u32) -> u32 {
    let list: u32 = h.read(instance + 0x10);
    let slot = element(h, list).unwrap_or(0);
    let material: u32 = h.read(slot);
    h.read(material + 4)
}

/// `LoadingScreen_prepareMesh(this)` (`0x006e4920`, `ecx`, plain ret;
/// `frame` see [`MESH_FRAME`]): with a texture list (`+0x20`), a texture
/// (`+0x2c`) and an effect variant (`+0x30`), builds in the cluster
/// (`+0x34`) the background triangle (`+0x38`) and a six-vertex quad mesh
/// (`+0x3c`, `+0x40` its instance), each with a fresh material: the
/// background textured with `+0x2c`, the quad with `[+0x24]` through the
/// scene's sampler 2; both coloured (1, 1, 1, 0.8), the scale (1, -1, 0,
/// 1). Returns the last `SetValue`'s result, `None` without the inputs
/// (the original leaves its frame's address in `eax`).
#[allow(clippy::too_many_lines)] // the original's straight run
pub fn prepare_mesh(h: &mut impl Host, frame: u32, this: u32) -> Option<u32> {
    let record2 = frame;
    let record = frame + 0x10;
    let colour = frame + 0x20;
    let uv_data = frame + 0x30;
    let pos_data = frame + 0x38;
    let count = h.read::<u32>(this + 0x20) & 0x7fff_ffff;
    if count == 0 || h.read::<u32>(this + 0x2c) == 0 || h.read::<u32>(this + 0x30) == 0 {
        return None;
    }
    let cluster: u32 = h.read(this + 0x34);
    let triangle = h.call_sym(CREATE_TRIANGLE, Conv::Cdecl, &[cluster]);
    h.write(this + 0x38, triangle);
    let materials_type = h.rebase(MATERIALS_TYPE);
    let cluster: u32 = h.read(this + 0x34);
    let list = h.call_sym(
        CLUSTER_CREATE,
        Conv::Thiscall(cluster),
        &[1, materials_type, 0],
    );
    one_word(h, list);
    let variant: u32 = h.read(this + 0x30);
    let cluster: u32 = h.read(this + 0x34);
    let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[cluster, 0, 0]);
    h.call_sym(SET_WORD, Conv::Thiscall(list), &[0, material]);
    let triangle: u32 = h.read(this + 0x38);
    h.write(record, 0x42u32);
    h.write(triangle + 0x10, list);
    let texture: u32 = h.read(this + 0x2c);
    h.write(record + 0xc, texture);
    h.write(record + 8, 0u32);
    h.write(record + 4, 0u32);
    let texture_param = h.rebase(TEXTURE_PARAM);
    let colour_param = h.rebase(COLOUR_PARAM);
    let scale_param = h.rebase(UV_SCALE_PARAM);
    let one = 1f32.to_bits();
    // the background's texture, colour and scale
    let buffer = first_buffer(h, h.read(this + 0x38));
    let variant: u32 = h.read(this + 0x30);
    let p = h.call_sym(FIND_PARAM, Conv::Thiscall(variant), &[texture_param]);
    h.call_sym(SET_TEXTURE, Conv::Thiscall(buffer), &[p, record]);
    for k in 0..3 {
        h.write(colour + 4 * k, one);
    }
    let alpha = copy(h, ALPHA);
    h.write(colour + 0xc, alpha);
    let buffer = first_buffer(h, h.read(this + 0x38));
    let variant: u32 = h.read(this + 0x30);
    let p = h.call_sym(FIND_PARAM, Conv::Thiscall(variant), &[colour_param]);
    h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, colour]);
    h.write(record, one);
    let minus = copy(h, MINUS_ONE);
    h.write(record + 4, minus);
    h.write(record + 8, 0u32);
    h.write(record + 0xc, one);
    let buffer = first_buffer(h, h.read(this + 0x38));
    let variant: u32 = h.read(this + 0x30);
    let p = h.call_sym(FIND_PARAM, Conv::Thiscall(variant), &[scale_param]);
    h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, record]);
    // the quad's mesh
    let mesh_type = h.rebase(MESH_TYPE);
    let cluster: u32 = h.read(this + 0x34);
    let mesh = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, mesh_type, 0]);
    h.write(this + 0x3c, mesh);
    one_segment(h, mesh);
    let segment: u32 = {
        let mesh: u32 = h.read(this + 0x3c);
        h.read(mesh + 4)
    };
    let (pos, uv) = segment_streams(h, segment);
    h.call_sym(MAP, Conv::Thiscall(pos), &[pos_data]);
    h.call_sym(MAP, Conv::Thiscall(uv), &[uv_data]);
    let (l, r) = (copy(h, LEFT), copy(h, RIGHT));
    let (t, b) = (copy(h, TOP), copy(h, BOTTOM));
    let vertices = [
        l, b, one, r, b, one, l, t, one, r, t, one, l, t, one, r, b, one,
    ];
    let p: u32 = h.read(pos_data);
    for (k, v) in (0u32..).zip(vertices) {
        h.write(p.wrapping_add(4 * k), v);
    }
    let uvs = [0, 0, one, 0, 0, one, one, one, 0, one, one, 0];
    let q: u32 = h.read(uv_data);
    for (k, v) in (0u32..).zip(uvs) {
        h.write(q.wrapping_add(4 * k), v);
    }
    unmap(h, pos, pos_data);
    unmap(h, uv, uv_data);
    let cluster: u32 = h.read(this + 0x34);
    let mesh: u32 = h.read(this + 0x3c);
    let instance = h.call_sym(ALLOCATE_INSTANCE, Conv::Thiscall(cluster), &[mesh]);
    h.write(this + 0x40, instance);
    // the quad's material
    let cluster: u32 = h.read(this + 0x34);
    let list = h.call_sym(
        CLUSTER_CREATE,
        Conv::Thiscall(cluster),
        &[1, materials_type, 0],
    );
    one_word(h, list);
    let variant: u32 = h.read(this + 0x30);
    let cluster: u32 = h.read(this + 0x34);
    let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[cluster, 0, 0]);
    h.call_sym(SET_WORD, Conv::Thiscall(list), &[0, material]);
    let instance: u32 = h.read(this + 0x40);
    h.write(instance + 0x10, list);
    h.write(record2, 0x42u32);
    let textures: u32 = h.read(this + 0x24);
    let texture: u32 = h.read(textures);
    h.write(record2 + 0xc, texture);
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let sampler = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[2]);
    h.write(record2 + 8, sampler);
    h.write(record2 + 4, 0u32);
    let buffer: u32 = h.read(material + 4);
    let variant: u32 = h.read(this + 0x30);
    let p = h.call_sym(FIND_PARAM, Conv::Thiscall(variant), &[texture_param]);
    h.call_sym(SET_TEXTURE, Conv::Thiscall(buffer), &[p, record2]);
    let variant: u32 = h.read(this + 0x30);
    let p = h.call_sym(FIND_PARAM, Conv::Thiscall(variant), &[colour_param]);
    let buffer: u32 = h.read(material + 4);
    h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, colour]);
    let variant: u32 = h.read(this + 0x30);
    let p = h.call_sym(FIND_PARAM, Conv::Thiscall(variant), &[scale_param]);
    let buffer: u32 = h.read(material + 4);
    Some(h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[p, record]))
}
