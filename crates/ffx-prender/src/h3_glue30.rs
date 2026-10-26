//! The render-side glue, thirtieth batch (M4 port round 7, list
//! `h3-glue-render`): a mesh instance registered under a key. The
//! exception frame and cookie are left out, as in [`crate::glue10`].

/// `operator new(n)` (cdecl), `allocateInstanceList(cluster; 1, type,
/// 0)` and `PCluster::create(cluster; 1, type, 0)` (thiscall, `ret 0xc`),
/// the `PMeshInstance` constructor (`ret 4`), the matrix type
/// `0x00c92090`, the bounds type and `PMeshInstanceBounds_link_004ff800
/// (this; instance)` (`ret 4`), `PMeshInstanceBounds::setMinAndSize(this;
/// &min, &size)` (`ret 8`), `SearchTree_findSigned_0064b9a0(tree; &key)`
/// (`ret 4`), `Array_Reserve_00667f00(array; n)` (`ret 4`),
/// `PRedBlackTree_insertArray_0065fca0(tree; &key, &array)` (`ret 8`) and
/// `user_free_thunk(p)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const ALLOCATE_INSTANCE_LIST: u32 = crate::p6_quads::ALLOCATE_INSTANCE_LIST;
pub const CLUSTER_CREATE: u32 = crate::p6_quads::CLUSTER_CREATE;
pub const INSTANCE_TYPE: u32 = crate::p6_quads::INSTANCE_TYPE;
pub const INSTANCE_CTOR: u32 = crate::p6_quads::INSTANCE_CTOR;
pub const MATRIX_TYPE: u32 = 0x00c9_2090;
pub const BOUNDS_TYPE: u32 = 0x00ca_89b8;
pub const BOUNDS_LINK: u32 = 0x004f_f800;
pub const SET_MIN_AND_SIZE: u32 = 0x0050_0c20;
pub const TREE_FIND: u32 = 0x0064_b9a0;
pub const ARRAY_RESERVE: u32 = 0x0066_7f00;
pub const TREE_INSERT_ARRAY: u32 = 0x0065_fca0;
pub const USER_FREE: u32 = 0x0068_7010;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// An instance from `cluster`'s list of `ty` (its `+0x34`), built by
    /// `ctor(instance; arg)`; 0 without a list or an instance. `ebp -
    /// 0x18` and `ebp - 0x14` take the instance when there is one.
    fn instance(h: &mut impl Host, ebp: u32, cluster: u32, ty: u32, ctor: u32, arg: u32) -> u32 {
        let ty = h.rebase(ty);
        let list = h.call_sym(ALLOCATE_INSTANCE_LIST, Conv::Thiscall(cluster), &[1, ty, 0]);
        if list == 0 {
            return 0;
        }
        let at: u32 = h.read(list + 0x34);
        h.write(ebp - 0x18, at);
        if at == 0 {
            return 0;
        }
        h.write(ebp - 0x14, at);
        h.call_sym(ctor, Conv::Thiscall(at), &[arg])
    }

    /// A float word as `fld m32` / `fstp m32` copy it.
    fn copied(h: &impl Host, at: u32) -> u32 {
        x87::store(x87::load(h.read(at)))
    }

    /// `0x0066f7d0(this, model, key, shared; slots)` (thiscall, `ret 0xc`;
    /// frame: the min record at `ebp - 0x3c`, the size record at `- 0x2c`,
    /// a one-entry array at `- 0x28`, the scene, then its map, at `-
    /// 0x1c`): on the cluster `+0x1234` (`shared`'s low byte set) or
    /// `+0x11e4` a mesh instance of `model`'s mesh (`*model`) gets a matrix
    /// (`+4`); with bounds (`model + 0x1c`: min `+0x10`, size `+0`) a
    /// bounds object is linked and sized. The record `{instance, matrix,
    /// bounds}` (operator new, 12 bytes) is appended to the array under
    /// `key` in the map `+0xf9d4` (a new one-entry array inserted when the
    /// key is missing). `model + 0x18`'s object, when set, makes the
    /// instance's `+0x18` (virtual 1 of its `+4` with the cluster and
    /// itself). Returns the instance; the cluster goes over `shared`'s
    /// slot.
    pub fn scene_instantiate_mesh(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        this: u32,
        model: u32,
        shared: u32,
    ) -> u32 {
        h.write(ebp - 0x1c, this);
        let cluster = if shared as u8 == 0 {
            this + 0x11e4
        } else {
            this + 0x1234
        };
        h.write(slots + 8, cluster);
        let record = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xc]);
        if record != 0 {
            for k in 0..3 {
                h.write(record + 4 * k, 0u32);
            }
        }
        let mesh: u32 = h.read(model);
        let mesh_instance = instance(h, ebp, cluster, INSTANCE_TYPE, INSTANCE_CTOR, mesh);
        h.write(record, mesh_instance);
        let ty = h.rebase(MATRIX_TYPE);
        let matrix = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, ty, 0]);
        h.write(record.wrapping_add(4), matrix);
        h.write(mesh_instance.wrapping_add(4), matrix);
        let bounds_src: u32 = h.read(model + 0x1c);
        h.write(ebp - 0x10, bounds_src);
        if bounds_src != 0 {
            let bounds = instance(h, ebp, cluster, BOUNDS_TYPE, BOUNDS_LINK, mesh_instance);
            set_bounds(h, ebp, bounds, bounds_src);
            h.write(record.wrapping_add(8), bounds);
        }
        let map = h.read::<u32>(ebp - 0x1c).wrapping_add(0xf9d4);
        h.write(ebp - 0x1c, map);
        let node = h.call_sym(TREE_FIND, Conv::Thiscall(map), &[slots + 4]);
        if node != 0 && node.wrapping_add(0x10) != 0 {
            append(h, node.wrapping_add(0x10), record);
        } else {
            insert_new(h, ebp, slots, map, record);
        }
        let object: u32 = h.read(model + 0x18);
        if object != 0 {
            let owner: u32 = h.read(object + 4);
            let cluster: u32 = h.read(slots + 8);
            let value = h.call_virtual(owner, 1, &[cluster, object]);
            h.write(mesh_instance.wrapping_add(0x18), value);
        }
        mesh_instance
    }

    /// The bounds' min (`source + 0x10`) and size (`source + 0`) through
    /// the frame's scratch words into the records at `ebp - 0x3c` and
    /// `ebp - 0x2c`, then `setMinAndSize`.
    fn set_bounds(h: &mut impl Host, ebp: u32, bounds: u32, source: u32) {
        for k in 0..3 {
            let v = copied(h, source + 0x10 + 4 * k);
            h.write(ebp - 0x10 - 4 * k, v);
        }
        for k in 0..3 {
            let v = copied(h, ebp - 0x10 - 4 * k);
            h.write(ebp - 0x2c + 4 * k, v);
        }
        for (k, at) in [0x18, 0x14, 0x10].into_iter().enumerate() {
            let v = copied(h, source + 4 * k as u32);
            h.write(ebp - at, v);
        }
        for (k, at) in [0x18, 0x14, 0x10].into_iter().enumerate() {
            let v = copied(h, ebp - at);
            h.write(ebp - 0x3c + 4 * k as u32, v);
        }
        h.call_sym(
            SET_MIN_AND_SIZE,
            Conv::Thiscall(bounds),
            &[ebp - 0x3c, ebp - 0x2c],
        );
    }

    /// `record` appended to the array `{count, capacity, data}` at
    /// `array` (grown by an eighth plus 2 when full).
    fn append(h: &mut impl Host, array: u32, record: u32) {
        let count = h.read::<u32>(array).wrapping_add(1);
        h.write(array, count);
        if (count as i32) > h.read::<u32>(array + 4) as i32 {
            let grow = ((count as i32) / 8)
                .wrapping_add(2)
                .wrapping_add(count as i32);
            h.call_sym(ARRAY_RESERVE, Conv::Thiscall(array), &[grow as u32]);
        }
        let count: u32 = h.read(array);
        let data: u32 = h.read(array + 8);
        let at = data.wrapping_add(count.wrapping_mul(4)).wrapping_sub(4);
        if at != 0 {
            h.write(at, record);
        }
    }

    /// A one-entry array `{1, 0, 0}` at `ebp - 0x28` reserved for 3, its
    /// last slot given `record`, inserted under the key (`slots + 4`), its
    /// data freed.
    fn insert_new(h: &mut impl Host, ebp: u32, slots: u32, map: u32, record: u32) {
        let array = ebp - 0x28;
        h.write(array + 4, 0u32);
        h.write(array + 8, 0u32);
        h.write(array, 1u32);
        h.call_sym(ARRAY_RESERVE, Conv::Thiscall(array), &[3]);
        let count: u32 = h.read(array);
        let data: u32 = h.read(array + 8);
        let at = data.wrapping_add(count.wrapping_sub(1).wrapping_mul(4));
        if at != 0 {
            h.write(at, record);
        }
        h.call_sym(TREE_INSERT_ARRAY, Conv::Thiscall(map), &[slots + 4, array]);
        if data != 0 {
            h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
        }
    }
}
