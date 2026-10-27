//! Continuation round 11 of list `h4-glue-other`, batch 1: the menu
//! geometry's shader cluster and material.

use ffx_graphic::host::{Conv, Host};

pub use crate::h4_c3::{MENU_TEXTURE_TREE, TREE_SUCCESSOR};

/// `PhyreSceneCursor_Seek4(cursor)` (`ecx`, plain ret): fills the cursor's
/// `+0` (an offset), `+0xc` (found) and `+0x10` (the entry's base).
pub const CURSOR_SEEK: u32 = 0x0065_ecb0;
/// `DJB_hash(text)` (cdecl).
pub const DJB_HASH: u32 = 0x0067_b7d0;
/// `ClusterManager_GetInstance()` (cdecl).
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
/// `ClusterManager_getPClusterByName(this; name)` (thiscall, ret 4).
pub const CLUSTER_BY_NAME: u32 = 0x0069_b4b0;
/// `ShaderParams_FindByName(shader; name)` (thiscall, ret 4).
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `PhyreScene_getLinearSampler(scene; which)` (thiscall, ret 4).
pub const LINEAR_SAMPLER: u32 = 0x0065_e050;
/// `Phyre_PParameterBuffer_setValueDesc_0056e600(buffer; desc, src)`
/// (thiscall, ret 8).
pub const SET_VALUE_DESC: u32 = 0x0056_e600;
/// `PTextureIterator_ctor_00463bf0(this; list)` (thiscall, ret 4): fills
/// the iterator's `+0` and `+0x10` among others.
pub const TEXTURE_ITERATOR: u32 = 0x0046_3bf0;
/// `Render_SurfaceFormat_0066e4f0(buffer; param, record)` (thiscall, ret
/// 8) -> 0, or an error code.
pub const SURFACE_FORMAT: u32 = 0x0066_e4f0;
/// `PhyreScene_createMaterial_00659710(scene; variant, cluster, a, b)`
/// (thiscall, ret 0x10).
pub const CREATE_MATERIAL: u32 = 0x0065_9710;
/// `ShaderParamBuffer_SetValue(buffer; param, src)` (thiscall, ret 8).
pub const SET_VALUE: u32 = 0x0063_8780;
/// `PostProcessManager_GetInstance()` (cdecl).
pub const POST_PROCESS: u32 = 0x0069_2be0;
/// `operator_new(size)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// `TaggedTree_add_006a2830(tree; &key, &value)` (thiscall, ret 8).
pub const TREE_ADD: u32 = 0x006a_2830;
/// `g_PhyreScene` (`0x00ccb9d8`, a pointer).
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// A byte the function clears first (`0x00ccca54`).
pub const BUSY_BYTE: u32 = 0x00cc_ca54;
/// The cursor's type tag (`0x00ca5268`).
pub const CURSOR_TAG: u32 = 0x00ca_5268;
/// `"FrameBuffer"` (`0x00b45b3c`) and `"RealFrameBuffer"` (`0x00b45b2c`).
pub const FRAME_BUFFER: u32 = 0x00b4_5b3c;
/// See [`FRAME_BUFFER`].
pub const REAL_FRAME_BUFFER: u32 = 0x00b4_5b2c;
/// The parameter names: the texture (`0x00b3017c`), its sampler
/// (`0x00b47ec4`), the material colour (`0x00b41980`).
pub const TEXTURE_PARAM: u32 = 0x00b3_017c;
/// See [`TEXTURE_PARAM`].
pub const SAMPLER_PARAM: u32 = 0x00b4_7ec4;
/// See [`TEXTURE_PARAM`].
pub const COLOUR_PARAM: u32 = 0x00b4_1980;

/// Bytes of [`set_shader_cluster_and_material`]'s frame, from the
/// original's `ebp - 0x4c` up to its last argument word (`ebp + 0x14`):
/// the texture iterator (`+0`, `ebp-0x4c`), the cursor (`+0x20`,
/// `ebp-0x2c`, 0x20 bytes; its last four words are reused as the
/// texture record and the colour), the cluster (`+0x40`), the shader
/// (`+0x44`), `this` (`+0x48`), and the argument words (`+0x54`: the
/// name's hash, `+0x58`: the new node, `+0x5c`: the material), which the
/// original reuses as locals and hands the tree by address.
pub const MATERIAL_FRAME: u32 = 0x60;

/// `strcmp(a, b) == 0` as the original's inline loop.
pub(crate) fn same_text(h: &impl Host, a: u32, b: u32) -> bool {
    let mut k = 0u32;
    loop {
        let x: u8 = h.read(a.wrapping_add(k));
        if x != h.read::<u8>(b.wrapping_add(k)) {
            return false;
        }
        if x == 0 {
            return true;
        }
        k = k.wrapping_add(1);
    }
}

/// Element 0 of the material array at `this + 0x14` (a count word with
/// bit 31 a flag, then the element inline for one, or a pointer to the
/// elements for more) set to `value`; `None` for an empty array.
fn set_first_material(h: &mut impl Host, this: u32, value: u32) -> Option<()> {
    let array: u32 = h.read(this + 0x14);
    let n = h.read::<u32>(array) & 0x7fff_ffff;
    if n == 0 {
        return None;
    }
    let slot = if n > 1 { h.read(array + 4) } else { array + 4 };
    h.write(slot, value);
    Some(())
}

/// The texture of a material's colour record: a `PTextureIterator` over
/// `cluster` (built at `iter`), its `+0` plus its `+0x10`.
fn cluster_texture(h: &mut impl Host, iter: u32, cluster: u32) -> u32 {
    h.call_sym(TEXTURE_ITERATOR, Conv::Thiscall(iter), &[cluster]);
    let a: u32 = h.read(iter);
    a.wrapping_add(h.read(iter + 0x10))
}

/// The scene's linear sampler for `this`'s mesh (`[[this+0x24]+0x94]+0x24`).
fn sampler(h: &mut impl Host, this: u32) -> u32 {
    let instance: u32 = h.read(this + 0x24);
    let info: u32 = h.read(instance + 0x94);
    let which: u32 = h.read(info + 0x24);
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[which])
}

/// `MenuDynamicGeometry_setShaderClusterAndMaterial(this; cluster, name,
/// load)` (`0x006a3100`, thiscall, ret 0xc; `frame` see
/// [`MATERIAL_FRAME`]). Clears the byte at `0x00ccca54`, keeps a
/// non-null `cluster` at `+0x20`, and finds the shader in that cluster
/// through a scene cursor (none: 0x13). The menu texture tree's entry
/// for `DJB_hash(name)` (never `"FrameBuffer"`'s hash) gives the
/// material when its record is loaded or `load` is 0; otherwise the
/// texture cluster `name` (or the post-process frame buffers for
/// `"FrameBuffer"` and `"RealFrameBuffer"`) is bound to a material made
/// for the shader, and a new loaded record goes into the tree. The
/// material lands in element 0 of `+0x14`'s array. Returns 0, 1 (no
/// texture cluster: element 0 cleared), 3 (an empty array) or the
/// texture binding's error.
#[allow(clippy::too_many_lines)] // the original's three paths
pub fn set_shader_cluster_and_material(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    cluster: u32,
    name: u32,
    load: u32,
) -> u32 {
    let iter = frame;
    let cursor = frame + 0x20;
    let record = frame + 0x30;
    let a_hash = frame + 0x54;
    let a_node = frame + 0x58;
    let a_material = frame + 0x5c;
    h.write(a_hash, cluster);
    h.write(a_node, name);
    h.write(a_material, load);
    h.write(frame + 0x48, this);
    let busy = h.rebase(BUSY_BYTE);
    h.write(busy, 0u8);
    if cluster != 0 {
        h.write(this + 0x20, cluster);
    }
    let list = h.read::<u32>(this + 0x20).wrapping_add(0x1c);
    let head: u32 = h.read(list);
    let first = if head == list { 0 } else { head };
    for k in 0..5 {
        h.write(cursor + 4 * k, 0u32);
    }
    h.write(cursor + 0x14, list);
    h.write(cursor + 0x18, first);
    h.write(cursor + 0x1c, h.rebase(CURSOR_TAG));
    h.call_sym(CURSOR_SEEK, Conv::Thiscall(cursor), &[]);
    if h.read::<u32>(cursor + 0xc) == 0 {
        return 0x13;
    }
    let shader: u32 = {
        let base: u32 = h.read(cursor + 0x10);
        h.read(base.wrapping_add(h.read(cursor)))
    };
    h.write(frame + 0x44, shader);
    let hash = h.call_sym(DJB_HASH, Conv::Cdecl, &[name]);
    h.write(a_hash, hash);
    let frame_buffer = h.rebase(FRAME_BUFFER);
    let real_frame_buffer = h.rebase(REAL_FRAME_BUFFER);
    let frame_hash = h.call_sym(DJB_HASH, Conv::Cdecl, &[frame_buffer]);

    // the tree's entry for the hash
    let tree = h.rebase(MENU_TEXTURE_TREE);
    let mut node: u32 = h.read(tree + 0xc);
    if node != tree && node != 0 {
        loop {
            let next: u32 = h.read(node);
            if next == tree {
                break;
            }
            node = next;
        }
    }
    let mut found = None;
    if node != tree {
        let mut next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(tree), &[node]);
        loop {
            let rec: u32 = h.read(node + 0xc);
            if h.read::<u32>(rec) == hash && hash != frame_hash {
                found = Some((node, rec));
                break;
            }
            node = next;
            if next != tree {
                next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(tree), &[next]);
            }
            if node == tree {
                break;
            }
        }
    }

    let texture_param = h.rebase(TEXTURE_PARAM);
    let sampler_param = h.rebase(SAMPLER_PARAM);
    let colour_param = h.rebase(COLOUR_PARAM);

    if let Some((node, rec)) = found {
        let material: u32 = h.read(node + 0x10);
        if h.read::<u32>(rec + 4) == 1 {
            return set_first_material(h, this, material).map_or(3, |()| 0);
        }
        h.write(a_hash, material);
        if load == 0 {
            return set_first_material(h, this, material).map_or(3, |()| 0);
        }
        let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
        let texture_cluster = h.call_sym(CLUSTER_BY_NAME, Conv::Thiscall(manager), &[name]);
        h.write(a_node, texture_cluster);
        if texture_cluster == 0 {
            h.write(rec + 4, 0u32);
            return set_first_material(h, this, 0).map_or(3, |()| 1);
        }
        let texture = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[texture_param]);
        h.write(a_material, texture);
        let sampler_slot = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[sampler_param]);
        if sampler_slot != 0 {
            let buffer: u32 = h.read(material + 4);
            let s = sampler(h, this);
            h.call_sym(SET_VALUE_DESC, Conv::Thiscall(buffer), &[sampler_slot, s]);
        }
        if texture == 0 {
            return set_first_material(h, this, material).map_or(3, |()| 0);
        }
        let tex = cluster_texture(h, iter, texture_cluster);
        h.write(record + 0xc, tex);
        h.write(record, 0xffu32);
        h.write(record + 4, 0u32);
        let s = sampler(h, this);
        h.write(record + 8, s);
        let buffer: u32 = h.read(material + 4);
        let r = h.call_sym(SURFACE_FORMAT, Conv::Thiscall(buffer), &[texture, record]);
        if r != 0 {
            return r;
        }
        h.write(rec + 4, 1u32);
        return set_first_material(h, this, material).map_or(3, |()| 0);
    }

    // no entry: a new material
    let mut texture_cluster = 0;
    if load != 0 {
        let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
        texture_cluster = h.call_sym(CLUSTER_BY_NAME, Conv::Thiscall(manager), &[name]);
        h.write(frame + 0x40, texture_cluster);
        if texture_cluster == 0
            && !same_text(h, name, frame_buffer)
            && !same_text(h, name, real_frame_buffer)
        {
            return set_first_material(h, this, 0).map_or(3, |()| 1);
        }
    }
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let variant: u32 = h.read(this + 0x2c);
    let material = h.call_sym(
        CREATE_MATERIAL,
        Conv::Thiscall(scene),
        &[shader, variant, 0, 0],
    );
    h.write(a_material, material);
    let colour = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[colour_param]);
    if colour != 0 {
        for k in 0..4 {
            h.write(record + 4 * k, 1f32.to_bits());
        }
        let buffer: u32 = h.read(material + 4);
        h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[colour, record]);
    }
    let texture = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[texture_param]);
    let sampler_slot = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[sampler_param]);
    if load != 0 {
        h.write(a_node, texture);
    } else {
        h.write(a_node, sampler_slot);
    }
    if sampler_slot != 0 {
        let buffer: u32 = h.read(material + 4);
        let s = sampler(h, this);
        h.call_sym(SET_VALUE_DESC, Conv::Thiscall(buffer), &[sampler_slot, s]);
    }
    if texture == 0 {
        return set_first_material(h, this, 0).map_or(3, |()| 0);
    }
    h.write(record, 0xffu32);
    let tex = if load == 0 {
        0
    } else if same_text(h, name, frame_buffer) {
        post_process_buffer(h, 0x1_232c)
    } else if same_text(h, name, real_frame_buffer) {
        post_process_buffer(h, 0x9c)
    } else {
        cluster_texture(h, iter, texture_cluster)
    };
    h.write(record + 0xc, tex);
    h.write(record + 4, 0u32);
    let s = sampler(h, this);
    h.write(record + 8, s);
    let buffer: u32 = h.read(material + 4);
    let r = h.call_sym(SURFACE_FORMAT, Conv::Thiscall(buffer), &[texture, record]);
    if r != 0 {
        return r;
    }
    let new = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[8]);
    if new != 0 {
        h.write(new, 0u32);
        h.write(new + 4, 0u32);
    }
    h.write(new, hash);
    h.write(a_node, new);
    h.write(new + 4, 1u32);
    h.call_sym(TREE_ADD, Conv::Thiscall(tree), &[a_node, a_material]);
    set_first_material(h, this, material).map_or(3, |()| 0)
}

/// The post-process manager's frame buffer texture at `field`: `0` when
/// its `+4` byte is set, else its `+0`.
fn post_process_buffer(h: &mut impl Host, field: u32) -> u32 {
    let manager = h.call_sym(POST_PROCESS, Conv::Cdecl, &[]);
    let t: u32 = h.read(manager.wrapping_add(field));
    if h.read::<u8>(t + 4) == 0 {
        h.read(t)
    } else {
        0
    }
}
